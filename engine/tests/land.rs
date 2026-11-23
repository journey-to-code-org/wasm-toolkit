//! The world in one piece: the default pack's town, and its roads laid out on from it.

use engine::game::{Game, Owned, Scene};
use engine::land::World;
use engine::pack::Pack;

const UP: u32 = 1;
const ENTER: u32 = 16;
const ESC: u32 = 32;

fn default_pack() -> Option<Vec<u8>> {
    std::fs::read("../web/default.pack").ok()
}

/// A new game in the default pack, with a car in the garage and the tank full.
fn game(bytes: &[u8]) -> Game {
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(bytes));
    g.seed(3);
    g.tick(ENTER, 160, 100);
    g.tick(0, 160, 100);
    let car = g.pack.manifest.cars.keys().next().unwrap().clone();
    g.state.cars.push(Owned::new(&car, None, 90));
    g.state.cur = Some(0);
    g.state.cars[0].gas = 14.0;
    g
}

fn click(g: &mut Game, x: i32, y: i32) {
    g.tick(0, x, y);
    g.tick(ENTER, x, y);
    g.tick(0, x, y);
}

/// Puts the car on a road, so far down it the way its points run, facing that way or back.
fn put(g: &mut Game, road: &str, along: f64, back: bool) {
    let w = g.land().unwrap();
    let lane = w.lanes.iter().find(|l| l.id == road).unwrap();
    let p = lane.at(along);
    let (x, z, _) = lane.beside(along, if back { -2.5 } else { 2.5 });
    let h = (p.h.to_degrees() + if back { 180.0 } else { 0.0 }).rem_euclid(360.0);
    let t = g.town.as_mut().unwrap();
    (t.x, t.z, t.heading, t.speed) = (x, z, h, 0.0);
}

/// Drives down whichever road the car is on, keeping to its own side, for so many ticks.
fn cruise(g: &mut Game, ticks: u32, fast: f64) {
    for _ in 0..ticks {
        if g.scene != Scene::Drive {
            return;
        }
        let t = g.town.as_ref().unwrap();
        let mut k = if t.speed < fast { UP } else { 0 };
        // Up a gear short of the redline, on every other tick so that each is a fresh press.
        if t.transmission.as_ref().is_some_and(|b| b.rpm > b.redline - 600.0) && g.ticks % 2 == 0 {
            k |= 64;
        }
        if let Some(n) = g.land().and_then(|w| w.near_on(g.dash.as_ref().map(|d| d.lane), t.x, t.z, 40.0)) {
            let lane = &g.land().unwrap().lanes[n.lane];
            let fwd = { let d = (t.heading.to_radians() - lane.at(n.along).h).sin_cos().1; d > 0.0 };
            let ahead = if fwd { (n.along + 25.0).min(lane.len()) } else { (n.along - 25.0).max(0.0) };
            let (tx, tz, _) = lane.beside(ahead, if fwd { 2.5 } else { -2.5 });
            let want = (tx - t.x).atan2(tz - t.z).to_degrees();
            let d = (want - t.heading + 540.0).rem_euclid(360.0) - 180.0;
            k |= if d < -3.0 { 4 } else if d > 3.0 { 8 } else { 0 };
        }
        g.tick(k, 0, 0);
    }
}

#[test]
fn the_default_world_is_laid_out() {
    let Some(bytes) = default_pack() else { return };
    let pack = Pack::parse(&bytes).unwrap();
    let w = World::build(&pack.manifest).expect("a world");
    // Every road is down, and every place they lead to.
    assert_eq!(w.lanes.len(), pack.manifest.roads.len());
    for place in ["overlook", "summit", "rollers", "norwalk"] {
        assert!(w.sites.iter().any(|s| s.place == place), "{place}");
    }
    // A road starts at its town gate, heading away from town.
    let map = &pack.manifest.maps["town"];
    let ridge = w.lanes.iter().position(|l| l.id == "ridge").unwrap();
    let (gx, gz, _) = map.gate("ridge").unwrap();
    assert!((w.lanes[ridge].pts[0].x - gx).abs() < 0.1 && (w.lanes[ridge].pts[0].z - gz).abs() < 0.1);
    // The mountain road goes on from the end of the ridge road, and climbs.
    let mountain = w.lanes.iter().find(|l| l.id == "mountain").unwrap();
    let top = w.lanes[ridge].pts.last().unwrap();
    assert!((mountain.pts[0].x - top.x).abs() < 0.1 && mountain.pts.last().unwrap().y > top.y + 200.0);
    // The loop comes back to where it started.
    let lake = w.lanes.iter().find(|l| l.id == "lake").unwrap();
    let (a, b) = (lake.pts[0], lake.pts[lake.pts.len() - 1]);
    assert!((a.x - b.x).abs() < 0.5 && (a.z - b.z).abs() < 0.5);
    // A point on a road is found on it, and one beside it, beside it.
    let (x, z, _) = w.lanes[ridge].beside(500.0, 2.0);
    let n = w.near(x, z, 20.0).unwrap();
    assert!(n.lane == ridge && (n.along - 500.0).abs() < 7.0 && (n.across - 2.0).abs() < 0.5, "{n:?}");
    // On the tar is open; well out in the trees is not.
    assert!(w.open(map, x, z));
    let trees = (0..w.lanes[ridge].pts.len()).find(|&i| w.lanes[ridge].pts[i].right == engine::road::Side::Trees).unwrap();
    let (x, z, _) = w.lanes[ridge].beside(trees as f64 * 6.0, 14.0);
    assert!(!w.open(map, x, z));
    // A pack whose map is not open has no world.
    let mut shut = pack.manifest.clone();
    shut.maps.get_mut("town").unwrap().open = false;
    assert!(World::build(&shut).is_none());
}

#[test]
fn out_of_town_is_more_of_the_same_drive() {
    let Some(bytes) = default_pack() else { return };
    let mut g = game(&bytes);
    click(&mut g, 270, 60);
    assert_eq!(g.scene, Scene::Drive);
    assert!(!g.out_of_town());
    // Out past the last street onto the ridge road, with no road view: still driving.
    let (x, z, h) = g.pack.manifest.maps["town"].gate("ridge").unwrap();
    {
        let t = g.town.as_mut().unwrap();
        let out = (h + 180.0).to_radians();
        (t.x, t.z, t.heading) = (x - out.sin() * 30.0, z - out.cos() * 30.0, (h + 180.0).rem_euclid(360.0));
    }
    cruise(&mut g, 600, 22.0);
    assert_eq!(g.scene, Scene::Drive);
    assert!(g.out_of_town() && g.trip.is_none());
    let n = g.lane_here().unwrap();
    assert_eq!(g.land().unwrap().lanes[n.lane].id, "ridge");
    assert!(n.along > 150.0, "{n:?}");
    // There is no Escape back to the garage from out here.
    g.tick(ESC, 0, 0);
    g.tick(0, 0, 0);
    assert_eq!(g.scene, Scene::Drive);
    // The road has traffic on it, and it moves.
    let before: Vec<f64> = g.town.as_ref().unwrap().rolling.iter().map(|c| c.along).collect();
    assert!(!before.is_empty());
    g.tick(0, 0, 0);
    assert!(g.town.as_ref().unwrap().rolling.iter().zip(&before).all(|(c, b)| c.along != *b));
    let mut s = engine::gfx::Screen::new();
    g.draw(&mut s);
}

#[test]
fn places_at_the_ends_are_pulled_into_and_driven_out_of() {
    let Some(bytes) = default_pack() else { return };
    let mut g = game(&bytes);
    click(&mut g, 270, 60);
    // Up at the top of the ridge road, stopped in the overlook's lot: Enter goes in.
    let len = g.land().unwrap().lanes.iter().find(|l| l.id == "ridge").unwrap().len();
    put(&mut g, "ridge", len - 4.0, false);
    g.tick(ENTER, 0, 0);
    g.tick(0, 0, 0);
    assert_eq!((g.scene, g.state.place.as_str()), (Scene::Place, "overlook"));
    // Out again by the mountain road: in the car in the lot, facing up it, and no road view.
    click(&mut g, 290, 186);
    assert_eq!(g.scene, Scene::Drive, "{:?}", g.note);
    let n = g.lane_here().unwrap();
    assert_eq!(g.land().unwrap().lanes[n.lane].id, "mountain");
    cruise(&mut g, 200, 16.0);
    let m = g.lane_here().unwrap();
    assert!(g.land().unwrap().lanes[m.lane].id == "mountain" && m.along > n.along + 40.0, "{m:?}");
    // And back down the ridge road from there, facing town.
    put(&mut g, "ridge", len - 4.0, true);
    g.tick(ENTER, 0, 0);
    g.tick(0, 0, 0);
    click(&mut g, 238, 186);
    assert_eq!(g.scene, Scene::Drive, "{:?}", g.note);
    cruise(&mut g, 200, 16.0);
    let k = g.lane_here().unwrap();
    assert!(g.land().unwrap().lanes[k.lane].id == "ridge" && k.along < len - 40.0, "{k:?}");
}

#[test]
fn a_long_road_takes_hours_and_waits_for_its_rung() {
    let Some(bytes) = default_pack() else { return };
    let mut g = game(&bytes);
    click(&mut g, 270, 60);
    // Not ready for Ohio: a little way down it you are turned round.
    put(&mut g, "ohio", 30.0, false);
    cruise(&mut g, 200, 20.0);
    assert!(g.note.as_deref().is_some_and(|n| n.starts_with("Not yet")), "{:?}", g.note);
    let n = g.lane_here().unwrap();
    assert!(n.along < 70.0, "{n:?}");
    // Ready for it: the further you go, the later it gets, a sixth of the trip each sixth of the way.
    g.state.rung = 6;
    g.note = None;
    put(&mut g, "ohio", 30.0, false);
    let hours = g.state.hours;
    let start = g.lane_here().unwrap().along;
    cruise(&mut g, 2400, 30.0);
    let went = g.lane_here().unwrap().along - start;
    let len = g.land().unwrap().lanes.iter().find(|l| l.id == "ohio").unwrap().len();
    let owed = (went * 6.0 / len) as i32;
    assert!(went > 1500.0 && g.state.hours - hours == owed, "{went} m, {} hours", g.state.hours - hours);
}

/// Into a place at the end of a road, by pulling up in its lot.
fn pull_into(g: &mut Game, road: &str, at_start: bool) {
    let len = g.land().unwrap().lanes.iter().find(|l| l.id == road).unwrap().len();
    put(g, road, if at_start { 4.0 } else { len - 4.0 }, false);
    g.tick(ENTER, 0, 0);
    g.tick(0, 0, 0);
    assert_eq!(g.scene, Scene::Place);
}

#[test]
fn the_police_sit_out_on_the_roads_too() {
    let Some(bytes) = default_pack() else { return };
    let mut g = game(&bytes);
    click(&mut g, 270, 60);
    let t = g.town.as_ref().unwrap();
    let roads: usize = g.pack.manifest.roads.values().map(|r| r.cops).sum();
    assert_eq!(t.cops.len(), t.town_cops + roads);
    // Past one on the ridge road at ninety: he is behind you, and stopping is the road's ticket.
    let (cx, cz) = t.cops[t.town_cops..].iter().copied().find(|&(x, z)| {
        let w = g.land().unwrap();
        w.near(x, z, 20.0).is_some_and(|n| w.lanes[n.lane].id == "ridge")
    }).unwrap();
    let n = g.land().unwrap().near(cx, cz, 20.0).unwrap();
    put(&mut g, "ridge", n.along - 30.0, false);
    g.town.as_mut().unwrap().speed = 40.0;
    let cash = g.state.cash;
    cruise(&mut g, 30, 40.0);
    assert!(g.town.as_ref().unwrap().chase.is_some(), "{:?}", g.note);
    for _ in 0..400 {
        g.tick(2, 0, 0);
    }
    let ticket = g.pack.manifest.roads["ridge"].ticket;
    assert!(g.town.as_ref().unwrap().chase.is_none() && g.state.cash <= cash - ticket.min(cash), "{} {}", g.state.cash, cash);
    // Inside the limit he lets you by.
    put(&mut g, "ridge", n.along - 30.0, false);
    cruise(&mut g, 200, 20.0);
    assert!(g.town.as_ref().unwrap().chase.is_none());
}

#[test]
fn a_road_race_is_run_on_the_road_itself() {
    let Some(bytes) = default_pack() else { return };
    let mut g = game(&bytes);
    click(&mut g, 270, 60);
    pull_into(&mut g, "ridge", false);
    assert_eq!(g.state.place, "overlook");
    click(&mut g, 186, 186);
    assert_eq!(g.scene, Scene::Bets, "{:?}", g.note);
    click(&mut g, 120, 186);
    assert_eq!(g.scene, Scene::Drive, "{:?}", g.note);
    assert!(g.trip.is_none() && g.dash.as_ref().is_some_and(|d| d.rival.is_some() && d.back));
    // Three, two, one: nobody moves until the off.
    let at = { let t = g.town.as_ref().unwrap(); (t.x, t.z) };
    for _ in 0..80 {
        g.tick(UP, 0, 0);
    }
    let t = g.town.as_ref().unwrap();
    assert!((t.x, t.z) == at && t.hold > 0 && g.dash.as_ref().unwrap().rival.as_ref().unwrap().s == 12.0);
    let mut s = engine::gfx::Screen::new();
    g.draw(&mut s);
    // Then down the hill to town, and it is settled when one of you gets there.
    let (wins, losses) = (g.state.wins, g.state.losses);
    cruise(&mut g, 6000, 45.0);
    assert_eq!(g.scene, Scene::Place, "{:?}", g.dash_at());
    assert_eq!((g.state.wins + g.state.losses, g.state.place.as_str()), (wins + losses + 1, "overlook"));
    assert!(g.dash.is_none() && g.town.is_none());
    // Lifting is losing.
    click(&mut g, 186, 186);
    click(&mut g, 120, 186);
    assert_eq!(g.scene, Scene::Drive, "{:?}", g.note);
    let losses = g.state.losses;
    for _ in 0..100 {
        g.tick(0, 0, 0);
    }
    g.tick(ESC, 0, 0);
    assert_eq!((g.scene, g.state.losses), (Scene::Place, losses + 1));
}

#[test]
fn the_poker_run_goes_round_the_lake() {
    let Some(bytes) = default_pack() else { return };
    let mut g = game(&bytes);
    g.state.rung = 6;
    click(&mut g, 270, 60);
    pull_into(&mut g, "ohio", false);
    assert_eq!(g.state.place, "norwalk");
    let cash = g.state.cash;
    click(&mut g, 80, 186);
    assert_eq!(g.scene, Scene::Drive, "{:?}", g.note);
    assert!(g.dash.as_ref().is_some_and(|d| d.poker) && g.state.cash == cash - g.pack.manifest.poker.entry);
    // The lake is on the inside all the way round, and the road comes back to where it left.
    let lake = g.land().unwrap().lanes.iter().find(|l| l.id == "lake").unwrap();
    let turned: f64 = lake.pts.windows(2).map(|w| { let d = w[1].h - w[0].h; d.sin().atan2(d.cos()) }).sum();
    assert!((turned - std::f64::consts::TAU).abs() < 0.3, "{turned}");
    cruise(&mut g, 2000, 19.0);
    assert!(g.scene == Scene::Drive && (1..5).contains(&g.dash.as_ref().unwrap().cards.len()));
    cruise(&mut g, 20000, 19.0);
    assert_eq!((g.scene, g.state.place.as_str()), (Scene::Place, "norwalk"), "{:?} {:?}", g.dash_at(), g.dash.as_ref().map(|d| d.far));
    assert!(g.dash.is_none() && g.note.is_some());
}
