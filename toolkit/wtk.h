/* wtk: a toolkit for games that look and run like 1989.
 *
 * One 320x240 screen of palette indices, 1,024 colours to choose from, drawn by the game in plain C and shown by a small
 * host page. No libc, no floats, no allocation: fixed-point maths and fixed-size tables, the way
 * the games this is for were written. It builds to WebAssembly with clang alone, and to a native
 * program for tests.
 *
 * A game supplies three functions (game_init, game_tick, game_draw); the toolkit calls them.
 */
#ifndef WTK_TOOLKIT_H
#define WTK_TOOLKIT_H

typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
typedef signed short i16;
typedef signed int i32;
typedef signed long long i64;

#define WTK_W 320
#define WTK_H 240
/** Ticks a second. The game moves in ticks; the host calls wtk_step once for each. */
#define WTK_HZ 30

/* ---- The screen ---------------------------------------------------------------------------- */

/** A colour: a place in the palette. */
typedef u16 col;
#define WTK_COLOURS 1024
/** The palette is 32 ramps of 32 shades: shade 0 is nearly black, 24 the colour itself, 31 a highlight. */
#define WTK_SHADES 32

extern col wtk_fb[WTK_W * WTK_H];
/** The palette, a colour an entry, as bytes R G B A in memory. */
extern u32 wtk_pal[WTK_COLOURS];

/** The sixteen EGA colours, in their usual order, at the start of the palette. */
enum {
  BLACK, BLUE, GREEN, CYAN, RED, MAGENTA, BROWN, LGRAY,
  DGRAY, LBLUE, LGREEN, LCYAN, LRED, LMAGENTA, YELLOW, WHITE
};

/** The ramps the default palette has. Ramp 0 is the EGA sixteen and is not a ramp at all. */
enum {
  R_SYSTEM, R_GREY, R_RED, R_ORANGE, R_YELLOW, R_LIME, R_GREEN, R_TEAL,
  R_CYAN, R_SKY, R_BLUE, R_INDIGO, R_VIOLET, R_MAGENTA, R_PINK, R_BROWN,
  R_TAN, R_BRICK, R_CONCRETE, R_ASPHALT, R_GRASS, R_GLASS, R_CHROME, R_RUBBER,
  R_WOOD, R_CREAM, R_NAVY, R_MAROON, R_FOREST, R_GOLD, R_SILVER, R_WHITE
};

static inline col wtk_shade(int ramp, int s) { return (col)(ramp * WTK_SHADES + (s < 0 ? 0 : s > 31 ? 31 : s)); }

void wtk_pal_set(int i, int r, int g, int b);
/** Load the EGA sixteen into entries 0 to 15. */
void wtk_pal_ega(void);
/** Make ramp `ramp` the 32 shades of one colour. */
void wtk_pal_ramp(int ramp, int r, int g, int b);
/** The EGA sixteen and the 31 ramps named above. wtk_init loads this. */
void wtk_pal_default(void);

/** Drawing is kept inside this box. wtk_clip_off puts it back to the whole screen. */
void wtk_clip(int x, int y, int w, int h);
void wtk_clip_off(void);

void wtk_clear(col c);
void wtk_pset(int x, int y, col c);
void wtk_hline(int x0, int x1, int y, col c);
void wtk_vline(int x, int y0, int y1, col c);
void wtk_line(int x0, int y0, int x1, int y1, col c);
/** A filled box, and the outline of one. */
void wtk_rect(int x, int y, int w, int h, col c);
void wtk_box(int x, int y, int w, int h, col c);
/** A box in a checkerboard of two colours: how sixteen colours were made to look like more. */
void wtk_dither(int x, int y, int w, int h, col a, col b);
void wtk_disc(int cx, int cy, int r, col c);
/** A box that fades down a ramp from shade s0 at the top to s1 at the bottom, dithered between shades. */
void wtk_vgrad(int x, int y, int w, int h, int ramp, int s0, int s1);
/** A filled convex polygon: n points as x0 y0 x1 y1 ... A second colour dithers it; pass the same twice for flat. */
void wtk_poly(const int *xy, int n, col a, col b);
/** A picture of palette indices; pixels of colour `key` are left out (pass -1 to draw them all). */
void wtk_blit(const col *src, int w, int h, int x, int y, int key);

/** A painted picture, run-length: width, height, then (count, colour) pairs, with 0xFFFF for clear.
    `dim` takes that many shades off every colour: the same picture, at night. */
void wtk_rle(const u16 *data, int x, int y, int dim);
/** A square picture of `size` by `size` colours (0xFFFF clear), turned by `angle` about its middle
    and drawn with its middle at (cx, cy). */
void wtk_spin(const col *px, int size, int cx, int cy, int angle, int dim);

/** Text in the 8x8 font. Returns the x after the last character. */
int wtk_text(int x, int y, const char *s, col c);
int wtk_num(int x, int y, int n, col c);
/** The same font with every pixel `scale` across. */
int wtk_text_big(int x, int y, const char *s, col c, int scale);
/** Text with a one-pixel shadow under it. */
int wtk_text_sh(int x, int y, const char *s, col c, col shadow);
int wtk_strlen(const char *s);

/* ---- Fixed-point maths ---------------------------------------------------------------------
 * fx is 16.16. Angles are whole numbers, 1024 to the turn. */

typedef i32 fx;
#define FX(n) ((fx)((n) * 65536))
#define FX_INT(a) ((a) >> 16)
#define WTK_TURN 1024

static inline fx fx_mul(fx a, fx b) { return (fx)(((i64)a * b) >> 16); }
static inline fx fx_div(fx a, fx b) { return (fx)(((i64)a * 65536) / b); }
fx fx_sin(int angle);
fx fx_cos(int angle);
u32 wtk_isqrt(u32 n);
/** A random number from a state the game keeps: the same seed gives the same game. */
u32 wtk_rand(u32 *state);

/* ---- Three dimensions ----------------------------------------------------------------------
 * Flat-shaded polygons, sorted far to near and painted in that order. x is east, y is up, z is
 * north; a yaw of 0 looks north and grows clockwise. */

typedef struct { fx x, y, z; } v3;

typedef struct {
  v3 pos;
  int yaw;
  /** Tipped down by this much: 0 looks level. */
  int pitch;
  /** How far the picture plane is, in pixels: a longer lens for a bigger number. */
  int focal;
  /** Where on the screen the camera looks: the horizon is at cy. */
  int cx, cy;
} wtk_cam;

#define WTK3_MAX_POLYS 4096
#define WTK3_MAX_VERTS 8

void wtk3_begin(const wtk_cam *cam);
/** Queue a convex polygon of up to WTK3_MAX_VERTS points. `bias` nudges it nearer (positive) in the sort. */
void wtk3_poly(const v3 *pts, int n, col a, col b, int bias);
/** The same, in a ramp, shaded by how squarely it faces the light. Either winding will do. */
void wtk3_lit(const v3 *pts, int n, int ramp, int bias);
/** Paint everything queued since wtk3_begin. Returns how many polygons were drawn. */
int wtk3_end(void);
/** Where a point lands on the screen. Returns 0 when it is behind the camera. */
int wtk3_point(v3 p, int *sx, int *sy, fx *depth);

/* ---- Input, sound and the host -------------------------------------------------------------- */

enum {
  KEY_UP = 1, KEY_DOWN = 2, KEY_LEFT = 4, KEY_RIGHT = 8,
  KEY_A = 16,     /* Enter, Space or the mouse button: yes, pick, go */
  KEY_B = 32,     /* Escape or Backspace: no, back */
  KEY_X = 64,     /* shift up */
  KEY_Y = 128     /* shift down */
};

/** Keys held this tick, and keys that went down this tick. */
extern u32 wtk_keys, wtk_hit;
/** The pointer, in screen pixels, and whether it moved this tick. */
extern int wtk_mx, wtk_my, wtk_moved;

/** A tone from the PC speaker: a square wave of `hz` for `ms`. 0 hz is silence. Supplied by the host. */
void wtk_tone(int hz, int ms);

/* What the game supplies. */
void game_init(u32 seed);
void game_tick(void);
void game_draw(void);

/* What the host calls. */
void wtk_init(u32 seed);
void wtk_input(u32 keys, int mx, int my);
void wtk_step(void);
col *wtk_screen(void);
u32 *wtk_palette(void);

#endif
