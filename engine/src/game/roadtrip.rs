//! Out of town: the road between two places, from the seat.
//!
//! The rules of the road are in `road.rs`; this is what the game makes of them, which is gas,
//! hours, dents, tickets, and the picture through the windscreen.

use super::*;
use crate::pack::Road;
use crate::road::{Pace, Side, Trip, HALF, MPH, SEG};

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
        let here = from_map.map_or(self.state.place.clone(), |m| format!("map:{m}"));
        let backwards = road.ends[1] == here && road.ends[0] != here;
        if !backwards && self.state.rung < road.rung {
            let more = road.rung - self.state.rung;
            return self.no(&format!("Not yet. Beat {more} more here first."));
        }
        let seed = self.rand();
        let Some(road) = self.pack.manifest.roads.get(id) else { return };
        let mut trip = Trip::new(id, road, backwards, seed);
        trip.pace = self.pace();
        trip.gearbox = self.gearbox();
        self.trip = Some(trip);
        self.trip_from = from_map.map(str::to_string);
        self.town = None;
        self.note = None;
        self.scene = Scene::Road;
        tone(520, 60);
    }

    /// What the car in the bay can do on a road: what it makes for what it weighs, and its tyres.
    pub fn pace(&self) -> Pace {
        let Some((o, car)) = self.owned().zip(self.car()) else { return Pace::default() };
        let (d, (gain, _)) = (self.tuned_drive(o, car), self.tune(o));
        let blown = self.blown(o, car);
        let hp = d.torque.iter().filter(|p| p[0] <= d.redline).map(|p| p[0] * p[1]).fold(0.0, f64::max) / 5252.0 * gain * blown;
        let wet = if self.raining() { 1.35 } else { 1.0 };
        // Brakes and suspension: a wet road stops slower and slides further.
        let (brakes, handling) = self.chassis(o);
        let slide = (12.0 / d.grip.max(1.0)).clamp(0.6, 1.6) * wet / handling.max(0.3);
        // A front-driven car on the gas on a road wanders across it; one driven from all four less.
        let pull = d.driven.torque_steer() / 3.2 * 0.3 * self.lean() * wet;
        Pace { accel: (5.0 * (hp / d.weight_lb / 0.0975).powf(0.7)).clamp(2.0, 9.0), top: (56.0 * (hp / 298.0).cbrt()).clamp(28.0, 76.0) * (1.0 - o.body.clamp(0, 100) as f64 * 0.0015), slide, brakes: brakes / wet.sqrt(), pull }
    }

    /// The gearbox of the car in the bay, for a road, if it is one worked by hand.
    pub fn gearbox(&self) -> Option<crate::road::Gearbox> {
        let (o, car) = self.owned().zip(self.car())?;
        let d = self.tuned_drive(o, car);
        if d.auto || d.gears.is_empty() {
            return None;
        }
        let peak = d.torque.iter().filter(|p| p[0] <= d.redline).fold((0.0, 0.0), |m, p| if p[0] * p[1] > m.1 { (p[0], p[0] * p[1]) } else { m }).0;
        Some(crate::road::Gearbox { ratios: d.gears.iter().map(|g| g * d.final_drive).collect(), redline: d.redline, peak: peak.max(1000.0), gear: 0, rpm: 900.0 })
    }

    /// Down a road against whoever has pulled in, for what was bet: no police, and first to the end.
    pub(super) fn start_road_race(&mut self, id: &str) {
        let Some(r) = self.rival().cloned() else { return };
        let Some(road) = self.pack.manifest.roads.get(id) else { return };
        let mut road = road.clone();
        road.cops = 0;
        let backwards = road.ends[1] == self.state.place && road.ends[0] != self.state.place;
        let seed = self.rand();
        let mut trip = Trip::new(id, &road, backwards, seed);
        trip.pace = self.pace();
        trip.gearbox = self.gearbox();
        let mut them = crate::road::Rival::new(r.et, r.paint.as_deref().unwrap_or("RED"));
        if self.raining() {
            // They are no braver in the wet than you can be.
            them.top *= 0.93;
        }
        trip.rival = Some(them);
        trip.hold = 90;
        self.trip = Some(trip);
        self.trip_from = None;
        self.note = None;
        self.scene = Scene::Road;
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
        let mut town = town;
        if let Some((x, z, heading)) = m.gate(road) {
            // In off the road without stopping: on the street it joins, still rolling.
            (town.x, town.z, town.heading, town.speed) = (x, z, heading, self.rolling.min(18.0));
        }
        self.rolling = 0.0;
        self.town = Some(town);
        self.fit_town();
        self.scene = Scene::Drive;
    }

    /// A poker run: the road and back to where you started, a card at each of five stops along
    /// it, and the best hand among everyone riding takes what they all paid in.
    pub(super) fn start_poker(&mut self, id: &str) {
        let Some(road) = self.pack.manifest.roads.get(id) else { return };
        let entry = self.pack.manifest.poker.entry;
        if self.state.cur.is_none() {
            return self.no("You have nothing to ride in.");
        }
        if let Some(why) = self.wont_run() {
            return self.no(&why);
        }
        if self.state.cash < entry {
            return self.no(&format!("It is ${entry} to ride. You don't have it."));
        }
        let seed = self.rng | 1;
        let mut trip = Trip::new(id, road, false, seed);
        trip.pace = self.pace();
        trip.gearbox = self.gearbox();
        trip.poker = true;
        self.rand();
        self.state.cash -= entry;
        self.trip = Some(trip);
        self.trip_from = None;
        self.scene = Scene::Road;
        self.say("A card at every stop. Mind the limit.");
    }

    /// The run is ridden: your hand against everyone else's.
    fn settle_poker(&mut self, cards: &[u8]) {
        let (poker, day) = (self.pack.manifest.poker.clone(), self.day() as u32);
        let mine = cards::rank(cards);
        // The others' hands come off what is left of the same deck.
        let mut deck: Vec<u8> = (0..52).filter(|c| !cards.contains(c)).collect();
        let mut best: Option<(usize, Vec<u8>)> = None;
        for rider in 0..poker.riders.min(9) {
            let hand: Vec<u8> = (0..5).map(|k| deck.remove(hash(day * 977 + rider as u32 * 31 + k + self.rng) as usize % deck.len())).collect();
            if best.as_ref().is_none_or(|b| cards::rank(&hand) > cards::rank(&b.1)) {
                best = Some((rider, hand));
            }
        }
        let pot = poker.entry * (poker.riders as i32 + 1);
        let hand = cards::name(&mine);
        match best.filter(|b| cards::rank(&b.1) > mine) {
            None => {
                self.state.cash += pot;
                tone(990, 400);
                self.say(&format!("{hand} takes the ${pot} pot."));
            }
            Some((rider, theirs)) => {
                let rivals = &self.pack.manifest.rivals;
                let who = if rivals.is_empty() { "Someone".to_string() } else { rivals[(rider * 3 + day as usize) % rivals.len()].name.clone() };
                self.say(&format!("{hand}. {who}: {}.", cards::name(&cards::rank(&theirs)).to_lowercase()).chars().take(38).collect::<String>());
            }
        }
        self.spend(2);
    }

    pub(super) fn road_tick(&mut self, keys: u32, hit: u32) {
        let Some(trip) = self.trip.as_mut() else { return };
        let Some(road) = self.pack.manifest.roads.get(&trip.road) else { return };
        if hit & KEY_B != 0 && trip.poker {
            // Out of the run: what you paid in stays in the pot.
            tone(0, 0);
            self.trip = None;
            self.scene = Scene::Place;
            return self.say("You dropped out of the run.");
        }
        if hit & KEY_B != 0 {
            if trip.rival.is_some() {
                // Lifting is losing.
                tone(0, 0);
                self.trip = None;
                return self.settle(Some(false));
            }
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
        // The bottle, if there is one with something in it and a foot on the gas.
        let spray = self.state.cur.and_then(|i| self.state.cars.get(i)).filter(|o| keys & KEY_N != 0 && keys & KEY_UP != 0 && o.bottle > 0.0 && trip.hold == 0 && trip.crash == 0)
            .and_then(|o| o.fitted.values().filter_map(|id| self.pack.manifest.parts.get(base(id))).find_map(|p| p.nitrous.as_ref())).map_or(0.0, |k| k.gain as f64 / 100.0);
        trip.spray = spray;
        if spray > 0.0 {
            if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
                o.bottle = (o.bottle - 1.0 / 30.0).max(0.0);
            }
        }
        let h = trip.tick(road, Pedals { go: keys & KEY_UP != 0, stop: keys & KEY_DOWN != 0, left: keys & KEY_LEFT != 0, right: keys & KEY_RIGHT != 0, up: hit & KEY_X != 0, down: hit & KEY_Y != 0 });
        let (speed, chased, stopped, warned) = (trip.speed, trip.chase.is_some(), trip.crash > 0, trip.radar);
        let (racing, counting) = (trip.rival.is_some(), trip.hold);
        let revs = trip.gearbox.as_ref().map(|b| b.rpm);
        // A poker run: a card at each sixth of the way, off a deck dealt for the day.
        let mut dealt = None;
        if trip.poker && trip.cards.len() < 5 && trip.z >= trip.len() * (trip.cards.len() + 1) as f64 / 6.0 {
            let mut k = self.rng ^ (trip.cards.len() as u32 + 1).wrapping_mul(2654435761);
            let card = loop {
                k = hash(k);
                if !trip.cards.contains(&((k % 52) as u8)) {
                    break (k % 52) as u8;
                }
            };
            trip.cards.push(card);
            dealt = Some((trip.cards.len(), card));
        }
        let poker = (h.arrived && trip.poker).then(|| trip.cards.clone());
        let (per_km, hours, end, id) = (road.gas_per_km, road.hours, road.ends[if trip.backwards { 0 } else { 1 }].clone(), trip.road.clone());
        if revs.is_none() {
            self.shift_auto(speed, keys & KEY_UP != 0);
        }
        if h.crashed {
            tone(70, 500);
        } else if counting > 0 {
            // Three, two, one, and a higher one for go.
            if counting % 30 == 0 {
                tone(440, 120);
            } else if counting == 1 {
                tone(880, 250);
            }
        } else if chased {
            self.hum(if self.ticks / 9 % 2 == 0 { 760 } else { 570 });
        } else if warned && self.radar() && self.ticks % 12 == 0 {
            tone(1500, 50);
        } else if !stopped {
            self.hum((revs.unwrap_or(self.auto.1) / 40.0) as i32);
        }
        self.burn(h.went / 1000.0 * per_km);
        self.scrub(h.went / 1000.0 * 0.4);
        self.roll(h.went / 1000.0);
        if h.crashed {
            // Bent metal is a tired car.
            if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
                o.cond = (o.cond - 3).max(25);
            }
            self.dent();
        }
        if let Some((n, card)) = dealt {
            tone(880, 90);
            self.say(&format!("Card {n}: the {}.", cards::card(card)));
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
        if racing && (h.arrived || h.beaten) {
            tone(if h.arrived { 990 } else { 160 }, 300);
            self.trip = None;
            return self.settle(Some(h.arrived));
        }
        if let Some(hand) = poker {
            tone(0, 0);
            self.trip = None;
            self.scene = Scene::Place;
            return self.settle_poker(&hand);
        }
        if h.arrived {
            tone(520, 60);
            self.rolling = speed;
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
            if racing {
                // Dry with the race on is the race lost, and then the walk.
                self.settle(Some(false));
                if self.owned().is_none() || matches!(self.scene, Scene::Over(_)) {
                    return;
                }
            }
            let home = self.pack.manifest.start.place.clone();
            self.enter(&home);
            self.say("Out of gas. You walked home.");
            self.spend(2);
        }
    }

    /// A radar detector, if there is one in the car.
    pub fn radar(&self) -> bool {
        self.owned().is_some_and(|o| o.fitted.values().any(|id| self.pack.manifest.parts.get(base(id)).is_some_and(|p| p.radar)))
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
        let dim = self.sky(s, horizon, view, sky, land);
        s.clip(0, 0, 320, view);
        let seg0 = trip.segs[((trip.z / SEG) as usize).min(trip.segs.len() - 1)];
        // The mountains turn with the road: a bend swings them across the glass.
        self.draw_ridge(s, trip.z / 900.0 + seg0.curve * 0.4 + trip.steer * 0.05, horizon);

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
        (scene.dim, scene.lamps) = (dim, 60.0);
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
                    Side::Drop | Side::Water => None,
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
                    Side::Water => {
                        // A shore, and then the lake as far as can be seen, lower than the road and never quite still.
                        let sand = shade(ramp("TAN"), 17 + band);
                        scene.poly_key(&[at(k, sign, 0.0), at(k + 1, sign, 0.0), at(k + 1, sign * 1.5, -0.6), at(k, sign * 1.5, -0.6)], sand, sand, key + 9.0);
                        let wave = ((i as u32 + self.ticks / 6) % 3) as i32;
                        let blue = shade(ramp("BLUE"), (15 + wave * 2 - dim.min(9)).max(3));
                        scene.poly_key(&[at(k, sign * 1.5, -0.6), at(k + 1, sign * 1.5, -0.6), at(k + 1, sign * 40.0, -0.6), at(k, sign * 40.0, -0.6)], blue, shade(ramp("BLUE"), (17 + wave * 2 - dim.min(9)).max(3)), key + 9.5);
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
        // Cars that have pictures of their backs and fronts in the pack are drawn with them.
        let cars = &self.pack.manifest.cars;
        let seen = |car: &Car, back: bool| (if back { car.front.as_ref() } else { car.rear.as_ref() }).and_then(|r| Some((self.pack.pictures.get(&r.picture)?, r.width_m)));
        let (backs, fronts): (Vec<&Car>, Vec<&Car>) = (cars.values().filter(|c| seen(c, false).is_some()).collect(), cars.values().filter(|c| seen(c, true).is_some()).collect());
        let mut looks: Vec<(&crate::gfx::Picture, Tint)> = Vec::new();
        let mut on_road = |z: f64, out: f64, back: bool, paint: &str, kind: u8, car: Option<&Car>| {
            if z <= trip.z + 1.0 || z >= trip.z + (n as f64 - 1.0) * SEG {
                return;
            }
            let k = ((z / SEG) as usize).saturating_sub(i0).min(n - 1);
            let t = (z / SEG - (i0 + k) as f64).clamp(0.0, 1.0);
            let (a, b) = (at(k, out, 0.0), at(k + 1, out, 0.0));
            let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
            let hd = joints[k].2 + if back { std::f64::consts::PI } else { 0.0 };
            let (fwd, key) = ((hd.sin(), hd.cos()), k as f64 * 10.0 + 3.0 - t);
            // Whichever car this is: the one it is said to be, or one of the pack's, the same one each time for its paint.
            let pool = if back { &fronts } else { &backs };
            let which = car.or_else(|| (kind == 0 && !pool.is_empty()).then(|| pool[paint.len() * 7 % pool.len()]));
            if let Some((c, (pic, wide))) = which.and_then(|c| Some((c, seen(c, back)?))) {
                looks.push((pic, Self::tint(c, Some(paint), if dim > 4 { -8 } else if dim > 0 { -3 } else { 0 })));
                if scene.sprite(p, wide, pic.h as f64 / pic.w as f64, looks.len() - 1, key) {
                    return;
                }
            }
            let paint = ramp(paint);
            if kind == 2 {
                // A truck, and behind it a trailer with someone's car on it.
                let lead = if back { -1.0 } else { 1.0 };
                let off = |d: f64, up: f64| [p[0] - hd.sin() * d * lead * if back { -1.0 } else { 1.0 }, p[1] + up, p[2] - hd.cos() * d * lead * if back { -1.0 } else { 1.0 }];
                Self::obox(&mut scene, off(7.2, 0.35), fwd, 3.3, 1.0, 0.45, ramp("GREY"), key + 0.4);
                Self::obox(&mut scene, off(7.2, 0.8), fwd, 2.1, 0.85, 0.6, paint, key + 0.3);
                Self::obox(&mut scene, off(7.2, 1.4), fwd, 1.0, 0.7, 0.45, ramp("GLASS"), key + 0.2);
                Self::obox(&mut scene, off(0.0, 0.3), fwd, 2.5, 1.0, 1.0, ramp("WHITE"), key);
                Self::obox(&mut scene, off(-0.4, 1.3), fwd, 1.2, 0.95, 0.75, ramp("GLASS"), key - 0.3);
                return;
            }
            Self::obox(&mut scene, [p[0], p[1] + 0.25, p[2]], fwd, 2.1, 0.9, 0.65, paint, key);
            Self::obox(&mut scene, [p[0], p[1] + 0.9, p[2]], fwd, 1.0, 0.75, 0.5, ramp(if kind == 1 { "WHITE" } else { "GLASS" }), key - 0.3);
            if dim > 4 {
                // After dark a car is its lamps: white coming, red going.
                let was = std::mem::replace(&mut scene.dim, 0);
                let (nose, lamp) = (2.15, ramp(if back { "CREAM" } else { "RED" }));
                let hd0 = joints[k].2;
                for side in [-0.6, 0.6] {
                    let q = [p[0] + hd0.sin() * -nose + hd0.cos() * side, p[1] + 0.55, p[2] + hd0.cos() * -nose - hd0.sin() * side];
                    Self::obox(&mut scene, q, fwd, 0.04, 0.2, 0.22, lamp, key - 0.5);
                }
                scene.dim = was;
            }
            if kind == 1 {
                // The lights on its roof, which are lit only if it has seen you.
                Self::obox(&mut scene, [p[0], p[1] + 1.4, p[2]], fwd, 0.15, 0.5, 0.2, ramp(if self.ticks / 6 % 2 == 0 { "RED" } else { "BLUE" }), key - 0.6);
            }
        };
        for o in &trip.others {
            on_road(o.z, o.lane, o.speed < 0.0, o.paint, if o.rig { 2 } else { 0 }, None);
        }
        if let Some(r) = &trip.rival {
            on_road(r.z, r.x, false, &r.paint, 0, self.opponent().and_then(|x| cars.get(&x.0)));
        }
        for c in &trip.cops {
            on_road(c.z, 1.3, false, "RUBBER", 1, None);
        }
        scene.end_with(s, |s, id, (x, y, w, h)| s.picture_sized(looks[id].0, x, y, w, h, Some(&looks[id].1)));
        self.draw_rain(s, 320, view);
        s.clip_off();

        let mph = trip.speed * MPH;
        if let Some(seat) = seat {
            // A gearbox shows the gear it is in and the revs it is at; an automatic, something like them.
            let (rpm, gear, light) = match &trip.gearbox {
                Some(b) => (b.rpm, b.gear + 1, b.rpm > b.redline - 700.0),
                None => (self.auto.1, if trip.speed > 0.0 { self.auto.0 + 1 } else { 0 }, false),
            };
            self.draw_cockpit(s, seat, self.dark() == 2, rpm, mph, if trip.speed > 8.0 { 6.0 } else { 0.0 }, gear, trip.steer * 90.0, light);
        } else {
            self.panel(s, &road.title, &format!("{} mph", mph as i32), "Arrows drive. Escape turns back.");
        }
        let (white, gold, red) = (shade(ramp("WHITE"), 26), shade(ramp("GOLD"), 28), shade(ramp("RED"), 27));
        s.text_sh(44, 18, &road.title.to_uppercase().chars().take(20).collect::<String>(), white, BLACK);
        let left = (trip.len() - trip.z).max(0.0);
        s.text_sh(44, 27, &if left >= 1000.0 { format!("{:.1} KM TO GO", left / 1000.0) } else { format!("{} M TO GO", left as i32 / 10 * 10) }, white, BLACK);
        if let Some(r) = &trip.rival {
            // No law out tonight: only who is in front, and by how much.
            let (gap, name) = ((r.z - trip.z) as i32, self.opponent().map_or("THEM".to_string(), |x| x.2.to_uppercase().chars().take(12).collect()));
            let words = if gap > 2 { format!("{name} +{gap} M") } else if gap < -2 { format!("{name} {gap} M") } else { format!("{name} LEVEL") };
            s.text_sh(44, 36, &words.chars().take(24).collect::<String>(), if gap > 2 { red } else { gold }, BLACK);
            if let Some(words) = self.versus_road_says() {
                s.text_sh(160 - words.len() as i32 * 4, 60, words, gold, BLACK);
            }
            if trip.hold > 0 && trip.hold <= 90 {
                let n = format!("{}", (trip.hold - 1) / 30 + 1);
                s.text_big(150, 50, &n, BLACK, 4);
                s.text_big(148, 48, &n, gold, 4);
            }
        } else {
            s.text_sh(44, 36, &format!("LIMIT {}", road.limit as i32), if mph > road.limit + 10.0 { red } else { white }, BLACK);
        }
        let gas = self.owned().map_or(0.0, |o| o.gas);
        s.text_sh(216, 18, &format!("GAS {gas:.1}"), if gas < 1.0 { red } else { white }, BLACK);
        if let Some(b) = &trip.gearbox {
            s.text_sh(280, 27, &format!("G{}", b.gear + 1), if b.rpm > b.redline - 700.0 { red } else { white }, BLACK);
        }
        s.text_sh(216, 27, &format!("MPH {:3}", mph as i32), if trip.rival.is_none() && mph > road.limit + 10.0 { red } else { white }, BLACK);
        if seat.is_some() && trip.z < 50.0 && trip.hold == 0 && self.note.is_none() && trip.crash == 0 {
            // Setting out: what the keys are.
            let kit = self.owned().is_some_and(|o| self.kit(o).is_some());
            let words = if trip.rival.is_some() { "FIRST TO THE END. ESCAPE GIVES IT UP." } else if trip.gearbox.is_some() { "ARROWS DRIVE. SPACE IS THE NEXT GEAR." } else if kit { "ARROWS DRIVE. N SPRAYS. ESC: BACK." } else { "ARROWS DRIVE. ESCAPE TURNS BACK." };
            s.text_sh(160 - words.len() as i32 * 4, view * 92 / 136, words, gold, BLACK);
        }
        self.draw_bottle(s, 36, self.owned().map_or(0.0, |o| o.bottle), trip.spray > 0.0);
        if trip.poker {
            let hand: Vec<String> = (0..5).map(|i| trip.cards.get(i).map_or("--".to_string(), |&c| cards::short(c))).collect();
            s.text_sh(44, 45, &hand.join(" "), gold, BLACK);
        }
        let blink = self.ticks / 8 % 2 == 0;
        if trip.chase.is_some() {
            // In the mirror: red and blue.
            s.rect(0, 0, 320, 4, shade(ramp(if blink { "RED" } else { "BLUE" }), 26));
            s.text_sh(132, 8, "POLICE!", if blink { red } else { white }, BLACK);
        } else if trip.radar && self.radar() && blink {
            s.text_sh(216, 54, "RADAR!", red, BLACK);
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
