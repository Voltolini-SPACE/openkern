# Context Engine Security (G8.9, G8.17–G8.19, G8.38)

Context retrieval is a security boundary. The engine treats the repository as untrusted and
is fail-closed.

## Boundaries enforced (each with an adversarial test)

| Threat | Defense | Test |
|---|---|---|
| Cross-repository leak (A1/A18) | a query must target the indexed `RepositoryId`/`WorktreeId`; else `RepositoryMismatch`. The engine holds one index, so another repo's symbols are structurally unreachable. | `cross_repository_query_is_denied` |
| `../` traversal (A3) | `safe_join` rejects parent components up front | `traversal_and_absolute_paths_refused` |
| Absolute out-of-worktree path (A4) | `safe_join` rejects absolute inputs | same |
| Symlink escape (A2) | indexer never follows symlinks; `safe_join` canonicalizes and requires worktree containment | `symlink_escape_is_refused` |
| Secret material (A7) | `sensitive_reason` default-deny for `.env`, keys (`.pem/.key/...`), credential stores, `.ssh`/`.aws`/`.gnupg` — skipped at index and refused on access | `secrets_are_never_indexed` |
| Stale / TOCTOU (A5/A6/A13) | every selected item is re-read and re-hashed against the **index-time** hash; mismatch ⇒ `StaleContent` (fail-closed) | `stale_content_toctou_is_detected` |
| Deleted-after-index (A13) | read fails ⇒ error, never silent | `deleted_file_after_index_fails_closed` |
| Malformed source (A12) | parse error is caught; file kept for lexical use, no panic | `malformed_source_does_not_crash_index` |
| Missing provenance (A17) | an item with empty file/hash provenance is refused (`MissingProvenance`) | enforced in `build_pack` |
| Ambiguous names (A8) | unique-name-only edge resolution; ambiguous names get no invented edges | `ambiguous_symbol_names_do_not_invent_edges` |
| Budget bypass (A10) | deterministic selection stops at `max_items/max_bytes/max_estimated_tokens` | `budget_bounds_selection` |
| Binary/huge (A9/A19) | binary extensions skipped at index | `is_binary_like` |

## Zero egress (G8.38)

The engine performs **no network I/O**: it uses only `std::fs` reads within the worktree
and pure computation. There is no socket, no HTTP client, no provider call anywhere in
`kern-context`. `NETWORK_EGRESS = 0` by construction (no networking API is linked).

## `.gitignore` is not a security boundary (G8.20)

`is_ignored_for_relevance` (skips `.git`, `target/`, vendor caches) is a *relevance*
filter, kept strictly separate from `sensitive_reason` (the security policy). Ignored files
are neither automatically secret nor automatically safe.
