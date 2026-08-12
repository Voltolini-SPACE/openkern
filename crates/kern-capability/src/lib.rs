//! `kern-capability` — bounded, one-use authority.
//!
//! A [`Capability`] is the *only* thing that authorizes an effect. It binds a concrete
//! agent, mission, repository, and (optionally) worktree, and it bounds the filesystem,
//! network, program/argv, deadline, side-effect risk, and number of uses. A policy
//! `Allow` is a precondition for minting a capability, never a substitute for one.
//!
//! Authority is spent through a [`CapabilityGrant`], which tracks remaining uses so a
//! one-shot grant cannot be replayed. Every check is fail-closed: anything not explicitly
//! permitted is [`Denied`].

pub mod fs;
pub mod scope;

use std::time::SystemTime;

use kern_types::{
    AgentId, CapabilityId, Deadline, MissionId, OperationId, RepositoryId, RiskClass, WorktreeId,
};

pub use fs::FsScope;
pub use scope::{network_allowed, ExecDenied, ExecScope, NetworkMode, NetworkNeed};

/// A request to perform a concrete access under a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessRequest {
    /// The agent making the request.
    pub agent: AgentId,
    /// The mission the request belongs to.
    pub mission: MissionId,
    /// The repository targeted.
    pub repository: RepositoryId,
    /// The worktree targeted, if any.
    pub worktree: Option<WorktreeId>,
    /// The operation label (must be in the capability's operation set).
    pub operation: String,
    /// The risk class of this access.
    pub risk: RiskClass,
    /// Paths the access will read.
    pub fs_reads: Vec<String>,
    /// Paths the access will write.
    pub fs_writes: Vec<String>,
    /// A subprocess the access will run, if any (`program`, `argv`).
    pub exec: Option<(String, Vec<String>)>,
    /// Network need of the access.
    pub network: NetworkNeed,
}

impl AccessRequest {
    /// A minimal read-only request against a repository.
    #[must_use]
    pub fn new(
        agent: AgentId,
        mission: MissionId,
        repository: RepositoryId,
        operation: impl Into<String>,
        risk: RiskClass,
    ) -> Self {
        Self {
            agent,
            mission,
            repository,
            worktree: None,
            operation: operation.into(),
            risk,
            fs_reads: Vec::new(),
            fs_writes: Vec::new(),
            exec: None,
            network: NetworkNeed::None,
        }
    }
}

/// Why authorization was refused. Every variant is an unauthorized effect that did **not**
/// happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Denied {
    /// The grant's deadline has passed.
    Expired,
    /// No uses remain (e.g. a replayed one-shot grant).
    Exhausted,
    /// The requesting agent is not the grantee.
    WrongAgent,
    /// The mission does not match the grant.
    WrongMission,
    /// The repository does not match the grant.
    WrongRepository,
    /// The worktree does not match the grant.
    WrongWorktree,
    /// The operation is not in the granted set.
    OperationNotGranted(String),
    /// The access risk exceeds the granted ceiling.
    RiskExceeded,
    /// A path escapes the granted filesystem scope.
    FsEscape(String),
    /// The program/argv was refused by the exec scope.
    Exec(ExecDenied),
    /// The network need is not permitted.
    NetworkNotAllowed(String),
}

impl core::fmt::Display for Denied {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Denied::Expired => f.write_str("capability expired"),
            Denied::Exhausted => f.write_str("capability uses exhausted"),
            Denied::WrongAgent => f.write_str("agent does not match grant"),
            Denied::WrongMission => f.write_str("mission does not match grant"),
            Denied::WrongRepository => f.write_str("repository does not match grant"),
            Denied::WrongWorktree => f.write_str("worktree does not match grant"),
            Denied::OperationNotGranted(op) => write!(f, "operation not granted: {op}"),
            Denied::RiskExceeded => f.write_str("access risk exceeds granted ceiling"),
            Denied::FsEscape(p) => write!(f, "path escapes granted scope: {p}"),
            Denied::Exec(e) => write!(f, "{e}"),
            Denied::NetworkNotAllowed(n) => write!(f, "network not allowed: {n}"),
        }
    }
}

impl std::error::Error for Denied {}

/// A bounded authority definition. Inert until wrapped in a [`CapabilityGrant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    id: CapabilityId,
    agent: AgentId,
    mission: MissionId,
    repository: RepositoryId,
    worktree: Option<WorktreeId>,
    operations: Vec<String>,
    fs: FsScope,
    exec: ExecScope,
    network: NetworkMode,
    deadline: Deadline,
    risk_ceiling: RiskClass,
}

/// Builder inputs common to constructing a [`Capability`].
#[derive(Debug, Clone)]
pub struct CapabilitySpec {
    /// The grantee agent.
    pub agent: AgentId,
    /// The bound mission.
    pub mission: MissionId,
    /// The bound repository.
    pub repository: RepositoryId,
    /// The bound worktree, if any.
    pub worktree: Option<WorktreeId>,
    /// Allowed operation labels.
    pub operations: Vec<String>,
    /// Filesystem scope.
    pub fs: FsScope,
    /// Execution scope.
    pub exec: ExecScope,
    /// Network mode.
    pub network: NetworkMode,
    /// Expiry.
    pub deadline: Deadline,
    /// Maximum risk class permitted.
    pub risk_ceiling: RiskClass,
}

impl Capability {
    /// Mint a capability from a spec (assigns a fresh id).
    #[must_use]
    pub fn new(spec: CapabilitySpec) -> Self {
        Self {
            id: CapabilityId::generate(),
            agent: spec.agent,
            mission: spec.mission,
            repository: spec.repository,
            worktree: spec.worktree,
            operations: spec.operations,
            fs: spec.fs,
            exec: spec.exec,
            network: spec.network,
            deadline: spec.deadline,
            risk_ceiling: spec.risk_ceiling,
        }
    }

    /// The capability id.
    #[must_use]
    pub fn id(&self) -> &CapabilityId {
        &self.id
    }

    /// The grantee agent.
    #[must_use]
    pub fn agent(&self) -> &AgentId {
        &self.agent
    }

    /// The bound mission.
    #[must_use]
    pub fn mission(&self) -> &MissionId {
        &self.mission
    }

    /// The bound repository.
    #[must_use]
    pub fn repository(&self) -> &RepositoryId {
        &self.repository
    }

    /// The bound worktree, if any.
    #[must_use]
    pub fn worktree(&self) -> Option<&WorktreeId> {
        self.worktree.as_ref()
    }

    /// Wrap in a grant with `uses` allowed invocations.
    #[must_use]
    pub fn grant(self, uses: u32) -> CapabilityGrant {
        CapabilityGrant {
            capability: self,
            uses_remaining: uses,
        }
    }

    /// Wrap in a single-use grant.
    #[must_use]
    pub fn one_shot(self) -> CapabilityGrant {
        self.grant(1)
    }
}

/// An active grant. Spending authority decrements the use counter, so a one-shot grant
/// cannot be replayed.
#[derive(Debug, Clone)]
pub struct CapabilityGrant {
    capability: Capability,
    uses_remaining: u32,
}

impl CapabilityGrant {
    /// The underlying capability.
    #[must_use]
    pub fn capability(&self) -> &Capability {
        &self.capability
    }

    /// Uses left.
    #[must_use]
    pub fn uses_remaining(&self) -> u32 {
        self.uses_remaining
    }

    /// Authorize a concrete access at time `now`. On success one use is consumed and a
    /// fresh [`OperationId`] is returned. On failure the grant is **unchanged** (no use is
    /// consumed) and the reason is returned.
    pub fn authorize(
        &mut self,
        req: &AccessRequest,
        now: SystemTime,
    ) -> Result<OperationId, Denied> {
        // Check everything first; only consume a use if fully authorized.
        self.check(req, now)?;
        self.uses_remaining -= 1;
        Ok(OperationId::generate())
    }

    /// Authorize an operation that stays within the grant's own bound agent/mission/repo,
    /// specifying only the operation label and its risk. Convenience for governed callers
    /// (e.g. the transactional Git layer) that operate strictly on the granted target.
    pub fn authorize_operation(
        &mut self,
        operation: impl Into<String>,
        risk: RiskClass,
        now: SystemTime,
    ) -> Result<OperationId, Denied> {
        let cap = &self.capability;
        let mut req = AccessRequest::new(
            cap.agent.clone(),
            cap.mission.clone(),
            cap.repository.clone(),
            operation,
            risk,
        );
        req.worktree = cap.worktree.clone();
        self.authorize(&req, now)
    }

    fn check(&self, req: &AccessRequest, now: SystemTime) -> Result<(), Denied> {
        let cap = &self.capability;
        if cap.deadline.is_expired(now) {
            return Err(Denied::Expired);
        }
        if self.uses_remaining == 0 {
            return Err(Denied::Exhausted);
        }
        if req.agent != cap.agent {
            return Err(Denied::WrongAgent);
        }
        if req.mission != cap.mission {
            return Err(Denied::WrongMission);
        }
        if req.repository != cap.repository {
            return Err(Denied::WrongRepository);
        }
        if let Some(granted_wt) = &cap.worktree {
            if req.worktree.as_ref() != Some(granted_wt) {
                return Err(Denied::WrongWorktree);
            }
        }
        if !cap.operations.iter().any(|o| o == &req.operation) {
            return Err(Denied::OperationNotGranted(req.operation.clone()));
        }
        if !req.risk.within(cap.risk_ceiling) {
            return Err(Denied::RiskExceeded);
        }
        for p in req.fs_reads.iter().chain(req.fs_writes.iter()) {
            if !cap.fs.allows(p) {
                return Err(Denied::FsEscape(p.clone()));
            }
        }
        if let Some((program, argv)) = &req.exec {
            cap.exec.check(program, argv).map_err(Denied::Exec)?;
        }
        if !network_allowed(&cap.network, &req.network) {
            return Err(Denied::NetworkNotAllowed(format!("{:?}", req.network)));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const T0: SystemTime = SystemTime::UNIX_EPOCH;

    fn ids() -> (AgentId, MissionId, RepositoryId) {
        (
            AgentId::new("agent-1").unwrap(),
            MissionId::new("mission-1").unwrap(),
            RepositoryId::new("repo-1").unwrap(),
        )
    }

    fn base_spec() -> CapabilitySpec {
        let (agent, mission, repository) = ids();
        CapabilitySpec {
            agent,
            mission,
            repository,
            worktree: None,
            operations: vec!["git.status".into(), "exec.git".into()],
            fs: FsScope::root("/work/repo"),
            exec: ExecScope::new(["git"]),
            network: NetworkMode::DenyAll,
            deadline: Deadline::after(T0, Duration::from_secs(45)),
            risk_ceiling: RiskClass::Executing,
        }
    }

    fn base_req() -> AccessRequest {
        let (agent, mission, repository) = ids();
        AccessRequest::new(
            agent,
            mission,
            repository,
            "git.status",
            RiskClass::ReadOnly,
        )
    }

    #[test]
    fn happy_path_authorizes_and_consumes_use() {
        let mut g = Capability::new(base_spec()).one_shot();
        assert_eq!(g.uses_remaining(), 1);
        let op = g.authorize(&base_req(), T0).expect("authorized");
        assert!(op.as_str().starts_with("op-"));
        assert_eq!(g.uses_remaining(), 0);
    }

    #[test]
    fn wrong_agent_is_denied() {
        let mut g = Capability::new(base_spec()).one_shot();
        let mut req = base_req();
        req.agent = AgentId::new("attacker").unwrap();
        assert_eq!(g.authorize(&req, T0), Err(Denied::WrongAgent));
        assert_eq!(g.uses_remaining(), 1, "denied request must not spend a use");
    }

    #[test]
    fn wrong_mission_and_repo_denied() {
        let mut g = Capability::new(base_spec()).one_shot();
        let mut r1 = base_req();
        r1.mission = MissionId::new("other").unwrap();
        assert_eq!(g.authorize(&r1, T0), Err(Denied::WrongMission));

        let mut r2 = base_req();
        r2.repository = RepositoryId::new("other-repo").unwrap();
        assert_eq!(g.authorize(&r2, T0), Err(Denied::WrongRepository));
    }

    #[test]
    fn expired_capability_denied() {
        let mut g = Capability::new(base_spec()).one_shot();
        let later = T0 + Duration::from_secs(46);
        assert_eq!(g.authorize(&base_req(), later), Err(Denied::Expired));
    }

    #[test]
    fn one_shot_cannot_be_replayed() {
        let mut g = Capability::new(base_spec()).one_shot();
        assert!(g.authorize(&base_req(), T0).is_ok());
        // second use of the same one-shot grant is refused
        assert_eq!(g.authorize(&base_req(), T0), Err(Denied::Exhausted));
    }

    #[test]
    fn absolute_path_and_traversal_denied() {
        let mut g = Capability::new(base_spec()).grant(10);
        let mut abs = base_req();
        abs.fs_reads = vec!["/etc/passwd".into()];
        assert_eq!(
            g.authorize(&abs, T0),
            Err(Denied::FsEscape("/etc/passwd".into()))
        );

        let mut trav = base_req();
        trav.fs_writes = vec!["../../root/.ssh/authorized_keys".into()];
        assert!(matches!(g.authorize(&trav, T0), Err(Denied::FsEscape(_))));
    }

    #[test]
    fn argument_injection_denied() {
        let mut g = Capability::new(base_spec()).grant(10);
        let mut req = base_req();
        req.operation = "exec.git".into();
        req.risk = RiskClass::Executing;
        req.exec = Some((
            "git".into(),
            vec!["-c".into(), "core.sshCommand=evil".into()],
        ));
        assert_eq!(
            g.authorize(&req, T0),
            Err(Denied::Exec(ExecDenied::ArgRejected("-c".into())))
        );
    }

    #[test]
    fn operation_not_granted_denied() {
        let mut g = Capability::new(base_spec()).grant(10);
        let mut req = base_req();
        req.operation = "git.push".into();
        assert_eq!(
            g.authorize(&req, T0),
            Err(Denied::OperationNotGranted("git.push".into()))
        );
    }

    #[test]
    fn risk_ceiling_enforced() {
        let mut spec = base_spec();
        spec.risk_ceiling = RiskClass::ReadOnly;
        let mut g = Capability::new(spec).grant(10);
        let mut req = base_req();
        req.risk = RiskClass::Destructive;
        assert_eq!(g.authorize(&req, T0), Err(Denied::RiskExceeded));
    }

    #[test]
    fn network_denied_by_default() {
        let mut g = Capability::new(base_spec()).grant(10);
        let mut req = base_req();
        req.network = NetworkNeed::Connect("evil.com:443".into());
        assert!(matches!(
            g.authorize(&req, T0),
            Err(Denied::NetworkNotAllowed(_))
        ));
    }

    #[test]
    fn authorize_operation_uses_bound_target() {
        let mut g = Capability::new(base_spec()).grant(2);
        assert!(g
            .authorize_operation("git.status", RiskClass::ReadOnly, T0)
            .is_ok());
        assert_eq!(g.uses_remaining(), 1);
        assert_eq!(
            g.authorize_operation("git.push", RiskClass::ReadOnly, T0),
            Err(Denied::OperationNotGranted("git.push".into()))
        );
        assert_eq!(g.uses_remaining(), 1);
    }

    #[test]
    fn worktree_binding_denies_other_worktree() {
        let mut spec = base_spec();
        spec.worktree = Some(WorktreeId::new("wt-a").unwrap());
        let mut g = Capability::new(spec).grant(10);

        // request against the bound worktree -> ok
        let mut ok = base_req();
        ok.worktree = Some(WorktreeId::new("wt-a").unwrap());
        assert!(g.authorize(&ok, T0).is_ok());

        // request against a different worktree -> denied
        let mut bad = base_req();
        bad.worktree = Some(WorktreeId::new("wt-b").unwrap());
        assert_eq!(g.authorize(&bad, T0), Err(Denied::WrongWorktree));
    }
}
