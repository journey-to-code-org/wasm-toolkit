// The host for a wtk game: it loads the .wasm, shows its 320x240 screen scaled up in whole pixels,
// feeds it the keyboard and the pointer thirty times a second, and is its PC speaker. For an engine
// that says what its car is doing (wtk_sound), it also makes the car's sound; M mutes it all.
import { voice } from "./voice.js";

const W = 320, H = 240, HZ = 30;
const KEYS = {
  ArrowUp: 1, KeyW: 1, ArrowDown: 2, KeyS: 2, ArrowLeft: 4, KeyA: 4, ArrowRight: 8, KeyD: 8,
  Enter: 16, Space: 16 | 1024, Escape: 32, Backspace: 32, KeyE: 64, KeyQ: 128, KeyN: 256, ShiftLeft: 256, ShiftRight: 256, KeyH: 512, Slash: 512,
};

// `pack` is a content pack, for an engine that plays one. The game and the pack are each an address
// to fetch or the bytes themselves.
// `net` is where other players are met, for an engine that races them: { url, name }, the address
// of a room's socket and what this player goes by. Without it the engine is told nobody can be reached.
export async function boot(canvas, url, seed = (Date.now() & 0x7fffffff) || 1, pack = null, net = null) {
  const bytesOf = async (from) => (typeof from === "string" ? new Uint8Array(await (await fetch(from)).arrayBuffer()) : from);
  let audio = null, osc = null, gain = null, quietAt = 0, master = null, car = null;
  let muted = false;
  try { muted = localStorage.getItem("wtk:muted") === "1"; } catch { /* no storage */ }
  const speaker = (hz, ms) => {
    if (!audio) return;
    if (!osc) {
      osc = audio.createOscillator();
      osc.type = "square";
      gain = audio.createGain();
      gain.gain.value = 0;
      osc.connect(gain).connect(master);
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
    if (!audio) {
      audio = new (window.AudioContext || window.webkitAudioContext)();
      master = audio.createGain();
      master.gain.value = muted ? 0 : 1;
      master.connect(audio.destination);
      // The engine's sound is made here, so it need not beep it.
      if (x?.wtk_sound) { car = voice(audio, master); x.wtk_voice?.(); }
    }
    if (audio.state === "suspended") void audio.resume();
  };
  const mute = () => {
    muted = !muted;
    try { localStorage.setItem("wtk:muted", muted ? "1" : "0"); } catch { /* no storage */ }
    if (master) master.gain.setTargetAtTime(muted ? 0 : 1, audio.currentTime, 0.02);
  };

  // A saved game lives in the browser, under the page's address.
  const slot = "wtk:" + location.pathname + (typeof pack === "string" ? ":" + pack : "");
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
  // The line to the other players. The engine asks for it and shuts it; what the room says is
  // handed to the engine a message at a time. The engine is never called back from inside one of
  // its own calls: answers wait for the call to return.
  let line = null;
  const later = (does) => setTimeout(does, 0);
  const netClose = () => { const was = line; line = null; if (was) { try { was.close(); } catch { /* already gone */ } } };
  const netOpen = () => {
    netClose();
    if (!net?.url) return void later(() => x.net_status(3));
    let sock;
    try { sock = new WebSocket(net.url); } catch { return void later(() => x.net_status(3)); }
    line = sock;
    sock.onopen = () => { if (line !== sock) return; sock.send(JSON.stringify({ t: "hello", name: net.name ?? "" })); x.net_status(2); };
    sock.onmessage = (e) => {
      if (line !== sock || typeof e.data !== "string") return;
      const bytes = new TextEncoder().encode(e.data);
      if (bytes.length > 4096) return;
      const at = x.net_alloc(bytes.length); // first, since making room can move the memory
      new Uint8Array(x.memory.buffer, at, bytes.length).set(bytes);
      x.net_recv();
    };
    sock.onclose = sock.onerror = () => { if (line === sock) { line = null; x.net_status(3); } };
  };
  const netSend = (ptr, n) => { if (line?.readyState === 1) line.send(new TextDecoder().decode(new Uint8Array(x.memory.buffer, ptr, n))); };
  const env = { wtk_tone: speaker, wtk_store: store, wtk_fetch: fetchSave, wtk_net_open: netOpen, wtk_net_close: netClose, wtk_net_send: netSend };
  const { instance } = await WebAssembly.instantiate(await bytesOf(url), { env });
  x = instance.exports;
  canvas.width = W;
  canvas.height = H;
  const ctx = canvas.getContext("2d");
  const image = ctx.createImageData(W, H);
  const out = new Uint32Array(image.data.buffer);

  // `latch` holds anything pressed since the last tick, so a tap shorter than a tick is not lost.
  let keys = 0, latch = 0, mx = W / 2, my = H / 2;
  const key = (down) => (e) => {
    if (e.code === "KeyM" && down && !e.repeat) { wake(); return mute(); }
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
    const bytes = await bytesOf(pack);
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
    if (stepped && car) car(new Float32Array(x.memory.buffer, x.wtk_sound(), 14));
    if (stepped) show();
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
  return x;
}
