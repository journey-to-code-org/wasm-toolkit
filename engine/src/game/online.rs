//! Racing other people. Whoever opens the lot's online meet is in one room with everyone else
//! who has; each can see the others' cars, dare one of them to a race for kicks or money, and run
//! it side by side.
//!
//! Each player's own engine runs their own car, so the throttle and the shifts are as sharp as
//! they are alone. What crosses the wire is small: who is here and in what, a dare and its
//! answer, "I am staged", where I am a few times a second, and my timeslip. Both ends work out
//! the winner from the two timeslips, the same way.
//!
//! The engine does not open sockets. It asks the page for a line (`want`), is told how that went
//! (`net_status`), hands the page what to send (`outbox`) and is handed what arrives (`net_in`).
//! What arrives is someone else's words: nothing in it is trusted further than it has to be.

use super::*;
use crate::gfx::RAMPS;
use serde_json::{json, Value};

pub const OFF: u8 = 0;
pub const CONNECTING: u8 = 1;
pub const ON: u8 = 2;
pub const LOST: u8 = 3;
/// How long to wait at the far end for the other timeslip before taking the win: forty seconds.
const PATIENCE: u32 = 40 * 30;
const MOST_PEOPLE: usize = 16;

/// Someone else in the meet, and what they brought, once they have said.
#[derive(Clone, Debug)]
pub struct Person {
    pub id: u32,
    pub name: String,
    pub car: Option<String>,
    pub paint: Option<String>,
}

/// The other lane's timeslip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slip {
    pub rt: i32,
    pub et: i32,
    pub red: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    Won,
    Lost,
    /// Both left early: nobody is paid.
    NoRace,
}

/// A race with someone, from the dare being taken to the money changing hands.
#[derive(Debug)]
pub struct Versus {
    pub id: u32,
    pub stake: i32,
    pub me_staged: bool,
    pub him_staged: bool,
    pub him: Option<Slip>,
    pub him_gone: bool,
    pub result: Option<Outcome>,
    him_v: f64,
    sent_slip: bool,
    waited: u32,
    /// The road it is down, if it is not the strip, and how long it took you, in thousandths of a second.
    pub road: Option<String>,
    mine: i32,
}

#[derive(Default)]
pub struct Net {
    /// Whether the engine wants a line to the meet. The page opens and shuts it to match.
    pub want: bool,
    pub state: u8,
    pub me: u32,
    pub people: Vec<Person>,
    /// What to send, a message apiece. The page takes them after every step.
    pub outbox: Vec<String>,
    /// A dare you have made and one made to you: who, and for how much.
    pub dare_out: Option<(u32, i32)>,
    pub dare_in: Option<(u32, i32)>,
    pub versus: Option<Versus>,
    picked: Option<u32>,
    /// Where a dare is for: 0 the strip, or one of the pack's roads, counted from 1.
    pub course: usize,
    /// The road of the dare made, of the dare made to you, and of the race about to start, each
    /// with the number the other cars on it are dealt from, so both see the same ones.
    road_out: Option<(String, u32)>,
    road_in: Option<(String, u32)>,
    road_now: Option<(String, u32)>,
}

/// Who wins, from the two timeslips. Leaving before the green loses; both doing it is no race.
pub fn decide(me: Slip, him: Slip) -> Outcome {
    match (me.red, him.red) {
        (true, true) => Outcome::NoRace,
        (true, false) => Outcome::Lost,
        (false, true) => Outcome::Won,
        _ if me.rt + me.et < him.rt + him.et => Outcome::Won,
        _ if me.rt + me.et == him.rt + him.et => Outcome::NoRace,
        _ => Outcome::Lost,
    }
}

fn words(v: &Value, key: &str, most: usize) -> Option<String> {
    v.get(key)?.as_str().map(|s| s.chars().filter(|c| !c.is_control()).take(most).collect())
}

fn whole(v: &Value, key: &str) -> Option<i64> {
    v.get(key)?.as_i64()
}

impl Game {
    fn send(&mut self, message: Value) {
        self.net.outbox.push(message.to_string());
    }

    fn person(&self, id: u32) -> Option<&Person> {
        self.net.people.iter().find(|p| p.id == id)
    }

    fn name_of(&self, id: u32) -> String {
        self.person(id).map(|p| p.name.clone()).unwrap_or_else(|| "Someone".into())
    }

    /// Who is in the other lane, online or not: their car, its paint and their name.
    pub(super) fn opponent(&self) -> Option<(String, Option<String>, String)> {
        match &self.net.versus {
            Some(v) => self.person(v.id).map(|p| (p.car.clone().unwrap_or_default(), p.paint.clone(), p.name.clone())),
            None => self.rival().map(|r| (r.car.clone(), r.paint.clone(), r.name.clone())),
        }
    }

    /// Into the meet. It needs a car that can make a pass.
    pub(super) fn go_online(&mut self) {
        if self.state.cur.is_none() {
            return self.no("You have nothing to race.");
        }
        if let Some(why) = self.cannot_race() {
            return self.no(&why);
        }
        self.net = Net { want: true, state: CONNECTING, ..Net::default() };
        self.note = None;
        self.scene = Scene::Online;
    }

    fn leave_online(&mut self) {
        self.net = Net::default();
        self.race = None;
        self.note = None;
        self.scene = Scene::Place;
    }

    /// Tell them what you are in: everyone, or the one who has just come.
    fn say_here(&mut self, to: Option<u32>) {
        let Some(o) = self.owned() else { return };
        let paint = o.paint.clone().or_else(|| self.car().and_then(|c| c.paint.clone()));
        let mut message = json!({ "t": "here", "car": o.car, "paint": paint });
        if let Some(to) = to {
            message["to"] = json!(to);
        }
        self.send(message);
    }

    /// The page says how the line is: 2 it is open, 3 it could not be had or has dropped.
    pub fn net_status(&mut self, code: u8) {
        if !self.net.want {
            return;
        }
        if code == LOST {
            // A race in the air is no race, and nobody is paid.
            let racing = self.net.versus.is_some();
            self.net.people.clear();
            self.net.versus = None;
            self.net.dare_in = None;
            self.net.dare_out = None;
            self.net.state = LOST;
            self.net.want = false;
            if racing {
                self.race = None;
                self.scene = Scene::Online;
                self.say("The line dropped. No race.");
            }
        } else if code == ON && self.net.state == CONNECTING {
            // Open, but nobody is here until the room has said who is.
            self.net.state = CONNECTING;
        }
    }

    /// Something has arrived from the room: who is here, or what one of them said.
    pub fn net_in(&mut self, text: &str) {
        if !self.net.want || text.len() > 2048 {
            return;
        }
        let Ok(v) = serde_json::from_str::<Value>(text) else { return };
        let Some(kind) = v.get("t").and_then(|t| t.as_str()) else { return };
        let id = |key: &str| whole(&v, key).and_then(|n| u32::try_from(n).ok());
        let person = |v: &Value| Some(Person { id: whole(v, "id").and_then(|n| u32::try_from(n).ok())?, name: words(v, "name", 16).filter(|n| !n.is_empty()).unwrap_or_else(|| "Someone".into()), car: None, paint: None });
        match kind {
            // From the room itself.
            "you" => {
                let Some(me) = id("id") else { return };
                self.net.me = me;
                let members = v.get("members").and_then(|m| m.as_array()).cloned().unwrap_or_default();
                self.net.people = members.iter().filter_map(person).filter(|p| p.id != me).take(MOST_PEOPLE).collect();
                self.net.state = ON;
                self.say_here(None);
            }
            "join" => {
                let Some(p) = person(&v) else { return };
                if p.id != self.net.me && self.person(p.id).is_none() && self.net.people.len() < MOST_PEOPLE {
                    let to = p.id;
                    self.net.people.push(p);
                    self.say_here(Some(to));
                }
            }
            "left" => {
                let Some(gone) = id("id") else { return };
                if self.net.dare_out.is_some_and(|d| d.0 == gone) {
                    self.net.dare_out = None;
                    self.say("They left.");
                }
                if self.net.dare_in.is_some_and(|d| d.0 == gone) {
                    self.net.dare_in = None;
                }
                if let Some(vs) = self.net.versus.as_mut().filter(|vs| vs.id == gone) {
                    vs.him_gone = true;
                } else {
                    self.net.people.retain(|p| p.id != gone);
                }
            }
            // From one of the others: the room has put who on it.
            _ => {
                let Some(from) = id("from").filter(|f| self.person(*f).is_some()) else { return };
                self.heard(from, kind, &v);
            }
        }
    }

    fn heard(&mut self, from: u32, kind: &str, v: &Value) {
        let racing = self.net.versus.as_ref().is_some_and(|vs| vs.id == from);
        match kind {
            "here" => {
                let car = words(v, "car", 40);
                let paint = words(v, "paint", 12).filter(|p| RAMPS.iter().skip(1).any(|r| r.eq_ignore_ascii_case(p)));
                if let Some(p) = self.net.people.iter_mut().find(|p| p.id == from) {
                    (p.car, p.paint) = (car, paint);
                }
            }
            "dare" => {
                let stake = whole(v, "stake").and_then(|s| i32::try_from(s).ok()).unwrap_or(-1);
                let fair = stake == 0 || self.stakes().contains(&stake);
                let free = self.scene == Scene::Online && self.net.versus.is_none() && self.net.dare_in.is_none() && self.net.dare_out.is_none();
                // Down a road, if it is one this pack has too.
                let road = words(v, "road", 40);
                let fair = fair && road.as_ref().is_none_or(|r| self.pack.manifest.roads.contains_key(r));
                self.net.road_in = road.map(|r| (r, whole(v, "seed").unwrap_or(1) as u32));
                if fair && free && self.state.cash >= stake && self.cannot_race().is_none() {
                    self.net.dare_in = Some((from, stake));
                    self.note = None;
                    tone(880, 120);
                } else {
                    self.send(json!({ "t": "no", "to": from }));
                }
            }
            "ok" => {
                if let Some((_, stake)) = self.net.dare_out.filter(|d| d.0 == from) {
                    self.net.road_now = self.net.road_out.take();
                    self.start_versus(from, stake);
                }
            }
            "no" => {
                if self.net.dare_out.is_some_and(|d| d.0 == from) {
                    self.net.dare_out = None;
                    let name = self.name_of(from);
                    self.say(&format!("{name} passes."));
                }
            }
            "never" => {
                if self.net.dare_in.is_some_and(|d| d.0 == from) {
                    self.net.dare_in = None;
                }
            }
            "staged" if racing => self.net.versus.as_mut().unwrap().him_staged = true,
            "pos" if racing => {
                let (p, speed) = (v.get("p").and_then(|p| p.as_f64()).unwrap_or(0.0), v.get("v").and_then(|p| p.as_f64()).unwrap_or(0.0));
                let x = v.get("x").and_then(|p| p.as_f64()).filter(|x| x.is_finite()).unwrap_or(-0.5);
                if let Some(r) = self.trip.as_mut().and_then(|t| t.rival.as_mut()).filter(|r| r.remote && p.is_finite() && speed.is_finite()) {
                    (r.z, r.speed, r.x) = (p.clamp(0.0, 40000.0), speed.clamp(0.0, 120.0), x.clamp(-2.2, 2.2));
                }
                if p.is_finite() && speed.is_finite() {
                    if let Some(race) = self.race.as_mut() {
                        race.rival_pos = p.clamp(0.0, QUARTER * 3.0);
                    }
                    self.net.versus.as_mut().unwrap().him_v = speed.clamp(0.0, 200.0);
                }
            }
            "slip" if racing => {
                let n = |key: &str| whole(v, key).and_then(|n| i32::try_from(n).ok()).unwrap_or(0).clamp(0, 600_000);
                let slip = Slip { rt: n("rt"), et: n("et"), red: v.get("red").and_then(|r| r.as_bool()).unwrap_or(false) };
                let vs = self.net.versus.as_mut().unwrap();
                vs.him = Some(slip);
                vs.him_v = 0.0;
            }
            "quit" if racing => self.net.versus.as_mut().unwrap().him_gone = true,
            _ => {}
        }
    }

    fn start_versus(&mut self, id: u32, stake: i32) {
        if let Some((road, seed)) = self.net.road_now.take() {
            return self.start_versus_road(id, stake, &road, seed);
        }
        let Some((o, car)) = self.owned().zip(self.car()) else { return };
        let (gain, boost) = self.tune(o);
        let drive = self.tuned_drive(o, car);
        self.burn(0.5);
        let mut race = Race::new(&drive, gain, boost, 0, 0);
        race.net = true;
        self.race = Some(race);
        self.net.dare_in = None;
        self.net.dare_out = None;
        self.net.versus = Some(Versus { id, stake, me_staged: false, him_staged: false, him: None, him_gone: false, result: None, him_v: 0.0, sent_slip: false, waited: 0, road: None, mine: 0 });
        self.note = None;
        self.scene = Scene::Race;
    }

    /// Down a road with someone: each drives it in their own engine, among the same other cars,
    /// and the quicker time wins. They are shown where they last said they were.
    fn start_versus_road(&mut self, id: u32, stake: i32, road: &str, seed: u32) {
        let Some(r) = self.pack.manifest.roads.get(road) else { return };
        let mut r = r.clone();
        r.cops = 0;
        let mut trip = crate::road::Trip::new(road, &r, false, seed);
        trip.pace = self.pace();
        let mut them = crate::road::Rival::new(14.0, self.person(id).and_then(|p| p.paint.as_deref()).unwrap_or("RED"));
        them.remote = true;
        trip.rival = Some(them);
        // Nobody moves until both are on the line; then it is counted down.
        trip.hold = i32::MAX / 2;
        self.trip = Some(trip);
        self.trip_from = None;
        self.net.dare_in = None;
        self.net.dare_out = None;
        self.net.versus = Some(Versus { id, stake, me_staged: true, him_staged: false, him: None, him_gone: false, result: None, him_v: 0.0, sent_slip: false, waited: 0, road: Some(road.to_string()), mine: 0 });
        self.send(json!({ "t": "staged", "to": id }));
        self.note = None;
        self.scene = Scene::Road;
    }

    /// A thirtieth of a second of a race down a road with someone.
    pub(super) fn versus_road_tick(&mut self, keys: u32, hit: u32) {
        let Some((trip, vs)) = self.trip.as_mut().zip(self.net.versus.as_mut()) else { return };
        let Some(road) = self.pack.manifest.roads.get(&trip.road) else { return };
        let to = vs.id;
        if hit & KEY_B != 0 && vs.result.is_none() {
            tone(0, 0);
            self.send(json!({ "t": "quit", "to": to }));
            return self.finish_versus(Outcome::Lost, "You backed out. Pay the man.");
        }
        if vs.him_gone && vs.him.is_none() {
            tone(0, 0);
            return self.finish_versus(Outcome::Won, "They are gone. That is a win.");
        }
        if vs.him_staged && trip.hold > 90 {
            trip.hold = 90;
        }
        let mut out: Vec<Value> = Vec::new();
        let (mut crashed, mut went, mut dry) = (false, 0.0, false);
        if !vs.sent_slip {
            let counting = trip.hold;
            trip.spray = 0.0;
            let h = trip.tick(road, Pedals { go: keys & KEY_UP != 0, stop: keys & KEY_DOWN != 0, left: keys & KEY_LEFT != 0, right: keys & KEY_RIGHT != 0 });
            (crashed, went) = (h.crashed, h.went);
            if h.crashed {
                tone(70, 500);
            } else if counting > 0 && counting <= 90 {
                if counting % 30 == 0 {
                    tone(440, 120);
                } else if counting == 1 {
                    tone(880, 250);
                }
            } else if trip.crash == 0 && counting == 0 {
                let v = (trip.speed * 10.0) as i32;
                tone(if trip.speed > 0.0 || keys & KEY_UP != 0 { 60 + v % 110 + v / 110 * 12 } else { 45 }, 60);
            }
            if counting == 0 && self.ticks % 5 == 0 {
                out.push(json!({ "t": "pos", "to": to, "p": (trip.z * 10.0).round() / 10.0, "v": (trip.speed * 10.0).round() / 10.0, "x": (trip.x * 100.0).round() / 100.0 }));
            }
            if h.arrived {
                // Over the line: what it took goes to them, and theirs is waited for.
                vs.sent_slip = true;
                vs.mine = (trip.clock as f64 * 1000.0 / 30.0) as i32;
                trip.speed = 0.0;
                tone(990, 200);
                out.push(json!({ "t": "slip", "to": to, "rt": 0, "et": vs.mine, "mph": 0, "red": false }));
            }
            dry = went > 0.0 && self.state.cur.and_then(|i| self.state.cars.get(i)).is_some_and(|o| o.gas <= went / 1000.0 * road.gas_per_km);
        } else if vs.result.is_none() {
            vs.waited += 1;
            let me = Slip { rt: 0, et: vs.mine, red: false };
            vs.result = match vs.him {
                Some(him) => Some(decide(me, him)),
                // Gone, or four minutes behind: the road was yours.
                None if vs.him_gone || vs.waited > PATIENCE * 6 => Some(Outcome::Won),
                None => None,
            };
        }
        let per_km = road.gas_per_km;
        let done = (hit & KEY_A != 0).then_some(vs.result).flatten();
        let stake = vs.stake;
        for message in out {
            self.send(message);
        }
        self.burn(went / 1000.0 * per_km);
        if crashed {
            if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
                o.cond = (o.cond - 3).max(25);
            }
        }
        if dry {
            tone(0, 0);
            self.send(json!({ "t": "quit", "to": to }));
            return self.finish_versus(Outcome::Lost, "Out of gas. That is their race.");
        }
        if let Some(outcome) = done {
            tone(0, 0);
            let why = match (outcome, stake) {
                (Outcome::Won, 0) => "You won. It was for kicks.".to_string(),
                (Outcome::Won, s) => format!("You won ${s}."),
                (Outcome::Lost, 0) => "You lost. It was for kicks.".to_string(),
                (Outcome::Lost, s) => format!("You lost ${s}."),
                (Outcome::NoRace, _) => "A dead heat. Nobody pays.".to_string(),
            };
            self.finish_versus(outcome, &why);
        }
    }

    /// What is said across the windscreen in a race down a road with someone.
    pub(super) fn versus_road_says(&self) -> Option<&'static str> {
        let (vs, trip) = self.net.versus.as_ref().zip(self.trip.as_ref())?;
        vs.road.as_ref()?;
        Some(match vs.result {
            Some(Outcome::Won) => "YOU WIN. ENTER.",
            Some(Outcome::Lost) => "YOU LOSE. ENTER.",
            Some(Outcome::NoRace) => "A DEAD HEAT. ENTER.",
            None if vs.sent_slip => "THERE. WAITING FOR THEM.",
            None if trip.hold > 90 => "WAITING FOR THEM TO LINE UP",
            None => return None,
        })
    }

    /// The roads a dare can be down, in the pack's order.
    pub(super) fn courses(&self) -> Vec<(&String, &crate::pack::Road)> {
        self.pack.manifest.roads.iter().collect()
    }

    /// The race is over: the money moves, two hours are gone, and you are back among the others.
    fn finish_versus(&mut self, outcome: Outcome, why: &str) {
        let Some(vs) = self.net.versus.take() else { return };
        self.race = None;
        self.trip = None;
        self.scene = Scene::Online;
        match outcome {
            Outcome::Won => self.state.cash += vs.stake,
            Outcome::Lost => self.state.cash -= vs.stake,
            Outcome::NoRace => {}
        }
        if vs.him_gone {
            self.net.people.retain(|p| p.id != vs.id);
        }
        self.say(why);
        self.spend(2);
        if self.scene != Scene::Online {
            // The season ended on it.
            self.net = Net::default();
        }
    }

    /// The meet: a list of who is here. Pick someone to dare them; answer a dare with Enter or Escape.
    pub(super) fn online_tick(&mut self, pick: bool, back: bool) {
        if let Some((from, stake)) = self.net.dare_in {
            if pick {
                self.send(json!({ "t": "ok", "to": from }));
                self.net.road_now = self.net.road_in.take();
                self.start_versus(from, stake);
            } else if back {
                self.send(json!({ "t": "no", "to": from }));
                self.net.dare_in = None;
            }
            return;
        }
        if let Some((to, _)) = self.net.dare_out {
            if back {
                self.send(json!({ "t": "never", "to": to }));
                self.net.dare_out = None;
                self.say("Never mind, then.");
            }
            return;
        }
        if back {
            return self.leave_online();
        }
        if pick && self.net.state == ON {
            if let Some(p) = self.row(8, 312, 40, 14, self.net.people.len()).and_then(|i| self.net.people.get(i)) {
                if p.car.is_none() {
                    return self.say("They are still pulling in.");
                }
                self.net.picked = Some(p.id);
                self.note = None;
                self.scene = Scene::OnlineBets;
            }
        }
    }

    pub(super) fn online_bets_tick(&mut self, pick: bool, back: bool) {
        let stakes = self.stakes();
        let Some(to) = self.net.picked.filter(|id| self.person(*id).is_some()) else {
            self.scene = Scene::Online;
            return;
        };
        if back {
            self.scene = Scene::Online;
        } else if pick && (150..166).contains(&self.py) && !self.pack.manifest.roads.is_empty() {
            // Where: the strip, or one of the roads, round and round.
            self.net.course = (self.net.course + 1) % (self.pack.manifest.roads.len() + 1);
            tone(520, 50);
        } else if pick {
            if let Some(b) = self.button(stakes.len() + 1) {
                let stake = if b == 0 { 0 } else { stakes[b - 1] };
                self.scene = Scene::Online;
                if self.state.cash < stake {
                    return self.no("You don't have that kind of dough.");
                }
                self.net.dare_out = Some((to, stake));
                let road = self.net.course.checked_sub(1).and_then(|i| self.pack.manifest.roads.keys().nth(i)).cloned();
                let seed = self.rand() & 0xFFFFF;
                self.net.road_out = road.clone().map(|r| (r, seed));
                match road {
                    Some(road) => self.send(json!({ "t": "dare", "to": to, "stake": stake, "road": road, "seed": seed })),
                    None => self.send(json!({ "t": "dare", "to": to, "stake": stake })),
                }
                self.note = None;
            }
        }
    }

    /// A thirtieth of a second of a race with someone.
    pub(super) fn versus_tick(&mut self, keys: u32, hit: u32) {
        let Some((race, vs)) = self.race.as_mut().zip(self.net.versus.as_mut()) else { return };
        let to = vs.id;
        let mut out: Vec<Value> = Vec::new();
        // Backing out is losing, and so is the other one backing out winning.
        if hit & KEY_B != 0 {
            tone(0, 0);
            self.send(json!({ "t": "quit", "to": to }));
            return self.finish_versus(Outcome::Lost, "You backed out. Pay the man.");
        }
        if vs.him_gone && vs.him.is_none() {
            tone(0, 0);
            return self.finish_versus(Outcome::Won, "They are gone. That is a win.");
        }
        if race.phase == Phase::Stage && hit & KEY_A != 0 && !vs.me_staged {
            vs.me_staged = true;
            out.push(json!({ "t": "staged", "to": to }));
        }
        let was = race.phase;
        // The tree comes down when both are staged.
        let stage = race.phase == Phase::Stage && vs.me_staged && vs.him_staged;
        race.tick(Controls { throttle: keys & KEY_UP != 0, stage, up: hit & KEY_X != 0, down: hit & KEY_Y != 0, nitrous: false });
        // Between hearing where they are, they carry on as fast as they were going.
        race.rival_pos += vs.him_v / crate::race::HZ;
        if matches!(race.phase, Phase::Tree | Phase::Run) && self.ticks % 5 == 0 {
            out.push(json!({ "t": "pos", "to": to, "p": (race.pos * 10.0).round() / 10.0, "v": (race.v * 10.0).round() / 10.0 }));
        }
        if race.phase == Phase::Slip && race.blown {
            // Blown up and rolled to a stop: that is their race.
            tone(0, 0);
            self.send(json!({ "t": "quit", "to": to }));
            self.finish_versus(Outcome::Lost, "");
            return self.blew();
        }
        if race.phase == Phase::Slip {
            let me = Slip { rt: race.rt, et: race.et, red: race.red };
            if !vs.sent_slip {
                vs.sent_slip = true;
                out.push(json!({ "t": "slip", "to": to, "rt": me.rt, "et": me.et, "mph": race.mph, "red": me.red }));
            }
            if vs.result.is_none() {
                vs.waited += 1;
                vs.result = match vs.him {
                    Some(him) => Some(decide(me, him)),
                    // Gone, or never came down: the lane was yours.
                    None if vs.him_gone || vs.waited > PATIENCE => Some(if me.red { Outcome::NoRace } else { Outcome::Won }),
                    None => None,
                };
                race.won = vs.result == Some(Outcome::Won);
            }
        }
        if was != Phase::Slip && race.phase == Phase::Slip {
            tone(990, 200);
        } else if was == Phase::Stage && race.phase == Phase::Tree {
            tone(660, 60);
        } else if race.phase != Phase::Slip || race.v > 0.0 {
            tone((race.rpm / 40.0) as i32, 60);
        }
        let done = (race.phase == Phase::Slip && hit & KEY_A != 0).then_some(vs.result).flatten();
        let stake = vs.stake;
        for message in out {
            self.send(message);
        }
        if let Some(outcome) = done {
            tone(0, 0);
            let why = match (outcome, stake) {
                (Outcome::Won, 0) => "You won. It was for kicks.".to_string(),
                (Outcome::Won, s) => format!("You won ${s}."),
                (Outcome::Lost, 0) => "You lost. It was for kicks.".to_string(),
                (Outcome::Lost, s) => format!("You lost ${s}."),
                (Outcome::NoRace, _) => "No race. Nobody pays.".to_string(),
            };
            self.finish_versus(outcome, &why);
        }
    }

    /// What the race says of the other lane while it is someone real: staging, and the verdict.
    pub(super) fn versus_says(&self) -> Option<(&'static str, Option<String>)> {
        let vs = self.net.versus.as_ref()?;
        let race = self.race.as_ref()?;
        let say = match race.phase {
            Phase::Stage if !vs.me_staged => "ENTER TO STAGE",
            Phase::Stage => "WAITING FOR THEM TO STAGE",
            Phase::Slip => match vs.result {
                None => "WAITING FOR THEM",
                Some(Outcome::Won) => "YOU WIN.",
                Some(Outcome::NoRace) => "NO RACE.",
                Some(Outcome::Lost) if race.red => "RED LIGHT.",
                Some(Outcome::Lost) => "YOU LOSE.",
            },
            _ => "",
        };
        let theirs = vs.him.map(|h| if h.red { "RED".to_string() } else { ms(h.rt + h.et) });
        Some((say, theirs))
    }

    pub(super) fn draw_online(&self, s: &mut Screen) {
        s.vgrad(0, 0, 320, 200, ramp("NAVY"), 3, 12);
        s.text_big(8, 8, "WHO IS HERE", shade(ramp("GOLD"), 27), 2);
        s.rect(8, 30, 304, 1, shade(ramp("CHROME"), 12));
        let dim = shade(ramp("CHROME"), 18);
        match self.net.state {
            ON if self.net.people.is_empty() => {
                s.text_sh(8, 44, "NOBODY ELSE YET.", WHITE, BLACK);
                s.text(8, 58, "ANYONE WHO COMES TO THE MEET", dim);
                s.text(8, 68, "WHILE YOU WAIT SHOWS UP HERE.", dim);
            }
            ON => {
                let over = self.row(8, 312, 40, 14, self.net.people.len());
                for (i, p) in self.net.people.iter().enumerate() {
                    let y = 40 + i as i32 * 14;
                    if over == Some(i) {
                        s.rect(8, y - 3, 304, 14, shade(ramp("NAVY"), 18));
                    }
                    let car = p.car.as_ref().map(|id| self.pack.manifest.cars.get(id).map_or("SOMETHING OF THEIR OWN".to_string(), |c| c.name.to_uppercase()));
                    s.rect(10, y - 1, 12, 9, BLACK);
                    s.vgrad(11, y, 10, 7, ramp(p.paint.as_deref().unwrap_or("GREY")), 27, 16);
                    s.text_sh(28, y, &p.name.to_uppercase(), WHITE, BLACK);
                    s.text(164, y, &car.unwrap_or_else(|| "PULLING IN".into()).chars().take(18).collect::<String>(), dim);
                }
            }
            LOST => {
                s.text_sh(8, 44, "NOBODY CAN BE REACHED FROM HERE.", shade(ramp("RED"), 26), BLACK);
                s.text(8, 58, "THE MEET NEEDS A PARLOR TO HOLD IT.", dim);
            }
            _ => {
                s.text_sh(8, 44, "LOOKING FOR THE OTHERS...", WHITE, BLACK);
            }
        }
        let money = |stake: i32| if stake == 0 { "for kicks".to_string() } else { format!("for ${stake}") };
        if self.scene == Scene::OnlineBets {
            let mut labels = vec!["KICKS".to_string()];
            labels.extend(self.stakes().iter().map(|b| format!("${b}")));
            self.buttons(s, &labels);
            if !self.pack.manifest.roads.is_empty() {
                let on = (150..166).contains(&self.py);
                let place = self.net.course.checked_sub(1).and_then(|i| self.courses().get(i).map(|c| c.1.title.to_uppercase())).unwrap_or_else(|| "THE STRIP".into());
                s.vgrad(4, 150, 312, 16, ramp(if on { "GOLD" } else { "NAVY" }), if on { 26 } else { 12 }, if on { 18 } else { 5 });
                s.frame(4, 150, 312, 16, shade(ramp("CHROME"), if on { 28 } else { 14 }));
                s.text(10, 154, &format!("WHERE: {place}. PICK TO CHANGE.").chars().take(37).collect::<String>(), if on { BLACK } else { WHITE });
            }
            let name = self.net.picked.map(|id| self.name_of(id)).unwrap_or_default();
            self.panel(s, &name, "What are we racing for?", "Escape: never mind");
        } else if let Some((from, stake)) = self.net.dare_in {
            let foot = match self.net.road_in.as_ref().and_then(|r| self.pack.manifest.roads.get(&r.0)) {
                Some(road) => format!("Down {}. Enter: yes. Esc: no.", road.title).chars().take(38).collect::<String>(),
                None => "Enter takes it. Escape passes.".to_string(),
            };
            self.panel(s, &self.name_of(from), &format!("Dares you, {}.", money(stake)), &foot);
        } else if let Some((to, stake)) = self.net.dare_out {
            self.panel(s, &self.name_of(to), &format!("You dared them, {}.", money(stake)), "Waiting. Escape calls it off.");
        } else {
            self.panel(s, "The meet", self.note.as_deref().unwrap_or("Pick someone to race them."), "Escape goes back to the lot.");
        }
    }
}
