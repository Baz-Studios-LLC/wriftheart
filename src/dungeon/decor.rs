//! decor.rs — room flavour props: metadata + per-theme pools + placement (js/dungeon.js
//! PROP/THEME_DECOR/QUAD_PATTERNS/placeDecor). The PROP paint routines port with the
//! room-bake renderer; generation only needs each kind's footprint + flags.
//!
//! PARITY: placeDecor's rng call ORDER is sacred — every `r()` here mirrors the js line
//! it came from, including draws whose results get discarded.

use super::{DRoom, Decor, RoomType, COLS, MIDC, MIDR, ROWS};
use crate::worldgen::rng::Mulberry32;

pub struct PropMeta {
    pub kind: &'static str,
    pub w: i32,
    pub solid: bool,
    pub detail: bool,
    /// Smashable furniture (live entity in play, skipped by the bake + solid grid).
    pub destructible: bool,
    /// Carries a live light (brazier/crystal/altar/…); params join with the app port.
    pub lit: bool,
}

/// js DESTRUCTIBLE — (debris colour, hp, flammable) per smashable kind.
pub fn smash_stats(kind: &str) -> Option<(u32, i32, bool)> {
    Some(match kind {
        "barrel" | "crate" => (0x8a5a2a, 3, true),
        "table" | "weaponrack" => (0x8a5a2a, 4, true),
        "bookshelf" => (0x7c4c1c, 4, true),
        "armorstand" => (0x9a9aa5, 4, false),
        "urn" => (0xb08060, 3, false),
        "bonepile" => (0xd8d4c4, 3, false),
        "cobweb" => (0xe4e4ea, 1, true), // cut in one swing; hangs in the air (never solid)
        "crystal" => (0x6fe6e0, 3, false), // pick-gated ore node (dungeon.rs adds GatherTool)
        _ => return None,
    })
}

const fn solid(kind: &'static str, w: i32) -> PropMeta {
    PropMeta { kind, w, solid: true, detail: false, destructible: false, lit: false }
}
const fn breakable(kind: &'static str, w: i32) -> PropMeta {
    PropMeta { kind, w, solid: true, detail: false, destructible: true, lit: false }
}
const fn fixture(kind: &'static str, w: i32, lit: bool) -> PropMeta {
    PropMeta { kind, w, solid: false, detail: false, destructible: false, lit }
}
const fn detail(kind: &'static str, w: i32) -> PropMeta {
    PropMeta { kind, w, solid: false, detail: true, destructible: false, lit: false }
}

/// js PROP (+ the DESTRUCTIBLE flag merge at module init).
pub static PROPS: &[PropMeta] = &[
    breakable("bookshelf", 2),
    breakable("table", 2),
    breakable("barrel", 1),
    breakable("crate", 1),
    solid("sarcophagus", 2),
    solid("statue", 1),
    PropMeta { kind: "urn", w: 1, solid: true, detail: false, destructible: true, lit: false },
    solid("brokenpillar", 1),
    solid("block", 1),
    solid("stalagmite", 1),
    // MINABLE (Baz): it wears the overworld ore-node sprite now, so a pick really mines it.
    PropMeta { kind: "crystal", w: 1, solid: true, detail: false, destructible: true, lit: true },
    PropMeta { kind: "armorstand", w: 1, solid: true, detail: false, destructible: true, lit: false },
    breakable("weaponrack", 2),
    PropMeta { kind: "altar", w: 2, solid: true, detail: false, destructible: false, lit: true },
    solid("throne", 2),
    PropMeta { kind: "brazier", w: 1, solid: true, detail: false, destructible: false, lit: true },
    fixture("candelabra", 1, true),
    PropMeta { kind: "bonepile", w: 1, solid: true, detail: false, destructible: true, lit: false },
    PropMeta { kind: "fireplace", w: 2, solid: true, detail: false, destructible: false, lit: true },
    fixture("banner", 1, false),
    fixture("painting", 1, false),
    fixture("chains", 1, false),
    fixture("vines", 1, false),
    PropMeta { kind: "cobweb", w: 1, solid: false, detail: true, destructible: true, lit: false },
    detail("crack", 1),
    detail("moss", 1),
    detail("bones", 1),
    detail("rubble", 1),
    detail("mushroompatch", 1),
    detail("bloodstain", 1),
    detail("puddle", 1),
    detail("rug", 3),
    detail("magiccircle", 3),
];

pub fn prop(kind: &str) -> &'static PropMeta {
    PROPS.iter().find(|p| p.kind == kind).unwrap_or(&PROPS[0])
}

/// js THEME_DECOR — (theme, wall fixtures, floor furniture, details, centrepieces).
type Pools = (&'static str, &'static [&'static str], &'static [&'static str], &'static [&'static str], &'static [&'static str]);
pub static THEME_DECOR: &[Pools] = &[
    ("cave", &["vines"], &["stalagmite", "stalagmite", "crystal", "barrel", "bonepile"], &["rubble", "moss", "bones", "mushroompatch", "crack", "puddle"], &["puddle"]),
    ("crypt", &["banner", "chains", "painting"], &["sarcophagus", "statue", "urn", "candelabra", "altar", "brokenpillar", "bonepile"], &["cobweb", "bones", "crack", "bloodstain", "rubble"], &["magiccircle", "rug"]),
    ("ruins", &["banner", "vines", "chains"], &["brokenpillar", "statue", "urn", "barrel", "crystal", "bonepile"], &["moss", "rubble", "crack", "bones", "mushroompatch"], &["rug", "magiccircle"]),
    ("tomb", &["banner", "painting", "chains"], &["sarcophagus", "urn", "statue", "altar", "brazier", "crate"], &["bones", "crack", "rubble", "cobweb"], &["magiccircle", "rug"]),
    ("castle", &["fireplace", "bookshelf", "banner", "painting", "weaponrack"], &["table", "bookshelf", "barrel", "crate", "armorstand", "throne", "brazier", "candelabra", "statue"], &["rug", "crack", "bloodstain", "cobweb"], &["rug", "magiccircle"]),
    ("bog", &["vines", "chains"], &["urn", "bonepile", "stalagmite", "crystal", "barrel"], &["moss", "puddle", "mushroompatch", "bones", "crack"], &["puddle"]),
    ("crystalcave", &["crystal"], &["crystal", "crystal", "stalagmite", "stalagmite"], &["crystal", "rubble", "crack", "puddle"], &["crystal"]),
    ("fungal", &["vines"], &["stalagmite", "crystal", "bonepile", "barrel"], &["mushroompatch", "moss", "puddle", "crack"], &["mushroompatch", "puddle"]),
    ("lavatube", &["chains"], &["stalagmite", "stalagmite", "brazier", "bonepile"], &["rubble", "crack", "bones"], &["brazier"]),
    ("darkdepths", &["chains"], &["stalagmite", "stalagmite", "crystal", "bonepile"], &["bones", "rubble", "crack", "cobweb"], &["puddle"]),
    ("frostcavern", &["crystal"], &["crystal", "stalagmite", "stalagmite"], &["crack", "rubble", "puddle"], &["crystal"]),
    ("ossuary", &["chains", "banner"], &["bonepile", "bonepile", "urn", "sarcophagus", "statue", "brokenpillar"], &["bones", "bones", "crack", "cobweb", "rubble"], &["magiccircle", "rug"]),
    ("charhall", &["chains"], &["brazier", "barrel", "crate", "bonepile", "brokenpillar"], &["rubble", "crack", "bones", "bloodstain"], &["brazier"]),
    ("riftvault", &["chains", "banner"], &["crystal", "altar", "brokenpillar", "statue", "urn"], &["crack", "rubble", "cobweb"], &["magiccircle"]),
    ("petalhall", &["vines", "banner"], &["urn", "statue", "candelabra", "table", "crystal"], &["moss", "mushroompatch", "crack", "puddle"], &["rug", "magiccircle"]),
    ("hivehollow", &["vines"], &["barrel", "urn", "table", "crate", "candelabra"], &["moss", "puddle", "crack"], &["rug"]),
    ("bellbarrow", &["vines", "banner"], &["urn", "statue", "candelabra", "brokenpillar", "altar"], &["moss", "crack", "cobweb", "puddle"], &["rug", "magiccircle"]),
    ("vinewarren", &["vines", "vines"], &["stalagmite", "urn", "bonepile", "brokenpillar"], &["moss", "moss", "mushroompatch", "puddle", "crack"], &["puddle"]),
    ("searuin", &["vines", "chains"], &["brokenpillar", "urn", "statue", "barrel", "crate"], &["puddle", "puddle", "moss", "crack", "bones"], &["puddle", "rug"]),
    ("stormspire", &["chains", "banner", "weaponrack"], &["brokenpillar", "statue", "brazier", "armorstand", "urn"], &["crack", "rubble", "puddle"], &["magiccircle", "rug"]),
    ("tarpit", &["chains", "vines"], &["stalagmite", "bonepile", "urn", "brokenpillar"], &["puddle", "bones", "crack", "bloodstain"], &["puddle"]),
    ("windbarrow", &["banner", "chains"], &["brokenpillar", "urn", "statue", "crate", "bonepile"], &["rubble", "crack", "bones"], &["rug"]),
    ("saltmine", &["chains"], &["stalagmite", "crystal", "barrel", "crate", "brokenpillar"], &["rubble", "crack", "bones"], &["crystal"]),
    ("hollowroot", &["vines", "chains", "painting"], &["bonepile", "urn", "statue", "candelabra", "brokenpillar"], &["cobweb", "cobweb", "bones", "moss", "crack"], &["magiccircle"]),
    ("blightvault", &["vines", "chains", "banner"], &["bonepile", "urn", "altar", "sarcophagus", "brokenpillar"], &["moss", "bones", "bloodstain", "crack", "mushroompatch"], &["magiccircle", "puddle"]),
    ("saltmaze", &["banner", "chains"], &["statue", "brazier", "urn", "brokenpillar", "altar", "candelabra"], &["bones", "crack", "rubble", "cobweb"], &["magiccircle", "rug"]),
    ("guildhall", &["banner", "painting", "fireplace"], &["candelabra", "bookshelf", "table", "armorstand", "weaponrack"], &["cobweb"], &["rug"]),
];

fn pools(theme_key: &str) -> &'static Pools {
    THEME_DECOR.iter().find(|(k, ..)| *k == theme_key).unwrap_or(&THEME_DECOR[0])
}

/// js QUAD_PATTERNS — offsets in the top-left quadrant, mirrored to all four.
const QUAD_PATTERNS: [&[(i32, i32)]; 6] = [
    &[(4, 3)],
    &[(4, 2), (5, 2), (4, 3), (5, 3)],
    &[(3, 3), (4, 3), (5, 3)],
    &[(2, 2), (3, 2), (2, 3)],
    &[(3, 2), (4, 3), (5, 4)],
    &[(3, 3), (5, 3)],
];

fn mirror_quad(cells: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for &(c, rr) in cells {
        for cand in [(c, rr), (COLS - 1 - c, rr), (c, ROWS - 1 - rr), (COLS - 1 - c, ROWS - 1 - rr)] {
            if seen.insert(cand) {
                out.push(cand);
            }
        }
    }
    out
}

/// Solid, PERMANENT decor tiles (destructibles block via their own live entity instead).
pub fn solid_decor_tiles(decor: &[Decor]) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for d in decor {
        let p = prop(d.kind);
        if p.solid && !p.destructible {
            for i in 0..p.w {
                out.push((d.c + i, d.r));
            }
        }
    }
    out
}

/// js placeDecor — assign a room's decor + pits (+ the rare secret block). Mutates `room`.
pub fn place_decor(r: &mut Mulberry32, theme_key: &str, room: &mut DRoom) {
    room.decor.clear();
    room.pits.clear();
    if matches!(room.rtype, RoomType::Start | RoomType::Arrival | RoomType::Stairs) {
        return; // keep entry/stairs rooms clean
    }
    let (_, wall_pool, floor_pool, detail_pool, center_pool) = pools(theme_key);
    let mut used: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
    if let Some((cx, cy)) = room.chest {
        used.insert(((cx as f64 / 16.0).round() as i32, (cy as f64 / 16.0).round() as i32));
    }
    used.insert((4, 3)); // the fixed stairs / secret-block tile stays clear

    let sparse = matches!(room.rtype, RoomType::Boss | RoomType::Treasure);
    // ~1 in 3 plain rooms gets a PATTERNED layout: quadrants filled with blocks or pits.
    let patterned = !sparse && r.next_f64() < 0.34;
    if patterned {
        let tmpl = QUAD_PATTERNS[(r.next_f64() * QUAD_PATTERNS.len() as f64) as usize];
        let use_pits = r.next_f64() < 0.5;
        for (c, rr) in mirror_quad(tmpl) {
            if !used.insert((c, rr)) {
                continue;
            }
            if use_pits {
                room.pits.push((c, rr));
            } else {
                room.decor.push(Decor { kind: "block", c, r: rr, detail: false, corner: None });
            }
        }
    }

    let fits = |used: &std::collections::HashSet<(i32, i32)>, kind: &str, c: i32, rr: i32| -> bool {
        let w = prop(kind).w;
        if c < 2 || c + w > COLS - 2 {
            return false;
        }
        (0..w).all(|i| !used.contains(&(c + i, rr)))
    };

    // Wall fixtures (top row).
    let n_wall = if sparse { 0 } else { (r.next_f64() * 3.0) as i32 };
    for _ in 0..n_wall {
        let kind = wall_pool[(r.next_f64() * wall_pool.len() as f64) as usize];
        let w = prop(kind).w;
        let c = if r.next_f64() < 0.5 { 2 + (r.next_f64() * (6 - w) as f64) as i32 } else { 11 + (r.next_f64() * (6 - w) as f64) as i32 };
        if fits(&used, kind, c, 1) {
            for i in 0..w {
                used.insert((c + i, 1));
            }
            room.decor.push(Decor { kind, c, r: 1, detail: false, corner: None });
        }
    }
    // Floor furniture (quadrants).
    let n_floor = if patterned { 0 } else if sparse { 1 } else { 2 + (r.next_f64() * 3.0) as i32 };
    for _ in 0..n_floor {
        let kind = floor_pool[(r.next_f64() * floor_pool.len() as f64) as usize];
        let w = prop(kind).w;
        let c = if r.next_f64() < 0.5 { 2 + (r.next_f64() * (6 - w) as f64) as i32 } else { 11 + (r.next_f64() * (6 - w) as f64) as i32 };
        let rr = if r.next_f64() < 0.5 {
            if r.next_f64() < 0.5 { 2 } else { 3 }
        } else if r.next_f64() < 0.5 { 9 } else { 10 };
        if fits(&used, kind, c, rr) {
            for i in 0..w {
                used.insert((c + i, rr));
            }
            room.decor.push(Decor { kind, c, r: rr, detail: false, corner: None });
        }
    }
    // A centrepiece (rug / circle) — js short-circuit order: the r() draw happens whenever
    // !sparse && !patterned, before the (always non-empty) pool check.
    if !sparse && !patterned && r.next_f64() < 0.45 && !center_pool.is_empty() {
        let kind = center_pool[(r.next_f64() * center_pool.len() as f64) as usize];
        let w = prop(kind).w;
        let clear = (0..w).all(|k| !used.contains(&(MIDC - 1 + k, MIDR - 1)));
        if clear {
            for k in 0..w {
                used.insert((MIDC - 1 + k, MIDR - 1));
            }
            room.decor.push(Decor { kind, c: MIDC - 1, r: MIDR - 1, detail: true, corner: None });
        }
    }
    // Corner cobwebs, oriented into their corner (the r() draw is unconditional per corner).
    for (cc, rr, cn) in [(1, 1, "tl"), (COLS - 2, 1, "tr"), (1, ROWS - 2, "bl"), (COLS - 2, ROWS - 2, "br")] {
        if r.next_f64() < 0.6 && !used.contains(&(cc, rr)) {
            room.decor.push(Decor { kind: "cobweb", c: cc, r: rr, detail: true, corner: Some(cn) });
        }
    }
    // Scattered grime.
    let n_detail = if sparse { 1 } else { 2 + (r.next_f64() * 4.0) as i32 };
    for _ in 0..n_detail {
        let c = 2 + (r.next_f64() * (COLS - 4) as f64) as i32;
        let rr = 2 + (r.next_f64() * (ROWS - 4) as f64) as i32;
        let kind = detail_pool[(r.next_f64() * detail_pool.len() as f64) as usize];
        let w = prop(kind).w;
        let mut clear = c + w <= COLS - 2;
        for k in 0..w {
            if clear && used.contains(&(c + k, rr)) {
                clear = false;
            }
        }
        if clear {
            for k in 0..w {
                used.insert((c + k, rr));
            }
            room.decor.push(Decor { kind, c, r: rr, detail: true, corner: None });
        }
    }
    // RARE secret: a lone push-block hiding stairs to a side-room (~15% of normal rooms).
    if !patterned && !sparse && room.rtype == RoomType::Normal && r.next_f64() < 0.25 {
        room.secret = Some((4, 3));
    }
}

/// A lit prop's darkness-hole radius (js PROP litR; the glow params join with the
/// additive lighting pass).
pub fn light_radius(kind: &str) -> Option<i32> {
    match kind {
        "fireplace" => Some(42),
        "brazier" => Some(30),
        "crystal" => Some(26),
        "altar" => Some(26),
        "candelabra" => Some(22),
        _ => None,
    }
}
