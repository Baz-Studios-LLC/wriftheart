//! water.rs — the WATER MASK + the living-surface overlay (PORT-ORIGINAL, part of the
//! 2026-07-16 water pass; "we moved to rust for a reason").
//!
//! On every room stand-up this bakes a 19x13 mask from the tile grid: r = water
//! ('~' and under-bridge 'B'), a = shore distance (tile BFS, clamped 3 deep) — the
//! water.wgsl overlay drifts glints + tints the deeps, and reflection.wgsl clips
//! actor mirrors to it. One overlay quad persists; rooms swap its mask, dry rooms
//! (and interiors) hide it.

use super::play::{ActiveRoot, CurGrid};
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
    grid: Res<CurGrid>,
    mut mask: ResMut<WaterMask>,
    mut overlay: Query<(&MeshMaterial2d<WaterMaterial>, &mut Visibility), With<WaterOverlay>>,
    mut last_root: Local<Option<Entity>>,
) {
    if *last_root == Some(root.0) {
        return;
    }
    *last_root = Some(root.0);

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

    // The one overlay quad: spawn on first need, then swap its mask + visibility.
    if let Ok((mat, mut vis)) = overlay.single_mut() {
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.mask = mask.image.clone();
        }
        *vis = if any { Visibility::Inherited } else { Visibility::Hidden };
    } else if any {
        let mut t = at(
            super::room_render::PLAY_X,
            super::room_render::PLAY_Y,
            PX_W as f32,
            PX_H as f32,
            layers::WATER_OVERLAY,
        );
        t.scale = Vec3::new(PX_W as f32, PX_H as f32, 1.0);
        commands.spawn((
            WaterOverlay,
            Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
            MeshMaterial2d(materials.add(WaterMaterial {
                mask: mask.image.clone(),
                params: WaterParams { time: 0.0, strength: 1.0, _p0: 0.0, _p1: 0.0 },
            })),
            t,
            PIXEL_LAYER,
        ));
    }
}

/// Drift the surface (the shared frame clock, in seconds). The overlay is ABSOLUTE
/// while rooms scroll during an edge slide — hide it for the ride.
fn tick_water(
    clock: Res<super::room_render::FrameClock>,
    sliding: Res<super::play::SlideActive>,
    mask: Res<WaterMask>,
    weather: Res<super::weather::WeatherState>,
    mut materials: ResMut<Assets<WaterMaterial>>,
    mut overlay: Query<(&MeshMaterial2d<WaterMaterial>, &mut Visibility), With<WaterOverlay>>,
) {
    for (mat, mut vis) in &mut overlay {
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.params.time = clock.0 as f32 / 60.0;
            m.params._p0 = weather.storm(); // rain chops the surface (weather tie-in)
        }
        *vis = if sliding.0 || !mask.any { Visibility::Hidden } else { Visibility::Inherited };
    }
}
