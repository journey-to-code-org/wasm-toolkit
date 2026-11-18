//! The engine, given a small pack made here: reading it, saying what is wrong with a bad one, and
//! playing it with the keys.

use engine::game::{Game, KEY_A, KEY_B};
use engine::gfx::{ramp, shade, Picture, Screen, CLEAR};
use engine::pack::Pack;

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
  "start": { "cash": 100, "place": "shed", "car": "box" },
  "cars": { "box": { "name": "Box", "side": "box.pic", "side_lamps": "box-lit.pic", "ground": 2,
                     "paint": "RED", "paint_ramps": { "mid": "BLUE" } } },
  "places": {
    "shed": { "title": "Shed", "picture": "shed.pic", "car_at": [10, 50],
              "spots": [ { "rect": [0, 0, 50, 20], "label": "Out", "do": "go:road" },
                         { "rect": [100, 0, 50, 20], "label": "Lamps", "do": "lamps", "needs_car": true },
                         { "rect": [200, 0, 50, 20], "label": "Sign", "do": "say:Hello." } ] },
    "road": { "title": "Road", "backdrop": "GRASS", "spots": [ { "rect": [0, 0, 320, 200], "label": "Back", "do": "go:shed" } ] }
  }
}"#;

fn good() -> Vec<u8> {
    let blue = shade(ramp("BLUE"), 20);
    pack(&[
        ("pack.json", MANIFEST.as_bytes().to_vec()),
        ("shed.pic", pic(320, 200, &[(64000, shade(ramp("WOOD"), 12))])),
        ("box.pic", pic(4, 2, &[(4, blue), (2, CLEAR), (2, blue)])),
        ("box-lit.pic", pic(4, 2, &[(8, shade(ramp("YELLOW"), 24))])),
    ])
}

fn click(g: &mut Game, x: i32, y: i32) {
    g.tick(0, x, y);
    g.tick(KEY_A, x, y);
    g.tick(0, x, y);
}

#[test]
fn a_good_pack_is_read() {
    let p = Pack::parse(&good()).expect("the pack should read");
    assert_eq!(p.manifest.name, "Test Pack");
    assert_eq!(p.manifest.start.cash, 100);
    assert_eq!(p.manifest.cars["box"].ground, 2);
    assert_eq!(p.manifest.places["shed"].spots.len(), 3);
    assert_eq!(p.pictures["box.pic"].w, 4);
}

#[test]
fn a_bad_pack_says_what_is_wrong() {
    let err = |bytes: Vec<u8>| Pack::parse(&bytes).err().expect("this pack should be refused");
    assert!(err(b"nope".to_vec()).contains("WPK1"));
    assert!(err(pack(&[("readme.txt", vec![1])])).contains("pack.json"));
    assert!(err(pack(&[("pack.json", b"{ not json".to_vec())])).contains("pack.json"));
    // A picture the manifest names but the pack does not hold.
    let mut files = vec![("pack.json", MANIFEST.as_bytes().to_vec()), ("shed.pic", pic(1, 1, &[(1, 5)])), ("box.pic", pic(1, 1, &[(1, 5)]))];
    assert!(err(pack(&files)).contains("box-lit.pic"));
    // A picture whose runs do not add up to its size.
    files.push(("box-lit.pic", pic(4, 2, &[(3, 5)])));
    assert!(err(pack(&files)).contains("not a picture"));
    // A spot that goes nowhere.
    let lost = MANIFEST.replace("go:road", "go:moon");
    let e = err(pack(&[("pack.json", lost.into_bytes()), ("shed.pic", pic(1, 1, &[(1, 5)])), ("box.pic", pic(1, 1, &[(1, 5)])), ("box-lit.pic", pic(1, 1, &[(1, 5)]))]));
    assert!(e.contains("moon"), "{e}");
    // A pack cut short.
    let whole = good();
    assert!(err(whole[..whole.len() - 10].to_vec()).contains("ends early"));
}

#[test]
fn the_game_is_what_the_pack_says() {
    let mut g = Game::new(Pack::parse(&good()));
    let mut s = Screen::new();
    assert!(g.on_title && g.problem.is_none());
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    assert!(!g.on_title);
    assert_eq!((g.place.as_str(), g.cash, g.car.as_deref()), ("shed", 100, Some("box")));

    // The room is the pack's picture, and the car stands in it in its own paint, not the picture's.
    g.draw(&mut s);
    assert_eq!(s.at(300, 100), shade(ramp("WOOD"), 12));
    assert_eq!(s.at(10, 48), shade(ramp("RED"), 20));
    assert_eq!(s.at(10, 49), shade(ramp("WOOD"), 12), "clear pixels of the car show the room");
    assert_eq!(s.at(12, 49), shade(ramp("RED"), 20));

    // A spot that says something, one that works the lamps, and one that goes somewhere.
    click(&mut g, 220, 10);
    assert_eq!(g.note.as_deref(), Some("Hello."));
    click(&mut g, 120, 10);
    assert!(g.lamps);
    g.draw(&mut s);
    assert_eq!(s.at(10, 48), shade(ramp("YELLOW"), 24));
    click(&mut g, 20, 10);
    assert_eq!(g.place, "road");
    g.draw(&mut s);
    assert_eq!(s.at(300, 100) >> 5, ramp("GRASS") as u16, "a place with no picture is its backdrop");
    click(&mut g, 20, 10);
    assert_eq!(g.place, "shed");

    // Without a car, the things that need one are not there.
    g.car = None;
    click(&mut g, 120, 10);
    assert!(g.lamps, "the lamps spot is gone, so nothing changed");
    g.tick(KEY_B, 120, 10);
    assert!(g.on_title);
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
