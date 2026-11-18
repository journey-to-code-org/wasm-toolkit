//! The engine, given packs made here and the default pack: reading them, saying what is wrong with
//! a bad one, and playing a season with the keys.

use engine::game::{Game, Loose, Owned, Scene, KEY_A, KEY_B, KEY_UP, KEY_X};
use engine::gfx::{ramp, shade, Picture, Screen, CLEAR};
use engine::pack::{Boost, Manifest, Pack};
use engine::race::{Controls, Phase, Race};

/// A pack file from named pieces, in the format `tools/pack.py` writes.
fn pack(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut out = b"WPK1".to_vec();
    out.extend((files.len() as u32).to_le_bytes());
    for (name, data) in files {
        out.extend((name.len() as u16).to_le_bytes());
        out.extend(name.as_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
    }
    out
}

/// A picture, as the bytes a pack holds.
fn pic(w: u16, h: u16, runs: &[(u16, u16)]) -> Vec<u8> {
    let mut words = vec![w, h];
    for &(n, c) in runs {
        words.push(n);
        words.push(c);
    }
    words.iter().flat_map(|v| v.to_le_bytes()).collect()
}

const MANIFEST: &str = r#"{
  "name": "Test Pack",
  "start": { "cash": 1000, "place": "shed", "car": "box" },
  "season": { "days": 3, "hours_per_day": 4 },
  "cars": { "box": { "name": "Box", "side": "box.pic", "side_lamps": "box-lit.pic", "ground": 2, "price": 350,
                     "paint": "RED", "paint_ramps": { "mid": "BLUE" } } },
  "parts": { "pipe": { "name": "Loud pipe", "slot": "Exhaust", "price": 100, "pitch": "Loud", "torque": 10 },
             "puffer": { "name": "Puffer", "slot": "Turbo", "price": 300, "boost": { "from": 3000, "psi": 27, "lag": 10 } } },
  "rivals": [ { "name": "Slowpoke", "says": "Hi.", "car": "box", "paint": "GREEN", "et": 30, "limit": 25 },
              { "name": "Boss", "says": "Cars only.", "car": "box", "paint": "GOLD", "et": 29, "king": true } ],
  "bets": [10, 25],
  "places": {
    "shed": { "title": "Shed", "picture": "shed.pic", "car_at": [10, 50],
              "spots": [ { "rect": [0, 0, 40, 20], "label": "Out", "do": "go:road" },
                         { "rect": [40, 0, 40, 20], "label": "Lamps", "do": "lamps", "needs_car": true },
                         { "rect": [80, 0, 40, 20], "label": "Sign", "do": "say:Hello." },
                         { "rect": [120, 0, 40, 20], "label": "Paper", "do": "paper" },
                         { "rect": [160, 0, 40, 20], "label": "Bench", "do": "parts" },
                         { "rect": [200, 0, 40, 20], "label": "Yard", "do": "yard" },
                         { "rect": [240, 0, 40, 20], "label": "Calendar", "do": "calendar" } ] },
    "road": { "title": "Road", "backdrop": "GRASS", "car_at": [10, 100], "rival_at": [10, 150],
              "spots": [ { "rect": [0, 0, 100, 20], "label": "Back", "do": "go:shed" },
                         { "rect": [100, 0, 100, 20], "label": "Race", "do": "race", "button": "Race" },
                         { "rect": [200, 0, 100, 20], "label": "Solo", "do": "solo" } ] }
  }
}"#;

fn files(manifest: &str) -> Vec<u8> {
    let blue = shade(ramp("BLUE"), 20);
    pack(&[
        ("pack.json", manifest.as_bytes().to_vec()),
        ("shed.pic", pic(320, 200, &[(64000, shade(ramp("WOOD"), 12))])),
        ("box.pic", pic(4, 2, &[(4, blue), (2, CLEAR), (2, blue)])),
        ("box-lit.pic", pic(4, 2, &[(8, shade(ramp("YELLOW"), 24))])),
    ])
}

fn game() -> Game {
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(MANIFEST)));
    g.seed(7);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    g
}

fn click(g: &mut Game, x: i32, y: i32) {
    g.tick(0, x, y);
    g.tick(KEY_A, x, y);
    g.tick(0, x, y);
}

/// Drive the race that is on: on the limiter, away a fifth of a second after the green, shifting
/// short of the redline. Then Enter past the timeslip.
fn drive(g: &mut Game) {
    g.tick(KEY_A, 0, 0);
    for _ in 0..4000 {
        let Some(r) = &g.race else { break };
        if r.phase == Phase::Slip {
            break;
        }
        let up = (r.gear == 0 && r.tree >= 51) || (r.gear > 0 && r.rpm > r.redline() - 250.0);
        g.tick(KEY_UP, 0, 0);
        if up {
            g.tick(KEY_UP | KEY_X, 0, 0);
        }
    }
    g.tick(0, 0, 0);
    g.tick(KEY_A, 0, 0);
    g.tick(0, 0, 0);
}

#[test]
fn a_good_pack_is_read() {
    let p = Pack::parse(&files(MANIFEST)).expect("the pack should read");
    let m = &p.manifest;
    assert_eq!((m.name.as_str(), m.start.cash, m.season.days), ("Test Pack", 1000, 3));
    assert_eq!(m.cars["box"].ground, 2);
    assert_eq!(m.cars["box"].drive.gears.len(), 5, "a car with no drive gets the usual one");
    assert_eq!(m.parts["puffer"].boost, Some(Boost { from: 3000.0, psi: 27.0, lag: 10.0 }));
    assert!(m.rivals[1].king && m.places["road"].rival_at.is_some());
    assert_eq!(p.pictures["box.pic"].w, 4);
}

#[test]
fn a_bad_pack_says_what_is_wrong() {
    let err = |bytes: Vec<u8>| Pack::parse(&bytes).err().expect("this pack should be refused");
    assert!(err(b"nope".to_vec()).contains("WPK1"));
    assert!(err(pack(&[("readme.txt", vec![1])])).contains("pack.json"));
    assert!(err(pack(&[("pack.json", b"{ not json".to_vec())])).contains("pack.json"));
    let whole = files(MANIFEST);
    assert!(err(whole[..whole.len() - 10].to_vec()).contains("ends early"));
    let one = [("shed.pic", pic(1, 1, &[(1, 5)])), ("box.pic", pic(1, 1, &[(1, 5)]))];
    let mut some = vec![("pack.json", MANIFEST.as_bytes().to_vec())];
    some.extend(one.iter().cloned());
    assert!(err(pack(&some)).contains("box-lit.pic"), "a picture the pack does not hold");
    some.push(("box-lit.pic", pic(4, 2, &[(3, 5)])));
    assert!(err(pack(&some)).contains("not a picture"), "runs that do not add up");
    for (from, to, says) in [
        ("go:road", "go:moon", "moon"),
        ("\"do\": \"paper\"", "\"do\": \"fly\"", "fly"),
        ("\"paint\": \"RED\"", "\"paint\": \"PUCE\"", "PUCE"),
        ("\"car\": \"box\", \"paint\": \"GREEN\"", "\"car\": \"van\", \"paint\": \"GREEN\"", "van"),
        ("\"et\": 30", "\"et\": 0", "quarter mile"),
        ("\"rival_at\": [10, 150],", "", "rival_at"),
        ("\"et\": 30, \"limit\": 25", "\"et\": 30, \"king\": true", "last"),
        ("\"days\": 3", "\"days\": 0", "season"),
    ] {
        assert!(MANIFEST.contains(from), "{from}");
        let e = err(files(&MANIFEST.replace(from, to)));
        assert!(e.contains(says), "{to}: {e}");
    }
}

#[test]
fn a_place_is_what_the_pack_says() {
    let mut g = game();
    let mut s = Screen::new();
    assert_eq!((g.scene, g.state.place.as_str(), g.state.cash), (Scene::Place, "shed", 1000));
    assert_eq!(g.owned().map(|o| o.car.as_str()), Some("box"));

    // The room is the pack's picture, and the car stands in it in its own paint, not the picture's.
    g.draw(&mut s);
    assert_eq!(s.at(300, 100), shade(ramp("WOOD"), 12));
    assert_eq!(s.at(10, 48), shade(ramp("RED"), 20));
    assert_eq!(s.at(10, 49), shade(ramp("WOOD"), 12), "clear pixels of the car show the room");

    click(&mut g, 100, 10);
    assert_eq!(g.note.as_deref(), Some("Hello."));
    click(&mut g, 60, 10);
    assert!(g.lamps);
    g.draw(&mut s);
    assert_eq!(s.at(10, 48), shade(ramp("YELLOW"), 24));
    click(&mut g, 260, 10);
    assert_eq!(g.note.as_deref(), Some("Day 1 of 3. 4 hours left today."));

    // Out on the road the first rival has pulled in, and the spot marked as a button is drawn.
    click(&mut g, 20, 10);
    assert_eq!((g.state.place.as_str(), g.rival().map(|r| r.name.as_str())), ("road", Some("Slowpoke")));
    g.draw(&mut s);
    assert_eq!(s.at(10, 148), shade(ramp("YELLOW"), 24), "he stands there with his lamps on");
    assert_ne!(s.at(150, 2) >> 5, ramp("GRASS") as u16, "the button is drawn over the backdrop");
    assert_eq!(s.at(310, 100) >> 5, ramp("GRASS") as u16);

    // Escape is the title, and Enter there carries on where you were.
    g.tick(KEY_B, 0, 0);
    g.tick(0, 0, 0);
    assert_eq!(g.scene, Scene::Title);
    g.tick(KEY_A, 0, 0);
    assert_eq!((g.scene, g.state.place.as_str()), (Scene::Place, "road"));
}

#[test]
fn the_paper_sells_cars_and_parts() {
    let mut g = game();
    let mut s = Screen::new();
    click(&mut g, 140, 10);
    assert_eq!((g.scene, g.state.hours), (Scene::Paper(0), 1), "reading the paper takes an hour");
    let offers = g.offers();
    assert_eq!(offers.len(), 3);
    let first = offers[0].as_ref().unwrap();
    assert!((55..=80).contains(&first.cond) && first.price >= 350 * 55 / 70 && first.price <= 350 * 80 / 70 + 40);
    let price = first.price;
    g.draw(&mut s);
    click(&mut g, 100, 70);
    assert_eq!((g.state.cars.len(), g.state.cash, g.state.cur), (2, 1000 - price, Some(0)), "the new one goes in the yard");
    assert!(g.offers()[0].is_none(), "and its ad is gone");
    click(&mut g, 100, 70);
    assert_eq!(g.state.cars.len(), 2, "a sold ad sells nothing");

    // The parts page.
    click(&mut g, 280, 40);
    assert_eq!(g.scene, Scene::Paper(1));
    g.draw(&mut s);
    click(&mut g, 100, 60);
    assert_eq!(g.state.spares, vec!["pipe".to_string()]);
    let cash = g.state.cash;
    click(&mut g, 100, 60);
    assert_eq!((g.state.cash, g.note.as_deref()), (cash, Some("You have one.")));
    g.state.cash = 50;
    click(&mut g, 100, 70);
    assert_eq!((g.state.spares.len(), g.note.as_deref()), (1, Some("You cannot afford it.")));
    g.tick(KEY_B, 0, 0);
    assert_eq!(g.scene, Scene::Place);
}

#[test]
fn the_bench_bolts_parts_on_and_the_yard_swaps_cars() {
    let mut g = game();
    let mut s = Screen::new();
    g.state.spares = vec!["pipe".into(), "puffer".into()];
    let stock = g.tune(g.owned().unwrap());
    assert_eq!(stock, (0.7, None));
    click(&mut g, 180, 10);
    assert_eq!(g.scene, Scene::Bench);
    assert_eq!(g.slots(), vec!["Exhaust".to_string(), "Turbo".to_string()]);
    g.draw(&mut s);
    click(&mut g, 200, 36);
    assert_eq!((g.state.spares.len(), g.state.hours), (1, 1), "bolting one on takes an hour");
    let (gain, boost) = g.tune(g.owned().unwrap());
    assert!((gain - 0.77).abs() < 1e-9 && boost.is_none());
    click(&mut g, 200, 36);
    assert_eq!(g.tune(g.owned().unwrap()).1.map(|b| b.psi), Some(27.0));
    // Off again.
    click(&mut g, 50, 36);
    assert_eq!((g.state.spares.clone(), g.state.hours), (vec!["pipe".to_string()], 3));
    // Selling what is on the shelf.
    let offer = g.spare_offer("pipe");
    assert!((70..90).contains(&offer));
    click(&mut g, 200, 186);
    click(&mut g, 200, 36);
    assert_eq!((g.state.spares.len(), g.state.cash), (0, 1000 + offer));
    g.tick(KEY_B, 0, 0);

    // The yard: another car, brought into the bay, and the first one sold with its turbo on it.
    g.state.cars.push(Owned::new("box", Some("GOLD".into()), 80));
    click(&mut g, 220, 10);
    assert_eq!(g.scene, Scene::Yard);
    g.draw(&mut s);
    click(&mut g, 100, 46);
    assert_eq!(g.state.cur, Some(1));
    let (cash, worth) = (g.state.cash, g.value(&g.state.cars[0]));
    assert_eq!(worth, 350 * 6 / 10 + 150);
    click(&mut g, 280, 36);
    assert_eq!((g.state.cars.len(), g.state.cur, g.state.cash), (1, Some(0), cash + worth));
}

#[test]
fn under_the_hood_parts_come_off_bolt_by_bolt() {
    engine::host::store(b"");
    let with_hood = MANIFEST
        .replace("\"ground\": 2,", "\"ground\": 2, \"hood\": { \"picture\": \"shed.pic\", \"slots\": { \"Exhaust\": [100, 50, 40, 40] } },")
        .replace("\"do\": \"say:Hello.\"", "\"do\": \"hood\"");
    let mut g = Game::new(Pack::parse(&files(&with_hood)));
    let mut s = Screen::new();
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    g.state.spares = vec!["pipe".into(), "puffer".into()];
    click(&mut g, 100, 10);
    assert_eq!(g.scene, Scene::Hood);
    g.draw(&mut s);
    assert!(g.floor() == vec![Loose::Part("pipe".into())], "the turbo has no place under this hood, so it is not on the floor");

    // It will not lift with a bolt in it.
    click(&mut g, 120, 70);
    assert_eq!(g.note.as_deref(), Some("It is bolted on. Undo the bolts first."));
    let bolts = [(105, 55), (134, 55), (134, 84), (105, 84)];
    for (i, (x, y)) in bolts.iter().enumerate() {
        click(&mut g, *x, *y);
        assert_eq!(g.owned().unwrap().bolts["Exhaust"].count_ones(), 3 - i as u32);
    }
    assert_eq!(g.wont_run().as_deref(), Some("The exhaust is not bolted down."));
    click(&mut g, 120, 70);
    assert_eq!((g.owned().unwrap().off.len(), g.state.hours), (1, 1), "lifting it off is an hour's work");
    assert_eq!(g.wont_run().as_deref(), Some("The exhaust is off. It will not run."));
    assert_eq!(g.floor(), vec![Loose::Stock("Exhaust".into()), Loose::Part("pipe".into())]);
    g.draw(&mut s);

    // The loud pipe off the floor, onto the empty place, and done up.
    click(&mut g, 90, 182);
    assert_eq!(g.carrying, Some(Loose::Part("pipe".into())));
    g.draw(&mut s);
    click(&mut g, 120, 70);
    assert_eq!((g.carrying.clone(), g.owned().unwrap().fitted.get("Exhaust").map(|p| p.as_str())), (None, Some("pipe")));
    assert_eq!(g.state.spares, vec!["puffer".to_string()]);
    assert!(g.wont_run().is_some());
    for (x, y) in bolts {
        click(&mut g, x, y);
    }
    assert_eq!(g.wont_run(), None);
    assert!((g.tune(g.owned().unwrap()).0 - 0.77).abs() < 1e-9);
    assert_eq!(g.floor(), vec![Loose::Stock("Exhaust".into())], "the stock one stays on the floor");

    // A loose bolt, and nobody is racing anybody. The bench will not do the hood's work either.
    click(&mut g, 105, 55);
    g.tick(KEY_B, 0, 0);
    click(&mut g, 20, 10);
    click(&mut g, 150, 10);
    assert_eq!((g.scene, g.note.as_deref()), (Scene::Place, Some("The exhaust is not bolted down.")));
    click(&mut g, 20, 10);
    click(&mut g, 180, 10);
    click(&mut g, 50, 36);
    assert_eq!(g.note.as_deref(), Some("That one is under the hood."));
}

#[test]
fn bets_are_made_and_settled() {
    let mut g = game();
    let mut s = Screen::new();
    click(&mut g, 20, 10);
    click(&mut g, 150, 10);
    assert_eq!(g.scene, Scene::Bets);
    g.draw(&mut s);
    // Kicks, $10, $25, pinks: four buttons along the bottom. He will not race for his car.
    click(&mut g, 4 + 78 * 3 + 30, 185);
    assert_eq!((g.scene, g.note.as_deref()), (Scene::Place, Some("No way. Too rich for me.")));
    click(&mut g, 150, 10);
    click(&mut g, 4 + 78 * 2 + 30, 185);
    assert_eq!(g.scene, Scene::Race);
    g.draw(&mut s);
    drive(&mut g);
    assert_eq!((g.scene, g.state.cash, g.state.rung, g.state.hours), (Scene::Place, 1025, 1, 2), "{:?}", g.note);
    assert_eq!(g.rival().map(|r| r.name.as_str()), Some("Boss"), "beating one for money brings the next");

    // The Boss races for cars and nothing else. Backing out of a race is losing it.
    click(&mut g, 150, 10);
    click(&mut g, 30, 185);
    assert_eq!(g.note.as_deref(), Some("Pink slips or go home."));
    click(&mut g, 150, 10);
    click(&mut g, 4 + 78 * 3 + 30, 185);
    assert_eq!(g.scene, Scene::Race);
    g.tick(KEY_B, 0, 0);
    assert_eq!((g.state.cars.len(), g.state.place.as_str(), g.state.hours), (0, "shed", 4), "he took the car and you walked home");
    assert_eq!(g.scene, Scene::Place, "with $1025 there is still a car to be bought");

    // With a car again, beating him for his car wins the game.
    g.tick(0, 0, 0);
    g.state.cars.push(Owned::new("box", None, 100));
    g.state.cur = Some(0);
    click(&mut g, 20, 10);
    click(&mut g, 150, 10);
    click(&mut g, 4 + 78 * 3 + 30, 185);
    drive(&mut g);
    assert_eq!(g.scene, Scene::Over(3));
    g.draw(&mut s);
}

#[test]
fn the_season_ends_and_so_does_the_money() {
    // Three days of four hours: a solo pass is an hour.
    let mut g = game();
    click(&mut g, 20, 10);
    for _ in 0..12 {
        assert_eq!(g.scene, Scene::Place);
        click(&mut g, 250, 10);
        g.tick(KEY_B, 0, 0);
        g.tick(0, 0, 0);
    }
    assert_eq!(g.scene, Scene::Over(1));
    g.tick(KEY_A, 0, 0);
    assert_eq!(g.scene, Scene::Title);

    // No car and no way to buy one.
    let mut g = game();
    g.state.cash = 100;
    click(&mut g, 220, 10);
    click(&mut g, 280, 36);
    assert_eq!(g.state.cash, 100 + 350 * 6 / 10, "the dealer's money is enough for another");
    assert_eq!(g.scene, Scene::Yard);
    g.state.cash = 100;
    g.tick(KEY_B, 0, 0);
    click(&mut g, 140, 10);
    assert_eq!(g.scene, Scene::Over(2));
}

#[test]
fn a_game_is_kept() {
    let mut g = game();
    g.state.cash = 4321;
    click(&mut g, 20, 10);
    let mut again = Game::new(Pack::parse(&files(MANIFEST)));
    again.tick(KEY_A, 0, 0);
    assert_eq!((again.state.cash, again.state.place.as_str()), (4321, "road"), "Enter carries on");
    let mut fresh = Game::new(Pack::parse(&files(MANIFEST)));
    fresh.tick(KEY_X, 0, 0);
    assert_eq!((fresh.state.cash, fresh.state.place.as_str()), (1000, "shed"), "E is a new game");
    // A game kept by another pack is not this one's.
    let other = Game::new(Pack::parse(&files(&MANIFEST.replace("Test Pack", "Other Pack"))));
    let mut s = Screen::new();
    other.draw(&mut s);
    let mut o = other;
    o.tick(KEY_A, 0, 0);
    assert_eq!(o.state.cash, 1000);
}

#[test]
fn a_pack_that_cannot_be_read_is_explained_on_the_screen() {
    let mut g = Game::new(Pack::parse(b"junk"));
    let mut s = Screen::new();
    g.tick(KEY_A, 0, 0);
    g.draw(&mut s);
    assert!(g.problem.is_some());
    assert!(s.fb.iter().any(|&c| c == 15), "there are words on it");
}

#[test]
fn pictures_must_add_up() {
    assert!(Picture::from_bytes(&pic(2, 2, &[(4, 7)])).is_some());
    assert!(Picture::from_bytes(&pic(2, 2, &[(5, 7)])).is_none());
    assert!(Picture::from_bytes(&[1, 0, 1]).is_none());
}

/// A tidy pass in one of the default pack's cars: its time in milliseconds.
fn pass(m: &Manifest, car: &str, gain: f64, boost: Option<Boost>) -> i32 {
    let drive = &m.cars[car].drive;
    let mut r = Race::new(drive, gain, boost, 0, 0);
    r.tick(Controls { stage: true, ..Controls::default() });
    for _ in 0..3000 {
        if r.phase == Phase::Slip {
            break;
        }
        let up = (r.gear == 0 && r.tree >= 51) || (r.gear > 0 && r.rpm > drive.redline - 250.0);
        r.tick(Controls { throttle: true, up, ..Controls::default() });
    }
    r.et
}

#[test]
fn the_default_pack_is_a_ladder_that_can_be_climbed() {
    let m: Manifest = serde_json::from_str(&std::fs::read_to_string("../content/default/pack.json").unwrap()).unwrap();
    let rival = |i: usize| (m.rivals[i].et * 1000.0) as i32;
    assert_eq!(m.rivals.len(), 22);
    assert!(m.rivals.windows(2).all(|w| w[0].et > w[1].et), "each is quicker than the last");
    // What you can afford on day one beats the first of them, and not the ones who run turbos.
    assert!(m.cars["umbra-base"].price.unwrap() * 80 / 70 + 40 <= m.start.cash && m.cars["umbra"].price.unwrap() * 55 / 70 > m.start.cash);
    let base = pass(&m, "umbra-base", 0.70, None);
    assert!(base < rival(0) && base > rival(7), "{base}");
    // A stock turbo car in fair shape is mid-field; the King takes a strong one with everything on it.
    let big = m.parts["big-turbo"].boost.clone();
    let all = 1.0 + m.parts.values().map(|p| p.torque).sum::<i32>() as f64 / 100.0;
    let stock = pass(&m, "umbra", 0.70, None);
    assert!(stock < rival(12) && stock > rival(15), "{stock}");
    assert!(pass(&m, "umbra", 0.80, None) > rival(21) + 540, "a stock one cannot beat the King however he leaves");
    assert!(pass(&m, "umbra", 0.80 * all, big) < rival(21), "a built one can");
    // A red light, and a missed race.
    let mut r = Race::new(&m.cars["umbra"].drive, 1.0, None, 12000, 300);
    r.tick(Controls { stage: true, ..Controls::default() });
    r.tick(Controls { throttle: true, up: true, ..Controls::default() });
    assert!(r.red);
    let mut r = Race::new(&m.cars["umbra"].drive, 1.0, None, 12000, 300);
    r.tick(Controls { stage: true, ..Controls::default() });
    for _ in 0..30 * 30 {
        r.tick(Controls::default());
    }
    assert!(r.phase == Phase::Slip && !r.won && r.et == 0, "he is long gone");
}
