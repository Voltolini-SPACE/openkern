# OpenKern Known Gaps

Honest ledger. `IMPLEMENTED` / `TESTED` / `PROVEN` / `DEFERRED` / `UNSUPPORTED` / `BLOCKED`
are distinguished; nothing is upgraded for schedule.

## Resolved

- **LICENSE_DECISION = MIT (owner, 2026-08-12, mission OPENKERN-BRAND-03-PUBLISH).**
  Manifest updated to `MIT`; `LICENSE` file added. Dependency audit at publication:
  `syn`, `proc-macro2`, `quote` — `MIT OR Apache-2.0`; `unicode-ident` —
  `(MIT OR Apache-2.0) AND Unicode-3.0`. All compatible. `NO_UNKNOWN_LICENSE_DEPENDENCY`
  holds; re-audit on any new dependency.

## Open
- **macOS OS-level sandbox: UNSUPPORTED / fail-closed.** The host backend does not provide
  kernel-enforced filesystem/network/pid isolation. Missions requiring those capabilities
  are refused, not run degraded. A real backend (Linux namespaces, macOS `sandbox_init`
  profiles, or a VM) is future work.
- **network allow-list: UNSUPPORTED.** Only `DenyAll` (by not granting network) and the
  refusal path are modeled; a real allow-list enforcement primitive is not implemented.
- **`setsid` escape from process-group teardown.** The host exec backend kills the child's
  process group, which reaps children and grandchildren *in that group*. A process that
  deliberately double-forks and `setsid`s into a new session escapes and would not be
  reaped. Containing that needs OS-level PID isolation (the deferred sandbox). Tracked, not
  claimed as solved.
- **Git filter/attribute containment depth.** The governed runner neutralizes filter and
  diff drivers by enumerating the repo config and overriding them via `-c` (L1 config
  sanitization), proven for the clean-filter vector with a positive control. Full
  containment of every `.gitattributes`-driven vector for a fully-untrusted repository
  ultimately relies on running git inside the OS sandbox (deferred / fail-closed on macOS).

## Deferred (out of scope)

- TUI, desktop app, cloud, billing, multi-provider routing, plugin marketplace, GitHub
  remote, public release. (G8 Context Engine is now implemented — see below.)

## G8 Context Engine — deferred / partial (honest)

- **File-source candidates DEFERRED**: the engine scores Rust *symbols* only. Non-`.rs`
  file candidates (config/docs retrieval) are modeled in the types (`SourceKind::File`) but
  not yet generated. "Find relevant config" tasks are therefore out of scope this round.
- **Cache (G8.10) DEFERRED**: no persistent index/graph cache yet. Determinism and
  content-hash validation are in place, so a cache can be added safely later.
- **Scale probe (G8.30) PARTIAL**: benchmark runs on a small fixture; dogfood on OpenKern
  itself indexes 78 files / 671 symbols / 1477 edges in-process, but formal small/medium/
  large scaling curves (index time, memory, latency) are not yet measured.
- **Cross-language indexing UNSUPPORTED**: Rust only.
- **Semantic resolution PARTIAL**: reference/call edges use unique-simple-name resolution;
  ambiguous names are left unresolved (no invented precision). Full type-aware resolution is
  future work.
- **Embedding/model ranking**: not implemented and not required; the kernel ranks with no
  provider (`MODEL_RANKING = OPTIONAL_ADAPTER`, deferred).
- **Adversarial A11/A14/A15/A16/A20** (cyclic graph, worktree switch mid-pack, HEAD change
  mid-pack, cache poisoning, unsupported encoding): not separately tested this round; the
  freshness + stale-hash + repo-binding defenses cover the substance of A14–A16.
- **New dependencies**: `syn`, `proc-macro2`, `quote`, `unicode-ident` — all `MIT OR
  Apache-2.0` (`unicode-ident` also `Unicode-3.0`). `NO_UNKNOWN_LICENSE_DEPENDENCY` holds.
