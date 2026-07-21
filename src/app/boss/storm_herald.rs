//! THE STORM HERALD — the sky-guardian of the high halls (stormspire / windbarrow). It
//! keeps its distance on the wind and calls the sky down: its signature is the LIGHTNING
//! STORM — a scatter of telegraphed columns that fall around you (favouring where you
//! stand), reusing the stormcaller's skystrike. Between storms it looses fans of CHAIN
//! BOLTS. Never a melee brawler — it drifts away and answers from above; each third of its
//! health lost thickens the storm.
//!
//! One of the six consolidated bespoke bosses (boss/mod.rs).

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 60.0; // js stormspire pool (x HP_MUL)
const BOLT: u32 = 0x9ad0ff;
const CLOUD: u32 = 0x8098d8;
const PAL: &[(char, u32)] = &[
    ('C', 0x6a7ec0),   // cloud body
    ('c', 0x4a5a94),   // cloud shade
    ('K', 0x2a3050),   // outline
    ('W', 0xeef4ff),   // highlight
    ('E', 0xffe64a),   // eyes / charge
    ('B', BOLT),       // lightning
];

const HERALD: [&str; 18] = [
    "....KKKKKK......",
    "..KKCCCCCCKK....",
    ".KCCCcCCCCCCK...",
    "KCCCCCCCcCCCCK..",
    "KCcCCEWWECCcCK..",
    "KCCCCEWWECCCCK..",
    ".KCCCCWWCCCCK...",
    "..KCcCBBcCCK....",
    "...KCCBBCCK.....",
    "....KBWWBK......",
    "...KBWBBWBK.....",
    "...KBBWWBBK.....",
    "....KBBBBK......",
    ".....KBBK.......",
    "......BB........",
    ".....B..B.......",
    "....B....B......",
    "................",
];

#[derive(Component)]
pub struct StormHerald {
    x: f32,
    y: f32,
    anim: u32,
    phase: u8,
    storm_cd: i32,
    volley_cd: i32,
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let img = images.add(crate::gfx::bake(&HERALD, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (bx, by) = (128.0, 40.0);
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + bx, PLAY_Y + by, 16.0, 18.0, actor_z(by + 18.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE STORM HERALD"),
        crate::app::dungeon::DungeonBoss,
        StormHerald { x: bx, y: by, anim: 0, phase: 0, storm_cd: 120, volley_cd: 80 },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(3), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2 * (1.0 - 0.5), kb_frames: 11 },
        Knockback::default(),
        Hitbox { x: bx + 2.0, y: by + 4.0, w: 12.0, h: 12.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player, Without<StormHerald>>,
    mut heralds: Query<(&mut StormHerald, &mut Health, &mut Hitbox, &mut Transform, &mut Visibility), Without<Player>>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut b, mut h, mut hb, mut tf, mut vis)) = heralds.single_mut() else { return };
    b.anim += 1;
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let (bcx, bcy) = (b.x + 8.0, b.y + 9.0);

    // Phase-up (66% / 33%): a thunderclap stagger.
    let frac = h.hp as f32 / h.max.max(1) as f32;
    let ph = if frac > 0.66 { 0 } else if frac > 0.33 { 1 } else { 2 };
    if ph > b.phase {
        b.phase = ph;
        h.flash = 16;
        h.invuln = h.invuln.max(18);
        b.storm_cd = 24;
        sfx.write(crate::app::sfx::Sfx("thunder"));
    }
    let tempo = 1.0 + b.phase as f32 * 0.22;

    // --- Movement: drift to keep its distance (retreat when crowded), bobbing on the wind. ---
    let dx = bcx - pcx;
    let dy = bcy - pcy;
    let d = (dx * dx + dy * dy).sqrt().max(0.001);
    let want = if d < 70.0 { 0.9 } else if d > 120.0 { -0.5 } else { 0.0 }; // + away, - toward
    b.x = (b.x + dx / d * want).clamp(8.0, PX_W as f32 - 24.0);
    b.y = (b.y + dy / d * want * 0.6 + ((b.anim as f32) * 0.05).sin() * 0.5).clamp(16.0, PX_H as f32 - 60.0);

    // --- LIGHTNING STORM (signature): columns fall around you, one right on your mark. ---
    b.storm_cd -= 1;
    if b.storm_cd <= 0 {
        b.storm_cd = (180.0 / tempo) as i32;
        let n = 2 + b.phase as i32;
        for i in 0..=n {
            let (sx, sy) = if i == 0 {
                (pcx - 8.0, pcy - 8.0) // dead on the player
            } else {
                let a = b.anim as f32 * 1.3 + i as f32 * 2.1;
                ((pcx + a.cos() * (26.0 + i as f32 * 10.0) - 8.0).clamp(2.0, PX_W as f32 - 22.0), (pcy + a.sin() * (26.0 + i as f32 * 10.0) - 8.0).clamp(2.0, PX_H as f32 - 22.0))
            };
            super::super::skystrike::spawn(&mut commands, sx + 8.0, sy + 8.0);
        }
        sfx.write(crate::app::sfx::Sfx("thunder"));
    }

    // --- CHAIN BOLTS: a fan of fast sparks aimed your way. ---
    b.volley_cd -= 1;
    if b.volley_cd <= 0 {
        b.volley_cd = (110.0 / tempo) as i32;
        let base = (pcy - bcy).atan2(pcx - bcx);
        let n = 3 + b.phase as i32;
        for i in 0..n {
            let a = base + 0.34 * (i as f32 - (n - 1) as f32 / 2.0);
            commands.spawn((
                EBolt { x: bcx - 4.0, y: bcy, vx: a.cos() * 2.8, vy: a.sin() * 2.8, life: 120 },
                Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                HitOnce::default(),
                Hitbox { x: bcx - 1.0, y: bcy + 3.0, w: 7.0, h: 7.0 },
                Sprite::from_image(art.bolt(BOLT, 0xffffff)),
                at(PLAY_X + bcx - 5.0, PLAY_Y + bcy + 1.0, 8.0, 8.0, 8.6),
                PIXEL_LAYER,
                RoomActor,
            ));
        }
        sfx.write(crate::app::sfx::Sfx("tink"));
    }

    // --- Sync. ---
    *hb = Hitbox { x: b.x + 2.0, y: b.y + 4.0, w: 12.0, h: 12.0 };
    let bob = ((b.anim as f32) * 0.16).sin() * 1.6;
    *tf = at(PLAY_X + b.x, PLAY_Y + b.y + bob, 16.0, 18.0, actor_z(b.y + 18.0));
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

/// The herald scatters into a last sheet of sparks; the arena banks the reward.
#[allow(clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    heralds: Query<(Entity, &StormHerald, &Health)>,
) {
    let Ok((e, b, h)) = heralds.single() else { return };
    if h.hp > 0 {
        return;
    }
    let (cx, cy) = (b.x + 8.0, b.y + 9.0);
    for i in 0..3 {
        let off = i as f32 * 7.0 - 7.0;
        spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy - off * 0.4), CLOUD, 12);
    }
    let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
    crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
    crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true, None);
    crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
    stats.bump("kills", 1.0);
    stats.bump_kill("boss");
    sfx.write(crate::app::sfx::Sfx("thunder"));
    commands.entity(e).despawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_rectangular() {
        for (i, r) in HERALD.iter().enumerate() {
            assert_eq!(r.chars().count(), 16, "herald row {i}");
        }
    }
}
