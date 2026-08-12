//! Risk classification.

/// The severity class of a mission or operation, ordered from least to most dangerous.
///
/// The ordering is meaningful: `ReadOnly < Mutating < Executing < Destructive`, so policy
/// can compare a request against a ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RiskClass {
    /// Reads only; no state change.
    ReadOnly,
    /// Mutates repository content but is reversible in-repo.
    Mutating,
    /// Runs a subprocess (build, test, tool).
    Executing,
    /// Irreversible or out-of-repo effects (history rewrite, force push, `clean -fdx`).
    Destructive,
}

impl RiskClass {
    /// True when `self` is at most as dangerous as `ceiling`.
    #[must_use]
    pub fn within(self, ceiling: RiskClass) -> bool {
        self <= ceiling
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_meaningful() {
        assert!(RiskClass::ReadOnly < RiskClass::Mutating);
        assert!(RiskClass::Mutating < RiskClass::Executing);
        assert!(RiskClass::Executing < RiskClass::Destructive);
    }

    #[test]
    fn within_ceiling() {
        assert!(RiskClass::ReadOnly.within(RiskClass::Executing));
        assert!(!RiskClass::Destructive.within(RiskClass::Executing));
    }
}
