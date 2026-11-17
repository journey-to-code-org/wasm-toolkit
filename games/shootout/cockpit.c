/* The view from the seat.
 *
 * The cockpit is a painting (tools/paint.py, in art.h): the pillars, the dash that wraps round
 * you, the dials, the centre stack. Over it the game draws what moves: the needles, the lights,
 * the gear in the gate, and the wheel, turned with your hands. There is a second painting for
 * the night, with the cabin dark and the dials lit from behind.
 *
 * Under the painting, through the glass, is the hood, in whatever colour the car is. */
#include "game.h"
#include "art.h"

static void needle(int cx, int cy, int r, int value, int top) {
  if (value > top) value = top;
  if (value < 0) value = 0;
  int a = -400 + value * 800 / top;
  int ex = cx + FX_INT(fx_sin(a) * r), ey = cy - FX_INT(fx_cos(a) * r);
  wtk_line(cx + 1, cy + 1, ex + 1, ey + 1, wtk_shade(R_RUBBER, 1));
  wtk_line(cx, cy, ex, ey, wtk_shade(R_ORANGE, 27));
  if (r > 10) wtk_line(cx + (ex > cx ? 0 : 1), cy + 1, ex + (ex > cx ? 0 : 1), ey + 1, wtk_shade(R_RED, 24));
  wtk_disc(cx, cy, r > 10 ? 2 : 1, wtk_shade(R_RUBBER, 9));
  wtk_pset(cx, cy - 1, wtk_shade(R_CHROME, 24));
}

void cockpit(const cockpit_look *k) {
  int d = k->night ? -17 : 0; /* how much darker the paint is */

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

  wtk_rle(k->night ? ART_COCKPIT_NIGHT : ART_COCKPIT, 0, 0, 0);

  needle(ART_TACH_X, ART_TACH_Y, ART_DIAL_R - 4, k->rpm / 10, 900);
  needle(ART_SPEEDO_X, ART_SPEEDO_Y, ART_DIAL_R - 4, k->mph, 160);
  needle(ART_BOOST_X, ART_BOOST_Y, ART_BOOST_R - 3, k->boost, 30);
  needle(ART_MINI_X, ART_MINI_Y, ART_MINI_R - 2, 46, 100);
  needle(ART_MINI_X, ART_MINI_Y + ART_MINI_STEP, ART_MINI_R - 2, 70, 100);
  /* The shift light between the dials, and the lamps tell-tale. */
  wtk_disc(ART_MINI_X, (ART_TACH_Y), 3, wtk_shade(R_RUBBER, 2));
  wtk_disc(ART_MINI_X, (ART_TACH_Y), 2, wtk_shade(R_RED, k->shift_light ? 30 : 5));
  if (k->shift_light) wtk_pset(ART_MINI_X - 1, ART_TACH_Y - 1, wtk_shade(R_WHITE, 30));
  wtk_rect(ART_MINI_X - 3, ART_TACH_Y + 7, 6, 2, wtk_shade(k->lamps ? R_LIME : R_RUBBER, k->lamps ? 26 : 6));
  /* The gear, lit in the gate. */
  static const i16 gate[6][2] = {{0, 0}, {-8, -7}, {-8, 7}, {0, -7}, {0, 7}, {8, -7}};
  if (k->gear >= 0 && k->gear <= 5) {
    wtk_disc(ART_GATE_X + gate[k->gear][0], ART_GATE_Y + gate[k->gear][1], 2, wtk_shade(R_ORANGE, 28));
    wtk_pset(ART_GATE_X + gate[k->gear][0], ART_GATE_Y + gate[k->gear][1], wtk_shade(R_WHITE, 31));
  }
  wtk_spin(k->night ? ART_WHEEL_NIGHT : ART_WHEEL, ART_WHEEL_SIZE, ART_WHEEL_X, ART_WHEEL_Y, k->wheel, 0);
}
