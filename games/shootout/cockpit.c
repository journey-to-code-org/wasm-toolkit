/* The view from the seat.
 *
 * A dash that wraps round the driver: a hooded cluster dead ahead, seen over the top of the
 * wheel, and a centre stack that is turned to face you, with the vents, the radio, the heater
 * and the shifter in it. By day the dials are white on black; at night they are lit red. */
#include "game.h"

static int lit; /* whether it is night */
static col ink(int bright) { return lit ? wtk_shade(R_ORANGE, bright) : wtk_shade(R_WHITE, bright); }

/* A dial: `marks` divisions round 280 degrees, numbered every other one in steps of `each`. */
static void gauge(int cx, int cy, int r, int value, int top, int marks, int each, int red_from, const char *label) {
  wtk_disc(cx, cy, r + 2, wtk_shade(R_RUBBER, 12));
  wtk_disc(cx, cy, r + 1, wtk_shade(R_RUBBER, 3));
  wtk_disc(cx, cy, r, lit ? wtk_shade(R_MAROON, 3) : wtk_shade(R_RUBBER, 1));
  for (int i = 0; i <= marks * 2; i++) {
    int a = -400 + i * 800 / (marks * 2), in = (i & 1) ? r - 3 : r - 5;
    col c = i / 2 >= red_from ? wtk_shade(R_RED, 27) : ink(26);
    wtk_line(cx + FX_INT(fx_sin(a) * in), cy - FX_INT(fx_cos(a) * in), cx + FX_INT(fx_sin(a) * (r - 1)), cy - FX_INT(fx_cos(a) * (r - 1)), c);
    int n = i / 2 * each;
    if (!(i & 3) && each && i > 0 && i < marks * 2) {
      wtk_num(cx + FX_INT(fx_sin(a) * (r - 11)) - (n > 99 ? 12 : n > 9 ? 8 : 4), cy - FX_INT(fx_cos(a) * (r - 11)) - 4, n, c);
    }
  }
  if (value > top) value = top;
  if (value < 0) value = 0;
  int a = -400 + value * 800 / top;
  for (int k = -1; k <= 1; k++)
    wtk_line(cx + k, cy, cx + k + FX_INT(fx_sin(a) * (r - 4)), cy - FX_INT(fx_cos(a) * (r - 4)), wtk_shade(k ? R_RED : R_ORANGE, 27 - (k != 0) * 5));
  wtk_disc(cx, cy, 2, wtk_shade(R_RUBBER, 11));
  if (label[0]) wtk_text(cx - wtk_strlen(label) * 4, cy + r - 10, label, ink(17));
}

/* A small quarter-dial: a needle over five marks. */
static void mini(int cx, int cy, int value) {
  wtk_rect(cx - 13, cy - 8, 27, 12, lit ? wtk_shade(R_MAROON, 3) : wtk_shade(R_RUBBER, 1));
  for (int i = 0; i <= 4; i++) wtk_pset(cx - 8 + i * 4, cy - 6 + (i == 0 || i == 4), ink(24));
  int a = -150 + value * 300 / 100;
  wtk_line(cx, cy + 3, cx + FX_INT(fx_sin(a) * 9), cy + 3 - FX_INT(fx_cos(a) * 9), wtk_shade(R_ORANGE, 27));
}

void cockpit(const cockpit_look *k) {
  lit = k->night;
  int d = k->night ? -17 : 0; /* how much darker everything is */
  int day = k->night ? 0 : 5; /* and how much lighter the trim is by day */
  col black = wtk_shade(R_RUBBER, 2);

  /* The hood, through the glass: lit from above, with a crease down each side of it. */
  for (int i = 0; i < 6; i++) {
    int y0 = 106 + i * 5, w0 = 78 + i * 15, w1 = 93 + i * 15;
    int hood[] = {150 - w0, y0, 150 + w0, y0, 150 + w1, y0 + 5, 150 - w1, y0 + 5};
    wtk_poly(hood, 4, wtk_shade(k->paint, 27 - i * 2 + d), wtk_shade(k->paint, 26 - i * 2 + d));
  }
  wtk_hline(72, 228, 106, wtk_shade(k->paint, 30 + d));
  wtk_line(110, 106, 80, 136, wtk_shade(k->paint, 15 + d));
  wtk_line(190, 106, 220, 136, wtk_shade(k->paint, 15 + d));
  if (k->lamps)
    for (int s = -1; s <= 1; s += 2) {
      /* The headlamps, up: you see the backs of them. */
      int x = 150 + s * 62;
      wtk_vgrad(x - 13, 97, 26, 10, k->paint, 24 + d, 14 + d);
      wtk_hline(x - 13, x + 12, 97, wtk_shade(k->paint, 30 + d));
      if (k->night) { wtk_hline(x - 16, x + 15, 96, wtk_shade(R_CREAM, 20)); wtk_hline(x - 12, x + 11, 95, wtk_shade(R_CREAM, 12)); }
    }

  /* The frame of the windscreen: a thick pillar by your shoulder, a thin one across the car, the
     header, the mirror, and the visors folded up. */
  int lp[] = {0, 0, 30, 0, 9, VIEW_H, 0, VIEW_H}, rp[] = {296, 0, 319, 0, 319, VIEW_H, 311, VIEW_H};
  wtk_poly(lp, 4, black, black);
  wtk_poly(rp, 4, black, black);
  wtk_line(30, 0, 9, VIEW_H, wtk_shade(R_RUBBER, 12 + d / 2));
  wtk_line(296, 0, 311, VIEW_H, wtk_shade(R_RUBBER, 10 + d / 2));
  wtk_vgrad(0, 0, WTK_W, 7, R_RUBBER, 2, 6 + d / 3);
  wtk_vgrad(34, 7, 88, 5, R_RUBBER, 8 + d / 3, 4);
  wtk_vgrad(208, 7, 84, 5, R_RUBBER, 8 + d / 3, 4);
  wtk_rect(157, 7, 4, 5, black);
  wtk_rect(130, 11, 58, 15, black);
  wtk_vgrad(132, 13, 54, 11, k->night ? R_NAVY : R_SKY, k->night ? 4 : 16, k->night ? 9 : 26);
  wtk_hline(132, 185, 13, wtk_shade(R_CHROME, 16 + d / 2));

  /* A boost gauge in a pod on the pillar. */
  wtk_disc(21, 108, 17, wtk_shade(R_RUBBER, 7));
  gauge(21, 108, 14, k->boost, 30, 6, 0, 5, "");
  wtk_text(13, 114, "PSI", ink(15));

  /* The dash: the top of it rolls away under the glass, and a hood stands up over the dials. */
  int cowl[] = {0, VIEW_H, 0, 128, 44, 123, 276, 123, 319, 128, 319, VIEW_H};
  wtk_poly(cowl, 6, wtk_shade(R_RUBBER, 9 + day + d / 3), wtk_shade(R_RUBBER, 10 + day + d / 3));
  wtk_hline(44, 276, 123, wtk_shade(R_RUBBER, 15 + d / 2));
  wtk_vgrad(0, VIEW_H, WTK_W, WTK_H - VIEW_H, R_RUBBER, 8 + day + d / 3, 2 + day);
  int brow[] = {46, 146, 58, 118, 180, 118, 192, 146};
  wtk_poly(brow, 4, wtk_shade(R_RUBBER, 6 + day + d / 4), wtk_shade(R_RUBBER, 7 + day + d / 4));
  wtk_hline(58, 180, 118, wtk_shade(R_RUBBER, 16 + d / 2));
  wtk_hline(59, 179, 119, wtk_shade(R_RUBBER, 11 + d / 3));
  /* The door, down your left side. */
  int door[] = {0, VIEW_H, 44, 142, 36, WTK_H, 0, WTK_H};
  wtk_poly(door, 4, wtk_shade(R_RUBBER, 5 + day), wtk_shade(R_RUBBER, 6 + day));
  wtk_line(44, 142, 36, WTK_H, wtk_shade(R_RUBBER, 12 + d / 2));
  wtk_vgrad(6, 176, 22, 8, R_RUBBER, 12 + d / 2, 6);

  /* The cluster: a tach and a speedometer, and three small ones between and below. */
  wtk_rect(52, 138, 134, 62, wtk_shade(R_RUBBER, 1));
  wtk_box(52, 138, 134, 62, wtk_shade(R_RUBBER, 9));
  gauge(84, 168, 26, k->rpm / 10, 900, 9, 1, 7, "");
  gauge(154, 168, 26, k->mph, 160, 8, 20, 99, "");
  wtk_text(72, 182, "RPM", ink(15));
  wtk_text(142, 182, "MPH", ink(15));
  mini(119, 150, 48);
  mini(119, 166, 62);
  mini(119, 182, 70);
  wtk_disc(119, 193, 3, wtk_shade(R_RED, k->shift_light ? 30 : 5));
  for (int i = 0; i < 4; i++) wtk_rect(60 + i * 8, 141, 5, 3, wtk_shade(i == 1 && k->lamps ? R_LIME : i == 3 && k->boost > 22 ? R_ORANGE : R_RUBBER, i == 1 && k->lamps ? 26 : i == 3 && k->boost > 22 ? 26 : 7));

  /* The centre stack, turned to face you: vents, the radio, the heater, and the shifter. */
  int stack[] = {198, 142, 300, 150, 304, WTK_H, 194, WTK_H};
  wtk_poly(stack, 4, wtk_shade(R_RUBBER, 10 + day + d / 3), wtk_shade(R_RUBBER, 11 + day + d / 3));
  wtk_line(198, 142, 194, WTK_H, wtk_shade(R_RUBBER, 16 + d / 2));
  wtk_line(198, 142, 300, 150, wtk_shade(R_RUBBER, 16 + d / 2));
  for (int v = 0; v < 2; v++) {
    int x = 206 + v * 46, y = 150 + v * 3;
    wtk_rect(x, y, 38, 13, wtk_shade(R_RUBBER, 2));
    for (int s = 0; s < 4; s++) wtk_hline(x + 1, x + 36, y + 2 + s * 3, wtk_shade(R_RUBBER, 9 + d / 3));
  }
  wtk_rect(206, 170, 86, 17, wtk_shade(R_RUBBER, 2));
  wtk_box(206, 170, 86, 17, wtk_shade(R_RUBBER, 13 + d / 2));
  wtk_rect(210, 173, 50, 11, wtk_shade(R_FOREST, 4));
  wtk_text(212, 175, "FM 105", wtk_shade(R_LIME, 25));
  wtk_disc(270, 178, 4, wtk_shade(R_RUBBER, 12));
  wtk_disc(283, 178, 4, wtk_shade(R_RUBBER, 12));
  for (int s = 0; s < 3; s++) {
    wtk_rect(208, 192 + s * 5, 52, 2, wtk_shade(R_RUBBER, 2));
    wtk_rect(216 + s * 14, 191 + s * 5, 5, 4, wtk_shade(s ? R_SKY : R_RED, 18 + d / 2));
  }
  wtk_disc(278, 198, 7, wtk_shade(R_RUBBER, 3));
  wtk_disc(278, 198, 5, wtk_shade(R_RUBBER, 12 + d / 2));
  /* The shifter: its gate, with the gear you are in lit, a boot, and the knob. */
  static const i16 gate[6][2] = {{0, 0}, {-8, -7}, {-8, 7}, {0, -7}, {0, 7}, {8, -7}};
  int gx = 276, gy = 222;
  wtk_rect(gx - 13, gy - 12, 27, 25, wtk_shade(R_RUBBER, 2));
  for (int c = -1; c <= 1; c++) wtk_vline(gx + c * 8, gy - 7, gy + (c == 1 ? 0 : 7), wtk_shade(R_RUBBER, 12));
  wtk_hline(gx - 8, gx + 8, gy, wtk_shade(R_RUBBER, 12));
  if (k->gear >= 0 && k->gear <= 5) wtk_disc(gx + gate[k->gear][0], gy + gate[k->gear][1], 2, wtk_shade(R_ORANGE, 28));
  int sx = 228 + (k->gear >= 0 && k->gear <= 5 ? gate[k->gear][0] : 0), sy = 222 + (k->gear >= 0 && k->gear <= 5 ? gate[k->gear][1] / 2 : 0);
  int boot[] = {214, WTK_H, 242, WTK_H, sx + 3, sy + 6, sx - 3, sy + 6};
  wtk_poly(boot, 4, wtk_shade(R_RUBBER, 4), wtk_shade(R_RUBBER, 5));
  wtk_disc(sx, sy, 7, wtk_shade(R_RUBBER, 3));
  wtk_disc(sx - 1, sy - 1, 5, wtk_shade(R_RUBBER, 13 + d / 2));
  wtk_disc(sx - 2, sy - 3, 1, wtk_shade(R_WHITE, 22 + d));

  /* The wheel: three spokes and a thick rim, turning with your hands. */
  int cx = 119, cy = 262, r = 62;
  for (int a = 0; a < WTK_TURN; a += 6) {
    int x = cx + FX_INT(fx_sin(a) * r), y = cy - FX_INT(fx_cos(a) * r);
    if (y < WTK_H + 4) wtk_disc(x, y, 4, wtk_shade(R_RUBBER, 5 + day));
  }
  for (int a = 0; a < WTK_TURN; a += 6) {
    int x = cx + FX_INT(fx_sin(a) * (r + 1)), y = cy - FX_INT(fx_cos(a) * (r + 1));
    if (y < WTK_H + 2) wtk_disc(x, y, 1, wtk_shade(R_RUBBER, 15 + day + d / 2));
  }
  for (int s = 0; s < 3; s++) {
    int a = k->wheel + (s == 0 ? -256 : s == 1 ? 256 : 512);
    int ox = FX_INT(fx_cos(a) * 6), oy = FX_INT(fx_sin(a) * 6);
    int ex = cx + FX_INT(fx_sin(a) * (r - 3)), ey = cy - FX_INT(fx_cos(a) * (r - 3));
    int spoke[] = {cx - ox, cy - oy, cx + ox, cy + oy, ex + ox / 2, ey + oy / 2, ex - ox / 2, ey - oy / 2};
    wtk_poly(spoke, 4, wtk_shade(R_RUBBER, 5), wtk_shade(R_RUBBER, 6));
  }
  wtk_disc(cx, cy, 20, wtk_shade(R_RUBBER, 5));
  wtk_disc(cx, cy, 17, wtk_shade(R_RUBBER, 8 + d / 3));
}
