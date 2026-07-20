//! procgen.rs — THE PROCEDURAL ITEM GENERATOR (js items.js generate/genItem/genWeapon/
//! genArmor): the Diablo-style gear engine. Every weapon/armor DROP and gear-vendor ware
//! is a rolled item — base archetype x material ramp x quality x affixes — packed into a
//! stable `~`-prefixed id (low 3 bits rarity tier, bit 3 kind, the rest entropy). An id
//! is a REGENERABLE handle: `resolve()` decodes it, rolls the def with the js Mulberry32
//! stream seeded off the id, and caches it (leaked to 'static, mirroring the js REGISTRY),
//! so the same id always yields the same item. `items::get` resolves `~` ids through here;
//! roll_loot substitutes weapon/armor drops with a fresh roll; the shops roll their wares.
//!
//! Fixed weapon/armor defs (the hand-authored catalog) survive only as shop STAPLES and
//! forge craftables; the LEGENDARY uniques, consumables and trinkets keep their fixed defs.
//! DEVIATION (flagged): the in-hand SWING sprite uses the tool's default art, not a
//! material-recoloured blade (the ICON is material-tinted); that recolour joins a polish
//! pass. Weapon combat stats ride the def's `stats` array (dmg/crit/critmult/leech/knock),
//! read by play.rs's swing branch — armor stats are worn-gear stats the pipeline already sums.

use crate::items::{ItemDef, Rarity, BASE};
use crate::worldgen::rng::Mulberry32;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// What class a roll produces.
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Weapon,
    Armor,
}

/// One material rung: the word, the stat multiplier, and the icon recolour overrides.
struct Mat {
    word: &'static str,
    mul: f64,
    ov: &'static [(char, u32)],
}

/// Metal ramp (js GEN_MATERIALS) — recolours W/A/a/n, scales stats.
const METALS: &[Mat] = &[
    Mat { word: "Rusty", mul: 0.85, ov: &[('W', 0x9a7a5a), ('A', 0x6e5238), ('a', 0x4e3826), ('n', 0x3a281a)] },
    Mat { word: "Iron", mul: 1.00, ov: &[('W', 0xe2e2ea), ('A', 0xa6a6b2), ('a', 0x70707c), ('n', 0x4a4a54)] },
    Mat { word: "Bronze", mul: 1.15, ov: &[('W', 0xf2c476), ('A', 0xc08840), ('a', 0x86541f), ('n', 0x5a3814)] },
    Mat { word: "Steel", mul: 1.32, ov: &[('W', 0xeef2fa), ('A', 0xaeb8cc), ('a', 0x727e96), ('n', 0x4c566a)] },
    Mat { word: "Silver", mul: 1.50, ov: &[('W', 0xffffff), ('A', 0xcdd6e4), ('a', 0x909cb2), ('n', 0x5e6a80)] },
    Mat { word: "Gold", mul: 1.70, ov: &[('W', 0xfff0a0), ('A', 0xf0c040), ('a', 0xb07e18), ('n', 0x7a560c)] },
    Mat { word: "Voidsteel", mul: 1.92, ov: &[('W', 0xd6b8ff), ('A', 0xa878e8), ('a', 0x6a44b0), ('n', 0x3c2470)] },
];

/// Leather ramp (js GEN_LEATHERS) — recolours D/d.
const LEATHERS: &[Mat] = &[
    Mat { word: "Worn", mul: 0.80, ov: &[('D', 0x7a5a34), ('d', 0x4a3418)] },
    Mat { word: "Hide", mul: 0.95, ov: &[('D', 0x946a34), ('d', 0x5a3c1c)] },
    Mat { word: "Leather", mul: 1.10, ov: &[('D', 0xa86e2c), ('d', 0x603c14)] },
    Mat { word: "Studded", mul: 1.28, ov: &[('D', 0x6e5436), ('d', 0x3e2e1c)] },
    Mat { word: "Hardened", mul: 1.45, ov: &[('D', 0x5a4636), ('d', 0x322318)] },
];

/// A weapon archetype (js GEN_WBASES): icon grid, tool, swing tuning, the gem pommel.
struct WBase {
    word: &'static str,
    icon: &'static [&'static str],
    tool: &'static str,
    cd: u32,
    lock: u32,
    damage: f64,
    crit: f64,
    knock: f64,
    pommel: (usize, usize),
}

const WBASES: &[WBase] = &[
    WBase { word: "Sword", icon: &["......WW", ".....WAW", "....WAW.", "...WAW..", "..WAW...", ".DAWD...", "DDDW....", "yD......"], tool: "sword", cd: 20, lock: 14, damage: 2.0, crit: 0.0, knock: 0.0, pommel: (6, 1) },
    WBase { word: "Saber", icon: &["......WW", ".....WA.", "....WA..", "...WA...", "..WA....", ".DDA....", "Dy......", "y......."], tool: "sword", cd: 18, lock: 12, damage: 2.0, crit: 0.0, knock: 0.0, pommel: (6, 1) },
    WBase { word: "Axe", icon: &[".KKKK...", "KAAAaK..", "KAaaanK.", ".KannK..", "..KdK...", "..KDK...", "..KDK...", "..KyK..."], tool: "axe", cd: 30, lock: 18, damage: 3.0, crit: 0.0, knock: 0.0, pommel: (7, 2) },
    WBase { word: "Dagger", icon: &["......W.", ".....WA.", "....WA..", "...WA...", "..WA....", ".DDA....", "Dy......", "........"], tool: "sword", cd: 14, lock: 8, damage: 1.0, crit: 0.12, knock: 0.0, pommel: (5, 1) },
    WBase { word: "Mace", icon: &["..AAA...", ".AAAAA..", ".AAAAA..", "..AAA...", "...D....", "...D....", "...D....", "...y...."], tool: "pick", cd: 30, lock: 18, damage: 3.0, crit: 0.0, knock: 1.0, pommel: (7, 3) },
    WBase { word: "Spear", icon: &["......AA", ".....AA.", "....DA..", "...D....", "..D.....", ".D......", "D.......", "........"], tool: "sword", cd: 24, lock: 14, damage: 2.0, crit: 0.0, knock: 0.0, pommel: (5, 1) },
];

/// An armor archetype (js GEN_ABASES): icon, slot, worn style, base armour, ramp class.
struct ABase {
    word: &'static str,
    icon: &'static [&'static str],
    slot: &'static str,
    kind: &'static str,
    style: &'static str,
    def: f64,
    leather: bool,
    pommel: (usize, usize),
}

const ABASES: &[ABase] = &[
    ABase { word: "Helm", icon: &["..aaaa..", ".aAAAAa.", "aAAAAAAa", "aAKKKKAa", "aAAAAAAa", ".aA..Aa.", "..a..a..", "........"], slot: "head", kind: "HEAD", style: "helm", def: 1.0, leather: false, pommel: (1, 3) },
    ABase { word: "Cuirass", icon: &[".aAAAAa.", "aAAAAAAa", "aAAaaAAa", "aAaAAaAa", "aAAaaAAa", "aAAAAAAa", ".aAAAAa.", "..a..a.."], slot: "body", kind: "BODY", style: "plate", def: 2.0, leather: false, pommel: (3, 3) },
    ABase { word: "Greaves", icon: &["........", ".aAa.aAa", ".aAa.aAa", ".aAa.aAa", ".aAa.aAa", ".aAa.aAa", ".aaa.aaa", ".Kaa.aaK"], slot: "feet", kind: "FEET", style: "greaves", def: 1.0, leather: false, pommel: (1, 2) },
    ABase { word: "Cap", icon: &["........", "..dddd..", ".dDDDDd.", "dDDDDDDd", "dDDDDDDd", ".dddddd.", "........", "........"], slot: "head", kind: "HEAD", style: "cap", def: 1.0, leather: true, pommel: (4, 2) },
    ABase { word: "Jerkin", icon: &[".dDDDDd.", "dDDDDDDd", "dDdDDdDd", "dDDDDDDd", "dDDDDDDd", "dDDDDDDd", ".dDDDDd.", "..d..d.."], slot: "body", kind: "BODY", style: "vest", def: 1.0, leather: true, pommel: (3, 3) },
    ABase { word: "Boots", icon: &["........", ".dDd.dDd", ".dDd.dDd", ".dDd.dDd", ".dDd.dDd", ".dDd.dDd", ".ddd.ddd", ".Kdd.ddK"], slot: "feet", kind: "FEET", style: "boots", def: 1.0, leather: true, pommel: (1, 2) },
];

/// (word, stat-key, value, gem) affix rows. Weapon affixes fold into combat stats.
struct Affix {
    word: &'static str,
    adds: &'static [(&'static str, f64)],
    gem: u32,
}

const W_PREFIX: &[Affix] = &[
    Affix { word: "Vicious", adds: &[("dmg", 1.0)], gem: 0xfc4030 },
    Affix { word: "Honed", adds: &[("crit", 0.10)], gem: 0xe8f0ff },
    Affix { word: "Brutal", adds: &[("critmult", 0.4)], gem: 0xfc4030 },
    Affix { word: "Vampiric", adds: &[("leech", 0.08)], gem: 0xd83048 },
    Affix { word: "Heavy", adds: &[("knock", 2.0), ("dmg", 1.0)], gem: 0xc0c0cc },
    Affix { word: "Keen", adds: &[("crit", 0.06), ("critmult", 0.2)], gem: 0xe8f0ff },
    Affix { word: "Savage", adds: &[("dmg", 1.0), ("knock", 1.0)], gem: 0xfc7030 },
];
const W_SUFFIX: &[Affix] = &[
    Affix { word: "of Fury", adds: &[("dmg", 1.0)], gem: 0xfc4030 },
    Affix { word: "of Leeching", adds: &[("leech", 0.1)], gem: 0xd83048 },
    Affix { word: "of Ruin", adds: &[("critmult", 0.5)], gem: 0xfc7030 },
    Affix { word: "of Force", adds: &[("knock", 2.0)], gem: 0xc0c0cc },
    Affix { word: "of Precision", adds: &[("crit", 0.1)], gem: 0xe8f0ff },
    Affix { word: "of the Wolf", adds: &[("dmg", 1.0), ("crit", 0.05)], gem: 0xcfe0ff },
];
const A_PREFIX: &[Affix] = &[
    Affix { word: "Sturdy", adds: &[("defense", 1.0)], gem: 0x8fe0ff },
    Affix { word: "Stalwart", adds: &[("maxhp", 1.0)], gem: 0x8fe0ff },
    Affix { word: "Nimble", adds: &[("move", 0.08)], gem: 0x8fe0ff },
    Affix { word: "Lucky", adds: &[("luck", 0.06)], gem: 0x8fe0ff },
    Affix { word: "Plated", adds: &[("defense", 1.0)], gem: 0x8fe0ff },
    Affix { word: "Hale", adds: &[("regen", 1.0)], gem: 0x8fe0ff },
];
const A_SUFFIX: &[Affix] = &[
    Affix { word: "of Warding", adds: &[("defense", 1.0)], gem: 0x8fe0ff },
    Affix { word: "of the Bear", adds: &[("maxhp", 1.0)], gem: 0x8fe0ff },
    Affix { word: "of the Fox", adds: &[("move", 0.08)], gem: 0x8fe0ff },
    Affix { word: "of Vigor", adds: &[("regen", 1.0)], gem: 0x8fe0ff },
    Affix { word: "of Greed", adds: &[("coin", 0.12)], gem: 0x8fe0ff },
    Affix { word: "of the Mage", adds: &[("maxmana", 1.0)], gem: 0x8fe0ff },
];

const RARITY_PRICE: [i32; 5] = [30, 120, 400, 1200, 4000];
fn rarity_of(t: i32) -> Rarity {
    [Rarity::Common, Rarity::Uncommon, Rarity::Rare, Rarity::Epic, Rarity::Legendary][t.clamp(0, 4) as usize]
}

fn clampi(v: i32, lo: i32, hi: i32) -> i32 {
    v.clamp(lo, hi)
}

// --- Leak helpers: generated defs live for the process (bounded by unique ids seen). ---
fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}
fn leak_grid(rows: Vec<String>) -> &'static [&'static str] {
    let refs: Vec<&'static str> = rows.into_iter().map(leak_str).collect();
    Box::leak(refs.into_boxed_slice())
}
fn leak_pal(v: Vec<(char, u32)>) -> &'static [(char, u32)] {
    Box::leak(v.into_boxed_slice())
}
fn leak_stats(v: Vec<(&'static str, f64)>) -> &'static [(&'static str, f64)] {
    Box::leak(v.into_boxed_slice())
}

/// Stamp a gem char 'm' on the icon's pommel (js gemmedIcon).
fn gemmed(icon: &[&str], pommel: (usize, usize)) -> Vec<String> {
    icon.iter()
        .enumerate()
        .map(|(r, row)| {
            if r == pommel.0 {
                let mut chars: Vec<char> = row.chars().collect();
                if pommel.1 < chars.len() {
                    chars[pommel.1] = 'm';
                }
                chars.into_iter().collect()
            } else {
                row.to_string()
            }
        })
        .collect()
}

use crate::actors::hero::ArmorLook;

/// (def cache, worn-look cache) keyed by packed seed — the js REGISTRY. The worn look
/// is leaked to 'static so worn_refresh can hand it to the geared-hero baker.
type GenCache = Mutex<(HashMap<u32, &'static ItemDef>, HashMap<u32, &'static ArmorLook>)>;
fn caches() -> &'static GenCache {
    static C: OnceLock<GenCache> = OnceLock::new();
    C.get_or_init(|| Mutex::new((HashMap::new(), HashMap::new())))
}

/// Decode `~<base36>` -> the seed it packs (or None if malformed).
fn seed_of(id: &str) -> Option<u32> {
    let body = id.strip_prefix('~')?;
    u32::from_str_radix(body, 36).ok()
}

/// Resolve a generated id to its (cached, leaked) def — the js genItem.
pub fn resolve(id: &str) -> Option<&'static ItemDef> {
    let seed = seed_of(id)?;
    {
        let c = caches().lock().ok()?;
        if let Some(d) = c.0.get(&seed) {
            return Some(*d);
        }
    }
    let r_tier = clampi((seed & 7) as i32, 0, 3);
    let kind_bit = (seed >> 3) & 1;
    let mut rng = Mulberry32::new(if seed == 0 { 1 } else { seed });
    let (def, look) = if kind_bit == 1 {
        gen_armor(id, r_tier, &mut rng)
    } else {
        (gen_weapon(id, r_tier, &mut rng), None)
    };
    let leaked: &'static ItemDef = Box::leak(Box::new(def));
    let mut c = caches().lock().ok()?;
    c.0.insert(seed, leaked);
    if let Some(l) = look {
        c.1.insert(seed, Box::leak(Box::new(l)));
    }
    Some(leaked)
}

/// The worn look for a generated armor id (worn_refresh consults this after the static
/// table). Returns the leaked &'static ArmorLook the geared-hero baker wants.
pub fn armor_look(id: &str) -> Option<&'static ArmorLook> {
    let seed = seed_of(id)?;
    resolve(id); // ensure it's generated + cached
    caches().lock().ok()?.1.get(&seed).copied()
}

fn pick<'a, T>(rng: &mut Mulberry32, arr: &'a [T]) -> &'a T {
    &arr[(rng.next_f64() * arr.len() as f64) as usize % arr.len()]
}

/// Roll `n` distinct affixes from a pool (js retry-until-unused, up to 8 tries).
fn roll_affixes(rng: &mut Mulberry32, pool: &'static [Affix], n: usize) -> Vec<&'static Affix> {
    let mut out: Vec<&'static Affix> = Vec::new();
    for _ in 0..n {
        let mut a: Option<&Affix> = None;
        for _ in 0..8 {
            let cand = pick(rng, pool);
            if !out.iter().any(|x| x.word == cand.word) {
                a = Some(cand);
                break;
            }
        }
        if let Some(a) = a
            && !out.iter().any(|x| x.word == a.word)
        {
            out.push(a);
        }
    }
    out
}

fn gen_weapon(id: &str, r_tier: i32, rng: &mut Mulberry32) -> ItemDef {
    let rarity = rarity_of(r_tier);
    let base = pick(rng, WBASES);
    let mat_idx = clampi(r_tier + (rng.next_f64() * 3.0) as i32, 0, METALS.len() as i32 - 1) as usize;
    let mat = &METALS[mat_idx];
    let quality = 0.9 + rng.next_f64() * 0.35;
    let (n_suf, n_pre) = if r_tier >= 2 { (1usize, (r_tier - 1) as usize) } else { (0, r_tier as usize) };
    let prefixes = roll_affixes(rng, W_PREFIX, n_pre);
    let suffix = roll_affixes(rng, W_SUFFIX, n_suf);

    // Fold affix combat mods (js addMod).
    let mut crit = base.crit;
    let mut critmult = 0.0;
    let mut leech = 0.0;
    let mut knock = base.knock;
    let mut affix_dmg = 0.0;
    let mut first_gem: Option<u32> = None;
    for a in prefixes.iter().chain(suffix.iter()) {
        for (k, v) in a.adds {
            match *k {
                "dmg" => affix_dmg += v,
                "crit" => crit += v,
                "critmult" => critmult += v,
                "leech" => leech += v,
                "knock" => knock += v,
                _ => {}
            }
        }
        first_gem = first_gem.or(Some(a.gem));
    }
    let dmg = clampi(
        ((base.damage + mat_idx as f64 * 0.3) * quality * mat.mul + affix_dmg).round() as i32,
        1,
        3 + r_tier,
    );

    let grid = first_gem.map(|_| gemmed(base.icon, base.pommel)).unwrap_or_else(|| base.icon.iter().map(|s| s.to_string()).collect());
    let mut pal: Vec<(char, u32)> = mat.ov.to_vec();
    if let Some(g) = first_gem {
        pal.push(('m', g));
    }
    let name = compose_name(&prefixes, mat.word, base.word, &suffix);
    let desc = format!("A {} {} {}.", rarity.name().to_lowercase(), mat.word.to_lowercase(), base.word.to_lowercase());
    let tool = match base.tool {
        "axe" => Some(crate::combat::Tool::Axe),
        "pick" => Some(crate::combat::Tool::Pick),
        _ => Some(crate::combat::Tool::Sword),
    };
    ItemDef {
        id: leak_str(id.to_string()),
        name: leak_str(name),
        icon: leak_grid(grid),
        icon_pal: leak_pal(pal),
        kind: "WEAPON",
        rarity,
        desc: leak_str(desc),
        price: Some((RARITY_PRICE[rarity.tier().clamp(0, 4) as usize] as f64 * mat.mul).round() as i32),
        weapon: true,
        tool,
        cooldown: base.cd,
        lock_frames: base.lock,
        stats: leak_stats(vec![("dmg", dmg as f64), ("crit", crit), ("critmult", critmult), ("leech", leech), ("knock", knock)]),
        ..BASE
    }
}

fn gen_armor(id: &str, r_tier: i32, rng: &mut Mulberry32) -> (ItemDef, Option<ArmorLook>) {
    let rarity = rarity_of(r_tier);
    let base = pick(rng, ABASES);
    let ramp = if base.leather { LEATHERS } else { METALS };
    let mat_idx = clampi(r_tier + (rng.next_f64() * 3.0) as i32, 0, ramp.len() as i32 - 1) as usize;
    let mat = &ramp[mat_idx];
    let (n_suf, n_pre) = if r_tier >= 2 { (1usize, (r_tier - 1) as usize) } else { (0, r_tier as usize) };
    let prefixes = roll_affixes(rng, A_PREFIX, n_pre);
    let suffix = roll_affixes(rng, A_SUFFIX, n_suf);

    // Base armour + affix stats (js addS). defense is summed then rarity-capped.
    let mut acc: Vec<(&'static str, f64)> = vec![("defense", (base.def + mat_idx as f64 * 0.3).round().max(1.0))];
    let bump = |acc: &mut Vec<(&'static str, f64)>, k: &'static str, v: f64| {
        if let Some(e) = acc.iter_mut().find(|(ek, _)| *ek == k) {
            e.1 += v;
        } else {
            acc.push((k, v));
        }
    };
    let affixed = !prefixes.is_empty() || !suffix.is_empty();
    for a in prefixes.iter().chain(suffix.iter()) {
        for (k, v) in a.adds {
            bump(&mut acc, k, *v);
        }
    }
    if let Some(e) = acc.iter_mut().find(|(k, _)| *k == "defense") {
        e.1 = e.1.min((2 + r_tier) as f64); // rarity caps total armour
    }

    let grid = if affixed { gemmed(base.icon, base.pommel) } else { base.icon.iter().map(|s| s.to_string()).collect() };
    let mut pal: Vec<(char, u32)> = mat.ov.to_vec();
    if affixed {
        pal.push(('m', 0x8fe0ff));
    }
    let name = compose_name(&prefixes, mat.word, base.word, &suffix);
    let desc = format!("A {} {} {}.", rarity.name().to_lowercase(), mat.word.to_lowercase(), base.word.to_lowercase());
    // The worn look: material's body colour (A metal / D leather) + its dark (n / d).
    let lite = mat.ov.iter().find(|(c, _)| *c == 'A' || *c == 'D').map(|(_, v)| *v).unwrap_or(0xa6a6b2);
    let dark = mat.ov.iter().find(|(c, _)| *c == 'n' || *c == 'd').map(|(_, v)| *v).unwrap_or(0x4a4a54);
    let def = ItemDef {
        id: leak_str(id.to_string()),
        name: leak_str(name),
        icon: leak_grid(grid),
        icon_pal: leak_pal(pal),
        kind: base.kind,
        rarity,
        desc: leak_str(desc),
        price: Some((RARITY_PRICE[rarity.tier().clamp(0, 4) as usize] as f64 * mat.mul).round() as i32),
        slot: Some(base.slot),
        stats: leak_stats(acc),
        ..BASE
    };
    (def, Some(ArmorLook { style: base.style, lite, dark }))
}

/// "Prefix… Material Base of Suffix" (js name join).
fn compose_name(prefixes: &[&Affix], mat: &str, base: &str, suffix: &[&Affix]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let pre = prefixes.iter().map(|a| a.word).collect::<Vec<_>>().join(" ");
    if !pre.is_empty() {
        parts.push(pre);
    }
    parts.push(mat.to_string());
    parts.push(base.to_string());
    let mut name = parts.join(" ");
    if let Some(s) = suffix.first() {
        name.push(' ');
        name.push_str(s.word);
    }
    name
}

/// Roll a NEW generated id (js generate): pack entropy above the kind+rarity bits.
/// `seed` gives a deterministic roll (shops); None draws fresh entropy off `rand`.
pub fn generate(kind: Kind, tier: i32, entropy: u32) -> &'static str {
    let tier = clampi(tier, 0, 3) as u32;
    let kind_bit = if kind == Kind::Armor { 1u32 } else { 0 };
    let seed = ((entropy & 0x07ff_ffff) << 4) | (kind_bit << 3) | (tier & 7);
    // Cache the id string too (so repeated rolls of the same seed share one 'static).
    let id = format!("~{}", radix36(seed));
    resolve(&id).map(|d| d.id).unwrap_or_else(|| leak_str(id))
}

/// Forge-commission preview cache (keyed by the craftw*/crafta* id).
fn preview_cache() -> &'static Mutex<HashMap<String, &'static ItemDef>> {
    static C: OnceLock<Mutex<HashMap<String, &'static ItemDef>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The material overlay palette for a commission's tier (js CRAFT_TIER_MAT bronze/steel/gold).
fn tier_mat(tier: i32) -> &'static Mat {
    &METALS[match tier {
        1 => 2, // Bronze
        2 => 3, // Steel
        _ => 5, // Gold
    }]
}

/// "RARE" -> "Rare".
fn title_case(s: &str) -> String {
    let low = s.to_lowercase();
    let mut c = low.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Build (once, cached) the forge menu's commission PREVIEW def for a craftw*/crafta* id
/// (js craftGen previews). Never granted as-is — crafting one rolls a real item (do_craft
/// reads `craft_gen` and calls generate_pinned).
pub fn preview(id: &str) -> Option<&'static ItemDef> {
    {
        let c = preview_cache().lock().ok()?;
        if let Some(d) = c.get(id) {
            return Some(*d);
        }
    }
    let tier = id.chars().last()?.to_digit(10)? as i32;
    if !(1..=3).contains(&tier) {
        return None;
    }
    let body = &id[..id.len() - 1];
    let rarity = rarity_of(tier);
    let mat = tier_mat(tier);
    let def = if let Some(word) = body.strip_prefix("craftw") {
        let base = WBASES.iter().find(|b| b.word.eq_ignore_ascii_case(word))?;
        ItemDef {
            id: leak_str(id.to_string()),
            name: leak_str(format!("{} {} (Rolled)", title_case(rarity.name()), base.word)),
            icon: base.icon,
            icon_pal: mat.ov,
            kind: "WEAPON",
            rarity,
            desc: leak_str(format!(
                "Commission a {} {}. What metal and what magic - the forge decides.",
                rarity.name().to_lowercase(),
                base.word.to_lowercase()
            )),
            craft_gen: Some(crate::items::CraftGen { armor: false, base: base.word, tier }),
            ..crate::items::BASE
        }
    } else if let Some(slot) = body.strip_prefix("crafta") {
        let base = ABASES.iter().find(|b| b.slot == slot && !b.leather)?;
        let slot_word = match slot {
            "head" => "Helm",
            "body" => "Armor",
            _ => "Boots",
        };
        ItemDef {
            id: leak_str(id.to_string()),
            name: leak_str(format!("{} {} (Rolled)", title_case(rarity.name()), slot_word)),
            icon: base.icon,
            icon_pal: mat.ov,
            kind: base.kind,
            rarity,
            desc: leak_str(format!(
                "Commission {} {} armor. Metal or leather, plain or charmed - the forge decides.",
                rarity.name().to_lowercase(),
                slot
            )),
            craft_gen: Some(crate::items::CraftGen { armor: true, base: base.slot, tier }),
            ..crate::items::BASE
        }
    } else {
        return None;
    };
    let leaked: &'static ItemDef = Box::leak(Box::new(def));
    preview_cache().lock().ok()?.insert(id.to_string(), leaked);
    Some(leaked)
}

/// Commission a rolled item of a PINNED base/slot (js craftGen: you pick the class, the
/// forge rolls material + magic). Rejection-samples entropy so the seed's first rng draw
/// lands the wanted base — the returned `~` id still round-trips through resolve.
pub fn generate_pinned(kind: Kind, want: &str, tier: i32, mut entropy: u32) -> &'static str {
    let tier_u = clampi(tier, 0, 3) as u32;
    let kind_bit = if kind == Kind::Armor { 1u32 } else { 0 };
    for _ in 0..512 {
        let seed = ((entropy & 0x07ff_ffff) << 4) | (kind_bit << 3) | (tier_u & 7);
        let mut rng = Mulberry32::new(if seed == 0 { 1 } else { seed });
        let hit = if kind == Kind::Armor {
            pick(&mut rng, ABASES).slot == want
        } else {
            pick(&mut rng, WBASES).word == want
        };
        if hit {
            let s = format!("~{}", radix36(seed));
            return resolve(&s).map(|d| d.id).unwrap_or_else(|| leak_str(s));
        }
        entropy = entropy.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    }
    generate(kind, tier, entropy)
}

fn radix36(mut n: u32) -> String {
    if n == 0 {
        return "0".to_string();
    }
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_and_cache() {
        // A generated id resolves, and the same id yields the SAME def (cached).
        let id = generate(Kind::Weapon, 2, 0xdead);
        let a = crate::items::get(id).unwrap();
        let b = crate::items::get(id).unwrap();
        assert!(std::ptr::eq(a, b), "cache returns one leaked def");
        assert_eq!(a.kind, "WEAPON");
        assert!(a.weapon && a.tool.is_some());
        assert!(a.stats.iter().any(|(k, _)| *k == "dmg"));
    }

    #[test]
    fn armor_is_wearable_with_a_look() {
        let id = generate(Kind::Armor, 3, 0xbeef);
        let d = crate::items::get(id).unwrap();
        assert!(matches!(d.slot, Some("head" | "body" | "feet")));
        assert!(d.stats.iter().any(|(k, v)| *k == "defense" && *v >= 1.0));
        assert!(armor_look(id).is_some(), "generated armor carries a worn look");
    }

    #[test]
    fn rarity_and_kind_bits_decode() {
        // tier 0 weapon, tier 3 armor — the low bits drive the class.
        let w = crate::items::get(generate(Kind::Weapon, 0, 1)).unwrap();
        assert_eq!(w.rarity, Rarity::Common);
        let a = crate::items::get(generate(Kind::Armor, 3, 1)).unwrap();
        assert_eq!(a.rarity, Rarity::Epic);
        assert!(a.kind == "HEAD" || a.kind == "BODY" || a.kind == "FEET");
    }

    #[test]
    fn forge_previews_resolve() {
        // Every craftGen preview id (js craftw*/crafta*) resolves to a preview def with a
        // craft_gen marker of the right class + tier, cached to one leaked def.
        let d = crate::items::get("craftwsword2").unwrap();
        assert_eq!(d.kind, "WEAPON");
        assert_eq!(d.rarity, Rarity::Rare); // tier 2
        let cg = d.craft_gen.expect("preview carries a craftGen spec");
        assert!(!cg.armor && cg.base == "Sword" && cg.tier == 2);
        assert!(std::ptr::eq(d, crate::items::get("craftwsword2").unwrap()), "preview is cached");
        let a = crate::items::get("craftahead1").unwrap().craft_gen.unwrap();
        assert!(a.armor && a.base == "head" && a.tier == 1);
    }

    #[test]
    fn commission_pins_the_base() {
        // generate_pinned rolls the WANTED base/slot; the id still round-trips through resolve.
        for e in [1u32, 7, 42, 1000, 999_999] {
            let sword = crate::items::get(generate_pinned(Kind::Weapon, "Axe", 2, e)).unwrap();
            assert!(sword.name.contains("Axe"), "pinned weapon is an Axe: {}", sword.name);
            let boots = crate::items::get(generate_pinned(Kind::Armor, "feet", 1, e)).unwrap();
            assert_eq!(boots.slot, Some("feet"), "pinned armor is feet: {}", boots.name);
        }
    }
}
