//! The world past the edge of town: driving on out along the roads with no switch to another
//! view, pulling in at the places at their ends, and the time and gas the miles take.
//!
//! The roads are laid out on the ground by `land.rs`. Town driving goes on working out there: the
//! same car, the same keys, the same view. What is left on the old road view is what is still
//! raced on it: the road races and the poker run.

use super::*;
use crate::land::{in_town, Near, Site, World, SITE};
use crate::road::{Side, HALF};

/// Someone raced down a road of the world: how far from the line, how far across to the right
/// of the way the race runs, and what their car can do.
pub struct Runner {
    pub s: f64,
    pub across: f64,
    pub speed: f64,
    pub top: f64,
    pub accel: f64,
    pub paint: String,
}

/// A race or a poker run out on a road of the world: which road, whether it is run against the
/// way its points go, who against, the cards so far, and the furthest down it the car has been.
pub struct Dash {
    pub lane: usize,
    pub back: bool,
    pub rival: Option<Runner>,
    pub poker: bool,
    pub cards: Vec<u8>,
    pub far: f64,
}

/// How far the roads are drawn, and how far the things beside them.
const ROADS_SEEN: f64 = 520.0;
const SCENERY_SEEN: f64 = 240.0;

impl Game {
    /// The world of this game's pack, if it has one.
    pub fn land(&self) -> Option<&World> {
        self.land.as_ref()
    }

    /// How high the ground is under a point of the map being driven.
    pub(super) fn ground(&self, map: &crate::pack::Map, x: f64, z: f64) -> f64 {
        match self.land() {
            Some(w) => w.ground(map, x, z),
            None => map.height(x, z),
        }
    }

    /// Whether the car is out past the last street.
    pub fn out_of_town(&self) -> bool {
        let Some(town) = &self.town else { return false };
        let Some(map) = self.pack.manifest.maps.get(&town.map) else { return false };
        self.land().is_some() && !in_town(map, town.x, town.z, 0.0)
    }

    /// The road the car is on or beside, out of town.
    pub fn lane_here(&self) -> Option<Near> {
        let town = self.town.as_ref()?;
        self.out_of_town().then(|| self.land()?.near(town.x, town.z, 40.0)).flatten()
    }

    /// A place at a road's end the car is stopped at.
    pub(super) fn site_beside(&self) -> Option<&Site> {
        let town = self.town.as_ref()?;
        if town.speed > 2.0 {
            return None;
        }
        self.land()?.sites.iter().find(|s| (s.x - town.x).powi(2) + (s.z - town.z).powi(2) < SITE.powi(2))
    }

    /// Whether a road is laid out in the world, so it is driven there and not on the road view.
    pub(super) fn laid(&self, road: &str) -> bool {
        self.land().is_some_and(|w| w.lanes.iter().any(|l| l.id == road))
    }

    /// The traffic for the roads, new.
    pub(super) fn new_traffic(&mut self) -> Vec<crate::land::Rolling> {
        let Some(w) = self.land.as_ref() else { return Vec::new() };
        let mut rng = self.rng;
        let cars = w.traffic(&self.pack.manifest, || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            rng
        });
        self.rng = rng;
        cars
    }

    /// Out of a place onto a road it is at the end of: in the car in its lot, facing down the road.
    pub(super) fn onto_road(&mut self, id: &str) {
        if self.state.cur.is_none() {
            return self.no("You have nothing to drive.");
        }
        if let Some(why) = self.wont_run() {
            return self.no(&why);
        }
        if self.set_out(id).is_none() {
            return self.no("That road does not start here.");
        }
        tone(520, 60);
    }

    /// The traffic and the police for the roads, put out for the car that is driving.
    pub(super) fn fill_roads(&mut self) {
        let traffic = self.new_traffic();
        let Some(w) = self.land.as_ref() else { return };
        let mut rng = self.rng;
        let cops = w.cops(&self.pack.manifest, || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            rng
        });
        self.rng = rng;
        if let Some(t) = self.town.as_mut() {
            t.rolling = traffic;
            t.cops.truncate(t.town_cops);
            t.cops.extend(cops);
        }
    }

    /// Into the car at this place's end of a road, facing down it. Which road of the world it is,
    /// and whether that is against the way its points run.
    fn set_out(&mut self, id: &str) -> Option<(usize, bool)> {
        let w = self.land.as_ref()?;
        let k = w.lanes.iter().position(|l| l.id == id)?;
        let lane = &w.lanes[k];
        let here = self.state.place.clone();
        let from_start = lane.ends[0] == here;
        if !from_start && lane.ends[1] != here {
            return None;
        }
        let along = if from_start { 12.0 } else { lane.len() - 12.0 };
        let p = lane.at(along);
        let (x, z, _) = lane.beside(along, if from_start { HALF / 2.0 } else { -HALF / 2.0 });
        let heading = (if from_start { p.h } else { p.h + std::f64::consts::PI }).to_degrees().rem_euclid(360.0);
        let home = w.home.clone();
        let map = self.pack.manifest.maps.get(&home)?;
        let mut rng = self.rng;
        let mut town = Town::new(&home, map, None, || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            rng
        });
        self.rng = rng;
        (town.x, town.z, town.heading, town.speed) = (x, z, heading, 0.0);
        self.town = Some(town);
        self.fill_roads();
        self.fit_town();
        // There is no going back to the garage from here but by driving there.
        self.town_by_road = true;
        self.dash = None;
        self.note = None;
        self.scene = Scene::Drive;
        Some((k, !from_start))
    }

    /// Down a road against whoever has pulled in, for what was bet, out on the road itself: side
    /// by side on the line, three, two, one, and first to the far end. No police.
    pub(super) fn start_dash(&mut self, id: &str) {
        let Some(r) = self.rival().cloned() else { return };
        let Some((lane, back)) = self.set_out(id) else { return };
        let quick = 14.0 / r.et.clamp(8.0, 30.0);
        let wet = if self.raining() { 0.93 } else { 1.0 };
        let them = Runner { s: 12.0, across: -HALF / 2.0, speed: 0.0, top: (56.0 * quick.powf(1.3)).clamp(30.0, 76.0) * wet, accel: (5.0 * quick * quick).clamp(2.5, 9.0), paint: r.paint.clone().unwrap_or_else(|| "RED".into()) };
        let (gearbox, best, pull) = (self.gearbox(), self.launch_speed(), self.pace().pull);
        if let Some(t) = self.town.as_mut() {
            (t.gearbox, t.best_launch, t.pull, t.hold) = (gearbox, best, pull, 90);
        }
        self.dash = Some(Dash { lane, back, rival: Some(them), poker: false, cards: Vec::new(), far: 12.0 });
    }

    /// A poker run out on the road itself: round it and back to where it starts, a card at each
    /// sixth of the way.
    pub(super) fn start_poker_out(&mut self, id: &str, entry: i32) {
        let Some((lane, back)) = self.set_out(id) else { return };
        self.rand();
        self.state.cash -= entry;
        self.dash = Some(Dash { lane, back, rival: None, poker: true, cards: Vec::new(), far: 12.0 });
        self.say("A card at every stop. Mind the limit.");
    }

    /// How far down the road of a race or a run the car has got, from where it set out, and how
    /// far across (to its right is more than nothing), if it is still on that road.
    pub fn dash_at(&self) -> Option<(f64, f64)> {
        let (d, t, w) = (self.dash.as_ref()?, self.town.as_ref()?, self.land.as_ref()?);
        let n = w.near_on(Some(d.lane), t.x, t.z, 60.0)?;
        Some(if d.back { (w.lanes[d.lane].len() - n.along, -n.across) } else { (n.along, n.across) })
    }

    /// Escape in a race or a run: lifting is losing, and a run is dropped out of.
    pub(super) fn quit_dash(&mut self) {
        let Some(d) = self.dash.take() else { return };
        tone(0, 0);
        self.town = None;
        if d.rival.is_some() {
            return self.settle(Some(false));
        }
        self.scene = Scene::Place;
        self.say("You dropped out of the run.");
    }

    /// A thirtieth of a second of a race or a run out on the road. True if it is over.
    pub(super) fn dash_tick(&mut self) -> bool {
        let Some((s, across)) = self.dash_at() else { return false };
        let waiting = self.town.as_ref().is_some_and(|t| t.hold > 0);
        let Some(w) = self.land.as_ref() else { return false };
        let Some(d) = self.dash.as_mut() else { return false };
        let lane = &w.lanes[d.lane];
        let len = lane.len();
        // Only the road in front counts: not a short cut back to the line.
        if s > d.far - 60.0 && s < d.far + 150.0 {
            d.far = d.far.max(s);
        }
        let mut beaten = false;
        if let Some(r) = d.rival.as_mut().filter(|_| !waiting) {
            // They lift for a bend they can see, and no more than they have to.
            let on = |s: f64| lane.bend(if d.back { len - s } else { s }).abs();
            let want = r.top * (1.0 - 0.42 * on(r.s).max(on(r.s + 40.0)));
            r.speed = if r.speed < want { r.speed + r.accel / 30.0 * (1.0 - r.speed / r.top) } else { r.speed - 8.0 / 30.0 };
            r.s += r.speed / 30.0;
            // Side by side they keep to the lane you are not in; clear of you, to the right one.
            let side = if (r.s - s).abs() < 9.0 { if across > 0.0 { -HALF / 2.0 } else { HALF / 2.0 } } else { HALF / 2.0 };
            r.across += (side - r.across) * 0.06;
            beaten = r.s >= len - 2.0 && d.far < len - 2.0;
        }
        let racing = d.rival.is_some();
        let arrived = d.far >= len - if d.poker { 40.0 } else { 2.0 };
        // A poker run: a card at each sixth of the way, off a deck dealt for the day.
        let mut dealt = None;
        if d.poker && d.cards.len() < 5 && d.far >= len * (d.cards.len() + 1) as f64 / 6.0 {
            let mut k = self.rng ^ (d.cards.len() as u32 + 1).wrapping_mul(2654435761);
            let card = loop {
                k = hash(k);
                if !d.cards.contains(&((k % 52) as u8)) {
                    break (k % 52) as u8;
                }
            };
            d.cards.push(card);
            dealt = Some((d.cards.len(), card));
        }
        if let Some((n, card)) = dealt {
            tone(880, 90);
            self.say(&format!("Card {n}: the {}.", cards::card(card)));
        }
        if racing && (arrived || beaten) {
            tone(if arrived { 990 } else { 160 }, 300);
            let abuse = self.town.as_ref().map_or(0.0, |t| t.abuse);
            self.ride_clutch(abuse);
            (self.dash, self.town) = (None, None);
            self.settle(Some(arrived));
            return true;
        }
        if !racing && arrived {
            let hand = self.dash.take().map(|d| d.cards).unwrap_or_default();
            tone(0, 0);
            self.town = None;
            self.scene = Scene::Place;
            self.settle_poker(&hand);
            return true;
        }
        false
    }

    /// Where whoever is being raced is on the ground, and which way they face, in degrees.
    pub(super) fn runner_at(&self) -> Option<(f64, f64, f64, &Runner)> {
        let (d, w) = (self.dash.as_ref()?, self.land.as_ref()?);
        let (r, lane) = (d.rival.as_ref()?, &w.lanes[d.lane]);
        let along = if d.back { lane.len() - r.s } else { r.s };
        let (x, z, _) = lane.beside(along.clamp(0.0, lane.len()), if d.back { -r.across } else { r.across });
        let h = lane.at(along).h.to_degrees() + if d.back { 180.0 } else { 0.0 };
        Some((x, z, h.rem_euclid(360.0), r))
    }

    /// From the block in town that leads to a road, out to where the road leaves town.
    pub(super) fn to_gate(&mut self, road: &str) {
        let Some(town) = self.town.as_mut() else { return };
        let Some(map) = self.pack.manifest.maps.get(&town.map) else { return };
        let Some((x, z, heading)) = map.gate(road) else { return };
        let out = (heading + 180.0).to_radians();
        (town.x, town.z, town.heading, town.speed) = (x - out.sin() * 6.0, z - out.cos() * 6.0, (heading + 180.0).rem_euclid(360.0), 0.0);
        let title = self.pack.manifest.roads.get(road).map_or(String::new(), |r| r.title.clone());
        self.say(&format!("{title}: straight on out."));
        tone(520, 60);
    }

    /// What driving a road costs that driving round town does not: hours, by how far down it you
    /// went. And a road you are not ready for turns you round. Returns the road's gas per km.
    pub(super) fn on_the_road(&mut self, went: f64) -> Option<f64> {
        let near = self.lane_here()?;
        let w = self.land.as_ref()?;
        let lane = &w.lanes[near.lane];
        let road = self.pack.manifest.roads.get(&lane.id)?;
        let (per_km, hours, rung, len, reversed, starts) = (road.gas_per_km, road.hours, road.rung, lane.len(), lane.reversed, lane.ends[0].clone());
        // How far from the end the road's rung is counted from: the start, as the pack has it.
        let from_start = if reversed { len - near.along } else { near.along };
        if self.dash.is_none() && self.state.rung < rung && starts.starts_with("map:") == !reversed && from_start > 60.0 {
            let more = rung - self.state.rung;
            let back = if reversed { len - 50.0 } else { 50.0 };
            let p = lane.at(back);
            let (x, z, _) = lane.beside(back, if reversed { HALF / 2.0 } else { -HALF / 2.0 });
            let heading = (if reversed { p.h } else { p.h + std::f64::consts::PI }).to_degrees().rem_euclid(360.0);
            if let Some(town) = self.town.as_mut() {
                (town.x, town.z, town.heading, town.speed) = (x, z, heading, 0.0);
            }
            self.say(&format!("Not yet. Beat {more} more here first."));
            tone(160, 200);
            return Some(per_km);
        }
        if len > 0.0 && hours > 0 && !self.dash.as_ref().is_some_and(|d| d.rival.is_some()) {
            self.clock += went * hours as f64 / len;
            if self.clock >= 1.0 {
                self.clock -= 1.0;
                self.spend(1);
            }
        }
        Some(per_km)
    }

    /// Whether Enter is for a place at a road's end, and went in.
    pub(super) fn pull_in(&mut self) -> bool {
        if self.dash.is_some() {
            return false;
        }
        let Some(place) = self.site_beside().map(|s| s.place.clone()) else { return false };
        tone(520, 60);
        self.town = None;
        self.note = None;
        self.enter(&place);
        true
    }

    /// Something squared-off standing on the ground, drawn in its turn by how far away it is.
    pub(super) fn stand(scene: &mut Scene3, at: V3, f: (f64, f64), hl: f64, hw: f64, tall: f64, ramp: usize) {
        let Some((_, _, depth)) = scene.point([at[0], at[1] + tall * 0.5, at[2]]) else { return };
        Self::obox(scene, at, f, hl, hw, tall, ramp, depth);
    }

    /// The roads out past town, what stands beside them, and the places at their ends.
    pub(super) fn draw_land(&self, scene: &mut Scene3, map: &crate::pack::Map, dim: i32) -> Vec<(V3, String)> {
        let mut names = Vec::new();
        let Some(w) = self.land() else { return names };
        let [ex, _, ez] = scene.cam.pos;
        let (yaw_s, yaw_c) = scene.cam.yaw.sin_cos();
        let ahead = |x: f64, z: f64| (x - ex) * yaw_s + (z - ez) * yaw_c;
        let (tar, line, edge, grass) = (ramp("ASPHALT"), shade(ramp("YELLOW"), 23), shade(ramp("WHITE"), 24), ramp("GRASS"));
        for lane in &w.lanes {
            let n = lane.pts.len();
            let mut i = 0;
            while i + 1 < n {
                let a = lane.pts[i];
                let d = ((a.x - ex).powi(2) + (a.z - ez).powi(2)).sqrt();
                // Far away, in longer pieces; far enough, not at all; behind, not at all.
                let step = if d < 160.0 { 1 } else if d < 320.0 { 2 } else { 4 };
                let j = (i + step).min(n - 1);
                if d > ROADS_SEEN + 40.0 || ahead(a.x, a.z) < -30.0 && ahead(lane.pts[j].x, lane.pts[j].z) < -30.0 {
                    i = j;
                    continue;
                }
                let b = lane.pts[j];
                // Where the road leaves town it runs over the street; the street is drawn.
                let in_streets = in_town(map, a.x, a.z, -1.0) && in_town(map, b.x, b.z, -1.0);
                let at = |p: &crate::land::Pt, out: f64, up: f64| -> V3 { [p.x + p.h.cos() * out, p.y + up, p.z - p.h.sin() * out] };
                let band = if (i / 2) % 2 == 0 { 0 } else { 1 };
                if !in_streets {
                    for (sign, side) in [(-1.0, a.left), (1.0, a.right)] {
                        let ground = match side {
                            Side::Drop | Side::Water => None,
                            Side::Fields => Some(ramp("TAN")),
                            Side::Rock => Some(ramp("CONCRETE")),
                            _ => Some(grass),
                        };
                        if let Some(g) = ground {
                            let c = shade(g, 15 + band);
                            scene.poly(&[at(&a, sign * HALF, 0.0), at(&b, sign * HALF, 0.0), at(&b, sign * 60.0, -0.2), at(&a, sign * 60.0, -0.2)], c, c, -8000.0);
                        }
                        if side == Side::Water {
                            let wave = ((i as u32 + self.ticks / 6) % 3) as i32;
                            let sand = shade(ramp("TAN"), 17 + band);
                            scene.poly(&[at(&a, sign * HALF, 0.0), at(&b, sign * HALF, 0.0), at(&b, sign * HALF * 1.5, -0.6), at(&a, sign * HALF * 1.5, -0.6)], sand, sand, -7000.0);
                            let blue = shade(ramp("BLUE"), (15 + wave * 2 - dim.min(9)).max(3));
                            scene.poly(&[at(&a, sign * HALF * 1.5, -0.6), at(&b, sign * HALF * 1.5, -0.6), at(&b, sign * 200.0, -0.6), at(&a, sign * 200.0, -0.6)], blue, shade(ramp("BLUE"), (17 + wave * 2 - dim.min(9)).max(3)), -9000.0);
                        }
                    }
                }
                let c = shade(tar, 15 + band);
                scene.poly(&[at(&a, -HALF, 0.02), at(&b, -HALF, 0.02), at(&b, HALF, 0.02), at(&a, HALF, 0.02)], c, c, -5000.0);
                if d < 200.0 {
                    for sign in [-1.0, 1.0] {
                        scene.poly(&[at(&a, sign * HALF * 0.9, 0.03), at(&b, sign * HALF * 0.9, 0.03), at(&b, sign * HALF * 0.94, 0.03), at(&a, sign * HALF * 0.94, 0.03)], edge, edge, -3000.0);
                    }
                    if i % 4 < 2 {
                        scene.poly(&[at(&a, -0.1, 0.03), at(&b, -0.1, 0.03), at(&b, 0.1, 0.03), at(&a, 0.1, 0.03)], line, line, -3000.0);
                    }
                }
                if d < SCENERY_SEEN && !in_streets {
                    let f = (a.h.sin(), a.h.cos());
                    for (sign, side) in [(-1.0, a.left), (1.0, a.right)] {
                        match side {
                            Side::Rock => {
                                // A cutting: the hill it was blasted through, close and tall.
                                scene.lit(&[at(&a, sign * HALF * 1.5, 0.0), at(&b, sign * HALF * 1.5, 0.0), at(&b, sign * HALF * 2.2, 9.0 + (i % 3) as f64), at(&a, sign * HALF * 2.2, 9.0 + ((i + 2) % 3) as f64)], ramp(if i % 2 == 0 { "CONCRETE" } else { "GREY" }), 0.0);
                            }
                            Side::Drop => {
                                // A rail, on posts, and the valley past it.
                                let c = shade(ramp("CHROME"), 22 + band * 3);
                                scene.poly(&[at(&a, sign * HALF * 1.2, 0.45), at(&b, sign * HALF * 1.2, 0.45), at(&b, sign * HALF * 1.2, 0.8), at(&a, sign * HALF * 1.2, 0.8)], c, c, 0.0);
                            }
                            Side::Posts | Side::Fields if i % if side == Side::Posts { 2 } else { 3 } == 0 => {
                                let (out, tall, col) = if side == Side::Posts { (1.12, 0.9, "WHITE") } else { (1.5, 1.2, "WOOD") };
                                Self::stand(scene, at(&a, sign * HALF * out, 0.0), f, 0.08, 0.08, tall, ramp(col));
                            }
                            Side::Trees if i % 2 == 0 => {
                                // The same tree every time you pass it.
                                let r = (i as u32 ^ lane.id.len() as u32 * 7919).wrapping_mul(2654435761) >> 8;
                                let (out, tall, wide) = (1.7 + (r % 13) as f64 * 0.15, 5.0 + (r % 5) as f64, 1.6 + (r % 3) as f64 * 0.4);
                                let base = at(&a, sign * HALF * out, 0.0);
                                Self::stand(scene, base, f, 0.2, 0.2, 1.6, ramp("WOOD"));
                                let (top, y) = ([base[0], base[1] + tall, base[2]], base[1] + 1.2);
                                let green = ramp(if r % 4 == 0 { "GRASS" } else { "FOREST" });
                                let corner = |p: f64, q: f64| [base[0] + p * wide, y, base[2] + q * wide];
                                for (p, q) in [((-1.0, -1.0), (1.0, -1.0)), ((1.0, -1.0), (1.0, 1.0)), ((1.0, 1.0), (-1.0, 1.0)), ((-1.0, 1.0), (-1.0, -1.0))] {
                                    let mid = [(p.0 + q.0) * 0.5, (p.1 + q.1) * 0.5];
                                    if mid[0] * (ex - base[0]) + mid[1] * (ez - base[2]) > 0.0 {
                                        scene.lit(&[corner(p.0, p.1), corner(q.0, q.1), top], green, 0.0);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                i = j;
            }
        }
        // The places at the roads' ends: a lot to pull onto, and a building beside it with its name up.
        for site in &w.sites {
            let d = ((site.x - ex).powi(2) + (site.z - ez).powi(2)).sqrt();
            if d > ROADS_SEEN {
                continue;
            }
            let (s, c) = site.h.sin_cos();
            let at = |along: f64, out: f64, up: f64| -> V3 { [site.x + s * along + c * out, site.y + up, site.z + c * along - s * out] };
            let lot = shade(ramp("CONCRETE"), 17);
            scene.poly(&[at(-SITE, -SITE, 0.01), at(SITE, -SITE, 0.01), at(SITE, SITE, 0.01), at(-SITE, SITE, 0.01)], lot, shade(ramp("CONCRETE"), 18), -6000.0);
            let title = self.pack.manifest.places.get(&site.place).map_or(site.place.clone(), |p| p.title.clone());
            let (wall, tall) = (ramp("CONCRETE"), 9.0);
            Self::stand(scene, at(0.0, SITE + 8.0, 0.0), (s, c), 14.0, 8.0, tall, wall);
            Self::stand(scene, at(0.0, SITE - 1.0, 3.2), (s, c), 15.5, 1.5, 0.8, ramp("GOLD"));
            Self::stand(scene, at(0.0, SITE + 8.0, tall), (s, c), 0.3, 0.3, 7.0, ramp("CHROME"));
            Self::stand(scene, at(0.0, SITE + 8.0, tall + 7.0), (s, c), 4.0, 0.6, 3.0, ramp("GOLD"));
            names.push((at(0.0, SITE + 8.0, tall + 12.0), title));
        }
        names
    }

    /// The world from above, in the corner, when out of town: the roads, the places, the town,
    /// and you.
    pub(super) fn draw_world_map(&self, s: &mut Screen, map: &crate::pack::Map, x: f64, z: f64, hs: f64, hc: f64) {
        let Some(w) = self.land() else { return };
        let (mut x0, mut z0, mut x1, mut z1) = (0.0f64, 0.0f64, map.cols() as f64 * map.pitch, map.depth() as f64 * map.pitch);
        for p in w.lanes.iter().flat_map(|l| l.pts.iter()) {
            (x0, z0, x1, z1) = (x0.min(p.x), z0.min(p.z), x1.max(p.x), z1.max(p.z));
        }
        let (bx, by, size) = (42, 14, 52);
        let scale = (size as f64 - 4.0) / (x1 - x0).max(z1 - z0).max(1.0);
        let (ox, oz) = ((x0 + x1) / 2.0, (z0 + z1) / 2.0);
        let to = |x: f64, z: f64| (bx + size / 2 + ((x - ox) * scale) as i32, by + size / 2 - ((z - oz) * scale) as i32);
        s.rect(bx, by, size, size, BLACK);
        s.frame(bx, by, size, size, shade(ramp("GREY"), 12));
        for lane in &w.lanes {
            for k in (0..lane.pts.len()).step_by(12) {
                let (a, b) = (lane.pts[k], lane.pts[(k + 12).min(lane.pts.len() - 1)]);
                let (p, q) = (to(a.x, a.z), to(b.x, b.z));
                s.line(p.0, p.1, q.0, q.1, shade(ramp("GREY"), 20));
            }
        }
        let (t0, t1) = (to(0.0, map.depth() as f64 * map.pitch), to(map.cols() as f64 * map.pitch, 0.0));
        s.rect(t0.0, t0.1, (t1.0 - t0.0).max(2), (t1.1 - t0.1).max(2), shade(ramp("GREY"), 10));
        for site in &w.sites {
            let p = to(site.x, site.z);
            s.rect(p.0 - 1, p.1 - 1, 3, 3, shade(ramp("GOLD"), 27));
        }
        if self.radar() {
            // A detector puts them on the map.
            for &(cx, cz) in self.town.iter().flat_map(|t| t.cops.iter()) {
                let p = to(cx, cz);
                s.rect(p.0, p.1, 2, 2, shade(ramp("CYAN"), 28));
            }
        }
        if let Some((rx, rz, _, _)) = self.runner_at() {
            let p = to(rx, rz);
            s.rect(p.0, p.1, 2, 2, shade(ramp("ORANGE"), 28));
        }
        for (_, at) in self.others_at(&format!("map:{}", map_id(self))) {
            let p = to(at.x, at.z);
            s.rect(p.0, p.1, 2, 2, shade(ramp("LIME"), 28));
        }
        let u = to(x, z);
        s.line(u.0, u.1, u.0 + (hs * 4.0) as i32, u.1 - (hc * 4.0) as i32, WHITE);
        s.disc(u.0, u.1, 1, shade(ramp("RED"), 28));
    }
}

fn map_id(g: &Game) -> String {
    g.town.as_ref().map_or(String::new(), |t| t.map.clone())
}
