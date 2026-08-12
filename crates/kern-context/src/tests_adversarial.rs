//! Adversarial battery + core behaviour (G8.8, G8.9, G8.11, G8.13).

use std::collections::BTreeSet;
use std::fs;

use kern_types::{MissionId, RepositoryId, WorktreeId};

use crate::engine::ContextEngine;
use crate::fixture::build_corpus;
use crate::types::{
    ContextBudget, ContextError, ContextEvent, ContextQuery, EdgeKind, Freshness, SourceKind,
};

fn query(text: &str, seeds: &[&str]) -> ContextQuery {
    let mut allowed = BTreeSet::new();
    allowed.insert(SourceKind::Symbol);
    ContextQuery {
        mission: MissionId::new("m1").unwrap(),
        repository: RepositoryId::new("repo-A").unwrap(),
        worktree: WorktreeId::new("wt-A").unwrap(),
        text: text.to_string(),
        seed_paths: Vec::new(),
        seed_symbols: seeds.iter().map(|s| (*s).to_string()).collect(),
        budget: ContextBudget::default(),
        max_depth: 2,
        allowed_sources: allowed,
        freshness: Freshness::AnyRevision,
    }
}

fn qnames(pack: &crate::types::ContextPack) -> Vec<String> {
    pack.items
        .iter()
        .filter_map(|i| i.provenance.symbol.clone())
        .map(|s| s.0)
        .collect()
}

#[test]
fn index_extracts_symbols_and_call_graph() {
    let corpus = build_corpus();
    let idx = corpus.index();

    assert!(!idx.by_name("RepositoryIdentity").is_empty());
    assert!(!idx.by_name("resolve").is_empty());
    assert!(!idx.by_name("head").is_empty());
    assert!(!idx.by_name("run_mission").is_empty());

    // run_mission -> resolve must be a Calls edge (unique-name resolution).
    let run = idx.by_name("run_mission")[0].clone();
    let resolve = idx.by_name("resolve")[0].clone();
    let has_call = idx
        .graph()
        .outgoing(&run)
        .iter()
        .any(|(to, kind)| *to == resolve && *kind == EdgeKind::Calls);
    assert!(has_call, "expected run_mission --Calls--> resolve");
}

#[test]
fn engine_pack_pulls_graph_neighbours_not_distractors() {
    let corpus = build_corpus();
    let idx = corpus.index();
    let mut engine = ContextEngine::new();
    let pack = engine
        .build_pack(&idx, &query("resolve repository identity", &["resolve"]))
        .unwrap();

    let names: Vec<String> = pack
        .items
        .iter()
        .filter_map(|i| i.provenance.symbol.as_ref())
        .filter_map(|s| idx.symbol(s))
        .map(|s| s.name.clone())
        .collect();

    assert!(names.iter().any(|n| n == "resolve"), "must include resolve");
    assert!(
        names.iter().any(|n| n == "head"),
        "must include head via graph"
    );
    assert!(
        names.iter().any(|n| n == "run_mission"),
        "must include caller via graph"
    );
    assert!(
        !names.iter().any(|n| n == "distractor_helper"),
        "must not include the distractor"
    );
    // pack is repository/mission-bound
    assert_eq!(pack.repository_id.as_str(), "repo-A");
}

#[test]
fn determinism_same_pack_hash_across_rebuilds() {
    let corpus = build_corpus();
    let q = query("resolve repository identity", &["resolve"]);

    let idx1 = corpus.index();
    let mut e1 = ContextEngine::new();
    let p1 = e1.build_pack(&idx1, &q).unwrap();

    let idx2 = corpus.index();
    let mut e2 = ContextEngine::new();
    let p2 = e2.build_pack(&idx2, &q).unwrap();

    assert_eq!(p1.pack_hash, p2.pack_hash, "deterministic pack hash");
    assert_eq!(p1.pack_id, p2.pack_id);
    assert_eq!(qnames(&p1), qnames(&p2), "deterministic selection order");
}

#[test]
fn budget_bounds_selection() {
    let corpus = build_corpus();
    let idx = corpus.index();
    let mut q = query("resolve repository identity head run mission", &["resolve"]);
    q.budget = ContextBudget {
        max_items: 2,
        max_bytes: 10_000,
        max_estimated_tokens: 10_000,
        max_depth: 3,
    };
    let mut engine = ContextEngine::new();
    let pack = engine.build_pack(&idx, &q).unwrap();
    assert!(pack.items.len() <= 2, "budget max_items respected");
    assert!(pack.total_bytes <= 10_000);
}

#[test]
fn cross_repository_query_is_denied() {
    let corpus = build_corpus();
    let idx = corpus.index();
    let mut q = query("resolve", &["resolve"]);
    q.repository = RepositoryId::new("repo-B").unwrap(); // different repo
    let mut engine = ContextEngine::new();
    let err = engine.build_pack(&idx, &q).unwrap_err();
    assert!(matches!(err, ContextError::RepositoryMismatch));
}

#[test]
fn secrets_are_never_indexed() {
    let corpus = build_corpus();
    let idx = corpus.index();
    // no file record for secret decoys
    for id in idx.files().keys() {
        assert!(!id.0.contains(".env"), "indexed a .env: {}", id.0);
        assert!(!id.0.contains("secrets/"), "indexed a secret: {}", id.0);
        assert!(!id.0.contains(".pem"), "indexed key material: {}", id.0);
    }
    // and the denial is observable
    let denied: Vec<&ContextEvent> = idx
        .events()
        .iter()
        .filter(|e| matches!(e, ContextEvent::Denied(_)))
        .collect();
    assert!(
        denied
            .iter()
            .any(|e| matches!(e, ContextEvent::Denied(m) if m.contains(".env"))),
        "expected a denial event for .env"
    );
}

#[test]
fn symlink_escape_is_refused() {
    let corpus = build_corpus();
    // create a symlink inside the worktree pointing outside it
    let link = corpus.path().join("escape.rs");
    std::os::unix::fs::symlink("/etc/hosts", &link).unwrap();

    // indexing skips symlinks
    let idx = corpus.index();
    for id in idx.files().keys() {
        assert_ne!(id.0, "escape.rs", "symlink must not be indexed");
    }

    // explicit access via safe_join is refused
    let err = crate::security::safe_join(corpus.path(), "escape.rs").unwrap_err();
    assert!(matches!(err, ContextError::PathEscape(_)));
}

#[test]
fn traversal_and_absolute_paths_refused() {
    let corpus = build_corpus();
    assert!(matches!(
        crate::security::safe_join(corpus.path(), "../etc/passwd"),
        Err(ContextError::PathEscape(_))
    ));
    assert!(matches!(
        crate::security::safe_join(corpus.path(), "/etc/passwd"),
        Err(ContextError::PathEscape(_))
    ));
}

#[test]
fn stale_content_toctou_is_detected() {
    let corpus = build_corpus();
    let idx = corpus.index();
    let q = query("resolve repository identity", &["resolve"]);

    // Mutate a file that will be selected, shifting its lines so the indexed span hashes differ.
    let repo_rs = corpus.path().join("src/repo.rs");
    let original = fs::read_to_string(&repo_rs).unwrap();
    fs::write(
        &repo_rs,
        format!("// injected line\n// another\n{original}"),
    )
    .unwrap();

    let mut engine = ContextEngine::new();
    let err = engine.build_pack(&idx, &q).unwrap_err();
    assert!(
        matches!(err, ContextError::StaleContent(_)),
        "expected StaleContent, got {err:?}"
    );
}

#[test]
fn freshness_mismatch_is_refused() {
    let corpus = build_corpus();
    let idx = corpus.index();
    let mut q = query("resolve", &["resolve"]);
    q.freshness = Freshness::RequireRevision(crate::types::Revision("other-rev".to_string()));
    let mut engine = ContextEngine::new();
    let err = engine.build_pack(&idx, &q).unwrap_err();
    assert!(matches!(err, ContextError::FreshnessMismatch { .. }));
}

#[test]
fn deleted_file_after_index_fails_closed() {
    let corpus = build_corpus();
    let idx = corpus.index();
    let q = query("resolve repository identity", &["resolve"]);
    fs::remove_file(corpus.path().join("src/repo.rs")).unwrap();
    let mut engine = ContextEngine::new();
    assert!(
        engine.build_pack(&idx, &q).is_err(),
        "deleted file must fail closed"
    );
}

#[test]
fn malformed_source_does_not_crash_index() {
    let corpus = build_corpus();
    fs::write(
        corpus.path().join("src/broken.rs"),
        "pub fn oops( {{{ not rust",
    )
    .unwrap();
    let idx = corpus.index(); // must not panic
    assert!(
        idx.events()
            .iter()
            .any(|e| matches!(e, ContextEvent::Denied(m) if m.contains("broken.rs"))),
        "expected a parse-denial event for the malformed file"
    );
    // valid symbols are still indexed
    assert!(!idx.by_name("resolve").is_empty());
}

#[test]
fn ambiguous_symbol_names_do_not_invent_edges() {
    let corpus = build_corpus();
    let idx = corpus.index();
    // `new` exists in both util.rs (Unrelated) and factory.rs (Widget)
    assert!(idx.by_name("new").len() >= 2, "expected an ambiguous name");
    // no Calls/References edge was created targeting an ambiguous `new`
    let new_ids: BTreeSet<_> = idx.by_name("new").iter().cloned().collect();
    for edge in idx.graph().edges() {
        if new_ids.contains(&edge.to) {
            assert_eq!(
                edge.kind,
                EdgeKind::Contains,
                "ambiguous name should only have structural (Contains) edges, not invented refs"
            );
        }
    }
}
