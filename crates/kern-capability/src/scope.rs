//! Execution and network scopes.

use std::collections::BTreeSet;

/// Network posture a capability permits.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NetworkMode {
    /// No network at all (the default).
    #[default]
    DenyAll,
    /// A set of allowed host:port endpoints. **Note:** enforcement of this mode is a
    /// sandbox-backend capability; on backends that cannot enforce it, a mission that
    /// requires it is refused (see `kern-exec::sandbox`). Represented here so a
    /// capability can *declare* the need.
    AllowList(BTreeSet<String>),
}

/// Whether an access needs the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkNeed {
    /// No network needed.
    None,
    /// A connection to `host:port`.
    Connect(String),
}

/// True when `need` is satisfied by `mode`.
#[must_use]
pub fn network_allowed(mode: &NetworkMode, need: &NetworkNeed) -> bool {
    match need {
        NetworkNeed::None => true,
        NetworkNeed::Connect(endpoint) => match mode {
            NetworkMode::DenyAll => false,
            NetworkMode::AllowList(set) => set.contains(endpoint),
        },
    }
}

/// Reason an exec request was refused by a scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecDenied {
    /// The program is not in the allowlist.
    ProgramNotAllowed(String),
    /// An argument matched a denied flag prefix (config/command injection).
    ArgRejected(String),
    /// The program name or an argument contained a NUL or control character.
    ControlCharacter,
}

impl core::fmt::Display for ExecDenied {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ExecDenied::ProgramNotAllowed(p) => write!(f, "program not allowed: {p}"),
            ExecDenied::ArgRejected(a) => write!(f, "argument rejected: {a}"),
            ExecDenied::ControlCharacter => f.write_str("control character in program/argument"),
        }
    }
}

fn has_control_char(s: &str) -> bool {
    s.chars().any(|c| c == '\0' || c.is_control())
}

/// What programs and arguments a capability permits.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExecScope {
    programs: BTreeSet<String>,
    deny_arg_prefixes: Vec<String>,
}

impl ExecScope {
    /// A scope allowing exactly `programs`, with a default denylist of flag prefixes that
    /// are classic vectors for config/command injection (e.g. git `-c`, `--upload-pack`,
    /// ssh `-o`). This blocks argument injection even though execution is already typed.
    #[must_use]
    pub fn new<I, S>(programs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            programs: programs.into_iter().map(Into::into).collect(),
            deny_arg_prefixes: default_denied_prefixes(),
        }
    }

    /// Replace the denied-argument-prefix list.
    #[must_use]
    pub fn deny_arg_prefixes(mut self, prefixes: Vec<String>) -> Self {
        self.deny_arg_prefixes = prefixes;
        self
    }

    /// Check a concrete `program + argv`.
    pub fn check(&self, program: &str, argv: &[String]) -> Result<(), ExecDenied> {
        if has_control_char(program) {
            return Err(ExecDenied::ControlCharacter);
        }
        if !self.programs.contains(program) {
            return Err(ExecDenied::ProgramNotAllowed(program.to_string()));
        }
        for arg in argv {
            if has_control_char(arg) {
                return Err(ExecDenied::ControlCharacter);
            }
            for deny in &self.deny_arg_prefixes {
                if arg.starts_with(deny.as_str()) {
                    return Err(ExecDenied::ArgRejected(arg.clone()));
                }
            }
        }
        Ok(())
    }
}

fn default_denied_prefixes() -> Vec<String> {
    [
        "-c",
        "--config",
        "--config-env",
        "--upload-pack",
        "--receive-pack",
        "--exec",
        "-o",
        "--output",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_scope() -> ExecScope {
        ExecScope::new(["git"])
    }

    #[test]
    fn allows_listed_program_clean_args() {
        assert!(git_scope().check("git", &["status".into()]).is_ok());
    }

    #[test]
    fn rejects_unlisted_program() {
        assert_eq!(
            git_scope().check("rm", &["-rf".into()]),
            Err(ExecDenied::ProgramNotAllowed("rm".into()))
        );
    }

    #[test]
    fn rejects_config_injection() {
        let r = git_scope().check("git", &["-c".into(), "core.sshCommand=evil".into()]);
        assert_eq!(r, Err(ExecDenied::ArgRejected("-c".into())));
        let r2 = git_scope().check("git", &["--upload-pack=touch /tmp/pwn".into()]);
        assert_eq!(
            r2,
            Err(ExecDenied::ArgRejected(
                "--upload-pack=touch /tmp/pwn".into()
            ))
        );
    }

    #[test]
    fn rejects_control_characters() {
        assert_eq!(
            git_scope().check("git", &["log\0--all".into()]),
            Err(ExecDenied::ControlCharacter)
        );
    }

    #[test]
    fn network_default_denies() {
        let mode = NetworkMode::default();
        assert!(network_allowed(&mode, &NetworkNeed::None));
        assert!(!network_allowed(
            &mode,
            &NetworkNeed::Connect("example.com:443".into())
        ));
    }

    #[test]
    fn network_allowlist_matches_exact_endpoint() {
        let mut set = BTreeSet::new();
        set.insert("example.com:443".to_string());
        let mode = NetworkMode::AllowList(set);
        assert!(network_allowed(
            &mode,
            &NetworkNeed::Connect("example.com:443".into())
        ));
        assert!(!network_allowed(
            &mode,
            &NetworkNeed::Connect("evil.com:443".into())
        ));
    }
}
