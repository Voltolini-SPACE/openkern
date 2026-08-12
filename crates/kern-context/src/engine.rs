//! The context engine (G8.3–G8.8, G8.11, G8.14).
//!
//! Given a [`SymbolIndex`] and a [`ContextQuery`], the engine scores candidates
//! deterministically, expands along the dependency graph under a depth bound, selects the
//! highest-scoring items until a [`ContextBudget`] is reached, and emits a [`ContextPack`]
//! with per-item provenance and a canonical hash. Every content item is re-read and
//! re-hashed at pack time against the indexed hash (TOCTOU / stale defense).

use std::fs;

use crate::hash::content_hash;
use crate::index::SymbolIndex;
use crate::scoring::{lexical_overlap, name_match, path_proximity, tokenize};
use crate::security::{safe_join, sensitive_reason};
use crate::types::{
    ContextError, ContextEvent, ContextItem, ContextPack, ContextQuery, Freshness, Provenance,
    ScoreComponents, SourceKind, Symbol, SymbolId,
};
use crate::ENGINE_VERSION;

/// Rough token estimate: bytes / 4 (documented approximation; no tokenizer dependency).
#[must_use]
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// The governed context engine.
#[derive(Debug, Default)]
pub struct ContextEngine {
    events: Vec<ContextEvent>,
}

/// An internal scored candidate before budget selection.
struct Candidate {
    symbol: Symbol,
    score: ScoreComponents,
    content: String,
}

impl ContextEngine {
    /// A fresh engine.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Events emitted by the most recent `build_pack` (secret-free).
    #[must_use]
    pub fn events(&self) -> &[ContextEvent] {
        &self.events
    }

    /// Build a [`ContextPack`] for `query` against `index`. Fail-closed on repository
    /// mismatch, freshness mismatch, path escape, sensitive material, and stale content.
    #[allow(clippy::too_many_lines)] // one cohesive selection pipeline; splitting hurts clarity
    pub fn build_pack(
        &mut self,
        index: &SymbolIndex,
        query: &ContextQuery,
    ) -> Result<ContextPack, ContextError> {
        self.events.clear();
        self.events.push(ContextEvent::QueryStarted);

        // Repository binding (G8.6): the query must target the indexed repository/worktree.
        if &query.repository != index.repository() || &query.worktree != index.worktree() {
            self.events
                .push(ContextEvent::Denied("repository/worktree mismatch".into()));
            return Err(ContextError::RepositoryMismatch);
        }

        // Freshness (G8.11 / part of TOCTOU): if a revision is required, it must match.
        if let Freshness::RequireRevision(req) = &query.freshness {
            if req != index.revision() {
                return Err(ContextError::FreshnessMismatch {
                    required: req.as_str().to_string(),
                    actual: index.revision().as_str().to_string(),
                });
            }
        }

        let q_tokens = tokenize(&query.text);
        let depth = query.max_depth.min(query.budget.max_depth);

        // Seeds: explicit symbol names + names matching the query text.
        let seeds = Self::resolve_seeds(index, query, &q_tokens);
        let distances = index.graph().bounded_distances(&seeds, depth);

        // Score every symbol.
        let mut candidates: Vec<Candidate> = Vec::new();
        for sym in index.symbols() {
            // Only symbol source is used here; SourceKind::Symbol must be allowed.
            if !query.allowed_sources.contains(&SourceKind::Symbol) {
                break;
            }
            let path_str = sym.path.to_string_lossy();
            let name_s = name_match(&q_tokens, &sym.name);
            let qualified_lex = lexical_overlap(&q_tokens, &sym.qualified);
            let dep = distances
                .get(&sym.id)
                .map_or(0.0, |d| 1.0 / (1.0 + f64::from(*d)));
            let pathp = path_proximity(&query.seed_paths, &path_str);
            let explicit = f64::from(u8::from(
                query.seed_symbols.iter().any(|s| s == &sym.name)
                    || query.seed_paths.iter().any(|p| p == &path_str),
            ));

            let score = ScoreComponents {
                lexical: qualified_lex,
                symbol_match: name_s,
                dependency: dep,
                path_proximity: pathp,
                explicit,
                test_relevance: test_relevance(sym),
            };

            // Drop candidates with no signal at all (keeps precision up; deterministic).
            if score.total() <= 0.0 {
                continue;
            }

            let (content, _current_hash) = Self::symbol_snippet(index, sym)?;
            self.events
                .push(ContextEvent::CandidateScored(sym.qualified.clone()));
            candidates.push(Candidate {
                symbol: sym.clone(),
                score,
                content,
            });
        }

        let candidates_considered = candidates.len();

        // Deterministic ordering: score desc, then symbol id asc as a stable tiebreak.
        candidates.sort_by(|a, b| {
            b.score
                .total()
                .partial_cmp(&a.score.total())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.symbol.id.cmp(&b.symbol.id))
        });

        // Budget-bounded selection (G8.5).
        let mut items: Vec<ContextItem> = Vec::new();
        let mut total_bytes = 0usize;
        let mut total_tokens = 0usize;
        for c in candidates {
            if items.len() >= query.budget.max_items {
                self.events.push(ContextEvent::ItemRejected(format!(
                    "{}: max_items",
                    c.symbol.qualified
                )));
                continue;
            }
            let bytes = c.content.len();
            let tokens = estimate_tokens(&c.content);
            if total_bytes + bytes > query.budget.max_bytes
                || total_tokens + tokens > query.budget.max_estimated_tokens
            {
                self.events.push(ContextEvent::ItemRejected(format!(
                    "{}: budget",
                    c.symbol.qualified
                )));
                continue;
            }

            // TOCTOU / stale re-verification at pack time (G8.8): re-read and re-hash, and
            // compare against the INDEX-TIME hash. If the file changed since indexing, the
            // span no longer identifies the same bytes -> refuse.
            let (fresh_content, fresh_hash) = Self::symbol_snippet(index, &c.symbol)?;
            if fresh_hash != c.symbol.content_hash.0 {
                self.events.push(ContextEvent::StaleDetected(
                    c.symbol.path.to_string_lossy().into_owned(),
                ));
                return Err(ContextError::StaleContent(
                    c.symbol.path.to_string_lossy().into_owned(),
                ));
            }

            let provenance = Provenance {
                source: SourceKind::Symbol,
                file: c.symbol.file.clone(),
                path: c.symbol.path.clone(),
                revision: c.symbol.revision.clone(),
                symbol: Some(c.symbol.id.clone()),
                content_hash: c.symbol.content_hash.clone(),
            };
            // Provenance completeness (G8.7): refuse an item whose provenance is empty.
            if provenance.content_hash.0.is_empty() || provenance.file.0.is_empty() {
                return Err(ContextError::MissingProvenance);
            }

            let reason = selection_reason(&c.score, &c.symbol.id, &distances);
            total_bytes += bytes;
            total_tokens += tokens;
            self.events
                .push(ContextEvent::ItemSelected(c.symbol.qualified.clone()));
            items.push(ContextItem {
                content: fresh_content,
                provenance,
                score: c.score,
                estimated_tokens: tokens,
                bytes,
                reason,
            });
        }

        let pack_hash = canonical_hash(query, &items);
        let pack_id = format!("pack:{pack_hash}");
        self.events.push(ContextEvent::PackCreated(pack_id.clone()));

        Ok(ContextPack {
            pack_id,
            mission_id: query.mission.clone(),
            repository_id: query.repository.clone(),
            worktree_id: query.worktree.clone(),
            revision: index.revision().clone(),
            query_text: query.text.clone(),
            items,
            total_bytes,
            total_estimated_tokens: total_tokens,
            budget: query.budget,
            candidates_considered,
            engine_version: ENGINE_VERSION.to_string(),
            pack_hash,
        })
    }

    fn resolve_seeds(
        index: &SymbolIndex,
        query: &ContextQuery,
        q_tokens: &[String],
    ) -> Vec<SymbolId> {
        let mut seeds: Vec<SymbolId> = Vec::new();
        for name in &query.seed_symbols {
            seeds.extend_from_slice(index.by_name(name));
        }
        // Names whose match strength is high become seeds too.
        for sym in index.symbols() {
            if name_match(q_tokens, &sym.name) >= 1.0 {
                seeds.push(sym.id.clone());
            }
        }
        seeds.sort();
        seeds.dedup();
        seeds
    }

    /// Re-read a symbol's source slice from disk under worktree containment + sensitivity
    /// checks, returning (content, hash). This is the single read path, so every read is
    /// re-validated (TOCTOU-safe).
    fn symbol_snippet(index: &SymbolIndex, sym: &Symbol) -> Result<(String, String), ContextError> {
        let rel = sym.path.to_string_lossy();
        if let Some(reason) = sensitive_reason(&sym.path) {
            return Err(ContextError::SensitiveMaterial(format!("{rel}: {reason}")));
        }
        let abs = safe_join(index.worktree_path(), &rel)?;
        let source = fs::read_to_string(&abs).map_err(|e| ContextError::Io(e.to_string()))?;
        let lines: Vec<&str> = source.lines().collect();
        let start = sym.span.start_line.saturating_sub(1);
        let end = sym.span.end_line.min(lines.len());
        let slice = lines.get(start..end).unwrap_or(&[]).join("\n");
        let hash = content_hash(slice.as_bytes());
        Ok((slice, hash))
    }
}

fn test_relevance(sym: &Symbol) -> f64 {
    let p = sym.path.to_string_lossy();
    if p.contains("test") || sym.qualified.contains("test") {
        0.5
    } else {
        0.0
    }
}

fn selection_reason(
    score: &ScoreComponents,
    id: &SymbolId,
    distances: &std::collections::BTreeMap<SymbolId, u32>,
) -> String {
    let mut parts = Vec::new();
    if score.explicit > 0.0 {
        parts.push("explicit-seed".to_string());
    }
    if score.symbol_match > 0.0 {
        parts.push(format!("name-match={:.2}", score.symbol_match));
    }
    if let Some(d) = distances.get(id) {
        parts.push(format!("graph-distance={d}"));
    }
    if score.lexical > 0.0 {
        parts.push(format!("lexical={:.2}", score.lexical));
    }
    if score.test_relevance > 0.0 {
        parts.push("test-related".to_string());
    }
    if parts.is_empty() {
        "low-signal".to_string()
    } else {
        parts.join(", ")
    }
}

/// Canonical, deterministic hash of a pack's *selection* (excludes non-deterministic fields
/// like timestamps; `pack_id` is derived from this, so it is not an input).
fn canonical_hash(query: &ContextQuery, items: &[ContextItem]) -> String {
    let mut buf = String::new();
    buf.push_str(ENGINE_VERSION);
    buf.push('\n');
    buf.push_str(query.mission.as_str());
    buf.push('\n');
    buf.push_str(query.repository.as_str());
    buf.push('\n');
    buf.push_str(query.worktree.as_str());
    buf.push('\n');
    buf.push_str(&query.text);
    buf.push('\n');
    for it in items {
        buf.push_str(&it.provenance.path.to_string_lossy());
        buf.push('|');
        if let Some(s) = &it.provenance.symbol {
            buf.push_str(&s.0);
        }
        buf.push('|');
        buf.push_str(&it.provenance.content_hash.0);
        buf.push('\n');
    }
    content_hash(buf.as_bytes())
}
