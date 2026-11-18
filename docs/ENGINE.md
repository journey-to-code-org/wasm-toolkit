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

Steps 1 and 2 of 5 are done. The engine reads a pack, shows its title, and lets you walk between its
places: each place is a picture (or a plain backdrop) with a car standing in it and spots to pick.
The paper, the hood, the bets, the strip and the cockpit are still only in the C game.

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

The workshop only edits cars so far. Places are still edited in `pack.json`.

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

- **start**: the cash, place and car a new game begins with. `car` may be left out.
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

Ramp names: see `RAMPS` in `engine/src/gfx.rs`.

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
3. Parts, rivals and the paper as data, and the season's rules in the engine.
4. Rooms with their spots drawn and edited in the workshop.
5. Maps: the town and the roads.
