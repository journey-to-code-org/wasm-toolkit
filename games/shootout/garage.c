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

int cash, lamps_up, has_car, car_paint, car_cond, hours, rung, over_kind;
static int fitted[N_SLOTS];   /* -1 for nothing, 0 for the stock part, 1 for the better one */
static int bolts[N_SLOTS];    /* a bit for each of the four bolts that is tight */
static int loose[N_SLOTS][2]; /* the parts that are off the car and on the floor */
static int carrying;          /* the part in your hand, as slot * 2 + which, or -1 */
static const char *note;      /* what just happened, for the bottom line */
static int px = 160, py = 100;
static int page;              /* which page of the paper is open: 0 cars, 1 parts */

void garage_reset(void) {
  /* As Street Rod has it: $750, no car, and the paper in your hand. */
  cash = 750; lamps_up = 0; carrying = -1; note = 0; px = 160; py = 100;
  has_car = 0; car_paint = R_BLUE; car_cond = 70; hours = 0; rung = 0; over_kind = 0; page = 0;
  for (int s = 0; s < N_SLOTS; s++) { fitted[s] = 0; bolts[s] = 15; loose[s][0] = loose[s][1] = 0; }
}

void car_lost(void) {
  has_car = 0; lamps_up = 0;
  for (int s = 0; s < N_SLOTS; s++) { fitted[s] = 0; bolts[s] = 15; }
}

int car_runs(const char **why) {
  if (!has_car) { *why = "YOU HAVE NO CAR. THE PAPER IS ON THE BENCH."; return 0; }
  for (int s = 0; s < N_SLOTS; s++) {
    if (fitted[s] < 0) { *why = PARTS[s].missing; return 0; }
    if (bolts[s] != 15) { *why = PARTS[s].undone; return 0; }
  }
  return 1;
}

int tune_gain(void) { return (100 + (fitted[INTAKE] == 1) * 4 + (fitted[EXHAUST] == 1) * 6 + (fitted[COOLER] == 1) * 5) * car_cond / 100; }
int tune_big_turbo(void) { return fitted[TURBO] == 1; }

static int owned(int s) { return loose[s][1] || fitted[s] == 1 || carrying == s * 2 + 1; }

/* ---- The pointer, and the things it can be on ------------------------------------------------ */

typedef struct { i16 x, y, w, h; const char *name; int does; } spot;
enum { DO_NOTHING, DO_STREET, DO_HOOD, DO_CALENDAR, DO_PAPER, DO_LAMPS };

int pointer_x(void) { return px; }
int pointer_y(void) { return py; }

void pointer_move(void) {
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

void pointer_draw(void) {
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
  {232, 14, 84, 108, "THE GARAGE DOOR: HIT THE STREET", DO_STREET},
  {16, 24, 68, 56, "A WINDOW. IT IS DAYLIGHT.", DO_NOTHING},
  {100, 22, 30, 38, "THE CALENDAR", DO_CALENDAR},
  {132, 30, 100, 26, "A BANNER", DO_NOTHING},
  {6, 86, 92, 34, "THE PAPER ON THE BENCH: CARS AND PARTS", DO_PAPER},
  {128, 118, 76, 44, "THE DOOR: GET IN AND HIT THE STREET", DO_STREET},
  {204, 126, 54, 24, "THE HOOD: GET UNDER IT", DO_HOOD},
  {256, 134, 28, 26, "THE HEADLAMPS", DO_LAMPS},
};
#define N_GARAGE (int)(sizeof GARAGE_SPOTS / sizeof GARAGE_SPOTS[0])

static void room_draw(void) {
  /* The room is a painting, and so is the car in it, side on, the way the old garages were. */
  wtk_rle(ART_GARAGE, 0, 0, 0);
  /* A banner, pinned up. */
  wtk_rect(133, 31, 100, 26, wtk_shade(R_WOOD, 5));
  wtk_vgrad(132, 30, 100, 26, R_WHITE, 27, 22);
  logo(134, 32, wtk_shade(R_RUBBER, 3), wtk_shade(R_RED, 24));
  /* The day, on the calendar. */
  int day = hours / DAY_HOURS + 1;
  wtk_rect(101, 32, 28, 27, wtk_shade(R_CREAM, 26));
  char d[3] = {(char)(day > 9 ? '0' + day / 10 : '0' + day), (char)(day > 9 ? '0' + day % 10 : 0), 0};
  wtk_text_big(day > 9 ? 99 : 107, 37, d, wtk_shade(R_RUBBER, 4), 2);
  if (has_car) car_picture(40, 182, car_paint, lamps_up);
  else wtk_text(132, 160, "EMPTY BAY", wtk_shade(R_CONCRETE, 10));
}

void car_picture(int x, int ground, int paint, int lamps) {
  /* The picture is painted blue. Its body colours are moved to the paint asked for: the mid tones,
     the shadows and the highlights each to their place in that ramp. */
  static u8 to[32];
  static signed char add[32];
  for (int r = 0; r < 32; r++) { to[r] = (u8)r; add[r] = 0; }
  to[R_BLUE] = (u8)paint;
  to[R_NAVY] = (u8)paint; add[R_NAVY] = -9;
  to[R_SKY] = (u8)paint; add[R_SKY] = 4;
  to[R_CYAN] = (u8)paint; add[R_CYAN] = 2;
  wtk_rle_tint(lamps ? ART_CAR_LAMPS : ART_CAR, x, ground - ART_CAR_GROUND, to, add);
}

void panel(const char *a, const char *b, const char *c) {
  wtk_vgrad(0, 200, WTK_W, 40, R_NAVY, 6, 2);
  wtk_box(0, 200, WTK_W, 40, wtk_shade(R_CHROME, 22));
  wtk_box(1, 201, WTK_W - 2, 38, wtk_shade(R_CHROME, 9));
  wtk_text_sh(8, 206, a, wtk_shade(R_GOLD, 26), BLACK);
  wtk_text_sh(8, 217, b, WHITE, BLACK);
  wtk_text(8, 228, c, wtk_shade(R_CHROME, 17));
}

static char daybuf[48];

static const char *days_left(void) {
  /* "DAY 12. THE SHOOTOUT IS IN 79 DAYS." */
  int day = hours / DAY_HOURS + 1, left = LAST_DAY - day, n = 0;
  const char *a = "DAY ", *b = ". THE SHOOTOUT IS IN ", *c = " DAYS.";
  while (*a) daybuf[n++] = *a++;
  if (day > 9) daybuf[n++] = (char)('0' + day / 10);
  daybuf[n++] = (char)('0' + day % 10);
  while (*b) daybuf[n++] = *b++;
  if (left > 9) daybuf[n++] = (char)('0' + left / 10);
  daybuf[n++] = (char)('0' + left % 10);
  while (*c) daybuf[n++] = *c++;
  daybuf[n] = 0;
  return daybuf;
}

static void room_tick(void) {
  /* The season ends at the Shootout, and with no car and no way to buy one there is no season. */
  int floor_parts = 0;
  for (int s = 0; s < N_SLOTS; s++) floor_parts += loose[s][0] + loose[s][1];
  if (hours / DAY_HOURS >= LAST_DAY) { over_kind = 1; screen = OVER; return; }
  if (!has_car && cash < 480 && !floor_parts) { over_kind = 2; screen = OVER; return; }

  pointer_move();
  const spot *s = spot_under(GARAGE_SPOTS, N_GARAGE);
  if (wtk_moved) note = 0;
  if ((wtk_hit & KEY_A) && s) {
    const char *why = 0;
    note = 0;
    if (s->does == DO_PAPER) { screen = PAPER; page = has_car ? 1 : 0; hours++; }
    if (s->does == DO_CALENDAR) note = days_left();
    if (s->does == DO_HOOD || s->does == DO_LAMPS || s->does == DO_STREET) {
      if (!has_car) { note = "YOU HAVE NO CAR. THE PAPER IS ON THE BENCH."; wtk_tone(90, 200); }
      else if (s->does == DO_HOOD) { screen = HOOD; px = 160; py = 100; }
      else if (s->does == DO_LAMPS) { lamps_up = !lamps_up; wtk_tone(lamps_up ? 440 : 330, 70); }
      else if (!car_runs(&why)) { note = why; wtk_tone(90, 200); }
      else { hours++; meet_start(); }
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

/* ---- The paper --------------------------------------------------------------------------------
 * Two pages: used cars, and speed parts. Reading it takes an hour, and it is a new paper every
 * day. You can own one car at a time; the dealer's ad at the foot of the cars page buys yours. */

typedef struct { int paint, cond, price; } used;
static const struct { u8 ramp; const char *name; } PAINTS[8] = {
  {R_BLUE, "BLUE"}, {R_RED, "RED"}, {R_GOLD, "GOLD"}, {R_GREEN, "GREEN"}, {R_SILVER, "SILVER"}, {R_WHITE, "WHITE"}, {R_VIOLET, "PURPLE"}, {R_TEAL, "TEAL"},
};

/* The i-th car in today's paper. */
static used used_car(int i) {
  u32 h = (u32)(hours / DAY_HOURS * 3 + i + 1) * 2654435761u;
  h ^= h >> 13; h *= 0x5BD1E995u; h ^= h >> 15;
  used u = {PAINTS[h % 8].ramp, 55 + (int)((h >> 8) % 26), 0};
  u.price = 150 + u.cond * 6 + (int)((h >> 16) % 60);
  return u;
}

static const char *paint_name(int ramp) {
  for (int i = 0; i < 8; i++) if (PAINTS[i].ramp == ramp) return PAINTS[i].name;
  return "";
}

/* What the dealer gives for yours: six tenths of a car like it, and half of what is bolted to it. */
static int trade_in(void) {
  int v = (150 + car_cond * 6) * 6 / 10;
  for (int s = 0; s < N_SLOTS; s++) if (fitted[s] == 1) v += PARTS[s].price / 2;
  return v;
}

static void ad_at(int i, int *x, int *y) { *x = 12 + (i % 2) * 152; *y = 58 + (i / 2) * 68; }

static void paper_tick(void) {
  pointer_move();
  if (wtk_hit & KEY_B) { screen = GARAGE; note = 0; return; }
  if (!(wtk_hit & KEY_A)) return;
  if (py >= 36 && py < 50 && px >= 200) { page = px >= 258; note = 0; return; }
  if (page == 0) {
    for (int i = 0; i < 3; i++) {
      used u = used_car(i);
      if (py < 54 + i * 40 || py >= 92 + i * 40) continue;
      if (has_car) { note = "ONE CAR AT A TIME. SELL YOURS FIRST."; wtk_tone(120, 80); }
      else if (cash < u.price) { note = "YOU CANNOT AFFORD IT."; wtk_tone(120, 80); }
      else {
        cash -= u.price; has_car = 1; car_paint = u.paint; car_cond = u.cond;
        for (int s = 0; s < N_SLOTS; s++) { fitted[s] = 0; bolts[s] = 15; }
        note = "IT IS YOURS. IT IS IN THE GARAGE."; wtk_tone(880, 80);
      }
      return;
    }
    if (py >= 176 && py < 196 && has_car) {
      cash += trade_in(); car_lost();
      note = "SOLD. YOU WALKED HOME."; wtk_tone(660, 80);
    }
    return;
  }
  for (int i = 0; i < N_SLOTS; i++) {
    int x, y;
    ad_at(i, &x, &y);
    if (px < x || px >= x + 144 || py < y || py >= y + 62) continue;
    if (owned(i)) note = "YOU HAVE ONE.";
    else if (cash < PARTS[i].price) { note = "YOU CANNOT AFFORD IT."; wtk_tone(120, 80); }
    else { cash -= PARTS[i].price; loose[i][1] = 1; note = "BOUGHT. IT IS ON THE GARAGE FLOOR."; wtk_tone(880, 80); }
  }
}

static int dollars(int x, int y, int v, col c, int scale) {
  char buf[8];
  int n = 0, div = 10000;
  buf[n++] = '$';
  while (div > 1 && v / div == 0) div /= 10;
  for (; div; div /= 10) buf[n++] = (char)('0' + v / div % 10);
  buf[n] = 0;
  return scale > 1 ? wtk_text_big(x, y, buf, c, scale) : wtk_text(x, y, buf, c);
}

static void paper_draw(void) {
  col inkc = wtk_shade(R_RUBBER, 3), grey = wtk_shade(R_BROWN, 12);
  wtk_vgrad(0, 0, WTK_W, 200, R_CREAM, 27, 23);
  wtk_text_big(24, 8, "THE VALLEY TRADER", inkc, 2);
  wtk_rect(8, 28, 304, 2, inkc);
  wtk_hline(8, 311, 32, inkc);
  cash_line(12, 38, inkc);
  for (int t = 0; t < 2; t++) {
    int x = 200 + t * 58;
    if (t == page) wtk_rect(x, 36, 54, 13, inkc);
    wtk_box(x, 36, 54, 13, inkc);
    wtk_text(x + (t ? 7 : 11), 39, t ? "PARTS" : "CARS", t == page ? wtk_shade(R_CREAM, 28) : inkc);
  }
  wtk_hline(8, 311, 52, grey);
  if (page == 0) {
    for (int i = 0; i < 3; i++) {
      used u = used_car(i);
      int y = 54 + i * 40, over = py >= y && py < y + 38 && px >= 12 && px < 308;
      if (over) wtk_rect(12, y, 296, 38, wtk_shade(R_CREAM, 30));
      wtk_box(12, y, 296, 38, inkc);
      wtk_rect(18, y + 6, 26, 26, inkc);
      wtk_vgrad(19, y + 7, 24, 24, u.paint, 27, 15);
      wtk_text(52, y + 5, "1990 KAGAMI UMBRA TURBO", inkc);
      int tx = wtk_text(52, y + 16, paint_name(u.paint), grey);
      wtk_text(tx, y + 16, u.cond < 62 ? ". TIRED. AS IS." : u.cond < 72 ? ". RUNS OK." : ". RUNS STRONG.", grey);
      dollars(52, y + 27, u.price, inkc, 1);
      wtk_text(188, y + 27, "CALL AFTER SIX", grey);
    }
    if (has_car) {
      int over = py >= 176 && py < 196;
      if (over) wtk_rect(12, 176, 296, 20, wtk_shade(R_CREAM, 30));
      wtk_box(12, 176, 296, 20, inkc);
      int tx = wtk_text(18, 182, "WE BUY CARS. CASH FOR YOURS: ", inkc);
      dollars(tx, 182, trade_in(), wtk_shade(R_RED, 20), 1);
    } else wtk_text(12, 184, "NEW ADS EVERY DAY.", grey);
  } else {
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
      dollars(x + 4, y + 36, PARTS[i].price, inkc, 2);
      if (have) {
        wtk_rect(x + 92, y + 38, 44, 13, wtk_shade(R_RED, 20));
        wtk_text(x + 98, y + 41, "SOLD", WHITE);
      }
    }
    wtk_text(12, 190, "A SELLER'S WORD IS A SELLER'S WORD.", grey);
  }
  panel("THE PAPER", note ? note : page ? "PICK AN AD TO BUY WHAT IS IN IT." : "PICK A CAR TO BUY IT.", "ESCAPE PUTS IT DOWN.");
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
