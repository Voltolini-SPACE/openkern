# OpenKern

**The Verified Kernel for Agentic Engineering.**

OpenKern is a `local-first`, `default-deny` runtime for coding agents. It is not a
chat wrapper and not a clone of an existing CLI agent. It is the *kernel* underneath
one: the layer that decides what an agent is allowed to do, executes it under typed
and capability-bounded control, keeps Git changes transactional and repository-scoped,
and produces evidence for every effect.

## Status

Pre-release bootstrap. **Not published. No public release. No GitHub remote.**
See [`docs/VALIDATION_REPORT.md`](docs/VALIDATION_REPORT.md) for the current gate
results and their evidence.

## Principles

| Principle | Meaning |
|---|---|
| `STANDALONE` | Works with no external control plane. NOMOS / OpenClaw / Hermes are optional integrations, never required. |
| `LOCAL_FIRST` | Runs on the developer's machine; no cloud dependency. |
| `DEFAULT_DENY` | Nothing is authorized unless a capability explicitly grants it. |
| `TYPED_EXECUTION_FIRST` | Execution is `program + argv + cwd + env`, never a raw shell string. |
| `TRANSACTIONAL_GIT` | Every Git mutation carries repository identity, an expected `HEAD`, a transaction id, and a capability grant. |
| `VERIFICATION_FIRST` | A gate is `PASS` only when a command, its output, a test, and a commit can be pointed to. |
| `EVIDENCE_OVER_CLAIMS` | No claim of "tested" without a reproducible command and result. |
| `SECRET_ISOLATION` | Child processes never inherit the ambient environment; only explicitly-granted variables pass through. |

## Workspace

```text
crates/
├── kern-types/       value types, Mission, budgets, evidence          (G1)
├── kern-fsm/         Mission finite-state machine                     (G1.1)
├── kern-policy/      policy decisions (Allow / Ask / Deny), default Deny (G2)
├── kern-capability/  capability grants + adversarial enforcement       (G3)
├── kern-repo/        repository identity (worktree / gitdir / HEAD)    (G4)
├── kern-git/         governed, transactional Git — the ONLY git caller (G5, G5.1)
├── kern-exec/        typed execution runtime + sandbox contract      (G6, G7)
└── kern-cli/         `kern` binary
```

## Build & verify

```bash
cargo check   --workspace
cargo test    --workspace
cargo fmt     --all --check
cargo clippy  --workspace --all-targets --all-features -- -D warnings
```

## License

Not yet decided — see [`docs/GAPS.md`](docs/GAPS.md). The manifest carries the
placeholder `LicenseRef-PROPRIETARY-UNRELEASED`; this is **not** a final licensing
decision and must be resolved (with a dependency-license audit) before any public
release.
