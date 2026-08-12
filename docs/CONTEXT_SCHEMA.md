# Context Schema (G8.0, G8.36)

Versioned by `engine_version` (`kern-context/0.1.0`). A breaking change to selection or
serialization must bump this string; `pack_hash` changes with it, so drift is detectable.

## `ContextPack`

| field | meaning |
|---|---|
| `pack_id` | `pack:<pack_hash>` — deterministic id |
| `mission_id` / `repository_id` / `worktree_id` / `revision` | binding |
| `query_text` | echo of the query |
| `items[]` | selected `ContextItem`s, deterministic order |
| `total_bytes` / `total_estimated_tokens` | size accounting |
| `budget` | the `ContextBudget` that shaped selection |
| `candidates_considered` | number scored before budget |
| `engine_version` | schema/selection version |
| `pack_hash` | canonical FNV-1a over `engine_version + mission + repo + worktree + query + [path|symbol|content_hash]*` — **excludes** non-deterministic fields |

## `ContextItem`

| field | meaning |
|---|---|
| `content` | the selected source slice (re-read & hash-verified at pack time) |
| `provenance` | `{ source, file, path, revision, symbol?, content_hash }` |
| `score` | `ScoreComponents { lexical, symbol_match, dependency, path_proximity, explicit, test_relevance }` |
| `estimated_tokens` / `bytes` | size |
| `reason` | human-readable selection justification |

## Determinism contract (G8.11/G8.22)

For the same `repo state + revision + query + engine_version`, the engine produces the same
ranking, the same selected items, and the same `pack_hash`. Non-deterministic values (e.g.
wall-clock timestamps) are excluded from the canonical hash.
