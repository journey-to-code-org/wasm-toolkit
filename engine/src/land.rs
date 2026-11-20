//! The world in one piece: the home town where it is, and every road laid out from it as a curve
//! on the ground, with the places they lead to standing at their ends. Driving it is driving the
//! town, on and on: there is no edge to cross, only streets that become roads.
//!
//! Nothing here is new data. A road is laid out from the stretches it already has (each bend
//! turning it as the road view turns it, each hill lifting it as the road view lifts it), starting
//! at its town gate, or at the far end of a road already laid if it starts at a place. A road
//! from a place back to the same place is bent round to close as a loop.

use crate::pack::{Manifest, Map};
use crate::road::{Side, Trip, HALF, SEG};

/// How far a bend turns a road for each length of it: the road view's own.
pub const BEND: f64 = 0.032;
/// How far out from a place at a road's end its lot reaches.
pub const SITE: f64 = 30.0;
/// How far past the edge of the tar the shoulder runs before the country starts.
pub const SHOULDER: f64 = 1.0;

/// A point down a road, every `SEG` metres: where it is, how high, which way it runs (radians,
/// clockwise from north), and what is on each side.
#[derive(Clone, Copy, Debug)]
pub struct Pt {
    pub x: f64,
    pub z: f64,
    pub y: f64,
    pub h: f64,
    pub left: Side,
    pub right: Side,
}

/// A road on the ground. `ends[0]` is where its points start, and `ends[1]` where they finish:
/// a place, or `map:<id>`.
pub struct Lane {
    pub id: String,
    pub pts: Vec<Pt>,
    pub ends: [String; 2],
    /// Whether the points run the road the opposite way to the pack's own ends.
    pub reversed: bool,
}

/// A place at the end of a road, where you pull in.
#[derive(Clone, Debug)]
pub struct Site {
    pub place: String,
    pub x: f64,
    pub z: f64,
    pub y: f64,
    pub h: f64,
}

/// Where a point is, near a road: which road, how far along it (metres), how far across from its
/// middle (metres, to the right of the way its points run is more than nothing), and how high it is there.
#[derive(Clone, Copy, Debug)]
pub struct Near {
    pub lane: usize,
    pub along: f64,
    pub across: f64,
    pub y: f64,
    pub h: f64,
}

pub struct World {
    /// The map the roads start from.
    pub home: String,
    pub lanes: Vec<Lane>,
    pub sites: Vec<Site>,
}

impl Lane {
    pub fn len(&self) -> f64 {
        (self.pts.len().saturating_sub(1)) as f64 * SEG
    }

    /// The point `along` metres down it, and the way it runs there.
    pub fn at(&self, along: f64) -> Pt {
        let f = (along / SEG).clamp(0.0, (self.pts.len() - 1) as f64);
        let i = (f.floor() as usize).min(self.pts.len().saturating_sub(2));
        let t = f - i as f64;
        let (a, b) = (self.pts[i], self.pts[(i + 1).min(self.pts.len() - 1)]);
        Pt { x: a.x + (b.x - a.x) * t, z: a.z + (b.z - a.z) * t, y: a.y + (b.y - a.y) * t, h: a.h + (b.h - a.h) * t, left: a.left, right: a.right }
    }

    /// `across` metres to the right of the point `along` metres down it.
    pub fn beside(&self, along: f64, across: f64) -> (f64, f64, f64) {
        let p = self.at(along);
        (p.x + p.h.cos() * across, p.z - p.h.sin() * across, p.y)
    }
}

impl World {
    /// The world of a pack, if it has a town that is open, with roads leading out of it.
    pub fn build(m: &Manifest) -> Option<World> {
        let (home, map) = m.maps.iter().find(|(_, map)| map.open && map.blocks.values().any(|b| b.road.as_ref().is_some_and(|r| map.gate(r).is_some() && m.roads.contains_key(r))))?;
        let at_home = format!("map:{home}");
        let mut lanes: Vec<Lane> = Vec::new();
        let mut sites: Vec<Site> = Vec::new();
        let mut left: Vec<&String> = m.roads.keys().collect();
        // Lay what can be laid, from what is already down, until nothing more can be.
        loop {
            let before = left.len();
            left.retain(|id| {
                let road = &m.roads[*id];
                let anchor = |end: &str| -> Option<(f64, f64, f64, f64)> {
                    if end == at_home {
                        let (gx, gz, h) = map.gate(id)?;
                        // Out of the gate, the way away from town.
                        Some((gx, gz, map.height(gx, gz), (h + 180.0).to_radians()))
                    } else {
                        sites.iter().find(|s| s.place == end).map(|s| (s.x, s.z, s.y, s.h))
                    }
                };
                let (start, reversed) = match (anchor(&road.ends[0]), anchor(&road.ends[1])) {
                    (Some(a), _) => (a, false),
                    (None, Some(b)) => (b, true),
                    _ => return true,
                };
                let ends = if reversed { [road.ends[1].clone(), road.ends[0].clone()] } else { road.ends.clone() };
                let segs = Trip::new(id, road, reversed, 1).segs;
                let (mut x, mut z, y0, mut h) = start;
                let mut pts = Vec::with_capacity(segs.len() + 1);
                for s in &segs {
                    pts.push(Pt { x, z, y: y0 + s.y, h, left: s.left, right: s.right });
                    x += h.sin() * SEG;
                    z += h.cos() * SEG;
                    h += s.curve * BEND;
                }
                let last = segs.last().map_or((Side::Trees, Side::Trees, 0.0), |s| (s.left, s.right, s.y));
                pts.push(Pt { x, z, y: y0 + last.2, h, left: last.0, right: last.1 });
                if ends[0] == ends[1] {
                    // A loop: bent round, a little at a time, to come back where it began.
                    close(&mut pts);
                }
                if pts.len() < 2 {
                    return true;
                }
                if !ends[1].starts_with("map:") && !sites.iter().any(|s| s.place == ends[1]) {
                    let p = pts[pts.len() - 1];
                    sites.push(Site { place: ends[1].clone(), x: p.x, z: p.z, y: p.y, h: p.h });
                }
                lanes.push(Lane { id: (*id).clone(), pts, ends, reversed });
                false
            });
            if left.len() == before {
                break;
            }
        }
        (!lanes.is_empty()).then(|| World { home: home.clone(), lanes, sites })
    }

    /// The road nearest a point, if one is within `reach` metres of it.
    pub fn near(&self, x: f64, z: f64, reach: f64) -> Option<Near> {
        let mut best: Option<(f64, Near)> = None;
        for (k, lane) in self.lanes.iter().enumerate() {
            for (i, w) in lane.pts.windows(2).enumerate() {
                let (a, b) = (w[0], w[1]);
                if (a.x - x).abs() > reach + SEG || (a.z - z).abs() > reach + SEG {
                    continue;
                }
                let (dx, dz) = (b.x - a.x, b.z - a.z);
                let len2 = (dx * dx + dz * dz).max(1e-9);
                let t = (((x - a.x) * dx + (z - a.z) * dz) / len2).clamp(0.0, 1.0);
                let (px, pz) = (a.x + dx * t, a.z + dz * t);
                let d2 = (x - px).powi(2) + (z - pz).powi(2);
                if d2 > reach * reach || best.as_ref().is_some_and(|b| b.0 <= d2) {
                    continue;
                }
                // To the right of the way the road runs is more than nothing.
                let side = (x - px) * a.h.cos() - (z - pz) * a.h.sin();
                let across = d2.sqrt() * if side < 0.0 { -1.0 } else { 1.0 };
                best = Some((d2, Near { lane: k, along: (i as f64 + t) * SEG, across, y: a.y + (b.y - a.y) * t, h: a.h }));
            }
        }
        best.map(|b| b.1)
    }

    /// How high the ground is: the town's own hills in town, the road's height near a road, and
    /// level away from both.
    pub fn ground(&self, map: &Map, x: f64, z: f64) -> f64 {
        if in_town(map, x, z, 0.0) {
            return map.height(x, z);
        }
        match self.near(x, z, 160.0) {
            Some(n) => n.y * (1.0 - ((n.across.abs() - 20.0) / 140.0).clamp(0.0, 1.0)),
            None => 0.0,
        }
    }

    /// Whether a car can be at a point outside town: on a road or its shoulder, at a place at a
    /// road's end, or out in open fields beside one; not in the trees, the rocks, the water or
    /// over the drop.
    pub fn open(&self, map: &Map, x: f64, z: f64) -> bool {
        if self.sites.iter().any(|s| (s.x - x).powi(2) + (s.z - z).powi(2) < SITE.powi(2)) {
            return true;
        }
        let Some(n) = self.near(x, z, 80.0) else { return false };
        if n.across.abs() <= HALF + SHOULDER {
            return true;
        }
        let p = self.lanes[n.lane].at(n.along);
        let side = if n.across > 0.0 { p.right } else { p.left };
        // Near the town's edge the streets still run.
        !side.solid() || in_town(map, x, z, 12.0)
    }

    /// Whether a point is on the tar of a road (rather than its shoulder, or the country).
    pub fn on_tar(&self, x: f64, z: f64) -> Option<Near> {
        self.near(x, z, HALF + 0.5).filter(|n| n.across.abs() <= HALF)
    }
}

/// Whether a point is inside a map's streets, give or take `margin` metres.
pub fn in_town(map: &Map, x: f64, z: f64, margin: f64) -> bool {
    let (w, d, e) = (map.cols() as f64 * map.pitch, map.depth() as f64 * map.pitch, map.road + margin);
    x >= -e && z >= -e && x <= w + e && z <= d + e
}

/// Bends a road that should end where it began round until it does: what it is short by is
/// taken up a little at every point along it, and the way it runs is worked out again.
fn close(pts: &mut [Pt]) {
    let n = pts.len();
    if n < 3 {
        return;
    }
    let (ex, ez) = (pts[0].x - pts[n - 1].x, pts[0].z - pts[n - 1].z);
    for (i, p) in pts.iter_mut().enumerate() {
        let t = i as f64 / (n - 1) as f64;
        p.x += ex * t;
        p.z += ez * t;
    }
    let (y0, y1) = (pts[0].y, pts[n - 1].y);
    for (i, p) in pts.iter_mut().enumerate() {
        p.y += (y0 - y1) * i as f64 / (n - 1) as f64;
    }
    for i in 0..n - 1 {
        pts[i].h = (pts[i + 1].x - pts[i].x).atan2(pts[i + 1].z - pts[i].z);
    }
    pts[n - 1].h = pts[0].h;
}

/// A car out on a road: which road, how far along it, how fast (less than nothing for one going
/// back the way the road's points run), its paint, and whether it is a truck pulling a trailer.
#[derive(Clone, Debug)]
pub struct Rolling {
    pub lane: usize,
    pub along: f64,
    pub speed: f64,
    pub paint: &'static str,
    pub rig: bool,
}

const PAINTS: [&str; 8] = ["BLUE", "SILVER", "WHITE", "GREEN", "GOLD", "TEAL", "MAROON", "ORANGE"];

impl World {
    /// The traffic out on every road: as many as the road has, spread down it, some each way.
    pub fn traffic(&self, m: &Manifest, mut rand: impl FnMut() -> u32) -> Vec<Rolling> {
        let mut out = Vec::new();
        for (k, lane) in self.lanes.iter().enumerate() {
            let Some(road) = m.roads.get(&lane.id) else { continue };
            let flow = ((road.limit - 55.0) / 2.237).max(0.0);
            for i in 0..road.traffic {
                let r = rand();
                let rig = ((r >> 12) % 100) as f64 / 100.0 < road.rigs.clamp(0.0, 1.0);
                let slow = if rig { 3.0 } else { 0.0 };
                let speed = 13.0 + flow - slow + (r % 8) as f64;
                let along = 60.0 + ((r >> 4) % 1000) as f64 / 1000.0 * (lane.len() - 120.0).max(1.0);
                out.push(Rolling { lane: k, along, speed: if (r >> 3) & 1 == 0 { -speed } else { speed }, paint: PAINTS[(r as usize >> 8).wrapping_add(i) % 8], rig });
            }
        }
        out
    }

    /// Where a car out on a road is, and which way it faces (radians): on the right of the road
    /// for the way it is going.
    pub fn place_of(&self, c: &Rolling) -> (f64, f64, f64, f64) {
        let lane = &self.lanes[c.lane];
        let p = lane.at(c.along);
        let (x, z, y) = lane.beside(c.along, if c.speed < 0.0 { -HALF / 2.0 } else { HALF / 2.0 });
        (x, z, y, if c.speed < 0.0 { p.h + std::f64::consts::PI } else { p.h })
    }

    /// A thirtieth of a second for every car out on the roads. At the end of its road it turns
    /// round and comes back on the other side.
    pub fn roll(&self, cars: &mut [Rolling]) {
        for c in cars {
            let len = self.lanes[c.lane].len();
            c.along += c.speed / 30.0;
            if c.along > len - 30.0 && c.speed > 0.0 || c.along < 30.0 && c.speed < 0.0 {
                c.speed = -c.speed;
            }
            c.along = c.along.clamp(0.0, len);
        }
    }
}
