//! The game: a pointer, the place you are in, and what picking things does.
//!
//! Nothing here knows what a garage is. A place is whatever the pack says it is: a picture, where
//! the car stands in it, and the things in it that can be picked.

use crate::gfx::{ramp, shade, Screen, Tint, BLACK, WHITE};
use crate::pack::{Pack, Place, Spot};

pub const KEY_UP: u32 = 1;
pub const KEY_DOWN: u32 = 2;
pub const KEY_LEFT: u32 = 4;
pub const KEY_RIGHT: u32 = 8;
/// Enter, Space or the mouse button: yes, pick.
pub const KEY_A: u32 = 16;
/// Escape: back.
pub const KEY_B: u32 = 32;

pub struct Game {
    pub pack: Pack,
    /// What is wrong with the pack, if it could not be read.
    pub problem: Option<String>,
    pub on_title: bool,
    pub place: String,
    pub cash: i32,
    /// The car you have, by its name in the pack, and whether its lamps are on.
    pub car: Option<String>,
    pub lamps: bool,
    pub note: Option<String>,
    pub px: i32,
    pub py: i32,
    last_keys: u32,
    last_mouse: (i32, i32),
    pub ticks: u32,
}

impl Game {
    pub fn new(pack: Result<Pack, String>) -> Game {
        let (pack, problem) = match pack {
            Ok(p) => (p, None),
            Err(e) => (Pack::default(), Some(e)),
        };
        let start = pack.manifest.start.clone();
        Game {
            place: start.place, cash: start.cash, car: start.car, pack, problem,
            on_title: true, lamps: false, note: None, px: 160, py: 100, last_keys: 0, last_mouse: (160, 100), ticks: 0,
        }
    }

    fn here(&self) -> Option<&Place> {
        self.pack.manifest.places.get(&self.place)
    }

    /// The spots of this place that are there just now, the last of them on top.
    fn spots(&self) -> Vec<&Spot> {
        self.here().map(|p| p.spots.iter().filter(|s| !s.needs_car || self.car.is_some()).collect()).unwrap_or_default()
    }

    fn spot_under(&self) -> Option<&Spot> {
        self.spots().into_iter().rev().find(|s| {
            let [x, y, w, h] = s.rect;
            self.px >= x && self.px < x + w && self.py >= y && self.py < y + h
        })
    }

    /// One thirtieth of a second: the keys held, and where the mouse is.
    pub fn tick(&mut self, keys: u32, mx: i32, my: i32) {
        let hit = keys & !self.last_keys;
        self.last_keys = keys;
        self.ticks += 1;
        if self.problem.is_some() {
            return;
        }
        if self.on_title {
            if hit & KEY_A != 0 {
                self.on_title = false;
                crate::host::tone(880, 80);
            }
            return;
        }
        if (mx, my) != self.last_mouse {
            self.last_mouse = (mx, my);
            self.px = mx;
            self.py = my;
            self.note = None;
        }
        let step = 3;
        if keys & KEY_LEFT != 0 { self.px -= step; }
        if keys & KEY_RIGHT != 0 { self.px += step; }
        if keys & KEY_UP != 0 { self.py -= step; }
        if keys & KEY_DOWN != 0 { self.py += step; }
        self.px = self.px.clamp(0, 319);
        self.py = self.py.clamp(0, 198);
        if hit & KEY_B != 0 {
            self.on_title = true;
            return;
        }
        if hit & KEY_A != 0 {
            if let Some(action) = self.spot_under().map(|s| s.action.clone()) {
                self.act(&action);
            }
        }
    }

    fn act(&mut self, action: &str) {
        self.note = None;
        if let Some(to) = action.strip_prefix("go:") {
            self.place = to.to_string();
            crate::host::tone(520, 60);
        } else if let Some(words) = action.strip_prefix("say:") {
            self.note = Some(words.to_string());
        } else if action == "lamps" {
            self.lamps = !self.lamps;
            crate::host::tone(if self.lamps { 440 } else { 330 }, 70);
        }
    }

    pub fn draw(&self, s: &mut Screen) {
        if let Some(problem) = &self.problem {
            // A pack that cannot be read says why, in words, where its maker will see it.
            s.clear(shade(ramp("NAVY"), 4));
            s.text_sh(8, 8, "THIS PACK CANNOT BE PLAYED:", shade(ramp("GOLD"), 26), BLACK);
            for (i, line) in wrap(&problem.to_uppercase(), 38).iter().enumerate() {
                s.text(8, 28 + i as i32 * 10, line, WHITE);
            }
            return;
        }
        if self.on_title {
            s.vgrad(0, 0, 320, 240, ramp("NAVY"), 3, 14);
            let name = self.pack.manifest.name.to_uppercase();
            for (i, line) in wrap(&name, 18).iter().enumerate() {
                let x = 160 - line.len() as i32 * 8;
                s.text_big(x + 1, 61 + i as i32 * 22, line, shade(ramp("RED"), 14), 2);
                s.text_big(x, 60 + i as i32 * 22, line, shade(ramp("GOLD"), 27), 2);
            }
            if (self.ticks / 15) & 1 == 1 {
                s.text_sh(116, 200, "PRESS ENTER", WHITE, BLACK);
            }
            return;
        }
        let Some(place) = self.here() else { return };
        match place.picture.as_ref().and_then(|p| self.pack.pictures.get(p)) {
            Some(pic) => {
                s.clear(BLACK);
                s.picture(pic, 0, 0);
            }
            None => s.vgrad(0, 0, 320, 200, ramp(place.backdrop.as_deref().unwrap_or("NAVY")), 4, 14),
        }
        if let (Some([x, ground]), Some(car)) = (place.car_at, self.car.as_ref().and_then(|c| self.pack.manifest.cars.get(c))) {
            let name = if self.lamps { car.side_lamps.as_ref().unwrap_or(&car.side) } else { &car.side };
            if let Some(pic) = self.pack.pictures.get(name) {
                // The picture is painted in one colour; its paint ramps are moved to the car's own.
                let mut tint = Tint::none();
                if let Some(paint) = &car.paint {
                    let to = ramp(paint) as u8;
                    for (from, add) in [(&car.paint_ramps.mid, 0i8), (&car.paint_ramps.dark, -9), (&car.paint_ramps.light, 4), (&car.paint_ramps.glint, 2)] {
                        if let Some(from) = from {
                            tint.to[ramp(from)] = to;
                            tint.add[ramp(from)] = add;
                        }
                    }
                }
                s.picture_tint(pic, x, ground - car.ground, Some(&tint));
            }
        }
        // The bottom lines: where you are, what the pointer is on, and how to play.
        let under = self.spot_under().map(|s| s.label.clone());
        s.vgrad(0, 200, 320, 40, ramp("NAVY"), 6, 2);
        s.frame(0, 200, 320, 40, shade(ramp("CHROME"), 22));
        s.frame(1, 201, 318, 38, shade(ramp("CHROME"), 9));
        s.text_sh(8, 206, &place.title.to_uppercase(), shade(ramp("GOLD"), 26), BLACK);
        let line = self.note.clone().or(under).unwrap_or_else(|| "LOOK ROUND.".into());
        s.text_sh(8, 217, &line.to_uppercase(), WHITE, BLACK);
        s.text(8, 228, "ARROWS OR MOUSE, ENTER TO PICK", shade(ramp("CHROME"), 17));
        let cash = format!("CASH ${}", self.cash);
        s.text(312 - cash.len() as i32 * 8, 206, &cash, shade(ramp("LIME"), 24));
        s.pointer(self.px, self.py);
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
