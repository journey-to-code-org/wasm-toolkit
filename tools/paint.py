#!/usr/bin/env python3
"""Paints the game's pictures and writes them out as C.

The old games were painted, not drawn by the program: a picture for the dash, with the needles
moving over it. This does the same. Each picture is built here at four times its size, with
gradients, grain, soft shadows and real lettering, brought down to 320x240, and matched to the
toolkit's 1,024 colours with an ordered dither. What comes out is run-length C arrays of palette
indices (0xFFFF is clear).

Usage: tools/paint.py [--png DIR]   writes games/shootout/art.h; --png also saves each picture.
Needs Pillow and numpy. The palette here must match wtk_pal_default in toolkit/wtk.c.
"""
import math
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

W, H, S = 320, 240, 4
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
rng = np.random.default_rng(7)

# ---- The palette, as the toolkit makes it ----------------------------------------------------

BASES = [
    (0, 0, 0), (150, 150, 150), (214, 40, 40), (236, 120, 28), (240, 206, 48), (150, 212, 40), (44, 170, 60), (30, 160, 140),
    (40, 190, 214), (110, 170, 240), (48, 90, 214), (76, 60, 190), (136, 70, 200), (206, 60, 180), (240, 130, 170), (130, 82, 44),
    (200, 170, 120), (166, 80, 58), (168, 164, 156), (74, 76, 84), (70, 130, 50), (96, 140, 170), (186, 196, 208), (46, 46, 50),
    (150, 100, 56), (236, 224, 192), (30, 46, 110), (120, 24, 40), (30, 84, 50), (212, 170, 60), (176, 180, 188), (236, 236, 240),
]


def palette():
    pal = np.zeros((1024, 3), dtype=np.float32)
    for ramp in range(1, 32):
        for s in range(32):
            for k in range(3):
                v = BASES[ramp][k]
                pal[ramp * 32 + s, k] = v * (26 + s * 230 // 24) // 256 if s <= 24 else v + (255 - v) * (s - 24) // 10
    return pal


PAL = palette()
BAYER = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]], dtype=np.float32) / 16 - 0.5


def to_indices(img, dither=14):
    """An RGBA picture at game size, as palette indices, dithered. Clear pixels are 0xFFFF."""
    a = np.asarray(img.convert("RGBA"), dtype=np.float32)
    h, w = a.shape[:2]
    rgb = a[..., :3] + np.tile(BAYER, (h // 4 + 1, w // 4 + 1))[:h, :w, None] * dither
    out = np.full((h, w), 0xFFFF, dtype=np.uint16)
    use = PAL[32:]
    for y in range(h):
        d = ((rgb[y][:, None, :] - use[None, :, :]) ** 2 * np.array([0.9, 1.2, 0.7])).sum(axis=2)
        out[y] = d.argmin(axis=1) + 32
    out[a[..., 3] < 128] = 0xFFFF
    return out


def rle(ix):
    flat = ix.reshape(-1)
    out = [ix.shape[1], ix.shape[0]]
    i = 0
    while i < len(flat):
        j = i
        while j < len(flat) and flat[j] == flat[i] and j - i < 65535:
            j += 1
        out += [j - i, int(flat[i])]
        i = j
    return out


def c_array(name, numbers):
    lines = [f"static const unsigned short {name}[] = {{"]
    for i in range(0, len(numbers), 24):
        lines.append("  " + ",".join(str(n) for n in numbers[i:i + 24]) + ",")
    lines.append("};")
    return "\n".join(lines)


# ---- Painting, at four times the size ---------------------------------------------------------

def canvas(w=W, h=H):
    return Image.new("RGBA", (w * S, h * S), (0, 0, 0, 0))


def mask_of(size, draw):
    m = Image.new("L", size, 0)
    draw(ImageDraw.Draw(m))
    return m


def poly(pts):
    return lambda d: d.polygon([(x * S, y * S) for x, y in pts], fill=255)


def disc(cx, cy, r):
    return lambda d: d.ellipse([(cx - r) * S, (cy - r) * S, (cx + r) * S, (cy + r) * S], fill=255)


def rrect(x, y, w, h, r=2):
    return lambda d: d.rounded_rectangle([x * S, y * S, (x + w) * S, (y + h) * S], radius=r * S, fill=255)


def fill(img, shape, top, bottom=None, y0=None, y1=None, across=False, grain=0, soften=0):
    """Fill a shape with a gradient (down it, or across it), with grain in it."""
    m = mask_of(img.size, shape)
    if soften:
        m = m.filter(ImageFilter.GaussianBlur(soften * S))
    box = m.getbbox()
    if not box:
        return
    bottom = bottom or top
    w, h = img.size
    if across:
        a, b = (y0 * S, y1 * S) if y0 is not None else (box[0], box[2])
        t = np.clip((np.arange(w) - a) / max(1, b - a), 0, 1)[None, :, None]
        t = np.broadcast_to(t, (h, w, 1))
    else:
        a, b = (y0 * S, y1 * S) if y0 is not None else (box[1], box[3])
        t = np.clip((np.arange(h) - a) / max(1, b - a), 0, 1)[:, None, None]
        t = np.broadcast_to(t, (h, w, 1))
    col = np.array(top, dtype=np.float32) * (1 - t) + np.array(bottom, dtype=np.float32) * t
    if grain:
        col = col + rng.normal(0, grain, (h // S + 1, w // S + 1)).repeat(S, 0).repeat(S, 1)[:h, :w, None]
    layer = Image.fromarray(np.clip(col, 0, 255).astype(np.uint8), "RGB").convert("RGBA")
    img.paste(layer, (0, 0), m)


def shade(img, shape, colour, alpha, blur=2, dx=0, dy=0):
    """A soft shadow or a soft light, laid over what is there."""
    m = mask_of(img.size, shape).filter(ImageFilter.GaussianBlur(blur * S))
    if dx or dy:
        m = Image.fromarray(np.roll(np.asarray(m), (int(dy * S), int(dx * S)), (0, 1)))
    m = m.point(lambda v: int(v * alpha))
    img.paste(Image.new("RGBA", img.size, colour + (255,)), (0, 0), m)


def line(img, pts, colour, width=1.0):
    ImageDraw.Draw(img).line([(x * S, y * S) for x, y in pts], fill=colour + (255,), width=max(1, int(width * S)), joint="curve")


def text(img, x, y, s, size, colour):
    f = ImageFont.truetype(FONT, int(size * S))
    ImageDraw.Draw(img).text((x * S, y * S), s, font=f, fill=colour + (255,), anchor="mm")


def finish(img):
    return img.resize((img.size[0] // S, img.size[1] // S), Image.LANCZOS)


def polar(cx, cy, r, turn):
    """A point r out from (cx, cy); turn 0 is straight up, 0.25 is to the right."""
    a = turn * 2 * math.pi
    return cx + math.sin(a) * r, cy - math.cos(a) * r


# ---- A dial ----------------------------------------------------------------------------------

SWEEP = 800 / 1024  # the needle's travel, as a part of a turn; it starts at -400/1024


def dial_face(img, cx, cy, r, marks, numbers, red_from=None, lit=False, small=5.5):
    ink = (255, 150, 60) if lit else (236, 236, 232)
    red = (255, 60, 40)
    fill(img, disc(cx, cy, r + 2.2), (96, 98, 106), (30, 30, 34))
    fill(img, disc(cx, cy, r + 1), (6, 6, 8))
    fill(img, disc(cx, cy, r), (40, 14, 12) if lit else (26, 26, 30), (16, 5, 5) if lit else (10, 10, 12))
    for i in range(marks * 2 + 1):
        turn = -400 / 1024 + SWEEP * i / (marks * 2)
        c = red if red_from is not None and i / 2 >= red_from else ink
        a, b = polar(cx, cy, r - (4.5 if i % 2 == 0 else 2.6), turn), polar(cx, cy, r - 0.8, turn)
        line(img, [a, b], c, 0.9 if i % 2 == 0 else 0.5)
    if red_from is not None:
        box = [(cx - r + 1.4) * S, (cy - r + 1.4) * S, (cx + r - 1.4) * S, (cy + r - 1.4) * S]
        start = -90 + (-400 / 1024 + SWEEP * red_from / marks) * 360
        ImageDraw.Draw(img).arc(box, start, -90 + (-400 / 1024 + SWEEP) * 360, fill=red + (255,), width=int(1.2 * S))
    for i, label in numbers:
        turn = -400 / 1024 + SWEEP * i / marks
        x, y = polar(cx, cy, r - (8.2 if r > 15 else 6.2), turn)
        text(img, x, y, label, small, red if red_from is not None and i >= red_from else ink)


def cluster(lit):
    """The dials behind the wheel: a tach, a speedometer, and three small ones between them."""
    img = canvas()
    fill(img, rrect(54, 132, 134, 64, 5), (8, 8, 10), (3, 3, 4))
    dial_face(img, *TACH, 9, [(i, str(i)) for i in range(1, 9)], red_from=7, lit=lit, small=4.8)
    dial_face(img, *SPEEDO, 8, [(i, str(i * 20)) for i in (1, 3, 5, 7)], lit=lit, small=4.4)
    text(img, TACH[0], TACH[1] + 11, "RPM", 3.4, (255, 150, 60) if lit else (150, 150, 150))
    text(img, SPEEDO[0], SPEEDO[1] + 11, "MPH", 3.4, (255, 150, 60) if lit else (150, 150, 150))
    for (x, y), tag in zip(MINIS, ["TEMP", "FUEL"]):
        dial_face(img, x, y, MINI_R, 2, [], lit=lit)
        text(img, x, y + 3.6, tag, 2.3, (255, 150, 60) if lit else (150, 150, 150))
    if lit:
        glow = img.filter(ImageFilter.GaussianBlur(1.2 * S))
        img = Image.alpha_composite(Image.blend(img, glow, 0.35), Image.new("RGBA", img.size, (0, 0, 0, 0)))
    return img


# Where the live parts go. The game reads these from art.h.
TACH = (95, 170, 21)
SPEEDO = (145, 170, 21)
MINIS = [(120, 151), (120, 189)]
MINI_R = 6.5
BOOST = (27, 96, 12)
WHEEL = (120, 216)
WHEEL_R = 84
GATE = (286, 224)


def cockpit(lit=False):
    img = canvas()
    dark, mid, lite = (22, 22, 25), (50, 51, 57), (92, 94, 104)

    # The roof lining across the top, the visors, and the mirror.
    fill(img, poly([(0, 0), (320, 0), (320, 8), (0, 10)]), (46, 46, 50), (20, 20, 23), grain=2)
    fill(img, rrect(40, 5, 84, 9, 3), (74, 74, 80), (40, 40, 44), grain=3)
    fill(img, rrect(204, 5, 84, 8, 3), (70, 70, 76), (38, 38, 42), grain=3)
    fill(img, poly([(156, 6), (164, 6), (163, 12), (157, 12)]), dark)
    fill(img, rrect(126, 10, 68, 19, 5), (34, 34, 38), (12, 12, 14))
    fill(img, rrect(129, 12.5, 62, 14, 3.5), (150, 176, 204), (86, 104, 128))
    shade(img, poly([(134, 13), (150, 13), (142, 26), (130, 26)]), (255, 255, 255), 0.22, 1.5)

    # The pillar by your shoulder, with a gauge in a pod on it, and the thin one across the car.
    fill(img, poly([(0, 0), (36, 0), (13, 152), (0, 152)]), (26, 26, 30), (70, 70, 78), across=True, grain=2)
    shade(img, poly([(31, 0), (36, 0), (13, 152), (9, 152)]), (150, 152, 164), 0.5, 1)
    shade(img, poly([(0, 0), (8, 0), (4, 152), (0, 152)]), (0, 0, 0), 0.6, 3)
    fill(img, poly([(14, 78), (30, 74), (44, 92), (40, 112), (22, 116), (10, 104)]), (58, 58, 64), (24, 24, 27), grain=2)
    shade(img, disc(BOOST[0] + 1, BOOST[1] + 2, BOOST[2] + 3), (0, 0, 0), 0.6, 2)
    dial_face(img, BOOST[0], BOOST[1], BOOST[2], 6, [(0, "0"), (2, "10"), (4, "20"), (6, "30")], red_from=5, small=3.3)
    text(img, BOOST[0], BOOST[1] + 6, "PSI", 2.6, (150, 150, 150))
    fill(img, poly([(290, 0), (320, 0), (320, 142), (309, 142)]), (60, 60, 66), (20, 20, 23), across=True, grain=2)
    shade(img, poly([(290, 0), (294, 0), (312, 142), (309, 142)]), (130, 132, 142), 0.45, 1)

    # The top of the dash, rolling away under the glass, with the defroster slots in it.
    top = [(0, 152), (12, 137), (60, 129), (150, 126), (250, 127), (312, 133), (320, 138), (320, 240), (0, 240)]
    fill(img, poly(top), (66, 67, 74), (24, 24, 27), y0=126, y1=240, grain=3)
    shade(img, poly([(12, 137), (60, 129), (150, 126), (250, 127), (312, 133), (312, 136), (250, 130), (150, 129), (60, 132), (12, 140)]), (150, 152, 164), 0.55, 1.2)
    for i in range(7):
        line(img, [(206 + i * 13, 131 + i * 0.5), (215 + i * 13, 131.4 + i * 0.5)], (12, 12, 14), 1.1)
    shade(img, poly([(0, 200), (320, 200), (320, 240), (0, 240)]), (0, 0, 0), 0.55, 14)

    # The hood over the dials, and the dials in its shadow.
    brow = [(42, 152), (50, 126), (62, 115), (120, 111), (180, 114), (192, 124), (202, 152)]
    shade(img, poly(brow), (0, 0, 0), 0.5, 4, 0, 3)
    fill(img, poly(brow), (58, 59, 66), (20, 20, 23), y0=111, y1=150, grain=3)
    shade(img, poly([(62, 115), (120, 111), (180, 114), (180, 116.5), (120, 113.5), (62, 117.5)]), (160, 162, 174), 0.6, 0.9)
    fill(img, rrect(52, 130, 138, 68, 7), (4, 4, 5))
    img = Image.alpha_composite(img, cluster(False))
    dials = cluster(True) if lit else None
    shade(img, poly([(52, 130), (190, 130), (190, 142), (52, 142)]), (0, 0, 0), 0.55, 4)

    # The column and its stalks, behind the wheel.
    fill(img, poly([(98, 196), (142, 196), (150, 240), (90, 240)]), (40, 40, 45), (14, 14, 16), grain=2)
    for sx, tip in ((98, 62), (142, 178)):
        line(img, [(sx, 203), (tip, 195)], (18, 18, 20), 4.5)
        line(img, [(sx, 202), (tip, 194)], (62, 62, 68), 1.2)
        fill(img, disc(tip, 195, 3.2), (70, 70, 76), (26, 26, 30))

    # The door down your left side: an armrest, switches, and a speaker.
    door = [(0, 152), (46, 158), (40, 240), (0, 240)]
    fill(img, poly(door), (84, 85, 92), (40, 40, 45), grain=4)
    shade(img, poly([(42, 158), (46, 158), (40, 240), (36, 240)]), (0, 0, 0), 0.5, 2)
    fill(img, poly([(0, 186), (34, 190), (32, 204), (0, 202)]), (56, 56, 62), (30, 30, 34), grain=3)
    shade(img, poly([(0, 186), (34, 190), (34, 192), (0, 188)]), (140, 142, 152), 0.5, 0.8)
    for i in range(3):
        fill(img, rrect(5 + i * 9, 192.5 + i, 7, 5, 1), (20, 20, 23), (44, 44, 50))
    fill(img, disc(17, 224, 12), (34, 34, 38), (22, 22, 25))
    for a in range(0, 360, 20):
        for r in (3, 6.5, 10):
            x, y = polar(17, 224, r, a / 360)
            fill(img, disc(x, y, 0.7), (6, 6, 8))

    # The centre stack, turned to face you: vents, the heater, the radio, and the gate for the shifter.
    stack = [(200, 142), (304, 147), (312, 240), (196, 240)]
    shade(img, poly(stack), (0, 0, 0), 0.5, 3, -2, 0)
    fill(img, poly(stack), (72, 73, 80), (34, 34, 38), grain=3)
    shade(img, poly([(200, 142), (203, 142), (199, 240), (196, 240)]), (160, 162, 174), 0.5, 0.8)
    shade(img, poly([(200, 142), (304, 147), (304, 149), (200, 144)]), (160, 162, 174), 0.5, 0.8)
    for vx, vy in ((208, 149), (255, 151.5)):
        fill(img, rrect(vx, vy, 41, 17, 2.5), (8, 8, 10))
        for i in range(5):
            fill(img, poly([(vx + 2, vy + 2.4 + i * 3), (vx + 39, vy + 2.4 + i * 3), (vx + 39, vy + 4.2 + i * 3), (vx + 2, vy + 4.2 + i * 3)]), (70, 70, 76), (30, 30, 34))
        fill(img, rrect(vx + 18.5, vy + 1, 4, 15, 1), (20, 20, 23), (46, 46, 52))
    for i, tint in enumerate(((70, 120, 220), (200, 200, 205), (220, 70, 60))):
        kx, ky = 221 + i * 28, 181 + i * 1.3
        shade(img, disc(kx + 1, ky + 2, 10), (0, 0, 0), 0.55, 2)
        fill(img, disc(kx, ky, 10), (26, 26, 30), (14, 14, 16))
        box = [(kx - 8.4) * S, (ky - 8.4) * S, (kx + 8.4) * S, (ky + 8.4) * S]
        ImageDraw.Draw(img).arc(box, 150, 390, fill=tint + (255,), width=int(1.1 * S))
        fill(img, disc(kx, ky, 6), (96, 98, 106), (38, 38, 43))
        line(img, [(kx, ky), polar(kx, ky, 5.4, -0.1 + i * 0.12)], (236, 236, 232), 1)
    fill(img, rrect(207, 198, 88, 21, 2), (10, 10, 12), (4, 4, 5))
    fill(img, rrect(210, 201, 42, 9, 1), (20, 44, 26), (12, 30, 18))
    text(img, 231, 205.6, "FM 105.3", 5, (140, 230, 120))
    for i in range(6):
        fill(img, rrect(210 + i * 7.2, 212.5, 6, 4.5, 0.8), (64, 64, 70), (30, 30, 34))
    for kx in (263, 284):
        fill(img, disc(kx, 208.5, 6.5), (86, 88, 96), (30, 30, 34))
        fill(img, disc(kx, 208.5, 3.4), (30, 30, 34), (60, 60, 66))
    # The shifter: a boot and a knob, and beside it the gate, where the game lights the gear.
    fill(img, rrect(GATE[0] - 15, GATE[1] - 13, 30, 27, 3), (10, 10, 12), (5, 5, 6))
    for c in (-8, 0, 8):
        line(img, [(GATE[0] + c, GATE[1] - 7), (GATE[0] + c, GATE[1] + (0 if c == 8 else 7))], (120, 122, 130), 1)
    line(img, [(GATE[0] - 8, GATE[1]), (GATE[0] + 8, GATE[1])], (120, 122, 130), 1)
    shade(img, poly([(214, 240), (256, 240), (244, 220), (228, 220)]), (0, 0, 0), 0.6, 3)
    fill(img, poly([(218, 240), (254, 240), (241, 222), (231, 222)]), (30, 30, 34), (10, 10, 12), grain=3)
    for i in range(4):
        line(img, [(224 + i * 2.6, 238 - i * 4), (249 - i * 2.4, 238 - i * 4)], (52, 52, 58), 0.6)
    fill(img, disc(236, 217, 8), (20, 20, 23))
    fill(img, disc(235.4, 216.2, 6.8), (110, 112, 122), (30, 30, 34))
    shade(img, disc(233.4, 213.6, 2.2), (255, 255, 255), 0.6, 1)
    if lit:
        # The same cabin with the lights off: everything falls away to blue-black, and the dials glow.
        a = np.asarray(img, dtype=np.float32).copy()
        a[..., :3] = a[..., :3] * np.array([0.26, 0.28, 0.36]) + np.array([1, 2, 5])
        img = Image.fromarray(np.clip(a, 0, 255).astype(np.uint8), "RGBA")
        img = Image.alpha_composite(img, dials)
        shade(img, rrect(46, 200, 150, 30, 8), (255, 110, 40), 0.07, 8)
    return img


def wheel():
    """The steering wheel, square on, to be turned by the game: a leather rim, three spokes, a pad."""
    n = (WHEEL_R + 4) * 2
    img = canvas(n, n)
    c = n / 2
    yy, xx = np.mgrid[0:n * S, 0:n * S].astype(np.float32) / S
    r = np.hypot(xx - c, yy - c)
    turn = np.arctan2(xx - c, c - yy)
    rin, rout = WHEEL_R - 11.5, WHEEL_R
    # The rim: round in section, lit from the upper left, with a line of stitching inside it.
    across = np.clip((r - rin) / (rout - rin), 0, 1)
    tube = np.sin(across * math.pi)
    light = 0.55 + 0.45 * np.cos(turn + 0.7)
    v = 16 + tube * (34 + 70 * light * tube ** 3)
    rim = np.stack([v * 0.92, v * 0.92, v * 1.12, np.where((r >= rin) & (r <= rout), 255, 0)], axis=-1)
    stitch = (np.abs(r - (rin + 2.4)) < 0.35) & ((turn * 60).astype(np.int32) % 2 == 0)
    rim[stitch, :3] = (120, 120, 140)
    img = Image.alpha_composite(img, Image.fromarray(np.clip(rim, 0, 255).astype(np.uint8), "RGBA"))
    # The spokes: one each side, drooping a little, and a broad one down to six o'clock.
    spoke = canvas(n, n)
    for sgn in (-1, 1):
        pts = [(c + sgn * 18, c - 13), (c + sgn * (rin + 1), c - 3), (c + sgn * (rin + 1), c + 17), (c + sgn * 20, c + 15)]
        shade(spoke, poly(pts), (0, 0, 0), 0.5, 2, 0, 2)
        fill(spoke, poly(pts), (62, 62, 76), (24, 24, 30), grain=2)
        shade(spoke, poly([pts[0], pts[1], (pts[1][0], pts[1][1] + 2.5), (pts[0][0], pts[0][1] + 2.5)]), (150, 150, 172), 0.45, 0.8)
    down = [(c - 15, c + 14), (c + 15, c + 14), (c + 11, c + rin + 1), (c - 11, c + rin + 1)]
    fill(spoke, poly(down), (48, 48, 60), (20, 20, 26), grain=2)
    fill(spoke, rrect(c - 7, c + 34, 14, 22, 3), (14, 14, 18), (26, 26, 32))
    # The pad in the middle.
    pad = [(c - 27, c - 15), (c + 27, c - 15), (c + 22, c + 19), (c - 22, c + 19)]
    shade(spoke, poly(pad), (0, 0, 0), 0.6, 3, 0, 2)
    fill(spoke, mask_shape_round(pad, 7), (70, 70, 86), (30, 30, 38), grain=2)
    shade(spoke, poly([(c - 24, c - 14), (c + 24, c - 14), (c + 22, c - 9), (c - 22, c - 9)]), (170, 170, 196), 0.35, 1.5)
    for sgn in (-1, 1):
        fill(spoke, rrect(c + sgn * 15 - 3, c + 9, 6, 4, 1), (20, 20, 26), (44, 44, 54))
    img = Image.alpha_composite(spoke, img)
    return img


def mask_shape_round(pts, radius):
    """A polygon with its corners rounded off."""
    def draw(d):
        m = Image.new("L", d.im.size, 0)
        ImageDraw.Draw(m).polygon([(x * S, y * S) for x, y in pts], fill=255)
        m = m.filter(ImageFilter.GaussianBlur(radius * S / 2)).point(lambda v: 255 if v > 127 else 0)
        d.bitmap((0, 0), m, fill=255)
    return draw


def picture(path, box, size):
    """A picture someone painted, cut to `box` and brought down to `size`. It is dithered already, so it
    is averaged down and matched to the palette with only a little more."""
    return Image.open(path).convert("RGBA").crop(box).resize(size, Image.BOX)


def main():
    save = sys.argv[2] if len(sys.argv) > 2 and sys.argv[1] == "--png" else None
    base, night, rim = finish(cockpit()), finish(cockpit(True)), finish(wheel())
    # Under the hood: Mikey Nichols's picture of his own engine bay, cut to the screen above the floor.
    bay = picture("art/engine-bay.png", (0, 230, 1448, 1026), (320, 176))
    dark = np.asarray(rim, dtype=np.float32).copy()
    dark[..., :3] = dark[..., :3] * np.array([0.26, 0.28, 0.36]) + np.array([1, 2, 5])
    rim_night = Image.fromarray(np.clip(dark, 0, 255).astype(np.uint8), "RGBA")
    if save:
        for name, im in (("cockpit", base), ("cockpit-night", night), ("wheel", rim), ("bay", bay)):
            im.save(f"{save}/{name}.png")
    ib, inn, iw = to_indices(base), to_indices(night, dither=9), to_indices(rim, dither=10)
    out = [
        "/* The game's painted pictures, as palette indices. Made by tools/paint.py; do not edit by hand.",
        "   The pictures are run-length: width, height, then (count, colour) pairs. 0xFFFF is clear. */",
        f"#define ART_TACH_X {TACH[0]}\n#define ART_TACH_Y {TACH[1]}\n#define ART_DIAL_R {TACH[2]}",
        f"#define ART_SPEEDO_X {SPEEDO[0]}\n#define ART_SPEEDO_Y {SPEEDO[1]}",
        f"#define ART_MINI_X {MINIS[0][0]}\n#define ART_MINI_Y {MINIS[0][1]}\n#define ART_MINI_STEP {MINIS[1][1] - MINIS[0][1]}\n#define ART_MINI_R 6",
        f"#define ART_BOOST_X {BOOST[0]}\n#define ART_BOOST_Y {BOOST[1]}\n#define ART_BOOST_R {BOOST[2]}",
        f"#define ART_WHEEL_X {WHEEL[0]}\n#define ART_WHEEL_Y {WHEEL[1]}\n#define ART_WHEEL_SIZE {iw.shape[0]}",
        f"#define ART_GATE_X {GATE[0]}\n#define ART_GATE_Y {GATE[1]}",
        c_array("ART_BAY", rle(to_indices(bay, dither=5))),
        c_array("ART_COCKPIT", rle(ib)),
        c_array("ART_COCKPIT_NIGHT", rle(inn)),
        c_array("ART_WHEEL", [int(v) for v in iw.reshape(-1)]),
        c_array("ART_WHEEL_NIGHT", [int(v) for v in to_indices(rim_night, dither=6).reshape(-1)]),
    ]
    open("games/shootout/art.h", "w").write("\n".join(out) + "\n")
    print("games/shootout/art.h", len(rle(ib)), len(rle(inn)), iw.shape)


main()
