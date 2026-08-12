# OpenKern Context Engine (G8)

The Context Engine builds the *smallest sufficient* context for a mission: it discovers,
selects, structures, and packages relevant code without becoming an agent, an IDE, or an
LLM wrapper. It is deterministic, bounded, provenance-carrying, and provider-independent
(no model, no network egress).

## Principle

> `RELEVANT CONTEXT > MAXIMUM CONTEXT` · `PROVENANCE > GUESSING` · `BOUNDED > UNLIMITED`
> `DETERMINISTIC > OPAQUE` · `LOCAL-FIRST > SILENT EGRESS` · **`CONTEXT ≠ AUTHORITY`**

A file or symbol appearing in a `ContextPack` grants **no** permission to read, write, or
run anything. Execution still flows through policy → capability → repository identity →
typed exec / transactional git (G2–G6).

## Pipeline

```text
ContextQuery (mission/repo/worktree/text/seeds/budget/freshness)
   │
SymbolIndex (syn-parsed, repo/worktree/revision-bound, content-hashed)
   │
seeds ──► DependencyGraph.bounded_distances(depth)     (graph expansion)
   │
score each symbol: explicit·3 + name·2 + dependency·1.5 + lexical·1 + path·0.5 + test·0.75
   │
sort (score desc, symbol-id asc)  ──►  budget-bounded selection (items/bytes/tokens)
   │
per-item: re-read + re-hash vs index-time hash (TOCTOU) ; attach provenance
   │
ContextPack + canonical pack_hash  (deterministic; pack_id = pack:<hash>)
```

## Crate layout (`kern-context`)

| module | responsibility | gate |
|---|---|---|
| `types` | contracts: ids, `ContextQuery`, `ContextPack`, `Provenance`, budget, events | G8.0 |
| `index` | `syn`-based symbol index + graph construction | G8.1/G8.2 |
| `graph` | typed dependency graph, bounded traversal | G8.2 |
| `scoring` | deterministic tokenization + relevance primitives | G8.4 |
| `engine` | query → score → budget → pack + canonical hash + TOCTOU | G8.3/5/6/8/11/14 |
| `security` | worktree/symlink containment, secret default-deny, ignore policy | G8.9/18/19 |
| `hash` | FNV-1a content hashing (documented, non-cryptographic) | G8.16 |

## CLI

```bash
kern context stats   <path>            # files / symbols / edges
kern context query   <path> <text...>  # ranked ContextPack (add --json)
kern context explain <path> <text...>  # + per-item score breakdown
```

`stats`, `query`, and `explain` resolve the repository identity via `kern-repo`, so the
pack is bound to the real `RepositoryId` / `WorktreeId` / `HEAD`.

## Integration-first (G8.14)

`ContextQuery` / `ContextPack` / `ContextSource` are typed contracts an external
orchestrator (`NOMOS`, `OpenClaw`, `Hermes`, MCP) can consume. `pack_hash` enables audit,
replay, and reproducibility. `NOMOS_REQUIRED = OPENCLAW_REQUIRED = HERMES_REQUIRED = false`.
