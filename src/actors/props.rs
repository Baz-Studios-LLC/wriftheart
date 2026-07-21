//! props.rs — wilderness prop art: the SEEDED tree generators (port of buildOak/buildPine/
//! buildCactus/buildDeadtree + their helpers from js/entities.js — every tree in the world
//! is unique, seeded by its tile) and the [`PropArt`] bank that bakes + caches everything,
//! including the fixed grids machine-extracted into `props_art.rs`.
//!
//! Grid parity with the JS generators is pinned by tests/treeart_parity.rs (sampled seeds).

use super::props_art::{
    BUSH_VARIANTS, CLUTTER_ART, FLOWER_BASE, FLOWER_COLS, GRASS_FRAMES, ORE_NODES, PROP_ANCHORS,
};
use crate::gfx::bake;
use bevy::image::Image;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

// ---------- shared helpers (ports of blank/outlined/pxHash/drawTrunk) ----------

pub(crate) fn blank(w: usize, h: usize) -> Vec<Vec<char>> {
    vec![vec!['.'; w]; h]
}

/// 1px 'K' outline around every non-empty pixel (port of `outlined`).
pub(crate) fn outlined(g: &[Vec<char>]) -> Vec<String> {
    let (w, h) = (g[0].len() as i32, g.len() as i32);
    let mut o = g.to_vec();
    for y in 0..h {
        for x in 0..w {
            if g[y as usize][x as usize] != '.' {
                continue;
            }
            let near = (-1..=1).any(|dy| {
                (-1..=1).any(|dx| {
                    let (nx, ny) = (x + dx, y + dy);
                    (0..w).contains(&nx)
                        && (0..h).contains(&ny)
                        && g[ny as usize][nx as usize] != '.'
                        && g[ny as usize][nx as usize] != 'K'
                })
            });
            if near {
                o[y as usize][x as usize] = 'K';
            }
        }
    }
    o.iter().map(|r| r.iter().collect()).collect()
}

/// Avalanche pixel hash 0..999 (port of `pxHash` — Math.imul chain, bit-exact).
pub(crate) fn px_hash(x: i32, y: i32, s: i32) -> u32 {
    let h = (x.wrapping_add(1))
        .wrapping_mul(374761393)
        .wrapping_add((y.wrapping_add(1)).wrapping_mul(668265263))
        .wrapping_add((s.wrapping_add(1)).wrapping_mul(1274126177));
    let h = h as u32;
    let h = ((h ^ (h >> 13)) as i32).wrapping_mul(2246822519u32 as i32) as u32;
    (h ^ (h >> 16)) % 1000
}

/// Tapered trunk with a root flare (port of `drawTrunk`).
fn draw_trunk(g: &mut [Vec<char>], cx: i32, top_y: i32, h: i32) {
    for y in top_y..h {
        let t = (y - top_y) as f64 / (h - top_y) as f64;
        let mut half = 4 + (t * 1.5).round() as i32;
        if y >= h - 2 {
            half += 1;
        }
        for x in cx - half..=cx + half {
            let f = (x - (cx - half)) as f64 / (2 * half) as f64;
            g[y as usize][x as usize] = if f > 0.72 { 'd' } else { 'D' };
        }
    }
}

/// Per-tile sprite seed (port of `seedAt` — plain f64 multiply then ToInt32, NOT imul).
pub fn seed_at(x: i32, y: i32) -> i32 {
    let tx = x.div_euclid(16) as i64;
    let ty = y.div_euclid(16) as i64;
    let h = ((tx * 73856093) as i32) ^ ((ty * 19349663) as i32);
    ((h as u32) % 4096) as i32
}

/// Stable per-tile variant index (port of `pickVariant`).
pub fn pick_variant(x: i32, y: i32, salt: i32, n: usize) -> usize {
    let tx = x.div_euclid(16) as i64;
    let ty = y.div_euclid(16) as i64;
    let h = (((tx * 73856093) as i32) ^ ((ty * 19349663) as i32)) ^ salt;
    ((h as u32) as usize) % n
}

// ---------- the seeded tree generators ----------

/// Oak: a clumped canopy whose silhouette is the union of its leaf lobes (port of `buildOak`).
pub fn build_oak(r_base: f64, seed: i32) -> Vec<String> {
    let (w, h, cx, cy) = (48i32, 72i32, 24i32, 23i32);
    let rb = r_base * 1.1;
    let mut g = blank(w as usize, h as usize);
    draw_trunk(&mut g, cx, 31, h);
    let rr_of = |n: i32| px_hash(n, seed, seed * 3 + 7) as f64 / 1000.0;
    struct Clump {
        x: f64,
        y: f64,
        r: f64,
    }
    let mut clumps = vec![Clump { x: cx as f64, y: (cy + 1) as f64, r: rb * 0.62 }];
    let nc = 5 + seed % 3;
    for i in 0..nc {
        let a = (i as f64 / nc as f64) * std::f64::consts::TAU + (rr_of(i) - 0.5) * 1.1;
        let rr = rb * (0.26 + rr_of(i + 9) * 0.24);
        let x = cx as f64 + a.cos() * rr;
        let y = cy as f64 + a.sin() * rr * 0.75;
        let r = (rb * (0.34 + rr_of(i + 17) * 0.18)).min(22.0 - (x - cx as f64).abs());
        clumps.push(Clump { x, y, r });
    }
    clumps.push(Clump {
        x: (cx - 2) as f64 + rr_of(31) * 5.0,
        y: cy as f64 - rb * 0.34,
        r: rb * (0.5 + rr_of(37) * 0.15),
    });
    for y in 0..h {
        for x in 0..w {
            let (mut d1, mut d2) = (1e9f64, 1e9f64);
            let mut own = &clumps[0];
            for c in &clumps {
                let d = (x as f64 - c.x).hypot((y as f64 - c.y) * 1.08) / c.r;
                if d < d1 {
                    d2 = d1;
                    d1 = d;
                    own = c;
                } else if d < d2 {
                    d2 = d;
                }
            }
            if d1 > 1.0 {
                continue;
            }
            let v = ((x as f64 - own.x) / own.r) * 0.5
                + ((y as f64 - own.y) / own.r) * 0.75
                + ((y - cy) as f64 / rb) * 0.35;
            let mut c = if v < -0.42 { 'l' } else if v > 0.48 { 'E' } else { 'G' };
            if d2 - d1 < 0.14 && d1 > 0.5 && d1 < 0.85 && px_hash(x, y, seed + 99) < 700 {
                c = 'E'; // crease where two lobes meet
            }
            let n = px_hash(x, y, seed);
            if c == 'G' && n < 45 {
                c = 'l';
            } else if c == 'G' && n > 972 {
                c = 'E';
            }
            g[y as usize][x as usize] = c;
        }
    }
    // 1-2 low branches with leaf tufts.
    let branch = |g: &mut Vec<Vec<char>>, side: i32, by: i32, len: i32, n: i32| {
        let tip_x = cx + side * (4 + len);
        let tip_y = by - 2 - (rr_of(n + 7) * 3.0).floor() as i32;
        for i in 0..=len {
            let bx = cx + side * (4 + i);
            let byy = (by as f64 + (tip_y - by) as f64 * (i as f64 / len as f64)).round() as i32;
            if bx <= 0 || bx >= w {
                continue;
            }
            if (0..h).contains(&byy) && g[byy as usize][bx as usize] == '.' {
                g[byy as usize][bx as usize] = 'D';
            }
            if (0..h - 1).contains(&byy) && g[(byy + 1) as usize][bx as usize] == '.' {
                g[(byy + 1) as usize][bx as usize] = 'd';
            }
        }
        let pr = 3.5 + rr_of(n + 11) * 1.5;
        for yy in (tip_y as f64 - pr).round() as i32..=(tip_y as f64 + pr).round() as i32 {
            for xx in (tip_x as f64 - pr - 1.0).round() as i32..=(tip_x as f64 + pr + 1.0).round() as i32 {
                if !(0..h).contains(&yy) || xx < 1 || xx >= w - 1 {
                    continue;
                }
                if ((xx - tip_x) as f64 * 0.9).hypot((yy - tip_y) as f64 * 1.15) / pr > 1.0 {
                    continue;
                }
                let vv = ((xx - tip_x) as f64 * 0.5 + (yy - tip_y) as f64 * 0.8) / pr;
                g[yy as usize][xx as usize] = if vv < -0.35 { 'l' } else if vv > 0.5 { 'E' } else { 'G' };
            }
        }
    };
    let s1 = if rr_of(51) < 0.5 { -1 } else { 1 };
    let by1 = 43 + (rr_of(50) * 6.0).floor() as i32;
    let ln1 = 10 + (rr_of(52) * 4.0).floor() as i32;
    branch(&mut g, s1, by1, ln1, 53);
    if rr_of(61) < 0.55 {
        let by2 = by1 + 6 + (rr_of(62) * 4.0).floor() as i32;
        let ln2 = 7.max(ln1 - 3 - (rr_of(63) * 3.0).floor() as i32);
        branch(&mut g, -s1, by2, ln2, 67);
    }
    outlined(&g)
}

/// Pine: discrete drooping tiers, drawn bottom-up (port of `buildPine`).
pub fn build_pine(max_hw: i32, bot: i32, seed: i32) -> Vec<String> {
    let (w, h, cx, top) = (40i32, 72i32, 20i32, 2i32);
    let mut g = blank(w as usize, h as usize);
    draw_trunk(&mut g, cx, bot - 5, h);
    const TIERS: i32 = 4;
    for i in (0..TIERS).rev() {
        let t0 = i as f64 / TIERS as f64;
        let t1 = (i + 1) as f64 / TIERS as f64;
        let y0 = (top as f64 + t0 * (bot - top) as f64).round() as i32;
        let y1 = (top as f64 + t1 * (bot - top) as f64).round() as i32;
        let hw_top = 1.max((1.0 + t0 * max_hw as f64 * 0.55).round() as i32);
        let hw_bot = 2.max((3.0 + t1 * max_hw as f64).round() as i32);
        for y in y0..=y1.min(bot) {
            let f = (y - y0) as f64 / 1.max(y1 - y0) as f64;
            let hw = (hw_top as f64 + (hw_bot - hw_top) as f64 * f).round() as i32;
            for x in cx - hw..=cx + hw {
                if !(0..w).contains(&x) {
                    continue;
                }
                let mut c = 'G';
                if f < 0.28 && (x as f64) < cx as f64 + hw as f64 * 0.5 {
                    c = 'l';
                } else if f > 0.9 || x > cx + hw - 2 {
                    c = 'E'; // shadowed hem row / dark right edge (kept as the JS wrote it)
                }
                let n = px_hash(x, y, seed);
                if c == 'G' && n < 40 {
                    c = 'l';
                } else if c == 'G' && n > 975 {
                    c = 'E';
                }
                g[y as usize][x as usize] = c;
            }
        }
    }
    outlined(&g)
}

/// Cactus: saguaro column + two arms; seeded height (port of `buildCactus`).
pub fn build_cactus(variant: i32, col_top: i32) -> Vec<String> {
    let (w, h, cx) = (32i32, 56i32, 16i32);
    let mut g = blank(w as usize, h as usize);
    let fill = |g: &mut Vec<Vec<char>>, x0: i32, x1: i32, y0: i32, y1: i32| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                if (0..w).contains(&x) && (0..h).contains(&y) {
                    g[y as usize][x as usize] = if x == x0 { 'G' } else { 'g' };
                }
            }
        }
    };
    fill(&mut g, cx - 3, cx + 2, col_top, h - 1);
    let span = h - 1 - col_top;
    let e1 = col_top + (span as f64 * 0.50).round() as i32;
    let t1 = col_top + (span as f64 * 0.20).round() as i32;
    let e2 = col_top + (span as f64 * 0.62).round() as i32;
    let t2 = col_top + (span as f64 * 0.34).round() as i32;
    if variant == 0 {
        fill(&mut g, cx - 8, cx - 4, e1, e1 + 4);
        fill(&mut g, cx - 8, cx - 7, t1, e1 + 4);
        fill(&mut g, cx + 3, cx + 7, e2, e2 + 4);
        fill(&mut g, cx + 6, cx + 7, t2, e2 + 4);
    } else {
        fill(&mut g, cx + 3, cx + 7, e1, e1 + 4);
        fill(&mut g, cx + 6, cx + 7, t1, e1 + 4);
        fill(&mut g, cx - 8, cx - 4, e2, e2 + 4);
        fill(&mut g, cx - 8, cx - 7, t2, e2 + 4);
    }
    outlined(&g)
}

/// Dead swamp tree: gnarled bare trunk, crooked limbs, hanging moss (port of `buildDeadtree`).
pub fn build_deadtree(seed: i32) -> Vec<String> {
    let (w, h, cx) = (48i32, 72i32, 24i32);
    let mut g = blank(w as usize, h as usize);
    let mut s = (seed as i64 * 2654435761 + 1) as u32;
    let mut rnd = move || {
        s = ((s ^ (s >> 15)) as i32).wrapping_mul(2246822519u32 as i32) as u32;
        s ^= s >> 13;
        s as f64 / 4294967296.0
    };
    let top = 16 + (rnd() * 6.0).floor() as i32;
    draw_trunk(&mut g, cx, top, h);
    let put = |g: &mut Vec<Vec<char>>, x: i32, y: i32, ch: char| {
        if (0..w).contains(&x) && (0..h).contains(&y) {
            g[y as usize][x as usize] = ch;
        }
    };
    let mut limb = |g: &mut Vec<Vec<char>>, mut x: i32, mut y: i32, dx: i32, len: i32| -> (i32, i32) {
        for _ in 0..len {
            put(g, x, y, 'd');
            put(g, x, y - 1, 'D');
            x += dx;
            if rnd() < 0.65 {
                y -= 1;
            }
        }
        (x, y)
    };
    let tips = [
        limb(&mut g, cx - 3, top + 8, -1, 9),
        limb(&mut g, cx + 3, top + 4, 1, 10),
        limb(&mut g, cx - 2, top + 17, -1, 7),
        limb(&mut g, cx + 2, top + 21, 1, 7),
    ];
    for (tx, ty) in tips {
        let n = 2 + (rnd() * 3.0).floor() as i32;
        for _ in 0..n {
            let mx = tx + (rnd() * 5.0).floor() as i32 - 2;
            let len = 3 + (rnd() * 5.0).floor() as i32;
            for k in 0..len {
                let ch = if rnd() < 0.4 { 'q' } else { 'E' };
                put(&mut g, mx, ty + k, ch);
            }
        }
    }
    outlined(&g)
}

/// A tree grid by kind + per-tile seed — the dispatch half of the JS `*Bmp` functions.
/// Unported exotic kinds fall back to the oak silhouette (their real art comes with their
/// biomes; anchors are identical so nothing shifts).
pub fn tree_grid(kind: &str, seed: i32) -> Vec<String> {
    use super::tree_art as ta;
    match kind {
        "pine" => build_pine(14 + (seed % 5), 54 + (seed % 6), seed),
        "cactus" => build_cactus((seed >> 5) & 1, 4 + (seed % 23)),
        "deadtree" => build_deadtree(seed),
        // The exotic biome kinds (tree_art.rs) — until these landed, every one fell
        // back to the green oak silhouette: eleven biomes of wrong trees.
        "shroom" => ta::build_shroom(seed),
        "burnttree" => ta::build_burnttree(seed),
        "riftbulb" => ta::build_riftbulb(seed),
        "voidspire" => ta::build_voidspire(seed),
        "mawtree" => ta::build_mawtree(seed),
        "giantflower" => ta::build_giantflower(seed),
        "crystalspire" => ta::build_crystalspire(seed),
        "stalagmite" => ta::build_stalagmite(seed),
        _ => build_oak((19 + (seed % 3)) as f64, seed),
    }
}

/// The per-kind bake palette: the oak-silhouette recolours (js oakVariant — blossom/
/// jungle/bluebell re-tint the l/G/E leaf chars), the giant flower's SEEDED petal hue,
/// and everyone else on the default global palette. (Pub for the headless tree-sheet
/// example — it must bake exactly as the game does.)
pub fn tree_pal(kind: &str, seed: i32) -> Vec<(char, u32)> {
    match kind {
        "blossom" => vec![('l', 0xffd0ec), ('G', 0xf070b0), ('E', 0xc84e90)], // candy-pink petals
        "jungletree" => vec![('l', 0x7fe88a), ('G', 0x1ca838), ('E', 0x0a6a1c)], // deep jungle green
        "bluebloom" => vec![('l', 0xa9d4ff), ('G', 0x5a8fe0), ('E', 0x2a5aa8)], // soft blue blossom
        "giantflower" => super::tree_art::giantflower_pal(seed),
        _ => vec![],
    }
}

// ---------- the art bank ----------

/// Baked prop art, cached: fixed grids at startup, seeded trees on demand.
#[derive(Resource, Default)]
pub struct PropArt {
    pub bushes: Vec<Handle<Image>>,
    /// Town storefronts by building kind (buildings_art.rs, 48x48).
    pub fronts: HashMap<&'static str, Handle<Image>>,
    /// The player's bespoke farmhouse (buildings_art::FARMHOUSE, 48x52).
    pub farmhouse: Handle<Image>,
    pub well: Handle<Image>,
    pub torch: [Handle<Image>; 2],
    /// Ore-node boulders: [tier 1..=5 -> 15 variants].
    pub boulders: Vec<Vec<Handle<Image>>>,
    pub grass: Vec<Handle<Image>>,
    pub flowers: Vec<Handle<Image>>,
    pub clutter: HashMap<&'static str, Handle<Image>>,
    trees: HashMap<(String, i32), Handle<Image>>,
    stages: HashMap<(String, u8), Handle<Image>>,
}

impl PropArt {
    pub fn build(images: &mut Assets<Image>) -> Self {
        let mut art = PropArt {
            bushes: BUSH_VARIANTS.iter().map(|g| images.add(bake(g, &[]))).collect(),
            farmhouse: {
                let (grid, pal) = crate::actors::buildings_art::FARMHOUSE;
                images.add(bake(grid, pal))
            },
            boulders: ORE_NODES
                .iter()
                .map(|(main, lite, grids)| {
                    grids.iter().map(|g| images.add(bake(g, &[('v', *main), ('V', *lite)]))).collect()
                })
                .collect(),
            grass: GRASS_FRAMES.iter().map(|g| images.add(bake(g, &[]))).collect(),
            flowers: FLOWER_COLS.iter().map(|c| images.add(bake(FLOWER_BASE, &[('p', *c)]))).collect(),
            clutter: HashMap::default(),
            trees: HashMap::default(),
            stages: HashMap::default(),
            fronts: crate::actors::buildings_art::TOWN_FRONTS
                .iter()
                .map(|(kind, grid, pal)| (*kind, images.add(bake(grid, pal))))
                .collect(),
            well: images.add(bake(crate::actors::buildings_art::WELL.0, crate::actors::buildings_art::WELL.1)),
            torch: [
                images.add(bake(crate::actors::buildings_art::TORCH_FRAMES[0].0, crate::actors::buildings_art::TORCH_FRAMES[0].1)),
                images.add(bake(crate::actors::buildings_art::TORCH_FRAMES[1].0, crate::actors::buildings_art::TORCH_FRAMES[1].1)),
            ],
        };
        for (kind, grid) in CLUTTER_ART {
            art.clutter.insert(kind, images.add(bake(grid, &[])));
        }
        art
    }

    /// The seeded tree sprite for a prop at room position (x, y) — cached per (kind, seed).
    pub fn tree(&mut self, kind: &str, x: i32, y: i32, images: &mut Assets<Image>) -> Handle<Image> {
        let seed = seed_at(x, y);
        if let Some(h) = self.trees.get(&(kind.to_string(), seed)) {
            return h.clone();
        }
        let h = images.add(bake(
            &tree_grid(kind, seed).iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            &tree_pal(kind, seed),
        ));
        self.trees.insert((kind.to_string(), seed), h.clone());
        h
    }
}

impl PropArt {
    /// A tree's growth-stage sprite (0 stump / 1 sapling / 2 young), tinted + cached.
    pub fn stage(&mut self, kind: &str, stage: u8, images: &mut Assets<Image>) -> Handle<Image> {
        if let Some(h) = self.stages.get(&(kind.to_string(), stage)) {
            return h.clone();
        }
        let (grid, pal) = stage_grid(kind, stage);
        let h = images.add(bake(&grid.iter().map(|s| s.as_str()).collect::<Vec<_>>(), &pal));
        self.stages.insert((kind.to_string(), stage), h.clone());
        h
    }
}

/// Draw anchor + hitbox for a big prop kind (port of the js PROPS table; oak fallback).
pub fn prop_anchor(kind: &str) -> (i32, i32, i32, i32, i32, i32) {
    PROP_ANCHORS
        .iter()
        .find(|(k, ..)| *k == kind)
        .or_else(|| PROP_ANCHORS.iter().find(|(k, ..)| *k == "oak"))
        .map(|&(_, ox, oy, hx, hy, hw, hh)| (ox, oy, hx, hy, hw, hh))
        .unwrap()
}

// ---------- tree growth stages (port of the stage builders in js/entities.js) ----------
// A felled tree grows back over 3 days: stump -> sapling -> young -> full. Stage sprites
// are built on the same 48x72 canvas as the full trees (same anchor), tinted per kind.

/// [trunk, trunkDark, foliage] per tree kind — port of STAGE_COL (oak fallback).
fn stage_col(kind: &str) -> (u32, u32, u32) {
    match kind {
        "pine" => (0x5a3c20, 0x3a2410, 0x2f7a38),
        "blossom" => (0x7c4c1c, 0x503000, 0xf070b0),
        "jungletree" => (0x5a4028, 0x3a2814, 0x1ca838),
        "bluebloom" => (0x7c4c1c, 0x503000, 0x5a8fe0),
        "giantflower" => (0x4a7a2a, 0x2f5018, 0xff6aa8),
        "deadtree" => (0x6a5a48, 0x463a2c, 0x6a5a48),
        "shroom" => (0xe8dcc0, 0xb8a878, 0xc05050),
        "burnttree" => (0x3a3230, 0x1c1614, 0x5a2a20),
        "riftbulb" => (0x7028a8, 0x4a1870, 0xb060f0),
        "voidspire" => (0x4a2070, 0x2a1048, 0x7028a8),
        "mawtree" => (0x5a2a2a, 0x3a1616, 0xa02828),
        _ => (0x7c4c1c, 0x503000, 0x3a9a4a), // oak + everything else
    }
}

/// Brighten/darken a colour channel-wise (port of `shadeHex`).
fn shade_hex(hex: u32, f: f64) -> u32 {
    let c = |v: u32| (((v as f64 * f).round() as i64).clamp(0, 255)) as u32;
    (c((hex >> 16) & 255) << 16) | (c((hex >> 8) & 255) << 8) | c(hex & 255)
}

const SG_W: i32 = 48;
const SG_H: i32 = 72;
const SG_CX: i32 = 24;

/// The full tree's own trunk taper (t anchored at the full trunk top, y=31), slimmable.
fn sg_trunk(g: &mut [Vec<char>], top_y: i32, shrink: f64) {
    for y in top_y..SG_H {
        let t = (y - 31) as f64 / (SG_H - 31) as f64;
        let mut half = 2.max(((4.0 + t * 1.5) * shrink).round() as i32);
        if y >= SG_H - 2 {
            half += 1;
        }
        for x in SG_CX - half..=SG_CX + half {
            let f = (x - (SG_CX - half)) as f64 / (2 * half) as f64;
            g[y as usize][x as usize] = if f > 0.72 { 'd' } else { 'D' };
        }
    }
}

fn sg_ell(g: &mut [Vec<char>], ecx: f64, ecy: f64, rx: f64, ry: f64, ch: char) {
    for y in 0f64.max((ecy - ry).floor()) as i32..=((SG_H - 1) as f64).min((ecy + ry).ceil()) as i32 {
        for x in 0f64.max((ecx - rx).floor()) as i32..=((SG_W - 1) as f64).min((ecx + rx).ceil()) as i32 {
            let dx = (x as f64 - ecx) / rx;
            let dy = (y as f64 - ecy) / ry;
            if dx * dx + dy * dy <= 1.0 {
                g[y as usize][x as usize] = ch;
            }
        }
    }
}

/// STUMP: the bottom 12 rows of the real trunk with a light cut face + ring.
fn build_stage_stump() -> Vec<String> {
    let mut g = blank(SG_W as usize, SG_H as usize);
    let top_y = SG_H - 12;
    sg_trunk(&mut g, top_y, 1.0);
    sg_ell(&mut g, SG_CX as f64, (top_y + 1) as f64, 5.5, 2.2, 'L');
    sg_ell(&mut g, SG_CX as f64, (top_y + 1) as f64, 3.0, 1.2, 'l');
    g[(top_y + 1) as usize][SG_CX as usize] = 'd';
    outlined(&g)
}

/// SAPLING (~26px): a slim stem with a crown tuft + two side leaves.
fn build_stage_sapling() -> Vec<String> {
    let mut g = blank(SG_W as usize, SG_H as usize);
    let top_y = SG_H - 26;
    for y in top_y + 6..SG_H {
        for x in SG_CX - 1..=SG_CX + 1 {
            g[y as usize][x as usize] = if x == SG_CX + 1 { 'd' } else { 'D' };
        }
        if y >= SG_H - 2 {
            g[y as usize][(SG_CX - 2) as usize] = 'D';
            g[y as usize][(SG_CX + 2) as usize] = 'd';
        }
    }
    let (cx, ty) = (SG_CX as f64, top_y as f64);
    sg_ell(&mut g, cx, ty + 4.0, 5.0, 4.0, 'F');
    sg_ell(&mut g, cx - 2.0, ty + 3.0, 2.5, 2.0, 'f');
    sg_ell(&mut g, cx - 6.0, ty + 10.0, 3.5, 2.5, 'F');
    sg_ell(&mut g, cx - 7.0, ty + 9.0, 2.0, 1.5, 'f');
    sg_ell(&mut g, cx + 6.0, ty + 13.0, 3.5, 2.5, 'F');
    sg_ell(&mut g, cx + 5.0, ty + 12.0, 2.0, 1.5, 'f');
    outlined(&g)
}

/// YOUNG (~46px): a real mini-tree in the round-canopy family silhouette.
fn build_stage_young_round(seed: i32) -> Vec<String> {
    let mut g = blank(SG_W as usize, SG_H as usize);
    let r_of = |n: i32| px_hash(n, seed, seed * 3 + 7) as f64 / 1000.0;
    sg_trunk(&mut g, SG_H - 26, 0.75);
    let cy = (SG_H - 34) as f64;
    let rb = 12.0;
    let mut clumps = vec![(SG_CX as f64, cy, rb * 0.7)];
    for i in 0..5 {
        let a = (i as f64 / 5.0) * std::f64::consts::TAU + (r_of(i) - 0.5);
        clumps.push((SG_CX as f64 + a.cos() * rb * 0.45, cy + a.sin() * rb * 0.34, rb * (0.42 + r_of(i + 9) * 0.16)));
    }
    clumps.push((SG_CX as f64, cy - rb * 0.4, rb * 0.55));
    for y in 0..SG_H {
        for x in 0..SG_W {
            let (mut d1, mut own) = (1e9f64, clumps[0]);
            for c in &clumps {
                let d = (x as f64 - c.0).hypot((y as f64 - c.1) * 1.08) / c.2;
                if d < d1 {
                    d1 = d;
                    own = *c;
                }
            }
            if d1 > 1.0 {
                continue;
            }
            let v = ((x as f64 - own.0) / own.2) * 0.5 + ((y as f64 - own.1) / own.2) * 0.75;
            g[y as usize][x as usize] = if v < -0.4 { 'f' } else if v > 0.5 { 'T' } else { 'F' };
        }
    }
    outlined(&g)
}

fn build_stage_young_conifer() -> Vec<String> {
    let mut g = blank(SG_W as usize, SG_H as usize);
    sg_trunk(&mut g, SG_H - 20, 0.7);
    let top = SG_H - 48;
    for ti in 0..3 {
        let ty = top + 8 + ti * 10;
        let hw = 7.0 + ti as f64 * 3.5;
        for y in ty..ty + 9 {
            let t = (y - ty) as f64 / 9.0;
            let half = (hw * (0.25 + t * 0.75)).round() as i32;
            for x in SG_CX - half..=SG_CX + half {
                let f = (x - (SG_CX - half)) as f64 / (2 * half).max(1) as f64;
                g[y as usize][x as usize] = if f < 0.3 { 'f' } else if f > 0.72 { 'T' } else { 'F' };
            }
        }
    }
    sg_ell(&mut g, SG_CX as f64, (top + 6) as f64, 2.5, 4.0, 'F');
    g[(top + 3) as usize][SG_CX as usize] = 'F';
    g[(top + 4) as usize][SG_CX as usize] = 'f';
    outlined(&g)
}

fn build_stage_young_bare(seed: i32) -> Vec<String> {
    let mut g = blank(SG_W as usize, SG_H as usize);
    let r_of = |n: i32| px_hash(n, seed, seed * 3 + 7) as f64 / 1000.0;
    sg_trunk(&mut g, SG_H - 30, 0.7);
    for i in 0..4 {
        let a = -std::f64::consts::FRAC_PI_2 + (i as f64 - 1.5) * 0.55 + (r_of(i) - 0.5) * 0.3;
        let len = 12.0 + r_of(i + 5) * 6.0;
        let (mut bx, mut by) = (SG_CX as f64, (SG_H - 28) as f64);
        let mut s = 0.0;
        while s < len {
            bx += a.cos();
            by += a.sin();
            let (ix, iy) = (bx.round() as i32, by.round() as i32);
            if (0..SG_H).contains(&iy) && ix > 0 && ix < SG_W - 1 {
                g[iy as usize][ix as usize] = 'D';
                if s < len * 0.5 && g[iy as usize][(ix + 1) as usize] == '.' {
                    g[iy as usize][(ix + 1) as usize] = 'd';
                }
            }
            s += 1.0;
        }
    }
    outlined(&g)
}

fn build_stage_young_shroom() -> Vec<String> {
    let mut g = blank(SG_W as usize, SG_H as usize);
    for y in SG_H - 24..SG_H {
        for x in SG_CX - 3..=SG_CX + 3 {
            g[y as usize][x as usize] = if x > SG_CX + 1 { 'd' } else { 'D' };
        }
    }
    sg_ell(&mut g, SG_CX as f64, (SG_H - 28) as f64, 13.0, 8.0, 'F');
    sg_ell(&mut g, (SG_CX - 4) as f64, (SG_H - 31) as f64, 6.0, 4.0, 'f');
    let yy = (SG_H - 23) as usize;
    let mut x = SG_CX - 9;
    while x <= SG_CX + 9 {
        if g[yy][x as usize] == '.' {
            g[yy][x as usize] = 'L';
            g[yy][(x + 1) as usize] = 'L';
        }
        x += 4;
    }
    sg_ell(&mut g, (SG_CX - 5) as f64, (SG_H - 29) as f64, 1.5, 1.0, 'L');
    sg_ell(&mut g, (SG_CX + 5) as f64, (SG_H - 26) as f64, 1.5, 1.0, 'L');
    outlined(&g)
}

/// Deterministic per-kind seed for the young-stage silhouette (port of `kindSeed`).
fn kind_seed(kind: &str) -> i32 {
    let mut h = 0u32;
    for b in kind.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as u32);
    }
    (h % 4096) as i32
}

/// A growth-stage sprite for a tree kind (0 stump, 1 sapling, 2 young) — tinted per kind,
/// cached. Same 48x72 canvas + anchor as the full tree.
pub fn stage_grid(kind: &str, stage: u8) -> (Vec<String>, Vec<(char, u32)>) {
    let (t, td, fol) = stage_col(kind);
    match stage {
        0 => (build_stage_stump(), vec![('D', t), ('d', td), ('L', 0xc89a5a), ('l', 0xa87840), ('K', 0x000000)]),
        1 => (build_stage_sapling(), vec![('D', t), ('d', td), ('F', fol), ('f', shade_hex(fol, 1.28)), ('K', 0x000000)]),
        _ => {
            let grid = match kind {
                "pine" => build_stage_young_conifer(),
                "deadtree" | "burnttree" => build_stage_young_bare(kind_seed(kind)),
                "shroom" => build_stage_young_shroom(),
                _ => build_stage_young_round(kind_seed(kind)),
            };
            let cut = if kind == "shroom" { 0xf8ecd8 } else { 0xc89a5a };
            (grid, vec![('D', t), ('d', td), ('F', fol), ('f', shade_hex(fol, 1.28)), ('T', shade_hex(fol, 0.6)), ('L', cut), ('K', 0x000000)])
        }
    }
}
