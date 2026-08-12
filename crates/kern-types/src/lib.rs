//! `kern-types` — the kernel's value types.
//!
//! Strongly-typed identifiers, the [`Mission`] request, execution [`ExecutionBudget`]s,
//! [`Deadline`]s, [`RiskClass`], and [`Evidence`]. std-only; no third-party dependencies.

pub mod budget;
pub mod evidence;
pub mod id;
pub mod mission;
pub mod risk;
pub mod time;

pub use budget::{BudgetError, ExecutionBudget};
pub use evidence::{Evidence, EvidenceKind};
pub use id::{
    AgentId, CapabilityId, EvidenceId, IdError, MissionId, OperationId, ProviderId, RepositoryId,
    TransactionId, WorktreeId,
};
pub use mission::Mission;
pub use risk::RiskClass;
pub use time::Deadline;
