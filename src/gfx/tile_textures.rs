//! tile_textures.rs — bake the real tileset (port of the procedural half of js/tiles.js).
//!
//! The raw art grids live in `tiles_art.rs` (GENERATED from the JS — never hand-edit). This
//! module ports the procedural parts: `thin_ground` (4 calmer speck variants per ground,
//! picked per world tile by an avalanche hash), `build_water` (4-phase scrolling wave loop,
//! blue + murk), and the bridge deck orientations. Everything bakes ONCE into a
//! [`TileTextures`] resource at startup.

// Lint policy: thin_ground/build_water mirror js/tiles.js statement-for-statement for
// auditability — allow the reshaping lints for this file only.
#![allow(clippy::needless_range_loop, clippy::manual_is_multiple_of)]

use super::bake::bake;
use super::tiles_art::{ART, CODES, GROUND_DEFS};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

const SIZE: usize = 16;
const GROUND_VARIANTS: usize = 4;

fn art(name: &str) -> &'static [&'static str; 16] {
    &ART.iter().find(|(n, _)| *n == name).expect("unknown art name").1
}

/// Port of `thinGround(art, base, speck, keep, seed)` — thins a ground tile's specks to
/// ~keep and relocates a couple, seeded exactly like the JS so variants match pixel-for-pixel.
pub fn thin_ground_rows(grid: &[&str; 16], base: char, speck: char, keep: f64, seed: u32) -> Vec<String> {
    let mut s: u32 = seed.wrapping_mul(2654435761).wrapping_add(0x9e3779b1);
    let mut rnd = move || -> f64 {
        s = (s ^ (s >> 15)).wrapping_mul(2246822519);
        s ^= s >> 13;
        s as f64 / 4294967296.0
    };
    let mut g: Vec<Vec<char>> = grid.iter().map(|r| r.chars().collect()).collect();
    let mut specks: Vec<(usize, usize)> = Vec::new();
    for y in 0..SIZE {
        for x in 0..SIZE {
            if g[y][x] == speck {
                specks.push((x, y));
            }
        }
    }
    let mut i = specks.len().saturating_sub(1);
    while i > 0 {
        let j = (rnd() * (i + 1) as f64).floor() as usize;
        specks.swap(i, j);
        i -= 1;
    }
    let keep_n = ((specks.len() as f64 * keep).round() as usize).max(1);
    for &(x, y) in specks.iter().skip(keep_n) {
        g[y][x] = base; // thin out the rest
    }
    for &(ox, oy) in specks.iter().take(2.min(keep_n)) {
        // relocate a couple of survivors for variety
        g[oy][ox] = base;
        let (mut nx, mut ny);
        let mut guard = 0;
        loop {
            nx = (rnd() * SIZE as f64).floor() as usize;
            ny = (rnd() * SIZE as f64).floor() as usize;
            if g[ny][nx] != speck || guard >= 16 {
                break;
            }
            guard += 1;
        }
        g[ny][nx] = speck;
    }
    g.into_iter().map(|r| r.into_iter().collect()).collect()
}

/// Port of `buildWater(phase, wave, base)` — wave dashes on a 4-row vertical loop.
pub fn build_water_rows(phase: usize, wave: char, base: char) -> Vec<String> {
    (0..SIZE)
        .map(|y| {
            (0..SIZE)
                .map(|x| if (y + phase) % 4 == 0 && (x + y) % 6 < 2 { wave } else { base })
                .collect()
        })
        .collect()
}

fn bake_rows(rows: &[String], overrides: &[(char, u32)], images: &mut Assets<Image>) -> Handle<Image> {
    let refs: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
    images.add(bake(&refs, overrides))
}

/// Every baked tile texture, ready at startup. One source of truth for tile LOOKS
/// (solidity stays in `tiles.rs` — one source of truth for tile PHYSICS).
#[derive(Resource)]
pub struct TileTextures {
    codes: HashMap<char, Handle<Image>>,
    grounds: HashMap<&'static str, [Handle<Image>; GROUND_VARIANTS]>,
    water: [Handle<Image>; 4],
    murk: [Handle<Image>; 4],
    bridge_v: Handle<Image>,
    bridge_h: Handle<Image>,
    bridge_x: Handle<Image>,
}

impl TileTextures {
    pub fn build(images: &mut Assets<Image>) -> Self {
        // Fixed-art tiles (walls, paths, roads) straight from the code table + overrides.
        let mut codes = HashMap::new();
        for (code, art_name, _solid, overrides) in CODES {
            codes.insert(*code, images.add(bake(art(art_name), overrides)));
        }
        // Ground variants: seeded exactly like the JS (i*131 + first char code * 17 + 1).
        let mut grounds = HashMap::new();
        for (name, base, speck, art_name) in GROUND_DEFS {
            let seed0 = name.bytes().next().unwrap_or(0) as u32 * 17 + 1;
            let variants: [Handle<Image>; GROUND_VARIANTS] = std::array::from_fn(|i| {
                let rows = thin_ground_rows(art(art_name), *base, *speck, 0.6, i as u32 * 131 + seed0);
                bake_rows(&rows, &[], images)
            });
            grounds.insert(*name, variants);
        }
        let water = std::array::from_fn(|p| bake_rows(&build_water_rows(p, 'w', 'V'), &[], images));
        let murk = std::array::from_fn(|p| bake_rows(&build_water_rows(p, 'u', 'U'), &[], images));
        Self {
            codes,
            grounds,
            water,
            murk,
            bridge_v: images.add(bake(art("bridge"), &[])),
            bridge_h: images.add(bake(art("bridgeH"), &[])),
            bridge_x: images.add(bake(art("bridgeX"), &[])),
        }
    }

    /// Fixed tile for a code (walls, paths, roads) — `Tiles.get(code).canvas`.
    pub fn code(&self, code: char) -> Handle<Image> {
        self.codes.get(&code).or_else(|| self.codes.get(&'.')).unwrap().clone()
    }

    /// Ground variant for a world tile — `Tiles.groundCanvas(name, x, y)` (avalanche hash so
    /// neighbours don't stripe).
    pub fn ground(&self, name: &str, x: i32, y: i32) -> Handle<Image> {
        let arr = self.grounds.get(name).unwrap_or_else(|| &self.grounds["sand"]);
        let mut h = (x.wrapping_add(1) as u32)
            .wrapping_mul(374761393)
            .wrapping_add((y.wrapping_add(1) as u32).wrapping_mul(668265263));
        h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        h ^= h >> 16;
        arr[(h as usize) % arr.len()].clone()
    }

    /// Water frame — `Tiles.waterCanvas(frame, style)`.
    pub fn water(&self, frame: i64, style: &str) -> Handle<Image> {
        let idx = frame.rem_euclid(4) as usize;
        if style == "murk" { self.murk[idx].clone() } else { self.water[idx].clone() }
    }

    /// Bridge deck by walkable-neighbour orientation — `Tiles.bridgeCanvas(h, v)`.
    pub fn bridge(&self, h: bool, v: bool) -> Handle<Image> {
        if h && v {
            self.bridge_x.clone()
        } else if h {
            self.bridge_h.clone()
        } else {
            self.bridge_v.clone()
        }
    }
}
