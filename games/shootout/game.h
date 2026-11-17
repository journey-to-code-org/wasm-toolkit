/* Road to the Shootout: what its parts share. Each screen is a file: the garage (and the work you
 * do in it), the town, the strip. main.c is the title and the switch between them. */
#ifndef SHOOTOUT_GAME_H
#define SHOOTOUT_GAME_H

#include "../../toolkit/wtk.h"

enum { TITLE, GARAGE, HOOD, PAPER, DRIVE, STRIP };
extern int screen;
extern u32 ticks, rng;

/** How much of the screen is windscreen when you are in the seat; the dash is below it. */
#define VIEW_H 136
/** What colour your car is. */
#define PLAYER_PAINT R_BLUE

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
void strip_start(void);
void strip_tick(void);
void strip_draw(void);

/* ---- main.c ---- */

/** The DSM mark, 96 by 22. */
void logo(int x, int y, col black, col red);

#endif
