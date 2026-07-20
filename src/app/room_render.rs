//! room_render.rs — spawn a room's tile sprites (REAL tileset art) under one root entity.
//!
//! The root is the slide-transition handle: a room's tiles are children with fixed local
//! transforms, so scrolling a whole room = animating one root translation. Water tiles (and
//! the water under bridges) carry [`WaterAnim`] and swap frames on the shared clock, exactly
//! like the JS (phase = clock/36, 4-frame loop).

use super::gather::{GatherState, TreeGrowth};
use super::room_props::spawn_room_props;
use crate::actors::props::PropArt;
use crate::gfx::{at, edge_dressing, TileTextures, PIXEL_LAYER};
use crate::room::{RoomGrid, PX_H, PX_W, TILE};
use crate::worldgen::{RoomEntity, World, COLS, ROWS};
use crate::SIDEBAR_W;
use bevy::prelude::*;

pub const PLAY_X: f32 = SIDEBAR_W; // the room draws right of the sidebar (js PLAY_X)
pub const PLAY_Y: f32 = 0.0; // the 208-high canvas fits the room exactly (was js 4)

/// Depth-band z for anything that sorts by its feet (actors, trees, thrown stones): the
/// painter's y-sort of the JS entity pass, as a z in [4, 8] over the room's height. Lower
/// feet draw over higher feet — walk behind a tree and its canopy covers you.
pub fn actor_z(foot_y: f32) -> f32 {
    4.0 + (foot_y / PX_H as f32).clamp(0.0, 1.0) * 4.0
}

/// Marker for a room's root entity (all its tiles are children).
#[derive(Component)]
pub struct RoomRoot;

/// An animated water sprite (standalone water or the water under a bridge deck).
#[derive(Component)]
pub struct WaterAnim {
    pub murk: bool,
}

/// The shared frame clock (port of `Entities.clock()` — drives water + future FX timing).
#[derive(Resource, Default)]
pub struct FrameClock(pub i64);

/// Spawn one room's tiles under a fresh root at `offset` (Bevy-space translation; ZERO for
/// the active room, one screen over for an incoming transition room). Returns the root.
#[allow(clippy::too_many_arguments)] // (world, grid, coords, offset, clock) — a RoomCtx struct can group these once more callers exist
pub fn spawn_room_root(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    tex: &TileTextures,
    prop_art: &mut PropArt,
    world: &World,
    grid: &RoomGrid,
    ents: &[RoomEntity],
    gather: &GatherState,
    growth: &mut TreeGrowth,
    farm: &crate::app::farm::FarmTiles,
    cleared: &crate::app::encounters::ClearedEncounters,
    caves: &crate::app::caves::CrackCaves,
    songs_opened: &crate::app::caves::OpenedSongstones,
    rx: i32,
    ry: i32,
    offset: Vec2,
    clock: i64,
) -> (Entity, Vec<(f32, f32, f32, f32)>) {
    let gx0 = rx * COLS;
    let gy0 = ry * ROWS;
    let phase = clock / 36;
    let root = commands
        .spawn((Transform::from_xyz(offset.x, offset.y, 0.0), Visibility::default(), RoomRoot))
        .id();
    for row in 0..ROWS {
        for col in 0..COLS {
            let code = grid.code_at(col, row);
            let tf = at(
                PLAY_X + (col * TILE) as f32,
                PLAY_Y + (row * TILE) as f32,
                TILE as f32,
                TILE as f32,
                1.0,
            );
            let murk = world.water_style(gx0 + col, gy0 + row) == "murk";
            let style = if murk { "murk" } else { "blue" };
            match code {
                '.' => {
                    let name = world.ground_name(gx0 + col, gy0 + row);
                    child(commands, root, Sprite::from_image(tex.ground(name, gx0 + col, gy0 + row)), tf);
                }
                '~' => {
                    let e = child(commands, root, Sprite::from_image(tex.water(phase, style)), tf);
                    commands.entity(e).insert(WaterAnim { murk });
                }
                'B' => {
                    // Bridge: animated water underneath, deck oriented to walkable neighbours
                    // (port of the room.js draw branch).
                    let e = child(commands, root, Sprite::from_image(tex.water(phase, style)), tf);
                    commands.entity(e).insert(WaterAnim { murk });
                    let land = |c: i32, r: i32| grid.code_at(c, r) != '~';
                    let h = land(col - 1, row) || land(col + 1, row);
                    let v = land(col, row - 1) || land(col, row + 1);
                    let deck_tf = tf.with_translation(tf.translation.with_z(1.5));
                    child(commands, root, Sprite::from_image(tex.bridge(h, v)), deck_tf);
                }
                c => {
                    child(commands, root, Sprite::from_image(tex.code(c)), tf);
                }
            }
        }
    }
    // Edge dressing: rounded shoreline/path corners + scalloped hedges, rasterised into one
    // static overlay above the tiles and bridge decks (the JS painted these every frame).
    let overlay = images.add(edge_dressing::build_overlay(grid, world, rx, ry));
    let tf = at(PLAY_X, PLAY_Y, PX_W as f32, PX_H as f32, 2.0);
    child(commands, root, Sprite::from_image(overlay), tf);
    // Props ride the root too (they scroll with the room during a slide).
    let blockers = spawn_room_props(
        commands, images, prop_art, world, grid, ents, root, gather, growth, farm, cleared, caves, songs_opened, clock, (rx, ry),
    );
    (root, blockers)
}

/// Spawn one sprite as a child of the room root.
pub fn child(commands: &mut Commands, root: Entity, sprite: Sprite, tf: Transform) -> Entity {
    let e = commands.spawn((sprite, tf, PIXEL_LAYER)).id();
    commands.entity(root).add_child(e);
    e
}

/// Swap water frames when the shared clock crosses a phase boundary.
pub fn animate_water(
    clock: Res<FrameClock>,
    tex: Res<TileTextures>,
    mut last: Local<i64>,
    mut q: Query<(&WaterAnim, &mut Sprite)>,
) {
    let phase = clock.0 / 36;
    if phase == *last {
        return;
    }
    *last = phase;
    for (w, mut sprite) in &mut q {
        sprite.image = tex.water(phase, if w.murk { "murk" } else { "blue" });
    }
}
