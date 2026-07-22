//! Bursts, blood and the sprite syncs — the render-facing tail of the battle
//! chain (split from battle.rs).

use super::{GameRng, RoomActor};
use crate::actors::goblin::{Goblin, GoblinArt, GoblinKind};
use crate::actors::mobs::{self, Mob, MobArtBank};
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Health, HitLanded};
use crate::gfx::{at, PIXEL_LAYER};
use bevy::prelude::*;

/// Frame pick + transform for every biome mob: wolf is per-facing, the thornling disguises
/// while dormant, the burrower is a dirt mound underground; fliers ride ABOVE the y-sort.
/// The sync's query rows (a type alias keeps clippy content).
type MobSprites<'w, 's> = Query<
    'w,
    's,
    (
        &'static Mob,
        &'static Health,
        &'static mut Sprite,
        &'static mut Transform,
        &'static mut Visibility,
        Option<&'static crate::app::uniques::MobAfflictions>,
    ),
>;

pub(super) fn sync_mobs(art: Res<MobArtBank>, mut q: MobSprites) {
    for (m, h, mut sprite, mut tf, mut vis, aff) in &mut q {
        let d = &mobs::MOB_DEFS[m.def];
        let is_hopper = matches!(d.ai, mobs::Ai::Hopper { .. });
        let (img, w, hh) = match d.kind {
            "wolf" => {
                let f = &art.wolf[m.facing.min(3)];
                f[(m.anim / 6) as usize % 2].clone()
            }
            "thornling" if m.st == 0 => art.thorn_dorm.clone(),
            "burrower" if m.st == 0 => (art.mound.clone(), 16.0, 16.0),
            // Two-state ambushers/statues (frame 0 = active, frame 1 = dormant/sealed/stone).
            "vinesnare" | "saltstatue" => {
                let set = &art.frames[d.kind];
                set[0][if m.st == 1 { 0 } else { 1 }].clone()
            }
            // The water snipers: frame 0 = the submerged ripple, frame 1 = surfaced.
            "spitgill" | "bogmaw" | "frostgill" => {
                let set = &art.frames[d.kind];
                let side = if m.facing == 3 { 1 } else { 0 };
                set[side][if m.st == 1 { 1 } else { 0 }].clone()
            }
            "bellsnail" => {
                // Sealed in the bell for the first 120 of its 220 clock, then out.
                let set = &art.frames[d.kind];
                set[0][if (m.anim % 220) >= 120 { 0 } else { 1 }].clone()
            }
            "emberling" => {
                // js: the HOT frame strobes only while the fuse burns (st 1, t & 2).
                let set = &art.frames[d.kind];
                let side = if m.facing == 3 { 1 } else { 0 };
                set[side][if m.st == 1 && (m.t & 2) != 0 { 1 } else { 0 }].clone()
            }
            "frog" => {
                // Poses: idle / mid-leap / tongue agape (js S_FROG_FR by state).
                let set = &art.frames["frog"];
                let side = if m.facing == 3 { 1 } else { 0 };
                let fr = if m.st == 1 { 1 } else if m.st == 2 { 2 } else { 0 };
                set[side][fr].clone()
            }
            _ => {
                // Fall back to the goblin-less default if a kind somehow lacks art (defensive).
                let Some(set) = art.frames.get(d.kind) else {
                    continue;
                };
                let side = if m.facing == 3 { 1 } else { 0 };
                let frames = &set[side];
                frames[(m.anim / 6) as usize % frames.len()].clone()
            }
        };
        sprite.image = img;
        // Ghosts drift translucent (js wraith globalAlpha 0.8); everyone else opaque.
        // FROZEN outranks everything: the ice-blue cast (this sync writes color every
        // frame, so the tint must live HERE or it gets stomped — the frost-beam fix).
        sprite.color = if aff.is_some_and(|a| a.freeze > 0) {
            Color::srgb_u8(0x60, 0xa8, 0xff)
        } else if aff.is_some_and(|a| a.poison > 0) {
            Color::srgb_u8(0xb8, 0x68, 0xe8) // envenomed: the purple cast
        } else if d.kind == "boglight" {
            // Half out of the world every other beat (js faded alpha 0.28).
            Color::srgba(1.0, 1.0, 1.0, if (m.anim % 260) >= 130 { 0.28 } else { 1.0 })
        } else if d.ghost {
            Color::srgba(1.0, 1.0, 1.0, 0.8)
        } else {
            Color::WHITE
        };
        // Hop bounce: slimes rise during their hop window; frogs during a leap (js draws).
        let hop = if is_hopper {
            let ph = (m.t % 36) as f32;
            if ph < 10.0 { ((ph / 10.0 * std::f32::consts::PI).sin() * 3.0).round() } else { 0.0 }
        } else if d.kind == "frog" && m.st == 1 {
            ((m.t.max(0) as f32 / 12.0 * std::f32::consts::PI).sin() * 4.0).round()
        } else {
            0.0
        };
        // Split-children draw at 10px (js drawImage 16 -> 10 downscale, smoothing off);
        // elites grow from the feet (js sizeMul via translate/scale).
        let (dw, dh, ox, oy) = if m.small {
            (10.0, 10.0 * hh / w, 3.0, 5.0)
        } else if m.size_mul > 1.0 {
            (w * m.size_mul, hh * m.size_mul, -(w * (m.size_mul - 1.0)) / 2.0, -(hh * (m.size_mul - 1.0)))
        } else {
            (w, hh, 0.0, 0.0)
        };
        sprite.custom_size = if m.small || m.size_mul > 1.0 { Some(Vec2::new(dw, dh)) } else { None };
        // js drawImage(img, x, y): top-left anchored at the entity origin; fliers skip the
        // y-sort and ride above the ground actors.
        let z = if d.fly { 8.2 } else { actor_z(m.y.round() + 16.0) };
        *tf = at(PLAY_X + m.x.round() + ox, PLAY_Y + m.y.round() + oy - hop, dw, dh, z);
        // Collapsed zombies sprawl sideways, flickering just before they rise (js draw).
        if m.downed {
            tf.rotation = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
            *vis = if m.down_t < 40 && ((m.down_t >> 1) & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
            continue;
        }
        tf.rotation = if m.sleep > 0 {
            Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2) // dreaming: sprawled flat
        } else {
            Quat::IDENTITY
        };
        *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
    }
}

/// Blood spray on every landed hit (reads the resolve pass's messages).
pub(super) fn blood_fx(mut commands: Commands, mut rng: ResMut<GameRng>, mut hits: MessageReader<HitLanded>) {
    for hit in hits.read() {
        if let Some(color) = hit.blood {
            spawn_burst(&mut commands, &mut rng, hit.at, color, 5);
        }
        if hit.crit {
            // js spark(): the gold sparkle that marks a critical hit.
            spawn_burst(&mut commands, &mut rng, hit.at, 0xfcd000, 8);
        }
    }
}

/// A tiny particle burst — APPROXIMATION of Entities.blood until the FX port (shape and
/// count are close; exact velocities/gravity come with entities.js).
#[derive(Component)]
pub(super) struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: u32,
}

pub fn spawn_burst(commands: &mut Commands, rng: &mut GameRng, at_px: Vec2, color: u32, n: usize) {
    for _ in 0..n {
        let a = rng.0.next_f64() as f32 * std::f32::consts::TAU;
        let sp = 0.6 + rng.0.next_f64() as f32 * 1.2;
        commands.spawn((
            Particle { x: at_px.x, y: at_px.y, vx: a.cos() * sp, vy: a.sin() * sp - 0.6, life: 16 },
            Sprite::from_color(
                Color::srgb_u8((color >> 16) as u8, (color >> 8) as u8, color as u8),
                Vec2::splat(2.0),
            ),
            at(PLAY_X + at_px.x, PLAY_Y + at_px.y, 2.0, 2.0, 12.0),
            PIXEL_LAYER,
            RoomActor,
        ));
    }
}

pub(super) fn particles_tick(mut commands: Commands, mut q: Query<(Entity, &mut Particle, &mut Transform)>) {
    for (e, mut p, mut tf) in &mut q {
        p.vy += 0.08; // light gravity
        p.x += p.vx;
        p.y += p.vy;
        p.life -= 1;
        *tf = at(PLAY_X + p.x, PLAY_Y + p.y, 2.0, 2.0, 12.0);
        if p.life == 0 {
            commands.entity(e).despawn();
        }
    }
}

/// Goblin sprite sync: frame by facing/step, hit-flash blink. A HumanSkin (bandit)
/// swaps the goblin art for its person-in-costume frames, same gait.
#[allow(clippy::type_complexity)] // the goblin render row: chassis + skin + sprite state
/// The goblin sync's query rows (a type alias keeps clippy content).
type GoblinSprites<'w, 's> = Query<
    'w,
    's,
    (
        &'static Goblin,
        &'static Health,
        Option<&'static crate::actors::goblin::HumanSkin>,
        Option<&'static crate::app::uniques::MobAfflictions>,
        &'static mut Sprite,
        &'static mut Transform,
        &'static mut Visibility,
    ),
>;

pub(super) fn sync_goblins(art: Res<GoblinArt>, mut q: GoblinSprites) {
    for (g, h, skin, aff, mut sprite, mut tf, mut vis) in &mut q {
        let kind_idx = if g.kind == GoblinKind::Spear { 1 } else { 0 };
        sprite.image = match skin {
            Some(s) => s.frames[g.facing as usize][g.frame].clone(), // stand / step
            None => art.0[kind_idx][g.facing as usize][g.frame].clone(),
        };
        // FROZEN/ENVENOMED wear the same casts the beasts do (sync_mobs' rule).
        sprite.color = if aff.is_some_and(|a| a.freeze > 0) {
            Color::srgb_u8(0x60, 0xa8, 0xff)
        } else if aff.is_some_and(|a| a.poison > 0) {
            Color::srgb_u8(0xb8, 0x68, 0xe8)
        } else {
            Color::WHITE
        };
        *tf = at(PLAY_X + g.x.round(), PLAY_Y + g.y.round(), 16.0, 16.0, actor_z(g.y.round() + 16.0));
        // Hit flash: skip-draw on alternating frames (js: if (e.flash & 1) return).
        *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
    }
}

/// Attack visuals are positioned in attacks_tick (they move on the fixed clock); nothing to
/// do per render frame yet — kept as the seam where interpolation lands later.
pub(super) fn sync_attacks() {}
