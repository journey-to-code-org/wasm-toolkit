//! A player who presses anything: thousands of ticks of random keys and clicks in the default
//! pack, game after game, checking after every one that nothing has broken. Clicks go mostly
//! where there is something to click, so it gets everywhere a player would.

use std::collections::BTreeMap;

use engine::game::{Game, Loose, Scene};
use engine::pack::Pack;

const ENTER: u32 = 16;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n.max(1)
    }
}

/// Where there is likely something to click in a scene.
fn somewhere(g: &Game, r: &mut Rng) -> (i32, i32) {
    match g.scene {
        Scene::Place if r.below(4) > 0 => {
            let spots: Vec<[i32; 4]> = g.here().map(|p| p.spots.iter().map(|s| s.rect).collect()).unwrap_or_default();
            if spots.is_empty() {
                return (r.below(320) as i32, r.below(200) as i32);
            }
            let [x, y, w, h] = spots[r.below(spots.len() as u32) as usize];
            (x + r.below(w.max(1) as u32) as i32, y + r.below(h.max(1) as u32) as i32)
        }
        Scene::Hood if r.below(3) > 0 => {
            // A bolt, a part, a tab or something on the floor.
            let Some((_, _, _, slots)) = g.area_here() else { return (160, 100) };
            let all: Vec<_> = slots.values().collect();
            let s = all[r.below(all.len() as u32) as usize];
            match r.below(4) {
                0 if !s.bolts.is_empty() => {
                    let b = s.bolts[r.below(s.bolts.len() as u32) as usize];
                    (b[0], b[1])
                }
                1 => (r.below(320) as i32, 6),
                2 => (4 + r.below(316) as i32, 178 + r.below(22) as i32),
                _ => s.middle(),
            }
        }
        _ => (r.below(320) as i32, r.below(240) as i32),
    }
}

/// Everything that should always be so.
fn sound(g: &Game, last_hours: i32) -> Result<(), String> {
    let s = &g.state;
    if s.cur.is_some_and(|i| i >= s.cars.len()) {
        return Err(format!("the car in the bay is {:?} of {}", s.cur, s.cars.len()));
    }
    if s.hours < last_hours {
        return Err(format!("time went back from {last_hours} to {}", s.hours));
    }
    for o in &s.cars {
        if !o.gas.is_finite() || o.gas < -0.01 || !o.bottle.is_finite() || !o.tires.is_finite() {
            return Err(format!("{} has gas {} bottle {} tires {}", o.car, o.gas, o.bottle, o.tires));
        }
        for slot in &o.off {
            if o.fitted.contains_key(slot) {
                return Err(format!("{} has its {slot} both off and on", o.car));
            }
        }
    }
    if let Some(Loose::Part(id)) = &g.carrying {
        if !s.spares.contains(id) {
            return Err(format!("holding {id}, which is not on the shelf"));
        }
    }
    if g.scene == Scene::Hood {
        // Every part that is off can be put back: what goes there is within reach on the floor.
        let (o, car) = (g.owned().ok_or("under the hood with no car")?, g.car().ok_or("no car")?);
        let (_, _, _, slots) = g.area_here().ok_or("no place to work on")?;
        let floor = g.floor();
        for slot in o.off.iter().filter(|s| slots.contains_key(*s)) {
            let on_hand = s.spares.iter().any(|p| g.pack.manifest.parts.get(p.split('@').next().unwrap()).is_some_and(|q| &q.slot == slot && q.goes_on(&o.car)));
            let stock_loose = !(o.sheet && car.stock.contains_key(slot));
            let held = g.carrying.as_ref().is_some_and(|l| match l { Loose::Stock(x) => x == slot, Loose::Part(p) => g.pack.manifest.parts.get(p.split('@').next().unwrap()).is_some_and(|q| &q.slot == slot) });
            let reachable = held || floor.iter().any(|l| match l { Loose::Stock(x) => x == slot, Loose::Part(p) => g.pack.manifest.parts.get(p.split('@').next().unwrap()).is_some_and(|q| &q.slot == slot) });
            if (on_hand || stock_loose) && !reachable {
                return Err(format!("the {slot} is off and what goes there is not within reach (floor {})", floor.len()));
            }
        }
    }
    Ok(())
}

fn play(seed: u64, ticks: u32, rich: bool) -> BTreeMap<String, u32> {
    let bytes = std::fs::read("../web/default.pack").unwrap();
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&bytes));
    g.seed(seed as u32);
    let mut r = Rng(seed * 2654435761 + 1);
    let mut seen: BTreeMap<String, u32> = BTreeMap::new();
    let (mut held, mut hold_for, mut last_hours) = (0u32, 0u32, 0);
    let (mut mx, mut my) = (160, 100);
    for t in 0..ticks {
        if rich && t == 50 {
            g.state.cash += 20000;
        }
        // Keys held for a while, as a driver does; clicks now and then; Escape sometimes.
        if hold_for == 0 {
            held = match r.below(10) {
                0..=3 => 1 | [0, 4, 8][r.below(3) as usize],
                4 => 2,
                5 => 1 | 256,
                _ => 0,
            };
            hold_for = 5 + r.below(90);
        }
        hold_for -= 1;
        let mut keys = held;
        match r.below(40) {
            0..=4 => {
                (mx, my) = somewhere(&g, &mut r);
                g.tick(0, mx, my);
                keys = ENTER;
            }
            5 => keys |= 32,
            6 => keys |= 64,
            7 => keys |= 128,
            8 if r.below(10) == 0 => keys |= 512,
            _ => {}
        }
        let was_title = g.scene == Scene::Title;
        g.tick(keys, mx, my);
        if was_title {
            last_hours = g.state.hours.min(last_hours);
        }
        let mut s = engine::gfx::Screen::new();
        if t % 97 == 0 {
            g.draw(&mut s);
        }
        let name = format!("{:?}", g.scene).split('(').next().unwrap().to_string();
        *seen.entry(name).or_default() += 1;
        if let Err(why) = sound(&g, last_hours) {
            panic!("seed {seed}, tick {t}, in {:?}: {why}", g.scene);
        }
        last_hours = g.state.hours;
        if matches!(g.scene, Scene::Over(_)) {
            *seen.entry("games over".into()).or_default() += 1;
            // A new game, as a player would.
            g.tick(ENTER, 160, 100);
            g.tick(0, 160, 100);
            last_hours = g.state.hours;
        }
    }
    seen
}

#[test]
fn a_player_who_presses_anything_breaks_nothing() {
    if std::fs::metadata("../web/default.pack").is_err() {
        return;
    }
    let seeds: u64 = std::env::var("PLAYTEST_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(6);
    let ticks: u32 = std::env::var("PLAYTEST_TICKS").ok().and_then(|s| s.parse().ok()).unwrap_or(12000);
    let mut all: BTreeMap<String, u32> = BTreeMap::new();
    for seed in 1..=seeds {
        for (k, v) in play(seed, ticks, seed % 2 == 0) {
            *all.entry(k).or_default() += v;
        }
    }
    eprintln!("{all:?}");
}
