//! The rollers: what the car in the bay makes, drawn as a curve, and what that is worth on the
//! strip and on the road.

use super::*;

/// What the rollers say about a car.
#[derive(Clone, Debug, PartialEq)]
pub struct Sheet {
    /// Horsepower and torque at each engine speed, as the car stands; and its horsepower as it left the factory.
    pub curve: Vec<(f64, f64, f64)>,
    pub stock: Vec<(f64, f64)>,
    pub hp: f64,
    pub hp_at: f64,
    pub torque: f64,
    pub torque_at: f64,
    pub redline: f64,
    pub weight_lb: f64,
    /// The quarter mile, driven well: thousandths of a second, and miles an hour through the lights.
    pub et: i32,
    pub mph: i32,
    /// Flat out on a road, in miles an hour.
    pub top: f64,
}

/// Where a visit to the rollers has got to: which of the four straps are on, whether the car is
/// being strapped (0), pulled (1) or read (2), the revs, and what the pull logged.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rollers {
    pub straps: u8,
    pub phase: u8,
    pub rpm: f64,
    pub started: bool,
    strain: f64,
    pub log: Vec<Logged>,
}

/// One line of the log: what the engine was doing at an engine speed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Logged {
    pub rpm: f64,
    pub hp: f64,
    pub boost: f64,
    pub afr: f64,
    pub timing: f64,
    pub knock: i32,
}

/// What a tune does: how much of its power the engine makes (1 is as it left the factory), and
/// how hard it knocks (nothing, up to 6 on this scale, is safe). Timing and boost make power and
/// knock; fuel cools it, and too much or too little costs power.
pub fn tuned(e: Ecu, turbo: bool) -> (f64, f64) {
    let boost = if turbo { e.boost as f64 } else { 0.0 };
    let knock = (e.timing as f64 + boost * 1.5 - e.fuel as f64 * 0.8 - 6.0).max(0.0);
    let fuel = if e.fuel > -2 { 0.004 * (e.fuel + 2) as f64 } else { 0.012 * (-2 - e.fuel) as f64 };
    // The computer pulls timing when it hears knock, and then some.
    ((1.0 + 0.01 * e.timing as f64 + 0.025 * boost + 0.008 - fuel - 0.035 * knock).clamp(0.5, 1.5), knock)
}

impl Game {
    /// Whether the car in the bay has a laptop to tune it with.
    pub fn tuner(&self) -> bool {
        self.owned().is_some_and(|o| o.fitted.values().filter_map(|id| self.pack.manifest.parts.get(base(id))).any(|p| p.tuner))
    }

    /// A run with the engine knocking takes it out of the engine.
    pub(super) fn ping(&mut self) {
        let Some((o, car)) = self.owned().zip(self.car()) else { return };
        let knock = tuned(o.ecu, self.tune(o).1.is_some() || car.drive.boost.is_some()).1;
        if knock > 0.0 {
            if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)) {
                o.cond = (o.cond - knock.ceil() as i32).max(25);
            }
        }
    }

    /// Where the four rings in the floor are, for the car in the bay: under its nose and tail.
    fn rings(&self) -> [(i32, i32); 4] {
        let wide = self.car().and_then(|c| self.pack.pictures.get(&c.side)).map_or(120, |p| p.w).clamp(120, 250);
        [(26, 172), (26 + wide * 3 / 10, 172), (26 + wide * 7 / 10, 172), (26 + wide, 172)]
    }

    pub(super) fn dyno_tick(&mut self, pick: bool, back: bool) {
        if back {
            self.scene = Scene::Place;
            self.sheet = None;
            return;
        }
        if !pick {
            return;
        }
        if self.rollers.phase == 0 {
            // A strap goes on at each ring that is picked; with all four on, it is ready.
            if let Some(i) = self.rings().iter().position(|&(x, y)| (self.px - x).abs() <= 9 && (self.py - y).abs() <= 9) {
                self.rollers.straps |= 1 << i;
                tone(300 + i as i32 * 60, 60);
                if self.rollers.straps == 15 {
                    self.rollers.phase = 1;
                    self.rollers.rpm = 1500.0;
                }
            }
        } else if let Some(b) = self.button(3) {
            match b {
                0 => {
                    let straps = self.rollers.straps;
                    self.rollers = Rollers { straps, phase: 1, rpm: 1500.0, ..Rollers::default() };
                    self.sheet = None;
                }
                1 if self.tuner() => {
                    // On the laptop with a fresh log: the map is made to fit what is on it now.
                    if let Some(o) = self.state.cur.and_then(|i| self.state.cars.get_mut(i)).filter(|o| o.untuned) {
                        o.untuned = false;
                        self.note = Some("Mapped to the new parts.".into());
                        self.save();
                    }
                    self.scene = Scene::Laptop;
                }
                1 => self.no("That takes a laptop and a cable."),
                _ => {
                    self.scene = Scene::Place;
                    self.sheet = None;
                }
            }
        }
    }

    /// The pull: foot down from fifteen hundred to the redline, and off. Held on the limiter it lets go.
    pub(super) fn pull_tick(&mut self, keys: u32, hit: u32) {
        let Some((o, car)) = self.owned().zip(self.car()) else { return };
        if hit & KEY_B != 0 {
            tone(0, 0);
            self.scene = Scene::Place;
            return;
        }
        let (d, (gain, turbo)) = (self.tuned_drive(o, car), self.tune(o));
        let (k, ecu) = (gain * self.blown(o, car), o.ecu);
        let boost = turbo.or(car.drive.boost.clone());
        let knock = tuned(ecu, boost.is_some()).1;
        let at = |rpm: f64| {
            let t = &d.torque;
            let mut v = t[t.len() - 1][1];
            for w in t.windows(2) {
                if rpm <= w[1][0] {
                    v = w[0][1] + (w[1][1] - w[0][1]) * ((rpm - w[0][0]) / (w[1][0] - w[0][0])).max(0.0);
                    break;
                }
            }
            v * k
        };
        let most = d.torque.iter().map(|p| p[1]).fold(1.0, f64::max) * k;
        let r = &mut self.rollers;
        let was = r.rpm;
        if keys & KEY_UP != 0 {
            r.started = true;
            r.rpm = (r.rpm + 9.0 + 26.0 * at(r.rpm) / most).min(d.redline);
            // A line in the log every two hundred and fifty revs.
            if (r.rpm / 250.0).floor() > (was / 250.0).floor() && r.rpm < d.redline {
                let rpm = (r.rpm / 250.0).floor() * 250.0;
                let psi = boost.as_ref().map_or(0.0, |b| (b.psi + ecu.boost as f64).max(0.0) * ((rpm - b.from) / 1500.0).clamp(0.0, 1.0));
                r.log.push(Logged {
                    rpm, hp: at(rpm) * rpm / 5252.0, boost: psi,
                    afr: if boost.is_some() { 11.6 } else { 12.9 } - ecu.fuel as f64 * 0.12,
                    timing: (8.0 + rpm / 350.0).min(30.0) + ecu.timing as f64 - knock * 1.5,
                    knock: if rpm > 3500.0 { (knock * rpm / d.redline).round() as i32 } else { 0 },
                });
            }
            if r.rpm >= d.redline {
                r.strain += 1.0;
            }
            let hz = (r.rpm / 40.0) as i32;
            if !self.voiced {
                tone(hz, 60);
            }
            if r.strain > crate::race::BLOWS_AT {
                self.scene = Scene::Place;
                self.blew();
                tone(70, 600);
            }
        } else if r.started {
            // Off it: the rollers run down, and there is the sheet.
            r.phase = 2;
            tone(0, 0);
            self.sheet = self.dyno();
            self.ping();
            if self.owned().is_some_and(|o| self.slip(o) < 0.99) {
                self.say("The clutch is slipping.");
            } else if knock > 0.0 {
                self.say("It was pinging. That hurts it.");
            } else {
                self.note = None;
            }
            self.save();
        } else {
            tone(22, 60);
        }
    }

    pub(super) fn laptop_tick(&mut self, pick: bool, back: bool) {
        if back {
            self.scene = Scene::Dyno;
            return;
        }
        let Some(cur) = self.state.cur.filter(|_| pick) else { return };
        let turbo = self.owned().zip(self.car()).is_some_and(|(o, car)| self.tune(o).1.is_some() || car.drive.boost.is_some());
        for (k, (low, high)) in [(-5, 10), (-10, 10), (-6, 10)].into_iter().enumerate() {
            let y = 46 + k as i32 * 38;
            let step = if (self.py - y - 7).abs() <= 8 { if (self.px - 222).abs() <= 11 { -1 } else if (self.px - 300).abs() <= 11 { 1 } else { 0 } } else { 0 };
            if step != 0 {
                if k == 0 && !turbo {
                    return self.no("There is no turbo to turn up.");
                }
                let e = &mut self.state.cars[cur].ecu;
                let v = match k { 0 => &mut e.boost, 1 => &mut e.fuel, _ => &mut e.timing };
                *v = (*v + step).clamp(low, high);
                tone(if step > 0 { 700 } else { 500 }, 40);
                self.note = Some("Changed. Pull it again to see.".into());
                self.save();
            }
        }
    }

    /// The laptop: the log of the last pull down the left, and the tune down the right.
    pub(super) fn draw_laptop(&self, s: &mut Screen) {
        let (green, dim, red, white) = (shade(ramp("LIME"), 24), shade(ramp("FOREST"), 16), shade(ramp("RED"), 27), shade(ramp("WHITE"), 26));
        s.vgrad(0, 0, 320, 200, ramp("GREY"), 6, 3);
        s.rect(4, 4, 312, 190, BLACK);
        s.frame(4, 4, 312, 190, shade(ramp("CHROME"), 14));
        s.text(10, 9, "DATALOG: THE LAST PULL", green);
        s.text(10, 22, " RPM  PSI  AFR  TIM  KN", dim);
        // Every other line of the log, so that it fits: one for each five hundred revs.
        let rows: Vec<&Logged> = self.rollers.log.iter().filter(|l| (l.rpm as i32) % 500 == 0).collect();
        let skip = rows.len().saturating_sub(17);
        for (i, l) in rows.iter().skip(skip).enumerate() {
            let line = format!("{:4} {:4.1} {:4.1} {:4.0} {:3}", l.rpm as i32, l.boost, l.afr, l.timing, l.knock);
            s.text(10, 32 + i as i32 * 9, &line, if l.knock > 0 { red } else { green });
        }
        if rows.is_empty() {
            s.text(10, 40, "NOTHING LOGGED YET.", dim);
        }
        let Some(o) = self.owned() else { return };
        let turbo = self.car().is_some_and(|car| self.tune(o).1.is_some() || car.drive.boost.is_some());
        for (k, (name, v, unit)) in [("BOOST", o.ecu.boost, "PSI"), ("FUEL", o.ecu.fuel, "%"), ("TIMING", o.ecu.timing, "DEG")].into_iter().enumerate() {
            let y = 46 + k as i32 * 38;
            s.text(212, y - 12, name, if k == 0 && !turbo { dim } else { white });
            for (x, sign) in [(222, "-"), (300, "+")] {
                let on = (self.px - x).abs() <= 11 && (self.py - y - 7).abs() <= 8;
                s.frame(x - 10, y, 20, 15, if on { white } else { dim });
                s.text(x - 4, y + 4, sign, if on { white } else { green });
            }
            let value = format!("{v:+} {unit}");
            s.text(261 - value.len() as i32 * 4, y + 4, &value, green);
        }
        let (power, knock) = tuned(o.ecu, turbo);
        s.text(212, 150, &format!("POWER {:3.0}%", power * 100.0), green);
        s.text(212, 162, if knock > 3.0 { "KNOCK: BAD" } else if knock > 0.0 { "KNOCK: SOME" } else { "KNOCK: NONE" }, if knock > 0.0 { red } else { green });
        self.panel(s, "The laptop", self.note.as_deref().unwrap_or("Red lines knocked. Fuel cools it."), "Pick - and +. Escape: the rollers.");
    }
    /// How much more than its measured curve a car makes for the turbo that is on it.
    pub(super) fn blown(&self, o: &Owned, car: &Car) -> f64 {
        match (self.tune_engine(o).1.or(car.drive.boost.clone()), car.drive.boost.as_ref()) {
            (Some(b), Some(stock)) => (0.45 + 0.55 * b.psi / stock.psi.max(1.0)).min(1.6),
            (Some(_), None) => 1.25,
            _ => 1.0,
        }
    }

    /// The car in the bay, on the rollers.
    pub fn dyno(&self) -> Option<Sheet> {
        let (o, car) = self.owned().zip(self.car())?;
        let (d, (gain, turbo)) = (self.tuned_drive(o, car), self.tune(o));
        let k = gain * self.blown(o, car);
        let curve: Vec<(f64, f64, f64)> = d.torque.iter().filter(|p| p[0] <= d.redline + 1.0).map(|p| (p[0], p[1] * k, p[0] * p[1] * k / 5252.0)).collect();
        let stock = car.drive.torque.iter().filter(|p| p[0] <= car.drive.redline + 1.0).map(|p| (p[0], p[0] * p[1] / 5252.0)).collect();
        let best = |f: fn(&(f64, f64, f64)) -> f64| curve.iter().fold((0.0, 0.0), |m, p| if f(p) > m.1 { (p.0, f(p)) } else { m });
        let ((hp_at, hp), (torque_at, torque)) = (best(|p| p.2), best(|p| p.1));
        // A pass with nobody watching: off the limiter on the last amber, and a shift short of the redline.
        let mut r = Race::new(&d, gain, turbo, 0, 0);
        r.tick(Controls { stage: true, ..Controls::default() });
        for _ in 0..3000 {
            if r.phase == Phase::Slip {
                break;
            }
            let up = (r.gear == 0 && r.tree >= 51) || (r.gear > 0 && r.rpm > d.redline - 250.0);
            r.tick(Controls { throttle: true, up, ..Controls::default() });
        }
        Some(Sheet { curve, stock, hp, hp_at, torque, torque_at, redline: d.redline, weight_lb: d.weight_lb, et: if r.blown { 0 } else { r.et }, mph: r.mph, top: self.pace().top * crate::road::MPH })
    }

    /// The bay with the rollers in the floor: the car on them, its straps, and the screen on the wall.
    fn draw_bay(&self, s: &mut Screen) {
        let (gold, white, dim) = (shade(ramp("GOLD"), 27), shade(ramp("WHITE"), 26), shade(ramp("GREY"), 12));
        s.vgrad(0, 0, 320, 150, ramp("GREY"), 7, 11);
        s.vgrad(0, 150, 320, 50, ramp("CONCRETE"), 14, 9);
        let r = &self.rollers;
        let rings = self.rings();
        // The rollers, turning when the wheels are.
        for &(x, _) in &rings[1..3] {
            s.rect(x - 22, 150, 44, 8, BLACK);
            for k in 0..6 {
                let off = if r.phase == 1 { (r.rpm as i32 / 90 + self.ticks as i32 * 3) % 8 } else { 0 };
                s.rect(x - 22 + (k * 8 + off) % 44, 151, 2, 6, shade(ramp("CHROME"), 16));
            }
        }
        if let Some(o) = self.owned() {
            let shake = if r.phase == 1 && r.started { (self.ticks % 2) as i32 } else { 0 };
            self.draw_owned(s, o, false, 26, 152 - shake, false);
        }
        for (i, &(x, y)) in rings.iter().enumerate() {
            let on = r.straps & (1 << i) != 0;
            let near = (self.px - x).abs() <= 9 && (self.py - y).abs() <= 9;
            if on {
                // A strap, from the ring up to the car.
                let to = if i < 2 { x + 10 } else { x - 10 };
                for d in 0..2 {
                    s.line(x + d, y, to + d, 146, shade(ramp("YELLOW"), 24));
                }
            }
            s.disc(x, y, 5, if on { shade(ramp("CHROME"), 12) } else if near && r.phase == 0 { white } else { gold });
            s.disc(x, y, 2, BLACK);
        }
        // The screen on the wall: the curve as it is made.
        s.rect(196, 8, 118, 76, BLACK);
        s.frame(196, 8, 118, 76, shade(ramp("CHROME"), 16));
        if r.phase == 0 {
            s.text(202, 14, "STRAP IT DOWN", gold);
            s.text(202, 28, &format!("{} OF 4 ON", r.straps.count_ones()), white);
        } else {
            let most = r.log.iter().map(|l| l.hp).fold(100.0, f64::max);
            let redline = self.car().map_or(7000.0, |c| c.drive.redline).max(r.rpm);
            for l in &r.log {
                let (x, y) = (200 + (l.rpm / redline * 108.0) as i32, 80 - (l.hp / most * 50.0) as i32);
                s.rect(x, y, 2, 2, if l.knock > 0 { shade(ramp("RED"), 27) } else { shade(ramp("LIME"), 24) });
            }
            s.text(202, 12, &format!("{:5.0} RPM", r.rpm), white);
            s.text(202, 22, &format!("{:5.0} HP", r.log.last().map_or(0.0, |l| l.hp)), gold);
        }
        let _ = dim;
        let (line, foot) = if r.phase == 0 { ("Pick each ring in the floor.", "Four straps, then the pull. Escape.") } else if r.started { ("Keep it down to the redline. Then off.", "Held on the limiter, it will let go.") } else { ("Strapped. Hold the gas for the pull.", "Up, or Gas. Let off at the redline.") };
        self.panel(s, "The rollers", self.note.as_deref().unwrap_or(line), foot);
    }

    pub(super) fn draw_dyno(&self, s: &mut Screen) {
        if self.rollers.phase < 2 {
            return self.draw_bay(s);
        }
        s.vgrad(0, 0, 320, 200, ramp("NAVY"), 3, 9);
        let (cream, grey, gold, red, dim) = (shade(ramp("CREAM"), 26), shade(ramp("GREY"), 14), shade(ramp("GOLD"), 27), shade(ramp("RED"), 26), shade(ramp("NAVY"), 14));
        s.text_sh(8, 6, "THE ROLLERS", WHITE, BLACK);
        let Some((sheet, car)) = self.sheet.as_ref().zip(self.car()) else { return };
        s.text_sh(112, 6, &car.name.to_uppercase().chars().take(25).collect::<String>(), gold, BLACK);
        // The graph: engine speed along the bottom, in thousands; power and torque up the side.
        let (x0, y0, w, h) = (26, 24, 176, 140);
        let most = sheet.curve.iter().map(|p| p.1.max(p.2)).chain(sheet.stock.iter().map(|p| p.1)).fold(50.0, f64::max);
        let (top, revs) = ((most / 50.0).ceil() * 50.0, ((sheet.redline.max(sheet.curve.last().map_or(0.0, |p| p.0)) / 1000.0).ceil() * 1000.0).max(1000.0));
        s.rect(x0, y0, w, h, shade(ramp("NAVY"), 2));
        let mut v = 0.0;
        while v <= top {
            let y = y0 + h - (v / top * h as f64) as i32;
            s.rect(x0, y.min(y0 + h - 1), w, 1, dim);
            if (v as i32) % 100 == 0 {
                s.text(2, y - 4, &format!("{:3}", v as i32), grey);
            }
            v += 50.0;
        }
        let mut k = 0.0;
        while k <= revs {
            let x = x0 + (k / revs * w as f64) as i32;
            s.rect(x.min(x0 + w - 1), y0, 1, h, dim);
            s.text(x - 3, y0 + h + 3, &format!("{}", (k / 1000.0) as i32), grey);
            k += 1000.0;
        }
        let at = |rpm: f64, v: f64| (x0 + (rpm / revs * w as f64) as i32, y0 + h - (v / top * h as f64) as i32);
        let mut plot = |pts: Vec<(i32, i32)>, c: u16| {
            for p in pts.windows(2) {
                s.line(p[0].0, p[0].1, p[1].0, p[1].1, c);
                s.line(p[0].0, p[0].1 + 1, p[1].0, p[1].1 + 1, c);
            }
        };
        plot(sheet.stock.iter().map(|p| at(p.0, p.1)).collect(), grey);
        plot(sheet.curve.iter().map(|p| at(p.0, p.1)).collect(), gold);
        plot(sheet.curve.iter().map(|p| at(p.0, p.2)).collect(), red);
        s.frame(x0, y0, w, h, shade(ramp("CHROME"), 16));
        s.text(x0 + 4, y0 + 4, "POWER", red);
        s.text(x0 + 52, y0 + 4, "TORQUE", gold);
        s.text(x0 + 108, y0 + 4, "STOCK", grey);
        // And the numbers.
        let o = self.owned();
        let lines = [
            ("POWER", format!("{:.0} HP", sheet.hp)),
            ("AT", format!("{:.0}", sheet.hp_at)),
            ("TORQUE", format!("{:.0}", sheet.torque)),
            ("AT", format!("{:.0}", sheet.torque_at)),
            ("WEIGHT", format!("{:.0} LB", sheet.weight_lb)),
            ("QUARTER", if sheet.et > 0 { format!("{:.2}", sheet.et as f64 / 1000.0) } else { "BLOWN".into() }),
            ("AT", format!("{} MPH", sheet.mph)),
            ("ROAD", format!("{:.0} MPH", sheet.top)),
            ("ENGINE", o.map_or(String::new(), |o| format!("{}%", o.cond))),
            ("WORTH", o.map_or(String::new(), |o| format!("${}", self.value(o)))),
            ("BODY", o.map_or(String::new(), |o| format!("{}%", 100 - o.body))),
            ("TYRES", o.map_or(String::new(), |o| format!("{:.0}%", 100.0 - o.tyres))),
            ("BRAKES", o.map_or(String::new(), |o| format!("{:.0}%", self.chassis(o).0 * 100.0))),
            ("HANDLING", o.map_or(String::new(), |o| format!("{:.0}%", self.chassis(o).1 * 100.0))),
        ];
        for (i, (name, value)) in lines.iter().enumerate() {
            let y = 26 + i as i32 * 11;
            s.text(208, y, name, grey);
            s.text(320 - 4 - value.len() as i32 * 8, y, value, cream);
        }
        self.buttons(s, &["PULL AGAIN".to_string(), "LAPTOP".to_string(), "DONE".to_string()]);
        let line = if self.owned().is_some_and(|o| o.untuned) { "New parts, old tune: it needs a tune." } else { "What it makes, and what that is worth." };
        self.panel(s, "The rollers", self.note.as_deref().unwrap_or(line), "Grey: as it left the factory. Escape.");
    }
}
