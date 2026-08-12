//! `GitExecutionProfile` — what repository-controlled Git extensibility is permitted.
//!
//! `.git/config`, `.gitattributes`, and friends are untrusted input. The default profile
//! is fully hardened: hooks, filters, external diff, textconv, credential helpers,
//! fsmonitor, remote operations, and submodule command execution are all **off**. The
//! runner translates a profile into a sanitized environment plus `-c` overrides that
//! neutralize the corresponding config keys before Git ever acts on them.

/// A set of switches controlling repository-driven Git behaviour.
///
/// This is intentionally a flat set of independent boolean switches — each maps to a
/// distinct Git extensibility vector — so the `struct_excessive_bools` lint is allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct GitExecutionProfile {
    /// Run `.git/hooks/*`.
    pub allow_hooks: bool,
    /// Honour `filter.*.clean/smudge/process`.
    pub allow_filters: bool,
    /// Honour `diff.*.command` (external diff).
    pub allow_external_diff: bool,
    /// Honour `diff.*.textconv`.
    pub allow_textconv: bool,
    /// Honour `credential.helper`.
    pub allow_credential_helpers: bool,
    /// Honour `core.fsmonitor`.
    pub allow_fsmonitor: bool,
    /// Permit remote operations and honour `core.sshCommand`.
    pub allow_remote: bool,
    /// Permit submodule command execution.
    pub allow_submodule_exec: bool,
    /// Read the ambient system/global Git config. When false, system config is disabled
    /// and global config is redirected to `/dev/null`.
    pub trust_ambient_config: bool,
}

impl Default for GitExecutionProfile {
    /// The hardened default — everything repository-controlled is denied.
    fn default() -> Self {
        Self {
            allow_hooks: false,
            allow_filters: false,
            allow_external_diff: false,
            allow_textconv: false,
            allow_credential_helpers: false,
            allow_fsmonitor: false,
            allow_remote: false,
            allow_submodule_exec: false,
            trust_ambient_config: false,
        }
    }
}

impl GitExecutionProfile {
    /// The hardened default.
    #[must_use]
    pub fn hardened() -> Self {
        Self::default()
    }

    /// A fully-permissive profile. Used **only** to demonstrate, as a positive control in
    /// tests, that an attack succeeds when hardening is absent — never for real work.
    #[must_use]
    pub fn unhardened() -> Self {
        Self {
            allow_hooks: true,
            allow_filters: true,
            allow_external_diff: true,
            allow_textconv: true,
            allow_credential_helpers: true,
            allow_fsmonitor: true,
            allow_remote: true,
            allow_submodule_exec: true,
            trust_ambient_config: true,
        }
    }
}
