//! The sandbox contract.
//!
//! A [`SandboxBackend`] advertises exactly what it can enforce via
//! [`SandboxBackendCapabilities`]. A mission states what it needs via a [`SandboxPolicy`].
//! [`negotiate`] compares them and is **fail-closed**: if the policy requires anything the
//! backend does not provide, the mission is *refused*
//! (`REFUSE_UNSUPPORTED_SANDBOX_CAPABILITY`) — never admitted with a warning.
//!
//! No capability may be inferred from a backend's name. The only host backend in this
//! bootstrap is [`HostProcessGroupBackend`], which is honest about its limits: it enforces
//! environment/secret isolation and best-effort process-tree teardown, but provides **no**
//! kernel-level filesystem, network, or PID-namespace isolation on this platform. Missions
//! that need those are refused. See `docs/GAPS.md`.

/// What a backend can enforce. Every field is an independent, kernel-meaningful
/// capability; nothing here is aspirational.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct SandboxBackendCapabilities {
    /// Kernel-enforced filesystem confinement (chroot/jail/namespace/profile).
    pub filesystem_isolation: bool,
    /// Kernel-enforced deny-all networking.
    pub network_deny_all: bool,
    /// Kernel-enforced network allow-listing.
    pub network_allowlist: bool,
    /// PID-namespace isolation (a child cannot see or escape via other pids).
    pub pid_isolation: bool,
    /// Best-effort teardown of the whole process group/tree.
    pub process_tree_kill: bool,
    /// The child's environment is cleared and repopulated only from an allowlist.
    pub environment_isolation: bool,
    /// Ambient secrets never reach the child.
    pub secret_isolation: bool,
}

/// What a mission requires. Same fields, read as "must be enforced".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct RequiredSandboxCapabilities {
    /// Require filesystem isolation.
    pub filesystem_isolation: bool,
    /// Require deny-all networking.
    pub network_deny_all: bool,
    /// Require network allow-listing.
    pub network_allowlist: bool,
    /// Require PID isolation.
    pub pid_isolation: bool,
    /// Require process-tree teardown.
    pub process_tree_kill: bool,
    /// Require environment isolation.
    pub environment_isolation: bool,
    /// Require secret isolation.
    pub secret_isolation: bool,
}

/// A named policy: the capabilities a mission requires of its sandbox.
#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    label: String,
    required: RequiredSandboxCapabilities,
}

impl SandboxPolicy {
    /// Build a policy.
    #[must_use]
    pub fn new(label: impl Into<String>, required: RequiredSandboxCapabilities) -> Self {
        Self {
            label: label.into(),
            required,
        }
    }

    /// The policy label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The required capabilities.
    #[must_use]
    pub fn required(&self) -> RequiredSandboxCapabilities {
        self.required
    }
}

/// Proof that a policy was satisfied by a backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxEvidence {
    /// The backend that admitted the mission.
    pub backend: String,
    /// The capabilities the backend provided.
    pub granted: SandboxBackendCapabilities,
}

/// A refusal: the backend cannot provide required capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxRefusal {
    /// The backend that was asked.
    pub backend: String,
    /// The capabilities that were required but not provided.
    pub missing: Vec<String>,
}

impl core::fmt::Display for SandboxRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "REFUSE_UNSUPPORTED_SANDBOX_CAPABILITY: backend {} cannot provide {:?}",
            self.backend, self.missing
        )
    }
}

impl std::error::Error for SandboxRefusal {}

/// A sandbox backend.
pub trait SandboxBackend {
    /// A stable backend name (informational only — never a source of capability).
    fn name(&self) -> &str;
    /// What this backend can actually enforce.
    fn capabilities(&self) -> SandboxBackendCapabilities;
}

/// The host backend: kern-exec's process-group runtime. Honest about its limits on this
/// platform.
#[derive(Debug, Clone, Copy, Default)]
pub struct HostProcessGroupBackend;

impl SandboxBackend for HostProcessGroupBackend {
    #[allow(clippy::unnecessary_literal_bound)] // trait allows dynamic (non-'static) names
    fn name(&self) -> &str {
        "host-process-group"
    }

    fn capabilities(&self) -> SandboxBackendCapabilities {
        SandboxBackendCapabilities {
            // No kernel-level FS/network/PID isolation on this platform.
            filesystem_isolation: false,
            network_deny_all: false,
            network_allowlist: false,
            pid_isolation: false,
            // What the runtime genuinely provides.
            process_tree_kill: true,
            environment_isolation: true,
            secret_isolation: true,
        }
    }
}

/// Compare a policy against a backend. Fail-closed: any required-but-unavailable capability
/// causes a refusal; only a fully-satisfiable policy is admitted.
pub fn negotiate(
    backend: &dyn SandboxBackend,
    policy: &SandboxPolicy,
) -> Result<SandboxEvidence, SandboxRefusal> {
    let have = backend.capabilities();
    let need = policy.required();
    let mut missing = Vec::new();

    let checks: [(&str, bool, bool); 7] = [
        (
            "filesystem_isolation",
            need.filesystem_isolation,
            have.filesystem_isolation,
        ),
        (
            "network_deny_all",
            need.network_deny_all,
            have.network_deny_all,
        ),
        (
            "network_allowlist",
            need.network_allowlist,
            have.network_allowlist,
        ),
        ("pid_isolation", need.pid_isolation, have.pid_isolation),
        (
            "process_tree_kill",
            need.process_tree_kill,
            have.process_tree_kill,
        ),
        (
            "environment_isolation",
            need.environment_isolation,
            have.environment_isolation,
        ),
        (
            "secret_isolation",
            need.secret_isolation,
            have.secret_isolation,
        ),
    ];
    for (name, required, provided) in checks {
        if required && !provided {
            missing.push(name.to_string());
        }
    }

    if missing.is_empty() {
        Ok(SandboxEvidence {
            backend: backend.name().to_string(),
            granted: have,
        })
    } else {
        Err(SandboxRefusal {
            backend: backend.name().to_string(),
            missing,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> HostProcessGroupBackend {
        HostProcessGroupBackend
    }

    #[test]
    fn host_backend_declares_honest_capabilities() {
        let c = host().capabilities();
        // What it genuinely provides:
        assert!(c.environment_isolation);
        assert!(c.secret_isolation);
        assert!(c.process_tree_kill);
        // What it does NOT provide on this platform (must not be overclaimed):
        assert!(!c.filesystem_isolation);
        assert!(!c.network_deny_all);
        assert!(!c.network_allowlist);
        assert!(!c.pid_isolation);
    }

    #[test]
    fn satisfiable_policy_is_admitted() {
        let policy = SandboxPolicy::new(
            "env-only",
            RequiredSandboxCapabilities {
                environment_isolation: true,
                secret_isolation: true,
                process_tree_kill: true,
                ..Default::default()
            },
        );
        let ev = negotiate(&host(), &policy).expect("should be admitted");
        assert_eq!(ev.backend, "host-process-group");
        assert!(ev.granted.environment_isolation);
    }

    #[test]
    fn filesystem_isolation_requirement_is_refused_fail_closed() {
        let policy = SandboxPolicy::new(
            "fs-jail",
            RequiredSandboxCapabilities {
                filesystem_isolation: true,
                environment_isolation: true,
                ..Default::default()
            },
        );
        let err = negotiate(&host(), &policy).unwrap_err();
        assert!(err.missing.contains(&"filesystem_isolation".to_string()));
        // fail-closed: NEVER admitted
        assert!(negotiate(&host(), &policy).is_err());
    }

    #[test]
    fn network_allowlist_requirement_is_refused() {
        let policy = SandboxPolicy::new(
            "net-allowlist",
            RequiredSandboxCapabilities {
                network_allowlist: true,
                ..Default::default()
            },
        );
        let err = negotiate(&host(), &policy).unwrap_err();
        assert_eq!(err.missing, vec!["network_allowlist".to_string()]);
    }

    #[test]
    fn multiple_missing_capabilities_all_reported() {
        let policy = SandboxPolicy::new(
            "strict",
            RequiredSandboxCapabilities {
                filesystem_isolation: true,
                network_deny_all: true,
                pid_isolation: true,
                ..Default::default()
            },
        );
        let err = negotiate(&host(), &policy).unwrap_err();
        assert_eq!(err.missing.len(), 3);
        assert!(err.missing.contains(&"pid_isolation".to_string()));
    }

    #[test]
    fn empty_policy_is_trivially_admitted() {
        let policy = SandboxPolicy::new("none", RequiredSandboxCapabilities::default());
        assert!(negotiate(&host(), &policy).is_ok());
    }
}
