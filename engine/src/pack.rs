//! A content pack: everything the engine plays, in one file.
//!
//! The engine knows rules; the pack says what there is. A pack is a list of named files: `pack.json`,
//! which describes the cars and the places, and the pictures it names. `tools/pack.py` makes one
//! from a folder. The format is in `docs/ENGINE.md`.

use crate::gfx::Picture;
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
    #[serde(default)]
    pub spots: Vec<Spot>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Manifest {
    pub name: String,
    pub start: Start,
    #[serde(default)]
    pub cars: BTreeMap<String, Car>,
    pub places: BTreeMap<String, Place>,
}

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
        for (id, car) in &m.cars {
            need(&format!("the car {id}"), &car.side)?;
            if let Some(l) = &car.side_lamps {
                need(&format!("the car {id}"), l)?;
            }
        }
        for (id, place) in &m.places {
            if let Some(p) = &place.picture {
                need(&format!("the place {id}"), p)?;
            }
            for spot in &place.spots {
                if let Some(to) = spot.action.strip_prefix("go:") {
                    if !m.places.contains_key(to) {
                        return Err(format!("in {id}, \"{}\" goes to {to}, which is not a place", spot.label));
                    }
                }
            }
        }
        Ok(())
    }
}
