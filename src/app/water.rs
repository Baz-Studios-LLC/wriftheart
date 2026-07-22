//! water.rs — the LIQUIDS pass: the WATER mask + living-surface overlay
//! (PORT-ORIGINAL, 2026-07-16), generalized for LAVA (task #48 — "we could use
//! the water system and add to it since the hard shader work is mostly done",
//! Baz). One shader, two liquids: water reads depth as darker deeps; lava reads
//! it as the molten heart brightening away from the crusted edge. Lava churns at
//! a third of water's pace, ignores the rain, BURNS the hero who wades it (a
//! sizzle every 24 frames — sprintable, never free), and feeds the lighting
//! pass so ember fields glow in the dark.
//!
//! On every room stand-up this bakes a PIXEL-RES (304x208) mask from the tile grid:
//! r = water coverage — the '~'/'B' tiles PLUS the corner nooks that round water
//! into land (the same [5,3,2,1,1] bite the edge dressing cuts, so every coast
//! corner curves, both ways); a = shore depth, bilinear-smoothed across tile
//! centers here so the shader reads rounded depth contours straight off the
//! texture. water.wgsl drifts glints + tints the deeps over it, and
//! reflection.wgsl clips actor mirrors to it. Each wet room's root carries its
//! OWN overlay quad as a child, so water rides edge slides with the tiles and
//! despawns with the room; dry rooms (and interiors) simply get none.

use super::play::{ActiveRoot, CurGrid, SlideState};
use crate::gfx::water_material::{WaterMaterial, WaterParams};
use crate::gfx::{at, layers, PIXEL_LAYER};
use crate::room::{COLS, PX_H, PX_W, ROWS, TILE};
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
#[derive(Component)]
struct LavaOverlay;

/// One lava bubble: swells from a fleck, domes, and pops. `t` steps the frames.
#[derive(Component)]
struct LavaBubble {
    t: i32,
}

/// Whether the CURRENT room has any lava (the bubble roller's cheap gate).
#[derive(Resource, Default)]
pub struct LavaAny(pub bool);

/// The bubble's four moments, baked once: fleck, swell, dome, burst.
const BUBBLE_FRAMES: [&[&str]; 4] = [
    &["......", "......", "..y...", "......", "......", "......"],
    &["......", "..yy..", "..oo..", "......", "......", "......"],
    &["......", ".yooy.", ".o..o.", ".yooy.", "......", "......"],
    &["y....y", ".y..y.", "......", ".y..y.", "y....y", "......"],
];
const BUBBLE_PAL: &[(char, u32)] = &[('y', 0xffd140), ('o', 0xff8a2a)];

pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaterMask>()
            .init_resource::<LavaAny>()
            .add_systems(Update, (rebake_mask, tick_water).chain())
            .add_systems(
                bevy::app::FixedUpdate,
                (lava_burn, lava_bubbles).run_if(super::screen::playing),
            );
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
    mut lava_any: ResMut<LavaAny>,
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

    // Coverage at pixel res: whole water tiles, then the water-into-land corner
    // nooks — a ground tile whose convex corner meets water on both sides and the
    // diagonal takes the dressing's bite of lake (the dressing itself now only
    // paints the LAND-coloured half of the rounding; see edge_dressing.rs).
    const NOOK: [i32; 5] = [5, 3, 2, 1, 1];
    let mut cover = vec![false; (PX_W * PX_H) as usize];
    for r in 0..ROWS {
        for c in 0..COLS {
            if !is_water(c, r) {
                continue;
            }
            for py in r * TILE..(r + 1) * TILE {
                for px in c * TILE..(c + 1) * TILE {
                    cover[(py * PX_W + px) as usize] = true;
                }
            }
        }
    }
    // The bite test wants OPEN water only ('~', in-room): a 'B' neighbour is a
    // bridge DECK — rounding the bank at a bridge mouth pinched the walkway (Baz).
    let open_water = |c: i32, r: i32| (0..COLS).contains(&c) && (0..ROWS).contains(&r) && grid.0.code_at(c, r) == '~';
    for r in 0..ROWS {
        for c in 0..COLS {
            if grid.0.code_at(c, r) != '.' {
                continue; // only open ground rounds — walls stay square, like the dressing
            }
            for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                if !(open_water(c + dx, r) && open_water(c, r + dy) && open_water(c + dx, r + dy)) {
                    continue;
                }
                for (j, w) in NOOK.into_iter().enumerate() {
                    let nx = if dx < 0 { c * TILE } else { c * TILE + TILE - w };
                    let ny = if dy < 0 { r * TILE + j as i32 } else { r * TILE + TILE - 1 - j as i32 };
                    for x in nx..nx + w {
                        cover[(ny * PX_W + x) as usize] = true;
                    }
                }
            }
        }
    }

    // Depth per pixel: bilinear between tile-center depths (land = 0, off-room
    // clamps — the exact smoothing the shader used to do with four taps).
    let dep = |cc: i32, rr: i32| -> f32 {
        let (cc, rr) = (cc.clamp(0, COLS - 1), rr.clamp(0, ROWS - 1));
        if is_water(cc, rr) { depth[rr as usize][cc as usize] as f32 / 3.0 } else { 0.0 }
    };
    let mut buf = vec![0u8; (PX_W * PX_H * 4) as usize];
    for y in 0..PX_H {
        for x in 0..PX_W {
            let i = (y * PX_W + x) as usize;
            if !cover[i] {
                continue;
            }
            let fx = (x as f32 + 0.5) / TILE as f32 - 0.5;
            let fy = (y as f32 + 0.5) / TILE as f32 - 0.5;
            let (c0, r0) = (fx.floor() as i32, fy.floor() as i32);
            let (fu, fv) = (fx - c0 as f32, fy - r0 as f32);
            let d = dep(c0, r0) * (1.0 - fu) * (1.0 - fv)
                + dep(c0 + 1, r0) * fu * (1.0 - fv)
                + dep(c0, r0 + 1) * (1.0 - fu) * fv
                + dep(c0 + 1, r0 + 1) * fu * fv;
            buf[i * 4] = 255;
            buf[i * 4 + 3] = (d * 255.0).round() as u8;
        }
    }
    let img = Image::new(
        Extent3d { width: PX_W as u32, height: PX_H as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        buf,
        TextureFormat::Rgba8Unorm, // data, not colour — no srgb curve on the mask
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
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

    // ---- THE LAVA PASS: alt-ground "lava" tiles get their own living surface. ----
    let (gx0, gy0) = (cur.rx * COLS, cur.ry * ROWS);
    let is_lava = |c: i32, r: i32| {
        if !(0..COLS).contains(&c) || !(0..ROWS).contains(&r) {
            return true; // off-room continues the field — border lava stays molten
        }
        grid.0.code_at(c, r) == '.' && world.0.ground_name(gx0 + c, gy0 + r) == "lava"
    };
    let mut ldepth = [[0u8; COLS as usize]; ROWS as usize];
    let mut lany = false;
    for r in 0..ROWS {
        for c in 0..COLS {
            if !is_lava(c, r) {
                continue;
            }
            lany = true;
            let mut d = 3u8;
            'probe: for ring in 1i32..=3 {
                for dr in -ring..=ring {
                    for dc in -ring..=ring {
                        if dr.abs().max(dc.abs()) == ring && !is_lava(c + dc, r + dr) {
                            d = (ring - 1) as u8;
                            break 'probe;
                        }
                    }
                }
            }
            ldepth[r as usize][c as usize] = d;
        }
    }
    lava_any.0 = lany;
    if lany {
        let mut cover = vec![false; (PX_W * PX_H) as usize];
        for r in 0..ROWS {
            for c in 0..COLS {
                if !is_lava(c, r) {
                    continue;
                }
                for py in r * TILE..(r + 1) * TILE {
                    for px in c * TILE..(c + 1) * TILE {
                        cover[(py * PX_W + px) as usize] = true;
                    }
                }
            }
        }
        // Lava rounds into the basalt exactly as water rounds into land.
        let open_lava = |c: i32, r: i32| (0..COLS).contains(&c) && (0..ROWS).contains(&r) && is_lava(c, r);
        for r in 0..ROWS {
            for c in 0..COLS {
                if grid.0.code_at(c, r) != '.' || is_lava(c, r) {
                    continue;
                }
                for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    if !(open_lava(c + dx, r) && open_lava(c, r + dy) && open_lava(c + dx, r + dy)) {
                        continue;
                    }
                    for (j, w) in NOOK.into_iter().enumerate() {
                        let nx = if dx < 0 { c * TILE } else { c * TILE + TILE - w };
                        let ny = if dy < 0 { r * TILE + j as i32 } else { r * TILE + TILE - 1 - j as i32 };
                        for x in nx..nx + w {
                            cover[(ny * PX_W + x) as usize] = true;
                        }
                    }
                }
            }
        }
        let dep = |cc: i32, rr: i32| -> f32 {
            let (cc, rr) = (cc.clamp(0, COLS - 1), rr.clamp(0, ROWS - 1));
            if is_lava(cc, rr) { ldepth[rr as usize][cc as usize] as f32 / 3.0 } else { 0.0 }
        };
        let mut buf = vec![0u8; (PX_W * PX_H * 4) as usize];
        for y in 0..PX_H {
            for x in 0..PX_W {
                let i = (y * PX_W + x) as usize;
                if !cover[i] {
                    continue;
                }
                let fx = (x as f32 + 0.5) / TILE as f32 - 0.5;
                let fy = (y as f32 + 0.5) / TILE as f32 - 0.5;
                let (c0, r0) = (fx.floor() as i32, fy.floor() as i32);
                let (fu, fv) = (fx - c0 as f32, fy - r0 as f32);
                let d = dep(c0, r0) * (1.0 - fu) * (1.0 - fv)
                    + dep(c0 + 1, r0) * fu * (1.0 - fv)
                    + dep(c0, r0 + 1) * (1.0 - fu) * fv
                    + dep(c0 + 1, r0 + 1) * fu * fv;
                buf[i * 4] = 255;
                buf[i * 4 + 3] = (d * 255.0).round() as u8;
            }
        }
        let limg = images.add(Image::new(
            Extent3d { width: PX_W as u32, height: PX_H as u32, depth_or_array_layers: 1 },
            TextureDimension::D2,
            buf,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        ));
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
                LavaOverlay,
                Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
                MeshMaterial2d(materials.add(WaterMaterial {
                    mask: limg,
                    params: WaterParams {
                        time: 0.0,
                        strength: 1.0,
                        _p0: 0.0,
                        _p1: 0.0,
                        shallow: LAVA_CRUST,
                        deep: LAVA_MOLTEN,
                        wave: LAVA_GLINT,
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
/// Lava reads the depth channel the other way round: crusted dark at the shore,
/// the molten heart brightening toward the middle, hot glints riding the churn.
const LAVA_CRUST: Vec4 = Vec4::new(0.45, 0.10, 0.03, 0.0); // 0x731a08 ember crust
const LAVA_MOLTEN: Vec4 = Vec4::new(1.0, 0.45, 0.10, 0.0); // 0xff731a molten heart
const LAVA_GLINT: Vec4 = Vec4::new(1.0, 0.82, 0.25, 0.0); // 0xffd140 hot spark

/// Drift every live surface (mid-slide that's two — the outgoing room's and the
/// incoming room's — so the water never freezes or blinks during the scroll).
fn tick_water(
    clock: Res<super::room_render::FrameClock>,
    weather: Res<super::weather::WeatherState>,
    mut materials: ResMut<Assets<WaterMaterial>>,
    overlay: Query<&MeshMaterial2d<WaterMaterial>, With<WaterOverlay>>,
    lava: Query<&MeshMaterial2d<WaterMaterial>, (With<LavaOverlay>, Without<WaterOverlay>)>,
) {
    for mat in &overlay {
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.params.time = clock.0 as f32 / 60.0;
            m.params._p0 = weather.storm(); // rain chops the surface (weather tie-in)
        }
    }
    for mat in &lava {
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.params.time = clock.0 as f32 / 170.0; // molten rock churns, it doesn't ripple
        }
    }
}

/// Wading lava sets you BURNING (Baz: a dot ticks on you, a debuff shows while
/// you stand in it): standing refreshes the burn status — the existing burn DoT
/// ticks 1 HP every 30f and keeps searing ~1.5s AFTER you step off (afterburn),
/// with the flame icon in the HUD buff row the whole while. Crossable at real
/// cost, so an unlucky field never soft-locks a room; springboots hop it clean,
/// god mode shrugs.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn lava_burn(
    mut commands: Commands,
    grid: Res<CurGrid>,
    world: Res<super::play::GameWorld>,
    cur: Res<super::play::CurRoom>,
    god: Res<super::dev::GodMode>,
    mut statuses: ResMut<super::status::Statuses>,
    mut rng: ResMut<super::battle::GameRng>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&super::play::Player>,
    mut sizzle: Local<i32>,
) {
    let Ok(p) = players.single() else { return };
    if god.0 || p.hop.is_some() {
        return;
    }
    let (c, r) = (((p.x + 8.0) / TILE as f32).floor() as i32, ((p.y + 12.0) / TILE as f32).floor() as i32);
    if !(0..COLS).contains(&c) || !(0..ROWS).contains(&r) || grid.0.code_at(c, r) != '.' {
        return;
    }
    if world.0.ground_name(cur.rx * COLS + c, cur.ry * ROWS + r) != "lava" {
        return;
    }
    statuses.add("burn", 90); // refreshed every frame you stand in it; the DoT does the biting
    *sizzle += 1;
    if *sizzle >= 20 {
        *sizzle = 0;
        sfx.write(super::sfx::Sfx("hurt"));
        super::battle::spawn_burst(&mut commands, &mut rng, Vec2::new(p.x + 8.0, p.y + 14.0), 0xff7a20, 6);
    }
}

/// The molten heart BUBBLES (Baz): now and then a fleck swells, domes, and pops
/// somewhere deep in the field. Sprite-side — four baked frames over the overlay.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn lava_bubbles(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    lava_any: Res<LavaAny>,
    grid: Res<CurGrid>,
    world: Res<super::play::GameWorld>,
    cur: Res<super::play::CurRoom>,
    mut rng: ResMut<super::battle::GameRng>,
    mut frames: Local<Vec<Handle<Image>>>,
    mut live: Query<(Entity, &mut LavaBubble, &mut Sprite)>,
) {
    // Advance every live bubble: a frame every 6 ticks, gone after the burst.
    for (e, mut b, mut spr) in &mut live {
        b.t += 1;
        let f = (b.t / 6) as usize;
        if f >= BUBBLE_FRAMES.len() {
            commands.entity(e).despawn();
            continue;
        }
        spr.image = frames[f].clone();
    }
    if !lava_any.0 {
        return;
    }
    if frames.is_empty() {
        *frames = BUBBLE_FRAMES.iter().map(|g| images.add(crate::gfx::bake(g, BUBBLE_PAL))).collect();
    }
    // Roll a pop: ~10% of ticks try three random tiles; the first HEARTED lava
    // tile (all four neighbours molten too) births a bubble with sub-tile jitter.
    if rng.0.next_f64() > 0.10 {
        return;
    }
    let (gx0, gy0) = (cur.rx * COLS, cur.ry * ROWS);
    let lava = |c: i32, r: i32| {
        (0..COLS).contains(&c)
            && (0..ROWS).contains(&r)
            && grid.0.code_at(c, r) == '.'
            && world.0.ground_name(gx0 + c, gy0 + r) == "lava"
    };
    for _ in 0..3 {
        let c = (rng.0.next_f64() * COLS as f64) as i32;
        let r = (rng.0.next_f64() * ROWS as f64) as i32;
        if !(lava(c, r) && lava(c - 1, r) && lava(c + 1, r) && lava(c, r - 1) && lava(c, r + 1)) {
            continue;
        }
        let x = (c * TILE) as f32 + 2.0 + (rng.0.next_f64() * 8.0) as f32;
        let y = (r * TILE) as f32 + 2.0 + (rng.0.next_f64() * 8.0) as f32;
        commands.spawn((
            LavaBubble { t: 0 },
            Sprite::from_image(frames[0].clone()),
            at(super::room_render::PLAY_X + x, super::room_render::PLAY_Y + y, 6.0, 6.0, layers::WATER_OVERLAY + 0.02),
            PIXEL_LAYER,
            super::battle::RoomActor,
        ));
        break;
    }
}
