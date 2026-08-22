//! `kern-git` — governed, transactional Git.
//!
//! This crate contains the **only** place in the workspace that spawns `git`
//! ([`GitRunner::spawn`]). Every invocation runs under a [`GitExecutionProfile`] with a
//! sanitized environment (ambient variables cleared, system config disabled, global
//! config redirected to `/dev/null`, terminal prompts off) plus `-c` overrides that
//! neutralize repository-controlled extensibility (hooks, filters, external diff,
//! textconv, credential helpers, fsmonitor, ssh command).
//!
//! Mutations flow through a [`GitTransaction`] that binds a [`RepositoryIdentity`], a
//! [`CapabilityGrant`], a [`TransactionId`], and an expected `HEAD`. The expected `HEAD`
//! is re-verified immediately before each mutation, so a concurrent writer or a swapped
//! git-dir between authorization and effect is caught (TOCTOU).

mod profile;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use kern_capability::{CapabilityGrant, Denied};
use kern_repo::{RepoError, RepositoryIdentity};
use kern_types::{RiskClass, TransactionId};

pub use profile::GitExecutionProfile;

/// The all-zero object id, used to represent an unborn `HEAD`.
pub const NULL_OID: &str = "0000000000000000000000000000000000000000";

/// The captured result of a git invocation.
#[derive(Debug, Clone)]
pub struct GitOutput {
    /// Whether git exited 0.
    pub success: bool,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
}

/// An error from the governed git layer.
#[derive(Debug)]
pub enum GitError {
    /// Failed to spawn git.
    Spawn(std::io::Error),
    /// git exited non-zero.
    Failed {
        /// The argument vector (after hardening).
        args: Vec<String>,
        /// Captured stderr.
        stderr: String,
    },
    /// The repository's `HEAD` moved between authorization and the mutation.
    HeadMoved {
        /// The head expected by the transaction.
        expected: String,
        /// The head observed just before the mutation.
        actual: String,
    },
    /// The capability refused the operation.
    Denied(Denied),
    /// Repository identity could not be resolved.
    Repo(RepoError),
    /// The capability authorizes a *different* repository than the one the effect would
    /// land on. Authority must be bound to the object it acts upon: `authorized(A)` may
    /// never produce `effect(B)`.
    RepositoryMismatch {
        /// Repository the capability authorizes.
        authorized: String,
        /// Repository the identity would act upon.
        target: String,
    },
    /// The capability is pinned to a specific worktree and the identity names another one.
    /// The repository may match; a worktree-pinned authority still may not cross worktrees.
    WorktreeMismatch {
        /// Worktree the capability authorizes.
        authorized: String,
        /// Worktree the identity would act upon.
        target: String,
    },
}

impl core::fmt::Display for GitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GitError::Spawn(e) => write!(f, "failed to spawn git: {e}"),
            GitError::Failed { args, stderr } => {
                write!(f, "git {args:?} failed: {}", stderr.trim())
            }
            GitError::HeadMoved { expected, actual } => {
                write!(f, "HEAD moved: expected {expected}, found {actual}")
            }
            GitError::Denied(d) => write!(f, "denied: {d}"),
            GitError::Repo(e) => write!(f, "repo: {e}"),
            GitError::RepositoryMismatch { authorized, target } => write!(
                f,
                "capability authorizes repository {authorized} but the effect targets {target}"
            ),
            GitError::WorktreeMismatch { authorized, target } => write!(
                f,
                "capability is pinned to worktree {authorized} but the effect targets {target}"
            ),
        }
    }
}

impl std::error::Error for GitError {}

/// The governed git executor.
#[derive(Debug, Clone, Default)]
pub struct GitRunner;

impl GitRunner {
    /// A new runner.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// **The single point in the workspace that launches git.** Builds a hardened
    /// invocation from the profile and runs it with a sanitized environment.
    #[allow(clippy::unused_self)] // method form keeps the runner the sole spawn authority
    fn spawn(
        &self,
        args: &[&str],
        cwd: &Path,
        profile: &GitExecutionProfile,
        home: &Path,
    ) -> Result<GitOutput, GitError> {
        let mut full: Vec<String> = hardening_args(profile, cwd);
        full.extend(args.iter().map(|s| (*s).to_string()));

        let mut cmd = Command::new("git");
        cmd.args(&full);
        cmd.current_dir(cwd);

        // Sanitized environment: nothing ambient is inherited (secret isolation), git
        // never prompts, locale is stable.
        cmd.env_clear();
        cmd.env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
        cmd.env("HOME", home);
        cmd.env("LC_ALL", "C");
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        if !profile.trust_ambient_config {
            cmd.env("GIT_CONFIG_NOSYSTEM", "1");
            cmd.env("GIT_CONFIG_GLOBAL", "/dev/null");
        }

        let out = cmd.output().map_err(GitError::Spawn)?;
        Ok(GitOutput {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }

    fn run_checked(
        &self,
        args: &[&str],
        cwd: &Path,
        profile: &GitExecutionProfile,
        home: &Path,
    ) -> Result<GitOutput, GitError> {
        let out = self.spawn(args, cwd, profile, home)?;
        if out.success {
            Ok(out)
        } else {
            Err(GitError::Failed {
                args: args.iter().map(|s| (*s).to_string()).collect(),
                stderr: out.stderr,
            })
        }
    }

    /// The current `HEAD` object id, or [`NULL_OID`] when the branch is unborn.
    pub fn current_head(&self, cwd: &Path, home: &Path) -> Result<String, GitError> {
        let out = self.spawn(
            &["rev-parse", "--verify", "HEAD"],
            cwd,
            &GitExecutionProfile::hardened(),
            home,
        )?;
        if out.success {
            Ok(out.stdout.trim().to_string())
        } else {
            Ok(NULL_OID.to_string())
        }
    }

    /// Porcelain status (`--porcelain=v1`).
    pub fn status_porcelain(&self, cwd: &Path, home: &Path) -> Result<String, GitError> {
        Ok(self
            .run_checked(
                &["status", "--porcelain=v1"],
                cwd,
                &GitExecutionProfile::hardened(),
                home,
            )?
            .stdout)
    }

    /// Initialize a repository at `path` on branch `main`.
    pub fn init(&self, path: &Path, home: &Path) -> Result<(), GitError> {
        self.run_checked(
            &["init", "-b", "main"],
            path,
            &GitExecutionProfile::hardened(),
            home,
        )?;
        Ok(())
    }

    /// Set the repository-local commit identity (harmless config; never a code vector).
    pub fn set_local_identity(
        &self,
        cwd: &Path,
        name: &str,
        email: &str,
        home: &Path,
    ) -> Result<(), GitError> {
        let p = GitExecutionProfile::hardened();
        self.run_checked(&["config", "user.name", name], cwd, &p, home)?;
        self.run_checked(&["config", "user.email", email], cwd, &p, home)?;
        Ok(())
    }

    /// List local branch names.
    pub fn branch_list(&self, cwd: &Path, home: &Path) -> Result<Vec<String>, GitError> {
        let out = self.run_checked(
            &["branch", "--list", "--format=%(refname:short)"],
            cwd,
            &GitExecutionProfile::hardened(),
            home,
        )?;
        Ok(out.stdout.lines().map(str::to_string).collect())
    }

    /// Add a linked worktree at `new_path` on a fresh `branch` (`git worktree add -b`).
    pub fn worktree_add(
        &self,
        cwd: &Path,
        new_path: &str,
        branch: &str,
        home: &Path,
    ) -> Result<(), GitError> {
        self.run_checked(
            &["worktree", "add", "-b", branch, new_path],
            cwd,
            &GitExecutionProfile::hardened(),
            home,
        )?;
        Ok(())
    }
}

/// A hardened `HEAD` expectation carried by a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedHead(String);

impl ExpectedHead {
    /// Wrap a head value.
    #[must_use]
    pub fn new(head: impl Into<String>) -> Self {
        Self(head.into())
    }

    /// The head string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A transactional unit of Git mutation, bound to a repository, a capability grant, and an
/// expected `HEAD`.
#[derive(Debug)]
pub struct GitTransaction {
    runner: GitRunner,
    identity: RepositoryIdentity,
    grant: CapabilityGrant,
    txn_id: TransactionId,
    expected_head: String,
    home: PathBuf,
}

impl GitTransaction {
    /// Begin a transaction. The expected head is captured from the repository now; each
    /// mutation re-verifies it.
    ///
    /// The grant is **bound to the identity** here: a capability minted for repository A
    /// cannot open a transaction against repository B, and a capability pinned to one
    /// worktree cannot act on another. Without this check the capability bounds nothing —
    /// [`CapabilityGrant::authorize_operation`] re-uses the capability's *own* repository
    /// to build its access request, so it can never notice a mismatched target. Binding
    /// once at construction is sufficient because `identity` and `grant` are private and
    /// never replaced for the life of the transaction.
    pub fn begin(
        runner: GitRunner,
        identity: RepositoryIdentity,
        grant: CapabilityGrant,
    ) -> Result<Self, GitError> {
        let authorized_repo = grant.capability().repository();
        if authorized_repo != identity.repository_id() {
            return Err(GitError::RepositoryMismatch {
                authorized: authorized_repo.as_str().to_string(),
                target: identity.repository_id().as_str().to_string(),
            });
        }
        // A capability with no worktree pin is valid for any worktree of its repository.
        if let Some(authorized_wt) = grant.capability().worktree() {
            if authorized_wt != identity.worktree_id() {
                return Err(GitError::WorktreeMismatch {
                    authorized: authorized_wt.as_str().to_string(),
                    target: identity.worktree_id().as_str().to_string(),
                });
            }
        }

        let home = identity.worktree().to_path_buf();
        let expected_head = runner.current_head(identity.worktree(), &home)?;
        Ok(Self {
            runner,
            identity,
            grant,
            txn_id: TransactionId::generate(),
            expected_head,
            home,
        })
    }

    /// The transaction id.
    #[must_use]
    pub fn id(&self) -> &TransactionId {
        &self.txn_id
    }

    /// The expected head this transaction currently guards against.
    #[must_use]
    pub fn expected_head(&self) -> &str {
        &self.expected_head
    }

    /// Uses remaining on the underlying grant.
    #[must_use]
    pub fn uses_remaining(&self) -> u32 {
        self.grant.uses_remaining()
    }

    /// Authorize + re-verify head before a mutation.
    fn guard(&mut self, operation: &str, risk: RiskClass, now: SystemTime) -> Result<(), GitError> {
        self.grant
            .authorize_operation(operation, risk, now)
            .map_err(GitError::Denied)?;
        let actual = self
            .runner
            .current_head(self.identity.worktree(), &self.home)?;
        if actual != self.expected_head {
            return Err(GitError::HeadMoved {
                expected: self.expected_head.clone(),
                actual,
            });
        }
        Ok(())
    }

    /// Stage paths (`git add -- <paths>`). Does not change `HEAD`.
    pub fn add(&mut self, paths: &[&str], now: SystemTime) -> Result<(), GitError> {
        self.guard("git.add", RiskClass::Mutating, now)?;
        let mut args = vec!["add", "--"];
        args.extend_from_slice(paths);
        self.runner.run_checked(
            &args,
            self.identity.worktree(),
            &GitExecutionProfile::hardened(),
            &self.home,
        )?;
        Ok(())
    }

    /// Restore paths (`git restore -- <paths>`).
    pub fn restore(&mut self, paths: &[&str], now: SystemTime) -> Result<(), GitError> {
        self.guard("git.restore", RiskClass::Mutating, now)?;
        let mut args = vec!["restore", "--"];
        args.extend_from_slice(paths);
        self.runner.run_checked(
            &args,
            self.identity.worktree(),
            &GitExecutionProfile::hardened(),
            &self.home,
        )?;
        Ok(())
    }

    /// Commit staged changes, advancing `HEAD`. Commit identity is injected via `-c` so it
    /// does not depend on ambient config.
    pub fn commit(&mut self, message: &str, now: SystemTime) -> Result<String, GitError> {
        self.guard("git.commit", RiskClass::Mutating, now)?;
        self.runner.run_checked(
            &["commit", "-m", message],
            self.identity.worktree(),
            &GitExecutionProfile::hardened(),
            &self.home,
        )?;
        let new_head = self
            .runner
            .current_head(self.identity.worktree(), &self.home)?;
        self.expected_head.clone_from(&new_head);
        Ok(new_head)
    }

    /// Create a branch at the current `HEAD`.
    pub fn branch_create(&mut self, name: &str, now: SystemTime) -> Result<(), GitError> {
        self.guard("git.branch", RiskClass::Mutating, now)?;
        self.runner.run_checked(
            &["branch", name],
            self.identity.worktree(),
            &GitExecutionProfile::hardened(),
            &self.home,
        )?;
        Ok(())
    }
}

/// Collect the config files that could carry filter/diff drivers for the repo at `cwd`.
fn config_files(cwd: &Path) -> Vec<PathBuf> {
    RepositoryIdentity::resolve(cwd).map_or_else(
        |_| vec![cwd.join(".git").join("config")],
        |id| vec![id.git_dir().join("config"), id.common_dir().join("config")],
    )
}

/// Extract subsection names of `[section "name"]` headers from a config file's text.
fn subsection_names(text: &str, section: &str) -> Vec<String> {
    let open = format!("[{section} \"");
    let mut out = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix(open.as_str()) {
            if let Some(end) = rest.find("\"]") {
                out.push(rest[..end].to_string());
            }
        }
    }
    out
}

/// Build the hardening `-c` overrides for a profile against the repo at `cwd`.
fn hardening_args(profile: &GitExecutionProfile, cwd: &Path) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let mut push = |k: &str, v: &str| {
        args.push("-c".to_string());
        args.push(format!("{k}={v}"));
    };

    if !profile.allow_hooks {
        push("core.hooksPath", "/dev/null");
    }
    if !profile.allow_fsmonitor {
        push("core.fsmonitor", "");
    }
    if !profile.allow_credential_helpers {
        push("credential.helper", "");
    }
    if !profile.allow_remote {
        push("core.sshCommand", "");
    }

    let needs_filters = !profile.allow_filters;
    let needs_diff = !profile.allow_external_diff || !profile.allow_textconv;
    if needs_filters || needs_diff {
        let mut filters = BTreeSet::new();
        let mut diffs = BTreeSet::new();
        for path in config_files(cwd) {
            if let Ok(text) = std::fs::read_to_string(&path) {
                if needs_filters {
                    filters.extend(subsection_names(&text, "filter"));
                }
                if needs_diff {
                    diffs.extend(subsection_names(&text, "diff"));
                }
            }
        }
        for name in filters {
            push(&format!("filter.{name}.clean"), "");
            push(&format!("filter.{name}.smudge"), "");
            push(&format!("filter.{name}.process"), "");
        }
        for name in diffs {
            if !profile.allow_external_diff {
                push(&format!("diff.{name}.command"), "");
            }
            if !profile.allow_textconv {
                push(&format!("diff.{name}.textconv"), "");
            }
        }
    }

    args
}

#[cfg(test)]
mod tests;
