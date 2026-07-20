//! THE DREAD KNIGHT — the armoured warden of the castle halls. A pure melee bruiser with
//! two telegraphed committals you learn to read and step around: the SHIELD CHARGE (it rears
//! behind its tower shield, then BARRELS across the arena on your line — sidestep it) and the
//! GREATSWORD SWEEP (it winds up, then its blade carves a wide arc across its front — back
//! out of reach). No ranged tricks: it closes the distance and makes you dance. Each third of
//! its health lost shortens the wind-ups.
//!
//! One of the six consolidated bespoke bosses (boss/mod.rs). Distinct from the overworld dark
//! knight (the castle-guard captain that rides the finale) — this is the dungeon's own warden.

use bevy::prelude::*;

use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 64.0; // js castle pool (x HP_MUL)
const STEEL: u32 = 0xb8bcc8;
const PLUME: u32 = 0xc03040;
const PAL: &[(char, u32)] = &[
    ('A', STEEL),      // plate
    ('a', 0x7c808c),   // plate shade
    ('K', 0x2a2c34),   // outline / visor
    ('n', 0x50525c),   // dark steel
    ('P', PLUME),      // helm plume
    ('G', 0xe8c84a),   // gold trim
    ('E', 0xff5050),   // visor glow
];

const KNIGHT: [&str; 20] = [
    ".......PP.......",
    "......PPPP......",
    ".....KAAAAK.....",
    "....KAAAAAAK....",
    "....KAEKKEAK....",
    "....KAAAAAAK....",
    "...KAAaaaaAAK...",
    "..KGAAAAAAAAGK..",
    "..KAAAaAAaAAAK..",
    "..KAAAAAAAAAAK..",
    "..KnAAAAAAAAnK..",
    "...KAAAaaAAAK...",
    "...KAAAAAAAAK...",
    "...KaAAAAAAaK...",
    "...KAAK..KAAK...",
    "...KAAK..KAAK...",
    "...KnnK..KnnK...",
    "...KKKK..KKKK...",
    "................",
    "................",
];

const CHARGE_WIND: i32 = 30; // rears behind the shield
const CHARGE_RUN: i32 = 30; // the barrel across
const SWEEP_WIND: i32 = 24; // winds the blade back
const SWEEP_CUT: i32 = 10; // the arc is live

#[derive(Component)]
pub struct DreadKnight {
    x: f32,
    y: f32,
    anim: u32,
    phase: u8,
    charge_cd: i32,
    sweep_cd: i32,
    /// 0 idle, 1 charge-wind, 2 charging, 3 sweep-wind, 4 sweeping.
    mode: u8,
    t: i32,
    vx: f32,
    vy: f32,   // charge velocity
    fx: f32,
    fy: f32,   // committed facing for the sweep arc
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let img = images.add(crate::gfx::bake(&KNIGHT, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (bx, by) = (128.0, 44.0);
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + bx, PLAY_Y + by, 16.0, 20.0, actor_z(by + 20.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE DREAD KNIGHT"),
        crate::app::dungeon::DungeonBoss,
        DreadKnight { x: bx, y: by, anim: 0, phase: 0, charge_cd: 120, sweep_cd: 70, mode: 0, t: 0, vx: 0.0, vy: 0.0, fx: 0.0, fy: 1.0 },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(3), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 1, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.0 * (1.0 - 0.85), kb_frames: 10 },
        Knockback::default(),
        Hitbox { x: bx + 2.0, y: by + 4.0, w: 12.0, h: 14.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player, Without<DreadKnight>>,
    mut knights: Query<(&mut DreadKnight, &mut Health, &mut Hitbox, &mut Transform, &mut Visibility), Without<Player>>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut b, mut h, mut hb, mut tf, mut vis)) = knights.single_mut() else { return };
    b.anim += 1;
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let (bcx, bcy) = (b.x + 8.0, b.y + 10.0);

    // Phase-up (66% / 33%): a wrathful stagger; wind-ups shorten via tempo below.
    let frac = h.hp as f32 / h.max.max(1) as f32;
    let ph = if frac > 0.66 { 0 } else if frac > 0.33 { 1 } else { 2 };
    if ph > b.phase {
        b.phase = ph;
        h.flash = 16;
        h.invuln = h.invuln.max(18);
        b.mode = 0;
        b.charge_cd = 26;
        sfx.write(crate::app::sfx::Sfx("stone"));
    }
    let tempo = 1.0 + b.phase as f32 * 0.22;

    // The default contact box (overwritten by a live sweep arc below).
    let mut new_hb = Hitbox { x: b.x + 2.0, y: b.y + 4.0, w: 12.0, h: 14.0 };

    match b.mode {
        1 => {
            // Shield-charge wind-up: hold, aim at the player, then commit.
            b.t -= 1;
            h.flash = h.flash.max(2);
            if b.t <= 0 {
                let d = ((pcx - bcx).powi(2) + (pcy - bcy).powi(2)).sqrt().max(0.001);
                let sp = 3.2 + b.phase as f32 * 0.6;
                b.vx = (pcx - bcx) / d * sp;
                b.vy = (pcy - bcy) / d * sp;
                b.mode = 2;
                b.t = CHARGE_RUN;
                sfx.write(crate::app::sfx::Sfx("stone"));
            }
        }
        2 => {
            // Charging: barrel along the committed line until it wedges or the run ends.
            let nx = (b.x + b.vx).clamp(6.0, PX_W as f32 - 22.0);
            let ny = (b.y + b.vy).clamp(16.0, PX_H as f32 - 28.0);
            let wedged = (nx - b.x).abs() < 0.1 && (ny - b.y).abs() < 0.1;
            b.x = nx;
            b.y = ny;
            b.t -= 1;
            if b.t <= 0 || wedged {
                b.mode = 0;
                b.charge_cd = (200.0 / tempo) as i32;
            }
        }
        3 => {
            // Greatsword wind-up: freeze facing, then the arc goes live.
            b.t -= 1;
            h.flash = h.flash.max(2);
            if b.t <= 0 {
                b.mode = 4;
                b.t = SWEEP_CUT;
                sfx.write(crate::app::sfx::Sfx("swing"));
            }
        }
        4 => {
            // Sweeping: a wide arc lunges out along the committed facing.
            b.t -= 1;
            let reach = 16.0;
            new_hb = Hitbox { x: bcx + b.fx * reach - 13.0, y: bcy + b.fy * reach - 13.0, w: 26.0, h: 26.0 };
            if b.t <= 0 {
                b.mode = 0;
                b.sweep_cd = (140.0 / tempo) as i32;
            }
        }
        _ => {
            // Advance on the player and pick a committal when in range / off cooldown.
            let s = 0.5 * tempo;
            b.x = (b.x + (pcx - bcx).signum() * s).clamp(6.0, PX_W as f32 - 22.0);
            b.y = (b.y + (pcy - bcy).signum() * s * 0.85).clamp(16.0, PX_H as f32 - 28.0);
            let d = ((pcx - bcx).powi(2) + (pcy - bcy).powi(2)).sqrt();
            b.charge_cd -= 1;
            b.sweep_cd -= 1;
            if b.sweep_cd <= 0 && d < 40.0 {
                // Close: commit the sweep, freezing the facing toward the player.
                let dd = d.max(0.001);
                b.fx = (pcx - bcx) / dd;
                b.fy = (pcy - bcy) / dd;
                b.mode = 3;
                b.t = (SWEEP_WIND as f32 / tempo) as i32;
                h.flash = 4;
            } else if b.charge_cd <= 0 && d >= 40.0 {
                // Far: commit the charge.
                b.mode = 1;
                b.t = (CHARGE_WIND as f32 / tempo) as i32;
                h.flash = 4;
            }
        }
    }

    *hb = new_hb;
    let shudder = if b.mode == 1 || b.mode == 3 { ((b.anim as f32) * 0.9).sin() * 1.2 } else { 0.0 };
    *tf = at(PLAY_X + b.x + shudder, PLAY_Y + b.y, 16.0, 20.0, actor_z(b.y + 20.0));
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

/// The knight falls: its plate scatters, the arena banks the reward.
#[allow(clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    knights: Query<(Entity, &DreadKnight, &Health)>,
) {
    let Ok((e, b, h)) = knights.single() else { return };
    if h.hp > 0 {
        return;
    }
    let (cx, cy) = (b.x + 8.0, b.y + 10.0);
    for i in 0..3 {
        let off = i as f32 * 7.0 - 7.0;
        spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.4), STEEL, 12);
    }
    let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
    crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
    crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true);
    crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
    stats.bump("kills", 1.0);
    stats.bump_kill("boss");
    sfx.write(crate::app::sfx::Sfx("stone"));
    commands.entity(e).despawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_rectangular() {
        for (i, r) in KNIGHT.iter().enumerate() {
            assert_eq!(r.chars().count(), 16, "knight row {i}");
        }
    }
}
