#!/usr/bin/env python3
"""Generate the Canglang app-icon SVG source: a wood-yellow chess-coin mark with the
hand-drawn 沧 glyph (演示夏行楷) rendered as a self-contained vector outline."""
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.ttLib import TTFont

FONT = "/tmp/slidexiaxing.ttf"
CHAR = "沧"          # U+6CA7
SIZE = 1024          # viewBox
GLYPH_TARGET = 648   # glyph ink height in the viewBox
RED = "#b32632"      # exact red-side colour (--side-red) for glyph and rim
DY = 32              # nudge the glyph up for optical centering (px)

font = TTFont(FONT)
gs = font.getGlyphSet()
cmap = font.getBestCmap()
glyph_name = cmap[ord(CHAR)]

glyph = gs[glyph_name]
bp = BoundsPen(gs)
glyph.draw(bp)
x_min, y_min, x_max, y_max = bp.bounds
gw, gh = x_max - x_min, y_max - y_min

sp = SVGPathPen(gs)
glyph.draw(sp)
d = sp.getCommands()

scale = GLYPH_TARGET / max(gw, gh)
tx = SIZE / 2 - scale * (x_min + x_max) / 2
ty = SIZE / 2 + scale * (y_min + y_max) / 2 - DY   # flip Y for SVG, then nudge up

svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {SIZE} {SIZE}" role="img" aria-label="沧浪象棋">
  <defs>
    <radialGradient id="wood" cx="50%" cy="34%" r="80%">
      <stop offset="0%" stop-color="#f2d49b"/>
      <stop offset="100%" stop-color="#d3a25c"/>
    </radialGradient>
  </defs>

  <!-- wood-yellow board surface -->
  <rect width="{SIZE}" height="{SIZE}" fill="url(#wood)"/>

  <!-- piece-coin rim (opaque) -->
  <circle cx="{SIZE/2}" cy="{SIZE/2}" r="430" fill="none" stroke="{RED}" stroke-width="24"/>

  <!-- hand-drawn 沧 -->
  <path d="{d}" transform="translate({tx:.2f},{ty:.2f}) scale({scale:.4f},{-scale:.4f})" fill="{RED}"/>
</svg>
'''

out = "design/icon-source.svg"
import os

os.makedirs("design", exist_ok=True)
with open(out, "w", encoding="utf-8") as f:
    f.write(svg)
print("wrote", out)
print("glyph_name", glyph_name, "bounds", (round(x_min), round(y_min), round(x_max), round(y_max)),
      "gw", round(gw), "gh", round(gh), "scale", round(scale, 4))
