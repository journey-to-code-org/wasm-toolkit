//! Cars sold as they stand: something is wrong with each, the ad says most of it, and they are
//! somewhere else. Bought, they come home on a trailer to be mended, stripped, or sold on.

use super::*;

/// A car in the paper that does not run.
pub struct Project {
    pub owned: Owned,
    pub price: i32,
    /// Where it is, and what it costs to have it brought.
    pub from: String,
    pub ship: i32,
    /// How many days it takes to get here.
    pub days: i32,
    /// What the seller says about it.
    pub says: Vec<String>,
}

/// How many are in the paper each day.
pub const PROJECTS: usize = 2;

impl Game {
    /// Today's, in the order they are printed: `None` for one that has gone.
    pub fn projects(&self) -> Vec<Option<Project>> {
        let m = &self.pack.manifest;
        let cars: Vec<(&String, &Car)> = m.cars.iter().filter(|(_, c)| !c.stock.is_empty()).collect();
        if cars.is_empty() || m.faults.is_empty() {
            return Vec::new();
        }
        let day = self.day();
        (0..PROJECTS).map(|i| {
            if self.state.sold.0 == day && self.state.sold.1.contains(&(10 + i)) {
                return None;
            }
            let mut k = hash(day as u32 * 7919 + i as u32 * 104729 + 99);
            let mut next = |n: usize| {
                k = hash(k);
                (k >> 8) as usize % n.max(1)
            };
            let (id, car) = cars[next(cars.len())];
            let paints = self.paints();
            let mut o = Owned::new(id, Some(paints[next(paints.len())].clone()), 35 + next(30) as i32);
            self.outfit(&mut o);
            o.body = next(45) as i32;
            let mut says = Vec::new();
            let done = |o: &mut Owned, slot: &String| {
                if let Some(id) = o.fitted.get(slot).cloned() {
                    // A battery that has sat is flat, not finished: the cheapest mend there is.
                    let left = if self.part(&id).is_some_and(|p| p.charges) { 8 } else { 0 };
                    o.fitted.insert(slot.clone(), worn(&id, left));
                }
            };
            // What is wrong with it: one thing, or two, or now and then three.
            let wrong = 1 + next(2) + usize::from(next(4) == 0);
            let mut picked: Vec<usize> = Vec::new();
            let mut unsaid: Vec<&String> = Vec::new();
            let mut fine: Vec<&String> = Vec::new();
            for n in 0..wrong.min(m.faults.len()) {
                let mut f = next(m.faults.len());
                while picked.contains(&f) {
                    f = (f + 1) % m.faults.len();
                }
                picked.push(f);
                let fault = &m.faults[f];
                // The second thing wrong is sometimes left out of the ad altogether.
                if n > 0 && next(4) == 0 {
                    unsaid.extend(&fault.breaks);
                } else {
                    says.push(fault.says.clone());
                    for slot in &fault.breaks {
                        done(&mut o, slot);
                    }
                    // And sometimes the seller is wrong about how bad it is.
                    if next(5) == 0 {
                        fine.extend(&fault.breaks);
                    }
                }
                unsaid.extend(&fault.hidden);
            }
            // Parts that someone has had off it already; the ad owns up to the first.
            let slots: Vec<&String> = car.stock.keys().collect();
            for n in 0..next(3) {
                let slot = slots[next(slots.len())];
                if o.fitted.remove(slot).is_some() && n == 0 {
                    says.push(format!("No {}.", slot.to_lowercase()));
                }
            }
            // And now and then something better than stock is still on it.
            let extras: Vec<(&String, &Part)> = m.parts.iter().filter(|(pid, p)| car.stock.get(&p.slot).is_some_and(|s| s != *pid) && p.goes_on(id) && p.needs.is_empty()).collect();
            if !extras.is_empty() && next(3) == 0 {
                let (pid, p) = extras[next(extras.len())];
                o.fitted.insert(p.slot.clone(), worn(pid, 60 + next(40) as i32));
                says.push(format!("Has a {}.", p.name.to_lowercase()).chars().take(38).collect());
            }
            // The price is for the car the ad describes, give or take; then the car is what it is.
            let asked = self.value(&o) * (50 + next(35) as i32) / 100 + 40;
            for slot in unsaid {
                done(&mut o, slot);
            }
            for slot in fine {
                if let Some(id) = o.fitted.get(slot).cloned() {
                    o.fitted.insert(slot.clone(), worn(&id, 45 + next(40) as i32));
                }
            }
            let off = if self.state.knocked.0 == day { self.state.knocked.1.iter().find(|k| k.0 == 10 + i).map_or(0, |k| k.1) } else { 0 };
            let (from, ship, days) = if m.sellers.is_empty() { ("out of town".to_string(), 60, 0) } else { let s = &m.sellers[next(m.sellers.len())]; (s.from.clone(), s.ship, s.days) };
            Some(Project { owned: o, price: asked * (100 - off) / 100, from, ship, days, says })
        }).collect()
    }

    /// Bought as it stands, and brought home: it goes in the yard, or in the bay if the bay is empty.
    pub fn buy_project(&mut self, i: usize) {
        let Some(Some(p)) = self.projects().into_iter().nth(i) else { return };
        if self.state.cash < p.price + p.ship {
            return self.no(&format!("${} and ${} to bring it. Not yet.", p.price, p.ship));
        }
        if self.state.cars.len() + self.state.coming.len() >= MOST_CARS {
            return self.no("No room in the yard for another.");
        }
        let day = self.day();
        self.state.cash -= p.price + p.ship;
        if self.state.sold.0 != day {
            self.state.sold = (day, Vec::new());
        }
        self.state.sold.1.push(10 + i);
        tone(880, 80);
        if p.days > 0 {
            // On a trailer: it gets here when it gets here.
            let when = day + p.days;
            self.state.coming.push((when, p.owned, p.from));
            self.say(&format!("Yours. On a trailer, here day {when}."));
            return self.save();
        }
        self.state.cars.push(p.owned);
        if self.state.cur.is_none() {
            self.state.cur = Some(self.state.cars.len() - 1);
        }
        self.say(&format!("Yours. ${} to bring it from {}.", p.ship, p.from).chars().take(38).collect::<String>());
        self.save();
    }

    /// Five per cent more off one of them, as for any car in the paper: the same floor, the same risk.
    pub(super) fn haggle_project(&mut self, i: usize) {
        let Some(Some(_)) = self.projects().into_iter().nth(i) else { return };
        let (day, k) = (self.day(), 10 + i);
        if self.state.knocked.0 != day {
            self.state.knocked = (day, Vec::new());
        }
        let had = self.state.knocked.1.iter().find(|x| x.0 == k).map_or(0, |x| x.1);
        if had + 5 > 5 + (hash(day as u32 * 31 + k as u32 + 17) % 5) as i32 * 5 {
            if self.state.sold.0 != day {
                self.state.sold = (day, Vec::new());
            }
            self.state.sold.1.push(k);
            self.no("\"Forget it.\" He hung up.");
        } else {
            self.state.knocked.1.retain(|x| x.0 != k);
            self.state.knocked.1.push((k, had + 5));
            let now = self.projects().into_iter().nth(i).flatten().map_or(0, |p| p.price);
            self.say(&format!("\"All right. ${now}.\""));
            tone(660, 80);
        }
        self.save();
    }

    pub(super) fn draw_projects(&self, s: &mut Screen, ink: u16, grey: u16, hover: u16) {
        let list = self.projects();
        let over = self.row(8, 312, 54, 62, PROJECTS);
        for (i, p) in list.iter().enumerate() {
            let y = 54 + i as i32 * 62;
            s.rect(8, y + 61, 304, 1, grey);
            let Some(p) = p else {
                s.text(12, y + 24, "GONE.", grey);
                continue;
            };
            if over == Some(i) {
                s.rect(8, y, 304, 61, hover);
            }
            let car = &self.pack.manifest.cars[&p.owned.car];
            self.draw_car(s, &p.owned.car, p.owned.paint.as_deref(), false, 12, y + 41, true);
            s.text(180, y + 4, &car.name.to_uppercase().chars().take(16).collect::<String>(), ink);
            s.text(180, y + 14, &format!("IN {}", p.from.to_uppercase()).chars().take(16).collect::<String>(), grey);
            s.text(180, y + 24, &format!("${} + ${}", p.price, p.ship), shade(ramp("FOREST"), 12));
            let on = over == Some(i) && self.px >= 248 && (self.py - 54) % 62 >= 30 && (self.py - 54) % 62 < 42;
            if p.days > 0 {
                s.text(180, y + 34, &format!("{} DAY{}", p.days, if p.days == 1 { "" } else { "S" }), grey);
            }
            s.text(256, y + 34, "HAGGLE", if on { shade(ramp("RED"), 20) } else { grey });
            // What the seller has to say for it, as much as there is room for.
            for (n, line) in p.says.iter().take(2).enumerate() {
                s.text(12, y + 43 + n as i32 * 9, &line.to_uppercase().chars().take(37).collect::<String>(), ink);
            }
        }
        if list.is_empty() {
            s.text(12, 60, "NOTHING SOLD AS IT STANDS TODAY.", grey);
        }
    }
}
