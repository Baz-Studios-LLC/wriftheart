//! traits.rs — character traits rolled at creation, 2 good + 2 bad (port of js/traits.js).
//!
//! A trait contributes to the hero's DERIVED stats exactly like the passive tree does
//! (folded into TreeStats by skills_tab::recompute), so damage/speed/drops respect it for
//! free. "Quirk" traits only apply by day or by night (the recompute re-runs on the flip).
//! Stat units match the tree: melee/crit/luck/coin/leech are fractions, move is px/frame,
//! maxhp/defense/iframes are flat.

use crate::worldgen::rng::Mulberry32;

pub struct TraitDef {
    pub key: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub good: bool,
    /// (stat, amount) rows: always-on, night-only, day-only.
    pub stats: &'static [(&'static str, f64)],
    pub night_stats: &'static [(&'static str, f64)],
    pub day_stats: &'static [(&'static str, f64)],
}

macro_rules! t {
    ($key:literal, $name:literal, $desc:literal, $good:literal, $s:expr, $n:expr, $d:expr) => {
        TraitDef { key: $key, name: $name, desc: $desc, good: $good, stats: $s, night_stats: $n, day_stats: $d }
    };
}

/// The full roster, js order (GOOD then BAD).
pub const TRAITS: &[TraitDef] = &[
    t!("strong", "Strong", "+15% damage", true, &[("melee", 0.15)], &[], &[]),
    t!("swift", "Swift", "+15% move speed", true, &[("move", 0.19)], &[], &[]),
    t!("lucky", "Lucky", "+15% loot drops", true, &[("luck", 0.15)], &[], &[]),
    t!("hardy", "Hardy", "+2 max HP", true, &[("maxhp", 2.0)], &[], &[]),
    t!("keen", "Keen Eye", "+8% crit chance", true, &[("crit", 0.08)], &[], &[]),
    t!("stalwart", "Stalwart", "+1 armor", true, &[("defense", 1.0)], &[], &[]),
    t!("wealthy", "Wealthy", "+20% gold", true, &[("coin", 0.2)], &[], &[]),
    t!("vampiric", "Vampiric", "chance to heal on hit", true, &[("leech", 0.06)], &[], &[]),
    t!("mending", "Deft Hands", "+10% crafting", true, &[("craft", 0.10)], &[], &[]),
    t!("nightowl", "Night Owl", "+25% damage at night", true, &[], &[("melee", 0.25)], &[]),
    t!("sunchild", "Sun Child", "swift, lucky by day", true, &[], &[], &[("move", 0.13), ("luck", 0.1)]),
    t!("frail", "Frail", "-2 max HP", false, &[("maxhp", -2.0)], &[], &[]),
    t!("sluggish", "Sluggish", "-15% move speed", false, &[("move", -0.19)], &[], &[]),
    t!("clumsy", "Clumsy", "-8% crit chance", false, &[("crit", -0.08)], &[], &[]),
    t!("brittle", "Brittle", "-1 armor", false, &[("defense", -1.0)], &[], &[]),
    t!("cursed", "Cursed", "-15% loot drops", false, &[("luck", -0.15)], &[], &[]),
    t!("feeble", "Feeble", "-15% damage", false, &[("melee", -0.15)], &[], &[]),
    t!("spendthrift", "Spendthrift", "-20% gold", false, &[("coin", -0.2)], &[], &[]),
    t!("reckless", "Reckless", "shorter mercy i-frames", false, &[("iframes", -20.0)], &[], &[]),
    t!("nyctophobe", "Nyctophobe", "weak, slow at night", false, &[], &[("melee", -0.2), ("move", -0.1)], &[]),
    t!("daydreamer", "Daydreamer", "-15% damage by day", false, &[], &[], &[("melee", -0.15)]),
];

/// Mirror pairs that touch the SAME stat with opposite sign — a roll never pairs a bad
/// trait with the good it undoes (js OPP, both directions).
const OPP: &[(&str, &str)] = &[
    ("strong", "feeble"),
    ("swift", "sluggish"),
    ("lucky", "cursed"),
    ("hardy", "frail"),
    ("keen", "clumsy"),
    ("stalwart", "brittle"),
    ("wealthy", "spendthrift"),
    ("nightowl", "nyctophobe"),
];

pub fn get(key: &str) -> Option<&'static TraitDef> {
    TRAITS.iter().find(|t| t.key == key)
}

fn mirror_of(key: &str) -> Option<&'static str> {
    OPP.iter().find_map(|(g, b)| {
        if *g == key { Some(*b) } else if *b == key { Some(*g) } else { None }
    })
}

/// Sum the rolled traits' contribution to one derived stat (day/night aware).
pub fn stat(keys: &[String], night: bool, name: &str) -> f64 {
    let mut total = 0.0;
    for key in keys {
        let Some(d) = get(key) else { continue };
        let pick = |rows: &[(&str, f64)]| rows.iter().find(|(s, _)| *s == name).map_or(0.0, |(_, v)| *v);
        total += pick(d.stats);
        total += pick(if night { d.night_stats } else { &[] });
        total += pick(if night { &[] } else { d.day_stats });
    }
    total
}

fn pick_n(pool: &[&'static TraitDef], n: usize, rng: &mut Mulberry32) -> Vec<String> {
    let mut a: Vec<&TraitDef> = pool.to_vec();
    let mut out = Vec::new();
    for _ in 0..n.min(a.len()) {
        let i = (rng.next_f64() * a.len() as f64) as usize;
        out.push(a.remove(i.min(a.len() - 1)).key.to_string());
    }
    out
}

/// Roll a fresh set: 2 good + 2 bad, no bad trait that simply undoes a rolled good.
pub fn roll(rng: &mut Mulberry32) -> Vec<String> {
    let good_pool: Vec<&TraitDef> = TRAITS.iter().filter(|t| t.good).collect();
    let mut out = pick_n(&good_pool, 2, rng);
    let banned: Vec<&str> = out.iter().filter_map(|k| mirror_of(k)).collect();
    let bad_pool: Vec<&TraitDef> =
        TRAITS.iter().filter(|t| !t.good && !banned.contains(&t.key)).collect();
    out.extend(pick_n(&bad_pool, 2, rng));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every roll: 2 good + 2 bad, all distinct, and never a good with its mirror bad.
    #[test]
    fn roll_shape_and_no_mirrors() {
        for seed in 0..200u32 {
            let mut rng = Mulberry32::new(seed);
            let set = roll(&mut rng);
            assert_eq!(set.len(), 4);
            let goods: Vec<_> = set.iter().filter(|k| get(k).unwrap().good).collect();
            assert_eq!(goods.len(), 2, "seed {seed}: {set:?}");
            for k in &set {
                if let Some(m) = mirror_of(k) {
                    assert!(!set.iter().any(|x| x == m), "seed {seed}: mirror pair in {set:?}");
                }
            }
            let mut dedup = set.clone();
            dedup.dedup();
            assert_eq!(dedup.len(), 4);
        }
    }

    /// Day/night quirks only count on their side of the clock.
    #[test]
    fn quirks_gate_on_night() {
        let keys = vec!["nightowl".to_string(), "daydreamer".to_string()];
        assert_eq!(stat(&keys, true, "melee"), 0.25); // owl on, daydreamer off
        assert_eq!(stat(&keys, false, "melee"), -0.15); // owl off, daydreamer on
        assert_eq!(stat(&keys, false, "luck"), 0.0);
    }
}
