//! A deck of cards, for a poker run: a card is a number from 0 to 51, thirteen to a suit.

const RANKS: [&str; 13] = ["two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "jack", "queen", "king", "ace"];
const SUITS: [&str; 4] = ["clubs", "diamonds", "hearts", "spades"];

pub fn card(c: u8) -> String {
    format!("{} of {}", RANKS[c as usize % 13], SUITS[c as usize / 13 % 4])
}

/// Two letters: KS is the king of spades, TH the ten of hearts.
pub fn short(c: u8) -> String {
    format!("{}{}", b"23456789TJQKA"[c as usize % 13] as char, b"CDHS"[c as usize / 13 % 4] as char)
}

/// How good a hand is, so that a better hand is a greater value: what kind it is (8 a straight
/// flush, down to 0 nothing), then its cards in the order that settles a tie.
pub fn rank(hand: &[u8]) -> Vec<u8> {
    let mut counts = [0u8; 13];
    for &c in hand {
        counts[c as usize % 13] += 1;
    }
    // Most of a kind first, and the higher card first among equals.
    let mut groups: Vec<(u8, u8)> = (0..13u8).filter(|&r| counts[r as usize] > 0).map(|r| (counts[r as usize], r)).collect();
    groups.sort_by(|a, b| b.cmp(a));
    let mut order: Vec<u8> = groups.iter().map(|g| g.1).collect();
    let flush = hand.len() == 5 && hand.iter().all(|&c| c / 13 == hand[0] / 13);
    let mut straight = groups.len() == 5 && order[0] - order[4] == 4;
    if order == [12, 3, 2, 1, 0] {
        // The wheel: the ace is the low card.
        (straight, order) = (true, vec![3, 2, 1, 0, 0]);
    }
    let shape: Vec<u8> = groups.iter().map(|g| g.0).collect();
    let kind = match (shape.as_slice(), straight, flush) {
        (_, true, true) => 8,
        ([4, ..], ..) => 7,
        ([3, 2], ..) => 6,
        (_, _, true) => 5,
        (_, true, _) => 4,
        ([3, ..], ..) => 3,
        ([2, 2, ..], ..) => 2,
        ([2, ..], ..) => 1,
        _ => 0,
    };
    let mut out = vec![kind];
    out.extend(order);
    out
}

/// What a ranked hand is called.
pub fn name(rank: &[u8]) -> String {
    let high = RANKS[rank.get(1).copied().unwrap_or(0) as usize];
    let plural = |r: &str| if r == "six" { "sixes".to_string() } else { format!("{r}s") };
    match rank[0] {
        8 => "A straight flush".into(),
        7 => format!("Four {}", plural(high)),
        6 => "A full house".into(),
        5 => "A flush".into(),
        4 => "A straight".into(),
        3 => format!("Three {}", plural(high)),
        2 => "Two pair".into(),
        1 => format!("A pair of {}", plural(high)),
        _ => format!("{} high", high[..1].to_uppercase() + &high[1..]),
    }
}
