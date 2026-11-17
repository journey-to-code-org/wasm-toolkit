/* The cars.
 *
 * A body is a row of cross-sections from the back bumper to the nose, in centimetres. Each has a
 * half-width, a sill height, a beltline, and, where there is a cabin over it, the roof's
 * half-width and height. The skin between two sections is paint, or glass where a cabin begins,
 * ends or runs. On top of that go the things that make it a car: arches, door lines, mirrors,
 * lamps, a wing, and headlamps that pop up. */
#include "game.h"

/* A low wedge of a fastback: a long glass hatch, a short tail, and lamps that fold into the nose. */
static const section WEDGE[] = {
  {0, 72, 44, 84, 0, 0}, {14, 84, 30, 93, 0, 0}, {92, 87, 24, 96, 56, 114}, {140, 88, 22, 97, 60, 123}, {190, 88, 22, 97, 61, 125},
  {236, 88, 22, 95, 58, 120}, {294, 87, 22, 87, 0, 0}, {352, 86, 22, 77, 0, 0}, {406, 82, 26, 66, 0, 0}, {436, 70, 34, 56, 0, 0},
};
/* The one that came after it: rounder everywhere, wider at the hips, with its lamps fixed in the nose. */
static const section CURVE[] = {
  {0, 66, 46, 80, 0, 0}, {12, 80, 32, 90, 0, 0}, {44, 87, 26, 97, 0, 0}, {104, 90, 23, 100, 56, 117}, {152, 91, 22, 101, 60, 127},
  {200, 91, 22, 100, 61, 128}, {246, 90, 22, 98, 57, 120}, {300, 89, 22, 92, 0, 0}, {350, 88, 23, 86, 0, 0}, {396, 84, 26, 78, 0, 0},
  {424, 76, 32, 70, 0, 0}, {438, 62, 40, 62, 0, 0},
};
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
const body BODIES[N_BODIES] = {
  {"WEDGE", WEDGE, 10, 6, 1, 1}, {"CURVE", CURVE, 12, 7, 0, 1}, {"COUPE", COUPE, 10, 6, 0, 0},
  {"SEDAN", SEDAN, 9, 5, 0, 0}, {"WAGON", WAGON, 7, 3, 0, 0}, {"PICKUP", PICKUP, 8, 4, 0, 0},
};

v3 eye;
/* Where the car being drawn stands, and which way it points. */
static fx m_x, m_z, m_sin, m_cos;
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

/* A box, sitting on the car: the four sides and the top, with one end a lamp if `lamp` says so. */
static void block(int x0, int x1, int y0, int y1, int z0, int z1, int paint, int bias, int lamp) {
  v3 top[4] = {M(x0, y1, z0), M(x1, y1, z0), M(x1, y1, z1), M(x0, y1, z1)};
  v3 back[4] = {M(x0, y0, z0), M(x0, y1, z0), M(x0, y1, z1), M(x0, y0, z1)};
  v3 front[4] = {M(x1, y0, z0), M(x1, y1, z0), M(x1, y1, z1), M(x1, y0, z1)};
  v3 in[4] = {M(x0, y0, z0), M(x1, y0, z0), M(x1, y1, z0), M(x0, y1, z0)};
  v3 out[4] = {M(x0, y0, z1), M(x1, y0, z1), M(x1, y1, z1), M(x0, y1, z1)};
  wtk3_lit(top, 4, paint, bias);
  wtk3_lit(back, 4, paint, bias);
  wtk3_lit(in, 4, paint, bias);
  wtk3_lit(out, 4, paint, bias);
  if (lamp) wtk3_poly(front, 4, wtk_shade(R_CREAM, 30), wtk_shade(R_WHITE, 31), bias);
  else wtk3_lit(front, 4, paint, bias);
}

static void wheel(int x, int z, int k, int near) {
  enum { R = 31 };
  v3 face[8], hub[8], arch[5];
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
    wtk3_lit(tread, 4, R_RUBBER, near ? 2 : -1);
  }
  if (!near) return;
  /* The arch it sits in, the tyre, and the wheel: five spokes' worth of bright metal. */
  for (int i = 0; i < 5; i++) arch[i] = M(x + FX_INT(fx_cos(i * 128) * (R + 8)), R + FX_INT(fx_sin(i * 128) * (R + 8)), k * (z + 1));
  wtk3_poly(arch, 5, wtk_shade(R_RUBBER, 2), wtk_shade(R_RUBBER, 3), 1);
  wtk3_lit(face, 8, R_RUBBER, 2);
  wtk3_lit(hub, 8, R_CHROME, 3);
  for (int i = 0; i < 5; i++) {
    int a = i * 205;
    v3 gap[3] = {M(x, R, k * (z + 7)), M(x + FX_INT(fx_cos(a + 40) * R * 55 / 100), R + FX_INT(fx_sin(a + 40) * R * 55 / 100), k * (z + 7)), M(x + FX_INT(fx_cos(a + 130) * R * 55 / 100), R + FX_INT(fx_sin(a + 130) * R * 55 / 100), k * (z + 7))};
    wtk3_poly(gap, 3, wtk_shade(R_RUBBER, 4), wtk_shade(R_RUBBER, 5), 4);
  }
}

void car_mesh(const body *b, fx x, fx z, int yaw, int paint, int lamps) {
  const section *s = b->sec;
  int last = b->n - 1;
  m_x = x; m_z = z; m_sin = fx_sin(yaw); m_cos = fx_cos(yaw); m_half = s[last].x / 2;
  fx right = fx_mul(eye.x - x, m_cos) - fx_mul(eye.z - z, m_sin);
  int near = right > 0 ? 1 : -1;

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
    wtk3_lit(top, 4, cabin == 1 ? R_GLASS : paint, 0);
  }
  /* The two ends, each with a bumper across the bottom of it and its lamps. */
  for (int e = 0; e < 2; e++) {
    const section *q = &s[e ? last : 0];
    int out = e ? 1 : -1;
    v3 l[3], r[3];
    side_of(q, -1, l); side_of(q, 1, r);
    v3 cap[6] = {l[0], l[1], l[2], r[2], r[1], r[0]};
    wtk3_lit(cap, 6, paint, 0);
    v3 bumper[4] = {M(q->x + out, q->sill, -q->w), M(q->x + out, q->sill + 13, -q->w), M(q->x + out, q->sill + 13, q->w), M(q->x + out, q->sill, q->w)};
    wtk3_lit(bumper, 4, R_RUBBER, 1);
    if (!e) {
      /* A bar of red right across the tail. */
      v3 bar[4] = {M(q->x - 2, q->belt - 16, -q->w + 4), M(q->x - 2, q->belt - 5, -q->w + 4), M(q->x - 2, q->belt - 5, q->w - 4), M(q->x - 2, q->belt - 16, q->w - 4)};
      wtk3_poly(bar, 4, wtk_shade(R_RED, 22), wtk_shade(R_RED, 24), 2);
    } else
      for (int k = -1; k <= 1; k += 2) {
        /* Fixed lamps are in the nose. A car with pop-ups has only its markers there. */
        int in = b->popups ? q->w * 70 / 100 : q->w * 40 / 100, tall = b->popups ? 5 : 11;
        v3 lamp[4] = {M(q->x + 2, q->belt - 6 - tall, k * in), M(q->x + 2, q->belt - 6, k * in), M(q->x + 2, q->belt - 6, k * (q->w - 3)), M(q->x + 2, q->belt - 6 - tall, k * (q->w - 3))};
        col c = b->popups ? wtk_shade(R_ORANGE, 24) : wtk_shade(R_CREAM, 29);
        wtk3_poly(lamp, 4, c, c, 2);
      }
  }
  if (b->popups && lamps) {
    const section *n = &s[last - 1];
    for (int k = -1; k <= 1; k += 2) {
      int z0 = k * (n->w - 30), z1 = k * (n->w - 6);
      block(n->x - 26, n->x - 4, n->belt + 2, n->belt + 15, z0 < z1 ? z0 : z1, z0 < z1 ? z1 : z0, paint, 2, 1);
    }
  }
  if (b->wing) {
    /* A wing across the tail, on two posts. */
    const section *t = &s[1];
    block(t->x - 8, t->x + 14, t->belt + 13, t->belt + 16, -t->w + 4, t->w - 4, paint, 2, 0);
    for (int k = -1; k <= 1; k += 2) block(t->x - 2, t->x + 8, t->belt + 3, t->belt + 13, k * (t->w - 16) - 3, k * (t->w - 16) + 3, paint, 2, 0);
  }
  /* A door on the side you can see, and a mirror on each. */
  const section *c = &s[b->cowl];
  for (int d = 0; d < 2; d++) {
    int dx = c->x - 6 - d * 104;
    v3 seam[4] = {M(dx, c->sill + 4, near * (c->w + 1)), M(dx + 2, c->sill + 4, near * (c->w + 1)), M(dx + 2, c->belt - 1, near * (c->w + 1)), M(dx, c->belt - 1, near * (c->w + 1))};
    wtk3_poly(seam, 4, wtk_shade(R_RUBBER, 2), wtk_shade(R_RUBBER, 2), 1);
  }
  for (int k = -1; k <= 1; k += 2) {
    int z0 = k * (c->w + 1), z1 = k * (c->w + 15);
    block(c->x - 20, c->x - 10, c->belt + 1, c->belt + 11, z0 < z1 ? z0 : z1, z0 < z1 ? z1 : z0, paint, k == near ? 2 : -1, 0);
  }
  for (int k = -1; k <= 1; k += 2) {
    wheel(78, s[2].w, k, k == near);
    wheel(s[last].x - 80, s[last - 2].w, k, k == near);
  }
}
