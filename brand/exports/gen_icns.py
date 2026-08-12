#!/usr/bin/env python3
"""Gera o appicon.icns do OpenKern Panel a partir da geometria congelada E2.
Reusa o desenho de gen_icons.py; produz iconset 16..1024 e roda iconutil.

Uso: python3 gen_icns.py <saida.icns>"""
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_icons import make  # noqa: E402

# nomes canônicos do iconutil: (arquivo, pixels)
ENTRIES = [
    ("icon_16x16.png", 16), ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32), ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128), ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256), ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512), ("icon_512x512@2x.png", 1024),
]

if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "appicon.icns"
    with tempfile.TemporaryDirectory() as td:
        iconset = os.path.join(td, "appicon.iconset")
        os.makedirs(iconset)
        for name, px in ENTRIES:
            make(px, 0.80, 0.1875, os.path.join(iconset, name))
        subprocess.run(["iconutil", "-c", "icns", iconset, "-o", out], check=True)
    print("icns:", out)
