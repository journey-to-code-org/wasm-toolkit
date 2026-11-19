// Two engines and a room between them, wired as the page wires one engine to the parlor: each
// buys a car, drives to the lot and opens the meet; one dares the other for ten dollars; they
// stage, race, and come back. The room here does what the parlor's does: it says who is in it,
// puts who a message is from on it, and passes it on.
import { readFileSync } from "node:fs";

const [wasm, packPath] = process.argv.slice(2);
const W = 320, H = 240, UP = 1, A = 16, B = 32, E = 64;
let fails = 0;
const check = (ok, what) => { if (!ok) { console.log("FAIL", what); fails++; } };

const module = await WebAssembly.compile(readFileSync(wasm));
const packBytes = readFileSync(packPath);
const players = [], waiting = [], sent = { pos: 0, slip: 0, dare: 0, ok: 0, staged: 0 };

async function player(name) {
  const me = { name, id: players.length + 1, open: false };
  let x;
  const say = (to, message) => waiting.push([to, JSON.stringify(message)]);
  const env = {
    wtk_tone: () => {}, wtk_store: () => {}, wtk_fetch: () => 0,
    wtk_net_open: () => {
      me.open = true;
      waiting.push([me, null]);
      say(me, { t: "you", id: me.id, members: players.filter((p) => p.open).map((p) => ({ id: p.id, name: p.name })) });
      for (const p of players) if (p !== me && p.open) say(p, { t: "join", id: me.id, name: me.name });
    },
    wtk_net_close: () => { me.open = false; for (const p of players) if (p !== me && p.open) say(p, { t: "left", id: me.id }); },
    wtk_net_send: (ptr, n) => {
      const message = JSON.parse(new TextDecoder().decode(new Uint8Array(x.memory.buffer, ptr, n)));
      if (message.t in sent) sent[message.t]++;
      for (const p of players) if (p !== me && p.open && (message.to === undefined || message.to === p.id)) say(p, { ...message, from: me.id });
    },
  };
  x = (await WebAssembly.instantiate(module, { env })).exports;
  const room = x.pack_alloc(packBytes.length);
  new Uint8Array(x.memory.buffer, room, packBytes.length).set(packBytes);
  check(x.pack_load() === 1, `${name}'s pack loads`);
  x.wtk_init(players.length + 1);
  Object.assign(me, {
    x,
    screen: () => new Uint16Array(x.memory.buffer, x.wtk_screen(), W * H).slice(),
    at: (px, py) => new Uint16Array(x.memory.buffer, x.wtk_screen(), W * H)[py * W + px],
    step: (keys, mx = 300, my = 150) => { x.wtk_input(keys, mx, my); x.wtk_step(); },
  });
  players.push(me);
  return me;
}

// What the room has to say is handed over between steps, never in the middle of one.
function deliver() {
  for (let n = 0; waiting.length && n < 1000; n++) {
    const [to, text] = waiting.shift();
    if (text === null) { to.x.net_status(2); continue; }
    const bytes = new TextEncoder().encode(text);
    const at = to.x.net_alloc(bytes.length); // first, since making room can move the memory
    new Uint8Array(to.x.memory.buffer, at, bytes.length).set(bytes);
    to.x.net_recv();
  }
}
const run = (p, n, keys, mx, my) => { for (let i = 0; i < n; i++) { p.step(keys, mx, my); deliver(); } };
const click = (p, mx, my) => { run(p, 1, 0, mx, my); run(p, 1, A, mx, my); run(p, 1, 0, mx, my); };
const press = (p, key) => { run(p, 1, key); run(p, 1, 0); };
const differ = (a, b) => a.reduce((n, v, i) => n + (v !== b[i]), 0);

const ay = await player("Ay"), bee = await player("Bee");
for (const p of [ay, bee]) {
  press(p, A);          // past the title
  click(p, 50, 108);    // the paper
  click(p, 100, 70);    // the first car in it
  press(p, B);
  click(p, 150, 130);   // the car's door: to the lot
  click(p, 185, 186);   // the meet
}
run(ay, 2, 0); run(bee, 2, 0);
check(ay.x.wtk_mode() === 1 && bee.x.wtk_mode() === 1, "both are in the meet");
const meet = ay.screen();
click(ay, 100, 44);               // Bee, the only one here
click(ay, 4 + 62 + 30, 185);      // for ten dollars
run(bee, 2, 0);
check(sent.dare === 1, "the dare went out");
press(bee, A);                    // taken
run(ay, 2, 0);
check(sent.ok === 1 && ay.x.wtk_mode() === 2 && bee.x.wtk_mode() === 2, "and both are on the strip");
check(differ(meet, ay.screen()) > 20000, "seen from the seat");
press(ay, A); press(bee, A);      // both stage
run(ay, 1, 0);
check(sent.staged === 2, "both said they were staged");

// Both on the limiter; Ay leaves on the green and Bee half a second later. Then each shifts by its own light.
const LIT = 2 * 32 + 30, slip = (p) => [[96, 60], [96, 100], [224, 100]].every(([px, py]) => (p.at(px, py) >> 5) === 25);
for (let t = 0; t < 46; t++) { ay.step(UP); bee.step(UP); deliver(); }
ay.step(UP | E); ay.step(UP);
// The lane marker at the top: each steers to keep it in the middle.
const MARK = [5 * 32 + 26, 2 * 32 + 28];
const wheel = (p) => {
  let at = -1;
  for (let px = 120; px < 203 && at < 0; px++) if (MARK.includes(p.at(px, 23))) at = px + 2;
  return at < 0 ? 0 : at > 161 ? 4 : at < 159 ? 8 : 0;
};
let ticks = 0;
for (; ticks < 1500 && !(slip(ay) && slip(bee)); ticks++) {
  for (const p of [ay, bee]) {
    const go = p === bee && ticks === 15, lit = p.at(121, 170) === LIT;
    // Bee sits with a foot off it first, which is what makes an automatic late as well.
    if (p === bee && ticks < 15) { p.step(0); continue; }
    p.step((go || (lit && !slip(p)) ? UP | E : UP) | wheel(p));
    if (go || lit) p.step(UP | wheel(p));
  }
  deliver();
}
check(ticks > 300 && ticks < 1500, `both come down the strip (${ticks} ticks)`);
run(ay, 3, 0); run(bee, 3, 0);
check(sent.pos > 40 && sent.slip === 2, `each told the other where it was (${sent.pos} times) and sent one timeslip`);
const slips = [ay.screen(), bee.screen()];
check(differ(slips[0], slips[1]) > 20, "the two timeslips read differently");
press(ay, A); press(bee, A);
check(ay.x.wtk_mode() === 1 && differ(meet, ay.screen()) < 8000, "and afterwards both are back in the meet");
press(ay, B);
run(bee, 2, 0);
check(differ(meet, bee.screen()) > 200, "when one leaves, the other's list says so");
console.log(fails ? `${fails} FAILED` : "online test passes");
process.exit(fails ? 1 : 0);
