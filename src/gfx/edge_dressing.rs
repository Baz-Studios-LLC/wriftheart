//! edge_dressing.rs — the room's terrain edge dressing: the two pixel passes at the bottom
//! of room.js `draw()`, ported rect-for-rect (see tests/dressing_parity.rs).
//!
//! 1. **Rounded terrain corners**: knock the blocky 90° corners off every water↔ground and
//!    ground-type↔ground-type boundary. A convex corner — where the three tiles meeting it
//!    are all ONE other kind — gets a small symmetric pixel nook (rows of width 5,3,2,1,1)
//!    filled with that neighbour's flat colour, so coastlines and dirt paths read as smooth
//!    curves instead of stair-steps.
//! 2. **Scalloped hedges**: leafy walls (tree 'T' / jungle 'G') blit as flat dark tiles, so
//!    a run reads as one boxy rectangle. Every edge facing OPEN ground gets rounded leaf
//!    bumps (5-9px wide, 2-4px tall, lit at the crown) swelling out into the ground, with a
//!    1px ground notch between bumps. Hashed on WORLD coords: stable and continuous across
//!    tiles and rooms.
//!
//! The passes emit a rect list (the JS fillRect stream) which [`build_overlay`] rasterises
//! into ONE transparent per-room image, spawned above the tile layer — visually identical
//! to the JS painting over its canvas, but static, so it costs one sprite.

use crate::room::{RoomGrid, PX_H, PX_W, TILE};
use crate::tiles::{ground_base, is_solid, water_color};
use crate::worldgen::{World, COLS, ROWS};
use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::math::UVec3;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// A tile's terrain kind: `Some("water")`, a ground-type name, or `None` (walls, paths,
/// bridges — they never round).
type Kind = Option<&'static str>;

const NOOK: [i32; 5] = [5, 3, 2, 1, 1]; // per-row widths of the corner cut (self-symmetric)
const CORN: [(i32, i32); 4] = [(-1, -1), (1, -1), (-1, 1), (1, 1)];
const FOL: u32 = 0x0f3418; // dark foliage bump
const FOLTIP: u32 = 0x2a6634; // its lit crown

/// The dressing as an ordered fill list `(x, y, w, h, rgb)` in room pixels — the exact
/// fillRect stream the JS emits (order matters where fills overlap).
pub fn dressing_rects(grid: &RoomGrid, world: &World, rx: i32, ry: i32) -> Vec<(i32, i32, i32, i32, u32)> {
    let (gx0, gy0) = (rx * COLS, ry * ROWS);
    let t = TILE;
    let mut out = Vec::new();

    // Kind grid, one ground_name per tile (precomputed once, like the JS).
    let mut kinds = [[None as Kind; COLS as usize]; ROWS as usize];
    for r in 0..ROWS {
        for c in 0..COLS {
            kinds[r as usize][c as usize] = match grid.code_at(c, r) {
                '~' => Some("water"),
                '.' => Some(world.ground_name(gx0 + c, gy0 + r)),
                _ => None,
            };
        }
    }
    let wcol = water_color(world.water_style(gx0, gy0)); // one water style per room
    let col_of = |k: Kind| match k {
        Some("water") => wcol,
        Some(name) => ground_base(name),
        None => ground_base(""), // JS: GROUND_BASE[null] || GROUND_BASE.sand
    };

    // --- Pass 1: rounded terrain corners ---
    for row in 0..ROWS {
        for col in 0..COLS {
            let Some(k_t) = kinds[row as usize][col as usize] else { continue };
            let (px, py) = (col * t, row * t);
            for (dx, dy) in CORN {
                if col + dx < 0 || col + dx >= COLS || row + dy < 0 || row + dy >= ROWS {
                    continue; // not across room seams
                }
                let kh = kinds[row as usize][(col + dx) as usize];
                let kv = kinds[(row + dy) as usize][col as usize];
                let kd = kinds[(row + dy) as usize][(col + dx) as usize];
                let Some(kh) = kh else { continue };
                if Some(kh) != kv || Some(kh) != kd || kh == k_t {
                    continue; // need a clean convex corner into ONE other kind
                }
                if kh == "lava" || k_t == "lava" {
                    // Lava NEVER rounds: the crusted overlay owns its edge. A rounded
                    // nook painted the raw molten base tile onto the land — the naked
                    // red corner wedge (Baz).
                    continue;
                }
                let color = col_of(Some(kh));
                for (j, w) in NOOK.into_iter().enumerate() {
                    let rx_ = if dx < 0 { px } else { px + t - w };
                    let ry_ = if dy < 0 { py + j as i32 } else { py + t - 1 - j as i32 };
                    out.push((rx_, ry_, w, 1, color));
                }
            }
        }
    }

    // --- Pass 2: scalloped hedge edges on leafy walls ---
    let leafy = |code: char| code == 'T' || code == 'G';
    let open_n = |c: i32, r: i32| !is_solid(grid.code_at(c, r));
    for row in 0..ROWS {
        for col in 0..COLS {
            if !leafy(grid.code_at(col, row)) {
                continue;
            }
            let (px, py) = (col * t, row * t);
            let (wx, wy) = ((gx0 + col) * t, (gy0 + row) * t);
            if open_n(col, row - 1) {
                let g = col_of(kinds[(row - 1) as usize][col as usize]);
                for x in 0..t {
                    match edge_h(wx + x, 1) {
                        o if o > 0 => {
                            out.push((px + x, py - o, 1, o, FOL));
                            if o > 1 {
                                out.push((px + x, py - o, 1, 1, FOLTIP));
                            }
                        }
                        o if o < 0 => out.push((px + x, py, 1, -o, g)),
                        _ => {}
                    }
                }
            }
            if open_n(col, row + 1) {
                let g = col_of(kinds[(row + 1) as usize][col as usize]);
                for x in 0..t {
                    match edge_h(wx + x, 2) {
                        o if o > 0 => {
                            out.push((px + x, py + t, 1, o, FOL));
                            if o > 1 {
                                out.push((px + x, py + t + o - 1, 1, 1, FOLTIP));
                            }
                        }
                        o if o < 0 => out.push((px + x, py + t + o, 1, -o, g)),
                        _ => {}
                    }
                }
            }
            if open_n(col - 1, row) {
                let g = col_of(kinds[row as usize][(col - 1) as usize]);
                for y in 0..t {
                    match edge_h(wy + y, 3) {
                        o if o > 0 => {
                            out.push((px - o, py + y, o, 1, FOL));
                            if o > 1 {
                                out.push((px - o, py + y, 1, 1, FOLTIP));
                            }
                        }
                        o if o < 0 => out.push((px, py + y, -o, 1, g)),
                        _ => {}
                    }
                }
            }
            if open_n(col + 1, row) {
                let g = col_of(kinds[row as usize][(col + 1) as usize]);
                for y in 0..t {
                    match edge_h(wy + y, 4) {
                        o if o > 0 => {
                            out.push((px + t, py + y, o, 1, FOL));
                            if o > 1 {
                                out.push((px + t + o - 1, py + y, 1, 1, FOLTIP));
                            }
                        }
                        o if o < 0 => out.push((px + t + o, py + y, -o, 1, g)),
                        _ => {}
                    }
                }
            }
        }
    }
    out
}

/// DEVIATION companion to [`dressing_rects`] (Baz): the JS never scalloped hedges
/// into water ('~' is solid, so those edges stayed razor-flat — fine over flat tile
/// water, jarring next to the living shader surface). Emits the same jittered leaf
/// bumps for every leafy edge FACING WATER — bumps only; the ground-notch cuts
/// would need a flat water colour, and the shader surface owns that edge now.
/// Same `edge_h` hash, so bumps stay continuous with the land scallops around
/// corners. Rasterised by [`build_overlay`] on top of the JS-parity stream.
fn water_hedge_rects(grid: &RoomGrid, rx: i32, ry: i32) -> Vec<(i32, i32, i32, i32, u32)> {
    let (gx0, gy0) = (rx * COLS, ry * ROWS);
    let t = TILE;
    let leafy = |code: char| code == 'T' || code == 'G';
    let watery = |c: i32, r: i32| grid.code_at(c, r) == '~';
    let mut out = Vec::new();
    // One bump: 1px-wide strip growing o px from (bx, by) along (dx, dy), lit at
    // the outermost pixel — the same geometry as the four literal arms in pass 2.
    let mut bump = |bx: i32, by: i32, dx: i32, dy: i32, o: i32| {
        let (x0, y0) = (if dx < 0 { bx - o } else { bx }, if dy < 0 { by - o } else { by });
        let (w, h) = if dx != 0 { (o, 1) } else { (1, o) };
        out.push((x0, y0, w, h, FOL));
        if o > 1 {
            let (tx, ty) = (if dx > 0 { bx + o - 1 } else { x0 }, if dy > 0 { by + o - 1 } else { y0 });
            out.push((tx, ty, 1, 1, FOLTIP));
        }
    };
    for row in 0..ROWS {
        for col in 0..COLS {
            if !leafy(grid.code_at(col, row)) {
                continue;
            }
            let (px, py) = (col * t, row * t);
            let (wx, wy) = ((gx0 + col) * t, (gy0 + row) * t);
            for x in 0..t {
                if watery(col, row - 1) {
                    let o = edge_h(wx + x, 1);
                    if o > 0 {
                        bump(px + x, py, 0, -1, o);
                    }
                }
                if watery(col, row + 1) {
                    let o = edge_h(wx + x, 2);
                    if o > 0 {
                        bump(px + x, py + t, 0, 1, o);
                    }
                }
            }
            for y in 0..t {
                if watery(col - 1, row) {
                    let o = edge_h(wy + y, 3);
                    if o > 0 {
                        bump(px, py + y, -1, 0, o);
                    }
                }
                if watery(col + 1, row) {
                    let o = edge_h(wy + y, 4);
                    if o > 0 {
                        bump(px + t, py + y, 1, 0, o);
                    }
                }
            }
        }
    }
    out
}

/// Hash for the scallop jitter — the JS `rnd` (Math.imul chain), bit-exact.
fn rnd(a: i32, b: i32, c: i32) -> u32 {
    let h = (a.wrapping_add(1))
        .wrapping_mul(374761393)
        .wrapping_add((b.wrapping_add(1)).wrapping_mul(668265263))
        .wrapping_add((c.wrapping_add(2)).wrapping_mul(2246822519u32 as i32));
    let h = h as u32;
    let h = ((h ^ (h >> 13)) as i32).wrapping_mul(1274126177) as u32;
    h ^ (h >> 16)
}

/// Signed scallop profile along a world-pixel line: +out (leaf bump, 2-4px rounded arc over
/// a jittered ~7px period) or -1 (the ground notch between bumps). Port of `edgeH`.
fn edge_h(n: i32, salt: i32) -> i32 {
    let j = |kk: i32| (rnd(kk, salt, 5) % 3) as i32;
    let mut k = n.div_euclid(7); // JS Math.floor(n / 7) on possibly-negative world px
    let mut b0 = k * 7 + j(k);
    if n < b0 {
        k -= 1;
        b0 = k * 7 + j(k);
    }
    let b1 = (k + 1) * 7 + j(k + 1);
    let h = 2 + (rnd(k, salt, 9) % 3) as i32;
    let t = ((n - b0) as f64 + 0.5) / (b1 - b0).max(1) as f64;
    let a = (h as f64 * (std::f64::consts::PI * t).sin()).round() as i32;
    if a > 0 { a } else { -1 }
}

/// Rasterise the dressing into one transparent room-sized image (later fills overwrite,
/// exactly like painting the canvas). One sprite per room, above the tile layer.
///
/// DEVIATION from the JS stream (Baz): the shader water paints its own surface and
/// bank edge now, so the flat water-colour fills — corner nooks + hedge notches in
/// the room's wcol — are dropped HERE at raster time. `dressing_rects` itself stays
/// bit-exact against the JS golden (tests/dressing_parity.rs).
pub fn build_overlay(grid: &RoomGrid, world: &World, rx: i32, ry: i32) -> Image {
    let mut img = Image::new_fill(
        Extent3d { width: PX_W as u32, height: PX_H as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let wcol = water_color(world.water_style(rx * COLS, ry * ROWS));
    let rects = dressing_rects(grid, world, rx, ry)
        .into_iter()
        // Water-colour fills: the shader surface owns that edge now.
        .filter(|&(_, _, _, _, rgb)| rgb != wcol)
        // ...and hedges get their leaf bumps over water too.
        .chain(water_hedge_rects(grid, rx, ry));
    for (x, y, w, h, rgb) in rects {
        for yy in y.max(0)..(y + h).min(PX_H) {
            for xx in x.max(0)..(x + w).min(PX_W) {
                if let Ok(px) = img.pixel_bytes_mut(UVec3::new(xx as u32, yy as u32, 0)) {
                    px.copy_from_slice(&[(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255]);
                }
            }
        }
    }
    img
}
