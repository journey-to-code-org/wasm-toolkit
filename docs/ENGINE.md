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
- **Gas**: every car has a tank. Cruising burns it, a pass down the strip burns half a gallon,
  and a dollar buys three gallons wherever the pack puts a pump. Run dry in town and you walk.
- **Saving**: the game is kept in the browser as you go, and the title offers to carry on.

Everything the C game did is now in the engine, except its 3D car models: other cars in town
are plain boxes, and a car ahead on the strip is a picture of its back.

## The workshop

`make engine && make serve`, then open http://localhost:8080/workshop.html. It opens the default
pack, or any `.pack` you give it, and edits it in the browser. The game beside the forms is the
engine itself, drawing the pack as it stands; if the engine would refuse the pack, it says why
there. **Save the pack** downloads it, **Play it** opens the game on it in another tab, and work
in progress is kept in the browser between visits.

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

**Maps**
- A map is painted from above: pick a kind of block and click or drag over the grid. Set how
  many blocks across and down it is.
- Each kind of block has a colour and a height, or is open ground, and can lead to a room, with
  the name it goes by on the list in the windscreen.
- The game beside it shows the map from the seat, beside the first place there is to go.
- A room gets a way onto a map from the Rooms tab: a thing to pick that goes "out for a drive".

**Parts**: name, slot, price, what the ad says, how much torque it adds or the turbo it is,
which cars it fits, and a picture of how it looks bolted on under the hood.

**Rivals**: in order, each with a car, a paint, a quarter-mile time, the most they will bet and
whether they race for cars. The last can be the one to beat.

**The game**: the pack's name, the money and car you start with, where you start, how long the
season is, and what can be bet.

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
  | `gas` | fill the tank, a dollar for three gallons |

  A spot with `"button": "Race"` is drawn as a button with that word on it, for a place whose
  picture does not show the thing itself.
- **places** may also have `rival_at`, like `car_at`: where whoever you are up against stands.

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
  is what the part the car came with is called. A place need not match any part in the paper:
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
  `"picture": "cone.pic"` is what the part looks like bolted on: it is drawn over its place
  under the hood, stretched to the box, in place of the stock part in the bay's picture. Without
  one, a fitted part is marked with a tag.
  At most 14 parts.
- **rivals**: in the order they turn up. `et` is their quarter mile in seconds. `limit` is the
  most money they will race for, `pinks` whether they will race for cars. The one with `king`
  must be last, races for cars only, and beating them wins the game.
- **bets** and **paints** may be left out.
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
place. `drive:<map>` from a place starts you beside the block that leads to it.

A pack that names a picture it does not hold, a place that is not there, or a ramp that does not
exist is refused, and the screen says which.

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
