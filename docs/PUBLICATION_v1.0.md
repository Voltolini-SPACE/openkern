# OPENKERN — PUBLICATION RECORD v1.0

```
MISSION            : OPENKERN-BRAND-03-PUBLISH
DATE               : 2026-08-12
PRODUCT_FREEZE     : openkern-g8-context-01 → f2d3b8f (imutável)
BRAND_FREEZE       : openkern-brand-v1.0 → 2ff6631 (imutável)
DOCUMENTARY_COMMITS: 8f9f7ab (MIT license, public README, page publication deltas)
                     + fix white-space terminal demo
REPOSITORY         : https://github.com/Voltolini-SPACE/openkern (public, main)
RELEASE            : https://github.com/Voltolini-SPACE/openkern/releases/tag/openkern-brand-v1.0
WEBSITE            : https://voltolini.space/openkern (GitHub Pages, repo voltolini.space)
LICENSE            : MIT (decisão do owner nesta missão; deps auditadas: syn,
                     proc-macro2, quote, unicode-ident — MIT OR Apache-2.0 [+Unicode-3.0])
```

## Evidência de publicação

- Push integrity: peeled tags remotas == locais (brand→2ff6631, g8→f2d3b8f).
- Clean-clone público: fmt RC=0 · clippy -D warnings RC=0 · **95/95 testes**.
- Site vivo: HTTP 200, HTTP/2, TLS OK, **SHA-256 servido == SHA-256 do repo**;
  canonical/OG/favicon presentes; zero assets externos; links site↔GitHub nos
  dois sentidos; validadores (check.py + brandbook) OK/APROVADO.
- Visual: dark + light (toggle persistente), mobile 375 / tablet 768 / desktop,
  foco de teclado visível, zero erros de console, reduced-motion respeitado.
- Rollback: revert range reproduz árvore pré-deploy exata (tree hash idêntico);
  estado anterior = site `7f3a005`, rota /openkern = 404.
- Não-interferência: crates/ + Cargo.lock + toolchain byte-idênticos ao freeze;
  única mudança de manifest = linha `license` (ordem do owner);
  NOMOS/EPISTEMOS/Hermes inalterados (HEAD/dirty/mtime iguais ao baseline).

## Postura pública

Core local-first **obrigatório**; providers externos são **adapters opcionais**.
Nenhuma claim além do provado. Estados: ALLOW · ASK · DENY · REFUSED · VERIFIED.
EVIDENCE > CLAIMS.
