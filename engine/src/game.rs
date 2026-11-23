//! The game: a season of days and hours, a pointer, the place you are in, and what picking things
//! does.
//!
//! Nothing here knows what a garage is, or who the King is. A place is whatever the pack says it
//! is; so are the cars, the parts in the paper, and the people you race. What is here is the
//! rules: that things cost money and hours, how a bet is made and settled, and how it ends.

use crate::gfx::{ramp, shade, Screen, Tint, BLACK, WHITE};
use crate::host::tone;
use crate::g3d::{Cam, Scene3, V3};
use crate::pack::{Boost, Car, Cockpit, Dial, Job, Pack, Part, Place, Rival, Slot, Spot};
use crate::race::{Controls, Phase, Race, QUARTER};
use crate::town::{Pedals, Town};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod online;
pub mod roadtrip;
pub mod dyno;
pub mod cards;
pub mod projects;
pub mod sound;
pub mod world;
pub mod open;
mod balance;
use online::Net;

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
/// N: the button on the wheel.
pub const KEY_N: u32 = 256;
/// H or ?: show what there is to pick.
pub const KEY_H: u32 = 512;
/// The space bar: picks, like Enter, except in a race, where it is the shift up, as it was in Street Rod.
pub const KEY_SPACE: u32 = 1024;
/// How many lines of a list fit a column at the bench, and a page of the paper.
const LINES: usize = 12;
/// Every bolt of a part with `n` of them, tight.
fn all_bolts(n: usize) -> u8 {
    ((1u16 << n.min(8)) - 1) as u8
}

/// Whether a slot's name is one thing or several: the plug wires are, the battery is.
fn is_are(slot: &str) -> &'static str {
    if slot.ends_with('s') && !slot.ends_with("ss") { "are" } else { "is" }
}

const PAGE: usize = 11;
/// Minutes to undo or do up one bolt, and to lift a part off or set one in.
const BOLT: f64 = 3.0;
const LIFT: f64 = 10.0;

/// The most cars you can keep.
pub const MOST_CARS: usize = 16;
const PAINTS: [&str; 8] = ["BLUE", "RED", "GOLD", "GREEN", "SILVER", "WHITE", "VIOLET", "TEAL"];
const BETS: [i32; 4] = [10, 25, 50, 100];
/// How many people will pay you in a day before word gets round. The next one up the ladder
/// always will: he wants to see for himself.
pub const PAID_A_DAY: usize = 2;

/// A car of yours: which one, its paint, how healthy its engine is as a percentage, and what is
/// bolted to it, by slot.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Owned {
    pub car: String,
    pub paint: Option<String>,
    pub cond: i32,
    pub fitted: BTreeMap<String, String>,
    /// Under the hood: slots with nothing on them (the stock part is on the floor), and which of
    /// each slot's bolts are tight, a bit apiece. A slot not listed is done up.
    #[serde(default)]
    pub off: Vec<String>,
    #[serde(default)]
    pub bolts: BTreeMap<String, u8>,
    /// Gallons in the tank.
    #[serde(default = "some_gas")]
    pub gas: f64,
    /// Seconds of nitrous left in the bottle, if it has a kit.
    #[serde(default)]
    pub bottle: f64,
    /// What has been done to its computer.
    #[serde(default)]
    pub ecu: Ecu,
    /// How knocked about its body is, and how worn its tires: each from nothing to a hundred.
    #[serde(default)]
    pub body: i32,
    #[serde(default, alias = "tyres")]
    pub tires: f64,
    /// How adjustable suspension is set: from -2, soft for the road, to 2, stiff for the strip.
    #[serde(default)]
    pub setup: i32,
    /// Parts that change how it breathes or fuels have been swapped since it was last tuned.
    #[serde(default)]
    pub untuned: bool,
    /// Road gone by since the parts last wore, in kilometers.
    #[serde(default)]
    pub run: f64,
    /// It has a build sheet: it is made of the parts fitted to it, and a slot with nothing in it
    /// has nothing in it. A car without is as it always was: what is not changed is stock.
    #[serde(default)]
    pub sheet: bool,
}

/// A part's name without the state it is in: `valves@40` is `valves`, at forty per cent.
pub fn base(id: &str) -> &str {
    id.split('@').next().unwrap_or(id)
}

/// How good a part is, from 100 (new) to 0 (done for).
pub fn cond_of(id: &str) -> i32 {
    id.split_once('@').and_then(|(_, c)| c.parse().ok()).unwrap_or(100)
}

/// A part in a state: new ones are just their name.
pub fn worn(id: &str, cond: i32) -> String {
    if cond >= 100 { base(id).to_string() } else { format!("{}@{}", base(id), cond.max(0)) }
}

/// A car's tune, as steps from how it left the factory: pounds of boost, fuel (more is richer),
/// and degrees of timing.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
pub struct Ecu {
    pub boost: i32,
    pub fuel: i32,
    pub timing: i32,
}

fn some_gas() -> f64 {
    8.0
}

impl Owned {
    pub fn new(car: &str, paint: Option<String>, cond: i32) -> Owned {
        Owned { car: car.to_string(), paint, cond, fitted: BTreeMap::new(), off: Vec::new(), bolts: BTreeMap::new(), gas: 8.0, bottle: 0.0, ecu: Ecu::default(), body: 0, tires: 0.0, setup: 0, untuned: false, run: 0.0, sheet: false }
    }

    /// Which of a slot's `n` bolts are tight, a bit apiece.
    fn tight(&self, slot: &str, n: usize) -> u8 {
        let all = ((1u16 << n.min(8)) - 1) as u8;
        self.bolts.get(slot).map_or(all, |m| m & all)
    }
}

/// Something lying on the floor under the hood, or in your hand: a slot's stock part, or a part
/// from the pack.
#[derive(Clone, Debug, PartialEq)]
pub enum Loose {
    Stock(String),
    Part(String),
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
    /// The day, and who has lost money to you that day: none of them will put money up again
    /// until tomorrow.
    #[serde(default)]
    pub paid_up: (i32, Vec<usize>),
    /// Races against someone won and lost, whatever they were for.
    #[serde(default)]
    pub knocked: (i32, Vec<(usize, i32)>),
    /// The last day a shootout was entered: one go at each.
    #[serde(default)]
    pub entered: i32,
    #[serde(default)]
    pub wins: u32,
    #[serde(default)]
    pub losses: u32,
    /// Other people's cars in the shared world can be hit: off, they are ghosts.
    #[serde(default)]
    pub solid: bool,
    /// Cars bought and on a trailer: the day each gets here, the car, and where it is from.
    #[serde(default)]
    pub coming: Vec<(i32, Owned, String)>,
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
    Hood,
    Yard,
    Bets,
    /// Picking a color for a respray.
    Paint,
    /// The rollers: what the car makes.
    Dyno,
    /// The laptop: the log of the last pull, and the tune.
    Laptop,
    /// The meet: other people, to race. And picking what a race with one of them is for.
    Online,
    OnlineBets,
    Race,
    /// Out on a map, behind the wheel.
    Drive,
    /// Out of town, on a road between two places.
    Road,
    /// 1 the season ran out, 2 broke, 3 you beat the one to beat.
    Over(u8),
}

pub struct Game {
    pub pack: Pack,
    /// The town and the roads out of it, laid out as one place to drive, if the pack has them.
    land: Option<crate::land::World>,
    /// Hours owed for miles of road driven, not yet a whole one.
    clock: f64,
    /// A race or a poker run out on a road of the world.
    pub dash: Option<open::Dash>,
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
    pub town: Option<Town>,
    pub trip: Option<crate::road::Trip>,
    /// The map a trip set out from, if it was not a place; and whether the town was come to by road,
    /// when there is no turning back to where the day started.
    trip_from: Option<String>,
    town_by_road: bool,
    /// The road the race being bet on is down, if it is not the strip.
    road_race: Option<String>,
    /// What the rollers said, while it is being looked at.
    pub sheet: Option<dyno::Sheet>,
    /// The round of the shootout being run, counted from nothing, while one is on.
    pub bracket: Option<usize>,
    /// How far down each long list has been turned: the car's parts and the shelf at the bench,
    /// and the parts in the paper.
    pub turned: [usize; 3],
    /// Which group of parts the paper's parts page shows: 0 all of them, then one place on the car each.
    pub shop_group: usize,
    /// How fast the car was going as it came off a road into a town, to carry on at.
    rolling: f64,
    pub rollers: dyno::Rollers,
    pub net: Net,
    selling: bool,
    pub carrying: Option<Loose>,
    /// Which part of the car is being worked on: under the hood, the trunk, underneath...
    pub area: usize,
    /// The car in the yard whose ways to get rid of it are being shown.
    pub parting: Option<usize>,
    saved: bool,
    /// Whether everything that can be picked in a place is being outlined, and whether E has been
    /// pressed once on the title with a game kept, so that once more starts over.
    pub hints: bool,
    sure: bool,
    rng: u32,
    last_under: Option<String>,
    pub(crate) last_keys: u32,
    /// The page makes the engine's sound itself: no beeping for it.
    pub voiced: bool,
    /// An automatic, or any car in town: the gear it has picked for itself, and the revs that makes.
    pub auto: (usize, f64),
    /// How much boost there was while the throttle was down, and how many times the valve has let go.
    spooled: f32,
    blowoffs: u32,
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
            who: 0, bet: 0, race: None, town: None, trip: None, trip_from: None, town_by_road: false, road_race: None, sheet: None, bracket: None, turned: [0; 3], shop_group: 0, rolling: 0.0, rollers: dyno::Rollers::default(), net: Net::default(), selling: false, carrying: None, area: 0, parting: None, saved: false, hints: false, sure: false, rng: 1, last_under: None, last_keys: 0, voiced: false, auto: (0, 900.0), spooled: 0.0, blowoffs: 0, last_mouse: (160, 100), ticks: 0, land: None, clock: 0.0, dash: None,
        };
        g.land = crate::land::World::build(&g.pack.manifest);
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
        let cars: Vec<Owned> = m.start.car.iter().filter_map(|id| m.cars.get(id).map(|c| { let mut o = Owned::new(id, c.paint.clone(), 70); self.outfit(&mut o); o })).collect();
        State { pack: m.name.clone(), place: m.start.place.clone(), cash: m.start.cash, cur: (!cars.is_empty()).then_some(0), cars, ..State::default() }
    }

    /// The saved game, if there is one and it is this pack's and still makes sense in it.
    fn kept(&self) -> Option<State> {
        let m = &self.pack.manifest;
        let s: State = serde_json::from_slice(&crate::host::fetch()).ok()?;
        let fits = s.pack == m.name
            && m.places.contains_key(&s.place)
            && s.cars.iter().chain(s.coming.iter().map(|c| &c.1)).all(|c| m.cars.contains_key(&c.car) && c.fitted.values().all(|p| m.parts.contains_key(base(p))))
            && s.spares.iter().all(|p| m.parts.contains_key(base(p)))
            && s.cur.is_none_or(|i| i < s.cars.len())
            && s.rung <= m.rivals.len();
        let mut s = fits.then_some(s)?;
        self.settle_hood(&mut s);
        Some(s)
    }

    /// A game saved before what came under the hood was on the build sheet: what was on there is
    /// fitted, and what was lying on the floor is on the shelf.
    pub fn settle_hood(&self, s: &mut State) {
        // Slots saved under their old names, before the pack said them the American way.
        for o in s.cars.iter_mut() {
            let Some(car) = self.pack.manifest.cars.get(&o.car) else { continue };
            for (old, new) in [("Tyres", "Tires"), ("Gearbox", "Transmission")] {
                if !car.stock.contains_key(new) && car.area_of(new).is_none() {
                    continue;
                }
                if let Some(p) = o.fitted.remove(old) {
                    o.fitted.insert(new.to_string(), p);
                }
                if let Some(b) = o.bolts.remove(old) {
                    o.bolts.insert(new.to_string(), b);
                }
                for slot in o.off.iter_mut().filter(|x| *x == old) {
                    *slot = new.to_string();
                }
            }
        }
        for i in 0..s.cars.len() {
            let Some(car) = self.pack.manifest.cars.get(&s.cars[i].car).filter(|_| s.cars[i].sheet) else { continue };
            for (slot, part) in car.stock.iter().filter(|(slot, _)| car.area_of(slot).is_some()) {
                let o = &mut s.cars[i];
                if !o.fitted.contains_key(slot) && !o.off.contains(slot) {
                    let cond = o.cond;
                    o.fitted.insert(slot.clone(), worn(part, cond));
                } else if !s.spares.iter().chain(s.cars.iter().flat_map(|c| c.fitted.values())).any(|p| base(p) == part) {
                    let cond = s.cars[i].cond;
                    s.spares.push(worn(part, cond));
                }
            }
        }
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

    /// The nitrous kit on a car, if it has one.
    pub fn kit(&self, o: &Owned) -> Option<&crate::pack::Nitrous> {
        o.fitted.values().filter_map(|id| self.part(id)).find_map(|p| p.nitrous.as_ref())
    }

    /// What is left in the bottle after a run goes back in the car.
    pub(crate) fn keep_bottle(&mut self, left: f64) {
        if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
            o.bottle = o.bottle.min(left.max(0.0));
        }
    }

    /// The colors a car can be had in, or sprayed.
    pub fn paints(&self) -> Vec<String> {
        let m = &self.pack.manifest;
        if m.paints.is_empty() { PAINTS.iter().map(|p| p.to_string()).collect() } else { m.paints.iter().take(12).cloned().collect() }
    }

    /// Whether it rains on a day. The same days every game of a pack, so the paper can say.
    pub fn rain_on(&self, day: i32) -> bool {
        let m = &self.pack.manifest;
        day > 1 && (hash(day as u32 ^ (m.name.len() as u32) << 8) % 100) < m.season.rain.clamp(0, 100) as u32
    }

    pub fn raining(&self) -> bool {
        self.rain_on(self.day())
    }

    /// Rain across a box of the screen: streaks that fall a little further every tick.
    pub(crate) fn draw_rain(&self, s: &mut Screen, w: i32, h: i32) {
        if !self.raining() || h < 20 {
            return;
        }
        let c = shade(ramp("CHROME"), 20);
        for i in 0..70u32 {
            let k = hash(i * 31 + 5);
            let (x, y) = ((k % (w as u32 + 40)) as i32 - 20, ((k >> 10) as i32 + self.ticks as i32 * (9 + (i % 4) as i32)) % (h - 8));
            s.line(x, y, x - 2, y + 7, c);
        }
    }

    /// The wipers, when it is raining: two arms across the glass and back.
    pub(crate) fn draw_wipers(&self, s: &mut Screen, view: i32) {
        if !self.raining() {
            return;
        }
        let t = (self.ticks % 40) as f64 / 40.0;
        let sweep = (if t < 0.5 { t * 2.0 } else { 2.0 - t * 2.0 }) * 1.45 + 0.12;
        for pivot in [96, 216] {
            let (ex, ey) = (pivot as f64 - sweep.cos() * 118.0, view as f64 - sweep.sin() * 118.0);
            for d in -2..4 {
                s.line(pivot + d, view, ex as i32 + d, ey as i32, shade(ramp("RUBBER"), if d == -2 || d == 3 { 8 } else { 2 }));
            }
        }
    }

    /// Mountains along the horizon: a far range and a nearer one, which turn as the car does.
    pub(crate) fn draw_ridge(&self, s: &mut Screen, yaw: f64, horizon: i32) {
        let dark = self.dark();
        for (layer, (scale, tall, name)) in [(1.0, 26.0, "INDIGO"), (1.7, 15.0, "FOREST")].into_iter().enumerate() {
            let c = shade(ramp(name), if dark == 2 { 3 } else if dark == 1 { 6 } else { 11 + layer as i32 * 2 } - if self.raining() { 2 } else { 0 });
            for x in 0..320 {
                let a = (yaw + (x - 160) as f64 / 200.0) * scale + layer as f64 * 2.0;
                let h = ((a * 3.0).sin() * 0.5 + (a * 7.3).sin() * 0.3 + (a * 17.0).sin() * 0.12 + 0.6).max(0.05) * tall;
                s.rect(x, horizon - h as i32, 1, h as i32, c);
            }
        }
    }

    /// How dark it is out: 0 by day, 1 as the day's last hours come on, 2 in its last sixth.
    pub fn dark(&self) -> i32 {
        let per = self.pack.manifest.season.hours_per_day.max(1);
        let gone = (self.state.hours % per) as f64 / per as f64;
        if gone >= 0.83 { 2 } else if gone >= 0.66 { 1 } else { 0 }
    }

    /// The sky and the far ground for how dark it is, and how many shades the world is taken down.
    pub(crate) fn sky(&self, s: &mut Screen, horizon: i32, view: i32, sky: usize, land: usize) -> i32 {
        match self.dark() {
            0 if self.raining() => {
                s.vgrad(0, 0, 320, horizon, ramp("GRAY"), 12, 22);
                s.vgrad(0, horizon, 320, view - horizon, land, 10, 14);
                2
            }
            0 => {
                s.vgrad(0, 0, 320, horizon, sky, 15, 29);
                s.vgrad(0, horizon, 320, view - horizon, land, 12, 17);
                0
            }
            1 => {
                s.vgrad(0, 0, 320, horizon * 2 / 3, ramp("INDIGO"), 8, 16);
                s.vgrad(0, horizon * 2 / 3, 320, horizon - horizon * 2 / 3, ramp("ORANGE"), 16, 26);
                s.vgrad(0, horizon, 320, view - horizon, land, 8, 12);
                5
            }
            _ => {
                s.vgrad(0, 0, 320, horizon, ramp("NAVY"), 2, 9);
                s.vgrad(0, horizon, 320, view - horizon, land, 3, 6);
                // Stars, the same ones every night.
                for i in 0..40u32 {
                    let k = i.wrapping_mul(2654435761) >> 7;
                    s.pset((k % 320) as i32, ((k >> 9) % (horizon.max(8) as u32 - 6)) as i32, shade(ramp("WHITE"), 18 + (k % 10) as i32));
                }
                11
            }
        }
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

    /// A part by what it is called, whatever state it is in.
    fn part(&self, id: &str) -> Option<&Part> {
        self.pack.manifest.parts.get(base(id))
    }

    /// A car with a build sheet is given its parts: each about as good as the car is, some better, some worse.
    pub fn outfit(&self, o: &mut Owned) {
        let Some(car) = self.pack.manifest.cars.get(&o.car).filter(|c| !c.stock.is_empty()) else { return };
        o.sheet = true;
        for (i, (slot, part)) in car.stock.iter().enumerate() {
            let roll = hash(i as u32 * 131 + o.cond as u32 + o.car.len() as u32);
            let cond = (o.cond + (roll % 31) as i32 - 15).clamp(20, 100);
            // Now and then one left the factory with something better there.
            let rare = self.pack.manifest.parts.iter().find(|(_, p)| p.rare > 0 && p.slot == *slot && p.goes_on(&o.car) && (roll >> 8) % 100 < p.rare).map(|(id, _)| id);
            o.fitted.entry(slot.clone()).or_insert_with(|| worn(rare.unwrap_or(part), cond));
        }
    }

    /// What the engine makes at its best, in pound-feet: what the driveline has to take.
    pub fn peak_torque(&self) -> f64 {
        let Some((o, car)) = self.owned().zip(self.car()) else { return 0.0 };
        let (d, (gain, _)) = (self.tuned_drive(o, car), self.tune_engine(o));
        d.torque.iter().filter(|p| p[0] <= d.redline).map(|p| p[1]).fold(0.0, f64::max) * gain * self.blown(o, car)
    }

    /// A launch with more torque than a part of the driveline will take: sooner or later it lets
    /// go, the weakest first. What broke, if anything did.
    pub fn strain_driveline(&mut self) -> Option<String> {
        let made = self.peak_torque();
        let cur = self.state.cur?;
        // The clutch first: over what it holds it slips, and slipping burns it. What it lets
        // through is all the rest of the driveline ever sees.
        let holds = self.clutch(&self.state.cars[cur]);
        if let Some(holds) = holds.filter(|&h| made > h) {
            let burn = (((made / holds - 1.0) * 40.0) as i32).clamp(2, 30);
            let found = self.state.cars[cur].fitted.iter().find(|(_, id)| cond_of(id) > 0 && self.part(id).is_some_and(|p| p.slips)).map(|(s, id)| (s.clone(), id.clone()));
            if let Some((slot, id)) = found {
                let left = (cond_of(&id) - burn).max(0);
                self.state.cars[cur].fitted.insert(slot, worn(&id, left));
                if left == 0 {
                    return Some(self.part(&id).map(|p| p.name.clone()).unwrap_or_default());
                }
            }
        }
        let torque = made * self.slip(&self.state.cars[cur]);
        let (slot, id, holds) = self.state.cars[cur].fitted.iter().filter(|(_, id)| cond_of(id) > 0)
            .filter_map(|(slot, id)| Some((slot.clone(), id.clone(), self.part(id).filter(|p| !p.slips)?.holds?))).filter(|&(_, _, h)| torque > h)
            .min_by(|a, b| a.2.total_cmp(&b.2))?;
        let chance = ((torque / holds - 1.0) * 150.0).clamp(4.0, 80.0) as u32;
        if self.rand() % 100 >= chance {
            return None;
        }
        self.state.cars[cur].fitted.insert(slot, worn(&id, 0));
        Some(self.part(&id).map(|p| p.name.clone()).unwrap_or_default())
    }

    fn has_part(&self, id: &str) -> bool {
        self.state.spares.iter().any(|p| p == id) || self.state.cars.iter().any(|c| c.fitted.values().any(|p| p == id))
    }

    /// How the brakes and the suspension of a car are: each a share of an ordinary car's, from
    /// what is fitted and how worn it is. A worn part does three fifths of its job, at worst.
    pub fn chassis(&self, o: &Owned) -> (f64, f64) {
        let part = |pick: fn(&Part) -> Option<i32>| o.fitted.values().filter_map(|id| Some((pick(self.part(id)?)?, cond_of(id))))
            .map(|(more, cond)| (1.0 + more as f64 / 100.0) * (0.6 + 0.4 * cond as f64 / 100.0)).product::<f64>();
        // Set up stiff for the strip, it takes a bend worse; soft, better.
        (part(|p| p.brakes), part(|p| p.handling) * (1.0 - 0.06 * self.setup_of(o) as f64))
    }

    /// How the adjustable suspension on a car is set, if it has any that works: nothing otherwise.
    pub fn setup_of(&self, o: &Owned) -> i32 {
        let adjustable = o.fitted.values().any(|id| cond_of(id) > 0 && self.part(id).is_some_and(|p| p.adjustable));
        if adjustable { o.setup.clamp(-2, 2) } else { 0 }
    }

    /// The car as it is on the strip: its drive, with what the suspension does for it off the line.
    /// Good suspension puts the power down a little better and tired dampers a little worse, and
    /// suspension set stiff for the strip sits down and hooks.
    pub fn strip_drive(&self, o: &Owned, car: &Car) -> crate::pack::Drive {
        let mut d = self.tuned_drive(o, car);
        let handling = o.fitted.values().filter_map(|id| Some((self.part(id)?.handling?, cond_of(id))))
            .map(|(more, cond)| (1.0 + more as f64 / 100.0) * (0.6 + 0.4 * cond as f64 / 100.0)).product::<f64>();
        d.grip *= (1.0 + (handling - 1.0) * 0.25) * (1.0 + 0.04 * self.setup_of(o) as f64);
        d
    }

    /// The town car brakes and turns as the car in the bay does.
    pub fn fit_town(&mut self) {
        let p = self.pace();
        let handling = self.owned().map_or(1.0, |o| self.chassis(o).1);
        if let Some(t) = self.town.as_mut() {
            (t.brakes, t.turn, t.accel, t.top) = (p.brakes, handling.sqrt().clamp(0.75, 1.3), p.accel, p.top);
        }
    }

    /// What the parts on a car do: torque as a fraction of stock, and the turbo fitted, if one is.
    pub fn tune(&self, o: &Owned) -> (f64, Option<Boost>) {
        let (gain, turbo) = self.tune_engine(o);
        (gain * self.slip(o), turbo)
    }

    /// What the clutch will take just now, if the car has one that slips: what it holds new, and
    /// down to three fifths of that as it wears. And what the engine makes at its best.
    fn clutch(&self, o: &Owned) -> Option<f64> {
        o.fitted.values().filter(|id| cond_of(id) > 0).filter_map(|id| Some((id, self.part(id)?))).filter(|(_, p)| p.slips)
            .filter_map(|(id, p)| Some(p.holds? * (0.6 + 0.4 * cond_of(id) as f64 / 100.0))).reduce(f64::min)
    }

    fn engine_torque(&self, o: &Owned) -> f64 {
        let Some(car) = self.pack.manifest.cars.get(&o.car) else { return 0.0 };
        let d = self.tuned_drive(o, car);
        d.torque.iter().filter(|p| p[0] <= d.redline).map(|p| p[1]).fold(0.0, f64::max) * self.tune_engine(o).0 * self.blown(o, car)
    }

    /// How much of what the engine makes gets through the clutch: all of it, until the engine
    /// makes more than the clutch holds.
    pub fn slip(&self, o: &Owned) -> f64 {
        match (self.clutch(o), self.engine_torque(o)) {
            (Some(holds), made) if made > holds => (holds / made).max(0.4),
            _ => 1.0,
        }
    }

    /// What the engine itself makes of its parts, its health and its tune, before the clutch.
    pub fn tune_engine(&self, o: &Owned) -> (f64, Option<Boost>) {
        let parts: Vec<&Part> = o.fitted.values().filter_map(|id| self.part(id)).collect();
        let more: i32 = parts.iter().map(|p| p.torque).sum();
        let turbo = parts.iter().find_map(|p| p.boost.clone());
        let blown = turbo.is_some() || self.pack.manifest.cars.get(&o.car).is_some_and(|c| c.drive.boost.is_some());
        // Parts that wear and are past half cost a little power each, down to three quarters in all.
        let tired = o.fitted.values().filter(|id| cond_of(id) < 50 && self.part(id).is_some_and(|p| p.wears > 0.0 && p.brakes.is_none() && p.handling.is_none()))
            .map(|id| 1.0 - (50 - cond_of(id)) as f64 * 0.002).product::<f64>().max(0.75);
        // New parts on the old tune: it runs, but rich here and lean there, and down on power.
        let rough = if o.untuned { 0.85 } else { 1.0 };
        // A tired engine has lost compression, not most of itself: at nothing it still makes over half.
        let health = 0.55 + 0.45 * o.cond as f64 / 100.0;
        ((100 + more) as f64 / 100.0 * health * tired * rough * dyno::tuned(o.ecu, blown).0, turbo)
    }

    /// The car as it is set up: its own running gear with what the parts on it change. Tires and
    /// the like add grip, light parts take weight off, a transmission brings its own ratios, and a
    /// built engine its own redline.
    pub fn tuned_drive(&self, o: &Owned, car: &Car) -> crate::pack::Drive {
        let mut d = car.drive.clone();
        let mut grip = 100;
        for p in o.fitted.values().filter_map(|id| self.part(id)) {
            grip += p.grip;
            d.weight_lb += p.weight_lb;
            if !p.gears.is_empty() {
                d.gears = p.gears.clone();
            }
            if let Some(f) = p.final_drive {
                d.final_drive = f;
            }
            if let Some(r) = p.redline {
                d.redline = r;
            }
            if let Some(auto) = p.auto {
                d.auto = auto;
            }
            if !p.curve.is_empty() {
                d.torque = p.curve.clone();
            }
        }
        // Worn tires have up to three tenths less bite.
        d.grip *= grip.max(10) as f64 / 100.0 * (1.0 - o.tires.clamp(0.0, 100.0) * 0.003);
        d.weight_lb = d.weight_lb.max(500.0);
        d.launch = d.launch.min(d.redline - 400.0);
        d
    }

    /// What blowing an engine up does to it: it runs, barely, until it is rebuilt.
    /// Which way the car in the bay pulls under power, and how hard: the same for a car all day,
    /// to one side or the other, give or take a fifth.
    pub fn lean(&self) -> f64 {
        let Some(o) = self.owned() else { return 0.0 };
        let k = hash(o.car.bytes().fold(self.day() as u32 * 977, |a, b| a.wrapping_mul(31).wrapping_add(b as u32)));
        (if k & 1 == 0 { 1.0 } else { -1.0 }) * (0.8 + (k >> 8) as f64 % 40.0 / 100.0)
    }

    fn blew(&mut self) {
        if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
            o.cond = (o.cond - 20).max(25);
        }
        self.say("You blew the engine. It needs a rebuild.");
        self.save();
    }

    /// The engine speed the car in the bay launches best from, if it has a clutch to launch with.
    pub fn launch_speed(&self) -> f64 {
        let Some((o, car)) = self.owned().zip(self.car()) else { return 0.0 };
        let d = self.tuned_drive(o, car);
        if d.auto { 0.0 } else { d.launch }
    }

    /// A clutch let out with the revs over the band: it slips, and burns, a point for every
    /// hundred and fifty revs over.
    pub fn ride_clutch(&mut self, over: f64) {
        if over <= 0.0 {
            return;
        }
        let Some(cur) = self.state.cur else { return };
        let found = self.state.cars[cur].fitted.iter().find(|(_, id)| cond_of(id) > 0 && self.part(id).is_some_and(|p| p.slips)).map(|(s, id)| (s.clone(), id.clone()));
        if let Some((slot, id)) = found {
            let left = cond_of(&id) - (over / 150.0).ceil() as i32;
            self.state.cars[cur].fitted.insert(slot, worn(&id, left.max(0)));
        }
    }

    /// A transmission that changes itself, worked out from the car's own ratios: up near the redline
    /// with the foot down and early when cruising, down again as the car slows, so a change drops
    /// the revs by what the next ratio drops them, not to idle. What it comes to is kept in `auto`.
    pub fn shift_auto(&mut self, speed: f64, gas: bool) {
        let Some((o, car)) = self.owned().zip(self.car()) else { return };
        let d = self.tuned_drive(o, car);
        if d.gears.is_empty() {
            return;
        }
        // Meters a second at the wheel to engine speed, through a gear.
        let revs = |g: usize| speed / 0.3 * d.gears[g] * d.final_drive * 60.0 / std::f64::consts::TAU;
        let (mut gear, top) = (self.auto.0.min(d.gears.len() - 1), d.gears.len() - 1);
        let (up, down) = if gas { (d.redline - 400.0, 2600.0) } else { (2800.0, 1300.0) };
        while gear < top && revs(gear) > up {
            gear += 1;
        }
        while gear > 0 && revs(gear) < down && revs(gear - 1) < d.redline - 600.0 {
            gear -= 1;
        }
        // Pulling away, the converter or the clutch lets the engine run above the wheels.
        let floor = if gas { 1800.0 } else { 850.0 };
        let rpm = revs(gear).clamp(floor, d.redline);
        self.auto = (gear, rpm);
    }

    /// What rebuilding the engine of the car in the bay costs: the pack's price, and a quarter of
    /// what the car costs in the paper, since a bigger engine is more to rebuild.
    pub fn rebuild_cost(&self) -> i32 {
        self.pack.manifest.rebuild.cost + self.car().and_then(|c| c.price).unwrap_or(0) / 4
    }

    /// What a car like this one costs in the paper, and what the dealer gives for yours.
    fn list_price(&self, car: &str, cond: i32) -> i32 {
        self.pack.manifest.cars.get(car).and_then(|c| c.price).unwrap_or(300) * cond / 70
    }

    pub fn value(&self, o: &Owned) -> i32 {
        if let Some(car) = self.pack.manifest.cars.get(&o.car).filter(|_| o.sheet) {
            // A car that is its parts is worth its shell and what is on it: the parts it came
            // with for an eighth of their price, anything better for two fifths, each for as much
            // of that as is left of it. A shell too far gone is only scrap.
            let shell = self.shell_offer(o).unwrap_or(40);
            return shell + o.fitted.iter().filter_map(|(slot, id)| Some((slot, id, self.part(id)?))).map(|(slot, id, p)| p.price * cond_of(id) / 100 * if car.stock.get(slot).map(String::as_str) == Some(base(id)) { 12 } else { 40 } / 100).sum::<i32>();
        }
        self.list_price(&o.car, o.cond) * 6 / 10 * (200 - o.body.clamp(0, 100)) / 200 + o.fitted.values().filter_map(|p| self.part(p)).map(|p| p.price / 2).sum::<i32>()
    }

    /// What a buyer gives for the bare shell, if it is good enough to sell: the car's price less
    /// what is wrong with its body.
    pub fn shell_offer(&self, o: &Owned) -> Option<i32> {
        let car = self.pack.manifest.cars.get(&o.car)?;
        (o.body <= 70).then(|| car.price.unwrap_or(300) / 4 * (200 - o.body.clamp(0, 100)) / 200)
    }

    /// What the scrapyard gives: the steel, and a little for each part still on it.
    pub fn scrap_offer(&self, o: &Owned) -> i32 {
        50 + o.fitted.values().filter_map(|id| Some(self.part(id)?.price * cond_of(id) / 100 * 6 / 100)).sum::<i32>()
    }

    /// The ways to get rid of a car with a build sheet: to the dealer if it runs, the shell with
    /// the parts stripped onto the shelf, or all of it for scrap. What each brings, if it can be done.
    pub fn partings(&self, o: &Owned) -> [Option<i32>; 3] {
        [self.wont_run_of(o).is_none().then(|| self.value(o)), self.shell_offer(o), Some(self.scrap_offer(o))]
    }

    /// What they give today for a part off the shelf: a used factory part 15 to 24 hundredths of
    /// what it cost new, anything else 55 to 74, and less as it is worn.
    pub fn spare_offer(&self, id: &str) -> i32 {
        let n = self.pack.manifest.parts.keys().position(|k| k == base(id)).unwrap_or(0) as u32;
        // A used factory part is worth a good deal less than a used part someone paid extra for.
        let stock = self.pack.manifest.cars.values().any(|c| c.stock.values().any(|s| s == base(id)));
        let (least, more) = if stock { (15, 10) } else { (55, 20) };
        self.part(id).map(|p| p.price).unwrap_or(0) * (least + (hash(self.day() as u32 * 16 + n + 3) >> 12) as i32 % more) / 100 * cond_of(id) / 100
    }

    /// What selling the spare on the shelf at `i` brings, and which spares go: a part that needs
    /// others goes with them as a set if they are on the shelf, for more than apart; alone, a
    /// buyer has to find the rest, and pays less.
    pub fn sale(&self, i: usize) -> (i32, Vec<usize>) {
        let Some(id) = self.state.spares.get(i) else { return (0, Vec::new()) };
        let needs = self.part(id).map(|p| p.needs.clone()).unwrap_or_default();
        if needs.is_empty() {
            return (self.spare_offer(id), vec![i]);
        }
        let mut set = vec![i];
        for need in &needs {
            let found = self.state.spares.iter().enumerate().find(|(j, other)| !set.contains(j) && self.part(other).is_some_and(|p| p.gives.contains(need)));
            match found {
                Some((j, _)) => set.push(j),
                None => return (self.spare_offer(id) * 80 / 100, vec![i]),
            }
        }
        (set.iter().map(|&j| self.spare_offer(&self.state.spares[j])).sum::<i32>() * 115 / 100, set)
    }

    /// The parts the paper sells new: all but the rare factory ones.
    /// The parts in the paper: those that go on the car in the bay, or all of them with no car.
    pub fn shop(&self) -> Vec<(&String, &Part)> {
        let car = self.owned().map(|o| o.car.clone());
        let mut all: Vec<(&String, &Part)> = self.pack.manifest.parts.iter().filter(|(_, p)| p.rare == 0 && car.as_ref().is_none_or(|c| p.goes_on(c))).collect();
        // Grouped by where on the car they go, and cheapest first in each.
        all.sort_by_key(|(_, p)| (self.group_of(p), p.price));
        all
    }

    /// Which group a part is in on the parts page: one for each place on the car, then the rest.
    fn group_of(&self, p: &Part) -> usize {
        let Some(car) = self.car() else { return 0 };
        car.area_of(&p.slot).map_or(car.areas().len(), |k| k) + 1
    }

    /// The groups across the top of the parts page, when the car in the bay has places to work on.
    pub fn shop_groups(&self) -> Vec<String> {
        let Some(car) = self.car().filter(|c| !c.areas().is_empty()) else { return Vec::new() };
        let shop = self.shop();
        let mut out = vec!["ALL".to_string()];
        for (k, a) in car.areas().iter().enumerate() {
            if shop.iter().any(|(_, p)| self.group_of(p) == k + 1) {
                out.push(a.0.to_uppercase().chars().take(6).collect());
            } else {
                out.push(String::new());
            }
        }
        if shop.iter().any(|(_, p)| self.group_of(p) == car.areas().len() + 1) {
            out.push("OTHER".to_string());
        }
        out
    }

    /// What the parts page shows: all of them, or one group.
    pub fn shop_page(&self) -> Vec<(&String, &Part)> {
        let g = self.shop_group;
        self.shop().into_iter().filter(|(_, p)| g == 0 || self.group_of(p) == g).collect()
    }

    /// Where the list on the parts page starts: lower when the groups are across the top.
    fn shop_top(&self) -> i32 {
        if self.shop_groups().is_empty() { 56 } else { 66 }
    }

    /// The group buttons: (x, width, words, group), leaving out groups with nothing in them.
    fn group_chips(&self) -> Vec<(i32, i32, String, usize)> {
        let groups: Vec<(usize, String)> = self.shop_groups().into_iter().enumerate().filter(|(_, w)| !w.is_empty()).collect();
        let w = 304 / groups.len().max(1) as i32;
        groups.into_iter().enumerate().map(|(k, (g, words))| (8 + k as i32 * w, w - 2, words, g)).collect()
    }

    /// What a part does, in a few words, for the paper.
    fn does(&self, p: &Part) -> String {
        let mut said = Vec::new();
        if p.torque != 0 {
            said.push(format!("{:+}% power", p.torque));
        }
        if p.boost.is_some() {
            said.push("boost".to_string());
        }
        if !p.curve.is_empty() {
            said.push("a new engine".to_string());
        }
        if p.grip != 0 {
            said.push(format!("{:+}% grip", p.grip));
        }
        if let Some(b) = p.brakes {
            said.push(format!("{:+}% braking", b));
        }
        if let Some(h) = p.handling {
            said.push(format!("{:+}% handling", h));
        }
        if !p.gears.is_empty() {
            said.push(format!("{} speeds", p.gears.len()));
        }
        if let Some(r) = p.redline {
            said.push(format!("{} redline", r as i32));
        }
        if p.weight_lb.abs() >= 5.0 {
            said.push(format!("{:+} lb", p.weight_lb as i32));
        }
        if p.nitrous.is_some() {
            said.push("nitrous".to_string());
        }
        if p.tuner {
            said.push("tune at the rollers".to_string());
        }
        if p.radar {
            said.push("warns of police".to_string());
        }
        let at = self.car().and_then(|c| Some(c.areas()[c.area_of(&p.slot)?].1.to_string()));
        let hood = at.map_or(String::new(), |t| format!(" {t}."));
        if said.is_empty() {
            format!("{}.{hood}", p.slot)
        } else {
            format!("{}: {}.{hood}", p.slot, said.join(", "))
        }
    }

    /// Whether a part is on the car in the bay, or on the shelf.
    fn part_where(&self, id: &str) -> Option<&'static str> {
        let on = |p: &String| base(p) == id;
        if self.owned().is_some_and(|o| o.fitted.values().any(on)) {
            Some("ON CAR")
        } else if self.state.spares.iter().any(on) {
            Some("SHELF")
        } else if self.state.cars.iter().any(|c| c.fitted.values().any(on)) {
            Some("OWNED")
        } else {
            None
        }
    }

    /// The button on the paper's parts page that goes to the bench, when there is something to fit.
    const FIT_BUTTON: [i32; 4] = [220, 185, 92, 14];

    fn fit_button_shown(&self) -> bool {
        self.owned().is_some() && !self.state.spares.is_empty() && self.state.place == "garage"
    }

    /// Today's cars in the paper: three, picked by the day from the cars that have a price.
    pub fn offers(&self) -> Vec<Option<Offer>> {
        let m = &self.pack.manifest;
        let sale: Vec<(&String, &Car)> = m.cars.iter().filter(|(_, c)| c.price.is_some()).collect();
        if sale.is_empty() {
            return Vec::new();
        }
        let paints = self.paints();
        let day = self.day();
        (0..3usize).map(|i| {
            if self.state.sold.0 == day && self.state.sold.1.contains(&i) {
                return None;
            }
            let h = hash(day as u32 * 3 + i as u32 + 1);
            let (id, car) = sale[(h >> 4) as usize % sale.len()];
            let cond = 55 + (h >> 8) as i32 % 26;
            let off = if self.state.knocked.0 == day { self.state.knocked.1.iter().find(|k| k.0 == i).map_or(0, |k| k.1) } else { 0 };
            let price = (self.list_price(id, cond) + (h >> 16) as i32 % 40) * (100 - off) / 100;
            let paint = if car.paint_ramps.mid.is_some() { paints[h as usize % paints.len()].clone() } else { car.paint.clone().unwrap_or_default() };
            Some(Offer { car: id.clone(), paint, cond, price })
        }).collect()
    }

    pub fn rival(&self) -> Option<&Rival> {
        self.pack.manifest.rivals.get(self.who)
    }

    /// What can be raced for: nothing, each of the pack's bets, and cars.
    /// The help-wanted ads: the pack's, or three ordinary ones. A way to money that needs no car.
    pub fn jobs(&self) -> Vec<Job> {
        let m = &self.pack.manifest;
        if !m.jobs.is_empty() {
            return m.jobs.clone();
        }
        [("Sweep up at Rix Racing", 3, 25), ("Run parts for the store", 6, 55), ("Night shift at the car wash", 8, 70)]
            .iter()
            .map(|&(name, hours, pay)| Job { name: name.into(), hours, pay })
            .collect()
    }

    fn stakes(&self) -> Vec<i32> {
        let m = &self.pack.manifest;
        if m.bets.is_empty() { BETS.to_vec() } else { m.bets.clone() }
    }

    /// Why the car in the bay will not start, if it will not: something under the hood is off, or
    /// not bolted down.
    pub fn wont_run(&self) -> Option<String> {
        self.wont_run_of(self.owned()?)
    }

    /// Why a car of yours will not start, if it will not.
    pub fn wont_run_of(&self, o: &Owned) -> Option<String> {
        let car = self.pack.manifest.cars.get(&o.car)?;
        // Something done for, something that wants a part that is not there, or nothing where a part should be.
        for id in o.fitted.values() {
            let Some(p) = self.part(id) else { continue };
            if cond_of(id) == 0 {
                return Some(format!("The {} is done for.", p.name.to_lowercase()).chars().take(38).collect());
            }
            if p.charges && cond_of(id) <= 15 {
                return Some(format!("The {} is flat. Charge it.", p.name.to_lowercase()).chars().take(38).collect());
            }
            let has = |need: &String| o.fitted.values().filter_map(|other| self.part(other)).any(|other| other.gives.contains(need));
            if let Some(need) = p.needs.iter().find(|n| !has(n)) {
                return Some(format!("{} needs {need}.", p.name).chars().take(38).collect());
            }
        }
        if o.sheet {
            if let Some(slot) = car.stock.keys().find(|s| !o.fitted.contains_key(*s) && !car.optional.contains(s)) {
                return Some(format!("It has no {}.", slot.to_lowercase()));
            }
        }
        car.areas().into_iter().flat_map(|a| a.3.iter()).find_map(|(slot, place)| {
            let n = place.bolts.len();
            if o.off.contains(slot) {
                Some(format!("The {} {} off. It will not run.", slot.to_lowercase(), is_are(slot)))
            } else if o.tight(slot, n).count_ones() as usize != n {
                Some(format!("The {} {} not bolted down.", slot.to_lowercase(), is_are(slot)))
            } else {
                None
            }
        })
    }

    /// Why the car in the bay cannot make a pass, if it cannot: it will not run, or it is nearly dry.
    fn cannot_race(&self) -> Option<String> {
        self.wont_run().or_else(|| self.owned().filter(|o| o.gas < 0.7).map(|_| "Not enough gas to race. Go fill up.".to_string()))
    }

    /// A knock: the body of the car in the bay is the worse for it.
    pub fn dent(&mut self) {
        if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
            o.body = (o.body + 8).min(100);
        }
    }

    /// Use wears parts out: `passes` is how hard, a pass down the strip being one. What wore
    /// out altogether, if anything did; failing that, what is newly close to it. Running also
    /// puts charge back in the battery.
    pub fn use_parts(&mut self, passes: f64) -> Option<(String, bool)> {
        let cur = self.state.cur?;
        let fitted: Vec<(String, String)> = self.state.cars[cur].fitted.iter().map(|(s, id)| (s.clone(), id.clone())).collect();
        let (mut gone, mut tired) = (None, None);
        for (slot, id) in fitted {
            let Some((wears, charges, name, takes)) = self.part(&id).map(|p| (p.wears, p.charges, p.name.clone(), p.takes.clone())) else { continue };
            let was = cond_of(&id);
            if charges && was > 0 && was < 100 {
                self.state.cars[cur].fitted.insert(slot, worn(&id, (was + (passes * 8.0).ceil() as i32).min(100)));
                continue;
            }
            if wears <= 0.0 || was == 0 {
                continue;
            }
            // Whole points, and the odds of one more for what is left over.
            let lost = wears * passes;
            let more = ((lost.fract() * 1000.0) as u32 > self.rand() % 1000) as i32;
            let left = (was - lost as i32 - more).max(0);
            self.state.cars[cur].fitted.insert(slot, worn(&id, left));
            if left == 0 {
                for slot in &takes {
                    if let Some(other) = self.state.cars[cur].fitted.get(slot).cloned() {
                        self.state.cars[cur].fitted.insert(slot.clone(), worn(&other, 0));
                    }
                }
                gone = Some(name);
            } else if left < 25 && was >= 25 {
                tired = Some(name);
            }
        }
        gone.map(|n| (n, true)).or(tired.map(|n| (n, false)))
    }

    /// Road under the car: every kilometer of it is a quarter of a pass to the parts.
    pub fn roll(&mut self, km: f64) {
        let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) else { return };
        o.run += km;
        if o.run < 1.0 {
            return;
        }
        o.run -= 1.0;
        match self.use_parts(0.25) {
            Some((name, true)) => self.no(&format!("The {} let go.", name.to_lowercase()).chars().take(38).collect::<String>()),
            Some((name, false)) => self.say(&format!("The {} will not last.", name.to_lowercase()).chars().take(38).collect::<String>()),
            None => {}
        }
    }

    /// What is left in a car's battery goes down by `by`, but sitting never kills one: it is flat.
    fn drain(&mut self, car: usize, by: i32) -> bool {
        let found = self.state.cars[car].fitted.iter().find(|(_, id)| cond_of(id) > 5 && self.part(id).is_some_and(|p| p.charges)).map(|(s, id)| (s.clone(), id.clone()));
        let Some((slot, id)) = found else { return false };
        self.state.cars[car].fitted.insert(slot, worn(&id, (cond_of(&id) - by).max(5)));
        true
    }

    /// Rubber left on the road.
    pub fn scrub(&mut self, wear: f64) {
        if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
            o.tires = (o.tires + wear).min(100.0);
        }
    }

    fn burn(&mut self, gallons: f64) {
        if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
            o.gas = (o.gas - gallons).max(0.0);
        }
    }

    // ---- Time and endings ----

    /// Everything takes time. When the days run out, or there is no way to get a car, it is over.
    /// Minutes of work on the car: they add up, and every whole hour of them is spent.
    pub(super) fn labor(&mut self, minutes: f64) {
        self.clock += minutes / 60.0;
        while self.clock >= 1.0 {
            self.clock -= 1.0;
            self.spend(1);
        }
    }

    pub fn spend(&mut self, hours: i32) {
        let before = self.day();
        self.state.hours += hours;
        self.unload();
        // A car that sits loses a little from its battery every day it is not the one driven.
        let days = self.day() - before;
        for car in 0..self.state.cars.len() {
            if days > 0 && Some(car) != self.state.cur {
                self.drain(car, 3 * days);
            }
        }
        self.check_over();
        self.save();
    }

    /// Trailers that have got here: their cars go in the yard, or in the bay if it is empty.
    fn unload(&mut self) {
        let day = self.day();
        while let Some(k) = self.state.coming.iter().position(|c| c.0 <= day) {
            if self.state.cars.len() >= MOST_CARS {
                return;
            }
            let (_, car, from) = self.state.coming.remove(k);
            self.state.cars.push(car);
            if self.state.cur.is_none() {
                self.state.cur = Some(self.state.cars.len() - 1);
            }
            self.say(&format!("The car from {from} is here.").chars().take(38).collect::<String>());
            tone(880, 80);
        }
    }

    pub fn check_over(&mut self) {
        let m = &self.pack.manifest;
        if self.day() > m.season.days {
            self.scene = Scene::Over(1);
        } else if self.state.cars.is_empty() && self.state.coming.is_empty() {
            let cheapest = m.cars.keys().filter(|id| m.cars[*id].price.is_some()).map(|id| self.list_price(id, 55)).min();
            let spare: i32 = self.state.spares.iter().map(|p| self.spare_offer(p)).sum();
            // What working could still bring in before the season is out.
            let left = m.season.days * m.season.hours_per_day - self.state.hours;
            let earn = self.jobs().iter().map(|j| left / j.hours * j.pay).max().unwrap_or(0);
            if cheapest.is_none_or(|c| self.state.cash + spare + earn < c) {
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
        self.road_race = None;
        if let Some(road) = action.strip_prefix("pokerrun:") {
            self.start_poker(&road.to_string());
        } else if let Some(road) = action.strip_prefix("roadrace:") {
            if self.state.cur.is_none() {
                self.no("You have nothing to race.");
            } else if let Some(why) = self.cannot_race() {
                self.no(&why);
            } else {
                self.road_race = Some(road.to_string());
                self.scene = Scene::Bets;
            }
        } else if let Some(to) = action.strip_prefix("go:") {
            tone(520, 60);
            self.enter(&to.to_string());
        } else if let Some(words) = action.strip_prefix("say:") {
            self.say(words);
        } else if let Some(road) = action.strip_prefix("road:") {
            if self.laid(road) {
                self.onto_road(&road.to_string());
            } else {
                self.take_road(&road.to_string(), None);
            }
        } else if let Some(id) = action.strip_prefix("work:") {
            // Straight to one place on the car: the trunk, the wheels...
            let Some(k) = self.car().and_then(|c| c.areas().iter().position(|x| x.0 == id)) else { return self.say("Nothing to work on there.") };
            self.area = k;
            self.scene = Scene::Hood;
            self.carrying = None;
        } else if let Some(map) = action.strip_prefix("drive:") {
            let Some(m) = self.pack.manifest.maps.get(map) else { return };
            if self.state.cur.is_none() {
                return self.no("You have nothing to drive.");
            }
            if let Some(why) = self.wont_run() {
                return self.no(&why);
            }
            if self.owned().is_some_and(|o| o.gas < 0.3) {
                // Dry at home: walk to the pump with a can. A gallon, two dollars, and an hour gone.
                if self.state.cash < 2 {
                    return self.no("No gas, and no $2 for a can of it.");
                }
                self.state.cash -= 2;
                if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
                    o.gas += 1.0;
                }
                self.say("A can from the pump: a gallon, $2.");
                tone(520, 80);
                return self.spend(1);
            }
            let mut rng = self.rng;
            let town = Town::new(map, m, Some(&self.state.place), || {
                rng ^= rng << 13;
                rng ^= rng >> 17;
                rng ^= rng << 5;
                rng
            });
            self.rng = rng;
            self.town = Some(town);
            self.dash = None;
            self.fill_roads();
            self.fit_town();
            self.town_by_road = false;
            self.scene = Scene::Drive;
        } else {
            match action {
                "lamps" => {
                    self.lamps = !self.lamps;
                    tone(if self.lamps { 440 } else { 330 }, 70);
                }
                "calendar" => {
                    let (day, days) = (self.day(), self.pack.manifest.season.days);
                    self.say(&format!("Day {day} of {days}. Won {}, lost {}.", self.state.wins, self.state.losses));
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
                "hood" | "work" => {
                    // Under the hood, or the first place on the car there is to work on.
                    let first = if action == "hood" { self.car().and_then(|c| c.areas().iter().position(|x| x.0 == "hood")) } else { None };
                    if self.car().is_some_and(|c| !c.areas().is_empty()) {
                        self.area = first.unwrap_or(0);
                        self.scene = Scene::Hood;
                        self.carrying = None;
                    } else {
                        self.say("Nothing to work on there.");
                    }
                }
                "yard" => self.scene = Scene::Yard,
                "online" => self.go_online(),
                "shootout" => {
                    let Some(sh) = self.pack.manifest.shootout.clone() else { return self.no("Nobody is holding one.") };
                    let day = self.day();
                    if day % sh.every != 0 {
                        return self.say(&format!("The next shootout is on day {}.", (day / sh.every + 1) * sh.every));
                    }
                    if self.state.entered == day {
                        return self.say("You have had your go today.");
                    }
                    if self.state.cur.is_none() {
                        return self.no("You have nothing to enter.");
                    }
                    if let Some(why) = self.cannot_race() {
                        return self.no(&why);
                    }
                    if self.state.cash < sh.entry {
                        return self.no(&format!("It is ${} to enter. You don't have it.", sh.entry));
                    }
                    self.state.cash -= sh.entry;
                    self.state.entered = day;
                    self.bracket = Some(0);
                    self.bracket_round();
                }
                "charge" => {
                    let Some(cur) = self.state.cur else { return self.no("There is no car in the bay.") };
                    let found = self.state.cars[cur].fitted.iter().find(|(_, id)| self.part(id).is_some_and(|p| p.charges)).map(|(s, id)| (s.clone(), id.clone()));
                    let Some((slot, id)) = found else { return self.no("There is no battery in it.") };
                    match cond_of(&id) {
                        0 => return self.no("It will not take a charge. It is done."),
                        90.. => return self.say("It is charged."),
                        _ => {}
                    }
                    if self.state.cash < 5 {
                        return self.no("A charge is $5. You don't have it.");
                    }
                    self.state.cash -= 5;
                    self.state.cars[cur].fitted.insert(slot, worn(&id, 90));
                    self.say("On the charger for two hours: $5.");
                    tone(700, 120);
                    self.spend(2);
                }
                "bodywork" => {
                    let Some(cur) = self.state.cur else { return self.no("There is no car in the bay.") };
                    let body = self.state.cars[cur].body;
                    if body == 0 {
                        return self.say("There is not a mark on it.");
                    }
                    let cost = 20 + body * 4;
                    if self.state.cash < cost {
                        return self.no(&format!("Straightening it is ${cost}. You don't have it."));
                    }
                    self.state.cash -= cost;
                    self.state.cars[cur].body = 0;
                    self.say(&format!("Straight again, for ${cost}. Three hours."));
                    tone(700, 120);
                    self.spend(3);
                }
                "tires" | "tyres" => {
                    let Some(cur) = self.state.cur else { return self.no("There is no car in the bay.") };
                    if self.state.cars[cur].tires < 5.0 {
                        return self.say("They are nearly new.");
                    }
                    if self.state.cash < 80 {
                        return self.no("A set of tires is $80. You don't have it.");
                    }
                    self.state.cash -= 80;
                    self.state.cars[cur].tires = 0.0;
                    self.say("A new set of tires, for $80.");
                    tone(700, 120);
                    self.save();
                }
                "sleep" => {
                    // Bed: whatever is left of today is gone, and it is morning.
                    let per = self.pack.manifest.season.hours_per_day.max(1);
                    // Lamps left burning all night: most of a battery.
                    let left_on = self.lamps && self.state.cur.is_some_and(|cur| self.drain(cur, 45));
                    self.lamps = false;
                    self.spend(per - self.state.hours % per);
                    if self.scene == Scene::Place {
                        self.say(if left_on { "You left the lamps on all night." } else { "You slept. It is morning." });
                    }
                }
                "nitrous" => {
                    let Some((kit, cur)) = self.owned().and_then(|o| self.kit(o)).cloned().zip(self.state.cur) else { return self.no("There is no bottle on this car.") };
                    if self.state.cars[cur].bottle >= kit.seconds - 0.05 {
                        return self.say("The bottle is full.");
                    }
                    if self.state.cash < kit.fill {
                        return self.no(&format!("A fill is ${}. You don't have it.", kit.fill));
                    }
                    self.state.cash -= kit.fill;
                    self.state.cars[cur].bottle = kit.seconds;
                    self.say(&format!("A full bottle: {:.0} seconds. That was ${}.", kit.seconds, kit.fill));
                    tone(700, 80);
                    self.save();
                }
                "dyno" => {
                    if let Some(why) = self.wont_run() {
                        return self.no(&why);
                    }
                    if self.owned().is_none() {
                        return self.no("There is no car in the bay.");
                    }
                    self.sheet = None;
                    self.rollers = dyno::Rollers::default();
                    self.scene = Scene::Dyno;
                }
                "paint" => {
                    if self.state.cur.is_none() {
                        return self.no("There is no car in the bay.");
                    }
                    self.scene = Scene::Paint;
                }
                "setup" => {
                    // Adjustable suspension: round from even, stiffer for the strip, then softer for the road.
                    let Some(cur) = self.state.cur else { return self.no("There is no car in the bay.") };
                    let o = &self.state.cars[cur];
                    if !o.fitted.values().any(|id| cond_of(id) > 0 && self.part(id).is_some_and(|p| p.adjustable)) {
                        return self.no("Nothing on it to adjust.");
                    }
                    let next = match o.setup { 0 => 1, 1 => 2, 2 => -1, -1 => -2, _ => 0 };
                    self.state.cars[cur].setup = next;
                    self.say(match next {
                        2 => "Set for the strip: it sits down hard.",
                        1 => "A little stiffer for the strip.",
                        -1 => "A little softer for the road.",
                        -2 => "Set for the road: flat through bends.",
                        _ => "Set even: half and half.",
                    });
                    tone(560, 60);
                    self.spend(1);
                }
                "ghosts" => {
                    // Other people's cars: something to drive through, or something to hit.
                    self.state.solid = !self.state.solid;
                    self.say(if self.state.solid { "Other people's cars are solid now." } else { "Other people's cars are ghosts now." });
                    tone(if self.state.solid { 520 } else { 400 }, 60);
                    self.save();
                }
                "tuneup" => {
                    // The shop's tune: back to the factory map, made to suit the parts on it.
                    let (cost, hours) = (self.pack.manifest.tuneup.cost, self.pack.manifest.tuneup.hours);
                    let Some(cur) = self.state.cur else { return self.no("There is no car in the bay.") };
                    let o = &self.state.cars[cur];
                    if !o.untuned && o.ecu == Ecu::default() {
                        return self.say("It runs right. It does not need one.");
                    }
                    if self.state.cash < cost {
                        return self.no(&format!("A tune is ${cost}. You don't have it."));
                    }
                    self.state.cash -= cost;
                    let o = &mut self.state.cars[cur];
                    (o.untuned, o.ecu) = (false, Ecu::default());
                    self.say(&format!("Tuned on the factory map: ${cost}."));
                    tone(700, 120);
                    self.spend(hours);
                }
                "rebuild" => {
                    let Some(cur) = self.state.cur else { return self.no("There is no car in the bay.") };
                    let (cost, cond) = (self.rebuild_cost(), self.pack.manifest.rebuild.cond.clamp(1, 100));
                    if self.state.cars[cur].cond >= cond {
                        return self.say("The engine is as good as a rebuild would make it.");
                    }
                    if self.state.cash < cost {
                        return self.no(&format!("A rebuild is ${cost}. You don't have it."));
                    }
                    self.state.cash -= cost;
                    self.state.cars[cur].cond = cond;
                    self.say(&format!("Rebuilt, for ${cost}. That was four hours."));
                    tone(700, 120);
                    self.spend(4);
                }
                "gas" => {
                    // A dollar buys three gallons.
                    let Some((o, car)) = self.owned().zip(self.car()) else { return self.no("You have nothing to fill.") };
                    let want = (car.tank - o.gas).max(0.0);
                    let cost = (want / 3.0).ceil() as i32;
                    if want < 0.05 {
                        return self.say("It is full.");
                    }
                    if self.state.cash < 1 {
                        return self.no("Gas costs money.");
                    }
                    let pay = cost.min(self.state.cash);
                    let (tank, cur) = (car.tank, self.state.cur.unwrap());
                    let o = &mut self.state.cars[cur];
                    o.gas = (o.gas + pay as f64 * 3.0).min(tank);
                    self.state.cash -= pay;
                    self.say(&format!("{:.1} gallons in the tank. That was ${pay}.", self.state.cars[cur].gas));
                    tone(700, 80);
                    self.save();
                }
                "race" => {
                    if self.state.cur.is_none() {
                        self.no("You have nothing to race.");
                    } else if let Some(why) = self.cannot_race() {
                        self.no(&why);
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
                    } else if let Some(why) = self.cannot_race() {
                        self.no(&why);
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
        if let Some(road) = self.road_race.take() {
            return self.start_road_race(&road);
        }
        let Some((o, car)) = self.owned().zip(self.car()) else { return };
        let (gain, boost) = self.tune(o);
        let drive = self.strip_drive(o, car);
        let spray = self.kit(o).map(|kit| (kit.gain as f64 / 100.0, o.bottle));
        let mut drive = drive;
        if self.raining() {
            // A wet strip: a fifth less bite off the line.
            drive.grip *= 0.8;
        }
        let rt = 280 + (self.rand() % 260) as i32;
        self.burn(0.5);
        // A launch is hard on the tires.
        self.scrub(2.0);
        let mut race = Race::new(&drive, gain, boost, rival_et, rt);
        race.lean = self.lean();
        if let Some(spray) = spray {
            (race.shot, race.bottle) = spray;
        }
        self.race = Some(race);
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
        // Someone you have beaten before is wary: he bets half what he would.
        let limit = if self.who < self.state.rung { r.limit / 2 } else { r.limit };
        if stake > limit || (pinks && !r.king && (!r.pinks || self.rand() % 3 == 0)) {
            return self.no("No way. Too rich for me.");
        }
        let today = if self.state.paid_up.0 == self.day() { self.state.paid_up.1.clone() } else { Vec::new() };
        if stake > 0 && today.contains(&self.who) {
            return self.no("You had my money today. Tomorrow.");
        }
        // Two paid up in a day and word gets round: nobody else bets you until tomorrow.
        if stake > 0 && today.len() >= PAID_A_DAY && self.who != self.state.rung {
            return self.no("Word is out on you. Not today.");
        }
        self.bet = b;
        let et = self.on_the_day(r.et);
        self.start_race(et);
    }

    /// What a rival runs today, in milliseconds: nobody runs the same number twice. Mostly within
    /// a tenth or two of his best, now and then well off it, now and then a little under.
    pub fn on_the_day(&mut self, et: f64) -> i32 {
        let spread = et * 1000.0 * 0.012;
        // Three draws added up lean to the middle; a bad day is longer than a good one.
        let k = (0..3).map(|_| self.rand() % 1001).sum::<u32>() as f64 / 1500.0 - 1.0;
        (et * 1000.0 + spread * if k > 0.0 { k * 1.6 } else { k }) as i32
    }

    /// The race is run. `won` is None for a pass with nobody in the other lane.
    /// The next round of the shootout: whoever it throws up, each a rung quicker than the last,
    /// starting a rung under where you stand on the ladder.
    fn bracket_round(&mut self) {
        let (Some(round), last) = (self.bracket, self.pack.manifest.rivals.len().saturating_sub(1)) else { return };
        self.who = (self.state.rung.saturating_sub(1) + round).min(last);
        self.bet = 0;
        self.road_race = None;
        let et = self.pack.manifest.rivals[self.who].et;
        let et = self.on_the_day(et);
        self.start_race(et);
    }

    /// A round of the shootout is run: through to the next, out, or the purse.
    fn settle_bracket(&mut self, won: bool, round: usize) {
        let Some(sh) = self.pack.manifest.shootout.clone() else { return };
        if won { self.state.wins += 1 } else { self.state.losses += 1 }
        self.ping();
        let home = self.state.rung.min(self.pack.manifest.rivals.len().saturating_sub(1));
        if !won {
            self.bracket = None;
            self.who = home;
            self.say(&format!("Out in round {}. There is always the next.", round + 1));
            return self.spend(2);
        }
        if round + 1 >= sh.rounds {
            self.bracket = None;
            self.who = home;
            self.state.cash += sh.purse;
            tone(990, 400);
            self.say(&format!("You won the shootout. ${} is yours.", sh.purse));
            return self.spend(2);
        }
        self.bracket = Some(round + 1);
        self.spend(2);
        if self.scene != Scene::Place {
            // The season ran out between rounds.
            self.bracket = None;
        } else if self.cannot_race().is_some() {
            self.bracket = None;
            self.who = home;
            self.say("You could not make the next round.");
        } else {
            self.bracket_round();
        }
    }

    fn settle(&mut self, won: Option<bool>) {
        self.race = None;
        self.scene = Scene::Place;
        if let (Some(round), Some(won)) = (self.bracket, won) {
            return self.settle_bracket(won, round);
        }
        let Some(won) = won else {
            self.say("A pass on your own. Nobody saw it.");
            return self.spend(1);
        };
        let Some(r) = self.rival().cloned() else { return };
        let stakes = self.stakes();
        let pinks = self.bet == stakes.len() + 1;
        let stake = if self.bet == 0 || pinks { 0 } else { stakes[self.bet - 1] };
        if won { self.state.wins += 1 } else { self.state.losses += 1 }
        self.ping();
        let broke = self.strain_driveline();
        // A pass is a pass to every part that wears. One that goes takes the car with it.
        let used = self.use_parts(1.0);
        let tired = used.clone().filter(|w| !w.1 && broke.is_none()).map(|w| w.0);
        let broke = broke.or(used.filter(|w| w.1).map(|w| w.0));
        if won {
            if pinks && r.king {
                self.scene = Scene::Over(3);
                return self.save();
            }
            if pinks {
                let mut theirs = Owned::new(&r.car, r.paint.clone(), (62 + self.who as i32).min(100));
                self.outfit(&mut theirs);
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
                if stake > 0 {
                    let day = self.day();
                    if self.state.paid_up.0 != day {
                        self.state.paid_up = (day, Vec::new());
                    }
                    self.state.paid_up.1.push(self.who);
                }
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
        if let Some(name) = broke {
            // More than it would take: whatever the race came to, the car is not going anywhere.
            self.say(&format!("The {} let go.", name.to_lowercase()).chars().take(38).collect::<String>());
            tone(90, 500);
        } else if let Some(name) = tired {
            // Fair warning, once, before it goes.
            self.say(&format!("The {} will not last.", name.to_lowercase()).chars().take(38).collect::<String>());
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
        // The space bar is whatever fits: staging and picking are Enter's work, and once the tree
        // is coming down it is the next gear. On the timeslip it is nothing, so a late shift does
        // not throw the slip away.
        let keys = if keys & KEY_SPACE == 0 {
            keys
        } else {
            let bare = keys & !(KEY_SPACE | KEY_A);
            match (self.scene == Scene::Race).then(|| self.race.as_ref().map(|r| r.phase)).flatten() {
                Some(Phase::Tree | Phase::Run) => bare | KEY_X,
                Some(Phase::Slip) => bare,
                _ if self.scene == Scene::Road && self.trip.as_ref().is_some_and(|t| t.transmission.is_some()) => bare | KEY_X,
                _ if self.scene == Scene::Drive && self.town.as_ref().is_some_and(|t| t.transmission.is_some()) => bare | KEY_X,
                _ => bare | KEY_A,
            }
        };
        let hit = keys & !self.last_keys;
        self.last_keys = keys;
        self.ticks += 1;
        if self.problem.is_some() {
            return;
        }
        self.world_tick();
        if self.street(hit) {
            return;
        }
        if matches!(self.scene, Scene::Drive | Scene::Road) && self.bump_others() {
            self.dent();
            tone(70, 400);
            self.say("You hit them. Ghosts? See the garage.");
        }
        if self.scene == Scene::Race {
            return if self.net.versus.is_some() { self.versus_tick(keys, hit) } else { self.race_tick(keys, hit) };
        }
        if self.scene == Scene::Drive {
            return self.drive_tick(keys, hit);
        }
        if self.scene == Scene::Dyno && self.rollers.phase == 1 {
            return self.pull_tick(keys, hit);
        }
        if self.scene == Scene::Road {
            if self.net.versus.is_some() {
                return self.versus_road_tick(keys, hit);
            }
            return self.road_tick(keys, hit);
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
                if hit & KEY_X != 0 && self.saved && !self.sure {
                    // A game is kept: starting over is asked for twice.
                    self.sure = true;
                    tone(200, 120);
                } else if pick || hit & KEY_X != 0 {
                    // Someone new is shown round; someone carrying on knows where things are.
                    self.hints = !(pick && self.saved);
                    self.sure = false;
                    self.state = if pick && self.saved { self.kept().unwrap_or_else(|| self.fresh()) } else { self.fresh() };
                    self.note = None;
                    self.lamps = false;
                    tone(880, 80);
                    let place = self.state.place.clone();
                    self.enter(&place);
                    if self.hints && self.state.cars.is_empty() && self.scene == Scene::Place {
                        self.say("No car yet. The paper has some.");
                    }
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
                if hit & KEY_H != 0 {
                    self.hints = !self.hints;
                    tone(if self.hints { 620 } else { 420 }, 50);
                }
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
                    if self.py >= 34 && self.py < 50 && (64..124).contains(&self.px) {
                        self.scene = Scene::Paper(3);
                        self.note = None;
                    } else if self.py >= 34 && self.py < 50 && self.px >= 128 {
                        // The tabs, left to right: jobs, cars, parts.
                        self.scene = Scene::Paper(if self.px >= 256 { 1 } else if self.px >= 192 { 0 } else { 2 });
                        self.note = None;
                    } else if page == 3 {
                        if let Some(i) = self.row(8, 312, 54, 62, projects::PROJECTS) {
                            let down = (self.py - 54) % 62;
                            if self.px >= 248 && (30..42).contains(&down) {
                                self.haggle_project(i);
                            } else {
                                self.buy_project(i);
                            }
                        }
                    } else if page == 1 && self.fit_button_shown() && {
                        let [x, y, w, h] = Self::FIT_BUTTON;
                        self.px >= x && self.px < x + w && self.py >= y && self.py < y + h
                    } {
                        self.scene = Scene::Bench;
                        self.selling = false;
                        self.note = None;
                    } else if page == 2 {
                        if let Some(i) = self.row(8, 312, 56, 20, self.jobs().len()) {
                            self.work(i);
                        }
                    } else if page == 0 {
                        if let Some(i) = self.row(8, 312, 54, 44, 3) {
                            // The word in the corner of an ad is the telephone; the rest of it is the money.
                            if self.px >= 248 && (self.py - 54) % 44 >= 30 {
                                self.haggle(i);
                            } else {
                                self.buy_car(i);
                            }
                        }
                    } else {
                        let (len, top) = (self.shop_page().len(), self.shop_top());
                        let chip = self.group_chips().into_iter().find(|(x, w, _, _)| self.py >= 52 && self.py < 62 && self.px >= *x && self.px < x + w);
                        if let Some((_, _, _, g)) = chip {
                            self.shop_group = g;
                            self.turned[2] = 0;
                            self.note = None;
                        } else if len > PAGE && self.row(8, 312, top + PAGE as i32 * 10, 10, 1).is_some() {
                            self.turned[2] = if self.turned[2] + PAGE >= len { 0 } else { self.turned[2] + PAGE };
                        } else if let Some(i) = self.row(8, 312, top, 10, (len - self.turned[2].min(len)).min(PAGE)) {
                            self.buy_part(i + self.turned[2]);
                        }
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
            Scene::Hood => {
                if back {
                    self.carrying = None;
                    self.scene = Scene::Place;
                    self.note = None;
                    self.save();
                } else if pick {
                    self.hood_pick();
                }
            }
            Scene::Yard => {
                if back && self.parting.take().is_some() {
                    self.note = None;
                } else if back {
                    self.scene = Scene::Place;
                    self.note = None;
                } else if let (true, Some(i)) = (pick, self.parting) {
                    self.parting_pick(i);
                } else if pick {
                    if let Some(i) = self.row(8, 312, 34, 10, self.state.cars.len()) {
                        self.yard_pick(i, self.px >= 264);
                    }
                }
            }
            Scene::Bets => {
                if back {
                    self.scene = Scene::Place;
                    self.road_race = None;
                } else if pick {
                    if let Some(b) = self.button(self.stakes().len() + 2) {
                        self.scene = Scene::Place;
                        self.offer_bet(b);
                        self.road_race = None;
                    }
                }
            }
            Scene::Dyno => self.dyno_tick(pick, back),
            Scene::Laptop => self.laptop_tick(pick, back),
            Scene::Paint => {
                if back {
                    self.scene = Scene::Place;
                } else if pick {
                    let paints = self.paints();
                    if let Some(b) = self.button(paints.len()) {
                        self.scene = Scene::Place;
                        let (cost, hours) = (self.pack.manifest.respray.cost, self.pack.manifest.respray.hours);
                        let Some(cur) = self.state.cur else { return };
                        if self.state.cars[cur].paint.as_deref() == Some(paints[b].as_str()) {
                            return self.say("It is that color already.");
                        }
                        if self.state.cash < cost {
                            return self.no(&format!("A respray is ${cost}. You don't have it."));
                        }
                        self.state.cash -= cost;
                        self.state.cars[cur].paint = Some(paints[b].clone());
                        self.say(&format!("Resprayed, for ${cost}. It was {hours} hours."));
                        tone(700, 120);
                        self.spend(hours);
                    }
                }
            }
            Scene::Online => self.online_tick(pick, back),
            Scene::OnlineBets => self.online_bets_tick(pick, back),
            Scene::Race | Scene::Drive | Scene::Road => {}
        }
    }

    /// Out on the map. Escape turns back to where you set out from; pulling up beside somewhere
    /// and pressing Enter is going in, and that is an hour gone.
    fn drive_tick(&mut self, keys: u32, hit: u32) {
        let chased = self.town.as_ref().is_some_and(|t| t.chase.is_some());
        if hit & KEY_B != 0 && !chased && self.dash.is_some() {
            return self.quit_dash();
        }
        if self.dash_tick() {
            return;
        }
        if hit & KEY_B != 0 && !chased && self.out_of_town() {
            self.note = Some("No turning back: pull up somewhere.".into());
            return;
        }
        if hit & KEY_A != 0 && !chased && self.dash.is_none() {
            // A place at a road's end, or a block in town that a road leads out from: out to the road itself.
            if self.pull_in() {
                return;
            }
            let road = self.town.as_ref().and_then(|t| Some(t.beside(self.pack.manifest.maps.get(&t.map)?)?.road.clone()?));
            if let Some(road) = road.filter(|r| self.laid(r)) {
                return self.to_gate(&road);
            }
        }
        let Some(town) = self.town.as_mut() else { return };
        let Some(map) = self.pack.manifest.maps.get(&town.map) else { return };
        if hit & KEY_B != 0 && town.chase.is_some() {
            self.note = Some("Not with him behind you.".into());
            return;
        }
        if hit & KEY_B != 0 {
            if self.town_by_road {
                // The garage is a road away.
                self.note = Some("No turning back: pull up somewhere.".into());
                return;
            }
            tone(0, 0);
            self.town = None;
            self.scene = Scene::Place;
            return;
        }
        if hit & (KEY_A | KEY_B) != 0 && town.chase.is_some() {
            self.note = Some("Not with him behind you.".into());
            return;
        }
        // The street out of town is the road: drive off the end of it and you are on it, still rolling.
        if town.speed > 3.0 && town.chase.is_none() {
            // Unless the road is laid out past the edge of town: then it is just driven on to.
            let laid = |road: &String| self.land.as_ref().is_some_and(|w| w.lanes.iter().any(|l| &l.id == road));
            let out = map.blocks.values().filter_map(|b| b.road.as_ref()).filter(|r| !laid(r)).find(|road| map.gate(road).is_some_and(|(gx, gz, heading)| {
                let turned = ((town.heading - (heading + 180.0)).rem_euclid(360.0) + 180.0).rem_euclid(360.0) - 180.0;
                (gx - town.x).abs() < map.road + 2.0 && (gz - town.z).abs() < map.road + 2.0 && turned.abs() < 55.0 && ((gx - town.x).abs() < 7.0 || (gz - town.z).abs() < 7.0)
            })).cloned();
            if let Some(road) = out {
                let (from, speed) = (town.map.clone(), town.speed);
                self.take_road(&road, Some(&from));
                if let Some(trip) = self.trip.as_mut() {
                    trip.speed = speed;
                }
                return;
            }
        }
        if hit & KEY_A != 0 {
            if let Some(road) = town.beside(map).and_then(|b| b.road.clone()) {
                let from = town.map.clone();
                return self.take_road(&road, Some(&from));
            }
            if let Some(to) = town.beside(map).and_then(|b| b.to.clone()) {
                tone(520, 60);
                self.town = None;
                self.note = None;
                self.enter(&to);
                return self.spend(1);
            }
        }
        // The bottle, if there is one with something in it and a foot on the gas.
        let spray = self.state.cur.and_then(|i| self.state.cars.get(i)).filter(|o| keys & KEY_N != 0 && keys & KEY_UP != 0 && o.bottle > 0.0 && town.hold == 0 && town.crash == 0)
            .and_then(|o| o.fitted.values().filter_map(|id| self.pack.manifest.parts.get(base(id))).find_map(|p| p.nitrous.as_ref())).map_or(0.0, |k| k.gain as f64 / 100.0);
        town.spray = spray;
        if spray > 0.0 {
            if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
                o.bottle = (o.bottle - 1.0 / 30.0).max(0.0);
            }
        }
        let counting = town.hold;
        let (went, crashed) = town.tick_on(map, self.land.as_ref(), Pedals { go: keys & KEY_UP != 0, stop: keys & KEY_DOWN != 0, left: keys & KEY_LEFT != 0, right: keys & KEY_RIGHT != 0, up: hit & KEY_X != 0, down: hit & KEY_Y != 0 });
        if counting > 0 {
            // Three, two, one, and a higher one for go.
            if counting % 30 == 0 {
                tone(440, 120);
            } else if counting == 1 {
                tone(880, 250);
            }
            return;
        }
        if (map.hills > 0.0 || self.land.is_some()) && town.speed > 0.2 {
            // Up a hill it drags, and down one it runs away.
            let (hs, hc) = town.heading.to_radians().sin_cos();
            let g = |x: f64, z: f64| self.land.as_ref().map_or_else(|| map.height(x, z), |w| w.ground(map, x, z));
            let slope = (g(town.x + hs * 6.0, town.z + hc * 6.0) - g(town.x, town.z)) / 6.0;
            town.speed = (town.speed - 9.8 * slope / 30.0).max(0.0);
        }
        // The law: past one of them well over the limit and he is behind you until you stop.
        // Out on a road it is the road's limit and the road's ticket, and he can be outrun.
        let road_law = self.land.as_ref().filter(|_| !crate::land::in_town(map, town.x, town.z, 0.0))
            .and_then(|w| Some(&w.lanes[w.near(town.x, town.z, 40.0)?.lane].id)).and_then(|id| self.pack.manifest.roads.get(id)).map(|r| (r.limit, r.ticket));
        let (limit, ticket) = road_law.unwrap_or((map.limit, map.ticket));
        let racing = self.dash.as_ref().is_some_and(|d| d.rival.is_some());
        let over = town.speed * 2.237 - limit;
        let near_cop = |reach: f64| town.cops.iter().any(|&(cx, cz)| (cx - town.x).abs() + (cz - town.z).abs() < reach);
        let warned = road_law.is_some() && !racing && near_cop(220.0);
        let mut said: Option<String> = None;
        let mut fine = None;
        match town.chase {
            None => {
                if over > 10.0 && !racing && near_cop(34.0) {
                    town.chase = Some((0, over));
                    town.fled = 0;
                    said = Some(if road_law.is_some() { "Police! Pull over, or run for it." } else { "Police! Pull over." }.into());
                }
            }
            Some((ticks, worst)) => {
                let worst = worst.max(over);
                if town.speed < 0.5 || crashed {
                    // Wrapped it up out on a road with the law behind you: that is the dear way to stop.
                    fine = Some((ticket + worst.max(0.0) as i32 * 2) * if crashed && road_law.is_some() { 2 } else { 1 });
                    town.chase = None;
                } else if road_law.is_some() {
                    town.fled = if over > 25.0 { town.fled + 1 } else { town.fled.saturating_sub(1) };
                    if town.fled > 20 * 30 {
                        said = Some("You lost him.".into());
                        town.chase = None;
                    } else if ticks > 45 * 30 {
                        // He gets alongside in the end, and it costs twice as much.
                        fine = Some((ticket + worst.max(0.0) as i32 * 2) * 2);
                        town.speed = 0.0;
                        town.chase = None;
                    } else {
                        town.chase = Some((ticks + 1, worst));
                    }
                } else if ticks > 25 * 30 {
                    // He will not be lost in town: run long enough and it is twice the ticket.
                    fine = Some((map.ticket + worst.max(0.0) as i32 * 2) * 2);
                    town.speed = 0.0;
                    town.chase = None;
                } else {
                    town.chase = Some((ticks + 1, worst));
                }
            }
        }
        let chased = town.chase.is_some();
        let (speed, per_km) = (town.speed, map.gas_per_km);
        // Out on a road, its own gas, and the hours it takes.
        let per_km = self.on_the_road(went).unwrap_or(per_km);
        if self.town.is_none() || self.scene != Scene::Drive {
            return;
        }
        if let Some(words) = said {
            self.say(&words);
        }
        if let Some(fine) = fine {
            let paid = fine.min(self.state.cash.max(0));
            self.state.cash -= paid;
            self.say(&format!("Speeding. That is a ${paid} ticket."));
            tone(160, 400);
            self.save();
        } else if chased && !crashed {
            self.hum(if self.ticks / 9 % 2 == 0 { 760 } else { 570 });
        }
        if crashed {
            self.dent();
        }
        self.scrub(went / 1000.0 * 0.4);
        self.roll(went / 1000.0);
        let revs = self.town.as_ref().and_then(|t| t.transmission.as_ref()).map(|b| b.rpm);
        if revs.is_none() {
            self.shift_auto(speed, keys & KEY_UP != 0);
        }
        let Some(town) = self.town.as_ref() else { return };
        if crashed {
            tone(70, 500);
        } else if warned && !chased && self.radar() && self.ticks % 12 == 0 {
            tone(1500, 50);
        } else if town.crash == 0 {
            // The engine note: up through each gear and down again at the change.
            self.hum((revs.unwrap_or(self.auto.1) / 40.0) as i32);
        }
        self.burn(went / 1000.0 * per_km);
        if self.owned().is_some_and(|o| o.gas <= 0.0) {
            // Dry. It is a long walk, and the car comes home on a rope.
            tone(0, 0);
            self.town = None;
            if self.dash.take().is_some_and(|d| d.rival.is_some()) {
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

    /// Asking for five per cent more off a car in the paper. Every seller has a price they will
    /// not go under, somewhere between five and twenty-five per cent off, and does not say what
    /// it is: ask for more than that and they hang up, and the car is gone.
    fn haggle(&mut self, i: usize) {
        let Some(Some(offer)) = self.offers().into_iter().nth(i) else { return };
        let day = self.day();
        if self.state.knocked.0 != day {
            self.state.knocked = (day, Vec::new());
        }
        let had = self.state.knocked.1.iter().find(|k| k.0 == i).map_or(0, |k| k.1);
        let most = 5 + (hash(day as u32 * 31 + i as u32 + 17) % 5) as i32 * 5;
        if had + 5 > most {
            if self.state.sold.0 != day {
                self.state.sold = (day, Vec::new());
            }
            self.state.sold.1.push(i);
            self.no("\"Forget it.\" He hung up.");
        } else {
            self.state.knocked.1.retain(|k| k.0 != i);
            self.state.knocked.1.push((i, had + 5));
            let now = self.offers().into_iter().nth(i).flatten().map_or(offer.price, |o| o.price);
            self.say(&format!("\"All right. ${now}.\""));
            tone(660, 80);
        }
        self.save();
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
        let mut bought = Owned::new(&offer.car, paint, offer.cond);
        self.outfit(&mut bought);
        bought.gas = 2.0 + (offer.price * 7 % 60) as f64 / 10.0;
        self.state.cars.push(bought);
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

    /// Work the job in the paper's ad: the pay now, and the hours gone.
    fn work(&mut self, i: usize) {
        let Some(job) = self.jobs().get(i).cloned() else { return };
        self.state.cash += job.pay;
        self.say(&format!("{} hours, ${} in your pocket.", job.hours, job.pay));
        tone(660, 80);
        self.spend(job.hours);
    }

    fn buy_part(&mut self, i: usize) {
        let Some((id, part)) = self.shop_page().get(i).map(|(k, p)| ((*k).clone(), (*p).clone())) else { return };
        if self.has_part(&id) {
            return self.say("You have one.");
        }
        if self.state.cash < part.price {
            return self.no("You cannot afford it.");
        }
        self.state.cash -= part.price;
        self.state.spares.push(id);
        self.say(if self.fit_button_shown() { "Bought. FIT PARTS puts it on." } else { "Bought. It is on the shelf in the garage." });
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
        // The last line of a long column turns it over to the next dozen.
        for (k, (x0, x1, len)) in [(8, 156, slots.len()), (164, 312, self.state.spares.len())].into_iter().enumerate() {
            if len > LINES && self.row(x0, x1, 34 + LINES as i32 * 10, 10, 1).is_some() {
                self.turned[k] = if self.turned[k] + LINES >= len { 0 } else { self.turned[k] + LINES };
                return;
            }
        }
        let (top, shelf) = (self.turned[0].min(slots.len().saturating_sub(1)), self.turned[1].min(self.state.spares.len().saturating_sub(1)));
        if let Some(i) = self.row(164, 312, 34, 10, (self.state.spares.len() - shelf.min(self.state.spares.len())).min(LINES)).map(|i| i + shelf) {
            let id = self.state.spares[i].clone();
            if self.selling {
                let (paid, mut set) = self.sale(i);
                self.state.cash += paid;
                set.sort_unstable();
                for j in set.iter().rev() {
                    self.state.spares.remove(*j);
                }
                let needs = self.part(&id).is_some_and(|p| !p.needs.is_empty());
                self.say(&match (needs, set.len()) {
                    (true, 1) => format!("Sold on its own: ${paid}."),
                    (true, n) => format!("Sold as a set of {n}: ${paid}."),
                    _ => "Sold.".to_string(),
                });
                tone(660, 80);
                return self.save();
            }
            let Some(cur) = self.state.cur else { return self.no("No car in the bay to put it on.") };
            let slot = self.part(&id).map(|p| p.slot.clone()).unwrap_or_default();
            if !self.part(&id).is_some_and(|p| p.goes_on(&self.state.cars[cur].car)) {
                return self.no("That does not fit this car.");
            }
            if self.under_hood(&slot) {
                // Where it goes, holding it: its place is marked, and so are its bolts.
                self.area = self.car().and_then(|c| c.area_of(&slot)).unwrap_or(0);
                self.scene = Scene::Hood;
                self.carrying = Some(Loose::Part(id));
                let bolted = !self.state.cars[cur].off.contains(&slot);
                return self.say(&if bolted { "Undo its ringed bolts, then lift it.".to_string() } else { format!("Set it on the {}.", slot.to_lowercase()) });
            }
            let was = self.state.cars[cur].untuned;
            self.state.spares.remove(i);
            self.swapped(&id);
            if let Some(old) = self.state.cars[cur].fitted.insert(slot, id) {
                self.swapped(&old);
                self.state.spares.push(old);
            }
            self.say(if self.state.cars[cur].untuned && !was { "Bolted on. Now it wants a tune." } else { "Bolted on. That was an hour." });
            tone(520, 60);
            self.spend(1);
        } else if let Some(i) = self.row(8, 156, 34, 10, (slots.len() - top.min(slots.len())).min(LINES)).map(|i| i + top) {
            let Some(cur) = self.state.cur else { return };
            if let Some(k) = self.car().and_then(|c| c.area_of(&slots[i])) {
                // Off where it is: there, with it marked.
                self.area = k;
                self.scene = Scene::Hood;
                self.carrying = None;
                let place = self.car().map_or(String::new(), |c| c.areas()[k].1.to_lowercase());
                return self.say(&format!("It comes off {place}.").chars().take(38).collect::<String>());
            }
            let sheet = self.state.cars[cur].sheet;
            if let Some(old) = self.state.cars[cur].fitted.remove(&slots[i]) {
                self.swapped(&old);
                self.state.spares.push(old);
                self.say(if sheet { "Off, and on the shelf. An hour." } else { "Off. The stock one is back on." });
                tone(400, 60);
                self.spend(1);
            } else {
                self.say(if sheet { "There is nothing there to take off." } else { "That is the stock one." });
            }
        }
    }

    /// A part has gone on or come off the car in the bay: if it is one the tune is built around,
    /// the car needs tuning again.
    fn swapped(&mut self, id: &str) {
        if self.part(id).is_some_and(|p| p.retune) {
            if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
                o.untuned = true;
            }
        }
    }

    /// Whether this slot of the car in the bay is worked on under its hood.
    fn under_hood(&self, slot: &str) -> bool {
        self.car().is_some_and(|c| c.area_of(slot).is_some())
    }

    /// The part of the car being worked on: its id, its name, its picture and its places.
    pub fn area_here(&self) -> Option<(&str, &str, &str, &std::collections::BTreeMap<String, Slot>)> {
        let areas = self.car()?.areas();
        areas.get(self.area.min(areas.len().saturating_sub(1))).copied()
    }

    /// Whether a place is to be seen: not hidden behind something still on.
    pub fn in_view(&self, slots: &std::collections::BTreeMap<String, Slot>, slot: &str) -> bool {
        let Some(o) = self.owned() else { return false };
        let mut at = slot;
        for _ in 0..8 {
            match slots.get(at).filter(|p| p.covered).and_then(|p| p.behind.as_deref()) {
                Some(front) if !o.off.iter().any(|s| s == front) => return false,
                Some(front) => at = front,
                None => return true,
            }
        }
        true
    }

    /// The tabs across the top of the work: one for each place on the car, (x, width).
    fn area_tabs(&self) -> Vec<(i32, i32, String)> {
        let Some(car) = self.car() else { return Vec::new() };
        let areas = car.areas();
        if areas.len() < 2 {
            return Vec::new();
        }
        let w = (316 / areas.len() as i32).min(80);
        areas.iter().enumerate().map(|(k, a)| (2 + k as i32 * w, w - 2, Self::short_title(a.0, a.1))).collect()
    }

    fn short_title(id: &str, title: &str) -> String {
        match id {
            "hood" => "HOOD".into(),
            _ if id.len() <= 7 => id.to_uppercase(),
            _ => title.split_whitespace().last().unwrap_or(id).to_uppercase().chars().take(9).collect(),
        }
    }

    /// The slot under the pointer: its name, and its place on the picture. A bolt counts as its
    /// slot even where it sits outside the box.
    fn hood_slot_under(&self) -> Option<(String, Slot)> {
        let (_, _, _, slots) = self.area_here()?;
        if self.py >= 176 || self.py < 15 && !self.area_tabs().is_empty() {
            return None;
        }
        let seen: Vec<(&String, &Slot)> = slots.iter().filter(|(s, _)| self.in_view(slots, s)).collect();
        let off = |name: &str| self.owned().is_some_and(|o| o.off.iter().any(|s| s == name));
        let on_bolt = |name: &str, p: &Slot| !off(name) && p.bolts.iter().any(|b| (self.px - b[0]).abs() <= 5 && (self.py - b[1]).abs() <= 5);
        // What is uncovered lies in front of what covered it: the smaller first.
        let found = seen.iter().find(|(name, p)| on_bolt(name, p)).or_else(|| seen.iter().filter(|(_, p)| p.contains(self.px, self.py)).min_by_key(|(_, p)| p.rect[2] * p.rect[3]));
        found.map(|(s, p)| ((*s).clone(), (*p).clone()))
    }

    /// What is on the floor under the hood: the stock part of every slot it is not on, and the
    /// spares that go on this car there. The first eight are in reach.
    pub fn floor(&self) -> Vec<Loose> {
        let Some((o, car)) = self.owned().zip(self.car()) else { return Vec::new() };
        let Some((_, _, _, slots)) = self.area_here() else { return Vec::new() };
        let held = |l: &Loose| self.carrying.as_ref() == Some(l);
        // On a car with a build sheet, what it came with there is a real part, on the shelf when it is off.
        let stock = slots.keys().filter(|s| !(o.sheet && car.stock.contains_key(*s))).filter(|s| o.off.contains(*s) || o.fitted.contains_key(*s)).map(|s| Loose::Stock(s.clone()));
        let spares = self.state.spares.iter().filter(|id| self.part(id).is_some_and(|p| slots.contains_key(&p.slot) && p.goes_on(&o.car))).map(|id| Loose::Part(id.clone()));
        stock.chain(spares).filter(|l| !held(l)).take(8).collect()
    }

    fn loose_slot(&self, l: &Loose) -> String {
        match l {
            Loose::Stock(slot) => slot.clone(),
            Loose::Part(id) => self.part(id).map(|p| p.slot.clone()).unwrap_or_default(),
        }
    }

    /// What the part a car came with in a slot is called.
    fn stock_name(&self, slot: &str) -> String {
        let named = self.car().and_then(|c| c.areas().into_iter().find_map(|a| a.3.get(slot))).and_then(|p| p.stock.clone());
        named.unwrap_or_else(|| format!("Stock {}", slot.to_lowercase()))
    }

    fn loose_name(&self, l: &Loose) -> String {
        match l {
            Loose::Stock(slot) => self.stock_name(slot),
            Loose::Part(id) => self.part(id).map(|p| p.name.clone()).unwrap_or_default(),
        }
    }

    fn chip_at(n: usize) -> (i32, i32) {
        (4 + (n as i32 % 4) * 79, 178 + (n as i32 / 4) * 11)
    }

    /// Under the hood. A part is held by its bolts: undo them, lift it off, and it is on the
    /// floor. Pick a part up off the floor, set it on the empty place it goes, and do the bolts up.
    fn hood_pick(&mut self) {
        let Some(cur) = self.state.cur else { return };
        if self.py < 15 {
            if let Some(k) = self.area_tabs().iter().position(|(x, w, _)| self.px >= *x && self.px < x + w) {
                // Over to another part of the car: what you hold comes too.
                self.area = k;
                self.note = None;
                return tone(480, 40);
            }
        }
        if self.py >= 176 {
            // The floor: put down what you are holding, or pick something up.
            if self.carrying.take().is_some() {
                return self.say("Down it goes.");
            }
            let under = self.floor().into_iter().enumerate().find(|(n, _)| {
                let (x, y) = Self::chip_at(*n);
                self.px >= x && self.px < x + 76 && self.py >= y && self.py < y + 10
            });
            if let Some((_, l)) = under {
                self.carrying = Some(l);
                self.say("Now set it where it goes.");
                tone(520, 40);
            }
            return;
        }
        let Some((slot, place)) = self.hood_slot_under() else { return };
        let empty = self.state.cars[cur].off.contains(&slot);
        // Holding a part, whatever is on the car can still be undone and lifted off: the old
        // one where it goes, or what is in the way of it.
        let carry = self.carrying.clone().filter(|_| empty);
        match (empty, carry) {
            (false, None) => {
                let tight = self.state.cars[cur].tight(&slot, place.bolts.len());
                for (b, &[x, y]) in place.bolts.iter().enumerate() {
                    if (self.px - x).abs() <= 5 && (self.py - y).abs() <= 5 {
                        let now = tight ^ (1 << b);
                        self.state.cars[cur].bolts.insert(slot.clone(), now);
                        tone(if now & (1 << b) != 0 { 700 } else { 400 }, 50);
                        self.note = None;
                        self.labor(BOLT);
                        if now == all_bolts(place.bolts.len()) && self.state.cars[cur].fitted.contains_key(&slot) {
                            self.say("Done up tight. It is on.");
                            self.save();
                        } else if now == 0 && self.carrying.is_some() {
                            self.say("Bolts out. Pick it to lift it off.");
                        }
                        return;
                    }
                }
                // Something in front of it has to come off first; then its own bolts.
                if let Some(front) = place.behind.as_ref().filter(|f| !self.state.cars[cur].off.contains(*f)) {
                    return self.no(&format!("The {} {} in the way.", front.to_lowercase(), is_are(front)));
                }
                if tight != 0 {
                    return self.no("It is bolted on. Undo the bolts first.");
                }
                let o = &mut self.state.cars[cur];
                if let Some(part) = o.fitted.remove(&slot) {
                    self.swapped(&part);
                    self.state.spares.push(part);
                }
                self.state.cars[cur].off.push(slot.clone());
                let goes_here = self.carrying.as_ref().is_some_and(|l| self.loose_slot(l) == slot);
                self.say(if goes_here { "Off. Now set the new one on." } else { "It is off, and on the floor." });
                tone(260, 90);
                self.labor(LIFT);
            }
            (true, Some(l)) => {
                if self.loose_slot(&l) != slot {
                    return self.no("That does not go there.");
                }
                // And what it sits in front of has to be on before it is.
                let waits = self.area_here().and_then(|a| a.3.iter().find(|(name, p)| p.behind.as_deref() == Some(slot.as_str()) && self.state.cars[cur].off.contains(*name)).map(|(name, _)| name.clone()));
                if let Some(first) = waits {
                    return self.no(&format!("The {} {} on first.", first.to_lowercase(), if is_are(&first) == "are" { "go" } else { "goes" }));
                }
                let o = &mut self.state.cars[cur];
                o.off.retain(|s| *s != slot);
                o.bolts.insert(slot.clone(), 0);
                if let Loose::Part(id) = l {
                    self.swapped(&id);
                    self.state.spares.retain(|p| *p != id);
                    self.state.cars[cur].fitted.insert(slot, id);
                }
                self.carrying = None;
                self.labor(LIFT);
                self.say(if place.bolts.is_empty() { "It is on." } else { "It is on. Now do up the bolts." });
                tone(300, 90);
            }
            (true, None) => self.say("Nothing on it. Pick a part up off the floor."),
            (false, Some(_)) => self.say("There is one on there already."),
        }
    }

    /// Which of the three ways out the pointer is on, in the box over the yard.
    fn parting_under(&self) -> Option<usize> {
        (self.py >= 96 && self.py < 126 && self.px >= 16 && self.px < 304).then(|| ((self.px - 16) / 96) as usize)
    }

    fn parting_pick(&mut self, i: usize) {
        let Some(way) = self.parting_under() else {
            self.parting = None;
            self.note = None;
            return;
        };
        let Some(o) = self.state.cars.get(i) else { return };
        let Some(paid) = self.partings(o)[way] else {
            return self.no(["It does not run. The dealer passes.", "The body is too far gone to sell.", ""][way]);
        };
        self.parting = None;
        let mut gone = self.drop_car(i);
        match way {
            0 => self.say("Sold to the dealer."),
            1 => {
                // Everything on it comes off first, onto the shelf.
                self.state.spares.extend(std::mem::take(&mut gone.fitted).into_values());
                self.say(&format!("Stripped. The shell went for ${paid}."));
            }
            _ => self.say(&format!("Scrapped, as it stood: ${paid}.")),
        }
        self.state.cash += paid;
        tone(660, 80);
        // Stripping a car is a day's work; a dealer or a tow is an hour.
        self.spend(if way == 1 { 8 } else { 1 });
    }

    /// A car off your hands: out of the yard, and out of the bay if it was there.
    fn drop_car(&mut self, i: usize) -> Owned {
        let gone = self.state.cars.remove(i);
        self.state.cur = match self.state.cur {
            Some(c) if c == i => None,
            Some(c) if c > i => Some(c - 1),
            other => other,
        };
        if self.state.cur.is_none() {
            self.lamps = false;
        }
        gone
    }

    fn yard_pick(&mut self, i: usize, sell: bool) {
        if sell && self.state.cars[i].sheet {
            self.parting = Some(i);
            self.note = None;
            tone(520, 40);
        } else if sell {
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
            let left = race.bottle;
            self.keep_bottle(left);
            return self.settle((!alone).then_some(false));
        }
        if race.phase == Phase::Slip && hit & KEY_A != 0 {
            let (won, blown, left, out, abuse) = (race.won, race.blown, race.bottle, race.out, race.abuse);
            tone(0, 0);
            self.keep_bottle(left);
            self.ride_clutch(abuse);
            self.settle((!alone).then_some(won));
            if blown {
                self.blew();
            } else if out == 1 {
                self.no("Over the center line. Disqualified.");
            } else if out == -1 {
                self.dent();
                self.no("Into the wall. Disqualified.");
            }
            return;
        }
        let was = race.phase;
        race.tick(Controls { throttle: keys & KEY_UP != 0, stage: hit & KEY_A != 0, up: hit & KEY_X != 0, down: hit & KEY_Y != 0, nitrous: keys & KEY_N != 0, left: keys & KEY_LEFT != 0, right: keys & KEY_RIGHT != 0, clutch: keys & KEY_DOWN != 0 });
        if was != Phase::Slip && race.phase == Phase::Slip && race.out != 0 {
            tone(90, 400);
        } else if was != Phase::Slip && race.phase == Phase::Slip {
            tone(990, 200);
        } else if was == Phase::Stage && race.phase == Phase::Tree {
            tone(660, 60);
        } else if race.phase != Phase::Slip || race.v > 0.0 {
            let hz = (race.rpm / 40.0) as i32;
            self.hum(hz);
        }
    }

    // ---- Drawing ----

    /// A car's picture is painted in one color; this moves its paint ramps to the paint it is in,
    /// `darker` shades down for the night.
    fn tint(car: &Car, paint: Option<&str>, darker: i8) -> Tint {
        let mut tint = Tint::none();
        if let Some(paint) = paint.or(car.paint.as_deref()) {
            let to = ramp(paint) as u8;
            for (from, add) in [(&car.paint_ramps.mid, 0i8), (&car.paint_ramps.dark, -9), (&car.paint_ramps.light, 4), (&car.paint_ramps.glint, 2)] {
                if let Some(from) = from {
                    tint.to[ramp(from)] = to;
                    tint.add[ramp(from)] = add - darker;
                }
            }
        }
        tint
    }

    /// A car from the pack in a paint, its tires on `ground`. Half size for one seen from further off.
    fn draw_car(&self, s: &mut Screen, id: &str, paint: Option<&str>, lamps: bool, x: i32, ground: i32, half: bool) {
        let Some(car) = self.pack.manifest.cars.get(id) else { return };
        let name = if lamps { car.side_lamps.as_ref().unwrap_or(&car.side) } else { &car.side };
        let Some(pic) = self.pack.pictures.get(name) else { return };
        let tint = Self::tint(car, paint, 0);
        if half {
            s.picture_half(pic, x, ground - (car.ground + 1) / 2, Some(&tint));
        } else {
            s.picture_tint(pic, x, ground - car.ground, Some(&tint));
        }
    }

    /// A car of yours: the car, and over it whatever its parts look like from the side.
    fn draw_owned(&self, s: &mut Screen, o: &Owned, lamps: bool, x: i32, ground: i32, half: bool) {
        self.draw_car(s, &o.car, o.paint.as_deref(), lamps, x, ground, half);
        let Some(car) = self.pack.manifest.cars.get(&o.car) else { return };
        if !half && o.body > 0 {
            // What it has been through shows: scuffs along its flanks, more of them the worse it is.
            if let Some(side) = self.pack.pictures.get(&car.side) {
                for i in 0..(o.body / 10 + 1) as u32 {
                    let k = hash(i * 97 + o.car.len() as u32);
                    s.darken(x + side.w * (8 + (k % 80) as i32) / 100, ground - car.ground + side.h * (42 + ((k >> 8) % 26) as i32) / 100, 5 + (k >> 16) as i32 % 6, 2 + (k >> 20) as i32 % 3, 7);
                }
            }
        }
        let tint = Self::tint(car, o.paint.as_deref(), 0);
        for pic in o.fitted.values().filter_map(|id| self.part(id)).filter_map(|p| p.side.as_ref()).filter_map(|n| self.pack.pictures.get(n)) {
            if half {
                s.picture_half(pic, x, ground - (car.ground + 1) / 2, Some(&tint));
            } else {
                s.picture_tint(pic, x, ground - car.ground, Some(&tint));
            }
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
            Scene::Place | Scene::Bets | Scene::Paint => self.draw_place(s),
            Scene::Paper(page) => self.draw_paper(s, page),
            Scene::Bench => self.draw_bench(s),
            Scene::Hood => self.draw_hood(s),
            Scene::Yard => self.draw_yard(s),
            Scene::Dyno => self.draw_dyno(s),
            Scene::Laptop => self.draw_laptop(s),
            Scene::Online | Scene::OnlineBets => self.draw_online(s),
            Scene::Race => return self.draw_race(s),
            Scene::Drive => return { self.draw_drive(s); self.draw_street(s) },
            Scene::Road => return { self.draw_road(s); self.draw_street(s) },
            Scene::Over(kind) => return self.draw_over(s, kind),
        }
        if self.scene == Scene::Place {
            // Who else is here, and anything on the street for you.
            let here = self.here_too();
            if !here.is_empty() {
                let line: String = format!("HERE TOO: {}", here.join(", ").to_uppercase()).chars().take(38).collect();
                s.text_sh(316 - line.len() as i32 * 8, 4, &line, shade(ramp("CYAN"), 28), BLACK);
            }
            self.draw_street(s);
        }
        if self.scene != Scene::Title {
            s.pointer(self.px, self.py);
        }
    }

    fn draw_title(&self, s: &mut Screen) {
        s.vgrad(0, 0, 320, 240, ramp("NAVY"), 3, 14);
        if let Some(pic) = self.pack.manifest.title.as_ref().and_then(|n| self.pack.pictures.get(n)) {
            s.picture(pic, 0, 0);
        } else {
            let name = self.pack.manifest.name.to_uppercase();
            for (i, line) in wrap(&name, 18).iter().enumerate() {
                let x = 160 - line.len() as i32 * 8;
                s.text_big(x + 1, 61 + i as i32 * 22, line, shade(ramp("RED"), 14), 2);
                s.text_big(x, 60 + i as i32 * 22, line, shade(ramp("GOLD"), 27), 2);
            }
        }
        if self.saved && self.sure {
            s.text_sh(92, 190, "ENTER: CARRY ON", WHITE, BLACK);
            s.text_sh(28, 204, "E AGAIN: START OVER, AND LOSE IT", shade(ramp("RED"), 27), BLACK);
        } else if self.saved {
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
        // Whoever is standing about is behind the cars; whoever you are racing is in front of theirs.
        for p in &place.people {
            if let Some(pic) = self.pack.pictures.get(&p.picture) {
                s.picture(pic, p.at[0], p.at[1] - pic.h);
            }
        }
        let rival = place.rival_at.and(self.rival());
        if let (Some([x, ground]), Some(r)) = (place.rival_at, rival) {
            self.draw_car(s, &r.car, r.paint.as_deref(), true, x, ground, false);
            if let Some(pic) = r.figure.as_ref().and_then(|n| self.pack.pictures.get(n)) {
                // By the nose of the car if there is room there, or else by its tail.
                let wide = self.pack.manifest.cars.get(&r.car).and_then(|c| self.pack.pictures.get(&c.side)).map_or(0, |c| c.w);
                let fx = if x + wide + 3 + pic.w <= 320 { x + wide + 3 } else { (x - pic.w - 3).max(0) };
                s.picture(pic, fx, ground + 2 - pic.h);
            }
        }
        if let (Some([x, ground]), Some(o)) = (place.car_at, self.owned()) {
            self.draw_owned(s, o, self.lamps, x, ground, false);
        }
        if place.outdoors {
            self.draw_rain(s, 320, 200);
        }
        if place.outdoors && self.dark() > 0 {
            s.darken(0, 0, 320, 200, if self.dark() == 1 { 5 } else { 11 });
        }
        if self.scene == Scene::Place && self.hints {
            // Everything there is to pick that the picture does not mark: a box round each.
            let under = self.spot_under().map(|s| s.rect);
            for spot in self.spots().into_iter().filter(|sp| sp.button.is_none()) {
                let [x, y, w, h] = spot.rect;
                s.frame(x, y, w, h, if under == Some(spot.rect) { WHITE } else { shade(ramp("GOLD"), 26) });
                s.frame(x + 1, y + 1, w - 2, h - 2, BLACK);
            }
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
        if self.scene == Scene::Paint {
            // A row of tins: each a button in its own color.
            let (paints, over) = (self.paints(), self.button(self.paints().len()));
            let w = 312 / paints.len() as i32;
            for (i, p) in paints.iter().enumerate() {
                let (x, on) = (4 + i as i32 * w, over == Some(i));
                s.vgrad(x + 1, 178, w - 2, 18, ramp(p), 26, 14);
                s.frame(x + 1, 178, w - 2, 18, if on { WHITE } else { shade(ramp("CHROME"), 10) });
            }
            let name = over.map(|i| paints[i].to_lowercase()).unwrap_or_else(|| "Pick a color.".into());
            self.panel(s, "A respray", &name, &format!("${}, and {} hours. Escape: never mind", self.pack.manifest.respray.cost, self.pack.manifest.respray.hours));
        } else if self.scene == Scene::Bets {
            let mut labels = vec!["KICKS".to_string()];
            labels.extend(self.stakes().iter().map(|b| format!("${b}")));
            labels.push("PINKS".into());
            self.buttons(s, &labels);
            self.panel(s, rival.map(|r| r.name.as_str()).unwrap_or(""), "What are we racing for?", "Escape: never mind");
        } else if let Some(r) = rival {
            let line = self.note.clone().or(under).unwrap_or_else(|| r.says.clone());
            self.panel(s, &r.name, &line, &format!("Runs {:.1}. Day {}, {} hours left.", r.et, self.day(), self.hours_left()));
        } else {
            let line = self.note.clone().or(under).unwrap_or_else(|| if self.hints { "Point at a box. H hides them.".into() } else { "Look round. H shows what there is.".into() });
            self.panel(s, &place.title, &line, &clock);
        }
    }

    fn draw_paper(&self, s: &mut Screen, page: usize) {
        let (ink, gray, cream) = (shade(ramp("RUBBER"), 3), shade(ramp("BROWN"), 14), ramp("CREAM"));
        s.vgrad(0, 0, 320, 200, cream, 27, 23);
        s.text_big(8, 8, "THE PAPER", ink, 2);
        s.text(200, 12, if self.rain_on(self.day() + 1) { "TOMORROW: RAIN" } else { "TOMORROW: DRY" }, gray);
        for (t, label, x) in [(3, "AS IS", 64), (2, "JOBS", 128), (0, "CARS", 192), (1, "PARTS", 256)] {
            if t == page {
                s.rect(x, 34, 60, 16, ink);
            }
            s.frame(x, 34, 60, 16, ink);
            s.text(x + 30 - label.len() as i32 * 4, 38, label, if t == page { shade(cream, 28) } else { ink });
        }
        s.rect(8, 52, 304, 1, gray);
        let hover = shade(cream, 31);
        if page == 3 {
            self.draw_projects(s, ink, gray, hover);
            // Whatever more the ad says than there was room for is read out for the one pointed at.
            let more: Vec<String> = self.row(8, 312, 54, 62, projects::PROJECTS).and_then(|i| self.projects().into_iter().nth(i).flatten()).map(|p| p.says.into_iter().skip(2).collect()).unwrap_or_default();
            let line = self.note.clone().or_else(|| more.first().cloned()).unwrap_or_else(|| "It does not run. The ad says why.".into());
            self.panel(s, "Sold as it stands", &line, more.get(1).map_or("Pick one to buy it. Escape: back.", |m| m.as_str()));
        } else if page == 0 {
            let offers = self.offers();
            let over = self.row(8, 312, 54, 44, 3);
            for (i, offer) in offers.iter().enumerate() {
                let y = 54 + i as i32 * 44;
                if over == Some(i) && offer.is_some() {
                    s.rect(8, y, 304, 43, hover);
                }
                s.rect(8, y + 43, 304, 1, gray);
                let Some(o) = offer else {
                    s.text(120, y + 18, "S O L D", shade(ramp("RED"), 20));
                    continue;
                };
                let car = &self.pack.manifest.cars[&o.car];
                self.draw_car(s, &o.car, Some(&o.paint).filter(|p| !p.is_empty()).map(|p| p.as_str()), false, 12, y + 41, true);
                for (k, line) in wrap(&car.name.to_uppercase(), 16).iter().take(2).enumerate() {
                    s.text(180, y + 4 + k as i32 * 9, line, ink);
                }
                s.text(180, y + 24, cond_word(o.cond), gray);
                s.text(180, y + 34, &format!("${}", o.price), shade(ramp("FOREST"), 12));
                let on = over == Some(i) && self.px >= 248 && (self.py - 54) % 44 >= 30;
                s.text(256, y + 34, "HAGGLE", if on { shade(ramp("RED"), 20) } else { gray });
            }
            if offers.is_empty() {
                s.text(12, 60, "NO CARS FOR SALE.", gray);
            }
            self.panel(s, "The paper", self.note.as_deref().unwrap_or("Pick a car to buy it, or haggle."), "Escape puts it down.");
        } else if page == 2 {
            let jobs = self.jobs();
            let over = self.row(8, 312, 56, 20, jobs.len());
            for (i, j) in jobs.iter().enumerate() {
                let y = 56 + i as i32 * 20;
                if over == Some(i) {
                    s.rect(8, y - 1, 304, 19, hover);
                }
                s.text(12, y + 1, &j.name.to_uppercase().chars().take(28).collect::<String>(), ink);
                s.text(12, y + 10, &format!("{} HOURS", j.hours), gray);
                let pay = format!("${}", j.pay);
                s.text(308 - pay.len() as i32 * 8, y + 5, &pay, shade(ramp("FOREST"), 12));
                s.rect(8, y + 18, 304, 1, gray);
            }
            self.panel(s, "Help wanted", self.note.as_deref().unwrap_or("Pick a job to work it. It pays, and it takes the hours."), "Escape puts it down.");
        } else {
            let parts = self.shop_page();
            let (top, y0) = (self.turned[2].min(parts.len().saturating_sub(1)), self.shop_top());
            // Where on the car: one button each, and one for all of them.
            for (x, w, words, g) in self.group_chips() {
                let (on, under) = (g == self.shop_group, self.py >= 52 && self.py < 62 && self.px >= x && self.px < x + w);
                s.rect(x, 52, w, 10, if on { ink } else if under { hover } else { shade(cream, 24) });
                s.frame(x, 52, w, 10, ink);
                s.text(x + w / 2 - words.len() as i32 * 4, 53, &words, if on { shade(cream, 28) } else { ink });
            }
            let over = self.row(8, 312, y0, 10, (parts.len() - top.min(parts.len())).min(PAGE)).map(|i| i + top);
            if parts.len() > PAGE {
                s.text(12, y0 + PAGE as i32 * 10, &format!("MORE: {} TO {} OF {}", top + 1, (top + PAGE).min(parts.len()), parts.len()), shade(ramp("BLUE"), 14));
            }
            for (i, &(id, p)) in parts.iter().enumerate().skip(top).take(PAGE) {
                let y = y0 + (i - top) as i32 * 10;
                if over == Some(i) {
                    s.rect(8, y - 1, 304, 10, hover);
                }
                s.text(12, y, &p.name.to_uppercase().chars().take(20).collect::<String>(), ink);
                s.text(180, y, &p.slot.to_uppercase().chars().take(9).collect::<String>(), gray);
                if let Some(at) = self.part_where(id) {
                    s.text(308 - at.len() as i32 * 8, y, at, shade(ramp(if at == "ON CAR" { "FOREST" } else { "BLUE" }), 14));
                } else {
                    let price = format!("${}", p.price);
                    s.text(308 - price.len() as i32 * 8, y, &price, shade(ramp("FOREST"), 12));
                }
            }
            if parts.is_empty() {
                s.text(12, 60, "NO PARTS FOR SALE.", gray);
            }
            if self.fit_button_shown() {
                let [x, y, w, h] = Self::FIT_BUTTON;
                let on = self.px >= x && self.px < x + w && self.py >= y && self.py < y + h;
                s.rect(x, y, w, h, if on { shade(ramp("GOLD"), 24) } else { ink });
                s.text(x + w / 2 - 36, y + 3, "FIT PARTS", if on { ink } else { shade(cream, 28) });
            }
            let pitch = over.and_then(|i| parts.get(i).map(|(_, p)| *p)).map(|p| format!("{} {}", self.does(p), p.pitch));
            let title = if self.owned().is_some() { "Parts that fit" } else { "The paper" };
            self.panel(s, title, self.note.as_deref().or(pitch.as_deref()).unwrap_or("Pick a part to buy it."), "Escape puts it down.");
        }
    }

    fn draw_bench(&self, s: &mut Screen) {
        s.vgrad(0, 0, 320, 200, ramp("WOOD"), 9, 5);
        s.text_sh(8, 8, "ON THE CAR", WHITE, BLACK);
        s.text_sh(164, 8, "ON THE SHELF", WHITE, BLACK);
        s.rect(8, 28, 148, 1, shade(ramp("WOOD"), 18));
        s.rect(164, 28, 148, 1, shade(ramp("WOOD"), 18));
        let (slots, lit) = (self.slots(), shade(ramp("WOOD"), 16));
        let (top, shelf) = (self.turned[0].min(slots.len().saturating_sub(1)), self.turned[1].min(self.state.spares.len().saturating_sub(1)));
        let (left, right) = (self.row(8, 156, 34, 10, (slots.len() - top.min(slots.len())).min(LINES)).map(|i| i + top), self.row(164, 312, 34, 10, (self.state.spares.len() - shelf.min(self.state.spares.len())).min(LINES)).map(|i| i + shelf));
        for (x, len, at) in [(10, slots.len(), top), (166, self.state.spares.len(), shelf)] {
            if len > LINES {
                s.text_sh(x, 34 + LINES as i32 * 10, &format!("MORE: {} TO {} OF {len}", at + 1, (at + LINES).min(len)), shade(ramp("CYAN"), 26), BLACK);
            }
        }
        let sheet = self.owned().is_some_and(|o| o.sheet);
        let mut about: Option<String> = None;
        for (i, slot) in slots.iter().enumerate().skip(top).take(LINES) {
            let y = 34 + (i - top) as i32 * 10;
            let fitted = self.owned().and_then(|o| o.fitted.get(slot));
            let on = fitted.and_then(|id| self.part(id));
            if left == Some(i) {
                s.rect(8, y - 1, 148, 10, lit);
                about = Some(match on { Some(p) if p.pitch.is_empty() => format!("{slot}. Pick it to take it off."), Some(p) => format!("{}. Pick it to take it off.", p.pitch), None if sheet => format!("{slot}: there is nothing there."), None => format!("{slot}: the stock one.") });
            }
            // What is there, and how good it is if it is not new; red when it is done for.
            let cond = fitted.map_or(100, |id| cond_of(id));
            // A slot the car left the factory with something in is one it wants something in.
            let needed = self.car().is_some_and(|c| c.stock.contains_key(slot));
            let name = match on {
                Some(p) if cond < 100 => format!("{:14} {cond:2}%", p.name.to_uppercase().chars().take(14).collect::<String>()),
                Some(p) => p.name.to_uppercase(),
                None if self.under_hood(slot) => format!("{}: HOOD", slot.to_uppercase()),
                None if sheet && needed => format!("NO {}", slot.to_uppercase()),
                None if sheet => format!("{}: NONE", slot.to_uppercase()),
                None => format!("STOCK {}", slot.to_uppercase()),
            };
            let stock = on.is_some() && self.car().is_some_and(|c| c.stock.get(slot).map(String::as_str) == fitted.map(|id| base(id)));
            let color = if cond == 0 || (on.is_none() && sheet && needed && !self.under_hood(slot)) { shade(ramp("RED"), 27) } else if on.is_none() && sheet { shade(ramp("CREAM"), 14) } else if on.is_some() && !stock { shade(ramp("GOLD"), 27) } else { shade(ramp("CREAM"), 22) };
            s.text_sh(10, y, &name.chars().take(18).collect::<String>(), color, BLACK);
        }
        if self.state.cur.is_none() {
            s.text_sh(10, 34 + slots.len() as i32 * 10 + 6, "NO CAR IN THE BAY.", shade(ramp("RED"), 26), BLACK);
        }
        for (i, id) in self.state.spares.iter().enumerate().skip(shelf).take(LINES) {
            let y = 34 + (i - shelf) as i32 * 10;
            let Some(p) = self.part(id) else { continue };
            if right == Some(i) {
                s.rect(164, y - 1, 148, 10, lit);
                about = Some(if self.selling { match self.sale(i) {
                    (paid, set) if set.len() > 1 => format!("${paid} today, with the {} that go with it.", set.len() - 1),
                    (paid, _) if p.needs.is_empty() => format!("They give ${paid} for it today."),
                    (paid, _) => format!("${paid} on its own. More with what it needs."),
                } } else { format!("{}: {}", p.slot, p.pitch) });
            }
            let cond = cond_of(id);
            let name = if cond < 100 { format!("{:14} {cond:2}%", p.name.to_uppercase().chars().take(14).collect::<String>()) } else { p.name.to_uppercase() };
            s.text_sh(166, y, &name.chars().take(18).collect::<String>(), if cond == 0 { shade(ramp("RED"), 27) } else { WHITE }, BLACK);
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

    fn chip(&self, s: &mut Screen, x: i32, y: i32, l: &Loose) {
        let better = matches!(l, Loose::Part(_));
        s.rect(x, y, 76, 10, shade(ramp(if better { "GOLD" } else { "GRAY" }), if better { 20 } else { 12 }));
        s.frame(x, y, 76, 10, shade(ramp("RUBBER"), 2));
        let name = self.loose_name(l).to_uppercase().replace("STOCK ", "STK ");
        s.text(x + 2, y + 1, &name.chars().take(9).collect::<String>(), if better { shade(ramp("RUBBER"), 2) } else { WHITE });
    }

    fn draw_hood(&self, s: &mut Screen) {
        let Some(o) = self.owned() else { return };
        let Some((_, area_title, picture, slots)) = self.area_here() else { return };
        s.clear(BLACK);
        if let Some(pic) = self.pack.pictures.get(picture) {
            s.picture(pic, 0, 0);
        }
        let over = self.hood_slot_under().map(|(slot, _)| slot);
        let (dark, chrome, gold) = (ramp("RUBBER"), ramp("CHROME"), ramp("GOLD"));
        let blink = self.ticks / 10 % 2 == 0;
        // Behind ones last, so that what is uncovered is drawn over what it was under.
        let mut order: Vec<(&String, &Slot)> = slots.iter().filter(|(name, _)| self.in_view(slots, name)).collect();
        order.sort_by_key(|(_, p)| p.covered);
        let stock = self.car().map(|c| c.stock.clone()).unwrap_or_default();
        let mut labels: Vec<(i32, i32, String)> = Vec::new();
        for (slot, place) in order {
            let [x, y, w, h] = place.rect;
            let pts: Vec<(i32, i32)> = if place.poly.len() >= 3 { place.poly.iter().map(|p| (p[0], p[1])).collect() } else { vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)] };
            let (empty, tight) = (o.off.contains(slot), o.tight(slot, place.bolts.len()));
            // Only something better than the car came with is new: on a car of real parts, the
            // stock part is fitted too.
            let fitted = o.fitted.get(slot).filter(|id| stock.get(slot.as_str()).is_none_or(|s| s != base(id))).and_then(|id| self.part(id));
            let (mut mx, mut my) = place.middle();
            if place.poly.len() >= 3 && !place.contains(mx, my) {
                // A ring, or some such: its tag goes on it, near its first corner.
                let [fx, fy] = place.poly[0];
                (mx, my) = (fx + (mx - fx) * 3 / 20, fy + (my - fy) * 3 / 20);
                (mx, my) = (mx - (mx - place.middle().0).signum() * 6, my);
            }
            if empty {
                // Off the car: a dark hole where it was.
                s.shape(&pts, shade(dark, 1), shade(ramp("NAVY"), 4));
                s.outline(&pts, shade(chrome, 12));
            } else if place.covered {
                // Uncovered: the picture never showed it, so it is drawn plain, with its name.
                s.shape(&pts, shade(ramp("CONCRETE"), 13), shade(ramp("CHROME"), 15));
                s.outline(&pts, shade(dark, 3));
                let words: String = slot.to_uppercase().chars().take(((w - 4) / 8).max(3) as usize).collect();
                labels.push((mx - words.len() as i32 * 4, my - 4, words));
            }
            if !empty {
                if let Some(pic) = fitted.and_then(|p| p.picture.as_ref()).and_then(|n| self.pack.pictures.get(n)) {
                    // The better part is on, and the pack has a picture of it there.
                    s.picture_sized(pic, x, y, w, h, None);
                } else if fitted.is_some() && !place.covered {
                    // The better part is on: a tag on it says so.
                    s.rect(mx - 12, my - 5, 25, 10, shade(gold, 24));
                    s.frame(mx - 12, my - 5, 25, 10, shade(dark, 2));
                    s.text(mx - 11, my - 4, "NEW", shade(dark, 2));
                }
            }
            // Holding a part, or with one on loose: where the work is, blinking.
            let loose = !empty && tight != all_bolts(place.bolts.len()) && self.carrying.is_none();
            let target = self.carrying.as_ref().is_some_and(|l| self.loose_slot(l) == *slot) || loose;
            if target && blink {
                s.outline(&pts, shade(ramp("LIME"), 28));
            }
            if over.as_ref() == Some(slot) {
                s.outline(&pts, shade(gold, 28));
            }
            for (b, &[bx, by]) in place.bolts.iter().enumerate() {
                // The bolts to undo, or to do up, ringed while they want doing.
                let todo = target && !empty && (self.carrying.is_some() == (tight & (1 << b) != 0));
                if todo && !blink {
                    s.disc(bx, by, 5, shade(ramp("LIME"), 28));
                }
                if !empty && tight & (1 << b) != 0 {
                    s.disc(bx, by, 3, shade(dark, 2));
                    s.disc(bx, by, 2, shade(chrome, 27));
                    s.pset(bx, by, shade(chrome, 10));
                } else {
                    s.disc(bx, by, 2, shade(dark, 1));
                    s.pset(bx, by, shade(chrome, if empty { 9 } else { 22 }));
                }
            }
        }
        // The names of what has been uncovered, none written over another.
        let mut placed: Vec<(i32, i32, i32)> = Vec::new();
        for (x, mut y, words) in labels {
            let w = words.len() as i32 * 8;
            while placed.iter().any(|&(px, py, pw)| x < px + pw && px < x + w && (y - py).abs() < 9) {
                y += 9;
            }
            placed.push((x, y, w));
            s.text_sh(x, y, &words, WHITE, BLACK);
        }
        // The other parts of the car, along the top.
        for (k, (x, w, words)) in self.area_tabs().into_iter().enumerate() {
            let (on, under) = (k == self.area, self.py < 15 && self.px >= x && self.px < x + w);
            s.rect(x, 2, w, 11, if on { shade(gold, 24) } else if under { shade(ramp("NAVY"), 14) } else { shade(ramp("NAVY"), 6) });
            s.frame(x, 2, w, 11, shade(chrome, if on { 28 } else { 14 }));
            s.text(x + w / 2 - words.len() as i32 * 4, 4, &words, if on { BLACK } else { WHITE });
        }
        // The floor in front of the car, and what is lying on it.
        s.vgrad(0, 176, 320, 24, ramp("CONCRETE"), 18, 22);
        s.rect(0, 176, 320, 1, shade(dark, 2));
        let floor = self.floor();
        let mut chip_over = None;
        for (n, l) in floor.iter().enumerate() {
            let (x, y) = Self::chip_at(n);
            self.chip(s, x, y, l);
            if self.px >= x && self.px < x + 76 && self.py >= y && self.py < y + 10 {
                chip_over = Some(l);
            }
        }
        if floor.is_empty() && self.carrying.is_none() {
            s.text(8, 184, "NOTHING ON THE FLOOR.", shade(ramp("CONCRETE"), 8));
        }
        let (title, line, foot) = match (&over, chip_over, &self.carrying) {
            (Some(slot), _, _) if o.off.contains(slot) => (slot.clone(), "Nothing on it.".to_string(), "Enter to pick. Escape goes back.".to_string()),
            (Some(slot), _, _) => {
                let name = o.fitted.get(slot).and_then(|id| self.part(id)).map(|p| p.name.clone()).unwrap_or_else(|| self.stock_name(slot));
                let n = slots[slot].bolts.len();
                let foot = match o.tight(slot, n).count_ones() as usize {
                    0 => "The bolts are out. Pick it to lift it.".to_string(),
                    k if k == n => "Bolted down. Pick a bolt to undo it.".to_string(),
                    k => format!("{k} of {n} bolts tight."),
                };
                (slot.clone(), name, foot)
            }
            (None, Some(l), _) => (self.loose_name(l), "Pick it up.".to_string(), "Enter to pick. Escape goes back.".to_string()),
            (None, None, Some(l)) => {
                let slot = self.loose_slot(l);
                let step = match slots.get(&slot) {
                    Some(p) if !o.off.contains(&slot) && o.tight(&slot, p.bolts.len()) != 0 => format!("Undo the ringed bolts on the {}.", slot.to_lowercase()),
                    Some(_) if !o.off.contains(&slot) => format!("Pick the {} to lift it off.", slot.to_lowercase()),
                    _ => format!("Set it on the {}, marked.", slot.to_lowercase()),
                };
                ("Holding".to_string(), self.loose_name(l), step)
            }
            _ => (area_title.to_string(), "Point at a part.".to_string(), "Enter to pick. Escape goes back.".to_string()),
        };
        self.panel(s, &title, &line, self.note.as_deref().unwrap_or(&foot));
        if let Some(l) = &self.carrying {
            self.chip(s, self.px + 6, self.py + 10, l);
        }
    }

    fn draw_yard(&self, s: &mut Screen) {
        s.vgrad(0, 0, 320, 200, ramp("GRASS"), 14, 8);
        s.vgrad(0, 0, 320, 26, ramp("SKY"), 20, 27);
        s.text_sh(8, 8, "YOUR CARS", WHITE, BLACK);
        s.text_sh(148, 8, "ENGINE", shade(ramp("CREAM"), 26), BLACK);
        s.text_sh(216, 8, "GAS", shade(ramp("CREAM"), 26), BLACK);
        let over = self.row(8, 312, 34, 10, self.state.cars.len());
        for (i, o) in self.state.cars.iter().enumerate() {
            let y = 34 + i as i32 * 10;
            if over == Some(i) {
                s.rect(8, y - 1, 304, 10, shade(ramp("GRASS"), 20));
            }
            let car = &self.pack.manifest.cars[&o.car];
            s.rect(10, y, 12, 8, BLACK);
            s.vgrad(11, y + 1, 10, 6, ramp(o.paint.as_deref().or(car.paint.as_deref()).unwrap_or("GRAY")), 27, 16);
            let here = self.state.cur == Some(i);
            s.text_sh(28, y, &car.name.to_uppercase().chars().take(14).collect::<String>(), if here { shade(ramp("GOLD"), 27) } else { WHITE }, BLACK);
            s.text_sh(148, y, cond_word(o.cond), WHITE, BLACK);
            s.text_sh(216, y, &format!("{:.1}", o.gas), WHITE, BLACK);
            s.text_sh(264, y, "SELL", if over == Some(i) && self.px >= 264 { shade(ramp("RED"), 28) } else { shade(ramp("CREAM"), 22) }, BLACK);
        }
        // What is on a trailer, under the cars that are here.
        let road = shade(ramp("CREAM"), 14);
        for (k, (when, o, _)) in self.state.coming.iter().enumerate().take(MOST_CARS.saturating_sub(self.state.cars.len())) {
            let y = 34 + (self.state.cars.len() + k) as i32 * 10;
            let car = &self.pack.manifest.cars[&o.car];
            s.text_sh(28, y, &car.name.to_uppercase().chars().take(14).collect::<String>(), road, BLACK);
            s.text_sh(148, y, "ON TRAILER", road, BLACK);
            s.text_sh(240, y, &format!("DAY {when}"), road, BLACK);
        }
        if self.state.cars.is_empty() && self.state.coming.is_empty() {
            s.text_sh(8, 34, "YOU HAVE NO CAR. TRY THE PAPER.", WHITE, BLACK);
        }
        if let Some(o) = self.parting.and_then(|i| self.state.cars.get(i)) {
            let car = &self.pack.manifest.cars[&o.car];
            s.rect(8, 70, 304, 64, shade(ramp("NAVY"), 4));
            s.frame(8, 70, 304, 64, shade(ramp("CHROME"), 22));
            s.text_sh(16, 78, &car.name.to_uppercase().chars().take(36).collect::<String>(), WHITE, BLACK);
            let under = self.parting_under();
            for (k, (word, paid)) in ["DEALER", "SHELL", "SCRAP"].into_iter().zip(self.partings(o)).enumerate() {
                let x = 16 + k as i32 * 96;
                let lit = under == Some(k);
                s.rect(x, 96, 92, 30, if lit && paid.is_some() { shade(ramp("GRASS"), 14) } else { shade(ramp("NAVY"), 9) });
                s.frame(x, 96, 92, 30, shade(ramp("CHROME"), if lit { 26 } else { 14 }));
                s.text_sh(x + 6, 100, word, if paid.is_some() { WHITE } else { shade(ramp("CREAM"), 12) }, BLACK);
                let says = paid.map_or_else(|| ["NO RUN", "TOO RUSTY", ""][k].to_string(), |p| if k == 1 { format!("${p} +PARTS") } else { format!("${p}") });
                s.text_sh(x + 6, 113, &says, if paid.is_some() { shade(ramp("GOLD"), 27) } else { shade(ramp("RED"), 22) }, BLACK);
            }
            let line = match under {
                Some(0) => "The dealer takes it whole, if it runs.",
                Some(1) => "Strip it, sell the shell. A day's work.",
                Some(2) => "Steel, and a bit for what is on it.",
                _ => "Pick one, or Escape to keep it.",
            };
            return self.panel(s, "The yard", self.note.as_deref().unwrap_or(line), "Escape keeps it.");
        }
        let worth = over.and_then(|i| self.state.cars.get(i)).map(|o| if o.sheet { "Sell: the dealer, a shell, or scrap.".to_string() } else { format!("The dealer gives ${}.", self.value(o)) });
        self.panel(s, "The yard", self.note.as_deref().or(worth.as_deref()).unwrap_or("Pick one to bring it into the bay."), "The gold one is in the bay. Escape.");
    }

    /// The strip from the side, for a car with no cockpit in the pack.
    fn draw_race_side(&self, s: &mut Screen, r: &Race) {
        const PER_M: f64 = 27.0;
        const NOSE: i32 = 172;
        let at = |meters: f64| NOSE + ((meters - r.pos) * PER_M) as i32;
        s.vgrad(0, 0, 320, 70, ramp("NAVY"), 1, 8);
        for i in 0..30u32 {
            let h = hash(i + 7);
            s.pset((h % 320) as i32, ((h >> 9) % 60) as i32, shade(ramp("WHITE"), 10 + ((h >> 20) % 16) as i32));
        }
        // The far wall, in boards ten meters long, and the floodlights over it.
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
            if let Some((car, paint, _)) = self.opponent() {
                let x = NOSE - 122 + ((r.rival_pos - r.pos) * PER_M) as i32;
                if (-200..400).contains(&x) {
                    self.draw_car(s, &car, paint.as_deref(), true, x, 128, true);
                }
            }
        }
        if let Some(o) = self.owned() {
            self.draw_owned(s, o, true, NOSE - 122, 170, true);
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

        self.draw_race_over(s, r, 40);
    }

    /// What is in the bottle, written where the gas is on a road: gold while it sprays.
    pub(crate) fn draw_bottle(&self, s: &mut Screen, y: i32, left: f64, spraying: bool) {
        if self.owned().is_some_and(|o| self.kit(o).is_some()) {
            let c = if spraying { shade(ramp("GOLD"), 28) } else if left <= 0.0 { shade(ramp("RED"), 27) } else { shade(ramp("CYAN"), 26) };
            s.text_sh(216, y, &format!("N2O {left:.1}"), c, BLACK);
        }
    }

    fn draw_race(&self, s: &mut Screen) {
        let Some(r) = &self.race else { return };
        self.draw_race_view(s, r);
        self.draw_rain(s, 320, self.car().and_then(|c| c.cockpit.as_ref()).map_or(200, |c| c.view_h));
        self.draw_bottle(s, 30, r.bottle, r.spraying);
    }

    fn draw_race_view(&self, s: &mut Screen, r: &Race) {
        match self.car().and_then(|c| c.cockpit.as_ref()) {
            Some(seat) => self.draw_race_seat(s, r, seat),
            None => self.draw_race_side(s, r),
        }
    }

    /// What is said over the race whichever way it is seen: who is where, what to do, and the timeslip.
    fn draw_race_over(&self, s: &mut Screen, r: &Race, say_y: i32) {
        if !r.alone() && r.phase != Phase::Slip {
            // Who is where: the two of you along a line the length of the strip.
            s.rect(40, 12, 201, 1, shade(ramp("CHROME"), 12));
            s.rect(240, 8, 1, 9, WHITE);
            let mark = |pos: f64| 40 + ((pos / QUARTER).clamp(0.0, 1.0) * 200.0) as i32 - 2;
            s.rect(mark(r.rival_pos), 13, 5, 3, shade(ramp(self.opponent().and_then(|v| v.1).as_deref().unwrap_or("GRAY")), 24));
            s.rect(mark(r.pos), 8, 5, 3, shade(ramp(self.owned().and_then(|o| o.paint.as_deref()).unwrap_or("WHITE")), 26));
        }
        if r.lean != 0.0 && r.phase == Phase::Run {
            // Where the car is in its lane: the wall on the left, the center line on the right.
            let (x0, w, y) = (120, 80, 22);
            let warn = r.drift.abs() > crate::race::LANE * 0.7;
            s.rect(x0, y, 2, 7, shade(ramp("CONCRETE"), 20));
            s.rect(x0 + w, y, 2, 7, shade(ramp("WHITE"), 28));
            s.rect(x0 + 2, y + 3, w - 2, 1, shade(ramp("CHROME"), 9));
            let at = x0 + w / 2 + (r.drift / crate::race::LANE * (w / 2 - 3) as f64) as i32;
            s.rect(at - 2, y + 1, 5, 5, if warn { shade(ramp("RED"), 28) } else { shade(ramp("LIME"), 26) });
        }
        let waiting = r.manual() && (r.gear == 0 || r.held) && matches!(r.phase, Phase::Stage | Phase::Tree);
        if waiting {
            // The revs, and the band to let the clutch out in.
            let (x0, w, y) = (120, 80, 22);
            let at = |rpm: f64| x0 + ((rpm / r.redline()).clamp(0.0, 1.0) * w as f64) as i32;
            let band = crate::race::BAND;
            s.rect(x0, y + 3, w, 1, shade(ramp("CHROME"), 9));
            s.rect(at(r.best_launch() - band), y + 1, at(r.best_launch() + band) - at(r.best_launch() - band), 5, shade(ramp("FOREST"), 16));
            let good = (r.rpm - r.best_launch()).abs() <= band;
            s.rect(at(r.rpm) - 1, y, 3, 7, if good { shade(ramp("LIME"), 28) } else { shade(ramp("ORANGE"), 28) });
        }
        let versus = self.versus_says();
        let started = r.phase == Phase::Run && r.pos < 30.0;
        let say = match r.phase {
            Phase::Stage if versus.is_some() => versus.as_ref().unwrap().0,
            Phase::Stage => "ENTER TO STAGE",
            Phase::Tree if r.held => "REVS IN THE GREEN. LET IT OUT ON GO.",
            Phase::Tree if r.gear == 0 && r.manual() => "DOWN: CLUTCH. SPACE: FIRST GEAR.",
            Phase::Tree if r.gear == 0 => "HOLD UP FOR REVS. SPACE ON THE GREEN.",
            _ if started && !r.red && r.start == crate::race::Launch::Bogged => "BOGGED IT",
            _ if started && !r.red && r.start == crate::race::Launch::Spun => "TOO MANY REVS: WHEELSPIN",
            Phase::Run if r.lean != 0.0 && r.gear == 1 && r.pos < 40.0 && !r.red => "LEFT AND RIGHT: HOLD YOUR LANE",
            _ if r.out == 1 => "OVER THE CENTER LINE",
            _ if r.out == -1 => "INTO THE WALL",
            _ if r.red => "RED LIGHT",
            _ => "",
        };
        s.text_sh(160 - say.len() as i32 * 4, say_y, say, if r.red { shade(ramp("RED"), 28) } else { WHITE }, BLACK);

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
                s.text(100, 110, if versus.is_some() { "THEM" } else { "HIM" }, ink);
                s.text(164, 110, &match &versus { Some((_, theirs)) => theirs.clone().unwrap_or_else(|| "...".into()), None => ms(r.rival_rt + r.rival_et) }, ink);
                let (words, c) = if let Some((words, _)) = &versus {
                    (*words, if r.won { shade(ramp("FOREST"), 14) } else { red })
                } else if r.blown { ("IT LET GO.", red) } else if r.out != 0 { ("OVER THE LINE.", red) } else if r.won { ("YOU WIN.", shade(ramp("FOREST"), 14)) } else if r.red { ("RED LIGHT.", red) } else if r.et > 0 { ("YOU LOSE.", red) } else { ("HE IS GONE.", red) };
                s.text(100, 126, words, c);
            }
            if versus.as_ref().is_none_or(|v| v.0 != "WAITING FOR THEM") {
                s.text(100, 140, "ENTER", shade(ramp("BROWN"), 14));
            }
        }
    }

    fn needle(s: &mut Screen, dial: &Dial, value: f64, top: f64) {
        let a = (-140.6 + (value / top.max(1.0)).clamp(0.0, 1.0) * 281.2_f64).to_radians();
        let [cx, cy] = dial.at;
        let (ex, ey) = (cx + (a.sin() * dial.r as f64) as i32, cy - (a.cos() * dial.r as f64) as i32);
        s.line(cx + 1, cy + 1, ex + 1, ey + 1, shade(ramp("RUBBER"), 1));
        s.line(cx, cy, ex, ey, shade(ramp("ORANGE"), 27));
        if dial.r > 10 {
            let k = if ex > cx { 0 } else { 1 };
            s.line(cx + k, cy + 1, ex + k, ey + 1, shade(ramp("RED"), 24));
        }
        s.disc(cx, cy, if dial.r > 10 { 2 } else { 1 }, shade(ramp("RUBBER"), 9));
        s.pset(cx, cy - 1, shade(ramp("CHROME"), 24));
    }

    /// The view from the seat: the cabin is the pack's painting, and over it go the things that
    /// move. Under it, through the glass, is the hood in the car's own paint.
    #[allow(clippy::too_many_arguments)]
    fn draw_cockpit(&self, s: &mut Screen, seat: &Cockpit, night: bool, rpm: f64, mph: f64, boost: f64, gear: usize, wheel: f64, shift: bool) {
        let Some((o, car)) = self.owned().zip(self.car()) else { return };
        let pic = |name: &Option<String>| name.as_ref().and_then(|n| self.pack.pictures.get(n));
        self.draw_wipers(s, seat.view_h);
        if let Some(hood) = pic(&seat.hood) {
            s.picture_tint(hood, 0, 0, Some(&Self::tint(car, o.paint.as_deref(), if night { 17 } else { 0 })));
        }
        let cabin = if night { pic(&seat.night) } else { None }.or_else(|| self.pack.pictures.get(&seat.picture));
        if let Some(cabin) = cabin {
            s.picture(cabin, 0, 0);
        }
        if let Some(d) = &seat.tach {
            Self::needle(s, d, rpm, if d.max > 0.0 { d.max } else { 9000.0 });
        }
        if let Some(d) = &seat.speedo {
            Self::needle(s, d, mph, if d.max > 0.0 { d.max } else { 160.0 });
        }
        if let Some(d) = &seat.boost {
            Self::needle(s, d, boost, if d.max > 0.0 { d.max } else { 30.0 });
        }
        for d in &seat.minis {
            let value = if d.reads.as_deref() == Some("gas") { o.gas / car.tank.max(1.0) * 100.0 } else { d.value };
            Self::needle(s, d, value, 100.0);
        }
        if let Some([x, y]) = seat.shift_light {
            s.disc(x, y, 3, shade(ramp("RUBBER"), 2));
            s.disc(x, y, 2, shade(ramp("RED"), if shift { 30 } else { 5 }));
            if shift {
                s.pset(x - 1, y - 1, shade(ramp("WHITE"), 30));
            }
        }
        if let Some([x, y]) = seat.gear_gate {
            // The gear, lit in the gate: neutral in the middle, the gears in an H either side.
            const GATE: [(i32, i32); 7] = [(0, 0), (-8, -7), (-8, 7), (0, -7), (0, 7), (8, -7), (8, 7)];
            if let Some(&(dx, dy)) = GATE.get(gear) {
                s.disc(x + dx, y + dy, 2, shade(ramp("ORANGE"), 28));
                s.pset(x + dx, y + dy, shade(ramp("WHITE"), 31));
            }
        }
        if let Some(w) = &seat.wheel {
            let rim = if night { pic(&w.night) } else { None }.or_else(|| self.pack.pictures.get(&w.picture));
            if let Some(rim) = rim {
                s.spin(rim, w.at[0], w.at[1], wheel);
            }
        }
    }

    /// The strip from the seat, at night: two lanes, the walls, the floodlights and the tree.
    fn draw_race_seat(&self, s: &mut Screen, r: &Race, seat: &Cockpit) {
        let (view, horizon) = (seat.view_h, seat.view_h * 62 / 136);
        let at = |x: f64, y: f64, z: f64| -> V3 { [x, y, z] };
        s.vgrad(0, 0, 320, horizon, ramp("NAVY"), 1, 7);
        s.vgrad(0, horizon, 320, view - horizon, ramp("ASPHALT"), 5, 10);
        s.clip(0, 0, 320, view);
        let cam = Cam { pos: [-3.0 + r.drift, 1.125, r.pos], yaw: 0.0, pitch: 0.0, focal: 200.0, cx: 160, cy: horizon };
        let mut scene = Scene3::new(cam);
        let (asphalt, rubber, white) = (ramp("ASPHALT"), ramp("RUBBER"), ramp("WHITE"));
        let here = (r.pos / 20.0).floor() as i32 * 20;
        let mut z = here - 20;
        while z < here + 440 {
            // Two lanes with a dark groove of rubber down each, a wall either side, and the stands behind.
            let (z0, z1, lit) = (z as f64, z as f64 + 20.0, if z < 80 { 13 } else { 10 });
            for k in [-1.0, 1.0] {
                scene.poly(&[at(k * 6.0, 0.0, z0), at(0.0, 0.0, z0), at(0.0, 0.0, z1), at(k * 6.0, 0.0, z1)], shade(asphalt, lit), shade(asphalt, lit + 1), -4000.0);
                scene.poly(&[at(k * 4.0, 0.0, z0), at(k * 2.0, 0.0, z0), at(k * 2.0, 0.0, z1), at(k * 4.0, 0.0, z1)], shade(rubber, lit - 3), shade(asphalt, lit - 3), -3900.0);
                let board = ramp(match (z / 20).rem_euclid(4) { 0 => "BLUE", 2 => "RED", _ => "CONCRETE" });
                scene.poly(&[at(k * 7.0, 0.0, z0), at(k * 7.0, 1.0, z0), at(k * 7.0, 1.0, z1), at(k * 7.0, 0.0, z1)], shade(board, 12), shade(board, 13), -3000.0);
                if (0..200).contains(&z) && (z / 20) % 2 == 0 {
                    let chrome = ramp("CHROME");
                    scene.poly(&[at(k * 14.0, 0.0, z0), at(k * 26.0, 9.0, z0), at(k * 26.0, 9.0, z0 + 40.0), at(k * 14.0, 0.0, z0 + 40.0)], shade(chrome, 5), shade(chrome, 7), -3500.0);
                }
            }
            scene.poly(&[at(-0.125, 0.0, z0), at(-0.125, 0.0, z1), at(0.125, 0.0, z1), at(0.125, 0.0, z0)], shade(white, 20), shade(white, 20), -3800.0);
            z += 20;
        }
        for line in [0.0, QUARTER] {
            scene.poly(&[at(-6.0, 0.0, line), at(6.0, 0.0, line), at(6.0, 0.0, line + 0.5), at(-6.0, 0.0, line + 0.5)], shade(white, 26), shade(white, 26), -3700.0);
        }
        // Whoever is in the other lane, from behind: where he is has to be worked out before the scene is spent.
        let him = (!r.alone()).then(|| self.opponent()).flatten().and_then(|rv| {
            let (x, _, depth) = scene.point(at(3.0, 0.0, r.rival_pos - 2.0))?;
            let (_, ground, _) = scene.point(at(3.0, 0.0, r.rival_pos - 2.0))?;
            (depth < 260.0).then_some((rv, x, ground, depth))
        });
        let lights: Vec<_> = (0..8).flat_map(|i| [-1.0, 1.0].map(|k| (20.0 + i as f64 * 60.0, k))).filter_map(|(z, k)| Some((scene.point(at(k * 11.0, 0.0, z))?, scene.point(at(k * 11.0, 15.0, z))?))).collect();
        let tree = scene.point(at(0.0, 0.0, 12.0)).zip(scene.point(at(0.0, 3.0, 12.0)));
        scene.end(s);
        if let Some((rv, x, ground, depth)) = him {
            let rv = Rival { name: rv.2, says: String::new(), car: rv.0, paint: rv.1, et: 0.0, limit: 0, pinks: false, king: false, figure: None };
            let car = self.pack.manifest.cars.get(&rv.car);
            match car.and_then(|c| c.rear.as_ref()).and_then(|rear| Some((rear, self.pack.pictures.get(&rear.picture)?))) {
                Some((rear, pic)) => {
                    let w = (rear.width_m * cam.focal / depth) as i32;
                    let h = w * pic.h / pic.w;
                    let tint = car.map(|c| Self::tint(c, rv.paint.as_deref(), 6));
                    s.picture_sized(pic, x - w / 2, ground - h, w, h, tint.as_ref());
                }
                None => {
                    // No picture of its back: a plain shape in its paint, with two lamps.
                    let w = (1.7 * cam.focal / depth) as i32;
                    let (h, paint) = (w * 5 / 8, ramp(rv.paint.as_deref().unwrap_or("GRAY")));
                    s.vgrad(x - w / 2, ground - h, w, h, paint, 20, 8);
                    s.rect(x - w / 2, ground - h * 2 / 5, w / 4, h / 8 + 1, shade(ramp("RED"), 22));
                    s.rect(x + w / 2 - w / 4, ground - h * 2 / 5, w / 4, h / 8 + 1, shade(ramp("RED"), 22));
                }
            }
        }
        // The floodlights: a pole, a hot spot, and the flare it makes on a dirty windshield.
        for ((bx, by, depth), (tx, ty, _)) in lights {
            let rad = (cam.focal * 0.69 / depth) as i32;
            s.line(bx, by, tx, ty, shade(ramp("CHROME"), 6));
            for f in 1..=3 {
                let len = rad * 3 + 3 - f;
                s.line(tx - len, ty, tx + len, ty, shade(white, 12 + f * 3));
                s.line(tx, ty - len, tx, ty + len, shade(white, 12 + f * 3));
            }
            s.disc(tx, ty, rad + 1, shade(ramp("CREAM"), 28));
            s.disc(tx, ty, rad, shade(white, 31));
        }
        // The tree, between the lanes just past the line.
        if let Some(((sx, sy, depth), (_, ey, _))) = tree {
            let unit = (cam.focal / depth) as i32;
            let rad = unit * 12 / 100 + 1;
            s.rect(sx - 1, ey, 3, sy - ey, shade(ramp("CHROME"), 8));
            s.rect(sx - unit * 45 / 100, ey, unit * 90 / 100 + 1, unit * 2, shade(rubber, 3));
            let light = r.light();
            for row in 0..6 {
                let y = ey + unit * (15 + row * 30) / 100;
                let on = match row { 0 => r.phase != Phase::Stage, 1..=3 => r.phase == Phase::Tree && light == row - 1, 4 => light >= 3 && !r.red, _ => r.red };
                let name = match row { 0 => "YELLOW", 1..=3 => "ORANGE", 4 => "LIME", _ => "RED" };
                for k in [-1, 1] {
                    s.disc(sx + k * unit * 22 / 100, y, if row == 0 { (rad + 1) / 2 } else { rad }, shade(ramp(name), if on { 29 } else { 5 }));
                }
            }
        }
        s.clip_off();
        self.draw_cockpit(s, seat, true, r.rpm, r.v * 2.237, r.boost, r.gear, r.wheel * 90.0, r.rpm > r.redline() - 700.0 && r.phase == Phase::Run);
        self.draw_race_over(s, r, seat.view_h * 92 / 136);
    }

    /// The walls of a box that face the camera.
    #[allow(clippy::too_many_arguments)]
    fn box3(scene: &mut Scene3, x0: f64, z0: f64, x1: f64, z1: f64, y0: f64, y1: f64, ramp: usize, bias: f64) {
        let [ex, _, ez] = scene.cam.pos;
        if ez < z0 { scene.lit(&[[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]], ramp, bias); }
        if ez > z1 { scene.lit(&[[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]], ramp, bias); }
        if ex < x0 { scene.lit(&[[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]], ramp, bias); }
        if ex > x1 { scene.lit(&[[x1, y0, z0], [x1, y0, z1], [x1, y1, z1], [x1, y1, z0]], ramp, bias); }
    }

    /// The town from the seat, by day: streets, what the map says stands on each block, the other
    /// cars, and a list of where there is to go.
    fn draw_drive(&self, s: &mut Screen) {
        let Some(town) = &self.town else { return };
        let Some(map) = self.pack.manifest.maps.get(&town.map) else { return };
        let seat = self.car().and_then(|c| c.cockpit.as_ref());
        let view = seat.map_or(200, |c| c.view_h);
        let horizon = view * 60 / 136;
        const SEE: i32 = 5;
        let (p, road) = (map.pitch, map.road);
        let g = |x: f64, z: f64| self.ground(map, x, z);
        let out = self.out_of_town();
        let road_here = self.lane_here().and_then(|n| self.pack.manifest.roads.get(&self.land()?.lanes[n.lane].id)).cloned();
        let dim = if self.dark() == 0 && !self.raining() {
            s.vgrad(0, 0, 320, horizon, ramp("SKY"), 15, 29);
            s.vgrad(0, horizon, 320, view - horizon, ramp("GRASS"), 14, 20);
            0
        } else {
            self.sky(s, horizon, view, ramp("SKY"), ramp("GRASS"))
        };
        self.draw_ridge(s, town.heading.to_radians(), horizon);
        s.clip(0, 0, 320, view);
        // The car sits on the hill it is on, and looks up or down it.
        let (hs, hc) = town.heading.to_radians().sin_cos();
        let here = g(town.x, town.z);
        let cam = Cam { pos: [town.x, here + 1.125, town.z], yaw: town.heading.to_radians(), pitch: -((g(town.x + hs * 8.0, town.z + hc * 8.0) - here) / 8.0).atan(), focal: 200.0, cx: 160, cy: horizon };
        let mut scene = Scene3::new(cam);
        (scene.dim, scene.lamps) = (dim, 45.0);
        let (tar, tar2, line) = (shade(ramp("ASPHALT"), 15), shade(ramp("ASPHALT"), 16), shade(ramp("YELLOW"), 23));
        let (cbx, cbz) = ((town.x / p).floor() as i32, (town.z / p).floor() as i32);
        let (cols, depth) = (map.cols(), map.depth());
        for i in -SEE..=SEE + 1 {
            for j in -SEE..=SEE {
                // A length of street going north, and one going east, each a block long, with dashes down the middle.
                let (c, r) = (cbx + i, cbz + j);
                if (0..=cols).contains(&c) && (0..depth).contains(&r) {
                    let (sx, sz) = (c as f64 * p, r as f64 * p);
                    scene.poly(&[[sx - road, g(sx - road, sz - road), sz - road], [sx + road, g(sx + road, sz - road), sz - road], [sx + road, g(sx + road, sz + p + road), sz + p + road], [sx - road, g(sx - road, sz + p + road), sz + p + road]], tar, tar2, -4000.0);
                    if i.abs() <= 3 && j.abs() <= 3 {
                        for a in [16.0, 40.0] {
                            scene.poly(&[[sx - 0.2, g(sx - 0.2, sz + a), sz + a], [sx - 0.2, g(sx - 0.2, sz + a + 8.0), sz + a + 8.0], [sx + 0.2, g(sx + 0.2, sz + a + 8.0), sz + a + 8.0], [sx + 0.2, g(sx + 0.2, sz + a), sz + a]], line, line, -3000.0);
                        }
                    }
                }
                let (c, r) = (cbx + j, cbz + i);
                if (0..cols).contains(&c) && (0..=depth).contains(&r) {
                    let (ex, ez) = (c as f64 * p, r as f64 * p);
                    scene.poly(&[[ex, g(ex, ez - road), ez - road], [ex + p, g(ex + p, ez - road), ez - road], [ex + p, g(ex + p, ez + road), ez + road], [ex, g(ex, ez + road), ez + road]], tar, tar2, -4000.0);
                    if i.abs() <= 3 && j.abs() <= 3 {
                        for a in [16.0, 40.0] {
                            scene.poly(&[[ex + a, g(ex + a, ez - 0.2), ez - 0.2], [ex + a + 8.0, g(ex + a + 8.0, ez - 0.2), ez - 0.2], [ex + a + 8.0, g(ex + a + 8.0, ez + 0.2), ez + 0.2], [ex + a, g(ex + a, ez + 0.2), ez + 0.2]], line, line, -3000.0);
                        }
                    }
                }
            }
        }
        for i in -SEE..=SEE {
            for j in -SEE..=SEE {
                let (c, r) = (cbx + i, cbz + j);
                if !(0..cols).contains(&c) || !(0..depth).contains(&r) {
                    continue;
                }
                let (x0, z0) = (c as f64 * p + road + 4.0, r as f64 * p + road + 4.0);
                let (x1, z1) = (x0 + p - 2.0 * road - 8.0, z0 + p - 2.0 * road - 8.0);
                let block = map.block(c, r);
                let base = g((x0 + x1) / 2.0, (z0 + z1) / 2.0);
                if block.is_none() && i.abs() <= 3 && j.abs() <= 3 {
                    // Open country between the streets: a few trees, the same ones each time.
                    for t in 0..3u32 {
                        let k = hash((c as u32).wrapping_mul(131) ^ (r as u32).wrapping_mul(977) ^ t.wrapping_mul(7919));
                        let (tx, tz, tall) = (x0 + 4.0 + (k % 100) as f64 / 100.0 * (x1 - x0 - 8.0), z0 + 4.0 + ((k >> 8) % 100) as f64 / 100.0 * (z1 - z0 - 8.0), 6.0 + (k >> 16) as f64 % 5.0);
                        let ty = g(tx, tz);
                        Self::box3(&mut scene, tx - 0.25, tz - 0.25, tx + 0.25, tz + 0.25, ty, ty + 2.0, ramp("WOOD"), 0.5);
                        let green = ramp(if k & 1 == 0 { "FOREST" } else { "GRASS" });
                        for (a, b) in [((-2.0, -2.0), (2.0, -2.0)), ((2.0, -2.0), (2.0, 2.0)), ((2.0, 2.0), (-2.0, 2.0)), ((-2.0, 2.0), (-2.0, -2.0))] {
                            scene.lit(&[[tx + a.0, ty + 1.6, tz + a.1], [tx + b.0, ty + 1.6, tz + b.1], [tx, ty + tall, tz]], green, 0.6);
                        }
                    }
                }
                let Some(b) = block.filter(|b| b.height > 0.0) else {
                    // Grass, and nothing to hit. Somewhere to go that is only a lot gets a marked curb.
                    if block.is_some_and(|b| b.water) {
                        // A pond: flat, a little below the curb, and catching the light.
                        let (wy, wave) = (base.min(g(x0, z0)).min(g(x1, z1)) - 0.4, ((self.ticks / 8 + c as u32) % 3) as i32);
                        scene.poly(&[[x0, wy, z0], [x1, wy, z0], [x1, wy, z1], [x0, wy, z1]], shade(ramp("BLUE"), 15 + wave * 2), shade(ramp("BLUE"), 17 + wave * 2), -3500.0);
                        continue;
                    }
                    let name = if block.is_some_and(|b| b.to.is_some() || b.road.is_some()) { "GOLD" } else { "GRASS" };
                    scene.poly(&[[x0, g(x0, z0), z0], [x1, g(x1, z0), z0], [x1, g(x1, z1), z1], [x0, g(x0, z1), z1]], shade(ramp(name), 20), shade(ramp(name), 22), -3500.0);
                    continue;
                };
                let wall = ramp(b.wall.as_deref().unwrap_or("CONCRETE"));
                Self::box3(&mut scene, x0, z0, x1, z1, base - 2.5, base + b.height, wall, 0.0);
                // Rows of glass, a storey at a time, on the ones near enough to see them.
                let (near, mid) = (i.abs() <= 1 && j.abs() <= 1, i.abs() <= 3 && j.abs() <= 3);
                if mid {
                    let mut f = 0;
                    while (f * 4 + 4) as f64 <= b.height && f < if near { 9 } else { 3 } {
                        Self::box3(&mut scene, x0 - 0.05, z0 - 0.05, x1 + 0.05, z1 + 0.05, base + f as f64 * 4.0 + 1.5, base + f as f64 * 4.0 + 3.0, ramp("GLASS"), 1.0);
                        f += 1;
                    }
                }
                // A cap round the top; and somewhere to go has an awning over the sidewalk.
                Self::box3(&mut scene, x0 - 0.03, z0 - 0.03, x1 + 0.03, z1 + 0.03, base + b.height - 0.5, base + b.height, ramp("RUBBER"), 1.0);
                if b.to.is_some() || b.road.is_some() {
                    Self::box3(&mut scene, x0 - 1.5, z0 - 1.5, x1 + 1.5, z1 + 1.5, base + 3.2, base + 4.0, ramp("GOLD"), 2.0);
                    // And a sign on a pole on the roof, to be seen from streets away.
                    let (mx, mz) = ((x0 + x1) / 2.0, (z0 + z1) / 2.0);
                    Self::box3(&mut scene, mx - 0.3, mz - 0.3, mx + 0.3, mz + 0.3, base + b.height, base + b.height + 7.0, ramp("CHROME"), 1.5);
                    Self::box3(&mut scene, mx - 4.0, mz - 4.0, mx + 4.0, mz + 4.0, base + b.height + 7.0, base + b.height + 10.0, ramp("GOLD"), 2.5);
                }
            }
        }
        // Past the last street, the roads on out and what is beside them.
        let land_names = self.draw_land(&mut scene, map, dim);
        // Cars about town, drawn from the pack's pictures of them where it has them: their backs
        // going your way, their fronts coming at you, their sides crossing. Boxes where it has none.
        let cars = &self.pack.manifest.cars;
        let pictured: Vec<&Car> = cars.values().filter(|c| c.rear.is_some() && c.front.is_some()).collect();
        let mut looks: Vec<(&crate::gfx::Picture, Tint, bool)> = Vec::new();
        let mut a_car = |scene: &mut Scene3, ox: f64, oz: f64, heading: f64, paint: &str, car: Option<&Car>| {
            let oy = g(ox, oz);
            let rel = (heading - town.heading + 540.0).rem_euclid(360.0) - 180.0;
            let car = car.or_else(|| (!pictured.is_empty()).then(|| pictured[paint.len() * 7 % pictured.len()]));
            if let Some(c) = car {
                let (pic, wide, mirror) = if rel.abs() < 50.0 {
                    (c.rear.as_ref().and_then(|r| Some((self.pack.pictures.get(&r.picture)?, r.width_m))), 0.0, false)
                } else if rel.abs() > 130.0 {
                    (c.front.as_ref().and_then(|r| Some((self.pack.pictures.get(&r.picture)?, r.width_m))), 0.0, false)
                } else {
                    (self.pack.pictures.get(&c.side).map(|p| (p, 4.4)), 0.0, rel < 0.0)
                };
                let _ = wide;
                if let Some((pic, width)) = pic {
                    if let Some((_, _, depth)) = scene.point([ox, oy, oz]) {
                        looks.push((pic, Self::tint(c, Some(paint), 0), mirror));
                        if scene.sprite([ox, oy, oz], width, pic.h as f64 / pic.w as f64, looks.len() - 1, depth) {
                            return;
                        }
                        looks.pop();
                    }
                }
            }
            // A body, and a cabin on it.
            let (hs, hc) = heading.to_radians().sin_cos();
            let (hl, hw) = if hs.abs() > hc.abs() { (2.1, 0.9) } else { (0.9, 2.1) };
            Self::box3(scene, ox - hl, oz - hw, ox + hl, oz + hw, oy + 0.25, oy + 0.9, ramp(paint), 0.0);
            Self::box3(scene, ox - hl * 0.8, oz - hw * 0.5, ox + hl * 0.8, oz + hw * 0.5, oy + 0.9, oy + 1.4, ramp("GLASS"), 0.5);
        };
        for o in &town.others {
            let (ox, oz) = o.at(map);
            if (ox - town.x).abs() + (oz - town.z).abs() > SEE as f64 * p {
                continue;
            }
            a_car(&mut scene, ox, oz, if o.north { 0.0 } else { 270.0 }, o.paint, None);
        }
        // And out on the roads, the traffic: trucks pulling trailers among it.
        if let Some(w) = self.land() {
            for o in &town.rolling {
                let (ox, oz, oy, h) = w.place_of(o);
                if (ox - town.x).abs() + (oz - town.z).abs() > 360.0 {
                    continue;
                }
                if o.rig {
                    let f = (h.sin(), h.cos());
                    let at = |d: f64, up: f64| [ox + f.0 * d, oy + up, oz + f.1 * d];
                    Self::stand(&mut scene, at(3.6, 0.35), f, 3.3, 1.0, 0.45, ramp("GRAY"));
                    Self::stand(&mut scene, at(3.6, 0.8), f, 2.1, 0.85, 0.6, ramp(o.paint));
                    Self::stand(&mut scene, at(-3.6, 0.3), f, 2.5, 1.0, 1.0, ramp("WHITE"));
                    Self::stand(&mut scene, at(-3.2, 1.3), f, 1.2, 0.95, 0.75, ramp("GLASS"));
                } else {
                    a_car(&mut scene, ox, oz, h.to_degrees(), o.paint, None);
                }
            }
        }
        for road_id in map.blocks.values().filter_map(|b| b.road.as_ref()) {
            // Where a road leaves town it is seen leaving: tar on out past the last street, with its line.
            // A road laid out in the world is drawn as itself, all the way.
            if self.laid(road_id) {
                continue;
            }
            let Some((gx, gz, heading)) = map.gate(road_id) else { continue };
            let (os, oc) = (heading + 180.0).to_radians().sin_cos();
            let (sx, sz) = (if os.abs() > 0.5 { gx } else { (gx / p).round() * p }, if oc.abs() > 0.5 { gz } else { (gz / p).round() * p });
            for step in 0..6 {
                let (a, b) = (step as f64 * 40.0, step as f64 * 40.0 + 40.0);
                let q = |d: f64, side: f64| { let (x, z) = (sx + os * d + oc * side, sz + oc * d - os * side); [x, g(x, z), z] };
                let tar = shade(ramp("ASPHALT"), 15 + step % 2);
                scene.poly(&[q(a, -road), q(b, -road), q(b, road), q(a, road)], tar, tar, -3900.0);
                let line = shade(ramp("YELLOW"), 23);
                scene.poly(&[q(a + 8.0, -0.15), q(a + 24.0, -0.15), q(a + 24.0, 0.15), q(a + 8.0, 0.15)], line, line, -3000.0);
            }
        }
        for &(cx, cz) in &town.cops {
            if (cx - town.x).abs() + (cz - town.z).abs() < SEE as f64 * p {
                let cy = g(cx, cz);
                Self::box3(&mut scene, cx - 0.9, cz - 2.1, cx + 0.9, cz + 2.1, cy + 0.25, cy + 0.9, ramp("RUBBER"), 0.0);
                Self::box3(&mut scene, cx - 0.75, cz - 1.0, cx + 0.75, cz + 1.0, cy + 0.9, cy + 1.4, ramp("WHITE"), 0.5);
                Self::box3(&mut scene, cx - 0.5, cz - 0.15, cx + 0.5, cz + 0.15, cy + 1.4, cy + 1.6, ramp(if town.chase.is_some() && self.ticks / 6 % 2 == 0 { "RED" } else { "BLUE" }), 0.8);
            }
        }
        // The name of each place there is to go, written over it where it can be seen.
        let mut names: Vec<(i32, i32, String, bool)> = Vec::new();
        for (c, r, b) in map.places() {
            let (mx, mz) = ((c as f64 + 0.5) * p, (r as f64 + 0.5) * p);
            let name = b.name.clone().or_else(|| b.to.as_ref().and_then(|to| self.pack.manifest.places.get(to)).map(|pl| pl.title.clone())).unwrap_or_default();
            if let Some((x, y, depth)) = scene.point([mx, g(mx, mz) + b.height + 12.0, mz]) {
                if depth > 6.0 && depth < p * 5.5 && (0..320).contains(&x) {
                    names.push((x, y.clamp(14, view - 30), name.to_uppercase().chars().take(14).collect(), depth < p * 1.6));
                }
            }
        }
        for (at, name) in land_names {
            if let Some((x, y, depth)) = scene.point(at) {
                if depth > 6.0 && depth < 360.0 && (0..320).contains(&x) {
                    // Below the gauges in the corner, which a place's name is not to be written over.
                    names.push((x, y.clamp(46, view - 30), name.to_uppercase().chars().take(14).collect(), depth < 90.0));
                }
            }
        }
        // Other people out driving here: their cars, and their names over them.
        for (who, at) in self.others_at(&format!("map:{}", town.map)) {
            if (at.x - town.x).abs() + (at.z - town.z).abs() > SEE as f64 * p {
                continue;
            }
            let (ox, oz) = (at.x, at.z);
            let oy = g(ox, oz);
            a_car(&mut scene, ox, oz, at.h, who.paint.as_deref().unwrap_or("GRAY"), who.car.as_ref().and_then(|c| cars.get(c)));
            if let Some((x, y, depth)) = scene.point([ox, oy + 1.9, oz]) {
                if depth > 2.0 && depth < p * 4.0 && (0..320).contains(&x) {
                    // People before places: a name over a car keeps its spot, below the mirror.
                    names.insert(0, (x, (y - 10).clamp(32, view - 30), who.name.to_uppercase().chars().take(12).collect(), true));
                }
            }
        }
        // Whoever is being raced down the road.
        if let Some((rx, rz, rh, r)) = self.runner_at() {
            a_car(&mut scene, rx, rz, rh, &r.paint, self.opponent().and_then(|x| cars.get(&x.0)));
        }
        scene.end_with(s, |s, id, (x, y, w, h)| {
            let (pic, tint, mirror) = &looks[id];
            if *mirror { s.picture_sized_mirror(pic, x, y, w, h, Some(tint)) } else { s.picture_sized(pic, x, y, w, h, Some(tint)) }
        });
        // One that would be written over another goes on the line above it.
        let mut put: Vec<(i32, i32, i32)> = Vec::new();
        for (x, y, name, near) in &names {
            let (wide, mut y) = (name.len() as i32 * 8, *y);
            let left = (x - wide / 2).clamp(2, 318 - wide);
            while put.iter().any(|&(l, w, py)| (py - y).abs() < 9 && left < l + w && l < left + wide) {
                y -= 9;
            }
            put.push((left, wide, y));
            s.text_sh(left, y, name, if *near { WHITE } else { shade(ramp("GOLD"), 28) }, BLACK);
        }
        self.draw_rain(s, 320, view);
        s.clip_off();
        if let Some(seat) = seat {
            let (gear, rpm, light) = match &town.transmission {
                Some(b) => (b.gear + 1, b.rpm, b.rpm > b.redline - 700.0),
                None => (if town.speed > 0.0 { self.auto.0 + 1 } else { 0 }, self.auto.1, false),
            };
            self.draw_cockpit(s, seat, self.dark() == 2, rpm, town.speed * 2.237, if town.speed > 8.0 { 6.0 } else { 0.0 }, gear, town.steer * 90.0, light);
        } else {
            let title = road_here.as_ref().map_or(map.title.clone(), |r| r.title.clone());
            self.panel(s, &title, &format!("{} mph", (town.speed * 2.237) as i32), if out { "Arrows drive. Pull in somewhere." } else { "Arrows drive. Escape turns back." });
        }
        if out {
            // Out of town, the world in the corner instead of the streets.
            self.draw_world_map(s, map, town.x, town.z, hs, hc);
        }
        // The town from above, in the corner: what is built, where there is to go, and you.
        let (mx0, my0, cell) = (44, 16, if cols.max(depth) <= 8 { 7 } else { 4 });
        if !out {
        s.rect(mx0 - 2, my0 - 2, cols * cell + 4, depth * cell + 4, BLACK);
        for r in 0..depth {
            for c in 0..cols {
                let b = map.block(c, r);
                let color = match b {
                    Some(b) if b.to.is_some() || b.road.is_some() => shade(ramp("GOLD"), 27),
                    Some(b) if b.water => shade(ramp("BLUE"), 18),
                    Some(b) if b.height > 0.0 => shade(ramp("GRAY"), 10),
                    _ => shade(ramp("GRASS"), 12),
                };
                // North is up, so the far rows of the map are drawn first.
                s.rect(mx0 + c * cell + 1, my0 + (depth - 1 - r) * cell + 1, cell - 1, cell - 1, color);
            }
        }
        if self.radar() {
            // A detector puts them on the map.
            for &(cx, cz) in town.cops.iter().take(town.town_cops) {
                s.rect(mx0 + (cx / p * cell as f64) as i32, my0 + depth * cell - (cz / p * cell as f64) as i32, 2, 2, shade(ramp("CYAN"), 28));
            }
        }
        // Other people here, as dots.
        for (_, at) in self.others_at(&format!("map:{}", town.map)) {
            s.rect(mx0 + (at.x / p * cell as f64) as i32, my0 + depth * cell - (at.z / p * cell as f64) as i32, 2, 2, shade(ramp("LIME"), 28));
        }
        let (ux, uy) = (mx0 + (town.x / p * cell as f64) as i32, my0 + depth * cell - (town.z / p * cell as f64) as i32);
        s.line(ux, uy, ux + (hs * 4.0) as i32, uy - (hc * 4.0) as i32, WHITE);
        s.disc(ux, uy, 1, shade(ramp("RED"), 28));
        }
        let gas = self.owned().map_or(0.0, |o| o.gas);
        s.text_sh(216, 18, &format!("GAS {gas:.1}"), if gas < 1.0 { shade(ramp("RED"), 27) } else { shade(ramp("WHITE"), 26) }, BLACK);
        let (limit, cops) = road_here.as_ref().map_or((map.limit, map.cops), |r| (r.limit, r.cops));
        let fast = cops > 0 && town.speed * 2.237 > limit + 10.0;
        s.text_sh(216, 27, &format!("MPH {:3}", (town.speed * 2.237) as i32), shade(ramp(if fast { "RED" } else { "WHITE" }), 26), BLACK);
        if cops > 0 {
            s.text_sh(216, 36, &format!("LIMIT {}", limit as i32), shade(ramp("WHITE"), 26), BLACK);
        }
        if town.chase.is_some() {
            let blink = self.ticks / 8 % 2 == 0;
            s.rect(0, 0, 320, 4, shade(ramp(if blink { "RED" } else { "BLUE" }), 26));
            s.text_sh(132, 8, "POLICE!", shade(ramp(if blink { "RED" } else { "WHITE" }), 27), BLACK);
        } else if out && self.radar() && self.ticks / 8 % 2 == 0 && self.dash.as_ref().is_none_or(|d| d.rival.is_none()) && town.cops.iter().any(|&(cx, cz)| (cx - town.x).abs() + (cz - town.z).abs() < 220.0) {
            s.text_sh(216, 54, "RADAR!", shade(ramp("RED"), 27), BLACK);
        }
        self.draw_bottle(s, 45, self.owned().map_or(0.0, |o| o.bottle), town.spray > 0.0);
        if let Some(b) = &town.transmission {
            s.text_sh(280, 27, &format!("G{}", b.gear + 1), shade(ramp(if b.rpm > b.redline - 700.0 { "RED" } else { "WHITE" }), 26), BLACK);
        }
        // A race or a run: who is in front and by how much, how far there is to go, and the cards.
        if let Some((d, (at, _))) = self.dash.as_ref().zip(self.dash_at().or(self.dash.as_ref().map(|d| (d.far, 0.0)))) {
            let (white, gold, red) = (shade(ramp("WHITE"), 26), shade(ramp("GOLD"), 28), shade(ramp("RED"), 27));
            let len = self.land().map_or(0.0, |w| w.lanes[d.lane].len());
            let left = (len - at).max(0.0);
            s.text_sh(98, 27, &if left >= 1000.0 { format!("{:.1} KM TO GO", left / 1000.0) } else { format!("{} M TO GO", left as i32 / 10 * 10) }, white, BLACK);
            if let Some(r) = &d.rival {
                let (gap, name) = ((r.s - at) as i32, self.opponent().map_or("THEM".to_string(), |x| x.2.to_uppercase().chars().take(6).collect()));
                let words = if gap > 2 { format!("{name} +{gap} M") } else if gap < -2 { format!("{name} {gap} M") } else { format!("{name} LEVEL") };
                s.text_sh(98, 18, &words.chars().take(14).collect::<String>(), if gap > 2 { red } else { gold }, BLACK);
                self.draw_launch(s, town.transmission.as_ref(), town.best_launch, town.hold, town.held, town.start, at - 12.0);
                if town.hold > 0 {
                    let n = format!("{}", (town.hold - 1) / 30 + 1);
                    s.text_big(150, 50, &n, BLACK, 4);
                    s.text_big(148, 48, &n, gold, 4);
                } else if at < 60.0 && self.note.is_none() && !town.held && town.start == crate::race::Launch::None {
                    let words = "FIRST TO THE END. ESCAPE GIVES IT UP.";
                    s.text_sh(160 - words.len() as i32 * 4, view * 92 / 136, words, gold, BLACK);
                }
            } else {
                let hand: Vec<String> = (0..5).map(|i| d.cards.get(i).map_or("--".to_string(), |&c| cards::short(c))).collect();
                s.text_sh(98, 18, &hand.join(" "), gold, BLACK);
            }
        }
        let site = self.site_beside().map(|st| self.pack.manifest.places.get(&st.place).map_or(st.place.clone(), |pl| pl.title.clone()));
        if let Some(name) = site.as_ref().filter(|_| self.note.is_none() && self.dash.is_none()) {
            let words = format!("ENTER: {}", name.to_uppercase());
            s.text_sh(160 - words.len() as i32 * 4, view * 92 / 136, &words, shade(ramp("GOLD"), 28), BLACK);
        }
        if seat.is_some() && town.speed < 0.5 && town.beside(map).is_none() && site.is_none() && !out && self.note.is_none() && town.crash == 0 {
            let words = "ARROWS DRIVE. PULL UP AND PRESS ENTER.";
            s.text_sh(160 - words.len() as i32 * 4, view * 92 / 136, words, shade(ramp("GOLD"), 28), BLACK);
        }
        // Someone to dare beside you: Enter is for them, and says so below.
        let someone = self.near_player().is_some() || self.net.dare_in.is_some() || self.net.dare_out.is_some();
        if let Some(b) = town.beside(map).filter(|_| !someone) {
            let name = b.name.clone().or_else(|| b.to.as_ref().and_then(|to| self.pack.manifest.places.get(to)).map(|pl| pl.title.clone())).unwrap_or_default();
            let words = format!("ENTER: {}", name.to_uppercase());
            s.text_sh(160 - words.len() as i32 * 4, view * 92 / 136, &words, shade(ramp("GOLD"), 28), BLACK);
        }
        if let Some(note) = &self.note {
            s.text_sh(160 - note.len() as i32 * 4, view * 80 / 136, &note.to_uppercase(), shade(ramp("GOLD"), 28), BLACK);
        }
        if town.crash > 0 {
            s.vgrad(108, 50, 104, 22, ramp("RED"), 26, 16);
            s.frame(108, 50, 104, 22, WHITE);
            s.text_sh(136, 57, "CRASH!", WHITE, BLACK);
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
        let record = format!("RACES WON {}  LOST {}", self.state.wins, self.state.losses);
        s.text_sh(160 - record.len() as i32 * 4, 164, &record, WHITE, BLACK);
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
