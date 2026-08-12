//! Deterministic benchmark: `OpenKern` vs three baselines (G8.12, §25–28).
//!
//! Corpus and ground truth are explicit. Metrics: Recall@K, Precision@K, MRR, context
//! bytes, estimated tokens, and UTR (Useful Token Ratio = tokens of relevant symbols
//! covered / total selected tokens). Superiority criteria (§28) are asserted, not assumed.
#![allow(clippy::cast_precision_loss)]

use std::collections::{BTreeMap, BTreeSet};

use kern_types::{MissionId, RepositoryId, WorktreeId};

use crate::engine::{estimate_tokens, ContextEngine};
use crate::fixture::{build_corpus, Corpus};
use crate::index::SymbolIndex;
use crate::scoring::{lexical_overlap, name_match, tokenize};
use crate::types::{ContextBudget, ContextQuery, Freshness, SourceKind, SymbolId, SymbolKind};

const K: usize = 8;

struct Task {
    name: &'static str,
    text: &'static str,
    seeds: Vec<&'static str>,
    relevant: BTreeSet<SymbolId>,
}

/// One baseline's selection: ordered items, each covering a set of symbol ids.
struct Selection {
    items: Vec<SelItem>,
}
struct SelItem {
    covers: BTreeSet<SymbolId>,
    tokens: usize,
    bytes: usize,
}

struct Metrics {
    recall: f64,
    precision: f64,
    mrr: f64,
    tokens: usize,
    bytes: usize,
    utr: f64,
}

/// Snippet tokens per symbol id (used for the UTR numerator).
fn symbol_tokens(index: &SymbolIndex) -> BTreeMap<SymbolId, usize> {
    let mut m = BTreeMap::new();
    for sym in index.symbols() {
        if let Some(rec) = index.files().get(&sym.file) {
            let start = sym.span.start_line.saturating_sub(1);
            let end = sym.span.end_line.min(rec.lines.len());
            let slice = rec.lines.get(start..end).unwrap_or(&[]).join("\n");
            m.insert(sym.id.clone(), estimate_tokens(&slice));
        }
    }
    m
}

fn resolve_relevant(index: &SymbolIndex, name: &str, kind: Option<SymbolKind>) -> Vec<SymbolId> {
    index
        .symbols()
        .iter()
        .filter(|s| s.name == name && kind.is_none_or(|k| s.kind == k))
        .map(|s| s.id.clone())
        .collect()
}

fn metrics(sel: &Selection, task: &Task, tok: &BTreeMap<SymbolId, usize>) -> Metrics {
    let mut covered: BTreeSet<SymbolId> = BTreeSet::new();
    for it in &sel.items {
        covered.extend(it.covers.iter().cloned());
    }
    let hit: BTreeSet<SymbolId> = task.relevant.intersection(&covered).cloned().collect();
    let recall = if task.relevant.is_empty() {
        0.0
    } else {
        hit.len() as f64 / task.relevant.len() as f64
    };
    let relevant_items = sel
        .items
        .iter()
        .filter(|it| !it.covers.is_disjoint(&task.relevant))
        .count();
    let precision = if sel.items.is_empty() {
        0.0
    } else {
        relevant_items as f64 / sel.items.len() as f64
    };
    let mrr = sel
        .items
        .iter()
        .position(|it| !it.covers.is_disjoint(&task.relevant))
        .map_or(0.0, |r| 1.0 / (r as f64 + 1.0));
    let tokens: usize = sel.items.iter().map(|it| it.tokens).sum();
    let bytes: usize = sel.items.iter().map(|it| it.bytes).sum();
    let useful: usize = hit.iter().map(|s| tok.get(s).copied().unwrap_or(0)).sum();
    let utr = if tokens == 0 {
        0.0
    } else {
        useful as f64 / tokens as f64
    };
    Metrics {
        recall,
        precision,
        mrr,
        tokens,
        bytes,
        utr,
    }
}

// ---- baselines ----

/// B0: naive — every indexed file, whole content.
fn b0_full_files(index: &SymbolIndex) -> Selection {
    let mut items = Vec::new();
    for (fid, rec) in index.files() {
        let covers: BTreeSet<SymbolId> = index
            .symbols()
            .iter()
            .filter(|s| &s.file == fid)
            .map(|s| s.id.clone())
            .collect();
        items.push(SelItem {
            covers,
            tokens: estimate_tokens(&rec.source),
            bytes: rec.source.len(),
        });
    }
    Selection { items }
}

/// B1: lexical-only — rank symbols by lexical overlap of the query over the snippet text.
fn b1_lexical(index: &SymbolIndex, task: &Task, tok: &BTreeMap<SymbolId, usize>) -> Selection {
    let q = tokenize(task.text);
    let mut scored: Vec<(f64, &SymbolId)> = Vec::new();
    for sym in index.symbols() {
        if let Some(rec) = index.files().get(&sym.file) {
            let start = sym.span.start_line.saturating_sub(1);
            let end = sym.span.end_line.min(rec.lines.len());
            let slice = rec.lines.get(start..end).unwrap_or(&[]).join("\n");
            let s = lexical_overlap(&q, &slice);
            if s > 0.0 {
                scored.push((s, &sym.id));
            }
        }
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then_with(|| a.1.cmp(b.1)));
    top_k(scored, index, tok)
}

/// B2: symbol-name-only — rank by name match.
fn b2_symbol(index: &SymbolIndex, task: &Task, tok: &BTreeMap<SymbolId, usize>) -> Selection {
    let q = tokenize(task.text);
    let mut scored: Vec<(f64, &SymbolId)> = Vec::new();
    for sym in index.symbols() {
        let s = name_match(&q, &sym.name);
        if s > 0.0 {
            scored.push((s, &sym.id));
        }
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then_with(|| a.1.cmp(b.1)));
    top_k(scored, index, tok)
}

fn top_k(
    scored: Vec<(f64, &SymbolId)>,
    index: &SymbolIndex,
    tok: &BTreeMap<SymbolId, usize>,
) -> Selection {
    let mut items = Vec::new();
    for (_, sid) in scored.into_iter().take(K) {
        let mut covers = BTreeSet::new();
        covers.insert(sid.clone());
        let tokens = tok.get(sid).copied().unwrap_or(0);
        let bytes = index.symbol(sid).map_or(0, |s| {
            // approximate bytes from tokens*4 lower bound is avoided; use snippet length
            index.files().get(&s.file).map_or(0, |rec| {
                let start = s.span.start_line.saturating_sub(1);
                let end = s.span.end_line.min(rec.lines.len());
                rec.lines.get(start..end).unwrap_or(&[]).join("\n").len()
            })
        });
        items.push(SelItem {
            covers,
            tokens,
            bytes,
        });
    }
    Selection { items }
}

/// B3: the `OpenKern` engine.
fn b3_engine(index: &SymbolIndex, task: &Task) -> Selection {
    let mut allowed = BTreeSet::new();
    allowed.insert(SourceKind::Symbol);
    let q = ContextQuery {
        mission: MissionId::new("m-bench").unwrap(),
        repository: RepositoryId::new("repo-A").unwrap(),
        worktree: WorktreeId::new("wt-A").unwrap(),
        text: task.text.to_string(),
        seed_paths: Vec::new(),
        seed_symbols: task.seeds.iter().map(|s| (*s).to_string()).collect(),
        budget: ContextBudget {
            max_items: K,
            ..ContextBudget::default()
        },
        max_depth: 2,
        allowed_sources: allowed,
        freshness: Freshness::AnyRevision,
    };
    let mut engine = ContextEngine::new();
    let pack = engine.build_pack(index, &q).expect("pack builds");
    let items = pack
        .items
        .iter()
        .map(|it| {
            let mut covers = BTreeSet::new();
            if let Some(s) = &it.provenance.symbol {
                covers.insert(s.clone());
            }
            SelItem {
                covers,
                tokens: it.estimated_tokens,
                bytes: it.bytes,
            }
        })
        .collect();
    Selection { items }
}

fn tasks(index: &SymbolIndex) -> Vec<Task> {
    let mut r1 = BTreeSet::new();
    r1.extend(resolve_relevant(
        index,
        "RepositoryIdentity",
        Some(SymbolKind::Struct),
    ));
    r1.extend(resolve_relevant(index, "resolve", None));
    r1.extend(resolve_relevant(index, "head", Some(SymbolKind::Method)));
    r1.extend(resolve_relevant(index, "run_mission", None));

    let mut r2 = BTreeSet::new();
    r2.extend(resolve_relevant(index, "run_mission", None));
    r2.extend(resolve_relevant(index, "resolve", None));
    r2.extend(resolve_relevant(index, "head", Some(SymbolKind::Method)));

    vec![
        Task {
            name: "find symbol + neighbours (resolve repository identity)",
            text: "resolve repository identity",
            seeds: vec!["resolve"],
            relevant: r1,
        },
        Task {
            name: "find callees of run_mission",
            text: "run mission",
            seeds: vec!["run_mission"],
            relevant: r2,
        },
    ]
}

#[test]
fn benchmark_openkern_beats_baselines() {
    let corpus: Corpus = build_corpus();
    let index = corpus.index();
    let tok = symbol_tokens(&index);
    let tasks = tasks(&index);

    let mut sum_recall_b1 = 0.0;
    let mut sum_recall_b3 = 0.0;

    eprintln!("\n=== OpenKern G8 Context Benchmark (frozen fixture corpus) ===");
    eprintln!(
        "{:<44} {:<10} {:>6} {:>6} {:>6} {:>7} {:>6}",
        "task", "baseline", "recall", "prec", "mrr", "tokens", "utr"
    );

    for task in &tasks {
        let b0 = metrics(&b0_full_files(&index), task, &tok);
        let b1 = metrics(&b1_lexical(&index, task, &tok), task, &tok);
        let b2 = metrics(&b2_symbol(&index, task, &tok), task, &tok);
        let b3 = metrics(&b3_engine(&index, task), task, &tok);

        for (label, m) in [
            ("B0-full", &b0),
            ("B1-lexical", &b1),
            ("B2-symbol", &b2),
            ("B3-openkern", &b3),
        ] {
            eprintln!(
                "{:<44} {:<10} {:>6.2} {:>6.2} {:>6.2} {:>7} {:>6.2}",
                task.name, label, m.recall, m.precision, m.mrr, m.tokens, m.utr
            );
        }

        // §28 superiority, per task:
        assert!(
            b3.recall + 1e-9 >= b1.recall,
            "B3 recall must be >= B1 lexical ({} vs {})",
            b3.recall,
            b1.recall
        );
        assert!(
            b3.utr > b0.utr,
            "B3 UTR must exceed naive full-file UTR ({} vs {})",
            b3.utr,
            b0.utr
        );
        assert!(
            b3.bytes * 2 <= b0.bytes,
            "B3 context must be materially smaller than full-file ({} vs {})",
            b3.bytes,
            b0.bytes
        );
        // MRR: first relevant item ranked at/near the top.
        assert!(
            b3.mrr >= 0.5,
            "B3 first relevant item should rank high, mrr={}",
            b3.mrr
        );

        sum_recall_b1 += b1.recall;
        sum_recall_b3 += b3.recall;
    }

    // Aggregate: strict improvement in recall over the lexical baseline somewhere.
    assert!(
        sum_recall_b3 > sum_recall_b1 + 1e-9,
        "B3 must strictly beat lexical recall in aggregate ({sum_recall_b3} vs {sum_recall_b1})"
    );
    eprintln!("aggregate recall: B3={sum_recall_b3:.2}  B1={sum_recall_b1:.2}\n");
}

#[test]
fn benchmark_is_deterministic_and_leak_free() {
    let corpus = build_corpus();
    let index = corpus.index();
    let task = &tasks(&index)[0];

    // determinism: identical pack across runs
    let mut e1 = ContextEngine::new();
    let mut e2 = ContextEngine::new();
    let q = ContextQuery {
        budget: ContextBudget {
            max_items: K,
            ..ContextBudget::default()
        },
        ..base_query(task)
    };
    let p1 = e1.build_pack(&index, &q).unwrap();
    let p2 = e2.build_pack(&index, &q).unwrap();
    assert_eq!(p1.pack_hash, p2.pack_hash, "deterministic replay");

    // no secret material ever appears in a pack
    for it in &p1.items {
        let p = it.provenance.path.to_string_lossy();
        assert!(!p.contains(".env") && !p.contains("secrets/") && !p.ends_with(".pem"));
    }
}

fn base_query(task: &Task) -> ContextQuery {
    let mut allowed = BTreeSet::new();
    allowed.insert(SourceKind::Symbol);
    ContextQuery {
        mission: MissionId::new("m-bench").unwrap(),
        repository: RepositoryId::new("repo-A").unwrap(),
        worktree: WorktreeId::new("wt-A").unwrap(),
        text: task.text.to_string(),
        seed_paths: Vec::new(),
        seed_symbols: task.seeds.iter().map(|s| (*s).to_string()).collect(),
        budget: ContextBudget::default(),
        max_depth: 2,
        allowed_sources: allowed,
        freshness: Freshness::AnyRevision,
    }
}
