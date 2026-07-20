//! people.rs — names, tastes, and friendship math for town villagers (port of
//! js/people.js; the data tables live in people_data.rs, GENERATED).
//!
//! Villagers are already SEED-STABLE (the same soul stands in the same spot every
//! visit), so identity costs nothing: the seed that picks their look also picks their
//! name, their gift tastes, and their voice. The relationship LEDGER (key -> points)
//! lives app-side (app/talk.rs), raised by daily chats — 100 points = 1 heart, 10 max.
//! Every bit-mix here is pinned to the js by tests/people_parity.rs.

use crate::people_data::{CONFIDANT, FRIENDLY, NAMES_F, NAMES_M, PROF};

pub const BDAY_HEARTS: i32 = 2; // hearts needed before you know their birthday
pub const BDAY_MULT: i32 = 4; // a (liked/loved) gift on the day is worth this much more
pub const HEART_PTS: i32 = 100;
pub const MAX_PTS: i32 = 1000;

pub fn gender_for(seed: u32) -> &'static str {
    if ((seed >> 6) ^ (seed >> 15)) & 1 == 1 { "F" } else { "M" }
}

pub fn name_for(seed: u32) -> &'static str {
    let pool = if gender_for(seed) == "F" { NAMES_F } else { NAMES_M };
    pool[(((seed >> 3) ^ (seed >> 13)) as usize) % pool.len()]
}

/// Keepers get their trade in the name — "MARA THE SMITH" (building kind -> style).
pub fn title_for(seed: u32, kind: &str) -> String {
    match PROF.iter().find(|(k, _)| *k == kind) {
        Some((_, prof)) => format!("{} {}", name_for(seed), prof),
        None => name_for(seed).to_string(),
    }
}

/// Season 0-3 (matches the calendar's SEASONS) + day 1-28, from identity bits chosen
/// to avoid correlating with name/gender/taste (js birthdayFor).
pub struct Birthday {
    pub season: i32,
    pub day: i32,
}

pub fn birthday_for(seed: u32) -> Birthday {
    Birthday {
        season: (((seed >> 9) ^ (seed >> 19)) & 3) as i32,
        day: (((seed >> 1) ^ (seed >> 23)) % 28 + 1) as i32,
    }
}

/// Gift tastes — one loved and one disliked item CATEGORY per person (js tasteFor).
/// Categories match item `kind` fields so any item resolves cleanly.
pub const TASTES: [&str; 6] = ["FISH", "CROP", "FOOD", "GEM", "TRINKET", "MAP"];

pub struct Taste {
    pub love: &'static str,
    pub hate: &'static str,
}

pub fn taste_for(seed: u32) -> Taste {
    let l = (((seed >> 5) ^ (seed >> 11)) as usize) % TASTES.len();
    let mut h = (((seed >> 7) ^ (seed >> 17)) as usize) % TASTES.len();
    if h == l {
        h = (h + 1) % TASTES.len();
    }
    Taste { love: TASTES[l], hate: TASTES[h] }
}

/// Points a gift is worth to this person (js giftPts: loved / disliked / merely polite).
pub fn gift_pts(kind: Option<&str>, taste: &Taste) -> i32 {
    match kind {
        None => 25,
        Some(k) if k == taste.love => 150,
        Some(k) if k == taste.hate => -30,
        Some(_) => 50,
    }
}

/// Friendly words for a taste category (codex lines, chat hints, reactions).
pub fn taste_word(kind: &str) -> &str {
    match kind {
        "CROP" => "CROPS",
        "FOOD" => "COOKED MEALS",
        "GEM" => "GEMS",
        "TRINKET" => "TRINKETS",
        "MAP" => "OLD MAPS",
        other => other, // FISH stays FISH
    }
}

pub fn hearts(pts: i32) -> i32 {
    (pts.max(0) / HEART_PTS).clamp(0, 10)
}

/// Dialogue by friendship tier: strangers keep their stock line; friends warm up;
/// confidants tell you things. Deterministic per person+day so lines don't flicker.
/// (js-verbatim — parity-pinned; the stranger VARIETY lives in [`greeting`].)
pub fn line_for(seed: u32, pts: i32, day: i64, stock_line: &str) -> String {
    let h = hearts(pts);
    let idx = |pool: &[&str]| ((seed >> 2) as i64 + day).rem_euclid(pool.len() as i64) as usize;
    if h >= 7 {
        return CONFIDANT[idx(CONFIDANT)].to_string();
    }
    if h >= 3 {
        return FRIENDLY[idx(FRIENDLY)].to_string();
    }
    if stock_line.is_empty() { "HELLO THERE.".to_string() } else { stock_line.to_string() }
}

/// PORT-ORIGINAL (Baz, 2026-07-16 — not in the js): small talk for the not-yet-friends,
/// so low-heart villagers don't repeat one stock line forever. Alternates with the stock
/// line day by day, deterministic per person+day like every other line roll.
pub const STRANGER: [&str; 16] = [
    "FINE WEATHER FOR IT.",
    "NEW FACE. WELCOME.",
    "THE ROADS ARE ROUGH OF LATE.",
    "MIND THE WOODS AFTER DARK.",
    "GOOD DAY FOR AN HONEST WALK.",
    "YOU LOOK FAR FROM HOME.",
    "WE KEEP TO OURSELVES HERE.",
    "THE BELLS RANG ODD THIS MORNING.",
    "TRADERS COME THROUGH LESS AND LESS.",
    "WATCH YOUR COIN AROUND STRANGERS.",
    "THE HARVEST WAS THIN THIS YEAR.",
    "SOME SAY THE RIFTS ARE WAKING.",
    "KEEP YOUR BLADE CLOSE OUT THERE.",
    "LOVELY MORNING, IS IT NOT?",
    "I HAVE NOT SEEN YOU BEFORE.",
    "SAFE TRAVELS, STRANGER.",
];

/// What they actually say when spoken to: [`line_for`]'s tiers, except that STRANGERS
/// (under 3 hearts) alternate their stock line with the small-talk pool.
pub fn greeting(seed: u32, pts: i32, day: i64, stock_line: &str) -> String {
    if hearts(pts) < 3 && (((seed >> 3) as i64) + day) & 1 == 1 {
        return STRANGER[(((seed >> 2) as i64 + day).rem_euclid(STRANGER.len() as i64)) as usize].to_string();
    }
    line_for(seed, pts, day, stock_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hearts_ladder() {
        assert_eq!(hearts(0), 0);
        assert_eq!(hearts(99), 0);
        assert_eq!(hearts(100), 1);
        assert_eq!(hearts(MAX_PTS), 10);
        assert_eq!(hearts(-5), 0);
    }

    #[test]
    fn taste_never_loves_and_hates_the_same() {
        for seed in 0..2000u32 {
            let t = taste_for(seed.wrapping_mul(0x9e3779b9));
            assert_ne!(t.love, t.hate, "seed={seed}");
        }
    }

    #[test]
    fn strangers_mix_small_talk_but_friends_do_not() {
        // Below 3 hearts the greeting alternates stock line and the STRANGER pool.
        let days: Vec<String> = (0..8).map(|d| greeting(1337, 0, d, "WELCOME IN.")).collect();
        assert!(days.iter().any(|l| l == "WELCOME IN."), "the stock line still appears");
        assert!(days.iter().any(|l| STRANGER.contains(&l.as_str())), "small talk appears");
        // At 3+ hearts the tiers own it — greeting == line_for exactly.
        for d in 0..8 {
            assert_eq!(greeting(1337, 350, d, "WELCOME IN."), line_for(1337, 350, d, "WELCOME IN."));
        }
    }

    #[test]
    fn name_matches_gender_pool() {
        for seed in 0..500u32 {
            let s = seed.wrapping_mul(2654435761);
            let name = name_for(s);
            let pool = if gender_for(s) == "F" { NAMES_F } else { NAMES_M };
            assert!(pool.contains(&name));
        }
    }
}
