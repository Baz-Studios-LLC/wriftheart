//! THE CAVERN TYRANT — the stone guardian of the deep caverns (cave / crystalcave /
//! darkdepths / saltmine). Bare rock has no ranged tricks, so it corners you with the
//! TERRAIN instead: its signature is ERUPTING STONE SPIKES — jagged columns that burst
//! from the floor around you on a telegraphed crack, biting as they rise and then standing
//! as no-go pillars until they crumble. Between eruptions it wades after you and SLAMS the
//! ground for a ring of flung rubble. Each third of its health lost quickens the tempo.
//!
//! One of the six consolidated bespoke bosses (boss/mod.rs) covering the themes THE TEN
//! left on elite stand-ins — authored, not templated (Baz's call).

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 56.0; // js cave/crystalcave pool (x HP_MUL)
const STONE: u32 = 0x8a8a92;
const DARK: u32 = 0x53535a;
const CORE: u32 = 0x9fe8ff;
const PAL: &[(char, u32)] = &[
    ('S', STONE),      // stone body
    ('s', DARK),       // shade / seams
    ('K', 0x2e2e34),   // outline
    ('E', CORE),       // crystal core glow
    ('e', 0x4a9ac0),   // core shade
];

const GOLEM: [&str; 20] = [
    "....KKKKKKKK....",
    "...KSSSSSSSSK...",
    "..KSSsSSSSsSSK..",
    "..KSSSSSSSSSSK..",
    ".KSSSKSSSSKSSSK.",
    ".KSSSSSSSSSSSSK.",
    "KKSSSSEEEESSSSKK",
    "KSSSSSeEEeSSSSSK",
    "KSSSSSEEEESSSSSK",
    "KSsSSSSEESSSSsSK",
    "KSSSSSSSSSSSSSSK",
    "KKSSSSSSSSSSSSKK",
    ".KSSSsSSSSsSSSK.",
    ".KSSSSSSSSSSSSK.",
    ".KKSSSSKKSSSSKK.",
    "..KSSSSK.KSSSK..",
    "..KSSSK...KSSK..",
    "..KKKK...KKKK...",
    "................",
    "................",
];

/// A jagged stone column: a crack telegraph, then a biting eruption, then a lingering
/// no-go pillar that crumbles. Its hitbox sits off-screen until the bite frames.
const SPIKE: [&str; 16] = [
    "......KK........",
    ".....KEEK.......",
    ".....KESK.......",
    "....KKSSKK......",
    "....KSSSSK......",
    "...KSSsSSK......",
    "...KSSSSSK......",
    "..KKSSSSSKK.....",
    "..KSSSsSSSK.....",
    "..KSSSSSSSK.....",
    ".KKSSSSSSSKK....",
    ".KSSsSSSsSSK....",
    ".KSSSSSSSSSK....",
    ".KKKSSSSKKK.....",
    "...KKKKKK.......",
    "................",
];

const SPIKE_TELE: i32 = 34; // crack warning before it bursts up
const SPIKE_BITE: i32 = 8; // the frames the rising column hurts
const SPIKE_HOLD: i32 = 150; // then it stands as a no-go pillar
const SPIKE_HB_OFF: f32 = -999.0;

#[derive(Component)]
pub struct CavernTyrant {
    x: f32,
    y: f32,
    anim: u32,
    phase: u8,
    slam_cd: i32,
    erupt_cd: i32,
    windup: i32, // slam telegraph counter (0 = not winding up)
    spike_img: Handle<Image>,
}

#[derive(Component)]
pub struct StoneSpike {
    x: f32,
    y: f32,
    t: i32,
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>, blockers: &mut crate::app::room_props::RoomBlockers) {
    let _ = &blockers;
    let golem_img = images.add(crate::gfx::bake(&GOLEM, PAL));
    let spike_img = images.add(crate::gfx::bake(&SPIKE, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (bx, by) = (128.0, 48.0);
    commands.spawn((
        Sprite::from_image(golem_img),
        at(PLAY_X + bx, PLAY_Y + by, 16.0, 20.0, actor_z(by + 20.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE CAVERN TYRANT"),
        crate::app::dungeon::DungeonBoss,
        CavernTyrant { x: bx, y: by, anim: 0, phase: 0, slam_cd: 150, erupt_cd: 90, windup: 0, spike_img },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(3), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 1, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.0 * (1.0 - 0.9), kb_frames: 10 },
        Knockback::default(),
        Hitbox { x: bx + 1.0, y: by + 6.0, w: 14.0, h: 14.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player, Without<CavernTyrant>>,
    mut tyrants: Query<(&mut CavernTyrant, &mut Health, &mut Hitbox, &mut Transform, &mut Visibility), (Without<Player>, Without<StoneSpike>)>,
    mut spikes: Query<(Entity, &mut StoneSpike, &mut Hitbox, &mut Sprite), (Without<CavernTyrant>, Without<Player>)>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut b, mut h, mut hb, mut tf, mut vis)) = tyrants.single_mut() else { return };
    b.anim += 1;
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let (bcx, bcy) = (b.x + 8.0, b.y + 10.0);

    // Phase-up bursts (66% / 33%): a stagger + a quick eruption to punish the closer.
    let frac = h.hp as f32 / h.max.max(1) as f32;
    let ph = if frac > 0.66 { 0 } else if frac > 0.33 { 1 } else { 2 };
    if ph > b.phase {
        b.phase = ph;
        h.flash = 16;
        h.invuln = h.invuln.max(18);
        b.windup = 0;
        b.erupt_cd = 20;
        sfx.write(crate::app::sfx::Sfx("stone"));
    }
    let tempo = 1.0 + b.phase as f32 * 0.22;

    // --- SLAM: a telegraphed rear-back, then a ring of flung rubble. ---
    if b.windup > 0 {
        b.windup -= 1;
        if b.windup == 0 {
            let n = 8 + b.phase as i32 * 2;
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                commands.spawn((
                    EBolt { x: bcx - 4.0, y: bcy - 4.0, vx: a.cos() * 2.3, vy: a.sin() * 2.3, life: 90 },
                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(3), persistent: false, knock: 0.0 },
                    HitOnce::default(),
                    Hitbox { x: bcx - 4.0, y: bcy - 4.0, w: 7.0, h: 7.0 },
                    Sprite::from_image(art.bolt(STONE, 0xd8d8e0)),
                    at(PLAY_X + bcx - 5.0, PLAY_Y + bcy - 5.0, 8.0, 8.0, 8.6),
                    PIXEL_LAYER,
                    RoomActor,
                ));
            }
            sfx.write(crate::app::sfx::Sfx("stone"));
        }
    } else {
        // Wade after the player (slow, floored so a slow status can't freeze it).
        let s = 0.36 * tempo;
        b.x = (b.x + (pcx - bcx).signum() * s).clamp(6.0, PX_W as f32 - 22.0);
        b.y = (b.y + (pcy - bcy).signum() * s).clamp(16.0, PX_H as f32 - 28.0);
        b.slam_cd -= 1;
        if b.slam_cd <= 0 {
            b.slam_cd = (170.0 / tempo) as i32;
            b.windup = 26; // rear back — the boss_bar reddens, the frame shudders
            h.flash = 4;
        }
        // --- ERUPT: crack the floor around the player, then spikes bite up. ---
        b.erupt_cd -= 1;
        if b.erupt_cd <= 0 {
            b.erupt_cd = (150.0 / tempo) as i32;
            let n = 2 + b.phase as i32;
            for i in 0..n {
                // Fan the spikes around the player's spot so they can't all be dodged one way.
                let a = b.anim as f32 * 0.7 + i as f32 * (std::f32::consts::TAU / n as f32);
                let (sx, sy) = ((pcx + a.cos() * 20.0 - 8.0).clamp(4.0, PX_W as f32 - 20.0), (pcy + a.sin() * 20.0 - 10.0).clamp(4.0, PX_H as f32 - 20.0));
                commands.spawn((
                    Sprite { image: b.spike_img.clone(), color: Color::srgba(1.0, 1.0, 1.0, 0.0), ..default() },
                    at(PLAY_X + sx, PLAY_Y + sy, 16.0, 16.0, actor_z(sy + 20.0)),
                    PIXEL_LAYER,
                    RoomActor,
                    StoneSpike { x: sx, y: sy, t: 0 },
                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(3), persistent: true, knock: 0.0 },
                    Hitbox { x: SPIKE_HB_OFF, y: SPIKE_HB_OFF, w: 10.0, h: 12.0 }, // harmless during the crack
                ));
            }
            sfx.write(crate::app::sfx::Sfx("stone"));
        }
    }

    // --- Spikes: crack (a faint ghost blinking underfoot) -> bite up (hurts) -> stand -> crumble. ---
    let off_hb = Hitbox { x: SPIKE_HB_OFF, y: SPIKE_HB_OFF, w: 10.0, h: 12.0 };
    for (e, mut sp, mut shb, mut sspr) in &mut spikes {
        sp.t += 1;
        if sp.t < SPIKE_TELE {
            let a = if (sp.t / 4) % 2 == 0 { 0.4 } else { 0.16 };
            sspr.color = Color::srgba(1.0, 0.7, 0.6, a); // a reddish warning ghost
            *shb = off_hb;
        } else if sp.t < SPIKE_TELE + SPIKE_BITE + SPIKE_HOLD {
            let held = sp.t - SPIKE_TELE - SPIKE_BITE;
            let fade = if held > SPIKE_HOLD - 30 { ((SPIKE_HOLD - held) as f32 / 30.0).max(0.0) } else { 1.0 };
            sspr.color = Color::srgba(1.0, 1.0, 1.0, fade); // risen: solid pillar, fading as it crumbles
            *shb = if sp.t < SPIKE_TELE + SPIKE_BITE {
                Hitbox { x: sp.x + 3.0, y: sp.y + 4.0, w: 10.0, h: 12.0 } // biting only on the way up
            } else {
                off_hb
            };
        } else {
            commands.entity(e).despawn();
        }
    }

    // --- Sync the tyrant. ---
    *hb = Hitbox { x: b.x + 1.0, y: b.y + 6.0, w: 14.0, h: 14.0 };
    let shudder = if b.windup > 0 { ((b.anim as f32) * 0.9).sin() * 1.2 } else { 0.0 };
    *tf = at(PLAY_X + b.x + shudder, PLAY_Y + b.y, 16.0, 20.0, actor_z(b.y + 20.0));
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

/// The tyrant falls: its spikes crumble, rubble scatters, the reward is banked by the arena.
#[allow(clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    tyrants: Query<(Entity, &CavernTyrant, &Health)>,
    spikes: Query<Entity, With<StoneSpike>>,
) {
    let Ok((e, b, h)) = tyrants.single() else { return };
    if h.hp > 0 {
        return;
    }
    for se in &spikes {
        commands.entity(se).despawn();
    }
    let (cx, cy) = (b.x + 8.0, b.y + 10.0);
    for i in 0..3 {
        let off = i as f32 * 7.0 - 7.0;
        spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.4), STONE, 12);
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
    fn grids_are_rectangular() {
        for (i, r) in GOLEM.iter().enumerate() {
            assert_eq!(r.chars().count(), 16, "golem row {i}");
        }
        for (i, r) in SPIKE.iter().enumerate() {
            assert_eq!(r.chars().count(), 16, "spike row {i}");
        }
    }
}
