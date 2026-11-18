// Loads the Rust engine the way the page does, gives it the default pack, and plays the start of a
// season by what is on the screen: buy a car out of the paper, look it over in the garage, drive
// to the lot, put ten dollars on a race and run it. Twice, and the two must match.
// Pass --shots <dir> to write each screen as a .ppm.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";

const [wasm, packPath, flag, dir] = process.argv.slice(2);
const W = 320, H = 240, UP = 1, A = 16, B = 32, E = 64;
let fails = 0;
const check = (ok, what) => { if (!ok) { console.log("FAIL", what); fails++; } };

async function play(shots, packBytes) {
  let kept = new Uint8Array(0), x;
  const env = {
    wtk_tone: () => {},
    wtk_store: (ptr, n) => { kept = new Uint8Array(x.memory.buffer, ptr, n).slice(); },
    wtk_fetch: (ptr, max) => { new Uint8Array(x.memory.buffer, ptr, max).set(kept.subarray(0, max)); return Math.min(kept.length, max); },
  };
  x = (await WebAssembly.instantiate(readFileSync(wasm), { env })).instance.exports;
  const at = x.pack_alloc(packBytes.length);
  new Uint8Array(x.memory.buffer, at, packBytes.length).set(packBytes);
  const ok = x.pack_load();
  const live = () => new Uint16Array(x.memory.buffer, x.wtk_screen(), W * H);
  const screen = () => live().slice();
  const shot = (name) => {
    if (!shots) return;
    const s = live(), p = new Uint32Array(x.memory.buffer, x.wtk_palette(), 1024), out = Buffer.alloc(W * H * 3);
    for (let i = 0; i < W * H; i++) { const c = p[s[i]]; out[i * 3] = c & 255; out[i * 3 + 1] = (c >> 8) & 255; out[i * 3 + 2] = (c >> 16) & 255; }
    mkdirSync(shots, { recursive: true });
    writeFileSync(`${shots}/${name}.ppm`, Buffer.concat([Buffer.from(`P6 ${W} ${H} 255\n`), out]));
  };
  const run = (n, keys, mx = 160, my = 100) => { for (let i = 0; i < n; i++) { x.wtk_input(keys, mx, my); x.wtk_step(); } };
  const click = (mx, my) => { run(1, 0, mx, my); run(1, A, mx, my); run(1, 0, mx, my); };
  const press = (key) => { run(1, key, 319, 199); run(1, 0, 319, 199); };
  const r = { ok };
  x.wtk_init(1);
  r.title = screen(); shot("e1-title");
  press(A);
  r.empty = screen(); shot("e2-garage-empty");
  if (!ok) return r;
  click(50, 108);
  r.paper = screen(); shot("e3-paper");
  click(100, 70);
  shot("e3b-bought");
  click(280, 40);
  r.parts = screen(); shot("e4-parts");
  press(B);
  r.garage = screen(); shot("e5-garage");
  click(230, 138); run(1, 0, 150, 120);
  r.hood = screen(); shot("e5b-hood");
  // The intake is held by two bolts, at opposite corners: both out, a look, and both back in.
  click(61, 103); click(92, 144); run(1, 0, 77, 124);
  r.bolts = screen(); shot("e5c-bolts");
  click(61, 103); click(92, 144);
  press(B);
  click(270, 146); run(1, 0, 319, 199);
  r.lamps = screen(); shot("e6-lamps");
  // Out of the big door and round the town: up to speed, left at the corner, and back again.
  click(270, 60);
  r.street = screen(); shot("e6b-street");
  run(60, UP); run(22, UP | 4); run(50, UP);
  r.town = screen(); shot("e6c-town");
  press(B);
  click(150, 130); run(1, 0, 319, 199);
  r.lot = screen(); shot("e7-lot");
  click(40, 186);
  r.bets = screen(); shot("e8-bets");
  click(4 + 52 + 26, 185);
  r.staged = screen(); shot("e9-staged");
  // Stage, bring the revs up, leave six ticks after the green, and shift when the light says to.
  press(A);
  run(50, UP);
  shot("e10-tree");
  // The shift light on the dash, and three places the timeslip covers with cream paper.
  const SHIFT_LIGHT = 170 * W + 121, LIT = 2 * 32 + 30, PAPER = [60 * W + 96, 100 * W + 96, 100 * W + 224];
  run(1, UP | E); run(1, UP);
  let ticks = 0, slip = false, mid = false;
  for (; ticks < 1500 && !slip; ticks++) {
    const lit = live()[SHIFT_LIGHT] === LIT;
    run(1, lit ? UP | E : UP);
    if (lit) run(1, UP);
    if (ticks === 150 && !mid) { shot("e11-run"); mid = true; }
    slip = PAPER.every((p) => (live()[p] >> 5) === 25);
  }
  r.ticks = ticks;
  r.slip = screen(); shot("e12-slip");
  press(A);
  r.after = screen(); shot("e13-after");
  press(B);
  r.back = screen(); shot("e14-title-again");
  return r;
}

const packBytes = readFileSync(packPath);
const one = await play(flag === "--shots" ? dir : null, packBytes);
const two = await play(null, packBytes);
const same = (a, b) => a.every((v, i) => v === b[i]);
const differ = (a, b) => a.reduce((n, v, i) => n + (v !== b[i]), 0);
check(one.ok === 1, "the default pack loads");
for (const k of ["title", "empty", "paper", "parts", "garage", "hood", "bolts", "lamps", "street", "town", "lot", "bets", "staged", "slip", "after", "back"]) check(same(one[k], two[k]), `${k} is the same both times`);
check(differ(one.title, one.empty) > 20000 && new Set(one.empty).size > 200, "Enter leaves the title for the garage, which is the pack's painting");
check(differ(one.empty, one.paper) > 40000, "the bench opens the paper");
check(differ(one.paper, one.parts) > 2000, "the paper has a parts page");
check(differ(one.empty, one.garage) > 3000 && differ(one.empty, one.garage) < 20000, "the car bought from the paper is in the garage");
check(differ(one.garage, one.hood) > 40000, "the hood opens on the pack's picture of the engine bay");
check(differ(one.hood, one.bolts) > 10 && differ(one.hood, one.bolts) < 3000, "bolts come undone where you pick them");
check(differ(one.garage, one.lamps) > 20 && differ(one.garage, one.lamps) < 6000, "the headlamps spot swaps the car's picture");
check(differ(one.lamps, one.street) > 30000, "the big door is the street, from the seat");
check(differ(one.street, one.town) > 8000, "and driving changes the view");
check(differ(one.lamps, one.lot) > 20000, "the car door goes to the lot");
check(differ(one.lot, one.bets) > 500, "racing someone asks what for");
check(differ(one.bets, one.staged) > 20000, "a bet he takes puts you on the strip");
check(one.ticks > 300 && one.ticks < 1000, `the quarter takes a believable while (${one.ticks} ticks)`);
check(differ(one.after, one.lot) > 0 && differ(one.after, one.lot) < 8000, "and afterwards you are back in the lot");
check(differ(one.back, one.title) > 100, "the title offers to carry on a game that is under way");
const bad = await play(null, Buffer.from("not a pack"));
check(bad.ok === 0 && differ(bad.title, one.title) > 1000, "a bad pack is refused, and the screen says so");
console.log(fails ? `${fails} FAILED` : "engine test passes");
process.exit(fails ? 1 : 0);
