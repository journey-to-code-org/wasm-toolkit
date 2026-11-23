//! Things with depth: a camera, and flat shapes in space drawn far to near. Enough for a road, the
//! walls beside it and what stands on it.
//!
//! x is to the right, y is up, z is ahead; lengths are meters and angles are radians.

use crate::gfx::{shade, Screen, PH, PW, S, W};

pub type V3 = [f64; 3];
const NEAR: f64 = 0.25;
const FAR_OFF: f64 = 20000.0;

#[derive(Clone, Copy)]
pub struct Cam {
    pub pos: V3,
    pub yaw: f64,
    pub pitch: f64,
    /// How many pixels a meter is, a meter away.
    pub focal: f64,
    /// Where on the screen straight ahead is.
    pub cx: i32,
    pub cy: i32,
}

struct Poly {
    key: f64,
    pts: Vec<(i32, i32)>,
    a: u16,
    b: u16,
}

pub struct Scene3 {
    pub cam: Cam,
    /// The dark: how many shades everything is taken down, and how far ahead the car's lamps reach,
    /// in meters. What is nearer than that is lit, and it fades to the dark from there.
    pub dim: i32,
    pub lamps: f64,
    polys: Vec<Poly>,
    sprites: Vec<Sprite>,
}

/// A flat picture standing in the scene, facing the camera: which one, and where on the screen.
struct Sprite {
    key: f64,
    id: usize,
    at: (i32, i32, i32, i32),
}

impl Scene3 {
    pub fn new(cam: Cam) -> Scene3 {
        Scene3 { cam, dim: 0, lamps: 0.0, polys: Vec::new(), sprites: Vec::new() }
    }

    /// Into the camera's own space: x to its right, y up, z ahead.
    fn to_cam(&self, p: V3) -> V3 {
        let (dx, dz) = (p[0] - self.cam.pos[0], p[2] - self.cam.pos[2]);
        let (s, c) = self.cam.yaw.sin_cos();
        let (rx, ry, rz) = (dx * c - dz * s, p[1] - self.cam.pos[1], dx * s + dz * c);
        let (ts, tc) = self.cam.pitch.sin_cos();
        [rx, ry * tc + rz * ts, rz * tc - ry * ts]
    }

    /// Where a point in the camera's space is on the screen, in places and parts of places.
    fn place(&self, c: V3) -> (f64, f64) {
        ((self.cam.cx as f64 + c[0] * self.cam.focal / c[2]).clamp(-FAR_OFF, FAR_OFF), (self.cam.cy as f64 - c[1] * self.cam.focal / c[2]).clamp(-FAR_OFF, FAR_OFF))
    }

    /// The place it is in.
    fn flat(&self, c: V3) -> (i32, i32) {
        let (x, y) = self.place(c);
        (x as i32, y as i32)
    }

    /// The dot it is on: shapes are drawn as finely as the screen has dots.
    fn fine(&self, c: V3) -> (i32, i32) {
        let (x, y) = self.place(c);
        ((x * S as f64) as i32, (y * S as f64) as i32)
    }

    /// Where a point is on the screen, and how far ahead of the camera. None if it is behind.
    pub fn point(&self, p: V3) -> Option<(i32, i32, f64)> {
        let c = self.to_cam(p);
        (c[2] >= NEAR).then(|| {
            let (x, y) = self.flat(c);
            (x, y, c[2])
        })
    }

    /// A flat shape, in two colors chequered. `bias` pushes it back (negative) or forward in the
    /// drawing order, for things that lie on one another.
    pub fn poly(&mut self, pts: &[V3], a: u16, b: u16, bias: f64) {
        self.poly_at(pts, a, b, bias, None);
    }

    /// A polygon drawn in an order that is given, not worked out: the bigger the key, the sooner
    /// it is drawn and the more goes over it. For a road, which knows what is in front of what.
    pub fn poly_key(&mut self, pts: &[V3], a: u16, b: u16, key: f64) {
        self.poly_at(pts, a, b, 0.0, Some(key));
    }

    fn poly_at(&mut self, pts: &[V3], a: u16, b: u16, bias: f64, key: Option<f64>) {
        let cam: Vec<V3> = pts.iter().map(|&p| self.to_cam(p)).collect();
        let depth = cam.iter().map(|c| c[2]).sum::<f64>() / cam.len() as f64;
        // Cut away what is behind the picture plane.
        let mut out: Vec<V3> = Vec::new();
        for i in 0..cam.len() {
            let (p, q) = (cam[i], cam[(i + 1) % cam.len()]);
            if p[2] >= NEAR {
                out.push(p);
            }
            if (p[2] >= NEAR) != (q[2] >= NEAR) {
                let t = (NEAR - p[2]) / (q[2] - p[2]);
                out.push([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t, NEAR]);
            }
        }
        if out.len() < 3 {
            return;
        }
        let flat: Vec<(i32, i32)> = out.iter().map(|&c| self.fine(c)).collect();
        if flat.iter().all(|p| p.0 < 0) || flat.iter().all(|p| p.0 >= PW) || flat.iter().all(|p| p.1 < 0) || flat.iter().all(|p| p.1 >= PH) {
            return;
        }
        let (mut a, mut b) = (a, b);
        if self.dim > 0 {
            let by = (self.dim as f64 * if self.lamps > 0.0 { (depth / self.lamps).clamp(0.45, 1.0) } else { 1.0 }) as u16;
            let down = |c: u16| if c < 32 { c } else { (c & !31) | (c & 31).saturating_sub(by).max(1) };
            (a, b) = (down(a), down(b));
        }
        self.polys.push(Poly { key: key.unwrap_or(depth - bias), pts: flat, a, b });
    }

    /// A flat shape in one ramp, lit from above, the west and the south.
    pub fn lit(&mut self, pts: &[V3], ramp: usize, bias: f64) {
        self.lit_at(pts, ramp, bias, None);
    }

    pub fn lit_key(&mut self, pts: &[V3], ramp: usize, key: f64) {
        self.lit_at(pts, ramp, 0.0, Some(key));
    }

    fn lit_at(&mut self, pts: &[V3], ramp: usize, bias: f64, key: Option<f64>) {
        let sub = |a: V3, b: V3| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let (u, v) = (sub(pts[1], pts[0]), sub(pts[pts.len() - 1], pts[0]));
        let mut n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len == 0.0 {
            return self.poly_at(pts, shade(ramp, 16), shade(ramp, 16), bias, key);
        }
        // Whichever side faces the camera is the side that is lit.
        let to = sub(self.cam.pos, pts[0]);
        if n[0] * to[0] + n[1] * to[1] + n[2] * to[2] < 0.0 {
            n = [-n[0], -n[1], -n[2]];
        }
        let d = (n[0] * -0.45 + n[1] * 0.80 + n[2] * -0.40) / len;
        let s = 8.0 + if d > 0.0 { d * 17.0 } else { d * 3.0 } + if n[1] / len > 0.94 { 2.0 } else { 0.0 };
        self.poly_at(pts, shade(ramp, s.floor() as i32), shade(ramp, (s + 0.5).floor() as i32), bias, key);
    }

    /// Paint everything, far to near. Returns how many shapes there were.
    pub fn end(self, s: &mut Screen) -> usize {
        self.end_with(s, |_, _, _| {})
    }

    /// A picture `width` meters wide and `aspect` times as tall, standing with the middle of its
    /// foot at `feet`, drawn in its turn by whoever ends the scene. False if it cannot be seen.
    pub fn sprite(&mut self, feet: V3, width: f64, aspect: f64, id: usize, key: f64) -> bool {
        let c = self.to_cam(feet);
        if c[2] < NEAR * 4.0 {
            return false;
        }
        let ((x, y), w) = (self.flat(c), (width * self.cam.focal / c[2]) as i32);
        let h = (w as f64 * aspect) as i32;
        if !(2..=640).contains(&w) || h < 1 || x + w / 2 < 0 || x - w / 2 >= W {
            return false;
        }
        self.sprites.push(Sprite { key, id, at: (x - w / 2, y - h, w, h) });
        true
    }

    /// Draw it all, farthest first. `draw` is handed each picture's number and its box on the screen.
    pub fn end_with(mut self, s: &mut Screen, mut draw: impl FnMut(&mut Screen, usize, (i32, i32, i32, i32))) -> usize {
        self.polys.sort_by(|a, b| b.key.total_cmp(&a.key));
        self.sprites.sort_by(|a, b| b.key.total_cmp(&a.key));
        let mut next = 0;
        for p in &self.polys {
            while next < self.sprites.len() && self.sprites[next].key >= p.key {
                draw(s, self.sprites[next].id, self.sprites[next].at);
                next += 1;
            }
            s.poly_fine(&p.pts, p.a, p.b);
        }
        for sp in &self.sprites[next..] {
            draw(s, sp.id, sp.at);
        }
        self.polys.len()
    }
}
