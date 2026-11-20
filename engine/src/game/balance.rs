//! A player made of code, for tuning the numbers: it plays whole seasons of the default pack the
//! way a sensible person would, and says where the money, the wear and the races go wrong.
//!
//!     cd engine && cargo test --lib balance -- --nocapture --ignored
//!
//! It keeps its car running (parts, a charge, a tune, a rebuild, tyres, gas), races the next one
//! up the ladder for what it can afford to lose, launches on the green a human reaction late,
//! shifts near the redline and steers to hold its lane. When it cannot win it works, and buys
//! whatever takes the most off its time for the money.

#![cfg(test)]

use super::*;
use std::collections::BTreeMap;

/// What a season came to.
#[derive(Default, Debug)]
pub struct Season {
    pub end: String,
    pub day: i32,
    pub rung: usize,
    pub cash: i32,
    pub races: u32,
    pub wins: u32,
    pub out: u32,
    pub blown: u32,
    pub broke: BTreeMap<String, u32>,
    pub worn: BTreeMap<String, u32>,
    pub repairs: i32,
    pub upgrades: i32,
    pub upgraded: Vec<String>,
    pub won_money: i32,
    pub farmed: u32,
    pub cars: Vec<String>,
    /// Hours spent, by what on.
    pub time: BTreeMap<String, i32>,
    pub lost_money: i32,
    pub worked: i32,
    pub tunes: u32,
    pub rebuilds: u32,
    /// The day each rung was reached.
    pub rungs: Vec<i32>,
}

fn default_game(seed: u32) -> Game {
    crate::host::store(b"");
    let bytes = std::fs::read("../web/default.pack").expect("run make engine first");
    let mut g = Game::new(Pack::parse(&bytes));
    g.seed(seed);
    g.state = g.fresh();
    g.scene = Scene::Place;
    g
}

impl Game {
    /// Bolts a part off the shelf onto the car in the bay, as the bench does; a dead part comes
    /// off into the bin.
    fn bot_fit(&mut self, id: &str) {
        let Some(cur) = self.state.cur else { return };
        let Some(slot) = self.part(id).map(|p| p.slot.clone()) else { return };
        self.state.spares.retain(|p| p != id);
        self.state.cars[cur].off.retain(|s| *s != slot);
        self.swapped(id);
        if let Some(old) = self.state.cars[cur].fitted.insert(slot, id.to_string()) {
            self.swapped(&old);
            if cond_of(&old) > 0 {
                self.state.spares.push(old);
            }
        }
        self.spend(1);
    }

    /// Buys a part new and bolts it on. False if it cannot be had or paid for.
    fn bot_buy_fit(&mut self, id: &str, s: &mut Season, upgrade: bool) -> bool {
        if let Some(have) = self.state.spares.iter().find(|p| base(p) == id && cond_of(p) > 30).cloned() {
            self.bot_fit(&have);
            return true;
        }
        let Some(i) = self.shop().iter().position(|(k, _)| k.as_str() == id) else { return false };
        let price = self.shop()[i].1.price;
        if self.state.cash < price {
            return false;
        }
        self.buy_part(i);
        if upgrade { s.upgrades += price } else { s.repairs += price }
        self.bot_fit(id);
        true
    }

    /// Works the best-paying job for an hour of time.
    fn bot_work(&mut self, s: &mut Season) {
        let before = self.state.hours;
        let jobs = self.jobs();
        let best = (0..jobs.len()).max_by(|&a, &b| (jobs[a].pay * jobs[b].hours).cmp(&(jobs[b].pay * jobs[a].hours))).unwrap_or(0);
        self.work(best);
        s.worked += self.state.hours - before;
    }

    /// Gets the car in the bay to where it will race. False if it cannot, for want of money.
    fn bot_keep_up(&mut self, s: &mut Season) -> bool {
        for _ in 0..40 {
            if matches!(self.scene, Scene::Over(_)) {
                return false;
            }
            let Some(cur) = self.state.cur else { return false };
            let car = self.pack.manifest.cars[&self.state.cars[cur].car].clone();
            let o = self.state.cars[cur].clone();
            // A worn part replaced before it goes; a dead or missing one replaced now.
            let dead = car.stock.iter().filter(|(slot, _)| !car.optional.contains(slot)).find(|(slot, _)| match o.fitted.get(*slot) {
                None => true,
                Some(id) => cond_of(id) == 0 || (cond_of(id) < 12 && self.part(id).is_some_and(|p| p.wears > 0.0)),
            });
            if let Some((slot, stock)) = dead {
                let fitted = o.fitted.get(slot).cloned();
                // Put back what was there if it was better than stock and can be had; otherwise stock.
                let mut want = fitted.as_deref().map(base).filter(|b| self.shop().iter().any(|(k, _)| k.as_str() == *b)).unwrap_or(stock).to_string();
                // A driveline part that let go: the strongest one for the slot that can be had and fits.
                if fitted.as_deref().is_some_and(|f| cond_of(f) == 0 && self.part(f).is_some_and(|p| p.holds.is_some())) {
                    let cash = self.state.cash;
                    if let Some((k, _)) = self.shop().into_iter().filter(|(_, p)| p.slot == *slot && p.goes_on(&o.car) && p.price <= cash && p.holds.is_some())
                        .filter(|(k, _)| self.bot_bundle(k).is_some_and(|b| b.len() == 1)).max_by(|a, b| a.1.holds.unwrap().total_cmp(&b.1.holds.unwrap())) {
                        want = k.clone();
                    }
                }
                if fitted.is_some() {
                    *s.worn.entry(slot.clone()).or_default() += 1;
                }
                if !self.bot_buy_fit(&want, s, false) && !self.bot_buy_fit(stock, s, false) {
                    self.bot_work(s);
                }
                continue;
            }
            if let Some(why) = self.cannot_race() {
                if why.contains("flat") {
                    s.repairs += 5;
                    self.act("charge");
                } else if why.contains("gas") {
                    if self.state.cash < 5 { self.bot_work(s) } else { self.act("gas") }
                } else {
                    // Something the bot does not know how to mend: it has a problem.
                    s.end = format!("stuck: {why}");
                    return false;
                }
                continue;
            }
            if o.untuned {
                if self.state.cash < self.pack.manifest.tuneup.cost { self.bot_work(s); continue; }
                s.tunes += 1;
                s.repairs += self.pack.manifest.tuneup.cost;
                self.act("tuneup");
                continue;
            }
            if o.cond < 55 {
                let cost = self.rebuild_cost();
                if self.state.cash < cost + 30 { self.bot_work(s); continue; }
                s.rebuilds += 1;
                s.repairs += cost;
                self.act("rebuild");
                continue;
            }
            if o.tyres > 70.0 {
                if self.state.cash < 100 { self.bot_work(s); continue; }
                s.repairs += 80;
                self.act("tyres");
                continue;
            }
            if o.body > 60 && self.state.cash > 20 + o.body * 4 + 150 {
                s.repairs += 20 + o.body * 4;
                self.act("bodywork");
                continue;
            }
            if o.gas < 3.0 && self.state.cash > 10 {
                self.act("gas");
            }
            return true;
        }
        false
    }

    /// The quarter as the car in the bay would run it, driven well, with these parts on.
    fn bot_et_with(&mut self, id: Option<&str>) -> i32 {
        self.bot_et_with_all(id.into_iter().map(String::from).collect())
    }

    fn bot_et_with_all(&mut self, ids: Vec<String>) -> i32 {
        let Some(cur) = self.state.cur else { return i32::MAX };
        let was = self.state.cars[cur].clone();
        for id in &ids {
            let slot = self.part(id).map(|p| p.slot.clone()).unwrap_or_default();
            self.state.cars[cur].fitted.insert(slot, id.to_string());
            self.state.cars[cur].untuned = false;
        }
        let et = if self.wont_run().is_some() { i32::MAX } else { self.dyno().map_or(i32::MAX, |d| if d.et > 0 { d.et } else { i32::MAX }) };
        self.state.cars[cur] = was;
        et
    }

    /// A part and, for each thing it needs that the car does not give, the cheapest part that does.
    fn bot_bundle(&self, id: &str) -> Option<Vec<String>> {
        let p = self.part(id)?;
        let o = self.owned()?;
        let mut out = vec![id.to_string()];
        for need in &p.needs {
            let given = o.fitted.iter().filter(|(slot, _)| **slot != p.slot).filter_map(|(_, x)| self.part(x)).any(|x| x.gives.contains(need));
            if !given {
                let (k, _) = self.shop().into_iter().filter(|(_, q)| q.gives.contains(need) && q.goes_on(&o.car)).min_by_key(|(_, q)| q.price)?;
                out.push(k.clone());
            }
        }
        Some(out)
    }

    /// The upgrade that takes most off the time for the money, if any can be paid for.
    fn bot_best_upgrade(&mut self) -> Option<(String, i32, i32)> {
        let now = self.bot_et_with(None);
        let car = self.owned()?.car.clone();
        let cash = self.state.cash;
        let tune = self.pack.manifest.tuneup.cost;
        let mut best: Option<(String, i32, i32, f64)> = None;
        let shop: Vec<(String, Part)> = self.shop().into_iter().map(|(k, p)| (k.clone(), p.clone())).collect();
        for (id, p) in shop {
            let owned = self.state.spares.iter().chain(self.owned().map(|o| o.fitted.values()).into_iter().flatten()).any(|x| base(x) == id);
            let Some(bundle) = self.bot_bundle(&id) else { continue };
            let cost = bundle.iter().filter_map(|x| self.part(x)).map(|q| q.price).sum::<i32>() + if p.retune { tune } else { 0 };
            if !p.goes_on(&car) || cost + 60 > cash || owned {
                continue;
            }
            let et = self.bot_et_with_all(bundle);
            if et >= now - 20 {
                continue;
            }
            let per = (now - et) as f64 / cost as f64;
            if best.as_ref().is_none_or(|b| per > b.3) {
                best = Some((id, et, cost, per));
            }
        }
        best.map(|(id, et, cost, _)| (id, et, cost))
    }

    /// A pass, driven: staged, a reaction late off the green, shifted near the redline, and the
    /// lane held looking at where the car was `lag` ticks ago. What it came to.
    fn bot_drive(&mut self, react: i32, lag: usize) -> (bool, i32, i8, bool) {
        let mut seen: Vec<f64> = Vec::new();
        let mut last = 0u32;
        let mut result = (false, 0, 0, false);
        // Hands off everything first: a key still down from before is not a press.
        self.tick(0, 0, 0);
        for _ in 0..4000 {
            if self.scene != Scene::Race {
                break;
            }
            let Some(r) = self.race.as_ref() else { break };
            seen.push(r.drift);
            let drift = seen[seen.len().saturating_sub(lag + 1)];
            let mut keys;
            match r.phase {
                Phase::Stage => keys = if last & KEY_A == 0 { KEY_A } else { 0 },
                Phase::Tree | Phase::Run if r.manual() && (r.gear == 0 || r.held) => {
                    // Clutch in, first, the revs fed to its best launch speed; let out a reaction after the green.
                    let go = r.tree >= crate::race::TREE_STEP * 3 + react;
                    keys = if r.rpm < r.best_launch() { KEY_UP } else { 0 };
                    if !go {
                        keys |= KEY_DOWN;
                    }
                    if r.gear == 0 && last & KEY_X == 0 {
                        keys |= KEY_X | KEY_DOWN;
                    }
                }
                Phase::Tree | Phase::Run => {
                    keys = KEY_UP;
                    let launch = r.gear == 0 && r.tree >= crate::race::TREE_STEP * 3 + react;
                    let shift = r.gear > 0 && r.rpm > r.redline() - 250.0;
                    if (launch || shift) && last & KEY_X == 0 {
                        keys |= KEY_X;
                    }
                    if drift > 0.15 { keys |= KEY_LEFT } else if drift < -0.15 { keys |= KEY_RIGHT }
                }
                Phase::Slip => {
                    result = (r.won, r.et, r.out, r.blown);
                    keys = if last & KEY_A == 0 { KEY_A } else { 0 };
                }
            }
            last = keys;
            self.tick(keys, 0, 0);
        }
        result
    }

    /// One season, played.
    pub fn bot_season(&mut self, react: i32, lag: usize) -> Season {
        let mut s = Season { rungs: vec![1], ..Season::default() };
        let days = self.pack.manifest.season.days;
        let stakes = self.stakes();
        let mut was = (self.state.hours, String::from("start"));
        for _ in 0..5000 {
            *s.time.entry(was.1.clone()).or_default() += self.state.hours - was.0;
            was = (self.state.hours, "upgrading, buying, working".into());
            if let Scene::Over(n) = self.scene {
                s.end = match n { 1 => "the season ran out", 2 => "broke", 3 => "beat the King", _ => "over" }.into();
                break;
            }
            if self.day() > days {
                break;
            }
            self.scene = Scene::Place;
            self.note = None;
            if self.state.cur.is_none() {
                // No car: the cheapest in the paper, when it can be paid for.
                let cheapest = self.offers().into_iter().enumerate().filter_map(|(i, o)| Some((i, o?.price))).min_by_key(|x| x.1);
                match cheapest {
                    Some((i, price)) if price <= self.state.cash => {
                        self.buy_car(i);
                        if let Some(o) = self.owned() { s.cars.push(format!("{} at {}% for ${price}", o.car, o.cond)); }
                    }
                    _ => self.bot_work(&mut s),
                }
                continue;
            }
            let before = self.state.hours;
            let up = self.bot_keep_up(&mut s);
            *s.time.entry("keeping it running".into()).or_default() += self.state.hours - before;
            was.0 = self.state.hours;
            if !up {
                if !s.end.is_empty() {
                    break;
                }
                continue;
            }
            let top = self.pack.manifest.rivals.len() - 1;
            let rivals = self.pack.manifest.rivals.clone();
            // Its time driven well, and a reaction, against each of theirs.
            let mine = self.bot_et_with(None) + react * 33 + 150;
            let beats = |k: usize| mine <= (rivals[k].et * 1000.0) as i32 + 410;
            let rung = self.state.rung.min(top);
            let rung_now = self.state.rung;
            let stake_for = |k: usize, cash: i32, share: i32| { let limit = if k < rung_now { rivals[k].limit / 2 } else { rivals[k].limit }; stakes.iter().rposition(|&x| x <= limit && x * share <= cash).map_or(0, |b| b + 1) };
            // The way up, if it can win it and afford a stake; the King only for pink slips.
            let (who, b) = if beats(rung) {
                if rivals[rung].king { (rung, stakes.len() + 1) } else { (rung, stake_for(rung, self.state.cash, 4)) }
            } else {
                // Not yet: money, for the part or the car that would do it. Racing someone it has
                // beaten, if that pays better than a job.
                let day = self.day();
                let paid = |k: usize, g: &Game| g.state.paid_up.0 == day && g.state.paid_up.1.contains(&k);
                let word_out = self.state.paid_up.0 == day && self.state.paid_up.1.len() >= PAID_A_DAY;
                let farm = (0..rung).rev().find(|&k| !word_out && beats(k) && !rivals[k].king && !paid(k, self)).map(|k| (k, stake_for(k, self.state.cash, 2)));
                let job = self.jobs().iter().map(|j| j.pay as f64 / j.hours as f64).fold(0.0, f64::max);
                let upgrade = self.bot_best_upgrade();
                // A tired engine rebuilt is power too.
                let mut rb = self.pack.manifest.rebuild.clone();
                rb.cost = self.rebuild_cost();
                if self.owned().is_some_and(|o| o.cond + 10 < rb.cond) && self.state.cash >= rb.cost + 60 {
                    let now = self.bot_et_with(None);
                    let cur = self.state.cur.unwrap();
                    let was = self.state.cars[cur].cond;
                    self.state.cars[cur].cond = rb.cond;
                    let then = self.bot_et_with(None);
                    self.state.cars[cur].cond = was;
                    let per = (now - then) as f64 / rb.cost as f64;
                    let other = upgrade.as_ref().map_or(0.0, |u| (now - u.1) as f64 / u.2 as f64);
                    if per > other {
                        s.rebuilds += 1;
                        s.upgrades += rb.cost;
                        self.act("rebuild");
                        continue;
                    }
                }
                if let Some((id, _, _)) = upgrade.as_ref().filter(|u| u.2 + 60 <= self.state.cash) {
                    let bundle = self.bot_bundle(id).unwrap_or_default();
                    // What it needs goes on first, then the part.
                    for x in bundle.iter().rev() {
                        s.upgraded.push(x.clone());
                        self.bot_buy_fit(x, &mut s, true);
                    }
                    continue;
                }
                if upgrade.is_none() && !self.bot_best_upgrade_any() && self.bot_better_car(&mut s) {
                    continue;
                }
                match farm {
                    Some((k, b)) if b > 0 && stakes[b - 1] as f64 / 2.0 > job => { s.farmed += 1; (k, b) }
                    _ => { self.bot_work(&mut s); continue; }
                }
            };
            if b == 0 {
                self.bot_work(&mut s);
                continue;
            }
            self.who = who;
            let stake = if b > stakes.len() { 0 } else { stakes[b - 1] };
            let rung = self.state.rung;
            was.1 = if who == self.state.rung { "racing up".into() } else { "racing for money".into() };
            self.offer_bet(b);
            if self.scene != Scene::Race {
                self.bot_work(&mut s);
                continue;
            }
            let (won, _et, out, blown) = self.bot_drive(react, lag);
            s.races += 1;
            if won { s.wins += 1; s.won_money += stake } else { s.lost_money += stake }
            if out != 0 { s.out += 1 }
            if blown { s.blown += 1 }
            if let Some(n) = self.note.as_deref().filter(|n| n.ends_with("let go.")) {
                *s.broke.entry(n.to_string()).or_default() += 1;
            }
            if self.state.rung > rung {
                s.rungs.push(self.day());
            }
        }
        if s.end.is_empty() {
            s.end = "the season ran out".into();
        }
        s.day = self.day();
        s.rung = self.state.rung;
        s.cash = self.state.cash;
        s
    }

    /// A quicker kind of car than this one, from the paper, when it can be paid for with what this
    /// one fetches: bought, and this one sold to the dealer. True if it changed cars.
    fn bot_better_car(&mut self, s: &mut Season) -> bool {
        let Some(cur) = self.state.cur else { return false };
        let mine = self.state.cars[cur].car.clone();
        let price_of = |g: &Game, id: &str| g.pack.manifest.cars.get(id).and_then(|c| c.price).unwrap_or(0);
        let here = price_of(self, &mine);
        let worth = self.value(&self.state.cars[cur]);
        let pick = self.offers().into_iter().enumerate().filter_map(|(i, o)| Some((i, o?))).filter(|(_, o)| price_of(self, &o.car) > here && o.price <= self.state.cash + worth).min_by_key(|(_, o)| o.price);
        let Some((i, offer)) = pick else { return false };
        // Sell this one first if the money is needed, then buy.
        let old = self.state.cars.remove(cur);
        self.state.cash += self.value(&old);
        self.state.cur = None;
        self.buy_car(i);
        s.cars.push(offer.car.clone());
        self.state.cur.is_some()
    }

    /// Whether any part at all, at any price, would make it quicker.
    fn bot_best_upgrade_any(&mut self) -> bool {
        let cash = self.state.cash;
        self.state.cash = 1_000_000;
        let any = self.bot_best_upgrade().is_some();
        self.state.cash = cash;
        any
    }
}

#[test]
#[ignore]
fn seasons() {
    let (react, lag) = (6, 5);
    let mut all = Vec::new();
    for seed in 1..=12u32 {
        let mut g = default_game(seed * 7919);
        let s = g.bot_season(react, lag);
        println!(
            "seed {seed:2}: {} on day {} at rung {} with ${}; {} races ({} for money) {} won, {} over the line, {} blown; won ${} lost ${}; repairs ${} upgrades ${} worked {}h; tunes {} rebuilds {}; cars {:?}",
            s.end, s.day, s.rung, s.cash, s.races, s.farmed, s.wins, s.out, s.blown, s.won_money, s.lost_money, s.repairs, s.upgrades, s.worked, s.tunes, s.rebuilds, s.cars
        );
        println!("         rungs on days {:?}", s.rungs);
        let mut t: Vec<_> = s.time.iter().filter(|x| *x.1 > 0).collect();
        t.sort_by_key(|x| -x.1);
        println!("         hours {:?}", &t[..t.len().min(8)]);
        println!("         replaced {:?}; let go {:?}", s.worn, s.broke);
        let held = g.owned().map(|o| o.fitted.values().filter_map(|id| Some((g.part(id)?.name.clone(), g.part(id)?.holds?))).fold((String::new(), f64::MAX), |m, x| if x.1 < m.1 { x } else { m }));
        println!("         peak torque {:.0}, weakest {:?}", g.peak_torque(), held);
        println!("         bought {:?}", s.upgraded);
        all.push(s);
    }
    let n = all.len() as f64;
    println!("mean rung {:.1}, mean races {:.0}, win rate {:.0}%", all.iter().map(|s| s.rung as f64).sum::<f64>() / n, all.iter().map(|s| s.races as f64).sum::<f64>() / n,
        100.0 * all.iter().map(|s| s.wins as f64).sum::<f64>() / all.iter().map(|s| s.races as f64).sum::<f64>().max(1.0));
}

#[test]
#[ignore]
fn parts_cars() {
    // A year of the paper's cars sold as they stand: what each costs home, against what it brings
    // parted out (the shell and every part still good, at the bench's prices), as scrap, and whole.
    let mut g = default_game(11);
    let (mut rows, mut n) = (Vec::new(), 0);
    for day in 0..120 {
        g.state.hours = day * g.pack.manifest.season.hours_per_day;
        for p in g.projects().into_iter().flatten() {
            n += 1;
            let cost = p.price + p.ship;
            let o = &p.owned;
            let parts: i32 = o.fitted.values().filter(|id| cond_of(id) > 0).map(|id| g.spare_offer(id)).sum();
            let shell = g.shell_offer(o).unwrap_or(0);
            let scrap = g.scrap_offer(o);
            let whole = g.value(o);
            rows.push((cost, shell + parts, scrap, whole, g.wont_run_of(o).unwrap_or_default()));
        }
    }
    let profit: Vec<i32> = rows.iter().map(|r| r.1 - r.0).collect();
    let scrap: Vec<i32> = rows.iter().map(|r| r.2 - r.0).collect();
    let mean = |v: &[i32]| v.iter().sum::<i32>() as f64 / v.len().max(1) as f64;
    println!("{n} parts cars: cost home mean ${:.0}", mean(&rows.iter().map(|r| r.0).collect::<Vec<_>>()));
    println!("  parted out: mean profit ${:.0}, {} of {} lose money, best ${}, worst ${}", mean(&profit), profit.iter().filter(|&&p| p < 0).count(), profit.len(), profit.iter().max().unwrap(), profit.iter().min().unwrap());
    println!("  scrapped:   mean profit ${:.0}, {} of {} lose money", mean(&scrap), scrap.iter().filter(|&&p| p < 0).count(), scrap.len());
    println!("  whole value against cost: mean ${:.0}", mean(&rows.iter().map(|r| r.3 - r.0).collect::<Vec<_>>()));
    for r in rows.iter().take(8) {
        println!("    cost ${:4}  parted ${:5}  scrap ${:4}  whole ${:5}  {}", r.0, r.1, r.2, r.3, r.4);
    }
}

#[test]
#[ignore]
fn torque_steer_by_hand() {
    // How often a person holding the lane by eye (a sixth of a second behind) goes over a line.
    for (car, name, lag) in [("umbra-base", "front", 5), ("umbra", "all", 5), ("umbra-base", "front", 10), ("umbra", "all", 10)] {
        let mut outs = 0;
        let runs = 40;
        for seed in 0..runs {
            let mut g = default_game(seed * 31 + 1);
            let mut o = Owned::new(car, None, 90);
            g.outfit(&mut o);
            g.state.cars = vec![o];
            g.state.cur = Some(0);
            g.state.hours = seed as i32 * 12;
            g.who = 0;
            g.offer_bet(1);
            let (_, _, out, _) = g.bot_drive(6, lag);
            if out != 0 { outs += 1 }
        }
        println!("{name}-drive, eyes {lag} ticks behind: {outs} of {runs} over a line");
    }
}

