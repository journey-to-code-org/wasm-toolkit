// The host for a wtk game: it loads the .wasm, shows its 320x240 screen scaled up in whole pixels,
// feeds it the keyboard and the pointer thirty times a second, and is its PC speaker.
const W = 320, H = 240, HZ = 30;
const KEYS = {
  ArrowUp: 1, KeyW: 1, ArrowDown: 2, KeyS: 2, ArrowLeft: 4, KeyA: 4, ArrowRight: 8, KeyD: 8,
  Enter: 16, Space: 16, Escape: 32, Backspace: 32, KeyE: 64, KeyQ: 128,
};

// `pack` is the address of a content pack, for an engine that plays one.
export async function boot(canvas, url, seed = (Date.now() & 0x7fffffff) || 1, pack = null) {
  let audio = null, osc = null, gain = null, quietAt = 0;
  const speaker = (hz, ms) => {
    if (!audio) return;
    if (!osc) {
      osc = audio.createOscillator();
      osc.type = "square";
      gain = audio.createGain();
      gain.gain.value = 0;
      osc.connect(gain).connect(audio.destination);
      osc.start();
    }
    const now = audio.currentTime;
    if (hz > 0) {
      osc.frequency.setValueAtTime(hz, now);
      gain.gain.setValueAtTime(0.04, now);
      quietAt = now + ms / 1000;
      gain.gain.setValueAtTime(0, quietAt);
    } else gain.gain.setValueAtTime(0, now);
  };
  const wake = () => {
    audio ??= new (window.AudioContext || window.webkitAudioContext)();
    if (audio.state === "suspended") void audio.resume();
  };

  // A saved game lives in the browser, under the page's address.
  const slot = "wtk:" + location.pathname;
  let x = null;
  const store = (ptr, n) => {
    try { localStorage.setItem(slot, btoa(String.fromCharCode(...new Uint8Array(x.memory.buffer, ptr, n)))); } catch { /* private window, or full */ }
  };
  const fetchSave = (ptr, max) => {
    try {
      const raw = atob(localStorage.getItem(slot) ?? "");
      const n = Math.min(raw.length, max);
      const out = new Uint8Array(x.memory.buffer, ptr, n);
      for (let i = 0; i < n; i++) out[i] = raw.charCodeAt(i);
      return n;
    } catch { return 0; }
  };
  const { instance } = await WebAssembly.instantiate(await (await fetch(url)).arrayBuffer(), { env: { wtk_tone: speaker, wtk_store: store, wtk_fetch: fetchSave } });
  x = instance.exports;
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d");
  const image = ctx.createImageData(W, H);
  const out = new Uint32Array(image.data.buffer);

  // `latch` holds anything pressed since the last tick, so a tap shorter than a tick is not lost.
  let keys = 0, latch = 0, mx = W / 2, my = H / 2;
  const key = (down) => (e) => {
    const bit = KEYS[e.code];
    if (!bit) return;
    e.preventDefault();
    wake();
    keys = down ? keys | bit : keys & ~bit;
    if (down) latch |= bit;
  };
  window.addEventListener("keydown", key(true));
  window.addEventListener("keyup", key(false));
  window.addEventListener("blur", () => (keys = 0));
  const point = (e) => {
    const r = canvas.getBoundingClientRect();
    mx = Math.floor(((e.clientX - r.left) / r.width) * W);
    my = Math.floor(((e.clientY - r.top) / r.height) * H);
  };
  canvas.addEventListener("pointermove", point);
  canvas.addEventListener("pointerdown", (e) => { wake(); point(e); keys |= 16; latch |= 16; });
  window.addEventListener("pointerup", () => (keys &= ~16));

  const show = () => {
    const mem = x.memory.buffer;
    const screen = new Uint16Array(mem, x.wtk_screen(), W * H);
    const pal = new Uint32Array(mem, x.wtk_palette(), 1024);
    for (let i = 0; i < W * H; i++) out[i] = pal[screen[i]];
    ctx.putImageData(image, 0, 0);
  };

  if (pack && x.pack_alloc) {
    // Hand the engine its content before it starts.
    const bytes = new Uint8Array(await (await fetch(pack)).arrayBuffer());
    const at = x.pack_alloc(bytes.length);
    new Uint8Array(x.memory.buffer, at, bytes.length).set(bytes);
    x.pack_load();
  }
  x.wtk_init(seed);
  show();
  let owed = 0, last = performance.now();
  const frame = (now) => {
    owed = Math.min(owed + (now - last) * HZ / 1000, 4);
    last = now;
    let stepped = false;
    for (; owed >= 1; owed--) { x.wtk_input(keys | latch, mx, my); latch = 0; x.wtk_step(); stepped = true; }
    if (stepped) show();
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
  return x;
}
