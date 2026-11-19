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

impl Game {
    /// How much more than its measured curve a car makes for the turbo that is on it.
    pub(super) fn blown(&self, o: &Owned, car: &Car) -> f64 {
        match (self.tune(o).1.or(car.drive.boost.clone()), car.drive.boost.as_ref()) {
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

    pub(super) fn draw_dyno(&self, s: &mut Screen) {
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
            ("WON", format!("{}", self.state.wins)),
            ("LOST", format!("{}", self.state.losses)),
        ];
        for (i, (name, value)) in lines.iter().enumerate() {
            let y = 26 + i as i32 * 12;
            s.text(208, y, name, grey);
            s.text(320 - 4 - value.len() as i32 * 8, y, value, cream);
        }
        self.panel(s, "The rollers", self.note.as_deref().unwrap_or("What it makes, and what that is worth."), "Grey: as it left the factory. Escape.");
    }
}
