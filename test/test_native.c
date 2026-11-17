/* The toolkit built for this machine, with the sanitizers on, and asked what it drew. */
#include <stdio.h>
#include "../toolkit/wtk.h"

static int failed;
#define CHECK(what) do { if (!(what)) { printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #what); failed++; } } while (0)

void wtk_tone(int hz, int ms) { (void)hz; (void)ms; }
void game_init(u32 seed) { (void)seed; }
void game_tick(void) {}
void game_draw(void) {}

static int count(u8 c) { int n = 0; for (int i = 0; i < WTK_W * WTK_H; i++) n += wtk_fb[i] == c; return n; }
static u8 at(int x, int y) { return wtk_fb[y * WTK_W + x]; }

int main(void) {
  wtk_init(1);

  /* Maths. */
  CHECK(fx_sin(0) == 0 && fx_sin(256) == FX(1) && fx_sin(768) == -FX(1) && fx_cos(0) == FX(1));
  CHECK(fx_sin(-256) == -FX(1) && fx_sin(1024 + 256) == FX(1));
  CHECK(fx_mul(FX(3), FX(4)) == FX(12) && fx_div(FX(12), FX(4)) == FX(3));
  CHECK(wtk_isqrt(0) == 0 && wtk_isqrt(144) == 12 && wtk_isqrt(145) == 12 && wtk_isqrt(0xFFFFFFFFu) == 65535);
  u32 a = 7, b = 7;
  CHECK(wtk_rand(&a) == wtk_rand(&b) && a == b && a != 7);

  /* Boxes, and staying on the screen. */
  wtk_clear(BLACK);
  wtk_rect(10, 10, 20, 5, RED);
  CHECK(count(RED) == 100 && at(10, 10) == RED && at(29, 14) == RED && at(30, 14) == BLACK);
  wtk_rect(-50, -50, 1000, 1000, BLUE);
  CHECK(count(BLUE) == WTK_W * WTK_H);
  wtk_line(-500, -500, 5000, 5000, WHITE);
  wtk_disc(0, 0, 400, GREEN);
  wtk_pset(-1, 0, RED); wtk_pset(WTK_W, 0, RED); wtk_pset(0, WTK_H, RED);
  CHECK(count(RED) == 0);

  /* Clipping. */
  wtk_clear(BLACK);
  wtk_clip(100, 50, 10, 10);
  wtk_rect(0, 0, WTK_W, WTK_H, YELLOW);
  CHECK(count(YELLOW) == 100 && at(100, 50) == YELLOW && at(99, 50) == BLACK && at(110, 59) == BLACK);
  wtk_clip_off();

  /* A dither is half of each. */
  wtk_clear(BLACK);
  wtk_dither(0, 0, 10, 10, RED, BLUE);
  CHECK(count(RED) == 50 && count(BLUE) == 50 && at(0, 0) == RED && at(1, 0) == BLUE && at(0, 1) == BLUE);

  /* A polygon: a square is its box, a triangle about half of one, and one far off the screen is cut to it. */
  wtk_clear(BLACK);
  int sq[] = {10, 10, 19, 10, 19, 19, 10, 19};
  wtk_poly(sq, 4, CYAN, CYAN);
  CHECK(count(CYAN) == 100);
  wtk_clear(BLACK);
  int tri[] = {0, 0, 99, 0, 0, 99};
  wtk_poly(tri, 3, CYAN, CYAN);
  CHECK(count(CYAN) > 4900 && count(CYAN) < 5200 && at(1, 1) == CYAN && at(98, 98) == BLACK);
  wtk_clear(BLACK);
  int huge[] = {-30000, -30000, 30000, -30000, 30000, 30000, -30000, 30000};
  wtk_poly(huge, 4, CYAN, CYAN);
  CHECK(count(CYAN) == WTK_W * WTK_H);

  /* Text: a glyph is eight wide, and a number reads the way it was written. */
  wtk_clear(BLACK);
  CHECK(wtk_text(0, 0, "AB", WHITE) == 16 && count(WHITE) > 20);
  CHECK(wtk_num(0, 20, -1234, WHITE) == 40 && wtk_num(0, 30, 0, WHITE) == 8);
  CHECK(wtk_strlen("shootout") == 8);

  /* Three dimensions: a point dead ahead lands on the horizon, one behind is refused, and of two
     walls the nearer is the one left showing. */
  wtk_cam cam = {{0, FX(1), 0}, 0, 200, 160, 100};
  wtk3_begin(&cam);
  int sx, sy;
  v3 ahead = {0, FX(1), FX(10)}, behind = {0, FX(1), -FX(10)}, right = {FX(5), FX(1), FX(10)}, up = {0, FX(6), FX(10)};
  CHECK(wtk3_point(ahead, &sx, &sy, 0) && sx == 160 && sy == 100);
  CHECK(!wtk3_point(behind, &sx, &sy, 0));
  CHECK(wtk3_point(right, &sx, &sy, 0) && sx == 260 && sy == 100);
  CHECK(wtk3_point(up, &sx, &sy, 0) && sx == 160 && sy == 0);
  wtk_clear(BLACK);
  v3 far_wall[4] = {{-FX(4), 0, FX(20)}, {FX(4), 0, FX(20)}, {FX(4), FX(4), FX(20)}, {-FX(4), FX(4), FX(20)}};
  v3 near_wall[4] = {{-FX(1), 0, FX(10)}, {FX(1), 0, FX(10)}, {FX(1), FX(2), FX(10)}, {-FX(1), FX(2), FX(10)}};
  wtk3_poly(near_wall, 4, RED, RED, 0);
  wtk3_poly(far_wall, 4, BLUE, BLUE, 0);
  CHECK(wtk3_end() == 2 && at(160, 100) == RED && at(130, 100) == BLUE);
  /* Turned to face east, what was to the right is ahead. */
  cam.yaw = 256;
  wtk3_begin(&cam);
  v3 east = {FX(10), FX(1), 0};
  CHECK(wtk3_point(east, &sx, &sy, 0) && sx == 160);
  /* A wall that straddles the camera is cut, not thrown away or drawn inside out. */
  cam.yaw = 0;
  wtk3_begin(&cam);
  wtk_clear(BLACK);
  v3 road[4] = {{-FX(4), 0, -FX(50)}, {FX(4), 0, -FX(50)}, {FX(4), 0, FX(50)}, {-FX(4), 0, FX(50)}};
  wtk3_poly(road, 4, DGRAY, DGRAY, 0);
  CHECK(wtk3_end() == 1 && at(160, 190) == DGRAY && at(160, 90) == BLACK);

  printf(failed ? "%d FAILED\n" : "native tests pass\n", failed);
  return failed ? 1 : 0;
}
