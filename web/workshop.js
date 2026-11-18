// The workshop: a pack, edited in the browser. Cars, rooms, parts, rivals and the game's own
// settings each have a tab; the engine itself draws the result beside them as you go.
import { RAMPS, CLEAR, parsePack, buildPack, picDecode, picEncode, toIndices, groundOf, slug, makeViewer } from "./pack.js";

const $ = (id) => document.getElementById(id);
const KEEP = "wtk:workshop:pack";
const viewer = await makeViewer(await (await fetch("engine.wasm")).arrayBuffer());
const pal = viewer.palette;
const css = (c) => `rgb(${c & 255},${(c >> 8) & 255},${(c >> 16) & 255})`;
const nice = (ramp) => ramp[0] + ramp.slice(1).toLowerCase();
const DOES = [["", "Nothing: only its name shows"], ["say", "Says something"], ["go", "Goes to another room"], ["lamps", "Works the headlamps"], ["paper", "Opens the paper"],
  ["hood", "Opens the hood"], ["parts", "Opens the shelf of parts"], ["yard", "Opens the yard"], ["calendar", "Says what day it is"], ["race", "Races whoever has pulled in"],
  ["next", "Waves them on"], ["solo", "A pass down the strip alone"], ["drive", "Out for a drive on a map"], ["gas", "Fills the tank"]];
const TABS = [["cars", "Cars"], ["rooms", "Rooms"], ["maps", "Maps"], ["parts", "Parts"], ["rivals", "Rivals"], ["game", "The game"]];

let pack = null;
const at = { tab: "cars", car: null, place: null, spot: -1, slot: null, map: null, brush: "." };
// The pictures dropped in this visit, by car, so they can be matched to the colours again when a
// setting changes: { side, lamps (ImageBitmaps), width, corner, body: [r, g, b] | null, within }.
const sources = new Map();

// ---- Small things for building forms ----

function h(tag, props = {}, ...kids) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "class") el.className = v; else if (k === "text") el.textContent = v; else if (k.startsWith("on")) el[k] = v;
    else if (v === true) el.setAttribute(k, ""); else if (v !== false && v != null) el.setAttribute(k, v);
  }
  el.append(...kids.flat().filter((k) => k != null));
  return el;
}
let ids = 0;
// A labelled box to type in. `set` is given the new value as it is typed; the picture catches up.
function field(label, value, set, { redo, ...props } = {}) {
  const id = props.id ?? "f" + ids++, input = h("input", { type: "text", ...props, id });
  input.value = value ?? "";
  input.oninput = () => { set(input.type === "number" ? (input.value === "" ? null : +input.value) : input.value); touch(); };
  if (redo && !props.onchange) input.onchange = () => render();
  return h("div", {}, h("label", { for: id, text: label }), input);
}
function choice(label, value, options, set) {
  const id = "f" + ids++, select = h("select", { id }, options.map(([v, words]) => new Option(words, v)));
  select.value = value ?? "";
  select.onchange = () => { set(select.value); render(); };
  return h("div", {}, h("label", { for: id, text: label }), select);
}
function tick(label, value, set) {
  const input = h("input", { type: "checkbox" });
  input.checked = !!value;
  input.onchange = () => { set(input.checked); render(); };
  return h("label", { class: "check" }, input, " " + label);
}
function list(items, current, pick) {
  return h("ul", { class: "list" }, items.map(([id, words]) => h("li", {}, h("button", { "aria-current": String(id === current), text: words, onclick: () => pick(id) }))));
}
const colours = (none) => [...(none ? [["", none]] : []), ...RAMPS.slice(1).map((r) => [r, nice(r)])];
const carChoices = () => Object.entries(pack.manifest.cars).map(([id, c]) => [id, c.name]);
function freshId(words, taken) {
  let id = slug(words);
  for (let n = 2; id in taken; n++) id = `${slug(words)}-${n}`;
  return id;
}
function fail(why) { $("problem").textContent = why; }

// ---- The pack as a whole ----

async function start(bytes) {
  pack = parsePack(bytes);
  // A place under a hood may be written as its box alone; here each is given its long form.
  for (const c of Object.values(pack.manifest.cars)) for (const [name, slot] of Object.entries(c.hood?.slots ?? {})) if (Array.isArray(slot)) c.hood.slots[name] = { rect: slot };
  sources.clear();
  const m = pack.manifest;
  Object.assign(at, { car: m.start.car ?? Object.keys(m.cars)[0] ?? null, place: m.start.place, spot: -1, slot: null });
  render();
}

function keep() {
  try { localStorage.setItem(KEEP, btoa(Array.from(buildPack(pack), (b) => String.fromCharCode(b)).join(""))); } catch { /* too big, or a private window */ }
}

let waiting = 0;
// Something was typed: keep it, and redraw the game once the typing stops.
function touch() {
  keep();
  clearTimeout(waiting);
  waiting = setTimeout(preview, 150);
}

const car = () => pack.manifest.cars[at.car];
const place = () => pack.manifest.places[at.place];

// A picture file drawn `width` wide, and no taller than `most`, as RGBA.
function scaled(bitmap, width, most = 200) {
  const w = Math.max(16, Math.min(320, width)), hh = Math.max(1, Math.round(bitmap.height * w / bitmap.width));
  if (hh > most) throw new Error(`At ${w} wide the picture is ${hh} tall; there is room for ${most}. Make it narrower.`);
  const c = new OffscreenCanvas(w, hh), g = c.getContext("2d", { willReadFrequently: true });
  g.imageSmoothingEnabled = bitmap.width > w * 1.5;
  g.imageSmoothingQuality = "high";
  g.drawImage(bitmap, 0, 0, w, hh);
  return { w, h: hh, rgba: g.getImageData(0, 0, w, hh).data };
}

// A picture for a room or an engine bay: made to fit the width, and cut to the height if it is taller.
async function wholePicture(file, most) {
  return sizedPicture(file, 320, most, true);
}

// A picture made exactly `w` by `h`: scaled to cover that, and what hangs over the sides or the
// top and bottom cut away evenly. With `shorter`, a picture too wide for its height is left
// shorter than `h` and not cut at the sides.
async function sizedPicture(file, w, h, shorter = false) {
  const bitmap = await createImageBitmap(file);
  if (shorter) h = Math.min(h, Math.round(bitmap.height * w / bitmap.width));
  const scale = Math.max(w / bitmap.width, h / bitmap.height), dw = bitmap.width * scale, dh = bitmap.height * scale;
  const c = new OffscreenCanvas(w, h), g = c.getContext("2d", { willReadFrequently: true });
  g.imageSmoothingEnabled = scale < 0.67;
  g.imageSmoothingQuality = "high";
  g.drawImage(bitmap, (w - dw) / 2, (h - dh) / 2, dw, dh);
  return picEncode(w, h, toIndices(g.getImageData(0, 0, w, h).data, w, h, pal, { dither: 5 }).ix);
}

// A small picture, such as a steering wheel or a car from behind: brought down to `most` wide if
// it is wider, and made square if asked.
async function smallPicture(file, most, square) {
  const bitmap = await createImageBitmap(file);
  const w = Math.min(most, bitmap.width), hh = square ? w : Math.max(1, Math.round(bitmap.height * w / bitmap.width));
  const c = new OffscreenCanvas(w, hh), g = c.getContext("2d", { willReadFrequently: true });
  g.imageSmoothingEnabled = bitmap.width > w * 1.5;
  g.imageSmoothingQuality = "high";
  g.drawImage(bitmap, 0, 0, w, hh);
  return picEncode(w, hh, toIndices(g.getImageData(0, 0, w, hh).data, w, hh, pal, { dither: 6 }).ix);
}

function dropZone(label, onFile) {
  const input = h("input", { type: "file", accept: "image/*", hidden: true });
  const zone = h("div", { class: "drop", tabindex: "0", role: "button", text: "Drop a PNG here, or click to choose" }, input);
  zone.onclick = () => input.click();
  zone.onkeydown = (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); input.click(); } };
  input.onclick = (e) => e.stopPropagation();
  input.onchange = () => { if (input.files[0]) onFile(input.files[0]); input.value = ""; };
  zone.ondragover = (e) => { e.preventDefault(); zone.classList.add("over"); };
  zone.ondragleave = () => zone.classList.remove("over");
  zone.ondrop = (e) => { e.preventDefault(); zone.classList.remove("over"); if (e.dataTransfer.files[0]) onFile(e.dataTransfer.files[0]); };
  return h("div", {}, h("label", { text: label }), zone);
}

// ---- Boxes drawn over a picture, to drag about ----

// `items()` gives the boxes: { rect: [x, y, w, h], name, point }. A point is a mark with no size.
// The canvas lies over a picture `scale` times smaller than itself.
function boxes(canvas, scale, items, { selected, select, moved, done }) {
  const g = canvas.getContext("2d");
  const draw = () => {
    g.clearRect(0, 0, canvas.width, canvas.height);
    g.font = "bold 11px ui-monospace, monospace";
    items().forEach((it, i) => {
      const [x, y, w, hh] = it.rect.map((v) => v * scale), on = i === selected();
      g.lineWidth = on ? 3 : 1.5;
      g.strokeStyle = on ? "#ffd24a" : it.point ? "#ff6a8a" : "#6ae0ff";
      g.fillStyle = on ? "rgba(255,210,74,.18)" : "rgba(106,224,255,.10)";
      if (it.point) { g.beginPath(); g.moveTo(x - 9, y); g.lineTo(x + 9, y); g.moveTo(x, y - 9); g.lineTo(x, y + 3); g.stroke(); }
      else { g.fillRect(x, y, w, hh); g.strokeRect(x, y, w, hh); if (on) { g.fillStyle = "#ffd24a"; g.fillRect(x + w - 8, y + hh - 8, 8, 8); } }
      g.fillStyle = "#000"; g.fillText(it.name, x + 4, y + 13); g.fillStyle = on ? "#ffd24a" : "#fff"; g.fillText(it.name, x + 3, y + 12);
    });
  };
  let drag = null;
  const where = (e) => { const r = canvas.getBoundingClientRect(); return [(e.clientX - r.left) / r.width * canvas.width / scale, (e.clientY - r.top) / r.height * canvas.height / scale]; };
  const hit = (it, x, y) => it.point ? Math.abs(x - it.rect[0]) < 6 && Math.abs(y - it.rect[1]) < 6 : x >= it.rect[0] && x < it.rect[0] + it.rect[2] && y >= it.rect[1] && y < it.rect[1] + it.rect[3];
  canvas.onpointerdown = (e) => {
    const [x, y] = where(e), all = items(), sel = all[selected()];
    let i = -1, size = false;
    if (sel && !sel.point && x >= sel.rect[0] + sel.rect[2] - 5 && x < sel.rect[0] + sel.rect[2] + 2 && y >= sel.rect[1] + sel.rect[3] - 5 && y < sel.rect[1] + sel.rect[3] + 2) { i = selected(); size = true; }
    else for (let k = all.length - 1; k >= 0 && i < 0; k--) if (hit(all[k], x, y)) i = k;
    if (i !== selected()) select(i);
    if (i < 0) return done();
    drag = { i, size, x, y, from: [...items()[i].rect] };
    canvas.setPointerCapture(e.pointerId);
    draw();
  };
  canvas.onpointermove = (e) => {
    if (!drag) return;
    const [x, y] = where(e), it = items()[drag.i], dx = Math.round(x - drag.x), dy = Math.round(y - drag.y);
    const maxW = canvas.width / scale, maxH = canvas.height / scale;
    if (drag.size) { it.rect[2] = Math.max(6, Math.min(maxW - it.rect[0], drag.from[2] + dx)); it.rect[3] = Math.max(6, Math.min(maxH - it.rect[1], drag.from[3] + dy)); }
    else { it.rect[0] = Math.max(0, Math.min(maxW - (it.point ? 0 : it.rect[2]), drag.from[0] + dx)); it.rect[1] = Math.max(0, Math.min(maxH - (it.point ? 0 : it.rect[3]), drag.from[1] + dy)); }
    moved(drag.i, it.rect);
    draw();
  };
  canvas.onpointerup = () => { if (drag) { drag = null; done(); } };
  draw();
  return draw;
}

// ---- Cars ----

// Match the dropped pictures to the game's colours again and put them in the pack.
function convert() {
  const src = sources.get(at.car), c = car();
  if (!src?.side) return;
  const options = (rgba) => ({
    clear: src.corner ? [rgba[0], rgba[1], rgba[2]] : null,
    body: src.body ? { rgb: src.body, within: src.within / 100 } : null,
  });
  const side = scaled(src.side, src.width);
  const done = toIndices(side.rgba, side.w, side.h, pal, options(side.rgba));
  src.rgba = side;
  src.painted = done.painted;
  pack.files.set(c.side, picEncode(side.w, side.h, done.ix));
  c.paint_ramps = src.body ? { mid: RAMPS[done.ramp] } : {};
  if (!src.body) delete c.paint;
  else c.paint ??= RAMPS[done.ramp];
  if (src.lamps) {
    const lit = scaled(src.lamps, src.width);
    c.side_lamps = `${at.car}-lamps.pic`;
    pack.files.set(c.side_lamps, picEncode(lit.w, lit.h, toIndices(lit.rgba, lit.w, lit.h, pal, options(lit.rgba)).ix));
  }
  if (!src.groundSet) c.ground = groundOf(side.w, side.h, done.ix);
}

// Make a change to the car's picture, match it again, and show it. One that cannot be made is undone.
function change(make) {
  const before = JSON.stringify(pack.manifest), src = sources.get(at.car), was = src && { ...src };
  try { make(); convert(); } catch (e) {
    pack.manifest = JSON.parse(before);
    if (src) Object.assign(src, was);
    render();
    return fail(e.message);
  }
  render();
}

async function dropped(file, which) {
  let bitmap;
  try { bitmap = await createImageBitmap(file); } catch { return fail("That file is not a picture the browser can read."); }
  const src = sources.get(at.car) ?? { width: Math.min(bitmap.width, 244), corner: false, body: null, within: 35 };
  if (which === "side") { src.side = bitmap; src.width = Math.min(bitmap.width, bitmap.width > 320 ? 244 : 320); src.body = null; src.groundSet = false; }
  else if (!src.side) return fail("Give the car its ordinary picture first.");
  else src.lamps = bitmap;
  sources.set(at.car, src);
  change(() => {});
}

function addCar() {
  const cars = pack.manifest.cars, id = freshId("new-car", cars), like = Object.values(cars)[0];
  // A new car borrows the first car's picture and running gear until it is given its own.
  if (like) {
    pack.files.set(`${id}.pic`, pack.files.get(like.side));
    cars[id] = JSON.parse(JSON.stringify({ ...like, name: "New car", side: `${id}.pic`, side_lamps: undefined }));
  } else {
    pack.files.set(`${id}.pic`, picEncode(1, 1, new Uint16Array([CLEAR])));
    cars[id] = { name: "New car", side: `${id}.pic`, ground: 1 };
  }
  at.car = id;
  render();
}

function removeCar() {
  const m = pack.manifest, c = car();
  const driver = m.rivals?.find((r) => r.car === at.car);
  if (driver) return fail(`${driver.name} drives this car. Give them another first.`);
  if (!confirm(`Remove ${c.name} from the pack?`)) return;
  pack.files.delete(c.side);
  if (c.side_lamps) pack.files.delete(c.side_lamps);
  delete m.cars[at.car];
  for (const p of Object.values(m.parts ?? {})) if (p.fits) p.fits = p.fits.filter((id) => id !== at.car);
  sources.delete(at.car);
  if (m.start.car === at.car) delete m.start.car;
  at.car = m.start.car ?? Object.keys(m.cars)[0] ?? null;
  tidy();
  render();
}

// A car added here is filed under its name once it has one.
function fileCar() {
  const cars = pack.manifest.cars, c = car(), old = at.car;
  if (!old.startsWith("new-car") || !c.name.trim() || slug(c.name) === "new-car") return;
  const id = freshId(c.name, cars);
  const move = (from, to) => { pack.files.set(to, pack.files.get(from)); pack.files.delete(from); return to; };
  c.side = move(c.side, `${id}.pic`);
  if (c.side_lamps) c.side_lamps = move(c.side_lamps, `${id}-lamps.pic`);
  cars[id] = c;
  delete cars[old];
  if (sources.has(old)) { sources.set(id, sources.get(old)); sources.delete(old); }
  if (pack.manifest.start.car === old) pack.manifest.start.car = id;
  at.car = id;
}

// Pictures nothing names any more are dropped from the pack.
function tidy() {
  const m = pack.manifest, used = new Set();
  for (const c of Object.values(m.cars)) {
    for (const p of Object.values(m.parts ?? {})) used.add(p.picture);
    for (const name of [c.side, c.side_lamps, c.hood?.picture, c.rear?.picture, c.cockpit?.picture, c.cockpit?.night, c.cockpit?.bonnet, c.cockpit?.wheel?.picture, c.cockpit?.wheel?.night]) used.add(name);
  }
  for (const p of Object.values(m.places)) used.add(p.picture);
  for (const name of [...pack.files.keys()]) if (name.endsWith(".pic") && !used.has(name)) pack.files.delete(name);
}

function carsPanel() {
  const m = pack.manifest;
  const out = [h("section", {}, h("h2", { text: "Cars in this pack" }),
    list(Object.entries(m.cars).map(([id, c]) => [id, c.name + (m.start.car === id ? "  (starts)" : "")]), at.car, (id) => { at.car = id; at.slot = null; render(); }),
    h("div", { class: "row" }, h("button", { text: "Add a car", onclick: addCar }), h("button", { text: "Remove this one", onclick: removeCar, disabled: !at.car })))];
  if (!at.car) return out;
  const c = car(), src = sources.get(at.car), pic = pack.files.get(c.side) && picDecode(pack.files.get(c.side)), canPaint = !!c.paint_ramps?.mid;
  const ground = h("input", { type: "range", min: 1, max: pic ? pic.h : 200, id: "ground" });
  ground.value = c.ground;
  ground.oninput = () => { c.ground = +ground.value; if (src) src.groundSet = true; $("groundOut").textContent = c.ground; drawSide(); touch(); };
  const paint = h("select", { id: "paint", style: "flex:1", disabled: !canPaint }, colours("As painted").map(([v, w]) => new Option(w, v)));
  paint.value = canPaint ? c.paint ?? "" : "";
  paint.onchange = () => { if (paint.value) c.paint = paint.value; else delete c.paint; render(); };
  const within = h("input", { type: "range", min: 5, max: 90, id: "within" });
  within.value = src?.within ?? 35;
  within.onchange = () => change(() => { src.within = +within.value; });
  const d = c.drive ?? {}, turbo = d.boost ?? null;
  const drive = (key, fallback) => (v) => { c.drive ??= {}; if (v == null) delete c.drive[key]; else c.drive[key] = v; if (fallback) c.drive[key] ??= fallback; };
  const numbers = (words) => words.split(/[\s,]+/).map(Number).filter((n) => n > 0);
  const curve = h("textarea", { id: "torque", spellcheck: "false" });
  curve.value = (d.torque ?? []).map((p) => p.join(" ")).join("\n");
  curve.oninput = () => {
    const rows = curve.value.split("\n").map(numbers).filter((p) => p.length === 2).sort((a, b) => a[0] - b[0]);
    c.drive ??= {};
    if (rows.length) c.drive.torque = rows; else delete c.drive.torque;
    touch();
  };
  out.push(h("section", {}, h("h2", { text: "This car" }),
    field("Its name", c.name, (v) => { c.name = v; }, { maxlength: 36, redo: true, id: "name", onchange: () => { fileCar(); render(); } }),
    dropZone("Its picture, from the side, nose to the right", (f) => dropped(f, "side")),
    dropZone("The same picture with the headlamps on (optional)", (f) => dropped(f, "lamps")),
    h("p", { class: "hint", text: "Use pictures that are yours to use. They stay in your browser until you save the pack." }),
    src?.side ? h("div", {},
      field("How wide it is in the game, in pixels (the screen is 320)", src.width, () => {}, { type: "number", min: 16, max: 320, id: "width", onchange: (e) => change(() => { src.width = +e.target.value || src.width; src.groundSet = false; }) }),
      tick("The corner's colour is background: make it clear", src.corner, (v) => change(() => { src.corner = v; }))) : null,
    h("label", { for: "ground" }, "Where the tyres touch the ground: row ", h("output", { id: "groundOut", text: String(c.ground) })), ground,
    h("label", { for: "paint", text: "Its paint" }),
    h("div", { class: "row" }, paint, h("span", { class: "swatch", style: `background:${canPaint && c.paint ? css(pal[RAMPS.indexOf(c.paint) * 32 + 20]) : "transparent"}` })),
    src?.body ? h("div", {}, h("label", { for: "within", text: "How much of the picture counts as paint" }), within) : null,
    h("p", { class: "hint", id: "paintHint", text: canPaint
      ? src?.body ? `${src.painted} pixels count as paint. Click the picture again to pick a different colour.` : "This picture's paint can be shown in any colour."
      : src?.side ? "To show this car in other colours, click its bodywork in the picture." : "Drop the picture in again to say which part of it is paint." }),
    field("What one in fair shape costs in the paper (blank: never for sale)", c.price, (v) => { if (v > 0) c.price = Math.floor(v); else delete c.price; }, { type: "number", min: 1, max: 99999, id: "price" }),
    tick("A new game starts with this car", m.start.car === at.car, (v) => { if (v) m.start.car = at.car; else delete m.start.car; }),
    h("details", { id: "driveBox" }, h("summary", { text: "What makes it go" }),
      h("p", { class: "hint", text: "Left blank, a car gets a two-litre turbo four and five gears." }),
      h("label", { for: "torque", text: "Torque at the wheels: an engine speed and the pound-feet it makes there, one pair to a line" }), curve,
      h("div", { class: "two" },
        field("Gears, first gear first", (d.gears ?? []).join(" "), (v) => drive("gears")(numbers(v).length ? numbers(v) : null), { id: "gears" }),
        field("Final drive", d.final, drive("final"), { type: "number", step: "0.01", min: 0.1 }),
        field("Redline, rpm", d.redline, drive("redline"), { type: "number", step: 100, min: 1000 }),
        field("Revs held on the line", d.launch, drive("launch"), { type: "number", step: 100, min: 1000 }),
        field("Weight, lb", d.weight_lb, drive("weight_lb"), { type: "number", min: 500 }),
        field("Grip (12 is all four tyres)", d.grip, drive("grip"), { type: "number", step: "0.5", min: 1 })),
      tick("It has a turbo", "boost" in d ? !!turbo : true, (v) => { c.drive ??= {}; c.drive.boost = v ? { from: 2200, psi: 20, lag: 6 } : null; }),
      turbo ? h("div", { class: "three" },
        field("Wakes at, rpm", turbo.from, (v) => { turbo.from = v ?? 2200; }, { type: "number", step: 100 }),
        field("Boost, psi", turbo.psi, (v) => { turbo.psi = v ?? 20; }, { type: "number" }),
        field("Lag (6 quick, 10 lazy)", turbo.lag, (v) => { turbo.lag = v ?? 6; }, { type: "number" })) : null),
    seatForm(c),
    h("details", { id: "hoodForm", open: !!c.hood }, h("summary", { text: "Under its hood" }),
      h("p", { class: "hint", text: "A picture of the engine bay as the car came, with every stock part on it. A photo will do: it is cut to 320 by 176 and brought into the game's colours. Parts are then taken off and put on there, bolt by bolt." }),
      dropZone("The engine bay", async (f) => {
        try { pack.files.set(`${at.car}-bay.pic`, await wholePicture(f, 176)); } catch { return fail("That file is not a picture the browser can read."); }
        c.hood = { picture: `${at.car}-bay.pic`, slots: c.hood?.slots ?? {} };
        tidy(); render();
      }),
      c.hood ? hoodSlots(c) : null)));
  return out;
}

// The view from the seat, and the car from behind.
function seatForm(c) {
  const k = c.cockpit;
  const drop = (label, name, most, set, small) => dropZone(label, async (f) => {
    try { pack.files.set(name, small ? await smallPicture(f, most, small === "square") : await wholePicture(f, most)); } catch { return fail("That file is not a picture the browser can read."); }
    set(name); tidy(); render();
  });
  const pair = (label, get, set, extra = []) => {
    const v = get() ?? [], put = (i) => (n) => { const now = [...(get() ?? [0, 0])]; now[i] = n ?? 0; set(now); };
    return h("div", {}, h("label", { text: label }), h("div", { class: extra.length ? "three" : "two" },
      field("Across", v[0], put(0), { type: "number" }), field("Down", v[1], put(1), { type: "number" }), extra));
  };
  const dial = (label, key, top) => {
    const d = k[key];
    return h("div", {}, tick(label, !!d, (on) => { if (on) k[key] = { at: [160, 170], r: 17, max: top }; else delete k[key]; }),
      d ? h("div", { class: "three" },
        field("Across", d.at[0], (n) => { d.at[0] = n ?? 0; }, { type: "number" }), field("Down", d.at[1], (n) => { d.at[1] = n ?? 0; }, { type: "number" }),
        field("Needle length", d.r, (n) => { d.r = n ?? 10; }, { type: "number" }), field("Full sweep reads", d.max, (n) => { d.max = n ?? top; }, { type: "number" })) : null);
  };
  return h("details", { id: "seatForm", ontoggle: () => preview() }, h("summary", { text: "From the seat" }),
    h("p", { class: "hint", text: "A painting of the cabin, 320 by 240, with the glass left clear. With one, the car is raced and driven from behind its wheel; without, from the side." }),
    drop("The cabin", `${at.car}-cockpit.pic`, 240, (n) => { c.cockpit = { view_h: 136, ...c.cockpit, picture: n }; }),
    k ? [
      drop("The cabin at night (optional)", `${at.car}-cockpit-night.pic`, 240, (n) => { k.night = n; }),
      drop("The bonnet through the glass, in the car's paint (optional)", `${at.car}-bonnet.pic`, 240, (n) => { k.bonnet = n; }),
      drop("The steering wheel: a square picture, up to 176", `${at.car}-wheel.pic`, 176, (n) => { k.wheel = { at: [120, 216], ...k.wheel, picture: n }; }, "square"),
      k.wheel ? pair("Where the middle of the wheel goes", () => k.wheel.at, (v) => { k.wheel.at = v; }) : null,
      field("How far down the view out goes", k.view_h, (n) => { k.view_h = n ?? 136; }, { type: "number", min: 60, max: 240 }),
      dial("A tachometer", "tach", 9000), dial("A speedometer", "speedo", 160), dial("A boost gauge", "boost", 30),
      tick("A shift light", !!k.shift_light, (on) => { if (on) k.shift_light = [120, 170]; else delete k.shift_light; }),
      k.shift_light ? pair("Where it is", () => k.shift_light, (v) => { k.shift_light = v; }) : null,
      tick("The gear lights up in the lever's gate", !!k.gear_gate, (on) => { if (on) k.gear_gate = [286, 224]; else delete k.gear_gate; }),
      k.gear_gate ? pair("The middle of the gate", () => k.gear_gate, (v) => { k.gear_gate = v; }) : null,
      h("div", { class: "row", style: "margin-top:10px" }, h("button", { text: "No cockpit for this car", onclick: () => { delete c.cockpit; tidy(); render(); } })),
    ] : null,
    drop("The car from behind, for when it is ahead of you (up to 96 wide)", `${at.car}-rear.pic`, 96, (n) => { c.rear = { width_m: 1.7, ...c.rear, picture: n }; }, "small"),
    field("Gallons its tank holds", c.tank ?? 17, (n) => { if (n > 0) c.tank = n; else delete c.tank; }, { type: "number", min: 1 }),
    Object.keys(pack.manifest.maps ?? {}).length ? h("p", { class: "hint", text: "While this is open, the game beside it is seen from this car's seat." }) : null);
}

// Where a slot's bolts are: where the pack says, or laid out round the box as the engine does it
// (corners first, then the middles of the sides).
function boltsOf(slot) {
  if (Array.isArray(slot.bolts) && slot.bolts.length) return slot.bolts.slice(0, 8).map((b) => [...b]);
  const [x, y, w, hh] = slot.rect, n = Math.max(1, Math.min(8, slot.bolts ?? 4));
  const l = x + 5, r = x + w - 6, t = y + 5, b = y + hh - 6, cx = x + (w >> 1), cy = y + (hh >> 1);
  const order = n === 1 ? [[cx, cy]] : n === 2 ? [[l, t], [r, b]] : n === 3 ? [[l, t], [r, t], [cx, b]] : [[l, t], [r, t], [r, b], [l, b], [cx, t], [cx, b], [l, cy], [r, cy]];
  return order.slice(0, n);
}

function hoodSlots(c) {
  const slots = c.hood.slots, all = [...new Set(Object.values(pack.manifest.parts ?? {}).map((p) => p.slot))].sort();
  const free = all.filter((s) => !(s in slots)), slot = slots[at.slot];
  const add = (name) => { slots[name] = { rect: [140, 70, 40, 40] }; at.slot = name; render(); };
  const named = h("input", { type: "text", id: "newSlot", maxlength: 14, placeholder: "Air filter", style: "flex:1" });
  return h("div", {},
    h("label", { text: "The places where a part sits" }),
    list(Object.keys(slots).map((s) => [s, `${s}: ${boltsOf(slots[s]).length} bolt${boltsOf(slots[s]).length === 1 ? "" : "s"}`]), at.slot, (s) => { at.slot = s; render(); }),
    h("div", { class: "row" }, free.map((name) => h("button", { text: `Add ${name}`, onclick: () => add(name) }))),
    h("label", { for: "newSlot", text: "Or a place of your own, for a part of the car that only ever comes off and goes back" }),
    h("div", { class: "row" }, named, h("button", { id: "addSlot", text: "Add it", onclick: () => { const name = named.value.trim(); if (!name) return; if (name in slots) return fail("There is one called that already."); add(name); } })),
    slot ? h("div", { class: "card" },
      h("div", { class: "two" },
        field(`Bolts holding the ${at.slot.toLowerCase()} on (1 to 8)`, boltsOf(slot).length, () => {}, { type: "number", min: 1, max: 8, id: "boltCount", onchange: (e) => { const n = Math.max(1, Math.min(8, Math.floor(+e.target.value) || 4)); if (n === 4) delete slot.bolts; else slot.bolts = n; render(); } }),
        field("What the stock part is called", slot.stock, (v) => { if (v) slot.stock = v; else delete slot.stock; }, { maxlength: 20, placeholder: `Stock ${at.slot.toLowerCase()}` })),
      h("p", { class: "hint", text: "On the picture, drag the box to where the part is, and drag each bolt to where it really is." }),
      h("div", { class: "row" }, h("button", { text: `Take ${at.slot} off the picture`, onclick: () => { delete slots[at.slot]; at.slot = null; render(); } }))) : null,
    h("div", { class: "row", style: "margin-top:10px" }, h("button", { text: "No hood for this car", onclick: () => { delete c.hood; tidy(); render(); } })));
}

function drawSide() {
  const c = at.car && car(), pic = c && pack.files.get(c.side) && picDecode(pack.files.get(c.side)), canvas = $("side");
  if (!pic) return;
  canvas.width = pic.w; canvas.height = pic.h + 2;
  canvas.style.width = pic.w * 2 + "px";
  const g = canvas.getContext("2d"), image = g.createImageData(pic.w, pic.h), out = new Uint32Array(image.data.buffer);
  pic.ix.forEach((v, i) => { out[i] = v === CLEAR ? 0 : pal[v]; });
  g.putImageData(image, 0, 0);
  g.fillStyle = "#ff4d6a";
  g.fillRect(0, c.ground, pic.w, 1);
  $("sideHint").textContent = `${pic.w} x ${pic.h}, ${new Set(pic.ix).size - 1} colours. The red line is the ground.`;
}

function drawHood() {
  const c = car(), pic = picDecode(pack.files.get(c.hood.picture)), canvas = $("hood"), over = $("hoodOver");
  canvas.width = pic.w; canvas.height = pic.h; canvas.style.width = "100%";
  over.width = pic.w * 2; over.height = pic.h * 2;
  const g = canvas.getContext("2d"), image = g.createImageData(pic.w, pic.h), out = new Uint32Array(image.data.buffer);
  pic.ix.forEach((v, i) => { out[i] = v === CLEAR ? 0xff000000 : pal[v]; });
  g.putImageData(image, 0, 0);
  // The boxes, and the bolts of the one that is chosen, each to drag.
  const slots = c.hood.slots, names = Object.keys(slots), chosen = slots[at.slot], bolts = chosen ? boltsOf(chosen) : [];
  let was = chosen ? [...chosen.rect] : null;
  boxes(over, 2, () => [...names.map((s) => ({ rect: slots[s].rect, name: s })), ...bolts.map((b) => ({ rect: b, name: "", point: true }))], {
    selected: () => names.indexOf(at.slot), select: (i) => { if (i < names.length) at.slot = names[i] ?? null; },
    moved: (i) => {
      if (i >= names.length) chosen.bolts = bolts.map((b) => [...b]);
      else if (slots[names[i]] === chosen && Array.isArray(chosen.bolts)) {
        // Bolts that have been put somewhere go with their box.
        const dx = chosen.rect[0] - was[0], dy = chosen.rect[1] - was[1];
        for (const b of bolts) { b[0] += dx; b[1] += dy; }
        chosen.bolts = bolts.map((b) => [...b]);
        was = [...chosen.rect];
      }
      keep();
    },
    done: () => render(),
  });
}

$("side").onclick = (e) => {
  const src = sources.get(at.car);
  if (!src?.rgba) return;
  const r = $("side").getBoundingClientRect();
  const x = Math.floor((e.clientX - r.left) / r.width * src.rgba.w), y = Math.floor((e.clientY - r.top) / r.height * (src.rgba.h + 2));
  if (y >= src.rgba.h) return;
  const i = (y * src.rgba.w + x) * 4;
  if (src.rgba.rgba[i + 3] < 128) return;
  change(() => { src.body = [src.rgba.rgba[i], src.rgba.rgba[i + 1], src.rgba.rgba[i + 2]]; });
};

// ---- Rooms ----

function roomItems() {
  const p = place(), items = p.spots.map((s) => ({ rect: s.rect, name: s.button || s.label.slice(0, 18) }));
  if (p.car_at) items.push({ rect: p.car_at, name: "your car", point: true });
  if (p.rival_at) items.push({ rect: p.rival_at, name: "the rival", point: true });
  return items;
}

function roomsPanel() {
  const m = pack.manifest, p = place();
  const out = [h("section", {}, h("h2", { text: "Rooms in this pack" }),
    list(Object.entries(m.places).map(([id, pl]) => [id, pl.title + (m.start.place === id ? "  (starts)" : "")]), at.place, (id) => { at.place = id; at.spot = -1; render(); }),
    h("div", { class: "row" },
      h("button", { text: "Add a room", onclick: () => { const id = freshId("new-room", m.places); m.places[id] = { title: "New room", backdrop: "NAVY", spots: [] }; at.place = id; at.spot = -1; render(); } }),
      h("button", { text: "Remove this one", onclick: () => {
        const from = Object.entries(m.places).find(([id, pl]) => id !== at.place && pl.spots.some((s) => s.do === "go:" + at.place));
        if (m.start.place === at.place) return fail("The game starts here. Start it somewhere else first.");
        if (from) return fail(`${from[1].title} has a way in here. Change that first.`);
        if (!confirm(`Remove ${p.title} from the pack?`)) return;
        delete m.places[at.place]; at.place = m.start.place; at.spot = -1; tidy(); render();
      } })))];
  out.push(h("section", {}, h("h2", { text: "This room" }),
    field("Its name", p.title, (v) => { p.title = v; }, { maxlength: 30, redo: true, id: "roomTitle" }),
    dropZone("Its picture: 320 wide, 200 tall", async (f) => {
      try { pack.files.set(`${at.place}.pic`, await wholePicture(f, 200)); } catch { return fail("That file is not a picture the browser can read."); }
      p.picture = `${at.place}.pic`; render();
    }),
    p.picture ? h("div", { class: "row", style: "margin-top:8px" }, h("button", { text: "Take the picture away", onclick: () => { delete p.picture; p.backdrop ??= "NAVY"; tidy(); render(); } }))
      : choice("With no picture, its colour", p.backdrop ?? "NAVY", colours(), (v) => { p.backdrop = v; }),
    tick("Your car stands here", !!p.car_at, (v) => { if (v) p.car_at = [40, 182]; else delete p.car_at; }),
    tick("Rivals pull in here", !!p.rival_at, (v) => { if (v) p.rival_at = [72, 174]; else { delete p.rival_at; } }),
    tick("A new game starts here", m.start.place === at.place, (v) => { if (v) m.start.place = at.place; })));
  const s = p.spots[at.spot];
  const spots = h("section", {}, h("h2", { text: "Things to pick in it" }),
    h("p", { class: "hint", text: "Each is a box on the picture. Click one to choose it; drag it to move it, or its corner to size it. Later ones lie on top." }),
    h("div", { class: "row" },
      h("button", { id: "addSpot", text: "Add one", onclick: () => { p.spots.push({ rect: [130, 80, 60, 30], label: "Something", do: "" }); at.spot = p.spots.length - 1; render(); } }),
      s ? h("button", { text: "Remove it", onclick: () => { p.spots.splice(at.spot, 1); at.spot = -1; render(); } }) : null));
  if (s) {
    const [kind, ...rest] = s.do.split(":"), arg = rest.join(":");
    const others = Object.entries(m.places).filter(([id]) => id !== at.place).map(([id, pl]) => [id, pl.title]);
    spots.append(h("div", { class: "card" },
      field("What it is called", s.label, (v) => { s.label = v; }, { maxlength: 38, id: "spotLabel" }),
      choice("What picking it does", kind, DOES, (v) => { s.do = v === "say" ? "say:" : v === "go" ? "go:" + (others[0]?.[0] ?? "") : v === "drive" ? "drive:" + (Object.keys(m.maps ?? {})[0] ?? "") : v; }),
      kind === "say" ? field("What it says", arg, (v) => { s.do = "say:" + v; }, { maxlength: 38, id: "spotSays" }) : null,
      kind === "go" ? choice("Where to", arg, others, (v) => { s.do = "go:" + v; }) : null,
      kind === "drive" ? choice("Which map", arg, Object.entries(m.maps ?? {}).map(([id, mp]) => [id, mp.title]), (v) => { s.do = "drive:" + v; }) : null,
      field("Draw it as a button with this word on it (blank: the picture shows it)", s.button, (v) => { if (v) s.button = v; else delete s.button; }, { maxlength: 9 }),
      tick("Only there when you have a car", s.needs_car, (v) => { if (v) s.needs_car = true; else delete s.needs_car; })));
  }
  out.push(spots);
  return out;
}

// ---- Maps ----

const map = () => pack.manifest.maps?.[at.map];

function mapsPanel() {
  const m = pack.manifest;
  m.maps ??= {};
  if (!(at.map in m.maps)) at.map = Object.keys(m.maps)[0] ?? null;
  const out = [h("section", {}, h("h2", { text: "Maps in this pack" }),
    list(Object.entries(m.maps).map(([id, mp]) => [id, mp.title]), at.map, (id) => { at.map = id; at.brush = "."; render(); }),
    h("div", { class: "row" },
      h("button", { id: "addMap", text: "Add a map", onclick: () => {
        const id = freshId("new-map", m.maps);
        m.maps[id] = { title: "New map", rows: ["B.B.", "....", ".B.B"], blocks: { B: { wall: "BRICK", height: 16 } } };
        at.map = id; at.brush = "B"; render();
      } }),
      at.map ? h("button", { text: "Remove this one", onclick: () => {
        const from = Object.values(m.places).find((p) => p.spots.some((s) => s.do === "drive:" + at.map));
        if (from) return fail(`${from.title} has a way out onto this map. Change that first.`);
        if (!confirm(`Remove ${map().title} from the pack?`)) return;
        delete m.maps[at.map]; at.map = null; render();
      } }) : null))];
  if (!at.map) return out;
  const mp = map(), cols = mp.rows[0].length, rows = mp.rows.length;
  const size = (c, r) => { mp.rows = Array.from({ length: r }, (_, j) => (mp.rows[j] ?? "").padEnd(c, ".").slice(0, c)); };
  out.push(h("section", {}, h("h2", { text: "This map" }),
    field("Its name", mp.title, (v) => { mp.title = v; }, { maxlength: 24, redo: true, id: "mapTitle" }),
    h("div", { class: "two" },
      field("Blocks across", cols, () => {}, { type: "number", min: 1, max: 24, id: "mapCols", onchange: (e) => { size(Math.max(1, Math.min(24, +e.target.value || cols)), rows); render(); } }),
      field("Blocks down", rows, () => {}, { type: "number", min: 1, max: 24, id: "mapRows", onchange: (e) => { size(cols, Math.max(1, Math.min(24, +e.target.value || rows))); render(); } }),
      field("Other cars about", mp.traffic ?? 8, (n) => { mp.traffic = Math.max(0, Math.min(40, Math.floor(n ?? 8))); }, { type: "number", min: 0, max: 40 }),
      field("Gallons burned a kilometre", mp.gas_per_km ?? 0.4, (n) => { mp.gas_per_km = Math.max(0, n ?? 0.4); }, { type: "number", step: "0.1", min: 0 })),
    h("p", { class: "hint", text: "There is a street between every two blocks and one round the outside. The top of the map is north." })));
  const places = [["", "Nowhere: it is only a building"], ...Object.entries(m.places).map(([id, p]) => [id, p.title])];
  const brush = (ch, words, b) => h("button", { "aria-current": String(at.brush === ch), onclick: () => { at.brush = ch; render(); } },
    h("span", { class: "swatch", style: `background:${ch === "." ? css(pal[RAMPS.indexOf("GRASS") * 32 + 20]) : b.height > 0 ? css(pal[RAMPS.indexOf(b.wall ?? "CONCRETE") * 32 + 20]) : "#e6b84a"};margin-right:8px` }), words);
  const kind = mp.blocks[at.brush];
  out.push(h("section", {}, h("h2", { text: "What stands on a block" }),
    h("p", { class: "hint", text: "Pick one, then click or drag over the map to put it down." }),
    h("ul", { class: "list" }, h("li", {}, brush(".", "Grass")), Object.entries(mp.blocks).map(([ch, b]) => h("li", {}, brush(ch, `${ch}: ${b.name ?? (b.height > 0 ? `${nice(b.wall ?? "CONCRETE")}, ${b.height} m` : "open ground")}`, b)))),
    h("div", { class: "row" },
      h("button", { id: "addBlock", text: "Add a kind", onclick: () => {
        const ch = [..."ABCDEFGHIJKLMNOPQRSTUVWXYZ"].find((x) => !(x in mp.blocks));
        if (!ch) return fail("A map has room for 26 kinds of block.");
        mp.blocks[ch] = { wall: "CONCRETE", height: 16 }; at.brush = ch; render();
      } }),
      kind ? h("button", { text: "Remove this kind", onclick: () => { mp.rows = mp.rows.map((r) => r.replaceAll(at.brush, ".")); delete mp.blocks[at.brush]; at.brush = "."; render(); } }) : null),
    kind ? h("div", { class: "card" },
      h("div", { class: "two" },
        choice("Built of", kind.wall ?? "CONCRETE", colours(), (v) => { kind.wall = v; }),
        field("How tall, in metres (0: open ground)", kind.height ?? 0, (n) => { kind.height = Math.max(0, n ?? 0); }, { type: "number", min: 0, max: 80, redo: true })),
      choice("Pulling up beside it takes you to", kind.to ?? "", places, (v) => { if (v) kind.to = v; else { delete kind.to; delete kind.name; } }),
      kind.to ? field("What it is called on the list in the windscreen", kind.name, (v) => { if (v) kind.name = v; else delete kind.name; }, { maxlength: 16, redo: true }) : null) : null));
  return out;
}

// The map from above, to paint on.
function drawMap() {
  const mp = map(), canvas = $("mapView");
  if (!mp) return;
  const cols = mp.rows[0].length, rows = mp.rows.length, cell = Math.max(14, Math.min(48, Math.floor(600 / Math.max(cols, rows)))), road = Math.max(3, cell >> 2);
  canvas.width = cols * cell + road; canvas.height = rows * cell + road;
  canvas.style.width = Math.min(640, canvas.width) + "px";
  const g = canvas.getContext("2d");
  g.fillStyle = css(pal[RAMPS.indexOf("ASPHALT") * 32 + 14]);
  g.fillRect(0, 0, canvas.width, canvas.height);
  g.font = `bold ${Math.max(9, cell >> 1)}px ui-monospace, monospace`;
  g.textAlign = "center"; g.textBaseline = "middle";
  mp.rows.forEach((row, j) => [...row].forEach((ch, i) => {
    const b = mp.blocks[ch], x = i * cell + road, y = j * cell + road, w = cell - road;
    g.fillStyle = !b ? css(pal[RAMPS.indexOf("GRASS") * 32 + 18]) : b.height > 0 ? css(pal[RAMPS.indexOf(b.wall ?? "CONCRETE") * 32 + 20]) : b.to ? "#e6b84a" : css(pal[RAMPS.indexOf("GRASS") * 32 + 22]);
    g.fillRect(x, y, w, w);
    if (b?.to) { g.strokeStyle = "#e6b84a"; g.lineWidth = 2; g.strokeRect(x + 1, y + 1, w - 2, w - 2); }
    if (b) { g.fillStyle = "rgba(0,0,0,.75)"; g.fillText(ch, x + w / 2 + 1, y + w / 2 + 1); g.fillStyle = "#fff"; g.fillText(ch, x + w / 2, y + w / 2); }
  }));
  const paint = (e) => {
    const r = canvas.getBoundingClientRect();
    const i = Math.floor(((e.clientX - r.left) / r.width * canvas.width - road / 2) / cell), j = Math.floor(((e.clientY - r.top) / r.height * canvas.height - road / 2) / cell);
    if (i < 0 || j < 0 || i >= cols || j >= rows || mp.rows[j][i] === at.brush) return;
    mp.rows[j] = mp.rows[j].slice(0, i) + at.brush + mp.rows[j].slice(i + 1);
    drawMap(); touch();
  };
  canvas.onpointerdown = (e) => { canvas.setPointerCapture(e.pointerId); canvas.onpointermove = paint; paint(e); };
  canvas.onpointerup = () => { canvas.onpointermove = null; };
}

// ---- Parts ----

function partsPanel() {
  const m = pack.manifest;
  m.parts ??= {};
  const cars = carChoices();
  return [h("section", {}, h("h2", { text: "Parts in the paper" }),
    h("p", { class: "hint", text: "One part to a slot on a car. The slots are whatever the parts name: Intake, Turbo, Tyres. The paper has room for 14." }),
    Object.entries(m.parts).map(([id, p]) => h("div", { class: "card" },
      h("div", { class: "two" },
        field("Its name", p.name, (v) => { p.name = v; }, { maxlength: 20 }),
        field("Its slot", p.slot, (v) => { p.slot = v; }, { maxlength: 14 }),
        field("Price", p.price, (v) => { p.price = Math.max(0, Math.floor(v ?? 0)); }, { type: "number", min: 0 }),
        field("More torque, per cent", p.torque, (v) => { if (v) p.torque = Math.floor(v); else delete p.torque; }, { type: "number" })),
      field("What the ad says", p.pitch, (v) => { p.pitch = v; }, { maxlength: 24 }),
      tick("It is a turbo, and replaces the car's own", !!p.boost, (v) => { if (v) p.boost = { from: 3000, psi: 27, lag: 10 }; else delete p.boost; }),
      p.boost ? h("div", { class: "three" },
        field("Wakes at, rpm", p.boost.from, (v) => { p.boost.from = v ?? 3000; }, { type: "number", step: 100 }),
        field("Boost, psi", p.boost.psi, (v) => { p.boost.psi = v ?? 27; }, { type: "number" }),
        field("Lag", p.boost.lag, (v) => { p.boost.lag = v ?? 10; }, { type: "number" })) : null,
      dropZone(p.picture ? "How it looks bolted on under the hood (it has a picture; drop another to change it)" : "How it looks bolted on under the hood: a picture of just this part (optional)", async (f) => {
        // Made the size of its place under the first hood that has one.
        const place = Object.values(m.cars).map((c) => c.hood?.slots?.[p.slot]).find(Boolean);
        if (!place) return fail(`No car has a place for the ${p.slot.toLowerCase()} under its hood yet. Add one under Cars first.`);
        try { pack.files.set(`${id}-on.pic`, await sizedPicture(f, place.rect[2], place.rect[3])); } catch { return fail("That file is not a picture the browser can read."); }
        p.picture = `${id}-on.pic`; render();
      }),
      p.picture ? h("div", { class: "row", style: "margin-top:8px" }, h("button", { text: "No picture: mark it with a tag", onclick: () => { delete p.picture; tidy(); render(); } })) : null,
      h("label", { text: "Fits (none ticked: every car)" }),
      cars.map(([cid, name]) => tick(name, p.fits?.includes(cid), (v) => { p.fits = (p.fits ?? []).filter((x) => x !== cid); if (v) p.fits.push(cid); if (!p.fits.length) delete p.fits; })),
      h("div", { class: "row", style: "margin-top:10px" }, h("button", { text: "Remove this part", onclick: () => { delete m.parts[id]; render(); } })))),
    h("button", { id: "addPart", text: "Add a part", disabled: Object.keys(m.parts).length >= 14, onclick: () => { m.parts[freshId("part", m.parts)] = { name: "New part", slot: "Intake", price: 100, pitch: "", torque: 5 }; render(); } }))];
}

// ---- Rivals ----

function rivalsPanel() {
  const m = pack.manifest, cars = carChoices();
  m.rivals ??= [];
  const last = m.rivals.length - 1;
  const order = () => m.rivals.forEach((r, i) => { if (i !== m.rivals.length - 1) delete r.king; });
  const swap = (i, j) => { [m.rivals[i], m.rivals[j]] = [m.rivals[j], m.rivals[i]]; order(); render(); };
  return [h("section", {}, h("h2", { text: "Who there is to race" }),
    h("p", { class: "hint", text: "They pull in in this order, each once the one before is beaten for money. Somewhere in a room has to be marked as where rivals pull in." }),
    cars.length ? null : h("p", { class: "hint bad", text: "There are no cars in the pack for them to drive." }),
    m.rivals.map((r, i) => h("div", { class: "card" },
      h("div", { class: "two" },
        field(`${i + 1}. Name`, r.name, (v) => { r.name = v; }, { maxlength: 20 }),
        field("Quarter mile, seconds", r.et, (v) => { r.et = v ?? 0; }, { type: "number", step: "0.05", min: 5 }),
        choice("Drives", r.car, cars, (v) => { r.car = v; }),
        choice("In", r.paint ?? "", colours("Its own paint"), (v) => { if (v) r.paint = v; else delete r.paint; }),
        field("Most money they will race for", r.limit, (v) => { r.limit = Math.max(0, Math.floor(v ?? 0)); }, { type: "number", min: 0 })),
      field("What they say", r.says, (v) => { r.says = v; }, { maxlength: 38 }),
      tick("Will race for cars", r.pinks || r.king, (v) => { if (v) r.pinks = true; else { delete r.pinks; delete r.king; } }),
      i === last ? tick("The one to beat: races for cars only, and beating them wins the game", r.king, (v) => { if (v) r.king = true; else delete r.king; }) : null,
      h("div", { class: "row", style: "margin-top:10px" },
        h("button", { text: "Earlier", disabled: i === 0, onclick: () => swap(i, i - 1) }),
        h("button", { text: "Later", disabled: i === last, onclick: () => swap(i, i + 1) }),
        h("button", { text: "Remove", onclick: () => { m.rivals.splice(i, 1); order(); render(); } })))),
    h("button", { id: "addRival", text: "Add a rival", disabled: !cars.length, onclick: () => {
      // A new one goes in ahead of the one to beat, who stays last.
      const king = m.rivals[last]?.king ? m.rivals.pop() : null, before = m.rivals[m.rivals.length - 1];
      m.rivals.push({ name: "Someone new", says: "", car: cars[0][0], et: before ? Math.round((before.et - 0.1) * 100) / 100 : 18, limit: before?.limit ?? 25 });
      if (king) m.rivals.push(king);
      render();
    } }))];
}

// ---- The game ----

function gamePanel() {
  const m = pack.manifest;
  m.season ??= { days: 90, hours_per_day: 12 };
  return [h("section", {}, h("h2", { text: "The game" }),
    field("What the pack is called", m.name, (v) => { m.name = v; $("packName").textContent = v; }, { maxlength: 40, id: "gameName" }),
    h("div", { class: "two" },
      field("Money to start with", m.start.cash, (v) => { m.start.cash = Math.max(0, Math.floor(v ?? 0)); }, { type: "number", min: 0, id: "cash" }),
      choice("Starts in", m.start.place, Object.entries(m.places).map(([id, p]) => [id, p.title]), (v) => { m.start.place = v; }),
      field("Days in the season", m.season.days, (v) => { m.season.days = Math.max(1, Math.floor(v ?? 90)); }, { type: "number", min: 1 }),
      field("Hours in a day", m.season.hours_per_day, (v) => { m.season.hours_per_day = Math.max(1, Math.floor(v ?? 12)); }, { type: "number", min: 1 })),
    choice("Starts with", m.start.car ?? "", [["", "No car: buy one from the paper"], ...carChoices()], (v) => { if (v) m.start.car = v; else delete m.start.car; }),
    field("What can be bet, in dollars (blank: 10 25 50 100)", (m.bets ?? []).join(" "), (v) => { const n = v.split(/[\s,]+/).map(Number).filter((x) => x > 0).slice(0, 4); if (n.length) m.bets = n; else delete m.bets; }),
    h("p", { class: "hint", text: "Reading the paper takes an hour, taking a part off takes an hour, a race takes two. When the days run out the game is over." }))];
}

// ---- Putting it on the page ----

const PANELS = { cars: carsPanel, rooms: roomsPanel, maps: mapsPanel, parts: partsPanel, rivals: rivalsPanel, game: gamePanel };
let redraw = () => {};

function render() {
  ids = 0;
  const m = pack.manifest, open = Object.fromEntries([...document.querySelectorAll("#panel details")].map((d) => [d.id, d.open]));
  if (!(at.car in m.cars)) at.car = Object.keys(m.cars)[0] ?? null;
  if (!(at.place in m.places)) at.place = m.start.place;
  fail("");
  $("packName").textContent = m.name;
  $("tabs").replaceChildren(...TABS.map(([id, words]) => h("button", { role: "tab", id: "tab-" + id, "aria-selected": String(at.tab === id), text: words, onclick: () => { at.tab = id; render(); } })));
  $("panel").replaceChildren(...PANELS[at.tab]());
  for (const d of document.querySelectorAll("#panel details")) if (d.id in open) d.open = open[d.id];
  const cars = at.tab === "cars" && at.car, rooms = at.tab === "rooms", maps = at.tab === "maps" && !!map();
  $("mapBox").hidden = !maps;
  if (maps) drawMap();
  $("sideBox").hidden = !cars;
  $("hoodBox").hidden = !(cars && car().hood);
  $("lampsBox").hidden = !cars;
  $("viewOver").hidden = !rooms;
  $("viewTitle").textContent = rooms ? place().title : maps ? "From the seat, in the south-west corner or beside the first place to go" : "In the game";
  $("viewHint").textContent = rooms ? "The crosses are where the tyres of your car and the rival's meet the floor: drag them." : "";
  if (cars) { drawSide(); if (car().hood) drawHood(); }
  redraw = rooms ? boxes($("viewOver"), 2, roomItems, {
    selected: () => at.spot, select: (i) => { at.spot = i < place().spots.length ? i : -1; },
    moved: () => touch(), done: () => render(),
  }) : () => {};
  keep();
  preview();
}

async function preview() {
  const m = pack.manifest;
  let shown = m, lamps = null;
  const seat = at.tab === "cars" && at.car && document.querySelector("#seatForm")?.open && Object.keys(m.maps ?? {})[0];
  if ((at.tab === "maps" && map()) || seat) {
    // Out on the map: a way onto it is added to a room for the look, and picked.
    const id = seat || at.map, mp = m.maps[id], car = seat ? at.car : m.start.car ?? Object.keys(m.cars)[0];
    const from = Object.values(mp.blocks).find((b) => b.to in m.places)?.to ?? m.start.place;
    const room = { ...m.places[from], spots: [...m.places[from].spots, { rect: [0, 0, 320, 200], label: "", do: "drive:" + id }] };
    shown = { ...m, start: { ...m.start, place: from, car }, places: { ...m.places, [from]: room } };
    if (!car) delete shown.start.car;
    lamps = [0, 0, 320, 200];
    const { ok, rgba } = await viewer.view(buildPack({ manifest: shown, files: pack.files }), { lamps: true, spot: lamps });
    $("view").getContext("2d").putImageData(new ImageData(rgba, 320, 240), 0, 0);
    if (!ok) fail("The engine will not play this pack as it stands; the screen says why.");
    else if (!car) fail("There is no car in the pack to drive it in.");
    else if ($("problem").textContent.startsWith("The engine will not")) fail("");
    return;
  }
  if (at.tab === "cars" && at.car) {
    shown = { ...m, start: { ...m.start, car: at.car } };
    lamps = m.places[m.start.place]?.spots?.find((s) => s.do === "lamps")?.rect ?? null;
  } else if (at.tab === "rooms") {
    // The room as it is with a car in it, so the things that need one show.
    shown = { ...m, start: { ...m.start, place: at.place, car: m.start.car ?? Object.keys(m.cars)[0] } };
    if (!shown.start.car) delete shown.start.car;
  }
  const { ok, rgba } = await viewer.view(buildPack({ manifest: shown, files: pack.files }), { lamps: $("lampsOn").checked && !!lamps, spot: lamps });
  $("view").getContext("2d").putImageData(new ImageData(rgba, 320, 240), 0, 0);
  if (!ok) fail("The engine will not play this pack as it stands; the screen says why.");
  else if ($("problem").textContent.startsWith("The engine will not")) fail("");
  redraw();
}

$("lampsOn").onchange = () => preview();
$("open").onchange = async () => {
  const f = $("open").files[0];
  if (!f) return;
  try { await start(new Uint8Array(await f.arrayBuffer())); } catch (e) { fail(e.message); }
  $("open").value = "";
};
$("fresh").onclick = async () => {
  if (!confirm("Throw away the changes and go back to the default pack?")) return;
  localStorage.removeItem(KEEP);
  await start(new Uint8Array(await (await fetch("default.pack")).arrayBuffer()));
};
const blob = () => new Blob([buildPack(pack)], { type: "application/octet-stream" });
$("save").onclick = () => {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob());
  a.download = slug(pack.manifest.name) + ".pack";
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
};
// The game opens in its own tab and reads the pack from this one, so keep this tab open.
$("play").onclick = () => window.open("engine.html?pack=" + encodeURIComponent(URL.createObjectURL(blob())));

let kept = null;
try { kept = Uint8Array.from(atob(localStorage.getItem(KEEP) ?? ""), (ch) => ch.charCodeAt(0)); } catch { /* nothing kept */ }
try { if (!kept?.length) throw 0; await start(kept); } catch { await start(new Uint8Array(await (await fetch("default.pack")).arrayBuffer())); }
