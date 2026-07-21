//! mobfx.rs — the two hardest roster mobs' bespoke attacks (js enemies.js): the
//! glimmerling's spark BURST + telegraphed light-BEAM, and the witherheart's slow
//! homing DRAIN-ORB (poppable with a swing). Kept out of the projectiles bank because
//! each has its own timing/lifecycle.

use bevy::prelude::*;

use super::battle::RoomActor;
use super::play::Player;
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Blood, Combatant, Health, HitOnce, HurtProfile, Hitbox, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};

// ---------------- glimmerBurst: an expanding SPARK RING, live for a beat ----------------
// The js draws a stroked golden CIRCLE swelling to r16, 11 spark points riding it
// outward, and a small centre flash — the old port scaled one solid square, which
// read as "a square flash, not the aoe ring" (Baz). Now: a baked ring image scaled
// 0->1 + BurstBit spark/flash entities, same frames, same hit window.

#[derive(Component)]
pub struct Burst {
    pub cx: f32,
    pub cy: f32,
    pub t: i32,
}

/// One flung spark (or the centre flash, sp 0) of a burst.
#[derive(Component)]
struct BurstBit {
    cx: f32,
    cy: f32,
    ang: f32,
    sp: f32,
    size: f32,
    t: i32,
}

/// A white circle OUTLINE (2px stroke, r15) — tinted gold per sprite, scaled per frame.
fn ring_image() -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const S: u32 = 34;
    let mut img = Image::new_fill(
        Extent3d { width: S, height: S, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    for y in 0..S {
        for x in 0..S {
            let d = ((x as f32 - 16.5).powi(2) + (y as f32 - 16.5).powi(2)).sqrt();
            let a = (1.6 - (d - 15.0).abs()).clamp(0.0, 1.0);
            if a > 0.0
                && let Ok(px) = img.pixel_bytes_mut(bevy::math::UVec3::new(x, y, 0))
            {
                px.copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
            }
        }
    }
    img
}

pub fn spawn_burst_ring(commands: &mut Commands, x: f32, y: f32) {
    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(2.0)), // ring art lands on tick 1
        at(PLAY_X + x + 7.0, PLAY_Y + y + 7.0, 2.0, 2.0, 9.0),
        PIXEL_LAYER,
        RoomActor,
        Burst { cx: x + 8.0, cy: y + 8.0, t: 0 },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: None, persistent: true, knock: 0.0 },
        HitOnce::default(),
        Hitbox { x: -99.0, y: -99.0, w: 0.0, h: 0.0 },
    ));
}

#[allow(clippy::type_complexity)] // the burst row: combat + sprite state in one pass
fn burst_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut ring: Local<Option<Handle<Image>>>,
    mut q: Query<(Entity, &mut Burst, &mut Hitbox, &mut Combatant, &mut Sprite, &mut Transform), Without<BurstBit>>,
    mut bits: Query<(Entity, &mut BurstBit, &mut Sprite, &mut Transform), Without<Burst>>,
) {
    for (e, mut b, mut hb, mut cb, mut spr, mut tf) in &mut q {
        if b.t == 0 {
            // First tick: install the ring art + fling the 11 sparks and the flash (js parts).
            let h = ring.get_or_insert_with(|| images.add(ring_image())).clone();
            *spr = Sprite::from_image(h);
            let jitter = |i: i32, s: i32| crate::actors::props::px_hash(i, s, (b.cx + b.cy) as i32) as f32 / 1000.0;
            for i in 0..11 {
                let ang = (i as f32 / 11.0) * std::f32::consts::TAU + jitter(i, 1) * 0.4;
                let sp = 0.75 + jitter(i, 2) * 0.6;
                commands.spawn((
                    Sprite::from_color(Color::srgba(1.0, 0.956, 0.706, 1.0), Vec2::splat(1.0)), // #fff4b4
                    at(PLAY_X + b.cx, PLAY_Y + b.cy, 1.0, 1.0, 9.05),
                    PIXEL_LAYER,
                    RoomActor,
                    BurstBit { cx: b.cx, cy: b.cy, ang, sp, size: 1.0, t: 0 },
                ));
            }
            commands.spawn((
                Sprite::from_color(Color::srgba(1.0, 0.933, 0.588, 0.6), Vec2::splat(4.0)), // the centre flash
                at(PLAY_X + b.cx - 2.0, PLAY_Y + b.cy - 2.0, 4.0, 4.0, 9.04),
                PIXEL_LAYER,
                RoomActor,
                BurstBit { cx: b.cx, cy: b.cy, ang: 0.0, sp: 0.0, size: 4.0, t: 0 },
            ));
        }
        b.t += 1;
        // Live during frames 6..=12 (js), radius 16.
        if (6..=12).contains(&b.t) {
            cb.damage = Some(1);
            *hb = Hitbox { x: b.cx - 16.0, y: b.cy - 16.0, w: 32.0, h: 32.0 };
        } else {
            cb.damage = None;
            *hb = Hitbox { x: -99.0, y: -99.0, w: 0.0, h: 0.0 };
        }
        let a = (1.0 - b.t as f32 / 22.0).max(0.0);
        let k = (b.t as f32 / 12.0).min(1.0);
        spr.color = Color::srgba(1.0, 0.878, 0.47, 0.7 * a); // #ffe078 stroke, fading
        let dia = (32.0 * k).max(2.0);
        spr.custom_size = Some(Vec2::splat(dia)); // the ring swells to r16
        *tf = at(PLAY_X + b.cx - dia / 2.0, PLAY_Y + b.cy - dia / 2.0, dia, dia, 9.0);
        if b.t >= 22 {
            commands.entity(e).despawn();
        }
    }
    for (e, mut bit, mut spr, mut tf) in &mut bits {
        bit.t += 1;
        let a = (1.0 - bit.t as f32 / 22.0).max(0.0);
        let k = (bit.t as f32 / 12.0).min(1.0);
        let d = 16.0 * k * bit.sp;
        let (x, y) = (bit.cx + bit.ang.cos() * d, bit.cy + bit.ang.sin() * d);
        spr.color = spr.color.with_alpha(if bit.sp > 0.0 { a } else { 0.6 * a });
        *tf = at(
            (PLAY_X + x - bit.size / 2.0).round(),
            (PLAY_Y + y - bit.size / 2.0).round(),
            bit.size,
            bit.size,
            if bit.sp > 0.0 { 9.05 } else { 9.04 },
        );
        if bit.t >= 22 {
            commands.entity(e).despawn();
        }
    }
}

// ---------------- glimmerBeam: a locked, telegraphed light lance ----------------
#[derive(Component)]
pub struct Beam {
    pub cx: f32,
    pub cy: f32,
    pub ux: f32,
    pub uy: f32,
    pub len: f32,
    pub t: i32,
}

const BEAM_TELE: i32 = 26;
const BEAM_FIRE: i32 = 8;
const BEAM_LIFE: i32 = BEAM_TELE + BEAM_FIRE + 6;

pub fn spawn_beam(commands: &mut Commands, x: f32, y: f32, tx: f32, ty: f32) {
    let (cx, cy) = (x + 8.0, y + 8.0);
    let m = (tx - cx).hypot(ty - cy).max(1.0);
    let (ux, uy) = ((tx - cx) / m, (ty - cy) / m);
    let len = (m + 40.0).min(150.0);
    // The glowing thread — a thin rotated bar the length of the beam.
    let mut spr = Sprite::from_color(Color::srgba(1.0, 0.88, 0.54, 0.25), Vec2::new(len, 1.0));
    spr.custom_size = Some(Vec2::new(len, 1.0));
    let mut tf = at(PLAY_X + cx + ux * len / 2.0, PLAY_Y + cy + uy * len / 2.0, len, 1.0, 8.9);
    tf.rotation = Quat::from_rotation_z(-uy.atan2(ux));
    commands.spawn((
        spr,
        tf,
        PIXEL_LAYER,
        RoomActor,
        Beam { cx, cy, ux, uy, len, t: 0 },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: None, persistent: true, knock: 0.0 },
        HitOnce::default(),
        Hitbox { x: -99.0, y: -99.0, w: 0.0, h: 0.0 },
    ));
}

fn beam_tick(
    mut commands: Commands,
    players: Query<&Player>,
    mut q: Query<(Entity, &mut Beam, &mut Hitbox, &mut Combatant, &mut Sprite)>,
) {
    let p = players.single().ok();
    for (e, mut b, mut hb, mut cb, mut spr) in &mut q {
        b.t += 1;
        cb.damage = None;
        *hb = Hitbox { x: -99.0, y: -99.0, w: 0.0, h: 0.0 };
        if b.t <= BEAM_TELE {
            // The shimmering thread — brightens as it charges (dodge window).
            let a = 0.22 + 0.3 * (b.t as f32 / BEAM_TELE as f32) + 0.12 * (b.t as f32 * 0.8).sin();
            spr.color = Color::srgba(1.0, 0.88, 0.54, a.clamp(0.0, 0.9));
        } else if b.t <= BEAM_TELE + BEAM_FIRE {
            // Live: a bright lance — hurts if the hero is within a hair of the line.
            spr.color = Color::srgba(1.0, 0.96, 0.7, 0.9);
            if let Some(p) = p {
                let (px, py) = (p.x + 8.0, p.y + 9.0);
                let proj = ((px - b.cx) * b.ux + (py - b.cy) * b.uy).clamp(0.0, b.len);
                let (qx, qy) = (b.cx + b.ux * proj, b.cy + b.uy * proj);
                if (px - qx).hypot(py - qy) < 7.0 {
                    cb.damage = Some(1);
                    // The damage box must overlap the hero, but combat.rs derives knockback
                    // from the ATTACKER box centre — placing it ON the hero gave a zero vector
                    // that flung him in a fixed direction ("teleport", Baz). Seat it a few px
                    // BEHIND him along the beam so the shove reads as the lance pushing through.
                    *hb = Hitbox { x: px - b.ux * 6.0 - 6.0, y: py - b.uy * 6.0 - 6.0, w: 12.0, h: 12.0 };
                }
            }
        } else {
            let a = (1.0 - (b.t - BEAM_TELE - BEAM_FIRE) as f32 / 6.0).max(0.0);
            spr.color = spr.color.with_alpha(a * 0.6);
        }
        if b.t >= BEAM_LIFE {
            commands.entity(e).despawn();
        }
    }
}

// ---------------- witherheart drainOrb: a slow homing orb, poppable ----------------
#[derive(Component)]
pub struct DrainOrb {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: i32,
}

pub fn spawn_drain_orb(commands: &mut Commands, x: f32, y: f32, vx: f32, vy: f32) {
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0xc8, 0xd0, 0x60), Vec2::splat(8.0)),
        at(PLAY_X + x + 4.0, PLAY_Y + y + 4.0, 8.0, 8.0, actor_z(y + 8.0)),
        PIXEL_LAYER,
        RoomActor,
        DrainOrb { x, y, vx, vy, life: 300 },
        // Hurts the player on touch AND can be POPPED by a swing (Health 1).
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: true, knock: 0.0 },
        Health { hp: 1, max: 1, defense: 0, invuln: 0, flash: 0 },
        HurtProfile { invuln: 0, flash: 0, kb_base: 0.0, kb_frames: 0 },
        Knockback::default(),
        Blood(0x96a050),
        HitOnce::default(),
        Hitbox { x: x + 5.0, y: y + 5.0, w: 6.0, h: 6.0 },
    ));
}

fn drain_orb_tick(
    mut commands: Commands,
    players: Query<&Player>,
    mut q: Query<(Entity, &mut DrainOrb, &mut Transform, &mut Hitbox, &Health)>,
) {
    let p = players.single().ok();
    for (e, mut o, mut tf, mut hb, h) in &mut q {
        o.life -= 1;
        if o.life <= 0 || h.hp <= 0 {
            commands.entity(e).despawn(); // faded, or popped by a swing
            continue;
        }
        // Turn toward the player, capped at 0.045 rad/tick (js drainOrb homing).
        if let Some(p) = p {
            let (dx, dy) = ((p.x + 8.0) - (o.x + 8.0), (p.y + 9.0) - (o.y + 8.0));
            let want = dy.atan2(dx);
            let cur = o.vy.atan2(if o.vx == 0.0 { 0.001 } else { o.vx });
            let mut dd = want - cur;
            while dd > std::f32::consts::PI {
                dd -= std::f32::consts::TAU;
            }
            while dd < -std::f32::consts::PI {
                dd += std::f32::consts::TAU;
            }
            let a = cur + dd.clamp(-0.045, 0.045);
            let sp = 1.1;
            o.vx = a.cos() * sp;
            o.vy = a.sin() * sp;
        }
        o.x += o.vx;
        o.y += o.vy;
        if o.x < -16.0 || o.x > crate::room::PX_W as f32 || o.y < -16.0 || o.y > crate::room::PX_H as f32 {
            commands.entity(e).despawn();
            continue;
        }
        *hb = Hitbox { x: o.x + 5.0, y: o.y + 5.0, w: 6.0, h: 6.0 };
        *tf = at(PLAY_X + o.x + 4.0, PLAY_Y + o.y + 4.0, 8.0, 8.0, actor_z(o.y + 8.0));
    }
}

// ---------------- frogTongue: the reeling lash's visual (twin of the mimic's) ----------------
// A purely-cosmetic line + tip flung from the maw and reeled back (js frogTongue: 8f extend
// / 3f hold / 9f retract, direction locked at launch). The actual grab/reel rides PullPlayer
// — this just makes the invisible pull legible. Modelled on dungeon.rs's mimic Tongue redraw.
const FT_EXT: i32 = 8;
const FT_HOLD: i32 = 3;
const FT_RET: i32 = 9;

/// State lives on the LINE sprite; it also owns the tip sprite's Entity.
#[derive(Component)]
pub struct FrogTongue {
    ax: f32,
    ay: f32,
    ux: f32,
    uy: f32,
    max_len: f32,
    t: i32,
    tip: Entity,
}

/// Marker on the tip blob — keeps its Transform query disjoint from the line's.
#[derive(Component)]
pub struct FrogTongueFx;

pub fn spawn_frog_tongue(commands: &mut Commands, ax: f32, ay: f32, ux: f32, uy: f32, len: f32) {
    let tip = commands
        .spawn((
            Sprite::from_color(Color::srgb_u8(0xe8, 0x7a, 0x96), Vec2::new(4.0, 4.0)),
            at(PLAY_X + ax - 2.0, PLAY_Y + ay - 2.0, 4.0, 4.0, 9.05),
            PIXEL_LAYER,
            RoomActor,
            FrogTongueFx,
        ))
        .id();
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0xc0, 0x50, 0x6e), Vec2::new(1.0, 2.0)),
        at(PLAY_X + ax, PLAY_Y + ay, 0.0, 0.0, 9.0),
        PIXEL_LAYER,
        RoomActor,
        FrogTongue { ax, ay, ux, uy, max_len: len, t: 0, tip },
    ));
}

fn frog_tongue_tick(
    mut commands: Commands,
    mut line_q: Query<(Entity, &mut FrogTongue, &mut Sprite, &mut Transform), Without<FrogTongueFx>>,
    mut tip_q: Query<&mut Transform, With<FrogTongueFx>>,
) {
    for (e, mut tg, mut ls, mut lt) in &mut line_q {
        tg.t += 1;
        let len = if tg.t <= FT_EXT {
            tg.t as f32 / FT_EXT as f32 * tg.max_len
        } else if tg.t <= FT_EXT + FT_HOLD {
            tg.max_len
        } else {
            (tg.max_len * (1.0 - (tg.t - FT_EXT - FT_HOLD) as f32 / FT_RET as f32)).max(0.0)
        };
        let (tipx, tipy) = (tg.ax + tg.ux * len, tg.ay + tg.uy * len);
        // Redraw the lash: a 2px line maw->tip + the 4px tip blob (js stroke + fillRect).
        let pa = at(PLAY_X + tg.ax, PLAY_Y + tg.ay, 0.0, 0.0, 9.0).translation;
        let pb = at(PLAY_X + tipx, PLAY_Y + tipy, 0.0, 0.0, 9.0).translation;
        ls.custom_size = Some(Vec2::new((pb - pa).truncate().length().max(1.0), 2.0));
        *lt = Transform::from_translation((pa + pb) / 2.0)
            .with_rotation(Quat::from_rotation_z((pb.y - pa.y).atan2(pb.x - pa.x)));
        if let Ok(mut tt) = tip_q.get_mut(tg.tip) {
            *tt = at(PLAY_X + tipx - 2.0, PLAY_Y + tipy - 2.0, 4.0, 4.0, 9.05);
        }
        if tg.t > FT_EXT + FT_HOLD + FT_RET {
            commands.entity(tg.tip).despawn();
            commands.entity(e).despawn();
        }
    }
}

pub struct MobFxPlugin;

impl Plugin for MobFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy::app::FixedUpdate,
            (burst_tick, beam_tick, drain_orb_tick, frog_tongue_tick)
                .before(crate::combat::resolve_combat)
                .run_if(super::screen::playing),
        );
    }
}
