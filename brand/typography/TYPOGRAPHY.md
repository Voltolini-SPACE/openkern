# OpenKern — Typography

> BRAND_G5_TYPOGRAPHY. Mono-as-identity: the display face IS the mono face.

## System

```
DISPLAY_FONT : JetBrains Mono 700       (headlines, wordmark, states)
BODY_FONT    : Geist 400/500            (long-form docs, website body)
MONO_FONT    : JetBrains Mono 400/700   (code, CLI, labels, data)
FALLBACKS    : ui-monospace, SF Mono, Menlo, Consolas, monospace
               system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial, sans-serif
```

Two families total. A third face is prohibited: restraint is an attribute, and the
mono-display is the identity. Where Geist cannot load, system sans is acceptable;
where JetBrains Mono cannot load, ui-monospace is acceptable — the identity survives
fallback because it is structural (spacing, weight, state labels), not letterform-exotic.

## Licenses (audited — §29)

| Face | License | Verdict |
|---|---|---|
| JetBrains Mono | SIL OFL 1.1 (JetBrains) | PASS — free for any use, redistribution allowed under OFL |
| Geist | SIL OFL 1.1 (Vercel) | PASS — same terms |

Font binaries are NOT vendored into this repo (mission §12: do not distribute font
files unnecessarily). Websites self-host from the upstream releases at build time;
provenance is recorded in `brand/legal/PROVENANCE.md`.

## Scale

px at 16px root · ratio ≈ 1.26 above body:

```
12   meta, uppercase labels (tracking 0.12em)
13.5 captions, table data
15   UI body
17   docs body (Geist)
21   h3
27   h2
34   h1
44   display (hero only)
```

- Body line-height 1.6; headings 1.25; `text-wrap: balance` on headings.
- Running text ≤ 68ch.
- Numbers in tables/dashboards: `font-variant-numeric: tabular-nums`.
- Uppercase happens only in mono at 12px with 0.12em tracking (labels, states).
- The wordmark is always lowercase `openkern` (mono; `open` 400 + `kern` 700).
  In prose the product is written "OpenKern". The binary is `kern`.

## Voice in type

State words (`ALLOW`, `ASK`, `DENY`, `REFUSED`, `VERIFIED`, `EXECUTED`) are always
set in mono 700, uppercase, with their state color AND their icon or prefix glyph.
Never bold-sans, never italic, never color-only.
