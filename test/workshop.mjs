// The workshop's half of the pack format (web/pack.js), checked against the engine: a pack read
// and written in the browser's code still plays, and a picture turned into a car there shows up
// in the garage in whatever paint the pack gives it.
import { readFileSync } from "node:fs";
import { RAMPS, CLEAR, parsePack, buildPack, picDecode, picEncode, toIndices, groundOf, rampOf, slug, makeViewer, knockOut } from "../web/pack.js";

const [wasm, packPath] = process.argv.slice(2);
let fails = 0;
const check = (ok, what) => { if (!ok) { console.log("FAIL", what); fails++; } };

const inRust = readFileSync(new URL("../engine/src/gfx.rs", import.meta.url), "utf8").match(/RAMPS: \[&str; 32\] = \[([^;]+)\];/)[1].match(/[A-Z]+/g);
check(inRust.join() === RAMPS.join(), "the ramp names are the engine's");

const viewer = await makeViewer(readFileSync(wasm));
const pal = viewer.palette;
const original = new Uint8Array(readFileSync(packPath));
const pack = parsePack(original);
check(pack.manifest.cars.umbra && pack.files.has("garage.pic"), "the default pack reads");

// Read and written again, the engine shows the same thing.
const before = await viewer.view(original), again = await viewer.view(buildPack(pack));
check(before.ok && again.ok && before.rgba.every((v, i) => v === again.rgba[i]), "a pack written by the page plays the same");

// A picture survives being stored.
const pic = picDecode(pack.files.get("car.pic"));
const stored = picDecode(picEncode(pic.w, pic.h, pic.ix));
check(stored.w === pic.w && stored.ix.every((v, i) => v === pic.ix[i]), "a picture is stored and read back the same");
check(groundOf(pic.w, pic.h, pic.ix) >= pack.manifest.cars.umbra.ground, "the ground is found under the tyres");

// A car made here: an orange slab with black wheels on a white background, 60 x 20.
const w = 60, h = 20, rgba = new Uint8ClampedArray(w * h * 4);
for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
  const wheel = y >= 12 && ((x >= 8 && x < 16) || (x >= 44 && x < 52)), bodywork = y >= 4 && y < 14 && x >= 2 && x < 58;
  const shadeOf = 0.5 + 0.5 * (1 - y / h);
  const c = wheel ? [10, 10, 10] : bodywork ? [230 * shadeOf, 110 * shadeOf, 20 * shadeOf] : [255, 255, 255];
  rgba.set([...c, 255], (y * w + x) * 4);
}
const made = toIndices(rgba, w, h, pal, { clear: [255, 255, 255], body: { rgb: [230, 110, 20], within: 0.35 } });
check(RAMPS[made.ramp] === "ORANGE" && rampOf(pal, [20, 40, 230]) === RAMPS.indexOf("BLUE"), "the paint's ramp is found");
check(made.ix[0] === CLEAR && made.ix[5 * w + 30] >> 5 === made.ramp && made.ix[13 * w + 10] >> 5 !== made.ramp, "background is clear, bodywork is paint, wheels are not");
check(made.painted > 400 && made.painted < 560, "only the bodywork counts as paint");
check(new Set(Array.from(made.ix).filter((v) => v >> 5 === made.ramp)).size > 3, "the paint keeps its light and shade");
check(groundOf(w, h, made.ix) === 20, "its tyres are on the bottom row");
check(slug("My '90 Coupe!") === "my-90-coupe", "names make ids");

pack.files.set("slab.pic", picEncode(w, h, made.ix));
pack.manifest.cars.slab = { name: "Slab", side: "slab.pic", ground: 20, paint: "GREEN", paint_ramps: { mid: RAMPS[made.ramp] } };
pack.manifest.start.car = "slab";
const [cx, cy] = pack.manifest.places.garage.car_at;
const withSlab = await viewer.view(buildPack(pack));
const at = (x, y) => { const i = (y * 320 + x) * 4; return [withSlab.rgba[i], withSlab.rgba[i + 1], withSlab.rgba[i + 2]]; };
const body = at(cx + 30, cy - 20 + 6), wheel = at(cx + 11, cy - 3);
check(withSlab.ok, "the engine plays the pack with the new car");
check(body[1] > body[0] && body[1] > body[2], "the orange car stands in the garage in green");
check(wheel[0] < 60 && wheel[1] < 60 && wheel[2] < 60, "on black wheels, on the floor");

// And a pack the engine should refuse.
delete pack.manifest.cars.slab;
check(!(await viewer.view(buildPack(pack))).ok, "a pack that starts with a missing car is refused");
let threw = false;
try { parsePack(new Uint8Array([1, 2, 3])); } catch { threw = true; }
check(threw, "a file that is not a pack is refused by the page");

// Cutting a sprite out of a sheet: a red square with a grey middle, on a noisy checkerboard.
{
  const w = 40, h = 30, rgba = new Uint8ClampedArray(w * h * 4);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const i = (y * w + x) * 4, board = ((x >> 3) + (y >> 3)) & 1 ? 104 : 72, noise = (x * 7 + y * 3) % 5;
    const inside = x >= 10 && x < 30 && y >= 8 && y < 22, middle = x >= 16 && x < 24 && y >= 12 && y < 18;
    rgba.set(middle ? [72, 72, 72, 255] : inside ? [200, 30, 30, 255] : [board + noise, board + noise, board + noise, 255], i);
  }
  const cut = knockOut(rgba, w, h, 40);
  check(cut && cut.w === 20 && cut.h === 14, "a cut is trimmed to the sprite: " + (cut && [cut.w, cut.h]));
  check(cut.rgba[3] === 255 && cut.rgba[0] === 200, "the sprite is kept");
  check(cut.rgba[(6 * 20 + 10) * 4 + 3] === 255, "and so is the same grey as the board, inside it");
  const whole = knockOut(rgba, w, h, 0);
  check(whole.w === w && whole.h === h, "at 0 nothing is taken away");
  check(knockOut(new Uint8ClampedArray(16 * 4).fill(90), 4, 4, 40) === null, "a box of nothing but background is nothing");
}

console.log(fails ? `${fails} FAILED` : "workshop test passes");
process.exit(fails ? 1 : 0);
