/* Road to the Shootout, the 1989 way: a first slice.
 *
 * Three screens. A title. A garage you look round with a pointer, where the thing under the
 * pointer is named on the bottom lines and Enter picks it. And a city of blocks to drive through
 * from behind the wheel, with traffic to miss and walls not to hit.
 *
 * Everything on the screen is drawn here from boxes, polygons and the sixteen EGA colours.
 */
#include "../../toolkit/wtk.h"

enum { TITLE, GARAGE, DRIVE };
static int screen;
static u32 ticks, rng;

/* ---- Shapes given in centimetres, drawn at so many pixels a metre ---------------------------- */

static void shape(const i16 *cm, int n, int ox, int ground, int ppm, u8 a, u8 b) {
  int xy[16];
  for (int i = 0; i < n; i++) {
    xy[i * 2] = ox + cm[i * 2] * ppm / 100;
    xy[i * 2 + 1] = ground - cm[i * 2 + 1] * ppm / 100;
  }
  wtk_poly(xy, n, a, b);
}

/* The Bexley Sparrow, a hatchback, from the side: back bumper at x = 0, nose to the right. */
static const i16 SPARROW_BODY[] = {0, 25, 0, 95, 255, 98, 360, 86, 370, 50, 370, 25};
static const i16 SPARROW_TOP[] = {0, 95, 28, 138, 190, 142, 255, 98};
static const i16 SPARROW_GLASS[] = {45, 98, 60, 133, 185, 136, 235, 100};
static const i16 SPARROW_HOOD_UP[] = {255, 98, 262, 104, 330, 168, 324, 172};

static void sparrow(int ox, int ground, int ppm, u8 paint, int hood_up) {
  int r = 31 * ppm / 100;
  shape(SPARROW_TOP, 4, ox, ground, ppm, paint, paint);
  shape(SPARROW_GLASS, 4, ox, ground, ppm, CYAN, BLUE);
  if (hood_up) {
    /* The engine, where the hood was. */
    wtk_rect(ox + 262 * ppm / 100, ground - 96 * ppm / 100, 96 * ppm / 100, 12 * ppm / 100, BLACK);
    wtk_rect(ox + 276 * ppm / 100, ground - 104 * ppm / 100, 50 * ppm / 100, 14 * ppm / 100, LGRAY);
    wtk_rect(ox + 284 * ppm / 100, ground - 110 * ppm / 100, 30 * ppm / 100, 6 * ppm / 100, RED);
    shape(SPARROW_HOOD_UP, 4, ox, ground, ppm, paint, paint);
  }
  shape(SPARROW_BODY, 6, ox, ground, ppm, paint, paint);
  wtk_hline(ox + 2, ox + 368 * ppm / 100, ground - 55 * ppm / 100, BLACK);
  wtk_vline(ox + 150 * ppm / 100, ground - 95 * ppm / 100, ground - 30 * ppm / 100, BLACK);
  wtk_rect(ox + 362 * ppm / 100, ground - 74 * ppm / 100, 4, 5, YELLOW);
  wtk_rect(ox, ground - 78 * ppm / 100, 3, 6, LRED);
  for (int i = 0; i < 2; i++) {
    int wx = ox + (i ? 292 : 78) * ppm / 100;
    wtk_disc(wx, ground - r, r + 2, BLACK);
    wtk_disc(wx, ground - r, r, DGRAY);
    wtk_disc(wx, ground - r, r * 6 / 10, LGRAY);
    wtk_disc(wx, ground - r, 2, BLACK);
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
  if (py > 158) py = 158;
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
  wtk_clear(BLACK);
  wtk_dither(0, 0, WTK_W, 96, BLUE, BLACK);
  wtk_rect(0, 96, WTK_W, 2, YELLOW);
  wtk_dither(0, 98, WTK_W, 102, DGRAY, BLACK);
  /* A road going away. */
  int road[] = {110, 199, 210, 199, 166, 98, 154, 98};
  wtk_poly(road, 4, DGRAY, DGRAY);
  for (int i = 0; i < 6; i++) {
    int y = 104 + i * i * 3, h = 2 + i * 2;
    wtk_rect(159 - i / 2, y, 2 + i, h, YELLOW);
  }
  wtk_text_big(64, 22, "ROAD TO THE", LRED, 2);
  wtk_text_big(64, 21, "ROAD TO THE", YELLOW, 2);
  wtk_text_big(41, 47, "SHOOTOUT", LRED, 4);
  wtk_text_big(40, 45, "SHOOTOUT", WHITE, 4);
  if ((ticks / 15) & 1) wtk_text_sh(96, 170, "PRESS ENTER", WHITE, BLACK);
  wtk_text(4, 190, "A SLICE", DGRAY);
}

/* ---- The garage ------------------------------------------------------------------------------- */

static const spot GARAGE_SPOTS[] = {
  {192, 16, 116, 104, "THE GARAGE DOOR: DRIVE OUT", DO_DRIVE},
  {20, 24, 60, 44, "A WINDOW. IT IS DAYLIGHT.", DO_NOTHING},
  {96, 20, 30, 36, "THE CALENDAR", DO_NOTHING},
  {22, 82, 70, 34, "THE BENCH. NOTHING ON IT YET.", DO_NOTHING},
  {60, 84, 100, 56, "THE SPARROW: GET IN AND DRIVE", DO_DRIVE},
  {160, 96, 58, 30, "THE HOOD", DO_HOOD},
};
#define N_GARAGE (int)(sizeof GARAGE_SPOTS / sizeof GARAGE_SPOTS[0])

static void garage_draw(void) {
  /* Plank walls, a beam along the top, a concrete floor. */
  wtk_rect(0, 0, WTK_W, 120, BROWN);
  for (int x = 0; x < WTK_W; x += 16) wtk_vline(x, 8, 119, BLACK);
  for (int x = 8; x < WTK_W; x += 32) wtk_dither(x - 6, 30 + (x * 7) % 60, 12, 3, BROWN, BLACK);
  wtk_rect(0, 0, WTK_W, 8, BLACK);
  wtk_dither(0, 2, WTK_W, 4, BROWN, BLACK);
  wtk_dither(0, 120, WTK_W, 40, LGRAY, DGRAY);
  wtk_hline(0, WTK_W - 1, 120, BLACK);
  /* The window. */
  wtk_rect(20, 24, 60, 44, WHITE);
  wtk_dither(23, 27, 54, 38, LCYAN, LBLUE);
  wtk_rect(23, 52, 54, 13, GREEN);
  wtk_vline(50, 24, 67, WHITE);
  wtk_hline(20, 79, 45, WHITE);
  /* The calendar. */
  wtk_rect(96, 20, 30, 36, WHITE);
  wtk_rect(96, 20, 30, 9, RED);
  wtk_text(99, 21, "DAY", WHITE);
  wtk_text_big(103, 33, "1", BLACK, 2);
  /* The bench, with a vice on it. */
  wtk_rect(22, 96, 70, 5, YELLOW);
  wtk_hline(22, 91, 101, BLACK);
  wtk_rect(26, 102, 4, 24, BLACK);
  wtk_rect(84, 102, 4, 24, BLACK);
  wtk_rect(70, 88, 12, 8, DGRAY);
  wtk_rect(74, 84, 4, 4, LGRAY);
  /* The door: four panels and a row of glass. */
  wtk_rect(192, 16, 116, 104, BLACK);
  wtk_rect(195, 19, 110, 101, LGRAY);
  for (int i = 1; i < 4; i++) wtk_hline(195, 304, 19 + i * 25, DGRAY);
  for (int i = 0; i < 4; i++) {
    wtk_rect(201 + i * 26, 48, 20, 14, DGRAY);
    wtk_dither(202 + i * 26, 49, 18, 12, LCYAN, LBLUE);
  }
  wtk_rect(244, 110, 12, 3, DGRAY);
  /* An oil stain, a shadow, and the car. */
  wtk_dither(86, 140, 150, 8, DGRAY, BLACK);
  sparrow(60, 142, 44, LRED, hood_up);
}

static void panel(const char *a, const char *b, const char *c) {
  wtk_rect(0, 160, WTK_W, 40, BLACK);
  wtk_box(0, 160, WTK_W, 40, LGRAY);
  wtk_box(2, 162, WTK_W - 4, 36, DGRAY);
  wtk_text(8, 166, a, YELLOW);
  wtk_text(8, 177, b, WHITE);
  wtk_text(8, 188, c, LGRAY);
}

/* ---- The city --------------------------------------------------------------------------------
 * Streets run north and south and east and west, one every PITCH metres, ROAD wide. What is
 * between four streets is a block, and what stands on a block comes from its number, so the city
 * goes on for ever and is the same every time. */

#define PITCH 64
#define ROAD 8 /* half the width of a street */
#define VIEW_H 140
#define SEE 5 /* blocks drawn each way from the car */
#define N_TRAFFIC 8

static fx car_x, car_z, speed, steer;
static int car_yaw, crash_ticks, dents, miles_cm;
typedef struct { i32 street; fx along; fx speed; int ns; u8 paint; } other;
static other traffic[N_TRAFFIC];

static u32 hash2(int a, int b) {
  u32 h = (u32)a * 0x9E3779B1u ^ (u32)b * 0x85EBCA77u;
  h ^= h >> 15; h *= 0x2C1B3C6Du; h ^= h >> 12;
  return h;
}

/* What stands on block (bx, bz): its height in metres (0 for a park) and its two wall colours. */
static int building(int bx, int bz, u8 *light, u8 *dark, u8 *trim) {
  static const u8 walls[6][3] = {
    {LGRAY, DGRAY, BLACK}, {BROWN, RED, YELLOW}, {WHITE, LGRAY, BLUE}, {CYAN, BLUE, WHITE}, {YELLOW, BROWN, BLACK}, {LRED, RED, WHITE},
  };
  u32 h = hash2(bx, bz);
  if (h % 7 == 0) return 0;
  const u8 *w = walls[(h >> 8) % 6];
  *light = w[0]; *dark = w[1]; *trim = w[2];
  return 6 + (int)((h >> 16) % 9) * 4;
}

static void drive_start(void) {
  car_x = FX(4); car_z = FX(0); car_yaw = 0; speed = 0; steer = 0; crash_ticks = 0;
  for (int i = 0; i < N_TRAFFIC; i++) {
    u32 r = wtk_rand(&rng);
    traffic[i].ns = i & 1;
    traffic[i].street = (i32)(r % 7) - 3;
    traffic[i].along = FX((i32)((r >> 8) % 600) - 300);
    traffic[i].speed = FX(8 + (i32)((r >> 20) % 8));
    traffic[i].paint = (u8)(9 + (r >> 28) % 6);
  }
}

static v3 at(int x, int y, int z) { v3 p = {FX(x), FX(y), FX(z)}; return p; }

/* Where one of the other cars is: on the right of its street, going its way. */
static void other_at(const other *o, fx *x, fx *z) {
  fx lane = FX(o->street * PITCH) + (o->ns ? FX(4) : FX(-4));
  if (o->ns) { *x = lane; *z = o->along; } else { *x = o->along; *z = lane; }
}

static void box(fx x0, fx z0, fx x1, fx z1, fx y0, fx y1, u8 light, u8 dark) {
  /* Only the walls that face the car are drawn. */
  if (car_z < z0) { v3 q[4] = {{x0, y0, z0}, {x1, y0, z0}, {x1, y1, z0}, {x0, y1, z0}}; wtk3_poly(q, 4, light, light, 0); }
  if (car_z > z1) { v3 q[4] = {{x0, y0, z1}, {x1, y0, z1}, {x1, y1, z1}, {x0, y1, z1}}; wtk3_poly(q, 4, light, light, 0); }
  if (car_x < x0) { v3 q[4] = {{x0, y0, z0}, {x0, y0, z1}, {x0, y1, z1}, {x0, y1, z0}}; wtk3_poly(q, 4, dark, dark, 0); }
  if (car_x > x1) { v3 q[4] = {{x1, y0, z0}, {x1, y0, z1}, {x1, y1, z1}, {x1, y1, z0}}; wtk3_poly(q, 4, dark, dark, 0); }
}

static int floor_div(int a, int b) { return a >= 0 ? a / b : -((-a + b - 1) / b); }

static int blocked(fx x, fx z) {
  /* A street is clear; a block with something on it is not, 4 metres in from the kerb. */
  int mx = FX_INT(x), mz = FX_INT(z);
  int bx = floor_div(mx, PITCH), bz = floor_div(mz, PITCH);
  int lx = mx - bx * PITCH, lz = mz - bz * PITCH;
  if (lx < ROAD + 3 || lx > PITCH - ROAD - 3 || lz < ROAD + 3 || lz > PITCH - ROAD - 3) return 0;
  u8 a, b, c;
  return building(bx, bz, &a, &b, &c) > 0;
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

static void needle(int cx, int cy, int len, int angle, u8 c) {
  wtk_line(cx, cy, cx + FX_INT(fx_sin(angle) * len), cy - FX_INT(fx_cos(angle) * len), c);
}

static void dial(int cx, int cy, int r, int value, int top, const char *label) {
  wtk_disc(cx, cy, r, BLACK);
  for (int i = 0; i <= 8; i++) {
    int a = -384 + i * 96;
    wtk_pset(cx + FX_INT(fx_sin(a) * (r - 2)), cy - FX_INT(fx_cos(a) * (r - 2)), WHITE);
  }
  needle(cx, cy, r - 4, -384 + value * 768 / top, LRED);
  wtk_disc(cx, cy, 2, LGRAY);
  wtk_text(cx - wtk_strlen(label) * 4, cy + r - 10, label, LGRAY);
}

static void drive_draw(void) {
  int horizon = 62;
  /* Sky, a band of haze at the bottom of it, and the ground. */
  wtk_rect(0, 0, WTK_W, horizon - 8, LBLUE);
  wtk_dither(0, horizon - 8, WTK_W, 8, LBLUE, LCYAN);
  wtk_rect(0, horizon, WTK_W, VIEW_H - horizon, LGRAY);
  wtk_clip(0, 0, WTK_W, VIEW_H);

  wtk_cam cam = {{car_x, FX(1) + FX(1) / 8, car_z}, car_yaw, 190, WTK_W / 2, horizon};
  wtk3_begin(&cam);

  int cbx = floor_div(FX_INT(car_x), PITCH), cbz = floor_div(FX_INT(car_z), PITCH);
  for (int i = -SEE; i <= SEE + 1; i++) {
    for (int j = -SEE; j <= SEE; j++) {
      /* A length of street going north, and one going east, each a block long, with a dash down the middle. */
      int sx = (cbx + i) * PITCH, sz = (cbz + j) * PITCH;
      v3 ns[4] = {at(sx - ROAD, 0, sz), at(sx + ROAD, 0, sz), at(sx + ROAD, 0, sz + PITCH), at(sx - ROAD, 0, sz + PITCH)};
      wtk3_poly(ns, 4, DGRAY, DGRAY, -4000);
      int ez = (cbz + i) * PITCH, ex = (cbx + j) * PITCH;
      v3 ew[4] = {at(ex, 0, ez - ROAD), at(ex + PITCH, 0, ez - ROAD), at(ex + PITCH, 0, ez + ROAD), at(ex, 0, ez + ROAD)};
      wtk3_poly(ew, 4, DGRAY, DGRAY, -4000);
      if (i >= -3 && i <= 3 && j >= -3 && j <= 3) {
        for (int d = 0; d < 2; d++) {
          int a = 16 + d * 24;
          v3 dn[4] = {at(sx, 0, sz + a), at(sx, 0, sz + a + 8), at(sx, 0, sz + a + 8), at(sx, 0, sz + a)};
          dn[0].x -= FX(1) / 5; dn[1].x -= FX(1) / 5; dn[2].x += FX(1) / 5; dn[3].x += FX(1) / 5;
          wtk3_poly(dn, 4, YELLOW, YELLOW, -3000);
          v3 de[4] = {at(ex + a, 0, ez), at(ex + a + 8, 0, ez), at(ex + a + 8, 0, ez), at(ex + a, 0, ez)};
          de[0].z -= FX(1) / 5; de[1].z -= FX(1) / 5; de[2].z += FX(1) / 5; de[3].z += FX(1) / 5;
          wtk3_poly(de, 4, YELLOW, YELLOW, -3000);
        }
      }
    }
  }
  for (int i = -SEE; i <= SEE; i++)
    for (int j = -SEE; j <= SEE; j++) {
      int bx = cbx + i, bz = cbz + j;
      u8 light, dark, trim;
      int h = building(bx, bz, &light, &dark, &trim);
      int x0 = bx * PITCH + ROAD + 4, z0 = bz * PITCH + ROAD + 4, x1 = (bx + 1) * PITCH - ROAD - 4, z1 = (bz + 1) * PITCH - ROAD - 4;
      if (!h) {
        /* A park: grass, and nothing to hit. */
        v3 g[4] = {at(x0, 0, z0), at(x1, 0, z0), at(x1, 0, z1), at(x0, 0, z1)};
        wtk3_poly(g, 4, GREEN, LGREEN, -3500);
        continue;
      }
      box(FX(x0), FX(z0), FX(x1), FX(z1), 0, FX(h), light, dark);
      /* A row of glass round the nearer ones. */
      if (i >= -2 && i <= 2 && j >= -2 && j <= 2)
        box(FX(x0) - 3000, FX(z0) - 3000, FX(x1) + 3000, FX(z1) + 3000, FX(2), FX(3) + FX(1) / 2, trim, trim);
    }
  for (int i = 0; i < N_TRAFFIC; i++) {
    fx ox, oz;
    other_at(&traffic[i], &ox, &oz);
    fx hl = traffic[i].ns ? FX(1) - FX(1) / 8 : FX(2), hw = traffic[i].ns ? FX(2) : FX(1) - FX(1) / 8;
    box(ox - hl, oz - hw, ox + hl, oz + hw, FX(1) / 4, FX(1), traffic[i].paint, traffic[i].paint - 8);
    box(ox - hl + 8000, oz - hw + 8000, ox + hl - 8000, oz + hw - 8000, FX(1), FX(1) + FX(1) / 2, CYAN, BLUE);
  }
  wtk3_end();
  wtk_clip_off();

  /* From the driver's seat: the hood, the pillars, the mirror and the dash. */
  int lean = FX_INT(steer * 6);
  int hood[] = {40 - lean, VIEW_H, 96 - lean, 116, 224 - lean, 116, 280 - lean, VIEW_H};
  wtk_poly(hood, 4, LRED, LRED);
  wtk_hline(96 - lean, 224 - lean, 116, RED);
  int lp[] = {0, 0, 14, 0, 4, VIEW_H, 0, VIEW_H}, rp[] = {305, 0, 319, 0, 319, VIEW_H, 315, VIEW_H};
  wtk_poly(lp, 4, BLACK, BLACK);
  wtk_poly(rp, 4, BLACK, BLACK);
  wtk_rect(0, 0, WTK_W, 5, BLACK);
  wtk_rect(132, 5, 56, 14, BLACK);
  wtk_dither(134, 7, 52, 10, LBLUE, LGRAY);
  wtk_rect(0, VIEW_H, WTK_W, WTK_H - VIEW_H, BLACK);
  wtk_dither(0, VIEW_H, WTK_W, 4, DGRAY, BLACK);
  wtk_rect(8, VIEW_H + 6, WTK_W - 16, 50, DGRAY);
  int mph = (int)(((i64)speed * 2237 / 1000) >> 16);
  int v = FX_INT(speed * 10), in = v % 110;
  dial(58, 172, 24, mph > 140 ? 140 : mph, 140, "MPH");
  dial(262, 172, 24, speed > 0 ? 15 + in * 55 / 110 : 10, 80, "RPM");
  wtk_rect(100, 152, 44, 12, BLACK);
  wtk_num(104, 154, mph, LGREEN);
  wtk_rect(176, 152, 44, 12, BLACK);
  wtk_num(180, 154, miles_cm / 160934, LGRAY);
  wtk_text(84, 168, "DENTS", LGRAY);
  wtk_num(128, 168, dents, dents ? LRED : LGRAY);
  /* The wheel. */
  int wa = FX_INT(steer * 150);
  wtk_disc(160, 196, 30, BLACK);
  wtk_disc(160, 196, 25, DGRAY);
  wtk_line(160, 196, 160 + FX_INT(fx_sin(wa - 256) * 27), 196 - FX_INT(fx_cos(wa - 256) * 27), BLACK);
  wtk_line(160, 196, 160 + FX_INT(fx_sin(wa + 256) * 27), 196 - FX_INT(fx_cos(wa + 256) * 27), BLACK);
  wtk_disc(160, 196, 6, BLACK);

  if (crash_ticks > 0) {
    wtk_rect(112, 44, 96, 20, RED);
    wtk_box(112, 44, 96, 20, WHITE);
    wtk_text(136, 50, "CRASH!", WHITE);
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
    panel("1984 BEXLEY SPARROW", s ? s->name : "LOOK ROUND.", s && s->does == DO_HOOD ? (hood_up ? "ENTER SHUTS IT" : "ENTER OPENS IT") : "ARROWS OR MOUSE, ENTER TO PICK");
    pointer_draw();
  } else drive_draw();
}
