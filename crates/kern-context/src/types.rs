//! Context value types (G8.0).
//!
//! Every selectable piece of context is typed, repository-bound, and carries provenance.
//! A [`ContextPack`] is the bounded, deterministic, explainable output of the engine.

use std::collections::BTreeSet;
use std::path::PathBuf;

use kern_types::{MissionId, RepositoryId, WorktreeId};

/// A repository revision (a `HEAD` oid, or the null oid for unborn).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Revision(pub String);

impl Revision {
    /// Borrow the revision string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A content hash string (see [`crate::hash`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash(pub String);

/// A stable file identifier (repository + worktree-relative path).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub String);

/// A stable symbol identifier, stable within a revision.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SymbolId(pub String);

/// A line/column span within a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// 1-based start line.
    pub start_line: usize,
    /// 1-based end line.
    pub end_line: usize,
}

/// The kind of a source symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKind {
    /// A module.
    Module,
    /// A struct.
    Struct,
    /// An enum.
    Enum,
    /// A trait.
    Trait,
    /// An `impl` block.
    Impl,
    /// A free function.
    Function,
    /// A method (function inside an impl).
    Method,
    /// A `const`.
    Const,
    /// A `static`.
    Static,
    /// A `type` alias.
    TypeAlias,
    /// A `use` import.
    Use,
}

/// Which source produced a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SourceKind {
    /// A source file (or a slice of one).
    File,
    /// A symbol from the index.
    Symbol,
    /// A test relationship.
    Test,
    /// Git status/diff.
    GitStatus,
}

/// A resolved source symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    /// Stable id.
    pub id: SymbolId,
    /// Kind.
    pub kind: SymbolKind,
    /// Simple name.
    pub name: String,
    /// Fully-qualified name (`module::path::name`).
    pub qualified: String,
    /// Owning file.
    pub file: FileId,
    /// Worktree-relative path.
    pub path: PathBuf,
    /// Line span.
    pub span: Span,
    /// Owning repository.
    pub repository: RepositoryId,
    /// Owning worktree.
    pub worktree: WorktreeId,
    /// Revision this symbol was indexed at.
    pub revision: Revision,
    /// Hash of the symbol's source slice.
    pub content_hash: ContentHash,
}

/// A relationship between two symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EdgeKind {
    /// A container defines a member.
    Defines,
    /// A file imports a path.
    Imports,
    /// An impl implements a trait/type.
    Implements,
    /// A symbol calls another.
    Calls,
    /// A symbol references another by name.
    References,
    /// A container contains a symbol.
    Contains,
    /// The relationship could not be resolved precisely.
    Unknown,
}

/// A directed edge in the dependency graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DependencyEdge {
    /// Source symbol.
    pub from: SymbolId,
    /// Target symbol.
    pub to: SymbolId,
    /// Relationship kind.
    pub kind: EdgeKind,
}

/// An explicit, bounded context budget. Absence of a limit is not unlimited — a builder
/// default provides safe ceilings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextBudget {
    /// Maximum total content bytes.
    pub max_bytes: usize,
    /// Maximum number of items.
    pub max_items: usize,
    /// Maximum estimated tokens.
    pub max_estimated_tokens: usize,
    /// Maximum traversal depth.
    pub max_depth: u32,
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024,
            max_items: 40,
            max_estimated_tokens: 16_000,
            max_depth: 3,
        }
    }
}

/// Freshness requirement for a query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Freshness {
    /// Any indexed revision is acceptable.
    AnyRevision,
    /// The index must match this revision or be refused/reindexed.
    RequireRevision(Revision),
}

/// A typed context query. Never a bare string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextQuery {
    /// The mission this query serves.
    pub mission: MissionId,
    /// The repository targeted.
    pub repository: RepositoryId,
    /// The worktree targeted.
    pub worktree: WorktreeId,
    /// Free-text query.
    pub text: String,
    /// Explicit seed paths.
    pub seed_paths: Vec<String>,
    /// Explicit seed symbol names.
    pub seed_symbols: Vec<String>,
    /// Budget.
    pub budget: ContextBudget,
    /// Traversal depth (bounded by budget's `max_depth` too).
    pub max_depth: u32,
    /// Which source kinds are permitted.
    pub allowed_sources: BTreeSet<SourceKind>,
    /// Freshness requirement.
    pub freshness: Freshness,
}

impl ContextQuery {
    /// A minimal free-text query over symbols and files with default budget/freshness.
    #[must_use]
    pub fn new(
        mission: MissionId,
        repository: RepositoryId,
        worktree: WorktreeId,
        text: impl Into<String>,
    ) -> Self {
        let mut allowed = BTreeSet::new();
        allowed.insert(SourceKind::Symbol);
        allowed.insert(SourceKind::File);
        Self {
            mission,
            repository,
            worktree,
            text: text.into(),
            seed_paths: Vec::new(),
            seed_symbols: Vec::new(),
            budget: ContextBudget::default(),
            max_depth: 2,
            allowed_sources: allowed,
            freshness: Freshness::AnyRevision,
        }
    }
}

/// The breakdown of a candidate's score. All components are deterministic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreComponents {
    /// Lexical overlap with the query text.
    pub lexical: f64,
    /// Symbol-name match strength.
    pub symbol_match: f64,
    /// Inverse dependency-graph distance from a seed.
    pub dependency: f64,
    /// Path proximity to seed paths.
    pub path_proximity: f64,
    /// Explicit seed reference bonus.
    pub explicit: f64,
    /// Test-relationship relevance.
    pub test_relevance: f64,
}

impl ScoreComponents {
    /// Weighted total. Weights are fixed for determinism.
    #[must_use]
    pub fn total(&self) -> f64 {
        3.0 * self.explicit
            + 2.0 * self.symbol_match
            + 1.5 * self.dependency
            + 1.0 * self.lexical
            + 0.5 * self.path_proximity
            + 0.75 * self.test_relevance
    }
}

/// Why a candidate exists and where it came from. Content without provenance may not enter
/// a trusted pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// The producing source.
    pub source: SourceKind,
    /// The file it came from.
    pub file: FileId,
    /// The worktree-relative path.
    pub path: PathBuf,
    /// The revision.
    pub revision: Revision,
    /// The symbol, if symbol-derived.
    pub symbol: Option<SymbolId>,
    /// The content hash.
    pub content_hash: ContentHash,
}

/// A selected piece of context in a pack.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextItem {
    /// The content text.
    pub content: String,
    /// Where it came from.
    pub provenance: Provenance,
    /// Its score breakdown.
    pub score: ScoreComponents,
    /// Estimated tokens.
    pub estimated_tokens: usize,
    /// Byte size.
    pub bytes: usize,
    /// Human-readable selection reason.
    pub reason: String,
}

/// The bounded, deterministic, explainable output of the engine.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextPack {
    /// Deterministic pack id (derived from the canonical hash).
    pub pack_id: String,
    /// Mission.
    pub mission_id: MissionId,
    /// Repository.
    pub repository_id: RepositoryId,
    /// Worktree.
    pub worktree_id: WorktreeId,
    /// Revision.
    pub revision: Revision,
    /// A short echo of the query text.
    pub query_text: String,
    /// The selected items, in deterministic order.
    pub items: Vec<ContextItem>,
    /// Total bytes across items.
    pub total_bytes: usize,
    /// Total estimated tokens.
    pub total_estimated_tokens: usize,
    /// The budget that shaped this pack.
    pub budget: ContextBudget,
    /// Number of candidates considered.
    pub candidates_considered: usize,
    /// Engine version string.
    pub engine_version: String,
    /// The canonical content hash of the pack (excludes non-deterministic fields).
    pub pack_hash: String,
}

/// A typed observability event. Never carries raw secret material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextEvent {
    /// Indexing started for a repository.
    IndexStarted,
    /// Indexing finished with a symbol count.
    IndexCompleted(usize),
    /// A query began.
    QueryStarted,
    /// A candidate was scored (label only, no content).
    CandidateScored(String),
    /// An item was selected.
    ItemSelected(String),
    /// A candidate was rejected with a reason.
    ItemRejected(String),
    /// A pack was created (id).
    PackCreated(String),
    /// Stale content was detected for a path.
    StaleDetected(String),
    /// Access was denied for a reason.
    Denied(String),
}

/// Errors from the context engine.
#[derive(Debug)]
pub enum ContextError {
    /// A path escaped the worktree (traversal, absolute, or symlink).
    PathEscape(String),
    /// A path pointed at sensitive/secret material and was refused.
    SensitiveMaterial(String),
    /// A candidate lacked provenance.
    MissingProvenance,
    /// Indexed content changed since indexing (TOCTOU / stale).
    StaleContent(String),
    /// The query targeted a different repository than the index.
    RepositoryMismatch,
    /// The freshness requirement was not met.
    FreshnessMismatch {
        /// Required revision.
        required: String,
        /// Index revision.
        actual: String,
    },
    /// An I/O error.
    Io(String),
    /// A source file could not be parsed.
    Parse(String),
}

impl core::fmt::Display for ContextError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ContextError::PathEscape(p) => write!(f, "path escapes worktree: {p}"),
            ContextError::SensitiveMaterial(p) => write!(f, "sensitive material refused: {p}"),
            ContextError::MissingProvenance => f.write_str("candidate lacks provenance"),
            ContextError::StaleContent(p) => write!(f, "stale content: {p}"),
            ContextError::RepositoryMismatch => f.write_str("query/index repository mismatch"),
            ContextError::FreshnessMismatch { required, actual } => {
                write!(f, "freshness mismatch: required {required}, index {actual}")
            }
            ContextError::Io(e) => write!(f, "io error: {e}"),
            ContextError::Parse(e) => write!(f, "parse error: {e}"),
        }
    }
}

impl std::error::Error for ContextError {}
