#!/usr/bin/env python3
"""OpenKern app icon generator — rasteriza o símbolo E2 Prompt [>▮] (geometria
congelada do favicon-32, brand v1.0) em PNGs para PWA/apple-touch.

Uso: python3 gen_icons.py <outdir>
Gera: icon-192.png, icon-512.png, icon-maskable-512.png, apple-touch-icon.png (180)
Tile #0C0F0E radius 18.75%, glifo #E9EDEA, traço 6/64, caps redondos (círculos)."""
import sys, os
from PIL import Image, ImageDraw

TILE = "#0C0F0E"
INK = "#E9EDEA"

# geometria base (viewBox 64, redraw favicon-32: traço 6)
SEGS = [  # polilinhas: brackets + caret
    [(21, 14), (12, 14), (12, 50), (21, 50)],
    [(43, 14), (52, 14), (52, 50), (43, 50)],
    [(25, 24), (35, 32), (25, 40)],
]
CURSOR = (39, 26, 8, 13, 1.5)  # x y w h r
STROKE = 6.0


def draw_glyph(img, scale, cx, cy, glyph_scale):
    """Desenha o glifo centrado em (cx,cy) com fator glyph_scale (1.0 = full)."""
    d = ImageDraw.Draw(img)
    s = scale * glyph_scale
    ox, oy = cx - 32 * s, cy - 32 * s
    w = STROKE * s
    r = w / 2
    for seg in SEGS:
        pts = [(ox + x * s, oy + y * s) for x, y in seg]
        d.line(pts, fill=INK, width=round(w), joint="curve")
        for p in (pts[0], pts[-1]):  # caps redondos
            d.ellipse([p[0] - r, p[1] - r, p[0] + r, p[1] + r], fill=INK)
    x, y, cw, ch, cr = CURSOR
    d.rounded_rectangle(
        [ox + x * s, oy + y * s, ox + (x + cw) * s, oy + (y + ch) * s],
        radius=cr * s, fill=INK)


def make(size, glyph_scale, radius_pct, out):
    ss = 4  # supersample p/ antialias
    S = size * ss
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([0, 0, S - 1, S - 1], radius=round(S * radius_pct), fill=TILE)
    draw_glyph(img, S / 64.0, S / 2, S / 2, glyph_scale)
    img = img.resize((size, size), Image.LANCZOS)
    img.save(out, "PNG", optimize=True)
    print(f"{out}  {size}x{size}  glyph={glyph_scale}")


if __name__ == "__main__":
    outdir = sys.argv[1] if len(sys.argv) > 1 else "."
    os.makedirs(outdir, exist_ok=True)
    make(192, 0.80, 0.1875, os.path.join(outdir, "icon-192.png"))
    make(512, 0.80, 0.1875, os.path.join(outdir, "icon-512.png"))
    # maskable: glifo reduzido para a zona segura (círculo de 80% do tile), tile full-bleed
    make(512, 0.58, 0.0, os.path.join(outdir, "icon-maskable-512.png"))
    # apple-touch: iOS arredonda sozinho — tile full-bleed sem transparência
    make(180, 0.80, 0.0, os.path.join(outdir, "apple-touch-icon.png"))
