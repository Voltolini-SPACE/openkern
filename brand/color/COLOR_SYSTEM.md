# OpenKern — Color System

> BRAND_G4_COLOR. Palette own to OpenKern; no sibling reuse (SE7EN cyan, EPISTEMOS
> indigo/amber, NOMOS green all avoided). Every number in this file is computed by
> `brand/color/generate_color_doc.py`, not typed by hand. EVIDENCE > CLAIMS.

## Principle: color is state, not decoration

The OpenKern mark is achromatic (currentColor). The identity's grounds and inks are
green-biased neutrals (a quiet nod to terminal heritage without phosphor-green cliché).
Chromatic color appears ONLY to encode state:

- ALLOW / success · green
- ASK / warning · amber
- DENY / danger · red
- INFO · desaturated blue

There is no decorative accent. Interactive elements use ink + underline/weight/border,
with INFO blue available for links in long-form documents. State is never encoded by
color alone (see §23: icons, labels and shape always accompany color).

## Dark theme (canonical)

| Token | HEX | RGB | HSL | Contrast vs bg / surface |
|---|---|---|---|---|
| background | `#0C0F0E` | rgb(12, 15, 14) | hsl(160, 11%, 5%) |  |
| surface | `#131715` | rgb(19, 23, 21) | hsl(150, 10%, 8%) |  |
| surface-2 | `#1A1F1C` | rgb(26, 31, 28) | hsl(144, 9%, 11%) |  |
| border | `#242B28` | rgb(36, 43, 40) | hsl(154, 9%, 15%) |  |
| border-strong | `#333B37` | rgb(51, 59, 55) | hsl(150, 7%, 22%) |  |
| text | `#E9EDEA` | rgb(233, 237, 234) | hsl(135, 10%, 92%) | 16.3 / 15.3 |
| text-dim | `#A8B3AD` | rgb(168, 179, 173) | hsl(147, 7%, 68%) | 8.91 / 8.37 |
| text-mute | `#7B877F` | rgb(123, 135, 127) | hsl(140, 5%, 51%) | 5.15 / 4.84 |
| allow (success) | `#57B87B` | rgb(87, 184, 123) | hsl(142, 41%, 53%) | 7.84 / 7.36 |
| ask (warning) | `#D9A03F` | rgb(217, 160, 63) | hsl(38, 67%, 55%) | 8.3 / 7.79 |
| deny (danger) | `#DE685E` | rgb(222, 104, 94) | hsl(5, 66%, 62%) | 5.73 / 5.38 |
| info | `#6FA8C4` | rgb(111, 168, 196) | hsl(200, 42%, 60%) | 7.41 / 6.95 |

## Light theme

| Token | HEX | RGB | HSL | Contrast vs bg / surface |
|---|---|---|---|---|
| background | `#F4F5F4` | rgb(244, 245, 244) | hsl(120, 5%, 96%) |  |
| surface | `#FFFFFF` | rgb(255, 255, 255) | hsl(0, 0%, 100%) |  |
| surface-2 | `#EBEEEC` | rgb(235, 238, 236) | hsl(140, 8%, 93%) |  |
| border | `#D8DDDA` | rgb(216, 221, 218) | hsl(144, 7%, 86%) |  |
| border-strong | `#BFC7C2` | rgb(191, 199, 194) | hsl(142, 7%, 76%) |  |
| text | `#141715` | rgb(20, 23, 21) | hsl(140, 7%, 8%) | 16.52 / 18.05 |
| text-dim | `#4A544E` | rgb(74, 84, 78) | hsl(144, 6%, 31%) | 7.2 / 7.87 |
| text-mute | `#687269` | rgb(104, 114, 105) | hsl(126, 5%, 43%) | 4.58 / 5.0 |
| allow (success) | `#22794A` | rgb(34, 121, 74) | hsl(148, 56%, 30%) | 4.92 / 5.38 |
| ask (warning) | `#7F630D` | rgb(127, 99, 13) | hsl(45, 81%, 27%) | 5.2 / 5.69 |
| deny (danger) | `#B03A31` | rgb(176, 58, 49) | hsl(4, 56%, 44%) | 5.5 / 6.01 |
| info | `#2E6E8E` | rgb(46, 110, 142) | hsl(200, 51%, 37%) | 5.14 / 5.62 |

## WCAG summary (computed)

- text: 16.3:1 dark · 16.52:1 light — AAA
- text-dim: 8.91:1 dark · 7.2:1 light — AAA/AA
- text-mute: 5.15:1 dark · 4.58:1 light — AA (meta text; on surface-2 treat as large-text only)
- All four state colors ≥ 4.5:1 on background and surface in both themes — AA for normal text.
- Focus ring: ink (`text`) at 2px offset 2px — achromatic, theme-proof.

## Usage rules

1. Never introduce a new hue. New needs must map to an existing state or neutral.
2. Never use state colors decoratively (no green headers, no amber highlights).
3. DENY red is reserved for denial/refusal/failure. It never means "hot" or "new".
4. Display-P3: not used in v1.0. sRGB values are canonical.
5. Dark is the canonical theme (terminal heritage); light is first-class, not derived.
