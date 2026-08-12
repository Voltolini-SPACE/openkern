<p align="center"><img src="brand/github/avatar.svg" width="88" alt="openkern"></p>

<h1 align="center">openkern</h1>
<p align="center"><b>Governed execution for AI agents.</b><br>
Explicit authority. Bounded capabilities. Typed execution. Evidence over claims.</p>

---

Your agents can already act. OpenKern decides what they may run, on whose
authority, inside which boundary, and proves what actually happened. Default-deny
policy, one-use capabilities, transactional Git through a single chokepoint, and
typed execution with no shell. If an action is not explicitly permitted, it does
not run.

OpenKern is the **Governed Execution Kernel for AI Agents** — not a chat wrapper,
not an agent framework. It is the kernel underneath one.

## Status

Public since 2026-08-12. Product baseline `openkern-g8-context-01` (G0–G8,
95 tests). Brand v1.0 frozen at `openkern-brand-v1.0`.
Evidence for every gate: [`docs/VALIDATION_REPORT.md`](docs/VALIDATION_REPORT.md).

Website: **https://voltolini.space/openkern**

## Principles

| Principle | Meaning |
|---|---|
| `STANDALONE` | Works with no external control plane. NOMOS / OpenClaw / Hermes are optional integrations, never required. |
| `LOCAL_FIRST` | Runs on the developer's machine; no cloud dependency. External providers are optional adapters. |
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
├── kern-context/     governed, deterministic context engine          (G8)
└── kern-cli/         `kern` binary (version / sandbox / context)
```

## Build & verify

```bash
cargo check   --workspace
cargo test    --workspace
cargo fmt     --all --check
cargo clippy  --workspace --all-targets --all-features -- -D warnings
```

## Brand

The OpenKern identity (symbol `[>▮]`, achromatic system where color encodes
state, JetBrains Mono + Geist) is frozen at tag `openkern-brand-v1.0` and
documented in [`docs/BRAND_BOOK.md`](docs/BRAND_BOOK.md). Assets live in
[`brand/`](brand/).

## License

[MIT](LICENSE). Dependency licenses audited: `syn`, `proc-macro2`, `quote`,
`unicode-ident` — all `MIT OR Apache-2.0` (plus `Unicode-3.0` for
`unicode-ident`), compatible.
