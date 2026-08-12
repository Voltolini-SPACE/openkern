# OpenKern Gates

A gate is `PASS` only with reproducible evidence: a command, its output, a test, and a
commit. No gate is inherited from any prior effort (`OPENKERN-VALIDATION-02` baseline is
`INVALID` / `DO_NOT_TRUST`). This project began at `OPENKERN_STATE = ZERO`.

| Gate | Scope | Crate(s) |
|---|---|---|
| G0 | Workspace foundation: check / test / fmt / clippy -D warnings | (all) |
| G1 | Kernel value types (typed IDs, Mission, budgets, evidence) | kern-types |
| G1.1 | Mission FSM — invalid transitions impossible | kern-fsm |
| G2 | Policy engine — Allow / Ask / Deny, default Deny | kern-policy |
| G3 | Capability engine — bounded authority + adversarial enforcement | kern-capability |
| G4 | Repository identity — worktree / gitdir / HEAD, no cross-repo effect | kern-repo |
| G5 | Transactional Git — governed mutation with identity + expected HEAD | kern-git |
| G5.1 | Worktree isolation — one mission per worktree | kern-git |
| G6 | Typed execution runtime — no shell, no orphans | kern-exec |
| G7 | Sandbox contract — capability declaration + fail-closed negotiation | kern-exec::sandbox |

Closing criteria per gate live in the mission spec; results live in `VALIDATION_REPORT.md`.
