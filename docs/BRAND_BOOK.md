# OpenKern Brand Book

```
BRAND_VERSION      : 1.0-rc (aguardando aprovação do owner para freeze)
BASE_PRODUCT       : openkern-bootstrap-01 · commit d06dddf
MISSION            : OPENKERN-BRAND-01
SOURCE_OF_TRUTH    : este arquivo + brand/ neste repositório
PUBLICATION        : PROHIBITED_UNTIL_OWNER_APPROVAL
```

---

## 1. Introdução

Este documento é a fonte única da identidade OpenKern. Ele descreve a estratégia,
a identidade verbal e visual, os tokens de produto e as regras de governança da
marca. Assets vivem em `brand/`; este livro os referencia e os rege.

O princípio que governa tudo:

```
EVIDENCE > CLAIMS
EXPLICIT AUTHORITY > IMPLICIT TRUST
CONTROL > MAGIC
SYSTEM > GIMMICK
```

## 2. Brand strategy

Ver `brand/strategy/00_STRATEGY.md` (normativo). Resumo: agentes de IA ganharam o
poder de agir sem ganhar autoridade legítima para isso. OpenKern é a camada onde a
intenção do agente vira ação governada ou não vira nada.

**Promessa:** toda ação passa por autoridade explícita, delimitada e verificável,
ou não executa. **Proibido prometer:** "100% seguro", "unhackable", "zero risk".

## 3. Positioning

Categoria própria: **Governed Execution Kernel for AI Agents**. Não é chatbot, IDE,
wrapper de LLM, CLI genérico nem "mais um framework de agente". É o substrato de
execução abaixo deles: default-deny, capabilities de uso único, execução tipada,
Git transacional, evidência auditável.

## 4. Audience

Seis personas (infra, security, platform builder, enterprise governance, open-source
dev, dev individual avançado) com necessidade, risco, promessa, objeção e linguagem
por persona em `brand/strategy/00_STRATEGY.md` §5.

## 5. Messaging

- Primária: **Governed execution for AI agents.**
- Secundária: **No implicit authority. No unverified action.**
- Reserva: **Evidence over claims.**
- Descritor técnico: Governed Execution Kernel for AI Agents.

## 6. Voice

Técnica, precisa, controlada, calma. Sete regras (§8 do strategy doc):
verdade plana, voz ativa, especificidade, nunca segurança absoluta, contenção,
sem travessão público e sem emoji em superfícies de produto/segurança, estado como
vocabulário de primeira classe. O kernel não comemora.

## 7. Naming

Nome validado: **OpenKern** (open + kernel; "kern" também acena à precisão
tipográfica). Wordmark desenhado em caixa baixa `openkern`; em prosa, "OpenKern";
o binário é `kern`. Arquitetura de extensões (hipóteses): Core, Runtime, Policy,
Capabilities, CLI, SDK, Cloud, Desktop. Renomear exige autorização explícita.

## 8. Tagline

Sistema completo com 24 opções avaliadas em `brand/strategy/00_STRATEGY.md` §7.
Selecionadas: ver §5 acima.

## 9. Logo

Direção aprovada pelo owner (12/08/2026): **E · Boundary Mono**. Um glifo monoline:
colchetes de fronteira em volta de um caret de prompt. Kernel boundary e terminal
no mesmo traço.

Variações (Etapa 5, escolha do owner pendente):
- **E1 Pure** `[ > ]` — a mais quieta (impresso, contextos formais)
- **E2 Prompt** `[ >▮]` — com cursor de bloco (recomendada como primária)
- **E3 Gate** — bracket direito com abertura única (narrativa de capability)

Lockups: horizontal (`lockup_horizontal.svg`), vertical (`lockup_vertical.svg`),
símbolo isolado, wordmark isolado, mono preto, mono branco.
Fontes em `brand/logo/refinement/`.

## 10. Construction

- Grid: viewBox 64; traço 4 (ratio 1:16); caps e joins arredondados.
- Cursor de bloco: 7×12 no grid 64, radius 1.
- Favicon 32: redesenho com traço 6. Favicon 16: caret + cursor apenas
  (brackets saem abaixo de 24 px, por regra).
- O símbolo é `currentColor`: nunca tem cor própria.

## 11. Clear space

Respiro mínimo em todos os lados = altura do caret (14/64 do tile ≈ 22% da altura
do símbolo). Em avatares circulares, padding de 12,5% do tile (512 → glifo 384).
Tamanho mínimo: símbolo completo 24 px; abaixo disso usar a variante favicon-16.

## 12. Misuse

Proibido: aplicar cor ao símbolo além de tinta/estado; gradiente; glow; sombra
projetada no glifo; rotação; itálico; outline duplo; recolorir o cursor separado;
usar o wordmark em sans; capitalizar o wordmark (`OpenKern` só em prosa); animar
qualquer coisa além do blink do cursor.

## 13. Colors

Fonte normativa: `brand/color/COLOR_SYSTEM.md` (valores computados por script).
Princípio: **cor é estado, não decoração**. Marca acromática; neutros com viés
verde (herança de terminal sem clichê phosphor); cromático só para ALLOW (verde),
ASK (âmbar), DENY (vermelho), INFO (azul dessaturado). Sem accent decorativo.
Dark é canônico; light é primeira classe. Todos os tokens de texto e estado ≥4.5:1
sobre background e surface nos dois temas (AA; texto principal AAA).

## 14. Typography

`brand/typography/TYPOGRAPHY.md`. Duas famílias, ambas OFL 1.1:
**JetBrains Mono** (display + mono + identidade) e **Geist** (corpo longo).
Terceira família proibida. Escala 12/13.5/15/17/21/27/34/44. Wordmark lowercase.

## 15. Iconography

`brand/icons/icons.svg` — 18 ícones (capability, policy, allow, ask, deny,
repository, worktree, execution, sandbox, audit, evidence, mission, agent, runtime,
network, filesystem, terminal, security + refused). Grid 24, traço 2, radius 2,
caps arredondados, `currentColor`. Preenchimento só em pontos de estado/cursor.

## 16. Visual language

Sem depender do logo: superfícies planas, hairlines, grids visíveis quando
estruturais, tipografia mono como textura, blocos de terminal como elemento
gráfico, trilhas de auditoria como listas de estado. Proibido: AI-gradient roxo,
cérebros, robôs, neural-net stock art, matrix rain, glow-as-security.

## 17. Motion

Movimento comunica estado, não decoração. Tokens em `brand/tokens/tokens.json`:
120/180/240 ms, easing `cubic-bezier(.2,0,0,1)`. Transições de estado são cortes
ou fades curtos; aprovação não "celebra", negação não "treme". Assinatura única:
o blink do cursor de bloco (1.1 s step-end). `prefers-reduced-motion` é respeitado
(motion NÃO é identidade do OpenKern: parar o blink não apaga a marca).

## 18. UI tokens

`brand/tokens/tokens.json` (máquina) + `brand/tokens/tokens.css` (humano/browser).
Cobrem color.*, type.*, spacing.*, radius.*, shadow.*, border.*, motion.*, state.*.
Nenhuma UI runtime foi introduzida no kernel nesta missão.

## 19. CLI

`brand/cli/CLI_IDENTITY.md` — especificação apenas (CLI real congelado em d06dddf).
Estado primeiro, prefixos de glifo redundantes à cor (`[+] [?] [-] [x] [=] [>] [!]`),
sem spinner que esconda estado, sem emoji, erros dizem o que NÃO foi executado.

## 20. GitHub

`brand/github/GITHUB_BRAND.md` + `avatar.svg` (512, crop circular seguro) +
`social_preview.svg` (1280×640). Badges só com lastro em evidência. Publicação
proibida até aprovação do owner; repo sem remote.

## 21. Documentation

Estados de maturidade com selo visual obrigatório em docs:

```
PROVEN        verde  · evidência anexada e reproduzível
SUPPORTED     ink    · mantido, testado em CI
EXPERIMENTAL  âmbar  · pode mudar sem aviso
UNSUPPORTED   cinza  · fora de contrato
BLOCKED       âmbar  · dependência fail-closed pendente
DEPRECATED    cinza  · saída programada
```

Selo = label mono + ícone + cor (nunca cor sozinha). Reflete EVIDENCE_OVER_CLAIMS:
nenhuma seção PROVEN sem artefato.

## 22. Website

Direção + página construída: `brand/web/openkern_page/index.html` (PT-BR, padrão
voltolini.space com hub-bar, dark/light com toggle persistente, 4 breakpoints,
validador APROVADO). Estrutura: hero com demo de terminal real, por-que, quatro
órgãos, vocabulário de estados, evidência com números do baseline congelado,
integração-como-contrato, CTA. Publicar SOMENTE após aprovação do owner.

## 23. Architecture diagrams

Convenções para diagramas OpenKern:
- **Shapes:** órgãos do kernel = retângulos radius 8; externos = retângulos
  tracejados; evidência = documento com hash-lines; agente = brackets.
- **Setas:** fluxo permitido = sólida; ASK = tracejada com nó ◆; DENY = terminada
  em barra ⊣ (nunca chega ao destino); privilegiada = traço duplo.
- **Trust boundaries:** moldura tracejada com rótulo mono no canto superior esquerdo.
- Cores dos estados apenas; resto acromático.

## 24. Integration diagrams

Externos (NOMOS, Hermes, OpenClaw, MCP, LLMs, tools, APIs, SDKs, webhooks, events,
streams) desenham-se FORA da boundary, ligados por setas de contrato (rotuladas com
o contrato negociado). Nunca desenhar dependência obrigatória entre produtos:
OpenKern compõe, não se subordina.

## 25. Accessibility

- Contraste: computado e registrado em `brand/color/COLOR_SYSTEM.md` (AA em tudo,
  AAA no texto principal).
- Daltonismo: estado nunca é só cor (glifo + label sempre).
- Reduced motion: respeitado; única animação é o blink, que degrada para estático.
- Monocromático/grayscale: marca é acromática por construção; estados mantêm
  prefixos `[+] [?] [-]`.
- Touch targets ≥44 px na web; foco visível acromático (2 px, offset 2).

## 26. Applications

Mockups representativos (demonstração, não produto): página web
(`brand/web/openkern_page/`), terminal (hero da página), GitHub preview
(`brand/github/social_preview.svg`), avatar, favicons, painel de exploração
(`brand/logo/exploration/PANEL_directions.html`). Sticker dev: símbolo E2 mono
branco em vinil escuro, sem tagline.

## 27. Governance

```
BRAND_VERSION       : 1.0-rc → 1.0 no freeze (tag openkern-brand-v1.0, local)
SOURCE_OF_TRUTH     : docs/BRAND_BOOK.md + brand/ neste repo (após registro no
                      cofre ~/Documents/BRANDBOOKS_OFICIAIS/, o cofre prevalece)
BRAND_CHANGE_POLICY : mudança em logo, cores primárias, naming, tagline ou
                      linguagem visual exige nova versão formal (v1.x) com
                      aprovação explícita do owner
ASSET_NAMING        : snake_case, prefixo ok- em ids de símbolo, SVG é fonte
DEPRECATION_POLICY  : asset deprecado move para brand/_deprecated/ com nota
APPROVAL_POLICY     : freeze, promoção a _VIGENTE e registro no cofre são ações
                      humanas (LEI_DA_MARCA art. 7.2); agentes não congelam marca
```

### Legal & license audit (§29)

Ver `brand/legal/PROVENANCE.md`. Resumo: JetBrains Mono OFL 1.1; Geist OFL 1.1;
todos os SVGs autorais desta missão; nenhum template, ícone ou imagem de terceiros
incorporado; binários de fonte não vendorizados. Licença de marca ≠ licença de
código (decisão do owner, pendente).

### Originality review (§30)

Comparado contra OpenAI, Anthropic, GitHub, Rust, Kubernetes, Docker, HashiCorp,
OpenCode, OpenClaw, NOMOS, EPISTEMOS: nenhum usa brackets+caret+cursor monoline
como marca; paleta acromática-com-estados não colide com nenhum irmão do
portfólio (SE7EN cyan, EPISTEMOS indigo/âmbar, NOMOS verde-neon, CONFRAPAG
azul/verde). Prompt-glyph existe como *categoria* em dev-tools (powershell `>_`),
mas a construção boundary-brackets + caret + cursor-de-bloco monoline com regra
acromática é composição própria. `ORIGINAL_IDENTITY=TRUE` com a ressalva honesta
acima registrada.
