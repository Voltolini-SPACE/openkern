//! `kern-repo` — repository identity.
//!
//! Identity is **never** `cwd` alone. It is resolved by reading the Git plumbing files
//! directly (`std::fs` only — no git subprocess, so a hostile `.git/config` cannot run
//! anything during resolution). A linked worktree, a submodule, a separate git-dir, a
//! `.git` redirect file, and a symlinked path all resolve to a canonical, comparable
//! identity, so a capability minted for one repository can be checked against the actual
//! target and never confused for another.

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use kern_types::{RepositoryId, WorktreeId};

/// Where `HEAD` points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    /// A symbolic ref, e.g. `refs/heads/main`.
    Ref(String),
    /// A detached object id.
    Detached(String),
}

/// How a worktree relates to its repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeKind {
    /// The main worktree (its `.git` is a directory).
    Main,
    /// A linked worktree (`.git` file -> `.../.git/worktrees/<name>`).
    Linked,
    /// A submodule (`.git` file -> `.../.git/modules/<name>`).
    Submodule,
    /// A worktree with a separate git-dir that is neither linked nor a submodule.
    SeparateGitDir,
}

/// Error resolving repository identity.
#[derive(Debug)]
pub enum RepoError {
    /// No `.git` was found walking up from the start path.
    NotARepository(PathBuf),
    /// A `.git` file was present but malformed.
    InvalidGitFile(PathBuf),
    /// An I/O error.
    Io(std::io::Error),
}

impl core::fmt::Display for RepoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RepoError::NotARepository(p) => write!(f, "not a repository: {}", p.display()),
            RepoError::InvalidGitFile(p) => write!(f, "invalid .git file: {}", p.display()),
            RepoError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for RepoError {}

impl From<std::io::Error> for RepoError {
    fn from(e: std::io::Error) -> Self {
        RepoError::Io(e)
    }
}

/// A resolved, canonical repository identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryIdentity {
    worktree: PathBuf,
    git_dir: PathBuf,
    common_dir: PathBuf,
    head: Head,
    kind: WorktreeKind,
    repository_id: RepositoryId,
    worktree_id: WorktreeId,
}

fn stable_id(prefix: &str, path: &Path) -> String {
    let mut h = DefaultHasher::new();
    path.to_string_lossy().as_ref().hash(&mut h);
    format!("{prefix}-{:016x}", h.finish())
}

fn parse_head(raw: &str) -> Head {
    let t = raw.trim();
    t.strip_prefix("ref:").map_or_else(
        || Head::Detached(t.to_string()),
        |r| Head::Ref(r.trim().to_string()),
    )
}

impl RepositoryIdentity {
    /// Resolve identity by walking up from `start` until a `.git` is found. The `.git`
    /// may be a directory (main worktree) or a file redirect (linked worktree, submodule,
    /// or separate git-dir). All paths are canonicalized, so symlinks and renames resolve
    /// to the real location.
    pub fn resolve(start: impl AsRef<Path>) -> Result<Self, RepoError> {
        let start = fs::canonicalize(start.as_ref())?;
        let (worktree, git_entry) =
            find_worktree_root(&start).ok_or_else(|| RepoError::NotARepository(start.clone()))?;

        let meta = fs::symlink_metadata(&git_entry)?;
        let (git_dir, kind_hint) = if meta.is_dir() {
            (fs::canonicalize(&git_entry)?, WorktreeKind::Main)
        } else {
            let contents = fs::read_to_string(&git_entry)?;
            let target = contents
                .trim()
                .strip_prefix("gitdir:")
                .map(str::trim)
                .ok_or_else(|| RepoError::InvalidGitFile(git_entry.clone()))?;
            let raw = PathBuf::from(target);
            let joined = if raw.is_absolute() {
                raw
            } else {
                worktree.join(raw)
            };
            (fs::canonicalize(&joined)?, WorktreeKind::SeparateGitDir)
        };

        // common_dir: linked worktrees carry a `commondir` file pointing back at the
        // shared store. Absent -> the git_dir is itself the common dir.
        let commondir_file = git_dir.join("commondir");
        let common_dir = if commondir_file.is_file() {
            let rel = fs::read_to_string(&commondir_file)?;
            fs::canonicalize(git_dir.join(rel.trim()))?
        } else {
            git_dir.clone()
        };

        let head_raw = fs::read_to_string(git_dir.join("HEAD"))?;
        let head = parse_head(&head_raw);

        let kind = classify(kind_hint, &git_dir, &common_dir);

        let repository_id = RepositoryId::new(stable_id("repo", &common_dir))
            .map_err(|_| RepoError::NotARepository(common_dir.clone()))?;
        let worktree_id = WorktreeId::new(stable_id("wt", &worktree))
            .map_err(|_| RepoError::NotARepository(worktree.clone()))?;

        Ok(Self {
            worktree,
            git_dir,
            common_dir,
            head,
            kind,
            repository_id,
            worktree_id,
        })
    }

    /// The canonical worktree root.
    #[must_use]
    pub fn worktree(&self) -> &Path {
        &self.worktree
    }

    /// The git-dir for this worktree.
    #[must_use]
    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    /// The shared common dir (object store). Two worktrees of the same repository share
    /// this, and therefore share [`repository_id`](Self::repository_id).
    #[must_use]
    pub fn common_dir(&self) -> &Path {
        &self.common_dir
    }

    /// Where `HEAD` points.
    #[must_use]
    pub fn head(&self) -> &Head {
        &self.head
    }

    /// How this worktree relates to its repository.
    #[must_use]
    pub fn kind(&self) -> WorktreeKind {
        self.kind
    }

    /// A stable id derived from the common dir. Same repository (even across linked
    /// worktrees) -> same id; different repository -> different id.
    #[must_use]
    pub fn repository_id(&self) -> &RepositoryId {
        &self.repository_id
    }

    /// A stable id derived from the worktree path. Distinct per worktree.
    #[must_use]
    pub fn worktree_id(&self) -> &WorktreeId {
        &self.worktree_id
    }

    /// True when `self` and `other` are the same repository (same common dir).
    #[must_use]
    pub fn same_repository(&self, other: &RepositoryIdentity) -> bool {
        self.repository_id == other.repository_id
    }
}

fn classify(hint: WorktreeKind, git_dir: &Path, common_dir: &Path) -> WorktreeKind {
    if hint == WorktreeKind::Main {
        return WorktreeKind::Main;
    }
    let g = git_dir.to_string_lossy();
    if g.contains("/worktrees/") && git_dir != common_dir {
        WorktreeKind::Linked
    } else if g.contains("/modules/") {
        WorktreeKind::Submodule
    } else {
        WorktreeKind::SeparateGitDir
    }
}

/// Walk up from `start` until a directory containing a `.git` entry is found.
fn find_worktree_root(start: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let git = dir.join(".git");
        if git.exists() {
            return Some((dir.to_path_buf(), git));
        }
        cur = dir.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

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
                "openkern-repo-{}-{nanos:x}-{n:x}",
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

    /// Build a bare-minimum main worktree: `<root>/.git/` with HEAD + objects.
    fn main_repo(root: &Path, branch: &str) {
        let git = root.join(".git");
        fs::create_dir_all(git.join("objects")).unwrap();
        fs::create_dir_all(git.join("refs")).unwrap();
        fs::write(git.join("HEAD"), format!("ref: refs/heads/{branch}\n")).unwrap();
        fs::write(git.join("config"), "[core]\n").unwrap();
    }

    #[test]
    fn resolves_main_worktree() {
        let t = TempDir::new();
        let root = t.path().join("repo");
        fs::create_dir_all(root.join("src")).unwrap();
        main_repo(&root, "main");

        // resolving from a subdirectory still finds the root
        let id = RepositoryIdentity::resolve(root.join("src")).unwrap();
        assert_eq!(id.worktree(), root.as_path());
        assert_eq!(id.kind(), WorktreeKind::Main);
        assert_eq!(id.head(), &Head::Ref("refs/heads/main".into()));
        assert_eq!(id.common_dir(), id.git_dir());
    }

    #[test]
    fn two_repos_have_distinct_identity_no_cross_effect() {
        let t = TempDir::new();
        let a = t.path().join("a");
        let b = t.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        main_repo(&a, "main");
        main_repo(&b, "main");

        let id_a = RepositoryIdentity::resolve(&a).unwrap();
        let id_b = RepositoryIdentity::resolve(&b).unwrap();
        assert_ne!(id_a.repository_id(), id_b.repository_id());
        assert!(!id_a.same_repository(&id_b));
    }

    #[test]
    fn linked_worktree_shares_repository_but_distinct_worktree() {
        let t = TempDir::new();
        let root = t.path().join("repo");
        fs::create_dir_all(&root).unwrap();
        main_repo(&root, "main");

        // linked worktree plumbing: <root>/.git/worktrees/wt1 with HEAD + commondir
        let wt_admin = root.join(".git/worktrees/wt1");
        fs::create_dir_all(&wt_admin).unwrap();
        fs::write(wt_admin.join("HEAD"), "ref: refs/heads/feature\n").unwrap();
        fs::write(wt_admin.join("commondir"), "../..\n").unwrap();

        let linked = t.path().join("wt1");
        fs::create_dir_all(&linked).unwrap();
        fs::write(
            linked.join(".git"),
            format!("gitdir: {}\n", wt_admin.display()),
        )
        .unwrap();

        let main = RepositoryIdentity::resolve(&root).unwrap();
        let link = RepositoryIdentity::resolve(&linked).unwrap();

        assert_eq!(link.kind(), WorktreeKind::Linked);
        assert!(main.same_repository(&link), "linked worktree = same repo");
        assert_ne!(main.worktree_id(), link.worktree_id(), "distinct worktrees");
        assert_eq!(link.head(), &Head::Ref("refs/heads/feature".into()));
    }

    #[test]
    fn submodule_is_a_distinct_repository() {
        let t = TempDir::new();
        let sup = t.path().join("super");
        fs::create_dir_all(&sup).unwrap();
        main_repo(&sup, "main");

        // submodule admin dir under the superproject's .git/modules/sub
        let mod_dir = sup.join(".git/modules/sub");
        fs::create_dir_all(mod_dir.join("objects")).unwrap();
        fs::write(mod_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let sub_wt = sup.join("sub");
        fs::create_dir_all(&sub_wt).unwrap();
        fs::write(
            sub_wt.join(".git"),
            format!("gitdir: {}\n", mod_dir.display()),
        )
        .unwrap();

        let super_id = RepositoryIdentity::resolve(&sup).unwrap();
        let module_id = RepositoryIdentity::resolve(&sub_wt).unwrap();
        assert_eq!(module_id.kind(), WorktreeKind::Submodule);
        assert!(
            !super_id.same_repository(&module_id),
            "submodule must not be the same repository as its superproject"
        );
    }

    #[test]
    fn symlinked_path_resolves_to_real_repo() {
        let t = TempDir::new();
        let root = t.path().join("repo");
        fs::create_dir_all(&root).unwrap();
        main_repo(&root, "main");

        let link = t.path().join("link");
        std::os::unix::fs::symlink(&root, &link).unwrap();

        let direct = RepositoryIdentity::resolve(&root).unwrap();
        let viasym = RepositoryIdentity::resolve(&link).unwrap();
        assert!(direct.same_repository(&viasym));
        assert_eq!(direct.worktree_id(), viasym.worktree_id());
    }

    #[test]
    fn head_change_is_observed_on_reresolve() {
        let t = TempDir::new();
        let root = t.path().join("repo");
        fs::create_dir_all(&root).unwrap();
        main_repo(&root, "main");

        let before = RepositoryIdentity::resolve(&root).unwrap();
        assert_eq!(before.head(), &Head::Ref("refs/heads/main".into()));
        // swap HEAD (simulating a branch checkout / repo swap)
        fs::write(root.join(".git/HEAD"), "ref: refs/heads/other\n").unwrap();
        let after = RepositoryIdentity::resolve(&root).unwrap();
        assert_eq!(after.head(), &Head::Ref("refs/heads/other".into()));
    }

    #[test]
    fn not_a_repository_errors() {
        let t = TempDir::new();
        let plain = t.path().join("plain");
        fs::create_dir_all(&plain).unwrap();
        assert!(matches!(
            RepositoryIdentity::resolve(&plain),
            Err(RepoError::NotARepository(_))
        ));
    }
}
