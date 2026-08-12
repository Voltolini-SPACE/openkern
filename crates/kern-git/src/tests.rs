//! Tests for the governed git layer.
//!
//! Positive controls (`unhardened` profile) demonstrate that the attack really fires when
//! hardening is absent; the governed path (`hardened`) must then contain it. This proves
//! the tests measure real containment rather than asserting a tautology.

use super::*;
use kern_capability::{Capability, CapabilitySpec, ExecScope, FsScope, NetworkMode};
use kern_types::{AgentId, Deadline, MissionId, RepositoryId};
use std::fmt::Write as _;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
            "openkern-git-{}-{nanos:x}-{n:x}",
            std::process::id()
        ));
        fs::create_dir_all(&p).unwrap();
        TempDir(fs::canonicalize(&p).unwrap())
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn now() -> SystemTime {
    SystemTime::now()
}

fn init_repo(runner: &GitRunner, dir: &Path) -> RepositoryIdentity {
    fs::create_dir_all(dir).unwrap();
    runner.init(dir, dir).unwrap();
    runner
        .set_local_identity(dir, "OpenKern Test", "test@openkern.local", dir)
        .unwrap();
    RepositoryIdentity::resolve(dir).unwrap()
}

fn ident(repo: &Path) -> RepositoryIdentity {
    RepositoryIdentity::resolve(repo).unwrap()
}

fn grant_for(identity: &RepositoryIdentity, ops: &[&str]) -> CapabilityGrant {
    let spec = CapabilitySpec {
        agent: AgentId::new("agent-git").unwrap(),
        mission: MissionId::new("mission-git").unwrap(),
        repository: RepositoryId::new(identity.repository_id().as_str()).unwrap(),
        worktree: None,
        operations: ops.iter().map(|s| (*s).to_string()).collect(),
        fs: FsScope::root(identity.worktree()),
        exec: ExecScope::new(["git"]),
        network: NetworkMode::DenyAll,
        deadline: Deadline::after(now(), Duration::from_secs(100_000)),
        risk_ceiling: RiskClass::Mutating,
    };
    Capability::new(spec).grant(50)
}

const ALL_OPS: &[&str] = &["git.add", "git.commit", "git.branch", "git.restore"];

#[test]
fn transactional_commit_flow() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    let id = init_repo(&runner, &repo);
    let grant = grant_for(&id, ALL_OPS);

    let mut txn = GitTransaction::begin(runner.clone(), id, grant).unwrap();
    assert_eq!(txn.expected_head(), NULL_OID, "unborn branch");

    fs::write(repo.join("a.txt"), "hello\n").unwrap();
    txn.add(&["a.txt"], now()).unwrap();
    let head = txn.commit("first commit", now()).unwrap();
    assert_ne!(head, NULL_OID);
    assert_eq!(head.len(), 40, "sha1 head");
    assert_eq!(txn.expected_head(), head, "expected head advanced");

    let status = runner.status_porcelain(&repo, &repo).unwrap();
    assert!(status.trim().is_empty(), "clean tree, got: {status:?}");
    assert_eq!(txn.uses_remaining(), 48, "two mutations consumed two uses");
}

#[test]
fn head_moved_between_authorization_and_mutation_is_refused() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);

    let id1 = ident(&repo);
    let g1 = grant_for(&id1, ALL_OPS);
    let mut txn1 = GitTransaction::begin(runner.clone(), id1, g1).unwrap();
    fs::write(repo.join("a.txt"), "one\n").unwrap();
    txn1.add(&["a.txt"], now()).unwrap();
    let h1 = txn1.commit("c1", now()).unwrap();
    assert_eq!(txn1.expected_head(), h1);

    // A concurrent writer (second transaction) advances HEAD.
    let id2 = ident(&repo);
    let g2 = grant_for(&id2, ALL_OPS);
    let mut txn2 = GitTransaction::begin(runner.clone(), id2, g2).unwrap();
    fs::write(repo.join("b.txt"), "two\n").unwrap();
    txn2.add(&["b.txt"], now()).unwrap();
    let h2 = txn2.commit("c2", now()).unwrap();
    assert_ne!(h1, h2);

    // txn1's next mutation is refused: its expected HEAD (h1) no longer matches.
    let err = txn1.branch_create("feature", now()).unwrap_err();
    match err {
        GitError::HeadMoved { expected, actual } => {
            assert_eq!(expected, h1);
            assert_eq!(actual, h2);
        }
        other => panic!("expected HeadMoved, got {other:?}"),
    }
}

#[test]
fn operation_outside_grant_is_denied() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    let id = init_repo(&runner, &repo);
    let grant = grant_for(&id, &["git.add"]); // add allowed, commit not
    let mut txn = GitTransaction::begin(runner.clone(), id, grant).unwrap();

    fs::write(repo.join("a.txt"), "x\n").unwrap();
    txn.add(&["a.txt"], now()).unwrap();
    let err = txn.commit("nope", now()).unwrap_err();
    assert!(
        matches!(err, GitError::Denied(_)),
        "commit without grant must be denied, got {err:?}"
    );
}

#[test]
fn git_hooks_are_contained_with_positive_control() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);

    let marker = t.path().join("hook_ran.marker");
    let hook = repo.join(".git/hooks/pre-commit");
    fs::create_dir_all(hook.parent().unwrap()).unwrap();
    fs::write(&hook, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();

    // POSITIVE CONTROL: unhardened profile lets the hook fire.
    fs::write(repo.join("f1.txt"), "1\n").unwrap();
    let up = GitExecutionProfile::unhardened();
    runner
        .spawn(&["add", "--", "f1.txt"], &repo, &up, &repo)
        .unwrap();
    let c = runner
        .spawn(&["commit", "-m", "control"], &repo, &up, &repo)
        .unwrap();
    assert!(c.success, "control commit failed: {}", c.stderr);
    assert!(marker.exists(), "positive control: hook should have run");
    fs::remove_file(&marker).unwrap();

    // GOVERNED: hardened profile sets core.hooksPath=/dev/null; the hook must not run.
    fs::write(repo.join("f2.txt"), "2\n").unwrap();
    let hp = GitExecutionProfile::hardened();
    runner
        .spawn(&["add", "--", "f2.txt"], &repo, &hp, &repo)
        .unwrap();
    let c2 = runner
        .spawn(&["commit", "-m", "governed"], &repo, &hp, &repo)
        .unwrap();
    assert!(c2.success, "governed commit failed: {}", c2.stderr);
    assert!(!marker.exists(), "GIT_HOOKS containment: hook must not run");
}

#[test]
fn git_clean_filter_is_contained_with_positive_control() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);

    let marker = t.path().join("filter_ran.marker");
    // A real filter driver script (avoids git-config comment/quote pitfalls with `;`).
    let script = repo.join("evil-clean.sh");
    fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\ncat\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let cfg = repo.join(".git/config");
    let mut cfg_text = fs::read_to_string(&cfg).unwrap();
    let _ = write!(
        cfg_text,
        "[filter \"evil\"]\n\tclean = {}\n\trequired = false\n",
        script.display()
    );
    fs::write(&cfg, cfg_text).unwrap();
    fs::write(repo.join(".gitattributes"), "*.dat filter=evil\n").unwrap();

    // POSITIVE CONTROL: unhardened `add` runs the clean filter.
    fs::write(repo.join("x.dat"), "data-x\n").unwrap();
    let up = GitExecutionProfile::unhardened();
    let a = runner
        .spawn(&["add", "--", "x.dat"], &repo, &up, &repo)
        .unwrap();
    assert!(a.success, "control add failed: {}", a.stderr);
    assert!(
        marker.exists(),
        "positive control: clean filter should have run"
    );
    fs::remove_file(&marker).unwrap();

    // GOVERNED: hardened `add` neutralizes filter.evil.* — the filter must not run.
    fs::write(repo.join("y.dat"), "data-y\n").unwrap();
    let hp = GitExecutionProfile::hardened();
    let a2 = runner
        .spawn(&["add", "--", "y.dat"], &repo, &hp, &repo)
        .unwrap();
    assert!(a2.success, "governed add failed: {}", a2.stderr);
    assert!(
        !marker.exists(),
        "GIT_FILTERS containment: clean filter must not run"
    );
}

#[test]
fn hostile_global_config_is_ignored_with_positive_control() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);

    let fake_home = t.path().join("fakehome");
    fs::create_dir_all(&fake_home).unwrap();
    fs::write(
        fake_home.join(".gitconfig"),
        "[injecttest]\n\tvalue = fromglobal\n",
    )
    .unwrap();

    // POSITIVE CONTROL: unhardened reads the hostile global config.
    let up = GitExecutionProfile::unhardened();
    let got = runner
        .spawn(
            &["config", "--get", "injecttest.value"],
            &repo,
            &up,
            &fake_home,
        )
        .unwrap();
    assert!(
        got.success && got.stdout.trim() == "fromglobal",
        "positive control failed: {got:?}"
    );

    // GOVERNED: hardened redirects global config to /dev/null — the value is invisible.
    let hp = GitExecutionProfile::hardened();
    let denied = runner
        .spawn(
            &["config", "--get", "injecttest.value"],
            &repo,
            &hp,
            &fake_home,
        )
        .unwrap();
    assert!(!denied.success, "hostile global config must be ignored");
    assert!(denied.stdout.trim().is_empty());
}

/// Structural invariant: exactly one `git` spawn site in the whole workspace, and zero
/// raw shell spawns. The needle is assembled at runtime so this test's own source is not a
/// match.
#[test]
fn single_git_chokepoint_and_no_shell_escapes() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let crates = workspace.join("crates");

    let mut rs_files = Vec::new();
    collect_rs(&crates, &mut rs_files);
    assert!(
        rs_files.len() >= 8,
        "expected to scan the crates, found {}",
        rs_files.len()
    );

    let prefix = "Command::new(\"";
    let git_needle = format!("{prefix}git\")");
    let shell_needles: Vec<String> = ["sh", "bash", "zsh", "cmd", "powershell"]
        .iter()
        .map(|s| format!("{prefix}{s}\")"))
        .collect();

    let mut git_hits = 0usize;
    let mut shell_hits = 0usize;
    for f in &rs_files {
        let text = fs::read_to_string(f).unwrap();
        git_hits += text.matches(git_needle.as_str()).count();
        for needle in &shell_needles {
            shell_hits += text.matches(needle.as_str()).count();
        }
    }
    assert_eq!(
        git_hits, 1,
        "UNGOVERNED_GIT_CALLS must be 0 (exactly one chokepoint)"
    );
    assert_eq!(shell_hits, 0, "UNGOVERNED_SHELL_CALLS must be 0");
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}
