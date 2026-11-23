//! The screen: 320x240 palette indices, 1,024 colors in 32 ramps of 32 shades, and what draws on it.
//! The palette and the picture format are the same as the C toolkit's, so the same pictures work.

use crate::font::FONT;

/// The screen as everything is placed on it: 320 by 240 places. Each place is `S` dots across
/// and down, so what is drawn from shapes, not pictures (the view from the seat, lines, needles,
/// letters) is drawn finer than what it is placed by.
pub const W: i32 = 320;
pub const H: i32 = 240;
pub const S: i32 = 2;
/// The screen in dots.
pub const PW: i32 = W * S;
pub const PH: i32 = H * S;
pub const CLEAR: u16 = 0xFFFF;

/// The ramps of the default palette, by name, in order. Ramp 0 is the EGA sixteen and sixteen grays.
pub const RAMPS: [&str; 32] = [
    "SYSTEM", "GRAY", "RED", "ORANGE", "YELLOW", "LIME", "GREEN", "TEAL", "CYAN", "SKY", "BLUE", "INDIGO", "VIOLET", "MAGENTA", "PINK", "BROWN",
    "TAN", "BRICK", "CONCRETE", "ASPHALT", "GRASS", "GLASS", "CHROME", "RUBBER", "WOOD", "CREAM", "NAVY", "MAROON", "FOREST", "GOLD", "SILVER", "WHITE",
];
const BASES: [[u8; 3]; 32] = [
    [0, 0, 0], [150, 150, 150], [214, 40, 40], [236, 120, 28], [240, 206, 48], [150, 212, 40], [44, 170, 60], [30, 160, 140],
    [40, 190, 214], [110, 170, 240], [48, 90, 214], [76, 60, 190], [136, 70, 200], [206, 60, 180], [240, 130, 170], [130, 82, 44],
    [200, 170, 120], [166, 80, 58], [168, 164, 156], [74, 76, 84], [70, 130, 50], [96, 140, 170], [186, 196, 208], [46, 46, 50],
    [150, 100, 56], [236, 224, 192], [30, 46, 110], [120, 24, 40], [30, 84, 50], [212, 170, 60], [176, 180, 188], [236, 236, 240],
];
const EGA: [[u8; 3]; 16] = [
    [0, 0, 0], [0, 0, 170], [0, 170, 0], [0, 170, 170], [170, 0, 0], [170, 0, 170], [170, 85, 0], [170, 170, 170],
    [85, 85, 85], [85, 85, 255], [85, 255, 85], [85, 255, 255], [255, 85, 85], [255, 85, 255], [255, 255, 85], [255, 255, 255],
];
pub const BLACK: u16 = 0;
pub const WHITE: u16 = 15;

/// The ramp with this name, or gray if there is none (GREY, as packs once had it, is gray too).
pub fn ramp(name: &str) -> usize {
    RAMPS.iter().position(|r| r.eq_ignore_ascii_case(name)).unwrap_or(1)
}

/// Shade `s` of a ramp: 0 is nearly black, 24 the color itself, 31 a highlight.
pub fn shade(ramp: usize, s: i32) -> u16 {
    (ramp as i32 * 32 + s.clamp(0, 31)) as u16
}

/// A picture: palette indices, run-length, with `CLEAR` for what shows through.
#[derive(Clone, Debug, Default)]
pub struct Picture {
    /// How big it is on the screen, in places.
    pub w: i32,
    pub h: i32,
    /// How many dots of it there are to a place, across and down: 1, or 2 for one painted fine.
    pub s: i32,
    pub runs: Vec<(u16, u16)>,
    /// The same picture a pixel at a time, for turning and scaling it.
    pub ix: Vec<u16>,
}

impl Picture {
    /// From the pack: little-endian words, width, height, then (count, color) pairs. A width
    /// with its top bit set is of a picture painted fine: twice the dots each way for its size.
    pub fn from_bytes(bytes: &[u8]) -> Option<Picture> {
        let words: Vec<u16> = bytes.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
        if words.len() < 2 || (words.len() - 2) % 2 != 0 {
            return None;
        }
        let runs: Vec<(u16, u16)> = words[2..].chunks_exact(2).map(|p| (p[0], p[1])).collect();
        let fine = words[0] & 0x8000 != 0;
        let (w, h) = ((words[0] & 0x7FFF) as i32, words[1] as i32);
        let total: i64 = runs.iter().map(|r| r.0 as i64).sum();
        if total != w as i64 * h as i64 || fine && (w % 2 != 0 || h % 2 != 0) {
            return None;
        }
        let ix = runs.iter().flat_map(|&(n, c)| std::iter::repeat_n(c, n as usize)).collect();
        let s = if fine { 2 } else { 1 };
        Some(Picture { w: w / s, h: h / s, s, runs, ix })
    }

    /// The dot of it at a place and a part of a place: `x` and `y` in dots of the screen.
    #[inline]
    fn dot(&self, x: i32, y: i32) -> u16 {
        let (px, py) = (x * self.s / S, y * self.s / S);
        self.ix[(py * self.w * self.s + px) as usize]
    }
}

/// Which ramp each ramp is drawn in, and how many shades lighter: one picture of a car, in any paint.
pub struct Tint {
    pub to: [u8; 32],
    pub add: [i8; 32],
}

impl Tint {
    pub fn none() -> Tint {
        let mut to = [0u8; 32];
        for (i, t) in to.iter_mut().enumerate() {
            *t = i as u8;
        }
        Tint { to, add: [0; 32] }
    }

    pub fn apply(&self, value: u16) -> u16 {
        if value < 32 {
            return value;
        }
        let r = (value >> 5) as usize;
        shade(self.to[r] as usize, (value & 31) as i32 + self.add[r] as i32)
    }
}

pub struct Screen {
    /// Every dot of it: `PW` across and `PH` down.
    pub fb: Vec<u16>,
    pub pal: Vec<u32>,
    /// Drawing is kept inside this box, in dots: left, top, right, bottom.
    clip: [i32; 4],
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen {
    pub fn new() -> Screen {
        let mut pal = vec![0xFF00_0000u32; 1024];
        let rgb = |r: i32, g: i32, b: i32| 0xFF00_0000u32 | ((b as u32 & 255) << 16) | ((g as u32 & 255) << 8) | (r as u32 & 255);
        for r in 1..32 {
            for s in 0..32i32 {
                let mut v = [0i32; 3];
                for k in 0..3 {
                    let c = BASES[r][k] as i32;
                    v[k] = if s <= 24 { c * (26 + s * 230 / 24) / 256 } else { c + (255 - c) * (s - 24) / 10 };
                }
                pal[r * 32 + s as usize] = rgb(v[0], v[1], v[2]);
            }
        }
        for i in 16..32 {
            let g = (i - 16) * 17;
            pal[i as usize] = rgb(g, g, g);
        }
        for (i, c) in EGA.iter().enumerate() {
            pal[i] = rgb(c[0] as i32, c[1] as i32, c[2] as i32);
        }
        Screen { fb: vec![0; (PW * PH) as usize], pal, clip: [0, 0, PW, PH] }
    }

    pub fn clear(&mut self, c: u16) {
        self.fb.fill(c);
    }

    /// Keep drawing inside a box, and let it out again.
    pub fn clip(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.clip = [x.max(0) * S, y.max(0) * S, (x + w).min(W) * S, (y + h).min(H) * S];
    }

    pub fn clip_off(&mut self) {
        self.clip = [0, 0, PW, PH];
    }

    /// One dot.
    #[inline]
    fn dot(&mut self, x: i32, y: i32, c: u16) {
        if x >= self.clip[0] && x < self.clip[2] && y >= self.clip[1] && y < self.clip[3] {
            self.fb[(y * PW + x) as usize] = c;
        }
    }

    /// A box of dots.
    fn dots(&mut self, x: i32, y: i32, w: i32, h: i32, c: u16) {
        let (x0, x1) = (x.max(self.clip[0]), (x + w).min(self.clip[2]));
        if x0 >= x1 {
            return;
        }
        for j in y.max(self.clip[1])..(y + h).min(self.clip[3]) {
            self.fb[(j * PW + x0) as usize..(j * PW + x1) as usize].fill(c);
        }
    }

    /// One place.
    pub fn pset(&mut self, x: i32, y: i32, c: u16) {
        self.dots(x * S, y * S, S, S, c);
    }

    /// Take a box of the screen down so many shades: the dark coming on.
    pub fn darken(&mut self, x: i32, y: i32, w: i32, h: i32, by: u16) {
        for py in (y * S).max(self.clip[1])..((y + h) * S).min(self.clip[3]) {
            for px in (x * S).max(self.clip[0])..((x + w) * S).min(self.clip[2]) {
                let c = self.fb[(py * PW + px) as usize];
                if c >= 32 {
                    self.fb[(py * PW + px) as usize] = (c & !31) | (c & 31).saturating_sub(by).max(1);
                }
            }
        }
    }

    /// What is at a place: its top left dot.
    pub fn at(&self, x: i32, y: i32) -> u16 {
        self.fb[(y * S * PW + x * S) as usize]
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u16) {
        self.dots(x * S, y * S, w * S, h * S, c);
    }

    /// A line from the middle of one place to the middle of another, a place thick.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u16) {
        let (mut x0, mut y0, x1, y1) = (x0 * S, y0 * S, x1 * S, y1 * S);
        let (dx, sx) = ((x1 - x0).abs(), if x0 < x1 { 1 } else { -1 });
        let (dy, sy) = (-(y1 - y0).abs(), if y0 < y1 { 1 } else { -1 });
        let mut err = dx + dy;
        for _ in 0..8192 {
            self.dots(x0, y0, S, S, c);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// A filled shape with no dents in it, in two colors chequered (the same twice for a flat one).
    pub fn poly(&mut self, pts: &[(i32, i32)], a: u16, b: u16) {
        let fine: Vec<(i32, i32)> = pts.iter().map(|p| (p.0 * S, p.1 * S)).collect();
        self.poly_fine(&fine, a, b);
    }

    /// The same, with its corners given in dots.
    pub fn poly_fine(&mut self, pts: &[(i32, i32)], a: u16, b: u16) {
        let top = pts.iter().map(|p| p.1).min().unwrap_or(0).max(self.clip[1]);
        let bottom = pts.iter().map(|p| p.1).max().unwrap_or(-1).min(self.clip[3] - 1);
        if top > bottom {
            return;
        }
        let rows = (bottom - top + 1) as usize;
        let (mut lo, mut hi) = (vec![i32::MAX; rows], vec![i32::MIN; rows]);
        let mut mark = |y: i32, x: i32| {
            let r = (y - top) as usize;
            lo[r] = lo[r].min(x);
            hi[r] = hi[r].max(x);
        };
        for i in 0..pts.len() {
            let ((mut x0, mut y0), (mut x1, mut y1)) = (pts[i], pts[(i + 1) % pts.len()]);
            if y0 > y1 {
                std::mem::swap(&mut x0, &mut x1);
                std::mem::swap(&mut y0, &mut y1);
            }
            if y0 == y1 {
                if y0 >= top && y0 <= bottom {
                    mark(y0, x0);
                    mark(y0, x1);
                }
                continue;
            }
            let step = (x1 - x0) as f64 / (y1 - y0) as f64;
            for y in y0.max(top)..=y1.min(bottom) {
                mark(y, (x0 as f64 + step * (y - y0) as f64 + 0.5).floor().clamp(-32000.0, 32000.0) as i32);
            }
        }
        for (r, (&l, &h)) in lo.iter().zip(&hi).enumerate() {
            let y = top + r as i32;
            let (l, h) = (l.max(self.clip[0]), h.min(self.clip[2] - 1));
            if l > h {
                continue;
            }
            let row = &mut self.fb[(y * PW + l) as usize..=(y * PW + h) as usize];
            if a == b {
                row.fill(a);
            } else {
                // Chequered by the dot: two shades of a surface, mixed.
                for (k, d) in row.iter_mut().enumerate() {
                    *d = if (l + k as i32 + y) & 1 == 0 { a } else { b };
                }
            }
        }
    }

    /// Any shape at all, dents and holes too, in places: a dot is in it if a line from it crosses
    /// the outline an odd number of times. Two colors chequered, as `poly`.
    pub fn shape(&mut self, pts: &[(i32, i32)], a: u16, b: u16) {
        if pts.len() < 3 {
            return;
        }
        let fine: Vec<(f64, f64)> = pts.iter().map(|p| ((p.0 * S) as f64, (p.1 * S) as f64)).collect();
        let top = (fine.iter().map(|p| p.1).fold(f64::MAX, f64::min) as i32).max(self.clip[1]);
        let bottom = (fine.iter().map(|p| p.1).fold(f64::MIN, f64::max) as i32).min(self.clip[3] - 1);
        let mut xs: Vec<i32> = Vec::new();
        for y in top..=bottom {
            let py = y as f64 + 0.5;
            xs.clear();
            for i in 0..fine.len() {
                let ((ax, ay), (bx, by)) = (fine[i], fine[(i + 1) % fine.len()]);
                if (ay > py) != (by > py) {
                    xs.push((ax + (bx - ax) * (py - ay) / (by - ay)).round() as i32);
                }
            }
            xs.sort_unstable();
            for pair in xs.chunks(2) {
                let [l, r] = [pair[0], *pair.get(1).unwrap_or(&pair[0])];
                for x in l.max(self.clip[0])..r.min(self.clip[2]) {
                    self.fb[(y * PW + x) as usize] = if (x + y) & 1 == 0 { a } else { b };
                }
            }
        }
    }

    /// The outline of a shape, a dot thick.
    pub fn outline(&mut self, pts: &[(i32, i32)], c: u16) {
        for i in 0..pts.len() {
            let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
            let (mut x0, mut y0, x1, y1) = (p.0 * S, p.1 * S, q.0 * S, q.1 * S);
            let (dx, sx) = ((x1 - x0).abs(), if x0 < x1 { 1 } else { -1 });
            let (dy, sy) = (-(y1 - y0).abs(), if y0 < y1 { 1 } else { -1 });
            let mut err = dx + dy;
            for _ in 0..8192 {
                self.dot(x0, y0, c);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        }
    }

    /// A square picture turned about its middle, which goes at (cx, cy). The angle is in degrees.
    pub fn spin(&mut self, pic: &Picture, cx: i32, cy: i32, degrees: f64) {
        let (s, c) = degrees.to_radians().sin_cos();
        let half = pic.w / 2 * S;
        for dy in -half..half {
            for dx in -half..half {
                // Where this dot came from, before the turn.
                let sx = half + ((dx as f64 + 0.5) * c + (dy as f64 + 0.5) * s).floor() as i32;
                let sy = half + ((dy as f64 + 0.5) * c - (dx as f64 + 0.5) * s).floor() as i32;
                if sx < 0 || sy < 0 || sx >= pic.w * S || sy >= pic.h * S {
                    continue;
                }
                let v = pic.dot(sx, sy);
                if v != CLEAR {
                    self.dot(cx * S + dx, cy * S + dy, v);
                }
            }
        }
    }

    /// The same, mirrored left to right: a car's side seen going the other way.
    pub fn picture_sized_mirror(&mut self, pic: &Picture, x: i32, y: i32, w: i32, h: i32, tint: Option<&Tint>) {
        self.sized(pic, x, y, w, h, tint, true);
    }

    /// A picture drawn `w` by `h`, whatever size it is: for something seen from a distance.
    pub fn picture_sized(&mut self, pic: &Picture, x: i32, y: i32, w: i32, h: i32, tint: Option<&Tint>) {
        self.sized(pic, x, y, w, h, tint, false);
    }

    fn sized(&mut self, pic: &Picture, x: i32, y: i32, w: i32, h: i32, tint: Option<&Tint>, mirror: bool) {
        if w <= 0 || h <= 0 {
            return;
        }
        // Every dot of the box takes the dot of the picture that falls on it.
        let (w, h, pw, ph) = (w * S, h * S, pic.w * pic.s, pic.h * pic.s);
        for j in (self.clip[1] - y * S).max(0)..h.min(self.clip[3] - y * S) {
            let row = (j * ph / h) * pw;
            for i in (self.clip[0] - x * S).max(0)..w.min(self.clip[2] - x * S) {
                let v = pic.ix[(row + if mirror { w - 1 - i } else { i } * pw / w) as usize];
                if v != CLEAR {
                    self.fb[((y * S + j) * PW + x * S + i) as usize] = tint.map_or(v, |t| t.apply(v));
                }
            }
        }
    }

    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, c: u16) {
        self.rect(x, y, w, 1, c);
        self.rect(x, y + h - 1, w, 1, c);
        self.rect(x, y, 1, h, c);
        self.rect(x + w - 1, y, 1, h, c);
    }

    /// A filled circle.
    pub fn disc(&mut self, cx: i32, cy: i32, r: i32, c: u16) {
        // Round in dots: about the middle of its middle place.
        let (mx, my, rr) = (cx * S + S / 2, cy * S + S / 2, r * S + S / 2);
        for j in -rr..rr {
            for i in -rr..rr {
                let (fx, fy) = (2 * i + 1, 2 * j + 1);
                if fx * fx + fy * fy <= 4 * rr * rr {
                    self.dot(mx + i, my + j, c);
                }
            }
        }
    }

    /// A box of two colors, chequered.
    pub fn dither(&mut self, x: i32, y: i32, w: i32, h: i32, a: u16, b: u16) {
        for j in y..y + h {
            for i in x..x + w {
                self.pset(i, j, if (i + j) & 1 == 0 { a } else { b });
            }
        }
    }

    /// A box that fades down a ramp from shade `s0` to `s1`, dithered between shades.
    pub fn vgrad(&mut self, x: i32, y: i32, w: i32, h: i32, ramp: usize, s0: i32, s1: i32) {
        const BAYER: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
        let (x, y, w, h) = (x * S, y * S, w * S, h * S);
        for j in 0..h {
            let yy = y + j;
            if !(0..PH).contains(&yy) {
                continue;
            }
            let s = s0 * 256 + (s1 - s0) * 256 * j / (h - 1).max(1);
            let (whole, part) = (s >> 8, (s & 255) >> 4);
            for xx in x.max(0)..(x + w).min(PW) {
                let up = (part > BAYER[(yy & 3) as usize][(xx & 3) as usize]) as i32;
                self.fb[(yy * PW + xx) as usize] = shade(ramp, whole + up);
            }
        }
    }

    pub fn picture(&mut self, pic: &Picture, x: i32, y: i32) {
        self.picture_tint(pic, x, y, None);
    }

    pub fn picture_tint(&mut self, pic: &Picture, x: i32, y: i32, tint: Option<&Tint>) {
        // A dot of the picture is a place on the screen, or a dot of it if it was painted fine.
        let (mut i, wide, each) = (0i32, (pic.w * pic.s).max(1), S / pic.s.max(1));
        for &(count, value) in &pic.runs {
            if value != CLEAR {
                let c = tint.map_or(value, |t| t.apply(value));
                let mut k = i;
                while k < i + count as i32 {
                    // As much of the run as is on this row, at once.
                    let (px, py) = (k % wide, k / wide);
                    let n = (wide - px).min(i + count as i32 - k);
                    self.dots(x * S + px * each, y * S + py * each, n * each, each, c);
                    k += n;
                }
            }
            i += count as i32;
        }
    }

    /// A picture at half size, for a car seen from further off.
    pub fn picture_half(&mut self, pic: &Picture, x: i32, y: i32, tint: Option<&Tint>) {
        self.sized(pic, x, y, pic.w / 2, pic.h / 2, tint, false);
    }

    /// One letter of the 8x8 font, each of its places drawn as four dots with the corners of
    /// its slanting strokes filled or cut, so that they slant and do not step.
    fn glyph(&mut self, x: i32, y: i32, ch: u8, c: u16, scale: i32) {
        let ch = if (32..128).contains(&ch) { ch } else { b'?' };
        let glyph = &FONT[(ch - 32) as usize];
        let on = |i: i32, j: i32| (0..8).contains(&i) && (0..8).contains(&j) && glyph[j as usize] & (1 << i) != 0;
        let (half, x, y) = (scale * S / 2, x * S, y * S);
        for j in 0..8 {
            for i in 0..8 {
                let (p, up, down, left, right) = (on(i, j), on(i, j - 1), on(i, j + 1), on(i - 1, j), on(i + 1, j));
                // Each quarter takes after the two neighbors it lies between, when they agree
                // with each other and not with the two across from them.
                let quarter = |a: bool, b: bool, far_a: bool, far_b: bool| if a == b && a != far_a && b != far_b { a } else { p };
                for (qx, qy, lit) in [(0, 0, quarter(left, up, right, down)), (1, 0, quarter(right, up, left, down)), (0, 1, quarter(left, down, right, up)), (1, 1, quarter(right, down, left, up))] {
                    if lit {
                        self.dots(x + (i * 2 + qx) * half, y + (j * 2 + qy) * half, half, half, c);
                    }
                }
            }
        }
    }

    /// Text in the 8x8 font. Returns the x after the last character.
    pub fn text(&mut self, mut x: i32, y: i32, s: &str, c: u16) -> i32 {
        for ch in s.bytes() {
            self.glyph(x, y, ch, c, 1);
            x += 8;
        }
        x
    }

    pub fn text_sh(&mut self, x: i32, y: i32, s: &str, c: u16, shadow: u16) -> i32 {
        self.text(x + 1, y + 1, s, shadow);
        self.text(x, y, s, c)
    }

    pub fn text_big(&mut self, mut x: i32, y: i32, s: &str, c: u16, scale: i32) -> i32 {
        for ch in s.bytes() {
            self.glyph(x, y, ch, c, scale);
            x += 8 * scale;
        }
        x
    }

    /// The pointer: a white arrow with a dark edge, its tip at (x, y).
    pub fn pointer(&mut self, x: i32, y: i32) {
        const ROWS: [i32; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 4, 2, 1];
        for (j, &n) in ROWS.iter().enumerate() {
            for i in 0..n {
                let edge = i == 0 || i == n - 1 || j == 8;
                self.pset(x + i, y + j as i32, if edge { BLACK } else { WHITE });
            }
        }
    }
}
