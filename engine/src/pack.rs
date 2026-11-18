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

/// Which of the picture's ramps are the paint, so a car can be drawn in any colour.
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
}

impl Default for Season {
    fn default() -> Season {
        Season { days: 90, hours_per_day: 12 }
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

/// What makes a car go. Left out, a car gets a two-litre turbo four and five gears.
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Drive {
    /// Torque at the wheels, in pound-feet, at each engine speed: pairs of rpm and torque, rising.
    pub torque: Vec<[f64; 2]>,
    /// The gearbox, first gear first, and the final drive behind it.
    pub gears: Vec<f64>,
    #[serde(rename = "final")]
    pub final_drive: f64,
    pub redline: f64,
    /// The engine speed it holds on the line with your foot down.
    pub launch: f64,
    pub weight_lb: f64,
    /// The hardest the tyres can push it, in metres a second, each second.
    pub grip: f64,
    pub boost: Option<Boost>,
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
}

/// Under the hood: a picture of the engine bay, and where on it each slot's part sits (x, y,
/// width, height). A car with one has its parts taken off and put on there, bolt by bolt.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Hood {
    pub picture: String,
    pub slots: BTreeMap<String, [i32; 4]>,
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
    /// The bonnet as seen through the glass, drawn under the painting in the car's paint.
    #[serde(default)]
    pub bonnet: Option<String>,
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

/// The car from behind, for when it is ahead of you on the road, and how wide it is in metres.
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
    /// How far down the side picture the tyres touch the ground.
    pub ground: i32,
    /// The colour it comes in, as a ramp name.
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
    /// How many gallons the tank holds.
    #[serde(default = "tank")]
    pub tank: f64,
    #[serde(default)]
    pub cockpit: Option<Cockpit>,
    #[serde(default)]
    pub rear: Option<Rear>,
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
}

/// What stands on a block of a map.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Block {
    /// What it is built of, as a colour, and how tall it is in metres. With neither it is grass.
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
}

/// A town to drive round: blocks in rows, with a street between every two and round the outside.
/// Each letter of a row is a block, looked up in `blocks`; a full stop is grass. The top row is
/// the north side.
#[derive(Deserialize, Clone, Debug)]
pub struct Map {
    pub title: String,
    pub rows: Vec<String>,
    #[serde(default)]
    pub blocks: BTreeMap<String, Block>,
    /// Metres from one street to the next, and half the width of a street.
    #[serde(default = "pitch")]
    pub pitch: f64,
    #[serde(default = "road")]
    pub road: f64,
    /// How many other cars are about.
    #[serde(default = "traffic")]
    pub traffic: usize,
    #[serde(default = "gas_per_km")]
    pub gas_per_km: f64,
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
    pub fn places(&self) -> Vec<(i32, i32, &Block)> {
        let mut out = Vec::new();
        for r in 0..self.depth() {
            for c in 0..self.cols() {
                if let Some(b) = self.block(c, r).filter(|b| b.to.is_some()) {
                    out.push((c, r, b));
                }
            }
        }
        out
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
    /// The colours cars in the paper come in. Left out, eight ordinary ones.
    #[serde(default)]
    pub paints: Vec<String>,
    /// What can be put on a race, in dollars. Left out: 10, 25, 50, 100.
    #[serde(default)]
    pub bets: Vec<i32>,
    #[serde(default)]
    pub maps: BTreeMap<String, Map>,
}

/// What a spot can do. Anything else in a pack is a mistake, and is said to be.
pub const ACTIONS: [&str; 11] = ["", "lamps", "paper", "parts", "hood", "yard", "calendar", "race", "next", "solo", "gas"];
/// The most parts the paper has room to list.
pub const MOST_PARTS: usize = 14;

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
        if !m.places.contains_key(&m.start.place) {
            return Err(format!("the game starts in {}, which is not a place", m.start.place));
        }
        if let Some(car) = &m.start.car {
            if !m.cars.contains_key(car) {
                return Err(format!("the game starts with the car {car}, which is not in the pack"));
            }
        }
        let colour = |what: &str, name: &Option<String>| -> Result<(), String> {
            match name {
                Some(n) if !RAMPS.iter().skip(1).any(|r| r.eq_ignore_ascii_case(n)) => Err(format!("{what} is in the colour {n}, which is not one of the engine's")),
                _ => Ok(()),
            }
        };
        if m.season.days < 1 || m.season.hours_per_day < 1 {
            return Err("the season has to be at least a day of at least an hour".into());
        }
        if m.parts.len() > MOST_PARTS {
            return Err(format!("the pack has {} parts; the paper has room for {MOST_PARTS}", m.parts.len()));
        }
        for (id, part) in &m.parts {
            if let Some(car) = part.fits.iter().find(|c| !m.cars.contains_key(*c)) {
                return Err(format!("the part {id} fits the car {car}, which is not in the pack"));
            }
        }
        for (id, map) in &m.maps {
            if map.rows.is_empty() || map.cols() == 0 || map.rows.iter().any(|r| r.chars().count() as i32 != map.cols()) {
                return Err(format!("the map {id} needs rows of blocks, all the same length"));
            }
            if map.pitch < map.road * 2.0 + 16.0 || map.road < 4.0 {
                return Err(format!("the map {id} needs streets at least 8 metres wide and blocks at least 16 across"));
            }
            if let Some(ch) = map.rows.iter().flat_map(|r| r.chars()).find(|ch| *ch != '.' && !map.blocks.contains_key(&ch.to_string())) {
                return Err(format!("the map {id} has a block marked {ch}, which its blocks do not explain"));
            }
            for (ch, b) in &map.blocks {
                colour(&format!("block {ch} of the map {id}"), &b.wall)?;
                if let Some(to) = b.to.as_ref().filter(|to| !m.places.contains_key(*to)) {
                    return Err(format!("block {ch} of the map {id} leads to {to}, which is not a place"));
                }
            }
        }
        for p in &m.paints {
            colour("a car in the paper", &Some(p.clone()))?;
        }
        for (i, r) in m.rivals.iter().enumerate() {
            if !m.cars.contains_key(&r.car) {
                return Err(format!("{} drives the car {}, which is not in the pack", r.name, r.car));
            }
            colour(&format!("{}'s car", r.name), &r.paint)?;
            if r.et <= 0.0 {
                return Err(format!("{} needs a time for the quarter mile", r.name));
            }
            if r.king && i + 1 != m.rivals.len() {
                return Err(format!("{} is the one to beat, so has to be last of the rivals", r.name));
            }
        }
        for (id, car) in &m.cars {
            colour(&format!("the car {id}"), &car.paint)?;
            for r in [&car.paint_ramps.mid, &car.paint_ramps.dark, &car.paint_ramps.light, &car.paint_ramps.glint] {
                colour(&format!("part of the car {id}'s picture"), r)?;
            }
            let d = &car.drive;
            if d.torque.is_empty() || d.gears.is_empty() || d.weight_lb <= 0.0 || d.final_drive <= 0.0 {
                return Err(format!("the car {id} needs a torque curve, gears, a final drive and a weight"));
            }
            need(&format!("the car {id}"), &car.side)?;
            let seat = format!("the cockpit of the car {id}");
            if let Some(c) = &car.cockpit {
                need(&seat, &c.picture)?;
                for name in [&c.night, &c.bonnet, &c.wheel.as_ref().map(|w| w.picture.clone()), &c.wheel.as_ref().and_then(|w| w.night.clone())].into_iter().flatten() {
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
            if let Some(hood) = &car.hood {
                need(&format!("under the hood of the car {id}"), &hood.picture)?;
            }
            if let Some(l) = &car.side_lamps {
                need(&format!("the car {id}"), l)?;
            }
        }
        for (id, place) in &m.places {
            if let Some(p) = &place.picture {
                need(&format!("the place {id}"), p)?;
            }
            colour(&format!("the place {id}"), &place.backdrop)?;
            for spot in &place.spots {
                if let Some(to) = spot.action.strip_prefix("go:") {
                    if !m.places.contains_key(to) {
                        return Err(format!("in {id}, \"{}\" goes to {to}, which is not a place", spot.label));
                    }
                } else if let Some(map) = spot.action.strip_prefix("drive:") {
                    if !m.maps.contains_key(map) {
                        return Err(format!("in {id}, \"{}\" drives round {map}, which is not a map", spot.label));
                    }
                } else if !spot.action.starts_with("say:") && !ACTIONS.contains(&spot.action.as_str()) {
                    return Err(format!("in {id}, \"{}\" does \"{}\", which the engine does not know how to do", spot.label, spot.action));
                } else if ["race", "next", "solo"].contains(&spot.action.as_str()) && (place.rival_at.is_none() || m.rivals.is_empty()) {
                    return Err(format!("in {id}, \"{}\" needs rivals in the pack and a rival_at in the place", spot.label));
                }
            }
        }
        Ok(())
    }
}
