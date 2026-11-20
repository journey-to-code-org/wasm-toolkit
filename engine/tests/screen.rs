//! The screen: laid out in 320 by 240 places, drawn in twice the dots each way.

use engine::gfx::{ramp, shade, Picture, Screen, PH, PW, S};

fn words(w: u16, h: u16, runs: &[(u16, u16)]) -> Vec<u8> {
    [w, h].into_iter().chain(runs.iter().flat_map(|&(n, c)| [n, c])).flat_map(|v| v.to_le_bytes()).collect()
}

#[test]
fn a_place_is_four_dots() {
    assert_eq!((PW, PH, S), (640, 480, 2));
    let mut s = Screen::new();
    let red = shade(ramp("RED"), 20);
    s.pset(10, 5, red);
    assert_eq!(s.fb.iter().filter(|&&c| c == red).count(), 4);
    assert_eq!(s.at(10, 5), red);
    // Off the screen is nowhere.
    s.pset(320, 0, red);
    s.rect(-5, -5, 3, 3, red);
    assert_eq!(s.fb.iter().filter(|&&c| c == red).count(), 4);
}

#[test]
fn a_picture_painted_fine_shows_every_dot() {
    let (a, b) = (shade(ramp("RED"), 20), shade(ramp("BLUE"), 20));
    // Four dots by two, alternating: two places by one, painted fine.
    let fine = Picture::from_bytes(&words(4 | 0x8000, 2, &[(1, a), (1, b), (1, a), (1, b), (1, b), (1, a), (1, b), (1, a)])).unwrap();
    assert_eq!((fine.w, fine.h, fine.s), (2, 1, 2));
    let mut s = Screen::new();
    s.picture(&fine, 0, 0);
    let dot = |x: i32, y: i32| s.fb[(y * PW + x) as usize];
    assert_eq!([dot(0, 0), dot(1, 0), dot(2, 0), dot(0, 1), dot(1, 1)], [a, b, a, b, a]);
    // Drawn at a size, it is the same.
    let mut t = Screen::new();
    t.picture_sized(&fine, 0, 0, 2, 1, None);
    assert_eq!(t.fb[..4], s.fb[..4]);
    // An ordinary one has a dot to a place, which is four on the screen.
    let plain = Picture::from_bytes(&words(2, 1, &[(1, a), (1, b)])).unwrap();
    assert_eq!((plain.w, plain.s), (2, 1));
    let mut u = Screen::new();
    u.picture(&plain, 0, 0);
    assert_eq!([u.fb[0], u.fb[1], u.fb[2], u.fb[PW as usize + 1]], [a, a, b, a]);
    // A fine one must be even each way.
    assert!(Picture::from_bytes(&words(3 | 0x8000, 2, &[(6, a)])).is_none());
}

#[test]
fn shapes_and_letters_are_drawn_in_dots() {
    let mut s = Screen::new();
    let c = shade(ramp("WHITE"), 20);
    // A slanting line is a dot thick at each step, so it steps half as far as a place would.
    s.line(0, 0, 40, 10, c);
    let lit: Vec<i32> = (0..PH).filter(|&y| s.fb[(y * PW + 40) as usize] == c).collect();
    assert!(!lit.is_empty() && lit.len() <= 4, "{lit:?}");
    // A letter's slant is smoothed: an A has dots lit that its 8x8 doubled would not.
    let mut t = Screen::new();
    t.text(0, 0, "A", c);
    let glyph = engine::font::FONT[(b'A' - 32) as usize];
    let (mut differ, mut same) = (0, 0);
    for y in 0..16 {
        for x in 0..16 {
            let doubled = glyph[y / 2] & (1 << (x / 2)) != 0;
            if (t.fb[y * PW as usize + x] == c) == doubled { same += 1 } else { differ += 1 }
        }
    }
    assert!(differ > 0 && same > differ * 8, "{differ} of {}", differ + same);
}
