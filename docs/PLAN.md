# Road to the Shootout, the 1989 way

The aim: Street Rod's garage game, with Test Drive's driving and Vette!'s city, written the way
they were written and looking the way they looked. Our own cars, town, art and words throughout:
it is those games' design, not their files.

## What makes it look right

These are the rules the toolkit enforces or the game keeps.

- **320x240, 1,024 colours.** A palette of 32 ramps of 32 shades. Rooms are painted in dithered
  gradients; nothing is smoothed or blended, and the screen is scaled up in whole pixels.
- **Lit polygons, painted far to near.** One light, one shade a face, no textures.
- **One 8x8 font**, capitals, for everything.
- **A pointer and a line of text.** A screen is a picture; the thing under the pointer is named at
  the bottom; Enter picks it. No buttons, panels or scrolling lists.
- **PC speaker.** One square wave at a time.

## The cars

The cars are our own: their own shapes, their own names, no maker's badge. What is true to life is
the engineering. The game is built round one engine family, a turbo two-litre four, and every kind
of car an engine like it went into through the nineties: liftback coupes in four generations, front
drive and all-wheel drive, with and without the turbo; the sedans; a small pickup; and a rally
sedan in ten generations. Parts interchange across the family, which is what makes a parts car
worth buying. Other makers' big names of the era get the same treatment: a kind of car, not a copy.

A body is a table of cross-sections (`games/shootout/main.c`), so a new car is a dozen lines.

## Done

- [x] The toolkit: screen, drawing, dither, font, fixed-point maths, 3D with near clipping and
      depth sort, input, tones, a native test build and a scripted play-through.
- [x] A title, a garage with a pointer and named things, and a hood that opens.
- [x] Cars as lit 3D models from cross-sections: a coupe, a sedan, a wagon and a pickup so far.
- [x] A city that goes on for ever from a seed: streets, blocks, buildings, parks.
- [x] Driving it from the seat: dash, wheel, traffic, walls that dent the car.

## Next, in order

### 1. The garage game (Street Rod)

- [ ] The paper: classifieds for cars and for parts, bought with cash, a new paper each day.
- [ ] Parts are things: bought, carried home, on the floor until fitted.
- [ ] Under the hood, as a screen: each part held on by bolts, undone one at a time.
- [ ] Under the car: transmission, tyres, with the jack.
- [ ] Tuning by hand: timing and mixture, with the engine telling you when it is wrong.
- [ ] Paint, stickers, chopping the top.
- [ ] Fuel, and a pump.
- [ ] Days, cash, and the save.

The rules for all of this exist already, in TypeScript, in open.games (`pinkslipscars`,
`pinkslipsgarage`): the cars, the parts, oil, mixture, wear, tools, Rix Racing, the scrappers.
They are whole-number and deterministic, so they port to C line for line. Port, do not redesign.

### 2. The meet (Street Rod)

- [ ] The drive-in: cars pull through one at a time; look one over, wave it on or challenge it.
- [ ] The bet: for fun, for cash, or for the car.
- [ ] The drag: the tree, the shift, the tach, missed shifts and blown engines.
- [ ] The road race out of town, with the law.
- [ ] The one to beat, at the end.

### 3. The drive (Test Drive)

- [ ] A real drivetrain behind the dash: gears, revs, redline, a shifter you can miss.
- [ ] The road out of town: hills, cliffs, oncoming traffic, a radar detector, the police.
- [ ] Gas stations as the stops between stages.
- [ ] The windscreen cracks.

### 4. The city (Vette!)

- [ ] A drawn town in place of the endless grid: districts, landmarks, a bridge, the strip, Rix
      Racing, the way to Morgantown.
- [ ] Traffic that obeys lights and turns at corners; pedestrians on the pavements.
- [ ] A map, and outside views of the car.
- [ ] Police who chase, pull you over, and ask questions.
- [ ] Races across town by any route.

### 5. Other people

- [ ] The same build, a tick at a time, on a server: races between two players, and wagers.
- [ ] Players in the town and at the meet.
- [ ] The Shootout.

## Not decided

- **Where it is played.** It can be dropped into open.parlor as one more game (a canvas and a
  `.wasm`), or stand alone. Nothing here depends on the answer yet.
- **The game that is in open.parlor now.** It stays as it is until this one can do what it does.
- **Touch.** A pointer works on a phone. Driving needs on-screen controls, which those games
  never had.

## Known rough edges in the slice

- Polygons are sorted by their middles, so a long wall can be painted over a nearer short one at
  some angles. The fix is to sort by block, not by polygon.
- The engine is a placeholder: it changes gear by itself and the tach only follows speed.
- Other cars drive straight for ever and through each other.
