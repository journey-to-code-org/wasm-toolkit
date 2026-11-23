#!/usr/bin/env python3
"""Draws the default pack's stand-in pictures of the places on a car that parts are changed in,
and says where each part is on them.

The trunk, underneath on a lift, a front wheel and the inside, each 640 by 352 (a picture painted
fine, for a place 320 by 176 on the screen), with every stock part drawn in where the slot's
shape says it is. They are made to be replaced: drop a photo on the place in the workshop and
move the shapes to where the parts are on it.

    tools/stand_ins.py content/default            pictures as <area>@2x.png
    tools/stand_ins.py content/default --slots    and print each area's slots as JSON

Nothing here is a real car's design: plain shapes, shaded.
"""
import json
import math
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

sys.path.insert(0, str(Path(__file__).parent))
from paint import PAL, to_indices  # noqa: E402

W, H = 320, 176          # places
K = 8                    # drawn this many times finer, then brought down to two dots a place
OUT = 2
DITHER = 0

PAINT = (18, 64, 196)    # the bay photo's blue
STEEL = (150, 154, 160)
ALLOY = (176, 180, 186)
BLACK = (22, 22, 24)
RUBBER = (30, 30, 32)


def ring(cx, cy, r, n=28, start=0.0):
    return [[round(cx + r * math.cos(start + 2 * math.pi * k / n)), round(cy + r * math.sin(start + 2 * math.pi * k / n))] for k in range(n)]


def annulus(cx, cy, r_out, r_in, n=28):
    """A ring as one outline: round the outside, in along a cut, round the inside backwards."""
    outer = ring(cx, cy, r_out, n)
    inner = ring(cx, cy, r_in, n)
    return outer + [outer[0], inner[0]] + inner[::-1] + [inner[0]]


def joined(*shapes):
    """Two or more shapes as one outline, each reached from the first along a line out and back."""
    out = list(shapes[0])
    for s in shapes[1:]:
        anchor = out[0]
        out += [anchor] + list(s) + [s[0], anchor]
    return out


def box(x, y, w, h):
    return [[x, y], [x + w, y], [x + w, y + h], [x, y + h]]


class Pic:
    def __init__(self, base):
        self.im = Image.new("RGB", (W * K, H * K), base)

    def _mask(self, pts, blur=0):
        m = Image.new("L", self.im.size, 0)
        ImageDraw.Draw(m).polygon([(x * K, y * K) for x, y in pts], fill=255)
        return m.filter(ImageFilter.GaussianBlur(blur * K)) if blur else m

    def grad(self, pts, top, bottom, horizontal=False):
        """A shape filled from one color to another, down it or across it."""
        xs, ys = [p[0] for p in pts], [p[1] for p in pts]
        x0, x1, y0, y1 = min(xs) * K, max(xs) * K, min(ys) * K, max(ys) * K
        n = max(1, int((x1 - x0) if horizontal else (y1 - y0)))
        t = np.linspace(0, 1, n)[:, None]
        rows = (np.array(top)[None, :] * (1 - t) + np.array(bottom)[None, :] * t).astype(np.uint8)
        band = Image.fromarray(np.repeat(rows[None, :, :], max(1, int(y1 - y0)), 0) if horizontal else np.repeat(rows[:, None, :], max(1, int(x1 - x0)), 1))
        layer = Image.new("RGB", self.im.size)
        layer.paste(band, (int(x0), int(y0)))
        self.im.paste(layer, (0, 0), self._mask(pts))

    def flat(self, pts, color):
        self.im.paste(Image.new("RGB", self.im.size, color), (0, 0), self._mask(pts))

    def shadow(self, pts, blur=3, dx=2, dy=3, alpha=150):
        m = self._mask([[x + dx, y + dy] for x, y in pts], blur).point(lambda v: v * alpha // 255)
        self.im.paste(Image.new("RGB", self.im.size, (0, 0, 0)), (0, 0), m)

    def glow(self, pts, color, blur=2, alpha=90):
        m = self._mask(pts, blur).point(lambda v: v * alpha // 255)
        self.im.paste(Image.new("RGB", self.im.size, color), (0, 0), m)

    def line(self, pts, color, width=1.0):
        ImageDraw.Draw(self.im).line([(x * K, y * K) for x, y in pts], fill=color, width=max(1, int(width * K)), joint="curve")

    def disc(self, cx, cy, r, inner, outer):
        """A round thing lit from the top left: lighter in, darker out."""
        for k in range(12, 0, -1):
            f = k / 12
            c = tuple(int(inner[i] * (1 - f) + outer[i] * f) for i in range(3))
            ox, oy = (1 - f) * -r * 0.25, (1 - f) * -r * 0.25
            self.flat(ring(cx + ox, cy + oy, r * f, 40), c)

    def bolt(self, x, y, r=2.2):
        self.shadow(ring(x, y, r, 12), blur=0.6, dx=0.6, dy=0.8, alpha=180)
        self.disc(x, y, r, (225, 228, 232), (90, 92, 96))
        hexa = ring(x, y, r * 0.55, 6, math.pi / 6)
        self.flat(hexa, (120, 122, 126))

    def cylinder(self, x0, y0, x1, y1, color, across=True):
        """A pipe or a shaft: lit along its length."""
        lit = tuple(min(255, int(c * 1.45)) for c in color)
        dim = tuple(int(c * 0.45) for c in color)
        if across:
            ym = (y0 + y1) / 2
            self.grad(box(x0, y0, x1 - x0, (ym - y0)), dim, lit)
            self.grad(box(x0, ym, x1 - x0, (y1 - ym)), lit, dim)
        else:
            xm = (x0 + x1) / 2
            self.grad(box(x0, y0, xm - x0, y1 - y0), dim, lit, horizontal=True)
            self.grad(box(xm, y0, x1 - xm, y1 - y0), lit, dim, horizontal=True)

    def carpet(self, pts, color, seed=1):
        """Carpet: flat, with a coarse weave so it still keeps to runs when stored."""
        self.flat(pts, color)
        rng = np.random.default_rng(seed)
        small = rng.integers(-1, 2, (H // 2 + 1, W // 2 + 1)).astype(np.float32) * 5
        weave = np.kron(small, np.ones((2 * K, 2 * K)))[: self.im.size[1], : self.im.size[0], None]
        m = np.asarray(self._mask(pts))[:, :, None] > 0
        arr = np.asarray(self.im).astype(np.float32)
        self.im = Image.fromarray(np.where(m, np.clip(arr + weave, 0, 255), arr).astype(np.uint8))

    def finish(self, path, seed=7):
        small = self.im.resize((W * OUT, H * OUT), Image.LANCZOS)
        arr = np.asarray(small).astype(np.float32)
        # A darker edge, like a photo.
        yy, xx = np.mgrid[0:arr.shape[0], 0:arr.shape[1]]
        d = np.hypot((xx - arr.shape[1] / 2) / arr.shape[1], (yy - arr.shape[0] / 2) / arr.shape[0])
        arr *= (1 - 0.55 * np.clip(d - 0.25, 0, 1)).round(1)[:, :, None]
        # In the engine's own colors already, stippled only a little: it is stored run by run, and
        # a stipple everywhere would make it large.
        ix = to_indices(Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8)).convert("RGBA"), dither=DITHER)
        Image.fromarray(np.clip(PAL[ix], 0, 255).round().astype(np.uint8)).save(path)


def shade(c, f):
    return tuple(max(0, min(255, int(v * f))) for v in c)


# ---- The trunk: looking down into it, the lid up out of sight. ----

def trunk():
    p = Pic((40, 40, 44))
    slots = {}
    # The opening, in the car's paint: quarter panels, the lip of the bumper, the lid's underside.
    p.grad(box(0, 0, W, 18), (70, 72, 78), (30, 30, 34))
    p.grad([[0, 0], [26, 14], [30, 176], [0, 176]], shade(PAINT, 1.2), shade(PAINT, 0.55), horizontal=True)
    p.grad([[320, 0], [294, 14], [290, 176], [320, 176]], shade(PAINT, 1.2), shade(PAINT, 0.55), horizontal=True)
    for x0, sign in ((4, 1), (316, -1)):
        lamp = [[x0, 130], [x0 + sign * 20, 134], [x0 + sign * 22, 166], [x0, 168]]
        p.grad(lamp, (230, 40, 40), (110, 10, 14))
        p.line([[x0 + sign * 3, 140], [x0 + sign * 18, 142]], (255, 150, 150), 0.6)
    p.grad(box(0, 156, W, 20), shade(PAINT, 1.1), shade(PAINT, 0.5))
    p.grad(box(130, 158, 60, 8), (60, 60, 64), (20, 20, 22))
    p.bolt(160, 162, 2.6)
    # The floor and the back of the rear seat.
    floor = [[28, 26], [292, 26], [290, 154], [30, 154]]
    p.carpet(floor, (52, 52, 56), seed=3)
    p.carpet([[28, 18], [292, 18], [292, 40], [28, 40]], (36, 36, 40), seed=4)
    p.line([[28, 40], [292, 40]], (20, 20, 22), 1.2)
    # The carpet folded back from the hatch over the fuel pump.
    flap = [[56, 54], [154, 50], [158, 60], [60, 64]]
    p.shadow(flap, blur=2, dy=2)
    p.grad(flap, (80, 80, 86), (46, 46, 50))
    well = [[60, 64], [154, 60], [152, 120], [62, 124]]
    p.flat(well, (34, 34, 36))
    cover = [[70, 70], [144, 66], [142, 114], [72, 118]]
    p.shadow(cover, blur=1.5)
    p.grad(cover, (110, 112, 118), (58, 60, 64))
    cover_bolts = [[76, 74], [138, 71], [137, 109], [77, 112]]
    for b in cover_bolts:
        p.bolt(*b)
    p.line([[86, 92], [128, 90]], (40, 40, 44), 1.4)
    slots["Access cover"] = {"poly": cover, "bolts": cover_bolts, "stock": "Pump access cover"}
    pump = ring(107, 92, 18, 20)
    slots["Fuel pump"] = {"poly": pump, "bolts": [[95, 84], [119, 100]], "behind": "Access cover", "covered": True}
    # A jack and its bag on the right, as trunks have.
    bag = [[196, 70], [266, 64], [270, 104], [200, 112]]
    p.shadow(bag, blur=2)
    p.grad(bag, (70, 66, 58), (38, 36, 30))
    p.line([[204, 84], [262, 79]], (24, 22, 20), 1.2)
    p.cylinder(210, 116, 262, 124, (110, 20, 20))
    return p, slots


# ---- Underneath, on the lift: the front to the left, the rear to the right. ----

def under():
    p = Pic((26, 26, 30))
    slots = {}
    # The floor pan, ribbed, and the four tires at the edges.
    p.grad(box(20, 22, 280, 132), (66, 68, 72), (44, 46, 50))
    for y in (40, 136):
        p.line([[110, y], [230, y]], (36, 38, 42), 1.5)
    for x in (24, 250):
        for y in (0, 154):
            t = box(x, y, 50, 22)
            p.grad(t, (36, 36, 38), (14, 14, 16), horizontal=False)
            for k in range(6):
                p.line([[x + 4 + k * 8, y], [x + 4 + k * 8, y + 22]], (24, 24, 26), 0.8)
    # The lift's arms, yellow, to their pads under the sills.
    for x, y0, y1 in ((112, 0, 26), (204, 0, 26), (112, 150, 176), (204, 150, 176)):
        p.cylinder(x - 4, y0, x + 4, y1, (210, 170, 20), across=False)
        p.flat(box(x - 7, (24 if y0 == 0 else 146), 14, 6), (40, 40, 40))
    # Front axles: from the transmission out to each front wheel, with their boots.
    up, down = box(58, 20, 8, 46), box(58, 110, 8, 46)
    for s in (up, down):
        p.shadow(s, blur=1)
        p.cylinder(58, s[0][1], 66, s[2][1], STEEL, across=False)
    for y in (24, 58, 114, 148):
        p.disc(62, y, 6, (70, 70, 72), (18, 18, 20))
    slots["Front axles"] = {"poly": joined(box(52, 18, 20, 50), box(52, 108, 20, 50)), "bolts": [[62, 24], [62, 148]]}
    # The transmission and its bellhousing; the clutch is inside.
    trans = [[36, 66], [86, 62], [94, 76], [94, 102], [86, 114], [36, 112], [30, 90]]
    p.shadow(trans, blur=2.5)
    p.grad(trans, (196, 198, 202), (98, 100, 106))
    for x in range(42, 88, 8):
        p.line([[x, 68], [x, 108]], (140, 142, 148), 0.5)
    trans_bolts = [[40, 70], [84, 67], [86, 108], [40, 108]]
    for b in trans_bolts:
        p.bolt(*b)
    slots["Transmission"] = {"poly": trans, "bolts": trans_bolts, "stock": "Stock transmission"}
    slots["Clutch"] = {"poly": ring(60, 89, 19, 20), "bolts": [[60, 74], [75, 89], [60, 104], [45, 89]], "behind": "Transmission", "covered": True}
    # The power transfer unit beside it, and the transfer case behind that.
    ptu = [[94, 78], [112, 78], [114, 100], [94, 102]]
    p.shadow(ptu, blur=1.5)
    p.grad(ptu, (150, 152, 158), (80, 82, 88))
    p.bolt(98, 82), p.bolt(110, 97)
    slots["PTU"] = {"poly": ptu, "bolts": [[98, 82], [110, 97]]}
    tcase = [[114, 80], [132, 82], [132, 98], [114, 100]]
    p.grad(tcase, (130, 132, 138), (70, 72, 78))
    p.bolt(118, 84), p.bolt(128, 95)
    slots["Transfer case"] = {"poly": tcase, "bolts": [[118, 84], [128, 95]]}
    # The driveshaft down the middle, with its bearing.
    p.shadow(box(132, 85, 104, 8), blur=1.5)
    p.cylinder(132, 85, 236, 93, (110, 112, 118))
    p.flat(box(180, 82, 8, 14), (40, 40, 42))
    p.bolt(140, 89, 1.8), p.bolt(228, 89, 1.8)
    slots["Driveshaft"] = {"poly": box(132, 83, 104, 12), "bolts": [[140, 89], [228, 89]]}
    # The rear differential, and the rear axles out of it.
    for s in (box(247, 20, 7, 56), box(247, 104, 7, 52)):
        p.shadow(s, blur=1)
        p.cylinder(247, s[0][1], 254, s[2][1], STEEL, across=False)
    for y in (24, 70, 110, 150):
        p.disc(250, y, 5, (70, 70, 72), (18, 18, 20))
    slots["Rear axles"] = {"poly": joined(box(242, 18, 17, 58), box(242, 104, 17, 54)), "bolts": [[250, 24], [250, 150]]}
    diff = ring(250, 90, 16, 24)
    p.shadow(diff, blur=2)
    p.disc(250, 90, 16, (190, 192, 198), (70, 72, 78))
    diff_bolts = [[250, 78], [262, 90], [250, 102], [238, 90]]
    for b in diff_bolts:
        p.bolt(*b, 1.8)
    slots["Rear diff"] = {"poly": diff, "bolts": diff_bolts}
    # The fuel tank under the back seat.
    tank = [[172, 28], [228, 26], [232, 70], [176, 74]]
    p.shadow(tank, blur=2)
    p.grad(tank, (88, 90, 96), (42, 44, 48))
    for x in range(180, 228, 12):
        p.line([[x, 30], [x + 2, 70]], (70, 72, 78), 0.8)
    tank_bolts = [[178, 32], [226, 30], [228, 66], [180, 70]]
    for b in tank_bolts:
        p.bolt(*b)
    slots["Fuel tank"] = {"poly": tank, "bolts": tank_bolts}
    # The downpipe and its oxygen sensor housing, then the exhaust all the way back to the muffler.
    o2 = [[96, 106], [122, 108], [122, 122], [96, 124]]
    p.shadow(o2, blur=1.5)
    p.cylinder(96, 108, 122, 122, (130, 110, 90))
    p.disc(109, 115, 3, (200, 200, 200), (60, 60, 60))
    slots["O2 housing"] = {"poly": o2, "bolts": [[100, 111], [118, 119]]}
    pipe = box(122, 110, 140, 10)
    muffler = [[262, 104], [306, 102], [310, 130], [264, 134]]
    p.shadow(pipe, blur=1.5)
    p.cylinder(122, 110, 262, 120, (120, 110, 100))
    p.shadow(muffler, blur=2)
    p.grad(muffler, (176, 170, 162), (84, 80, 74))
    p.cylinder(306, 112, 318, 120, (150, 150, 150))
    p.flat(box(170, 108, 4, 14), (34, 34, 36)), p.flat(box(226, 108, 4, 14), (34, 34, 36))
    slots["Exhaust system"] = {"poly": [[122, 108], [262, 108], [262, 102], [310, 100], [312, 132], [264, 136], [262, 122], [122, 122]], "bolts": [[126, 115], [258, 115], [302, 106]], "stock": "Stock exhaust"}
    return p, slots


# ---- A front wheel, from the side, in its arch. ----

def wheel():
    p = Pic((30, 30, 32))
    slots = {}
    cx, cy = 160, 92
    # The garage floor, the body round the arch.
    p.grad(box(0, 150, W, 26), (96, 98, 102), (60, 62, 66))
    body = [[0, 0], [320, 0], [320, 128], [250, 128]] + [[round(cx + 84 * math.cos(a)), round(cy + 84 * math.sin(a))] for a in np.linspace(-0.25, -math.pi + 0.25, 30)] + [[70, 128], [0, 128]]
    p.grad(body, shade(PAINT, 1.35), shade(PAINT, 0.6))
    p.line([[0, 128], [72, 128]], shade(PAINT, 0.35), 1.2)
    p.line([[248, 128], [320, 128]], shade(PAINT, 0.35), 1.2)
    p.flat(ring(cx, cy, 80, 48), (12, 12, 14))
    # The strut and spring up in the arch.
    p.cylinder(152, 10, 168, 40, (60, 60, 64), across=False)
    for y in range(14, 36, 5):
        p.line([[146, y], [174, y + 3]], (200, 40, 40), 1.6)
    # The tire.
    p.shadow(ring(cx, cy, 72, 48), blur=3, dx=3, dy=4)
    p.disc(cx, cy, 72, (58, 58, 62), (18, 18, 20))
    for k in range(40):
        a = 2 * math.pi * k / 40
        p.line([[cx + 66 * math.cos(a), cy + 66 * math.sin(a)], [cx + 71 * math.cos(a), cy + 71 * math.sin(a)]], (14, 14, 16), 1.0)
    # The brake disc and caliper, through the spokes.
    p.disc(cx, cy, 50, (40, 40, 44), (16, 16, 18))
    p.disc(cx, cy, 40, (170, 168, 164), (90, 88, 86))
    p.flat(ring(cx, cy, 14, 24), (50, 50, 54))
    caliper = [[cx + 18, cy - 40], [cx + 34, cy - 30], [cx + 40, cy - 8], [cx + 30, cy - 6], [cx + 24, cy - 26], [cx + 14, cy - 34]]
    p.grad(caliper, (220, 60, 50), (120, 20, 18))
    # The rim: five spokes.
    rim_disc = ring(cx, cy, 50, 48)
    p.glow(rim_disc, (210, 214, 220), blur=0.5, alpha=40)
    p.line([[cx + 50 * math.cos(a), cy + 50 * math.sin(a)] for a in np.linspace(0, 2 * math.pi, 60)], (200, 204, 210), 2.4)
    p.line([[cx + 46 * math.cos(a), cy + 46 * math.sin(a)] for a in np.linspace(0, 2 * math.pi, 60)], (110, 112, 118), 0.8)
    for k in range(5):
        a = -math.pi / 2 + 2 * math.pi * k / 5
        n = (-math.sin(a), math.cos(a))
        spoke = [[cx + 12 * math.cos(a) + 5 * n[0], cy + 12 * math.sin(a) + 5 * n[1]], [cx + 48 * math.cos(a) + 9 * n[0], cy + 48 * math.sin(a) + 9 * n[1]],
                 [cx + 48 * math.cos(a) - 9 * n[0], cy + 48 * math.sin(a) - 9 * n[1]], [cx + 12 * math.cos(a) - 5 * n[0], cy + 12 * math.sin(a) - 5 * n[1]]]
        p.shadow(spoke, blur=1, dx=1.5, dy=2, alpha=120)
        p.grad(spoke, (226, 228, 234), (128, 130, 138))
    p.disc(cx, cy, 16, (230, 232, 236), (120, 122, 128))
    lugs = [[round(cx + 11 * math.cos(-math.pi / 2 + 2 * math.pi * k / 5 + math.pi / 5)), round(cy + 11 * math.sin(-math.pi / 2 + 2 * math.pi * k / 5 + math.pi / 5))] for k in range(5)]
    for b in lugs:
        p.bolt(*b, 2.4)
    p.disc(cx, cy, 5, (90, 92, 98), (40, 40, 44))
    slots["Rims"] = {"poly": ring(cx, cy, 50, 32), "bolts": lugs, "stock": "Stock wheels"}
    slots["Tires"] = {"poly": annulus(cx, cy, 72, 51, 32), "bolts": 0, "behind": "Rims", "stock": "Stock tires"}
    slots["Brakes"] = {"poly": ring(cx, cy, 42, 28), "bolts": [[cx + 30, cy - 24], [cx + 34, cy - 12]], "behind": "Rims", "covered": True}
    slots["Suspension"] = {"poly": [[142, 8], [178, 8], [178, 62], [142, 62]], "bolts": [[150, 14], [170, 14], [160, 56]], "behind": "Brakes", "covered": True, "stock": "Stock struts"}
    return p, slots


# ---- Inside, through the driver's door. ----

def inside():
    p = Pic((24, 24, 26))
    slots = {}
    # The door frame, the far window, the headliner.
    p.grad(box(0, 0, W, 14), (80, 82, 86), (40, 40, 44))
    p.grad(box(150, 10, 170, 40), (150, 170, 196), (90, 110, 140))
    p.grad([[0, 0], [14, 0], [14, 176], [0, 176]], shade(PAINT, 1.2), shade(PAINT, 0.5), horizontal=True)
    # The dashboard across, the cluster hood, the vents, the steering column.
    dash = [[14, 22], [320, 34], [320, 66], [230, 70], [140, 72], [14, 64]]
    p.shadow(dash, blur=3, dy=4)
    p.grad(dash, (70, 70, 74), (30, 30, 32))
    for x in (176, 204):
        p.flat(box(x, 46, 20, 10), (16, 16, 18))
        for k in range(4):
            p.line([[x + 2, 48 + k * 2.4], [x + 18, 48 + k * 2.4]], (50, 50, 54), 0.5)
    p.grad([[40, 24], [116, 26], [112, 42], [44, 40]], (40, 40, 42), (14, 14, 16))
    p.disc(62, 44, 9, (200, 200, 190), (40, 40, 40)), p.disc(92, 44, 9, (200, 200, 190), (40, 40, 40))
    slots["Dashboard"] = {"poly": dash, "bolts": [[22, 30], [226, 64], [314, 40]], "stock": "Stock dash"}
    # The steering wheel, and the cruise control switch under its column.
    p.cylinder(74, 60, 92, 100, (40, 40, 42), across=False)
    for r, c in ((34, (34, 34, 36)), (28, (24, 24, 26))):
        p.flat(ring(76, 76, r, 40), c)
    p.flat(ring(76, 76, 24, 40), (24, 24, 26))
    p.flat(annulus(76, 76, 34, 27, 40), (44, 44, 46))
    p.flat(box(72, 60, 8, 32), (40, 40, 42))
    p.disc(76, 76, 9, (70, 70, 74), (30, 30, 32))
    cruise = [[98, 92], [122, 90], [124, 102], [100, 104]]
    p.grad(cruise, (86, 86, 90), (40, 40, 44))
    p.flat(box(104, 94, 4, 4), (40, 180, 70)), p.flat(box(112, 94, 4, 4), (200, 60, 50))
    slots["Cruise control"] = {"poly": cruise, "bolts": [[101, 95], [120, 100]]}
    # The kick panel in the far footwell, with the ECU behind it.
    p.grad([[236, 70], [320, 66], [320, 176], [240, 176]], (34, 34, 36), (18, 18, 20))
    kick = [[250, 74], [306, 72], [308, 104], [252, 108]]
    p.shadow(kick, blur=1.5)
    p.grad(kick, (64, 64, 68), (36, 36, 40))
    for b in [[255, 78], [302, 76], [302, 100], [256, 103]]:
        p.bolt(*b, 1.8)
    slots["Kick panel"] = {"poly": kick, "bolts": [[255, 78], [302, 76], [302, 100], [256, 103]], "stock": "Kick panel"}
    slots["ECU"] = {"poly": [[258, 80], [298, 78], [300, 98], [260, 100]], "bolts": [[263, 84], [294, 95]], "behind": "Kick panel", "covered": True}
    # The center console and the shifter.
    con = [[200, 72], [238, 70], [244, 176], [196, 176]]
    p.grad(con, (52, 52, 56), (22, 22, 24), horizontal=True)
    p.cylinder(216, 92, 222, 118, (30, 30, 32), across=False)
    p.disc(219, 90, 6, (90, 90, 96), (20, 20, 22))
    # The seat, nearest: its cushion and its back.
    seat = [[60, 132], [170, 126], [190, 118], [194, 40], [214, 36], [220, 62], [206, 150], [180, 168], [64, 170], [52, 152]]
    p.shadow(seat, blur=3, dx=3, dy=4)
    p.grad(seat, (78, 76, 82), (28, 26, 30))
    for x in range(80, 170, 22):
        p.line([[x, 138], [x - 2, 164]], (30, 28, 32), 1.0)
    p.line([[198, 50], [212, 48]], (36, 34, 38), 1.0)
    p.line([[196, 80], [214, 78]], (36, 34, 38), 1.0)
    seat_bolts = [[70, 166], [176, 162]]
    for b in seat_bolts:
        p.bolt(*b)
    slots["Seats"] = {"poly": seat, "bolts": seat_bolts, "stock": "Stock seats"}
    return p, slots


AREAS = {
    "trunk": ("In the trunk", trunk),
    "under": ("Under the car", under),
    "wheels": ("At the wheels", wheel),
    "inside": ("Inside", inside),
}


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "content/default")
    said = {}
    for area, (title, draw) in AREAS.items():
        pic, slots = draw()
        pic.finish(out / f"{area}@2x.png")
        said[area] = {"id": area, "title": title, "picture": f"{area}.pic", "slots": slots}
    if "--slots" in sys.argv:
        print(json.dumps(said, indent=2))


if __name__ == "__main__":
    main()
