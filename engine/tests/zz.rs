use engine::game::{Game, Scene};
use engine::pack::Pack;
fn click(g: &mut Game, x: i32, y: i32) { g.tick(0, x, y); g.tick(16, x, y); g.tick(0, x, y); }
#[test]
fn many_off() {
    let b = std::fs::read("../web/default.pack").unwrap();
    engine::host::store(b"");
    let mut g = Game::new(Pack::parse(&b)); g.seed(3);
    g.tick(16,160,100); g.tick(0,160,100); click(&mut g,50,108); click(&mut g,100,70);
    g.tick(32,0,0); g.tick(0,0,0); click(&mut g, 295, 10);
    // Take everything off under the hood that can come off, over and over.
    for _round in 0..6 {
        let slots = g.area_here().unwrap().3.clone();
        for (name, s) in &slots {
            if g.owned().unwrap().off.contains(name) || !g.in_view(&slots, name) { continue; }
            for &[x,y] in &s.bolts { click(&mut g, x, y); }
            let (mx,my) = s.middle();
            let p = (0..s.rect[2]).flat_map(|dx| (0..s.rect[3]).map(move |dy| (s.rect[0]+dx, s.rect[1]+dy))).find(|&(x,y)| s.contains(x,y) && s.bolts.iter().all(|b| (b[0]-x).abs()>5 || (b[1]-y).abs()>5) && slots.iter().filter(|(n,o)| *n != name && g.in_view(&slots,n) && !g.owned().unwrap().off.contains(n) && o.contains(x,y)).all(|(_,o)| o.rect[2]*o.rect[3] > s.rect[2]*s.rect[3])).unwrap_or((mx,my));
            click(&mut g, p.0, p.1);
        }
    }
    let o = g.owned().unwrap();
    println!("off {} : {:?}", o.off.len(), o.off);
    println!("floor {} : {:?}", g.floor().len(), g.floor());
    println!("hours {} day {}", g.state.hours, g.state.hours / 12);
}
