# OPENKERN BRAND v1.0 — FREEZE RECORD

```
MISSION            : OPENKERN-BRAND-02-FREEZE
DATE               : 2026-08-12
BRAND_VERSION      : 1.0
SYMBOL_PRIMARY     : E2 Prompt [>▮]
TAGLINE            : Governed execution for AI agents.
PALETTE            : acromática + estados semânticos (ALLOW/ASK/DENY/INFO)
BRAND_TAG          : openkern-brand-v1.0 (anotada; aponta para o commit deste freeze)
PUSH               : NOT_PUSHED (sem remote)
PUBLICATION        : NOT_PUBLISHED (missão separada: OPENKERN-BRAND-03-PUBLISH)
```

## 1. Contexto do desentrelaçamento

Durante OPENKERN-BRAND-01 (marca) rodou em paralelo uma sessão G8 de produto
(intencional, confirmada pelo owner). O `git add -A` da sessão G8 absorveu os
artefatos de marca em commits de produto. Nenhum conteúdo foi perdido ou alterado:
os 31 arquivos `brand/` + `docs/BRAND_BOOK.md` chegaram ao HEAD íntegros
(verificado por fingerprint contra o estado autoral).

## 2. Genealogia real (forense)

| Path | Owner pretendido | Introduzido em | Estado no freeze | Ação |
|---|---|---|---|---|
| `brand/strategy/` `logo/` `color/` `tokens/` `icons/` `cli/` `github/` `web/` `typography/` | BRAND-01 | `dc5f2fa` (G8 core, varredura acidental) | íntegro | ownership documentado aqui |
| `brand/legal/` `brand/README.md` `docs/BRAND_BOOK.md` | BRAND-01 | `f2d3b8f` (G8 freeze, varredura acidental) | íntegro | ownership documentado aqui |
| `crates/kern-context/` · mods `kern-cli` · `docs/CONTEXT_*.md` · `Cargo.*` | G8 (produto) | `dc5f2fa` / `f2d3b8f` | legítimo | nenhuma |
| arquivos mistos (marca+produto no mesmo arquivo) | — | — | **NENHUM** | — |

## 3. Decisão de desentrelaçamento

Os commits `dc5f2fa` e `f2d3b8f` estão sob a tag de produto selada
`openkern-g8-context-01`. Reescrever essa história invalidaria o selo do produto —
o maior blast radius possível. A correção adotada é a mais conservadora:

1. **História preservada** — nenhum rewrite, nenhum reset, nenhum revert.
2. **Este commit é exclusivamente de marca** (paths adicionados explicitamente,
   nunca `add -A`) e contém as decisões finais do owner (E2, paleta, tagline, v1.0).
3. **Declaração formal de ownership:** os paths `brand/**` e `docs/BRAND_BOOK.md`
   pertencem à linhagem de missões OPENKERN-BRAND-*, independentemente do commit
   que primeiro os introduziu. A referência autoritativa da marca é a tag
   `openkern-brand-v1.0`, não os commits G8 que acidentalmente os carregaram.
4. **Prova de não-interferência:** hash agregado SHA-256 de `crates/` + manifests
   idêntico antes/depois desta missão; suíte Rust completa, fmt e clippy verdes
   (evidência na seção 4).

## 4. Evidência de não-interferência no produto

Preenchido no fechamento da missão (valores reais, computados):

```
PRODUCT_AGGREGATE_SHA256_BEFORE : 27d2d202450ee45e4b7c4f9d5e05084b092099ae443f7a8b109d73533188a824
PRODUCT_AGGREGATE_SHA256_AFTER  : 27d2d202450ee45e4b7c4f9d5e05084b092099ae443f7a8b109d73533188a824
PRODUCT_UNCHANGED               : TRUE (45 arquivos: crates/ + 6 manifests/docs raiz)
CARGO_TEST                      : RC=0 · 95 passed · 0 failed (workspace completo)
CARGO_FMT_CHECK                 : RC=0
CARGO_CLIPPY                    : RC=0 (contrato do workspace: clippy all=deny, pedantic=warn)
BRAND_FREEZE_MUTATED_PRODUCT_CODE = FALSE
```

## 5. Governança pós-freeze

- Mudança em logo, cores primárias, naming, tagline ou linguagem visual ⇒ v1.x
  com nova aprovação explícita do owner (BRAND_BOOK §27).
- Registro no cofre (`~/Documents/BRANDBOOKS_OFICIAIS/REGISTRO_DE_MARCAS.md`)
  executado nesta missão por ordem explícita do owner.
- Publicação (push, GitHub, site voltolini.space) exige OPENKERN-BRAND-03-PUBLISH.
