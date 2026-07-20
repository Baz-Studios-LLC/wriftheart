//! flyover.rs — the title backdrop (js titlescreen.js): a slow diagonal drift over REAL
//! procedural terrain. Each visible room is CPU-baked to ONE image (tiles, edge dressing,
//! and the js flyRoom prop set: trees/bushes/boulders) so the whole backdrop lives at a
//! single controlled z ABOVE the frozen play world; reusing the live per-sprite room
//! spawner here would interleave its z-bands with the world underneath.
//!
//! DEVIATION (flagged): the js blurs the backdrop 1.4px via ctx.filter — there's no cheap
//! sprite blur here, so ours drifts sharp under the same darkening gradient. Water is also
//! frozen at frame 0 (the js flyover animates it).
//!
//! Parity note: the tile-code -> texture match below mirrors room_render::spawn_room_root;
//! if a new tile code lands there, add it here too.

use super::TitleBackdrop;
use crate::actors::props::{pick_variant, prop_anchor, PropArt};
use crate::app::play::GameWorld;
use crate::app::room_props::is_big_prop;
use crate::gfx::{at, edge_dressing, TileTextures, PIXEL_LAYER};
use crate::room::{RoomGrid, PX_H, PX_W, TILE};
use crate::worldgen::{COLS, ROWS};
use crate::{CANVAS_H, CANVAS_W};
use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

pub(super) const FLY_Z: f32 = 18.74; // over the play world + HUD (<=18.7), under the title text
const PROP_Z: f32 = 18.75; // the prop layer: EVERY room's props over EVERY room's terrain

/// Prop-layer overhang margin: a tree's canopy rises 56px above its foot tile (and
/// spills 16px to each side; its shadow dips ~26px below a bottom-row foot), so props
/// near a room edge need room to draw PAST it — in play the sprites just overlap the
/// neighbour room; here each room is its own bake (Baz: "trees are cut off").
const MARGIN: i32 = 56;

/// One drifting room's live pieces: [terrain, props] sprites + their baked images.
type RoomLayers = ([Entity; 2], [Handle<Image>; 2]);

/// The drift state + live room sprites (two baked images each: terrain, props).
#[derive(Resource, Default)]
pub struct Flyover {
    t: f64,
    rooms: HashMap<(i32, i32), RoomLayers>,
}

impl Flyover {
    /// Whether the backdrop is already up (re-entering Title from OPTIONS keeps it).
    pub fn active(&self) -> bool {
        !self.rooms.is_empty()
    }
    /// Free every baked room image (leaving the title for play). The sprites themselves
    /// carry TitleBackdrop and despawn with the marker sweep — not here, or they'd
    /// double-despawn.
    pub fn clear_images(&mut self, images: &mut Assets<Image>) {
        for (_, (_, handles)) in self.rooms.drain() {
            for h in handles {
                images.remove(&h);
            }
        }
        self.t = 0.0;
    }
}

/// The two per-room sprite transforms (terrain at the room rect, props margin-expanded).
fn room_transforms(sx: f32, sy: f32) -> [Transform; 2] {
    let (pxw, pxh, m) = (PX_W as f32, PX_H as f32, MARGIN as f32);
    [
        at(sx, sy, pxw, pxh, FLY_Z),
        at(sx - m, sy - m, pxw + 2.0 * m, pxh + 2.0 * m, PROP_Z),
    ]
}

/// Drift the camera and keep the visible room set baked + positioned (js Title.draw's
/// backdrop pass, as retained sprites instead of an immediate-mode repaint).
pub fn flyover_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fly: ResMut<Flyover>,
    world: Res<GameWorld>,
    tex: Res<TileTextures>,
    mut prop_art: ResMut<PropArt>,
    mut sprites: Query<&mut Transform, With<TitleBackdrop>>,
) {
    fly.t += 1.0;
    let (w, h) = (CANVAS_W as f32, CANVAS_H as f32);
    let cwx = (fly.t * 0.35).floor() as f32; // the js drift speeds
    let cwy = (fly.t * 0.22).floor() as f32;
    let (pxw, pxh) = (PX_W as f32, PX_H as f32);
    let min_rx = (cwx / pxw).floor() as i32;
    let max_rx = ((cwx + w) / pxw).floor() as i32;
    let min_ry = (cwy / pxh).floor() as i32;
    let max_ry = ((cwy + h) / pxh).floor() as i32;

    // Rooms drifting out of view free their baked images (the drift never revisits).
    let visible = |rx: i32, ry: i32| (min_rx..=max_rx).contains(&rx) && (min_ry..=max_ry).contains(&ry);
    let gone: Vec<(i32, i32)> = fly.rooms.keys().copied().filter(|(rx, ry)| !visible(*rx, *ry)).collect();
    for k in gone {
        if let Some((ents, handles)) = fly.rooms.remove(&k) {
            for e in ents {
                commands.entity(e).despawn();
            }
            for h in handles {
                images.remove(&h);
            }
        }
    }

    for ry in min_ry..=max_ry {
        for rx in min_rx..=max_rx {
            let (sx, sy) = (rx as f32 * pxw - cwx, ry as f32 * pxh - cwy);
            let tfs = room_transforms(sx, sy);
            match fly.rooms.get(&(rx, ry)) {
                Some((ents, _)) => {
                    for (e, tf) in ents.iter().zip(tfs) {
                        if let Ok(mut t) = sprites.get_mut(*e) {
                            *t = tf;
                        }
                    }
                }
                None => {
                    let baked = bake_room(&world.0, &tex, &mut prop_art, &mut images, rx, ry);
                    let handles = baked.map(|img| images.add(img));
                    let ents = [0, 1].map(|i| {
                        commands
                            .spawn((Sprite::from_image(handles[i].clone()), tfs[i], PIXEL_LAYER, TitleBackdrop))
                            .id()
                    });
                    fly.rooms.insert((rx, ry), (ents, handles));
                }
            }
        }
    }
}

/// Composite one room to TWO RGBA images: [terrain (tiles + edge dressing, the room
/// rect), props (shadows + the js flyRoom prop set, margin-expanded so edge canopies
/// overhang the seam)]. Painter-sorted by foot row within the room; across rooms the
/// PROP_Z layer split keeps every canopy over every floor.
fn bake_room(
    world: &crate::worldgen::World,
    tex: &TileTextures,
    prop_art: &mut PropArt,
    images: &mut Assets<Image>,
    rx: i32,
    ry: i32,
) -> [Image; 2] {
    let grid = RoomGrid::from_map(&world.generate(rx, ry));
    let (gx0, gy0) = (rx * COLS, ry * ROWS);
    let mut buf = vec![0u8; (PX_W * PX_H * 4) as usize];

    // Terrain (mirrors spawn_room_root's code match; water frozen at phase 0).
    for row in 0..ROWS {
        for col in 0..COLS {
            let (dx, dy) = (col * TILE, row * TILE);
            let code = grid.code_at(col, row);
            let murk = world.water_style(gx0 + col, gy0 + row) == "murk";
            let style = if murk { "murk" } else { "blue" };
            match code {
                '.' => blit(&mut buf, PX_W, PX_H, images, &tex.ground(world.ground_name(gx0 + col, gy0 + row), gx0 + col, gy0 + row), dx, dy),
                '~' => blit(&mut buf, PX_W, PX_H, images, &tex.water(0, style), dx, dy),
                'B' => {
                    blit(&mut buf, PX_W, PX_H, images, &tex.water(0, style), dx, dy);
                    let land = |c: i32, r: i32| grid.code_at(c, r) != '~';
                    let h = land(col - 1, row) || land(col + 1, row);
                    let v = land(col, row - 1) || land(col, row + 1);
                    blit(&mut buf, PX_W, PX_H, images, &tex.bridge(h, v), dx, dy);
                }
                c => blit(&mut buf, PX_W, PX_H, images, &tex.code(c), dx, dy),
            }
        }
    }
    let dressing = edge_dressing::build_overlay(&grid, world, rx, ry);
    blit_image(&mut buf, PX_W, PX_H, &dressing, 0, 0);

    // Props — the js flyover set (bush/boulder/big trees), y-sorted like the play painter.
    // (img, x, y, foot, shadow_top): shadow_top is the play anchor's tucked feet line
    // (tree fy+12, others fy+11, both -2 into the base — shadows.rs person/prop rules).
    let mut props: Vec<(Handle<Image>, i32, i32, i32, i32)> = Vec::new();
    for e in world.room_entities(rx, ry) {
        let (x, y) = (e.x, e.y);
        match e.kind {
            k if is_big_prop(k) => {
                let (ox, oy, ..) = prop_anchor(k);
                props.push((prop_art.tree(k, x, y, images), x + ox, y + oy, y + TILE, y + 10));
            }
            "bush" => props.push((prop_art.bushes[pick_variant(x, y, 0x11, prop_art.bushes.len())].clone(), x, y, y + TILE, y + 9)),
            "boulder" => {
                let pool = &prop_art.boulders[0];
                props.push((pool[pick_variant(x, y, 0x23, pool.len())].clone(), x, y, y + TILE, y + 9));
            }
            _ => {}
        }
    }
    props.sort_by_key(|(_, _, _, foot, _)| *foot);
    // The prop layer (margin-expanded): shadows first, ALL under ALL props (the play
    // z-ladder: every quad below every actor), at the flyover's frozen NOON sun — and
    // clipped off water, like in play.
    let (pw, ph) = (PX_W + 2 * MARGIN, PX_H + 2 * MARGIN);
    let mut pbuf = vec![0u8; (pw * ph * 4) as usize];
    for (img, x, _, _, stop) in &props {
        blit_shadow(&mut pbuf, pw, ph, images, img, &grid, x + MARGIN, stop + MARGIN);
    }
    for (img, x, y, ..) in &props {
        blit(&mut pbuf, pw, ph, images, img, x + MARGIN, y + MARGIN);
    }

    let bake = |w: i32, h: i32, data: Vec<u8>| {
        Image::new(
            Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        )
    };
    [bake(PX_W, PX_H, buf), bake(pw, ph, pbuf)]
}

/// A prop's noon cast shadow, CPU-composited into the room bake (the shadow.wgsl look
/// at the flyover's frozen time): the art's silhouette flipped at the feet, squashed to
/// the noon stretch (0.45), no shear, darkening what's under it by the play alpha
/// (day 0.38 x prop 0.9). Water tiles are skipped — shadows drown there in play too.
#[allow(clippy::too_many_arguments)] // a blit is coordinates all the way down
fn blit_shadow(
    buf: &mut [u8],
    bw: i32,
    bh: i32,
    images: &Assets<Image>,
    handle: &Handle<Image>,
    grid: &RoomGrid,
    dx: i32,
    top: i32,
) {
    let Some(img) = images.get(handle) else { return };
    let Some(data) = img.data.as_ref() else { return };
    let (sw, sh) = (img.width() as i32, img.height() as i32);
    let out_h = ((sh as f32 * 0.45).round() as i32).max(2);
    // The water test lives in ROOM pixels; dest coords are prop-layer (margin) space.
    // Outside the room the neighbour's grid is unknown — treat as land (a rare seam
    // shadow over border water is subtler than a hole in it).
    let wet = |tx: i32, ty: i32| {
        let (rx, ry) = (tx - MARGIN, ty - MARGIN);
        (0..PX_W).contains(&rx)
            && (0..PX_H).contains(&ry)
            && matches!(grid.code_at(rx / TILE, ry / TILE), '~' | 'B')
    };
    for j in 0..out_h {
        let ty = top + j;
        if !(0..bh).contains(&ty) {
            continue;
        }
        // Flip at the feet: output row 0 (the contact line) reads the art's bottom row.
        let sy = (((1.0 - (j as f32 + 0.5) / out_h as f32) * sh as f32) as i32).clamp(0, sh - 1);
        for sx in 0..sw {
            let tx = dx + sx;
            if !(0..bw).contains(&tx) || wet(tx, ty) {
                continue;
            }
            let a = data[((sy * sw + sx) * 4) as usize + 3];
            if a == 0 {
                continue;
            }
            // The GPU blends black in LINEAR space; on these sRGB bytes the same darkening
            // is a (1-k)^(1/2.2) scale (the lighting.rs gamma gotcha, inverted).
            let k = 0.342 * (a as f32 / 255.0);
            let f = (1.0 - k).powf(1.0 / 2.2);
            let di = ((ty * PX_W + tx) * 4) as usize;
            for c in 0..3 {
                buf[di + c] = (buf[di + c] as f32 * f) as u8;
            }
        }
    }
}

/// Alpha-tested copy of a baked asset image into a buffer (clipped to its bounds).
fn blit(buf: &mut [u8], bw: i32, bh: i32, images: &Assets<Image>, handle: &Handle<Image>, dx: i32, dy: i32) {
    if let Some(img) = images.get(handle) {
        blit_image(buf, bw, bh, img, dx, dy);
    }
}

fn blit_image(buf: &mut [u8], bw: i32, bh: i32, img: &Image, dx: i32, dy: i32) {
    let Some(data) = img.data.as_ref() else { return };
    let (sw, sh) = (img.width() as i32, img.height() as i32);
    for sy in 0..sh {
        let ty = dy + sy;
        if !(0..bh).contains(&ty) {
            continue;
        }
        for sx in 0..sw {
            let tx = dx + sx;
            if !(0..bw).contains(&tx) {
                continue;
            }
            let si = ((sy * sw + sx) * 4) as usize;
            if data[si + 3] == 0 {
                continue;
            }
            let di = ((ty * bw + tx) * 4) as usize;
            buf[di..di + 4].copy_from_slice(&data[si..si + 4]);
        }
    }
}
