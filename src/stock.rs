//! stock.rs — vendor stock builders (port of js/items.js shopStock/wildStock). The
//! tables live in stock_tables.rs (GENERATED — the js pushes blueprint ids into STOCK
//! at module init, so they're extracted from the live module, not the source literal).
//!
//! REGISTRY RULE (same as the loot pools): the js selection runs verbatim — full lists,
//! same rng draws — and only THEN drops ids this registry doesn't know yet. As items
//! port, every shelf converges to the js shop bit-for-bit without touching this file.
//!
//! DEVIATION (flagged): gear vendors (blacksmith/armory/fletcher) roll 4 extra
//! PROCEDURAL wares in the js — that joins with the item-generator port. The rolls
//! happen after the fixed selection, so skipping them changes no draw here.

use crate::stock_tables::{GUARANTEED, STOCK, WILD_CATS, WILD_POOLS};

/// One shelf row: the item + its asking price (js `{ id, price }`).
#[derive(Clone, Debug, PartialEq)]
pub struct ShopEntry {
    pub id: &'static str,
    pub price: i32,
}

/// The js inline xxhash-style rng both builders share: two mixes per draw, /2^32.
struct StockRng(u32);

impl StockRng {
    fn new(seed: u32, fallback: u32) -> Self {
        Self(if seed == 0 { fallback } else { seed })
    }
    fn next(&mut self) -> f64 {
        let mut s = self.0;
        s = (s ^ (s >> 15)).wrapping_mul(2_246_822_519);
        s ^= s >> 13;
        self.0 = s;
        s as f64 / 4_294_967_296.0
    }
}

fn table<'a>(rows: &'a [(&str, &'a [&'static str])], key: &str) -> Option<&'a [&'static str]> {
    rows.iter().find(|(k, _)| *k == key).map(|(_, l)| *l)
}

/// The fixed id selection — guaranteed staples + a curated 4 of the shuffled catalogue —
/// BEFORE the registry filter (pinned to the js draw-for-draw; tests/stock_parity.rs).
pub fn shop_stock_ids(kind: &str, seed: u32) -> Vec<&'static str> {
    let mut list: Vec<&'static str> = table(STOCK, kind).unwrap_or_else(|| table(STOCK, "general").unwrap()).to_vec();
    let mut rng = StockRng::new(seed, 0x9e37_79b1);
    for i in (1..list.len()).rev() {
        let j = (rng.next() * (i + 1) as f64) as usize;
        list.swap(i, j);
    }
    let guaranteed = table(GUARANTEED, kind).unwrap_or(&[]);
    let mut ids: Vec<&'static str> = guaranteed.to_vec();
    ids.extend(list.into_iter().filter(|id| !guaranteed.contains(id)).take(4));
    ids
}

/// A themed town vendor's shelf (js shopStock). `_zone` gates the PROCEDURAL wares'
/// tier — unused until the item generator ports.
pub fn shop_stock(kind: &str, seed: u32, zone: i32) -> Vec<ShopEntry> {
    let mut out: Vec<ShopEntry> = shop_stock_ids(kind, seed)
        .into_iter()
        .filter(|id| crate::items::get(id).is_some())
        .map(|id| ShopEntry { id, price: crate::items::price_of(id) })
        .collect();
    // Gear vendors are ALL procedural (js GEN_VENDOR): 4 rolled wares up to the zone's
    // tier ceiling, commons included — the deterministic id keeps the shelf stable.
    let gen_kind = match kind {
        "blacksmith" | "fletcher" => Some(crate::procgen::Kind::Weapon),
        "armory" => Some(crate::procgen::Kind::Armor),
        _ => None,
    };
    if let Some(gk) = gen_kind {
        let max_t = (zone.clamp(0, 6) / 2).clamp(0, 3); // deeper zones unlock rarer rolls
        let mut r = StockRng::new(seed ^ 0x6ea3_10cf, 0x51ed_270b);
        for i in 0..4u32 {
            let tier = (r.next() * (max_t + 1) as f64) as i32;
            let ware_seed = seed ^ (i + 1).wrapping_mul(2_654_435_761);
            let id = crate::procgen::generate(gk, tier, ware_seed);
            out.push(ShopEntry { id, price: crate::items::price_of(id) });
        }
    }
    out
}

/// A WILD shop's scavenged haul (js wildStock): 4-5 wares drawn across every category,
/// rarity gated by the land's depth (`boost` skews cave stashes a notch richer). The
/// category pools filter to the registry BEFORE the pick — the js filters against ITS
/// registry the same way, so the algorithm is verbatim and the results converge as
/// items port.
pub fn wild_stock(seed: u32, zone: i32, boost: f64) -> Vec<ShopEntry> {
    let z = zone as f64 + boost;
    let mut rng = StockRng::new(seed, 0x0123_4567);
    let pick_cat = {
        let total: f64 = WILD_CATS.iter().map(|(_, w)| w).sum();
        move |rng: &mut StockRng| {
            let mut r = rng.next() * total;
            for (cat, w) in WILD_CATS {
                r -= w;
                if r <= 0.0 {
                    return *cat;
                }
            }
            WILD_CATS[0].0
        }
    };
    // Target rarity tier (0 common .. 3 epic) — higher tiers unlock (and grow) with depth.
    let roll_rarity = |rng: &mut StockRng| {
        let w = [12.0, 5.0 + z * 0.5, (z - 1.0).max(0.0) * 1.2, (z - 3.0).max(0.0) * 1.6];
        let total: f64 = w.iter().sum();
        let mut r = rng.next() * total;
        for (i, wi) in w.iter().enumerate() {
            r -= wi;
            if r <= 0.0 {
                return i as i32;
            }
        }
        0
    };
    let count = 4 + (rng.next() * 2.0) as usize; // 4 or 5
    let mut chosen: Vec<&'static str> = Vec::new();
    let mut guard = 0;
    while chosen.len() < count && guard < 60 {
        guard += 1;
        let cat = pick_cat(&mut rng);
        let rt = roll_rarity(&mut rng);
        if cat == "weapon" || cat == "armor" {
            // PROCEDURAL wares (js generates here): a rolled weapon/armor at the target tier.
            let gk = if cat == "weapon" { crate::procgen::Kind::Weapon } else { crate::procgen::Kind::Armor };
            let ware_seed = seed ^ (chosen.len() as u32 + 7).wrapping_mul(2_654_435_761);
            let id = crate::procgen::generate(gk, rt.min(3), ware_seed);
            if !chosen.contains(&id) {
                chosen.push(id);
            }
            continue;
        }
        let pool: Vec<&'static str> = table(WILD_POOLS, cat)
            .unwrap_or(&[])
            .iter()
            .filter(|id| crate::items::get(id).is_some_and(|d| d.rarity.tier() <= rt))
            .copied()
            .collect();
        if pool.is_empty() {
            continue; // rolled a rarity too rich for this category here — try again
        }
        let id = pool[((rng.next() * pool.len() as f64) as usize).min(pool.len() - 1)];
        if chosen.contains(&id) {
            continue;
        }
        chosen.push(id);
    }
    chosen.into_iter().map(|id| ShopEntry { id, price: crate::items::price_of(id) }).collect()
}

/// A travelling tradesman's wagon (js caravanStock): 6-8 MATERIAL wares in small stacks,
/// the finer stock (iron/silver/gold/mithril/voidsteel, hardwoods, silk, gems) only riding
/// with caravans out in the deeper lands. Registry-filtered + seeded-stable per site.
pub fn caravan_stock(seed: u32, zone: i32) -> Vec<ShopEntry> {
    // (mats, qty-lo, qty-hi) unlocked at each depth.
    const TIERS: &[(i32, &[&str], i32, i32)] = &[
        (0, &["wood", "stone", "fiber", "copper", "herb", "leather"], 4, 8),
        (2, &["iron", "hardwood", "silver", "rareherb", "silk", "gem"], 2, 4),
        (4, &["gold", "ironbark", "mithril", "petalwood", "greenheart"], 1, 3),
        (5, &["voidwood", "voidsteel"], 1, 2),
    ];
    let mut pool: Vec<(&'static str, i32, i32)> = Vec::new();
    for (z, mats, lo, hi) in TIERS {
        if zone >= *z {
            for id in *mats {
                if crate::items::get(id).is_some() {
                    pool.push((id, *lo, *hi));
                }
            }
        }
    }
    let mut rng = StockRng::new(seed, 0x2b1de3);
    for i in (1..pool.len()).rev() {
        let j = (rng.next() * (i + 1) as f64) as usize;
        pool.swap(i, j);
    }
    let count = pool.len().min(6 + (rng.next() * 3.0) as usize);
    pool.into_iter()
        .take(count)
        .map(|(id, lo, hi)| {
            let qty = lo + (rng.next() * (hi - lo + 1) as f64) as i32;
            ShopEntry { id, price: (crate::items::price_of(id) * qty).max(1) }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shop_stock_is_stable_and_guaranteed_first() {
        let a = shop_stock("general", 1337, 3);
        let b = shop_stock("general", 1337, 3);
        assert_eq!(a, b, "same seed -> same shelf");
        // The registered general staples lead the shelf (herb, meat port with this increment).
        assert_eq!(a[0].id, "herb");
        assert_eq!(a[1].id, "meat");
        assert!(a.iter().all(|e| e.price > 0));
    }

    #[test]
    fn different_seeds_curate_differently() {
        // The catalogue picks (past the staples) should differ across seeds somewhere.
        let picks = |seed| -> Vec<&str> { shop_stock_ids("magic", seed).into_iter().skip(2).collect() };
        assert_ne!(picks(1337), picks(90210));
    }

    #[test]
    fn wild_stock_shape() {
        let stock = wild_stock(0xbeef, 2, 0.0);
        assert_eq!(stock, wild_stock(0xbeef, 2, 0.0), "deterministic");
        assert!(stock.len() <= 5);
        for e in &stock {
            assert!(crate::items::get(e.id).is_some(), "wild shelves only carry registered wares");
        }
        // No duplicates.
        let mut ids: Vec<_> = stock.iter().map(|e| e.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), stock.len());
    }
}
