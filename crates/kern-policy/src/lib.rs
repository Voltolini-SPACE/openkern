//! `kern-policy` — the policy engine.
//!
//! A [`PolicyEngine`] maps a [`PolicyRequest`] to a [`Decision`]. The engine is
//! **default-deny**: a request that matches no rule is denied. Rules are evaluated in
//! order and the first match wins, so explicit `Deny` rules placed early cannot be
//! overridden by later `Allow` rules.
//!
//! Crucially, `Allow != UNLIMITED_AUTHORITY`. A [`Verdict::Allow`] only means the request
//! may *proceed to ask for a capability*. It confers no ability to touch the filesystem,
//! run a process, or mutate a repository — that authority is minted, and bounded, by
//! `kern-capability`. A [`Decision`] deliberately exposes no execution method.

use kern_types::{AgentId, RiskClass};

/// The three possible verdicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// The request may proceed to request a capability. Not authority in itself.
    Allow,
    /// A human/operator must confirm before proceeding.
    Ask,
    /// Refused.
    Deny,
}

/// A policy decision with a reason for the audit trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    verdict: Verdict,
    reason: String,
}

impl Decision {
    /// Build a decision.
    #[must_use]
    pub fn new(verdict: Verdict, reason: impl Into<String>) -> Self {
        Self {
            verdict,
            reason: reason.into(),
        }
    }

    /// The default-deny decision used when no rule matches.
    #[must_use]
    pub fn default_deny() -> Self {
        Self::new(Verdict::Deny, "no matching policy rule (default-deny)")
    }

    /// The verdict.
    #[must_use]
    pub fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// The reason.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// True only for [`Verdict::Allow`].
    #[must_use]
    pub fn is_allow(&self) -> bool {
        self.verdict == Verdict::Allow
    }
}

/// What is being asked of policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRequest {
    agent: AgentId,
    risk: RiskClass,
    operation: String,
}

impl PolicyRequest {
    /// Build a request.
    #[must_use]
    pub fn new(agent: AgentId, risk: RiskClass, operation: impl Into<String>) -> Self {
        Self {
            agent,
            risk,
            operation: operation.into(),
        }
    }

    /// The requesting agent.
    #[must_use]
    pub fn agent(&self) -> &AgentId {
        &self.agent
    }

    /// The risk class of the request.
    #[must_use]
    pub fn risk(&self) -> RiskClass {
        self.risk
    }

    /// The operation label (e.g. `"git.commit"`, `"exec.cargo"`).
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }
}

/// A single declarative rule. `None` fields are wildcards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRule {
    agent: Option<AgentId>,
    max_risk: Option<RiskClass>,
    operation_prefix: Option<String>,
    verdict: Verdict,
    reason: String,
}

impl PolicyRule {
    /// A rule that yields `verdict` when it matches.
    #[must_use]
    pub fn new(verdict: Verdict, reason: impl Into<String>) -> Self {
        Self {
            agent: None,
            max_risk: None,
            operation_prefix: None,
            verdict,
            reason: reason.into(),
        }
    }

    /// Restrict the rule to a specific agent.
    #[must_use]
    pub fn for_agent(mut self, agent: AgentId) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Restrict the rule to requests at or below `max_risk`.
    #[must_use]
    pub fn max_risk(mut self, risk: RiskClass) -> Self {
        self.max_risk = Some(risk);
        self
    }

    /// Restrict the rule to operations with this prefix.
    #[must_use]
    pub fn operation_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.operation_prefix = Some(prefix.into());
        self
    }

    fn matches(&self, req: &PolicyRequest) -> bool {
        if let Some(a) = &self.agent {
            if a != req.agent() {
                return false;
            }
        }
        if let Some(r) = self.max_risk {
            if !req.risk().within(r) {
                return false;
            }
        }
        if let Some(p) = &self.operation_prefix {
            if !req.operation().starts_with(p.as_str()) {
                return false;
            }
        }
        true
    }
}

/// An ordered set of rules, evaluated first-match-wins, default-deny.
#[derive(Debug, Clone, Default)]
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
}

impl PolicyEngine {
    /// An empty engine — denies everything.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a rule (evaluated after all previously-added rules).
    #[must_use]
    pub fn with_rule(mut self, rule: PolicyRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Evaluate a request. First matching rule wins; otherwise default-deny.
    #[must_use]
    pub fn evaluate(&self, req: &PolicyRequest) -> Decision {
        for rule in &self.rules {
            if rule.matches(req) {
                return Decision::new(rule.verdict, rule.reason.clone());
            }
        }
        Decision::default_deny()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(s: &str) -> AgentId {
        AgentId::new(s).unwrap()
    }

    #[test]
    fn empty_engine_denies_by_default() {
        let engine = PolicyEngine::new();
        let req = PolicyRequest::new(agent("a1"), RiskClass::ReadOnly, "git.status");
        let d = engine.evaluate(&req);
        assert_eq!(d.verdict(), Verdict::Deny);
        assert!(d.reason().contains("default-deny"));
    }

    #[test]
    fn first_matching_rule_wins() {
        // An early explicit Deny for destructive ops must beat a later broad Allow.
        let engine = PolicyEngine::new()
            .with_rule(
                PolicyRule::new(Verdict::Deny, "no destructive")
                    .max_risk(RiskClass::Destructive)
                    .operation_prefix("git.push"),
            )
            .with_rule(PolicyRule::new(Verdict::Allow, "broad allow"));

        let push = PolicyRequest::new(agent("a1"), RiskClass::Destructive, "git.push.force");
        assert_eq!(engine.evaluate(&push).verdict(), Verdict::Deny);

        let status = PolicyRequest::new(agent("a1"), RiskClass::ReadOnly, "git.status");
        assert_eq!(engine.evaluate(&status).verdict(), Verdict::Allow);
    }

    #[test]
    fn risk_ceiling_and_agent_scoping() {
        let engine = PolicyEngine::new().with_rule(
            PolicyRule::new(Verdict::Allow, "a1 may mutate")
                .for_agent(agent("a1"))
                .max_risk(RiskClass::Mutating),
        );
        // right agent, within ceiling -> allow
        assert!(engine
            .evaluate(&PolicyRequest::new(
                agent("a1"),
                RiskClass::Mutating,
                "git.add"
            ))
            .is_allow());
        // right agent, above ceiling -> default deny
        assert_eq!(
            engine
                .evaluate(&PolicyRequest::new(
                    agent("a1"),
                    RiskClass::Executing,
                    "exec"
                ))
                .verdict(),
            Verdict::Deny
        );
        // wrong agent -> default deny
        assert_eq!(
            engine
                .evaluate(&PolicyRequest::new(
                    agent("a2"),
                    RiskClass::ReadOnly,
                    "git.add"
                ))
                .verdict(),
            Verdict::Deny
        );
    }

    #[test]
    fn ask_is_representable() {
        let engine = PolicyEngine::new().with_rule(
            PolicyRule::new(Verdict::Ask, "confirm first").max_risk(RiskClass::Executing),
        );
        let d = engine.evaluate(&PolicyRequest::new(
            agent("a1"),
            RiskClass::Executing,
            "exec.cargo",
        ));
        assert_eq!(d.verdict(), Verdict::Ask);
    }
}
