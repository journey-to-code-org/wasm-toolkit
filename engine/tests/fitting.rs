//! Buying a part and putting it on, the way a player does: by the buttons and the pointer.

use engine::game::{Game, Scene};
use engine::pack::Pack;

const ENTER: u32 = 16;
const ESC: u32 = 32;

fn click(g: &mut Game, x: i32, y: i32) {
    g.tick(0, x, y);
    g.tick(ENTER, x, y);
    g.tick(0, x, y);
}

fn shot(g: &Game, name: &str) {
    if std::env::var("SHOTS").is_err() {
        return;
    }
    let mut s = engine::gfx::Screen::new();
    g.draw(&mut s);
    let mut out = Vec::new();
    for &i in s.fb.iter() {
        out.extend_from_slice(&s.pal[i as usize].to_le_bytes());
    }
    std::fs::write(format!("/tmp/{name}.raw"), out).unwrap();
}

/// Buys a part from the paper's parts page, under HOOD or wherever it is, by the pointer.
fn buy(g: &mut Game, id: &str) {
    let all = g.shop_page();
    let group = all.iter().position(|(k, _)| k.as_str() == id).map(|_| 0).unwrap();
    assert_eq!(group, 0);
    let i = g.shop_page().iter().position(|(k, _)| k.as_str() == id).unwrap();
    let (top, lines) = (g.shop_top(), g.shop_lines());
    while i >= g.turned[2] + lines {
        click(g, 100, top + lines as i32 * 10 + 2);
    }
    let row = top + (i - g.turned[2]) as i32 * 10 + 4;
    click(g, 100, row);
}

/// A new game in the default pack with the first car in the paper bought.
fn with_a_car() -> Option<Game> {
    let bytes = std::fs::read("../web/default.pack").ok()?;
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&bytes));
    g.seed(3);
    g.tick(ENTER, 160, 100);
    g.tick(0, 160, 100);
    click(&mut g, 50, 108);
    click(&mut g, 100, 70);
    assert_eq!(g.state.cars.len(), 1);
    Some(g)
}

#[test]
fn the_garage_has_buttons_for_the_shelf_and_the_hood() {
    let Some(mut g) = with_a_car() else { return };
    g.tick(ESC, 0, 0);
    g.tick(0, 0, 0);
    assert_eq!(g.scene, Scene::Place);
    click(&mut g, 250, 10);
    assert_eq!(g.scene, Scene::Bench);
    g.tick(ESC, 0, 0);
    g.tick(0, 0, 0);
    click(&mut g, 295, 10);
    assert_eq!(g.scene, Scene::Hood);
    // Across the top, the other places on the car; and back in the garage, a spot for each.
    let areas: Vec<String> = g.car().unwrap().areas().iter().map(|a| a.0.to_string()).collect();
    assert_eq!(areas, ["hood", "trunk", "under", "wheels", "inside", "engine"]);
    let w = (316 / 6).min(80);
    click(&mut g, 2 + 3 * w + 10, 6);
    assert_eq!(g.area_here().unwrap().0, "wheels");
    shot(&g, "a-wheels");
    for (k, name) in ["trunk", "under", "inside"].iter().enumerate() {
        click(&mut g, 2 + [1, 2, 4][k] * w + 10, 6);
        assert_eq!(g.area_here().unwrap().0, *name);
        shot(&g, &format!("a-{name}"));
    }
    g.tick(ESC, 0, 0);
    g.tick(0, 0, 0);
    click(&mut g, 60, 140);
    assert_eq!((g.scene, g.area_here().unwrap().0), (Scene::Hood, "trunk"));
}

#[test]
fn the_paper_sells_what_fits_and_says_what_it_does() {
    let Some(g) = with_a_car() else { return };
    let car = g.owned().unwrap().car.clone();
    let shop = g.shop();
    assert!(!shop.is_empty() && shop.iter().all(|(_, p)| p.goes_on(&car)));
    assert!(shop.len() < g.pack.manifest.parts.values().filter(|p| p.rare == 0).count());
}

#[test]
fn a_part_from_the_paper_goes_on_under_the_hood() {
    let Some(mut g) = with_a_car() else { return };
    click(&mut g, 285, 42);
    assert_eq!(g.scene, Scene::Paper(1));
    buy(&mut g, "cone-filter");
    assert_eq!(g.state.spares, ["cone-filter"], "{:?}", g.note);
    shot(&g, "f1-bought");
    // FIT PARTS goes to the bench, and the filter on the shelf goes under the hood, held.
    click(&mut g, 260, 190);
    assert_eq!(g.scene, Scene::Bench);
    click(&mut g, 230, 37);
    assert_eq!(g.scene, Scene::Hood, "{:?}", g.note);
    assert_eq!(g.note.as_deref(), Some("Undo its ringed bolts, then lift it."));
    shot(&g, "f2-holding");
    // Undo its bolts, lift the old one off, set the new one in, do the bolts up.
    let place = g.car().unwrap().hood.as_ref().unwrap().slots["Intake"].clone();
    for &[x, y] in &place.bolts {
        click(&mut g, x, y);
    }
    let [x, y, w, h] = place.rect;
    let (cx, cy) = (x + w / 2, y + h / 2);
    let clear = || place.bolts.iter().all(|b| (cx - b[0]).abs() > 5 || (cy - b[1]).abs() > 5);
    assert!(clear());
    click(&mut g, cx, cy);
    assert!(g.owned().unwrap().off.contains(&"Intake".to_string()), "{:?}", g.note);
    shot(&g, "f3-off");
    click(&mut g, cx, cy);
    assert_eq!(g.owned().unwrap().fitted.get("Intake").map(String::as_str), Some("cone-filter"), "{:?}", g.note);
    shot(&g, "f4-set");
    for &[x, y] in &place.bolts {
        click(&mut g, x, y);
    }
    assert_eq!(g.note.as_deref(), Some("Done up tight. It is on."));
    assert!(g.state.spares.iter().any(|p| !p.starts_with("cone-filter")) || g.state.spares.is_empty());
    shot(&g, "f5-done");
    // And the paper says so.
    g.tick(ESC, 0, 0);
    g.tick(0, 0, 0);
    click(&mut g, 50, 108);
    click(&mut g, 285, 42);
    shot(&g, "f6-paper");
}

/// Picks a bolt or a part on the picture, by the pointer.
fn pick_at(g: &mut Game, x: i32, y: i32) {
    click(g, x, y);
}

fn slot(g: &Game, name: &str) -> engine::pack::Slot {
    g.car().unwrap().areas().iter().find_map(|a| a.3.get(name).cloned()).unwrap()
}

#[test]
fn new_tires_go_on_at_the_wheel() {
    let Some(mut g) = with_a_car() else { return };
    g.state.cash = 5000;
    click(&mut g, 285, 42);
    let tires = g.shop_page().iter().find(|(_, p)| p.slot == "Tires" && !p.name.starts_with("All")).map(|(k, _)| k.to_string()).unwrap();
    buy(&mut g, &tires);
    assert!(g.state.spares.contains(&tires), "{:?}", g.note);
    // At the bench, the tires go to the wheel, held.
    g.tick(ESC, 0, 0);
    g.tick(0, 0, 0);
    click(&mut g, 250, 10);
    click(&mut g, 230, 37);
    assert_eq!((g.scene, g.area_here().unwrap().0), (Scene::Hood, "wheels"));
    // The tire will not come off with the wheel on: the lug nuts, then the wheel, then the tire.
    let (rims, tire) = (slot(&g, "Rims"), slot(&g, "Tires"));
    let (rx, ry) = rims.middle();
    let on_tire = (rx + 60, ry);
    assert!(tire.contains(on_tire.0, on_tire.1) && !tire.contains(rx, ry) && !tire.contains(rx + 30, ry), "a ring");
    pick_at(&mut g, on_tire.0, on_tire.1);
    assert!(g.note.as_deref().is_some_and(|n| n.contains("rims") || n.contains("in the way")), "{:?}", g.note);
    for &[x, y] in &rims.bolts {
        pick_at(&mut g, x, y);
    }
    pick_at(&mut g, rx + 30, ry + 30);
    assert!(g.owned().unwrap().off.contains(&"Rims".to_string()), "{:?}", g.note);
    shot(&g, "w1-rim-off");
    // The brakes are out of sight no more.
    assert!(g.in_view(g.area_here().unwrap().3, "Brakes"));
    pick_at(&mut g, on_tire.0, on_tire.1);
    assert!(g.owned().unwrap().off.contains(&"Tires".to_string()), "{:?}", g.note);
    pick_at(&mut g, on_tire.0, on_tire.1);
    assert_eq!(g.owned().unwrap().fitted.get("Tires").map(String::as_str), Some(tires.as_str()), "{:?}", g.note);
    // The old ones are on the floor; the wheel goes back on and its nuts are done up.
    assert!(g.floor().iter().any(|l| matches!(l, engine::game::Loose::Part(p) if p.starts_with("s-tyres"))), "{:?}", g.floor());
    let rim_back = g.floor().into_iter().position(|l| matches!(&l, engine::game::Loose::Part(p) if p.starts_with("s-rims"))).unwrap();
    pick_at(&mut g, 4 + (rim_back as i32 % 4) * 79 + 20, 178 + (rim_back as i32 / 4) * 11 + 4);
    pick_at(&mut g, rx + 30, ry + 30);
    for &[x, y] in &rims.bolts {
        pick_at(&mut g, x, y);
    }
    assert_eq!(g.note.as_deref(), Some("Done up tight. It is on."));
    assert_eq!(g.wont_run(), None);
    shot(&g, "w2-done");
}

#[test]
fn what_is_behind_a_cover_is_out_of_sight_until_it_is_off() {
    let Some(mut g) = with_a_car() else { return };
    click(&mut g, 250, 10);
    g.tick(ESC, 0, 0);
    g.tick(0, 0, 0);
    click(&mut g, 295, 10);
    assert_eq!(g.area_here().unwrap().0, "hood");
    let slots = g.area_here().unwrap().3.clone();
    assert!(!g.in_view(&slots, "Camshafts") && !g.in_view(&slots, "Spark plugs") && g.in_view(&slots, "Valve cover"));
    // Plug wires off (no bolts), and the plugs show; the valve cover off, and the cams do.
    let (px, py) = slot(&g, "Plug wires").middle();
    pick_at(&mut g, px, py);
    assert!(g.owned().unwrap().off.contains(&"Plug wires".to_string()), "{:?}", g.note);
    assert!(g.in_view(&slots, "Spark plugs") && !g.in_view(&slots, "Camshafts"));
    let cover = slot(&g, "Valve cover");
    for &[x, y] in &cover.bolts {
        pick_at(&mut g, x, y);
    }
    pick_at(&mut g, 180, 98);
    assert!(g.owned().unwrap().off.contains(&"Valve cover".to_string()), "{:?}", g.note);
    assert!(g.in_view(&slots, "Camshafts") && !g.in_view(&slots, "Valve springs"));
    // With it off it will not run.
    assert!(g.wont_run().is_some());
    shot(&g, "c1-cams");
}

#[test]
fn a_game_saved_with_the_old_names_comes_back() {
    let Some(g) = with_a_car() else { return };
    let mut s = g.state.clone();
    let o = &mut s.cars[0];
    let t = o.fitted.remove("Tires").unwrap();
    o.fitted.insert("Tyres".into(), t.clone());
    let b = o.fitted.remove("Transmission").unwrap();
    o.fitted.insert("Gearbox".into(), b.clone());
    o.off.push("Gearbox".into());
    g.settle_hood(&mut s);
    let o = &s.cars[0];
    assert_eq!((o.fitted.get("Tires"), o.fitted.get("Transmission")), (Some(&t), Some(&b)));
    assert!(o.off.contains(&"Transmission".to_string()) && !o.fitted.contains_key("Tyres"));
    // And the old words in a save are read too.
    let json = serde_json::to_string(&s).unwrap().replace("\"tires\"", "\"tyres\"");
    assert!(json.contains("\"tyres\""));
    let back: engine::game::State = serde_json::from_str(&json).unwrap();
    assert_eq!(back.cars[0].tires, s.cars[0].tires);
}

#[test]
fn the_default_pack_is_in_american_english() {
    let text = std::fs::read_to_string("../content/default/pack.json").unwrap().to_lowercase();
    for word in ["colour", "tyre", "metre", "litre", "centre", "gearbox", "bonnet", "windscreen", "grey", "kerb", "petrol"] {
        // Part ids are kept as they were, so that saved games find them.
        let n = text.matches(word).count() - text.matches(&format!("s-{word}")).count() - text.matches(&format!("sticky-{word}")).count();
        assert_eq!(n, 0, "{word}");
    }
}

#[test]
fn the_engine_comes_apart_on_the_stand_and_work_is_counted_in_minutes() {
    let Some(mut g) = with_a_car() else { return };
    g.tick(ESC, 0, 0);
    g.tick(0, 0, 0);
    click(&mut g, 295, 10);
    let w = (316 / 6).min(80);
    click(&mut g, 2 + 5 * w + 10, 6);
    assert_eq!(g.area_here().unwrap().0, "engine");
    let slots = g.area_here().unwrap().3.clone();
    assert!(!g.in_view(&slots, "Bottom end") && !g.in_view(&slots, "Crankshaft"));
    // Eight head bolts and a lift: about thirty-four minutes, not hours.
    let (hours, head) = (g.state.hours, slot(&g, "Cylinder head"));
    for &[x, y] in &head.bolts {
        pick_at(&mut g, x, y);
    }
    pick_at(&mut g, 160, 50);
    assert!(g.owned().unwrap().off.contains(&"Cylinder head".to_string()), "{:?}", g.note);
    assert!(g.in_view(&slots, "Bottom end") && g.state.hours - hours <= 1);
    // The block will not come out with the crank in it.
    pick_at(&mut g, 160, 100);
    assert!(g.note.as_deref().is_some_and(|n| n.contains("crankshaft") && n.contains("in the way")), "{:?}", g.note);
    assert_eq!(g.wont_run().as_deref(), Some("It has no cylinder head."));
    shot(&g, "e1-engine");
}

#[test]
fn no_two_bolts_that_can_be_seen_together_are_within_reach_of_one_pointer() {
    let Some(bytes) = std::fs::read("../web/default.pack").ok() else { return };
    let pack = Pack::parse(&bytes).unwrap();
    for (id, car) in &pack.manifest.cars {
        for (area, _, _, slots) in car.areas() {
            // Whether one is out of sight for as long as the other is on.
            let hidden_by = |a: &str, b: &str| {
                let mut at = a;
                for _ in 0..8 {
                    match slots.get(at).filter(|s| s.covered).and_then(|s| s.behind.as_deref()) {
                        Some(f) if f == b => return true,
                        Some(f) => at = f,
                        None => return false,
                    }
                }
                false
            };
            for (n1, s1) in slots {
                for (n2, s2) in slots.iter().filter(|(n2, _)| *n2 > n1) {
                    if hidden_by(n1, n2) || hidden_by(n2, n1) {
                        continue;
                    }
                    for b1 in &s1.bolts {
                        for b2 in &s2.bolts {
                            assert!((b1[0] - b2[0]).abs() > 5 || (b1[1] - b2[1]).abs() > 5, "{id} {area}: {n1} {b1:?} and {n2} {b2:?}");
                        }
                    }
                }
            }
        }
    }
}
