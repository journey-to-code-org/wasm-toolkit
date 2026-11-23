//! A content pack: everything the engine plays, in one file.
//!
//! The engine knows rules; the pack says what there is. A pack is a list of named files: `pack.json`,
//! which describes the cars and the places, and the pictures it names. `tools/pack.py` makes one
//! from a folder. The format is in `docs/ENGINE.md`.

use crate::gfx::{Picture, RAMPS};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Where a new game starts.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Start {
    #[serde(default)]
    pub cash: i32,
    pub place: String,
    /// The car you begin with, if any.
    #[serde(default)]
    pub car: Option<String>,
}

/// Which of the picture's ramps are the paint, so a car can be drawn in any color.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct PaintRamps {
    /// The paint's mid tones, its shadows, its highlights and its glints: the ramps the picture uses for each.
    #[serde(default)]
    pub mid: Option<String>,
    #[serde(default)]
    pub dark: Option<String>,
    #[serde(default)]
    pub light: Option<String>,
    #[serde(default)]
    pub glint: Option<String>,
}

/// How long the game runs: so many days, of so many hours. Everything you do takes hours.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Season {
    pub days: i32,
    pub hours_per_day: i32,
    /// How many days in a hundred it rains. The first day is always dry.
    pub rain: i32,
}

impl Default for Season {
    fn default() -> Season {
        Season { days: 90, hours_per_day: 12, rain: 15 }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Rebuild {
    pub cost: i32,
    pub cond: i32,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Nitrous {
    pub gain: i32,
    #[serde(default = "bottle")]
    pub seconds: f64,
    /// What a fill costs.
    #[serde(default = "fill")]
    pub fill: i32,
}

fn bottle() -> f64 {
    8.0
}
fn fill() -> i32 {
    20
}

/// The event the season is named for: what it costs to enter, what the winner takes, how many
/// rounds there are, and how often it is held, in days.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Shootout {
    pub entry: i32,
    pub purse: i32,
    pub rounds: usize,
    pub every: i32,
}

impl Default for Shootout {
    fn default() -> Shootout {
        Shootout { entry: 50, purse: 400, rounds: 3, every: 7 }
    }
}

/// Something wrong with a car sold as it stands: what the ad says about it, the slots whose
/// parts it has done for, and the ones it has done for that the ad does not mention.
#[derive(Deserialize, Clone, Debug)]
pub struct Fault {
    pub says: String,
    #[serde(default)]
    pub breaks: Vec<String>,
    #[serde(default)]
    pub hidden: Vec<String>,
}

/// Somewhere a car sold as it stands might be, and what bringing it home from there costs.
#[derive(Deserialize, Clone, Debug)]
pub struct Seller {
    pub from: String,
    pub ship: i32,
    /// How many days the trailer takes to bring one home. Nothing: the same day.
    #[serde(default)]
    pub days: i32,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Poker {
    pub entry: i32,
    pub riders: usize,
}

impl Default for Poker {
    fn default() -> Poker {
        Poker { entry: 20, riders: 5 }
    }
}

/// What a part does to the sound of a car, each as a percentage more (or, below nothing, less):
/// how loud the exhaust is, how deep, how raspy, how much the intake roars, and how loud and high
/// the turbo whistles and how hard it blows off.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(default)]
pub struct PartSound {
    pub loud: i32,
    pub deep: i32,
    pub rasp: i32,
    pub intake: i32,
    pub whistle: i32,
}

/// What having the shop tune a car costs, and how many hours it takes.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Tuneup {
    pub cost: i32,
    pub hours: i32,
}

impl Default for Tuneup {
    fn default() -> Tuneup {
        Tuneup { cost: 80, hours: 3 }
    }
}

/// What a respray costs, and how many hours it takes.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Respray {
    pub cost: i32,
    pub hours: i32,
}

impl Default for Respray {
    fn default() -> Respray {
        Respray { cost: 60, hours: 3 }
    }
}

impl Default for Rebuild {
    fn default() -> Rebuild {
        Rebuild { cost: 250, cond: 85 }
    }
}

/// A turbo: the engine speed it starts to make boost at, how much it makes, and how slowly it
/// gets there (bigger is lazier).
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Boost {
    pub from: f64,
    pub psi: f64,
    #[serde(default = "six")]
    pub lag: f64,
}

fn six() -> f64 {
    6.0
}

/// What makes a car go. Left out, a car gets a two-liter turbo four and five gears.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Drive {
    /// Torque at the wheels, in pound-feet, at each engine speed: pairs of rpm and torque, rising.
    pub torque: Vec<[f64; 2]>,
    /// The transmission, first gear first, and the final drive behind it.
    pub gears: Vec<f64>,
    #[serde(rename = "final")]
    pub final_drive: f64,
    pub redline: f64,
    /// The engine speed it holds on the line with your foot down.
    pub launch: f64,
    pub weight_lb: f64,
    /// The hardest the tires can push it, in meters a second, each second.
    pub grip: f64,
    pub boost: Option<Boost>,
    /// An automatic: it changes gear itself, on the strip and on the road.
    pub auto: bool,
    /// Which wheels it drives: the front ones pull it sideways on a hard launch (torque steer),
    /// all four less so, the back ones hardly at all.
    pub driven: Driven,
    /// How many cylinders it has: what the engine sounds like, a four or a V8.
    pub cylinders: u32,
}

/// The wheels the engine turns.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Driven {
    #[default]
    Rear,
    Front,
    All,
}

impl Driven {
    /// How hard a full-grip launch in first pulls the car sideways, in meters a second each second.
    pub fn torque_steer(self) -> f64 {
        match self {
            Driven::Front => 3.2,
            Driven::All => 1.3,
            Driven::Rear => 0.15,
        }
    }
}

impl Default for Drive {
    fn default() -> Drive {
        Drive {
            torque: [[800, 60], [2000, 105], [3000, 146], [3400, 190], [3800, 250], [4200, 305], [4400, 314], [4600, 310], [5000, 300],
                [5400, 285], [5800, 270], [6200, 250], [6450, 240], [7200, 205], [8000, 150]].iter().map(|p| [p[0] as f64, p[1] as f64]).collect(),
            gears: vec![3.10, 1.85, 1.25, 0.92, 0.72],
            final_drive: 4.2,
            redline: 7200.0,
            launch: 5600.0,
            weight_lb: 3060.0,
            auto: false,
            driven: Driven::Rear,
            cylinders: 4,
            grip: 12.0,
            boost: Some(Boost { from: 2200.0, psi: 20.0, lag: 6.0 }),
        }
    }
}

/// Something to bolt on. It goes in a slot (one part to a slot), and makes more torque, or
/// changes the turbo, or both.
#[derive(Deserialize, Clone, Debug)]
pub struct Part {
    pub name: String,
    pub slot: String,
    pub price: i32,
    /// What the ad says about it.
    #[serde(default)]
    pub pitch: String,
    /// How much more torque, as a percentage.
    #[serde(default)]
    pub torque: i32,
    #[serde(default)]
    pub boost: Option<Boost>,
    /// The cars it goes on. Left out, it goes on any.
    #[serde(default)]
    pub fits: Vec<String>,
    /// How much more the tires can push, as a percentage: for tires, a differential, anything that puts it down.
    #[serde(default)]
    pub grip: i32,
    /// What it adds to the car's weight, in pounds. Less than nothing for a part that saves some.
    #[serde(default)]
    pub weight_lb: f64,
    /// A transmission: its ratios, first gear first, in place of the car's own. And a final drive.
    #[serde(default)]
    pub gears: Vec<f64>,
    #[serde(default, rename = "final")]
    pub final_drive: Option<f64>,
    /// A redline, in place of the car's own: for a built bottom end.
    #[serde(default)]
    pub redline: Option<f64>,
    /// A different engine: its torque curve, in place of the car's own. Pairs of rpm and
    /// pound-feet, rising, as for a car.
    #[serde(default)]
    pub curve: Vec<[f64; 2]>,
    /// What it has to have on the car with it to go on: each a few words, such as "a big-frame
    /// manifold", that some other fitted part gives.
    #[serde(default)]
    pub needs: Vec<String>,
    #[serde(default)]
    pub gives: Vec<String>,
    /// For a part of the driveline: the torque it will take, in pound-feet. An engine that makes
    /// more will sooner or later break it on a launch.
    #[serde(default)]
    pub holds: Option<f64>,
    /// A clutch: given more torque than it holds it does not snap, it slips. The car gets only
    /// what the clutch will pass, and every launch like that wears it, until it is done for.
    #[serde(default)]
    pub slips: bool,
    /// Brakes: how much harder (or, below nothing, softer) than stock they stop, as a percentage.
    /// A part with this is the car's brakes, and stops worse as it wears.
    #[serde(default)]
    pub brakes: Option<i32>,
    /// Suspension: how much less a fast bend throws the car than on stock, as a percentage. A part
    /// with this is the car's suspension, and handles worse as it wears.
    #[serde(default)]
    pub handling: Option<i32>,
    /// Suspension that can be set up: from the garage, stiff for the strip or soft for the road.
    #[serde(default)]
    pub adjustable: bool,
    /// Swapping it, on or off, leaves the engine needing a tune: until it has one it runs down.
    #[serde(default)]
    pub retune: bool,
    /// How it changes what the car sounds like.
    #[serde(default)]
    pub sound: Option<PartSound>,
    /// A factory part only some cars came with: the per cent of cars with this slot on their
    /// build sheet that have it in place of the stock one. It is never in the paper new.
    #[serde(default)]
    pub rare: u32,
    /// How fast it wears: the per cent of itself a pass down the strip takes, and four kilometers
    /// of road the same. Under half it costs power; at nothing it is done for.
    #[serde(default)]
    pub wears: f64,
    /// Slots whose parts go with it when it wears out in use: a timing belt takes the valves.
    #[serde(default)]
    pub takes: Vec<String>,
    /// A battery: run down to 15 per cent or less it is flat, not finished, and a charge brings it back.
    #[serde(default)]
    pub charges: bool,
    /// A laptop and a cable: with it the car's fuel, timing and boost can be changed at the rollers.
    #[serde(default)]
    pub tuner: bool,
    /// For a part that is a transmission: whether it is an automatic. Left out, the car's own kind stays.
    #[serde(default)]
    pub auto: Option<bool>,
    /// A nitrous kit: how much more the engine makes while it sprays, as a percentage, and how
    /// many seconds a full bottle lasts.
    #[serde(default)]
    pub nitrous: Option<Nitrous>,
    /// A radar detector: it warns of police ahead on a road.
    #[serde(default)]
    pub radar: bool,
    /// What it looks like on the car from the side: a picture the size of the car's own, drawn
    /// over it wherever the car is seen from the side. A wing, a scoop, a set of wheels.
    #[serde(default)]
    pub side: Option<String>,
    /// What it looks like bolted on under the hood: a picture drawn over its slot's box, in place
    /// of the stock part that the engine bay's picture shows there.
    #[serde(default)]
    pub picture: Option<String>,
}

impl Part {
    pub fn goes_on(&self, car: &str) -> bool {
        self.fits.is_empty() || self.fits.iter().any(|c| c == car)
    }
}

/// Someone to race. They turn up in the order the pack lists them.
#[derive(Deserialize, Clone, Debug)]
pub struct Rival {
    pub name: String,
    #[serde(default)]
    pub says: String,
    pub car: String,
    #[serde(default)]
    pub paint: Option<String>,
    /// What they run the quarter mile in, in seconds.
    pub et: f64,
    /// The most money they will race for.
    #[serde(default)]
    pub limit: i32,
    /// Whether they will race for cars.
    #[serde(default)]
    pub pinks: bool,
    /// The one to beat: races for cars and nothing else, and beating them wins the game.
    #[serde(default)]
    pub king: bool,
    /// A picture of them, to stand by their car.
    #[serde(default)]
    pub figure: Option<String>,
}

/// A place under the hood where a part sits: its box on the picture (x, y, width, height), the
/// bolts that hold the part there, and what the part the car came with is called.
///
/// In a pack it is the box alone, `[56, 98, 42, 52]`, which has a bolt at each corner; or
/// `{ "rect": [...], "bolts": 2 }` for some other number, laid out for you; or
/// `{ "rect": [...], "bolts": [[60, 100], [90, 140]] }` to say where each bolt is. None to eight.
///
/// Or its shape, corner by corner, for a part that is no box: `{ "poly": [[x, y], ...] }`. A
/// point is inside it if a line from it crosses the outline an odd number of times, so a ring is
/// drawn as its outside, a cut in to the inside, and its inside the other way round.
#[derive(Deserialize, Clone, Debug)]
#[serde(from = "SlotIn")]
pub struct Slot {
    /// Its box: the shape's bounds, when it has a shape.
    pub rect: [i32; 4],
    pub poly: Vec<[i32; 2]>,
    pub bolts: Vec<[i32; 2]>,
    /// Out of sight until what it is behind is off: the picture does not show it, so it is
    /// drawn plain, with its name, once uncovered.
    pub covered: bool,
    pub stock: Option<String>,
    /// The place this one is behind: its part cannot be lifted until that one's is off, and that
    /// one's cannot go back on until this one's is.
    pub behind: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BoltsIn {
    Count(usize),
    At(Vec<[i32; 2]>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SlotIn {
    Rect([i32; 4]),
    Full {
        #[serde(default)]
        rect: Option<[i32; 4]>,
        #[serde(default)]
        poly: Vec<[i32; 2]>,
        #[serde(default)]
        bolts: Option<BoltsIn>,
        #[serde(default)]
        stock: Option<String>,
        #[serde(default)]
        behind: Option<String>,
        #[serde(default)]
        covered: bool,
    },
}

/// Where `n` bolts go on a box when the pack does not say: the corners first, then the middles of the sides.
pub fn bolts_round(rect: [i32; 4], n: usize) -> Vec<[i32; 2]> {
    let [x, y, w, h] = rect;
    let (l, r, t, b, cx, cy) = (x + 5, x + w - 6, y + 5, y + h - 6, x + w / 2, y + h / 2);
    let order: &[[i32; 2]] = match n {
        0 | 1 => &[[cx, cy]],
        2 => &[[l, t], [r, b]],
        3 => &[[l, t], [r, t], [cx, b]],
        _ => &[[l, t], [r, t], [r, b], [l, b], [cx, t], [cx, b], [l, cy], [r, cy]],
    };
    order.iter().copied().take(n.min(8)).collect()
}

impl Slot {
    /// Whether a point on the picture is on this part.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        if self.poly.len() < 3 {
            let [rx, ry, w, h] = self.rect;
            return x >= rx && x < rx + w && y >= ry && y < ry + h;
        }
        let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
        let mut inside = false;
        let n = self.poly.len();
        for i in 0..n {
            let ([ax, ay], [bx, by]) = (self.poly[i], self.poly[(i + 1) % n]);
            let (ax, ay, bx, by) = (ax as f64, ay as f64, bx as f64, by as f64);
            if (ay > py) != (by > py) && px < ax + (bx - ax) * (py - ay) / (by - ay) {
                inside = !inside;
            }
        }
        inside
    }

    /// Where its middle is, for a tag on it.
    pub fn middle(&self) -> (i32, i32) {
        let [x, y, w, h] = self.rect;
        (x + w / 2, y + h / 2)
    }
}

/// The box round some points.
pub fn bounds(pts: &[[i32; 2]]) -> [i32; 4] {
    let (x0, y0) = (pts.iter().map(|p| p[0]).min().unwrap_or(0), pts.iter().map(|p| p[1]).min().unwrap_or(0));
    let (x1, y1) = (pts.iter().map(|p| p[0]).max().unwrap_or(0), pts.iter().map(|p| p[1]).max().unwrap_or(0));
    [x0, y0, (x1 - x0).max(1), (y1 - y0).max(1)]
}

impl From<SlotIn> for Slot {
    fn from(s: SlotIn) -> Slot {
        let (rect, poly, bolts, stock, behind, covered) = match s {
            SlotIn::Rect(rect) => (rect, Vec::new(), None, None, None, false),
            SlotIn::Full { rect, poly, bolts, stock, behind, covered } => {
                let poly: Vec<[i32; 2]> = if poly.len() >= 3 { poly.into_iter().take(160).collect() } else { Vec::new() };
                (if poly.is_empty() { rect.unwrap_or([0, 0, 1, 1]) } else { bounds(&poly) }, poly, bolts, stock, behind.clone(), covered && behind.is_some())
            }
        };
        let bolts = match bolts {
            Some(BoltsIn::At(at)) if !at.is_empty() => at.into_iter().take(8).collect(),
            Some(BoltsIn::Count(n)) => bolts_round(rect, n),
            _ => bolts_round(rect, 4),
        };
        Slot { rect, poly, bolts, stock, behind, covered }
    }
}

/// Under the hood: a picture of the engine bay, and the places on it where parts sit. A car with
/// one has its parts taken off and put on there, bolt by bolt. A slot here need not have any part
/// in the paper: it is then only the stock part, which still comes off.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Hood {
    pub picture: String,
    pub slots: BTreeMap<String, Slot>,
}

/// Somewhere else on the car that is worked on as under the hood is: the trunk, underneath it on
/// the lift, a wheel, the inside. Its own picture and its own places, and a name for its tab.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Area {
    pub id: String,
    pub title: String,
    pub picture: String,
    pub slots: BTreeMap<String, Slot>,
}

impl Car {
    /// Every part of the car that is worked on bolt by bolt: under the hood first, then the
    /// rest in the pack's order. Each as its id, its name, and its picture and places.
    pub fn areas(&self) -> Vec<(&str, &str, &str, &BTreeMap<String, Slot>)> {
        let hood = self.hood.iter().map(|h| ("hood", "Under the hood", h.picture.as_str(), &h.slots));
        hood.chain(self.areas.iter().map(|a| (a.id.as_str(), a.title.as_str(), a.picture.as_str(), &a.slots))).collect()
    }

    /// Which of them a slot is worked on in.
    pub fn area_of(&self, slot: &str) -> Option<usize> {
        self.areas().iter().position(|a| a.3.contains_key(slot))
    }
}

/// A dial on the dash: where its needle turns, how long the needle is, and what it reads at full sweep.
#[derive(Deserialize, Clone, Debug)]
pub struct Dial {
    pub at: [i32; 2],
    pub r: i32,
    #[serde(default)]
    pub max: f64,
    /// `"gas"` for a small dial that shows what is in the tank.
    #[serde(default)]
    pub reads: Option<String>,
    /// For a dial that only sits there (fuel, temperature): where its needle points, 0 to 100.
    #[serde(default)]
    pub value: f64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Wheel {
    /// A square picture of the steering wheel, turned about its middle.
    pub picture: String,
    #[serde(default)]
    pub night: Option<String>,
    pub at: [i32; 2],
}

/// The view from the seat: a painting of the cabin with the glass left clear, and where on it the
/// things that move are. A car with one is raced from behind its wheel.
#[derive(Deserialize, Clone, Debug)]
pub struct Cockpit {
    pub picture: String,
    /// The same at night, the cabin dark and the dials lit.
    #[serde(default)]
    pub night: Option<String>,
    /// The hood as seen through the glass, drawn under the painting in the car's paint.
    #[serde(default, alias = "bonnet")]
    pub hood: Option<String>,
    /// How much of the screen, from the top, is the view out; the horizon sits partway down it.
    #[serde(default = "view_h")]
    pub view_h: i32,
    #[serde(default)]
    pub tach: Option<Dial>,
    #[serde(default)]
    pub speedo: Option<Dial>,
    #[serde(default)]
    pub boost: Option<Dial>,
    #[serde(default)]
    pub minis: Vec<Dial>,
    #[serde(default)]
    pub shift_light: Option<[i32; 2]>,
    /// The middle of the gear lever's gate, where the gear in use lights up.
    #[serde(default)]
    pub gear_gate: Option<[i32; 2]>,
    #[serde(default)]
    pub wheel: Option<Wheel>,
}

fn tank() -> f64 {
    17.0
}

fn view_h() -> i32 {
    136
}

/// The car from behind, for when it is ahead of you on the road, and how wide it is in meters.
#[derive(Deserialize, Clone, Debug)]
pub struct Rear {
    pub picture: String,
    #[serde(default = "car_width")]
    pub width_m: f64,
}

fn car_width() -> f64 {
    1.7
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Car {
    pub name: String,
    /// The car from the side, and the same with its headlamps on or up.
    pub side: String,
    #[serde(default)]
    pub side_lamps: Option<String>,
    /// How far down the side picture the tires touch the ground.
    pub ground: i32,
    /// The color it comes in, as a ramp name.
    #[serde(default)]
    pub paint: Option<String>,
    #[serde(default)]
    pub paint_ramps: PaintRamps,
    /// What one in fair shape costs in the paper. A car with no price is never for sale.
    #[serde(default)]
    pub price: Option<i32>,
    #[serde(default)]
    pub drive: Drive,
    #[serde(default)]
    pub hood: Option<Hood>,
    /// The other places on it worked on as under the hood is.
    #[serde(default)]
    pub areas: Vec<Area>,
    /// How many gallons the tank holds.
    #[serde(default = "tank")]
    pub tank: f64,
    /// The build sheet: what the car left the factory with, slot by slot, each an ordinary part
    /// of the pack. A car with one is made of these parts, any of which can come off, wear out or
    /// break. Slots under the hood are not on it: those keep to the hood's own way of working.
    #[serde(default)]
    pub stock: BTreeMap<String, String>,
    /// The slots of the build sheet the car will run without: seats, cruise control.
    #[serde(default)]
    pub optional: Vec<String>,
    #[serde(default)]
    pub cockpit: Option<Cockpit>,
    #[serde(default)]
    pub rear: Option<Rear>,
    /// And from in front, for when it is coming the other way.
    #[serde(default)]
    pub front: Option<Rear>,
}

/// Something in a place that can be picked: where it is, what it is called, and what picking it does.
#[derive(Deserialize, Clone, Debug)]
pub struct Spot {
    /// x, y, width, height. A spot marked `on_car` moves with the car.
    pub rect: [i32; 4],
    pub label: String,
    /// `go:<place>`, `say:<words>`, `lamps`, or nothing.
    #[serde(default, rename = "do")]
    pub action: String,
    /// Only there when you have a car.
    #[serde(default)]
    pub needs_car: bool,
    /// Words to draw the spot as a button with, for a place whose picture does not show it.
    #[serde(default)]
    pub button: Option<String>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Place {
    pub title: String,
    /// The room, as a picture. Without one the place is a plain backdrop in `backdrop`'s ramp.
    #[serde(default)]
    pub picture: Option<String>,
    #[serde(default)]
    pub backdrop: Option<String>,
    /// Where your car stands: the x of its picture's left edge, and the y of the ground.
    #[serde(default)]
    pub car_at: Option<[i32; 2]>,
    /// Where whoever you are up against stands. A place with this is where the rivals pull in.
    #[serde(default)]
    pub rival_at: Option<[i32; 2]>,
    #[serde(default)]
    pub spots: Vec<Spot>,
    /// Out of doors: it gets dark here when the day does.
    #[serde(default)]
    pub outdoors: bool,
    /// Whoever else is standing about: pictures, each with the x of its left edge and the y of its feet.
    #[serde(default)]
    pub people: Vec<Stand>,
}

/// A picture standing in a place.
#[derive(Deserialize, Clone, Debug)]
pub struct Stand {
    pub picture: String,
    pub at: [i32; 2],
}

/// The most people a place has room for.
pub const MOST_PEOPLE: usize = 12;

/// What stands on a block of a map.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Block {
    /// What it is built of, as a color, and how tall it is in meters. With neither it is grass.
    #[serde(default)]
    pub wall: Option<String>,
    #[serde(default)]
    pub height: f64,
    /// Somewhere to go: pull up beside it and you are in this place.
    #[serde(default)]
    pub to: Option<String>,
    /// What it is called on the list of places to go.
    #[serde(default)]
    pub name: Option<String>,
    /// The way out of town: pull up beside it and you are on this road.
    #[serde(default)]
    pub road: Option<String>,
    /// Water: a pond or a river, which cannot be driven on.
    #[serde(default)]
    pub water: bool,
}

/// A town to drive round: blocks in rows, with a street between every two and round the outside.
/// Each letter of a row is a block, looked up in `blocks`; a full stop is grass. The top row is
/// the north side.
#[derive(Deserialize, Clone, Debug)]
pub struct Map {
    pub title: String,
    pub rows: Vec<String>,
    /// Whether the roads out of it are laid out on the ground past its last street and driven
    /// there, one world with no change of view, rather than each on the road view of its own.
    #[serde(default)]
    pub open: bool,
    #[serde(default)]
    pub blocks: BTreeMap<String, Block>,
    /// Meters from one street to the next, and half the width of a street.
    #[serde(default = "pitch")]
    pub pitch: f64,
    #[serde(default = "road")]
    pub road: f64,
    /// How many other cars are about.
    #[serde(default = "traffic")]
    pub traffic: usize,
    #[serde(default = "gas_per_km")]
    pub gas_per_km: f64,
    /// How hilly the ground is: the height of the hills, in meters. Left out, it is flat.
    #[serde(default)]
    pub hills: f64,
    /// The law in town: how many police sit at its corners, the limit in miles an hour, and what
    /// being caught well over it costs. With no police there is no limit.
    #[serde(default)]
    pub cops: usize,
    #[serde(default = "town_limit")]
    pub limit: f64,
    #[serde(default = "town_ticket")]
    pub ticket: i32,
}

fn pitch() -> f64 {
    64.0
}
fn road() -> f64 {
    8.0
}
fn traffic() -> usize {
    8
}
fn gas_per_km() -> f64 {
    0.4
}

impl Map {
    pub fn cols(&self) -> i32 {
        self.rows.first().map_or(0, |r| r.chars().count() as i32)
    }

    pub fn depth(&self) -> i32 {
        self.rows.len() as i32
    }

    /// The block in column `c` from the west and row `r` from the south, if the map goes that far
    /// and something stands there.
    pub fn block(&self, c: i32, r: i32) -> Option<&Block> {
        if c < 0 || r < 0 || c >= self.cols() || r >= self.depth() {
            return None;
        }
        let ch = self.rows[(self.depth() - 1 - r) as usize].chars().nth(c as usize)?;
        self.blocks.get(&ch.to_string())
    }

    /// Every block that is somewhere to go: its column, its row from the south, and the block.
    /// Where a road leaves this map, if the block that leads onto it is at the edge: a point on
    /// the street beside it, in the lane coming in, and the way that faces, in degrees.
    pub fn gate(&self, road: &str) -> Option<(f64, f64, f64)> {
        let (c, r, _) = self.places().into_iter().find(|(_, _, b)| b.road.as_deref() == Some(road))?;
        let (p, w, d) = (self.pitch, self.cols() as f64 * self.pitch, self.depth() as f64 * self.pitch);
        let (cf, rf) = (c as f64, r as f64);
        if r == self.depth() - 1 {
            Some(((cf + 1.0) * p - 2.0, d + self.road - 6.0, 180.0))
        } else if r == 0 {
            Some((cf * p + 2.0, 6.0 - self.road, 0.0))
        } else if c == self.cols() - 1 {
            Some((w + self.road - 6.0, rf * p + 2.0, 270.0))
        } else if c == 0 {
            Some((6.0 - self.road, (rf + 1.0) * p - 2.0, 90.0))
        } else {
            None
        }
    }

    /// How high the ground is at a point: long, slow hills, the same ones every time.
    pub fn height(&self, x: f64, z: f64) -> f64 {
        if self.hills == 0.0 {
            return 0.0;
        }
        let p = self.pitch;
        self.hills.clamp(0.0, 30.0) * ((x / (p * 1.3)).sin() + (z / (p * 1.7)).cos() + ((x + z) / (p * 2.3)).sin()) / 3.0
    }

    pub fn places(&self) -> Vec<(i32, i32, &Block)> {
        let mut out = Vec::new();
        for r in 0..self.depth() {
            for c in 0..self.cols() {
                if let Some(b) = self.block(c, r).filter(|b| b.to.is_some() || b.road.is_some()) {
                    out.push((c, r, b));
                }
            }
        }
        out
    }
}

/// A stretch of a road: how long, how it bends (-1 hard left to 1 hard right), how much it
/// climbs in meters, and what is beside it: "trees", "rock", "drop", "fields" or "posts".
#[derive(Deserialize, Clone, Debug)]
pub struct Stretch {
    pub len: f64,
    #[serde(default)]
    pub curve: f64,
    #[serde(default)]
    pub hill: f64,
    #[serde(default = "trees")]
    pub left: String,
    #[serde(default = "trees")]
    pub right: String,
}

fn trees() -> String {
    "trees".into()
}

pub const SIDES: [&str; 6] = ["trees", "rock", "drop", "fields", "posts", "water"];

/// A road out of town: two lanes that bend and climb between two ends, with other cars on it and
/// the law beside it. An end is a place, or `map:<id>` for a block of that map marked with this road.
#[derive(Deserialize, Clone, Debug)]
pub struct Road {
    pub title: String,
    pub ends: [String; 2],
    pub parts: Vec<Stretch>,
    /// The speed limit, in miles an hour, and what being caught well over it costs.
    #[serde(default = "limit")]
    pub limit: f64,
    #[serde(default = "ticket")]
    pub ticket: i32,
    /// How many other cars are on it, and how many police are parked beside it.
    #[serde(default = "road_traffic")]
    pub traffic: usize,
    #[serde(default = "one")]
    pub cops: usize,
    #[serde(default = "road_gas")]
    pub gas_per_km: f64,
    /// How long the trip takes off the clock.
    #[serde(default = "two")]
    pub hours: i32,
    #[serde(default)]
    pub sky: Option<String>,
    #[serde(default)]
    pub ground: Option<String>,
    /// How much of the other traffic is trucks towing cars on trailers: 0 none, 1 all of it.
    #[serde(default)]
    pub rigs: f64,
    /// How many rivals have to have been beaten before it can be taken from its first end.
    #[serde(default)]
    pub rung: usize,
}

fn town_limit() -> f64 {
    35.0
}
fn town_ticket() -> i32 {
    30
}
fn limit() -> f64 {
    55.0
}
fn ticket() -> i32 {
    40
}
fn road_traffic() -> usize {
    6
}
fn one() -> usize {
    1
}
fn two() -> i32 {
    2
}
fn road_gas() -> f64 {
    0.25
}

impl Road {
    pub fn len(&self) -> f64 {
        self.parts.iter().map(|p| p.len).sum()
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Manifest {
    pub name: String,
    pub start: Start,
    #[serde(default)]
    pub cars: BTreeMap<String, Car>,
    pub places: BTreeMap<String, Place>,
    #[serde(default)]
    pub season: Season,
    #[serde(default)]
    pub parts: BTreeMap<String, Part>,
    #[serde(default)]
    pub rivals: Vec<Rival>,
    /// The colors cars in the paper come in. Left out, eight ordinary ones.
    #[serde(default)]
    pub paints: Vec<String>,
    /// What it costs to have a tired or blown engine rebuilt, and how healthy it comes back, as a percentage.
    #[serde(default)]
    pub rebuild: Rebuild,
    #[serde(default)]
    pub respray: Respray,
    #[serde(default)]
    pub tuneup: Tuneup,
    /// The shootout: a knockout on the strip every so many days. Left out, there is none.
    #[serde(default)]
    pub shootout: Option<Shootout>,
    /// What can be wrong with a car sold as it stands, and where such cars are.
    #[serde(default)]
    pub faults: Vec<Fault>,
    #[serde(default)]
    pub sellers: Vec<Seller>,
    /// What a poker run costs to ride in, and how many others ride.
    #[serde(default)]
    pub poker: Poker,
    /// A picture for the title screen, 320 by 240. Without one the pack's name is written up.
    #[serde(default)]
    pub title: Option<String>,
    /// What can be put on a race, in dollars. Left out: 10, 25, 50, 100.
    #[serde(default)]
    pub bets: Vec<i32>,
    #[serde(default)]
    pub maps: BTreeMap<String, Map>,
    #[serde(default)]
    pub roads: BTreeMap<String, Road>,
    /// The help-wanted ads in the paper: work that pays whether you have a car or not. Left out,
    /// three ordinary ones.
    #[serde(default)]
    pub jobs: Vec<Job>,
}

/// A job in the paper: so many hours of your day for so many dollars.
#[derive(Deserialize, Clone, Debug)]
pub struct Job {
    pub name: String,
    pub hours: i32,
    pub pay: i32,
}

/// The most jobs the paper has room to list.
pub const MOST_JOBS: usize = 6;

/// What a spot can do. Anything else in a pack is a mistake, and is said to be.
pub const ACTIONS: [&str; 25] = ["", "lamps", "paper", "parts", "hood", "work", "yard", "calendar", "race", "next", "solo", "gas", "online", "rebuild", "paint", "dyno", "nitrous", "sleep", "shootout", "bodywork", "tires", "charge", "tuneup", "ghosts", "setup"];
/// The most parts the paper has room to list.
pub const MOST_PARTS: usize = 120;

#[derive(Default)]
pub struct Pack {
    pub manifest: Manifest,
    pub pictures: BTreeMap<String, Picture>,
}

impl Pack {
    /// Read a pack. The file is `WPK1`, a count, then for each entry a name and its bytes, with
    /// lengths as little-endian numbers. What is wrong with a bad pack is said in words.
    pub fn parse(bytes: &[u8]) -> Result<Pack, String> {
        let mut at = 0usize;
        let mut take = |n: usize| -> Result<&[u8], String> {
            let piece = bytes.get(at..at + n).ok_or_else(|| "the pack ends early".to_string())?;
            at += n;
            Ok(piece)
        };
        if take(4)? != b"WPK1" {
            return Err("this is not a pack: it does not begin WPK1".into());
        }
        let count = u32::from_le_bytes(take(4)?.try_into().unwrap());
        let mut files: BTreeMap<String, &[u8]> = BTreeMap::new();
        for _ in 0..count {
            let n = u16::from_le_bytes(take(2)?.try_into().unwrap()) as usize;
            let name = String::from_utf8(take(n)?.to_vec()).map_err(|_| "a file's name is not text".to_string())?;
            let len = u32::from_le_bytes(take(4)?.try_into().unwrap()) as usize;
            files.insert(name, take(len)?);
        }
        let json = files.get("pack.json").ok_or_else(|| "the pack has no pack.json".to_string())?;
        let manifest: Manifest = serde_json::from_slice(json).map_err(|e| format!("pack.json: {e}"))?;
        let mut pictures = BTreeMap::new();
        for (name, data) in &files {
            if name.ends_with(".pic") {
                let pic = Picture::from_bytes(data).ok_or_else(|| format!("{name} is not a picture"))?;
                pictures.insert(name.clone(), pic);
            }
        }
        let pack = Pack { manifest, pictures };
        pack.check()?;
        Ok(pack)
    }

    /// Everything the manifest names has to be there.
    fn check(&self) -> Result<(), String> {
        let m = &self.manifest;
        let need = |what: &str, name: &str| -> Result<(), String> {
            if self.pictures.contains_key(name) { Ok(()) } else { Err(format!("{what} names the picture {name}, which is not in the pack")) }
        };
        if m.faults.len() > 24 || m.faults.iter().any(|f| f.says.is_empty() || f.says.len() > 38 || (f.breaks.is_empty() && f.hidden.is_empty())) {
            return Err("a fault needs what the ad says of it, in 38 letters or fewer, and at least one slot it breaks; and there is room for 24".into());
        }
        if m.sellers.iter().any(|s| s.from.is_empty() || s.ship < 0) {
            return Err("somewhere a car is sold from needs a name, and what it costs to bring one from there".into());
        }
        if let Some(sh) = &m.shootout {
            if m.rivals.is_empty() || sh.rounds < 1 || sh.rounds > 6 || sh.every < 1 || sh.entry < 0 || sh.purse < 0 {
                return Err("the shootout needs rivals in the pack, one to six rounds, and to be held every day or less often".into());
            }
        }
        if let Some(name) = &m.title {
            need("the title screen", name)?;
        }
        if !m.places.contains_key(&m.start.place) {
            return Err(format!("the game starts in {}, which is not a place", m.start.place));
        }
        if let Some(car) = &m.start.car {
            if !m.cars.contains_key(car) {
                return Err(format!("the game starts with the car {car}, which is not in the pack"));
            }
        }
        let color = |what: &str, name: &Option<String>| -> Result<(), String> {
            match name {
                Some(n) if !RAMPS.iter().skip(1).any(|r| r.eq_ignore_ascii_case(n)) && !n.eq_ignore_ascii_case("GREY") => Err(format!("{what} is in the color {n}, which is not one of the engine's")),
                _ => Ok(()),
            }
        };
        if m.season.days < 1 || m.season.hours_per_day < 1 {
            return Err("the season has to be at least a day of at least an hour".into());
        }
        if m.jobs.len() > MOST_JOBS {
            return Err(format!("the pack has {} jobs; the paper has room for {MOST_JOBS}", m.jobs.len()));
        }
        if let Some(j) = m.jobs.iter().find(|j| j.hours < 1 || j.pay < 1) {
            return Err(format!("the job {} has to take at least an hour and pay at least a dollar", j.name));
        }
        if m.parts.len() > MOST_PARTS {
            return Err(format!("the pack has {} parts; there is room for {MOST_PARTS}", m.parts.len()));
        }
        for (id, part) in &m.parts {
            if let Some(name) = &part.picture {
                need(&format!("the part {id}"), name)?;
            }
            if id.contains('@') || part.holds.is_some_and(|h| h <= 0.0) {
                return Err(format!("the part {id} cannot have an @ in what it is called, or hold no torque at all"));
            }
            if part.curve.len() == 1 || part.curve.windows(2).any(|w| w[1][0] <= w[0][0]) || part.curve.iter().any(|p| p[1] <= 0.0) {
                return Err(format!("the part {id} has a torque curve that is not rising pairs of rpm and torque"));
            }
            if part.nitrous.as_ref().is_some_and(|n| n.gain < 1 || n.gain > 200 || n.seconds <= 0.0 || n.seconds > 60.0 || n.fill < 0) {
                return Err(format!("the part {id} is a nitrous kit that has to add 1 to 200 per cent, for up to a minute"));
            }
            if let Some(name) = &part.side {
                need(&format!("the part {id}"), name)?;
            }
            if let Some(car) = part.fits.iter().find(|c| !m.cars.contains_key(*c)) {
                return Err(format!("the part {id} fits the car {car}, which is not in the pack"));
            }
        }
        for r in &m.rivals {
            if let Some(name) = &r.figure {
                need(&format!("the rival {}", r.name), name)?;
            }
        }
        for (id, place) in &m.places {
            if place.people.len() > MOST_PEOPLE {
                return Err(format!("{id} has {} people standing in it; there is room for {MOST_PEOPLE}", place.people.len()));
            }
            for p in &place.people {
                need(&format!("someone standing in {id}"), &p.picture)?;
            }
        }
        for (id, road) in &m.roads {
            if road.parts.is_empty() || road.parts.iter().any(|p| p.len < 12.0 || !p.len.is_finite()) || road.len() < 200.0 || road.len() > 30000.0 {
                return Err(format!("the road {id} needs stretches of 12 meters or more, adding up to between 200 meters and 30 kilometers"));
            }
            if let Some(p) = road.parts.iter().find(|p| !SIDES.contains(&p.left.as_str()) || !SIDES.contains(&p.right.as_str())) {
                return Err(format!("the road {id} has \"{}\" and \"{}\" beside it; what can be beside a road is {}", p.left, p.right, SIDES.join(", ")));
            }
            if road.parts.iter().any(|p| p.curve.abs() > 1.0 || p.hill.abs() > 80.0) {
                return Err(format!("the road {id} bends more than 1 or climbs more than 80 meters in one stretch"));
            }
            for end in &road.ends {
                let there = match end.strip_prefix("map:") {
                    Some(map) => m.maps.get(map).is_some_and(|mp| mp.blocks.values().any(|b| b.road.as_deref() == Some(id.as_str()))),
                    None => m.places.contains_key(end),
                };
                if !there {
                    return Err(format!("the road {id} ends at {end}, which is not a place, or a map with a block marked for this road"));
                }
            }
            color(&format!("the sky over the road {id}"), &road.sky)?;
            color(&format!("the ground beside the road {id}"), &road.ground)?;
        }
        for (id, map) in &m.maps {
            if map.rows.is_empty() || map.cols() == 0 || map.rows.iter().any(|r| r.chars().count() as i32 != map.cols()) {
                return Err(format!("the map {id} needs rows of blocks, all the same length"));
            }
            if map.pitch < map.road * 2.0 + 16.0 || map.road < 4.0 {
                return Err(format!("the map {id} needs streets at least 8 meters wide and blocks at least 16 across"));
            }
            if let Some(ch) = map.rows.iter().flat_map(|r| r.chars()).find(|ch| *ch != '.' && !map.blocks.contains_key(&ch.to_string())) {
                return Err(format!("the map {id} has a block marked {ch}, which its blocks do not explain"));
            }
            for (ch, b) in &map.blocks {
                color(&format!("block {ch} of the map {id}"), &b.wall)?;
                if let Some(to) = b.to.as_ref().filter(|to| !m.places.contains_key(*to)) {
                    return Err(format!("block {ch} of the map {id} leads to {to}, which is not a place"));
                }
                if let Some(road) = b.road.as_ref().filter(|r| !m.roads.get(*r).is_some_and(|road| road.ends.contains(&format!("map:{id}")))) {
                    return Err(format!("block {ch} of the map {id} is the way onto the road {road}, which has no end at this map"));
                }
            }
        }
        for p in &m.paints {
            color("a car in the paper", &Some(p.clone()))?;
        }
        for (i, r) in m.rivals.iter().enumerate() {
            if !m.cars.contains_key(&r.car) {
                return Err(format!("{} drives the car {}, which is not in the pack", r.name, r.car));
            }
            color(&format!("{}'s car", r.name), &r.paint)?;
            if r.et <= 0.0 {
                return Err(format!("{} needs a time for the quarter mile", r.name));
            }
            if r.king && i + 1 != m.rivals.len() {
                return Err(format!("{} is the one to beat, so has to be last of the rivals", r.name));
            }
        }
        for (id, car) in &m.cars {
            color(&format!("the car {id}"), &car.paint)?;
            for r in [&car.paint_ramps.mid, &car.paint_ramps.dark, &car.paint_ramps.light, &car.paint_ramps.glint] {
                color(&format!("part of the car {id}'s picture"), r)?;
            }
            let d = &car.drive;
            if d.torque.is_empty() || d.gears.is_empty() || d.weight_lb <= 0.0 || d.final_drive <= 0.0 {
                return Err(format!("the car {id} needs a torque curve, gears, a final drive and a weight"));
            }
            need(&format!("the car {id}"), &car.side)?;
            let seat = format!("the cockpit of the car {id}");
            if let Some(c) = &car.cockpit {
                need(&seat, &c.picture)?;
                for name in [&c.night, &c.hood, &c.wheel.as_ref().map(|w| w.picture.clone()), &c.wheel.as_ref().and_then(|w| w.night.clone())].into_iter().flatten() {
                    need(&seat, name)?;
                }
                if let Some(w) = &c.wheel {
                    if self.pictures.get(&w.picture).is_some_and(|p| p.w != p.h) {
                        return Err(format!("{seat} needs a square picture for its wheel"));
                    }
                }
            }
            if let Some(r) = &car.rear {
                need(&format!("the back of the car {id}"), &r.picture)?;
            }
            for (slot, part) in &car.stock {
                match m.parts.get(part) {
                    None => return Err(format!("the car {id} leaves the factory with {part} for its {slot}, which is not a part in the pack")),
                    Some(p) if p.slot != *slot => return Err(format!("the car {id} has {part} down for its {slot}, but that part goes in {}", p.slot)),
                    _ => {}
                }
            }
            if let Some(slot) = car.optional.iter().find(|s| !car.stock.contains_key(*s)) {
                return Err(format!("the car {id} can do without its {slot}, which is not on its build sheet"));
            }
            if let Some(r) = &car.front {
                need(&format!("the front of the car {id}"), &r.picture)?;
            }
            let mut seen = std::collections::BTreeSet::new();
            for (k, (area, title, picture, slots)) in car.areas().into_iter().enumerate() {
                if area.is_empty() || car.areas().iter().take(k).any(|a| a.0 == area) {
                    return Err(format!("the car {id} has two places to work on called {area:?}"));
                }
                for (name, place) in slots {
                    if !seen.insert(name.clone()) {
                        return Err(format!("the car {id} has its {name} in two places to work on"));
                    }
                    if let Some(front) = place.behind.as_ref().filter(|f| !slots.contains_key(*f) || *f == name) {
                        return Err(format!("{} on the car {id}, the {name} is behind the {front}, which is not a place there", title.to_lowercase()));
                    }
                }
                need(&format!("{} on the car {id}", title.to_lowercase()), picture)?;
            }
            if let Some(l) = &car.side_lamps {
                need(&format!("the car {id}"), l)?;
            }
        }
        for (id, place) in &m.places {
            if let Some(p) = &place.picture {
                need(&format!("the place {id}"), p)?;
            }
            color(&format!("the place {id}"), &place.backdrop)?;
            for spot in &place.spots {
                if let Some(to) = spot.action.strip_prefix("go:") {
                    if !m.places.contains_key(to) {
                        return Err(format!("in {id}, \"{}\" goes to {to}, which is not a place", spot.label));
                    }
                } else if let Some(road) = spot.action.strip_prefix("pokerrun:") {
                    if !m.roads.contains_key(road) {
                        return Err(format!("in {id}, \"{}\" is a poker run down the road {road}, which is not in the pack", spot.label));
                    }
                } else if let Some(road) = spot.action.strip_prefix("roadrace:") {
                    if !m.roads.contains_key(road) {
                        return Err(format!("in {id}, \"{}\" races down the road {road}, which is not in the pack", spot.label));
                    }
                    if place.rival_at.is_none() || m.rivals.is_empty() {
                        return Err(format!("in {id}, \"{}\" needs rivals in the pack and a rival_at in the place", spot.label));
                    }
                } else if let Some(road) = spot.action.strip_prefix("road:") {
                    if !m.roads.contains_key(road) {
                        return Err(format!("in {id}, \"{}\" takes the road {road}, which is not in the pack", spot.label));
                    }
                } else if let Some(map) = spot.action.strip_prefix("drive:") {
                    if !m.maps.contains_key(map) {
                        return Err(format!("in {id}, \"{}\" drives round {map}, which is not a map", spot.label));
                    }
                } else if !spot.action.starts_with("say:") && !ACTIONS.contains(&spot.action.as_str()) && spot.action != "tyres" && !spot.action.starts_with("work:") {
                    return Err(format!("in {id}, \"{}\" does \"{}\", which the engine does not know how to do", spot.label, spot.action));
                } else if ["race", "next", "solo"].contains(&spot.action.as_str()) && (place.rival_at.is_none() || m.rivals.is_empty()) {
                    return Err(format!("in {id}, \"{}\" needs rivals in the pack and a rival_at in the place", spot.label));
                }
            }
        }
        Ok(())
    }
}
