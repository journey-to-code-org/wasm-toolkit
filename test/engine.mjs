// Loads the Rust engine the way the page does, gives it the default pack, and plays it: the title,
// the garage out of the pack, the headlamps, out to the lot and home again. Twice, and the two
// must match. Pass --shots <dir> to write each screen as a .ppm.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";

const [wasm, packPath, flag, dir] = process.argv.slice(2);
const W = 320, H = 240, A = 16;
let fails = 0;
const check = (ok, what) => { if (!ok) { console.log("FAIL", what); fails++; } };

async function play(shots, packBytes) {
  const { instance } = await WebAssembly.instantiate(readFileSync(wasm), { env: { wtk_tone: () => {} } });
  const x = instance.exports;
  const at = x.pack_alloc(packBytes.length);
  new Uint8Array(x.memory.buffer, at, packBytes.length).set(packBytes);
  const ok = x.pack_load();
  const screen = () => new Uint16Array(x.memory.buffer, x.wtk_screen(), W * H).slice();
  const shot = (name) => {
    if (!shots) return;
    const s = screen(), p = new Uint32Array(x.memory.buffer, x.wtk_palette(), 1024), out = Buffer.alloc(W * H * 3);
    for (let i = 0; i < W * H; i++) { const c = p[s[i]]; out[i * 3] = c & 255; out[i * 3 + 1] = (c >> 8) & 255; out[i * 3 + 2] = (c >> 16) & 255; }
    mkdirSync(shots, { recursive: true });
    writeFileSync(`${shots}/${name}.ppm`, Buffer.concat([Buffer.from(`P6 ${W} ${H} 255\n`), out]));
  };
  const run = (n, keys, mx = 160, my = 100) => { for (let i = 0; i < n; i++) { x.wtk_input(keys, mx, my); x.wtk_step(); } };
  const click = (mx, my) => { run(1, 0, mx, my); run(1, A, mx, my); run(1, 0, mx, my); };
  x.wtk_init(1);
  const title = screen(); shot("e1-title");
  run(1, A); run(2, 0);
  const garage = screen(); shot("e2-garage");
  click(268, 146);
  const lamps = screen(); shot("e3-lamps");
  click(270, 60);
  const lot = screen(); shot("e4-lot");
  click(160, 186);
  const home = screen();
  return { ok, title, garage, lamps, lot, home };
}

const packBytes = readFileSync(packPath);
const one = await play(flag === "--shots" ? dir : null, packBytes);
const two = await play(null, packBytes);
const same = (a, b) => a.every((v, i) => v === b[i]);
const differ = (a, b) => a.reduce((n, v, i) => n + (v !== b[i]), 0);
check(one.ok === 1, "the default pack loads");
for (const k of ["title", "garage", "lamps", "lot", "home"]) check(same(one[k], two[k]), `${k} is the same both times`);
check(differ(one.title, one.garage) > 20000, "Enter leaves the title for where the pack starts");
check(new Set(one.garage).size > 200, "the garage is the pack's painting");
check(differ(one.garage, one.lamps) > 20 && differ(one.garage, one.lamps) < 6000, "the headlamps spot swaps the car's picture");
check(differ(one.lamps, one.lot) > 20000, "the door goes to the lot");
check(differ(one.home, one.lamps) < 2000, "and the lot goes home again");
const bad = await play(null, Buffer.from("not a pack"));
check(bad.ok === 0 && differ(bad.title, one.title) > 1000, "a bad pack is refused, and the screen says so");
console.log(fails ? `${fails} FAILED` : "engine test passes");
process.exit(fails ? 1 : 0);
