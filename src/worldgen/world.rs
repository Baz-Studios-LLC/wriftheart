//! world.rs — the `World`: a seed plus the per-seed site tables (shard dungeons, the Saltmaze),
//! and every room/tile query the generator needs. Port of the js/world.js module state.
//!
//! Construction order matters and mirrors the JS lazy caches: shard sites are computed first
//! (they consult only biomes + towns), then the Saltmaze (it consults shard sites). Everything
//! after that — rifts, flat rooms, terrain — may consult both.

// Lint policy: this file mirrors js/world.js statement-for-statement so it can be
// audited by side-by-side diff. Stylistic reshaping (collapsed ifs, range-contains)
// would break that mapping — allowed here, and ONLY here.
#![allow(clippy::collapsible_if, clippy::manual_range_contains, clippy::manual_is_multiple_of, clippy::needless_range_loop, clippy::int_plus_one, clippy::ptr_arg, clippy::too_many_arguments, clippy::type_complexity)]

use super::biomes::{biome, Biome, BIOME_KEYS};
use super::rng::{hash, value_noise, Mulberry32};
use super::towns::{is_town, town_role, town_site_of, TownRole, TownSite, TOWN_CELL};

pub const COLS: i32 = 19;
pub const ROWS: i32 = 13;
pub const CASTLE_RX: i32 = 0;
pub const CASTLE_RY: i32 = -43;

// Salts (js/world.js) — only the ones terrain generation touches; entity salts join later.
pub const SALT_V: u32 = 0x9e37;
pub const SALT_H: u32 = 0x85eb;
const SALT_LAKE: u32 = 0x6f3a;
const SALT_RIVER: u32 = 0x1bd5;
const SALT_COAST: u32 = 0x7c4e;
const SALT_GRASS: u32 = 0x51c7;
const SALT_DIRT: u32 = 0x73e1;
const SALT_BIOME: u32 = 0x9c1f;
const SALT_TALL: u32 = 0x3ef5;
const SALT_FLOWER: u32 = 0x39c8;
const SALT_RIFT: u32 = 0xd15c;
const SALT_SHARD: u32 = 0x51ab;

// Terrain noise frequencies/levels (js/world.js — keep identical).
const LAKE_FREQ: f64 = 0.021;
const COAST_FREQ: f64 = 0.11;
const COAST_AMP: f64 = 0.10;
const WATER_CUT: f64 = 0.85;
const RIVER_FREQ: f64 = 0.017;
const RIVER_HALF: f64 = 0.025;
const GRASS_FREQ: f64 = 0.065;
const DIRT_FREQ: f64 = 0.060;
const TALL_FREQ: f64 = 0.22;
const TALL_LEVEL: f64 = 0.68;
const FLOWER_FREQ: f64 = 0.09;
const FLOWER_LEVEL: f64 = 0.62;

const CELL: i32 = 6; // biome region size in rooms
const ZONE_RING: i32 = 7;
const MAX_TIER: i32 = 6;
const RING: i32 = 4;

/// The ten shard-dungeon lands for this seed: chosen biomes + each one's site room.
pub struct ShardData {
    pub biomes: Vec<&'static str>,
    pub sites: Vec<(&'static str, (i32, i32))>,
}

pub struct World {
    pub seed: u32,
    shard: ShardData,
    saltmaze: Option<(i32, i32)>,
    rift: (i32, i32),
}

impl World {
    /// `setSeed` + the lazy caches, made eager: shard sites first, then the Saltmaze.
    pub fn new(seed: u32) -> Self {
        let seed = if seed == 0 { 1 } else { seed }; // JS: (s >>> 0) || 1
        let mut w = World { seed, shard: ShardData { biomes: vec![], sites: vec![] }, saltmaze: None, rift: (0, 0) };
        w.shard = w.compute_shard();
        w.saltmaze = w.compute_saltmaze();
        w.rift = w.compute_rift(); // after shard + saltmaze — the walk dodges both
        w
    }

    // --- Distance tiers ---------------------------------------------------------------
    pub fn ring_dist(rx: i32, ry: i32) -> i32 {
        rx.abs().max(ry.abs())
    }
    pub fn threat_tier(rx: i32, ry: i32) -> i32 {
        Self::ring_dist(rx, ry) / RING // both non-negative: truncation == floor
    }
    pub fn zone_tier(rx: i32, ry: i32) -> i32 {
        MAX_TIER.min(Self::ring_dist(rx, ry) / ZONE_RING)
    }
    pub fn is_castle(rx: i32, ry: i32) -> bool {
        rx == CASTLE_RX && ry == CASTLE_RY
    }

    // --- Biome assignment ---------------------------------------------------------------
    /// Port of `biomeKeyAt` — deterministic per CELLxCELL region, weighted toward the
    /// region's zone tier with the neighbouring tiers bleeding in.
    pub fn biome_key_at(&self, rx: i32, ry: i32) -> &'static str {
        if Self::is_castle(rx, ry) {
            return "graveyard"; // the Black Castle stands on cursed ground
        }
        let cx = rx.div_euclid(CELL);
        let cy = ry.div_euclid(CELL);
        let rt = Self::zone_tier(cx * CELL + (CELL >> 1), cy * CELL + (CELL >> 1));
        let mut total: u32 = 0;
        let mut pool: Vec<(&'static str, u32)> = Vec::new();
        for key in BIOME_KEYS {
            let d = (biome(key).tier - rt).abs();
            let w = if d == 0 { 3 } else if d == 1 { 1 } else { 0 };
            if w > 0 {
                pool.push((key, w));
                total += w;
            }
        }
        let mut pick = hash(self.seed, cx, cy, SALT_BIOME) % total;
        for (key, w) in &pool {
            if pick < *w {
                return key;
            }
            pick -= w;
        }
        pool[0].0
    }
    pub fn biome_at(&self, rx: i32, ry: i32) -> &'static Biome {
        biome(self.biome_key_at(rx, ry))
    }
    fn rules_at_tile(&self, gx: i32, gy: i32) -> &'static Biome {
        self.biome_at(gx.div_euclid(COLS), gy.div_euclid(ROWS))
    }

    // --- Towns (thin wrappers binding the seed) ------------------------------------------
    pub fn town_site_of(&self, rx: i32, ry: i32) -> Option<TownSite> {
        town_site_of(self.seed, rx, ry)
    }
    pub fn town_role(&self, rx: i32, ry: i32) -> Option<TownRole> {
        town_role(self.seed, rx, ry)
    }
    pub fn is_town(&self, rx: i32, ry: i32) -> bool {
        is_town(self.seed, rx, ry)
    }

    // --- Shard dungeons + the Saltmaze (per-seed site tables) ----------------------------
    /// Port of `shardData`: 10 of the biomes (per-tier quotas, seeded shuffles), then a
    /// spiral over region cells siting each chosen biome's dungeon.
    fn compute_shard(&self) -> ShardData {
        let take = [2, 2, 2, 1, 1, 1, 1];
        let mut biomes: Vec<&'static str> = Vec::new();
        for tr in 0..=MAX_TIER {
            let mut pool: Vec<&'static str> =
                BIOME_KEYS.iter().copied().filter(|k| biome(k).tier == tr).collect();
            let mut rng = Mulberry32::new(hash(self.seed, tr, 77, SALT_SHARD));
            let mut i = pool.len().saturating_sub(1);
            while i > 0 {
                let j = (rng.next_f64() * (i + 1) as f64).floor() as usize;
                pool.swap(i, j);
                i -= 1;
            }
            for k in pool.iter().take(take[tr as usize].min(pool.len())) {
                biomes.push(k);
            }
        }
        let mut sites: Vec<(&'static str, (i32, i32))> = Vec::new();
        let mut found = 0;
        for ring in 0i32..=48 {
            if found >= biomes.len() {
                break;
            }
            for cx in -ring..=ring {
                for cy in -ring..=ring {
                    if cx.abs().max(cy.abs()) != ring {
                        continue; // this shell only
                    }
                    let rx = cx * CELL + (CELL >> 1);
                    let ry = cy * CELL + (CELL >> 1);
                    let b = self.biome_key_at(rx, ry);
                    if sites.iter().any(|(sb, _)| *sb == b) || !biomes.contains(&b) {
                        continue;
                    }
                    if self.is_town(rx, ry)
                        || Self::is_castle(rx, ry)
                        || (rx.abs() <= 1 && ry.abs() <= 1)
                    {
                        continue;
                    }
                    sites.push((b, (rx, ry)));
                    found += 1;
                }
            }
        }
        ShardData { biomes, sites }
    }
    /// Port of `saltmazeSite`: the first Saltwastes region-centre spiralling out from home.
    fn compute_saltmaze(&self) -> Option<(i32, i32)> {
        for ring in 0i32..=48 {
            for cx in -ring..=ring {
                for cy in -ring..=ring {
                    if cx.abs().max(cy.abs()) != ring {
                        continue;
                    }
                    let rx = cx * CELL + (CELL >> 1);
                    let ry = cy * CELL + (CELL >> 1);
                    if self.biome_key_at(rx, ry) != "saltwastes" {
                        continue;
                    }
                    if self.is_town(rx, ry)
                        || Self::is_castle(rx, ry)
                        || self.shard_dungeon_at(rx, ry).is_some()
                    {
                        continue;
                    }
                    return Some((rx, ry));
                }
            }
        }
        None
    }
    pub fn shard_dungeon_at(&self, rx: i32, ry: i32) -> Option<&'static str> {
        self.shard.sites.iter().find(|(_, s)| *s == (rx, ry)).map(|(b, _)| *b)
    }
    pub fn shard_biomes(&self) -> &[&'static str] {
        &self.shard.biomes
    }
    /// Every shard dungeon's (biome, room) — the dev panel's warp ring.
    pub fn shard_sites(&self) -> &[(&'static str, (i32, i32))] {
        &self.shard.sites
    }
    pub fn saltmaze_at(&self, rx: i32, ry: i32) -> bool {
        self.saltmaze == Some((rx, ry))
    }

    // --- Set-piece room predicates --------------------------------------------------------
    /// THE RIFT SPIRE — one tear in all the world (Baz; the js scattered them
    /// tier-3+ at 1-in-53). It stands in the FARTHEST ring: a seeded walk round
    /// the max-tier ring lands on the first room free of other set-pieces.
    fn compute_rift(&self) -> (i32, i32) {
        let r = MAX_TIER * ZONE_RING + 2;
        let per = 8 * r;
        let start = (hash(self.seed, 0, 0, SALT_RIFT) % per as u32) as i32;
        for i in 0..per {
            let idx = (start + i) % per;
            let (side, o) = (idx / (2 * r), idx % (2 * r));
            let (ax, ay) = match side {
                0 => (-r + o, -r),
                1 => (r, -r + o),
                2 => (r - o, r),
                _ => (-r, r - o),
            };
            if !self.is_town(ax, ay)
                && !Self::is_castle(ax, ay)
                && self.shard_dungeon_at(ax, ay).is_none()
                && !self.saltmaze_at(ax, ay)
            {
                return (ax, ay);
            }
        }
        (r, 0) // unreachable — a whole ring can't be all set-pieces
    }
    /// Is this room THE rift spire's? (One per world.)
    pub fn rift_at(&self, ax: i32, ay: i32) -> bool {
        (ax, ay) == self.rift
    }
    /// Rooms whose terrain is flattened (water suppressed) — port of `flatRoom`.
    pub fn flat_room(&self, ax: i32, ay: i32) -> bool {
        self.is_town(ax, ay)
            || Self::is_castle(ax, ay)
            || (ax == 0 && ay == 0)
            || (ax == 1 && ay == 0)
            || self.shard_dungeon_at(ax, ay).is_some()
            || self.saltmaze_at(ax, ay)
            || self.rift_at(ax, ay)
    }

    // --- Terrain fields (per world TILE) ---------------------------------------------------
    /// Port of `isWater`: big low-frequency basins + a coast-jitter octave + river bands,
    /// with lake coverage fading in over the first rooms out from the start.
    pub fn is_water(&self, gx: i32, gy: i32) -> bool {
        let b = self.rules_at_tile(gx, gy);
        let d_rooms = (gx.abs() as f64 / COLS as f64).max(gy.abs() as f64 / ROWS as f64);
        let damp = if d_rooms >= 3.0 {
            1.0
        } else if d_rooms <= 1.0 {
            0.0
        } else {
            (d_rooms - 1.0) / 2.0
        };
        let basin = value_noise(self.seed, gx as f64 * LAKE_FREQ, gy as f64 * LAKE_FREQ, SALT_LAKE)
            + (value_noise(self.seed, gx as f64 * COAST_FREQ, gy as f64 * COAST_FREQ, SALT_COAST)
                - 0.5)
                * COAST_AMP;
        if basin < b.water * WATER_CUT * damp {
            return true;
        }
        if b.river
            && (value_noise(self.seed, gx as f64 * RIVER_FREQ, gy as f64 * RIVER_FREQ, SALT_RIVER)
                - 0.5)
                .abs()
                < RIVER_HALF
        {
            return true;
        }
        false
    }
    /// Water with the flat-room override applied by the tile's OWNING room — port of `tileWater`.
    pub fn tile_water(&self, gx: i32, gy: i32) -> bool {
        !self.flat_room(gx.div_euclid(COLS), gy.div_euclid(ROWS)) && self.is_water(gx, gy)
    }
    /// Ground texture name for a walkable tile — port of `groundName`.
    pub fn ground_name(&self, gx: i32, gy: i32) -> &'static str {
        let b = self.rules_at_tile(gx, gy);
        if value_noise(self.seed, gx as f64 * GRASS_FREQ, gy as f64 * GRASS_FREQ, SALT_GRASS)
            > b.grass
        {
            return "grass";
        }
        if value_noise(self.seed, gx as f64 * DIRT_FREQ, gy as f64 * DIRT_FREQ, SALT_DIRT)
            > b.alt_lvl
        {
            // LAVA holds back from FOREIGN seams (Baz): within its own biome it
            // flows room to room exactly like water, but at a biome border it
            // stops 3 tiles short of the edge so a transition never cuts a
            // molten field mid-stream. (ground_name is the one source of truth —
            // the tile paint, the liquid overlay, the burn, the bubbles, and the
            // glow all follow this decision.)
            if b.alt == "lava" {
                let (rx, ry) = (gx.div_euclid(COLS), gy.div_euclid(ROWS));
                let (c, r) = (gx.rem_euclid(COLS), gy.rem_euclid(ROWS));
                let me = self.biome_key_at(rx, ry);
                let foreign = |nx: i32, ny: i32| self.biome_key_at(nx, ny) != me;
                if (c < 3 && foreign(rx - 1, ry))
                    || (c >= COLS - 3 && foreign(rx + 1, ry))
                    || (r < 3 && foreign(rx, ry - 1))
                    || (r >= ROWS - 3 && foreign(rx, ry + 1))
                {
                    return b.ground;
                }
            }
            return b.alt;
        }
        b.ground
    }
    /// Stagnant murk in the grim biomes, clear blue elsewhere — port of `waterStyle`.
    pub fn water_style(&self, gx: i32, gy: i32) -> &'static str {
        let b = self.biome_key_at(gx.div_euclid(COLS), gy.div_euclid(ROWS));
        match b {
            "swamp" | "graveyard" | "burnt" | "chaos" | "hollowwood" | "blackdeep" | "tarmire"
            | "witherlands" => "murk",
            _ => "blue",
        }
    }
    pub fn is_tall_grass(&self, gx: i32, gy: i32) -> bool {
        value_noise(self.seed, gx as f64 * TALL_FREQ, gy as f64 * TALL_FREQ, SALT_TALL) > TALL_LEVEL
    }
    pub fn is_flower_tile(&self, gx: i32, gy: i32) -> bool {
        value_noise(self.seed, gx as f64 * FLOWER_FREQ, gy as f64 * FLOWER_FREQ, SALT_FLOWER)
            > FLOWER_LEVEL
            && (hash(self.seed, gx, gy, SALT_FLOWER) % 100) < 38
    }

    // --- Roads + gates ---------------------------------------------------------------------
    /// Roads never run through a sacred site — port of `roadSpecial`.
    pub fn road_special(&self, rx: i32, ry: i32) -> bool {
        Self::is_castle(rx, ry) || self.shard_dungeon_at(rx, ry).is_some() || self.rift_at(rx, ry)
    }
    /// The edges ('N'/'S'/'E'/'W') a town-to-town road leaves this room by — port of `roadEdges`.
    pub fn road_edges(&self, rx: i32, ry: i32) -> [bool; 4] {
        let mut edges = [false; 4]; // N, S, E, W
        if self.is_town(rx, ry) || self.road_special(rx, ry) {
            return edges;
        }
        let bx = rx.div_euclid(TOWN_CELL);
        let by = ry.div_euclid(TOWN_CELL);
        let add_dir = |nx: i32, ny: i32, edges: &mut [bool; 4]| {
            if nx < rx {
                edges[3] = true; // W
            } else if nx > rx {
                edges[2] = true; // E
            } else if ny < ry {
                edges[0] = true; // N
            } else {
                edges[1] = true; // S
            }
        };
        for dcx in -2..=2 {
            for dcy in -2..=2 {
                let a = self.town_site_of((bx + dcx) * TOWN_CELL + 3, (by + dcy) * TOWN_CELL + 3);
                let Some(a) = a else { continue };
                if !self.is_town(a.tx, a.ty) {
                    continue;
                }
                let e = self
                    .town_site_of((bx + dcx + 1) * TOWN_CELL + 3, (by + dcy) * TOWN_CELL + 3);
                let s = self
                    .town_site_of((bx + dcx) * TOWN_CELL + 3, (by + dcy + 1) * TOWN_CELL + 3);
                for b in [e, s].into_iter().flatten() {
                    if !self.is_town(b.tx, b.ty) {
                        continue;
                    }
                    let path = super::towns::road_path(a.tx, a.ty, b.tx, b.ty);
                    for i in 0..path.len() {
                        if path[i] != (rx, ry) {
                            continue;
                        }
                        if i > 0 && !self.road_special(path[i - 1].0, path[i - 1].1) {
                            add_dir(path[i - 1].0, path[i - 1].1, &mut edges);
                        }
                        if i + 1 < path.len() && !self.road_special(path[i + 1].0, path[i + 1].1) {
                            add_dir(path[i + 1].0, path[i + 1].1, &mut edges);
                        }
                    }
                }
            }
        }
        edges
    }
    /// The edges where a TOWN GATE sits (exactly one side is a town) — port of `gateEdges`.
    pub fn gate_edges(&self, rx: i32, ry: i32) -> [bool; 4] {
        let mut edges = [false; 4]; // N, S, E, W
        if self.road_special(rx, ry) {
            return edges;
        }
        let is_gate = |ax: i32, ay: i32, bx: i32, by: i32| {
            if self.road_special(ax, ay) || self.road_special(bx, by) {
                return false;
            }
            let ta = self.is_town(ax, ay);
            let tb = self.is_town(bx, by);
            (ta || tb) && !(ta && tb)
        };
        if is_gate(rx - 1, ry, rx, ry) {
            edges[3] = true; // W
        }
        if is_gate(rx, ry, rx + 1, ry) {
            edges[2] = true; // E
        }
        if is_gate(rx, ry - 1, rx, ry) {
            edges[0] = true; // N
        }
        if is_gate(rx, ry, rx, ry + 1) {
            edges[1] = true; // S
        }
        edges
    }
}
