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
  run(2, 0); run(1, A); run(2, 0);
  const garage = screen();
  shot("02-garage");
  click(244, 174);
  const lamps = screen();
  shot("03-lamps");

  // Under the hood: undo the four bolts on the intake and lift it off.
  click(200, 138);
  const hood = screen();
  shot("04-hood");
  for (const [bx, by] of [[61, 103], [92, 103], [92, 144], [61, 144]]) click(bx, by);
  click(77, 123);
  const stripped = screen();
  shot("05-intake-off");
  run(1, B); run(2, 0);
  // With a part off it, the car goes nowhere.
  click(110, 40);
  const refused = screen();
  shot("06-wont-start");

  // The paper on the bench: buy the better intake.
  click(28, 98);
  const paper = screen();
  click(84, 90);
  const bought = screen();
  shot("07-paper");
  run(1, B); run(2, 0);

  // Pick it up off the floor, set it on, and do up the bolts.
  click(200, 138);
  click(120, 183);
  click(77, 123);
  for (const [bx, by] of [[61, 103], [92, 103], [92, 144], [61, 144]]) click(bx, by);
  const fitted = screen();
  shot("08-fitted");
  run(1, B); run(2, 0);

  // Into town.
  click(250, 60);
  const parked = screen();
  shot("09-street");
  run(90, UP); run(20, UP | RIGHT); run(60, UP);
  const moving = screen();
  shot("10-driving");
  run(400, UP | LEFT);
  const later = screen();
  run(1, B); run(2, 0);

  // The strip: pick the calendar, stage, hold the throttle, go on the green, and shift.
  click(110, 40);
  const staged = screen();
  shot("11-staged");
  run(1, 0); run(1, A); run(50, UP);
  const green = screen();
  shot("12-green");
  run(1, UP | X);
  for (const wait of [43, 54, 103]) { run(wait, UP); run(1, UP | X); }
  run(60, UP);
  const running = screen();
  shot("13-running");
  run(200, UP);
  const slip = screen();
  shot("14-slip");
  return { title, garage, lamps, hood, stripped, refused, paper, bought, fitted, parked, moving, later, staged, green, running, slip, tones, colours };
}

const one = await play(flag === "--shots" ? dir : null);
const two = await play(null);
const same = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);
const differ = (a, b) => a.reduce((n, v, i) => n + (v !== b[i]), 0);

for (const k of Object.keys(one)) if (k !== "tones" && k !== "colours") check(same(one[k], two[k]), `${k} is the same both times`);
check(one.colours(one.title) >= 4, "the title has something on it");
check(differ(one.title, one.garage) > 20000, "Enter leaves the title for the garage");
check(differ(one.garage, one.lamps) > 20 && differ(one.garage, one.lamps) < 6000, "the headlamps pop up, and only they");
check(differ(one.lamps, one.hood) > 20000, "picking the hood goes under it");
check(differ(one.hood, one.stripped) > 300, "four bolts out and the intake lifts off");
check(differ(one.refused, one.lamps) < 9000, "with a part off, the car will not leave the garage");
check(differ(one.refused, one.paper) > 20000, "the bench has the paper on it");
check(differ(one.paper, one.bought) > 100, "an ad can be bought");
check(differ(one.stripped, one.fitted) > 300 && differ(one.hood, one.fitted) > 100, "the new part goes on in place of the old");
check(differ(one.fitted, one.parked) > 20000, "bolted back together, the door goes out to the street");
check(differ(one.parked, one.moving) > 3000, "the street goes by when you drive");
check(one.colours(one.moving) >= 40, "there is a town to look at, in a good many colours");
check(one.tones.some((hz) => hz > 60), "the engine makes a noise");
check(one.tones.includes(70), "driving in circles ends at a wall");
check(differ(one.later, one.staged) > 20000, "the calendar goes to the strip");
check(differ(one.staged, one.green) > 100, "the tree comes down");
check(differ(one.green, one.running) > 3000, "the car leaves the line");
check(differ(one.running, one.slip) > 5000, "there is a timeslip at the other end");
check(Math.max(...one.later) < 1024 && Math.max(...one.title) < 1024, "nothing is drawn outside the palette");

console.log(fails ? `${fails} FAILED` : "smoke test passes");
process.exit(fails ? 1 : 0);
