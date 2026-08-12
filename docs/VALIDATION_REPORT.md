# OpenKern Validation Report — OPENKERN-BOOTSTRAP-01

Every `PASS` below points to code, tests, and a commit. No gate was inherited; the project
began at `OPENKERN_STATE = ZERO` (the prior `OPENKERN-VALIDATION-02` baseline was declared
`INVALID` / `DO_NOT_TRUST`).

## Environment

| | |
|---|---|
| HOST | macOS 26.3.1, arm64 (Apple Silicon) |
| RUST_VERSION | rustc 1.97.1 (8bab26f4f 2026-07-14) |
| CARGO_VERSION | cargo 1.97.1 (c980f4866 2026-06-30) |
| REPO | `~/Projects/openkern`, branch `main` |
| BASELINE_SHA (G0) | `677a73cc9035dfdd22fe8e14b8b73728f774b8dd` |
| DEPENDENCIES | std-only in all kernel crates (zero third-party runtime crates) |
| UNSAFE | exactly one audited `unsafe` block (`kern-exec` `killpg` FFI); rest `forbid(unsafe_code)` |

## Gate results

| Gate | Status | Tests | Evidence |
|---|---|---|---|
| G0 workspace foundation | PASS | — | `cargo check/test/fmt/clippy -D warnings` all exit 0 · commit `677a73c` |
| G1 value types | PASS | 12 | `kern-types`: typed IDs, `Mission`, budget, deadline, risk, evidence · `f8f76fc` |
| G1.1 mission FSM | PASS | 5 | `kern-fsm`: `CREATED->COMPLETED` denied; terminals have no out-edges · `59341e3` |
| G2 policy engine | PASS | 4 | `kern-policy`: Allow/Ask/Deny, default-deny, `Allow != authority` · `a8e1bfd` |
| G3 capability engine | PASS | 22 | `kern-capability`: 8 adversarial vectors denied, denials spend no use, `UNAUTHORIZED_SUCCESS=0` · `fd1d803` |
| G4 repository identity | PASS | 7 | `kern-repo`: worktree/gitdir/HEAD from plumbing; `CROSS_REPOSITORY_EFFECT=0` · `8d8d626` |
| G5 transactional Git | PASS | 8 | `kern-git`: single chokepoint, hardened profile, expected-HEAD TOCTOU guard, hostile-config positive controls · `a7c6b91` |
| G5.1 worktree isolation | PASS | (in G5) | linked worktree isolated; capability worktree-binding · `477ae6f` |
| G6 typed execution | PASS | 5 | `kern-exec`: no shell, env allowlist (secret isolation), process-group teardown, `ORPHAN_PROCESS=0` · `2556be8` |
| G7 sandbox contract | PASS (explicit fail-closed capability set) | 6 | `kern-exec::sandbox`: honest host capabilities, `REFUSE_UNSUPPORTED_SANDBOX_CAPABILITY` · `612b403` |

**TEST_COUNT = 69** (12 + 5 + 4 + 22 + 7 + 8 + 11), all passing.

## Security properties proven

| Property | Result | Where |
|---|---|---|
| `UNAUTHORIZED_SUCCESS` (wrong agent/mission/repo, expired, replayed one-shot, abs path, `..`, arg-injection) | 0 | `kern-capability` tests |
| `CROSS_REPOSITORY_EFFECT` (submodule/linked-worktree/separate-gitdir/symlink) | 0 | `kern-repo` tests |
| `UNGOVERNED_GIT_CALLS` | 0 (exactly one chokepoint, structurally asserted) | `kern-git` `single_git_chokepoint_and_no_shell_escapes` |
| `UNGOVERNED_SHELL_CALLS` | 0 | same test |
| Git hooks / clean filter / hostile global config | contained (each with a positive control that fires when unhardened) | `kern-git` tests |
| `HEAD` moved between authorization and mutation (TOCTOU) | refused | `kern-git` `head_moved_*` |
| Secret leakage into child env | none (env cleared + allowlist) | `kern-exec` `environment_is_isolated_to_allowlist` |
| `ORPHAN_PROCESS` (grandchild after timeout) | 0 (positive control confirms the effect is real) | `kern-exec` `grandchild_is_reaped_*` |
| Unsupported sandbox capability | refused, never warn-and-run | `kern-exec::sandbox` tests |

## Regression (final)

```
CHECK  rc=0   TEST rc=0 (69 passed)   FMT rc=0   CLIPPY -D warnings rc=0
```

## Known gaps (see docs/GAPS.md)

- macOS OS-level sandbox (kernel FS/network/PID isolation): **UNSUPPORTED / fail-closed**.
  The host backend refuses missions requiring it rather than running degraded.
- `network_allowlist`: **UNSUPPORTED** (declared, not enforced) — refused.
- Git filter/diff/textconv containment is config-sanitization (L1), verified for the
  clean-filter vector; full containment of a `.gitattributes`-triggered filter for an
  untrusted repo ultimately depends on the (deferred) OS sandbox.
- A grandchild that deliberately `setsid`s out of its process group is not reaped by the
  host backend (needs OS sandbox / PID namespace).
- `LICENSE_DECISION = PENDING_OWNER_FREEZE`.
