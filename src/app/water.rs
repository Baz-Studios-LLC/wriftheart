//! water.rs — the WATER MASK + the living-surface overlay (PORT-ORIGINAL, part of the
//! 2026-07-16 water pass; "we moved to rust for a reason").
//!
//! On every room stand-up this bakes a 19x13 mask from the tile grid: r = water
//! ('~' and under-bridge 'B'), a = shore distance (tile BFS, clamped 3 deep) — the
//! water.wgsl overlay drifts glints + tints the deeps, and reflection.wgsl clips
//! actor mirrors to it. Each wet room's root carries its OWN overlay quad as a
//! child, so water rides edge slides with the tiles and despawns with the room;
//! dry rooms (and interiors) simply get none.

use super::play::{ActiveRoot, CurGrid, SlideState};
use crate::gfx::water_material::{WaterMaterial, WaterParams};
use crate::gfx::{at, layers, PIXEL_LAYER};
use crate::room::{COLS, PX_H, PX_W, ROWS};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite_render::MeshMaterial2d;

/// The current room's water mask (reflections sample it too) + whether any water
/// exists (dry rooms skip the whole pass).
#[derive(Resource, Default)]
pub struct WaterMask {
    pub image: Handle<Image>,
    pub any: bool,
}

#[derive(Component)]
struct WaterOverlay;

pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaterMask>()
            .add_systems(Update, (rebake_mask, tick_water).chain());
    }
}

/// Bake the room's mask when its root changes (the critters' reactive pattern).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn rebake_mask(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<WaterMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    root: Res<ActiveRoot>,
    slide: Res<SlideState>,
    grid: Res<CurGrid>,
    world: Res<super::play::GameWorld>,
    cur: Res<super::play::CurRoom>,
    mut mask: ResMut<WaterMask>,
    mut last_root: Local<Option<Entity>>,
) {
    // Mid-slide the grid already describes the INCOMING room but ActiveRoot still
    // points at the outgoing one — bake for (and parent to) the incoming root so
    // its water scrolls in WITH it instead of popping in at the settle.
    let target = slide.incoming_root().unwrap_or(root.0);
    if *last_root == Some(target) {
        return;
    }
    *last_root = Some(target);

    // Tile pass: water flags, then a BFS for shore distance (depth shading).
    let is_water = |c: i32, r: i32| {
        if !(0..COLS).contains(&c) || !(0..ROWS).contains(&r) {
            return true; // off-room continues the lake — border water stays deep
        }
        matches!(grid.0.code_at(c, r), '~' | 'B')
    };
    let mut depth = [[0u8; COLS as usize]; ROWS as usize];
    let mut any = false;
    for r in 0..ROWS {
        for c in 0..COLS {
            if !is_water(c, r) {
                continue;
            }
            any = true;
            // Distance to the nearest land, probed outward (rooms are tiny — brute force).
            let mut d = 3u8;
            'probe: for ring in 1i32..=3 {
                for dr in -ring..=ring {
                    for dc in -ring..=ring {
                        if dr.abs().max(dc.abs()) == ring && !is_water(c + dc, r + dr) {
                            d = (ring - 1) as u8;
                            break 'probe;
                        }
                    }
                }
            }
            depth[r as usize][c as usize] = d;
        }
    }

    let mut img = Image::new_fill(
        Extent3d { width: COLS as u32, height: ROWS as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8Unorm, // data, not colour — no srgb curve on the mask
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    for r in 0..ROWS as u32 {
        for c in 0..COLS as u32 {
            if is_water(c as i32, r as i32)
                && let Ok(px) = img.pixel_bytes_mut(UVec3::new(c, r, 0))
            {
                let d = depth[r as usize][c as usize];
                px.copy_from_slice(&[255, 0, 0, d * 85]); // a = depth/3
            }
        }
    }
    mask.image = images.add(img);
    mask.any = any;

    // A fresh quad PARENTED to this room's root: it rides edge slides alongside
    // the tiles (both rooms keep their water mid-scroll) and despawns with the
    // room. Palette is per-room, so it bakes here once — tick only drives time.
    if any {
        let murk = world.0.water_style(cur.rx * COLS, cur.ry * ROWS) == "murk";
        let (shallow, deep, wave) = if murk {
            (MURK_SHALLOW, MURK_DEEP, MURK_WAVE)
        } else {
            (BLUE_SHALLOW, BLUE_DEEP, BLUE_WAVE)
        };
        let mut t = at(
            super::room_render::PLAY_X,
            super::room_render::PLAY_Y,
            PX_W as f32,
            PX_H as f32,
            layers::WATER_OVERLAY,
        );
        t.scale = Vec3::new(PX_W as f32, PX_H as f32, 1.0);
        let quad = commands
            .spawn((
                WaterOverlay,
                Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
                MeshMaterial2d(materials.add(WaterMaterial {
                    mask: mask.image.clone(),
                    params: WaterParams {
                        time: 0.0,
                        strength: 1.0,
                        _p0: 0.0,
                        _p1: 0.0,
                        shallow,
                        deep,
                        wave,
                    },
                })),
                t,
                PIXEL_LAYER,
            ))
            .id();
        commands.entity(target).add_child(quad);
    }
}

/// The two style palettes, anchored to the game's own water chars ('w' 3cbcfc
/// light / 'V' 0070ec dark; the murk greens) — rgb in xyz, w unused.
const BLUE_SHALLOW: Vec4 = Vec4::new(0.07, 0.40, 0.82, 0.0); // the 'V' 0070ec family
const BLUE_DEEP: Vec4 = Vec4::new(0.01, 0.20, 0.55, 0.0); // rich navy body
const BLUE_WAVE: Vec4 = Vec4::new(0.24, 0.62, 0.95, 0.0); // toward 'w', restrained
const MURK_SHALLOW: Vec4 = Vec4::new(0.19, 0.42, 0.36, 0.0); // u-family
const MURK_DEEP: Vec4 = Vec4::new(0.08, 0.22, 0.19, 0.0);
const MURK_WAVE: Vec4 = Vec4::new(0.31, 0.62, 0.55, 0.0); // u

/// Drift every live surface (mid-slide that's two — the outgoing room's and the
/// incoming room's — so the water never freezes or blinks during the scroll).
fn tick_water(
    clock: Res<super::room_render::FrameClock>,
    weather: Res<super::weather::WeatherState>,
    mut materials: ResMut<Assets<WaterMaterial>>,
    overlay: Query<&MeshMaterial2d<WaterMaterial>, With<WaterOverlay>>,
) {
    for mat in &overlay {
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.params.time = clock.0 as f32 / 60.0;
            m.params._p0 = weather.storm(); // rain chops the surface (weather tie-in)
        }
    }
}
