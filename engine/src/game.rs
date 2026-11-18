//! The game: a season of days and hours, a pointer, the place you are in, and what picking things
//! does.
//!
//! Nothing here knows what a garage is, or who the King is. A place is whatever the pack says it
//! is; so are the cars, the parts in the paper, and the people you race. What is here is the
//! rules: that things cost money and hours, how a bet is made and settled, and how it ends.

use crate::gfx::{ramp, shade, Screen, Tint, BLACK, WHITE};
use crate::host::tone;
use crate::pack::{Boost, Car, Pack, Part, Place, Rival, Spot};
use crate::race::{Controls, Phase, Race, QUARTER};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const KEY_UP: u32 = 1;
pub const KEY_DOWN: u32 = 2;
pub const KEY_LEFT: u32 = 4;
pub const KEY_RIGHT: u32 = 8;
/// Enter, Space or the mouse button: yes, pick.
pub const KEY_A: u32 = 16;
/// Escape: back.
pub const KEY_B: u32 = 32;
/// E and Q: up a gear and down one.
pub const KEY_X: u32 = 64;
pub const KEY_Y: u32 = 128;

/// The most cars you can keep.
pub const MOST_CARS: usize = 16;
const PAINTS: [&str; 8] = ["BLUE", "RED", "GOLD", "GREEN", "SILVER", "WHITE", "VIOLET", "TEAL"];
const BETS: [i32; 4] = [10, 25, 50, 100];

/// A car of yours: which one, its paint, how healthy its engine is as a percentage, and what is
/// bolted to it, by slot.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Owned {
    pub car: String,
    pub paint: Option<String>,
    pub cond: i32,
    pub fitted: BTreeMap<String, String>,
}

/// Everything that is kept when the game is saved.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct State {
    /// The pack this game belongs to; a game saved in another pack is not this one.
    pub pack: String,
    pub place: String,
    pub cash: i32,
    /// Hours of the season gone.
    pub hours: i32,
    /// How many of the rivals you have beaten, in order.
    pub rung: usize,
    pub cars: Vec<Owned>,
    /// Which of them is in the bay.
    pub cur: Option<usize>,
    /// Parts you own that are not on a car.
    pub spares: Vec<String>,
    /// The day, and which of that day's cars in the paper have gone.
    pub sold: (i32, Vec<usize>),
}

/// A car in today's paper.
pub struct Offer {
    pub car: String,
    pub paint: String,
    pub cond: i32,
    pub price: i32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Scene {
    Title,
    Place,
    /// The paper: page 0 is cars, 1 is parts.
    Paper(usize),
    /// The bench: what is on the car and what is on the shelf.
    Bench,
    Yard,
    Bets,
    Race,
    /// 1 the season ran out, 2 broke, 3 you beat the one to beat.
    Over(u8),
}

pub struct Game {
    pub pack: Pack,
    /// What is wrong with the pack, if it could not be read.
    pub problem: Option<String>,
    pub scene: Scene,
    pub state: State,
    pub lamps: bool,
    pub note: Option<String>,
    pub px: i32,
    pub py: i32,
    /// Who has pulled in, what the race is for (0 kicks, then the pack's bets, last pinks), and the race.
    pub who: usize,
    bet: usize,
    pub race: Option<Race>,
    selling: bool,
    saved: bool,
    rng: u32,
    last_under: Option<String>,
    last_keys: u32,
    last_mouse: (i32, i32),
    pub ticks: u32,
}

fn hash(n: u32) -> u32 {
    let mut h = n.wrapping_mul(2654435761);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^ (h >> 15)
}

fn cond_word(cond: i32) -> &'static str {
    if cond < 62 { "TIRED" } else if cond < 72 { "RUNS OK" } else { "STRONG" }
}

fn ms(t: i32) -> String {
    format!("{}.{:03}", t / 1000, t % 1000)
}

impl Game {
    pub fn new(pack: Result<Pack, String>) -> Game {
        let (pack, problem) = match pack {
            Ok(p) => (p, None),
            Err(e) => (Pack::default(), Some(e)),
        };
        let mut g = Game {
            pack, problem, scene: Scene::Title, state: State::default(), lamps: false, note: None, px: 160, py: 100,
            who: 0, bet: 0, race: None, selling: false, saved: false, rng: 1, last_under: None, last_keys: 0, last_mouse: (160, 100), ticks: 0,
        };
        g.state = g.fresh();
        g.saved = g.kept().is_some();
        g
    }

    pub fn seed(&mut self, seed: u32) {
        self.rng = seed | 1;
    }

    fn rand(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    /// A new game, as the pack starts it.
    fn fresh(&self) -> State {
        let m = &self.pack.manifest;
        let cars: Vec<Owned> = m.start.car.iter().filter_map(|id| m.cars.get(id).map(|c| Owned { car: id.clone(), paint: c.paint.clone(), cond: 70, fitted: BTreeMap::new() })).collect();
        State { pack: m.name.clone(), place: m.start.place.clone(), cash: m.start.cash, cur: (!cars.is_empty()).then_some(0), cars, ..State::default() }
    }

    /// The saved game, if there is one and it is this pack's and still makes sense in it.
    fn kept(&self) -> Option<State> {
        let m = &self.pack.manifest;
        let s: State = serde_json::from_slice(&crate::host::fetch()).ok()?;
        let fits = s.pack == m.name
            && m.places.contains_key(&s.place)
            && s.cars.iter().all(|c| m.cars.contains_key(&c.car) && c.fitted.values().all(|p| m.parts.contains_key(p)))
            && s.spares.iter().all(|p| m.parts.contains_key(p))
            && s.cur.is_none_or(|i| i < s.cars.len())
            && s.rung <= m.rivals.len();
        fits.then_some(s)
    }

    fn save(&self) {
        if let Ok(bytes) = serde_json::to_vec(&self.state) {
            crate::host::store(&bytes);
        }
    }

    // ---- What you have ----

    pub fn owned(&self) -> Option<&Owned> {
        self.state.cur.and_then(|i| self.state.cars.get(i))
    }

    /// The car in the bay, as the pack describes it.
    pub fn car(&self) -> Option<&Car> {
        self.owned().and_then(|o| self.pack.manifest.cars.get(&o.car))
    }

    pub fn day(&self) -> i32 {
        self.state.hours / self.pack.manifest.season.hours_per_day + 1
    }

    fn hours_left(&self) -> i32 {
        let per = self.pack.manifest.season.hours_per_day;
        per - self.state.hours % per
    }

    /// Every slot there is a part for, in order.
    pub fn slots(&self) -> Vec<String> {
        let mut slots: Vec<String> = self.pack.manifest.parts.values().map(|p| p.slot.clone()).collect();
        slots.sort();
        slots.dedup();
        slots
    }

    fn part(&self, id: &str) -> Option<&Part> {
        self.pack.manifest.parts.get(id)
    }

    fn has_part(&self, id: &str) -> bool {
        self.state.spares.iter().any(|p| p == id) || self.state.cars.iter().any(|c| c.fitted.values().any(|p| p == id))
    }

    /// What the parts on a car do: torque as a fraction of stock, and the turbo fitted, if one is.
    pub fn tune(&self, o: &Owned) -> (f64, Option<Boost>) {
        let parts: Vec<&Part> = o.fitted.values().filter_map(|id| self.part(id)).collect();
        let more: i32 = parts.iter().map(|p| p.torque).sum();
        ((100 + more) as f64 / 100.0 * o.cond as f64 / 100.0, parts.iter().find_map(|p| p.boost.clone()))
    }

    /// What a car like this one costs in the paper, and what the dealer gives for yours.
    fn list_price(&self, car: &str, cond: i32) -> i32 {
        self.pack.manifest.cars.get(car).and_then(|c| c.price).unwrap_or(300) * cond / 70
    }

    pub fn value(&self, o: &Owned) -> i32 {
        self.list_price(&o.car, o.cond) * 6 / 10 + o.fitted.values().filter_map(|p| self.part(p)).map(|p| p.price / 2).sum::<i32>()
    }

    /// What they give today for a part off the shelf: 70 to 89 hundredths of what it cost.
    pub fn spare_offer(&self, id: &str) -> i32 {
        let n = self.pack.manifest.parts.keys().position(|k| k == id).unwrap_or(0) as u32;
        self.part(id).map(|p| p.price).unwrap_or(0) * (70 + (hash(self.day() as u32 * 16 + n + 3) >> 12) as i32 % 20) / 100
    }

    /// Today's cars in the paper: three, picked by the day from the cars that have a price.
    pub fn offers(&self) -> Vec<Option<Offer>> {
        let m = &self.pack.manifest;
        let sale: Vec<(&String, &Car)> = m.cars.iter().filter(|(_, c)| c.price.is_some()).collect();
        if sale.is_empty() {
            return Vec::new();
        }
        let paints: Vec<String> = if m.paints.is_empty() { PAINTS.iter().map(|p| p.to_string()).collect() } else { m.paints.clone() };
        let day = self.day();
        (0..3usize).map(|i| {
            if self.state.sold.0 == day && self.state.sold.1.contains(&i) {
                return None;
            }
            let h = hash(day as u32 * 3 + i as u32 + 1);
            let (id, car) = sale[(h >> 4) as usize % sale.len()];
            let cond = 55 + (h >> 8) as i32 % 26;
            let price = self.list_price(id, cond) + (h >> 16) as i32 % 40;
            let paint = if car.paint_ramps.mid.is_some() { paints[h as usize % paints.len()].clone() } else { car.paint.clone().unwrap_or_default() };
            Some(Offer { car: id.clone(), paint, cond, price })
        }).collect()
    }

    pub fn rival(&self) -> Option<&Rival> {
        self.pack.manifest.rivals.get(self.who)
    }

    /// What can be raced for: nothing, each of the pack's bets, and cars.
    fn stakes(&self) -> Vec<i32> {
        let m = &self.pack.manifest;
        if m.bets.is_empty() { BETS.to_vec() } else { m.bets.clone() }
    }

    // ---- Time and endings ----

    /// Everything takes time. When the days run out, or there is no way to get a car, it is over.
    fn spend(&mut self, hours: i32) {
        self.state.hours += hours;
        self.check_over();
        self.save();
    }

    fn check_over(&mut self) {
        let m = &self.pack.manifest;
        if self.day() > m.season.days {
            self.scene = Scene::Over(1);
        } else if self.state.cars.is_empty() {
            let cheapest = m.cars.keys().filter(|id| m.cars[*id].price.is_some()).map(|id| self.list_price(id, 55)).min();
            let spare: i32 = self.state.spares.iter().map(|p| self.spare_offer(p)).sum();
            if cheapest.is_none_or(|c| self.state.cash + spare < c) {
                self.scene = Scene::Over(2);
            }
        }
    }

    // ---- Places ----

    fn here(&self) -> Option<&Place> {
        self.pack.manifest.places.get(&self.state.place)
    }

    /// The spots of this place that are there just now, the last of them on top.
    fn spots(&self) -> Vec<&Spot> {
        self.here().map(|p| p.spots.iter().filter(|s| !s.needs_car || self.state.cur.is_some()).collect()).unwrap_or_default()
    }

    fn spot_under(&self) -> Option<&Spot> {
        self.spots().into_iter().rev().find(|s| {
            let [x, y, w, h] = s.rect;
            self.px >= x && self.px < x + w && self.py >= y && self.py < y + h
        })
    }

    fn enter(&mut self, place: &str) {
        self.state.place = place.to_string();
        self.scene = Scene::Place;
        if self.here().is_some_and(|p| p.rival_at.is_some()) {
            self.who = self.state.rung.min(self.pack.manifest.rivals.len().saturating_sub(1));
        }
        self.save();
    }

    fn say(&mut self, words: &str) {
        self.note = Some(words.to_string());
    }

    fn no(&mut self, words: &str) {
        self.say(words);
        tone(120, 80);
    }

    fn act(&mut self, action: &str) {
        self.note = None;
        if let Some(to) = action.strip_prefix("go:") {
            tone(520, 60);
            self.enter(&to.to_string());
        } else if let Some(words) = action.strip_prefix("say:") {
            self.say(words);
        } else {
            match action {
                "lamps" => {
                    self.lamps = !self.lamps;
                    tone(if self.lamps { 440 } else { 330 }, 70);
                }
                "calendar" => {
                    let (day, days, left) = (self.day(), self.pack.manifest.season.days, self.hours_left());
                    self.say(&format!("Day {day} of {days}. {left} hours left today."));
                }
                "paper" => {
                    // Reading the paper takes an hour.
                    self.spend(1);
                    if self.scene == Scene::Place {
                        self.scene = Scene::Paper(0);
                    }
                }
                "parts" => {
                    self.scene = Scene::Bench;
                    self.selling = false;
                }
                "yard" => self.scene = Scene::Yard,
                "race" => {
                    if self.state.cur.is_none() {
                        self.no("You have nothing to race.");
                    } else {
                        self.scene = Scene::Bets;
                    }
                }
                "next" => {
                    // Someone else: anyone you have already beaten, or the one you have not.
                    let top = self.state.rung.min(self.pack.manifest.rivals.len().saturating_sub(1));
                    if top > 0 {
                        self.who = (self.who + 1 + self.rand() as usize % top) % (top + 1);
                    }
                    tone(300, 60);
                }
                "solo" => {
                    if self.state.cur.is_none() {
                        self.no("You have nothing to run.");
                    } else {
                        self.start_race(0);
                    }
                }
                _ => {}
            }
        }
    }

    // ---- Racing ----

    fn start_race(&mut self, rival_et: i32) {
        let Some((o, car)) = self.owned().zip(self.car()) else { return };
        let (gain, boost) = self.tune(o);
        let drive = car.drive.clone();
        let rt = 280 + (self.rand() % 260) as i32;
        self.race = Some(Race::new(&drive, gain, boost, rival_et, rt));
        self.scene = Scene::Race;
    }

    /// Will they take the bet? Kicks, always. Money, up to what they are good for. Their car, only
    /// if they race for cars and think they have you; the one to beat races for nothing else.
    fn offer_bet(&mut self, b: usize) {
        let stakes = self.stakes();
        let pinks = b == stakes.len() + 1;
        let Some(r) = self.rival().cloned() else { return };
        let stake = if b == 0 || pinks { 0 } else { stakes[b - 1] };
        if r.king && !pinks {
            return self.no("Pink slips or go home.");
        }
        if self.state.cash < stake {
            return self.no("You don't have that kind of dough.");
        }
        if stake > r.limit || (pinks && !r.king && (!r.pinks || self.rand() % 3 == 0)) {
            return self.no("No way. Too rich for me.");
        }
        self.bet = b;
        self.start_race((r.et * 1000.0) as i32);
    }

    /// The race is run. `won` is None for a pass with nobody in the other lane.
    fn settle(&mut self, won: Option<bool>) {
        self.race = None;
        self.scene = Scene::Place;
        let Some(won) = won else {
            self.say("A pass on your own. Nobody saw it.");
            return self.spend(1);
        };
        let Some(r) = self.rival().cloned() else { return };
        let stakes = self.stakes();
        let pinks = self.bet == stakes.len() + 1;
        let stake = if self.bet == 0 || pinks { 0 } else { stakes[self.bet - 1] };
        if won {
            if pinks && r.king {
                self.scene = Scene::Over(3);
                return self.save();
            }
            if pinks {
                let theirs = Owned { car: r.car.clone(), paint: r.paint.clone(), cond: (62 + self.who as i32).min(100), fitted: BTreeMap::new() };
                if self.state.cars.len() < MOST_CARS {
                    self.state.cars.push(theirs);
                    self.say("You won his car. It is in your yard.");
                } else {
                    self.state.cash += self.value(&theirs);
                    self.say("No room. The dealer took his car.");
                }
            } else {
                self.state.cash += stake;
                self.say(if stake > 0 { "Pay up. He did." } else { "You won. It was for kicks." });
            }
            // Beating the next one up brings the one after out. Kicks do not count.
            if self.who == self.state.rung && self.bet > 0 && self.state.rung + 1 < self.pack.manifest.rivals.len() {
                self.state.rung += 1;
                self.who = self.state.rung;
                if !pinks {
                    self.say("Paid. Word gets round. Here is another.");
                }
            }
        } else if pinks {
            // "Take a walk, sucker." What was bolted to it goes with it.
            if let Some(i) = self.state.cur.take() {
                self.state.cars.remove(i);
            }
            self.lamps = false;
            let home = self.pack.manifest.start.place.clone();
            self.enter(&home);
            self.say("He took your car. You walked home.");
        } else {
            self.state.cash -= stake;
            self.say(if stake > 0 { "You lose, sucker. Pay the man." } else { "You lost. It was for kicks." });
        }
        self.spend(2);
    }

    // ---- One thirtieth of a second ----

    /// Which row of a list the pointer is on: rows `h` tall from `y0`, between `x0` and `x1`.
    fn row(&self, x0: i32, x1: i32, y0: i32, h: i32, rows: usize) -> Option<usize> {
        (self.px >= x0 && self.px < x1 && self.py >= y0 && self.py < y0 + h * rows as i32).then(|| ((self.py - y0) / h) as usize)
    }

    /// Which of `n` buttons along the bottom of a place the pointer is on.
    fn button(&self, n: usize) -> Option<usize> {
        let w = 312 / n as i32;
        (self.py >= 178 && self.py < 196 && self.px >= 4 && self.px < 4 + w * n as i32).then(|| ((self.px - 4) / w) as usize)
    }

    pub fn tick(&mut self, keys: u32, mx: i32, my: i32) {
        let hit = keys & !self.last_keys;
        self.last_keys = keys;
        self.ticks += 1;
        if self.problem.is_some() {
            return;
        }
        if self.scene == Scene::Race {
            return self.race_tick(keys, hit);
        }
        if (mx, my) != self.last_mouse {
            self.last_mouse = (mx, my);
            self.px = mx;
            self.py = my;
        }
        let step = 3;
        if keys & KEY_LEFT != 0 { self.px -= step; }
        if keys & KEY_RIGHT != 0 { self.px += step; }
        if keys & KEY_UP != 0 { self.py -= step; }
        if keys & KEY_DOWN != 0 { self.py += step; }
        self.px = self.px.clamp(0, 319);
        self.py = self.py.clamp(0, 198);
        let (pick, back) = (hit & KEY_A != 0, hit & KEY_B != 0);
        match self.scene {
            Scene::Title => {
                if pick || hit & KEY_X != 0 {
                    self.state = if pick && self.saved { self.kept().unwrap_or_else(|| self.fresh()) } else { self.fresh() };
                    self.note = None;
                    self.lamps = false;
                    tone(880, 80);
                    let place = self.state.place.clone();
                    self.enter(&place);
                    self.check_over();
                }
            }
            Scene::Over(_) => {
                if pick {
                    crate::host::store(b"{}");
                    self.saved = false;
                    self.state = self.fresh();
                    self.scene = Scene::Title;
                }
            }
            Scene::Place => {
                // What was last said stays until the pointer finds something else to talk about.
                let under = self.spot_under().map(|s| s.label.clone());
                if under != self.last_under {
                    if under.is_some() {
                        self.note = None;
                    }
                    self.last_under = under;
                }
                if back {
                    self.save();
                    self.saved = true;
                    self.scene = Scene::Title;
                } else if pick {
                    if let Some(action) = self.spot_under().map(|s| s.action.clone()) {
                        self.act(&action);
                    }
                }
            }
            Scene::Paper(page) => {
                if back {
                    self.scene = Scene::Place;
                    self.note = None;
                } else if pick {
                    if self.py >= 34 && self.py < 50 && self.px >= 192 {
                        self.scene = Scene::Paper((self.px >= 256) as usize);
                        self.note = None;
                    } else if page == 0 {
                        if let Some(i) = self.row(8, 312, 54, 44, 3) {
                            self.buy_car(i);
                        }
                    } else if let Some(i) = self.row(8, 312, 56, 10, self.pack.manifest.parts.len()) {
                        self.buy_part(i);
                    }
                }
            }
            Scene::Bench => {
                if back {
                    self.scene = Scene::Place;
                    self.note = None;
                } else if pick {
                    self.bench_pick();
                }
            }
            Scene::Yard => {
                if back {
                    self.scene = Scene::Place;
                    self.note = None;
                } else if pick {
                    if let Some(i) = self.row(8, 312, 34, 10, self.state.cars.len()) {
                        self.yard_pick(i, self.px >= 264);
                    }
                }
            }
            Scene::Bets => {
                if back {
                    self.scene = Scene::Place;
                } else if pick {
                    if let Some(b) = self.button(self.stakes().len() + 2) {
                        self.scene = Scene::Place;
                        self.offer_bet(b);
                    }
                }
            }
            Scene::Race => {}
        }
    }

    fn buy_car(&mut self, i: usize) {
        let Some(Some(offer)) = self.offers().into_iter().nth(i) else { return };
        if self.state.cars.len() >= MOST_CARS {
            return self.no("No room. Sell a car first.");
        }
        if self.state.cash < offer.price {
            return self.no("You cannot afford it.");
        }
        self.state.cash -= offer.price;
        let paint = (!offer.paint.is_empty()).then_some(offer.paint);
        self.state.cars.push(Owned { car: offer.car, paint, cond: offer.cond, fitted: BTreeMap::new() });
        let day = self.day();
        if self.state.sold.0 != day {
            self.state.sold = (day, Vec::new());
        }
        self.state.sold.1.push(i);
        if self.state.cur.is_none() {
            self.state.cur = Some(self.state.cars.len() - 1);
            self.say("It is yours. It is in the garage.");
        } else {
            self.say("It is yours. It is out in the yard.");
        }
        tone(880, 80);
        self.save();
    }

    fn buy_part(&mut self, i: usize) {
        let Some((id, part)) = self.pack.manifest.parts.iter().nth(i).map(|(k, p)| (k.clone(), p.clone())) else { return };
        if self.has_part(&id) {
            return self.say("You have one.");
        }
        if self.state.cash < part.price {
            return self.no("You cannot afford it.");
        }
        self.state.cash -= part.price;
        self.state.spares.push(id);
        self.say("Bought. It is on the shelf.");
        tone(880, 80);
        self.save();
    }

    /// The bench: the car's slots down the left, the shelf down the right. Picking a part on the
    /// shelf bolts it on, and whatever was there comes off; picking one on the car takes it off.
    /// Either is an hour's work.
    fn bench_pick(&mut self) {
        if self.py >= 180 && self.px >= 164 {
            self.selling = !self.selling;
            self.note = None;
            return;
        }
        let slots = self.slots();
        if let Some(i) = self.row(164, 312, 34, 10, self.state.spares.len()) {
            let id = self.state.spares[i].clone();
            if self.selling {
                self.state.cash += self.spare_offer(&id);
                self.state.spares.remove(i);
                self.say("Sold.");
                tone(660, 80);
                return self.save();
            }
            let Some(cur) = self.state.cur else { return self.no("No car in the bay to put it on.") };
            let slot = self.part(&id).map(|p| p.slot.clone()).unwrap_or_default();
            self.state.spares.remove(i);
            if let Some(old) = self.state.cars[cur].fitted.insert(slot, id) {
                self.state.spares.push(old);
            }
            self.say("Bolted on. That was an hour.");
            tone(520, 60);
            self.spend(1);
        } else if let Some(i) = self.row(8, 156, 34, 10, slots.len()) {
            let Some(cur) = self.state.cur else { return };
            if let Some(old) = self.state.cars[cur].fitted.remove(&slots[i]) {
                self.state.spares.push(old);
                self.say("Off. The stock one is back on.");
                tone(400, 60);
                self.spend(1);
            } else {
                self.say("That is the stock one.");
            }
        }
    }

    fn yard_pick(&mut self, i: usize, sell: bool) {
        if sell {
            let gone = self.state.cars.remove(i);
            self.state.cash += self.value(&gone);
            self.state.cur = match self.state.cur {
                Some(c) if c == i => None,
                Some(c) if c > i => Some(c - 1),
                other => other,
            };
            if self.state.cur.is_none() {
                self.lamps = false;
            }
            self.say("Sold to the dealer.");
            tone(660, 80);
            self.check_over();
            self.save();
        } else if self.state.cur != Some(i) {
            self.state.cur = Some(i);
            self.say("It is in the bay.");
            tone(520, 60);
            self.save();
        }
    }

    fn race_tick(&mut self, keys: u32, hit: u32) {
        let Some(race) = self.race.as_mut() else { return };
        let alone = race.alone();
        // Backing out of a race is losing it.
        if hit & KEY_B != 0 {
            tone(0, 0);
            return self.settle((!alone).then_some(false));
        }
        if race.phase == Phase::Slip && hit & KEY_A != 0 {
            let won = race.won;
            tone(0, 0);
            return self.settle((!alone).then_some(won));
        }
        let was = race.phase;
        race.tick(Controls { throttle: keys & KEY_UP != 0, stage: hit & KEY_A != 0, up: hit & KEY_X != 0, down: hit & KEY_Y != 0 });
        if was != Phase::Slip && race.phase == Phase::Slip {
            tone(990, 200);
        } else if was == Phase::Stage && race.phase == Phase::Tree {
            tone(660, 60);
        } else if race.phase != Phase::Slip || race.v > 0.0 {
            tone((race.rpm / 40.0) as i32, 60);
        }
    }

    // ---- Drawing ----

    /// A car from the pack in a paint, its tyres on `ground`. Half size for one seen from further off.
    fn draw_car(&self, s: &mut Screen, id: &str, paint: Option<&str>, lamps: bool, x: i32, ground: i32, half: bool) {
        let Some(car) = self.pack.manifest.cars.get(id) else { return };
        let name = if lamps { car.side_lamps.as_ref().unwrap_or(&car.side) } else { &car.side };
        let Some(pic) = self.pack.pictures.get(name) else { return };
        // The picture is painted in one colour; its paint ramps are moved to the car's own.
        let mut tint = Tint::none();
        if let Some(paint) = paint.or(car.paint.as_deref()) {
            let to = ramp(paint) as u8;
            for (from, add) in [(&car.paint_ramps.mid, 0i8), (&car.paint_ramps.dark, -9), (&car.paint_ramps.light, 4), (&car.paint_ramps.glint, 2)] {
                if let Some(from) = from {
                    tint.to[ramp(from)] = to;
                    tint.add[ramp(from)] = add;
                }
            }
        }
        if half {
            s.picture_half(pic, x, ground - (car.ground + 1) / 2, Some(&tint));
        } else {
            s.picture_tint(pic, x, ground - car.ground, Some(&tint));
        }
    }

    /// The three lines along the bottom, and the cash.
    fn panel(&self, s: &mut Screen, title: &str, line: &str, foot: &str) {
        s.vgrad(0, 200, 320, 40, ramp("NAVY"), 6, 2);
        s.frame(0, 200, 320, 40, shade(ramp("CHROME"), 22));
        s.frame(1, 201, 318, 38, shade(ramp("CHROME"), 9));
        let cash = format!("CASH ${}", self.state.cash);
        let room = (296 - cash.len() * 8) / 8;
        s.text_sh(8, 206, &title.to_uppercase().chars().take(room).collect::<String>(), shade(ramp("GOLD"), 26), BLACK);
        s.text_sh(8, 217, &line.to_uppercase().chars().take(38).collect::<String>(), WHITE, BLACK);
        s.text(8, 228, &foot.to_uppercase().chars().take(38).collect::<String>(), shade(ramp("CHROME"), 17));
        s.text(312 - cash.len() as i32 * 8, 206, &cash, shade(ramp("LIME"), 24));
    }

    fn buttons(&self, s: &mut Screen, labels: &[String]) {
        let (n, over) = (labels.len(), self.button(labels.len()));
        let w = 312 / n as i32;
        for (i, label) in labels.iter().enumerate() {
            let (x, on) = (4 + i as i32 * w, over == Some(i));
            s.vgrad(x + 1, 178, w - 2, 18, ramp(if on { "GOLD" } else { "NAVY" }), if on { 26 } else { 12 }, if on { 18 } else { 5 });
            s.frame(x + 1, 178, w - 2, 18, shade(ramp("CHROME"), if on { 28 } else { 14 }));
            s.text(x + w / 2 - label.len() as i32 * 4, 183, label, if on { BLACK } else { WHITE });
        }
    }

    pub fn draw(&self, s: &mut Screen) {
        if let Some(problem) = &self.problem {
            s.clear(shade(ramp("NAVY"), 4));
            s.text_sh(8, 8, "THIS PACK CANNOT BE PLAYED:", shade(ramp("GOLD"), 26), BLACK);
            for (i, line) in wrap(&problem.to_uppercase(), 38).iter().enumerate() {
                s.text(8, 28 + i as i32 * 10, line, WHITE);
            }
            return;
        }
        match self.scene {
            Scene::Title => self.draw_title(s),
            Scene::Place | Scene::Bets => self.draw_place(s),
            Scene::Paper(page) => self.draw_paper(s, page),
            Scene::Bench => self.draw_bench(s),
            Scene::Yard => self.draw_yard(s),
            Scene::Race => return self.draw_race(s),
            Scene::Over(kind) => return self.draw_over(s, kind),
        }
        if self.scene != Scene::Title {
            s.pointer(self.px, self.py);
        }
    }

    fn draw_title(&self, s: &mut Screen) {
        s.vgrad(0, 0, 320, 240, ramp("NAVY"), 3, 14);
        let name = self.pack.manifest.name.to_uppercase();
        for (i, line) in wrap(&name, 18).iter().enumerate() {
            let x = 160 - line.len() as i32 * 8;
            s.text_big(x + 1, 61 + i as i32 * 22, line, shade(ramp("RED"), 14), 2);
            s.text_big(x, 60 + i as i32 * 22, line, shade(ramp("GOLD"), 27), 2);
        }
        if self.saved {
            s.text_sh(92, 190, "ENTER: CARRY ON", WHITE, BLACK);
            s.text_sh(100, 204, "E: A NEW GAME", shade(ramp("CHROME"), 20), BLACK);
        } else if (self.ticks / 15) & 1 == 1 {
            s.text_sh(116, 200, "PRESS ENTER", WHITE, BLACK);
        }
    }

    fn draw_place(&self, s: &mut Screen) {
        let Some(place) = self.here() else { return };
        match place.picture.as_ref().and_then(|p| self.pack.pictures.get(p)) {
            Some(pic) => {
                s.clear(BLACK);
                s.picture(pic, 0, 0);
            }
            None => s.vgrad(0, 0, 320, 200, ramp(place.backdrop.as_deref().unwrap_or("NAVY")), 4, 14),
        }
        let rival = place.rival_at.and(self.rival());
        if let (Some([x, ground]), Some(r)) = (place.rival_at, rival) {
            self.draw_car(s, &r.car, r.paint.as_deref(), true, x, ground, false);
        }
        if let (Some([x, ground]), Some(o)) = (place.car_at, self.owned()) {
            self.draw_car(s, &o.car, o.paint.as_deref(), self.lamps, x, ground, false);
        }
        if self.scene == Scene::Place {
            let over = self.spot_under().map(|s| s.rect);
            for spot in self.spots() {
                let Some(words) = &spot.button else { continue };
                let ([x, y, w, h], on) = (spot.rect, over == Some(spot.rect));
                s.vgrad(x, y, w, h, ramp(if on { "GOLD" } else { "NAVY" }), if on { 26 } else { 12 }, if on { 18 } else { 5 });
                s.frame(x, y, w, h, shade(ramp("CHROME"), if on { 28 } else { 14 }));
                s.text(x + w / 2 - words.len() as i32 * 4, y + h / 2 - 4, &words.to_uppercase(), if on { BLACK } else { WHITE });
            }
        }
        let under = self.spot_under().map(|s| s.label.clone());
        let clock = format!("DAY {} OF {}. {} HOURS LEFT TODAY.", self.day(), self.pack.manifest.season.days, self.hours_left());
        if self.scene == Scene::Bets {
            let mut labels = vec!["KICKS".to_string()];
            labels.extend(self.stakes().iter().map(|b| format!("${b}")));
            labels.push("PINKS".into());
            self.buttons(s, &labels);
            self.panel(s, rival.map(|r| r.name.as_str()).unwrap_or(""), "What are we racing for?", "Escape: never mind");
        } else if let Some(r) = rival {
            let line = self.note.clone().or(under).unwrap_or_else(|| r.says.clone());
            self.panel(s, &r.name, &line, &format!("Runs {:.1}. Day {}, {} hours left.", r.et, self.day(), self.hours_left()));
        } else {
            let line = self.note.clone().or(under).unwrap_or_else(|| "Look round.".into());
            self.panel(s, &place.title, &line, &clock);
        }
    }

    fn draw_paper(&self, s: &mut Screen, page: usize) {
        let (ink, grey, cream) = (shade(ramp("RUBBER"), 3), shade(ramp("BROWN"), 14), ramp("CREAM"));
        s.vgrad(0, 0, 320, 200, cream, 27, 23);
        s.text_big(8, 8, "THE PAPER", ink, 2);
        s.text(8, 30, &format!("DAY {}", self.day()), grey);
        for (t, label) in ["CARS", "PARTS"].iter().enumerate() {
            let x = 192 + t as i32 * 64;
            if t == page {
                s.rect(x, 34, 60, 16, ink);
            }
            s.frame(x, 34, 60, 16, ink);
            s.text(x + 30 - label.len() as i32 * 4, 38, label, if t == page { shade(cream, 28) } else { ink });
        }
        s.rect(8, 52, 304, 1, grey);
        let hover = shade(cream, 31);
        if page == 0 {
            let offers = self.offers();
            let over = self.row(8, 312, 54, 44, 3);
            for (i, offer) in offers.iter().enumerate() {
                let y = 54 + i as i32 * 44;
                if over == Some(i) && offer.is_some() {
                    s.rect(8, y, 304, 43, hover);
                }
                s.rect(8, y + 43, 304, 1, grey);
                let Some(o) = offer else {
                    s.text(120, y + 18, "S O L D", shade(ramp("RED"), 20));
                    continue;
                };
                let car = &self.pack.manifest.cars[&o.car];
                self.draw_car(s, &o.car, Some(&o.paint).filter(|p| !p.is_empty()).map(|p| p.as_str()), false, 12, y + 41, true);
                for (k, line) in wrap(&car.name.to_uppercase(), 16).iter().take(2).enumerate() {
                    s.text(180, y + 4 + k as i32 * 9, line, ink);
                }
                s.text(180, y + 24, cond_word(o.cond), grey);
                s.text(180, y + 34, &format!("${}", o.price), shade(ramp("FOREST"), 12));
            }
            if offers.is_empty() {
                s.text(12, 60, "NO CARS FOR SALE.", grey);
            }
            self.panel(s, "The paper", self.note.as_deref().unwrap_or("Pick a car to buy it."), "Escape puts it down.");
        } else {
            let parts = &self.pack.manifest.parts;
            let over = self.row(8, 312, 56, 10, parts.len());
            for (i, (id, p)) in parts.iter().enumerate() {
                let y = 56 + i as i32 * 10;
                if over == Some(i) {
                    s.rect(8, y - 1, 304, 10, hover);
                }
                s.text(12, y, &p.name.to_uppercase().chars().take(20).collect::<String>(), ink);
                if self.has_part(id) {
                    s.text(272, y, "SOLD", shade(ramp("RED"), 20));
                } else {
                    let price = format!("${}", p.price);
                    s.text(308 - price.len() as i32 * 8, y, &price, shade(ramp("FOREST"), 12));
                }
            }
            if parts.is_empty() {
                s.text(12, 60, "NO PARTS FOR SALE.", grey);
            }
            let pitch = over.and_then(|i| parts.values().nth(i)).map(|p| format!("{}: {}", p.slot, p.pitch));
            self.panel(s, "The paper", self.note.as_deref().or(pitch.as_deref()).unwrap_or("Pick a part to buy it."), "Escape puts it down.");
        }
    }

    fn draw_bench(&self, s: &mut Screen) {
        s.vgrad(0, 0, 320, 200, ramp("WOOD"), 9, 5);
        s.text_sh(8, 8, "ON THE CAR", WHITE, BLACK);
        s.text_sh(164, 8, "ON THE SHELF", WHITE, BLACK);
        s.rect(8, 28, 148, 1, shade(ramp("WOOD"), 18));
        s.rect(164, 28, 148, 1, shade(ramp("WOOD"), 18));
        let (slots, lit) = (self.slots(), shade(ramp("WOOD"), 16));
        let (left, right) = (self.row(8, 156, 34, 10, slots.len()), self.row(164, 312, 34, 10, self.state.spares.len()));
        let mut about: Option<String> = None;
        for (i, slot) in slots.iter().enumerate() {
            let y = 34 + i as i32 * 10;
            let on = self.owned().and_then(|o| o.fitted.get(slot)).and_then(|id| self.part(id));
            if left == Some(i) {
                s.rect(8, y - 1, 148, 10, lit);
                about = Some(match on { Some(p) => format!("{}. Pick it to take it off.", p.pitch), None => format!("{slot}: the stock one.") });
            }
            let name = on.map(|p| p.name.to_uppercase()).unwrap_or_else(|| format!("STOCK {}", slot.to_uppercase()));
            s.text_sh(10, y, &name.chars().take(18).collect::<String>(), if on.is_some() { shade(ramp("GOLD"), 27) } else { shade(ramp("CREAM"), 22) }, BLACK);
        }
        if self.state.cur.is_none() {
            s.text_sh(10, 34 + slots.len() as i32 * 10 + 6, "NO CAR IN THE BAY.", shade(ramp("RED"), 26), BLACK);
        }
        for (i, id) in self.state.spares.iter().enumerate() {
            let y = 34 + i as i32 * 10;
            let Some(p) = self.part(id) else { continue };
            if right == Some(i) {
                s.rect(164, y - 1, 148, 10, lit);
                about = Some(if self.selling { format!("They give ${} for it today.", self.spare_offer(id)) } else { format!("{}: {}", p.slot, p.pitch) });
            }
            s.text_sh(166, y, &p.name.to_uppercase().chars().take(18).collect::<String>(), WHITE, BLACK);
        }
        if self.state.spares.is_empty() {
            s.text_sh(166, 34, "NOTHING.", shade(ramp("CREAM"), 18), BLACK);
        }
        let on = self.py >= 180 && self.px >= 164;
        s.vgrad(164, 180, 148, 16, ramp(if self.selling { "RED" } else { "NAVY" }), if on { 20 } else { 12 }, 6);
        s.frame(164, 180, 148, 16, shade(ramp("CHROME"), if on { 28 } else { 14 }));
        let label = if self.selling { "SELLING: PICK ONE" } else { "SELL SPARE PARTS" };
        s.text(238 - label.len() as i32 * 4, 184, label, WHITE);
        self.panel(s, "The bench", self.note.as_deref().or(about.as_deref()).unwrap_or("Pick a part. A change takes an hour."), "Escape goes back.");
    }

    fn draw_yard(&self, s: &mut Screen) {
        s.vgrad(0, 0, 320, 200, ramp("GRASS"), 14, 8);
        s.vgrad(0, 0, 320, 26, ramp("SKY"), 20, 27);
        s.text_sh(8, 8, "YOUR CARS", WHITE, BLACK);
        s.text_sh(192, 8, "ENGINE", shade(ramp("CREAM"), 26), BLACK);
        let over = self.row(8, 312, 34, 10, self.state.cars.len());
        for (i, o) in self.state.cars.iter().enumerate() {
            let y = 34 + i as i32 * 10;
            if over == Some(i) {
                s.rect(8, y - 1, 304, 10, shade(ramp("GRASS"), 20));
            }
            let car = &self.pack.manifest.cars[&o.car];
            s.rect(10, y, 12, 8, BLACK);
            s.vgrad(11, y + 1, 10, 6, ramp(o.paint.as_deref().or(car.paint.as_deref()).unwrap_or("GREY")), 27, 16);
            let here = self.state.cur == Some(i);
            s.text_sh(28, y, &car.name.to_uppercase().chars().take(20).collect::<String>(), if here { shade(ramp("GOLD"), 27) } else { WHITE }, BLACK);
            s.text_sh(192, y, cond_word(o.cond), WHITE, BLACK);
            s.text_sh(264, y, "SELL", if over == Some(i) && self.px >= 264 { shade(ramp("RED"), 28) } else { shade(ramp("CREAM"), 22) }, BLACK);
        }
        if self.state.cars.is_empty() {
            s.text_sh(8, 34, "YOU HAVE NO CAR. TRY THE PAPER.", WHITE, BLACK);
        }
        let worth = over.and_then(|i| self.state.cars.get(i)).map(|o| format!("The dealer gives ${}.", self.value(o)));
        self.panel(s, "The yard", self.note.as_deref().or(worth.as_deref()).unwrap_or("Pick one to bring it into the bay."), "The gold one is in the bay. Escape.");
    }

    /// The strip from the side, at night: your lane nearest, the tree, and the dash along the bottom.
    fn draw_race(&self, s: &mut Screen) {
        let Some(r) = &self.race else { return };
        const PER_M: f64 = 27.0;
        const NOSE: i32 = 172;
        let at = |metres: f64| NOSE + ((metres - r.pos) * PER_M) as i32;
        s.vgrad(0, 0, 320, 70, ramp("NAVY"), 1, 8);
        for i in 0..30u32 {
            let h = hash(i + 7);
            s.pset((h % 320) as i32, ((h >> 9) % 60) as i32, shade(ramp("WHITE"), 10 + ((h >> 20) % 16) as i32));
        }
        // The far wall, in boards ten metres long, and the floodlights over it.
        let first = (r.pos / 10.0).floor() as i32 - 1;
        for k in first..first + 3 {
            let x = at(k as f64 * 10.0);
            let name = match k.rem_euclid(4) { 0 => "BLUE", 2 => "RED", _ => "CONCRETE" };
            s.vgrad(x, 70, (10.0 * PER_M) as i32, 22, ramp(name), 13, 8);
            s.rect(x, 70, 1, 22, shade(ramp("RUBBER"), 3));
            if k.rem_euclid(3) == 0 {
                s.rect(x + 40, 18, 2, 52, shade(ramp("CHROME"), 7));
                s.rect(x + 32, 14, 18, 5, shade(ramp("CREAM"), 30));
                s.rect(x + 28, 16, 26, 1, shade(ramp("WHITE"), 18));
            }
        }
        s.vgrad(0, 92, 320, 86, ramp("ASPHALT"), 8, 13);
        s.rect(0, 136, 320, 1, shade(ramp("WHITE"), 16));
        for (line, name) in [(0.0, "WHITE"), (QUARTER, "YELLOW")] {
            let x = at(line);
            s.rect(x, 96, 3, 82, shade(ramp(name), 26));
        }
        if !r.alone() {
            if let Some(rv) = self.rival() {
                let x = NOSE - 122 + ((r.rival_pos - r.pos) * PER_M) as i32;
                if (-200..400).contains(&x) {
                    self.draw_car(s, &rv.car, rv.paint.as_deref(), true, x, 128, true);
                }
            }
        }
        if let Some(o) = self.owned() {
            self.draw_car(s, &o.car, o.paint.as_deref(), true, NOSE - 122, 170, true);
        }
        // The tree: a staging light, three ambers, the green, and the red.
        s.rect(286, 6, 28, 76, shade(ramp("RUBBER"), 3));
        s.frame(286, 6, 28, 76, shade(ramp("CHROME"), 12));
        let light = r.light();
        for row in 0..6 {
            let on = match row { 0 => r.phase != Phase::Stage, 1..=3 => r.phase == Phase::Tree && light == row - 1, 4 => light >= 3 && !r.red, _ => r.red };
            let name = match row { 0 => "YELLOW", 1..=3 => "ORANGE", 4 => "LIME", _ => "RED" };
            for k in 0..2 {
                s.rect(290 + k * 12, 10 + row * 12, 8, 8, shade(ramp(name), if on { 29 } else { 5 }));
            }
        }
        if !r.alone() && r.phase != Phase::Slip {
            // Who is where: the two of you along a line the length of the strip.
            s.rect(40, 12, 201, 1, shade(ramp("CHROME"), 12));
            s.rect(240, 8, 1, 9, WHITE);
            let mark = |pos: f64| 40 + ((pos / QUARTER).clamp(0.0, 1.0) * 200.0) as i32 - 2;
            s.rect(mark(r.rival_pos), 13, 5, 3, shade(ramp(self.rival().and_then(|v| v.paint.as_deref()).unwrap_or("GREY")), 24));
            s.rect(mark(r.pos), 8, 5, 3, shade(ramp(self.owned().and_then(|o| o.paint.as_deref()).unwrap_or("WHITE")), 26));
        }
        let say = match r.phase {
            Phase::Stage => "ENTER TO STAGE",
            Phase::Tree if r.gear == 0 => "HOLD UP FOR REVS. E ON THE GREEN.",
            _ if r.red => "RED LIGHT",
            _ => "",
        };
        s.text_sh(160 - say.len() as i32 * 4, 40, say, if r.red { shade(ramp("RED"), 28) } else { WHITE }, BLACK);

        // The dash: the tachometer as a bar with its red end, and what the car is doing.
        s.vgrad(0, 178, 320, 62, ramp("RUBBER"), 7, 2);
        s.frame(0, 178, 320, 62, shade(ramp("CHROME"), 16));
        let top = (r.redline() + 800.0).max(1000.0);
        let bar = |rpm: f64| (rpm / top * 200.0).clamp(0.0, 200.0) as i32;
        s.rect(8, 188, 202, 14, BLACK);
        s.rect(9 + bar(r.redline()), 189, 200 - bar(r.redline()), 12, shade(ramp("RED"), 10));
        s.vgrad(9, 189, bar(r.rpm), 12, ramp(if r.rpm >= r.redline() { "RED" } else { "LIME" }), 26, 16);
        for k in 1..(top as i32 / 1000 + 1) {
            s.rect(9 + bar(k as f64 * 1000.0), 202, 1, 3, shade(ramp("CHROME"), 20));
        }
        s.text(8, 208, &format!("RPM {:4}", (r.rpm as i32) / 10 * 10), WHITE);
        if r.turbo() {
            s.text(8, 222, &format!("BOOST {:2} PSI", r.boost as i32), shade(ramp("SKY"), 24));
        }
        let gear = if r.gear == 0 { "N".to_string() } else { r.gear.to_string() };
        s.text(222, 184, "GEAR", shade(ramp("CHROME"), 18));
        s.text_big(228, 196, &gear, shade(ramp("GOLD"), 28), 3);
        s.text(268, 184, "MPH", shade(ramp("CHROME"), 18));
        s.text_big(262, 200, &format!("{:3}", (r.v * 2.237) as i32), WHITE, 2);
        let shift = r.rpm > r.redline() - 700.0 && r.phase == Phase::Run;
        s.rect(120, 208, 90, 10, shade(ramp("RED"), if shift { 28 } else { 4 }));
        s.text(141, 209, "SHIFT", if shift { WHITE } else { shade(ramp("RUBBER"), 10) });
        s.text(120, 224, "E UP  Q DOWN", shade(ramp("CHROME"), 14));

        if r.phase == Phase::Slip {
            // The timeslip.
            let (ink, red) = (shade(ramp("RUBBER"), 4), shade(ramp("RED"), 20));
            s.rect(95, 31, 136, 124, shade(ramp("RUBBER"), 2));
            s.vgrad(92, 28, 136, 124, ramp("CREAM"), 27, 23);
            s.text(100, 36, "TIMESLIP", ink);
            s.rect(98, 48, 124, 1, shade(ramp("BROWN"), 12));
            let rows = [("R/T", if r.red { "RED".to_string() } else { ms(r.rt) }), ("60 FT", ms(r.sixty)), ("1/4 ET", ms(r.et)), ("MPH", r.mph.to_string())];
            for (i, (label, value)) in rows.iter().enumerate() {
                s.text(100, 56 + i as i32 * 12, label, ink);
                s.text(164, 56 + i as i32 * 12, value, if r.red && i == 0 { red } else { ink });
            }
            if !r.alone() {
                s.text(100, 110, "HIM", ink);
                s.text(164, 110, &ms(r.rival_rt + r.rival_et), ink);
                let (words, c) = if r.won { ("YOU WIN.", shade(ramp("FOREST"), 14)) } else if r.red { ("RED LIGHT.", red) } else if r.et > 0 { ("YOU LOSE.", red) } else { ("HE IS GONE.", red) };
                s.text(100, 126, words, c);
            }
            s.text(100, 140, "ENTER", shade(ramp("BROWN"), 14));
        }
    }

    fn draw_over(&self, s: &mut Screen, kind: u8) {
        s.vgrad(0, 0, 320, 240, ramp(if kind == 3 { "GOLD" } else { "NAVY" }), if kind == 3 { 8 } else { 3 }, if kind == 3 { 20 } else { 10 });
        let (head, words) = match kind {
            1 => ("THE SEASON IS OVER", "The days ran out and someone else is still the one to beat."),
            2 => ("YOU ARE BROKE", "No car, and not the money for one. That is the end of the road."),
            _ => ("YOU ARE THE ONE TO BEAT", "You won the car nobody could. The strip is yours."),
        };
        s.text_sh(160 - head.len() as i32 * 4, 60, head, WHITE, BLACK);
        for (i, line) in wrap(&words.to_uppercase(), 34).iter().enumerate() {
            s.text_sh(160 - line.len() as i32 * 4, 90 + i as i32 * 12, line, shade(ramp("CREAM"), 28), BLACK);
        }
        let tally = format!("DAY {}  CASH ${}  BEATEN {}", self.day().min(self.pack.manifest.season.days), self.state.cash, self.state.rung);
        s.text_sh(160 - tally.len() as i32 * 4, 150, &tally, WHITE, BLACK);
        s.text_sh(116, 200, "PRESS ENTER", WHITE, BLACK);
    }
}

/// Break a line into pieces no longer than `width`, at spaces where it can.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(last) if last.len() + 1 + word.len() <= width => {
                last.push(' ');
                last.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}
