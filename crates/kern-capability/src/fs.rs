//! Filesystem scope: lexical containment of requested paths within granted roots.
//!
//! Containment is **lexical** (it folds `.` and `..` without touching the filesystem), so
//! it is deterministic and cannot be fooled into blocking on a slow or adversarial path.
//! Symlink resolution is a *separate* boundary enforced at actual FS-access time; a
//! capability that lexically contains a path still does not authorize following a symlink
//! out of scope.

use std::path::{Component, Path, PathBuf};

/// Fold `.` and `..` components without consulting the filesystem. `..` pops the
/// accumulated path but never above the root, so an over-long `..` chain collapses to the
/// root (and therefore fails a `starts_with` check against a deeper root).
pub(crate) fn lexical_normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::Prefix(pre) => out.push(pre.as_os_str()),
            Component::RootDir => out.push(Component::RootDir.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(s) => out.push(s),
        }
    }
    out
}

/// A set of canonical roots a capability permits access within.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FsScope {
    roots: Vec<PathBuf>,
}

impl FsScope {
    /// Build a scope from roots (each is lexically normalized on entry).
    #[must_use]
    pub fn new(roots: &[PathBuf]) -> Self {
        let roots = roots.iter().map(|r| lexical_normalize(r)).collect();
        Self { roots }
    }

    /// A scope granting a single root.
    #[must_use]
    pub fn root(root: impl Into<PathBuf>) -> Self {
        Self::new(&[root.into()])
    }

    /// The granted roots.
    #[must_use]
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// True when `requested` resolves inside one of the roots.
    ///
    /// Relative requests are joined onto each root; absolute requests are checked as-is.
    /// A `..` chain that escapes, or an absolute path outside every root, is refused.
    #[must_use]
    pub fn allows(&self, requested: &str) -> bool {
        let req = Path::new(requested);
        for root in &self.roots {
            let candidate = if req.is_absolute() {
                req.to_path_buf()
            } else {
                root.join(req)
            };
            let cn = lexical_normalize(&candidate);
            if cn == *root || cn.starts_with(root) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> FsScope {
        FsScope::root("/work/repo")
    }

    #[test]
    fn allows_inside() {
        assert!(scope().allows("src/main.rs"));
        assert!(scope().allows("/work/repo/src/lib.rs"));
        assert!(scope().allows("."));
    }

    #[test]
    fn blocks_parent_traversal() {
        assert!(!scope().allows("../secret"));
        assert!(!scope().allows("../../etc/passwd"));
        assert!(!scope().allows("src/../../escape"));
    }

    #[test]
    fn blocks_absolute_outside() {
        assert!(!scope().allows("/etc/passwd"));
        assert!(!scope().allows("/work/repo-sibling/x"));
    }

    #[test]
    fn sibling_prefix_is_not_contained() {
        // "/work/repository" must not be considered inside "/work/repo".
        assert!(!FsScope::root("/work/repo").allows("/work/repository/x"));
    }
}
