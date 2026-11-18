#!/usr/bin/env python3
"""Makes a content pack from a folder.

A pack is everything the engine plays, in one file. The folder holds `pack.json`, which describes
the cars and the places, and the pictures it names, as ordinary PNG files of any size up to the
screen's. Each picture is matched to the engine's 1,024 colours with a dither and stored
run-length; `garage.png` goes in as `garage.pic`, which is the name `pack.json` uses for it.

Usage: tools/pack.py content/default web/default.pack
Needs Pillow and numpy.
"""
import json
import struct
import sys
from pathlib import Path

from PIL import Image

sys.path.insert(0, str(Path(__file__).parent))
from paint import rle, to_indices  # noqa: E402  (the palette and the dither are paint.py's)


def picture(path):
    img = Image.open(path).convert("RGBA")
    if img.width > 320 or img.height > 240:
        raise SystemExit(f"{path.name} is {img.width}x{img.height}; the screen is 320x240")
    words = rle(to_indices(img, dither=8))
    return struct.pack(f"<{len(words)}H", *words)


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    folder, out = Path(sys.argv[1]), Path(sys.argv[2])
    manifest = (folder / "pack.json").read_bytes()
    json.loads(manifest)  # a pack.json that is not JSON stops here, with the line it went wrong on
    files = {"pack.json": manifest}
    for png in sorted(folder.glob("*.png")):
        files[png.stem + ".pic"] = picture(png)
    blob = b"WPK1" + struct.pack("<I", len(files))
    for name, data in files.items():
        raw = name.encode()
        blob += struct.pack("<H", len(raw)) + raw + struct.pack("<I", len(data)) + data
    out.write_bytes(blob)
    print(f"{out}: {len(files)} files, {len(blob)} bytes")


if __name__ == "__main__":
    main()
