//! critters.rs — peaceful ambiance for the gentle biomes (js Entities.critter +
//! Entities.firefly + game.js spawnCritters). Purely cosmetic: no team, no health,
//! never saved, never cached — re-rolled on every room entry.
//!
//! By day: 2-4 of rabbit / deer / bird / butterfly (butterflies favour the flowery
//! biomes). Startled ground critters bolt away and veer along walls; a startled BIRD
//! commits toward the NEAREST TOWN and flies off the map — a subtle, diegetic compass.
//! After dark the meadows belong to the fireflies (5-9 blinking drifters, plus maybe
//! one prowler). Seeded per room + day + night, the js co-op determinism rule.
//!
//! DEVIATIONS (flagged): bird colours roll from the seeded stream (the js uses raw
//! Math.random there — the one unseeded roll in its own determinism comment); the
//! critter-day is the DAWN day (js seeds on the noon dayNumber) — one world clock.

use crate::app::battle::{not_sliding, GameRng, RoomActor};
use crate::app::play::{ActiveRoot, CurGrid, CurRoom, GameWorld, Player};
use crate::app::room_render::{actor_z, FrameClock, PLAY_X, PLAY_Y};
use crate::gfx::PIXEL_LAYER;
use crate::room::{RoomGrid, COLS, PX_H, PX_W, ROWS, TILE};
use crate::worldgen::rng::Mulberry32;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// The kind table (js CRITTER_KINDS): flight, wander speed, flee speed, startle radius.
#[derive(Clone, Copy, PartialEq)]
pub enum CritterKind {
    Rabbit,
    Deer,
    Bird,
    Butterfly,
}

struct KindDef {
    fly: bool,
    spd: f32,
    flee: f32,
    r: f32,
}

const fn def(kind: CritterKind) -> KindDef {
    match kind {
        CritterKind::Rabbit => KindDef { fly: false, spd: 0.55, flee: 2.0, r: 40.0 },
        CritterKind::Deer => KindDef { fly: false, spd: 0.5, flee: 2.4, r: 60.0 },
        CritterKind::Bird => KindDef { fly: true, spd: 0.5, flee: 3.0, r: 46.0 },
        CritterKind::Butterfly => KindDef { fly: true, spd: 0.35, flee: 1.7, r: 26.0 },
    }
}

/// Birds come in a few natural colours so a flock isn't monochrome (js BIRD_COLORS).
const BIRD_COLORS: [(u32, u32); 5] = [
    (0x8aa6d8, 0x4a6a9a), // bluebird
    (0xd86a4a, 0x93412c), // robin
    (0xc9a86a, 0x8a6a3a), // sparrow
    (0xe6d24e, 0xb0992c), // finch
    (0xa6acb6, 0x6a707a), // dove
];

/// The js writes 6.28 where it means a full turn — kept verbatim (NOT std TAU).
#[allow(clippy::approx_constant)]
const JS_TAU: f32 = 6.28;

/// Gentle lands only (js CRITTER_BIOMES).
const CRITTER_BIOMES: [&str; 6] = ["grassland", "forest", "honeyglade", "bluebell", "petalwood", "greenmaw"];

#[derive(Component)]
pub struct Critter {
    kind: CritterKind,
    pub(crate) x: f32,
    pub(crate) y: f32,
    dirx: f32,
    diry: f32,
    move_t: i32,
    hop: f32,
    fleeing: bool,
    committed: bool,
    seed: f32, // wobble phase (js (x*7 + y*13) % 628)
    t: i32,
    aim: Option<Vec2>, // a bird's way home: the nearest town's direction
    frame_bank: usize, // index into CritterArt (kind + bird colour)
}

/// Each kind's NATIVE sprite size (its baked image) — the draw box must match, or
/// `at()` centres the image in the wrong rect and the critter renders offset.
pub(crate) fn sprite_size(kind: CritterKind) -> (f32, f32) {
    match kind {
        CritterKind::Rabbit => (9.0, 10.0),
        CritterKind::Deer => (12.0, 13.0),
        CritterKind::Bird => (8.0, 6.0),
        CritterKind::Butterfly => (7.0, 6.0),
    }
}

impl Critter {
    /// The shadow blob's rect (left, top, w, h, alpha) — sized to the kind, parity-
    /// matched to its image width so the centre is exact; fliers smaller + fainter.
    pub(crate) fn shadow_rect(&self) -> (f32, f32, u32, u32, f32) {
        let (iw, ih) = sprite_size(self.kind);
        let (w, a) = match self.kind {
            CritterKind::Rabbit => (7u32, 1.0),
            CritterKind::Deer => (10, 1.0),
            CritterKind::Bird => (6, 0.55),
            CritterKind::Butterfly => (5, 0.55),
        };
        let left = self.x.round() + ((iw as u32 - w) / 2) as f32;
        let top = self.y.round() + ih - 2.0;
        (left, top, w, 3, a)
    }
}

#[derive(Component)]
pub struct Firefly {
    x: f32,
    y: f32,
    hx: f32,
    hy: f32,
    t: f32,
    blink: f32,
}

/// Baked critter frames: [bank][frame 0/1]. Banks: rabbit, deer, butterfly, then one
/// per bird colour (ground kinds reuse frame 0; fliers flap between the two).
#[derive(Resource)]
pub struct CritterArt(Vec<[Handle<Image>; 2]>);

const BANK_RABBIT: usize = 0;
const BANK_DEER: usize = 1;
const BANK_BUTTERFLY: usize = 2;
const BANK_BIRD0: usize = 3;

pub struct CritterPlugin;

impl Plugin for CritterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
            commands.insert_resource(CritterArt::build(&mut images));
        })
        .add_systems(
            bevy::app::FixedUpdate,
            (
                spawn_on_room_change,
                critter_tick.run_if(not_sliding),
                firefly_tick.run_if(not_sliding),
            )
                .run_if(crate::app::screen::playing),
        )
        .add_systems(Update, sync_critters);
    }
}

/// Paint a rect list into an image (the js drawCritter fillRects, baked once).
fn rects_image(w: u32, h: u32, rects: &[(u32, i32, i32, i32, i32)], images: &mut Assets<Image>) -> Handle<Image> {
    let mut img = Image::new_fill(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    for &(col, x, y, rw, rh) in rects {
        for yy in y.max(0)..(y + rh).min(h as i32) {
            for xx in x.max(0)..(x + rw).min(w as i32) {
                if let Ok(px) = img.pixel_bytes_mut(UVec3::new(xx as u32, yy as u32, 0)) {
                    px.copy_from_slice(&[(col >> 16) as u8, (col >> 8) as u8, col as u8, 255]);
                }
            }
        }
    }
    images.add(img)
}

impl CritterArt {
    pub fn build(images: &mut Assets<Image>) -> Self {
        let mut banks: Vec<[Handle<Image>; 2]> = Vec::new();
        // Rabbit (js drawCritter 'rabbit'), one pose — the hop is a whole-sprite bob.
        let (rc, rd) = (0xc9baa4, 0x8a7458);
        let rabbit = rects_image(
            9,
            10,
            &[
                (rd, 1, 6, 6, 4),
                (rc, 1, 6, 6, 2),
                (rc, 5, 3, 3, 3),
                (rd, 5, 0, 1, 3),
                (rd, 7, 0, 1, 3),
                (0xf4efe6, 0, 8, 2, 2),
                (0x000000, 7, 4, 1, 1),
            ],
            images,
        );
        banks.push([rabbit.clone(), rabbit]);
        // Deer — antlers reach 2px above the js origin, so everything shifts +2.
        let (dc, dd) = (0xb0885a, 0x7a5a34);
        let deer = rects_image(
            12,
            13,
            &[
                (dc, 2, 6, 7, 5),
                (dd, 2, 10, 1, 3),
                (dd, 4, 10, 1, 3),
                (dd, 6, 10, 1, 3),
                (dd, 8, 10, 1, 3),
                (dc, 8, 3, 3, 4),
                (dd, 9, 0, 1, 3),
                (dd, 11, 0, 1, 3),
                (0x000000, 10, 4, 1, 1),
            ],
            images,
        );
        banks.push([deer.clone(), deer]);
        // Butterfly: wings up / folded.
        let bc = 0xf2c040;
        let body = (0x3a2a1a, 3, 2, 1, 4);
        banks.push([
            rects_image(7, 6, &[body, (bc, 0, 1, 3, 3), (bc, 4, 1, 3, 3)], images),
            rects_image(7, 6, &[body, (bc, 1, 2, 2, 2), (bc, 4, 2, 2, 2)], images),
        ]);
        // One bank per bird colour: wings up / down.
        for (bc, bd) in BIRD_COLORS {
            let base = [(bd, 2, 3, 5, 3), (bc, 2, 3, 5, 1), (bd, 6, 2, 2, 2), (0xf0a030, 7, 3, 1, 1)];
            let up: Vec<_> = base.iter().copied().chain([(bc, 0, 1, 3, 2), (bc, 5, 1, 3, 2)]).collect();
            let down: Vec<_> = base.iter().copied().chain([(bc, 0, 4, 3, 1), (bc, 5, 4, 3, 1)]).collect();
            banks.push([rects_image(8, 6, &up, images), rects_image(8, 6, &down, images)]);
        }
        Self(banks)
    }
}

/// A fresh room root means a fresh ambiance roll (js spawnCritters from every room
/// load). Old critters carried RoomActor and left with the room's cast.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn spawn_on_room_change(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    root: Res<ActiveRoot>,
    cur: Res<CurRoom>,
    world: Res<GameWorld>,
    grid: Res<CurGrid>,
    clock: Res<FrameClock>,
    inside: Res<crate::app::interior::Inside>,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    mut last_root: Local<Option<Entity>>,
) {
    if *last_root == Some(root.0) {
        return;
    }
    *last_root = Some(root.0);
    if inside.0.is_some() || in_dungeon.0.is_some() {
        return; // no meadows indoors
    }
    let biome = world.0.biome_key_at(cur.rx, cur.ry);
    if !CRITTER_BIOMES.contains(&biome) {
        return;
    }
    let night = crate::app::lighting::day_darkness(clock.0) > 0.5;
    // Seeded per room + day + night so the ambiance is identical on every visit within
    // the phase (js co-op determinism; the rng IS mulberry32).
    let day = crate::app::gather::farm_day(clock.0);
    let seed = world.0.seed
        ^ ((cur.rx + 1) as u32).wrapping_mul(374761393)
        ^ ((cur.ry + 1) as u32).wrapping_mul(668265263)
        ^ ((day + 1) as u32).wrapping_mul(2246822519)
        ^ if night { 0x9e37 } else { 0 };
    let mut rng = Mulberry32::new(seed);
    let mut rnd = move || rng.next_f64() as f32;

    let clear_spot = |rng: &mut dyn FnMut() -> f32, grid: &RoomGrid, tries: i32| -> Option<(i32, i32)> {
        for _ in 0..tries {
            let c = 2 + (rng() * (COLS - 4) as f32) as i32;
            let r = 2 + (rng() * (ROWS - 4) as f32) as i32;
            if !grid.solid_at((c * TILE + 8) as f32, (r * TILE + 8) as f32) {
                return Some((c, r));
            }
        }
        None
    };

    if night {
        // After dark the meadows belong to the FIREFLIES (plus maybe one prowler).
        let n = 5 + (rnd() * 5.0) as i32;
        for _ in 0..n {
            if let Some((c, r)) = clear_spot(&mut rnd, &grid.0, 10) {
                spawn_firefly(&mut commands, &mut images, (c * TILE + 8) as f32, (r * TILE + 8) as f32);
            }
        }
    }
    let n = if night { if rnd() < 0.5 { 1 } else { 0 } } else { 2 + (rnd() * 3.0) as i32 };
    let flowery = matches!(biome, "honeyglade" | "bluebell" | "petalwood");
    // The compass: startled birds break toward the nearest town.
    let town_dir = crate::worldgen::towns::nearest_town(world.0.seed, cur.rx, cur.ry)
        .filter(|&(tx, ty)| (tx, ty) != (cur.rx, cur.ry))
        .map(|(tx, ty)| Vec2::new((tx - cur.rx) as f32, (ty - cur.ry) as f32).normalize_or_zero());
    for i in 0..n {
        let Some((c, r)) = clear_spot(&mut rnd, &grid.0, 12) else { continue };
        let roll = rnd();
        // Guarantee one bird when a town's in range, so the compass is always readable.
        let kind = if i == 0 && town_dir.is_some() {
            CritterKind::Bird
        } else if flowery && roll < 0.4 {
            CritterKind::Butterfly
        } else if roll < 0.55 {
            CritterKind::Bird
        } else if roll < 0.85 {
            CritterKind::Rabbit
        } else {
            CritterKind::Deer
        };
        let (x, y) = ((c * TILE) as f32, (r * TILE) as f32);
        let frame_bank = match kind {
            CritterKind::Rabbit => BANK_RABBIT,
            CritterKind::Deer => BANK_DEER,
            CritterKind::Butterfly => BANK_BUTTERFLY,
            CritterKind::Bird => BANK_BIRD0 + (rnd() * BIRD_COLORS.len() as f32) as usize % BIRD_COLORS.len(),
        };
        commands.spawn((
            Critter {
                kind,
                x,
                y,
                dirx: 0.0,
                diry: 0.0,
                move_t: 0,
                hop: 0.0,
                fleeing: false,
                committed: false,
                seed: ((x * 7.0 + y * 13.0) as i32 % 628) as f32,
                t: 0,
                aim: if kind == CritterKind::Bird { town_dir } else { None },
                frame_bank,
            },
            Sprite::default(),
            {
                let (iw, ih) = sprite_size(kind);
                crate::gfx::at(PLAY_X + x, PLAY_Y + y, iw, ih, actor_z(y + 8.0))
            },
            PIXEL_LAYER,
            RoomActor,
        ));
    }
}

fn spawn_firefly(commands: &mut Commands, images: &mut Assets<Image>, x: f32, y: f32) {
    // The soft green glow (js radial gradient r5, 'lighter') — alpha-blended here, so
    // halved like every ported additive (PORT.md gotcha). Blink drives sprite alpha.
    let s = 10u32;
    let mut img = Image::new_fill(
        Extent3d { width: s, height: s, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    for yy in 0..s {
        for xx in 0..s {
            let d = (xx as f32 + 0.5 - 5.0).hypot(yy as f32 + 0.5 - 5.0);
            let a = (0.55 * 0.5 * (1.0 - d / 5.0)).max(0.0);
            if a > 0.0
                && let Ok(px) = img.pixel_bytes_mut(UVec3::new(xx, yy, 0))
            {
                px.copy_from_slice(&[210, 255, 140, (a * 255.0) as u8]);
            }
        }
    }
    commands.spawn((
        Firefly { x, y, hx: x, hy: y, t: ((x as i32 * 13 + y as i32 * 7) & 511) as f32, blink: (((x as i32 * 29 + y as i32 * 53) & 1023) as f32 / 1023.0) * JS_TAU },
        Sprite::from_image(images.add(img)),
        crate::gfx::at(PLAY_X + x - 5.0, PLAY_Y + y - 5.0, 10.0, 10.0, crate::gfx::layers::FIREFLY),
        PIXEL_LAYER,
        RoomActor,
    ));
}

/// The js critter update, verbatim: startle, bolt (birds commit toward town), wall
/// veering, flier drift, ground wander-in-bursts.
fn critter_tick(
    mut commands: Commands,
    grid: Res<CurGrid>,
    mut rng: ResMut<GameRng>,
    players: Query<&Player>,
    mut critters: Query<(Entity, &mut Critter)>,
) {
    let Ok(p) = players.single() else { return };
    for (entity, mut e) in &mut critters {
        let d = def(e.kind);
        let (cx, cy) = (e.x + 4.0, e.y + 4.0);
        let (dxp, dyp) = ((p.x + 8.0) - cx, (p.y + 9.0) - cy);
        let pd = dxp.hypot(dyp);
        let solid_at = |nx: f32, ny: f32| {
            nx < 2.0
                || ny < 2.0
                || nx > (PX_W - 8) as f32
                || ny > (PX_H - 8) as f32
                || grid.0.box_hits_solid(nx + 1.0, ny + 5.0, 6.0, 3.0)
        };
        // Per-axis ground moves slide along walls; fliers go straight through.
        macro_rules! mv {
            ($mx:expr, $my:expr) => {{
                let (mx, my) = ($mx, $my);
                if d.fly {
                    e.x += mx;
                    e.y += my;
                } else {
                    let mut moved = false;
                    if mx != 0.0 && !solid_at(e.x + mx, e.y) {
                        e.x += mx;
                        moved = true;
                    }
                    if my != 0.0 && !solid_at(e.x, e.y + my) {
                        e.y += my;
                        moved = true;
                    }
                    if !moved {
                        e.move_t = 0;
                    }
                    e.x = e.x.clamp(2.0, (PX_W - 8) as f32);
                    e.y = e.y.clamp(2.0, (PX_H - 8) as f32);
                }
            }};
        }
        if e.committed {
            // A spooked bird COMMITS: streak toward town + off the map.
            let aim = e.aim.unwrap_or(Vec2::X);
            mv!(aim.x * d.flee, aim.y * d.flee);
            e.hop += 0.4;
            if e.x < -10.0 || e.y < -10.0 || e.x > (PX_W + 10) as f32 || e.y > (PX_H + 10) as f32 {
                commands.entity(entity).despawn();
            }
            continue;
        }
        if pd < d.r {
            // Startled -> bolt.
            e.fleeing = true;
            if d.fly && e.aim.is_some() {
                e.committed = true; // a bird makes for the nearest town and doesn't look back
            } else {
                let m = if pd == 0.0 { 1.0 } else { pd };
                let (mut ax, mut ay) = (-dxp / m, -dyp / m);
                if solid_at(e.x + ax * d.flee, e.y + ay * d.flee) {
                    // Rotate the escape heading until one clears (veer along the wall).
                    for a in [0.7f32, -0.7, 1.4, -1.4, 2.2, -2.2] {
                        let (c, s) = (a.cos(), a.sin());
                        let (nx, ny) = (ax * c - ay * s, ax * s + ay * c);
                        if !solid_at(e.x + nx * d.flee, e.y + ny * d.flee) {
                            ax = nx;
                            ay = ny;
                            break;
                        }
                    }
                }
                e.dirx = ax;
                e.diry = ay;
            }
            mv!(e.dirx * d.flee, e.diry * d.flee);
        } else if d.fly {
            // Fliers never fully settle — gentle drift.
            e.fleeing = false;
            e.t += 1;
            e.x += ((e.t as f32) * 0.05 + e.seed).sin() * 0.5;
            e.y += ((e.t as f32) * 0.037 + e.seed).cos() * 0.4;
        } else {
            // Ground: wander in bursts, pause often.
            e.fleeing = false;
            e.move_t -= 1;
            if e.move_t <= 0 {
                e.move_t = 50 + (rng.0.next_f64() * 80.0) as i32;
                if rng.0.next_f64() < 0.45 {
                    e.dirx = 0.0;
                    e.diry = 0.0;
                } else {
                    let a = rng.0.next_f64() as f32 * JS_TAU;
                    e.dirx = a.cos();
                    e.diry = a.sin();
                }
            }
            mv!(e.dirx * d.spd, e.diry * d.spd);
        }
        e.hop += if d.fly { 0.4 } else if e.dirx != 0.0 || e.diry != 0.0 || e.fleeing { 0.35 } else { 0.0 };
        if d.fly && e.fleeing && (e.x < -10.0 || e.y < -10.0 || e.x > (PX_W + 10) as f32 || e.y > (PX_H + 10) as f32) {
            commands.entity(entity).despawn(); // flew off
        }
    }
}

/// The lazy figure-eight drift + easing away from a wading player (js firefly.update).
/// The blink lands in sync_critters' alpha.
fn firefly_tick(players: Query<&Player>, mut flies: Query<&mut Firefly>) {
    let Ok(p) = players.single() else { return };
    for mut e in &mut flies {
        e.t += 1.0;
        e.hx += (e.t * 0.006 + e.blink).sin() * 0.12;
        e.hy += (e.t * 0.0045 + e.blink).cos() * 0.09;
        e.x = e.hx + (e.t * 0.021 + e.blink).sin() * 14.0 + (e.t * 0.045).sin() * 4.0;
        e.y = e.hy + (e.t * 0.017 + e.blink * 2.0).cos() * 9.0 + (e.t * 0.052).cos() * 3.0;
        let (dx, dy) = (e.x - (p.x + 8.0), e.y - (p.y + 9.0));
        let pd = dx.hypot(dy);
        if pd < 18.0 && pd > 0.01 {
            e.hx += (dx / pd) * 0.9;
            e.hy += (dy / pd) * 0.9;
        }
    }
}

/// Dress positions/frames each render frame (the js draw pass).
fn sync_critters(
    art: Res<CritterArt>,
    mut critters: Query<(&Critter, &mut Sprite, &mut Transform), Without<Firefly>>,
    mut flies: Query<(&Firefly, &mut Sprite, &mut Transform), Without<Critter>>,
) {
    for (e, mut sprite, mut tf) in &mut critters {
        let d = def(e.kind);
        // Ground hop bob: the whole body lifts; fliers flap frames instead.
        let hb = if d.fly { 0.0 } else { -((e.hop.sin().abs() * 2.0).round()) };
        let frame = match e.kind {
            CritterKind::Bird => usize::from(!(e.fleeing || (e.hop * 3.0) as i32 % 2 == 0)),
            CritterKind::Butterfly => usize::from((e.hop * 4.0) as i32 % 2 != 0),
            _ => 0,
        };
        sprite.image = art.0[e.frame_bank][frame].clone();
        // Depth-sort at the body so a critter behind a trunk tucks behind (js baseY).
        let (iw, ih) = sprite_size(e.kind);
        *tf = crate::gfx::at(PLAY_X + e.x.round(), PLAY_Y + e.y.round() + hb, iw, ih, actor_z(e.y + 8.0));
    }
    for (e, mut sprite, mut tf) in &mut flies {
        // Slow blink cycle: dark between flashes (js: skip draw below 0.15).
        let cyc = (e.t * 0.055 + e.blink).sin();
        let a = if cyc < 0.15 { 0.0 } else { ((cyc - 0.15) / 0.5).min(1.0) };
        sprite.color = Color::srgba(1.0, 1.0, 1.0, a);
        *tf = crate::gfx::at(PLAY_X + e.x.round() - 5.0, PLAY_Y + e.y.round() - 5.0, 10.0, 10.0, crate::gfx::layers::FIREFLY);
    }
}
