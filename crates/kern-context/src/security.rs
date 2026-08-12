//! Context security boundaries (G8.9, G8.18, G8.19).
//!
//! Three distinct concerns are kept separate:
//! - **Containment** ([`safe_join`]): a resolved path must stay inside the worktree, even
//!   through symlinks (`fs::canonicalize` resolves the link, then we require containment).
//! - **Sensitive material** ([`sensitive_reason`]): a default-deny classifier for secret
//!   classes (`.env`, keys, credential stores, `.ssh`, `.aws`, ...). This is a *security*
//!   policy.
//! - **Relevance ignores** ([`is_ignored_for_relevance`]): `.git`, `target/`, vendor caches
//!   are skipped for *retrieval relevance*, which is explicitly **not** a security boundary.

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::types::ContextError;

/// Join `rel` onto `worktree` and require the real (symlink-resolved) result to stay inside
/// the worktree. Absolute inputs, `..` escapes, and out-of-tree symlink targets are refused.
pub fn safe_join(worktree: &Path, rel: &str) -> Result<PathBuf, ContextError> {
    let rel_path = Path::new(rel);
    // Reject absolute inputs and any explicit parent traversal up front (defense in depth).
    if rel_path.is_absolute() {
        return Err(ContextError::PathEscape(rel.to_string()));
    }
    if rel_path
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(ContextError::PathEscape(rel.to_string()));
    }
    let candidate = worktree.join(rel_path);

    let wt_canon = fs::canonicalize(worktree).map_err(|e| ContextError::Io(e.to_string()))?;
    let cand_canon =
        fs::canonicalize(&candidate).map_err(|_| ContextError::PathEscape(rel.to_string()))?;

    if cand_canon == wt_canon || cand_canon.starts_with(&wt_canon) {
        Ok(cand_canon)
    } else {
        // A symlink (or other indirection) pointed outside the worktree.
        Err(ContextError::PathEscape(rel.to_string()))
    }
}

/// If `rel` names a class of sensitive material, return a reason. Default-deny: callers must
/// refuse a `Some(_)` unless a future capability explicitly permits it.
#[must_use]
pub fn sensitive_reason(rel: &Path) -> Option<String> {
    let lower = rel.to_string_lossy().to_lowercase();

    // Sensitive directory components anywhere in the path.
    for comp in rel.components() {
        if let Component::Normal(os) = comp {
            let seg = os.to_string_lossy().to_lowercase();
            if matches!(
                seg.as_str(),
                ".ssh" | ".aws" | ".gnupg" | ".gpg" | "secrets"
            ) {
                return Some(format!("sensitive directory: {seg}"));
            }
        }
    }

    let file = rel
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // `.env` and its variants (`.env`, `.env.local`, ...).
    if file == ".env" || file.starts_with(".env.") {
        return Some("dotenv file".to_string());
    }

    // Exact sensitive filenames.
    if matches!(
        file.as_str(),
        "id_rsa"
            | "id_dsa"
            | "id_ecdsa"
            | "id_ed25519"
            | ".netrc"
            | ".npmrc"
            | ".pypirc"
            | "credentials"
    ) || file.contains("credentials")
    {
        return Some(format!("credential file: {file}"));
    }

    // Sensitive extensions.
    if let Some(ext) = rel.extension().map(|e| e.to_string_lossy().to_lowercase()) {
        if matches!(
            ext.as_str(),
            "pem" | "key" | "p12" | "pfx" | "keystore" | "asc" | "gpg"
        ) {
            return Some(format!("key material: .{ext}"));
        }
    }

    let _ = &lower;
    None
}

/// Paths skipped for retrieval relevance (NOT a security boundary).
#[must_use]
pub fn is_ignored_for_relevance(rel: &Path) -> bool {
    rel.components().any(|c| {
        matches!(c, Component::Normal(os) if {
            let s = os.to_string_lossy();
            s == ".git" || s == "target" || s == "node_modules" || s == "vendor"
        })
    })
}

/// True when the extension suggests non-text/binary content the engine should not ingest.
#[must_use]
pub fn is_binary_like(rel: &Path) -> bool {
    rel.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .is_some_and(|ext| {
            matches!(
                ext.as_str(),
                "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "pdf"
                    | "zip"
                    | "gz"
                    | "tar"
                    | "wasm"
                    | "so"
                    | "dylib"
                    | "a"
                    | "o"
                    | "bin"
                    | "exe"
                    | "class"
                    | "jar"
                    | "ico"
                    | "woff"
                    | "woff2"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_classes_flagged() {
        assert!(sensitive_reason(Path::new(".env")).is_some());
        assert!(sensitive_reason(Path::new("config/.env.local")).is_some());
        assert!(sensitive_reason(Path::new("deploy/id_rsa")).is_some());
        assert!(sensitive_reason(Path::new("certs/server.pem")).is_some());
        assert!(sensitive_reason(Path::new("home/.ssh/known_hosts")).is_some());
        assert!(sensitive_reason(Path::new("aws/credentials")).is_some());
        // ordinary source is not sensitive
        assert!(sensitive_reason(Path::new("src/main.rs")).is_none());
    }

    #[test]
    fn ignore_is_not_security() {
        assert!(is_ignored_for_relevance(Path::new("target/debug/foo")));
        assert!(is_ignored_for_relevance(Path::new(".git/config")));
        assert!(!is_ignored_for_relevance(Path::new("src/lib.rs")));
    }

    #[test]
    fn binary_detection() {
        assert!(is_binary_like(Path::new("a/b.png")));
        assert!(!is_binary_like(Path::new("a/b.rs")));
    }
}
