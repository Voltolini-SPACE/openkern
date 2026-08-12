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

## Deferred (out of scope for the bootstrap)

- G8 Context Engine, TUI, desktop app, cloud, billing, multi-provider routing, plugin
  marketplace, GitHub remote, public release.
