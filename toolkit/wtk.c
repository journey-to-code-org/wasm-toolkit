/* wtk: the screen, the maths, the polygons and the door to the host. See wtk.h. */
#include "wtk.h"

col wtk_fb[WTK_W * WTK_H];
u32 wtk_pal[WTK_COLOURS];
u32 wtk_keys, wtk_hit;
int wtk_mx = WTK_W / 2, wtk_my = WTK_H / 2, wtk_moved;

extern const u8 wtk_font[96][8];
extern const i32 wtk_sintab[257];

static int cx0, cy0, cx1 = WTK_W, cy1 = WTK_H; /* the clip box: x0 <= x < x1 */

/* ---- The palette ---------------------------------------------------------------------------- */

void wtk_pal_set(int i, int r, int g, int b) {
  wtk_pal[i & (WTK_COLOURS - 1)] = 0xFF000000u | ((u32)(b & 255) << 16) | ((u32)(g & 255) << 8) | (u32)(r & 255);
}

void wtk_pal_ega(void) {
  static const u8 ega[16][3] = {
    {0, 0, 0}, {0, 0, 170}, {0, 170, 0}, {0, 170, 170}, {170, 0, 0}, {170, 0, 170}, {170, 85, 0}, {170, 170, 170},
    {85, 85, 85}, {85, 85, 255}, {85, 255, 85}, {85, 255, 255}, {255, 85, 85}, {255, 85, 255}, {255, 255, 85}, {255, 255, 255},
  };
  for (int i = 0; i < 16; i++) wtk_pal_set(i, ega[i][0], ega[i][1], ega[i][2]);
}

void wtk_pal_ramp(int ramp, int r, int g, int b) {
  for (int s = 0; s < WTK_SHADES; s++) {
    int v[3] = {r, g, b};
    for (int k = 0; k < 3; k++) {
      /* Up from a tenth of the colour to all of it at shade 24, then on towards white. */
      if (s <= 24) v[k] = v[k] * (26 + s * 230 / 24) / 256;
      else v[k] = v[k] + (255 - v[k]) * (s - 24) / 10;
    }
    wtk_pal_set(ramp * WTK_SHADES + s, v[0], v[1], v[2]);
  }
}

void wtk_pal_default(void) {
  static const u8 base[32][3] = {
    {0, 0, 0}, {150, 150, 150}, {214, 40, 40}, {236, 120, 28}, {240, 206, 48}, {150, 212, 40}, {44, 170, 60}, {30, 160, 140},
    {40, 190, 214}, {110, 170, 240}, {48, 90, 214}, {76, 60, 190}, {136, 70, 200}, {206, 60, 180}, {240, 130, 170}, {130, 82, 44},
    {200, 170, 120}, {166, 80, 58}, {168, 164, 156}, {74, 76, 84}, {70, 130, 50}, {96, 140, 170}, {186, 196, 208}, {46, 46, 50},
    {150, 100, 56}, {236, 224, 192}, {30, 46, 110}, {120, 24, 40}, {30, 84, 50}, {212, 170, 60}, {176, 180, 188}, {236, 236, 240},
  };
  for (int i = 1; i < 32; i++) wtk_pal_ramp(i, base[i][0], base[i][1], base[i][2]);
  /* Ramp 0 is the EGA sixteen, and sixteen greys after them. */
  for (int i = 16; i < 32; i++) wtk_pal_set(i, (i - 16) * 17, (i - 16) * 17, (i - 16) * 17);
  wtk_pal_ega();
}

/* ---- Drawing -------------------------------------------------------------------------------- */

void wtk_clip(int x, int y, int w, int h) {
  cx0 = x < 0 ? 0 : x;
  cy0 = y < 0 ? 0 : y;
  cx1 = x + w > WTK_W ? WTK_W : x + w;
  cy1 = y + h > WTK_H ? WTK_H : y + h;
}
void wtk_clip_off(void) { cx0 = cy0 = 0; cx1 = WTK_W; cy1 = WTK_H; }

void wtk_clear(col c) { for (int i = 0; i < WTK_W * WTK_H; i++) wtk_fb[i] = c; }

void wtk_pset(int x, int y, col c) {
  if (x >= cx0 && x < cx1 && y >= cy0 && y < cy1) wtk_fb[y * WTK_W + x] = c;
}

void wtk_hline(int x0, int x1, int y, col c) {
  if (x0 > x1) { int t = x0; x0 = x1; x1 = t; }
  if (y < cy0 || y >= cy1 || x1 < cx0 || x0 >= cx1) return;
  if (x0 < cx0) x0 = cx0;
  if (x1 >= cx1) x1 = cx1 - 1;
  col *p = wtk_fb + y * WTK_W + x0;
  for (int n = x1 - x0 + 1; n > 0; n--) *p++ = c;
}

void wtk_vline(int x, int y0, int y1, col c) {
  if (y0 > y1) { int t = y0; y0 = y1; y1 = t; }
  for (int y = y0; y <= y1; y++) wtk_pset(x, y, c);
}

void wtk_line(int x0, int y0, int x1, int y1, col c) {
  int dx = x1 > x0 ? x1 - x0 : x0 - x1, sx = x0 < x1 ? 1 : -1;
  int dy = y1 > y0 ? y0 - y1 : y1 - y0, sy = y0 < y1 ? 1 : -1;
  int err = dx + dy;
  for (int guard = 0; guard < 4096; guard++) {
    wtk_pset(x0, y0, c);
    if (x0 == x1 && y0 == y1) break;
    int e2 = 2 * err;
    if (e2 >= dy) { err += dy; x0 += sx; }
    if (e2 <= dx) { err += dx; y0 += sy; }
  }
}

void wtk_rect(int x, int y, int w, int h, col c) {
  for (int j = 0; j < h; j++) wtk_hline(x, x + w - 1, y + j, c);
}

void wtk_box(int x, int y, int w, int h, col c) {
  wtk_hline(x, x + w - 1, y, c);
  wtk_hline(x, x + w - 1, y + h - 1, c);
  wtk_vline(x, y, y + h - 1, c);
  wtk_vline(x + w - 1, y, y + h - 1, c);
}

/* A run of a row in one colour or a checkerboard of two. */
static void span(int x0, int x1, int y, col a, col b) {
  if (a == b) { wtk_hline(x0, x1, y, a); return; }
  if (y < cy0 || y >= cy1 || x1 < cx0 || x0 >= cx1) return;
  if (x0 < cx0) x0 = cx0;
  if (x1 >= cx1) x1 = cx1 - 1;
  col *p = wtk_fb + y * WTK_W + x0;
  for (int x = x0; x <= x1; x++) *p++ = ((x ^ y) & 1) ? b : a;
}

void wtk_dither(int x, int y, int w, int h, col a, col b) {
  for (int j = 0; j < h; j++) span(x, x + w - 1, y + j, a, b);
}

void wtk_vgrad(int x, int y, int w, int h, int ramp, int s0, int s1) {
  static const u8 bayer[4][4] = {{0, 8, 2, 10}, {12, 4, 14, 6}, {3, 11, 1, 9}, {15, 7, 13, 5}};
  for (int j = 0; j < h; j++) {
    int yy = y + j;
    if (yy < cy0 || yy >= cy1) continue;
    int s = s0 * 256 + (s1 - s0) * 256 * j / (h > 1 ? h - 1 : 1);
    int whole = s >> 8, part = (s & 255) >> 4;
    int xa = x < cx0 ? cx0 : x, xb = x + w > cx1 ? cx1 : x + w;
    for (int xx = xa; xx < xb; xx++) wtk_fb[yy * WTK_W + xx] = wtk_shade(ramp, whole + (part > bayer[yy & 3][xx & 3]));
  }
}

void wtk_disc(int cx, int cy, int r, col c) {
  for (int y = -r; y <= r; y++) {
    int w = (int)wtk_isqrt((u32)(r * r - y * y));
    wtk_hline(cx - w, cx + w, cy + y, c);
  }
}

void wtk_poly(const int *xy, int n, col a, col b) {
  static i16 lo[WTK_H], hi[WTK_H];
  int top = cy1, bottom = cy0 - 1;
  for (int i = 0; i < n; i++) {
    int y = xy[i * 2 + 1];
    if (y < top) top = y;
    if (y > bottom) bottom = y;
  }
  if (top < cy0) top = cy0;
  if (bottom >= cy1) bottom = cy1 - 1;
  if (top > bottom) return;
  for (int y = top; y <= bottom; y++) { lo[y] = 32767; hi[y] = -32768; }
  for (int i = 0; i < n; i++) {
    int x0 = xy[i * 2], y0 = xy[i * 2 + 1];
    int j = (i + 1) % n;
    int x1 = xy[j * 2], y1 = xy[j * 2 + 1];
    if (y0 > y1) { int t = x0; x0 = x1; x1 = t; t = y0; y0 = y1; y1 = t; }
    if (y0 == y1) {
      if (y0 < top || y0 > bottom) continue;
      int l = x0 < x1 ? x0 : x1, h = x0 < x1 ? x1 : x0;
      if (l < lo[y0]) lo[y0] = (i16)(l < -32000 ? -32000 : l);
      if (h > hi[y0]) hi[y0] = (i16)(h > 32000 ? 32000 : h);
      continue;
    }
    i64 step = ((i64)(x1 - x0) * 65536) / (y1 - y0);
    i64 x = (i64)x0 * 65536 + 32768;
    int ya = y0, yb = y1;
    if (ya < top) { x += step * (top - ya); ya = top; }
    if (yb > bottom) yb = bottom;
    for (int y = ya; y <= yb; y++, x += step) {
      int px = (int)(x >> 16);
      if (px < -32000) px = -32000;
      if (px > 32000) px = 32000;
      if (px < lo[y]) lo[y] = (i16)px;
      if (px > hi[y]) hi[y] = (i16)px;
    }
  }
  for (int y = top; y <= bottom; y++)
    if (lo[y] <= hi[y]) span(lo[y], hi[y], y, a, b);
}

void wtk_blit(const col *src, int w, int h, int x, int y, int key) {
  for (int j = 0; j < h; j++)
    for (int i = 0; i < w; i++) {
      col c = src[j * w + i];
      if (c != key) wtk_pset(x + i, y + j, c);
    }
}

static col dimmed(col c, int dim) {
  if (!dim || c < WTK_SHADES) return c;
  int s = (c & (WTK_SHADES - 1)) - dim;
  return (col)((c & ~(WTK_SHADES - 1)) | (s < 0 ? 0 : s));
}

void wtk_rle(const u16 *data, int x, int y, int dim) {
  int w = data[0], h = data[1], i = 0, n = w * h;
  for (const u16 *p = data + 2; i < n; p += 2) {
    int count = p[0];
    if (p[1] != 0xFFFF) {
      col c = dimmed(p[1], dim);
      for (int k = 0; k < count; k++) wtk_pset(x + (i + k) % w, y + (i + k) / w, c);
    }
    i += count;
  }
}

void wtk_spin(const col *px, int size, int cx, int cy, int angle, int dim) {
  fx s = fx_sin(angle), c = fx_cos(angle);
  int half = size / 2;
  for (int dy = -half; dy < half; dy++)
    for (int dx = -half; dx < half; dx++) {
      /* Where this pixel came from, before the turn. */
      int sx = half + FX_INT(dx * c + dy * s + 32768), sy = half + FX_INT(dy * c - dx * s + 32768);
      if (sx < 0 || sy < 0 || sx >= size || sy >= size) continue;
      col v = px[sy * size + sx];
      if (v != 0xFFFF) wtk_pset(cx + dx, cy + dy, dimmed(v, dim));
    }
}

int wtk_strlen(const char *s) { int n = 0; while (s[n]) n++; return n; }

int wtk_text(int x, int y, const char *s, col c) {
  for (; *s; s++, x += 8) {
    int ch = (u8)*s;
    if (ch < 32 || ch > 127) ch = '?';
    const u8 *g = wtk_font[ch - 32];
    for (int j = 0; j < 8; j++)
      for (int i = 0; i < 8; i++)
        if (g[j] & (1 << i)) wtk_pset(x + i, y + j, c);
  }
  return x;
}

int wtk_text_big(int x, int y, const char *s, col c, int scale) {
  for (; *s; s++, x += 8 * scale) {
    int ch = (u8)*s;
    if (ch < 32 || ch > 127) ch = '?';
    const u8 *g = wtk_font[ch - 32];
    for (int j = 0; j < 8; j++)
      for (int i = 0; i < 8; i++)
        if (g[j] & (1 << i)) wtk_rect(x + i * scale, y + j * scale, scale, scale, c);
  }
  return x;
}

int wtk_text_sh(int x, int y, const char *s, col c, col shadow) {
  wtk_text(x + 1, y + 1, s, shadow);
  return wtk_text(x, y, s, c);
}

int wtk_num(int x, int y, int n, col c) {
  char buf[12];
  int i = 11;
  u32 v = n < 0 ? (u32)(-(i64)n) : (u32)n;
  buf[i] = 0;
  do { buf[--i] = (char)('0' + v % 10); v /= 10; } while (v);
  if (n < 0) buf[--i] = '-';
  return wtk_text(x, y, buf + i, c);
}

/* ---- Maths ---------------------------------------------------------------------------------- */

fx fx_sin(int angle) {
  angle &= WTK_TURN - 1;
  int q = angle >> 8, i = angle & 255;
  switch (q) {
    case 0: return wtk_sintab[i];
    case 1: return wtk_sintab[256 - i];
    case 2: return -wtk_sintab[i];
    default: return -wtk_sintab[256 - i];
  }
}
fx fx_cos(int angle) { return fx_sin(angle + WTK_TURN / 4); }

u32 wtk_isqrt(u32 n) {
  u32 r = 0, bit = 1u << 30;
  while (bit > n) bit >>= 2;
  while (bit) {
    if (n >= r + bit) { n -= r + bit; r = (r >> 1) + bit; } else r >>= 1;
    bit >>= 2;
  }
  return r;
}

u32 wtk_rand(u32 *state) {
  u32 x = *state ? *state : 0x9E3779B9u;
  x ^= x << 13; x ^= x >> 17; x ^= x << 5;
  return *state = x;
}

/* ---- Three dimensions ------------------------------------------------------------------------ */

#define NEAR (FX(1) / 2)
#define FAR_OFF 30000

typedef struct { i32 key; col a, b; u8 n; i16 xy[WTK3_MAX_VERTS * 2 + 2]; } poly3;
static poly3 pool[WTK3_MAX_POLYS];
static u16 order[WTK3_MAX_POLYS];
static int pooled;
static wtk_cam cam;
static fx cam_sin, cam_cos, tip_sin, tip_cos;

void wtk3_begin(const wtk_cam *c) {
  cam = *c;
  cam_sin = fx_sin(c->yaw);
  cam_cos = fx_cos(c->yaw);
  tip_sin = fx_sin(c->pitch);
  tip_cos = fx_cos(c->pitch);
  pooled = 0;
}

/* Into the camera's own space: x to its right, y up, z ahead. */
static v3 to_cam(v3 p) {
  fx dx = p.x - cam.pos.x, dz = p.z - cam.pos.z;
  fx rx = fx_mul(dx, cam_cos) - fx_mul(dz, cam_sin), ry = p.y - cam.pos.y, rz = fx_mul(dx, cam_sin) + fx_mul(dz, cam_cos);
  v3 r = { rx, fx_mul(ry, tip_cos) + fx_mul(rz, tip_sin), fx_mul(rz, tip_cos) - fx_mul(ry, tip_sin) };
  return r;
}

static int clampi(i64 v) { return v < -FAR_OFF ? -FAR_OFF : v > FAR_OFF ? FAR_OFF : (int)v; }

int wtk3_point(v3 p, int *sx, int *sy, fx *depth) {
  v3 c = to_cam(p);
  if (depth) *depth = c.z;
  if (c.z < NEAR) return 0;
  *sx = clampi(cam.cx + ((i64)c.x * cam.focal) / c.z);
  *sy = clampi(cam.cy - ((i64)c.y * cam.focal) / c.z);
  return 1;
}

void wtk3_poly(const v3 *pts, int n, col a, col b, int bias) {
  if (pooled >= WTK3_MAX_POLYS || n < 3 || n > WTK3_MAX_VERTS) return;
  v3 in[WTK3_MAX_VERTS], out[WTK3_MAX_VERTS + 1];
  i64 depth = 0;
  for (int i = 0; i < n; i++) { in[i] = to_cam(pts[i]); depth += in[i].z; }
  /* Cut away what is behind the picture plane. */
  int m = 0;
  for (int i = 0; i < n; i++) {
    v3 p = in[i], q = in[(i + 1) % n];
    int pin = p.z >= NEAR, qin = q.z >= NEAR;
    if (pin) out[m++] = p;
    if (pin != qin) {
      fx t = fx_div(NEAR - p.z, q.z - p.z);
      v3 cut = { p.x + fx_mul(q.x - p.x, t), p.y + fx_mul(q.y - p.y, t), NEAR };
      out[m++] = cut;
    }
  }
  if (m < 3) return;
  poly3 *o = &pool[pooled];
  int left = 1, right = 1, above = 1, below = 1;
  for (int i = 0; i < m; i++) {
    int sx = clampi(cam.cx + ((i64)out[i].x * cam.focal) / out[i].z);
    int sy = clampi(cam.cy - ((i64)out[i].y * cam.focal) / out[i].z);
    o->xy[i * 2] = (i16)sx;
    o->xy[i * 2 + 1] = (i16)sy;
    if (sx >= 0) left = 0;
    if (sx < WTK_W) right = 0;
    if (sy >= 0) above = 0;
    if (sy < WTK_H) below = 0;
  }
  if (left || right || above || below) return;
  o->n = (u8)m; o->a = a; o->b = b;
  o->key = (i32)(depth / n) - bias * 65536;
  order[pooled] = (u16)pooled;
  pooled++;
}

void wtk3_lit(const v3 *pts, int n, int ramp, int bias) {
  /* The way the face points, from two of its edges, in units small enough to multiply. */
  i64 ax = (pts[1].x - pts[0].x) >> 6, ay = (pts[1].y - pts[0].y) >> 6, az = (pts[1].z - pts[0].z) >> 6;
  i64 bx = (pts[n - 1].x - pts[0].x) >> 6, by = (pts[n - 1].y - pts[0].y) >> 6, bz = (pts[n - 1].z - pts[0].z) >> 6;
  i64 nx = ay * bz - az * by, ny = az * bx - ax * bz, nz = ax * by - ay * bx;
  i64 big = nx < 0 ? -nx : nx, t = ny < 0 ? -ny : ny;
  if (t > big) big = t;
  t = nz < 0 ? -nz : nz;
  if (t > big) big = t;
  while (big > 30000) { nx /= 2; ny /= 2; nz /= 2; big /= 2; }
  i32 len = (i32)wtk_isqrt((u32)(nx * nx + ny * ny + nz * nz));
  if (len == 0) { wtk3_poly(pts, n, wtk_shade(ramp, 16), wtk_shade(ramp, 16), bias); return; }
  /* Whichever side faces the camera is the side that is lit. */
  i64 toward = nx * ((cam.pos.x - pts[0].x) >> 8) + ny * ((cam.pos.y - pts[0].y) >> 8) + nz * ((cam.pos.z - pts[0].z) >> 8);
  if (toward < 0) { nx = -nx; ny = -ny; nz = -nz; }
  /* The light comes from above, the west and the south: (-0.45, 0.80, -0.40). */
  i64 d = (nx * -115 + ny * 205 + nz * -102) / len; /* -256 .. 256 */
  int s = 8 * 256 + (d > 0 ? (int)d * 17 : (int)d * 3);
  if (ny * 256 / len > 240) s += 512;
  int whole = s >> 8;
  wtk3_poly(pts, n, wtk_shade(ramp, whole), wtk_shade(ramp, whole + ((s & 255) >= 128)), bias);
}

int wtk3_end(void) {
  /* Far to near. Shell's sort: no recursion, no memory, quick enough for a thousand. */
  for (int gap = pooled / 2; gap > 0; gap /= 2)
    for (int i = gap; i < pooled; i++) {
      u16 t = order[i];
      i32 k = pool[t].key;
      int j = i;
      for (; j >= gap && pool[order[j - gap]].key < k; j -= gap) order[j] = order[j - gap];
      order[j] = t;
    }
  for (int i = 0; i < pooled; i++) {
    poly3 *o = &pool[order[i]];
    int xy[WTK3_MAX_VERTS * 2 + 2];
    for (int k = 0; k < o->n * 2; k++) xy[k] = o->xy[k];
    wtk_poly(xy, o->n, o->a, o->b);
  }
  return pooled;
}

/* ---- The host -------------------------------------------------------------------------------- */

static u32 last_keys;

void wtk_init(u32 seed) {
  wtk_pal_default();
  wtk_clip_off();
  wtk_clear(BLACK);
  last_keys = 0;
  game_init(seed);
  game_draw();
}

void wtk_input(u32 keys, int mx, int my) {
  wtk_keys = keys;
  wtk_moved = mx != wtk_mx || my != wtk_my;
  wtk_mx = mx < 0 ? 0 : mx >= WTK_W ? WTK_W - 1 : mx;
  wtk_my = my < 0 ? 0 : my >= WTK_H ? WTK_H - 1 : my;
}

void wtk_step(void) {
  wtk_hit = wtk_keys & ~last_keys;
  last_keys = wtk_keys;
  game_tick();
  wtk_clip_off();
  game_draw();
  wtk_moved = 0;
}

col *wtk_screen(void) { return wtk_fb; }
u32 *wtk_palette(void) { return wtk_pal; }

#ifdef __wasm__
/* clang turns the loops above into calls to these; with no libc, they have to be here. */
void *memset(void *d, int c, unsigned long n) { u8 *p = d; while (n--) *p++ = (u8)c; return d; }
void *memcpy(void *d, const void *s, unsigned long n) { u8 *p = d; const u8 *q = s; while (n--) *p++ = *q++; return d; }
#endif
