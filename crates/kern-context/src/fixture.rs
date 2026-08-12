//! A deterministic benchmark/adversarial fixture corpus (G8.25).
//!
//! A tiny project with *known* relationships: `run_mission` (in `exec.rs`) calls
//! `RepositoryIdentity::resolve` and `.head()` (in `repo.rs`); `util.rs` is an irrelevant
//! distractor; there are secret decoys (`.env`, `secrets/key.pem`) and a same-name symbol.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use kern_types::{RepositoryId, WorktreeId};

use crate::index::SymbolIndex;
use crate::types::Revision;

static N: AtomicU64 = AtomicU64::new(0);

/// A temp worktree that cleans up on drop.
pub struct Corpus {
    pub root: PathBuf,
}

impl Corpus {
    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn index(&self) -> SymbolIndex {
        SymbolIndex::build(
            RepositoryId::new("repo-A").unwrap(),
            WorktreeId::new("wt-A").unwrap(),
            &self.root,
            Revision("rev1".to_string()),
        )
        .expect("index builds")
    }
}

impl Drop for Corpus {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write(root: &Path, rel: &str, content: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

/// Build the standard corpus.
pub fn build_corpus() -> Corpus {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let n = N.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "openkern-ctx-{}-{nanos:x}-{n:x}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let root = fs::canonicalize(&root).unwrap();

    write(
        &root,
        "src/lib.rs",
        "pub mod repo;\npub mod exec;\npub mod util;\n",
    );
    write(
        &root,
        "src/repo.rs",
        "/// Repository identity.\n\
         pub struct RepositoryIdentity {\n    head: String,\n}\n\n\
         impl RepositoryIdentity {\n    \
         pub fn resolve(path: &str) -> RepositoryIdentity {\n        \
         RepositoryIdentity { head: path.to_string() }\n    }\n    \
         pub fn head(&self) -> &str {\n        &self.head\n    }\n}\n",
    );
    write(
        &root,
        "src/exec.rs",
        "use crate::repo::RepositoryIdentity;\n\n\
         pub fn run_mission(path: &str) -> String {\n    \
         let identity = RepositoryIdentity::resolve(path);\n    \
         identity.head().to_string()\n}\n",
    );
    write(
        &root,
        "src/util.rs",
        "pub fn distractor_helper(x: u32) -> u32 {\n    x + 1\n}\n\n\
         pub struct Unrelated;\n\n\
         impl Unrelated {\n    pub fn new() -> Unrelated {\n        Unrelated\n    }\n}\n",
    );
    // same-name symbol `new` also appears here (ambiguous name on purpose).
    write(
        &root,
        "src/factory.rs",
        "pub struct Widget;\n\nimpl Widget {\n    pub fn new() -> Widget {\n        Widget\n    }\n}\n",
    );
    write(
        &root,
        "tests/repo_test.rs",
        "fn covers_resolve() {\n    let _ = crate::repo::RepositoryIdentity::resolve(\"x\");\n}\n",
    );
    write(&root, "Cargo.toml", "[package]\nname = \"fixture\"\n");
    write(
        &root,
        "README.md",
        "# Fixture\nrun_mission resolves repository identity.\n",
    );

    // Distractor bulk: irrelevant modules that dominate a real repo. None match the
    // benchmark queries, so they inflate the naive full-file baseline without helping it.
    for m in 0..3 {
        let mut noise = String::new();
        for f in 0..20 {
            let _ = write!(
                noise,
                "/// Unrelated helper number {m}-{f}.\n\
                 pub fn noise_{m}_{f}(input: u64) -> u64 {{\n    \
                 let mut acc = input.wrapping_add({f});\n    \
                 acc = acc.wrapping_mul(2654435761);\n    \
                 acc ^= acc >> 15;\n    \
                 acc.wrapping_add({m})\n}}\n\n"
            );
        }
        write(&root, &format!("src/noise_{m}.rs"), &noise);
    }

    // Secret decoys (must never be indexed).
    write(&root, ".env", "SECRET_TOKEN=leak-me\n");
    write(&root, "secrets/key.pem", "-----BEGIN PRIVATE KEY-----\nx\n");

    Corpus { root }
}
