# OpenKern Validation Report — OPENKERN-G8-01 (Context Engine)

Built on the proven G0–G7 foundation (`openkern-bootstrap-01`, `d06dddf`). Every `PASS`
points to code, tests, and a commit.

## Environment

| | |
|---|---|
| HOST | macOS 26.3.1, arm64 (Apple Silicon) |
| RUST / CARGO | 1.97.1 |
| BASELINE_COMMIT | `d06dddf` (tag `openkern-bootstrap-01`) |
| NEW CRATE | `kern-context` |
| NEW DEPENDENCIES | `syn`, `proc-macro2`, `quote`, `unicode-ident` — all `MIT OR Apache-2.0` (`unicode-ident` also `Unicode-3.0`); `NO_UNKNOWN_LICENSE_DEPENDENCY` holds |

## Gate results (G8)

See `docs/GATES.md` for the full table. Summary: G8.0–G8.9, G8.11–G8.15 = **PASS**;
G8.10 (cache) = **DEFERRED**; scale probe (G8.30) = **PARTIAL**. G0–G7 regression = **0**.

## Metrics (frozen fixture corpus; see CONTEXT_BENCHMARK.md)

| | B0 full | B1 lexical | B2 symbol | **B3 OpenKern** |
|---|---:|---:|---:|---:|
| Recall (task 1) | 1.00 | 0.75 | 0.50 | **1.00** |
| Recall (task 2) | 1.00 | 0.33 | 0.33 | **1.00** |
| UTR (task 1) | 0.03 | 0.50 | 0.38 | **0.55** |
| UTR (task 2) | 0.02 | 1.00 | 1.00 | **0.52** |
| Tokens (est.) | 3265 | 33–161 | 33–128 | **148–176** |

- `RECALL(B3) ≥ RECALL(B1)`, strict in aggregate (2.00 vs 1.08).
- `UTR(B3) > UTR(B0)`; context ≈ 18× smaller than naive full-file.
- Determinism = 100% (identical `pack_hash`); `CROSS_REPOSITORY_LEAK = 0`; `SECRET_LEAK = 0`;
  `TOCTOU` fail-closed; `ZERO_EGRESS` (no networking linked).

## Dogfood (real repo)

`kern context stats .` on OpenKern → 78 files, 671 symbols, 1477 edges. `kern context query
. "transactional git commit"` ranks `crates/kern-git/src/lib.rs` first.

## Test count

`kern-context` = 26 tests; workspace total = **95** (69 baseline + 26). All gates
(`check/test/fmt/clippy -D warnings`) green; `git fsck` clean; worktree clean.

## Known gaps

See `docs/GAPS.md` (file-source candidates, cache, scale curves, cross-language, type-aware
resolution, model ranking, and A11/A14–A16/A20 — all explicitly deferred, none masked).

## Status

`OPENKERN_G8_CONTEXT_ENGINE_PASS` — with the deferred items above recorded as gaps, not
PASS. Provider integration (G9) must not begin before this freeze holds.
