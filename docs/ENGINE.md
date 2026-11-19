# The engine and its packs

The C game in `games/shootout` has its cars, rooms and rules written into the code. The engine in
`engine/` is the other way round: a Rust program that knows how to show a room, stand a car in it
and act on what you pick, and nothing about which room or which car. Those come from a **content
pack**, one file, made from a folder of JSON and PNGs.

    make engine          # web/engine.wasm and web/default.pack
    make test-engine     # the Rust tests, then a play-through of the default pack
    make serve           # then open http://localhost:8080/engine.html
                         # another pack: engine.html?pack=mine.pack

`python3 tools/bundle.py engine` then writes `build/engine.html`: the engine, its page and the
default pack in one file, to open from disk or put anywhere a single page can go. On a phone the
page lays buttons over the picture, and only the ones in use: Back in the menus, left, right, gas
and brake in town, the two shifts and gas on the strip. Tapping the picture is Enter.

Needs Rust with the wasm target (`rustup target add wasm32-unknown-unknown`), and Python with
Pillow and numpy for the pack tool.

## What is in the engine so far

All five steps are in the engine. The engine plays a season out of a pack:

- **Places**: a picture or backdrop, your car standing in it, spots to pick, and buttons.
- **The season**: so many days of so many hours. Reading the paper is an hour, bolting a part on
  or off is an hour, a race is two, a pass alone is one. It ends when the days run out, when you
  have no car and no way to buy one, or when you beat the one to beat.
- **The paper**: three cars a day picked from the pack's cars, in differing paint and health,
  and every part in the pack.
- **Under the hood**: a picture of the engine bay with the parts as places on it. A part is
  held by four bolts: undo them, lift it off, pick another up off the floor, set it down, and do
  the bolts up. A car with something off or loose will not race.
- **Engines that break**: held flat against the redline in gear for a second and a half, an
  engine lets go. The race is lost, and it runs badly until it is rebuilt, which costs money and
  four hours.
- **The bench**: the shelf of spare parts, which are sold from here. For a car with no hood in
  the pack, or a slot its hood does not show, parts go on and off here with one pick.
- **The yard**: up to sixteen cars; bring one into the bay or sell it to the dealer.
- **Rivals**: they pull in one at a time, in the pack's order. Race for kicks, for money up to
  what each will take, or for cars. Beating one for money or better brings out the next.
- **The strip**: the quarter mile from the seat, for a car with a cockpit in the pack: the
  cabin is the pack's painting, with live needles, the gear in the gate and the wheel, and the
  road is drawn in depth beyond the glass. A car with no cockpit is raced from the side. The
  car's torque curve, gears, weight, grip and turbo are the pack's; the parts fitted change them.
- **The town**: a map of blocks and streets, driven from the seat. Buildings are what the map
  says they are; other cars are about; a list shows where there is to go and how many blocks off.
  Pull up beside somewhere, press Enter, and you are in it, an hour later.
- **Roads**: out of town there are roads between two places: two lanes that bend and climb,
  through trees, cuttings and along drops, with cars coming the other way and police parked
  beside them. Pass one well over the limit and he comes after you: stop and it is a ticket, or
  keep your foot in it and lose him. A bend taken too fast throws the car wide, and what is beside
  the road decides what that costs. A road takes hours off the day and gas out of the tank.
  Rivals can be raced down one as well as on the strip.
- **Gas**: every car has a tank. Cruising burns it, a pass down the strip burns half a gallon,
  and a dollar buys three gallons wherever the pack puts a pump. Run dry in town and you walk.
- **The meet**: other people, to race. Whoever opens it is in a room with everyone else who
  has; pick someone, dare them for kicks or money, and run it side by side from the seat. Each
  engine runs its own car, so the throttle and shifts are as sharp as alone; the tree comes down
  when both have staged, and the winner is worked out from the two timeslips. It needs a server
  to hold the room: see [The meet](#the-meet).
- **Day and night**: the last third of each day is dusk and then dark, on the road and in town:
  the sky goes, the country beyond the lamps is dim, other cars are their lights, and the dash
  is lit if the cockpit has a night picture.
- **People**: whoever you are up against can stand by their car, and a place can have a crowd.
- **The record**: races won and lost are counted, and the calendar says so.
- **Saving**: the game is kept in the browser as you go, and the title offers to carry on.

Everything the C game did is now in the engine, except its 3D car models: other cars in town
are plain boxes, and a car ahead on the strip is a picture of its back.

## The workshop

`make engine && make serve`, then open http://localhost:8080/workshop.html. It opens the default
pack, or any `.pack` you give it, and edits it in the browser. The game beside the forms is the
engine itself, drawing the pack as it stands; if the engine would refuse the pack, it says why
there. **Save the pack** downloads it, **Play it** opens the game on it in another tab, and work
in progress is kept in the browser between visits.

**Cutting pictures out of a sheet.** Wherever a picture can be dropped there is also "Or cut one
out of a sheet". Open a sheet of sprites, drag a box round the one you want, and it is cut out,
its background taken away, trimmed, and used as if you had dropped it. The background is
whatever runs round the edge of the box, a flat colour or the two greys of a checkerboard, and
it is taken from the edges inwards, so the same colour inside the picture stays. A slider says
how alike a colour has to be to count; at 0 nothing is taken away. The sheet stays open for the
next cut.

**Cars**
- Drop in the car's picture from the side, nose to the right. Any PNG will do; one wider than
  the screen is scaled down, and you choose how wide it stands in the game.
- A PNG with a clear background is best. For one saved on a plain colour, tick the box and the
  corner's colour is made clear.
- The red line is where the tyres touch. It starts under the lowest pixel; drag it up if the
  picture has a shadow under the car.
- Click the bodywork in the picture and everything of about that colour becomes the paint; the
  car can then be shown in any of the 31 colours. Bright highlights that are nearly white are
  left as they are.
- A second picture, the same size, is the car with its headlamps on.
- Its price in the paper, and under "What makes it go" its torque curve, gears, weight, grip
  and turbo.
- From the seat: drop in the cabin painting, its night version, the bonnet, the steering wheel
  and the car from behind, and say where the dials, shift light and gear gate are. While that
  section is open, the game beside it is seen from the seat.
- Under its hood: drop in a photo of the engine bay as the car came. Add a place for each part
  that comes off: one for each slot the pack's parts name, and any of your own. For each, set
  how many bolts hold it (1 to 8) and what the stock part is called, drag its box to where the
  part is, and drag each bolt to where it really is.

**Rooms**
- A room is a picture (320 by 200) or a plain colour. Tick whether your car stands in it and
  whether rivals pull in, and drag the two crosses to where their tyres meet the floor.
- The things to pick are boxes on the picture: add one, drag it, size it by its corner, and say
  what it is called and what picking it does. A box can be drawn as a button.
- People standing about: drop or cut a picture, say how tall it is made, and place it by its
  left edge and its feet.

**Maps**
- A map is painted from above: pick a kind of block and click or drag over the grid. Set how
  many blocks across and down it is.
- Each kind of block has a colour and a height, or is open ground, and can lead to a room, with
  the name it goes by on the list in the windscreen.
- The game beside it shows the map from the seat, beside the first place there is to go.
- A room gets a way onto a map from the Rooms tab: a thing to pick that goes "out for a drive".
- A kind of block can be the way onto a road in place of leading to a room.

**Roads**
- A road has a name, two ends (a room, or the streets of a map), a speed limit, what a ticket
  costs, how many other cars and police are on it, and how many hours the trip takes.
- It is built as a list of stretches: a length, a bend, a climb and what is on each side. The
  road is drawn from above as you type, with how high it runs under it, and the engine shows it
  from the seat at the start of whichever stretch you pick with "Look from here".
- Pointing an end at a map makes a kind of block for it there, to be put down under Maps. At a
  room, add something to pick that "Takes a road out of town".

**Parts**: name, slot, price, what the ad says, how much torque it adds or the turbo it is,
which cars it fits, and a picture of how it looks bolted on under the hood. A part can also have a picture of how it
looks on the car from the side.

**Rivals**: in order, each with a car, a picture of them if you have one, a paint, a quarter-mile time, the most they will bet and
whether they race for cars. The last can be the one to beat.

**The game**: the pack's name, the money and car you start with, where you start, how long the
season is, and what can be bet.

**Kept somewhere with players.** Whoever serves the workshop can put a `host.js` beside it and
open it as `workshop.html?host`. The host's default export says where the game's files are and
may take a finished pack:

```js
export default {
  base: "/wasm/shootout/",              // where engine.wasm, default.pack and engine.html are
  publishLabel: "Publish to the parlor",
  async publish(bytes) { /* send the pack; throw new Error("why not") if it cannot be done */ },
};
```

With a host, the workshop starts from the pack being played now and shows a Publish button.

The workshop has no tests of its own in CI; `web/pack.js`, which reads and writes the packs, has.

## A pack folder

    content/default/
      pack.json
      garage.png
      car.png
      car-lamps.png

`tools/pack.py content/default web/default.pack` matches each PNG to the 1,024 colours, stores it
run-length as `<name>.pic`, and puts everything in one file. PNGs may be any size up to 320x240;
transparent pixels stay clear.

## Photos

`tools/pixelate.py` turns a photo, or any picture, into a PNG that the engine draws exactly as
the PNG looks: cut to shape, brought down to size and matched to the 1,024 colours.

    tools/pixelate.py bay.jpg --as bay                  # 320 x 176, for under a hood
    tools/pixelate.py garage.jpg --as room --pop 1.2    # 320 x 200, with more punch
    tools/pixelate.py filter.jpg --size 42x52           # a part, the size of its place
    tools/pixelate.py car.png --as car --width 244      # a car; its background must be clear

`--as` is one of bay, room, cockpit, car, rear, wheel. `--fit cover` (the default) fills the
shape and cuts what hangs over, keeping the `--focus` part; `contain` keeps all of it. `--dither`
sets how much the colours are stippled, `--sharpen` crispens a shrunken photo, and `--big 3`
saves a blown-up copy to look at. A picture it has made goes into a pack untouched, by
`tools/pack.py` or by the workshop. The workshop does the same conversion itself for anything
dropped into it, so the script is for doing many, or for seeing the result first.

## pack.json

```json
{
  "name": "Road to the Shootout",
  "start": { "cash": 750, "place": "garage", "car": "umbra" },
  "cars": {
    "umbra": {
      "name": "1990 Kagami Umbra Turbo",
      "side": "car.pic",
      "side_lamps": "car-lamps.pic",
      "ground": 74,
      "paint": "BLUE",
      "paint_ramps": { "mid": "BLUE", "dark": "NAVY", "light": "SKY", "glint": "CYAN" }
    }
  },
  "places": {
    "garage": {
      "title": "Your garage",
      "picture": "garage.pic",
      "car_at": [40, 182],
      "spots": [
        { "rect": [232, 14, 84, 108], "label": "Hit the street", "do": "go:lot" },
        { "rect": [256, 134, 28, 26], "label": "The headlamps", "do": "lamps", "needs_car": true }
      ]
    }
  }
}
```

- **start**: the cash, place and car a new game begins with. `car` may be left out, and then
  the first thing to do is buy one.
- **cars**: `side` is the car from the side. `side_lamps` is the same picture with the lamps on,
  if there is one. `ground` is the row of the picture the tyres stand on. `paint_ramps` names the
  colour ramps the picture was painted in, and `paint` the ramp the body is shown in, so one
  picture serves every colour.
- **places**: `picture` fills the top 200 rows; without one, `backdrop` names a ramp to fade.
  `car_at` is where the car stands: x of the picture's left edge, y of the floor under its tyres.
- **spots**: `rect` is x, y, width, height. `needs_car` hides the spot when you have no car.
  An empty `do` only shows the label. Otherwise `do` is one of:

  | do | what happens |
  |---|---|
  | `go:<place>` | walk to that place |
  | `say:<words>` | show the words in the panel |
  | `lamps` | work the car's headlamps |

  | `calendar` | say what day it is and how many hours are left |
  | `paper` | open the paper |
  | `parts` | open the bench |
  | `hood` | open the hood of the car in the bay |
  | `yard` | open the yard |
  | `race` | race whoever has pulled in (the place needs `rival_at`) |
  | `next` | wave them on and see who else is about |
  | `solo` | a pass down the strip alone |
  | `drive:<map>` | out onto that map, beside the block that leads back here |
  | `road:<road>` | onto that road, from whichever of its ends this place is |
  | `roadrace:<road>` | race whoever has pulled in down that road, for a bet |
  | `gas` | fill the tank, a dollar for three gallons |
  | `online` | the meet: race other people |
  | `rebuild` | have the engine of the car in the bay rebuilt |
  | `paint` | a respray: pick one of the pack's paints |

  A spot with `"button": "Race"` is drawn as a button with that word on it, for a place whose
  picture does not show the thing itself.
- **places** may also have `rival_at`, like `car_at`: where whoever you are up against stands.
  `"people": [ { "picture": "crowd.pic", "at": [140, 182] } ]` stands pictures in the place,
  behind the cars: `at` is the left edge and where the feet are. Twelve at most.

Ramp names: see `RAMPS` in `engine/src/gfx.rs`.

### The season, cars that go, parts and rivals

```json
{
  "season": { "days": 90, "hours_per_day": 12 },
  "bets": [10, 25, 50, 100],
  "paints": ["BLUE", "RED", "GOLD"],
  "cars": {
    "umbra-base": {
      "name": "1990 Kagami Umbra", "side": "car.pic", "ground": 74,
      "price": 420,
      "drive": {
        "torque": [[800, 70], [2000, 100], [4000, 125], [6800, 95]],
        "gears": [3.36, 1.95, 1.29, 0.94, 0.76], "final": 4.3,
        "redline": 6800, "launch": 4500, "weight_lb": 2700, "grip": 9,
        "boost": null
      }
    }
  },
  "parts": {
    "header": { "name": "Tubular header", "slot": "Exhaust", "price": 420, "pitch": "Lets it all out", "torque": 6 },
    "big-turbo": { "name": "Big turbo", "slot": "Turbo", "price": 900, "pitch": "27 psi. Hits late",
                   "boost": { "from": 3000, "psi": 27, "lag": 10 } }
  },
  "rivals": [
    { "name": "Skeeter", "says": "My mama's car. Don't scratch it.", "car": "umbra-base", "paint": "TAN", "et": 19.4, "limit": 25 },
    { "name": "The King", "says": "Pink slips. Nothing else.", "car": "umbra", "paint": "GOLD", "et": 11.9, "king": true }
  ]
}
```

- **cars**: `price` is what one in fair shape costs; a car with no price never comes up in the
  paper. `drive` is what makes it go: `torque` is pound-feet at the wheels at each engine speed,
  at full boost if it has a turbo; `boost` is the turbo (`from` the rpm it wakes at, `psi`, and
  `lag`, bigger being lazier) or `null`. A car with no `drive` gets a two-litre turbo four.
  Two cars may share a picture.
- **cars** may have a `hood`: the engine bay, up to 320x176, as the car came, and the places on
  it where parts sit.

  ```json
  "hood": { "picture": "bay.pic", "slots": {
    "Turbo": [138, 128, 28, 30],
    "Intake": { "rect": [56, 98, 42, 52], "bolts": 2, "stock": "Stock airbox" },
    "Battery": { "rect": [250, 40, 40, 40], "bolts": [[240, 30], [284, 45], [270, 74]] }
  } }
  ```

  A place is its box (x, y, width, height), which has a bolt at each corner; or the box with a
  number of `bolts`, one to eight, laid out for you; or the box with where each bolt is. `stock`
  is what the part the car came with is called. `"behind": "Air filter"` says the part there
  cannot be lifted until the air filter is off, and the air filter cannot go back until it is on. A place need not match any part in the paper:
  it is then just a stock part that comes off and goes back on, and the car will not run
  without it. Split things as finely as you like: an air filter and an intake pipe can be two
  places, each with its own parts. The floor under the hood shows up to eight loose parts.
- **cars** may have a `cockpit`, the view from the seat, and a `rear`, the car from behind for
  when it is ahead of you:

  ```json
  "cockpit": {
    "picture": "cockpit.pic", "night": "cockpit-night.pic", "bonnet": "bonnet.pic", "view_h": 136,
    "tach": { "at": [95, 170], "r": 17, "max": 9000 },
    "speedo": { "at": [145, 170], "r": 17, "max": 160 },
    "boost": { "at": [27, 96], "r": 9, "max": 30 },
    "minis": [{ "at": [120, 151], "r": 4, "value": 46 }],
    "shift_light": [120, 170], "gear_gate": [286, 224],
    "wheel": { "picture": "wheel.pic", "night": "wheel-night.pic", "at": [120, 216] }
  },
  "rear": { "picture": "rear.pic", "width_m": 1.7 }
  ```

  `picture` is the cabin, 320 by 240, with the glass left clear; `view_h` is how far down the
  view out goes. `bonnet` is drawn under it in the car's paint, using the car's `paint_ramps`.
  Each dial is where its needle turns, the needle's length and what it reads at full sweep;
  `minis` are dials that only sit there. The wheel is a square picture turned about `at`.
  Everything but `picture` may be left out.
- **parts**: one part to a `slot`; the slots are whatever the parts name. `torque` is a
  percentage more. A part with `boost` replaces the car's turbo. `"fits": ["umbra"]` keeps a part to those cars.
  Beyond torque a part can have `grip` (per cent more, for tyres), `weight_lb` (what it adds;
  less than nothing saves weight), `gears` and `final` (a gearbox in place of the car's own) and
  `redline` (a built engine's). `"curve": [[1000, 120], [4000, 300], [7000, 240]]` is a different
  engine altogether: its torque curve, in place of the car's. `"radar": true` makes it a radar detector for the road.
  `"side": "wing.pic"` is what it looks like on the car from the side: a picture the size of
  the car's own side picture, drawn over it in the garage, the yard and on the strip. Its clear
  pixels leave the car showing, so it can be only the wing, or the whole car again with the wing on.
  `"picture": "cone.pic"` is what the part looks like bolted on: it is drawn over its place
  under the hood, stretched to the box, in place of the stock part in the bay's picture. Without
  one, a fitted part is marked with a tag.
  At most 14 parts.
- **rivals**: in the order they turn up. `et` is their quarter mile in seconds. `limit` is the
  most money they will race for, `pinks` whether they will race for cars. The one with `king`
  must be last, races for cars only, and beating them wins the game.
  `"figure": "skeeter.pic"` is a picture of them, which stands by their car.
- `"title": "title.pic"` is a picture for the title screen, 320 by 240. `"respray": { "cost": 60,
  "hours": 3 }` is what the `paint` action charges.
- **bets** and **paints** may be left out. So may `"rebuild": { "cost": 250, "cond": 85 }`:
  what an engine rebuild costs and how healthy it comes back.
- **cars** may have `"tank": 17`, in gallons. A small dial in the cockpit with `"reads": "gas"`
  shows what is in it.

### Maps

```json
"maps": {
  "town": {
    "title": "Town",
    "rows": ["BOT.CB",
             "G.BPOT",
             "HCOB.S"],
    "blocks": {
      "B": { "wall": "BRICK", "height": 16 },
      "H": { "wall": "WOOD", "height": 6, "to": "garage", "name": "Home" },
      "S": { "to": "lot", "name": "The strip" },
      "P": { "wall": "RED", "height": 5, "to": "station", "name": "Gas" }
    },
    "pitch": 64, "road": 8, "traffic": 8, "gas_per_km": 0.4
  }
}
```

Each letter of a row is a block, the top row being the north side; a full stop is grass. There
is a street between every two blocks and one round the outside, and nothing beyond. A block has
a `wall` colour and a `height` in metres, or neither for open ground. A block with `to` is
somewhere to go: it gets an awning (or a gold lot, if nothing is built on it), it is on the list
in the windscreen under `name`, and pulling up beside it and pressing Enter puts you in that
place. `drive:<map>` from a place starts you beside the block that leads to it. A block with
`road` in place of `to` is the way onto that road.

### Roads

```json
"roads": {
  "ridge": {
    "title": "Ridge road", "ends": ["map:town", "overlook"],
    "limit": 55, "ticket": 40, "traffic": 7, "cops": 2, "hours": 1, "gas_per_km": 0.25,
    "sky": "SKY", "ground": "GRASS",
    "parts": [
      { "len": 240, "left": "fields", "right": "fields" },
      { "len": 320, "curve": 0.7, "hill": 18, "left": "rock", "right": "drop" }
    ]
  }
}
```

A road runs between two `ends`. An end is a place, or `map:<map>` for the streets of a map, where
a block marked `"road": "<this road>"` is the way onto it and where you come out. From a place, a
spot that does `road:<road>` takes it. It is the same road both ways: driven from its second end,
the bends and climbs are turned about.

It is made of stretches (`parts`), first end first. Each has a `len` in metres (12 or more; the
whole road between 200 metres and 30 kilometres), one `curve` from -1 (hard left) to 1 (hard
right) that eases in and out, one `hill` in metres climbed (less than nothing falls; 80 at most),
and what is on its `left` and `right`:

| Beside the road | What it is | Leaving the road there |
| --- | --- | --- |
| `trees` (if left out) | woods | a crash |
| `rock` | a cutting | a crash |
| `drop` | a rail, and the valley | a crash |
| `fields` | open ground and a fence | only slow |
| `posts` | open ground and marker posts | only slow |

The rules of the road:

- You drive in the right-hand lane. `traffic` other cars are on it, half of them coming the
  other way in the left. Hitting one, or the trees, stops you dead for a second and takes three
  points off the car's health.
- At speed a bend throws the car to its outside, by the square of the speed: a bend of 1 cannot be
  held flat out.
- `cops` police are parked along it. Pass one at more than 10 mph over the `limit` and he comes
  after you. Stop, and the ticket is `ticket` dollars and two more for every mile an hour over
  you were seen at. Stay 25 over the limit for twenty seconds and you have lost him. Do neither
  for forty-five seconds, or crash with him behind you, and it is twice the ticket. He takes
  what you have if you do not have it all.
- A part with `"radar": true` fitted to the car is a detector: it beeps, and the screen says
  so, when police are within 220 metres ahead.
- Reaching the far end takes `hours` off the day. Escape turns round and puts you back where you
  set out, unless you are being chased. A town you came to by road has no stepping back out of it:
  pull up somewhere.
- It burns `gas_per_km` gallons a kilometre. Run dry and you walk home, as in town.
- The car is the one in the bay: what its engine makes for what it weighs, with its parts and
  its health, is how hard it pulls and how fast it will go, and its tyres are how much a bend
  throws it. The default car with nothing done to it pulls 5 metres a second each second from
  rest and runs out at 125 mph.

**Racing down a road.** A spot that does `roadrace:<road>`, in a place where rivals pull in, is
a race against whoever is there, bet on like a race on the strip: kicks, money, or cars. It is
counted down from three, side by side; there are no police, the other cars are still about, and
the first to the end wins. Escape, or running dry, loses it. How quick a rival is down a road
comes from their `et`: one who runs the quarter in 12 is some way quicker than one who runs it
in 17, and all of them lift for the bends. It takes two hours, as a race on the strip does, and
you are back where you set out. Winning for money counts as beating them.

A pack that names a picture it does not hold, a place that is not there, or a ramp that does not
exist is refused, and the screen says which.

## The meet

The engine opens no sockets itself. The page does (`web/wtk.js`), to the address it is given:
by default `../meet/shootout` beside the page, which is where the parlor keeps its room, or
whatever `engine.html?rooms=` names. `?name=` is what the player goes by. Opened from a file,
or with no server there, the meet says nobody can be reached and the rest of the game goes on.

The room is simple enough to keep anywhere. Everything is a line of JSON:

| Who says it | What | Meaning |
|---|---|---|
| A page, first | `{"t":"hello","name":"Mikey"}` | Let me in |
| The room | `{"t":"you","id":3,"members":[{"id":1,"name":"Ay"},...]}` | You are in, and who else is |
| The room | `{"t":"join","id":4,"name":"Bee"}` and `{"t":"left","id":4}` | Someone came or went |
| A page | anything else | Passed on with `"from": <id>` added, to the member `"to"` names, or to everyone else |

What the engines say to each other through it: `here` (the car and paint they are in), `dare`
(with a `stake`), `ok`, `no`, `never`, `staged`, `pos` (where they are on the strip, six times a
second), `slip` (their timeslip) and `quit`. Nothing a page receives is trusted further than it
must be: names are cut short, stakes must be ones the pack offers, and a car the pack does not
have is drawn as a plain shape. The timeslip is taken at its word, so this is for racing friends:
nothing stops a changed page from sending a time it did not run. Only kicks and money are raced
for online, not cars.

## The file

`WPK1`, a u32 count, then for each file: u16 name length, the name, u32 length, the bytes. All
little-endian. A `.pic` is u16 width, u16 height, then (count, colour) pairs; colour `0xFFFF` is
clear.

## What comes next

1. ~~The pack and its loader, with the garage and the car coming out of it.~~
2. ~~The workshop: a page where you drop in a car's picture, set its ground line and paint, see it
   in the garage, and save the pack.~~
3. ~~Parts, rivals and the paper as data, and the season's rules in the engine.~~
4. ~~Under the hood as a place where parts come off bolt by bolt. Rooms with their spots
   drawn and edited in the workshop, and parts and rivals edited there too.~~
5. ~~The view from the seat, the town as a map, gas, and both edited in the workshop.~~
6. ~~Racing other people on the strip.~~
7. ~~Parts that change more than torque, engines that break, and an order to the work under the hood.~~
8. ~~Roads out of town: bends, hills, traffic and police.~~
9. ~~Racing rivals down a road, in a car that is as quick there as its parts make it.~~
