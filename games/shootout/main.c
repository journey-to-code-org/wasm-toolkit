/* Road to the Shootout: the title, and the switch between the screens. See game.h.
 *
 * 320x240 in 1,024 colours: rooms are painted in dithered gradients, and the cars and the town
 * are lit polygons. Everything on the screen is drawn in code; the one picture is the logo. */
#include "game.h"
#include "logo.h"

int screen;
u32 ticks, rng;
static int old_game; /* whether there is a saved game to go back to */

void logo(int x, int y, col black, col red) {
  for (int j = 0; j < LOGO_H; j++)
    for (int i = 0; i < LOGO_W; i++) {
      int v = LOGO[j * LOGO_W + i];
      if (v) wtk_pset(x + i, y + j, v == 1 ? black : red);
    }
}

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
  car_mesh(&BODIES[(ticks / 340) & 1 ? B_CURVE : B_WEDGE], 0, 0, (int)ticks * 3, (ticks / 340) & 1 ? R_RED : PLAYER_PAINT, 1);
  wtk3_end();

  wtk_text_big(65, 19, "ROAD TO THE", wtk_shade(R_RED, 12), 2);
  wtk_text_big(64, 18, "ROAD TO THE", wtk_shade(R_GOLD, 26), 2);
  wtk_text_big(42, 44, "SHOOTOUT", wtk_shade(R_RED, 14), 4);
  for (int j = 0; j < 4; j++) {
    wtk_clip(0, 42 + j * 8, WTK_W, 8);
    wtk_text_big(40, 42, "SHOOTOUT", wtk_shade(R_GOLD, 31 - j * 3), 4);
  }
  wtk_clip_off();
  if ((ticks / 15) & 1) wtk_text_sh(88, 214, "ENTER: A NEW GAME", WHITE, BLACK);
  if (old_game) wtk_text_sh(100, 226, "E: AN OLD GAME", wtk_shade(R_GOLD, 26), BLACK);
}

/* How it ended. */
static void over_draw(void) {
  static const char *const head[4] = {"", "THE SEASON IS OVER", "OUTTA DOUGH", "YOU BEAT THE KING"};
  static const char *const line1[4] = {"", "THE SHOOTOUT CAME AND YOU WERE NOT READY.", "NO CAR, AND NOT ENOUGH TO BUY ONE.", "HIS CAR IS IN YOUR GARAGE."};
  static const char *const line2[4] = {"", "THE KING STILL HAS HIS CAR.", "YOU ARE DONE FOR THE YEAR.", "SEE YOU AT THE SHOOTOUT."};
  wtk_vgrad(0, 0, WTK_W, WTK_H, over_kind == 3 ? R_GOLD : R_NAVY, over_kind == 3 ? 6 : 2, over_kind == 3 ? 20 : 10);
  logo(112, 40, wtk_shade(R_RUBBER, 2), wtk_shade(R_RED, 24));
  wtk_text_big(160 - wtk_strlen(head[over_kind]) * 8, 90, head[over_kind], WHITE, 2);
  wtk_text_sh(160 - wtk_strlen(line1[over_kind]) * 4, 126, line1[over_kind], WHITE, BLACK);
  wtk_text_sh(160 - wtk_strlen(line2[over_kind]) * 4, 140, line2[over_kind], WHITE, BLACK);
  int x = wtk_text(96, 170, "DAY ", wtk_shade(R_CHROME, 24));
  x = wtk_num(x, 170, hours / DAY_HOURS + 1, wtk_shade(R_CHROME, 24));
  x = wtk_text(x, 170, "   CASH $", wtk_shade(R_CHROME, 24));
  wtk_num(x, 170, cash, wtk_shade(R_CHROME, 24));
  if ((ticks / 15) & 1) wtk_text_sh(116, 210, "PRESS ENTER", WHITE, BLACK);
}

void game_init(u32 seed) {
  rng = seed ? seed : 1;
  screen = TITLE;
  ticks = 0;
  garage_reset();
  old_game = game_saved();
}

void game_tick(void) {
  ticks++;
  if (screen == TITLE) {
    /* A new game starts with the paper in your hand, as Street Rod's does. */
    if (wtk_hit & KEY_A) { garage_reset(); screen = PAPER; wtk_tone(880, 80); }
    /* E is "Old Game": back to where you left it. */
    else if ((wtk_hit & KEY_X) && game_load()) { screen = GARAGE; wtk_tone(660, 80); }
  } else if (screen == OVER) {
    if (wtk_hit & KEY_A) { game_init(rng); game_save(); }
  } else if (screen == MEET) meet_tick();
  else if (screen == DRIVE) drive_tick();
  else if (screen == STRIP) strip_tick();
  else garage_tick();
}

void game_draw(void) {
  if (screen == TITLE) title_draw();
  else if (screen == OVER) over_draw();
  else if (screen == MEET) meet_draw();
  else if (screen == DRIVE) drive_draw();
  else if (screen == STRIP) strip_draw();
  else garage_draw();
}
