# OpenKern — Refinamento da direção aprovada: E · Boundary Mono

> Etapa 4 do método. Direção E aprovada pelo owner em 12/08/2026 ("prossiga").
> Conceito: um glifo monoline — colchetes de fronteira em volta de um caret de prompt.
> Kernel boundary e terminal no mesmo traço. Honesto, unix, quieto.

## Decisões de refinamento

1. **Acromático por princípio.** O símbolo não tem cor própria: é tinta sobre fundo
   (`currentColor`). Toda cor no sistema OpenKern codifica **estado**, nunca decoração.
   Isso torna a marca imune a dark/light e a impressão 1-cor por construção.
2. **Wordmark em mono, caixa baixa:** `openkern`, JetBrains Mono, com `kern` em peso 700
   (`open` regular 400). A grafia em prosa continua "OpenKern"; o wordmark desenhado é
   lowercase, como convém à herança unix. O binário é `kern`.
3. **Grid:** viewBox 64, traço 4 (ratio 1:16), terminações e junções arredondadas,
   respiro interno mínimo de 8 unidades. Clear space externo = altura do caret (1×).
4. **Três variações para a segunda aprovação:**

| Variação | Ideia | Diferença |
|---|---|---|
| **E1 Pure** | `[ > ]` | brackets + caret, como no estudo original. A mais quieta. |
| **E2 Prompt** | `[ >▮]` | caret + cursor de bloco. O cursor é o único elemento que pisca no motion system: o kernel está vivo, aguardando ordem explícita. |
| **E3 Gate** | `[ > ]` com fresta | o bracket direito tem uma abertura (o gate): nada sai da fronteira exceto pela passagem declarada. Liga o glifo à história de capability boundary. |

5. **Favicon:** redesenho dedicado (não redução automática): traço 6/64 e geometria
   apertada para 32 px e 16 px.
6. **Legibilidade pequena:** testado no painel a 16/24/32/48 px; brackets permanecem
   legíveis porque o glifo é 100% monoline sem detalhe interno fino.

## Recomendação

**E2 Prompt** como símbolo primário: o cursor de bloco dá massa ao glifo em tamanhos
pequenos (favicon/avatar) e carrega o único motion sancionado da marca (blink).
E1 fica como variante "quiet" para contextos impressos; E3 como variante narrativa
para diagramas de fronteira.

## Riscos residuais

- Minimalismo exige espaçamento rigoroso: o clear space é regra dura, não sugestão.
- Em avatares circulares (GitHub), os brackets tangenciam a borda: usar container com
  padding 12.5% (tile 512 → glifo 384).
