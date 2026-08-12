# OpenKern Known Gaps

Honest ledger. `IMPLEMENTED` / `TESTED` / `PROVEN` / `DEFERRED` / `UNSUPPORTED` / `BLOCKED`
are distinguished; nothing is upgraded for schedule.

## Open

- **LICENSE_DECISION = PENDING_OWNER_FREEZE.** Manifest carries a placeholder
  (`LicenseRef-PROPRIETARY-UNRELEASED`). Before any public release: audit project
  license + dependency licenses + embedded code + third-party assets. Current dependency
  license surface: **std-only** (zero third-party runtime crates) — `NO_UNKNOWN_LICENSE_DEPENDENCY`
  holds for the kernel by construction, to be re-audited if a dependency is ever added.
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

## Deferred (out of scope for the bootstrap)

- G8 Context Engine, TUI, desktop app, cloud, billing, multi-provider routing, plugin
  marketplace, GitHub remote, public release.
