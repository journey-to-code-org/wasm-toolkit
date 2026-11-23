# What the originals did, and where we are

The game is meant to do what Street Rod, Test Drive and Vette! did. This is the ledger: each thing
those games had, where it is written down, and whether ours has it yet. It is the list to work
down; `docs/PLAN.md` says in what order.

Status: **done** does what the original did; **part** is there but simpler; **no** is not there.

Sources: Street Rod from `kylofon/street-rod-sdl3` (`port/spec/game_flow.md`, `garage.md`,
`race.md`, section numbers as given). Test Drive from `kylofon/testdrive-enhanced` and
`kylofon/testdrive3-enhanced` (their README and `ENHANCED.md`). Vette! from `Vesuri/vette`
(`docs/manual.md`, `docs/gameplay-coverage.md`). The first three are MIT; the Vette! repository has
no license, so it is read and not copied. Nothing here is copied from any of them: our cars, town,
words and pictures are our own.

## Street Rod

### The season and the garage loop (`game_flow` §4)

| What the original had | Ours | Status |
| --- | --- | --- |
| New game: $750, no car, straight to the paper (§4.6, §4.3) | The same | done |
| A driver's license where you type your name; "Old Game" on it (§4.6) | Enter for new, E for old; no name | part |
| One game hour for each thing you do; 12 hours a day; real time also passes (§4.7) | An hour a thing, 12 a day; real time does not pass | part |
| Day 0 is a Sunday in June; day 91 ends the summer (§4.7) | Day 1 to day 91, ending at the Shootout; no weekdays or months | part |
| Calendar screen (§4.7) | The day on the wall, and a line when you pick it | part |
| Broke: no car, no spare cars, no spares, under $400 (§4.5) | No car, no spares, under $480 | done |
| Garage holds 16 cars and 130 parts, with "sell some first" (§4.5) | 16 cars; spares are at most one of each | part |
| Save and load, 15 slots, from "Time to quit" (§4.11) | One save in the browser, kept as you go | part |
| Hall of fame, "King Street Rodders" wall (§4.9, §5.3) | None | no |
| Win ending after the King; summer-over screen (§4.9, §4.10) | One screen for each ending | part |
| Copy protection (§4.12) | None, on purpose | n/a |
| Demo mode (§4.14) | None | no |

### The garage screen (`garage` §1.1, §4.7)

| Hot spot in the original | Ours | Status |
| --- | --- | --- |
| Check out the newspaper | The paper on the bench | done |
| Hit the street | The garage door, or the car door | done |
| Calendar | The calendar | part |
| Time to quit (save, load, new, quit) | Escape goes to the title; saving is automatic | part |
| Sell parts | The tool board | done |
| Cars (switch it, sell it) | Out the window: the yard | done |
| New paint job, $20 | None | no |
| Change tires | None | no |
| Change transmission | None | no |
| Pop the hood | The hood | done |
| Front bumper, rear bumper, the roof (customizing) | None | no |
| Stickers, $5 | None | no |
| Wanna tank? | The gas can by the door | part |
| Car info | None | no |
| Wanna drink? ("Don't drink and drive, bub!") | None | no |
| Juke box: music on and off | None; there is no music | no |

### The paper, buying and selling (`garage` §4.4 to §4.6, §5.1, §5.2)

| What the original had | Ours | Status |
| --- | --- | --- |
| Front page with the date, then Used Cars or Auto Parts (§4.4) | Two pages, Cars and Parts | part |
| 25 car models in classes, three makes; ads change (§5.2) | One model, in eight paints and a range of conditions | no |
| 43 parts: engines, transmissions, carbs, manifolds, tires, by make and grade (§5.1) | 4 bolt-ons: intake, turbo, exhaust, intercooler | no |
| Parts only fit their make; manifolds and carbs must match (§5.1) | Everything fits | no |
| Wear on engines, transmissions and tires (§4.2) | A fixed engine condition a car; nothing wears | no |
| Selling spares: an offer of 70 to 89 hundredths for bolt-ons, by condition for the rest (§4.5) | The same for bolt-ons | done |
| Selling a car, and what it is worth (§4.6) | The dealer's price, in the paper and the yard | done |

### Working on the car (`garage` §4.9 to §4.13)

| What the original had | Ours | Status |
| --- | --- | --- |
| Engine bay: bolts out, part off, part on, bolts in (§4.11) | The same, on four parts | part |
| Swapping the whole engine; carbs on a manifold, 1 to 3 of them (§4.11) | None | no |
| Tuning the ignition by ear (§4.13) | None | no |
| Changing the transmission from under the car (§4.10) | None | no |
| Changing tires (§4.9) | None | no |
| "Is it runnable?" before you can leave (§4.12) | Every part on and bolted | done |
| Top speed worked out from the parts (§4.16) | Torque and boost from the parts, at the strip | part |

### Bob's Drive-In and the challenge (`race` §4.14)

| What the original had | Ours | Status |
| --- | --- | --- |
| The cruise across town to get there, 0.2 gallons a leg (§4.1) | The gas is spent; the cruise is not driven | part |
| Opponents arrive one at a time after a wait; a car-hop comes and goes | One is always there; "Next" brings another | part |
| 21 opponents in three classes of seven, and the King (`game_flow` §4.6) | 21 in three classes and the King, met in order | part |
| Their cars: model, color, and what has been done to them (§4.9) | A paint and a time each | part |
| Drag race or road race | Drag only | no |
| Bets: kicks, $10, $25, $50, $100, pink slips; he may refuse (`challenge_accept`) | The same; refusal by his limit and a dice roll | part |
| Kicks do not count (§4.14) | The same | done |
| Win his car for pink slips; lose yours and walk (§4.13) | The same | done |
| The King: a drag race, then a road race, for pink slips (§4.14) | A drag race for pink slips | part |
| Fuel to race: 0.7 gal drag, 1.7 road, 2.2 the King; used 0.5 and 1.5 (§4.14) | 0.7 to race, 0.5 used | part |
| Two hours for a drag, three and a half for a road race | Two hours | part |

### The races (`race` §4.2 to §4.13, §4.16)

| What the original had | Ours | Status |
| --- | --- | --- |
| "Get ready!" and "Go!"; jumping it loses | A tree; a red light loses | done |
| Gears, revs, a dash with tach and speedometer (§4.7, §4.16) | The same, with boost | done |
| Blowing the engine or dropping the transmission from abuse (§4.13) | None | no |
| The other car on the road, with its own driving (§4.9 to §4.11) | It follows a curve to its time | part |
| Road race: a winding road, traffic, the finish (§4.8) | None | no |
| The police on the road race; fines; jail (§4.12, §4.15) | None | no |
| Crashing and its damage | None | no |

### The gas station (`garage` §4.20)

| What the original had | Ours | Status |
| --- | --- | --- |
| A screen: take the cap off, take the nozzle, pump, pay | Filled in one go from the garage | part |
| $1 for three gallons; $2 if you wait for full service | $1 for three gallons | part |
| The attendant, the windshield wash, forgetting the cap ($10) | None | no |
| A tank of 17 gallons; "You've got a full tank!"; "Gas ain't free!" | The same | done |

## Test Drive

| What the originals had | Source | Ours | Status |
| --- | --- | --- | --- |
| Driving from the seat with a painted dash and a wheel that turns | TD1 | The same, by day and night | done |
| A gear lever you move through its gate | TD1 README, Controls | E and Q shift; the gate lights | part |
| A choice of cars, each with its own dash and spec sheet | TD1 | One car, one dash | no |
| A mountain road in stages: a cliff one side, a drop the other | TD1 | None; the town is a flat grid | no |
| Traffic both ways, and passing it | TD1 | Traffic on the grid, one way a street | part |
| A radar detector, speed traps, the police chasing and stopping you | TD1 | None | no |
| A gas station at the end of each stage; time and average speed scored | TD1 | None | no |
| Crashing cracks the windshield and costs a life | TD1 | A bump and a count of dents | part |
| Rear-view mirror that shows what is behind | TD1, TD3 (R) | A mirror with nothing in it | no |
| Open 3D country you can leave the road in | TD3 `ENHANCED.md` Projection, Faces | A 3D grid of blocks | part |
| Hills, bridges, tunnels, scenery as sprites | TD3 Sprites, Sky and ground | None | no |
| Night, headlights, fog, rain and wipers | TD3 Headlight beams, Distance fog; README (H, W) | Night at the strip only; lamps go up | part |
| Radio; engine and music switches | TD3 README (M) | A radio painted on the dash | no |
| Chase view, replay | TD3 README (F5, F10) | None | no |
| Courses with a finish, a compass, skill levels | TD3 README | None | no |
| Handbrake, spins and slides | TD3 README | None | no |

## Vette!

| What the original had | Source | Ours | Status |
| --- | --- | --- | --- |
| A real city, San Francisco, with named places and a freeway | manual, Racing | A grid that goes on for ever | no |
| Four courses between landmarks; any route you like | manual, Racing | None | no |
| A choice of four cars and four opponents | coverage, vehicles | None in town | no |
| Three difficulties: damage, traction, police, cruise | manual, Racing | None | no |
| An opponent racing you across town by his own route | manual | None | no |
| Traffic, and pedestrians | manual | Traffic only, in straight lines | part |
| Police: tickets and penalties | coverage, recovery | None | no |
| Damage, repairs at a gas station, a tow when it is too bad | manual, Racing | Dents counted | part |
| Water ends the race | manual, Racing | None | no |
| Views: left, ahead, right, helicopter; higher and lower | manual, keys F1 to F8 | Ahead only | no |
| A map and navigation help | manual, key H | None | no |
| Automatic or manual, cruise control, a horn | manual, keys | None in town | no |
| A dyno test in the garage | coverage, `garage_dyno` | A torque curve from a dyno sheet; no dyno screen | part |
| Scores kept between games | coverage, session | None | no |
| Two players over a link | manual: not in the port either | None | no |

## Count

| Game | done | part | no |
| --- | ---: | ---: | ---: |
| Street Rod | 15 | 24 | 24 |
| Test Drive | 1 | 5 | 10 |
| Vette! | 0 | 3 | 12 |
