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

## G8 — Context Engine (crate `kern-context`)

| Gate | Scope | Status |
|---|---|---|
| G8.0 context types | typed contracts | PASS |
| G8.1 symbol index | syn-based, repo/rev-bound, content-hashed | PASS |
| G8.2 dependency graph | Contains/Calls/References, bounded traversal | PASS |
| G8.3 context query | typed query | PASS |
| G8.4 scoring | deterministic multi-signal | PASS |
| G8.5 budget | bounded, deterministic truncation | PASS |
| G8.6 ContextPack | assembled, repo/mission-bound | PASS |
| G8.7 provenance | every item; missing ⇒ refused | PASS |
| G8.8 TOCTOU/stale | re-hash vs index-time hash | PASS |
| G8.9 security boundaries | worktree/symlink/secret | PASS |
| G8.10 cache | DEFERRED (see GAPS) | DEFERRED |
| G8.11 determinism | same input ⇒ same pack_hash | PASS |
| G8.12 benchmark | 4 baselines, superiority asserted | PASS |
| G8.13 adversarial | A1–A13, A17–A19 covered | PASS |
| G8.14 integration contract | typed ContextQuery/Pack/hash | PASS |
| G8.15 final regression | G0–G7 intact | PASS |
