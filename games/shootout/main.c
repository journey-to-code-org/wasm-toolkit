/* Road to the Shootout, the old way: a first slice.
 *
 * Three screens. A title. A garage you look round with a pointer, where the thing under the
 * pointer is named on the bottom lines and Enter picks it. And a city of blocks to drive through
 * from behind the wheel, with traffic to miss and walls not to hit.
 *
 * 320x240 in 1,024 colours: rooms are painted in dithered gradients, and the cars and the city
 * are lit polygons. Everything on the screen is drawn here; there are no picture files.
 */
#include "../../toolkit/wtk.h"

enum { TITLE, GARAGE, DRIVE };
static int screen;
static u32 ticks, rng;

/* ---- Cars ------------------------------------------------------------------------------------
 * A body is a row of cross-sections from the back bumper to the nose, in centimetres. Each has a
 * half-width, a sill height, a beltline, and, where there is a cabin over it, the roof's
 * half-width and height. The skin between two sections is paint, or glass where a cabin begins,
 * ends or runs. */

typedef struct { i16 x, w, sill, belt, roof_w, roof; } section;
typedef struct { const char *name; const section *sec; u8 n, cowl; } body;

static const section COUPE[] = {
  {0, 70, 40, 80, 0, 0}, {18, 82, 27, 88, 0, 0}, {72, 86, 22, 92, 0, 0}, {128, 87, 22, 94, 57, 119}, {180, 87, 22, 95, 61, 128},
  {238, 87, 22, 94, 59, 124}, {292, 86, 22, 89, 0, 0}, {362, 84, 22, 81, 0, 0}, {418, 80, 27, 71, 0, 0}, {440, 68, 37, 63, 0, 0},
};
static const section SEDAN[] = {
  {0, 74, 40, 88, 0, 0}, {18, 84, 27, 95, 0, 0}, {96, 87, 22, 97, 0, 0}, {142, 87, 22, 97, 59, 137}, {252, 87, 22, 96, 61, 139},
  {304, 86, 22, 93, 0, 0}, {382, 84, 22, 87, 0, 0}, {442, 80, 27, 79, 0, 0}, {462, 70, 37, 71, 0, 0},
};
static const section WAGON[] = {
  {0, 78, 38, 97, 58, 135}, {12, 85, 27, 97, 61, 141}, {272, 87, 22, 96, 61, 141}, {324, 86, 22, 93, 0, 0}, {402, 84, 22, 87, 0, 0},
  {452, 80, 27, 79, 0, 0}, {472, 70, 37, 71, 0, 0},
};
static const section PICKUP[] = {
  {0, 80, 42, 97, 0, 0}, {202, 82, 36, 97, 0, 0}, {206, 82, 36, 99, 63, 151}, {292, 82, 36, 99, 63, 151}, {338, 82, 36, 97, 0, 0},
  {422, 80, 36, 93, 0, 0}, {456, 76, 42, 85, 0, 0}, {462, 70, 48, 79, 0, 0},
};
static const body BODIES[] = {
  {"COUPE", COUPE, 10, 6}, {"SEDAN", SEDAN, 9, 5}, {"WAGON", WAGON, 7, 3}, {"PICKUP", PICKUP, 8, 4},
};

/* Where the car being drawn stands, and which way it points. */
static fx m_x, m_z, m_sin, m_cos;
/* Where the car is being looked at from: the wheels on the far side are behind the body. */
static v3 eye;
static int m_half;

static v3 M(int lx, int ly, int lz) {
  fx f = (lx - m_half) * 655, u = ly * 655, r = lz * 655;
  v3 p = {m_x + fx_mul(f, m_sin) + fx_mul(r, m_cos), u, m_z + fx_mul(f, m_cos) - fx_mul(r, m_sin)};
  return p;
}

/* The three points down one side of a section: the sill, the beltline, and the edge of the top. */
static void side_of(const section *s, int k, v3 out[3]) {
  int top_w = s->roof ? s->roof_w : s->w * 4 / 5, top = s->roof ? s->roof : s->belt + 3;
  out[0] = M(s->x, s->sill, k * s->w);
  out[1] = M(s->x, s->belt, k * s->w);
  out[2] = M(s->x, top, k * top_w);
}

static void wheel(int x, int z, int k, int near) {
  enum { R = 31 };
  int bias = near ? 1 : -1;
  v3 face[8], hub[8];
  for (int i = 0; i < 8; i++) {
    int c = FX_INT(fx_cos(i * 128 + 64) * R), s = FX_INT(fx_sin(i * 128 + 64) * R);
    face[i] = M(x + c, R + s, k * (z + 5));
    hub[i] = M(x + c * 6 / 10, R + s * 6 / 10, k * (z + 6));
  }
  for (int i = 0; i < 8; i++) {
    int j = (i + 1) & 7;
    int c = FX_INT(fx_cos(i * 128 + 64) * R), s = FX_INT(fx_sin(i * 128 + 64) * R);
    int c2 = FX_INT(fx_cos(j * 128 + 64) * R), s2 = FX_INT(fx_sin(j * 128 + 64) * R);
    v3 tread[4] = {face[i], face[j], M(x + c2, R + s2, k * (z - 16)), M(x + c, R + s, k * (z - 16))};
    wtk3_lit(tread, 4, R_RUBBER, bias);
  }
  if (!near) return;
  wtk3_lit(face, 8, R_RUBBER, 1);
  wtk3_lit(hub, 8, R_CHROME, 2);
}

static void car_mesh(const body *b, fx x, fx z, int yaw, int paint, int hood_up) {
  const section *s = b->sec;
  int last = b->n - 1;
  m_x = x; m_z = z; m_sin = fx_sin(yaw); m_cos = fx_cos(yaw); m_half = s[last].x / 2;

  /* What it stands on: a shadow. */
  v3 shadow[4] = {M(-10, 1, -95), M(s[last].x + 10, 1, -95), M(s[last].x + 10, 1, 95), M(-10, 1, 95)};
  wtk3_poly(shadow, 4, wtk_shade(R_ASPHALT, 3), wtk_shade(R_ASPHALT, 5), -3);

  for (int i = 0; i < last; i++) {
    int cabin = (s[i].roof != 0) + (s[i + 1].roof != 0);
    v3 al[3], ar[3], bl[3], br[3];
    side_of(&s[i], -1, al); side_of(&s[i], 1, ar); side_of(&s[i + 1], -1, bl); side_of(&s[i + 1], 1, br);
    v3 low_l[4] = {al[0], bl[0], bl[1], al[1]}, low_r[4] = {ar[0], br[0], br[1], ar[1]};
    v3 up_l[4] = {al[1], bl[1], bl[2], al[2]}, up_r[4] = {ar[1], br[1], br[2], ar[2]};
    v3 top[4] = {al[2], ar[2], br[2], bl[2]};
    wtk3_lit(low_l, 4, paint, 0);
    wtk3_lit(low_r, 4, paint, 0);
    wtk3_lit(up_l, 4, cabin ? R_GLASS : paint, 0);
    wtk3_lit(up_r, 4, cabin ? R_GLASS : paint, 0);
    if (hood_up && i >= b->cowl) continue;
    wtk3_lit(top, 4, cabin == 1 ? R_GLASS : paint, 0);
  }
  /* The two ends, each with a bumper across the bottom of it. */
  for (int e = 0; e < 2; e++) {
    const section *q = &s[e ? last : 0];
    int out = e ? 1 : -1;
    v3 l[3], r[3];
    side_of(q, -1, l); side_of(q, 1, r);
    v3 cap[6] = {l[0], l[1], l[2], r[2], r[1], r[0]};
    wtk3_lit(cap, 6, paint, 0);
    v3 bumper[4] = {M(q->x + out, q->sill, -q->w), M(q->x + out, q->sill + 15, -q->w), M(q->x + out, q->sill + 15, q->w), M(q->x + out, q->sill, q->w)};
    wtk3_lit(bumper, 4, R_RUBBER, 1);
    for (int k = -1; k <= 1; k += 2) {
      /* Lamps: white at the front, red at the back. */
      int in = q->w * 45 / 100;
      v3 lamp[4] = {M(q->x + out * 2, q->belt - 17, k * in), M(q->x + out * 2, q->belt - 6, k * in), M(q->x + out * 2, q->belt - 6, k * (q->w - 4)), M(q->x + out * 2, q->belt - 17, k * (q->w - 4))};
      col c = e ? wtk_shade(R_CREAM, 29) : wtk_shade(R_RED, 23);
      wtk3_poly(lamp, 4, c, c, 2);
    }
  }
  if (hood_up) {
    const section *c = &s[b->cowl], *n = &s[last - 1];
    int w = c->w * 4 / 5;
    v3 bay[4] = {M(c->x, c->belt - 10, -w), M(n->x, n->belt - 8, -w), M(n->x, n->belt - 8, w), M(c->x, c->belt - 10, w)};
    wtk3_poly(bay, 4, wtk_shade(R_RUBBER, 6), wtk_shade(R_RUBBER, 8), 0);
    int ex = (c->x + n->x) / 2;
    v3 block[4] = {M(ex - 26, c->belt - 4, -34), M(ex + 26, c->belt - 4, -34), M(ex + 26, c->belt - 4, 34), M(ex - 26, c->belt - 4, 34)};
    wtk3_lit(block, 4, R_GREY, 1);
    v3 cover[4] = {M(ex - 10, c->belt - 2, -30), M(ex + 10, c->belt - 2, -30), M(ex + 10, c->belt - 2, 30), M(ex - 10, c->belt - 2, 30)};
    wtk3_lit(cover, 4, R_RED, 2);
    v3 hood[4] = {M(c->x, c->belt + 3, -w), M(c->x + 56, c->belt + 84, -w), M(c->x + 56, c->belt + 84, w), M(c->x, c->belt + 3, w)};
    wtk3_lit(hood, 4, paint, 3);
  }
  fx right = fx_mul(eye.x - x, m_cos) - fx_mul(eye.z - z, m_sin);
  for (int k = -1; k <= 1; k += 2) {
    wheel(78, s[2].w, k, (k > 0) == (right > 0));
    wheel(s[last].x - 80, s[last - 2].w, k, (k > 0) == (right > 0));
  }
}

/* ---- The pointer, and the things it can be on ------------------------------------------------ */

typedef struct { i16 x, y, w, h; const char *name; int does; } spot;
enum { DO_NOTHING, DO_DRIVE, DO_HOOD };
static int px = 160, py = 100, hood_up;

static void pointer_move(void) {
  if (wtk_moved) { px = wtk_mx; py = wtk_my; }
  int step = 3;
  if (wtk_keys & KEY_LEFT) px -= step;
  if (wtk_keys & KEY_RIGHT) px += step;
  if (wtk_keys & KEY_UP) py -= step;
  if (wtk_keys & KEY_DOWN) py += step;
  if (px < 0) px = 0;
  if (px > WTK_W - 1) px = WTK_W - 1;
  if (py < 0) py = 0;
  if (py > 198) py = 198;
}

static void pointer_draw(void) {
  static const int arrow[] = {0, 0, 0, 11, 3, 8, 8, 8};
  int xy[8], sh[8];
  for (int i = 0; i < 4; i++) {
    xy[i * 2] = px + arrow[i * 2];
    xy[i * 2 + 1] = py + arrow[i * 2 + 1];
    sh[i * 2] = xy[i * 2] + 1;
    sh[i * 2 + 1] = xy[i * 2 + 1] + 1;
  }
  wtk_poly(sh, 4, BLACK, BLACK);
  wtk_poly(xy, 4, WHITE, WHITE);
  wtk_line(px, py, px, py + 11, BLACK);
  wtk_line(px, py, px + 8, py + 8, BLACK);
}

static const spot *spot_under(const spot *all, int n) {
  for (int i = n - 1; i >= 0; i--)
    if (px >= all[i].x && px < all[i].x + all[i].w && py >= all[i].y && py < all[i].y + all[i].h) return &all[i];
  return 0;
}

/* ---- The title -------------------------------------------------------------------------------- */

static void title_draw(void) {
  wtk_vgrad(0, 0, WTK_W, 150, R_NAVY, 3, 20);
  wtk_vgrad(0, 150, WTK_W, 90, R_ASPHALT, 16, 4);
  for (int i = 0; i < 40; i++) {
    u32 h = (u32)i * 2654435761u;
    wtk_pset((int)(h % 320), (int)((h >> 9) % 110), wtk_shade(R_WHITE, 12 + (int)((h >> 20) % 16)));
  }
  /* The car, turning under a light. */
  wtk_cam cam = {{0, FX(2) + FX(1) / 4, -FX(9)}, 0, 0, 380, WTK_W / 2, 104};
  eye = cam.pos;
  wtk3_begin(&cam);
  v3 pool[8];
  for (int i = 0; i < 8; i++) { pool[i].x = fx_cos(i * 128) * 7 / 2; pool[i].y = 0; pool[i].z = fx_sin(i * 128) * 7 / 2; }
  wtk3_poly(pool, 8, wtk_shade(R_ASPHALT, 20), wtk_shade(R_ASPHALT, 22), -9);
  car_mesh(&BODIES[0], 0, 0, (int)ticks * 3, R_RED, 0);
  wtk3_end();

  wtk_text_big(65, 19, "ROAD TO THE", wtk_shade(R_RED, 12), 2);
  wtk_text_big(64, 18, "ROAD TO THE", wtk_shade(R_GOLD, 26), 2);
  wtk_text_big(42, 44, "SHOOTOUT", wtk_shade(R_RED, 14), 4);
  for (int j = 0; j < 4; j++) {
    wtk_clip(0, 42 + j * 8, WTK_W, 8);
    wtk_text_big(40, 42, "SHOOTOUT", wtk_shade(R_GOLD, 31 - j * 3), 4);
  }
  wtk_clip_off();
  if ((ticks / 15) & 1) wtk_text_sh(116, 222, "PRESS ENTER", WHITE, BLACK);
}

/* ---- The garage ------------------------------------------------------------------------------- */

static const spot GARAGE_SPOTS[] = {
  {196, 20, 112, 120, "THE GARAGE DOOR: DRIVE OUT", DO_DRIVE},
  {18, 26, 64, 52, "A WINDOW. IT IS DAYLIGHT.", DO_NOTHING},
  {96, 24, 30, 38, "THE CALENDAR", DO_NOTHING},
  {14, 98, 70, 42, "THE BENCH. NOTHING ON IT YET.", DO_NOTHING},
  {34, 114, 150, 70, "THE CAR: GET IN AND DRIVE", DO_DRIVE},
  {184, 128, 76, 56, "THE HOOD", DO_HOOD},
};
#define N_GARAGE (int)(sizeof GARAGE_SPOTS / sizeof GARAGE_SPOTS[0])

static void garage_draw(void) {
  /* Plank walls in shadow at the top, a beam, and a concrete floor that is brightest by the door. */
  wtk_vgrad(0, 0, WTK_W, 140, R_WOOD, 9, 19);
  for (int x = 0; x < WTK_W; x += 16) {
    wtk_vline(x, 10, 139, wtk_shade(R_WOOD, 5));
    wtk_vline(x + 1, 10, 139, wtk_shade(R_WOOD, 22));
  }
  for (int x = 8; x < WTK_W; x += 32) wtk_disc(x + (x * 5) % 9, 40 + (x * 7) % 80, 1, wtk_shade(R_WOOD, 4));
  wtk_vgrad(0, 0, WTK_W, 10, R_WOOD, 3, 8);
  wtk_hline(0, WTK_W - 1, 10, wtk_shade(R_WOOD, 2));
  wtk_vgrad(0, 140, WTK_W, 60, R_CONCRETE, 13, 23);
  wtk_hline(0, WTK_W - 1, 140, wtk_shade(R_CONCRETE, 5));
  wtk_hline(0, WTK_W - 1, 141, wtk_shade(R_CONCRETE, 9));
  /* The window, with the day outside it and the light it throws on the floor. */
  wtk_rect(18, 26, 64, 52, wtk_shade(R_WHITE, 22));
  wtk_vgrad(21, 29, 58, 32, R_SKY, 20, 29);
  wtk_vgrad(21, 61, 58, 14, R_GRASS, 24, 15);
  wtk_rect(49, 26, 2, 52, wtk_shade(R_WHITE, 24));
  wtk_rect(18, 50, 64, 2, wtk_shade(R_WHITE, 24));
  wtk_hline(18, 81, 78, wtk_shade(R_WOOD, 4));
  /* The calendar. */
  wtk_rect(97, 25, 30, 38, wtk_shade(R_WOOD, 5));
  wtk_rect(96, 24, 30, 38, wtk_shade(R_CREAM, 25));
  wtk_rect(96, 24, 30, 9, wtk_shade(R_RED, 22));
  wtk_text(99, 25, "DAY", WHITE);
  wtk_text_big(103, 38, "1", wtk_shade(R_RUBBER, 6), 2);
  /* A pegboard, and a bench under it with a vice. */
  wtk_vgrad(14, 86, 70, 16, R_TAN, 14, 18);
  for (int x = 18; x < 82; x += 6) wtk_pset(x, 92, wtk_shade(R_TAN, 6));
  wtk_vgrad(12, 102, 74, 6, R_WOOD, 26, 20);
  wtk_hline(12, 85, 108, wtk_shade(R_WOOD, 4));
  wtk_vgrad(16, 109, 5, 31, R_WOOD, 10, 6);
  wtk_vgrad(78, 109, 5, 31, R_WOOD, 10, 6);
  wtk_vgrad(62, 92, 14, 10, R_CHROME, 24, 10);
  wtk_rect(66, 87, 5, 5, wtk_shade(R_CHROME, 26));
  /* The door: four panels that catch the light from above, and a row of glass. */
  wtk_rect(194, 18, 116, 123, wtk_shade(R_RUBBER, 4));
  for (int i = 0; i < 4; i++) {
    wtk_vgrad(198, 22 + i * 30, 108, 28, R_SILVER, 25, 15);
    wtk_hline(198, 305, 22 + i * 30, wtk_shade(R_WHITE, 28));
  }
  for (int i = 0; i < 4; i++) {
    wtk_rect(203 + i * 26, 56, 22, 16, wtk_shade(R_RUBBER, 8));
    wtk_vgrad(205 + i * 26, 58, 18, 12, R_SKY, 22, 29);
  }
  wtk_vgrad(244, 128, 16, 4, R_CHROME, 28, 10);

  /* The car, three-quarters on. */
  wtk_clip(0, 0, WTK_W, 200);
  wtk_cam cam = {{-FX(1) / 2, FX(2) - FX(1) / 4, -FX(8)}, 0, 0, 400, 140, 90};
  eye = cam.pos;
  wtk3_begin(&cam);
  car_mesh(&BODIES[0], 0, 0, 256 + 58, R_RED, hood_up);
  wtk3_end();
  wtk_clip_off();
}

static void panel(const char *a, const char *b, const char *c) {
  wtk_vgrad(0, 200, WTK_W, 40, R_NAVY, 6, 2);
  wtk_box(0, 200, WTK_W, 40, wtk_shade(R_CHROME, 22));
  wtk_box(1, 201, WTK_W - 2, 38, wtk_shade(R_CHROME, 9));
  wtk_text_sh(8, 206, a, wtk_shade(R_GOLD, 26), BLACK);
  wtk_text_sh(8, 217, b, WHITE, BLACK);
  wtk_text(8, 228, c, wtk_shade(R_CHROME, 17));
}

/* ---- The city --------------------------------------------------------------------------------
 * Streets run north and south and east and west, one every PITCH metres, ROAD wide. What is
 * between four streets is a block, and what stands on a block comes from its number, so the city
 * goes on for ever and is the same every time. */

#define PITCH 64
#define ROAD 8 /* half the width of a street */
#define VIEW_H 168
#define SEE 5 /* blocks drawn each way from the car */
#define N_TRAFFIC 8

static fx car_x, car_z, speed, steer;
static int car_yaw, crash_ticks, dents, miles_cm;
typedef struct { i32 street; fx along; fx speed; int ns; u8 paint, body; } other;
static other traffic[N_TRAFFIC];

static u32 hash2(int a, int b) {
  u32 h = (u32)a * 0x9E3779B1u ^ (u32)b * 0x85EBCA77u;
  h ^= h >> 15; h *= 0x2C1B3C6Du; h ^= h >> 12;
  return h;
}

/* What stands on block (bx, bz): its height in metres (0 for a park) and what it is built of. */
static int building(int bx, int bz, int *ramp) {
  static const u8 walls[8] = {R_CONCRETE, R_BRICK, R_TAN, R_CREAM, R_GREY, R_BROWN, R_SILVER, R_MAROON};
  u32 h = hash2(bx, bz);
  if (h % 7 == 0) return 0;
  *ramp = walls[(h >> 8) % 8];
  return 8 + (int)((h >> 16) % 9) * 4;
}

static void drive_start(void) {
  static const u8 paints[8] = {R_BLUE, R_SILVER, R_WHITE, R_GREEN, R_GOLD, R_TEAL, R_MAROON, R_ORANGE};
  car_x = FX(4); car_z = FX(0); car_yaw = 0; speed = 0; steer = 0; crash_ticks = 0;
  for (int i = 0; i < N_TRAFFIC; i++) {
    u32 r = wtk_rand(&rng);
    traffic[i].ns = i & 1;
    traffic[i].street = (i32)(r % 7) - 3;
    traffic[i].along = FX((i32)((r >> 8) % 600) - 300);
    traffic[i].speed = FX(8 + (i32)((r >> 20) % 8));
    traffic[i].paint = paints[(r >> 4) % 8];
    traffic[i].body = (u8)((r >> 28) % 4);
  }
}

static v3 at(int x, int y, int z) { v3 p = {FX(x), FX(y), FX(z)}; return p; }

/* Where one of the other cars is: on the right of its street, going its way. */
static void other_at(const other *o, fx *x, fx *z) {
  fx lane = FX(o->street * PITCH) + (o->ns ? FX(4) : FX(-4));
  if (o->ns) { *x = lane; *z = o->along; } else { *x = o->along; *z = lane; }
}

/* The walls of a box that face the car. */
static void box(fx x0, fx z0, fx x1, fx z1, fx y0, fx y1, int ramp, int bias) {
  if (car_z < z0) { v3 q[4] = {{x0, y0, z0}, {x1, y0, z0}, {x1, y1, z0}, {x0, y1, z0}}; wtk3_lit(q, 4, ramp, bias); }
  if (car_z > z1) { v3 q[4] = {{x0, y0, z1}, {x1, y0, z1}, {x1, y1, z1}, {x0, y1, z1}}; wtk3_lit(q, 4, ramp, bias); }
  if (car_x < x0) { v3 q[4] = {{x0, y0, z0}, {x0, y0, z1}, {x0, y1, z1}, {x0, y1, z0}}; wtk3_lit(q, 4, ramp, bias); }
  if (car_x > x1) { v3 q[4] = {{x1, y0, z0}, {x1, y0, z1}, {x1, y1, z1}, {x1, y1, z0}}; wtk3_lit(q, 4, ramp, bias); }
}

static int floor_div(int a, int b) { return a >= 0 ? a / b : -((-a + b - 1) / b); }

static int blocked(fx x, fx z) {
  /* A street is clear; a block with something on it is not, 4 metres in from the kerb. */
  int mx = FX_INT(x), mz = FX_INT(z);
  int bx = floor_div(mx, PITCH), bz = floor_div(mz, PITCH);
  int lx = mx - bx * PITCH, lz = mz - bz * PITCH;
  if (lx < ROAD + 3 || lx > PITCH - ROAD - 3 || lz < ROAD + 3 || lz > PITCH - ROAD - 3) return 0;
  int ramp;
  return building(bx, bz, &ramp) > 0;
}

static void drive_tick(void) {
  if (wtk_hit & KEY_B) { screen = GARAGE; wtk_tone(0, 0); return; }
  if (crash_ticks > 0) { crash_ticks--; return; }

  /* The pedals. It pulls hardest from rest and runs out of breath near 55 metres a second. */
  if (wtk_keys & KEY_UP) speed += fx_mul(FX(5) / WTK_HZ, FX(1) - fx_div(speed, FX(56)));
  else speed -= FX(1) / WTK_HZ;
  if (wtk_keys & KEY_DOWN) speed -= FX(11) / WTK_HZ;
  if (speed < 0) speed = 0;

  /* The wheel eases over and eases back; the faster the car, the less it turns. */
  fx want = (wtk_keys & KEY_LEFT) ? -FX(1) : (wtk_keys & KEY_RIGHT) ? FX(1) : 0;
  steer += (want - steer) / 4;
  fx grip = speed > FX(12) ? FX(1) : fx_div(speed, FX(12));
  fx turn = fx_mul(fx_mul(steer, grip), FX(9) - speed / 9);
  car_yaw = (car_yaw + FX_INT(turn + (turn < 0 ? -32768 : 32768)) + WTK_TURN) & (WTK_TURN - 1);

  fx nx = car_x + fx_mul(fx_sin(car_yaw), speed) / WTK_HZ;
  fx nz = car_z + fx_mul(fx_cos(car_yaw), speed) / WTK_HZ;
  int hit = blocked(nx + fx_sin(car_yaw) * 2, nz + fx_cos(car_yaw) * 2);

  for (int i = 0; i < N_TRAFFIC; i++) {
    other *o = &traffic[i];
    o->along += o->speed / WTK_HZ * (o->ns ? 1 : -1);
    /* They stay within a few blocks of you, one way or the other. */
    fx mine = o->ns ? car_z : car_x;
    if (o->along - mine > FX(SEE * PITCH)) o->along -= FX(2 * SEE * PITCH);
    if (mine - o->along > FX(SEE * PITCH)) o->along += FX(2 * SEE * PITCH);
    fx ox, oz;
    other_at(o, &ox, &oz);
    fx dx = nx - ox, dz = nz - oz;
    if (dx < 0) dx = -dx;
    if (dz < 0) dz = -dz;
    if (o->ns ? (dx < FX(2) && dz < FX(7) / 2) : (dx < FX(7) / 2 && dz < FX(2))) hit = 1;
  }

  if (hit) {
    if (speed > FX(3)) { dents++; crash_ticks = WTK_HZ; wtk_tone(70, 500); }
    speed = 0;
  } else {
    miles_cm += FX_INT(speed * 100 / WTK_HZ);
    car_x = nx; car_z = nz;
    /* The engine note: up through each gear and down again at the change. */
    int v = FX_INT(speed * 10), gear = v / 110, in = v % 110;
    wtk_tone(speed > 0 || (wtk_keys & KEY_UP) ? 60 + in + gear * 12 : 45, 60);
  }
}

static void dial(int cx, int cy, int r, int value, int top, const char *label) {
  wtk_disc(cx, cy, r + 2, wtk_shade(R_CHROME, 12));
  wtk_disc(cx, cy, r + 1, wtk_shade(R_CHROME, 26));
  wtk_disc(cx, cy, r, wtk_shade(R_RUBBER, 3));
  for (int i = 0; i <= 16; i++) {
    int a = -384 + i * 48, in = (i & 1) ? r - 3 : r - 6;
    wtk_line(cx + FX_INT(fx_sin(a) * in), cy - FX_INT(fx_cos(a) * in), cx + FX_INT(fx_sin(a) * (r - 1)), cy - FX_INT(fx_cos(a) * (r - 1)), i >= 13 && top == 80 ? wtk_shade(R_RED, 24) : wtk_shade(R_WHITE, 24));
  }
  int a = -384 + value * 768 / top;
  wtk_line(cx, cy, cx + FX_INT(fx_sin(a) * (r - 5)), cy - FX_INT(fx_cos(a) * (r - 5)), wtk_shade(R_ORANGE, 26));
  wtk_line(cx + 1, cy, cx + 1 + FX_INT(fx_sin(a) * (r - 5)), cy - FX_INT(fx_cos(a) * (r - 5)), wtk_shade(R_ORANGE, 20));
  wtk_disc(cx, cy, 3, wtk_shade(R_CHROME, 18));
  wtk_text(cx - wtk_strlen(label) * 4, cy + r - 13, label, wtk_shade(R_CHROME, 20));
}

static void drive_draw(void) {
  int horizon = 76;
  wtk_vgrad(0, 0, WTK_W, horizon, R_SKY, 15, 29);
  wtk_vgrad(0, horizon, WTK_W, VIEW_H - horizon, R_CONCRETE, 17, 24);
  wtk_clip(0, 0, WTK_W, VIEW_H);

  wtk_cam cam = {{car_x, FX(1) + FX(1) / 8, car_z}, car_yaw, 0, 200, WTK_W / 2, horizon};
  eye = cam.pos;
  wtk3_begin(&cam);

  col tar = wtk_shade(R_ASPHALT, 15), tar2 = wtk_shade(R_ASPHALT, 16), paint = wtk_shade(R_YELLOW, 23);
  int cbx = floor_div(FX_INT(car_x), PITCH), cbz = floor_div(FX_INT(car_z), PITCH);
  for (int i = -SEE; i <= SEE + 1; i++) {
    for (int j = -SEE; j <= SEE; j++) {
      /* A length of street going north, and one going east, each a block long, with dashes down the middle. */
      int sx = (cbx + i) * PITCH, sz = (cbz + j) * PITCH;
      v3 ns[4] = {at(sx - ROAD, 0, sz), at(sx + ROAD, 0, sz), at(sx + ROAD, 0, sz + PITCH), at(sx - ROAD, 0, sz + PITCH)};
      wtk3_poly(ns, 4, tar, tar2, -4000);
      int ez = (cbz + i) * PITCH, ex = (cbx + j) * PITCH;
      v3 ew[4] = {at(ex, 0, ez - ROAD), at(ex + PITCH, 0, ez - ROAD), at(ex + PITCH, 0, ez + ROAD), at(ex, 0, ez + ROAD)};
      wtk3_poly(ew, 4, tar, tar2, -4000);
      if (i >= -3 && i <= 3 && j >= -3 && j <= 3) {
        for (int d = 0; d < 2; d++) {
          int a = 16 + d * 24;
          v3 dn[4] = {at(sx, 0, sz + a), at(sx, 0, sz + a + 8), at(sx, 0, sz + a + 8), at(sx, 0, sz + a)};
          dn[0].x -= FX(1) / 5; dn[1].x -= FX(1) / 5; dn[2].x += FX(1) / 5; dn[3].x += FX(1) / 5;
          wtk3_poly(dn, 4, paint, paint, -3000);
          v3 de[4] = {at(ex + a, 0, ez), at(ex + a + 8, 0, ez), at(ex + a + 8, 0, ez), at(ex + a, 0, ez)};
          de[0].z -= FX(1) / 5; de[1].z -= FX(1) / 5; de[2].z += FX(1) / 5; de[3].z += FX(1) / 5;
          wtk3_poly(de, 4, paint, paint, -3000);
        }
      }
    }
  }
  for (int i = -SEE; i <= SEE; i++)
    for (int j = -SEE; j <= SEE; j++) {
      int bx = cbx + i, bz = cbz + j, ramp = 0;
      int h = building(bx, bz, &ramp);
      int x0 = bx * PITCH + ROAD + 4, z0 = bz * PITCH + ROAD + 4, x1 = (bx + 1) * PITCH - ROAD - 4, z1 = (bz + 1) * PITCH - ROAD - 4;
      if (!h) {
        /* A park: grass, and nothing to hit. */
        v3 g[4] = {at(x0, 0, z0), at(x1, 0, z0), at(x1, 0, z1), at(x0, 0, z1)};
        wtk3_poly(g, 4, wtk_shade(R_GRASS, 20), wtk_shade(R_GRASS, 22), -3500);
        continue;
      }
      box(FX(x0), FX(z0), FX(x1), FX(z1), 0, FX(h), ramp, 0);
      /* Rows of glass, a storey at a time, on the ones near enough to see them. */
      int near = i >= -1 && i <= 1 && j >= -1 && j <= 1, mid = i >= -3 && i <= 3 && j >= -3 && j <= 3;
      if (mid)
        for (int f = 0; f * 4 + 4 <= h && f < (near ? 9 : 3); f++)
          box(FX(x0) - 3000, FX(z0) - 3000, FX(x1) + 3000, FX(z1) + 3000, FX(f * 4 + 1) + FX(1) / 2, FX(f * 4 + 3), R_GLASS, 1);
      /* A doorway at street level, and a cap round the top. */
      box(FX(x0) - 2000, FX(z0) - 2000, FX(x1) + 2000, FX(z1) + 2000, FX(h) - FX(1) / 2, FX(h), R_RUBBER, 1);
    }
  for (int i = 0; i < N_TRAFFIC; i++) {
    fx ox, oz;
    other_at(&traffic[i], &ox, &oz);
    fx dx = ox - car_x, dz = oz - car_z;
    if (dx < 0) dx = -dx;
    if (dz < 0) dz = -dz;
    if (dx + dz < FX(110)) car_mesh(&BODIES[traffic[i].body], ox, oz, traffic[i].ns ? 0 : 768, traffic[i].paint, 0);
    else {
      fx hl = traffic[i].ns ? FX(1) - FX(1) / 8 : FX(2), hw = traffic[i].ns ? FX(2) : FX(1) - FX(1) / 8;
      box(ox - hl, oz - hw, ox + hl, oz + hw, FX(1) / 4, FX(1) + FX(1) / 4, traffic[i].paint, 0);
    }
  }
  wtk3_end();
  wtk_clip_off();

  /* From the driver's seat: the hood, the pillars, the mirror and the dash. */
  int lean = FX_INT(steer * 6);
  for (int k = 0; k < 6; k++) {
    int y0 = 138 + k * 5, y1 = y0 + 5, w0 = 64 + k * 14, w1 = 78 + k * 14;
    int hood[] = {160 - lean - w0, y0, 160 - lean + w0, y0, 160 - lean + w1, y1, 160 - lean - w1, y1};
    wtk_poly(hood, 4, wtk_shade(R_RED, 27 - k * 2), wtk_shade(R_RED, 26 - k * 2));
  }
  wtk_hline(160 - lean - 64, 160 - lean + 64, 138, wtk_shade(R_RED, 30));
  int lp[] = {0, 0, 16, 0, 5, VIEW_H, 0, VIEW_H}, rp[] = {303, 0, 319, 0, 319, VIEW_H, 314, VIEW_H};
  wtk_poly(lp, 4, wtk_shade(R_RUBBER, 3), wtk_shade(R_RUBBER, 3));
  wtk_poly(rp, 4, wtk_shade(R_RUBBER, 3), wtk_shade(R_RUBBER, 3));
  wtk_rect(0, 0, WTK_W, 6, wtk_shade(R_RUBBER, 3));
  wtk_rect(130, 6, 60, 16, wtk_shade(R_RUBBER, 3));
  wtk_vgrad(132, 8, 56, 12, R_SKY, 18, 26);
  wtk_vgrad(0, VIEW_H, WTK_W, 6, R_RUBBER, 14, 6);
  wtk_vgrad(0, VIEW_H + 6, WTK_W, WTK_H - VIEW_H - 6, R_RUBBER, 9, 3);
  wtk_vgrad(52, VIEW_H + 4, 216, 5, R_RUBBER, 20, 10);
  int mph = (int)(((i64)speed * 2237 / 1000) >> 16);
  int v = FX_INT(speed * 10), in = v % 110;
  dial(52, 206, 27, mph > 140 ? 140 : mph, 140, "MPH");
  dial(268, 206, 27, speed > 0 ? 15 + in * 55 / 110 : 10, 80, "RPM");
  wtk_rect(96, 180, 44, 12, BLACK);
  wtk_box(95, 179, 46, 14, wtk_shade(R_CHROME, 12));
  wtk_num(100, 182, mph, wtk_shade(R_LIME, 26));
  wtk_rect(180, 180, 44, 12, BLACK);
  wtk_box(179, 179, 46, 14, wtk_shade(R_CHROME, 12));
  wtk_num(184, 182, miles_cm / 160934, wtk_shade(R_CHROME, 24));
  wtk_text(88, 198, "DENTS", wtk_shade(R_CHROME, 16));
  wtk_num(132, 198, dents, dents ? wtk_shade(R_RED, 26) : wtk_shade(R_CHROME, 16));
  /* The wheel. */
  int wa = FX_INT(steer * 150);
  wtk_disc(160, 238, 38, wtk_shade(R_RUBBER, 2));
  wtk_disc(160, 238, 36, wtk_shade(R_RUBBER, 12));
  wtk_disc(160, 238, 31, wtk_shade(R_RUBBER, 5));
  for (int k = -1; k <= 1; k++) {
    wtk_line(160 + k, 238, 160 + k + FX_INT(fx_sin(wa - 256) * 33), 238 - FX_INT(fx_cos(wa - 256) * 33), wtk_shade(R_RUBBER, 12));
    wtk_line(160 + k, 238, 160 + k + FX_INT(fx_sin(wa + 256) * 33), 238 - FX_INT(fx_cos(wa + 256) * 33), wtk_shade(R_RUBBER, 12));
  }
  wtk_disc(160, 238, 8, wtk_shade(R_RUBBER, 14));

  if (crash_ticks > 0) {
    wtk_vgrad(108, 50, 104, 22, R_RED, 26, 16);
    wtk_box(108, 50, 104, 22, WHITE);
    wtk_text_sh(136, 57, "CRASH!", WHITE, BLACK);
  }
}

/* ---- The game --------------------------------------------------------------------------------- */

void game_init(u32 seed) {
  rng = seed ? seed : 1;
  screen = TITLE;
  ticks = 0;
  dents = 0;
  miles_cm = 0;
  hood_up = 0;
}

void game_tick(void) {
  ticks++;
  if (screen == TITLE) {
    if (wtk_hit & KEY_A) { screen = GARAGE; wtk_tone(880, 80); }
  } else if (screen == GARAGE) {
    pointer_move();
    const spot *s = spot_under(GARAGE_SPOTS, N_GARAGE);
    if ((wtk_hit & KEY_A) && s) {
      if (s->does == DO_DRIVE) { screen = DRIVE; drive_start(); }
      if (s->does == DO_HOOD) { hood_up = !hood_up; wtk_tone(hood_up ? 330 : 220, 60); }
    }
    if (wtk_hit & KEY_B) screen = TITLE;
  } else drive_tick();
}

void game_draw(void) {
  if (screen == TITLE) title_draw();
  else if (screen == GARAGE) {
    garage_draw();
    const spot *s = spot_under(GARAGE_SPOTS, N_GARAGE);
    panel("YOUR CAR", s ? s->name : "LOOK ROUND.", s && s->does == DO_HOOD ? (hood_up ? "ENTER SHUTS IT" : "ENTER OPENS IT") : "ARROWS OR MOUSE, ENTER TO PICK");
    pointer_draw();
  } else drive_draw();
}
