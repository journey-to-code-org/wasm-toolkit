//! Driving round a map: the car, the other cars, and what there is to run into.
//!
//! Streets run north and south and east and west, one every `pitch` metres. What is between four
//! streets is a block, and what stands on it is whatever the pack's map says. x is east, z is
//! north; a heading of 0 faces north and turns clockwise.

use crate::pack::{Block, Map};

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
}

#[derive(Clone, Copy, Default)]
pub struct Pedals {
    pub go: bool,
    pub stop: bool,
    pub left: bool,
    pub right: bool,
    /// A gear up and a gear down, for a car with a gearbox to work.
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
        Town { map: id.to_string(), x: (c as f64 + 0.5) * map.pitch, z: r as f64 * map.pitch - 4.0, heading: 90.0, speed: 0.0, steer: 0.0, crash: 0, dents: 0, others }
    }

    /// The block this point is over: its column and row.
    fn cell(map: &Map, x: f64, z: f64) -> (i32, i32) {
        ((x / map.pitch).floor() as i32, (z / map.pitch).floor() as i32)
    }

    /// A street is clear. A block with something built on it is not, three metres in from the
    /// kerb, and nor is the country beyond the last street.
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
        map.block(c, r).is_some_and(|b| b.height > 0.0)
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

    /// A thirtieth of a second. Returns how far the car went, in metres, and whether it just hit something.
    pub fn tick(&mut self, map: &Map, p: Pedals) -> (f64, bool) {
        if self.crash > 0 {
            self.crash -= 1;
            return (0.0, false);
        }
        // The pedals. It pulls hardest from rest and runs out of breath near 55 metres a second.
        if p.go {
            self.speed += 5.0 / HZ * (1.0 - self.speed / 56.0);
        } else {
            self.speed -= 1.0 / HZ;
        }
        if p.stop {
            self.speed -= 11.0 / HZ;
        }
        self.speed = self.speed.max(0.0);
        // The wheel eases over and eases back; the faster the car, the less it turns.
        let want = if p.left { -1.0 } else if p.right { 1.0 } else { 0.0 };
        self.steer += (want - self.steer) / 4.0;
        let grip = (self.speed / 12.0).min(1.0);
        self.heading = (self.heading + self.steer * grip * (9.0 - self.speed / 9.0) * 360.0 / 1024.0).rem_euclid(360.0);
        let (s, c) = self.heading.to_radians().sin_cos();
        let (nx, nz) = (self.x + s * self.speed / HZ, self.z + c * self.speed / HZ);
        let mut hit = Self::blocked(map, nx + s * 2.0, nz + c * 2.0);
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
