#!/usr/bin/env python3
"""Writes games/shootout/logo.h from the DSM mark in mnichols08/dsm-legacy (images/dsmlogo.jpeg).

Usage: tools/logo.py path/to/dsmlogo.jpeg. Needs Pillow. The lettering is kept and the line of
small type under it is cropped off: at this size it would be three pixels tall.
"""
import sys
from PIL import Image

W, H = 96, 22
im = Image.open(sys.argv[1]).convert("RGB").crop((60, 180, 1300, 460)).resize((W, H), Image.LANCZOS)
out = "/* The DSM mark, by Mikey Nichols (github.com/mnichols08/dsm-legacy, images/dsmlogo), brought down to\n   %dx%d: 0 is clear, 1 black, 2 red. Made by tools/logo.py. */\n#define LOGO_W %d\n#define LOGO_H %d\nstatic const unsigned char LOGO[LOGO_W * LOGO_H] = {\n" % (W, H, W, H)
for y in range(H):
    row = []
    for x in range(W):
        r, g, b = im.getpixel((x, y))
        row.append(2 if r > 150 and g < 120 and b < 120 else 1 if r + g + b < 330 else 0)
    out += "  " + ",".join(str(v) for v in row) + ",\n"
open("games/shootout/logo.h", "w").write(out + "};\n")
