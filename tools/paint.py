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


# ---- The garage, and the car in it, side on ------------------------------------------------------

def stops(t, pairs):
    """A value for each t in 0..1, read off a list of (t, value) stops."""
    xs, ys = zip(*pairs)
    return np.interp(t, xs, ys)


def garage():
    """The room: plank walls, a window with the day outside, a bench and a board of tools, a calendar,
    the big door, and a concrete floor with a lamp's light on it. The floor line is at y = 120."""
    img = canvas(W, 200)
    # Walls: planks, each a little different, darker up in the rafters.
    for i in range(20):
        tone = 0.86 + 0.14 * math.sin(i * 12.9898) ** 2
        fill(img, poly([(i * 16, 0), (i * 16 + 16, 0), (i * 16 + 16, 120), (i * 16, 120)]), tuple(int(c * tone) for c in (96, 62, 34)), tuple(int(c * tone) for c in (156, 104, 58)), grain=5)
        line(img, [(i * 16, 8), (i * 16, 120)], (48, 30, 16), 0.9)
        line(img, [(i * 16 + 1, 8), (i * 16 + 1, 120)], (176, 122, 70), 0.4)
    for kx, ky in ((44, 96), (148, 78), (212, 100), (296, 60), (116, 110)):
        fill(img, disc(kx, ky, 1.6), (58, 36, 18))
    fill(img, poly([(0, 0), (320, 0), (320, 10), (0, 10)]), (40, 26, 14), (70, 46, 24), grain=3)
    shade(img, poly([(0, 10), (320, 10), (320, 30), (0, 30)]), (0, 0, 0), 0.45, 8)
    # The floor, with the door's daylight across it and stains where cars have stood.
    fill(img, poly([(0, 120), (320, 120), (320, 200), (0, 200)]), (120, 118, 112), (176, 172, 164), grain=5)
    shade(img, poly([(0, 120), (320, 120), (320, 126), (0, 126)]), (0, 0, 0), 0.5, 3)
    shade(img, poly([(226, 122), (312, 122), (320, 200), (190, 200)]), (255, 250, 230), 0.18, 10)
    for sx, sy, sr in ((120, 176, 16), (196, 182, 10), (70, 168, 7)):
        shade(img, lambda d, sx=sx, sy=sy, sr=sr: d.ellipse([(sx - sr) * S, (sy - sr * 0.3) * S, (sx + sr) * S, (sy + sr * 0.3) * S], fill=255), (40, 36, 32), 0.35, 2)
    for i in range(1, 5):
        line(img, [(i * 64, 122), (i * 80 - 40, 200)], (104, 102, 96), 0.5)

    # The window, and the light it throws.
    fill(img, rrect(16, 24, 68, 56, 1), (232, 230, 222), (190, 188, 180))
    fill(img, rrect(20, 28, 60, 48, 0.5), (120, 176, 236), (196, 222, 244))
    fill(img, poly([(20, 60), (80, 56), (80, 76), (20, 76)]), (88, 142, 70), (58, 104, 48), grain=6)
    fill(img, disc(62, 54, 9), (70, 118, 56), (48, 90, 40), grain=6)
    fill(img, poly([(48.5, 28), (51.5, 28), (51.5, 76), (48.5, 76)]), (224, 222, 214))
    fill(img, poly([(20, 50.5), (80, 50.5), (80, 53.5), (20, 53.5)]), (224, 222, 214))
    shade(img, poly([(16, 80), (84, 80), (84, 84), (16, 84)]), (0, 0, 0), 0.5, 2)
    shade(img, poly([(24, 122), (96, 122), (150, 200), (40, 200)]), (255, 252, 236), 0.16, 8)

    # A board of tools over the bench, the bench, a vice, and the paper folded on it.
    fill(img, rrect(8, 86, 84, 20, 1), (196, 168, 122), (160, 132, 92), grain=4)
    for bx in range(12, 90, 5):
        for by in (90, 95, 100):
            fill(img, disc(bx, by, 0.5), (98, 78, 50))
    for tx in (18, 28, 38):
        line(img, [(tx, 89), (tx, 102)], (188, 194, 204), 1.6)
        fill(img, disc(tx, 89.5, 2.2), (200, 206, 216), (120, 124, 134))
    line(img, [(52, 90), (52, 103)], (150, 100, 56), 2)
    fill(img, rrect(47, 88, 10, 4.5, 0.6), (90, 92, 100), (50, 52, 58))
    line(img, [(66, 90), (78, 102)], (200, 60, 50), 1.6)
    shade(img, poly([(4, 108), (98, 108), (98, 124), (4, 124)]), (0, 0, 0), 0.35, 4, 0, 3)
    fill(img, poly([(4, 106), (98, 106), (96, 113), (6, 113)]), (190, 140, 84), (140, 98, 56), grain=4)
    line(img, [(4, 106), (98, 106)], (226, 180, 120), 0.7)
    for lx in (10, 88):
        fill(img, poly([(lx, 113), (lx + 5, 113), (lx + 5, 146), (lx, 146)]), (120, 82, 46), (70, 46, 24), grain=3)
    fill(img, rrect(72, 96, 15, 10, 1), (150, 156, 168), (70, 74, 84))
    fill(img, rrect(76, 91, 6, 6, 1), (176, 182, 194), (96, 100, 110))
    line(img, [(70, 101), (64, 101)], (190, 196, 206), 1)
    fill(img, poly([(20, 106), (27, 98), (58, 98), (55, 106)]), (238, 232, 212), (214, 206, 184))
    line(img, [(30, 100), (54, 100)], (40, 40, 44), 1)
    for ly in (102, 103.6):
        line(img, [(28, ly), (54, ly)], (130, 126, 116), 0.4)

    # The calendar, and a nail above the place where the banner hangs.
    shade(img, rrect(102, 24, 30, 38, 0.5), (0, 0, 0), 0.4, 2, 1, 2)
    fill(img, rrect(100, 22, 30, 38, 0.5), (244, 238, 220), (222, 214, 194))
    fill(img, poly([(100, 22), (130, 22), (130, 31), (100, 31)]), (196, 44, 40), (160, 30, 30))
    text(img, 115, 26.6, "DAY", 5, (250, 244, 232))
    text(img, 115, 46, "1", 20, (36, 34, 34))

    # The big door: four panels with a row of glass, in a dark frame.
    fill(img, poly([(232, 14), (316, 14), (316, 122), (232, 122)]), (30, 28, 30))
    for i in range(4):
        y = 18 + i * 26
        fill(img, poly([(236, y), (312, y), (312, y + 24), (236, y + 24)]), (206, 210, 218), (140, 144, 154), grain=3)
        line(img, [(236, y + 0.5), (312, y + 0.5)], (244, 246, 250), 0.6)
        line(img, [(236, y + 23.5), (312, y + 23.5)], (90, 94, 104), 0.6)
    for i in range(4):
        fill(img, rrect(240 + i * 18, 48, 15, 14, 1), (44, 46, 52))
        fill(img, rrect(241.5 + i * 18, 49.5, 12, 11, 0.5), (130, 184, 238), (206, 228, 246))
    fill(img, rrect(266, 108, 16, 4, 1), (220, 224, 232), (110, 114, 124))
    # A lamp under the rafters and the pool of light under it.
    fill(img, poly([(150, 10), (170, 10), (176, 17), (144, 17)]), (60, 62, 68), (30, 30, 34))
    fill(img, poly([(146, 17), (174, 17), (174, 19), (146, 19)]), (255, 250, 220))
    shade(img, poly([(146, 19), (174, 19), (230, 120), (90, 120)]), (255, 244, 200), 0.10, 10)
    return img


# The wedge coupe, side on, in metres from the back bumper and up from the road: the outline of the
# body, and of the glass in it.
BODY = [(0.00, 0.34), (0.02, 0.78), (0.06, 0.93), (0.02, 0.99), (0.32, 0.985), (0.36, 0.95), (1.20, 1.14), (1.88, 1.25), (2.34, 1.21),
        (2.96, 0.88), (3.50, 0.80), (4.04, 0.68), (4.30, 0.60), (4.36, 0.52), (4.36, 0.36), (4.28, 0.25), (0.14, 0.25)]
GLASS = [(0.52, 0.965), (1.22, 1.115), (1.86, 1.215), (2.32, 1.18), (2.86, 0.895), (1.6, 0.93)]
PPM, CAR_W, CAR_H, CAR_GROUND = 54, 244, 78, 74
AXLES = (0.80, 3.54)


def car_side(lamps):
    img = canvas(CAR_W, CAR_H)
    ox = 4

    def at(x, y):
        return ox + x * PPM, CAR_GROUND - y * PPM

    def shape(pts, r=2.5):
        return mask_shape_round([at(x, y) for x, y in pts], r)

    # The shadow it stands in.
    shade(img, lambda d: d.ellipse([at(-0.1, 0)[0] * S, (CAR_GROUND - 3) * S, at(4.46, 0)[0] * S, (CAR_GROUND + 3) * S], fill=255), (0, 0, 0), 0.75, 1.5)
    # The body: paint that is light on top, bright along the shoulder, dark down the door, and picks
    # the floor back up along the sill.
    m = mask_of(img.size, shape(BODY))
    h, w = img.size[1], img.size[0]
    top, bottom = at(0, 1.25)[1] * S, at(0, 0.25)[1] * S
    t = np.clip((np.arange(h) - top) / (bottom - top), 0, 1)
    lum = stops(t, [(0, 1.05), (0.22, 0.9), (0.36, 1.0), (0.43, 1.75), (0.47, 1.2), (0.56, 0.78), (0.78, 0.5), (0.9, 0.62), (1, 0.4)])
    across = 0.94 + 0.12 * np.sin(np.arange(w) / w * math.pi)
    base = np.array([38, 82, 196], dtype=np.float32)
    col = lum[:, None, None] * across[None, :, None] * base[None, None, :]
    col = col + (np.clip(lum - 1.2, 0, 1) * 150)[:, None, None]
    col = col + rng.normal(0, 3, (h // S + 1, w // S + 1)).repeat(S, 0).repeat(S, 1)[:h, :w, None]
    img.paste(Image.fromarray(np.clip(col, 0, 255).astype(np.uint8), "RGB").convert("RGBA"), (0, 0), m)
    # The glass, with the sky in it, and the pillar between the door and the hatch.
    fill(img, shape(GLASS, 1.5), (30, 40, 58), (10, 14, 22))
    gx0, gy0 = at(1.0, 1.1)
    shade(img, poly([(gx0, gy0 - 3), (gx0 + 20, gy0 - 9), (gx0 + 44, gy0 + 10), (gx0 + 16, gy0 + 10)]), (170, 200, 236), 0.28, 1.5)
    px0 = at(1.52, 0)[0]
    fill(img, poly([(px0, at(0, 1.2)[1]), (px0 + 6, at(0, 1.21)[1]), (px0 + 8, at(0, 0.93)[1]), (px0 + 1, at(0, 0.93)[1])]), (14, 16, 22))
    # The door, its handle, a mirror, and the seams of the hood and the bumpers.
    seam = (12, 24, 70)
    dx0, dx1 = at(1.62, 0)[0], at(2.92, 0)[0]
    line(img, [(dx0, at(0, 0.93)[1]), (dx0 - 1, at(0, 0.33)[1])], seam, 0.6)
    line(img, [(dx1, at(0, 0.88)[1]), (dx1 + 3, at(0, 0.62)[1]), (dx1 + 1, at(0, 0.33)[1])], seam, 0.6)
    line(img, [(dx0, at(0, 0.33)[1]), (dx1 + 1, at(0, 0.33)[1])], seam, 0.5)
    fill(img, rrect(dx0 + 5, at(0, 0.80)[1], 9, 2.4, 1), (16, 30, 84), (60, 100, 200))
    mx, my = at(2.80, 0.95)
    fill(img, mask_shape_round([(mx, my - 1), (mx + 9, my - 3), (mx + 10, my + 4), (mx + 1, my + 5)], 1.2), (64, 110, 220), (20, 46, 130))
    line(img, [at(0.0, 0.50), at(0.62, 0.50)], seam, 0.5)
    line(img, [at(3.86, 0.50), at(4.36, 0.50)], seam, 0.5)
    line(img, [at(0.06, 0.40), at(4.30, 0.40)], (20, 40, 110), 0.5)
    # Lamps: a bar of red round the tail, an amber marker at each end, and the headlamp folded flat.
    fill(img, mask_shape_round([at(0.02, 0.80), at(0.30, 0.82), at(0.30, 0.68), (at(0.02, 0.66))], 1), (236, 60, 50), (150, 24, 28))
    fill(img, rrect(at(0.34, 0.62)[0], at(0.34, 0.62)[1], 6, 2.2, 0.6), (240, 120, 40), (190, 80, 20))
    fill(img, rrect(at(4.12, 0.56)[0], at(4.12, 0.56)[1], 8, 2.2, 0.6), (250, 170, 60), (210, 110, 30))
    if lamps:
        lx, ly = at(3.88, 0.83)
        fill(img, poly([(lx, ly), (lx + 13, ly + 1), (lx + 14, ly + 8), (lx - 1, ly + 7)]), (56, 104, 220), (26, 60, 150))
        fill(img, poly([(lx + 12, ly + 1.5), (lx + 15, ly + 2), (lx + 15.5, ly + 7.5), (lx + 12.5, ly + 7)]), (255, 252, 226))
        shade(img, disc(lx + 17, ly + 4.5, 3), (255, 250, 200), 0.6, 1.5)
    else:
        line(img, [at(3.86, 0.70), at(4.14, 0.645)], seam, 0.5)
    # The wheels in their arches: a tyre, a rim with five spokes, and a bright lip.
    for ax in AXLES:
        cx, cy = at(ax, 0.31)
        fill(img, disc(cx, cy, 0.385 * PPM), (6, 8, 14))
        fill(img, disc(cx, cy, 0.31 * PPM), (38, 38, 42), (14, 14, 16))
        fill(img, disc(cx, cy, 0.215 * PPM), (214, 218, 226), (120, 124, 134))
        fill(img, disc(cx, cy, 0.19 * PPM), (40, 42, 48), (18, 18, 22))
        for k in range(5):
            a = k / 5 + 0.03
            p1, p2, p3 = polar(cx, cy, 0.19 * PPM, a - 0.035), polar(cx, cy, 0.19 * PPM, a + 0.035), polar(cx, cy, 0.03 * PPM, a)
            fill(img, poly([p1, p2, p3]), (226, 230, 238), (150, 154, 164))
        fill(img, disc(cx, cy, 0.05 * PPM), (190, 194, 204), (90, 94, 104))
        shade(img, lambda d, cx=cx, cy=cy: d.pieslice([(cx - 17) * S, (cy - 17) * S, (cx + 17) * S, (cy + 17) * S], 200, 300, fill=255), (255, 255, 255), 0.12, 1)
    return img


def main():
    save = sys.argv[2] if len(sys.argv) > 2 and sys.argv[1] == "--png" else None
    base, night, rim = finish(cockpit()), finish(cockpit(True)), finish(wheel())
    # Under the hood: Mikey Nichols's picture of his own engine bay, cut to the screen above the floor.
    bay = picture("art/engine-bay.png", (0, 230, 1448, 1026), (320, 176))
    room, car, car_lit = finish(garage()), finish(car_side(False)), finish(car_side(True))
    dark = np.asarray(rim, dtype=np.float32).copy()
    dark[..., :3] = dark[..., :3] * np.array([0.26, 0.28, 0.36]) + np.array([1, 2, 5])
    rim_night = Image.fromarray(np.clip(dark, 0, 255).astype(np.uint8), "RGBA")
    if save:
        for name, im in (("cockpit", base), ("cockpit-night", night), ("wheel", rim), ("bay", bay), ("garage", room), ("car", car), ("car-lamps", car_lit)):
            im.save(f"{save}/{name}.png")
    ib, inn, iw = to_indices(base), to_indices(night, dither=9), to_indices(rim, dither=10)
    out = [
        "/* The game's painted pictures, as palette indices. Made by tools/paint.py; do not edit by hand.",
        "   The pictures are run-length: width, height, then (count, colour) pairs. 0xFFFF is clear. */",
        "#ifndef SHOOTOUT_ART_H\n#define SHOOTOUT_ART_H",
        f"#define ART_TACH_X {TACH[0]}\n#define ART_TACH_Y {TACH[1]}\n#define ART_DIAL_R {TACH[2]}",
        f"#define ART_SPEEDO_X {SPEEDO[0]}\n#define ART_SPEEDO_Y {SPEEDO[1]}",
        f"#define ART_MINI_X {MINIS[0][0]}\n#define ART_MINI_Y {MINIS[0][1]}\n#define ART_MINI_STEP {MINIS[1][1] - MINIS[0][1]}\n#define ART_MINI_R 6",
        f"#define ART_BOOST_X {BOOST[0]}\n#define ART_BOOST_Y {BOOST[1]}\n#define ART_BOOST_R {BOOST[2]}",
        f"#define ART_WHEEL_X {WHEEL[0]}\n#define ART_WHEEL_Y {WHEEL[1]}\n#define ART_WHEEL_SIZE {iw.shape[0]}",
        f"#define ART_GATE_X {GATE[0]}\n#define ART_GATE_Y {GATE[1]}",
        c_array("ART_BAY", rle(to_indices(bay, dither=5))),
        c_array("ART_GARAGE", rle(to_indices(room, dither=12))),
        c_array("ART_CAR", rle(to_indices(car, dither=10))),
        c_array("ART_CAR_LAMPS", rle(to_indices(car_lit, dither=10))),
        f"#define ART_CAR_W {CAR_W}\n#define ART_CAR_GROUND {CAR_GROUND}",
        c_array("ART_COCKPIT", rle(ib)),
        c_array("ART_COCKPIT_NIGHT", rle(inn)),
        c_array("ART_WHEEL", [int(v) for v in iw.reshape(-1)]),
        c_array("ART_WHEEL_NIGHT", [int(v) for v in to_indices(rim_night, dither=6).reshape(-1)]),
    ]
    open("games/shootout/art.h", "w").write("\n".join(out) + "\n#endif\n")
    print("games/shootout/art.h", len(rle(ib)), len(rle(inn)), iw.shape)


main()
