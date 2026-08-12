//! `kern-exec` — the typed execution runtime.
//!
//! Nothing runs as a shell string. A [`TypedCommand`] is `program + argv + cwd + env +
//! timeout`, authorized by a [`CapabilityGrant`] before it launches. The child's
//! environment is cleared and repopulated only from an explicit allowlist (secret
//! isolation), and the child is placed in its own process group so that a timeout — or
//! dropping the runtime — tears down the whole tree, leaving no orphaned grandchildren.
//!
//! A note on `unsafe`: process-group teardown requires `killpg`, which safe std does not
//! expose. This crate contains exactly one audited FFI call for that purpose; it is the
//! only `unsafe` in the workspace.

pub mod sandbox;

use std::collections::BTreeMap;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

use kern_capability::{AccessRequest, CapabilityGrant, Denied};
use kern_types::RiskClass;

/// The result of a typed execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecOutcome {
    /// The process exited on its own.
    Exited {
        /// Exit code, if the process exited normally.
        code: Option<i32>,
        /// Captured stdout.
        stdout: String,
        /// Captured stderr.
        stderr: String,
    },
    /// The process exceeded its timeout and its process group was killed.
    TimedOut {
        /// Any stdout captured before the kill.
        stdout: String,
        /// Any stderr captured before the kill.
        stderr: String,
    },
}

/// Error running a typed command.
#[derive(Debug)]
pub enum ExecError {
    /// The capability refused the execution.
    Denied(Denied),
    /// The process could not be spawned.
    Spawn(std::io::Error),
    /// An I/O error while supervising.
    Io(std::io::Error),
}

impl core::fmt::Display for ExecError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ExecError::Denied(d) => write!(f, "denied: {d}"),
            ExecError::Spawn(e) => write!(f, "spawn failed: {e}"),
            ExecError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for ExecError {}

/// A fully-specified, shell-free command.
#[derive(Debug, Clone)]
pub struct TypedCommand {
    program: String,
    argv: Vec<String>,
    cwd: PathBuf,
    env: BTreeMap<String, String>,
    timeout: Duration,
}

impl TypedCommand {
    /// Build a command to run `program` with `argv` in `cwd`.
    #[must_use]
    pub fn new(program: impl Into<String>, argv: Vec<String>, cwd: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            argv,
            cwd: cwd.into(),
            env: BTreeMap::new(),
            timeout: Duration::from_secs(30),
        }
    }

    /// Add an allowlisted environment variable (the child sees only these plus `PATH`).
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set the wall-clock timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Authorize against `grant`, then run under process-group supervision. On timeout the
    /// whole process group is killed; in all cases the group is torn down before return so
    /// no grandchild is left orphaned.
    pub fn run(
        &self,
        grant: &mut CapabilityGrant,
        now: SystemTime,
    ) -> Result<ExecOutcome, ExecError> {
        // Authorize: binds this exec to the grant's agent/mission/repo, and runs the
        // program/argv through the capability's exec scope (allowlist + arg-injection).
        let cap = grant.capability();
        let mut req = AccessRequest::new(
            cap.agent().clone(),
            cap.mission().clone(),
            cap.repository().clone(),
            "exec",
            RiskClass::Executing,
        );
        req.exec = Some((self.program.clone(), self.argv.clone()));
        req.fs_reads = vec![self.cwd.to_string_lossy().into_owned()];
        grant.authorize(&req, now).map_err(ExecError::Denied)?;

        let mut cmd = Command::new(&self.program);
        cmd.args(&self.argv);
        cmd.current_dir(&self.cwd);
        cmd.env_clear();
        cmd.env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.process_group(0); // new group; pgid == child pid

        let mut child = cmd.spawn().map_err(ExecError::Spawn)?;
        let pgid = child.id();

        let start = Instant::now();
        let status = loop {
            if let Some(s) = child.try_wait().map_err(ExecError::Io)? {
                break Some(s);
            }
            if start.elapsed() >= self.timeout {
                break None;
            }
            std::thread::sleep(Duration::from_millis(10));
        };

        // Tear down the whole group: kills any lingering (grand)children in the group.
        kill_group(pgid);

        let mut stdout = String::new();
        let mut stderr = String::new();
        if let Some(mut o) = child.stdout.take() {
            let _ = o.read_to_string(&mut stdout);
        }
        if let Some(mut e) = child.stderr.take() {
            let _ = e.read_to_string(&mut stderr);
        }
        let _ = child.wait();

        match status {
            Some(s) => Ok(ExecOutcome::Exited {
                code: s.code(),
                stdout,
                stderr,
            }),
            None => Ok(ExecOutcome::TimedOut { stdout, stderr }),
        }
    }
}

/// The one audited FFI: send `SIGKILL` to an entire process group.
#[allow(unsafe_code)]
mod ffi {
    extern "C" {
        pub fn killpg(pgrp: i32, sig: i32) -> i32;
    }
}

const SIGKILL: i32 = 9;

/// Kill an entire process group (best-effort; `ESRCH` when already gone is fine).
#[allow(unsafe_code)]
fn kill_group(pgid: u32) {
    // A pid/pgid always fits in i32; if it somehow does not, refuse rather than risk a
    // wrapped value targeting the wrong group.
    let Ok(pg) = i32::try_from(pgid) else {
        return;
    };
    // SAFETY: `killpg` is an async-signal-safe libc call taking two ints. `pg` is the
    // group id of a child we placed in its own group via `process_group(0)`, so this can
    // only target that group (never our own). A failure (e.g. the group already exited)
    // is ignored.
    unsafe {
        let _ = ffi::killpg(pg, SIGKILL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kern_capability::{Capability, CapabilitySpec, ExecScope, FsScope, NetworkMode};
    use kern_types::{AgentId, Deadline, MissionId, RepositoryId};
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::UNIX_EPOCH;

    static N: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let n = N.fetch_add(1, Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!(
                "openkern-exec-{}-{nanos:x}-{n:x}",
                std::process::id()
            ));
            std::fs::create_dir_all(&p).unwrap();
            TempDir(std::fs::canonicalize(&p).unwrap())
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn now() -> SystemTime {
        SystemTime::now()
    }

    /// A grant that allows running `/bin/sh` in `cwd` (arg-prefix denylist cleared, since
    /// these tests deliberately drive `sh -c`).
    fn sh_grant(cwd: &Path) -> CapabilityGrant {
        let spec = CapabilitySpec {
            agent: AgentId::new("agent-exec").unwrap(),
            mission: MissionId::new("mission-exec").unwrap(),
            repository: RepositoryId::new("repo-exec").unwrap(),
            worktree: None,
            operations: vec!["exec".into()],
            fs: FsScope::root(cwd.to_path_buf()),
            exec: ExecScope::new(["/bin/sh"]).deny_arg_prefixes(vec![]),
            network: NetworkMode::DenyAll,
            deadline: Deadline::after(now(), Duration::from_secs(100_000)),
            risk_ceiling: RiskClass::Executing,
        };
        Capability::new(spec).grant(50)
    }

    #[test]
    fn runs_and_captures_output() {
        let t = TempDir::new();
        let mut g = sh_grant(&t.0);
        let cmd = TypedCommand::new(
            "/bin/sh",
            vec!["-c".into(), "printf hello".into()],
            t.0.clone(),
        );
        let out = cmd.run(&mut g, now()).unwrap();
        match out {
            ExecOutcome::Exited { code, stdout, .. } => {
                assert_eq!(code, Some(0));
                assert_eq!(stdout, "hello");
            }
            ExecOutcome::TimedOut { .. } => panic!("expected Exited, got TimedOut"),
        }
    }

    #[test]
    fn environment_is_isolated_to_allowlist() {
        let t = TempDir::new();
        // A secret in the parent environment must NOT reach the child.
        std::env::set_var("OPENKERN_SECRET_ISO", "leaked");
        let mut g = sh_grant(&t.0);
        let cmd = TypedCommand::new(
            "/bin/sh",
            vec![
                "-c".into(),
                "printf 'SEC=[%s] ALLOW=[%s]' \"$OPENKERN_SECRET_ISO\" \"$ALLOWED\"".into(),
            ],
            t.0.clone(),
        )
        .env("ALLOWED", "ok");
        let out = cmd.run(&mut g, now()).unwrap();
        std::env::remove_var("OPENKERN_SECRET_ISO");
        match out {
            ExecOutcome::Exited { stdout, .. } => {
                assert_eq!(stdout, "SEC=[] ALLOW=[ok]", "secret leaked into child env");
            }
            ExecOutcome::TimedOut { .. } => panic!("expected Exited, got TimedOut"),
        }
    }

    #[test]
    fn timeout_kills_long_process() {
        let t = TempDir::new();
        let mut g = sh_grant(&t.0);
        let cmd = TypedCommand::new("/bin/sh", vec!["-c".into(), "sleep 30".into()], t.0.clone())
            .timeout(Duration::from_millis(300));
        let start = Instant::now();
        let out = cmd.run(&mut g, now()).unwrap();
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "did not time out promptly"
        );
        assert!(
            matches!(out, ExecOutcome::TimedOut { .. }),
            "expected TimedOut, got {out:?}"
        );
    }

    #[test]
    fn grandchild_is_reaped_no_orphan_with_positive_control() {
        let t = TempDir::new();

        // POSITIVE CONTROL: the grandchild really does create its marker when allowed to
        // run to completion.
        let marker_ok = t.0.join("grandchild_ran.marker");
        let mut g1 = sh_grant(&t.0);
        let control = TypedCommand::new(
            "/bin/sh",
            vec![
                "-c".into(),
                format!("(sleep 1; touch '{}') & wait", marker_ok.display()),
            ],
            t.0.clone(),
        )
        .timeout(Duration::from_secs(10));
        control.run(&mut g1, now()).unwrap();
        assert!(
            marker_ok.exists(),
            "positive control: grandchild should have run"
        );

        // CONTAINED: a grandchild that would fire after the timeout must be killed with the
        // group and never create its marker.
        let marker_never = t.0.join("orphan.marker");
        let mut g2 = sh_grant(&t.0);
        let contained = TypedCommand::new(
            "/bin/sh",
            vec![
                "-c".into(),
                format!("(sleep 3; touch '{}') & sleep 30", marker_never.display()),
            ],
            t.0.clone(),
        )
        .timeout(Duration::from_millis(500));
        let out = contained.run(&mut g2, now()).unwrap();
        assert!(matches!(out, ExecOutcome::TimedOut { .. }));
        // Wait past when the orphaned grandchild WOULD have fired.
        std::thread::sleep(Duration::from_secs(4));
        assert!(
            !marker_never.exists(),
            "ORPHAN_PROCESS: grandchild survived group teardown"
        );
    }

    #[test]
    fn unauthorized_program_is_denied_before_spawn() {
        let t = TempDir::new();
        // grant only permits git, not /bin/sh
        let spec = CapabilitySpec {
            agent: AgentId::new("a").unwrap(),
            mission: MissionId::new("m").unwrap(),
            repository: RepositoryId::new("r").unwrap(),
            worktree: None,
            operations: vec!["exec".into()],
            fs: FsScope::root(t.0.clone()),
            exec: ExecScope::new(["git"]),
            network: NetworkMode::DenyAll,
            deadline: Deadline::after(now(), Duration::from_secs(100_000)),
            risk_ceiling: RiskClass::Executing,
        };
        let mut g = Capability::new(spec).grant(1);
        let cmd = TypedCommand::new("/bin/sh", vec!["-c".into(), "echo hi".into()], t.0.clone());
        let err = cmd.run(&mut g, now()).unwrap_err();
        assert!(matches!(err, ExecError::Denied(_)), "got {err:?}");
        assert_eq!(g.uses_remaining(), 1, "denied exec must not spend a use");
    }
}
