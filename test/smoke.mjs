// Loads the built game the way the page does and plays it: through the title, into the garage,
// out of the door and round the block. Every step is done twice from the same seed and must
// come out the same, pixel for pixel. Pass --shots <dir> to write each screen as a .ppm.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";

const [path, flag, dir] = process.argv.slice(2);
const W = 320, H = 240;
const A = 16, B = 32, UP = 1, LEFT = 4, RIGHT = 8, X = 64;
let fails = 0;
const check = (ok, what) => { if (!ok) { console.log("FAIL", what); fails++; } };

async function play(shots) {
  const tones = [];
  const { instance } = await WebAssembly.instantiate(readFileSync(path), { env: { wtk_tone: (hz, ms) => tones.push(hz) } });
  const x = instance.exports;
  const screen = () => new Uint16Array(x.memory.buffer, x.wtk_screen(), W * H).slice();
  const pal = () => new Uint32Array(x.memory.buffer, x.wtk_palette(), 1024);
  const shot = (name) => {
    if (!shots) return;
    const s = screen(), p = pal(), out = Buffer.alloc(W * H * 3);
    for (let i = 0; i < W * H; i++) { const c = p[s[i]]; out[i * 3] = c & 255; out[i * 3 + 1] = (c >> 8) & 255; out[i * 3 + 2] = (c >> 16) & 255; }
    mkdirSync(shots, { recursive: true });
    writeFileSync(`${shots}/${name}.ppm`, Buffer.concat([Buffer.from(`P6 ${W} ${H} 255\n`), out]));
  };
  const run = (n, keys, mx = 160, my = 100) => { for (let i = 0; i < n; i++) { x.wtk_input(keys, mx, my); x.wtk_step(); } };
  const colours = (s) => new Set(s).size;

  const click = (mx, my) => { run(1, 0, mx, my); run(1, A, mx, my); run(1, 0, mx, my); };

  x.wtk_init(1234);
  const title = screen();
  shot("01-title");
  // A new game opens on the paper: $750 and no car. Buy the first one in it.
  run(2, 0); run(1, A); run(2, 0);
  const paper = screen();
  shot("02-paper");
  click(100, 70);
  const bought = screen();
  shot("03-bought");
  run(1, B); run(2, 0);
  const garage = screen();
  shot("04-garage");
  click(268, 146);
  const lamps = screen();
  shot("05-lamps");

  // Under the hood: undo the four bolts on the intake and lift it off.
  click(225, 136);
  const hood = screen();
  shot("06-hood");
  const bolts = [[61, 103], [92, 103], [92, 144], [61, 144]];
  for (const [bx, by] of bolts) click(bx, by);
  click(77, 123);
  const stripped = screen();
  shot("07-intake-off");
  run(1, B); run(2, 0);
  // With a part off it, the car goes nowhere.
  click(270, 60);
  const refused = screen();
  shot("08-wont-start");
  // Pick it up off the floor, set it back on, and do up the bolts.
  click(225, 136);
  click(40, 183);
  click(77, 123);
  for (const [bx, by] of bolts) click(bx, by);
  const fitted = screen();
  run(1, B); run(2, 0);

  // Out of the door to the lot. The parts page of the paper first, to see it.
  click(30, 100);
  shot("09-parts");
  run(1, B); run(2, 0);
  click(270, 60);
  const lot = screen();
  shot("10-lot");
  // Race whoever is there, for kicks: stage, hold the throttle, go on the green, and shift.
  click(30, 186);
  const betting = screen();
  shot("11-bet");
  click(20, 186);
  const staged = screen();
  shot("12-staged");
  run(1, 0); run(1, A); run(50, UP);
  const green = screen();
  shot("13-green");
  run(1, UP | X);
  for (const wait of [43, 54, 103]) { run(wait, UP); run(1, UP | X); }
  run(60, UP);
  const running = screen();
  shot("14-running");
  run(260, UP);
  const slip = screen();
  shot("15-slip");
  run(1, 0); run(1, A); run(2, 0);
  const after = screen();
  shot("16-after");
  // A cruise round town, and home.
  click(215, 186);
  const parked = screen();
  run(90, UP); run(20, UP | RIGHT); run(60, UP);
  const moving = screen();
  shot("17-driving");
  run(400, UP | LEFT);
  const later = screen();
  run(1, B); run(2, 0);
  return { title, paper, bought, garage, lamps, hood, stripped, refused, fitted, lot, betting, staged, green, running, slip, after, parked, moving, later, tones, colours };
}

const one = await play(flag === "--shots" ? dir : null);
const two = await play(null);
const same = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);
const differ = (a, b) => a.reduce((n, v, i) => n + (v !== b[i]), 0);

for (const k of Object.keys(one)) if (k !== "tones" && k !== "colours") check(same(one[k], two[k]), `${k} is the same both times`);
check(one.colours(one.title) >= 4, "the title has something on it");
check(differ(one.title, one.paper) > 20000, "a new game opens on the paper");
check(differ(one.paper, one.bought) > 50, "a car can be bought out of it");
check(differ(one.bought, one.garage) > 20000, "and it is in the garage");
check(differ(one.garage, one.lamps) > 20 && differ(one.garage, one.lamps) < 6000, "the headlamps pop up, and only they");
check(differ(one.lamps, one.hood) > 20000, "picking the hood goes under it");
check(differ(one.hood, one.stripped) > 300, "four bolts out and the intake lifts off");
check(differ(one.refused, one.lamps) < 9000, "with a part off, the car will not leave the garage");
check(differ(one.stripped, one.fitted) > 300, "the part goes back on");
check(differ(one.refused, one.lot) > 20000, "bolted back together, the door goes out to the lot");
check(differ(one.lot, one.betting) > 200 && differ(one.lot, one.betting) < 20000, "racing him means naming a bet");
check(differ(one.betting, one.staged) > 20000, "and then the strip");
check(differ(one.staged, one.green) > 100, "the tree comes down");
check(differ(one.green, one.running) > 3000, "the car leaves the line");
check(differ(one.running, one.slip) > 5000, "there is a timeslip at the other end");
check(differ(one.slip, one.after) > 20000 && differ(one.after, one.lot) < 20000, "and then the lot again");
check(differ(one.after, one.parked) > 20000 && differ(one.parked, one.moving) > 3000, "a cruise goes into town, and the street goes by");
check(one.colours(one.moving) >= 40, "there is a town to look at, in a good many colours");
check(one.tones.some((hz) => hz > 60), "the engine makes a noise");
check(one.tones.includes(70), "driving in circles ends at a wall");
check(Math.max(...one.later) < 1024 && Math.max(...one.title) < 1024, "nothing is drawn outside the palette");

console.log(fails ? `${fails} FAILED` : "smoke test passes");
process.exit(fails ? 1 : 0);
