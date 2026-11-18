/* Road to the Shootout: what its parts share. Each screen is a file: the garage (and the work you
 * do in it), the town, the strip. main.c is the title and the switch between them. */
#ifndef SHOOTOUT_GAME_H
#define SHOOTOUT_GAME_H

#include "../../toolkit/wtk.h"

enum { TITLE, GARAGE, HOOD, PAPER, MEET, DRIVE, STRIP, OVER };
extern int screen;
extern u32 ticks, rng;

/** How much of the screen is windscreen when you are in the seat; the dash is below it. */
#define VIEW_H 136
/** What colour your car is. */
#define PLAYER_PAINT car_paint
/** The game is a season: it ends on this day, at the Shootout. Twelve hours make a day. */
#define LAST_DAY 91
#define DAY_HOURS 12

static inline v3 at(int x, int y, int z) { v3 p = {FX(x), FX(y), FX(z)}; return p; }

/* ---- cars.c: the cars, as lit models ---- */

typedef struct { i16 x, w, sill, belt, roof_w, roof; } section;
typedef struct { const char *name; const section *sec; u8 n, cowl, popups, wing; } body;
enum { B_WEDGE, B_CURVE, B_COUPE, B_SEDAN, B_WAGON, B_PICKUP, N_BODIES };
extern const body BODIES[N_BODIES];
/** Where the scene is being looked at from. Set it before drawing cars. */
extern v3 eye;
/** A car at (x, z) pointing `yaw`. `lamps` raises pop-up headlamps on a body that has them. */
void car_mesh(const body *b, fx x, fx z, int yaw, int paint, int lamps);

/* ---- cockpit.c: the view from the seat ---- */

typedef struct { int night, paint, rpm, mph, boost, gear, wheel, shift_light, lamps; } cockpit_look;
void cockpit(const cockpit_look *look);

/* ---- garage.c: the garage, under the hood, and the paper ---- */

extern int cash, lamps_up;
/** Your car: whether you have one, its paint, and how healthy its engine is, as a percentage. */
extern int has_car, car_paint, car_cond;
/** How many hours of the season are gone, and how many of the regulars you have beaten. */
extern int hours, rung;
/** How it ended: 1 the season ran out, 2 broke, 3 you beat the King. */
extern int over_kind;
/** Your car, or anyone's, as a picture in its own paint, standing on `ground`. */
void car_picture(int x, int ground, int paint, int lamps);
/** Losing your car: what was bolted to it goes with it. */
void car_lost(void);
void garage_reset(void);
void garage_tick(void);
void garage_draw(void);
/** Whether the car will start, and if not, why not. */
int car_runs(const char **why);
/** What the parts on the car do: torque as a percentage of stock, and whether the big turbo is on. */
int tune_gain(void);
int tune_big_turbo(void);

/* ---- town.c and strip.c ---- */

void drive_start(void);
void drive_tick(void);
void drive_draw(void);
/** A pass down the strip. `rival_ms` is what the car in the other lane runs, or 0 to run alone. */
void strip_race(int rival_ms, int rival_paint);

/* ---- meet.c: the lot by the strip, where the regulars pull up ---- */

void meet_start(void);
void meet_tick(void);
void meet_draw(void);
/** The race is run: 1 you won it, 0 you lost, -1 there was nobody in the other lane. */
void meet_settle(int won);
/** The bottom panel, and the pointer, which the garage owns. */
void panel(const char *a, const char *b, const char *c);
void pointer_move(void);
void pointer_draw(void);
int pointer_x(void);
int pointer_y(void);
void strip_tick(void);
void strip_draw(void);

/* ---- main.c ---- */

/** The DSM mark, 96 by 22. */
void logo(int x, int y, col black, col red);

#endif
