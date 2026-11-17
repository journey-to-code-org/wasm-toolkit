/* The garage: the room, and the work you do in it.
 *
 * The car is made of parts, and a part is held on by four bolts. Under the hood you undo the
 * bolts, lift the part off, set another in its place and do the bolts up again; until then the
 * car will not start. Parts come out of the paper on the bench, for cash. What is on the car is
 * what the engine makes at the strip. */
#include "game.h"
#include "art.h"

/* ---- The parts -------------------------------------------------------------------------------- */

enum { INTAKE, TURBO, EXHAUST, COOLER, N_SLOTS };
/* A place on the engine, the stock part and the better one that go there, and where it is under the hood. */
typedef struct { const char *slot, *name[2], *chip[2], *pitch, *missing, *undone; int price; i16 x, y, w, h; } part;
static const part PARTS[N_SLOTS] = {
  {"THE INTAKE", {"STOCK AIRBOX", "CONE FILTER KIT"}, {"AIRBOX", "CONE FLTR"}, "BREATHES UP TOP", "THE INTAKE IS OFF. IT WILL NOT RUN.", "THE INTAKE IS NOT BOLTED DOWN.", 180, 56, 98, 42, 52},
  {"THE TURBO", {"SMALL TURBO", "BIG TURBO"}, {"SM TURBO", "BIG TURBO"}, "27 PSI. HITS LATE", "THE TURBO IS OFF. IT WILL NOT RUN.", "THE TURBO IS NOT BOLTED DOWN.", 900, 138, 128, 28, 30},
  {"THE EXHAUST", {"CAST MANIFOLD", "TUBULAR HEADER"}, {"MANIFOLD", "HEADER"}, "LETS IT ALL OUT", "THE MANIFOLD IS OFF. IT WILL NOT RUN.", "THE MANIFOLD IS NOT BOLTED DOWN.", 420, 168, 112, 62, 36},
  {"THE INTERCOOLER PIPE", {"SIDE INTERCOOLER", "FRONT INTERCOOLER"}, {"SM COOLER", "BIG COOLR"}, "COLD, DENSE AIR", "THE INTERCOOLER IS OFF. IT WILL NOT RUN.", "THE INTERCOOLER IS NOT BOLTED DOWN.", 350, 100, 100, 28, 56},
};

int cash, lamps_up;
static int fitted[N_SLOTS];   /* -1 for nothing, 0 for the stock part, 1 for the better one */
static int bolts[N_SLOTS];    /* a bit for each of the four bolts that is tight */
static int loose[N_SLOTS][2]; /* the parts that are off the car and on the floor */
static int carrying;          /* the part in your hand, as slot * 2 + which, or -1 */
static const char *note;      /* what just happened, for the bottom line */
static int px = 160, py = 100;

void garage_reset(void) {
  cash = 1500; lamps_up = 0; carrying = -1; note = 0; px = 160; py = 100;
  for (int s = 0; s < N_SLOTS; s++) { fitted[s] = 0; bolts[s] = 15; loose[s][0] = loose[s][1] = 0; }
}

int car_runs(const char **why) {
  for (int s = 0; s < N_SLOTS; s++) {
    if (fitted[s] < 0) { *why = PARTS[s].missing; return 0; }
    if (bolts[s] != 15) { *why = PARTS[s].undone; return 0; }
  }
  return 1;
}

int tune_gain(void) { return 100 + (fitted[INTAKE] == 1) * 4 + (fitted[EXHAUST] == 1) * 6 + (fitted[COOLER] == 1) * 5; }
int tune_big_turbo(void) { return fitted[TURBO] == 1; }

static int owned(int s) { return loose[s][1] || fitted[s] == 1 || carrying == s * 2 + 1; }

/* ---- The pointer, and the things it can be on ------------------------------------------------ */

typedef struct { i16 x, y, w, h; const char *name; int does; } spot;
enum { DO_NOTHING, DO_DRIVE, DO_HOOD, DO_STRIP, DO_PAPER, DO_LAMPS };

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

static void cash_line(int x, int y, col c) {
  x = wtk_text(x, y, "CASH $", c);
  wtk_num(x, y, cash, c);
}

/* ---- The room --------------------------------------------------------------------------------- */

static const spot GARAGE_SPOTS[] = {
  {196, 20, 112, 120, "THE GARAGE DOOR: DRIVE INTO TOWN", DO_DRIVE},
  {18, 26, 64, 52, "A WINDOW. IT IS DAYLIGHT.", DO_NOTHING},
  {96, 24, 30, 38, "TONIGHT: TEST AND TUNE AT THE STRIP", DO_STRIP},
  {92, 68, 100, 28, "A BANNER", DO_NOTHING},
  {12, 86, 76, 54, "THE PAPER ON THE BENCH: PARTS FOR SALE", DO_PAPER},
  {92, 112, 80, 70, "THE CAR: GET IN AND DRIVE", DO_DRIVE},
  {170, 122, 62, 34, "THE HOOD: GET UNDER IT", DO_HOOD},
  {214, 156, 60, 36, "THE HEADLAMPS", DO_LAMPS},
};
#define N_GARAGE (int)(sizeof GARAGE_SPOTS / sizeof GARAGE_SPOTS[0])

static void room_draw(void) {
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
  /* A banner, pinned up. */
  wtk_rect(93, 71, 100, 26, wtk_shade(R_WOOD, 5));
  wtk_vgrad(92, 70, 100, 26, R_WHITE, 27, 22);
  logo(94, 72, wtk_shade(R_RUBBER, 3), wtk_shade(R_RED, 24));
  /* A pegboard, and a bench under it with a vice. */
  wtk_vgrad(14, 86, 70, 16, R_TAN, 14, 18);
  for (int x = 18; x < 82; x += 6) wtk_pset(x, 92, wtk_shade(R_TAN, 6));
  wtk_vgrad(12, 102, 74, 6, R_WOOD, 26, 20);
  wtk_hline(12, 85, 108, wtk_shade(R_WOOD, 4));
  wtk_vgrad(16, 109, 5, 31, R_WOOD, 10, 6);
  wtk_vgrad(78, 109, 5, 31, R_WOOD, 10, 6);
  wtk_vgrad(62, 92, 14, 10, R_CHROME, 24, 10);
  wtk_rect(66, 87, 5, 5, wtk_shade(R_CHROME, 26));
  /* The paper, folded on the bench. */
  int paper[] = {24, 101, 30, 94, 56, 94, 54, 101};
  wtk_poly(paper, 4, wtk_shade(R_CREAM, 27), wtk_shade(R_CREAM, 26));
  wtk_hline(32, 50, 96, wtk_shade(R_RUBBER, 4));
  wtk_hline(30, 52, 98, wtk_shade(R_RUBBER, 12));
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
  car_mesh(&BODIES[B_WEDGE], 0, 0, 256 + 58, PLAYER_PAINT, lamps_up);
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

static void room_tick(void) {
  pointer_move();
  const spot *s = spot_under(GARAGE_SPOTS, N_GARAGE);
  if (wtk_moved) note = 0;
  if ((wtk_hit & KEY_A) && s) {
    const char *why = 0;
    note = 0;
    if (s->does == DO_HOOD) { screen = HOOD; px = 160; py = 100; }
    if (s->does == DO_PAPER) { screen = PAPER; }
    if (s->does == DO_LAMPS) { lamps_up = !lamps_up; wtk_tone(lamps_up ? 440 : 330, 70); }
    if (s->does == DO_DRIVE || s->does == DO_STRIP) {
      if (!car_runs(&why)) { note = why; wtk_tone(90, 200); }
      else if (s->does == DO_DRIVE) { screen = DRIVE; drive_start(); }
      else { screen = STRIP; strip_start(); }
    }
  }
  if (wtk_hit & KEY_B) screen = TITLE;
}

/* ---- Under the hood --------------------------------------------------------------------------- */

static void bolt_at(int s, int b, int *x, int *y) {
  const part *p = &PARTS[s];
  *x = (b == 0 || b == 3) ? p->x + 5 : p->x + p->w - 6;
  *y = b < 2 ? p->y + 5 : p->y + p->h - 6;
}

static int slot_under(void) {
  for (int s = 0; s < N_SLOTS; s++)
    if (px >= PARTS[s].x && px < PARTS[s].x + PARTS[s].w && py >= PARTS[s].y && py < PARTS[s].y + PARTS[s].h) return s;
  return -1;
}

/* The parts on the floor, in a row: where the n-th one is. */
static void chip_at(int n, int *x, int *y) { *x = 4 + (n % 4) * 79; *y = 178 + (n / 4) * 11; }

static int chip_under(void) {
  int n = 0;
  for (int s = 0; s < N_SLOTS; s++)
    for (int l = 0; l < 2; l++)
      if (loose[s][l]) {
        int x, y;
        chip_at(n++, &x, &y);
        if (px >= x && px < x + 76 && py >= y && py < y + 10) return s * 2 + l;
      }
  return -1;
}

static void hood_tick(void) {
  pointer_move();
  if (wtk_hit & KEY_B) {
    if (carrying >= 0) { loose[carrying / 2][carrying % 2] = 1; carrying = -1; }
    screen = GARAGE;
    note = 0;
    return;
  }
  if (!(wtk_hit & KEY_A)) return;
  if (py >= 176) {
    /* The floor: put down what you are holding, or pick something up. */
    int c = chip_under();
    if (carrying >= 0) { loose[carrying / 2][carrying % 2] = 1; carrying = -1; note = "DOWN IT GOES."; }
    else if (c >= 0) { loose[c / 2][c % 2] = 0; carrying = c; note = "NOW SET IT WHERE IT GOES."; wtk_tone(520, 40); }
    return;
  }
  int s = slot_under();
  if (s < 0) return;
  if (fitted[s] >= 0 && carrying < 0) {
    for (int b = 0; b < 4; b++) {
      int x, y;
      bolt_at(s, b, &x, &y);
      if (px >= x - 5 && px <= x + 5 && py >= y - 5 && py <= y + 5) {
        bolts[s] ^= 1 << b;
        wtk_tone(bolts[s] & (1 << b) ? 700 : 400, 50);
        note = 0;
        return;
      }
    }
    if (bolts[s] == 0) { loose[s][fitted[s]] = 1; fitted[s] = -1; note = "IT IS OFF, AND ON THE FLOOR."; wtk_tone(260, 90); }
    else { note = "IT IS BOLTED ON. UNDO THE BOLTS FIRST."; wtk_tone(120, 80); }
  } else if (fitted[s] < 0 && carrying >= 0) {
    if (carrying / 2 == s) { fitted[s] = carrying % 2; bolts[s] = 0; carrying = -1; note = "IT IS ON. NOW DO UP THE BOLTS."; wtk_tone(300, 90); }
    else { note = "THAT DOES NOT GO THERE."; wtk_tone(120, 80); }
  } else if (fitted[s] < 0) note = "NOTHING ON IT. PICK A PART UP OFF THE FLOOR.";
  else note = "THERE IS ONE ON THERE ALREADY.";
}

static void chip(int x, int y, int which, const char *name) {
  wtk_rect(x, y, 76, 10, wtk_shade(which ? R_GOLD : R_GREY, which ? 20 : 12));
  wtk_box(x, y, 76, 10, wtk_shade(R_RUBBER, 2));
  wtk_text(x + 2, y + 1, name, which ? wtk_shade(R_RUBBER, 2) : WHITE);
}

static void hood_draw(void) {
  /* The engine bay is a picture (art/engine-bay.png, by Mikey Nichols, of his own); the parts
     you can work on are places on it. */
  wtk_rle(ART_BAY, 0, 0, 0);

  int over = py < 176 ? slot_under() : -1;
  for (int s = 0; s < N_SLOTS; s++) {
    const part *p = &PARTS[s];
    if (fitted[s] < 0) {
      /* Off the car: a dark hole where it was. */
      wtk_dither(p->x, p->y, p->w, p->h, wtk_shade(R_RUBBER, 1), wtk_shade(R_NAVY, 4));
      wtk_box(p->x, p->y, p->w, p->h, wtk_shade(R_CHROME, 12));
    } else if (fitted[s] == 1) {
      /* The better part is on: a tag on it says so. */
      wtk_rect(p->x + p->w / 2 - 9, p->y + p->h / 2 - 5, 19, 10, wtk_shade(R_GOLD, 24));
      wtk_box(p->x + p->w / 2 - 9, p->y + p->h / 2 - 5, 19, 10, wtk_shade(R_RUBBER, 2));
      wtk_text(p->x + p->w / 2 - 7, p->y + p->h / 2 - 4, "+1", wtk_shade(R_RUBBER, 2));
    }
    if (s == over) { wtk_box(p->x - 1, p->y - 1, p->w + 2, p->h + 2, wtk_shade(R_GOLD, 28)); wtk_box(p->x - 2, p->y - 2, p->w + 4, p->h + 4, wtk_shade(R_RUBBER, 1)); }
    for (int b = 0; b < 4; b++) {
      int x, y;
      bolt_at(s, b, &x, &y);
      if (fitted[s] >= 0 && (bolts[s] & (1 << b))) {
        wtk_disc(x, y, 3, wtk_shade(R_RUBBER, 2));
        wtk_disc(x, y, 2, wtk_shade(R_CHROME, 27));
        wtk_pset(x, y, wtk_shade(R_CHROME, 10));
      } else {
        wtk_disc(x, y, 2, wtk_shade(R_RUBBER, 1));
        wtk_pset(x, y, wtk_shade(R_CHROME, fitted[s] >= 0 ? 22 : 9));
      }
    }
  }

  /* The floor in front of the car, and what is lying on it. */
  wtk_vgrad(0, 176, WTK_W, 24, R_CONCRETE, 18, 22);
  wtk_hline(0, WTK_W - 1, 176, wtk_shade(R_RUBBER, 2));
  int n = 0;
  for (int s = 0; s < N_SLOTS; s++)
    for (int l = 0; l < 2; l++)
      if (loose[s][l]) { int x, y; chip_at(n++, &x, &y); chip(x, y, l, PARTS[s].chip[l]); }
  if (!n && carrying < 0) wtk_text(8, 184, "NOTHING ON THE FLOOR.", wtk_shade(R_CONCRETE, 8));

  static const char *const tight[5] = {"ALL FOUR BOLTS ARE OUT. CLICK IT TO LIFT IT.", "ONE BOLT TIGHT.", "TWO BOLTS TIGHT.", "THREE BOLTS TIGHT.", "BOLTED DOWN. CLICK A BOLT TO UNDO IT."};
  int count = over >= 0 ? (bolts[over] & 1) + ((bolts[over] >> 1) & 1) + ((bolts[over] >> 2) & 1) + ((bolts[over] >> 3) & 1) : 0;
  int c = py >= 176 ? chip_under() : -1;
  panel(over >= 0 ? PARTS[over].slot : c >= 0 ? PARTS[c / 2].name[c % 2] : "UNDER THE HOOD",
        over >= 0 ? (fitted[over] >= 0 ? PARTS[over].name[fitted[over]] : "NOTHING ON IT.") : carrying >= 0 ? PARTS[carrying / 2].name[carrying % 2] : "POINT AT A PART.",
        note ? note : over >= 0 && fitted[over] >= 0 ? tight[count] : "ENTER TO PICK. ESCAPE SHUTS THE HOOD.");
  if (carrying >= 0) chip(px + 6, py + 10, carrying % 2, PARTS[carrying / 2].chip[carrying % 2]);
  pointer_draw();
}

/* ---- The paper -------------------------------------------------------------------------------- */

static void ad_at(int i, int *x, int *y) { *x = 12 + (i % 2) * 152; *y = 58 + (i / 2) * 68; }

static void paper_tick(void) {
  pointer_move();
  if (wtk_hit & KEY_B) { screen = GARAGE; note = 0; return; }
  if (!(wtk_hit & KEY_A)) return;
  for (int i = 0; i < N_SLOTS; i++) {
    int x, y;
    ad_at(i, &x, &y);
    if (px < x || px >= x + 144 || py < y || py >= y + 62) continue;
    if (owned(i)) note = "YOU HAVE ONE.";
    else if (cash < PARTS[i].price) { note = "YOU CANNOT AFFORD IT."; wtk_tone(120, 80); }
    else { cash -= PARTS[i].price; loose[i][1] = 1; note = "BOUGHT. IT IS ON THE GARAGE FLOOR."; wtk_tone(880, 80); }
  }
}

static void paper_draw(void) {
  col inkc = wtk_shade(R_RUBBER, 3), grey = wtk_shade(R_BROWN, 12);
  wtk_vgrad(0, 0, WTK_W, 200, R_CREAM, 27, 23);
  wtk_text_big(24, 8, "THE VALLEY TRADER", inkc, 2);
  wtk_rect(8, 28, 304, 2, inkc);
  wtk_hline(8, 311, 32, inkc);
  wtk_text(12, 38, "SPEED PARTS", inkc);
  cash_line(208, 38, inkc);
  wtk_hline(8, 311, 50, grey);
  for (int i = 0; i < N_SLOTS; i++) {
    int x, y;
    ad_at(i, &x, &y);
    int over = px >= x && px < x + 144 && py >= y && py < y + 62, have = owned(i);
    if (over) wtk_rect(x, y, 144, 62, wtk_shade(R_CREAM, 30));
    wtk_box(x, y, 144, 62, inkc);
    if (over) wtk_box(x + 1, y + 1, 142, 60, inkc);
    wtk_text(x + 4, y + 5, PARTS[i].name[1], inkc);
    wtk_hline(x + 4, x + 139, y + 15, grey);
    wtk_text(x + 4, y + 20, PARTS[i].pitch, grey);
    int tx = wtk_text_big(x + 4, y + 36, "$", inkc, 2);
    for (int v = PARTS[i].price, div = 100; div; div /= 10) { char dgt[2] = {(char)('0' + v / div % 10), 0}; tx = wtk_text_big(tx, y + 36, dgt, inkc, 2); }
    if (have) {
      wtk_rect(x + 92, y + 38, 44, 13, wtk_shade(R_RED, 20));
      wtk_text(x + 98, y + 41, "SOLD", WHITE);
    }
  }
  wtk_text(12, 190, "A SELLER'S WORD IS A SELLER'S WORD.", grey);
  panel("THE PAPER", note ? note : "PICK AN AD TO BUY WHAT IS IN IT.", "ESCAPE PUTS IT DOWN.");
  pointer_draw();
}

/* ---- All of it -------------------------------------------------------------------------------- */

void garage_tick(void) {
  if (screen == HOOD) hood_tick();
  else if (screen == PAPER) paper_tick();
  else room_tick();
}

void garage_draw(void) {
  if (screen == HOOD) { hood_draw(); return; }
  if (screen == PAPER) { paper_draw(); return; }
  room_draw();
  const spot *s = spot_under(GARAGE_SPOTS, N_GARAGE);
  panel("YOUR CAR", note ? note : s ? s->name : "LOOK ROUND.", "ARROWS OR MOUSE, ENTER TO PICK");
  cash_line(216, 206, wtk_shade(R_LIME, 24));
  pointer_draw();
}
