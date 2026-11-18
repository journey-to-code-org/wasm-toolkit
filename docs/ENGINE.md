# The engine and its packs

The C game in `games/shootout` has its cars, rooms and rules written into the code. The engine in
`engine/` is the other way round: a Rust program that knows how to show a room, stand a car in it
and act on what you pick, and nothing about which room or which car. Those come from a **content
pack**, one file, made from a folder of JSON and PNGs.

    make engine          # web/engine.wasm and web/default.pack
    make test-engine     # the Rust tests, then a play-through of the default pack
    make serve           # then open http://localhost:8080/engine.html
                         # another pack: engine.html?pack=mine.pack

Needs Rust with the wasm target (`rustup target add wasm32-unknown-unknown`), and Python with
Pillow and numpy for the pack tool.

## What is in the engine so far

Steps 1 to 3 of 5 are done. The engine plays a season out of a pack:

- **Places**: a picture or backdrop, your car standing in it, spots to pick, and buttons.
- **The season**: so many days of so many hours. Reading the paper is an hour, bolting a part on
  or off is an hour, a race is two, a pass alone is one. It ends when the days run out, when you
  have no car and no way to buy one, or when you beat the one to beat.
- **The paper**: three cars a day picked from the pack's cars, in differing paint and health,
  and every part in the pack.
- **The bench**: what is on the car, slot by slot, and what is on the shelf. Parts go on, come
  off, and are sold from here.
- **The yard**: up to sixteen cars; bring one into the bay or sell it to the dealer.
- **Rivals**: they pull in one at a time, in the pack's order. Race for kicks, for money up to
  what each will take, or for cars. Beating one for money or better brings out the next.
- **The strip**: the quarter mile from the side, with the pack's car pictures. The car's torque
  curve, gears, weight, grip and turbo are the pack's; the parts fitted change them.
- **Saving**: the game is kept in the browser as you go, and the title offers to carry on.

Not in the engine yet: the view from the seat, the drive across town, gas, and taking parts
off bolt by bolt under the hood. Those are in the C game, and come with steps 4 and 5.

## The workshop

`make engine && make serve`, then open http://localhost:8080/workshop.html. It opens the default
pack, or any `.pack` you give it, and edits its cars in the browser:

- **Add a car** and drop in its picture from the side, nose to the right. Any PNG will do; one
  wider than the screen is scaled down, and you choose how wide it stands in the game.
- **Background**: a PNG with a clear background is best. For one saved on a plain colour, tick
  the box and the corner's colour is made clear.
- **Ground**: the red line is where the tyres touch. It starts under the lowest pixel; drag it up
  if the picture has a shadow under the car.
- **Paint**: click the bodywork in the picture. Everything of about that colour becomes the
  paint, and the car can then be shown in any of the 31 colours. The slider widens or narrows
  what counts. Bright highlights that are nearly white are left as they are.
- **Headlamps**: a second picture, the same size, with the lamps on.
- The car is shown in the game's garage, by the engine itself, as you change things.
- **Save the pack** downloads it. **Play it** opens the game on it in another tab. Work in
  progress is kept in the browser between visits.

The workshop only edits cars so far: their pictures, paint and price. Places, parts, rivals and
a car's `drive` are still edited in `pack.json`; a car added in the workshop borrows the first
car's `drive`.

## A pack folder

    content/default/
      pack.json
      garage.png
      car.png
      car-lamps.png

`tools/pack.py content/default web/default.pack` matches each PNG to the 1,024 colours, stores it
run-length as `<name>.pic`, and puts everything in one file. PNGs may be any size up to 320x240;
transparent pixels stay clear.

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
  | `yard` | open the yard |
  | `race` | race whoever has pulled in (the place needs `rival_at`) |
  | `next` | wave them on and see who else is about |
  | `solo` | a pass down the strip alone |

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
- **parts**: one part to a `slot`; the slots are whatever the parts name. `torque` is a
  percentage more. A part with `boost` replaces the car's turbo. At most 14 parts.
- **rivals**: in the order they turn up. `et` is their quarter mile in seconds. `limit` is the
  most money they will race for, `pinks` whether they will race for cars. The one with `king`
  must be last, races for cars only, and beating them wins the game.
- **bets** and **paints** may be left out.

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
4. Rooms with their spots drawn and edited in the workshop, parts and rivals edited there too,
   and under the hood as a place where parts come off bolt by bolt.
5. Maps: the town and the roads, the view from the seat, and gas.
