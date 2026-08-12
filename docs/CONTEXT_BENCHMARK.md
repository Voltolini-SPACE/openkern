# Context Engine Benchmark (G8.12, §25–28)

Deterministic benchmark on a frozen fixture corpus with explicit ground truth (see
`crates/kern-context/src/fixture.rs` and `src/tests_benchmark.rs`). No model, no network.

## Method

- **Corpus**: a small project with known relationships — `run_mission` (exec.rs) calls
  `RepositoryIdentity::resolve` and `.head()` (repo.rs) — plus distractor bulk (3 × 20
  unrelated functions) and secret decoys.
- **Baselines**: `B0` full files (naive), `B1` lexical-only (query overlap on snippet
  text), `B2` symbol-name-only, `B3` OpenKern engine (name + lexical + graph + path).
- **Ground truth**: the relevant symbol-id set per task.
- **Metrics**: Recall@K, Precision@K, MRR, tokens, bytes, and **UTR** (Useful Token Ratio
  = tokens of relevant symbols covered ÷ total selected tokens). Tokens are estimated as
  `bytes/4` (documented; no tokenizer dependency), so all token figures are `ESTIMATED`.

## Results (K=8, host: macOS 26.3.1 / arm64)

| Task | Baseline | Recall | Precision | MRR | Tokens (est.) | UTR |
|---|---|---:|---:|---:|---:|---:|
| find symbol + neighbours | B0 full | 1.00 | 0.18 | 0.33 | 3265 | 0.03 |
| | B1 lexical | 0.75 | 0.50 | 1.00 | 161 | 0.50 |
| | B2 symbol | 0.50 | 0.40 | 1.00 | 128 | 0.38 |
| | **B3 OpenKern** | **1.00** | **0.57** | **1.00** | 176 | **0.55** |
| find callees of run_mission | B0 full | 1.00 | 0.18 | 0.33 | 3265 | 0.02 |
| | B1 lexical | 0.33 | 1.00 | 1.00 | 33 | 1.00 |
| | B2 symbol | 0.33 | 1.00 | 1.00 | 33 | 1.00 |
| | **B3 OpenKern** | **1.00** | 0.60 | 1.00 | 148 | **0.52** |

## Superiority criteria (§28) — asserted in `benchmark_openkern_beats_baselines`

- `Recall@K(B3) ≥ Recall@K(B1)` on every task, and **strictly greater in aggregate**
  (B3 = 2.00 vs B1 = 1.08). Lexical/symbol baselines miss graph-reachable relevant symbols
  (e.g. `head`, or the callees of `run_mission`); the graph recovers them.
- `UTR(B3) > UTR(B0)` on every task (≈ 18× the naive full-file UTR).
- Context size materially lower than naive: `bytes(B3) × 2 ≤ bytes(B0)` (measured ≈ 18×
  fewer tokens than full-file).
- Deterministic replay = 100% (identical `pack_hash`); cross-repo leak = 0; secret leak = 0.

## Honesty caveats

- Numbers are from a **small fixture**, not a production repo — they demonstrate the
  *mechanism*, not production-scale performance. Scale behaviour (index time / memory /
  latency at larger corpora) is a documented gap (see `GAPS.md`).
- The precision/recall trade-off is real and visible: on task 2 the lexical baseline has
  higher precision but far lower recall; B3 trades a little precision to recover the
  graph-reachable set, which is the point of the engine.
- Tokens are estimated, not tokenizer-exact.
