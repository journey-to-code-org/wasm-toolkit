// Loads the built game the way the page does and plays it: through the title, into the garage,
// out of the door and round the block. Every step is done twice from the same seed and must
// come out the same, pixel for pixel. Pass --shots <dir> to write each screen as a .ppm.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";

const [path, flag, dir] = process.argv.slice(2);
const W = 320, H = 240;
const A = 16, B = 32, UP = 1, LEFT = 4, RIGHT = 8;
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

  x.wtk_init(1234);
  const title = screen();
  shot("1-title");
  run(2, 0); run(1, A); run(2, 0);
  const garage = screen();
  shot("2-garage");
  run(1, 0, 220, 152); run(1, A, 220, 152); run(2, 0, 220, 152);
  const hood = screen();
  shot("3-hood");
  run(1, 0, 250, 60); run(1, A, 250, 60); run(1, 0, 250, 60);
  const parked = screen();
  shot("4-street");
  run(90, UP); run(20, UP | RIGHT); run(60, UP);
  const moving = screen();
  shot("5-driving");
  run(400, UP | LEFT);
  const later = screen();
  shot("6-later");
  run(1, B); run(2, 0);
  const back = screen();
  return { title, garage, hood, parked, moving, later, back, tones, colours };
}

const one = await play(flag === "--shots" ? dir : null);
const two = await play(null);
const same = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);
const differ = (a, b) => a.reduce((n, v, i) => n + (v !== b[i]), 0);

for (const k of ["title", "garage", "hood", "parked", "moving", "later", "back"]) check(same(one[k], two[k]), `${k} is the same both times`);
check(one.colours(one.title) >= 4, "the title has something on it");
check(differ(one.title, one.garage) > 20000, "Enter leaves the title for the garage");
check(differ(one.garage, one.hood) > 200 && differ(one.garage, one.hood) < 12000, "picking the hood opens it, and only it");
check(differ(one.hood, one.parked) > 20000, "picking the door goes out to the street");
check(differ(one.parked, one.moving) > 3000, "the street goes by when you drive");
check(one.colours(one.moving) >= 40, "there is a city to look at, in a good many colours");
check(one.tones.some((hz) => hz > 60), "the engine makes a noise");
check(one.tones.includes(70), "driving in circles ends at a wall");
check(differ(one.back, one.hood) < 9000 && differ(one.back, one.later) > 20000, "Escape is the garage again, as it was left");
check(Math.max(...one.later) < 1024 && Math.max(...one.title) < 1024, "nothing is drawn outside the palette");

console.log(fails ? `${fails} FAILED` : "smoke test passes");
process.exit(fails ? 1 : 0);
