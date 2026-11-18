//! Things with depth: a camera, and flat shapes in space drawn far to near. Enough for a road, the
//! walls beside it and what stands on it.
//!
//! x is to the right, y is up, z is ahead; lengths are metres and angles are radians.

use crate::gfx::{shade, Screen, H, W};

pub type V3 = [f64; 3];
const NEAR: f64 = 0.25;
const FAR_OFF: f64 = 20000.0;

#[derive(Clone, Copy)]
pub struct Cam {
    pub pos: V3,
    pub yaw: f64,
    pub pitch: f64,
    /// How many pixels a metre is, a metre away.
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
    polys: Vec<Poly>,
}

impl Scene3 {
    pub fn new(cam: Cam) -> Scene3 {
        Scene3 { cam, polys: Vec::new() }
    }

    /// Into the camera's own space: x to its right, y up, z ahead.
    fn to_cam(&self, p: V3) -> V3 {
        let (dx, dz) = (p[0] - self.cam.pos[0], p[2] - self.cam.pos[2]);
        let (s, c) = self.cam.yaw.sin_cos();
        let (rx, ry, rz) = (dx * c - dz * s, p[1] - self.cam.pos[1], dx * s + dz * c);
        let (ts, tc) = self.cam.pitch.sin_cos();
        [rx, ry * tc + rz * ts, rz * tc - ry * ts]
    }

    fn flat(&self, c: V3) -> (i32, i32) {
        let x = self.cam.cx as f64 + c[0] * self.cam.focal / c[2];
        let y = self.cam.cy as f64 - c[1] * self.cam.focal / c[2];
        (x.clamp(-FAR_OFF, FAR_OFF) as i32, y.clamp(-FAR_OFF, FAR_OFF) as i32)
    }

    /// Where a point is on the screen, and how far ahead of the camera. None if it is behind.
    pub fn point(&self, p: V3) -> Option<(i32, i32, f64)> {
        let c = self.to_cam(p);
        (c[2] >= NEAR).then(|| {
            let (x, y) = self.flat(c);
            (x, y, c[2])
        })
    }

    /// A flat shape, in two colours chequered. `bias` pushes it back (negative) or forward in the
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
        let flat: Vec<(i32, i32)> = out.iter().map(|&c| self.flat(c)).collect();
        if flat.iter().all(|p| p.0 < 0) || flat.iter().all(|p| p.0 >= W) || flat.iter().all(|p| p.1 < 0) || flat.iter().all(|p| p.1 >= H) {
            return;
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
    pub fn end(mut self, s: &mut Screen) -> usize {
        self.polys.sort_by(|a, b| b.key.total_cmp(&a.key));
        for p in &self.polys {
            s.poly(&p.pts, p.a, p.b);
        }
        self.polys.len()
    }
}
