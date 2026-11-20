#!/usr/bin/env python3
"""Makes a content pack from a folder.

A pack is everything the engine plays, in one file. The folder holds `pack.json`, which describes
the cars and the places, and the pictures it names, as ordinary PNG files of any size up to the
screen's. A picture named `name@2x.png`, up to 640x480 and even each way, is painted fine: it
goes in as `name.pic` standing half its size in dots, and the screen, which has two dots to each
place it is laid out by, shows every dot of it. Each picture is matched to the engine's 1,024 colours with a dither and stored
run-length; `garage.png` goes in as `garage.pic`, which is the name `pack.json` uses for it.

Usage: tools/pack.py content/default web/default.pack
Needs Pillow and numpy.
"""
import json
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).parent))
from paint import PAL, rle, to_indices  # noqa: E402  (the palette and the dither are paint.py's)


def picture(path, fine=False):
    img = Image.open(path).convert("RGBA")
    most = (640, 480) if fine else (320, 240)
    if img.width > most[0] or img.height > most[1]:
        raise SystemExit(f"{path.name} is {img.width}x{img.height}; the most it can be is {most[0]}x{most[1]}")
    if fine and (img.width % 2 or img.height % 2):
        raise SystemExit(f"{path.name} is {img.width}x{img.height}; a fine picture is even each way")
    # A picture already in the engine's own colours (tools/pixelate.py makes them) goes in as it is.
    a = np.asarray(img)
    seen = np.unique(a[a[..., 3] >= 128][:, :3].astype(np.uint32) @ np.array([65536, 256, 1], np.uint32))
    ours = np.asarray(PAL[32:], np.float32).round().astype(np.uint32) @ np.array([65536, 256, 1], np.uint32)
    words = rle(to_indices(img, dither=0 if np.isin(seen, ours).all() else 8))
    if fine:
        words[0] |= 0x8000
    return struct.pack(f"<{len(words)}H", *words)


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    folder, out = Path(sys.argv[1]), Path(sys.argv[2])
    manifest = (folder / "pack.json").read_bytes()
    json.loads(manifest)  # a pack.json that is not JSON stops here, with the line it went wrong on
    files = {"pack.json": manifest}
    for png in sorted(folder.glob("*.png")):
        fine = png.stem.endswith("@2x")
        name = png.stem.removesuffix("@2x") + ".pic"
        if name in files:
            raise SystemExit(f"{name} is there twice: as {png.name} and as another")
        files[name] = picture(png, fine)
    blob = b"WPK1" + struct.pack("<I", len(files))
    for name, data in files.items():
        raw = name.encode()
        blob += struct.pack("<H", len(raw)) + raw + struct.pack("<I", len(data)) + data
    out.write_bytes(blob)
    print(f"{out}: {len(files)} files, {len(blob)} bytes")


if __name__ == "__main__":
    main()
