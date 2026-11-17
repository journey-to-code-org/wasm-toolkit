# wasm-toolkit

A toolkit for games that look and run like 1989, and the first game built on it.

`wtk` is one 320x240 screen of palette indices, 1,024 colours in 32 ramps of 32 shades, drawn by the
game in plain C: boxes, lines, dithered gradients, an 8x8 font, and lit polygons sorted far to near. No libc, no floats, no allocation. It
builds to WebAssembly with clang and nothing else, and a small page shows the screen, scaled up in
whole pixels, and plays the PC speaker.

The game is **Road to the Shootout**: a garage, a car, and a city to drive it in, after Street Rod,
Test Drive and Vette!. What is here is a first slice. [`docs/PLAN.md`](docs/PLAN.md) is the rest.

## Build and play

Needs `clang` with the wasm32 target, `wasm-ld` (the `lld` package), `make`, and `node` for the tests.

```
make          # web/shootout.wasm
make serve    # then open http://localhost:8080
make test     # the toolkit's tests, natively, and the game played by a script
```

`python3 tools/bundle.py` writes `build/shootout.html`: the whole game in one file that opens from disk.

Keys: arrows or WASD, Enter or Space to pick, Escape to go back. The mouse moves the pointer.

## Layout

| Path | What it is |
| --- | --- |
| `toolkit/wtk.h` | The whole interface, with what each call does |
| `toolkit/wtk.c` | The screen, drawing, fixed-point maths, 3D, and the door to the host |
| `toolkit/font.c` | The 8x8 font (public domain, from Daniel Hepper's font8x8) |
| `web/wtk.js` | The host: loads the `.wasm`, shows the screen, reads keys and pointer, makes the tones |
| `games/shootout/` | The game |
| `test/` | `test_native.c` checks the toolkit; `smoke.mjs` plays the built game twice and compares |

## Writing a game on it

A game is three functions, and the toolkit calls them thirty times a second:

```c
#include "../../toolkit/wtk.h"

void game_init(u32 seed) {}
void game_tick(void) {}                       /* wtk_keys, wtk_hit, wtk_mx, wtk_my */
void game_draw(void) { wtk_vgrad(0, 0, WTK_W, WTK_H, R_SKY, 8, 28); wtk_text(8, 8, "HELLO", WHITE); }
```

Put it in `games/<name>/` and build it with `make GAME=<name>`.

The rules the toolkit keeps, and a game on it should: whole numbers only (`fx` is 16.16), nothing
from the clock, and every random number from a state the game owns. The same seed and the same keys
give the same game, pixel for pixel, which is what the tests check and what multiplayer will need.
