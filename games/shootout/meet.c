/* The lot by the strip, at night: where the regulars pull up, one at a time.
 *
 * This is Street Rod's drive-in. Someone rolls in alongside; you look his car over, and race him
 * or wave him on. A race is for kicks, for money, or for pink slips, and he has to take the bet.
 * Beat them in order and the last one to turn up is the King: beat him before the Shootout and
 * the season is yours. Each race takes two hours of it. */
#include "game.h"

typedef struct { const char *name, *says; int et; u8 paint; u8 limit; } rival;
/* Twenty-one regulars in three classes of seven, and the King, as Street Rod had. What each runs
   the quarter in, in thousandths of a second, and the biggest bet he will take: 1 for $10, 2 for
   $25, 3 for $50, 4 for $100, 5 for his car. */
static const rival RIVALS[] = {
  {"SKEETER", "MY MAMA'S CAR. DON'T SCRATCH IT.", 16800, R_TAN, 2},
  {"DALE", "PULLED IT OUT OF A FIELD TUESDAY.", 16500, R_GREEN, 2},
  {"JUNIOR", "IT'S GOT A TAPE DECK.", 16200, R_SILVER, 2},
  {"PATTY", "YOU LOOK LOST, HONEY.", 15900, R_VIOLET, 2},
  {"CRICKET", "SHE SMOKES A LITTLE. SO WHAT.", 15600, R_BROWN, 3},
  {"WENDELL", "I GOT WORK IN THE MORNING.", 15300, R_TEAL, 3},
  {"BIG RHONDA", "TEN BUCKS SAYS YOU MISS A SHIFT.", 15000, R_RED, 3},
  {"TWO-BAY TONY", "I BUILT THIS ONE BETWEEN OIL CHANGES.", 14750, R_ORANGE, 3},
  {"LURLENE", "MY EX PAID FOR THE TURBO.", 14500, R_PINK, 3},
  {"HOLLIS", "DON'T LEAN ON THE PAINT.", 14250, R_NAVY, 4},
  {"THE TWINS", "WE TAKE TURNS DRIVING.", 14000, R_LIME, 4},
  {"EARLENE", "SLICKS ARE IN THE TRUNK.", 13800, R_MAGENTA, 4},
  {"COACH", "RUN IT BACK IF YOU DON'T LIKE IT.", 13600, R_GREY, 4},
  {"DOC", "I'LL FIX WHAT YOU BREAK. FOR A FEE.", 13400, R_CREAM, 5},
  {"BOONE", "CASH. NO CHECKS.", 13150, R_FOREST, 5},
  {"SISTER RUTH", "PRIDE GOETH BEFORE THE TREE.", 12950, R_WHITE, 5},
  {"MARGO", "I DON'T LIFT.", 12750, R_CYAN, 5},
  {"THE JUDGE", "COURT IS IN SESSION.", 12600, R_MAROON, 5},
  {"SNAKE", "NICE CAR. BE A SHAME.", 12450, R_YELLOW, 5},
  {"NITRO NAN", "SMELL THAT? THAT'S YOU LOSING.", 12300, R_INDIGO, 5},
  {"THE DEACON", "SAY YOUR PRAYERS, SON.", 12150, R_RUBBER, 5},
  {"THE KING", "PINK SLIPS. NOTHING ELSE.", 11900, R_GOLD, 5},
};
#define N_RIVALS (int)(sizeof RIVALS / sizeof RIVALS[0])
#define KING (N_RIVALS - 1)

static const int STAKES[6] = {0, 10, 25, 50, 100, 0};
static const char *const BETS[6] = {"KICKS", "$10", "$25", "$50", "$100", "PINKS"};
static const char *const DOES[5] = {"RACE", "NEXT", "SOLO", "CRUISE", "HOME"};

static int who, betting, bet;
static const char *said;

void meet_start(void) {
  screen = MEET;
  who = rung < N_RIVALS ? rung : KING;
  betting = 0; bet = 0; said = 0;
}

/* The row of buttons along the bottom of the lot: which one the pointer is on, or -1. */
static int button_under(int n) {
  int px = pointer_x(), py = pointer_y(), w = 312 / n;
  if (py < 178 || py >= 196 || px < 4 || px >= 4 + w * n) return -1;
  return (px - 4) / w;
}

void meet_tick(void) {
  pointer_move();
  if (wtk_hit & KEY_B) { if (betting) betting = 0; else screen = GARAGE; return; }
  if (!(wtk_hit & KEY_A)) return;
  int b = button_under(betting ? 6 : 5);
  if (b < 0) return;
  const rival *r = &RIVALS[who];
  if (betting) {
    /* Will he take it? Kicks, always. Money, up to what he is good for. His car, only if he
       thinks he has you, and the King races for nothing else. */
    if (who == KING && b != 5) { said = "PINK SLIPS OR GO HOME."; wtk_tone(120, 80); return; }
    if (cash < STAKES[b]) { said = "YOU DON'T HAVE THAT KIND OF DOUGH."; wtk_tone(120, 80); return; }
    if (b > r->limit || (b == 5 && who != KING && wtk_rand(&rng) % 3 == 0)) { said = "NO WAY. TOO RICH FOR ME."; wtk_tone(120, 80); return; }
    if (car_gas < 7) { said = "NOT ENOUGH GAS TO RACE. GO FILL UP."; wtk_tone(120, 80); return; }
    bet = b; betting = 0; car_gas -= 5;
    strip_race(r->et, r->paint);
    return;
  }
  said = 0;
  if (b == 0) betting = 1;
  if (b == 1) {
    /* Someone else: anyone you have already beaten, or the one you have not. */
    who = rung > 0 ? (who + 1 + (int)(wtk_rand(&rng) % (u32)(rung < KING ? rung : KING))) % ((rung < KING ? rung : KING) + 1) : 0;
    wtk_tone(300, 60);
  }
  if (b == 2) {
    if (car_gas < 7) { said = "NOT ENOUGH GAS FOR A PASS. GO FILL UP."; wtk_tone(120, 80); return; }
    car_gas -= 5; strip_race(0, 0);
  }
  if (b == 3) { screen = DRIVE; drive_start(); }
  if (b == 4) { car_gas = car_gas > 2 ? car_gas - 2 : 0; screen = GARAGE; }
}

void meet_settle(int won) {
  screen = MEET;
  betting = 0;
  if (won < 0) { hours++; said = "A PASS ON YOUR OWN. NOBODY SAW IT."; return; }
  hours += 2;
  if (won) {
    if (bet == 5 && who == KING) { over_kind = 3; screen = OVER; return; }
    if (bet == 5) {
      /* His car is yours: it goes in the yard, if there is room for it. */
      if (car_gain(RIVALS[who].paint, 62 + who, 40)) said = "YOU JUST WON A HELLUVA CAR. IT IS IN YOUR YARD.";
      else { cash += 300 + who * 20; said = "NO ROOM IN YOUR YARD. THE DEALER TOOK HIS CAR."; }
    }
    else { cash += STAKES[bet]; said = bet ? "PAY UP. HE DID." : "YOU WON. IT WAS FOR KICKS."; }
    /* Beating the next one up brings the one after him out. Kicks do not count. */
    if (who == rung && bet && rung < KING) { rung++; who = rung; if (bet != 5) said = "PAID. WORD GETS ROUND: SOMEONE NEW PULLS IN."; }
  } else {
    if (bet == 5) { car_lost(); screen = GARAGE; return; } /* "Take a walk, sucker." */
    cash -= STAKES[bet];
    said = bet ? "YOU LOSE, SUCKER. PAY THE MAN." : "YOU LOST. IT WAS FOR KICKS.";
  }
}

void meet_draw(void) {
  /* A dark lot under two lamps, the strip's lights beyond the fence. */
  wtk_vgrad(0, 0, WTK_W, 96, R_NAVY, 1, 8);
  for (int i = 0; i < 30; i++) {
    u32 h = (u32)(i + 7) * 2654435761u;
    wtk_pset((int)(h % 320), (int)((h >> 9) % 70), wtk_shade(R_WHITE, 10 + (int)((h >> 20) % 16)));
  }
  wtk_vgrad(0, 86, WTK_W, 10, R_ASPHALT, 2, 5);
  for (int x = 0; x < WTK_W; x += 12) wtk_vline(x, 80, 95, wtk_shade(R_CHROME, 6));
  wtk_hline(0, WTK_W - 1, 80, wtk_shade(R_CHROME, 8));
  for (int i = 0; i < 6; i++) {
    int x = 30 + i * 52;
    wtk_disc(x, 62 - (i & 1) * 6, 2, wtk_shade(R_WHITE, 30));
    wtk_hline(x - 5, x + 5, 62 - (i & 1) * 6, wtk_shade(R_WHITE, 16));
  }
  wtk_vgrad(0, 96, WTK_W, 104, R_ASPHALT, 7, 13);
  for (int k = 0; k < 2; k++) {
    int x = k ? 268 : 44;
    wtk_rect(x, 18, 3, 82, wtk_shade(R_CHROME, 7));
    wtk_rect(x - 10, 14, 23, 5, wtk_shade(R_CHROME, 10));
    wtk_rect(x - 8, 19, 19, 2, wtk_shade(R_CREAM, 30));
    int pool[] = {x - 8, 21, x + 11, 21, x + 70, 176, x - 66, 176};
    wtk_poly(pool, 4, wtk_shade(R_ASPHALT, 12), wtk_shade(R_ASPHALT, 14));
  }
  for (int i = 0; i < 5; i++) wtk_rect(20 + i * 70, 150, 2, 26, wtk_shade(R_WHITE, 16));

  const rival *r = &RIVALS[who];
  car_picture(6, 136, car_paint, 1);
  car_picture(72, 172, r->paint, 1);

  int n = betting ? 6 : 5, w = 312 / n, over = button_under(n);
  for (int i = 0; i < n; i++) {
    const char *label = betting ? BETS[i] : DOES[i];
    int x = 4 + i * w;
    wtk_vgrad(x + 1, 178, w - 2, 18, i == over ? R_GOLD : R_NAVY, i == over ? 26 : 12, i == over ? 18 : 5);
    wtk_box(x + 1, 178, w - 2, 18, wtk_shade(R_CHROME, i == over ? 28 : 14));
    wtk_text_sh(x + w / 2 - wtk_strlen(label) * 4, 183, label, i == over ? BLACK : WHITE, i == over ? wtk_shade(R_GOLD, 28) : BLACK);
  }
  static char runs[24];
  const char *a = "RUNS ABOUT ";
  int k = 0, t = r->et / 100;
  while (*a) runs[k++] = *a++;
  runs[k++] = (char)('0' + t / 100); runs[k++] = (char)('0' + t / 10 % 10); runs[k++] = '.'; runs[k++] = (char)('0' + t % 10); runs[k] = 0;
  panel(r->name, said ? said : betting ? "WHAT ARE WE RACING FOR?" : r->says, betting ? "ESCAPE: NEVER MIND" : runs);
  int tx = wtk_text(216, 206, "CASH $", wtk_shade(R_LIME, 24));
  wtk_num(tx, 206, cash, wtk_shade(R_LIME, 24));
  pointer_draw();
}
