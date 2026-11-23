//! Driving round a map: the car, the other cars, and what there is to run into.
//!
//! Streets run north and south and east and west, one every `pitch` meters. What is between four
//! streets is a block, and what stands on it is whatever the pack's map says. x is east, z is
//! north; a heading of 0 faces north and turns clockwise.

use crate::land::{in_town, Rolling, World};
use crate::pack::{Block, Map};
use crate::race::{Launch, BAND};
use crate::road::Transmission;

const HZ: f64 = 30.0;
const PAINTS: [&str; 8] = ["BLUE", "SILVER", "WHITE", "GREEN", "GOLD", "TEAL", "MAROON", "ORANGE"];

/// Another car: which street it is on, whether that street runs north, how far along it is.
pub struct Other {
    pub north: bool,
    pub street: i32,
    pub along: f64,
    pub speed: f64,
    pub paint: &'static str,
}

impl Other {
    /// Where it is: on the right of its street, going its way.
    pub fn at(&self, map: &Map) -> (f64, f64) {
        let lane = self.street as f64 * map.pitch + if self.north { 4.0 } else { -4.0 };
        if self.north { (lane, self.along) } else { (self.along, lane) }
    }
}

pub struct Town {
    pub map: String,
    pub x: f64,
    pub z: f64,
    /// Degrees clockwise from north.
    pub heading: f64,
    pub speed: f64,
    pub steer: f64,
    pub crash: i32,
    pub dents: i32,
    pub others: Vec<Other>,
    /// Where the police sit, and a chase: for how many ticks, and the most over the limit you were seen at.
    pub cops: Vec<(f64, f64)>,
    pub chase: Option<(u32, f64)>,
    /// How hard it stops and how sharply it turns, as shares of an ordinary car's.
    pub brakes: f64,
    pub turn: f64,
    /// How hard it pulls and how fast it will go, meters a second: the car's own, out of town.
    pub accel: f64,
    pub top: f64,
    /// The traffic out on the roads, when there is a world past the edge of town.
    pub rolling: Vec<Rolling>,
    /// How many of `cops` are the town's own; the rest sit out on the roads. And how long you
    /// have run from one out there, well over the limit.
    pub town_cops: usize,
    pub fled: u32,
    /// Nitrous going in: how much more the car makes just now, as a fraction.
    pub spray: f64,
    /// A race from a line: the transmission, if the car has one to work (it is worked only in a race),
    /// ticks until the off, and the start as the road has it: the best revs to launch from,
    /// whether the clutch is still in, how it went, and how long the tires spin or the engine bogs.
    pub transmission: Option<Transmission>,
    pub hold: i32,
    pub best_launch: f64,
    pub held: bool,
    pub start: Launch,
    pub smoke: f64,
    pub bog: f64,
    pub abuse: f64,
    /// Torque steer: how far the power drags it sideways, in half road-widths a second.
    pub pull: f64,
}

#[derive(Clone, Copy, Default)]
pub struct Pedals {
    pub go: bool,
    pub stop: bool,
    pub left: bool,
    pub right: bool,
    /// A gear up and a gear down, for a car with a transmission to work.
    pub up: bool,
    pub down: bool,
}

impl Town {
    /// Out on the street beside the block that leads to `from`, or in the south-west corner.
    pub fn new(id: &str, map: &Map, from: Option<&str>, rand: impl FnMut() -> u32) -> Town {
        Town::at(id, map, |b| b.to.as_deref() == from && from.is_some(), rand)
    }

    /// Out on the street beside the first block that is the one wanted.
    pub fn at(id: &str, map: &Map, wanted: impl Fn(&Block) -> bool, mut rand: impl FnMut() -> u32) -> Town {
        let (c, r) = map.places().into_iter().find(|(_, _, b)| wanted(b)).map(|(c, r, _)| (c, r)).unwrap_or((0, 0));
        let others = (0..map.traffic).map(|i| {
            let (k, north) = (rand(), i & 1 == 1);
            let (streets, length) = if north { (map.cols() + 1, map.depth()) } else { (map.depth() + 1, map.cols()) };
            Other { north, street: (k % streets as u32) as i32, along: ((k >> 8) % 1000) as f64 / 1000.0 * length as f64 * map.pitch, speed: 8.0 + ((k >> 20) % 8) as f64, paint: PAINTS[(k >> 4) as usize % 8] }
        }).collect();
        // On the street along the block's south side, in the right-hand lane, facing east.
        // The police park on the sidewalk at a corner, never the one you set out from.
        let cops = (0..map.cops.min(12)).map(|_| {
            let k = rand();
            let (cc, cr) = ((k % (map.cols() as u32 + 1)) as i32, ((k >> 8) % (map.depth() as u32 + 1)) as i32);
            (cc as f64 * map.pitch + map.road + 2.0, if (cc - c).abs() + (cr - r).abs() < 2 { (cr + 2) as f64 } else { cr as f64 } * map.pitch + map.road + 2.0)
        }).collect();
        Town { cops, chase: None, map: id.to_string(), x: (c as f64 + 0.5) * map.pitch, z: r as f64 * map.pitch - 4.0, heading: 90.0, speed: 0.0, steer: 0.0, crash: 0, dents: 0, others, brakes: 1.0, turn: 1.0, accel: 5.0, top: 56.0, rolling: Vec::new(), town_cops: map.cops.min(12), fled: 0, spray: 0.0, transmission: None, hold: 0, best_launch: 0.0, held: false, start: Launch::None, smoke: 0.0, bog: 0.0, abuse: 0.0, pull: 0.0 }
    }

    /// The block this point is over: its column and row.
    fn cell(map: &Map, x: f64, z: f64) -> (i32, i32) {
        ((x / map.pitch).floor() as i32, (z / map.pitch).floor() as i32)
    }

    /// A street is clear. A block with something built on it is not, three meters in from the
    /// curb, and nor is the country beyond the last street.
    pub fn blocked(map: &Map, x: f64, z: f64) -> bool {
        let (w, d) = (map.cols() as f64 * map.pitch, map.depth() as f64 * map.pitch);
        if x < -map.road || z < -map.road || x > w + map.road || z > d + map.road {
            return true;
        }
        let (c, r) = Self::cell(map, x, z);
        let (lx, lz, edge) = (x - c as f64 * map.pitch, z - r as f64 * map.pitch, map.road + 3.0);
        if lx < edge || lx > map.pitch - edge || lz < edge || lz > map.pitch - edge {
            return false;
        }
        map.block(c, r).is_some_and(|b| b.height > 0.0 || b.water)
    }

    /// Somewhere to go that you are stopped beside: within a street's width of its block.
    pub fn beside<'a>(&self, map: &'a Map) -> Option<&'a Block> {
        if self.speed > 2.0 {
            return None;
        }
        let reach = map.road * 2.0;
        map.places().into_iter().find(|&(c, r, _)| {
            let (x0, z0) = (c as f64 * map.pitch + map.road, r as f64 * map.pitch + map.road);
            let (x1, z1) = (x0 + map.pitch - 2.0 * map.road, z0 + map.pitch - 2.0 * map.road);
            self.x > x0 - reach && self.x < x1 + reach && self.z > z0 - reach && self.z < z1 + reach
        }).map(|(_, _, b)| b)
    }

    /// A thirtieth of a second. Returns how far the car went, in meters, and whether it just hit something.
    pub fn tick(&mut self, map: &Map, p: Pedals) -> (f64, bool) {
        self.tick_on(map, None, p)
    }

    /// Whether the car can be at a point: the streets in town, and past them, the world's roads
    /// and what is beside them. With no world, past the last street is the end.
    pub fn clear(map: &Map, land: Option<&World>, x: f64, z: f64) -> bool {
        match land {
            Some(w) if !in_town(map, x, z, 0.0) => w.open(map, x, z),
            _ => !Self::blocked(map, x, z),
        }
    }

    /// On the line in a car with a clutch: the gas revs it freely, up to the limiter.
    fn rev_free(&mut self, gas: bool) {
        if let Some(b) = self.transmission.as_mut() {
            b.rpm = if gas { (b.rpm + 5200.0 / HZ).min(b.redline - 250.0) } else { (b.rpm - 3600.0 / HZ).max(900.0) };
        }
    }

    /// Away from the line at whatever the revs are: a good start in the band, a bog well under
    /// it, and wheelspin over it.
    fn away(&mut self) {
        let Some(b) = self.transmission.as_mut().filter(|_| self.best_launch > 0.0) else { return };
        let over = b.rpm - (self.best_launch + BAND);
        self.start = if b.rpm < self.best_launch - BAND * 2.0 { Launch::Bogged } else if over > 0.0 { Launch::Spun } else { Launch::Good };
        match self.start {
            Launch::Bogged => self.bog = 1.2,
            Launch::Spun => (self.smoke, self.abuse) = ((over / 1400.0).min(2.0), over),
            _ => {}
        }
        b.gear = 0;
    }

    /// A thirtieth of a second, in a world that may run on past the edge of town.
    pub fn tick_on(&mut self, map: &Map, land: Option<&World>, p: Pedals) -> (f64, bool) {
        let clutch = self.best_launch > 0.0 && self.transmission.is_some();
        if self.hold > 0 {
            self.hold -= 1;
            if clutch {
                self.rev_free(p.go);
                // GO with the clutch in: it waits on the line until it is let out.
                if self.hold == 0 {
                    if p.stop { self.held = true } else { self.away() }
                }
            }
            return (0.0, false);
        }
        if self.held {
            if p.stop {
                self.rev_free(p.go);
                return (0.0, false);
            }
            self.held = false;
            self.away();
        }
        if self.crash > 0 {
            self.crash -= 1;
            return (0.0, false);
        }
        // The pedals. It pulls hardest from rest and runs out of breath near 55 meters a second.
        let (accel, top) = (self.accel.max(1.0), self.top.max(20.0));
        // A transmission, in a race: how hard it pulls is the gear it is in and where the revs are.
        let mut pull = 1.0;
        if let Some(b) = self.transmission.as_mut() {
            if p.up && b.gear + 1 < b.ratios.len() {
                b.gear += 1;
            }
            if p.down && b.gear > 0 && b.revs(b.gear - 1, self.speed) < b.redline + 300.0 {
                b.gear -= 1;
            }
            b.rpm = b.revs(b.gear, self.speed);
            let last = b.ratios[b.ratios.len() - 1];
            pull = if b.rpm >= b.redline { 0.0 } else { 0.75 * (b.ratios[b.gear] / last).sqrt() * (b.rpm / b.peak).clamp(0.4, 1.0) };
        }
        let bite = if self.smoke > 0.0 {
            self.smoke -= 1.0 / HZ;
            0.6
        } else if self.bog > 0.0 {
            self.bog -= 1.0 / HZ;
            0.45
        } else {
            1.0
        };
        let mut drag = 0.0;
        if p.go {
            let pull = pull * bite;
            // On the power the driven wheels drag it to one side: hardest low in the gears, going.
            let low = match self.transmission.as_ref() { Some(b) => 1.0 / ((b.gear + 1) as f64).powf(1.5), None => (1.0 - self.speed / top).max(0.0).powi(2) };
            drag = self.pull * (pull.min(1.0) * low * (self.speed / 12.0).min(1.0) + if bite < 1.0 && self.smoke > 0.0 { 1.5 * (self.speed / 8.0).min(1.0) } else { 0.0 });
            self.speed += pull * accel * (1.0 + self.spray) / HZ * (1.0 - self.speed / (top * (1.0 + self.spray * 0.3)));
        } else {
            self.speed -= 1.0 / HZ;
        }
        if p.stop {
            self.speed -= 11.0 * self.brakes / HZ;
        }
        self.speed = self.speed.max(0.0);
        // The wheel eases over and eases back; the faster the car, the less it turns.
        let want = if p.left { -1.0 } else if p.right { 1.0 } else { 0.0 };
        self.steer += (want - self.steer) / 4.0;
        let grip = (self.speed / 12.0).min(1.0);
        self.heading = (self.heading + self.steer * grip * (9.0 - self.speed / 9.0).max(0.8) * self.turn * 360.0 / 1024.0).rem_euclid(360.0);
        let out = land.is_some_and(|_| !in_town(map, self.x, self.z, 0.0));
        if out && land.is_some_and(|w| w.on_tar(self.x, self.z).is_none()) && self.speed > 9.0 {
            // Off the tar it is gravel and grass, and the car will not go quickly on them.
            self.speed -= (self.speed - 9.0) * 1.5 / HZ;
        }
        let (s, c) = self.heading.to_radians().sin_cos();
        // What the power drags it sideways, it goes sideways: half a road's width is five meters.
        let side = drag * 5.0 / HZ;
        let (nx, nz) = (self.x + s * self.speed / HZ + c * side, self.z + c * self.speed / HZ - s * side);
        let mut hit = !Self::clear(map, land, nx + s * 2.0, nz + c * 2.0);
        // The edge of the country: trees, rock, water, the drop. Into them slowly is only stopping,
        // and quickly is a crash, as anywhere.
        if let Some(w) = land {
            w.roll(&mut self.rolling);
            for o in &self.rolling {
                let (ox, oz, _, _) = w.place_of(o);
                if (nx - ox).powi(2) + (nz - oz).powi(2) < if o.rig { 3.2f64 } else { 2.4 }.powi(2) {
                    hit = true;
                }
            }
        }
        for o in &mut self.others {
            let length = if o.north { map.depth() } else { map.cols() } as f64 * map.pitch;
            o.along = (o.along + o.speed / HZ * if o.north { 1.0 } else { -1.0 }).rem_euclid(length);
            let (ox, oz) = o.at(map);
            let (dx, dz) = ((nx - ox).abs(), (nz - oz).abs());
            if if o.north { dx < 2.0 && dz < 3.5 } else { dx < 3.5 && dz < 2.0 } {
                hit = true;
            }
        }
        if hit {
            let hard = self.speed > 3.0;
            if hard {
                self.dents += 1;
                self.crash = HZ as i32;
            }
            self.speed = 0.0;
            return (0.0, hard);
        }
        (self.x, self.z) = (nx, nz);
        (self.speed / HZ, false)
    }
}
