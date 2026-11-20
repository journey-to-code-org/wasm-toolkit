//! The world, shared. Whenever the game is open and the page can reach the meet, everyone playing
//! is in it together: each game says a few times a second where its car is (in a place, on the
//! streets of a map, or down a road), and draws everyone else's. Pull up beside someone and you
//! can dare them on the spot; a dare can come to you wherever you are.
//!
//! Other people's cars are ghosts unless you make them solid (the `ghosts` action). Either way
//! each game only ever decides about its own car: lag can make a near thing look nearer.

use super::online::{Person, CONNECTING, LOST, ON, OFF};
use super::*;
use serde_json::{json, Value};

/// Where someone last said they were, and when that was heard.
#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    /// `place:<id>`, `map:<id>` or `road:<id>`.
    pub w: String,
    pub x: f64,
    pub z: f64,
    /// Heading in degrees, on a map; on a road, whether they are driving it end to start.
    pub h: f64,
    pub back: bool,
    pub v: f64,
    pub tick: u32,
}

/// How long to wait before trying the meet again, in ticks: half a minute.
const AGAIN: u32 = 30 * 30;
/// How near, in metres, someone has to be to be dared from the street.
const NEAR_TOWN: f64 = 14.0;
const NEAR_ROAD: f64 = 30.0;

impl Game {
    /// Where this car is, as it is told to the others.
    fn whereabouts(&self) -> (String, f64, f64, f64, bool, f64) {
        match self.scene {
            Scene::Drive => self.town.as_ref().map_or_else(|| (format!("place:{}", self.state.place), 0.0, 0.0, 0.0, false, 0.0), |t| (format!("map:{}", t.map), t.x, t.z, t.heading, false, t.speed)),
            Scene::Road => self.trip.as_ref().map_or_else(|| (format!("place:{}", self.state.place), 0.0, 0.0, 0.0, false, 0.0), |t| (format!("road:{}", t.road), t.x, t.z, 0.0, t.backwards, t.speed)),
            _ => (format!("place:{}", self.state.place), 0.0, 0.0, 0.0, false, 0.0),
        }
    }

    /// Once a tick: keep the line to the meet up, and tell it where this car is.
    pub(super) fn world_tick(&mut self) {
        if matches!(self.scene, Scene::Title | Scene::Over(_)) {
            return;
        }
        if !self.net.want {
            // Never tried, or dropped: try (again) after a while.
            if self.net.state == OFF || self.net.retry >= AGAIN {
                self.net.want = true;
                self.net.state = CONNECTING;
                self.net.retry = 0;
            } else if self.net.state == LOST {
                self.net.retry += 1;
            }
            return;
        }
        if self.net.state != ON || self.state.cur.is_none() {
            return;
        }
        // A car bought, swapped or painted since the others were told: tell them.
        let now = self.owned().map(|o| (o.car.clone(), o.paint.clone().or_else(|| self.car().and_then(|c| c.paint.clone()))));
        if now != self.net.told_car {
            self.say_here(None);
        }
        let (w, x, z, h, back, v) = self.whereabouts();
        let (tx, tz) = self.net.told_at;
        let moved = w != self.net.told.0 || (x - tx).abs() + (z - tz).abs() > 3.0;
        let due = if v > 0.5 { self.ticks % 6 == 0 } else { self.ticks % 150 == 0 };
        if moved || due {
            self.net.told = (w.clone(), self.ticks);
            self.net.told_at = (x, z);
            let r = |n: f64| (n * 10.0).round() / 10.0;
            self.send(json!({ "t": "at", "w": w, "x": r(x), "z": r(z), "h": r(h), "b": back, "v": r(v) }));
        }
    }

    /// Someone has said where they are.
    pub(super) fn heard_at(&mut self, from: u32, v: &Value) {
        let Some(w) = v.get("w").and_then(|w| w.as_str()).filter(|w| w.len() <= 48) else { return };
        let (kind, id) = w.split_once(':').unwrap_or(("", ""));
        let known = match kind {
            "place" => self.pack.manifest.places.contains_key(id),
            "map" => self.pack.manifest.maps.contains_key(id),
            "road" => self.pack.manifest.roads.contains_key(id),
            _ => false,
        };
        if !known {
            return;
        }
        let n = |k: &str, most: f64| v.get(k).and_then(|x| x.as_f64()).filter(|x| x.is_finite()).unwrap_or(0.0).clamp(-most, most);
        let seen = Seen { w: w.to_string(), x: n("x", 100_000.0), z: n("z", 100_000.0), h: n("h", 360.0), back: v.get("b").and_then(|b| b.as_bool()).unwrap_or(false), v: n("v", 120.0), tick: self.ticks };
        if let Some(p) = self.net.people.iter_mut().find(|p| p.id == from) {
            p.seen = Some(seen);
        }
    }

    /// Everyone else who is where this is (`place:lot`, `map:town`, `road:pike`), and where they
    /// should be by now, carried on from what they last said by how fast they were going.
    pub fn others_at(&self, w: &str) -> Vec<(Person, Seen)> {
        self.net.people.iter().filter_map(|p| {
            let seen = p.seen.as_ref().filter(|s| s.w == w && self.ticks.saturating_sub(s.tick) < 30 * 20)?;
            let mut s = seen.clone();
            let dt = (self.ticks.saturating_sub(seen.tick) as f64 / 30.0).min(1.0);
            if w.starts_with("map:") {
                let (hs, hc) = s.h.to_radians().sin_cos();
                (s.x, s.z) = (s.x + hs * s.v * dt, s.z + hc * s.v * dt);
            } else if w.starts_with("road:") {
                s.z += s.v * dt;
            }
            Some((p.clone(), s))
        }).collect()
    }

    /// On this road, where someone else is as this car sees it: how far along, which side, and
    /// whether they are coming the other way.
    pub fn on_my_road(&self, s: &Seen) -> Option<(f64, f64, bool)> {
        let t = self.trip.as_ref()?;
        Some(if s.back == t.backwards { (s.z, s.x, false) } else { (t.len() - s.z, -s.x, true) })
    }

    /// The one near enough to dare from where this car is, if anyone is, and moving slowly enough.
    pub fn near_player(&self) -> Option<Person> {
        match self.scene {
            Scene::Drive => {
                let t = self.town.as_ref().filter(|t| t.speed < 6.0)?;
                self.others_at(&format!("map:{}", t.map)).into_iter().filter(|(p, _)| p.car.is_some())
                    .map(|(p, s)| (((s.x - t.x).powi(2) + (s.z - t.z).powi(2)).sqrt(), p)).filter(|x| x.0 < NEAR_TOWN).min_by(|a, b| a.0.total_cmp(&b.0)).map(|x| x.1)
            }
            Scene::Road if self.net.versus.is_none() => {
                let t = self.trip.as_ref().filter(|t| t.speed < 8.0 && t.rival.is_none() && !t.poker)?;
                self.others_at(&format!("road:{}", t.road)).into_iter().filter(|(p, _)| p.car.is_some())
                    .filter_map(|(p, s)| Some(((self.on_my_road(&s)?.0 - t.z).abs(), p))).filter(|x| x.0 < NEAR_ROAD).min_by(|a, b| a.0.total_cmp(&b.0)).map(|x| x.1)
            }
            _ => None,
        }
    }

    /// Whether a dare can come to you now: out and about, and not in a race of any kind.
    pub(super) fn free_for_a_dare(&self) -> bool {
        let scene = matches!(self.scene, Scene::Online | Scene::Place | Scene::Drive) || (self.scene == Scene::Road && self.trip.as_ref().is_some_and(|t| t.rival.is_none() && !t.poker));
        scene && self.net.versus.is_none() && self.race.is_none() && self.bracket.is_none() && self.net.dare_in.is_none() && self.net.dare_out.is_none()
    }

    /// Dares on the street: answering one, calling one off, or making one to whoever you have
    /// pulled up beside. True if the keys went on that.
    pub(super) fn street(&mut self, hit: u32) -> bool {
        if !matches!(self.scene, Scene::Place | Scene::Drive | Scene::Road) || self.net.versus.is_some() {
            return false;
        }
        if let Some((from, stake)) = self.net.dare_in {
            if hit & KEY_A != 0 {
                self.send(json!({ "t": "ok", "to": from }));
                self.net.road_now = self.net.road_in.take();
                self.town = None;
                self.trip = None;
                self.start_versus(from, stake);
                return true;
            }
            if hit & KEY_B != 0 {
                self.send(json!({ "t": "no", "to": from }));
                self.net.dare_in = None;
                return true;
            }
            return false;
        }
        if let Some((to, _)) = self.net.dare_out {
            if hit & KEY_B != 0 {
                self.send(json!({ "t": "never", "to": to }));
                self.net.dare_out = None;
                self.say("Never mind, then.");
                return true;
            }
            return false;
        }
        if hit & KEY_A != 0 && self.scene != Scene::Place {
            if let Some(p) = self.near_player() {
                if let Some(why) = self.cannot_race() {
                    self.no(&why);
                    return true;
                }
                // The road you are both on, if it is one; otherwise the strip.
                self.net.course = match (self.scene, self.trip.as_ref()) {
                    (Scene::Road, Some(t)) => self.pack.manifest.roads.keys().position(|k| *k == t.road).map_or(0, |i| i + 1),
                    _ => 0,
                };
                self.net.picked = Some(p.id);
                self.net.back_to = Some(self.scene);
                self.note = None;
                self.scene = Scene::OnlineBets;
                return true;
            }
        }
        false
    }

    /// Other people's cars, if they are solid: a knock, the same as any other.
    pub(super) fn bump_others(&mut self) -> bool {
        if !self.state.solid {
            return false;
        }
        match self.scene {
            Scene::Drive => {
                let Some(t) = self.town.as_ref().filter(|t| t.crash == 0) else { return false };
                let hit = self.others_at(&format!("map:{}", t.map)).iter().any(|(_, s)| (s.x - t.x).powi(2) + (s.z - t.z).powi(2) < 2.4f64.powi(2));
                if hit {
                    let t = self.town.as_mut().unwrap();
                    (t.speed, t.crash) = (0.0, 20);
                }
                hit
            }
            Scene::Road => {
                let Some(t) = self.trip.as_ref().filter(|t| t.crash == 0) else { return false };
                let (z, x) = (t.z, t.x);
                let others = self.others_at(&format!("road:{}", t.road));
                let hit = others.iter().filter_map(|(_, s)| self.on_my_road(s)).any(|(oz, ox, _)| (oz - z).abs() < 4.5 && (ox - x).abs() < 0.5);
                if hit {
                    let t = self.trip.as_mut().unwrap();
                    (t.speed, t.crash) = (t.speed * 0.3, 30);
                }
                hit
            }
            _ => false,
        }
    }

    /// Over whatever is on the screen: a dare waiting on you, one you are waiting on, or someone
    /// near enough to dare.
    pub(super) fn draw_street(&self, s: &mut Screen) {
        if !matches!(self.scene, Scene::Place | Scene::Drive | Scene::Road) || self.net.versus.is_some() {
            return;
        }
        let course = |road: &Option<(String, u32)>| road.as_ref().and_then(|(r, _)| self.pack.manifest.roads.get(r)).map_or("THE STRIP".to_string(), |r| r.title.to_uppercase());
        let line = if let Some((from, stake)) = self.net.dare_in {
            let money = if stake > 0 { format!("${stake}") } else { "KICKS".into() };
            Some((format!("{} DARES YOU: {money}, {}", self.name_of(from).to_uppercase(), course(&self.net.road_in)), "ENTER: YOU'RE ON. ESC: NO.".to_string()))
        } else if let Some((to, _)) = self.net.dare_out {
            Some((format!("WAITING ON {}", self.name_of(to).to_uppercase()), "ESC: NEVER MIND.".to_string()))
        } else {
            self.near_player().map(|p| (format!("{} IS RIGHT HERE", p.name.to_uppercase()), "ENTER: DARE THEM.".to_string()))
        };
        let Some((top, bottom)) = line else { return };
        let (top, bottom): (String, String) = (top.chars().take(38).collect(), bottom.chars().take(38).collect());
        s.rect(0, 150, 320, 22, BLACK);
        s.text_sh(160 - top.len() as i32 * 4, 152, &top, shade(ramp("GOLD"), 28), BLACK);
        s.text_sh(160 - bottom.len() as i32 * 4, 162, &bottom, WHITE, BLACK);
    }

    /// Who else is in this place, for its screen.
    pub(super) fn here_too(&self) -> Vec<String> {
        self.others_at(&format!("place:{}", self.state.place)).into_iter().map(|(p, _)| p.name).collect()
    }
}
