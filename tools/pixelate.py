#!/usr/bin/env python3
"""Turns a photo or any picture into one the engine will draw exactly as you see it.

The engine has 1,024 colors and a 320 by 240 screen. This cuts a picture to the right shape,
brings it down to size, matches it to those colors, and saves a PNG. What the PNG shows is what
the game shows: drop it into a pack folder (tools/pack.py) or into the workshop.

    tools/pixelate.py bay.jpg --as bay                 ->  bay.pixel.png, 320 x 176
    tools/pixelate.py garage.jpg --as room --out content/mine/garage.png
    tools/pixelate.py filter.png --size 42x52          a part, the size of its place under the hood
    tools/pixelate.py car.png --as car --width 244     a car from the side; its background must be clear

  --as       bay 320x176, room 320x200, cockpit 320x240, rear 96 wide, wheel 176 square, car 244 wide
  --size     WxH, for anything else
  --width    for car and rear: how wide, keeping the picture's shape
  --fit      how to make the shape: cover (fill it and cut what hangs over, the default),
             contain (all of the picture, with clear bars), stretch
  --focus    with cover, which part to keep: center (default), top, bottom, left, right
  --dither   0 to 16: how much the colors are stippled into one another (default 6; 0 is flat)
  --pop      1.0 leaves the colors alone; 1.2 or so gives a photo the punch of painted art
  --sharpen  crispen edges after shrinking a photo
  --big N    also save <name>.xN.png, blown up N times, to look at
"""
import argparse
import os
import sys

import numpy as np
from PIL import Image, ImageEnhance, ImageFilter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import paint  # noqa: E402  the palette, and the matching the pack tool uses

KINDS = {"bay": (320, 176), "room": (320, 200), "cockpit": (320, 240), "wheel": (176, 176), "car": (244, None), "rear": (96, None)}


def shape(im, w, h, fit, focus):
    """The picture made w by h."""
    if h is None:
        return im.resize((w, max(1, round(im.height * w / im.width))), Image.LANCZOS)
    if fit == "stretch":
        return im.resize((w, h), Image.LANCZOS)
    scale = (max if fit == "cover" else min)(w / im.width, h / im.height)
    sized = im.resize((max(1, round(im.width * scale)), max(1, round(im.height * scale))), Image.LANCZOS)
    out = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    fx = {"left": 0.0, "right": 1.0}.get(focus, 0.5)
    fy = {"top": 0.0, "bottom": 1.0}.get(focus, 0.5)
    out.paste(sized, (round((w - sized.width) * fx), round((h - sized.height) * fy)))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("picture")
    ap.add_argument("--as", dest="kind", choices=sorted(KINDS))
    ap.add_argument("--size")
    ap.add_argument("--width", type=int)
    ap.add_argument("--fit", choices=["cover", "contain", "stretch"], default="cover")
    ap.add_argument("--focus", choices=["center", "centre", "top", "bottom", "left", "right"], default="center")
    ap.add_argument("--dither", type=float, default=6)
    ap.add_argument("--pop", type=float, default=1.0)
    ap.add_argument("--sharpen", action="store_true")
    ap.add_argument("--big", type=int, default=0)
    ap.add_argument("--out")
    a = ap.parse_args()

    if a.size:
        w, h = (int(v) for v in a.size.lower().split("x"))
    elif a.kind:
        w, h = KINDS[a.kind]
    else:
        ap.error("say what it is for with --as, or how big with --size WxH")
    if a.width:
        w, h = a.width, (None if h is None else round(h * a.width / w))
    if w > 320 or (h or 0) > 240:
        ap.error("the screen is 320 by 240; nothing can be bigger")

    im = shape(Image.open(a.picture).convert("RGBA"), w, h, a.fit, a.focus)
    if im.height > 240:
        ap.error(f"at {w} wide this picture is {im.height} tall, and the screen is 240. Give a smaller --width")
    if a.pop != 1.0:
        alpha = im.getchannel("A")
        rgb = ImageEnhance.Contrast(ImageEnhance.Color(im.convert("RGB")).enhance(a.pop)).enhance(1 + (a.pop - 1) / 2)
        im = Image.merge("RGBA", (*rgb.split(), alpha))
    if a.sharpen:
        alpha = im.getchannel("A")
        im = Image.merge("RGBA", (*im.convert("RGB").filter(ImageFilter.UnsharpMask(radius=1, percent=90, threshold=2)).split(), alpha))

    ix = paint.to_indices(im, dither=a.dither)
    clear = ix == 0xFFFF
    rgba = np.zeros((*ix.shape, 4), np.uint8)
    rgba[..., :3] = np.asarray(paint.PAL, np.uint8)[np.where(clear, 0, ix)]
    rgba[..., 3] = np.where(clear, 0, 255)
    out = a.out or os.path.splitext(a.picture)[0] + ".pixel.png"
    done = Image.fromarray(rgba, "RGBA")
    done.save(out)
    print(f"{out}: {done.width} x {done.height}, {len(np.unique(ix[~clear]))} of the engine's colors")
    if a.big > 1:
        big = os.path.splitext(out)[0] + f".x{a.big}.png"
        done.resize((done.width * a.big, done.height * a.big), Image.NEAREST).save(big)
        print(big)


if __name__ == "__main__":
    main()
