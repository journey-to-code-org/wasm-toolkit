//! The engine, given packs made here and the default pack: reading them, saying what is wrong with
//! a bad one, and playing a season with the keys.

use engine::game::{Game, Loose, Owned, Scene, KEY_A, KEY_B, KEY_N, KEY_UP, KEY_X};
use engine::gfx::{ramp, shade, Picture, Screen, CLEAR};
use engine::pack::{bolts_round, Boost, Manifest, Pack};
use engine::race::{Controls, Phase, Race};
use engine::town::{Pedals, Town};

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
    assert_eq!(g.note.as_deref(), Some("Day 1 of 3. Won 0, lost 0."));

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
fn a_slot_has_its_own_bolts_and_a_part_its_own_picture() {
    for n in 1..=8 {
        let at = bolts_round([10, 10, 60, 40], n);
        assert_eq!(at.len(), n);
        assert!(at.iter().all(|b| (10..70).contains(&b[0]) && (10..50).contains(&b[1])), "{n}: {at:?}");
        assert!((0..n).all(|i| (0..i).all(|j| at[i] != at[j])), "no two in one place");
    }
    assert_eq!((bolts_round([0, 0, 40, 40], 0).len(), bolts_round([0, 0, 40, 40], 20).len()), (1, 8));

    engine::host::store(b"");
    // Two places under this hood: the exhaust on two bolts, with a name for the stock one, and a
    // filter on one bolt that no part in the paper goes in. The loud pipe has a picture of itself.
    let hood = r#""ground": 2, "hood": { "picture": "shed.pic", "slots": {
        "Exhaust": { "rect": [100, 50, 40, 40], "bolts": 2, "stock": "Cast manifold" },
        "Filter": { "rect": [200, 50, 30, 30], "bolts": [[190, 45]] } } },"#;
    let manifest = MANIFEST.replace("\"ground\": 2,", hood).replace("\"do\": \"say:Hello.\"", "\"do\": \"hood\"").replace("\"pitch\": \"Loud\",", "\"pitch\": \"Loud\", \"picture\": \"box-lit.pic\",");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    let mut s = Screen::new();
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    g.state.spares = vec!["pipe".into()];
    let slots = &g.car().unwrap().hood.as_ref().unwrap().slots;
    assert_eq!((slots["Exhaust"].bolts.clone(), slots["Filter"].bolts.clone()), (vec![[105, 55], [134, 84]], vec![[190, 45]]));
    click(&mut g, 100, 10);

    // The filter comes off with its one bolt, which sits outside its box, and leaves the car unable to run.
    click(&mut g, 190, 45);
    click(&mut g, 215, 65);
    assert_eq!(g.floor(), vec![Loose::Stock("Filter".into()), Loose::Part("pipe".into())]);
    assert_eq!(g.wont_run().as_deref(), Some("The filter is off. It will not run."));
    click(&mut g, 20, 182);
    click(&mut g, 215, 65);
    click(&mut g, 190, 45);
    assert_eq!(g.wont_run(), None, "back on, and done up");

    // The exhaust: two bolts, and the stock one goes by its name.
    click(&mut g, 105, 55);
    assert_eq!(g.wont_run().as_deref(), Some("The exhaust is not bolted down."));
    click(&mut g, 120, 70);
    assert_eq!(g.note.as_deref(), Some("It is bolted on. Undo the bolts first."));
    click(&mut g, 134, 84);
    click(&mut g, 120, 70);
    assert_eq!(g.floor().len(), 2);
    g.tick(0, 20, 182);
    g.draw(&mut s);
    // The pipe goes on, and it is the pipe's own picture that shows there, stretched to the box.
    click(&mut g, 90, 182);
    assert_eq!(g.carrying, Some(Loose::Part("pipe".into())));
    click(&mut g, 120, 70);
    click(&mut g, 105, 55);
    click(&mut g, 134, 84);
    assert_eq!(g.wont_run(), None);
    g.tick(0, 300, 150);
    g.draw(&mut s);
    assert_eq!(s.at(120, 70), shade(ramp("YELLOW"), 24));
    assert_eq!(g.floor(), vec![Loose::Stock("Exhaust".into())]);
    g.tick(0, 20, 182);
    g.draw(&mut s);
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
fn the_paper_has_work_for_when_the_money_is_gone() {
    let mut s = Screen::new();
    // A month this time, so there are days enough to work back to a car.
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&MANIFEST.replace(r#""days": 3"#, r#""days": 30"#))));
    g.seed(7);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    // Broke, and the car sold to the dealer: the help-wanted page is the way back.
    click(&mut g, 220, 10);
    click(&mut g, 280, 36);
    g.state.cash = 0;
    g.tick(KEY_B, 0, 0);
    click(&mut g, 140, 10);
    assert_eq!(g.scene, Scene::Paper(0), "with a season of work left, it is not over");
    click(&mut g, 150, 40);
    assert_eq!(g.scene, Scene::Paper(2), "the jobs tab");
    g.draw(&mut s);
    let jobs = g.jobs();
    assert_eq!(jobs.len(), 3, "the pack has none of its own, so the ordinary three");
    let (hours, cash) = (g.state.hours, g.state.cash);
    click(&mut g, 100, 60);
    assert_eq!((g.state.cash, g.state.hours), (cash + jobs[0].pay, hours + jobs[0].hours), "it pays, and it takes the hours");
    assert_eq!(g.scene, Scene::Paper(2));
    click(&mut g, 280, 40);
    assert_eq!(g.scene, Scene::Paper(1), "the parts tab is where it was");
    click(&mut g, 220, 40);
    assert_eq!(g.scene, Scene::Paper(0), "and so is the cars tab");
}

#[test]
fn a_pack_can_list_its_own_jobs() {
    let m = MANIFEST.replace(r#""bets": [10, 25],"#, r#""bets": [10, 25], "jobs": [ { "name": "Wash cars", "hours": 2, "pay": 40 } ],"#);
    engine::host::store(b"");
    let g = Game::new(Pack::parse(&files(&m)));
    assert_eq!(g.jobs().iter().map(|j| (j.name.as_str(), j.hours, j.pay)).collect::<Vec<_>>(), vec![("Wash cars", 2, 40)]);
    let bad = MANIFEST.replace(r#""bets": [10, 25],"#, r#""bets": [10, 25], "jobs": [ { "name": "Nothing", "hours": 0, "pay": 40 } ],"#);
    assert!(Pack::parse(&files(&bad)).is_err(), "a job has to take some time");
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
    assert_eq!(fresh.scene, Scene::Title, "with a game kept, E has to be meant");
    fresh.tick(0, 0, 0);
    fresh.tick(KEY_X, 0, 0);
    assert_eq!((fresh.scene, fresh.state.cash, fresh.state.place.as_str(), fresh.hints), (Scene::Place, 1000, "shed", true), "E twice is a new game, and a new game is shown round");
    assert!(!again.hints, "someone carrying on is not");
    // H outlines what there is to pick, and H again takes the boxes away.
    let mut s = Screen::new();
    fresh.tick(0, 300, 190);
    fresh.draw(&mut s);
    assert_eq!(s.at(40, 0), shade(ramp("GOLD"), 26), "a box round the lamps");
    fresh.tick(engine::game::KEY_H, 300, 190);
    fresh.draw(&mut s);
    assert_eq!((fresh.hints, s.at(40, 0)), (false, shade(ramp("WOOD"), 12)));
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

/// The default pack as the tool packs it, if it has been built (`make engine`).
fn default_pack() -> Option<Vec<u8>> {
    std::fs::read("../web/default.pack").ok()
}

#[test]
fn the_race_is_seen_from_the_seat_when_the_car_has_a_cockpit() {
    let Some(bytes) = default_pack() else { return };
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&bytes));
    let mut s = Screen::new();
    g.seed(3);
    g.tick(KEY_A, 0, 0);
    g.tick(0, 0, 0);
    g.state.cars.push(Owned::new("umbra", Some("RED".into()), 70));
    g.state.cur = Some(0);
    assert!(g.car().unwrap().cockpit.is_some());
    // Out to the lot, and a race for kicks against whoever is there; then sit on the line while he goes.
    click(&mut g, 150, 130);
    assert_eq!(g.state.place, "lot");
    g.who = 12;
    click(&mut g, 40, 186);
    click(&mut g, 30, 185);
    assert_eq!(g.scene, Scene::Race);
    g.tick(KEY_A, 0, 0);
    for _ in 0..110 {
        g.tick(0, 0, 0);
    }
    g.draw(&mut s);
    let paint = ramp(g.rival().unwrap().paint.as_deref().unwrap()) as u16;
    let seen = (0..136).flat_map(|y| (160..320).map(move |x| (x, y))).filter(|&(x, y)| s.at(x, y) >> 5 == paint).count();
    assert!(seen > 30, "he is ahead in the other lane, in his own paint ({seen} pixels)");
    assert!((0..320).any(|x| s.at(x, 120) >> 5 == ramp("RED") as u16), "your bonnet is in your paint");
    if let Ok(dir) = std::env::var("SHOTS") {
        let mut out = b"P6 320 240 255\n".to_vec();
        out.extend(s.fb.iter().flat_map(|&c| { let v = s.pal[c as usize]; [v as u8, (v >> 8) as u8, (v >> 16) as u8] }));
        std::fs::write(format!("{dir}/seat-rival.ppm"), out).unwrap();
    }
    // A car with no cockpit is raced from the side, as before.
    g.pack.manifest.cars.get_mut("umbra").unwrap().cockpit = None;
    g.draw(&mut s);
    assert_eq!(s.at(300, 230) >> 5, ramp("RUBBER") as u16, "the side view's dash is along the bottom");
}

const TOWN: &str = r#""maps": { "grid": { "title": "Grid", "traffic": 0, "rows": ["A.S", "H.B"],
    "blocks": { "A": { "wall": "BRICK", "height": 12 }, "B": { "wall": "TAN", "height": 8 },
                "H": { "wall": "WOOD", "height": 6, "to": "shed", "name": "Home" }, "S": { "to": "road", "name": "Strip" } } } },
  "places": {"#;

fn town_game() -> Game {
    engine::host::store(b"");
    let manifest = MANIFEST.replace("\"places\": {", TOWN).replace("\"do\": \"say:Hello.\"", "\"do\": \"drive:grid\"").replace("\"label\": \"Solo\", \"do\": \"solo\"", "\"label\": \"Gas\", \"do\": \"gas\"");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(5);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    g
}

#[test]
fn a_map_is_blocks_and_streets() {
    let g = town_game();
    let map = &g.pack.manifest.maps["grid"];
    assert_eq!((map.cols(), map.depth()), (3, 2));
    assert_eq!(map.block(0, 0).and_then(|b| b.name.as_deref()), Some("Home"), "the bottom row of the text is the south");
    assert_eq!(map.block(2, 1).and_then(|b| b.to.as_deref()), Some("road"));
    assert!(map.block(1, 0).is_none() && map.block(3, 0).is_none());
    assert_eq!(map.places().len(), 2);
    // Streets are clear, buildings are not, a lot with nothing built on it is, and the country beyond is not.
    assert!(!Town::blocked(map, 0.0, 30.0) && !Town::blocked(map, 30.0, 64.0));
    assert!(Town::blocked(map, 32.0, 32.0) && Town::blocked(map, 160.0, 32.0));
    assert!(!Town::blocked(map, 160.0, 96.0) && !Town::blocked(map, 96.0, 32.0));
    assert!(Town::blocked(map, -20.0, 30.0) && Town::blocked(map, 30.0, 150.0));
    // A bad map is explained.
    let bad = |from: &str, to: &str| Pack::parse(&files(&MANIFEST.replace("\"places\": {", &TOWN.replace(from, to)))).err().unwrap();
    assert!(bad("\"A.S\"", "\"A.SS\"").contains("same length"));
    assert!(bad("\"A.S\"", "\"AxS\"").contains("marked x"));
    assert!(bad("\"to\": \"road\"", "\"to\": \"moon\"").contains("moon"));
}

#[test]
fn you_drive_from_one_place_to_another_and_it_burns_gas() {
    let mut g = town_game();
    let mut s = Screen::new();
    click(&mut g, 100, 10);
    assert_eq!(g.scene, Scene::Drive);
    let t = g.town.as_ref().unwrap();
    assert_eq!((t.x, t.z, t.heading), (32.0, -4.0, 90.0), "out on the street south of home, facing east");
    g.draw(&mut s);
    assert!(t.beside(&g.pack.manifest.maps["grid"]).is_some_and(|b| b.name.as_deref() == Some("Home")));

    // East along the south street to the far corner, north up the east street, and stop beside the strip.
    let hold = |g: &mut Game, keys: u32, until: &dyn Fn(&Town) -> bool| {
        for _ in 0..3000 {
            if until(g.town.as_ref().unwrap()) {
                return;
            }
            g.tick(keys, 0, 0);
        }
        panic!("never got there: {:?}", g.town.as_ref().map(|t| (t.x, t.z, t.heading)));
    };
    hold(&mut g, KEY_UP, &|t| t.x > 150.0);
    hold(&mut g, 2, &|t| t.speed < 6.0);
    hold(&mut g, KEY_UP | 4, &|t| t.heading < 6.0 || t.heading > 354.0);
    hold(&mut g, KEY_UP, &|t| t.z > 70.0);
    hold(&mut g, 2, &|t| t.speed == 0.0);
    g.draw(&mut s);
    let t = g.town.as_ref().unwrap();
    assert_eq!(t.dents, 0, "without hitting anything");
    assert!(t.beside(&g.pack.manifest.maps["grid"]).is_some_and(|b| b.name.as_deref() == Some("Strip")), "{:?}", (t.x, t.z));
    let gas = g.owned().unwrap().gas;
    assert!(gas < 8.0 && gas > 7.8, "{gas}");
    g.tick(0, 0, 0);
    g.tick(KEY_A, 0, 0);
    assert_eq!((g.scene, g.state.place.as_str(), g.state.hours), (Scene::Place, "road", 1), "going in is an hour");

    // Gas: a dollar buys three gallons, up to a full tank.
    g.tick(0, 0, 0);
    click(&mut g, 250, 10);
    assert_eq!((g.owned().unwrap().gas, g.state.cash), (17.0, 1000 - 4));
    click(&mut g, 250, 10);
    assert_eq!(g.note.as_deref(), Some("It is full."));

    // Into a wall, and out of gas.
    let map = g.pack.manifest.maps["grid"].clone();
    let mut t = Town::new("grid", &map, Some("shed"), || 1);
    t.heading = 0.0;
    let mut hit = false;
    for _ in 0..200 {
        hit |= t.tick(&map, Pedals { go: true, ..Pedals::default() }).1;
    }
    assert!(hit && t.dents == 1 && t.z < 12.0, "the building north of the street stops it");
    g.state.cars[0].gas = 0.31;
    click(&mut g, 20, 10);
    click(&mut g, 100, 10);
    assert_eq!(g.scene, Scene::Drive);
    for _ in 0..3000 {
        if g.scene != Scene::Drive { break; }
        g.tick(KEY_UP, 0, 0);
        if g.town.as_ref().is_some_and(|t| t.x > 150.0) { g.town.as_mut().unwrap().x = 20.0; }
    }
    assert_eq!((g.scene, g.state.place.as_str(), g.note.as_deref()), (Scene::Place, "shed", Some("Out of gas. You walked home.")));
    click(&mut g, 100, 10);
    assert_eq!(g.note.as_deref(), Some("A can from the pump: a gallon, $2."));
    // With no two dollars for the can, that is where it stays.
    g.state.cars[0].gas = 0.0;
    g.state.cash = 1;
    click(&mut g, 100, 10);
    assert_eq!((g.scene, g.note.as_deref()), (Scene::Place, Some("No gas, and no $2 for a can of it.")));
}

#[test]
fn parts_change_more_than_torque() {
    let extra = r#""parts": { "tyres": { "name": "Tyres", "slot": "Tyres", "price": 10, "grip": 25 },
        "box": { "name": "Box", "slot": "Gearbox", "price": 10, "gears": [3.4, 2.0, 1.4, 1.0], "final": 4.5 },
        "light": { "name": "Light bits", "slot": "Body", "price": 10, "weight_lb": -400 },
        "forged": { "name": "Forged", "slot": "Bottom end", "price": 10, "redline": 8200 },"#;
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&MANIFEST.replacen("\"parts\": {", extra, 1))));
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    let car = g.car().unwrap().clone();
    let stock = g.tuned_drive(g.owned().unwrap(), &car);
    assert_eq!((stock.grip, stock.weight_lb, stock.gears.len(), stock.redline), (12.0, 3060.0, 5, 7200.0));
    let time = |g: &Game| {
        let (o, car) = (g.owned().unwrap(), g.car().unwrap());
        let drive = g.tuned_drive(o, car);
        let mut r = Race::new(&drive, 1.0, None, 0, 0);
        r.tick(Controls { stage: true, ..Controls::default() });
        for _ in 0..3000 {
            if r.phase == Phase::Slip {
                break;
            }
            let up = (r.gear == 0 && r.tree >= 51) || (r.gear > 0 && r.rpm > drive.redline - 250.0);
            r.tick(Controls { throttle: true, up, ..Controls::default() });
        }
        (r.sixty, r.et)
    };
    let before = time(&g);
    for (slot, part) in [("Tyres", "tyres"), ("Body", "light")] {
        g.state.cars[0].fitted.insert(slot.into(), part.into());
    }
    let d = g.tuned_drive(g.owned().unwrap(), &car);
    assert_eq!((d.grip, d.weight_lb), (15.0, 2660.0));
    let after = time(&g);
    assert!(after.0 < before.0 && after.1 < before.1 - 200, "grip and less weight are quicker: {before:?} then {after:?}");
    for (slot, part) in [("Gearbox", "box"), ("Bottom end", "forged")] {
        g.state.cars[0].fitted.insert(slot.into(), part.into());
    }
    let d = g.tuned_drive(g.owned().unwrap(), &car);
    assert_eq!((d.gears.clone(), d.final_drive, d.redline), (vec![3.4, 2.0, 1.4, 1.0], 4.5, 8200.0));
    assert!(time(&g).1 > 0, "and a four-speed still gets there");
}

#[test]
fn an_engine_held_on_the_limiter_lets_go_and_is_rebuilt() {
    let mut g = game();
    click(&mut g, 20, 10);
    click(&mut g, 150, 10);
    click(&mut g, 4 + 78 * 2 + 30, 185);
    assert_eq!(g.scene, Scene::Race);
    // Away on the green, and then never a shift.
    g.tick(KEY_A, 0, 0);
    for _ in 0..52 {
        g.tick(KEY_UP, 0, 0);
    }
    g.tick(KEY_UP | KEY_X, 0, 0);
    for _ in 0..3000 {
        if g.race.as_ref().is_none_or(|r| r.phase == Phase::Slip) {
            break;
        }
        g.tick(KEY_UP, 0, 0);
    }
    let r = g.race.as_ref().unwrap();
    assert!(r.blown && r.et == 0 && !r.won, "{:?}", (r.blown, r.et, r.pos));
    let mut s = Screen::new();
    g.draw(&mut s);
    g.tick(0, 0, 0);
    g.tick(KEY_A, 0, 0);
    assert_eq!((g.scene, g.state.cash, g.owned().unwrap().cond), (Scene::Place, 975, 50), "the bet is lost and so is the engine");
    assert_eq!(g.note.as_deref(), Some("You blew the engine. It needs a rebuild."));

    // A shift in time saves it: strain is given back.
    let mut r = Race::new(&g.car().unwrap().drive, 1.0, None, 0, 0);
    r.tick(Controls { stage: true, ..Controls::default() });
    for t in 0..400 {
        let up = (r.gear == 0 && r.tree >= 51) || (r.gear > 0 && r.strain > 20.0);
        r.tick(Controls { throttle: true, up, ..Controls::default() });
        assert!(!r.blown, "tick {t}");
    }

    // The rebuild: four hours and the pack's price, and it is healthy again.
    let rebuilt = MANIFEST.replace("\"do\": \"say:Hello.\"", "\"do\": \"rebuild\"").replace("\"name\": \"Test Pack\",", "\"name\": \"Test Pack\", \"rebuild\": { \"cost\": 300, \"cond\": 90 },");
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&rebuilt)));
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    g.state.cars[0].cond = 30;
    g.state.cash = 100;
    click(&mut g, 100, 10);
    assert_eq!((g.owned().unwrap().cond, g.note.as_deref()), (30, Some("A rebuild is $300. You don't have it.")));
    g.state.cash = 1000;
    click(&mut g, 100, 10);
    assert_eq!((g.owned().unwrap().cond, g.state.cash, g.state.hours), (90, 700, 4));
    click(&mut g, 100, 10);
    assert_eq!(g.state.cash, 700, "a healthy engine is not rebuilt");
}

#[test]
fn a_part_behind_another_waits_for_it() {
    engine::host::store(b"");
    let hood = r#""ground": 2, "hood": { "picture": "shed.pic", "slots": {
        "Filter": { "rect": [200, 50, 30, 30], "bolts": 1 },
        "Pipe": { "rect": [100, 50, 40, 40], "bolts": 1, "behind": "Filter" } } },"#;
    let manifest = MANIFEST.replace("\"ground\": 2,", hood).replace("\"do\": \"say:Hello.\"", "\"do\": \"hood\"");
    assert!(Pack::parse(&files(&manifest.replace("\"behind\": \"Filter\"", "\"behind\": \"Moon\""))).err().unwrap().contains("Moon"));
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    click(&mut g, 100, 10);
    let (pipe_bolt, pipe, filter_bolt, filter) = ((120, 70), (110, 60), (215, 65), (205, 55));
    // The pipe's bolt comes out, but the pipe will not lift past the filter.
    click(&mut g, pipe_bolt.0, pipe_bolt.1);
    click(&mut g, pipe.0, pipe.1);
    assert_eq!((g.note.as_deref(), g.owned().unwrap().off.len()), (Some("The filter is in the way."), 0));
    click(&mut g, filter_bolt.0, filter_bolt.1);
    click(&mut g, filter.0, filter.1);
    click(&mut g, pipe.0, pipe.1);
    assert_eq!(g.owned().unwrap().off, vec!["Filter".to_string(), "Pipe".to_string()]);
    // Going back together, the filter waits for the pipe.
    assert_eq!(g.floor(), vec![Loose::Stock("Filter".into()), Loose::Stock("Pipe".into())]);
    click(&mut g, 20, 182);
    click(&mut g, filter.0, filter.1);
    assert_eq!((g.note.as_deref(), g.owned().unwrap().off.len()), (Some("The pipe goes on first."), 2));
    click(&mut g, 20, 182);
    click(&mut g, 90, 182);
    click(&mut g, pipe.0, pipe.1);
    click(&mut g, 20, 182);
    click(&mut g, filter.0, filter.1);
    assert!(g.owned().unwrap().off.is_empty());
}

const ROADS: &str = r#""roads": { "pass": { "title": "The pass", "ends": ["shed", "map:grid"], "limit": 50, "ticket": 30, "traffic": 0, "cops": 1, "hours": 2,
      "parts": [ { "len": 300 }, { "len": 240, "curve": 0.6, "hill": 12, "left": "rock", "right": "drop" }, { "len": 300, "hill": -12, "left": "fields", "right": "posts" } ] } },
  "maps": { "grid": { "title": "Grid", "traffic": 0, "rows": ["A.S", "H.P"],
    "blocks": { "A": { "wall": "BRICK", "height": 12 }, "P": { "road": "pass", "name": "The pass" },
                "H": { "wall": "WOOD", "height": 6, "to": "shed", "name": "Home" }, "S": { "to": "road", "name": "Strip" } } } },
  "places": {"#;

fn road_game() -> Game {
    engine::host::store(b"");
    let manifest = MANIFEST.replace("\"places\": {", ROADS).replace("\"do\": \"say:Hello.\"", "\"do\": \"road:pass\"")
        .replace("\"label\": \"Solo\", \"do\": \"solo\"", "\"label\": \"Run\", \"do\": \"roadrace:pass\"")
        .replace("\"puffer\": {", "\"fuzz\": { \"name\": \"Detector\", \"slot\": \"Dash\", \"price\": 50, \"radar\": true }, \"puffer\": {");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(5);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    g
}

/// Hold the middle of the lane at about a speed, until something is so.
fn cruise(g: &mut Game, mph: f64, until: &dyn Fn(&Game) -> bool) {
    for _ in 0..20000 {
        if until(g) {
            return;
        }
        let Some(t) = g.trip.as_ref() else { return };
        let mut keys = if t.speed * 2.237 < mph { KEY_UP } else { 0 };
        if t.x > 0.55 { keys |= 4 } else if t.x < 0.45 { keys |= 8 }
    // Up a gear short of the redline, on every other tick so that each is a fresh press.
    if t.gearbox.as_ref().is_some_and(|b| b.rpm > b.redline - 600.0) && t.clock % 2 == 0 { keys |= KEY_X }
        g.tick(keys, 0, 0);
    }
    panic!("never happened: {:?}", g.trip.as_ref().map(|t| (t.z, t.x, t.speed)));
}

#[test]
fn a_road_is_stretches_between_two_ends() {
    let g = road_game();
    let road = &g.pack.manifest.roads["pass"];
    assert_eq!((road.len(), road.limit, road.cops), (840.0, 50.0, 1));
    let t = engine::road::Trip::new("pass", road, false, 3);
    assert_eq!((t.len(), t.segs.len()), (840.0, 140));
    assert!((t.height(540.0) - 12.0).abs() < 0.2 && t.height(840.0).abs() < 0.7, "up twelve metres and down again");
    assert!(t.seg(420.0).curve > 0.5 && t.seg(100.0).curve == 0.0 && t.seg(310.0).curve < 0.3, "a bend eases in");
    // The other way it is the same road turned about.
    let back = engine::road::Trip::new("pass", road, true, 3);
    assert!(back.seg(420.0).curve < -0.5 && (back.height(300.0) - 12.0).abs() < 0.2);
    assert_eq!((t.seg(420.0).right, back.seg(420.0).left), (engine::road::Side::Drop, engine::road::Side::Drop));

    let bad = |from: &str, to: &str| Pack::parse(&files(&MANIFEST.replace("\"places\": {", &ROADS.replace(from, to)))).err().unwrap();
    assert!(bad("\"ends\": [\"shed\"", "\"ends\": [\"moon\"").contains("moon"));
    assert!(bad("\"right\": \"drop\"", "\"right\": \"lava\"").contains("lava"));
    assert!(bad("{ \"len\": 300 }", "{ \"len\": 3 }").contains("12 metres"));
    assert!(bad("\"road\": \"pass\"", "\"road\": \"lane\"").contains("map:grid"));
    assert!(bad("\"curve\": 0.6", "\"curve\": 6").contains("bends more"));
    let spot = MANIFEST.replace("\"do\": \"say:Hello.\"", "\"do\": \"road:nowhere\"");
    assert!(Pack::parse(&files(&spot)).err().unwrap().contains("nowhere"));
}

#[test]
fn a_road_is_driven_to_its_other_end() {
    let mut g = road_game();
    let mut s = Screen::new();
    click(&mut g, 100, 10);
    assert_eq!(g.scene, Scene::Road);
    assert!(g.trip.as_ref().is_some_and(|t| !t.backwards && t.x == 0.5));
    g.draw(&mut s);
    assert_eq!(s.at(160, 2) / 32, ramp("SKY") as u16);
    g.state.hours = 3;
    g.draw(&mut s);
    assert_eq!(s.at(160, 2) / 32, ramp("INDIGO") as u16, "late in the day the sky goes");
    g.state.hours = 0;
    g.draw(&mut s);
    assert_eq!(s.at(160, 150), shade(ramp("ASPHALT"), 15), "the road is under the nose of the car");

    // Turning round costs nothing but the gas.
    cruise(&mut g, 40.0, &|g| g.trip.as_ref().unwrap().z > 60.0);
    g.tick(KEY_B, 0, 0);
    assert_eq!((g.scene, g.state.place.as_str(), g.state.hours), (Scene::Place, "shed", 0));
    g.tick(0, 0, 0);

    // Inside the limit all the way: past the police, round the bend, and into town at the block the road comes in by.
    click(&mut g, 100, 10);
    cruise(&mut g, 45.0, &|g| g.trip.as_ref().unwrap().z > 500.0);
    g.draw(&mut s);
    cruise(&mut g, 45.0, &|g| g.scene != Scene::Road);
    assert_eq!((g.scene, g.state.hours, g.state.cash), (Scene::Drive, 2, 1000), "two hours, and no ticket");
    // In off the road still rolling, on the street beside the block it comes in by, facing into town.
    let t = g.town.as_ref().unwrap();
    assert!(t.speed > 5.0 && t.heading == 0.0 && (t.x - 130.0).abs() < 1.0 && t.z < 0.0, "{:?}", (t.x, t.z, t.speed));
    for _ in 0..90 { g.tick(2, 0, 0); }
    let t = g.town.as_ref().unwrap();
    assert!(t.beside(&g.pack.manifest.maps["grid"]).is_some_and(|b| b.road.as_deref() == Some("pass")), "{:?}", (t.x, t.z));
    let gas = g.owned().unwrap().gas;
    assert!(gas < 8.0 && gas > 7.6, "{gas}");
    // The shed is a road away now: no stepping back to it.
    g.tick(KEY_B, 0, 0);
    assert_eq!(g.scene, Scene::Drive);
    g.draw(&mut s);

    // And back out the way you came, which is the road the other way round.
    g.tick(0, 0, 0);
    g.tick(KEY_A, 0, 0);
    assert_eq!(g.scene, Scene::Road);
    assert!(g.trip.as_ref().unwrap().backwards);
    cruise(&mut g, 45.0, &|g| g.scene != Scene::Road);
    assert_eq!((g.scene, g.state.place.as_str(), g.state.hours), (Scene::Place, "shed", 4));
}

#[test]
fn the_law_is_on_the_road() {
    // Past the police well over the limit: lights. Stop, and it is a ticket.
    let mut g = road_game();
    click(&mut g, 100, 10);
    let cop = g.trip.as_ref().unwrap().cops[0].z;
    cruise(&mut g, 80.0, &|g| g.trip.as_ref().unwrap().chase.is_some());
    assert!(g.trip.as_ref().unwrap().z >= cop && g.note.as_deref() == Some("Police! Pull over, or run for it."));
    g.tick(KEY_B, 0, 0);
    assert_eq!((g.scene, g.note.as_deref()), (Scene::Road, Some("Not with him behind you.")));
    for _ in 0..400 {
        g.tick(2, 0, 0);
    }
    let t = g.trip.as_ref().unwrap();
    assert!(t.chase.is_none() && t.speed == 0.0);
    let fine = 1000 - g.state.cash;
    assert!((30 + 2 * 15..30 + 2 * 45).contains(&fine), "thirty dollars and two for every mile an hour over: {fine}");
    assert_eq!(g.note, Some(format!("Speeding. That is a ${fine} ticket.")));

    // Keep your foot in it for twenty seconds and he is gone.
    let road = g.pack.manifest.roads["pass"].clone();
    let mut long = road.clone();
    long.parts[2].len = 4000.0;
    let mut t = engine::road::Trip::new("pass", &long, false, 9);
    let (mut siren, mut lost, mut fined) = (false, false, false);
    for _ in 0..3000 {
        let keys = Pedals { go: true, left: t.x > 0.55, right: t.x < 0.45, ..Pedals::default() };
        let h = t.tick(&long, keys);
        siren |= h.siren;
        lost |= h.lost_him;
        fined |= h.ticket.is_some();
    }
    assert!(siren && lost && !fined, "{:?}", (siren, lost, fined, t.z, t.speed));

    // A detector knows about him before you do, if it is in the car.
    let mut g = road_game();
    click(&mut g, 100, 10);
    cruise(&mut g, 40.0, &|g| g.trip.as_ref().unwrap().radar);
    assert!(g.trip.as_ref().unwrap().z < cop && !g.radar());
    g.state.cars[0].fitted.insert("Dash".into(), "fuzz".into());
    assert!(g.radar());
}

#[test]
fn the_road_has_edges_and_other_cars() {
    let g = road_game();
    let mut road = g.pack.manifest.roads["pass"].clone();
    // Off the right-hand side of the bend is the drop.
    let mut t = engine::road::Trip::new("pass", &road, false, 9);
    let mut crashed = false;
    for _ in 0..2000 {
        let h = t.tick(&road, Pedals { go: t.speed < 25.0, right: t.z > 400.0 || t.x < 0.45, left: t.z <= 400.0 && t.x > 0.55, ..Pedals::default() });
        if h.crashed {
            crashed = true;
            break;
        }
    }
    assert!(crashed && t.z > 400.0 && t.scrapes == 1 && t.speed == 0.0 && t.crash > 0, "{:?}", (t.z, t.x));
    // Fields are only slow.
    let mut t = engine::road::Trip::new("pass", &road, false, 9);
    t.z = 600.0;
    for _ in 0..300 {
        assert!(!t.tick(&road, Pedals { go: true, left: true, ..Pedals::default() }).crashed);
    }
    assert!(t.x < -1.5 && t.speed < 16.0, "{:?}", (t.x, t.speed));
    // Other cars keep to their lanes, and one in yours is in the way.
    road.traffic = 12;
    let mut t = engine::road::Trip::new("pass", &road, false, 4);
    assert_eq!(t.others.len(), 12);
    assert!(t.others.iter().all(|o| (o.lane == 0.5) == (o.speed > 0.0)) && t.others.iter().any(|o| o.speed < 0.0) && t.others.iter().any(|o| o.speed > 0.0));
    t.others[0] = engine::road::Other { z: 40.0, lane: 0.5, speed: 5.0, paint: "RED", rig: false };
    let mut hit = false;
    for _ in 0..600 {
        hit |= t.tick(&road, Pedals { go: true, left: t.x > 0.55, right: t.x < 0.45, ..Pedals::default() }).crashed;
    }
    assert!(hit);

    // In the game a crash is a tired car, and running dry is a walk.
    let mut g = road_game();
    click(&mut g, 100, 10);
    g.trip.as_mut().unwrap().others.push(engine::road::Other { z: 60.0, lane: 0.5, speed: 0.0, paint: "RED", rig: false });
    let cond = g.owned().unwrap().cond;
    cruise(&mut g, 40.0, &|g| g.trip.as_ref().unwrap().crash > 0);
    assert_eq!(g.owned().unwrap().cond, cond - 3);
    g.state.cars[0].gas = 0.01;
    cruise(&mut g, 40.0, &|g| g.scene != Scene::Road);
    assert_eq!((g.scene, g.state.place.as_str(), g.note.as_deref()), (Scene::Place, "shed", Some("Out of gas. You walked home.")));
}

#[test]
fn a_road_is_somewhere_to_race() {
    // Out to where the rivals are, and a ten-dollar race down the pass with the slow one.
    let mut g = road_game();
    click(&mut g, 20, 10);
    click(&mut g, 250, 10);
    assert_eq!(g.scene, Scene::Bets);
    click(&mut g, 120, 186);
    assert_eq!(g.scene, Scene::Road);
    let t = g.trip.as_ref().unwrap();
    assert!(t.rival.is_some() && t.cops.is_empty() && t.hold == 90 - 1, "no law, and a count before the off: {}", t.hold);
    let mut s = Screen::new();
    g.draw(&mut s);
    for _ in 0..80 {
        g.tick(KEY_UP, 0, 0);
    }
    assert_eq!(g.trip.as_ref().unwrap().z, 0.0, "nobody moves until it is counted down");
    cruise(&mut g, 80.0, &|g| g.trip.as_ref().is_some_and(|t| t.z > 300.0));
    g.draw(&mut s);
    cruise(&mut g, 80.0, &|g| g.scene != Scene::Road);
    assert_eq!((g.scene, g.state.place.as_str(), g.state.cash, g.state.hours, g.state.rung), (Scene::Place, "road", 1010, 2, 1), "{:?}", g.note);

    // Sit on the line and they get there first; Escape is giving it to them.
    g.tick(0, 0, 0);
    g.who = 0;
    click(&mut g, 250, 10);
    click(&mut g, 120, 186);
    for _ in 0..3000 {
        if g.scene != Scene::Road { break; }
        g.tick(0, 0, 0);
    }
    assert_eq!((g.scene, g.state.cash, g.state.hours), (Scene::Place, 1000, 4));
    g.who = 0;
    click(&mut g, 250, 10);
    click(&mut g, 120, 186);
    g.tick(0, 0, 0);
    g.tick(KEY_B, 0, 0);
    assert_eq!((g.scene, g.state.cash), (Scene::Place, 990));
    // A place with nobody to race cannot start one.
    let bad = MANIFEST.replace("\"places\": {", ROADS).replace("\"do\": \"say:Hello.\"", "\"do\": \"roadrace:pass\"");
    assert!(Pack::parse(&files(&bad)).err().unwrap().contains("rival_at"));
}

#[test]
fn the_car_is_what_it_is_on_the_road_too() {
    let mut g = road_game();
    g.state.cars[0].cond = 100;
    let stock = g.pace();
    assert!((stock.accel - 5.0).abs() < 0.6 && (stock.top - 56.0).abs() < 3.0 && stock.slide == 1.0, "{stock:?}");
    g.state.cars[0].fitted.insert("Exhaust".into(), "pipe".into());
    let piped = g.pace();
    assert!(piped.accel > stock.accel && piped.top > stock.top);
    g.state.cars[0].cond = 50;
    assert!(g.pace().accel < stock.accel, "a tired engine pulls less");
    // And the trip is given it.
    click(&mut g, 100, 10);
    assert_eq!(g.trip.as_ref().unwrap().pace, g.pace());
    // A quicker rival is quicker down a road as well.
    let (slow, quick) = (engine::road::Rival::new(17.0, "RED"), engine::road::Rival::new(12.0, "RED"));
    assert!(quick.top > slow.top && quick.accel > slow.accel);
}

#[test]
fn people_stand_about_and_parts_show_on_the_car() {
    let green = shade(ramp("GREEN"), 20);
    let manifest = MANIFEST.replace("\"et\": 30, \"limit\": 25", "\"et\": 30, \"limit\": 25, \"figure\": \"guy.pic\"")
        .replace("\"pitch\": \"Loud\"", "\"pitch\": \"Loud\", \"side\": \"pipe-side.pic\"")
        .replace("\"car_at\": [10, 50],", "\"car_at\": [10, 50], \"people\": [ { \"picture\": \"guy.pic\", \"at\": [100, 90] } ],");
    let with = |manifest: &str| {
        let mut bytes = files(manifest);
        // Two more pictures on the end of the pack, and the count at its head put up to match.
        for (name, data) in [("guy.pic", pic(2, 3, &[(6, green)])), ("pipe-side.pic", pic(4, 2, &[(7, CLEAR), (1, 15)]))] {
            bytes.extend((name.len() as u16).to_le_bytes());
            bytes.extend(name.as_bytes());
            bytes.extend((data.len() as u32).to_le_bytes());
            bytes.extend(data);
        }
        bytes[4..8].copy_from_slice(&6u32.to_le_bytes());
        bytes
    };
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&with(&manifest)));
    g.seed(7);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    let mut s = Screen::new();
    g.draw(&mut s);
    assert_eq!((s.at(100, 88), s.at(101, 89), s.at(102, 88)), (green, green, shade(ramp("WOOD"), 12)), "someone stands in the shed, feet at 90");
    assert_ne!(s.at(13, 49), 15);
    g.state.cars[0].fitted.insert("Exhaust".into(), "pipe".into());
    g.draw(&mut s);
    assert_eq!(s.at(13, 49), 15, "the pipe shows on the car");
    // Whoever has pulled in stands by the nose of their car.
    click(&mut g, 20, 10);
    g.draw(&mut s);
    assert_eq!((s.at(17, 150), s.at(18, 149)), (green, green));
    // A race, and it is on the record.
    click(&mut g, 150, 10);
    click(&mut g, 40, 186);
    drive(&mut g);
    assert_eq!(g.state.wins + g.state.losses, 1);
    click(&mut g, 20, 10);
    click(&mut g, 250, 10);
    assert_eq!(g.note, Some(format!("Day 1 of 3. Won {}, lost {}.", g.state.wins, g.state.losses)));
    // Pictures that are named have to be there.
    assert!(Pack::parse(&files(&manifest)).err().unwrap().contains("which is not in the pack"));
}

#[test]
fn a_respray_a_swap_a_title_and_the_dark() {
    let manifest = MANIFEST.replace("\"do\": \"say:Hello.\"", "\"do\": \"paint\"").replace("\"name\": \"Test Pack\",", "\"name\": \"Test Pack\", \"title\": \"shed.pic\",")
        .replace("\"torque\": 10 }", "\"torque\": 10, \"curve\": [[1000, 100], [4000, 400], [6000, 380]] }");
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(7);
    let mut s = Screen::new();
    g.draw(&mut s);
    assert_eq!(s.at(5, 5), shade(ramp("WOOD"), 12), "the title is the pack's picture");
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);

    // The third tin along is gold, and it is sixty dollars and three hours.
    click(&mut g, 100, 10);
    assert_eq!(g.scene, Scene::Paint);
    g.draw(&mut s);
    assert_eq!(s.at(95, 186) / 32, ramp("GOLD") as u16);
    click(&mut g, 95, 186);
    assert_eq!((g.scene, g.owned().unwrap().paint.as_deref(), g.state.cash, g.state.hours), (Scene::Place, Some("GOLD"), 940, 3));
    click(&mut g, 100, 10);
    click(&mut g, 95, 186);
    assert_eq!((g.note.as_deref(), g.state.cash), (Some("It is that colour already."), 940));

    // Three hours into a day of four it is getting dark.
    assert_eq!(g.dark(), 1);
    g.state.hours = 4;
    assert_eq!(g.dark(), 0);

    // A different engine is a different curve.
    let car = g.car().unwrap().clone();
    g.state.cars[0].fitted.insert("Exhaust".into(), "pipe".into());
    assert_eq!(g.tuned_drive(g.owned().unwrap(), &car).torque, vec![[1000.0, 100.0], [4000.0, 400.0], [6000.0, 380.0]]);
    assert!(Pack::parse(&files(&manifest.replace("[4000, 400]", "[900, 400]"))).err().unwrap().contains("torque curve"));
    assert!(Pack::parse(&files(&MANIFEST.replace("\"name\": \"Test Pack\",", "\"name\": \"Test Pack\", \"title\": \"none.pic\","))).err().unwrap().contains("title"));
}

#[test]
fn the_rollers_are_strapped_pulled_and_read() {
    engine::host::store(b"");
    let manifest = MANIFEST.replace("\"do\": \"say:Hello.\"", "\"do\": \"dyno\"").replace("\"puffer\": {", "\"lap\": { \"name\": \"Laptop\", \"slot\": \"Computer\", \"price\": 1, \"tuner\": true }, \"puffer\": {");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(7);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    g.state.cars[0].cond = 100;
    let mut s = Screen::new();
    click(&mut g, 100, 10);
    assert_eq!((g.scene, g.rollers.phase), (Scene::Dyno, 0));
    // Four rings in the floor, a strap at each, and then it can be pulled.
    for (n, x) in [26, 62, 110, 146].into_iter().enumerate() {
        g.draw(&mut s);
        assert_eq!(g.rollers.straps.count_ones() as usize, n);
        click(&mut g, x, 172);
    }
    assert_eq!((g.rollers.phase, g.rollers.rpm), (1, 1500.0));
    let pull = |g: &mut Game| {
        for _ in 0..600 {
            if g.rollers.rpm >= 7200.0 { break; }
            g.tick(KEY_UP, 0, 0);
        }
        g.tick(0, 0, 0);
    };
    g.tick(0, 0, 0);
    assert_eq!(g.rollers.phase, 1, "nothing happens until the gas is down");
    g.tick(KEY_UP, 0, 0);
    g.draw(&mut s);
    pull(&mut g);
    assert_eq!(g.rollers.phase, 2);
    let stock = g.sheet.clone().unwrap();
    assert!((stock.hp - 298.0).abs() < 2.0 && stock.hp_at == 5800.0 && stock.torque == 314.0 && stock.torque_at == 4400.0, "{:?}", (stock.hp, stock.hp_at, stock.torque));
    assert!(stock.et > 11000 && stock.et < 16000 && stock.mph > 80 && (stock.top - 125.0).abs() < 3.0, "{:?}", (stock.et, stock.mph, stock.top));
    // The log: a line every 250 revs from 1750 to short of the redline, and nothing knocking.
    let log = g.rollers.log.clone();
    assert!(log.len() >= 20 && log[0].rpm == 1750.0 && log.iter().all(|l| l.knock == 0) && log.iter().any(|l| (l.hp - 298.0).abs() < 3.0), "{}", log.len());
    assert!(log[0].afr == 11.6 && log[0].boost == 0.0 && log.last().unwrap().boost > 5.0, "rich, and on boost by the top");
    g.draw(&mut s);

    // The laptop needs a laptop.
    click(&mut g, 150, 186);
    assert_eq!((g.scene, g.note.as_deref()), (Scene::Dyno, Some("That takes a laptop and a cable.")));
    g.state.cars[0].fitted.insert("Computer".into(), "lap".into());
    click(&mut g, 150, 186);
    assert_eq!(g.scene, Scene::Laptop);
    g.draw(&mut s);
    click(&mut g, 300, 53);
    assert_eq!((g.owned().unwrap().ecu.boost, g.note.as_deref()), (1, Some("Changed. Pull it again to see.")));
    click(&mut g, 222, 53);
    assert_eq!(g.owned().unwrap().ecu.boost, 0);
    // Three degrees of timing is three per cent, and safe.
    for _ in 0..3 { click(&mut g, 300, 129); }
    assert_eq!(g.owned().unwrap().ecu.timing, 3);
    g.tick(KEY_B, 0, 0);
    g.tick(0, 0, 0);
    click(&mut g, 50, 186);
    assert_eq!((g.scene, g.rollers.phase, g.rollers.log.len()), (Scene::Dyno, 1, 0), "pull again");
    pull(&mut g);
    let sharp = g.sheet.clone().unwrap();
    assert!((sharp.hp / stock.hp - 1.03).abs() < 0.002 && g.owned().unwrap().cond == 100, "{}", sharp.hp / stock.hp);
    // Ten degrees and leaned out knocks: less power than it promised, red in the log, and a sorer engine.
    g.state.cars[0].ecu = engine::game::Ecu { boost: 0, fuel: -3, timing: 10 };
    click(&mut g, 50, 186);
    pull(&mut g);
    let pinging = g.sheet.clone().unwrap();
    assert!(pinging.hp < sharp.hp && g.rollers.log.iter().any(|l| l.knock > 0) && g.owned().unwrap().cond < 100, "{:?}", (pinging.hp, g.owned().unwrap().cond));
    assert_eq!(g.note.as_deref(), Some("It was pinging. That hurts it."));
    // Held on the limiter on the rollers, it lets go, the same as on the strip.
    click(&mut g, 50, 186);
    let cond = g.owned().unwrap().cond;
    for _ in 0..700 { g.tick(KEY_UP, 0, 0); }
    assert_eq!((g.scene, g.owned().unwrap().cond), (Scene::Place, (cond - 20).max(25)));
    // Ten per cent more torque is ten per cent more of everything, and a quicker slip.
    g.state.cars[0].ecu = engine::game::Ecu::default();
    g.state.cars[0].cond = 100;
    g.state.cars[0].fitted.insert("Exhaust".into(), "pipe".into());
    let piped = g.dyno().unwrap();
    assert!((piped.hp / stock.hp - 1.1).abs() < 0.001 && piped.et < stock.et && piped.stock == stock.stock);
}

#[test]
fn a_car_with_a_picture_of_its_back_is_seen_on_the_road() {
    // The pack's car gets a back, all one colour that nothing else on a road is.
    let pink = shade(ramp("PINK"), 20);
    let mut bytes = files(&MANIFEST.replace("\"places\": {", ROADS).replace("\"do\": \"say:Hello.\"", "\"do\": \"road:pass\"").replace("\"price\": 350,", "\"price\": 350, \"rear\": { \"picture\": \"back.pic\" },"));
    let (name, data) = ("back.pic", pic(8, 5, &[(40, pink)]));
    bytes.extend((name.len() as u16).to_le_bytes());
    bytes.extend(name.as_bytes());
    bytes.extend((data.len() as u32).to_le_bytes());
    bytes.extend(data);
    bytes[4..8].copy_from_slice(&5u32.to_le_bytes());
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&bytes));
    g.seed(5);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    click(&mut g, 100, 10);
    let mut s = Screen::new();
    let count = |s: &Screen| (0..240).flat_map(|y| (0..320).map(move |x| (x, y))).filter(|&(x, y)| s.at(x, y) == pink).count();
    g.draw(&mut s);
    assert_eq!(count(&s), 0);
    // One going the same way, twenty metres up the road, in your lane: 1.7 metres is 17 pixels there.
    g.trip.as_mut().unwrap().others.push(engine::road::Other { z: 20.0, lane: 0.5, speed: 0.0, paint: "PINK", rig: false });
    g.draw(&mut s);
    let n = count(&s);
    assert!((100..260).contains(&n), "its back is on the screen: {n} pixels");
    // One coming the other way has no picture of its front, and is a plain shape.
    g.trip.as_mut().unwrap().others[0] = engine::road::Other { z: 20.0, lane: -0.5, speed: -1.0, paint: "PINK", rig: false };
    g.draw(&mut s);
    assert_eq!(count(&s), 0);
}

#[test]
fn nitrous_is_a_bottle_that_empties() {
    let manifest = MANIFEST.replace("\"places\": {", ROADS).replace("\"do\": \"say:Hello.\"", "\"do\": \"nitrous\"").replace("\"do\": \"lamps\", \"needs_car\": true", "\"do\": \"road:pass\"")
        .replace("\"puffer\": {", "\"juice\": { \"name\": \"Juice\", \"slot\": \"Nitrous\", \"price\": 50, \"nitrous\": { \"gain\": 50, \"seconds\": 4 } }, \"puffer\": {");
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(7);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    click(&mut g, 100, 10);
    assert_eq!(g.note.as_deref(), Some("There is no bottle on this car."));
    g.state.cars[0].fitted.insert("Nitrous".into(), "juice".into());
    click(&mut g, 100, 10);
    assert_eq!((g.owned().unwrap().bottle, g.state.cash), (4.0, 980), "a kit comes empty, and a fill is twenty dollars");
    click(&mut g, 100, 10);
    assert_eq!(g.note.as_deref(), Some("The bottle is full."));

    // On the strip it is quicker with the button down, and the bottle is what it costs.
    let drive = g.car().unwrap().drive.clone();
    let run = |spray: bool| {
        let mut r = Race::new(&drive, 1.0, None, 0, 0);
        (r.shot, r.bottle) = (0.5, 4.0);
        r.tick(Controls { stage: true, ..Controls::default() });
        for _ in 0..3000 {
            if r.phase == Phase::Slip { break; }
            let up = (r.gear == 0 && r.tree >= 51) || (r.gear > 0 && r.rpm > drive.redline - 250.0);
            r.tick(Controls { throttle: true, up, nitrous: spray, ..Controls::default() });
        }
        (r.et, r.bottle)
    };
    let ((dry, kept), (wet, left)) = (run(false), run(true));
    assert!(wet < dry - 200 && kept == 4.0 && left < 0.01, "{:?}", (dry, wet, left));

    // A race in the game takes it out of the car's own bottle.
    click(&mut g, 20, 10);
    click(&mut g, 150, 10);
    click(&mut g, 40, 186);
    g.tick(KEY_A, 0, 0);
    for _ in 0..4000 {
        let Some(r) = &g.race else { break };
        if r.phase == Phase::Slip { break; }
        let up = (r.gear == 0 && r.tree >= 51) || (r.gear > 0 && r.rpm > r.redline() - 250.0);
        g.tick(KEY_UP | KEY_N | if up { KEY_X } else { 0 }, 0, 0);
        g.tick(KEY_UP | KEY_N, 0, 0);
    }
    let mut s = Screen::new();
    g.draw(&mut s);
    g.tick(0, 0, 0);
    g.tick(KEY_A, 0, 0);
    g.tick(0, 0, 0);
    assert_eq!(g.owned().unwrap().bottle, 0.0);

    // And on a road it is more shove while it lasts.
    g.state.cars[0].bottle = 1.0;
    click(&mut g, 20, 10);
    click(&mut g, 60, 10);
    assert_eq!(g.scene, Scene::Road);
    g.tick(KEY_UP | KEY_N, 0, 0);
    assert_eq!(g.trip.as_ref().unwrap().spray, 0.5);
    g.draw(&mut s);
    for _ in 0..40 { g.tick(KEY_UP | KEY_N, 0, 0); }
    assert_eq!((g.trip.as_ref().unwrap().spray, g.owned().unwrap().bottle), (0.0, 0.0));
    assert!(Pack::parse(&files(&manifest.replace("\"gain\": 50", "\"gain\": 0"))).err().unwrap().contains("nitrous"));
}

#[test]
fn it_gets_dark_out_of_doors_and_bed_is_morning() {
    let manifest = MANIFEST.replace("\"do\": \"say:Hello.\"", "\"do\": \"sleep\"").replace("\"backdrop\": \"GRASS\",", "\"backdrop\": \"GRASS\", \"outdoors\": true,");
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(7);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    let mut s = Screen::new();
    // Indoors the hour makes no difference; out on the road it does.
    g.draw(&mut s);
    let shed = s.at(300, 100);
    g.state.hours = 3;
    g.draw(&mut s);
    assert_eq!(s.at(300, 100), shed);
    g.state.hours = 0;
    click(&mut g, 20, 10);
    g.draw(&mut s);
    let day = s.at(300, 100);
    g.state.hours = 3;
    g.draw(&mut s);
    assert_eq!((s.at(300, 100) / 32, s.at(300, 100) % 32), (day / 32, day % 32 - 5), "five shades down at dusk");
    // Bed at three o'clock of a four-hour day is one hour, and the morning of day two.
    click(&mut g, 20, 10);
    click(&mut g, 100, 10);
    assert_eq!((g.state.hours, g.day(), g.dark(), g.note.as_deref()), (4, 2, 0, Some("You slept. It is morning.")));
}

#[test]
fn space_shifts_and_some_days_it_rains() {
    // Space stages, then it is the shift, and on the timeslip it is nothing.
    let mut g = game();
    click(&mut g, 20, 10);
    click(&mut g, 150, 10);
    click(&mut g, 40, 186);
    let space = engine::game::KEY_SPACE | KEY_A;
    g.tick(space, 0, 0);
    g.tick(0, 0, 0);
    assert_eq!(g.race.as_ref().unwrap().phase, Phase::Tree, "space staged it");
    for _ in 0..4000 {
        let Some(r) = &g.race else { break };
        if r.phase == Phase::Slip { break; }
        let up = (r.gear == 0 && r.tree >= 51) || (r.gear > 0 && r.rpm > r.redline() - 250.0);
        g.tick(KEY_UP | if up { space } else { 0 }, 0, 0);
        g.tick(KEY_UP, 0, 0);
    }
    let r = g.race.as_ref().unwrap();
    assert!(r.phase == Phase::Slip && r.gear >= 3 && r.et > 0, "space went up through the gears: {:?}", (r.gear, r.et));
    g.tick(0, 0, 0);
    g.tick(space, 0, 0);
    g.tick(0, 0, 0);
    assert_eq!(g.scene, Scene::Race, "space does not throw the timeslip away");
    g.tick(KEY_A, 0, 0);
    assert_eq!(g.scene, Scene::Place);
    // Elsewhere it picks, as Enter does.
    g.tick(0, 20, 10);
    g.tick(space, 20, 10);
    assert_eq!(g.state.place, "shed");

    // Rain: never on the first day, on about as many of the rest as the pack says, and none if it says none.
    let wet = (1..=200).filter(|&d| g.rain_on(d)).count();
    assert!(!g.rain_on(1) && (12..=50).contains(&wet), "{wet} wet days in 200");
    let dry = Game::new(Pack::parse(&files(&MANIFEST.replace("\"hours_per_day\": 4", "\"hours_per_day\": 4, \"rain\": 0"))));
    assert!((1..=200).all(|d| !dry.rain_on(d)));
    // In the wet a bend throws the car further.
    let day = (2..200).find(|&d| g.rain_on(d)).unwrap();
    let before = g.pace().slide;
    g.state.hours = (day - 1) * 4;
    assert!(g.raining() && (g.pace().slide / before - 1.35).abs() < 0.001);
    let mut s = Screen::new();
    click(&mut g, 20, 10);
    g.draw(&mut s);
}

#[test]
fn a_car_in_the_paper_can_be_haggled_over() {
    let mut g = game();
    click(&mut g, 140, 10);
    click(&mut g, 200, 40);
    assert_eq!(g.scene, Scene::Paper(0));
    let mut s = Screen::new();
    g.draw(&mut s);
    // Every ad: ask until they hang up. Each yes is five per cent off the price in the ad, and no seller goes past a quarter.
    for i in 0..3 {
        let list = g.offers()[i].as_ref().unwrap().price;
        let mut yeses = 0;
        loop {
            click(&mut g, 270, 54 + i as i32 * 44 + 36);
            match &g.offers()[i] {
                Some(o) => {
                    yeses += 1;
                    assert_eq!(o.price, list * (100 - yeses * 5) / 100);
                    assert_eq!(g.note, Some(format!("\"All right. ${}.\"", o.price)));
                }
                None => break,
            }
            assert!(yeses <= 5);
        }
        assert!((1..=5).contains(&yeses), "everyone gives something: {yeses}");
        assert_eq!(g.note.as_deref(), Some("\"Forget it.\" He hung up."));
    }
    assert_eq!((g.state.cash, g.state.cars.len()), (1000, 1), "asking costs nothing, and buys nothing");
    // The next day's ads are at the price they are printed at.
    g.state.hours = 4;
    assert!(g.offers().iter().all(|o| o.is_some()));
    // Picking the rest of an ad still buys it, at what it was talked down to.
    click(&mut g, 270, 54 + 36);
    let price = g.offers()[0].as_ref().unwrap().price;
    click(&mut g, 100, 60);
    assert_eq!((g.state.cash, g.state.cars.len()), (1000 - price, 2));
}

#[test]
fn an_automatic_shifts_itself_and_a_manual_is_shifted_on_the_road() {
    // On the strip an automatic wants only a foot: it leaves a moment after the green and changes up.
    let mut drive = engine::pack::Drive::default();
    drive.auto = true;
    let mut r = Race::new(&drive, 1.0, None, 0, 0);
    r.tick(Controls { stage: true, ..Controls::default() });
    for _ in 0..3000 {
        if r.phase == Phase::Slip { break; }
        r.tick(Controls { throttle: true, ..Controls::default() });
    }
    assert!(r.phase == Phase::Slip && !r.red && r.rt > 200 && r.gear >= 4 && r.et > 0 && !r.blown, "{:?}", (r.rt, r.gear, r.et));

    // On the road the test car has a gearbox: first gear is good for about forty, and no more until it is changed.
    let mut g = road_game();
    click(&mut g, 100, 10);
    let b = g.trip.as_ref().unwrap().gearbox.clone().unwrap();
    assert_eq!((b.gear, b.ratios.len(), b.redline), (0, 5, 7200.0));
    for _ in 0..400 {
        let t = g.trip.as_ref().unwrap();
        g.tick(KEY_UP | if t.x > 0.55 { 4 } else if t.x < 0.45 { 8 } else { 0 }, 0, 0);
    }
    let t = g.trip.as_ref().unwrap();
    assert!((t.speed * 2.237 - 39.0).abs() < 3.0 && t.gearbox.as_ref().unwrap().rpm >= 7200.0, "{}", t.speed * 2.237);
    // Space is the next gear, and it pulls again.
    g.tick(KEY_UP | engine::game::KEY_SPACE | KEY_A, 0, 0);
    assert_eq!(g.trip.as_ref().unwrap().gearbox.as_ref().unwrap().gear, 1);
    for _ in 0..60 { g.tick(KEY_UP, 0, 0); }
    assert!(g.trip.as_ref().unwrap().speed * 2.237 > 45.0);
    // Down a gear is refused when it would send the revs past the redline.
    g.tick(KEY_UP | 128, 0, 0);
    assert_eq!(g.trip.as_ref().unwrap().gearbox.as_ref().unwrap().gear, 1);
    // A part can make it an automatic, and then there is nothing to work.
    let auto = MANIFEST.replace("\"places\": {", ROADS).replace("\"torque\": 10 }", "\"torque\": 10, \"auto\": true }");
    let mut g = Game::new(Pack::parse(&files(&auto)));
    g.tick(KEY_A, 160, 100);
    assert!(g.gearbox().is_some());
    g.state.cars[0].fitted.insert("Exhaust".into(), "pipe".into());
    assert!(g.gearbox().is_none());
}

#[test]
fn a_town_can_have_hills_and_the_default_one_has_a_mountain_road() {
    // Flat unless it says otherwise; with hills, never higher than they are said to be, and not all one height.
    let g = town_game();
    let flat = g.pack.manifest.maps["grid"].clone();
    assert_eq!(flat.height(40.0, 90.0), 0.0);
    let mut hilly = flat.clone();
    hilly.hills = 9.0;
    let heights: Vec<f64> = (0..40).map(|i| hilly.height(i as f64 * 17.0, i as f64 * 5.0)).collect();
    assert!(heights.iter().all(|h| h.abs() <= 9.0) && heights.iter().any(|&h| h > 3.0) && heights.iter().any(|&h| h < -3.0));
    // The town still draws on its hills, with its map in the corner.
    let mut g = town_game();
    g.pack.manifest.maps.get_mut("grid").unwrap().hills = 9.0;
    click(&mut g, 100, 10);
    let mut s = Screen::new();
    g.draw(&mut s);
    assert_eq!(s.at(43, 15), 0, "the corner map's border");
    let Some(bytes) = default_pack() else { return };
    let pack = Pack::parse(&bytes).unwrap();
    let up = &pack.manifest.roads["mountain"];
    assert!(up.parts.iter().map(|p| p.hill).sum::<f64>() > 200.0 && up.parts.iter().any(|p| p.curve.abs() == 1.0) && pack.manifest.places.contains_key("summit"));
    assert!(pack.manifest.maps["town"].hills > 0.0);
}

#[test]
fn the_shootout_is_a_knockout() {
    let with = |every: i32| MANIFEST.replace("\"do\": \"say:Hello.\"", "\"do\": \"shootout\"").replace("\"hours_per_day\": 4 }", "\"hours_per_day\": 8 }")
        .replace("\"bets\": [10, 25],", &format!("\"bets\": [10, 25], \"shootout\": {{ \"entry\": 50, \"purse\": 300, \"rounds\": 2, \"every\": {every} }},"));
    let start = |every: i32| {
        engine::host::store(b"");
        let mut g = Game::new(Pack::parse(&files(&with(every))));
        g.seed(7);
        g.tick(KEY_A, 160, 100);
        g.tick(0, 160, 100);
        g
    };
    // Not today, if it is held every seventh day.
    let mut g = start(7);
    click(&mut g, 100, 10);
    assert_eq!((g.scene, g.note.as_deref(), g.state.cash), (Scene::Place, Some("The next shootout is on day 7."), 1000));

    // Fifty dollars in, the slow one first and then the next up, and three hundred for winning both.
    let mut g = start(1);
    click(&mut g, 100, 10);
    assert_eq!((g.scene, g.state.cash, g.bracket, g.who), (Scene::Race, 950, Some(0), 0));
    drive(&mut g);
    assert_eq!((g.scene, g.bracket, g.who, g.state.hours), (Scene::Race, Some(1), 1, 2), "straight into the next round");
    drive(&mut g);
    assert_eq!((g.scene, g.bracket, g.state.cash, g.state.hours, g.state.wins), (Scene::Place, None, 1250, 4, 2));
    assert_eq!(g.state.rung, 0, "it is not the ladder");
    click(&mut g, 100, 10);
    assert_eq!(g.note.as_deref(), Some("You have had your go today."));
    // The next day, backing out of the first round is being out of it.
    g.state.hours = 8;
    click(&mut g, 100, 10);
    assert_eq!(g.scene, Scene::Race);
    g.tick(KEY_B, 0, 0);
    assert_eq!((g.scene, g.bracket, g.state.cash, g.note.as_deref(), g.state.losses), (Scene::Place, None, 1200, Some("Out in round 1. There is always the next."), 1));
    assert!(Pack::parse(&files(&with(0))).err().unwrap().contains("shootout"));
}

#[test]
fn the_law_is_in_town_too() {
    let mut g = town_game();
    {
        let map = g.pack.manifest.maps.get_mut("grid").unwrap();
        (map.cops, map.limit, map.ticket) = (1, 30.0, 20);
    }
    click(&mut g, 100, 10);
    assert_eq!(g.scene, Scene::Drive);
    // One of them, parked up the street; past him at twice the limit.
    g.town.as_mut().unwrap().cops = vec![(110.0, 2.0)];
    for _ in 0..400 {
        if g.town.as_ref().unwrap().chase.is_some() { break; }
        g.tick(KEY_UP, 0, 0);
    }
    let t = g.town.as_ref().unwrap();
    assert!(t.chase.is_some() && t.x > 70.0 && t.speed * 2.237 > 40.0 && g.note.as_deref() == Some("Police! Pull over."), "{:?}", (t.x, t.speed));
    let mut s = Screen::new();
    g.draw(&mut s);
    g.tick(KEY_B, 0, 0);
    assert_eq!((g.scene, g.note.as_deref()), (Scene::Drive, Some("Not with him behind you.")));
    for _ in 0..300 { g.tick(2, 0, 0); }
    let paid = 1000 - g.state.cash;
    assert!(g.town.as_ref().unwrap().chase.is_none() && (20 + 2 * 10..20 + 2 * 60).contains(&paid), "{paid}");
    assert_eq!(g.note, Some(format!("Speeding. That is a ${paid} ticket.")));
    // With no police there is no limit.
    let mut g = town_game();
    click(&mut g, 100, 10);
    for _ in 0..200 { g.tick(KEY_UP, 0, 0); }
    assert!(g.town.as_ref().unwrap().chase.is_none() && g.town.as_ref().unwrap().cops.is_empty());
}

#[test]
fn the_street_out_of_town_is_the_road() {
    let mut g = road_game();
    click(&mut g, 100, 10);
    cruise(&mut g, 45.0, &|g| g.scene != Scene::Road);
    assert_eq!(g.scene, Scene::Drive);
    // Turned about and driven back off the end of the street: on the road again without stopping or asking.
    let (gx, gz, heading) = g.pack.manifest.maps["grid"].gate("pass").unwrap();
    assert_eq!((gx, gz, heading), (130.0, -2.0, 0.0));
    let t = g.town.as_mut().unwrap();
    (t.x, t.z, t.heading, t.speed) = (126.0, 6.0, 180.0, 12.0);
    for _ in 0..30 {
        if g.scene == Scene::Road { break; }
        g.tick(KEY_UP, 0, 0);
    }
    let trip = g.trip.as_ref().expect("out of town and onto the road");
    assert!(g.scene == Scene::Road && trip.backwards && trip.speed > 10.0);
    // Along the edge of town, and not out of it, nothing happens.
    let mut g = road_game();
    click(&mut g, 100, 10);
    cruise(&mut g, 45.0, &|g| g.scene != Scene::Road);
    let t = g.town.as_mut().unwrap();
    (t.x, t.z, t.heading, t.speed) = (126.0, 0.0, 90.0, 12.0);
    g.tick(KEY_UP, 0, 0);
    assert_eq!(g.scene, Scene::Drive);
}

#[test]
fn a_poker_run_is_five_cards_and_the_best_hand() {
    use engine::game::cards::{name, rank, short};
    // Hands, from nothing up: each beats the one before.
    let c = |text: &str| -> Vec<u8> { text.split(' ').map(|t| (b"23456789TJQKA".iter().position(|&r| r == t.as_bytes()[0]).unwrap() + 13 * b"CDHS".iter().position(|&s| s == t.as_bytes()[1]).unwrap()) as u8).collect() };
    let hands = ["KS 9H 7D 4C 2S", "AS 9H 7D 4C 2S", "4S 4H AD KC QS", "KS KH 7D 4C 2S", "2S 2H 3D 3C 4S", "5S 5H 5D AC KS", "AS 2H 3D 4C 5S", "6S 2H 3D 4C 5D", "2H 9H JH KH 4H", "3S 3H 3D 2C 2S", "9S 9H 9D 9C 2S", "5C 6C 7C 8C 9C"];
    for pair in hands.windows(2) {
        assert!(rank(&c(pair[1])) > rank(&c(pair[0])), "{} should beat {}", pair[1], pair[0]);
    }
    assert_eq!((name(&rank(&c(hands[3]))), name(&rank(&c(hands[5]))), name(&rank(&c(hands[0]))), short(c("TH")[0])), ("A pair of kings".into(), "Three fives".into(), "King high".into(), "TH".to_string()));

    let manifest = MANIFEST.replace("\"places\": {", ROADS).replace("\"do\": \"say:Hello.\"", "\"do\": \"pokerrun:pass\"");
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(11);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    click(&mut g, 100, 10);
    assert_eq!((g.scene, g.state.cash), (Scene::Road, 980));
    assert!(g.trip.as_ref().unwrap().poker);
    let mut s = Screen::new();
    cruise(&mut g, 40.0, &|g| g.trip.as_ref().is_none_or(|t| t.cards.len() == 3));
    g.draw(&mut s);
    assert!(g.note.as_deref().is_some_and(|n| n.starts_with("Card 3: the ")), "{:?}", g.note);
    let mut hand = Vec::new();
    cruise(&mut g, 40.0, &|g| { g.scene != Scene::Road });
    hand.extend(g.note.clone());
    // Back where it started, two hours on, five cards played: twenty dollars down, or a hundred up.
    assert_eq!((g.scene, g.state.place.as_str(), g.state.hours), (Scene::Place, "shed", 2));
    assert!(g.state.cash == 980 || g.state.cash == 1100, "{} {:?}", g.state.cash, hand);
    // Dropping out leaves the money in the pot.
    click(&mut g, 100, 10);
    g.tick(KEY_B, 0, 0);
    assert_eq!((g.scene, g.note.as_deref()), (Scene::Place, Some("You dropped out of the run.")));
}

#[test]
fn a_road_can_have_water_trailers_and_a_ladder_to_climb_first() {
    let g = road_game();
    let mut road = g.pack.manifest.roads["pass"].clone();
    road.parts[0].right = "water".into();
    road.rigs = 1.0;
    road.traffic = 10;
    let mut t = engine::road::Trip::new("pass", &road, false, 4);
    assert!(t.others.iter().all(|o| o.rig) && t.seg(50.0).right == engine::road::Side::Water);
    // Into the lake is as bad as into the trees.
    let mut wet = false;
    t.others.clear();
    for _ in 0..200 {
        wet |= t.tick(&road, Pedals { go: true, right: true, ..Pedals::default() }).crashed;
    }
    assert!(wet);
    // The back of a trailer is a long way behind the truck.
    let mut t = engine::road::Trip::new("pass", &road, false, 4);
    t.others = vec![engine::road::Other { z: 30.0, lane: 0.5, speed: 0.0, paint: "RED", rig: true }];
    let mut at = None;
    for _ in 0..300 {
        if t.tick(&road, Pedals { go: true, left: t.x > 0.55, right: t.x < 0.45, ..Pedals::default() }).crashed { at = Some(t.z); break; }
    }
    assert!(at.is_some_and(|z| z < 23.0), "{at:?}");
    // A road that asks for two wins is shut until there are two, from its first end only.
    let manifest = MANIFEST.replace("\"places\": {", &ROADS.replace("\"limit\": 50,", "\"limit\": 50, \"rung\": 2,")).replace("\"do\": \"say:Hello.\"", "\"do\": \"road:pass\"");
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    click(&mut g, 100, 10);
    assert_eq!((g.scene, g.note.as_deref()), (Scene::Place, Some("Not yet. Beat 2 more here first.")));
    g.state.rung = 2;
    click(&mut g, 100, 10);
    assert_eq!(g.scene, Scene::Road);
    let Some(bytes) = default_pack() else { return };
    let pack = Pack::parse(&bytes).unwrap();
    let m = &pack.manifest;
    assert!(m.roads["ohio"].rung > 0 && m.roads["ohio"].rigs > 0.0 && m.places.contains_key("norwalk") && m.maps["town"].blocks.values().any(|b| b.water));
    assert!(["ridge", "pike", "ohio"].iter().all(|r| m.maps["town"].gate(r).is_some()), "every road out of town leaves from its edge");
}

#[test]
fn bodies_get_knocked_about_and_tyres_wear() {
    let manifest = MANIFEST.replace("\"places\": {", ROADS).replace("\"do\": \"say:Hello.\"", "\"do\": \"bodywork\"").replace("\"do\": \"lamps\", \"needs_car\": true", "\"do\": \"tyres\"");
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(7);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    click(&mut g, 100, 10);
    assert_eq!(g.note.as_deref(), Some("There is not a mark on it."));
    click(&mut g, 60, 10);
    assert_eq!(g.note.as_deref(), Some("They are nearly new."));
    let car = g.car().unwrap().clone();
    let (worth, grip, top) = (g.value(g.owned().unwrap()), g.tuned_drive(g.owned().unwrap(), &car).grip, g.pace().top);
    // Three knocks and a lot of miles later it is worth less, slower, and has less bite.
    for _ in 0..3 { g.dent(); }
    g.scrub(60.0);
    let o = g.owned().unwrap().clone();
    assert_eq!((o.body, o.tyres), (24, 60.0));
    assert!(g.value(&o) < worth && g.pace().top < top && (g.tuned_drive(&o, &car).grip / grip - 0.82).abs() < 0.001);
    let mut s = Screen::new();
    g.draw(&mut s);
    // Straightened for twenty dollars and four a point; tyres for eighty.
    click(&mut g, 100, 10);
    assert_eq!((g.owned().unwrap().body, g.state.cash, g.state.hours), (0, 1000 - 116, 3));
    click(&mut g, 60, 10);
    assert_eq!((g.owned().unwrap().tyres, g.state.cash), (0.0, 1000 - 196));
    assert_eq!(g.value(g.owned().unwrap()), worth);
}

#[test]
fn a_car_with_a_build_sheet_is_made_of_its_parts() {
    use engine::game::{base, cond_of, worn};
    let manifest = MANIFEST.replace("\"paint\": \"RED\", \"paint_ramps\": { \"mid\": \"BLUE\" } }", "\"paint\": \"RED\", \"paint_ramps\": { \"mid\": \"BLUE\" },
        \"stock\": { \"Shocks\": \"struts\", \"Axles\": \"axle\", \"Seats\": \"seat\" }, \"optional\": [\"Seats\"] }")
        .replace("\"puffer\": {", "\"struts\": { \"name\": \"Struts\", \"slot\": \"Shocks\", \"price\": 100 }, \"seat\": { \"name\": \"Seat\", \"slot\": \"Seats\", \"price\": 40 },
        \"axle\": { \"name\": \"Axle\", \"slot\": \"Axles\", \"price\": 80, \"holds\": 150 }, \"axle2\": { \"name\": \"Big axle\", \"slot\": \"Axles\", \"price\": 200, \"holds\": 900 },
        \"cam\": { \"name\": \"Cam\", \"slot\": \"Cams\", \"price\": 90, \"needs\": [\"stiffer springs\"] }, \"spr\": { \"name\": \"Springs\", \"slot\": \"Springs\", \"price\": 30, \"gives\": [\"stiffer springs\"] }, \"puffer\": {");
    assert_eq!((base("valves@40"), cond_of("valves@40"), cond_of("valves"), worn("valves@40", 100), worn("valves", 7)), ("valves", 40, 100, "valves".to_string(), "valves@7".to_string()));
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(7);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    // It comes with what is on its sheet, each part in some state of its own, none of them new on a tired car.
    let o = g.owned().unwrap().clone();
    assert!(o.sheet && o.fitted.len() == 3 && ["Shocks", "Axles", "Seats"].iter().all(|s| (20..100).contains(&cond_of(&o.fitted[*s]))), "{:?}", o.fitted);
    assert_eq!(g.wont_run(), None);
    // It will run without its seat, not without its shocks, and not on a part that is done for.
    let seat = g.state.cars[0].fitted.remove("Seats").unwrap();
    assert_eq!(g.wont_run(), None);
    let struts = g.state.cars[0].fitted.remove("Shocks").unwrap();
    assert_eq!(g.wont_run().as_deref(), Some("It has no shocks."));
    g.state.cars[0].fitted.insert("Shocks".into(), worn(&struts, 0));
    assert_eq!(g.wont_run().as_deref(), Some("The struts is done for."));
    g.state.cars[0].fitted.insert("Shocks".into(), struts.clone());
    // A cam that wants stiffer springs wants them on the car.
    g.state.cars[0].fitted.insert("Cams".into(), "cam".into());
    assert_eq!(g.wont_run().as_deref(), Some("Cam needs stiffer springs."));
    g.state.cars[0].fitted.insert("Springs".into(), "spr".into());
    assert_eq!(g.wont_run(), None);
    // A used part is worth what is left of it.
    assert!(g.spare_offer(&seat) < g.spare_offer("seat") && g.spare_offer("seat@0") == 0);
    // The engine makes more than the axle will take: launch after launch, and it goes. The big one never does.
    assert!(g.peak_torque() > 150.0 && g.peak_torque() < 900.0);
    let mut broke = None;
    for _ in 0..200 {
        broke = g.strain_driveline();
        if broke.is_some() { break; }
    }
    assert_eq!((broke.as_deref(), g.owned().unwrap().fitted["Axles"].as_str(), g.wont_run().as_deref()), (Some("Axle"), "axle@0", Some("The axle is done for.")));
    g.state.cars[0].fitted.insert("Axles".into(), "axle2".into());
    assert!((0..300).all(|_| g.strain_driveline().is_none()));
    // The bench lists it all, with what state each is in.
    g.state.spares = vec![seat, "axle@0".into()];
    click(&mut g, 60, 10);
    let mut s = Screen::new();
    g.draw(&mut s);
    // A build sheet has to name parts that are there, in the slots they go in.
    assert!(Pack::parse(&files(&manifest.replace("\"Axles\": \"axle\"", "\"Axles\": \"struts\""))).err().unwrap().contains("goes in Shocks"));
    assert!(Pack::parse(&files(&manifest.replace("\"optional\": [\"Seats\"]", "\"optional\": [\"Wings\"]"))).err().unwrap().contains("Wings"));

    // The default car: whole as it comes; a big turbo wants its manifold, its housing and its pump; and then it breaks axles.
    let Some(bytes) = default_pack() else { return };
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&bytes));
    let mut o = Owned::new("umbra", None, 100);
    g.outfit(&mut o);
    assert!(o.sheet && o.fitted.len() >= 30);
    g.state.cars.push(o);
    g.state.cur = Some(0);
    assert_eq!(g.wont_run(), None);
    assert!((0..300).all(|_| g.strain_driveline().is_none()), "as it left the factory, it holds");
    g.state.cars[0].fitted.insert("Turbo".into(), "big-turbo".into());
    assert!(g.wont_run().is_some_and(|w| w.contains("needs")));
    for (slot, part) in [("Exhaust", "header"), ("O2 housing", "o2-big"), ("Fuel pump", "pump-255")] {
        g.state.cars[0].fitted.insert(slot.into(), part.into());
    }
    assert_eq!(g.wont_run(), None);
    // The stock clutch slips at that, which is all that saves the axles; with one that holds, they are first to go.
    let o = g.owned().unwrap().clone();
    assert!(g.slip(&o) < 1.0);
    g.state.cars[0].fitted.insert("Clutch".into(), "clutch-twin".into());
    let broke = (0..300).find_map(|_| g.strain_driveline());
    assert_eq!(broke.as_deref(), Some("Front axles"), "the weakest goes first");
}

#[test]
fn cars_are_sold_as_they_stand() {
    use engine::game::cond_of;
    let manifest = MANIFEST.replace("\"paint\": \"RED\", \"paint_ramps\": { \"mid\": \"BLUE\" } }", "\"paint\": \"RED\", \"paint_ramps\": { \"mid\": \"BLUE\" },
        \"stock\": { \"Shocks\": \"struts\", \"Axles\": \"axle\", \"Seats\": \"seat\" }, \"optional\": [\"Seats\"] }")
        .replace("\"puffer\": {", "\"struts\": { \"name\": \"Struts\", \"slot\": \"Shocks\", \"price\": 100 }, \"seat\": { \"name\": \"Seat\", \"slot\": \"Seats\", \"price\": 40 },
        \"axle\": { \"name\": \"Axle\", \"slot\": \"Axles\", \"price\": 80 }, \"axle2\": { \"name\": \"Big axle\", \"slot\": \"Axles\", \"price\": 200 }, \"puffer\": {")
        .replace("\"bets\": [10, 25],", "\"bets\": [10, 25], \"faults\": [ { \"says\": \"Bent an axle.\", \"breaks\": [\"Axles\"], \"hidden\": [\"Shocks\"] } ], \"sellers\": [ { \"from\": \"Ohio\", \"ship\": 120 } ],");
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(7);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    // A car that is its parts is worth its shell and its parts, and less than it cost.
    let worth = g.value(g.owned().unwrap());
    assert!(worth > 60 && worth < 350, "{worth}");
    // Two a day, the same two however often the paper is opened, each from somewhere, saying something.
    let (a, b) = (g.projects(), g.projects());
    assert_eq!(a.len(), 2);
    for (x, y) in a.iter().zip(&b) {
        let (x, y) = (x.as_ref().unwrap(), y.as_ref().unwrap());
        assert_eq!((x.price, &x.says, &x.owned.fitted), (y.price, &y.says, &y.owned.fitted));
        assert!(x.price > 0 && x.ship == 120 && x.from == "Ohio" && x.owned.sheet && x.says.iter().any(|s| s == "Bent an axle."));
        // What the ad does not say went with it: the shocks, unless someone has had them off already.
        assert!(x.owned.fitted.get("Shocks").is_none_or(|id| cond_of(id) == 0), "{:?}", x.owned.fitted);
    }
    // Over many days: nearly all of them do not run, and now and then the axle the ad wrote off is fine.
    let (mut dead, mut lucky, mut all) = (0, 0, 0);
    for day in 0..60 {
        g.state.hours = day * 4;
        for p in g.projects().into_iter().flatten() {
            all += 1;
            lucky += usize::from(p.owned.fitted.get("Axles").is_some_and(|id| cond_of(id) > 0));
            g.state.cars = vec![p.owned];
            g.state.cur = Some(0);
            dead += usize::from(g.wont_run().is_some());
        }
    }
    assert!(all == 120 && dead > 100 && lucky > 3 && lucky < 60, "{:?}", (all, dead, lucky));
    // Bought from the AS IS page: the price and the haul, and it is in the yard with what was wrong with it.
    g.state.hours = 0;
    g.state.cars.truncate(1);
    click(&mut g, 140, 10);
    click(&mut g, 90, 40);
    assert_eq!(g.scene, Scene::Paper(3));
    let mut s = Screen::new();
    g.draw(&mut s);
    let first = g.projects().remove(0).unwrap();
    click(&mut g, 270, 54 + 36);
    let talked = g.projects().remove(0);
    assert!(talked.as_ref().is_none_or(|p| p.price == first.price * 95 / 100));
    let pay = g.projects().into_iter().position(|p| p.is_some()).unwrap();
    let it = g.projects().remove(pay).unwrap();
    let cash = g.state.cash;
    click(&mut g, 100, 54 + pay as i32 * 62 + 10);
    assert_eq!((g.state.cars.len(), g.state.cash, g.projects()[pay].is_none()), (2, cash - it.price - 120, true));
    assert_eq!(g.state.cars[1].fitted, it.owned.fitted);
    // The other tabs still turn from there.
    click(&mut g, 280, 40);
    assert_eq!(g.scene, Scene::Paper(1));
    // A pack with nothing that can be wrong has none.
    assert!(Game::new(Pack::parse(&files(MANIFEST))).projects().is_empty());
    assert!(Pack::parse(&files(&manifest.replace("\"breaks\": [\"Axles\"], \"hidden\": [\"Shocks\"]", "\"breaks\": []"))).err().unwrap().contains("fault"));
}

#[test]
fn a_clutch_slips_before_it_goes() {
    use engine::game::cond_of;
    let manifest = MANIFEST.replace("\"paint\": \"RED\", \"paint_ramps\": { \"mid\": \"BLUE\" } }", "\"paint\": \"RED\", \"paint_ramps\": { \"mid\": \"BLUE\" },
        \"stock\": { \"Clutch\": \"disc\", \"Axles\": \"axle\" } }")
        .replace("\"puffer\": {", "\"disc\": { \"name\": \"Clutch\", \"slot\": \"Clutch\", \"price\": 100, \"holds\": 200, \"slips\": true },
        \"twin\": { \"name\": \"Twin disc\", \"slot\": \"Clutch\", \"price\": 400, \"holds\": 900, \"slips\": true },
        \"axle\": { \"name\": \"Axle\", \"slot\": \"Axles\", \"price\": 80, \"holds\": 250 }, \"puffer\": {");
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(7);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    g.state.cars[0].cond = 100;
    g.state.cars[0].fitted.insert("Clutch".into(), "disc".into());
    g.state.cars[0].fitted.insert("Axles".into(), "axle".into());
    // The engine makes 314 and the clutch holds 200: that is all the car gets, and all the axle sees.
    let o = g.owned().unwrap().clone();
    assert!((g.peak_torque() - 314.0).abs() < 1.0 && (g.slip(&o) - 200.0 / 314.0).abs() < 0.001);
    assert!((g.tune(&o).0 - 200.0 / 314.0).abs() < 0.001 && g.dyno().unwrap().torque < 201.0);
    // Launch after launch it burns, holds less, and is done for; the axle behind it never breaks.
    let mut launches = 0;
    let gone = loop {
        launches += 1;
        if let Some(name) = g.strain_driveline() { break name; }
        assert!(launches < 40);
    };
    assert_eq!((gone.as_str(), cond_of(&g.owned().unwrap().fitted["Clutch"]), cond_of(&g.owned().unwrap().fitted["Axles"])), ("Clutch", 0, 100));
    assert!(launches >= 3 && g.wont_run().as_deref() == Some("The clutch is done for."), "{launches}");
    // A clutch that holds passes it all on, and now it is the axle that goes.
    g.state.cars[0].fitted.insert("Clutch".into(), "twin".into());
    let o = g.owned().unwrap().clone();
    assert_eq!(g.slip(&o), 1.0);
    let broke = (0..200).find_map(|_| g.strain_driveline());
    assert_eq!((broke.as_deref(), cond_of(&g.owned().unwrap().fitted["Clutch"])), (Some("Axle"), 100));
}

#[test]
fn a_dry_tank_and_a_flat_battery_are_not_the_end() {
    // Dry at home: the door out is a walk to the pump with a can, and then it will go.
    let mut g = town_game();
    g.state.cars[0].gas = 0.0;
    click(&mut g, 100, 10);
    assert_eq!((g.scene, g.owned().unwrap().gas, g.state.cash, g.state.hours, g.note.as_deref()), (Scene::Place, 1.0, 998, 1, Some("A can from the pump: a gallon, $2.")));
    click(&mut g, 100, 10);
    assert_eq!(g.scene, Scene::Drive);

    let manifest = MANIFEST.replace("\"do\": \"say:Hello.\"", "\"do\": \"charge\"").replace("\"paint\": \"RED\", \"paint_ramps\": { \"mid\": \"BLUE\" } }", "\"paint\": \"RED\", \"paint_ramps\": { \"mid\": \"BLUE\" }, \"stock\": { \"Battery\": \"cell\" } }")
        .replace("\"puffer\": {", "\"cell\": { \"name\": \"Battery\", \"slot\": \"Battery\", \"price\": 60, \"charges\": true }, \"puffer\": {")
        .replace("\"bets\": [10, 25],", "\"bets\": [10, 25], \"faults\": [ { \"says\": \"Ran when parked.\", \"hidden\": [\"Battery\"] } ],");
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(7);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    // Flat, it will not start; five dollars and two hours on the charger, and it will.
    g.state.cars[0].fitted.insert("Battery".into(), "cell@10".into());
    assert_eq!(g.wont_run().as_deref(), Some("The battery is flat. Charge it."));
    click(&mut g, 100, 10);
    assert_eq!((g.owned().unwrap().fitted["Battery"].as_str(), g.state.cash, g.state.hours, g.wont_run()), ("cell@90", 995, 2, None));
    click(&mut g, 100, 10);
    assert_eq!(g.note.as_deref(), Some("It is charged."));
    g.state.cars[0].fitted.insert("Battery".into(), "cell@0".into());
    click(&mut g, 100, 10);
    assert_eq!(g.note.as_deref(), Some("It will not take a charge. It is done."));
    // A car sold as it stands whose only trouble is a battery that sat is a charge away from running.
    let p = g.projects().into_iter().flatten().find(|p| p.owned.fitted.contains_key("Battery")).unwrap();
    assert_eq!(p.owned.fitted["Battery"], "cell@8");
}

#[test]
fn parts_wear_with_use_and_a_battery_runs_down() {
    let manifest = MANIFEST.replace("\"do\": \"say:Hello.\"", "\"do\": \"sleep\"")
        .replace("\"paint\": \"RED\", \"paint_ramps\": { \"mid\": \"BLUE\" } }", "\"paint\": \"RED\", \"paint_ramps\": { \"mid\": \"BLUE\" }, \"stock\": { \"Battery\": \"cell\", \"Belt\": \"belt\", \"Valves\": \"valves\" } }")
        .replace("\"puffer\": {", "\"cell\": { \"name\": \"Battery\", \"slot\": \"Battery\", \"price\": 60, \"charges\": true }, \"belt\": { \"name\": \"Timing belt\", \"slot\": \"Belt\", \"price\": 45, \"wears\": 30, \"takes\": [\"Valves\"] }, \"valves\": { \"name\": \"Valves\", \"slot\": \"Valves\", \"price\": 160 }, \"puffer\": {");
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&files(&manifest)));
    g.seed(7);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    for (slot, id) in [("Battery", "cell@50"), ("Belt", "belt"), ("Valves", "valves")] {
        g.state.cars[0].fitted.insert(slot.into(), id.into());
    }
    let power = g.tune_engine(&g.state.cars[0]).0;
    // A pass takes thirty off the belt, and the engine running puts eight in the battery.
    assert_eq!(g.use_parts(1.0), None);
    assert_eq!((g.state.cars[0].fitted["Belt"].as_str(), g.state.cars[0].fitted["Battery"].as_str()), ("belt@70", "cell@58"));
    assert_eq!(g.tune_engine(&g.state.cars[0]).0, power);
    // Past half it costs power, and under a quarter it says so.
    assert_eq!(g.use_parts(1.0), None);
    assert!(g.tune_engine(&g.state.cars[0]).0 < power);
    assert_eq!(g.use_parts(1.0), Some(("Timing belt".to_string(), false)));
    // When it goes it takes the valves, and the car will not run.
    assert_eq!(g.use_parts(1.0), Some(("Timing belt".to_string(), true)));
    assert_eq!((g.state.cars[0].fitted["Belt"].as_str(), g.state.cars[0].fitted["Valves"].as_str()), ("belt@0", "valves@0"));
    assert!(g.wont_run().is_some());
    // Four kilometres of road are a pass.
    g.state.cars[0].fitted.insert("Belt".into(), "belt".into());
    for _ in 0..8 { g.roll(0.5); }
    assert_eq!(g.state.cars[0].fitted["Belt"], "belt@70");

    // The lamps left on overnight take most of a battery; a car in the yard loses a little a day.
    let mut parked = g.state.cars[0].clone();
    parked.fitted.insert("Battery".into(), "cell@6".into());
    g.state.cars.push(parked);
    g.state.cars[0].fitted.insert("Battery".into(), "cell".into());
    g.lamps = true;
    click(&mut g, 100, 10);
    assert_eq!((g.state.cars[0].fitted["Battery"].as_str(), g.state.cars[1].fitted["Battery"].as_str(), g.lamps, g.note.as_deref()), ("cell@55", "cell@5", false, Some("You left the lamps on all night.")));
    click(&mut g, 100, 10);
    assert_eq!((g.state.cars[0].fitted["Battery"].as_str(), g.note.as_deref()), ("cell@55", Some("You slept. It is morning.")));
}

#[test]
fn what_is_under_the_hood_is_on_the_build_sheet() {
    let Some(bytes) = default_pack() else { return };
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&bytes));
    let mut o = Owned::new("umbra", None, 100);
    g.outfit(&mut o);
    for slot in ["Intake", "Exhaust", "Turbo", "Intercooler"] {
        assert!(o.fitted.contains_key(slot), "{slot} is on the sheet");
    }
    g.state.cars = vec![o];
    g.state.cur = Some(0);
    assert_eq!(g.wont_run(), None);
    // The stock turbo wants the stock manifold: a header on its own will not do.
    g.state.cars[0].fitted.insert("Exhaust".into(), "header".into());
    assert!(g.wont_run().is_some_and(|w| w == "Stock turbo needs a stock manifold."));
    // Off the car it is a real part, and nothing is left lying on the floor in its place.
    g.state.cars[0].fitted.remove("Exhaust");
    g.state.cars[0].off.push("Exhaust".into());
    assert!(!g.floor().iter().any(|l| matches!(l, Loose::Stock(_))));

    // A turbo sells best with its manifold and its housing; alone it fetches less.
    g.state.spares = vec!["s-turbo".into(), "s-exmani".into(), "s-o2".into(), "s-plugs".into()];
    let apart: i32 = ["s-turbo", "s-exmani", "s-o2"].iter().map(|p| g.spare_offer(p)).sum();
    let (paid, set) = g.sale(0);
    assert_eq!((set.len(), paid), (3, apart * 115 / 100));
    g.state.spares.remove(2);
    assert_eq!(g.sale(0), (g.spare_offer("s-turbo") * 80 / 100, vec![0]));
    assert_eq!(g.sale(2), (g.spare_offer("s-plugs"), vec![2]));
    g.state.spares.insert(2, "s-o2".into());
    g.scene = Scene::Bench;
    click(&mut g, 200, 185);
    let before = g.state.cash;
    click(&mut g, 200, 35);
    assert_eq!((g.state.spares.clone(), g.state.cash - before), (vec!["s-plugs".to_string()], paid));
    assert_eq!(g.note.as_deref(), Some(format!("Sold as a set of 3: ${paid}.").as_str()));

    // The double-core side mount: only ever from the factory, on a few cars, and never in the paper.
    assert!(g.shop().iter().all(|(id, _)| *id != "sidemount-twin"));
    let lucky = (0..100).filter(|&c| { let mut o = Owned::new("umbra", None, c); g.outfit(&mut o); o.fitted["Intercooler"] == "sidemount-twin" || o.fitted["Intercooler"].starts_with("sidemount-twin@") }).count();
    assert!((1..20).contains(&lucky), "{lucky} in a hundred");

    // A game saved before: what was implied under the hood is fitted, or on the shelf if it was off.
    let mut st = g.state.clone();
    st.spares.clear();
    let mut old = Owned::new("umbra", None, 80);
    g.outfit(&mut old);
    for slot in ["Intake", "Exhaust", "Turbo", "Intercooler"] {
        old.fitted.remove(slot);
    }
    old.off.push("Intake".into());
    old.fitted.insert("Exhaust".into(), "header".into());
    st.cars = vec![old];
    g.settle_hood(&mut st);
    g.settle_hood(&mut st);
    let o = &st.cars[0];
    assert_eq!((o.fitted["Turbo"].as_str(), o.fitted["Exhaust"].as_str(), o.fitted.contains_key("Intake")), ("s-turbo@80", "header", false));
    assert_eq!(st.spares, vec!["s-exmani@80".to_string(), "s-airbox@80".to_string()], "once each, run twice");
}

#[test]
fn a_car_in_the_yard_goes_to_the_dealer_as_a_shell_or_for_scrap() {
    let Some(bytes) = default_pack() else { return };
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&bytes));
    let mut runs = Owned::new("umbra", None, 90);
    g.outfit(&mut runs);
    let mut dead = runs.clone();
    dead.fitted.insert("Timing belt".into(), "s-belt@0".into());
    let mut rusty = dead.clone();
    rusty.body = 90;
    g.state.cars = vec![runs.clone(), dead.clone(), rusty.clone()];
    g.state.spares.clear();
    g.state.cur = Some(0);
    g.scene = Scene::Yard;
    // Each way pays what it says, and the dealer will not take one that does not run, nor anyone a rotten shell.
    assert!(g.partings(&runs).iter().all(Option::is_some));
    assert_eq!(g.partings(&dead)[0], None);
    assert_eq!(g.partings(&rusty)[1], None);
    let scrap = g.scrap_offer(&rusty);
    assert!(scrap > 50 && scrap < g.value(&runs));

    // Sell on the second car: the choices come up. The dealer passes on it; the shell does not.
    click(&mut g, 280, 45);
    assert_eq!(g.parting, Some(1));
    click(&mut g, 60, 110);
    assert_eq!((g.parting, g.note.as_deref()), (Some(1), Some("It does not run. The dealer passes.")));
    let (cash, hours, shell) = (g.state.cash, g.state.hours, g.shell_offer(&dead).unwrap());
    click(&mut g, 150, 110);
    assert_eq!((g.state.cars.len(), g.state.cash - cash, g.state.hours - hours, g.state.spares.len()), (2, shell, 4, dead.fitted.len()));
    assert_eq!(g.parting, None);

    // Escape backs out of the choices, not out of the yard.
    click(&mut g, 280, 45);
    g.tick(KEY_B, 280, 45);
    g.tick(0, 280, 45);
    assert_eq!((g.parting, g.scene), (None, Scene::Yard));
    // The rusty one, for scrap: everything on it goes with it.
    click(&mut g, 280, 45);
    let (cash, spares) = (g.state.cash, g.state.spares.len());
    click(&mut g, 250, 110);
    assert_eq!((g.state.cars.len(), g.state.cash - cash, g.state.spares.len()), (1, scrap, spares));
    // And the one that runs, whole, to the dealer: it was in the bay, and the bay is empty.
    click(&mut g, 280, 35);
    let worth = g.value(&g.state.cars[0]);
    let cash = g.state.cash;
    click(&mut g, 60, 110);
    assert_eq!((g.state.cash - cash, g.state.cur), (worth, None));
}

#[test]
fn a_parts_car_from_far_off_comes_on_a_trailer() {
    let Some(bytes) = default_pack() else { return };
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&bytes));
    g.state.cars.clear();
    g.state.cur = None;
    g.state.cash = 100_000;
    // Find a day with one in the paper from somewhere it takes days to bring.
    while !g.projects().iter().flatten().any(|p| p.days > 1) {
        g.state.hours += g.pack.manifest.season.hours_per_day;
    }
    let i = g.projects().iter().position(|p| p.as_ref().is_some_and(|p| p.days > 1)).unwrap();
    let p = g.projects().into_iter().nth(i).flatten().unwrap();
    let (day, cash) = (g.day(), g.state.cash);
    g.buy_project(i);
    assert_eq!((g.state.cars.len(), g.state.coming.len(), cash - g.state.cash), (0, 1, p.price + p.ship));
    assert_eq!(g.note, Some(format!("Yours. On a trailer, here day {}.", day + p.days)));
    // With no car here and one coming, the game is not over.
    g.check_over();
    assert!(!matches!(g.scene, Scene::Over(_)));
    // A day short, it is still on the road; on the day, it is in the yard, and in the bay.
    g.spend((p.days - 1) * g.pack.manifest.season.hours_per_day);
    assert_eq!(g.state.coming.len(), 1);
    g.spend(g.pack.manifest.season.hours_per_day);
    assert_eq!((g.state.cars.len(), g.state.coming.len(), g.state.cur), (1, 0, Some(0)));
    assert_eq!(g.state.cars[0], p.owned);
    assert_eq!(g.note, Some(format!("The car from {} is here.", p.from)));
}
