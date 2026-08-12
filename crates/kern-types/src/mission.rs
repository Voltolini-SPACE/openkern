//! The `Mission` — a unit of authorized work.

use crate::budget::ExecutionBudget;
use crate::id::{AgentId, MissionId};
use crate::risk::RiskClass;
use crate::time::Deadline;

/// A unit of work an agent wants to perform, before any authority is granted.
///
/// A mission is a *request*, not permission. Policy and capability decide whether — and
/// how narrowly — it may proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mission {
    id: MissionId,
    agent: AgentId,
    intent: String,
    risk: RiskClass,
    budget: ExecutionBudget,
    deadline: Option<Deadline>,
}

impl Mission {
    /// Create a mission request.
    #[must_use]
    pub fn new(
        id: MissionId,
        agent: AgentId,
        intent: impl Into<String>,
        risk: RiskClass,
        budget: ExecutionBudget,
    ) -> Self {
        Self {
            id,
            agent,
            intent: intent.into(),
            risk,
            budget,
            deadline: None,
        }
    }

    /// Attach a deadline.
    #[must_use]
    pub fn with_deadline(mut self, deadline: Deadline) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// The mission id.
    #[must_use]
    pub fn id(&self) -> &MissionId {
        &self.id
    }

    /// The agent on whose behalf the mission runs.
    #[must_use]
    pub fn agent(&self) -> &AgentId {
        &self.agent
    }

    /// Free-text intent.
    #[must_use]
    pub fn intent(&self) -> &str {
        &self.intent
    }

    /// The declared risk ceiling of the mission.
    #[must_use]
    pub fn risk(&self) -> RiskClass {
        self.risk
    }

    /// The execution budget.
    #[must_use]
    pub fn budget(&self) -> &ExecutionBudget {
        &self.budget
    }

    /// The optional deadline.
    #[must_use]
    pub fn deadline(&self) -> Option<Deadline> {
        self.deadline
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn sample() -> Mission {
        Mission::new(
            MissionId::new("m1").unwrap(),
            AgentId::new("a1").unwrap(),
            "add a test",
            RiskClass::Mutating,
            ExecutionBudget::new(4, 1024, Duration::from_secs(30)),
        )
    }

    #[test]
    fn builds_and_exposes_fields() {
        let m = sample();
        assert_eq!(m.id().as_str(), "m1");
        assert_eq!(m.agent().as_str(), "a1");
        assert_eq!(m.risk(), RiskClass::Mutating);
        assert!(m.deadline().is_none());
    }

    #[test]
    fn deadline_is_attachable() {
        let d = Deadline::after(SystemTime::UNIX_EPOCH, Duration::from_secs(5));
        let m = sample().with_deadline(d);
        assert_eq!(m.deadline(), Some(d));
    }
}
