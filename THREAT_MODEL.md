# OpenKern Threat Model

OpenKern runs untrusted inputs: a repository it did not author, a prompt it did not
write, model output it cannot trust, and MCP servers it did not vet. The kernel's job is
to let an agent do useful engineering work while making unauthorized effects impossible
or refused — not merely discouraged.

## Assets

- The host filesystem outside the mission's granted roots (`~/.ssh`, `~/.aws`, `/etc`,
  other repositories, secrets).
- Other Git repositories on the host (no cross-repo mutation).
- Ambient secrets in the environment (`*_API_KEY`, `SSH_AUTH_SOCK`, cloud creds).
- Network egress.
- Process/host integrity (no orphaned or escaped processes).

## Adversaries & vectors

| # | Threat | Containment (crate) | Status |
|---|---|---|---|
| T1 | Malicious repository content | typed exec, no shell interpolation (`kern-exec`) | see gates |
| T2 | Malicious prompt / model output requesting broad authority | default-deny policy + capability bounds (`kern-policy`, `kern-capability`) | see gates |
| T3 | Hostile Git config (filters, hooks, external diff, textconv, credential helper, fsmonitor, sshCommand, include/includeIf, aliases, submodule helpers) | sanitized env + `GitExecutionProfile` in the single governed runner (`kern-git`) | see gates |
| T4 | TOCTOU — config/HEAD/gitdir swapped between validation and execution | `ExpectedHead` re-checked at mutation time (`kern-git`) | see gates |
| T5 | Confused deputy — capability for repo A used against repo B | `RepositoryIdentity` binding on every grant (`kern-repo`, `kern-capability`) | see gates |
| T6 | Cross-repository mutation | identity + worktree isolation (`kern-repo`, `kern-git`) | see gates |
| T7 | Filesystem escape (`../`, absolute path, symlink) | path canonicalization + root containment (`kern-capability`) | see gates |
| T8 | Network escape | sandbox network mode, fail-closed if unsupported (`kern-exec::sandbox`) | see gates |
| T9 | Process escape (fork / setsid / daemonize) & orphans | process-group supervision + kill-on-drop (`kern-exec`) | see gates |
| T10 | Secret disclosure to child processes | env cleared, allowlist-only (`kern-exec`) | see gates |
| T11 | Concurrent writer to the same worktree | one-mission-per-worktree isolation (`kern-git`) | see gates |
| T12 | Stale authority — expired or reused one-shot capability | deadline + single-use accounting (`kern-capability`) | see gates |

## Non-goals (this bootstrap)

- OpenKern does not yet provide OS-level kernel sandboxing on macOS. Where a backend
  cannot *prove* an isolation capability, the mission that requires it is **refused**,
  not run degraded. This is tracked honestly in `docs/GAPS.md` and `docs/VALIDATION_REPORT.md`.
- No network allow-listing primitive is claimed as "supported" unless a backend enforces
  it. `NetworkMode::AllowList` is `Unsupported` on the host backend and fails closed.
