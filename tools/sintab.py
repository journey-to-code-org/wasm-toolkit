#!/usr/bin/env python3
"""Writes toolkit/sintab.c: the toolkit has no floats, so its sine comes from a table."""
import math

vals = [round(math.sin(i * math.pi / 512) * 65536) for i in range(257)]
out = "/* A quarter of a sine wave in 16.16, 256 steps to the right angle. Made by tools/sintab.py. */\n#include \"wtk.h\"\n\nconst i32 wtk_sintab[257] = {\n"
for i in range(0, 257, 8):
    out += "  " + ", ".join(str(v) for v in vals[i:i + 8]) + ",\n"
out += "};\n"
open("toolkit/sintab.c", "w").write(out)
