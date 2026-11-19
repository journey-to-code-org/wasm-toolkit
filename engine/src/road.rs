//! A road out of town: two lanes that bend and climb, other cars both ways, and police.
//!
//! The road is cut into short lengths, each with a bend and a height. Along it, `z` is metres
//! from the start; across it, `x` is in half-widths: 0 the centre line, 0.5 the middle of your
//! lane, 1 the edge, and past that whatever is beside the road.

use crate::pack::{Road, Stretch};
pub use crate::town::Pedals;

pub const HZ: f64 = 30.0;
/// The length of one piece of road, and half its width, in metres.
pub const SEG: f64 = 6.0;
pub const HALF: f64 = 5.0;
pub const MPH: f64 = 2.237;
const PAINTS: [&str; 8] = ["BLUE", "SILVER", "WHITE", "GREEN", "GOLD", "TEAL", "MAROON", "ORANGE"];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Side {
    Trees,
    Rock,
    Drop,
    Fields,
    Posts,
}

impl Side {
    fn of(name: &str) -> Side {
        match name {
            "rock" => Side::Rock,
            "drop" => Side::Drop,
            "fields" => Side::Fields,
            "posts" => Side::Posts,
            _ => Side::Trees,
        }
    }

    /// Whether leaving the road this side is into something.
    pub fn solid(self) -> bool {
        matches!(self, Side::Trees | Side::Rock | Side::Drop)
    }
}

/// One piece of road: how it bends, how high it starts, and what is on each side.
#[derive(Clone, Copy, Debug)]
pub struct Seg {
    pub curve: f64,
    pub y: f64,
    pub left: Side,
    pub right: Side,
}

/// Another car: where it is, which lane (0.5 yours, -0.5 the oncoming one), and how fast, which
/// is less than nothing for one coming the other way.
pub struct Other {
    pub z: f64,
    pub lane: f64,
    pub speed: f64,
    pub paint: &'static str,
}

pub struct Cop {
    pub z: f64,
    pub done: bool,
}

/// What the car being driven can do on a road: how hard it pulls, in metres a second each second
/// from rest; how fast it will go, in metres a second; and how much a bend throws it, 1 being usual.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pace {
    pub accel: f64,
    pub top: f64,
    pub slide: f64,
}

impl Default for Pace {
    fn default() -> Pace {
        Pace { accel: 5.0, top: 56.0, slide: 1.0 }
    }
}

/// Someone to race down the road: where they are, across and along, and what their car can do.
pub struct Rival {
    pub z: f64,
    pub x: f64,
    pub speed: f64,
    pub top: f64,
    pub accel: f64,
    pub paint: String,
}

impl Rival {
    /// Off the line beside you, in the other lane, in a car that runs the quarter in `et` seconds.
    pub fn new(et: f64, paint: &str) -> Rival {
        let quick = 14.0 / et.clamp(8.0, 30.0);
        Rival { z: 0.0, x: -0.5, speed: 0.0, top: (56.0 * quick.powf(1.3)).clamp(30.0, 76.0), accel: (5.0 * quick * quick).clamp(2.5, 9.0), paint: paint.to_string() }
    }
}

/// What a thirtieth of a second brought.
#[derive(Default, Debug, PartialEq)]
pub struct Happened {
    pub went: f64,
    pub crashed: bool,
    /// The law lit up behind you.
    pub siren: bool,
    /// You were written up, for this many dollars.
    pub ticket: Option<i32>,
    pub lost_him: bool,
    pub arrived: bool,
    /// Whoever you were racing got to the end first.
    pub beaten: bool,
}

pub struct Trip {
    pub road: String,
    /// Driven from its second end to its first.
    pub backwards: bool,
    pub segs: Vec<Seg>,
    pub z: f64,
    pub x: f64,
    pub speed: f64,
    pub steer: f64,
    pub crash: i32,
    pub scrapes: u32,
    pub others: Vec<Other>,
    pub cops: Vec<Cop>,
    /// Being chased: for how many ticks, and the most over the limit you have been seen at, in mph.
    pub chase: Option<(u32, f64)>,
    fled: u32,
    /// Police are waiting not far ahead: what a radar detector would say.
    pub radar: bool,
    pub pace: Pace,
    /// Nitrous going in: how much more the car makes just now, as a fraction.
    pub spray: f64,
    /// A race: who against, and how many ticks until the off.
    pub rival: Option<Rival>,
    pub hold: i32,
    rng: u32,
}

impl Trip {
    pub fn new(id: &str, road: &Road, backwards: bool, seed: u32) -> Trip {
        let mut parts: Vec<Stretch> = road.parts.clone();
        if backwards {
            // The same road the other way: the bends and the climbs are turned about, and so are the sides.
            parts.reverse();
            for p in &mut parts {
                (p.curve, p.hill) = (-p.curve, -p.hill);
                std::mem::swap(&mut p.left, &mut p.right);
            }
        }
        let mut segs = Vec::new();
        let mut y = 0.0;
        for p in &parts {
            let n = (p.len / SEG).round().max(1.0) as usize;
            for i in 0..n {
                // A bend eases in and out; a hill is steepest in its middle.
                let t = i as f64 / n as f64;
                let ease = (t / 0.25).min((1.0 - t) / 0.25).min(1.0);
                segs.push(Seg { curve: p.curve * ease, y: y + p.hill * t * t * (3.0 - 2.0 * t), left: Side::of(&p.left), right: Side::of(&p.right) });
            }
            y += p.hill;
        }
        let len = segs.len() as f64 * SEG;
        let mut trip = Trip { road: id.to_string(), backwards, segs, z: 0.0, x: 0.5, speed: 0.0, steer: 0.0, crash: 0, scrapes: 0, others: Vec::new(), cops: Vec::new(), chase: None, fled: 0, radar: false, pace: Pace::default(), spray: 0.0, rival: None, hold: 0, rng: seed | 1 };
        for i in 0..road.traffic {
            let z = 80.0 + (trip.rand() % (len - 80.0).max(1.0) as u32) as f64;
            let other = trip.other(z, i);
            trip.others.push(other);
        }
        for i in 0..road.cops {
            let spread = len / (road.cops + 1) as f64;
            let jitter = (trip.rand() % 100) as f64 / 100.0 - 0.5;
            trip.cops.push(Cop { z: (spread * (i as f64 + 1.0 + jitter * 0.5)).clamp(150.0, (len - 100.0).max(150.0)), done: false });
        }
        trip
    }

    fn rand(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    fn other(&mut self, z: f64, i: usize) -> Other {
        let k = self.rand();
        let oncoming = (k >> 3) & 1 == 0;
        Other { z, lane: if oncoming { -0.5 } else { 0.5 }, speed: if oncoming { -(18.0 + (k % 7) as f64) } else { 13.0 + (k % 8) as f64 }, paint: PAINTS[(k as usize >> 8).wrapping_add(i) % 8] }
    }

    pub fn len(&self) -> f64 {
        self.segs.len() as f64 * SEG
    }

    pub fn is_empty(&self) -> bool {
        self.segs.is_empty()
    }

    pub fn seg(&self, z: f64) -> &Seg {
        &self.segs[((z / SEG).max(0.0) as usize).min(self.segs.len() - 1)]
    }

    /// How high the road is at `z`.
    pub fn height(&self, z: f64) -> f64 {
        let i = ((z / SEG).max(0.0) as usize).min(self.segs.len() - 1);
        let next = self.segs.get(i + 1).map_or(self.segs[i].y, |s| s.y);
        self.segs[i].y + (next - self.segs[i].y) * ((z / SEG) - i as f64).clamp(0.0, 1.0)
    }

    pub fn tick(&mut self, road: &Road, p: Pedals) -> Happened {
        let mut out = Happened::default();
        if self.hold > 0 {
            self.hold -= 1;
            return out;
        }
        if let Some(mut r) = self.rival.take() {
            // They lift for a bend they can see, and no more than they have to.
            let bend = self.seg(r.z).curve.abs().max(self.seg(r.z + 40.0).curve.abs());
            let want = r.top * (1.0 - 0.42 * bend);
            r.speed = if r.speed < want { r.speed + r.accel / HZ * (1.0 - r.speed / r.top) } else { r.speed - 8.0 / HZ };
            r.z += r.speed / HZ;
            // Side by side they keep to the lane you are not in; clear of you, to the right one.
            let gap = r.z - self.z;
            let lane = if gap.abs() < 9.0 { if self.x > 0.0 { -0.5 } else { 0.5 } } else { 0.5 };
            r.x += (lane - r.x) * 0.06;
            out.beaten = r.z >= self.len() - 1.0 && self.z < self.len() - 1.0;
            self.rival = Some(r);
        }
        if self.crash > 0 {
            self.crash -= 1;
            return out;
        }
        if p.go {
            self.speed += self.pace.accel * (1.0 + self.spray) / HZ * (1.0 - self.speed / (self.pace.top * (1.0 + self.spray * 0.3)));
        } else {
            self.speed -= 1.0 / HZ;
        }
        if p.stop {
            self.speed -= 11.0 / HZ;
        }
        // Off the tar it is slow going.
        if self.x.abs() > 1.0 && self.speed > 15.0 {
            self.speed -= 9.0 / HZ;
        }
        self.speed = self.speed.max(0.0);
        let want = if p.left { -1.0 } else if p.right { 1.0 } else { 0.0 };
        self.steer += (want - self.steer) / 4.0;
        let seg = *self.seg(self.z);
        // The wheel moves the car across the road; a bend taken fast throws it the other way.
        self.x += self.steer * 0.055 * (self.speed / 15.0).min(1.0) - seg.curve * (self.speed / 56.0).powi(2) * 0.07 * self.pace.slide;
        let side = if self.x > 0.0 { seg.right } else { seg.left };
        let mut hit = false;
        if self.x.abs() > 1.35 && side.solid() {
            hit = true;
            self.x = self.x.signum() * 1.05;
        }
        self.x = self.x.clamp(-2.2, 2.2);
        let before = self.z;
        self.z += self.speed / HZ;
        out.went = self.speed / HZ;
        let len = self.len();
        for i in 0..self.others.len() {
            self.others[i].z += self.others[i].speed / HZ;
            let (gap, across) = (self.others[i].z - self.z, (self.others[i].lane - self.x).abs());
            if gap > -1.0 && gap < 4.5 && across < 0.38 {
                hit = true;
                // It carries on; you do not.
                self.others[i].z += if self.others[i].speed > 0.0 { 14.0 } else { -14.0 };
            }
            if self.others[i].z < self.z - 90.0 || self.others[i].z > self.z + 900.0 {
                let ahead = self.z + 320.0 + (self.rand() % 480) as f64;
                self.others[i] = self.other(ahead, i);
            }
        }
        if let Some(r) = self.rival.as_mut() {
            let gap = r.z - self.z;
            if gap > -1.0 && gap < 4.5 && (r.x - self.x).abs() < 0.38 {
                hit = true;
                r.z += 6.0;
            }
        }
        let over = self.speed * MPH - road.limit;
        let (mut radar, mut caught) = (false, false);
        for cop in self.cops.iter_mut().filter(|c| !c.done) {
            radar |= cop.z > self.z && cop.z - self.z < 220.0;
            if before < cop.z && self.z >= cop.z {
                cop.done = true;
                caught |= over > 10.0;
            }
        }
        self.radar = radar;
        if caught && self.chase.is_none() {
            self.chase = Some((0, over));
            self.fled = 0;
            out.siren = true;
        }
        if hit {
            let hard = self.speed > 3.0;
            if hard {
                self.scrapes += 1;
                self.crash = HZ as i32;
            }
            self.speed = 0.0;
            out.crashed = hard;
        }
        if let Some((ticks, worst)) = self.chase {
            let worst = worst.max(over);
            let fine = |times: i32| Some((road.ticket + (worst.max(0.0) as i32) * 2) * times);
            if hit {
                // Wrapped it up with the law behind you: that is the dear way to stop.
                out.ticket = fine(2);
                self.chase = None;
            } else if self.speed < 0.5 {
                out.ticket = fine(1);
                self.chase = None;
            } else {
                self.fled = if over > 25.0 { self.fled + 1 } else { self.fled.saturating_sub(1) };
                if self.fled > 20 * HZ as u32 {
                    out.lost_him = true;
                    self.chase = None;
                } else if ticks > 45 * HZ as u32 {
                    // He gets alongside in the end, and it costs twice as much.
                    out.ticket = fine(2);
                    self.speed = 0.0;
                    self.chase = None;
                } else {
                    self.chase = Some((ticks + 1, worst));
                }
            }
        }
        out.arrived = self.z >= len - 1.0;
        out
    }
}
