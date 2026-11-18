// Content packs, in the browser: reading one, writing one, and turning an ordinary picture into
// the engine's kind. It does what tools/pack.py does, so the workshop needs nothing but the page.
// The format is in docs/ENGINE.md.

export const W = 320, H = 240, CLEAR = 0xffff;

// The colour ramps, in the engine's order (engine/src/gfx.rs). A colour is ramp * 32 + shade.
export const RAMPS = [
  "SYSTEM", "GREY", "RED", "ORANGE", "YELLOW", "LIME", "GREEN", "TEAL", "CYAN", "SKY", "BLUE", "INDIGO", "VIOLET", "MAGENTA", "PINK", "BROWN",
  "TAN", "BRICK", "CONCRETE", "ASPHALT", "GRASS", "GLASS", "CHROME", "RUBBER", "WOOD", "CREAM", "NAVY", "MAROON", "FOREST", "GOLD", "SILVER", "WHITE",
];

const text = new TextDecoder(), bytesOf = new TextEncoder();

// A pack file, as its description and its other files by name.
export function parsePack(bytes) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (bytes.length < 8 || text.decode(bytes.subarray(0, 4)) !== "WPK1") throw new Error("This is not a pack: it does not begin WPK1.");
  const files = new Map();
  let at = 8, manifest = null;
  const need = (n) => { if (at + n > bytes.length) throw new Error("The pack ends early."); };
  for (let i = view.getUint32(4, true); i > 0; i--) {
    need(2);
    const n = view.getUint16(at, true); at += 2;
    need(n + 4);
    const name = text.decode(bytes.subarray(at, at + n)); at += n;
    const len = view.getUint32(at, true); at += 4;
    need(len);
    const data = bytes.slice(at, at + len); at += len;
    if (name === "pack.json") manifest = JSON.parse(text.decode(data));
    else files.set(name, data);
  }
  if (!manifest) throw new Error("The pack has no pack.json.");
  return { manifest, files };
}

export function buildPack({ manifest, files }) {
  const all = [["pack.json", bytesOf.encode(JSON.stringify(manifest, null, 2))], ...[...files].sort((a, b) => (a[0] < b[0] ? -1 : 1))];
  const names = all.map(([name]) => bytesOf.encode(name));
  const out = new Uint8Array(8 + all.reduce((n, [, data], i) => n + 6 + names[i].length + data.length, 0));
  const view = new DataView(out.buffer);
  out.set(bytesOf.encode("WPK1"));
  view.setUint32(4, all.length, true);
  let at = 8;
  all.forEach(([, data], i) => {
    view.setUint16(at, names[i].length, true); at += 2;
    out.set(names[i], at); at += names[i].length;
    view.setUint32(at, data.length, true); at += 4;
    out.set(data, at); at += data.length;
  });
  return out;
}

// A .pic: width, height, then runs of (count, colour), all 16-bit.
export function picDecode(bytes) {
  const words = new Uint16Array(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + (bytes.byteLength & ~1)));
  const w = words[0], h = words[1], ix = new Uint16Array(w * h);
  let at = 0;
  for (let i = 2; i + 1 < words.length; i += 2) { ix.fill(words[i + 1], at, at + words[i]); at += words[i]; }
  return { w, h, ix };
}

export function picEncode(w, h, ix) {
  const words = [w, h];
  for (let i = 0; i < ix.length;) {
    let j = i;
    while (j < ix.length && ix[j] === ix[i] && j - i < 65535) j++;
    words.push(j - i, ix[i]);
    i = j;
  }
  return new Uint8Array(new Uint16Array(words).buffer);
}

const BAYER = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5].map((v) => v / 16 - 0.5);

// The nearest of the palette's colours to r, g, b, among the ramps from..to. `pal` is the
// engine's palette: 1,024 colours with red in the low byte.
function nearest(pal, r, g, b, from, to) {
  let best = from * 32, bestD = Infinity;
  for (let i = from * 32; i < to * 32; i++) {
    const c = pal[i], dr = r - (c & 255), dg = g - ((c >> 8) & 255), db = b - ((c >> 16) & 255);
    const d = dr * dr * 0.9 + dg * dg * 1.2 + db * db * 0.7;
    if (d < bestD) { bestD = d; best = i; }
  }
  return best;
}

// The ramp a colour belongs to.
export function rampOf(pal, [r, g, b]) {
  return nearest(pal, r, g, b, 1, 32) >> 5;
}

// How unlike two colours are with their brightness taken out, 0 for the same hue.
function hueApart(r, g, b, to) {
  const m = Math.max(r, g, b, 1), n = Math.max(to[0], to[1], to[2], 1);
  return Math.hypot(r / m - to[0] / n, g / m - to[1] / n, b / m - to[2] / n);
}

// An RGBA picture as the engine's colours, dithered. Clear pixels stay clear; with `clear` set,
// that colour is clear too (for pictures saved on a plain background).
//
// `body` says which part is paint: { rgb, within }. Pixels of about that hue are all put on one
// ramp, whatever their brightness, so the engine can show the car in another colour by swapping
// that ramp. The ramp used comes back as `ramp`.
export function toIndices(rgba, w, h, pal, { dither = 8, clear = null, body = null } = {}) {
  const ix = new Uint16Array(w * h).fill(CLEAR);
  const ramp = body ? rampOf(pal, body.rgb) : 0;
  const seen = new Map();
  let painted = 0;
  for (let i = 0; i < w * h; i++) {
    const r0 = rgba[i * 4], g0 = rgba[i * 4 + 1], b0 = rgba[i * 4 + 2];
    if (rgba[i * 4 + 3] < 128) continue;
    if (clear && Math.abs(r0 - clear[0]) + Math.abs(g0 - clear[1]) + Math.abs(b0 - clear[2]) < 24) continue;
    const d = BAYER[((i / w | 0) & 3) * 4 + (i % w & 3)] * dither;
    const r = Math.round(r0 + d), g = Math.round(g0 + d), b = Math.round(b0 + d);
    const isBody = body && Math.max(r0, g0, b0) >= 24 && hueApart(r0, g0, b0, body.rgb) < body.within;
    const key = ((r + 64) << 20) | ((g + 64) << 10) | (b + 64) | (isBody ? 1 << 30 : 0);
    let c = seen.get(key);
    if (c === undefined) {
      c = isBody ? nearest(pal, r, g, b, ramp, ramp + 1) : nearest(pal, r, g, b, 1, 32);
      seen.set(key, c);
    }
    ix[i] = c;
    if (isBody) painted++;
  }
  return { ix, ramp, painted };
}

// The lowest row with anything on it, plus one: where a car's tyres meet the ground.
export function groundOf(w, h, ix) {
  for (let y = h - 1; y >= 0; y--) for (let x = 0; x < w; x++) if (ix[y * w + x] !== CLEAR) return y + 1;
  return h;
}

// A name fit for a file or an id: lower case, digits and dashes.
export function slug(words) {
  return words.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "car";
}

// The engine, for looking at a pack without playing it: load it, press Enter past the title, and
// hand back the screen as RGBA. `lamps` picks the first spot that works the headlamps, if any.
export async function makeViewer(wasmBytes) {
  const module = await WebAssembly.compile(wasmBytes);
  const start = async () => (await WebAssembly.instantiate(module, { env: { wtk_tone: () => {}, wtk_store: () => {}, wtk_fetch: () => 0 } })).exports;
  const first = await start();
  first.wtk_init(1);
  const palette = new Uint32Array(first.memory.buffer, first.wtk_palette(), 1024).slice();
  return {
    palette,
    async view(packBytes, { lamps = false, spot = null } = {}) {
      const x = await start();
      const at = x.pack_alloc(packBytes.length);
      new Uint8Array(x.memory.buffer, at, packBytes.length).set(packBytes);
      const ok = x.pack_load() === 1;
      x.wtk_init(1);
      const step = (keys, mx, my) => { x.wtk_input(keys, mx, my); x.wtk_step(); };
      if (ok) {
        step(16, 160, 100); step(0, 160, 100);
        if (lamps && spot) { const [sx, sy, sw, sh] = spot; step(0, sx + (sw >> 1), sy + (sh >> 1)); step(16, sx + (sw >> 1), sy + (sh >> 1)); }
        step(0, 319, 199);
      }
      const screen = new Uint16Array(x.memory.buffer, x.wtk_screen(), W * H);
      const out = new Uint8ClampedArray(W * H * 4);
      new Uint32Array(out.buffer).set(Array.from(screen, (c) => palette[c]));
      return { ok, rgba: out };
    },
  };
}
