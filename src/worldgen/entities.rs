//! entities.rs — a room's full entity layout (port of `getEntities` in js/world.js), the
//! keystone of prop parity: every salted stream runs in the EXACT JS order and shares the
//! `used`-tile occupancy set, so bushes never stand where a shop landed and mobs never
//! stand where a tree grew. Pinned descriptor-for-descriptor by tests/entities_parity.rs.
//!
//! JS short-circuit traps live here: `!elited && mrng() < x` only ADVANCES the stream when
//! the left side passes — every such site is ported as an explicit branch, because one
//! extra rng call shifts every position after it.
//!
//! TODO (their own ports): townEntities (towns return empty), the mob section stays the
//! source of truth here (superseding spawns.rs mob_roster).

use super::rng::{hash, Mulberry32};
use super::spawns::mob_tier;
use super::world::{World, COLS, ROWS};
use crate::room::TILE;

const SALT_BUSH: u32 = 0x2a9d;
const SALT_ROCK: u32 = 0xc2b2;
const SALT_TREE: u32 = 0x4d8b;
const SALT_SHOP: u32 = 0x5e29;
const SALT_CHEST: u32 = 0x3b71;
const SALT_SONG: u32 = 0x4f2b;
const SALT_CRACK: u32 = 0x8af1;
const SALT_CLUTTER: u32 = 0x6b2d;
const SALT_REED: u32 = 0x5c1f;
const SALT_MOB: u32 = 0xb47d;
const SALT_LOOTGOB: u32 = 0xe6a7;
const SALT_WAGON: u32 = 0x9d4f;

const CHAOS_TREES: [&str; 3] = ["riftbulb", "voidspire", "mawtree"];

/// Per-biome ground-clutter kinds (port of the CLUTTER table in js/world.js).
const CLUTTER: &[(&str, &[&str])] = &[
    ("grassland", &["pebble", "shrub", "shrub", "twig"]),
    ("forest", &["fern", "fern", "toadstool", "twig", "shrub"]),
    ("desert", &["deadbush", "deadbush", "pebble", "pebble", "bones"]),
    ("mountains", &["scree", "pebble", "deadbush"]),
    ("graveyard", &["gravestone", "gravestone", "bones", "pillar", "deadbush"]),
    ("burnt", &["ashpile", "charcoal", "charcoal", "charredlog", "embers", "bones"]),
    ("embermaw", &["lavarock", "lavarock", "obsidianshard", "sulfur", "emberpile", "bones"]),
    ("petalwood", &["bloom", "bloom", "bloom", "shrub", "toadstool", "pebble"]),
    ("hollowwood", &["bones", "deadbush", "deadbush", "toadstool", "bones", "gravestone"]),
    ("greenmaw", &["fern", "fern", "bloom", "shrub", "toadstool", "vine"]),
    ("prismwastes", &["crystalshard", "crystalshard", "scree", "obsidianshard", "pebble", "crystalshard"]),
    ("blackdeep", &["pebble", "bones", "obsidianshard", "crystalshard", "scree", "pebble"]),
    ("honeyglade", &["bloom", "bloom", "toadstool", "shrub", "bloom", "pebble"]),
    ("bluebell", &["bloom", "bloom", "shrub", "toadstool", "bloom", "pebble"]),
    ("suncoast", &["pebble", "scree", "bones", "pebble", "shrub", "scree"]),
    ("stormreach", &["scree", "pebble", "scree", "deadbush", "pebble", "scree"]),
    ("tarmire", &["bones", "deadbush", "obsidianshard", "bones", "pebble", "deadbush"]),
    ("galewind", &["scree", "deadbush", "pebble", "twig", "scree", "deadbush"]),
    ("saltwastes", &["pebble", "bones", "scree", "pebble", "deadbush", "bones"]),
    ("witherlands", &["bones", "gravestone", "deadbush", "pillar", "bones", "deadbush"]),
];

/// One placed entity: `kind` is the JS `type`, `sub` its `kind`/`dest`/`line` payload
/// ("" if none), `seed` the js per-entity seed (villager identity; 0 if none).
#[derive(Clone, Debug, PartialEq)]
pub struct RoomEntity {
    pub kind: &'static str,
    pub sub: String,
    pub x: i32,
    pub y: i32,
    pub seed: u32,
    pub champ: bool,
    pub elite: bool,
}

fn ent(kind: &'static str, c: i32, r: i32) -> RoomEntity {
    RoomEntity { kind, sub: String::new(), x: c * TILE, y: r * TILE, seed: 0, champ: false, elite: false }
}
fn ent_sub(kind: &'static str, sub: &str, c: i32, r: i32) -> RoomEntity {
    RoomEntity { kind, sub: sub.into(), x: c * TILE, y: r * TILE, seed: 0, champ: false, elite: false }
}

impl World {
    /// The room's full entity list — port of `getEntities(rx, ry)`.
    pub fn room_entities(&self, rx: i32, ry: i32) -> Vec<RoomEntity> {
        let (mid_c, mid_r) = (COLS >> 1, ROWS >> 1);
        if World::is_castle(rx, ry) {
            return castle_entities(mid_c);
        }
        if self.is_town(rx, ry) {
            return self.town_entities(rx, ry); // buildings + villagers, no mobs
        }
        let room = self.generate(rx, ry);
        let (map, prot) = (&room.map, &room.prot);
        let b = self.biome_at(rx, ry);
        if self.shard_dungeon_at(rx, ry).is_some() {
            // Authored monument grounds: flames + waymarkers + an honour guard of the land's
            // own growth around the dungeon mouth.
            let mut e = vec![ent_sub("dungeon", self.biome_key_at(rx, ry), mid_c, 4)];
            let tier = World::threat_tier(rx, ry);
            let flame = if tier >= 3 { "gravebrazier" } else { "torch" };
            e.push(ent(flame, 6, 4));
            e.push(ent(flame, 12, 4));
            e.push(ent_sub("clutter", "pillar", 6, 7));
            e.push(ent_sub("clutter", "pillar", 12, 7));
            let guard: &'static str = match b.tree_kind {
                Some("mix") => "pine",
                Some("chaosmix") => "voidspire",
                Some(k) => k,
                None => {
                    if b.cacti > 0 {
                        "cactus"
                    } else {
                        "boulder"
                    }
                }
            };
            for (gc, gr) in [(3, 2), (15, 2), (2, 6), (16, 6), (4, 9), (14, 9)] {
                e.push(ent(guard, gc, gr));
            }
            if tier >= 4 {
                e.push(ent("wisp", 5, 3));
                e.push(ent("wisp", 13, 3));
            }
            return e;
        }
        if self.rift_at(rx, ry) {
            let mut e = vec![ent("rift", mid_c, 7)];
            e.push(ent("gravebrazier", 6, 9));
            e.push(ent("gravebrazier", 12, 9));
            e.push(ent_sub("clutter", "pillar", 5, 11));
            e.push(ent_sub("clutter", "pillar", 13, 11));
            for (c, r) in [(2, 3), (16, 3), (3, 10), (15, 10)] {
                e.push(ent("deadtree", c, r));
            }
            e.push(ent_sub("clutter", "bones", 4, 7));
            e.push(ent_sub("clutter", "bones", 14, 7));
            e.push(ent_sub("clutter", "deadbush", 7, 12));
            e.push(ent_sub("clutter", "deadbush", 11, 12));
            e.push(ent("wisp", 4, 4));
            e.push(ent("wisp", 14, 4));
            e.push(ent("wisp", 9, 11));
            return e;
        }

        let mut used: Vec<bool> = vec![false; (COLS * ROWS) as usize];
        let mut out: Vec<RoomEntity> = Vec::new();
        let ground_at = |c: i32, r: i32| -> bool {
            (0..ROWS).contains(&r)
                && map[r as usize].chars().nth(c as usize).is_some_and(|ch| ch == '.')
        };
        let key = |c: i32, r: i32| (r * COLS + c) as usize;

        // The Saltmaze's half-buried door: normal generation continues around it.
        if self.saltmaze_at(rx, ry) {
            out.push(ent("saltmaze", mid_c, 4));
            for dc in -2..=2 {
                for dr in -3..=1 {
                    used[key(mid_c + dc, 4 + dr)] = true;
                }
            }
        }

        let start_room = rx == 0 && ry == 0;

        // Shop: rare timber storefront on a clear patch.
        if !start_room {
            let mut srng = Mulberry32::new(hash(self.seed, rx, ry, SALT_SHOP));
            if srng.next_f64() < 0.035 {
                for _ in 0..16 {
                    let c = 3 + (srng.next_f64() * (COLS - 6) as f64).floor() as i32;
                    let r = 3 + (srng.next_f64() * (ROWS - 5) as f64).floor() as i32;
                    let mut clear = true;
                    'scan: for dc in -1..=1 {
                        for dr in -2..=1 {
                            let (cc, rr) = (c + dc, r + dr);
                            if !ground_at(cc, rr) || prot.contains(&key(cc, rr)) || used[key(cc, rr)] {
                                clear = false;
                                break 'scan;
                            }
                        }
                    }
                    if !clear {
                        continue;
                    }
                    for dc in -1..=1 {
                        for dr in -2..=1 {
                            used[key(c + dc, r + dr)] = true;
                        }
                    }
                    out.push(ent("shop", c, r));
                    break;
                }
            }
        }

        // Travelling tradesman: rarer roadside caravan on a wide clear patch.
        if !start_room {
            let mut wrng = Mulberry32::new(hash(self.seed, rx, ry, SALT_WAGON));
            if wrng.next_f64() < 0.015 {
                for _ in 0..16 {
                    let c = 4 + (wrng.next_f64() * (COLS - 8) as f64).floor() as i32;
                    let r = 3 + (wrng.next_f64() * (ROWS - 5) as f64).floor() as i32;
                    let mut clear = true;
                    'scan2: for dc in -3..=2 {
                        for dr in -1..=1 {
                            let (cc, rr) = (c + dc, r + dr);
                            if !(1..=COLS - 2).contains(&cc)
                                || !ground_at(cc, rr)
                                || prot.contains(&key(cc, rr))
                                || used[key(cc, rr)]
                            {
                                clear = false;
                                break 'scan2;
                            }
                        }
                    }
                    if !clear {
                        continue;
                    }
                    for dc in -3..=2 {
                        for dr in -1..=1 {
                            used[key(c + dc, r + dr)] = true;
                        }
                    }
                    out.push(ent("tradewagon", c, r));
                    break;
                }
            }
        }

        // Treasure chest: a rare surprise on open ground.
        let mut crng = Mulberry32::new(hash(self.seed, rx, ry, SALT_CHEST));
        if !start_room && crng.next_f64() < 0.05 {
            for _ in 0..14 {
                let c = 2 + (crng.next_f64() * (COLS - 4) as f64).floor() as i32;
                let r = 2 + (crng.next_f64() * (ROWS - 4) as f64).floor() as i32;
                if !ground_at(c, r) || prot.contains(&key(c, r)) || used[key(c, r)] {
                    continue;
                }
                used[key(c, r)] = true;
                out.push(ent("chest", c, r));
                break;
            }
        }

        // Singing stone: only the Song of Opening unseals it; its secret is rolled here.
        let mut srng2 = Mulberry32::new(hash(self.seed, rx, ry, SALT_SONG));
        if !start_room && srng2.next_f64() < 0.025 {
            let dest = if srng2.next_f64() < 0.7 { "biome" } else { "shop" };
            for _ in 0..14 {
                let c = 3 + (srng2.next_f64() * (COLS - 6) as f64).floor() as i32;
                let r = 3 + (srng2.next_f64() * (ROWS - 5) as f64).floor() as i32;
                if !ground_at(c, r)
                    || !ground_at(c, r - 1)
                    || prot.contains(&key(c, r))
                    || used[key(c, r)]
                {
                    continue; // the stone stands 2 tiles tall
                }
                used[key(c, r)] = true;
                used[key(c, r - 1)] = true;
                out.push(ent_sub("songstone", dest, c, r));
                break;
            }
        }

        // Cracked wall: a fissured border-wall tile hiding a secret.
        let mut krng = Mulberry32::new(hash(self.seed, rx, ry, SALT_CRACK));
        if !start_room && krng.next_f64() < 0.07 {
            let wall_tile = |c: i32, r: i32| -> bool {
                (0..ROWS).contains(&r)
                    && map[r as usize]
                        .chars()
                        .nth(c as usize)
                        .is_some_and(|ch| ch != '.' && ch != '~' && ch != 'B' && ch != '_' && ch != 'p')
            };
            let mut spots: Vec<(i32, i32, i32, i32)> = Vec::new();
            for c in 2..COLS - 2 {
                spots.push((c, 1, c, 0));
                spots.push((c, ROWS - 2, c, ROWS - 1));
            }
            for r in 2..ROWS - 2 {
                spots.push((1, r, 0, r));
                spots.push((COLS - 2, r, COLS - 1, r));
            }
            // Deterministic Fisher-Yates, exactly the JS loop.
            for i in (1..spots.len()).rev() {
                let j = (krng.next_f64() * (i + 1) as f64).floor() as usize;
                spots.swap(i, j);
            }
            for (c, r, bc, br) in spots {
                if !ground_at(c, r) || prot.contains(&key(c, r)) || used[key(c, r)] || !wall_tile(bc, br) {
                    continue;
                }
                used[key(c, r)] = true;
                out.push(ent("crackedrock", bc, br));
                break;
            }
        }

        // A solid 1-tile prop on clear ground (shared by bushes/boulders): the js placeSolid.
        macro_rules! place_solid {
            ($kind:expr, $c:expr, $r:expr) => {{
                let (c, r) = ($c, $r);
                if (1..=COLS - 2).contains(&c)
                    && (1..=ROWS - 2).contains(&r)
                    && ground_at(c, r)
                    && !prot.contains(&key(c, r))
                    && !used[key(c, r)]
                {
                    used[key(c, r)] = true;
                    out.push(ent($kind, c, r));
                }
            }};
        }

        // Bushes in rows / columns / grids.
        let mut brng = Mulberry32::new(hash(self.seed, rx, ry, SALT_BUSH));
        let hedges = (brng.next_f64() * (b.bushes + 1) as f64).floor() as i32;
        for _ in 0..hedges {
            let kind = brng.next_f64();
            if kind < 0.45 {
                let r = 2 + (brng.next_f64() * (ROWS - 4) as f64).floor() as i32;
                let len = 4 + (brng.next_f64() * 7.0).floor() as i32;
                let c0 = 1 + (brng.next_f64() * 1.max(COLS - 2 - len) as f64).floor() as i32;
                for c in c0..c0 + len {
                    place_solid!("bush", c, r);
                }
            } else if kind < 0.85 {
                let c = 2 + (brng.next_f64() * (COLS - 4) as f64).floor() as i32;
                let len = 3 + (brng.next_f64() * 6.0).floor() as i32;
                let r0 = 1 + (brng.next_f64() * 1.max(ROWS - 2 - len) as f64).floor() as i32;
                for r in r0..r0 + len {
                    place_solid!("bush", c, r);
                }
            } else {
                let w = 2 + (brng.next_f64() * 3.0).floor() as i32;
                let hh = 2 + (brng.next_f64() * 3.0).floor() as i32;
                let c0 = 2 + (brng.next_f64() * 1.max(COLS - 4 - w * 2) as f64).floor() as i32;
                let r0 = 2 + (brng.next_f64() * 1.max(ROWS - 4 - hh * 2) as f64).floor() as i32;
                for i in 0..w {
                    for j in 0..hh {
                        place_solid!("bush", c0 + i * 2, r0 + j * 2);
                    }
                }
            }
        }

        // Boulders (small 1-tile clusters).
        let mut orng = Mulberry32::new(hash(self.seed, rx, ry, SALT_ROCK));
        let boulders = (orng.next_f64() * (b.boulders + 1) as f64).floor() as i32;
        for _ in 0..boulders {
            let c = 2 + (orng.next_f64() * (COLS - 4) as f64).floor() as i32;
            let r = 2 + (orng.next_f64() * (ROWS - 4) as f64).floor() as i32;
            place_solid!("boulder", c, r);
            if orng.next_f64() < 0.4 {
                place_solid!("boulder", c + 1, r);
                place_solid!("boulder", c, r + 1);
            }
        }

        // Big props (trees + cacti): spaced >= 3 tiles apart so canopies don't pile up.
        let mut big: Vec<(i32, i32)> = Vec::new();
        macro_rules! place_big {
            ($kind:expr, $rng:expr) => {{
                let c = 2 + ($rng.next_f64() * (COLS - 4) as f64).floor() as i32;
                let r = 2 + ($rng.next_f64() * (ROWS - 4) as f64).floor() as i32;
                if ground_at(c, r)
                    && !prot.contains(&key(c, r))
                    && !used[key(c, r)]
                    && !big.iter().any(|&(bc, br)| (bc - c).abs() < 3 && (br - r).abs() < 3)
                {
                    used[key(c, r)] = true;
                    big.push((c, r));
                    out.push(ent($kind, c, r));
                }
            }};
        }
        let mut trng = Mulberry32::new(hash(self.seed, rx, ry, SALT_TREE));
        let tree_target = if b.trees > 0 { b.trees + 1 } else { 0 };
        let mut i = 0;
        while i < tree_target * 6 && (big.len() as i32) < tree_target {
            let kind: Option<&'static str> = match b.tree_kind {
                Some("mix") => Some(if trng.next_f64() < 0.5 { "oak" } else { "pine" }),
                Some("chaosmix") => {
                    Some(CHAOS_TREES[(trng.next_f64() * CHAOS_TREES.len() as f64).floor() as usize])
                }
                k => k,
            };
            if let Some(kind) = kind {
                place_big!(kind, trng);
            }
            i += 1;
        }
        let cactus_target = b.cacti;
        let mut i = 0;
        while i < cactus_target * 6 && (big.len() as i32) < tree_target + cactus_target {
            place_big!("cactus", orng);
            i += 1;
        }

        // Mobs: biome creatures + roaming goblins (~30% fill). SHORT-CIRCUIT SEMANTICS —
        // see the module note; every skipped rng call here is deliberate.
        let mut mrng = Mulberry32::new(hash(self.seed, rx, ry, SALT_MOB));
        let tier = World::threat_tier(rx, ry);
        let dist = World::ring_dist(rx, ry);
        let eligible: Vec<&'static str> =
            b.mob_kinds.iter().copied().filter(|k| mob_tier(k) <= tier).collect();
        let champ_chance = (tier as f64 * 0.04).min(0.35);
        let elite_chance = (0.018 + tier as f64 * 0.004).min(0.06);
        let mut elited = false;
        let mobs = (mrng.next_f64() * (b.mobs + 1) as f64).floor() as i32 + (dist / 8).min(4);
        for _ in 0..mobs {
            let c = 2 + (mrng.next_f64() * (COLS - 4) as f64).floor() as i32;
            let r = 2 + (mrng.next_f64() * (ROWS - 4) as f64).floor() as i32;
            if !ground_at(c, r) || used[key(c, r)] || (c == mid_c && r == mid_r) {
                continue;
            }
            let elite = !elited && mrng.next_f64() < elite_chance;
            if elite {
                elited = true;
            }
            let champ = !elite && mrng.next_f64() < champ_chance;
            if eligible.is_empty() || mrng.next_f64() < 0.3 {
                // Roaming goblins fill in; ONE roll picks the strain.
                let mut gk = "melee";
                if tier >= 1 {
                    let gr = mrng.next_f64();
                    gk = if tier >= 2 && gr < 0.12 {
                        "red"
                    } else if gr < 0.3 {
                        "spear"
                    } else {
                        "melee"
                    };
                }
                out.push(RoomEntity { kind: "goblin", sub: gk.into(), x: c * TILE, y: r * TILE, seed: 0, champ, elite });
            } else {
                let kind = eligible[(mrng.next_f64() * eligible.len() as f64).floor() as usize];
                out.push(RoomEntity { kind: "mob", sub: kind.into(), x: c * TILE, y: r * TILE, seed: 0, champ, elite });
                if kind == "wasp" || kind == "bat" || kind == "wolf" || kind == "gnat" {
                    out.push(ent_mob(kind, c * TILE - 14, r * TILE + 10));
                    if mrng.next_f64() < 0.6 {
                        out.push(ent_mob(kind, c * TILE + 14, r * TILE + 10));
                    }
                }
            }
        }

        // Super ultra rare: the golden LOOT GOBLIN (its own salted stream).
        if !self.is_town(rx, ry) {
            let mut lg = Mulberry32::new(hash(self.seed, rx, ry, SALT_LOOTGOB));
            if lg.next_f64() < 0.011 {
                for _ in 0..8 {
                    let c = 2 + (lg.next_f64() * (COLS - 4) as f64).floor() as i32;
                    let r = 2 + (lg.next_f64() * (ROWS - 4) as f64).floor() as i32;
                    if ground_at(c, r) && !used[key(c, r)] && !(c == mid_c && r == mid_r) {
                        out.push(ent_mob("lootgoblin", c * TILE, r * TILE));
                        break;
                    }
                }
            }
        }

        // Tall grass in noise-gated clumps; wildflowers on the bare grass between.
        let (gx0, gy0) = (rx * COLS, ry * ROWS);
        for r in 1..ROWS - 1 {
            for c in 1..COLS - 1 {
                if !ground_at(c, r) || used[key(c, r)] {
                    continue;
                }
                let (gx, gy) = (gx0 + c, gy0 + r);
                if self.ground_name(gx, gy) != "grass" {
                    continue;
                }
                if self.is_tall_grass(gx, gy) {
                    out.push(ent("grass", c, r));
                } else if self.is_flower_tile(gx, gy) {
                    out.push(ent("flower", c, r));
                }
            }
        }

        // Swamp dressing: reeds & mushrooms on the muck, lily pads on the pools.
        if self.biome_key_at(rx, ry) == "swamp" {
            let mut srng = Mulberry32::new(hash(self.seed, rx, ry, SALT_REED));
            for r in 1..ROWS - 1 {
                for c in 1..COLS - 1 {
                    if used[key(c, r)] {
                        continue;
                    }
                    let ch = map[r as usize].chars().nth(c as usize);
                    if ch == Some('~') {
                        if srng.next_f64() < 0.05 {
                            out.push(ent("lilypad", c, r));
                        }
                    } else if ground_at(c, r) {
                        let roll = srng.next_f64();
                        if roll < 0.07 {
                            out.push(ent("reed", c, r));
                        } else if roll < 0.10 {
                            out.push(ent("mushroom", c, r));
                        }
                    }
                }
            }
        }

        // Theme ground clutter, scattered by biome.
        if let Some((_, kinds)) = CLUTTER.iter().find(|(k, _)| *k == self.biome_key_at(rx, ry)) {
            let mut krng = Mulberry32::new(hash(self.seed, rx, ry, SALT_CLUTTER));
            for r in 1..ROWS - 1 {
                for c in 1..COLS - 1 {
                    if used[key(c, r)] || !ground_at(c, r) {
                        continue;
                    }
                    if krng.next_f64() < 0.035 {
                        let kind = kinds[(krng.next_f64() * kinds.len() as f64).floor() as usize];
                        out.push(ent_sub("clutter", kind, c, r));
                    }
                }
            }
        }
        out
    }
}

fn ent_mob(kind: &str, x: i32, y: i32) -> RoomEntity {
    RoomEntity { kind: "mob", sub: kind.into(), x, y, seed: 0, champ: false, elite: false }
}

/// The Black Castle grounds — a fully authored set-piece (verbatim placement list).
fn castle_entities(mid_c: i32) -> Vec<RoomEntity> {
    let mut e = vec![ent("castle", mid_c, 7)];
    for c in [2, 3, 4, 5, 12, 13, 14, 15, 16] {
        e.push(ent("ironfence", c, 9));
    }
    for r in [10, 11] {
        e.push(ent("ironfence", 2, r));
        e.push(ent("ironfence", 16, r));
    }
    e.push(ent("gravebrazier", 7, 8));
    e.push(ent("gravebrazier", 10, 8));
    e.push(ent("guard", 6, 10));
    e.push(ent("guard", 11, 10));
    for (c, r) in [(1, 6), (17, 6), (4, 8), (14, 8), (3, 12), (15, 12)] {
        e.push(ent("deadtree", c, r));
    }
    for (c, r) in [(4, 11), (14, 11), (6, 12), (12, 12)] {
        e.push(ent_sub("clutter", "gravestone", c, r));
    }
    for (c, r) in [(5, 10), (13, 10), (7, 12)] {
        e.push(ent_sub("clutter", "bones", c, r));
    }
    for (c, r) in [(2, 8), (16, 8), (11, 12)] {
        e.push(ent_sub("clutter", "deadbush", c, r));
    }
    e.push(ent_sub("clutter", "pillar", 4, 12));
    e.push(ent_sub("clutter", "pillar", 14, 12));
    for (c, r) in [(3, 5), (15, 5), (9, 3), (6, 11), (13, 11)] {
        e.push(ent("wisp", c, r));
    }
    e
}
