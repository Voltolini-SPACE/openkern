//! `kern-context` — `OpenKern`'s governed context engine (G8).
//!
//! Discovers, selects, structures, and packages the *smallest sufficient* context for a
//! mission. Deterministic, bounded, provenance-carrying, repository/mission-bound, and
//! provider-independent (no LLM, no network egress). **Context is not authority**: a symbol
//! or file appearing in a [`ContextPack`] confers no permission to touch or run anything —
//! execution still flows through policy → capability → identity → typed exec / transactional git.

pub mod engine;
pub mod graph;
pub mod hash;
pub mod index;
pub mod scoring;
pub mod security;
pub mod types;

/// The engine version stamped into every pack (bump on any change that affects selection).
pub const ENGINE_VERSION: &str = "kern-context/0.1.0";

pub use engine::ContextEngine;
pub use index::{FileRecord, SymbolIndex};

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests_adversarial;
#[cfg(test)]
mod tests_benchmark;
pub use types::{
    ContentHash, ContextBudget, ContextError, ContextEvent, ContextItem, ContextPack, ContextQuery,
    DependencyEdge, EdgeKind, FileId, Freshness, Provenance, Revision, ScoreComponents, SourceKind,
    Span, Symbol, SymbolId, SymbolKind,
};
