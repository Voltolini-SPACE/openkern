# OpenKern Brand — Provenance & License Audit (BRAND_G10_LEGAL)

Data: 2026-08-12 · Missão OPENKERN-BRAND-01

## Fontes tipográficas

| Ativo | Origem | Licença | Incorporado? |
|---|---|---|---|
| JetBrains Mono | JetBrains (github.com/JetBrains/JetBrainsMono) | SIL OFL 1.1 | NÃO — referenciada por nome; binário não vendorizado |
| Geist | Vercel (github.com/vercel/geist-font) | SIL OFL 1.1 | NÃO — idem |

OFL 1.1 permite uso, redistribuição e embedding, inclusive comercial; proíbe vender
a fonte isolada e exige manter a licença junto do binário quando redistribuído.
Como NÃO redistribuímos binários nesta missão, não há obrigação acionada. No deploy
do site, self-host a partir dos releases upstream e incluir o texto OFL.

## Ativos visuais

Todos os SVGs em `brand/` (símbolos E1/E2/E3, lockups, favicons, avatar, social
preview, sprite de 18 ícones, painéis HTML) são **autorais**, desenhados nesta
missão do zero em geometria própria. Nenhum ícone de biblioteca de terceiros
(Lucide, Feather, Heroicons etc.) foi copiado; semelhanças eventuais em ícones
utilitários (check, folder) decorrem de convenção funcional, não de cópia de path.

## Templates e referências

- Método: skill interna `brandbook-master` (própria).
- Nenhum template externo, foto de stock, imagem gerada por terceiros ou artwork
  licenciado incorporado.

## Auditoria de SVG (§26)

Verificado em todos os SVGs: sem metadata de editor, sem raster embutido, sem
fontes incorporadas, sem `<script>`, sem referência externa (href http). Os
lockups usam `<text>` com font-family por nome + fallback: renderização depende
da fonte instalada; exports PNG/PDF finais devem converter texto em outline no
momento do freeze.

## Separação de licenças

A licença DESTES ativos de marca é decisão do owner e é independente da licença
do código OpenKern (também pendente de decisão do owner). Nada nesta missão
define ou implica licença pública.

VEREDITO: **LICENSE_AUDIT=PASS** (nenhum ativo de terceiro incorporado; fontes
apenas referenciadas, ambas OFL 1.1).
