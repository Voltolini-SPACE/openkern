//! Strongly-typed identifiers.
//!
//! Each id is a distinct type, so a [`MissionId`] can never be passed where an
//! [`AgentId`] is expected. Ids are opaque, non-empty strings; [`generate`] produces a
//! process-unique value without any third-party dependency.
//!
//! [`generate`]: MissionId::generate

use core::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Error constructing a typed id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    /// The supplied value was empty or whitespace-only.
    Empty,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdError::Empty => f.write_str("identifier must not be empty"),
        }
    }
}

impl std::error::Error for IdError {}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A process-unique suffix built from wall-clock nanoseconds, the process id, and a
/// monotonic counter. Not cryptographic; sufficient to keep local ids distinct.
fn unique_suffix() -> String {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let pid = std::process::id();
    format!("{nanos:x}-{pid:x}-{n:x}")
}

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident, $prefix:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// Wrap an existing value. Fails if empty or whitespace-only.
            pub fn new(value: impl Into<String>) -> Result<Self, IdError> {
                let v = value.into();
                if v.trim().is_empty() {
                    return Err(IdError::Empty);
                }
                Ok(Self(v))
            }

            /// Generate a fresh, process-unique id with a stable prefix.
            #[must_use]
            pub fn generate() -> Self {
                Self(format!("{}-{}", $prefix, unique_suffix()))
            }

            /// Borrow the underlying string.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

define_id!(
    /// Identifies a single mission.
    MissionId, "msn"
);
define_id!(
    /// Identifies an agent (the actor a mission runs on behalf of).
    AgentId, "agt"
);
define_id!(
    /// Identifies a capability grant.
    CapabilityId, "cap"
);
define_id!(
    /// Identifies a repository.
    RepositoryId, "repo"
);
define_id!(
    /// Identifies a worktree within a repository.
    WorktreeId, "wt"
);
define_id!(
    /// Identifies a transactional unit of Git mutation.
    TransactionId, "txn"
);
define_id!(
    /// Identifies a single governed operation.
    OperationId, "op"
);
define_id!(
    /// Identifies a piece of recorded evidence.
    EvidenceId, "ev"
);
define_id!(
    /// Identifies a model/provider at an integration edge.
    ProviderId, "prov"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert_eq!(MissionId::new(""), Err(IdError::Empty));
        assert_eq!(AgentId::new("   "), Err(IdError::Empty));
    }

    #[test]
    fn accepts_and_borrows() {
        let m = MissionId::new("m-1").unwrap();
        assert_eq!(m.as_str(), "m-1");
        assert_eq!(m.to_string(), "m-1");
    }

    #[test]
    fn generate_is_unique_and_prefixed() {
        let a = MissionId::generate();
        let b = MissionId::generate();
        assert_ne!(a, b);
        assert!(a.as_str().starts_with("msn-"));
    }

    #[test]
    fn ids_are_distinct_types() {
        // This is a compile-time guarantee; the runtime check just exercises both.
        let m = MissionId::new("x").unwrap();
        let a = AgentId::new("x").unwrap();
        assert_eq!(m.as_str(), a.as_str());
        // `m == a` would not compile — different types.
    }
}
