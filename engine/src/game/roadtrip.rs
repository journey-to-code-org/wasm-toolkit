//! Out of town: the road between two places, from the seat.
//!
//! The rules of the road are in `road.rs`; this is what the game makes of them, which is gas,
//! hours, dents, tickets, and the picture through the windscreen.

use super::*;
use crate::pack::Road;
use crate::road::{Side, Trip, HALF, MPH, SEG};

/// How far ahead is drawn, in pieces of road, and how much a full bend turns in one piece.
const AHEAD: usize = 56;
const BEND: f64 = 0.032;

impl Game {
    /// Setting out along a road, from a place or from the block of a map that leads onto it.
    pub(super) fn take_road(&mut self, id: &str, from_map: Option<&str>) {
        let Some(road) = self.pack.manifest.roads.get(id) else { return };
        if self.state.cur.is_none() {
            return self.no("You have nothing to drive.");
        }
        if let Some(why) = self.wont_run() {
            return self.no(&why);
        }
        if self.owned().is_some_and(|o| o.gas < 0.3) {
            return self.no("Not enough gas for the road.");
        }
        let here = from_map.map_or(self.state.place.clone(), |m| format!("map:{m}"));
        let backwards = road.ends[1] == here && road.ends[0] != here;
        let seed = self.rand();
        let Some(road) = self.pack.manifest.roads.get(id) else { return };
        self.trip = Some(Trip::new(id, road, backwards, seed));
        self.trip_from = from_map.map(str::to_string);
        self.town = None;
        self.note = None;
        self.scene = Scene::Road;
        tone(520, 60);
    }

    /// Onto the streets of a map at the block that leads to a road.
    pub(super) fn town_at_road(&mut self, map: &str, road: &str) {
        let Some(m) = self.pack.manifest.maps.get(map) else { return };
        let mut rng = self.rng;
        let town = Town::at(map, m, |b| b.road.as_deref() == Some(road), || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            rng
        });
        self.rng = rng;
        self.town = Some(town);
        self.scene = Scene::Drive;
    }

    pub(super) fn road_tick(&mut self, keys: u32, hit: u32) {
        let Some(trip) = self.trip.as_mut() else { return };
        let Some(road) = self.pack.manifest.roads.get(&trip.road) else { return };
        if hit & KEY_B != 0 {
            if trip.chase.is_some() {
                self.note = Some("Not with him behind you.".into());
                return;
            }
            // Turned round: back where you set out from, and nothing lost but the gas.
            tone(0, 0);
            let (id, from) = (trip.road.clone(), self.trip_from.take());
            self.trip = None;
            self.note = None;
            return match from {
                Some(map) => self.town_at_road(&map, &id),
                None => self.scene = Scene::Place,
            };
        }
        let h = trip.tick(road, Pedals { go: keys & KEY_UP != 0, stop: keys & KEY_DOWN != 0, left: keys & KEY_LEFT != 0, right: keys & KEY_RIGHT != 0 });
        let (speed, chased, stopped, warned) = (trip.speed, trip.chase.is_some(), trip.crash > 0, trip.radar);
        let (per_km, hours, end, id) = (road.gas_per_km, road.hours, road.ends[if trip.backwards { 0 } else { 1 }].clone(), trip.road.clone());
        if h.crashed {
            tone(70, 500);
        } else if chased {
            tone(if self.ticks / 9 % 2 == 0 { 760 } else { 570 }, 60);
        } else if warned && self.radar() && self.ticks % 12 == 0 {
            tone(1500, 50);
        } else if !stopped {
            let v = (speed * 10.0) as i32;
            tone(if speed > 0.0 || keys & KEY_UP != 0 { 60 + v % 110 + v / 110 * 12 } else { 45 }, 60);
        }
        self.burn(h.went / 1000.0 * per_km);
        if h.crashed {
            // Bent metal is a tired car.
            if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
                o.cond = (o.cond - 3).max(25);
            }
        }
        if h.siren {
            self.say("Police! Pull over, or run for it.");
        }
        if h.lost_him {
            self.say("You lost him.");
        }
        if let Some(fine) = h.ticket {
            // He takes what you have, if you do not have it all.
            let paid = fine.min(self.state.cash.max(0));
            self.state.cash -= paid;
            self.say(&format!("Speeding. That is a ${paid} ticket."));
            tone(160, 400);
            self.save();
        }
        if h.arrived {
            tone(520, 60);
            self.trip = None;
            self.trip_from = None;
            match end.strip_prefix("map:") {
                Some(map) => {
                    self.town_at_road(map, &id);
                    self.town_by_road = true;
                }
                None => self.enter(&end),
            }
            return self.spend(hours);
        }
        if self.owned().is_some_and(|o| o.gas <= 0.0) {
            tone(0, 0);
            self.trip = None;
            self.trip_from = None;
            let home = self.pack.manifest.start.place.clone();
            self.enter(&home);
            self.say("Out of gas. You walked home.");
            self.spend(2);
        }
    }

    /// A radar detector, if there is one in the car.
    pub fn radar(&self) -> bool {
        self.owned().is_some_and(|o| o.fitted.values().any(|id| self.pack.manifest.parts.get(id).is_some_and(|p| p.radar)))
    }

    /// A box standing on the road, pointing the way `f` points: only the sides that can be seen.
    fn obox(scene: &mut Scene3, at: V3, f: (f64, f64), hl: f64, hw: f64, tall: f64, ramp: usize, key: f64) {
        let r = (f.1, -f.0);
        let c = |a: f64, b: f64, y: f64| [at[0] + f.0 * a * hl + r.0 * b * hw, at[1] + y, at[2] + f.1 * a * hl + r.1 * b * hw];
        let eye = scene.cam.pos;
        for (n, a, b) in [((f.0, f.1), (1.0, -1.0), (1.0, 1.0)), ((-f.0, -f.1), (-1.0, 1.0), (-1.0, -1.0)), (r, (1.0, 1.0), (-1.0, 1.0)), ((-r.0, -r.1), (-1.0, -1.0), (1.0, -1.0))] {
            let p = c(a.0, a.1, 0.0);
            if n.0 * (eye[0] - p[0]) + n.1 * (eye[2] - p[2]) > 0.0 {
                scene.lit_key(&[p, c(b.0, b.1, 0.0), c(b.0, b.1, tall), c(a.0, a.1, tall)], ramp, key);
            }
        }
        if eye[1] > at[1] + tall {
            scene.lit_key(&[c(-1.0, -1.0, tall), c(1.0, -1.0, tall), c(1.0, 1.0, tall), c(-1.0, 1.0, tall)], ramp, key);
        }
    }

    pub(super) fn draw_road(&self, s: &mut Screen) {
        let Some(trip) = &self.trip else { return };
        let Some(road) = self.pack.manifest.roads.get(&trip.road) else { return };
        let seat = self.car().and_then(|c| c.cockpit.as_ref());
        let view = seat.map_or(200, |c| c.view_h);
        let horizon = view * 60 / 136;
        let (sky, land) = (ramp(road.sky.as_deref().unwrap_or("SKY")), ramp(road.ground.as_deref().unwrap_or("GRASS")));
        s.vgrad(0, 0, 320, horizon, sky, 15, 29);
        s.vgrad(0, horizon, 320, view - horizon, land, 12, 17);
        s.clip(0, 0, 320, view);

        // The road ahead, laid out from where the car is: each piece turns a little from the last.
        let i0 = ((trip.z / SEG) as usize).min(trip.segs.len() - 1);
        let n = AHEAD.min(trip.segs.len() - i0);
        let (mut px, mut pz, mut h) = (0.0f64, 0.0f64, 0.0f64);
        // Each joint: where the middle of the road is, which way it points, and how high.
        let mut joints: Vec<(f64, f64, f64, f64)> = Vec::with_capacity(n + 1);
        for k in 0..=n {
            let seg = trip.segs[(i0 + k).min(trip.segs.len() - 1)];
            joints.push((px, pz, h, seg.y));
            px += h.sin() * SEG;
            pz += h.cos() * SEG;
            h += seg.curve * BEND;
        }
        let f = (trip.z / SEG - i0 as f64).clamp(0.0, 1.0);
        let (h0, here) = (joints[0].2 + trip.segs[i0].curve * BEND * f, trip.height(trip.z));
        let slope = (trip.height(trip.z + 14.0) - here) / 14.0;
        let eye = [f * SEG * h0.sin() + h0.cos() * trip.x * HALF, here + 1.125, f * SEG * h0.cos() - h0.sin() * trip.x * HALF];
        let cam = Cam { pos: eye, yaw: h0 + trip.steer * 0.05, pitch: -slope.atan(), focal: 200.0, cx: 160, cy: horizon };
        let mut scene = Scene3::new(cam);
        // A point so many half-widths out from the middle of the road at a joint.
        let at = |k: usize, out: f64, up: f64| -> V3 {
            let (x, z, h, y) = joints[k];
            [x + h.cos() * out * HALF, y + up, z - h.sin() * out * HALF]
        };
        let (tar, line, edge) = (ramp("ASPHALT"), shade(ramp("YELLOW"), 23), shade(ramp("WHITE"), 24));
        for k in (0..n).rev() {
            let i = i0 + k;
            let seg = trip.segs[i];
            let key = k as f64 * 10.0;
            let band = if (i / 2) % 2 == 0 { 0 } else { 1 };
            for (sign, side) in [(-1.0, seg.left), (1.0, seg.right)] {
                // What the road runs through: nothing at all, where the ground falls away.
                let ground = match side {
                    Side::Drop => None,
                    Side::Fields => Some(ramp("TAN")),
                    Side::Rock => Some(ramp("CONCRETE")),
                    _ => Some(land),
                };
                if let Some(g) = ground {
                    let c = shade(g, 15 + band);
                    scene.poly_key(&[at(k, sign, 0.0), at(k + 1, sign, 0.0), at(k + 1, sign * 16.0, 0.0), at(k, sign * 16.0, 0.0)], c, c, key + 9.0);
                }
                match side {
                    Side::Rock => {
                        // A cutting: the hill it was blasted through, close and tall.
                        scene.lit_key(&[at(k, sign * 1.5, 0.0), at(k + 1, sign * 1.5, 0.0), at(k + 1, sign * 2.2, 9.0 + (i % 3) as f64), at(k, sign * 2.2, 9.0 + ((i + 2) % 3) as f64)], ramp(if i % 2 == 0 { "CONCRETE" } else { "GREY" }), key + 4.0);
                    }
                    Side::Drop => {
                        // A rail, on posts, and the valley past it.
                        let c = shade(ramp("CHROME"), 22 + band * 3);
                        scene.poly_key(&[at(k, sign * 1.2, 0.45), at(k + 1, sign * 1.2, 0.45), at(k + 1, sign * 1.2, 0.8), at(k, sign * 1.2, 0.8)], c, c, key + 4.0);
                        if i % 2 == 0 {
                            let c = shade(ramp("GREY"), 14);
                            scene.poly_key(&[at(k, sign * 1.2, 0.0), at(k, sign * 1.23, 0.0), at(k, sign * 1.23, 0.8), at(k, sign * 1.2, 0.8)], c, c, key + 4.5);
                        }
                    }
                    Side::Posts | Side::Fields => {
                        let every = if side == Side::Posts { 2 } else { 3 };
                        if i % every == 0 {
                            let (out, tall, col) = if side == Side::Posts { (1.12, 0.9, "WHITE") } else { (1.5, 1.2, "WOOD") };
                            Self::obox(&mut scene, at(k, sign * out, 0.0), (joints[k].2.sin(), joints[k].2.cos()), 0.08, 0.08, tall, ramp(col), key + 4.0);
                        }
                    }
                    Side::Trees => {
                        if i % 2 == 0 {
                            // The same tree every time you pass it: how far back and how tall come from where it is.
                            let r = (i as u32).wrapping_mul(2654435761) >> 8;
                            let (out, tall, wide) = (1.6 + (r % 13) as f64 * 0.12, 5.0 + (r % 5) as f64, 1.6 + (r % 3) as f64 * 0.4);
                            let base = at(k, sign * out, 0.0);
                            Self::obox(&mut scene, base, (joints[k].2.sin(), joints[k].2.cos()), 0.2, 0.2, 1.6, ramp("WOOD"), key + 4.0);
                            let (top, y) = ([base[0], base[1] + tall, base[2]], base[1] + 1.2);
                            let green = ramp(if r % 4 == 0 { "GRASS" } else { "FOREST" });
                            let corner = |a: f64, b: f64| [base[0] + a * wide, y, base[2] + b * wide];
                            for (a, b) in [((-1.0, -1.0), (1.0, -1.0)), ((1.0, -1.0), (1.0, 1.0)), ((1.0, 1.0), (-1.0, 1.0)), ((-1.0, 1.0), (-1.0, -1.0))] {
                                // Only the two faces of the cone that look this way.
                                let mid = [(a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5];
                                if mid[0] * (eye[0] - base[0]) + mid[1] * (eye[2] - base[2]) > 0.0 {
                                    scene.lit_key(&[corner(a.0, a.1), corner(b.0, b.1), top], green, key + 3.5);
                                }
                            }
                        }
                    }
                }
            }
            let c = shade(tar, 15 + band);
            scene.poly_key(&[at(k, -1.0, 0.0), at(k + 1, -1.0, 0.0), at(k + 1, 1.0, 0.0), at(k, 1.0, 0.0)], c, c, key + 8.0);
            for sign in [-1.0, 1.0] {
                scene.poly_key(&[at(k, sign * 0.9, 0.0), at(k + 1, sign * 0.9, 0.0), at(k + 1, sign * 0.94, 0.0), at(k, sign * 0.94, 0.0)], edge, edge, key + 7.0);
            }
            if i % 4 < 2 {
                scene.poly_key(&[at(k, -0.02, 0.0), at(k + 1, -0.02, 0.0), at(k + 1, 0.02, 0.0), at(k, 0.02, 0.0)], line, line, key + 7.0);
            }
            if i + 1 == trip.segs.len() {
                // The end of the road: a sign across it.
                for sign in [-1.0, 1.0] {
                    Self::obox(&mut scene, at(k + 1, sign * 1.15, 0.0), (joints[k + 1].2.sin(), joints[k + 1].2.cos()), 0.15, 0.15, 5.5, ramp("CHROME"), key + 4.0);
                }
                let c = shade(ramp("GOLD"), 24);
                scene.poly_key(&[at(k + 1, -1.15, 4.2), at(k + 1, 1.15, 4.2), at(k + 1, 1.15, 5.5), at(k + 1, -1.15, 5.5)], c, shade(ramp("GOLD"), 22), key + 3.9);
            }
        }
        // Whatever is standing or moving on a piece of road is drawn after that piece.
        let mut on_road = |z: f64, out: f64, back: bool, paint: usize, kind: u8| {
            if z <= trip.z + 1.0 || z >= trip.z + (n as f64 - 1.0) * SEG {
                return;
            }
            let k = ((z / SEG) as usize).saturating_sub(i0).min(n - 1);
            let t = (z / SEG - (i0 + k) as f64).clamp(0.0, 1.0);
            let (a, b) = (at(k, out, 0.0), at(k + 1, out, 0.0));
            let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
            let hd = joints[k].2 + if back { std::f64::consts::PI } else { 0.0 };
            let (fwd, key) = ((hd.sin(), hd.cos()), k as f64 * 10.0 + 3.0 - t);
            Self::obox(&mut scene, [p[0], p[1] + 0.25, p[2]], fwd, 2.1, 0.9, 0.65, paint, key);
            Self::obox(&mut scene, [p[0], p[1] + 0.9, p[2]], fwd, 1.0, 0.75, 0.5, ramp(if kind == 1 { "WHITE" } else { "GLASS" }), key - 0.3);
            if kind == 1 {
                // The lights on its roof, which are lit only if it has seen you.
                Self::obox(&mut scene, [p[0], p[1] + 1.4, p[2]], fwd, 0.15, 0.5, 0.2, ramp(if self.ticks / 6 % 2 == 0 { "RED" } else { "BLUE" }), key - 0.6);
            }
        };
        for o in &trip.others {
            on_road(o.z, o.lane, o.speed < 0.0, ramp(o.paint), 0);
        }
        for c in &trip.cops {
            on_road(c.z, 1.3, false, ramp("RUBBER"), 1);
        }
        scene.end(s);
        s.clip_off();

        let v = (trip.speed * 10.0) as i32;
        let mph = trip.speed * MPH;
        if let Some(seat) = seat {
            let rpm = if trip.speed > 0.0 { 1500.0 + (v % 110) as f64 * 50.0 } else { 900.0 };
            self.draw_cockpit(s, seat, false, rpm, mph, if trip.speed > 8.0 { 6.0 } else { 0.0 }, if trip.speed > 0.0 { (v / 110 + 1) as usize } else { 0 }, trip.steer * 90.0, false);
        } else {
            self.panel(s, &road.title, &format!("{} mph", mph as i32), "Arrows drive. Escape turns back.");
        }
        let (white, gold, red) = (shade(ramp("WHITE"), 26), shade(ramp("GOLD"), 28), shade(ramp("RED"), 27));
        s.text_sh(44, 18, &road.title.to_uppercase().chars().take(20).collect::<String>(), white, BLACK);
        let left = (trip.len() - trip.z).max(0.0);
        s.text_sh(44, 27, &if left >= 1000.0 { format!("{:.1} KM TO GO", left / 1000.0) } else { format!("{} M TO GO", left as i32 / 10 * 10) }, white, BLACK);
        s.text_sh(44, 36, &format!("LIMIT {}", road.limit as i32), if mph > road.limit + 10.0 { red } else { white }, BLACK);
        let gas = self.owned().map_or(0.0, |o| o.gas);
        s.text_sh(216, 18, &format!("GAS {gas:.1}"), if gas < 1.0 { red } else { white }, BLACK);
        let blink = self.ticks / 8 % 2 == 0;
        if trip.chase.is_some() {
            // In the mirror: red and blue.
            s.rect(0, 0, 320, 4, shade(ramp(if blink { "RED" } else { "BLUE" }), 26));
            s.text_sh(132, 8, "POLICE!", if blink { red } else { white }, BLACK);
        } else if trip.radar && self.radar() && blink {
            s.text_sh(216, 27, "RADAR!", red, BLACK);
        }
        if let Some(note) = &self.note {
            s.text_sh(160 - note.len() as i32 * 4, view * 92 / 136, &note.to_uppercase(), gold, BLACK);
        }
        if trip.crash > 0 {
            s.vgrad(108, 50, 104, 22, ramp("RED"), 26, 16);
            s.frame(108, 50, 104, 22, WHITE);
            s.text_sh(136, 57, "CRASH!", WHITE, BLACK);
        }
    }

    /// What a road is, for the tests: the road a trip is on.
    pub fn on_road(&self) -> Option<(&Trip, &Road)> {
        let trip = self.trip.as_ref()?;
        Some((trip, self.pack.manifest.roads.get(&trip.road)?))
    }
}
