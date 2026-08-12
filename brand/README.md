# brand/ — OpenKern Brand System

Fonte dos ativos de marca OpenKern. Regido por `docs/BRAND_BOOK.md`.
Versão: **1.0 CONGELADA** (tag `openkern-brand-v1.0` · símbolo primário **E2 Prompt [>▮]**
· tagline "Governed execution for AI agents." · freeze: `docs/BRAND_FREEZE_v1.0.md`).

```
brand/
├── README.md               este arquivo
├── strategy/               estratégia, posicionamento, personas, taglines, voz
├── logo/
│   ├── exploration/        5 direções (A–E) + painel de decisão HTML
│   └── refinement/         direção E aprovada: E1/E2/E3, lockups, mono, favicons
├── color/                  COLOR_SYSTEM.md (computado) + generate_color_doc.py
├── typography/             TYPOGRAPHY.md (JetBrains Mono + Geist, OFL)
├── icons/                  icons.svg — sprite com 18 símbolos grid-24
├── tokens/                 tokens.json (máquina) + tokens.css (browser)
├── cli/                    CLI_IDENTITY.md (especificação; CLI real congelado)
├── github/                 avatar.svg · social_preview.svg (1280×640) · guia
├── web/openkern_page/      página de apresentação (PT-BR, validador APROVADO)
└── legal/                  PROVENANCE.md (auditoria de licenças e proveniência)
```

Regras rápidas:
- O símbolo é `currentColor`. Cor é estado (`ALLOW/ASK/DENY/INFO`), nunca decoração.
- Wordmark: `openkern` lowercase, JetBrains Mono, `kern` em 700. Em prosa: OpenKern.
- Nada aqui altera `crates/` (CODE_MUTATION=PROHIBITED nesta missão).
- Publicação/push/release: proibidos até autorização explícita do owner.
