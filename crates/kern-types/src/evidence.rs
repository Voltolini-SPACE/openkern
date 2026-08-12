//! Evidence records.
//!
//! Every meaningful effect (or refusal) produces an [`Evidence`] value. Evidence is what
//! lets a report point to *what happened* rather than assert that something happened.

use crate::id::EvidenceId;

/// The kind of effect an [`Evidence`] records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceKind {
    /// A subprocess that ran (program + argv + exit status captured in the payload).
    Command,
    /// A test or check result.
    Test,
    /// Observed Git state (e.g. a `HEAD` or `status` snapshot).
    GitState,
    /// A denial: authority was requested and refused.
    Denial,
}

/// A single recorded fact about an effect or refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    id: EvidenceId,
    kind: EvidenceKind,
    label: String,
    payload: String,
}

impl Evidence {
    /// Record a new piece of evidence.
    #[must_use]
    pub fn new(kind: EvidenceKind, label: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            id: EvidenceId::generate(),
            kind,
            label: label.into(),
            payload: payload.into(),
        }
    }

    /// The evidence id.
    #[must_use]
    pub fn id(&self) -> &EvidenceId {
        &self.id
    }

    /// The evidence kind.
    #[must_use]
    pub fn kind(&self) -> &EvidenceKind {
        &self.kind
    }

    /// A short human label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The full payload (command line, output excerpt, reason, ...).
    #[must_use]
    pub fn payload(&self) -> &str {
        &self.payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_fields() {
        let e = Evidence::new(EvidenceKind::Denial, "denied", "capability expired");
        assert_eq!(e.kind(), &EvidenceKind::Denial);
        assert_eq!(e.label(), "denied");
        assert_eq!(e.payload(), "capability expired");
        assert!(e.id().as_str().starts_with("ev-"));
    }
}
