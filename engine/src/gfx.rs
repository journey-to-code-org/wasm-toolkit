//! The screen: 320x240 palette indices, 1,024 colours in 32 ramps of 32 shades, and what draws on it.
//! The palette and the picture format are the same as the C toolkit's, so the same pictures work.

use crate::font::FONT;

pub const W: i32 = 320;
pub const H: i32 = 240;
pub const CLEAR: u16 = 0xFFFF;

/// The ramps of the default palette, by name, in order. Ramp 0 is the EGA sixteen and sixteen greys.
pub const RAMPS: [&str; 32] = [
    "SYSTEM", "GREY", "RED", "ORANGE", "YELLOW", "LIME", "GREEN", "TEAL", "CYAN", "SKY", "BLUE", "INDIGO", "VIOLET", "MAGENTA", "PINK", "BROWN",
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

/// The ramp with this name, or grey if there is none.
pub fn ramp(name: &str) -> usize {
    RAMPS.iter().position(|r| r.eq_ignore_ascii_case(name)).unwrap_or(1)
}

/// Shade `s` of a ramp: 0 is nearly black, 24 the colour itself, 31 a highlight.
pub fn shade(ramp: usize, s: i32) -> u16 {
    (ramp as i32 * 32 + s.clamp(0, 31)) as u16
}

/// A picture: palette indices, run-length, with `CLEAR` for what shows through.
#[derive(Clone, Debug, Default)]
pub struct Picture {
    pub w: i32,
    pub h: i32,
    pub runs: Vec<(u16, u16)>,
}

impl Picture {
    /// From the pack: little-endian words, width, height, then (count, colour) pairs.
    pub fn from_bytes(bytes: &[u8]) -> Option<Picture> {
        let words: Vec<u16> = bytes.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
        if words.len() < 2 || (words.len() - 2) % 2 != 0 {
            return None;
        }
        let runs: Vec<(u16, u16)> = words[2..].chunks_exact(2).map(|p| (p[0], p[1])).collect();
        let (w, h) = (words[0] as i32, words[1] as i32);
        let total: i64 = runs.iter().map(|r| r.0 as i64).sum();
        (total == w as i64 * h as i64).then_some(Picture { w, h, runs })
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
}

pub struct Screen {
    pub fb: Vec<u16>,
    pub pal: Vec<u32>,
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
        Screen { fb: vec![0; (W * H) as usize], pal }
    }

    pub fn clear(&mut self, c: u16) {
        self.fb.fill(c);
    }

    pub fn pset(&mut self, x: i32, y: i32, c: u16) {
        if x >= 0 && x < W && y >= 0 && y < H {
            self.fb[(y * W + x) as usize] = c;
        }
    }

    pub fn at(&self, x: i32, y: i32) -> u16 {
        self.fb[(y * W + x) as usize]
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u16) {
        for j in y.max(0)..(y + h).min(H) {
            for i in x.max(0)..(x + w).min(W) {
                self.fb[(j * W + i) as usize] = c;
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
        for j in -r..=r {
            for i in -r..=r {
                if i * i + j * j <= r * r + r / 2 {
                    self.pset(cx + i, cy + j, c);
                }
            }
        }
    }

    /// A box of two colours, chequered.
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
        for j in 0..h {
            let yy = y + j;
            if !(0..H).contains(&yy) {
                continue;
            }
            let s = s0 * 256 + (s1 - s0) * 256 * j / (h - 1).max(1);
            let (whole, part) = (s >> 8, (s & 255) >> 4);
            for xx in x.max(0)..(x + w).min(W) {
                let up = (part > BAYER[(yy & 3) as usize][(xx & 3) as usize]) as i32;
                self.fb[(yy * W + xx) as usize] = shade(ramp, whole + up);
            }
        }
    }

    pub fn picture(&mut self, pic: &Picture, x: i32, y: i32) {
        self.picture_tint(pic, x, y, None);
    }

    pub fn picture_tint(&mut self, pic: &Picture, x: i32, y: i32, tint: Option<&Tint>) {
        let mut i: i32 = 0;
        for &(count, value) in &pic.runs {
            if value != CLEAR {
                let c = match tint {
                    Some(t) if value >= 32 => {
                        let r = (value >> 5) as usize;
                        shade(t.to[r] as usize, (value & 31) as i32 + t.add[r] as i32)
                    }
                    _ => value,
                };
                for k in 0..count as i32 {
                    self.pset(x + (i + k) % pic.w, y + (i + k) / pic.w, c);
                }
            }
            i += count as i32;
        }
    }

    /// A picture at half size, for a car seen from further off.
    pub fn picture_half(&mut self, pic: &Picture, x: i32, y: i32, tint: Option<&Tint>) {
        let mut i: i32 = 0;
        for &(count, value) in &pic.runs {
            if value != CLEAR {
                let c = match tint {
                    Some(t) if value >= 32 => {
                        let r = (value >> 5) as usize;
                        shade(t.to[r] as usize, (value & 31) as i32 + t.add[r] as i32)
                    }
                    _ => value,
                };
                for k in i..i + count as i32 {
                    let (px, py) = (k % pic.w, k / pic.w);
                    if px & 1 == 0 && py & 1 == 0 {
                        self.pset(x + px / 2, y + py / 2, c);
                    }
                }
            }
            i += count as i32;
        }
    }

    /// Text in the 8x8 font. Returns the x after the last character.
    pub fn text(&mut self, mut x: i32, y: i32, s: &str, c: u16) -> i32 {
        for ch in s.bytes() {
            let ch = if (32..128).contains(&ch) { ch } else { b'?' };
            let glyph = &FONT[(ch - 32) as usize];
            for (j, row) in glyph.iter().enumerate() {
                for i in 0..8 {
                    if row & (1 << i) != 0 {
                        self.pset(x + i, y + j as i32, c);
                    }
                }
            }
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
            let ch = if (32..128).contains(&ch) { ch } else { b'?' };
            let glyph = &FONT[(ch - 32) as usize];
            for (j, row) in glyph.iter().enumerate() {
                for i in 0..8 {
                    if row & (1 << i) != 0 {
                        self.rect(x + i * scale, y + j as i32 * scale, scale, scale, c);
                    }
                }
            }
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
