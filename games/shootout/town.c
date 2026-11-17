/* The town: an endless grid of streets and blocks, driven from the seat. */
#include "game.h"

/* ---- The city --------------------------------------------------------------------------------
 * Streets run north and south and east and west, one every PITCH metres, ROAD wide. What is
 * between four streets is a block, and what stands on a block comes from its number, so the city
 * goes on for ever and is the same every time. */

#define PITCH 64
#define ROAD 8 /* half the width of a street */
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

void drive_start(void) {
  static const u8 paints[8] = {R_BLUE, R_SILVER, R_WHITE, R_GREEN, R_GOLD, R_TEAL, R_MAROON, R_ORANGE};
  car_x = FX(4); car_z = FX(0); car_yaw = 0; speed = 0; steer = 0; crash_ticks = 0;
  for (int i = 0; i < N_TRAFFIC; i++) {
    u32 r = wtk_rand(&rng);
    traffic[i].ns = i & 1;
    traffic[i].street = (i32)(r % 7) - 3;
    traffic[i].along = FX((i32)((r >> 8) % 600) - 300);
    traffic[i].speed = FX(8 + (i32)((r >> 20) % 8));
    traffic[i].paint = paints[(r >> 4) % 8];
    traffic[i].body = (u8)(B_COUPE + (r >> 28) % (N_BODIES - B_COUPE));
  }
}


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

void drive_tick(void) {
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

void drive_draw(void) {
  int horizon = 60;
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

  int v = FX_INT(speed * 10);
  cockpit_look look = {0, PLAYER_PAINT, speed > 0 ? 1500 + (v % 110) * 50 : 900, (int)(((i64)speed * 2237 / 1000) >> 16), speed > FX(8) ? 6 : 0, speed > 0 ? v / 110 + 1 : 0, FX_INT(steer * 256), 0, lamps_up};
  cockpit(&look);
  wtk_text_sh(44, 18, "DENTS", wtk_shade(R_WHITE, 26), BLACK);
  wtk_num(88, 18, dents, dents ? wtk_shade(R_RED, 27) : wtk_shade(R_WHITE, 26));

  if (crash_ticks > 0) {
    wtk_vgrad(108, 50, 104, 22, R_RED, 26, 16);
    wtk_box(108, 50, 104, 22, WHITE);
    wtk_text_sh(136, 57, "CRASH!", WHITE, BLACK);
  }
}
