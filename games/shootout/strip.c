#include "game.h"

/* ---- The strip, at night ----------------------------------------------------------------------
 * A quarter mile from behind the wheel. Hold the throttle to bring the revs and the boost up
 * against the limiter, shift into first on the green, and shift up before the redline.
 *
 * The engine is a turbo two-litre, and its torque at each engine speed is read off a real dyno
 * sheet (Rix Racing, Masontown WV: 314 lb-ft at the wheels, 292 hp), at full boost. */

enum { S_STAGE, S_TREE, S_RUN, S_SLIP };
#define QUARTER 402
#define REDLINE 7200
#define TREE_STEP 15
static int s_phase, s_gear, s_rpm, s_launch, s_shift, s_clock, s_tree, s_red, s_rt, s_sixty, s_et, s_mph;
static fx s_pos, s_v, s_boost;
static int best, purse;

static const i16 TORQUE[][2] = {
  {800, 60}, {2000, 105}, {3000, 146}, {3400, 190}, {3800, 250}, {4200, 305}, {4400, 314}, {4600, 310},
  {5000, 300}, {5400, 285}, {5800, 270}, {6200, 250}, {6450, 240}, {7200, 205}, {8000, 150},
};
static const i16 GEARS[6] = {0, 310, 185, 125, 92, 72}; /* in hundredths, over a 4.2 final drive */

static int torque_at(int rpm) {
  int n = (int)(sizeof TORQUE / sizeof TORQUE[0]);
  if (rpm <= TORQUE[0][0]) return TORQUE[0][1];
  for (int i = 1; i < n; i++)
    if (rpm <= TORQUE[i][0])
      return TORQUE[i - 1][1] + (TORQUE[i][1] - TORQUE[i - 1][1]) * (rpm - TORQUE[i - 1][0]) / (TORQUE[i][0] - TORQUE[i - 1][0]);
  return TORQUE[n - 1][1];
}

void strip_start(void) {
  s_phase = S_STAGE; s_gear = 0; s_rpm = 900; s_launch = 0; s_shift = 0; s_clock = 0; s_tree = 0; s_red = 0;
  s_rt = 0; s_sixty = 0; s_et = 0; s_mph = 0; s_pos = 0; s_v = 0; s_boost = 0;
}

void strip_tick(void) {
  if (wtk_hit & KEY_B) { screen = GARAGE; wtk_tone(0, 0); return; }
  if (s_phase == S_SLIP) {
    /* Past the line: off the throttle and on the brakes. */
    s_v -= FX(8) / WTK_HZ;
    if (s_v < 0) s_v = 0;
    s_pos += s_v / WTK_HZ;
    s_rpm += (900 - s_rpm) / 8;
    wtk_tone(s_v > 0 ? s_rpm / 40 : 0, 60);
    if (wtk_hit & KEY_A) screen = GARAGE;
    return;
  }
  if (s_phase == S_STAGE && (wtk_hit & KEY_A)) { s_phase = S_TREE; s_tree = 0; wtk_tone(660, 60); }
  else if (s_phase == S_TREE) s_tree++;
  int green = s_phase == S_TREE && s_tree >= TREE_STEP * 3;
  if (s_phase == S_RUN || green) s_clock++;

  int throttle = (wtk_keys & KEY_UP) != 0;
  if (wtk_hit & KEY_X) {
    if (s_gear == 0 && s_phase != S_STAGE) {
      /* Into first: that is the launch, and before the green it is a red light. */
      s_gear = 1;
      s_launch = s_rpm;
      s_red = !green;
      s_rt = green ? (s_tree - TREE_STEP * 3) * 1000 / WTK_HZ : 0;
      s_phase = S_RUN;
      if (s_red) s_clock = 0;
    } else if (s_gear > 0 && s_gear < 5) { s_gear++; s_shift = 5; }
  }
  if ((wtk_hit & KEY_Y) && s_gear > 1) { s_gear--; s_shift = 5; }

  if (s_gear == 0) {
    /* In neutral the revs go where your foot puts them, up to the launch limiter. */
    int want = throttle ? 5600 : 900;
    s_rpm += (want - s_rpm) / 4;
  } else {
    int road = (int)(((i64)s_v * GEARS[s_gear] * 1337 / 1000) >> 16);
    /* The clutch slips the revs down to meet the road, and a hard launch keeps them up on the boost. */
    int hold = s_launch * 3 / 4 > 1500 ? s_launch * 3 / 4 : 1500;
    int slip = s_gear == 1 && throttle ? (s_rpm - 60 > hold ? s_rpm - 60 : hold) : 900;
    s_rpm = road > slip ? road : slip;
  }
  /* The big turbo makes more, but it wakes up later and takes longer about it. */
  int big = tune_big_turbo(), from = big ? 3000 : 2200, most = big ? 27 : 20;
  fx want = throttle ? FX(s_rpm < from ? 0 : s_rpm > from + 2200 ? most : (s_rpm - from) * most / 2200) : 0;
  s_boost += (want - s_boost) / (big ? 10 : 6);

  if (s_gear > 0) {
    int t = torque_at(s_rpm);
    fx push = 0;
    if (s_shift > 0) s_shift--;
    else if (throttle && s_rpm < REDLINE) {
      /* Off boost it makes under half of what it makes on it. */
      fx part = FX(45) / 100 + fx_mul(FX(55) / 100, s_boost / 20);
      push = fx_mul((fx)((i64)t * GEARS[s_gear] * 858 / 100 * tune_gain() / 100), part);
      if (push > FX(12)) push = FX(12); /* all four tyres, and no more than they will take */
    } else push = -FX(1);
    push -= fx_mul(fx_mul(s_v, s_v), 19) + FX(15) / 100;
    s_v += push / WTK_HZ;
    if (s_v < 0) s_v = 0;
    fx before = s_pos;
    s_pos += s_v / WTK_HZ;
    int now = s_clock * 1000 / WTK_HZ;
    if (!s_sixty && s_pos >= FX(183) / 10) s_sixty = now;
    if (before < FX(QUARTER) && s_pos >= FX(QUARTER)) {
      s_et = now - (s_v > 0 ? (int)(((i64)(s_pos - FX(QUARTER)) * 1000) / s_v) : 0);
      s_mph = (int)(((i64)s_v * 2237 / 1000) >> 16);
      s_phase = S_SLIP;
      /* A clean pass pays a little, and a new best pays more. */
      purse = s_red ? 0 : 25;
      if (!s_red && (!best || s_et < best)) { if (best) purse += 150; best = s_et; }
      cash += purse;
      wtk_tone(990, 200);
      return;
    }
  }
  wtk_tone(s_rpm / 40, 60);
}

static void ms(int x, int y, int t, col c) {
  x = wtk_num(x, y, t / 1000, c);
  x = wtk_text(x, y, ".", c);
  x = wtk_num(x, y, t / 100 % 10, c);
  x = wtk_num(x, y, t / 10 % 10, c);
  wtk_num(x, y, t % 10, c);
}

void strip_draw(void) {
  int horizon = 62;
  wtk_vgrad(0, 0, WTK_W, horizon, R_NAVY, 1, 7);
  wtk_vgrad(0, horizon, WTK_W, VIEW_H - horizon, R_ASPHALT, 5, 10);
  wtk_clip(0, 0, WTK_W, VIEW_H);
  wtk_cam cam = {{-FX(3), FX(1) + FX(1) / 8, s_pos}, 0, 0, 200, WTK_W / 2, horizon};
  eye = cam.pos;
  wtk3_begin(&cam);

  int here = FX_INT(s_pos) / 20 * 20;
  for (int z = here - 20; z < here + 440; z += 20) {
    /* Two lanes with a dark groove of rubber down each, a wall either side, and the stands behind. */
    int lit = z < 80 ? 13 : 10;
    for (int k = -1; k <= 1; k += 2) {
      v3 lane[4] = {at(k * 6, 0, z), at(0, 0, z), at(0, 0, z + 20), at(k * 6, 0, z + 20)};
      wtk3_poly(lane, 4, wtk_shade(R_ASPHALT, lit), wtk_shade(R_ASPHALT, lit + 1), -4000);
      v3 groove[4] = {at(k * 4, 0, z), at(k * 2, 0, z), at(k * 2, 0, z + 20), at(k * 4, 0, z + 20)};
      wtk3_poly(groove, 4, wtk_shade(R_RUBBER, lit - 3), wtk_shade(R_ASPHALT, lit - 3), -3900);
      v3 wall[4] = {at(k * 7, 0, z), at(k * 7, 1, z), at(k * 7, 1, z + 20), at(k * 7, 0, z + 20)};
      int board = (z / 20) & 3;
      int ramp = board == 0 ? R_BLUE : board == 2 ? R_RED : R_CONCRETE;
      wtk3_poly(wall, 4, wtk_shade(ramp, 12), wtk_shade(ramp, 13), -3000);
      if (z >= 0 && z < 200 && (z / 20) % 2 == 0) {
        v3 stand[4] = {at(k * 14, 0, z), at(k * 26, 9, z), at(k * 26, 9, z + 40), at(k * 14, 0, z + 40)};
        wtk3_poly(stand, 4, wtk_shade(R_CHROME, 5), wtk_shade(R_CHROME, 7), -3500);
      }
    }
    v3 mid[4] = {at(0, 0, z), at(0, 0, z + 20), at(0, 0, z + 20), at(0, 0, z)};
    mid[0].x -= FX(1) / 8; mid[1].x -= FX(1) / 8; mid[2].x += FX(1) / 8; mid[3].x += FX(1) / 8;
    wtk3_poly(mid, 4, wtk_shade(R_WHITE, 20), wtk_shade(R_WHITE, 20), -3800);
  }
  for (int i = 0; i < 2; i++) {
    /* The start line and the finish line. */
    int z = i ? QUARTER : 0;
    v3 line[4] = {at(-6, 0, z), at(6, 0, z), at(6, 0, z), at(-6, 0, z)};
    line[2].z += FX(1) / 2; line[3].z += FX(1) / 2;
    wtk3_poly(line, 4, wtk_shade(R_WHITE, 26), wtk_shade(R_WHITE, 26), -3700);
  }
  wtk3_end();

  /* The floodlights: a pole, a hot spot, and the flare it makes on a dirty windscreen. */
  for (int z = 20; z <= 440; z += 60)
    for (int k = -1; k <= 1; k += 2) {
      int bx, by, tx, ty;
      fx depth;
      if (!wtk3_point(at(k * 11, 0, z), &bx, &by, &depth) || !wtk3_point(at(k * 11, 15, z), &tx, &ty, 0)) continue;
      int r = (int)(((i64)200 * 45000) / depth);
      wtk_line(bx, by, tx, ty, wtk_shade(R_CHROME, 6));
      for (int f = 1; f <= 3; f++) {
        int len = r * 3 + 3 - f;
        wtk_line(tx - len, ty, tx + len, ty, wtk_shade(R_WHITE, 12 + f * 3));
        wtk_line(tx, ty - len, tx, ty + len, wtk_shade(R_WHITE, 12 + f * 3));
      }
      wtk_disc(tx, ty, r + 1, wtk_shade(R_CREAM, 28));
      wtk_disc(tx, ty, r, wtk_shade(R_WHITE, 31));
    }

  /* The tree, between the lanes just past the line. */
  int sx, sy, ex, ey;
  fx depth;
  if (wtk3_point(at(0, 0, 12), &sx, &sy, &depth) && wtk3_point(at(0, 3, 12), &ex, &ey, 0)) {
    int unit = (int)(((i64)200 * 65536) / depth); /* pixels to the metre, there */
    int r = unit * 12 / 100 + 1;
    wtk_rect(sx - 1, ey, 3, sy - ey, wtk_shade(R_CHROME, 8));
    wtk_rect(sx - unit * 45 / 100, ey, unit * 90 / 100 + 1, unit * 2, wtk_shade(R_RUBBER, 3));
    int step = s_phase == S_TREE ? s_tree / TREE_STEP : s_phase >= S_RUN ? 3 : -1;
    for (int row = 0; row < 6; row++) {
      int y = ey + unit * (15 + row * 30) / 100;
      int on = row == 0 ? s_phase != S_STAGE : row <= 3 ? step == row - 1 : row == 4 ? step >= 3 && !s_red : s_red;
      int ramp = row == 0 ? R_YELLOW : row <= 3 ? R_ORANGE : row == 4 ? R_LIME : R_RED;
      for (int k = -1; k <= 1; k += 2) wtk_disc(sx + k * unit * 22 / 100, y, row == 0 ? (r + 1) / 2 : r, wtk_shade(ramp, on ? 29 : 5));
    }
  }
  wtk_clip_off();

  cockpit_look look = {1, PLAYER_PAINT, s_rpm, (int)(((i64)s_v * 2237 / 1000) >> 16), FX_INT(s_boost), s_gear, 0, s_rpm > 6500, 1};
  cockpit(&look);

  const char *say = s_phase == S_STAGE ? "ENTER TO STAGE" : s_phase == S_TREE && s_gear == 0 ? "HOLD W FOR BOOST - E ON GREEN" : s_red ? "RED LIGHT" : "";
  wtk_text_sh(160 - wtk_strlen(say) * 4, 12, say, s_red ? wtk_shade(R_RED, 28) : WHITE, BLACK);

  if (s_phase == S_SLIP) {
    /* The timeslip. */
    wtk_rect(93, 31, 136, 112, wtk_shade(R_RUBBER, 2));
    wtk_vgrad(90, 28, 136, 112, R_CREAM, 27, 23);
    logo(110, 34, wtk_shade(R_RUBBER, 3), wtk_shade(R_RED, 24));
    wtk_hline(96, 219, 60, wtk_shade(R_BROWN, 12));
    col ink = wtk_shade(R_RUBBER, 4);
    wtk_text(98, 66, "LEFT LANE", ink);
    wtk_text(98, 80, "R/T", ink);
    if (s_red) wtk_text(162, 80, "RED", wtk_shade(R_RED, 22)); else ms(162, 80, s_rt, ink);
    wtk_text(98, 91, "60 FT", ink); ms(162, 91, s_sixty, ink);
    wtk_text(98, 102, "1/4 ET", ink); ms(162, 102, s_et, ink);
    wtk_text(98, 113, "MPH", ink); wtk_num(162, 113, s_mph, ink);
    int tx = wtk_text(98, 126, "PURSE $", wtk_shade(R_FOREST, 14));
    wtk_num(tx, 126, purse, wtk_shade(R_FOREST, 14));
    if (purse > 25) wtk_text(162, 126, "BEST!", wtk_shade(R_RED, 22));
  }
}
