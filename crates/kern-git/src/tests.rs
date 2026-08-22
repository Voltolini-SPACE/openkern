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

#[test]
fn linked_worktrees_are_isolated() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    let id = init_repo(&runner, &repo);

    // Need an initial commit before a linked worktree can be added.
    let grant = grant_for(&id, ALL_OPS);
    let mut main_txn = GitTransaction::begin(runner.clone(), id, grant).unwrap();
    fs::write(repo.join("base.txt"), "base\n").unwrap();
    main_txn.add(&["base.txt"], now()).unwrap();
    let main_head_before = main_txn.commit("base", now()).unwrap();

    // Add a linked worktree on its own branch.
    let wt_path = t.path().join("wt-feature");
    runner
        .worktree_add(&repo, wt_path.to_str().unwrap(), "feature", &repo)
        .unwrap();

    let main_id = ident(&repo);
    let wt_id = ident(&wt_path);
    assert!(
        main_id.same_repository(&wt_id),
        "linked worktree shares the repository"
    );
    assert_ne!(
        main_id.worktree_id(),
        wt_id.worktree_id(),
        "distinct worktrees"
    );

    // Mutate inside the linked worktree via its own transaction.
    let wt_grant = grant_for(&wt_id, ALL_OPS);
    let wt_ident = ident(&wt_path);
    let mut wt_txn = GitTransaction::begin(runner.clone(), wt_ident, wt_grant).unwrap();
    fs::write(wt_path.join("feat.txt"), "feature\n").unwrap();
    wt_txn.add(&["feat.txt"], now()).unwrap();
    let feature_head = wt_txn.commit("feature commit", now()).unwrap();
    assert_ne!(feature_head, main_head_before);

    // The main worktree's HEAD (branch `main`) is unaffected by the feature commit.
    let main_head_after = runner.current_head(&repo, &repo).unwrap();
    assert_eq!(
        main_head_after, main_head_before,
        "main worktree HEAD isolated from linked-worktree commit"
    );
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
// ---------------------------------------------------------------------------
// CRITICAL #1 — repository/capability binding
// ---------------------------------------------------------------------------

/// Bind a grant to a specific repository id *and* worktree id.
fn grant_bound(
    repository: &RepositoryId,
    worktree: Option<&kern_types::WorktreeId>,
    fs_root: &Path,
    ops: &[&str],
) -> CapabilityGrant {
    let spec = CapabilitySpec {
        agent: AgentId::new("agent-git").unwrap(),
        mission: MissionId::new("mission-git").unwrap(),
        repository: RepositoryId::new(repository.as_str()).unwrap(),
        worktree: worktree.map(|w| kern_types::WorktreeId::new(w.as_str()).unwrap()),
        operations: ops.iter().map(|s| (*s).to_string()).collect(),
        fs: FsScope::root(fs_root),
        exec: ExecScope::new(["git"]),
        network: NetworkMode::DenyAll,
        deadline: Deadline::after(now(), Duration::from_secs(100_000)),
        risk_ceiling: RiskClass::Mutating,
    };
    Capability::new(spec).grant(50)
}

// Reproduction record: before the binding check in `GitTransaction::begin`, a probe that
// began a transaction with `grant(A)` and `identity(B)` succeeded and landed a real commit
// in repository B. That probe passed against the unfixed code and fails against the fixed
// code; `capability_of_a_is_denied_against_b` below is its permanent, inverted form.

#[test]
fn same_repository_binding_is_allowed_a_a() {
    let t = TempDir::new();
    let repo_a = t.path().join("repo-a");
    let runner = GitRunner::new();
    let id_a = init_repo(&runner, &repo_a);
    let grant_a = grant_for(&id_a, ALL_OPS);

    let mut txn = GitTransaction::begin(runner.clone(), ident(&repo_a), grant_a)
        .expect("A/A must remain allowed");
    fs::write(repo_a.join("a.txt"), "a\n").unwrap();
    txn.add(&["a.txt"], now()).unwrap();
    assert_ne!(txn.commit("legit A", now()).unwrap(), NULL_OID);
}

#[test]
fn same_repository_binding_is_allowed_b_b() {
    let t = TempDir::new();
    let repo_b = t.path().join("repo-b");
    let runner = GitRunner::new();
    let id_b = init_repo(&runner, &repo_b);
    let grant_b = grant_for(&id_b, ALL_OPS);

    let mut txn = GitTransaction::begin(runner.clone(), ident(&repo_b), grant_b)
        .expect("B/B must remain allowed");
    fs::write(repo_b.join("b.txt"), "b\n").unwrap();
    txn.add(&["b.txt"], now()).unwrap();
    assert_ne!(txn.commit("legit B", now()).unwrap(), NULL_OID);
}

#[test]
fn capability_of_a_is_denied_against_b() {
    let t = TempDir::new();
    let repo_a = t.path().join("repo-a");
    let repo_b = t.path().join("repo-b");
    let runner = GitRunner::new();
    let id_a = init_repo(&runner, &repo_a);
    let id_b = init_repo(&runner, &repo_b);

    let grant_a = grant_for(&id_a, ALL_OPS);
    let err = GitTransaction::begin(runner.clone(), id_b, grant_a).expect_err("A/B must be denied");
    assert!(
        matches!(err, GitError::RepositoryMismatch { .. }),
        "expected RepositoryMismatch, got {err:?}"
    );

    // And nothing was written to B.
    assert!(!repo_b.join("pwned.txt").exists());
}

#[test]
fn capability_of_b_is_denied_against_a() {
    let t = TempDir::new();
    let repo_a = t.path().join("repo-a");
    let repo_b = t.path().join("repo-b");
    let runner = GitRunner::new();
    let id_a = init_repo(&runner, &repo_a);
    let id_b = init_repo(&runner, &repo_b);

    let grant_b = grant_for(&id_b, ALL_OPS);
    let err = GitTransaction::begin(runner.clone(), id_a, grant_b).expect_err("B/A must be denied");
    assert!(matches!(err, GitError::RepositoryMismatch { .. }));
}

#[test]
fn capability_bound_to_another_worktree_is_denied() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    let id = init_repo(&runner, &repo);

    // An initial commit is required before a linked worktree can be added.
    let grant = grant_for(&id, ALL_OPS);
    let mut txn = GitTransaction::begin(runner.clone(), ident(&repo), grant).unwrap();
    fs::write(repo.join("base.txt"), "base\n").unwrap();
    txn.add(&["base.txt"], now()).unwrap();
    txn.commit("base", now()).unwrap();

    let wt_path = t.path().join("wt-feature");
    runner
        .worktree_add(&repo, wt_path.to_str().unwrap(), "feature", &repo)
        .unwrap();

    let main_id = ident(&repo);
    let wt_id = ident(&wt_path);
    assert!(main_id.same_repository(&wt_id), "same repository");
    assert_ne!(
        main_id.worktree_id(),
        wt_id.worktree_id(),
        "distinct worktrees"
    );

    // A capability pinned to the MAIN worktree must not act on the LINKED worktree,
    // even though the repository id matches.
    let grant_main = grant_bound(
        main_id.repository_id(),
        Some(main_id.worktree_id()),
        &repo,
        ALL_OPS,
    );
    let err = GitTransaction::begin(runner.clone(), ident(&wt_path), grant_main)
        .expect_err("worktree-pinned capability must not cross worktrees");
    assert!(
        matches!(err, GitError::WorktreeMismatch { .. }),
        "expected WorktreeMismatch, got {err:?}"
    );

    // The matching pair still works.
    let grant_wt = grant_bound(
        wt_id.repository_id(),
        Some(wt_id.worktree_id()),
        &wt_path,
        ALL_OPS,
    );
    GitTransaction::begin(runner.clone(), ident(&wt_path), grant_wt)
        .expect("worktree-matched capability must be allowed");
}

#[test]
fn unpinned_worktree_capability_still_works_within_its_repository() {
    // `worktree: None` means "any worktree of this repository" — repository binding
    // must still be enforced, but worktree binding is not claimed.
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    let id = init_repo(&runner, &repo);
    let grant = grant_bound(id.repository_id(), None, &repo, ALL_OPS);
    GitTransaction::begin(runner.clone(), ident(&repo), grant)
        .expect("unpinned worktree capability is valid within its own repository");
}

#[test]
fn exhausted_grant_cannot_mutate() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    let id = init_repo(&runner, &repo);
    let grant = Capability::new(CapabilitySpec {
        agent: AgentId::new("agent-git").unwrap(),
        mission: MissionId::new("mission-git").unwrap(),
        repository: RepositoryId::new(id.repository_id().as_str()).unwrap(),
        worktree: None,
        operations: ALL_OPS.iter().map(|s| (*s).to_string()).collect(),
        fs: FsScope::root(&repo),
        exec: ExecScope::new(["git"]),
        network: NetworkMode::DenyAll,
        deadline: Deadline::after(now(), Duration::from_secs(100_000)),
        risk_ceiling: RiskClass::Mutating,
    })
    .grant(0);

    let mut txn = GitTransaction::begin(runner.clone(), ident(&repo), grant).unwrap();
    fs::write(repo.join("x.txt"), "x\n").unwrap();
    let err = txn.add(&["x.txt"], now()).expect_err("no uses left");
    assert!(matches!(err, GitError::Denied(_)), "got {err:?}");
}

#[test]
fn expired_grant_cannot_mutate() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    let id = init_repo(&runner, &repo);
    let past = now().checked_sub(Duration::from_secs(10)).unwrap();
    let grant = Capability::new(CapabilitySpec {
        agent: AgentId::new("agent-git").unwrap(),
        mission: MissionId::new("mission-git").unwrap(),
        repository: RepositoryId::new(id.repository_id().as_str()).unwrap(),
        worktree: None,
        operations: ALL_OPS.iter().map(|s| (*s).to_string()).collect(),
        fs: FsScope::root(&repo),
        exec: ExecScope::new(["git"]),
        network: NetworkMode::DenyAll,
        deadline: Deadline::after(past, Duration::from_secs(1)),
        risk_ceiling: RiskClass::Mutating,
    })
    .grant(50);

    let mut txn = GitTransaction::begin(runner.clone(), ident(&repo), grant).unwrap();
    fs::write(repo.join("x.txt"), "x\n").unwrap();
    let err = txn.add(&["x.txt"], now()).expect_err("deadline passed");
    assert!(matches!(err, GitError::Denied(_)), "got {err:?}");
}

// ---------------------------------------------------------------------------
// CRITICAL #2 — hardened profile escape via config the textual scan cannot see
// ---------------------------------------------------------------------------

/// Write a real clean-filter driver and return (`script_path`, `marker_path`).
fn evil_filter_script(t: &TempDir, repo: &Path) -> (PathBuf, PathBuf) {
    let marker = t.path().join("filter_ran.marker");
    let script = repo.join("evil-clean.sh");
    fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\ncat\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    (script, marker)
}

#[test]
fn include_cannot_smuggle_a_filter_past_the_hardened_profile() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    let (script, marker) = evil_filter_script(&t, &repo);

    // The filter lives in a SEPARATE file, pulled in by [include]. A textual scan of
    // .git/config alone never sees `[filter "evil"]`, but git resolves the include.
    let included = repo.join(".git").join("smuggled.config");
    fs::write(
        &included,
        format!(
            "[filter \"evil\"]\n\tclean = {}\n\trequired = false\n",
            script.display()
        ),
    )
    .unwrap();
    let cfg = repo.join(".git/config");
    let mut cfg_text = fs::read_to_string(&cfg).unwrap();
    let _ = write!(cfg_text, "[include]\n\tpath = {}\n", included.display());
    fs::write(&cfg, cfg_text).unwrap();
    fs::write(repo.join(".gitattributes"), "*.dat filter=evil\n").unwrap();

    // POSITIVE CONTROL: unhardened `add` runs the smuggled filter — the include works.
    fs::write(repo.join("x.dat"), "data-x\n").unwrap();
    let up = GitExecutionProfile::unhardened();
    let a = runner
        .spawn(&["add", "--", "x.dat"], &repo, &up, &repo)
        .unwrap();
    assert!(a.success, "control add failed: {}", a.stderr);
    assert!(
        marker.exists(),
        "positive control: the included filter must actually run"
    );
    fs::remove_file(&marker).unwrap();

    // GOVERNED: the hardened profile must not let the smuggled filter run.
    fs::write(repo.join("y.dat"), "data-y\n").unwrap();
    let hp = GitExecutionProfile::hardened();
    let governed = runner.spawn(&["add", "--", "y.dat"], &repo, &hp, &repo);
    assert!(
        !marker.exists(),
        "ESCAPE: a filter smuggled via [include] ran under the hardened profile"
    );
    // Fail-closed: config that cannot be exhaustively enumerated must be refused, not
    // silently run with incomplete neutralization.
    assert!(
        governed.is_err(),
        "hardened profile must refuse config it cannot fully enumerate"
    );
}

#[test]
fn case_variant_section_header_cannot_smuggle_a_filter() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    let (script, marker) = evil_filter_script(&t, &repo);

    // git treats section NAMES case-insensitively; a scan for the literal `[filter "`
    // misses `[FiLtEr "evil"]` while git still honours it.
    let cfg = repo.join(".git/config");
    let mut cfg_text = fs::read_to_string(&cfg).unwrap();
    let _ = write!(
        cfg_text,
        "[FiLtEr \"evil\"]\n\tclean = {}\n\trequired = false\n",
        script.display()
    );
    fs::write(&cfg, cfg_text).unwrap();
    fs::write(repo.join(".gitattributes"), "*.dat filter=evil\n").unwrap();

    fs::write(repo.join("x.dat"), "data-x\n").unwrap();
    let up = GitExecutionProfile::unhardened();
    let a = runner
        .spawn(&["add", "--", "x.dat"], &repo, &up, &repo)
        .unwrap();
    assert!(a.success, "control add failed: {}", a.stderr);
    assert!(
        marker.exists(),
        "positive control: case-variant section is honoured by git"
    );
    fs::remove_file(&marker).unwrap();

    fs::write(repo.join("y.dat"), "data-y\n").unwrap();
    let hp = GitExecutionProfile::hardened();
    let a2 = runner
        .spawn(&["add", "--", "y.dat"], &repo, &hp, &repo)
        .unwrap();
    assert!(a2.success, "governed add failed: {}", a2.stderr);
    assert!(
        !marker.exists(),
        "ESCAPE: case-variant [FiLtEr] section was not neutralized"
    );
}

// --- CRITICAL #2 adversarial battery: every include shape must fail closed ---------

/// Append raw text to the repository's `.git/config`.
fn append_cfg(repo: &Path, text: &str) {
    let cfg = repo.join(".git/config");
    let mut s = fs::read_to_string(&cfg).unwrap();
    s.push_str(text);
    fs::write(&cfg, s).unwrap();
}

/// A hardened `add` against `repo`, returning the result for inspection.
fn hardened_add(runner: &GitRunner, repo: &Path, file: &str) -> Result<GitOutput, GitError> {
    fs::write(repo.join(file), "payload\n").unwrap();
    runner.spawn(
        &["add", "--", file],
        repo,
        &GitExecutionProfile::hardened(),
        repo,
    )
}

#[test]
fn every_include_shape_fails_closed() {
    let cases: &[(&str, &str)] = &[
        ("include relativo", "[include]\n\tpath = ./rel.config\n"),
        ("include absoluto", "[include]\n\tpath = /etc/gitconfig\n"),
        (
            "include inexistente",
            "[include]\n\tpath = ./does-not-exist\n",
        ),
        (
            "include fora do repo",
            "[include]\n\tpath = /tmp/outside.config\n",
        ),
        ("include em uma linha", "[include] path = ./rel.config\n"),
        (
            "includeIf gitdir",
            "[includeIf \"gitdir:/\"]\n\tpath = ./rel.config\n",
        ),
        (
            "includeIf onbranch",
            "[includeIf \"onbranch:main\"]\n\tpath = ./rel.config\n",
        ),
        ("include caixa alta", "[INCLUDE]\n\tpath = ./rel.config\n"),
        ("include caixa mista", "[InClUdE]\n\tpath = ./rel.config\n"),
        ("include recuado", "   [include]\n\tpath = ./rel.config\n"),
    ];

    for (nome, cfg) in cases {
        let t = TempDir::new();
        let repo = t.path().join("repo");
        let runner = GitRunner::new();
        init_repo(&runner, &repo);
        append_cfg(&repo, cfg);

        let r = hardened_add(&runner, &repo, "f.dat");
        assert!(
            matches!(r, Err(GitError::UnenumerableConfig { .. })),
            "{nome}: hardened profile must fail closed, got {r:?}"
        );
    }
}

#[test]
fn chained_and_circular_includes_fail_closed() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);

    // a -> b -> a (circular), reached from .git/config.
    let a = repo.join(".git").join("a.config");
    let b = repo.join(".git").join("b.config");
    fs::write(&a, format!("[include]\n\tpath = {}\n", b.display())).unwrap();
    fs::write(&b, format!("[include]\n\tpath = {}\n", a.display())).unwrap();
    append_cfg(&repo, &format!("[include]\n\tpath = {}\n", a.display()));

    let r = hardened_add(&runner, &repo, "f.dat");
    assert!(
        matches!(r, Err(GitError::UnenumerableConfig { .. })),
        "chained/circular include must fail closed, got {r:?}"
    );
}

#[test]
fn include_combined_with_an_authorized_filter_still_fails_closed() {
    // An include next to a legitimately-enumerable filter must not be waved through
    // just because the visible part looks containable.
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    let (script, marker) = evil_filter_script(&t, &repo);

    let hidden = repo.join(".git").join("hidden.config");
    fs::write(
        &hidden,
        format!(
            "[filter \"sneaky\"]\n\tclean = {}\n\trequired = false\n",
            script.display()
        ),
    )
    .unwrap();
    append_cfg(
        &repo,
        &format!(
            "[filter \"visible\"]\n\tclean = cat\n[include]\n\tpath = {}\n",
            hidden.display()
        ),
    );
    fs::write(repo.join(".gitattributes"), "*.dat filter=sneaky\n").unwrap();

    let r = hardened_add(&runner, &repo, "f.dat");
    assert!(
        matches!(r, Err(GitError::UnenumerableConfig { .. })),
        "include beside a visible filter must still fail closed, got {r:?}"
    );
    assert!(!marker.exists(), "the smuggled filter must never have run");
}

#[test]
fn malformed_config_does_not_silently_disable_containment() {
    // Garbage that git itself rejects must not become a way to skip neutralization:
    // the operation must not succeed while a filter is live.
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    let (script, marker) = evil_filter_script(&t, &repo);

    append_cfg(
        &repo,
        &format!(
            "[filter \"evil\"]\n\tclean = {}\n\trequired = false\n[unclosed section\n",
            script.display()
        ),
    );
    fs::write(repo.join(".gitattributes"), "*.dat filter=evil\n").unwrap();

    let r = hardened_add(&runner, &repo, "f.dat");
    // Either git refuses the malformed config, or we ran with the filter neutralized.
    // What must never happen is a successful add with the filter having executed.
    assert!(
        !marker.exists(),
        "malformed config must not become an escape: the filter ran"
    );
    if let Ok(out) = &r {
        assert!(
            !out.success || !marker.exists(),
            "add succeeded with a live filter"
        );
    }
}

#[test]
fn unhardened_profile_is_unaffected_by_the_include_refusal() {
    // The refusal belongs to containment. A profile that allows filters has nothing to
    // enumerate, so includes must not break it.
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    append_cfg(&repo, "[include]\n\tpath = ./whatever.config\n");

    fs::write(repo.join("f.dat"), "payload\n").unwrap();
    let out = runner
        .spawn(
            &["add", "--", "f.dat"],
            &repo,
            &GitExecutionProfile::unhardened(),
            &repo,
        )
        .expect("unhardened profile must not be refused");
    assert!(out.success, "unhardened add failed: {}", out.stderr);
}

#[test]
fn clean_repository_is_not_refused() {
    // Anti-regression for the refusal itself: no include, no refusal.
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    let out = hardened_add(&runner, &repo, "f.dat").expect("clean repo must not be refused");
    assert!(
        out.success,
        "hardened add on a clean repo failed: {}",
        out.stderr
    );
}

#[test]
fn include_refusal_blast_radius_is_the_whole_hardened_surface() {
    // Documents a real consequence of failing closed: every public GitRunner helper uses
    // the hardened profile, so a repository whose config carries an [include] becomes
    // unusable through kern-git — including read-only operations. This is deliberate
    // (refuse rather than run with a blind spot) but it is a wholesale refusal, not a
    // mutation-only one, and callers must know that.
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    append_cfg(&repo, "[include]\n\tpath = ./whatever.config\n");

    for (name, r) in [
        (
            "current_head",
            runner.current_head(&repo, &repo).map(|_| ()),
        ),
        (
            "status_porcelain",
            runner.status_porcelain(&repo, &repo).map(|_| ()),
        ),
        ("branch_list", runner.branch_list(&repo, &repo).map(|_| ())),
    ] {
        assert!(
            matches!(r, Err(GitError::UnenumerableConfig { .. })),
            "{name} must fail closed on unenumerable config, got {r:?}"
        );
    }

    // And a transaction cannot even be opened, because begin() reads HEAD.
    let id = ident(&repo);
    let grant = grant_for(&id, ALL_OPS);
    let r = GitTransaction::begin(runner.clone(), ident(&repo), grant);
    assert!(
        matches!(r, Err(GitError::UnenumerableConfig { .. })),
        "got {r:?}"
    );
}

// --- §6 casos remanescentes -------------------------------------------------------

#[test]
fn multiple_includes_fail_closed() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    let (script, marker) = evil_filter_script(&t, &repo);

    let one = repo.join(".git").join("one.config");
    let two = repo.join(".git").join("two.config");
    fs::write(&one, "[core]\n\tquotePath = false\n").unwrap();
    fs::write(
        &two,
        format!(
            "[filter \"evil\"]\n\tclean = {}\n\trequired = false\n",
            script.display()
        ),
    )
    .unwrap();
    append_cfg(
        &repo,
        &format!(
            "[include]\n\tpath = {}\n[include]\n\tpath = {}\n",
            one.display(),
            two.display()
        ),
    );
    fs::write(repo.join(".gitattributes"), "*.dat filter=evil\n").unwrap();

    let r = hardened_add(&runner, &repo, "f.dat");
    assert!(
        matches!(r, Err(GitError::UnenumerableConfig { .. })),
        "multiple includes must fail closed, got {r:?}"
    );
    assert!(!marker.exists(), "the smuggled filter must never have run");
}

#[test]
fn include_plus_case_variant_filter_fails_closed() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    let (script, marker) = evil_filter_script(&t, &repo);

    // Case-variant section inside an included file: both bypasses combined.
    let inc = repo.join(".git").join("inc.config");
    fs::write(
        &inc,
        format!(
            "[FiLtEr \"evil\"]\n\tclean = {}\n\trequired = false\n",
            script.display()
        ),
    )
    .unwrap();
    append_cfg(&repo, &format!("[include]\n\tpath = {}\n", inc.display()));
    fs::write(repo.join(".gitattributes"), "*.dat filter=evil\n").unwrap();

    let r = hardened_add(&runner, &repo, "f.dat");
    assert!(
        matches!(r, Err(GitError::UnenumerableConfig { .. })),
        "include + case variant must fail closed, got {r:?}"
    );
    assert!(!marker.exists());
}

// --- §7 PROVA NEGATIVA ------------------------------------------------------------

/// Every known way to get a filter driver into the effective config, against every public
/// entry point of the crate. The marker must never appear: no path may reach arbitrary
/// execution, and no path may quietly succeed with containment skipped.
///
/// The universal claim in the name is quantified over the mechanisms enumerated below,
/// which is every mechanism by which repository-controlled input can define a filter for a
/// governed invocation:
///
/// - a driver written straight into `.git/config` (covered here and by
///   `git_clean_filter_is_contained_with_positive_control`);
/// - a case-variant section header, which git matches but a literal scan does not;
/// - `[include]` / `[includeIf]`, directly, chained, and beside a visible definition;
/// - `extensions.worktreeConfig`, which relocates part of the config into
///   `.git/config.worktree`;
/// - a config file that cannot be read (`unreadable_config_fails_closed_…`).
///
/// Ambient sources are neutralized outside this test and are therefore not variants here:
/// system and global config via `GIT_CONFIG_NOSYSTEM` and `GIT_CONFIG_GLOBAL=/dev/null`,
/// `GIT_CONFIG_COUNT`/`KEY`/`VALUE` via `env_clear`, and `-c` because the argument vector
/// is ours. Adding a mechanism to git means adding a variant here before the name holds
/// again.
#[test]
fn no_public_entry_point_can_execute_a_smuggled_filter() {
    for (nome, variante) in [
        ("include", 0u8),
        ("include em cadeia", 1),
        ("multiplas definicoes", 2),
        ("caixa variante", 3),
        ("malformada + include", 4),
        ("extensions.worktreeConfig", 5),
    ] {
        let t = TempDir::new();
        let repo = t.path().join("repo");
        let runner = GitRunner::new();
        init_repo(&runner, &repo);
        let (script, marker) = evil_filter_script(&t, &repo);
        let filtro = format!(
            "[filter \"evil\"]\n\tclean = {}\n\tsmudge = {}\n\trequired = false\n",
            script.display(),
            script.display()
        );
        let gd = repo.join(".git");
        match variante {
            0 => {
                fs::write(gd.join("i.config"), &filtro).unwrap();
                append_cfg(
                    &repo,
                    &format!("[include]\n\tpath = {}\n", gd.join("i.config").display()),
                );
            }
            1 => {
                fs::write(gd.join("b.config"), &filtro).unwrap();
                fs::write(
                    gd.join("a.config"),
                    format!("[include]\n\tpath = {}\n", gd.join("b.config").display()),
                )
                .unwrap();
                append_cfg(
                    &repo,
                    &format!("[include]\n\tpath = {}\n", gd.join("a.config").display()),
                );
            }
            2 => {
                fs::write(gd.join("i.config"), &filtro).unwrap();
                append_cfg(&repo, &filtro);
                append_cfg(
                    &repo,
                    &format!("[include]\n\tpath = {}\n", gd.join("i.config").display()),
                );
            }
            3 => {
                append_cfg(
                    &repo,
                    &format!(
                        "[FiLtEr \"evil\"]\n\tclean = {}\n\trequired = false\n",
                        script.display()
                    ),
                );
            }
            4 => {
                fs::write(gd.join("i.config"), &filtro).unwrap();
                append_cfg(
                    &repo,
                    &format!(
                        "[include]\n\tpath = {}\n[unclosed\n",
                        gd.join("i.config").display()
                    ),
                );
            }
            _ => {
                // The driver lives in config.worktree; `.git/config` carries neither a
                // filter nor an include — only the extension that relocates the config.
                fs::write(gd.join("config.worktree"), &filtro).unwrap();
                append_cfg(&repo, "[extensions]\n\tworktreeConfig = true\n");
            }
        }
        fs::write(repo.join(".gitattributes"), "*.dat filter=evil\n").unwrap();
        fs::write(repo.join("hostile.dat"), "payload\n").unwrap();

        // Every public entry point.
        let _ = runner.current_head(&repo, &repo);
        let _ = runner.status_porcelain(&repo, &repo);
        let _ = runner.branch_list(&repo, &repo);
        let _ = runner.set_local_identity(&repo, "N", "e@x", &repo);
        let _ = runner.init(&repo, &repo);
        let _ = runner.worktree_add(&repo, t.path().join("wt-x").to_str().unwrap(), "bx", &repo);
        // And the transactional surface, if it can even be opened.
        let id = RepositoryIdentity::resolve(&repo);
        if let Ok(id) = id {
            let grant = grant_for(&id, ALL_OPS);
            if let Ok(mut txn) = GitTransaction::begin(runner.clone(), ident(&repo), grant) {
                let _ = txn.add(&["hostile.dat"], now());
                let _ = txn.commit("hostile", now());
                let _ = txn.branch_create("hb", now());
                let _ = txn.restore(&["hostile.dat"], now());
            }
        }

        assert!(
            !marker.exists(),
            "{nome}: arbitrary execution reached through a public entry point"
        );
    }
}
#[test]
fn unreadable_config_fails_closed_instead_of_skipping_enumeration() {
    // A config file the scanner cannot read must not be silently skipped: skipping is a
    // permissive fallback — enumeration would come back empty and nothing would be
    // neutralized. Unreadable config is indeterminable config, so it must be refused.
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);

    let cfg = repo.join(".git/config");
    fs::set_permissions(&cfg, fs::Permissions::from_mode(0o000)).unwrap();

    let r = hardened_add(&runner, &repo, "f.dat");
    // Restore permissions before any assertion can unwind and leak an unreadable file.
    fs::set_permissions(&cfg, fs::Permissions::from_mode(0o644)).unwrap();

    assert!(
        matches!(r, Err(GitError::UnenumerableConfig { .. })),
        "unreadable config must fail closed, got {r:?}"
    );
}

// --- config.worktree: a extensão desloca a fonte da verdade -----------------------

/// Install a filter driver in `.git/config.worktree` and return its marker path.
fn smuggle_via_worktree_config(t: &TempDir, repo: &Path) -> PathBuf {
    let marker = t.path().join("wtc_filter_ran.marker");
    let script = repo.join("wtc-clean.sh");
    fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\ncat\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        repo.join(".git").join("config.worktree"),
        format!(
            "[filter \"evil\"]\n\tclean = {}\n\trequired = false\n",
            script.display()
        ),
    )
    .unwrap();
    fs::write(repo.join(".gitattributes"), "*.dat filter=evil\n").unwrap();
    marker
}

#[test]
fn worktree_config_extension_fails_closed() {
    // `extensions.worktreeConfig` moves part of the effective config into
    // `.git/config.worktree`, which the containment scan does not enumerate. The
    // scanned `.git/config` carries no `[filter]` and no `[include]` — only the
    // extension — so nothing else in the scanner can notice.
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    let marker = smuggle_via_worktree_config(&t, &repo);
    append_cfg(&repo, "[extensions]\n\tworktreeConfig = true\n");

    // POSITIVE CONTROL: without hardening the smuggled filter really runs, so this
    // test measures containment rather than a broken fixture.
    fs::write(repo.join("x.dat"), "x\n").unwrap();
    let up = GitExecutionProfile::unhardened();
    let a = runner
        .spawn(&["add", "--", "x.dat"], &repo, &up, &repo)
        .unwrap();
    assert!(a.success, "control add failed: {}", a.stderr);
    assert!(
        marker.exists(),
        "positive control: config.worktree must be honoured by git"
    );
    fs::remove_file(&marker).unwrap();

    // GOVERNED: refused, and the filter never runs.
    let r = hardened_add(&runner, &repo, "y.dat");
    assert!(
        matches!(r, Err(GitError::UnenumerableConfig { .. })),
        "worktreeConfig must fail closed, got {r:?}"
    );
    assert!(
        !marker.exists(),
        "ESCAPE: a filter in config.worktree ran under the hardened profile"
    );
}

#[test]
fn worktree_config_activation_forms_all_fail_closed() {
    // Every spelling git accepts as "enabled" must be caught: bare key, one-line
    // section, mixed case, and the alternate boolean words.
    for forma in [
        "[extensions]\n\tworktreeConfig = true\n",
        "[extensions]\n\tworktreeConfig\n",
        "[extensions] worktreeConfig = true\n",
        "[ExTeNsIoNs]\n\tWorkTreeConfig = TRUE\n",
        "[extensions]\n\tworktreeConfig = yes\n",
        "[extensions]\n\tworktreeConfig = on\n",
        "[extensions]\n\tworktreeConfig = 1\n",
    ] {
        let t = TempDir::new();
        let repo = t.path().join("repo");
        let runner = GitRunner::new();
        init_repo(&runner, &repo);
        let marker = smuggle_via_worktree_config(&t, &repo);
        append_cfg(&repo, forma);

        let r = hardened_add(&runner, &repo, "y.dat");
        assert!(
            matches!(r, Err(GitError::UnenumerableConfig { .. })),
            "forma {forma:?} must fail closed, got {r:?}"
        );
        assert!(!marker.exists(), "forma {forma:?}: filter ran");
    }
}

#[test]
fn worktree_config_disabled_or_absent_is_not_an_activation() {
    // Refusing must not spread to configs that do not move the source of truth.
    // git honours `config.worktree` only when the extension is enabled; `false`,
    // an empty value, and absence are all "not enabled" and must stay ALLOW.
    // Ordinary repositories: our refusal must not fire *and* the operation must still
    // work end to end.
    for (nome, forma) in [
        ("false", "[extensions]\n\tworktreeConfig = false\n"),
        ("no", "[extensions]\n\tworktreeConfig = no\n"),
        ("off", "[extensions]\n\tworktreeConfig = off\n"),
        ("0", "[extensions]\n\tworktreeConfig = 0\n"),
        ("vazio", "[extensions]\n\tworktreeConfig =\n"),
        ("fora de [extensions]", "[core]\n\tworktreeConfig = true\n"),
        ("ausente", ""),
    ] {
        let t = TempDir::new();
        let repo = t.path().join("repo");
        let runner = GitRunner::new();
        init_repo(&runner, &repo);
        if !forma.is_empty() {
            append_cfg(&repo, forma);
        }
        let out = hardened_add(&runner, &repo, "y.dat")
            .unwrap_or_else(|e| panic!("{nome}: must not be refused, got {e:?}"));
        assert!(out.success, "{nome}: hardened add failed: {}", out.stderr);
    }

    // Other `[extensions]` keys. git itself may reject these depending on
    // `repositoryformatversion` — `objectFormat` is v1-only and is fatal at v0, while
    // `worktreeConfig` is deliberately exempt. That is git's business, not our
    // containment's. The invariant we own is narrower and is the one asserted here:
    // *our* refusal must not fire for a key that is not `worktreeConfig`.
    for (nome, forma) in [
        ("outra extensao", "[extensions]\n\tobjectFormat = sha1\n"),
        (
            "chave parecida",
            "[extensions]\n\tworktreeConfigNotReally = true\n",
        ),
    ] {
        let t = TempDir::new();
        let repo = t.path().join("repo");
        let runner = GitRunner::new();
        init_repo(&runner, &repo);
        append_cfg(&repo, forma);
        let r = hardened_add(&runner, &repo, "y.dat");
        assert!(
            !matches!(r, Err(GitError::UnenumerableConfig { .. })),
            "{nome}: our refusal must not fire for a non-worktreeConfig key, got {r:?}"
        );
    }
}

#[test]
fn global_external_diff_is_contained_with_positive_control() {
    // `diff.external` is a *top-level* key, not a `[diff "name"]` subsection, so the
    // subsection scan structurally cannot see it — no enumeration, no neutralization.
    // It is exactly the setting `GitExecutionProfile::hardened()` claims to disable.
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    let up = GitExecutionProfile::unhardened();

    let marker = t.path().join("ext_diff_ran.marker");
    let script = repo.join("ext-diff.sh");
    fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\nexit 0\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    fs::write(repo.join("a.txt"), "v1\n").unwrap();
    runner.spawn(&["add", "-A"], &repo, &up, &repo).unwrap();
    runner
        .spawn(&["commit", "-m", "seed"], &repo, &up, &repo)
        .unwrap();
    fs::write(repo.join("a.txt"), "v2\n").unwrap();

    append_cfg(
        &repo,
        &format!("[diff]\n\texternal = {}\n", script.display()),
    );

    // POSITIVE CONTROL: unhardened runs the external diff program.
    runner.spawn(&["diff"], &repo, &up, &repo).unwrap();
    assert!(
        marker.exists(),
        "positive control: diff.external must run without hardening"
    );
    fs::remove_file(&marker).unwrap();

    // GOVERNED: the hardened profile must neutralize it.
    let hp = GitExecutionProfile::hardened();
    runner.spawn(&["diff"], &repo, &hp, &repo).unwrap();
    assert!(
        !marker.exists(),
        "ESCAPE: diff.external ran under the hardened profile"
    );
}

#[test]
fn global_external_diff_neutralization_does_not_depend_on_enumeration() {
    // The blanking must be unconditional, not driven by scanning the config. A repo whose
    // config the scanner refuses to enumerate is already denied; a repo it *can* enumerate
    // must still get `diff.external` blanked even though no `[diff "name"]` exists.
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    let hp = GitExecutionProfile::hardened();
    let args = hardening_args(&hp, &repo).expect("clean repo must not be refused");
    assert!(
        args.windows(2)
            .any(|w| w[0] == "-c" && w[1] == "diff.external="),
        "hardened args must blank diff.external unconditionally, got {args:?}"
    );
}

// --- merge drivers: uma classe de driver própria, com flag própria ------------------

/// Build a repo with a real merge conflict plus a driver script, and return the marker the
/// driver touches. The driver definition itself is left to the caller so tests can vary
/// *where* in the config it lives.
fn merge_driver_repo(
    t: &TempDir,
    repo: &Path,
    runner: &GitRunner,
    attrs: &str,
) -> (PathBuf, PathBuf) {
    let up = GitExecutionProfile::unhardened();
    init_repo(runner, repo);
    let marker = t.path().join("merge_driver_ran.marker");
    let script = repo.join("merge-drv.sh");
    fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\nexit 0\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    fs::write(repo.join("a.txt"), "base\n").unwrap();
    fs::write(repo.join(".gitattributes"), attrs).unwrap();
    runner.spawn(&["add", "-A"], repo, &up, repo).unwrap();
    runner
        .spawn(&["commit", "-m", "base"], repo, &up, repo)
        .unwrap();
    runner
        .spawn(&["checkout", "-b", "feat"], repo, &up, repo)
        .unwrap();
    fs::write(repo.join("a.txt"), "feat\n").unwrap();
    runner
        .spawn(&["commit", "-am", "feat"], repo, &up, repo)
        .unwrap();
    runner
        .spawn(&["checkout", "main"], repo, &up, repo)
        .unwrap();
    fs::write(repo.join("a.txt"), "main\n").unwrap();
    runner
        .spawn(&["commit", "-am", "main"], repo, &up, repo)
        .unwrap();
    (marker, script)
}

fn merge_feat(runner: &GitRunner, repo: &Path, hardened: bool) -> Result<GitOutput, GitError> {
    let p = if hardened {
        GitExecutionProfile::hardened()
    } else {
        GitExecutionProfile::unhardened()
    };
    runner.spawn(&["merge", "feat", "-m", "m"], repo, &p, repo)
}

#[test]
fn merge_driver_asymmetry_unhardened_executes_hardened_does_not() {
    // The acceptance matrix in one test: the same hostile repository must execute the
    // driver without hardening and must not execute it with hardening. One half alone
    // proves nothing — together they prove the vector is real *and* contained.
    // Each half gets its own repository. Reusing one would make the second half vacuous:
    // the first merge completes and creates a merge commit, so a second `merge feat` is
    // "Already up to date" and never invokes a driver at all.
    let runner = GitRunner::new();

    // UNHARDENED + malicious merge driver = EXECUTED
    let t1 = TempDir::new();
    let repo1 = t1.path().join("repo");
    let (marker1, script1) = merge_driver_repo(&t1, &repo1, &runner, "*.txt merge=evil\n");
    append_cfg(
        &repo1,
        &format!(
            "[merge \"evil\"]\n\tname = evil\n\tdriver = {} %O %A %B\n",
            script1.display()
        ),
    );
    merge_feat(&runner, &repo1, false).unwrap();
    assert!(
        marker1.exists(),
        "positive control: the merge driver must run without hardening"
    );

    // HARDENED + malicious merge driver = NOT EXECUTED
    let t2 = TempDir::new();
    let repo2 = t2.path().join("repo");
    let (marker2, script2) = merge_driver_repo(&t2, &repo2, &runner, "*.txt merge=evil\n");
    append_cfg(
        &repo2,
        &format!(
            "[merge \"evil\"]\n\tname = evil\n\tdriver = {} %O %A %B\n",
            script2.display()
        ),
    );
    merge_feat(&runner, &repo2, true).unwrap();
    assert!(
        !marker2.exists(),
        "ESCAPE: the merge driver ran under the hardened profile"
    );
}

#[test]
fn merge_driver_case_variant_is_neutralized() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    let (marker, script) = merge_driver_repo(&t, &repo, &runner, "*.txt merge=evil\n");
    // Section name is case-insensitive to git; the subsection name is not.
    append_cfg(
        &repo,
        &format!(
            "[MeRgE \"evil\"]\n\tname = evil\n\tdriver = {} %O %A %B\n",
            script.display()
        ),
    );
    merge_feat(&runner, &repo, true).unwrap();
    assert!(!marker.exists(), "ESCAPE: [MeRgE] was not neutralized");
}

#[test]
fn multiple_merge_drivers_are_all_neutralized() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    let (marker, script) = merge_driver_repo(&t, &repo, &runner, "*.txt merge=second\n");
    append_cfg(
        &repo,
        &format!(
            "[merge \"first\"]\n\tname = a\n\tdriver = {s} %O %A %B\n\
             [merge \"second\"]\n\tname = b\n\tdriver = {s} %O %A %B\n",
            s = script.display()
        ),
    );
    let args = hardening_args(&GitExecutionProfile::hardened(), &repo).unwrap();
    for nome in ["first", "second"] {
        assert!(
            args.windows(2)
                .any(|w| w[0] == "-c" && w[1] == format!("merge.{nome}.driver=")),
            "driver {nome} must be blanked, got {args:?}"
        );
    }
    merge_feat(&runner, &repo, true).unwrap();
    assert!(!marker.exists(), "ESCAPE: a second merge driver ran");
}

#[test]
fn merge_driver_smuggled_via_include_fails_closed() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    let (marker, script) = merge_driver_repo(&t, &repo, &runner, "*.txt merge=evil\n");
    let inc = repo.join(".git").join("m.config");
    fs::write(
        &inc,
        format!(
            "[merge \"evil\"]\n\tname = evil\n\tdriver = {} %O %A %B\n",
            script.display()
        ),
    )
    .unwrap();
    append_cfg(&repo, &format!("[include]\n\tpath = {}\n", inc.display()));

    let r = merge_feat(&runner, &repo, true);
    assert!(
        matches!(r, Err(GitError::UnenumerableConfig { .. })),
        "include carrying a merge driver must fail closed, got {r:?}"
    );
    assert!(!marker.exists(), "ESCAPE: driver ran via include");
}

#[test]
fn merge_driver_smuggled_via_worktree_config_fails_closed() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    let (marker, script) = merge_driver_repo(&t, &repo, &runner, "*.txt merge=evil\n");
    fs::write(
        repo.join(".git").join("config.worktree"),
        format!(
            "[merge \"evil\"]\n\tname = evil\n\tdriver = {} %O %A %B\n",
            script.display()
        ),
    )
    .unwrap();
    append_cfg(&repo, "[extensions]\n\tworktreeConfig = true\n");

    let r = merge_feat(&runner, &repo, true);
    assert!(
        matches!(r, Err(GitError::UnenumerableConfig { .. })),
        "config.worktree carrying a merge driver must fail closed, got {r:?}"
    );
    assert!(!marker.exists(), "ESCAPE: driver ran via config.worktree");
}

#[test]
fn repository_without_merge_driver_is_unaffected() {
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    let (marker, _) = merge_driver_repo(&t, &repo, &runner, "*.txt text\n");
    let out = merge_feat(&runner, &repo, true).expect("ordinary merge must not be refused");
    let _ = out;
    assert!(!marker.exists());
    // And no merge.* override is emitted when there is nothing to neutralize.
    let args = hardening_args(&GitExecutionProfile::hardened(), &repo).unwrap();
    assert!(
        !args.iter().any(|a| a.starts_with("merge.")),
        "no merge override expected, got {args:?}"
    );
}

#[test]
fn unhardened_profile_still_permits_merge_drivers() {
    // `unhardened()` must remain a real positive control. If it ever stopped permitting
    // merge drivers, every containment test above would silently become vacuous.
    let p = GitExecutionProfile::unhardened();
    assert!(
        p.allow_merge_drivers,
        "unhardened must permit merge drivers"
    );
    assert!(
        !GitExecutionProfile::hardened().allow_merge_drivers,
        "hardened must deny merge drivers"
    );
    let t = TempDir::new();
    let repo = t.path().join("repo");
    let runner = GitRunner::new();
    init_repo(&runner, &repo);
    let args = hardening_args(&p, &repo).unwrap();
    assert!(
        !args.iter().any(|a| a.starts_with("merge.")),
        "unhardened must not blank merge drivers, got {args:?}"
    );
}
