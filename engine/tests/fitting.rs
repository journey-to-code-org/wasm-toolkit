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
    let i = g.shop().iter().position(|(id, _)| id.as_str() == "cone-filter").unwrap();
    // Down the pages to it, and buy it.
    while i >= g.turned[2] + 11 {
        click(&mut g, 100, 56 + 11 * 10 + 2);
    }
    let row = 56 + (i - g.turned[2]) as i32 * 10 + 4;
    click(&mut g, 100, row);
    assert_eq!(g.state.spares, ["cone-filter"], "{:?}", g.note);
    shot(&g, "f1-bought");
    // FIT PARTS goes to the bench, and the filter on the shelf goes under the hood, held.
    click(&mut g, 260, 188);
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
