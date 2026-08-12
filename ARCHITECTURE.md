# OpenKern Architecture

OpenKern is a set of small, std-only Rust crates. The kernel has **zero third-party
runtime dependencies**: this keeps the supply-chain and license surface auditable and
removes a class of confused-deputy risk from transitive code.

## Layering (a request flows down, evidence flows up)

```text
        Mission (kern-types)                 what the operator wants
              │
      PolicyDecision (kern-policy)           Allow / Ask / Deny   (default Deny)
              │
      CapabilityGrant (kern-capability)      a bounded, one-use authority
              │
      RepositoryIdentity (kern-repo)         canonical worktree / gitdir / HEAD
              │
   ┌──────────┴───────────┐
GovernedGit (kern-git)   TypedExec (kern-exec)   the only spawn points
   │                        │
   └──────────┬─────────────┘
              │
        SandboxPolicy (kern-exec::sandbox)   fail-closed capability negotiation
              │
        Validated Effect  ->  Evidence (kern-types)
```

### Key invariants

- **`ALLOW != UNLIMITED_AUTHORITY`.** A policy `Allow` only means "you may now ask for a
  capability." The capability is what actually bounds the effect.
- **Single Git chokepoint.** `kern-git::GitRunner::spawn` is the *only* place in the
  workspace that constructs `Command::new("git")`. A structural test greps the source
  tree to keep `UNGOVERNED_GIT_CALLS = 0`. The same test enforces `UNGOVERNED_SHELL_CALLS = 0`.
- **Repository identity is never `cwd` alone.** Identity is resolved by reading the Git
  plumbing files (`.git` redirect, `commondir`, `HEAD`) so a linked worktree, a
  separate git-dir, or a symlink cannot be confused for another repository.
- **Hostile Git config.** `.git/config`, `.gitattributes`, `.gitmodules`, and global /
  system config are untrusted input. The governed runner runs Git with a sanitized
  environment (`GIT_CONFIG_NOSYSTEM=1`, neutralized global config, no hooks path, no
  fsmonitor, no terminal prompt) and a `GitExecutionProfile` that disables filters,
  external diff, textconv, credential helpers, and remote operations by default.
- **Typed execution.** Nothing runs as a shell string. A `Command` spec is
  `program + argv[] + cwd + env`, with the environment cleared and repopulated only
  from an explicit allowlist (secret isolation).
- **Fail-closed sandbox.** A backend advertises `SandboxBackendCapabilities`. If a
  mission's `RequiredSandboxCapabilities` exceed what the backend proves, execution is
  *refused* (`REFUSE_UNSUPPORTED_SANDBOX_CAPABILITY`) — never "warn and run".

## Integration edges (future)

NOMOS, OpenClaw, Hermes, MCP, GitHub, CI, and IDEs attach at the edges through a
`gateway/` layer (not built in this bootstrap). The kernel never learns a third party's
internal implementation; integrations depend on the kernel's typed contracts, not the
reverse. `NOMOS_REQUIRED = OPENCLAW_REQUIRED = HERMES_REQUIRED = FALSE`.
