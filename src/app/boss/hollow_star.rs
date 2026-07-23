//! THE HOLLOW STAR — boss 10 of THE TEN (BOSSES.md): the Wrift Vault's guardian,
//! keeper of the deepest shard.
//!
//! The lights go OUT. The vault's torches die the moment it wakes; all you get is
//! its own cold radiance, a small lantern-glow of your own, and the gleam of its
//! four STAR SHARDS wheeling wide around it. While any shard burns the star is
//! hollow — untouchable — and the shards chain CONSTELLATION BEAMS between each
//! other that you thread in the dark, while METEORS fall on telegraphed rings.
//! Break every shard and the star BARES: each one you shatter snuffs more of the
//! light, until the end is fought in a darkness lit only by the thing you're
//! killing.

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 100.0;
const STARLIGHT: u32 = 0xc0d8ff;
const PAL: &[(char, u32)] = &[
    ('S', STARLIGHT), // radiance
    ('s', 0x8098d8),  // radiance deep
    ('P', 0x0a0a18),  // hollow core
    ('W', 0xffffff),  // gleam
];

const STAR: [&str; 20] = [
    ".........KK.........",
    "........KSSK........",
    ".......KSsSSK.......",
    "..K....KSSsSK....K..",
    ".KSK..KSsSSsSK..KSK.",
    ".KsSKKSSPPPPSSKKSsK.",
    "..KSSsSPPPPPPSsSSK..",
    "...KSsPPWPPWPPsSK...",
    "..KSSsPPPPPPPPsSSK..",
    ".KsSSsPPPPPPPPsSSsK.",
    ".KSsSsPPWPPWPPsSsSK.",
    "..KSSsPPPPPPPPsSSK..",
    "...KSsSPPPPPPSsSK...",
    "..KSsSSsPPPPsSSsSK..",
    ".KSK..KSsSSsSK..KSK.",
    ".KK....KSSsSK....KK.",
    "........KSsSK.......",
    ".........KSSK.......",
    "..........KK........",
    "....................",
];
const SHARD: [&str; 10] = [
    "....KK....",
    "...KSSK...",
    "..KSWsSK..",
    "..KSsSSK..",
    ".KSsSWsSK.",
    ".KSSsSSsK.",
    "..KSWsSK..",
    "..KsSSK...",
    "...KSK....",
    "....K.....",
];
const RING: [&str; 12] = [
    "....SSSSS.....",
    "..SS.....SS...",
    ".S.........S..",
    ".S.........S..",
    "S...........S.",
    "S...........S.",
    "S...........S.",
    ".S.........S..",
    ".S.........S..",
    "..SS.....SS...",
    "....SSSSS.....",
    "..............",
];

#[derive(Component)]
pub struct HollowStar {
    x: f32,
    y: f32,
    anim: u32,
    meteor: Option<(f32, f32, i32, Entity)>,
    meteor_cd: i32,
    beam_cd: i32,
    nova_cd: i32,
    ring_img: Handle<Image>,
}

#[derive(Component)]
pub struct StarShard {
    slot: usize,
    x: f32,
    y: f32,
}

#[derive(Component)]
pub struct StarBeam {
    t: i32,
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let star_img = images.add(crate::gfx::bake(&STAR, PAL));
    let shard_img = images.add(crate::gfx::bake(&SHARD, PAL));
    let ring_img = images.add(crate::gfx::bake(&RING, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (sx, sy) = (142.0, 78.0);
    for slot in 0..4 {
        commands.spawn((
            Sprite::from_image(shard_img.clone()),
            at(PLAY_X + sx, PLAY_Y + sy, 10.0, 10.0, actor_z(sy + 8.0)),
            PIXEL_LAYER,
            RoomActor,
            StarShard { slot, x: sx, y: sy },
            Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(1), persistent: true, knock: 0.0 },
            Health { hp: 6, max: 6, defense: 0, invuln: 0, flash: 0 },
            HurtProfile { invuln: 10, flash: 8, kb_base: 0.0, kb_resist: 0.0, kb_frames: 0 },
            Knockback::default(),
            Hitbox { x: sx, y: sy, w: 9.0, h: 9.0 },
        ));
    }
    commands.spawn((
        Sprite::from_image(star_img),
        at(PLAY_X + sx, PLAY_Y + sy, 20.0, 20.0, actor_z(sy + 18.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE HOLLOW STAR"),
        crate::app::dungeon::DungeonBoss,
        HollowStar { x: sx, y: sy, anim: 0, meteor: None, meteor_cd: 220, beam_cd: 200, nova_cd: 170, ring_img },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(4), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2, kb_resist: 0.92, kb_frames: 11 },
        Knockback::default(),
        Hitbox { x: sx + 3.0, y: sy + 3.0, w: 14.0, h: 14.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut rng: ResMut<GameRng>,
    mut lights: ResMut<crate::app::dungeon::DungeonLights>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player>,
    mut stars: Query<
        (&mut HollowStar, &mut Health, &mut Hitbox, &mut Transform, &mut Visibility),
        (Without<StarShard>, Without<StarBeam>, Without<Player>),
    >,
    mut shards: Query<
        (&mut StarShard, &mut Hitbox, &mut Transform, &mut Visibility, &Health),
        (Without<HollowStar>, Without<StarBeam>, Without<Player>),
    >,
    mut beams: Query<(Entity, &mut StarBeam, &mut Sprite), (Without<HollowStar>, Without<StarShard>)>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut st, mut h, mut hb, mut tf, mut vis)) = stars.single_mut() else { return };
    st.anim += 1;
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let shard_count = shards.iter().count();
    let bared = shard_count == 0;
    if !bared {
        h.invuln = h.invuln.max(2); // hollow: there is nothing to cut yet
    }

    // --- THE DARKNESS IS ITS OWN: the room's torches die; only these glows remain. ---
    let (scx, scy) = (st.x + 10.0, st.y + 10.0);
    lights.0.clear();
    lights.0.push((scx as i32, scy as i32, 22 + shard_count as i32 * 6));
    lights.0.push((pcx as i32, pcy as i32, 20)); // the hero's own small lantern-glow
    let mut shard_pts: Vec<(f32, f32)> = Vec::new();
    for (mut sh, mut shb, mut stf, mut svis, shh) in &mut shards {
        let a = (st.anim as f32) * 0.009 + sh.slot as f32 / 4.0 * std::f32::consts::TAU;
        sh.x = scx + a.cos() * 58.0 - 5.0;
        sh.y = scy + a.sin() * 38.0 - 5.0;
        shard_pts.push((sh.x + 5.0, sh.y + 5.0));
        lights.0.push(((sh.x + 5.0) as i32, (sh.y + 5.0) as i32, 14));
        *shb = Hitbox { x: sh.x, y: sh.y, w: 9.0, h: 9.0 };
        *stf = at(PLAY_X + sh.x, PLAY_Y + sh.y, 10.0, 10.0, actor_z(sh.y + 8.0));
        *svis = if shh.flash > 0 && (shh.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
    }

    // --- Constellation beams: the shards chain, and the chains cut. ---
    for (e, mut beam, mut bs) in &mut beams {
        beam.t -= 1;
        bs.color = bs.color.with_alpha((beam.t as f32 / 44.0).min(1.0) * 0.9);
        if beam.t <= 0 {
            commands.entity(e).despawn();
        }
    }
    st.beam_cd -= 1;
    if st.beam_cd <= 0 && shard_pts.len() >= 2 {
        st.beam_cd = 260;
        for w in shard_pts.windows(2) {
            let (ax, ay) = w[0];
            let (bx, by) = w[1];
            let pa = at(PLAY_X + ax, PLAY_Y + ay, 0.0, 0.0, 9.1).translation;
            let pb = at(PLAY_X + bx, PLAY_Y + by, 0.0, 0.0, 9.1).translation;
            let len = (pb - pa).truncate().length().max(1.0);
            let mut s = Sprite::from_color(Color::srgb_u8(0xc0, 0xd8, 0xff), Vec2::new(1.0, 2.0));
            s.custom_size = Some(Vec2::new(len, 2.0));
            commands.spawn((
                s,
                Transform::from_translation((pa + pb) / 2.0)
                    .with_rotation(Quat::from_rotation_z((pb.y - pa.y).atan2(pb.x - pa.x))),
                PIXEL_LAYER,
                RoomActor,
                StarBeam { t: 44 },
            ));
            for k in [0.25f32, 0.5, 0.75] {
                let (mx, my) = (ax + (bx - ax) * k, ay + (by - ay) * k);
                commands.spawn((
                    EBolt { x: mx - 4.0, y: my - 4.0, vx: 0.0, vy: 0.0, life: 44 },
                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                    HitOnce::default(),
                    Hitbox { x: mx - 3.0, y: my - 3.0, w: 6.0, h: 6.0 },
                    Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.0), Vec2::new(1.0, 1.0)),
                    at(PLAY_X + mx, PLAY_Y + my, 1.0, 1.0, 9.1),
                    PIXEL_LAYER,
                    RoomActor,
                ));
            }
        }
        sfx.write(crate::app::sfx::Sfx("tink"));
    }

    // --- Meteors on telegraphed rings. ---
    if let Some((mx, my, mut mt, ring)) = st.meteor {
        mt -= 1;
        if mt <= 0 {
            commands.entity(ring).despawn();
            spawn_burst(&mut commands, &mut rng, Vec2::new(mx, my), STARLIGHT, 12);
            for i in 0..4 {
                let a = i as f32 / 4.0 * std::f32::consts::TAU + 0.5;
                commands.spawn((
                    EBolt { x: mx - 4.0, y: my - 4.0, vx: a.cos() * 1.6, vy: a.sin() * 1.6, life: 60 },
                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                    HitOnce::default(),
                    Hitbox { x: mx - 1.0, y: my - 1.0, w: 7.0, h: 7.0 },
                    Sprite::from_image(art.bolt(STARLIGHT, 0xffffff)),
                    at(PLAY_X + mx - 3.0, PLAY_Y + my - 3.0, 8.0, 8.0, 8.6),
                    PIXEL_LAYER,
                    RoomActor,
                ));
            }
            commands.spawn((
                EBolt { x: mx - 6.0, y: my - 6.0, vx: 0.0, vy: 0.0, life: 6 },
                Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(3), persistent: false, knock: 0.0 },
                HitOnce::default(),
                Hitbox { x: mx - 7.0, y: my - 7.0, w: 14.0, h: 14.0 },
                Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.0), Vec2::new(1.0, 1.0)),
                at(PLAY_X + mx, PLAY_Y + my, 1.0, 1.0, 9.1),
                PIXEL_LAYER,
                RoomActor,
            ));
            st.meteor = None;
            sfx.write(crate::app::sfx::Sfx("stone"));
        } else {
            st.meteor = Some((mx, my, mt, ring));
        }
    } else {
        st.meteor_cd -= 1;
        if st.meteor_cd <= 0 {
            st.meteor_cd = if bared { 150 } else { 220 };
            let ring = commands
                .spawn((
                    Sprite::from_image(st.ring_img.clone()),
                    at(PLAY_X + pcx - 7.0, PLAY_Y + pcy - 6.0, 14.0, 12.0, 1.9),
                    PIXEL_LAYER,
                    RoomActor,
                ))
                .id();
            st.meteor = Some((pcx, pcy, 46, ring));
            sfx.write(crate::app::sfx::Sfx("tink"));
        }
    }

    // --- The drift (bared: it comes for you). ---
    if bared {
        let (dx, dy) = (pcx - scx, pcy - scy);
        let d = (dx * dx + dy * dy).sqrt().max(0.001);
        st.x = (st.x + dx / d * 0.7).clamp(12.0, PX_W as f32 - 32.0);
        st.y = (st.y + dy / d * 0.7).clamp(22.0, PX_H as f32 - 40.0);
        st.nova_cd -= 1;
        if st.nova_cd <= 0 {
            st.nova_cd = 160;
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU + (st.anim as f32) * 0.02;
                commands.spawn((
                    EBolt { x: scx - 4.0, y: scy - 4.0, vx: a.cos() * 2.0, vy: a.sin() * 2.0, life: 110 },
                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                    HitOnce::default(),
                    Hitbox { x: scx - 1.0, y: scy - 1.0, w: 7.0, h: 7.0 },
                    Sprite::from_image(art.bolt(STARLIGHT, 0xffffff)),
                    at(PLAY_X + scx - 3.0, PLAY_Y + scy - 3.0, 8.0, 8.0, 8.6),
                    PIXEL_LAYER,
                    RoomActor,
                ));
            }
        }
    } else {
        st.x = 142.0 + ((st.anim as f32) * 0.013).sin() * 30.0;
        st.y = 74.0 + ((st.anim as f32) * 0.026).sin() * 12.0;
    }

    // --- Sync. ---
    *hb = Hitbox { x: st.x + 3.0, y: st.y + 3.0, w: 14.0, h: 14.0 };
    let pulse = 1.0 + ((st.anim as f32) * 0.1).sin() * 0.04;
    *tf = at(PLAY_X + st.x, PLAY_Y + st.y, 20.0, 20.0, actor_z(st.y + 18.0));
    tf.scale = Vec3::new(pulse, pulse, 1.0);
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

/// Shattered shards snuff their own light; the broken star gives the vault its
/// torches back (room teardown rebuilds DungeonLights).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    stars: Query<(Entity, &HollowStar, &Health), Without<StarShard>>,
    shards: Query<(Entity, &StarShard, &Health), Without<HollowStar>>,
    beams: Query<Entity, With<StarBeam>>,
) {
    let Ok((se, st, sh)) = stars.single() else { return };
    for (e, shard, shh) in &shards {
        if shh.hp > 0 {
            continue;
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(shard.x + 5.0, shard.y + 5.0), STARLIGHT, 10);
        commands.entity(e).despawn();
        sfx.write(crate::app::sfx::Sfx("stone"));
    }
    if sh.hp <= 0 {
        for (e, ..) in &shards {
            commands.entity(e).despawn();
        }
        for e in &beams {
            commands.entity(e).despawn();
        }
        if let Some((.., ring)) = st.meteor {
            commands.entity(ring).despawn();
        }
        let (cx, cy) = (st.x + 10.0, st.y + 10.0);
        for i in 0..3 {
            let off = i as f32 * 8.0 - 8.0;
            spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.5), STARLIGHT, 14);
        }
        let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
        crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
        crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true, None);
        crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
        stats.bump("kills", 1.0);
        stats.bump_kill("boss");
        sfx.write(crate::app::sfx::Sfx("stone"));
        commands.entity(se).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids_are_rectangular() {
        let check = |name: &str, g: &[&str], w: usize| {
            for (i, r) in g.iter().enumerate() {
                assert_eq!(r.chars().count(), w, "{name} row {i} width");
            }
        };
        check("star", &STAR, 20);
        check("shard", &SHARD, 10);
        check("ring", &RING, 14);
    }
}
