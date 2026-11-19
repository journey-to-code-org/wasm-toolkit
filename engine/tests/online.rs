//! Two engines and a room between them, as the parlor's server would be: who is here, a dare
//! taken, a race run in both at once, and the money moving the same way at both ends.

use engine::game::online::{decide, Outcome, Slip, CONNECTING, LOST, ON};
use engine::game::{Game, Scene, KEY_A, KEY_B, KEY_UP, KEY_X};
use engine::gfx::{ramp, shade, Screen, CLEAR};
use engine::pack::Pack;
use engine::race::Phase;
use serde_json::{json, Value};

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

fn pic(w: u16, h: u16, runs: &[(u16, u16)]) -> Vec<u8> {
    let mut words = vec![w, h];
    for &(n, c) in runs {
        words.push(n);
        words.push(c);
    }
    words.iter().flat_map(|v| v.to_le_bytes()).collect()
}

const MANIFEST: &str = r#"{
  "name": "Two Lanes",
  "start": { "cash": 1000, "place": "lot", "car": "box" },
  "bets": [10, 25],
  "cars": { "box": { "name": "Box", "side": "box.pic", "ground": 2, "paint": "RED", "paint_ramps": { "mid": "BLUE" } } },
  "places": { "lot": { "title": "Lot", "backdrop": "NAVY", "car_at": [10, 100],
                        "spots": [ { "rect": [0, 0, 320, 30], "label": "The meet", "do": "online" } ] } }
}"#;

fn game(seed: u32) -> Game {
    engine::host::store(b"");
    let blue = shade(ramp("BLUE"), 20);
    let mut g = Game::new(Pack::parse(&pack(&[("pack.json", MANIFEST.as_bytes().to_vec()), ("box.pic", pic(4, 2, &[(4, blue), (2, CLEAR), (2, blue)]))])));
    g.seed(seed);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    g
}

fn click(g: &mut Game, x: i32, y: i32) {
    g.tick(0, x, y);
    g.tick(KEY_A, x, y);
    g.tick(0, x, y);
}

fn press(g: &mut Game, key: u32) {
    g.tick(key, 300, 150);
    g.tick(0, 300, 150);
}

/// The room: it puts who a message is from on it, and passes it to whoever it is for, or to
/// everyone else.
struct Room {
    names: Vec<&'static str>,
}

impl Room {
    /// Everyone comes in, one after another, as the room would tell it.
    fn join(&mut self, games: &mut [&mut Game]) {
        for i in 0..games.len() {
            games[i].net_status(ON);
            let members: Vec<Value> = (0..=i).map(|k| json!({ "id": k + 1, "name": self.names[k] })).collect();
            games[i].net_in(&json!({ "t": "you", "id": i + 1, "members": members }).to_string());
            for k in 0..i {
                games[k].net_in(&json!({ "t": "join", "id": i + 1, "name": self.names[i] }).to_string());
            }
            self.pump(games);
        }
    }

    fn pump(&mut self, games: &mut [&mut Game]) {
        for _ in 0..8 {
            let mut any = false;
            for i in 0..games.len() {
                for text in std::mem::take(&mut games[i].net.outbox) {
                    any = true;
                    let mut v: Value = serde_json::from_str(&text).unwrap();
                    v["from"] = json!(i + 1);
                    let to = v.get("to").and_then(|t| t.as_u64());
                    for k in (0..games.len()).filter(|&k| k != i && to.is_none_or(|t| t as usize == k + 1)) {
                        games[k].net_in(&v.to_string());
                    }
                }
            }
            if !any {
                break;
            }
        }
    }
}

/// One tick of driving: on the limiter, away `late` ticks after the green, shifting short of the redline.
fn drive(g: &mut Game, late: i32) {
    let Some(r) = &g.race else { return g.tick(0, 300, 150) };
    if r.phase == Phase::Slip {
        return g.tick(0, 300, 150);
    }
    let up = (r.gear == 0 && r.tree >= 46 + late) || (r.gear > 0 && r.rpm > r.redline() - 250.0);
    g.tick(KEY_UP, 300, 150);
    if up {
        g.tick(KEY_UP | KEY_X, 300, 150);
    }
}

#[test]
fn the_winner_is_worked_out_from_two_timeslips() {
    let slip = |rt, et, red| Slip { rt, et, red };
    assert_eq!(decide(slip(200, 14000, false), slip(400, 13900, false)), Outcome::Won, "leaving sooner beats running quicker");
    assert_eq!(decide(slip(400, 13900, false), slip(200, 14000, false)), Outcome::Lost);
    assert_eq!(decide(slip(0, 12000, true), slip(900, 20000, false)), Outcome::Lost, "a red light loses to anything");
    assert_eq!(decide(slip(900, 20000, false), slip(0, 12000, true)), Outcome::Won);
    assert_eq!(decide(slip(0, 12000, true), slip(0, 13000, true)), Outcome::NoRace);
    assert_eq!(decide(slip(300, 14000, false), slip(300, 14000, false)), Outcome::NoRace);
}

#[test]
fn two_people_meet_dare_race_and_settle() {
    let (mut a, mut b) = (game(3), game(9));
    let mut s = Screen::new();
    let mut room = Room { names: vec!["Ay", "Bee"] };
    // Both open the meet. Until the room answers, there is nobody.
    click(&mut a, 100, 10);
    click(&mut b, 100, 10);
    assert_eq!((a.scene, a.net.want, a.net.state), (Scene::Online, true, CONNECTING));
    a.draw(&mut s);
    room.join(&mut [&mut a, &mut b]);
    assert_eq!((a.net.state, a.net.me, b.net.me), (ON, 1, 2));
    assert_eq!(a.net.people.iter().map(|p| (p.name.as_str(), p.car.as_deref(), p.paint.as_deref())).collect::<Vec<_>>(), vec![("Bee", Some("box"), Some("RED"))]);
    assert_eq!(b.net.people.iter().map(|p| (p.name.as_str(), p.car.as_deref())).collect::<Vec<_>>(), vec![("Ay", Some("box"))]);
    a.draw(&mut s);

    // Ay picks Bee, for ten dollars. Bee sees it and takes it.
    click(&mut a, 100, 44);
    assert_eq!(a.scene, Scene::OnlineBets);
    a.draw(&mut s);
    click(&mut a, 4 + 104 + 50, 185);
    assert_eq!((a.scene, a.net.dare_out), (Scene::Online, Some((2, 10))));
    room.pump(&mut [&mut a, &mut b]);
    assert_eq!(b.net.dare_in, Some((1, 10)));
    b.draw(&mut s);
    a.draw(&mut s);
    press(&mut b, KEY_A);
    room.pump(&mut [&mut a, &mut b]);
    assert_eq!((a.scene, b.scene), (Scene::Race, Scene::Race));
    assert!(a.race.as_ref().unwrap().net && a.net.versus.as_ref().unwrap().stake == 10);

    // The tree waits for both to stage.
    press(&mut a, KEY_A);
    room.pump(&mut [&mut a, &mut b]);
    for _ in 0..20 {
        a.tick(0, 300, 150);
    }
    assert_eq!(a.race.as_ref().unwrap().phase, Phase::Stage, "one staged is not a race");
    a.draw(&mut s);
    press(&mut b, KEY_A);
    room.pump(&mut [&mut a, &mut b]);
    a.tick(0, 300, 150);
    assert_eq!((a.race.as_ref().unwrap().phase, b.race.as_ref().unwrap().phase), (Phase::Tree, Phase::Tree));

    // Ay leaves on the green; Bee sits there for a second first. Each sees the other go.
    let mut seen_ahead = false;
    for _ in 0..3000 {
        drive(&mut a, 3);
        drive(&mut b, 33);
        room.pump(&mut [&mut a, &mut b]);
        let (ra, rb) = (b.race.as_ref().unwrap(), a.race.as_ref().unwrap());
        seen_ahead |= ra.rival_pos > ra.pos + 5.0 && ra.phase == Phase::Run;
        if ra.phase == Phase::Slip && rb.phase == Phase::Slip && a.net.versus.as_ref().unwrap().result.is_some() && b.net.versus.as_ref().unwrap().result.is_some() {
            break;
        }
    }
    assert!(seen_ahead, "Bee saw Ay out in front");
    a.draw(&mut s);
    b.draw(&mut s);
    assert_eq!((a.net.versus.as_ref().unwrap().result, b.net.versus.as_ref().unwrap().result), (Some(Outcome::Won), Some(Outcome::Lost)));
    let theirs = a.net.versus.as_ref().unwrap().him.unwrap();
    let br = b.race.as_ref().unwrap();
    assert_eq!((theirs.rt, theirs.et), (br.rt, br.et), "each has the other's own timeslip");
    press(&mut a, KEY_A);
    press(&mut b, KEY_A);
    assert_eq!((a.state.cash, b.state.cash, a.state.hours, b.state.hours), (1010, 990, 2, 2));
    assert_eq!((a.scene, b.scene, a.note.as_deref(), b.note.as_deref()), (Scene::Online, Scene::Online, Some("You won $10."), Some("You lost $10.")));

    // For kicks this time, and Bee backs out on the line: that is Ay's.
    click(&mut b, 100, 44);
    click(&mut b, 50, 185);
    room.pump(&mut [&mut a, &mut b]);
    assert_eq!(a.net.dare_in, Some((2, 0)));
    press(&mut a, KEY_A);
    room.pump(&mut [&mut a, &mut b]);
    press(&mut b, KEY_B);
    room.pump(&mut [&mut a, &mut b]);
    a.tick(0, 300, 150);
    assert_eq!((a.scene, b.scene, a.note.as_deref()), (Scene::Online, Scene::Online, Some("They are gone. That is a win.")));
    assert_eq!((a.state.cash, b.state.cash), (1010, 990));

    // Leaving the meet shuts the line.
    press(&mut a, KEY_B);
    assert_eq!((a.scene, a.net.want), (Scene::Place, false));
}

#[test]
fn what_arrives_is_not_trusted() {
    let (mut a, mut b) = (game(1), game(2));
    let mut room = Room { names: vec!["Ay", "Bee"] };
    click(&mut a, 100, 10);
    click(&mut b, 100, 10);
    room.join(&mut [&mut a, &mut b]);
    // Rubbish, a stranger, a stake nobody offered, a paint that is not a colour.
    for text in ["", "{", "[1,2]", r#"{"t":5}"#, r#"{"t":"dare","from":77,"stake":10}"#, r#"{"t":"ok","from":2}"#, r#"{"t":"slip","from":2,"rt":1,"et":1}"#] {
        a.net_in(text);
    }
    assert_eq!((a.scene, a.net.dare_in, a.state.cash), (Scene::Online, None, 1000));
    a.net_in(r#"{"t":"dare","from":2,"to":1,"stake":5000}"#);
    assert_eq!(a.net.dare_in, None);
    assert!(a.net.outbox.iter().any(|m| m.contains("\"no\"")), "it is turned down, not taken");
    a.net_in(r#"{"t":"here","from":2,"car":"spaceship","paint":"PUCE"}"#);
    assert_eq!((a.net.people[0].car.as_deref(), a.net.people[0].paint.as_deref()), (Some("spaceship"), None));
    let mut s = Screen::new();
    a.draw(&mut s);
    a.net_in(&format!(r#"{{"t":"join","id":9,"name":"{}"}}"#, "x".repeat(300)));
    assert_eq!(a.net.people[1].name.len(), 16);

    // A dare to someone who then leaves is off. A race when the line drops is no race.
    a.net.outbox.clear();
    click(&mut a, 100, 44);
    click(&mut a, 50, 185);
    assert_eq!(a.net.dare_out, Some((2, 0)));
    a.net_in(r#"{"t":"left","id":2}"#);
    assert_eq!((a.net.dare_out, a.net.people.len()), (None, 1));

    let (mut a, mut b) = (game(1), game(2));
    click(&mut a, 100, 10);
    click(&mut b, 100, 10);
    room.join(&mut [&mut a, &mut b]);
    click(&mut a, 100, 44);
    click(&mut a, 4 + 104 * 2 + 50, 185);
    room.pump(&mut [&mut a, &mut b]);
    press(&mut b, KEY_A);
    room.pump(&mut [&mut a, &mut b]);
    assert_eq!(a.scene, Scene::Race);
    a.net_status(LOST);
    assert_eq!((a.scene, a.state.cash, a.net.want, a.note.as_deref()), (Scene::Online, 1000, false, Some("The line dropped. No race.")));
    a.draw(&mut s);
    // And without a line at all, the meet says so.
    let mut c = game(4);
    click(&mut c, 100, 10);
    c.net_status(LOST);
    c.draw(&mut s);
    assert_eq!((c.scene, c.net.state), (Scene::Online, LOST));
    press(&mut c, KEY_B);
    assert_eq!(c.scene, Scene::Place);
}

fn road_game(seed: u32) -> Game {
    engine::host::store(b"");
    let blue = shade(ramp("BLUE"), 20);
    let manifest = MANIFEST.replace("\"places\":", "\"roads\": { \"pass\": { \"title\": \"The pass\", \"ends\": [\"lot\", \"lot\"], \"traffic\": 0, \"cops\": 3,
        \"parts\": [ { \"len\": 300, \"left\": \"fields\", \"right\": \"fields\" }, { \"len\": 300, \"curve\": 0.3, \"left\": \"fields\", \"right\": \"fields\" } ] } },\n  \"places\":");
    let mut g = Game::new(Pack::parse(&pack(&[("pack.json", manifest.as_bytes().to_vec()), ("box.pic", pic(4, 2, &[(4, blue), (2, CLEAR), (2, blue)]))])));
    g.seed(seed);
    g.tick(KEY_A, 160, 100);
    g.tick(0, 160, 100);
    g
}

/// One tick of a road: the middle of the lane, at about a speed.
fn cruise(g: &mut Game, mph: f64) {
    let Some(t) = g.trip.as_ref() else { return g.tick(0, 300, 100) };
    let mut keys = if t.speed * 2.237 < mph { KEY_UP } else { 0 };
    if t.x > 0.55 { keys |= 4 } else if t.x < 0.45 { keys |= 8 }
    g.tick(keys, 300, 100);
}

#[test]
fn two_people_race_down_a_road() {
    let (mut a, mut b) = (road_game(3), road_game(9));
    let mut s = Screen::new();
    let mut room = Room { names: vec!["Ay", "Bee"] };
    click(&mut a, 100, 10);
    click(&mut b, 100, 10);
    room.join(&mut [&mut a, &mut b]);

    // Ay picks Bee, changes where from the strip to the pass, and makes it ten dollars.
    click(&mut a, 100, 44);
    a.draw(&mut s);
    click(&mut a, 100, 158);
    assert_eq!((a.scene, a.net.course), (Scene::OnlineBets, 1));
    a.draw(&mut s);
    click(&mut a, 4 + 104 + 50, 185);
    room.pump(&mut [&mut a, &mut b]);
    assert_eq!(b.net.dare_in, Some((1, 10)));
    b.draw(&mut s);
    press(&mut b, KEY_A);
    room.pump(&mut [&mut a, &mut b]);
    assert_eq!((a.scene, b.scene), (Scene::Road, Scene::Road));
    let (ta, tb) = (a.trip.as_ref().unwrap(), b.trip.as_ref().unwrap());
    assert!(ta.cops.is_empty() && ta.rival.as_ref().unwrap().remote && tb.rival.as_ref().unwrap().remote);

    // Both are on the line, so it counts down; nobody moves for three seconds.
    for _ in 0..85 {
        cruise(&mut a, 70.0);
        cruise(&mut b, 45.0);
        room.pump(&mut [&mut a, &mut b]);
    }
    a.draw(&mut s);
    assert_eq!((a.trip.as_ref().unwrap().z, b.trip.as_ref().unwrap().z), (0.0, 0.0));
    // Ay is the quicker. Each sees the other where they said they were.
    let mut seen = false;
    for _ in 0..4000 {
        cruise(&mut a, 70.0);
        cruise(&mut b, 45.0);
        room.pump(&mut [&mut a, &mut b]);
        if let (Some(ta), Some(tb)) = (a.trip.as_ref(), b.trip.as_ref()) {
            if tb.z > 100.0 && tb.z < 500.0 {
                let him = ta.rival.as_ref().unwrap();
                assert!((him.z - tb.z).abs() < 8.0 && (him.x - tb.x).abs() < 0.2, "{:?}", (him.z, tb.z));
                seen = true;
            }
        }
        let done = |g: &Game| g.net.versus.as_ref().is_some_and(|v| v.result.is_some());
        if done(&a) && done(&b) {
            break;
        }
    }
    assert!(seen);
    a.draw(&mut s);
    b.draw(&mut s);
    assert_eq!((a.net.versus.as_ref().unwrap().result, b.net.versus.as_ref().unwrap().result), (Some(Outcome::Won), Some(Outcome::Lost)));
    press(&mut a, KEY_A);
    press(&mut b, KEY_A);
    assert_eq!((a.scene, b.scene, a.state.cash, b.state.cash, a.state.hours), (Scene::Online, Scene::Online, 1010, 990, 2));
    assert!(a.trip.is_none() && a.note.as_deref() == Some("You won $10."));

    // A dare down a road the other pack does not have is passed on, and backing out is losing.
    b.net_in(&json!({ "t": "dare", "from": 1, "stake": 0, "road": "nowhere", "seed": 5 }).to_string());
    assert_eq!(b.net.dare_in, None);
    b.net.outbox.clear();
    b.net_in(&json!({ "t": "dare", "from": 1, "stake": 10, "road": "pass", "seed": 5 }).to_string());
    press(&mut b, KEY_A);
    assert_eq!(b.scene, Scene::Road);
    press(&mut b, KEY_B);
    assert_eq!((b.scene, b.state.cash), (Scene::Online, 980));
}
