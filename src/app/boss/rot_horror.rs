//! THE ROT HORROR — the festering thing of the drowned and blighted deeps (searuin /
//! tarpit / blightvault). It denies the floor: its signature is the SPREADING BLIGHT — it
//! hawks globs of filth that splatter into POISON POOLS you cannot stand in (they cling
//! poison + drag your boots), slowly carpeting the arena until there's nowhere clean to
//! fight from. Between globs it spits POISON BOLTS. It shambles, never rushes — the pools do
//! the cornering. Each third of its health lost thickens the spew.
//!
//! One of the six consolidated bespoke bosses (boss/mod.rs).

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Afflicts, Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 62.0; // js tarpit/blightvault pool (x HP_MUL)
const ROT: u32 = 0x8aa050;
const BILE: u32 = 0x6a8030;
const PAL: &[(char, u32)] = &[
    ('R', ROT),        // rot flesh
    ('r', BILE),       // deeper rot
    ('K', 0x2a3418),   // outline
    ('W', 0xc8e078),   // pustule highlight
    ('E', 0xd8ff60),   // eye bile
    ('d', 0x3a4a20),   // shadow
];

const HORROR: [&str; 20] = [
    "................",
    "....KKKKKK......",
    "..KKRRRRRRKK....",
    ".KRRWRRRRWRRK...",
    ".KRRRREERRRRK...",
    "KRRWRRRRRRWRRK..",
    "KRRRRrRRrRRRRK..",
    "KRWRRRRRRRRWRK..",
    "KRRRrRRRRrRRRK..",
    "KRRRRRWWRRRRRK..",
    ".KRRrRRRRrRRK...",
    ".KRRRRRRRRRRK...",
    "..KRRWRRRWRRK...",
    "..KdRRRRRRdK....",
    "...KdRRRRdK.....",
    "....KdddK.......",
    "...KrK..KrK.....",
    "..KrrK..KrrK....",
    "................",
    "................",
];

const POOL: [&str; 10] = [
    "................",
    "...rRRRRRRr.....",
    "..rRRWRRRRRr....",
    ".rRRRRRRWRRr....",
    ".rRWRRRRRRRr....",
    ".rRRRRWRRRRr....",
    "..rRRRRRRRr.....",
    "...rRRWRRr......",
    "................",
    "................",
];

#[derive(Component)]
pub struct RotHorror {
    x: f32,
    y: f32,
    anim: u32,
    phase: u8,
    spit_cd: i32,
    volley_cd: i32,
    pool_img: Handle<Image>,
}

/// A poison pool the horror spat — clings poison + slow to boots that stand in it, fades slowly.
#[derive(Component)]
pub struct RotPool {
    x: f32,
    y: f32,
    t: i32,
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let img = images.add(crate::gfx::bake(&HORROR, PAL));
    let pool_img = images.add(crate::gfx::bake(&POOL, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (bx, by) = (128.0, 44.0);
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + bx, PLAY_Y + by, 16.0, 20.0, actor_z(by + 20.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE ROT HORROR"),
        crate::app::dungeon::DungeonBoss,
        RotHorror { x: bx, y: by, anim: 0, phase: 0, spit_cd: 90, volley_cd: 130, pool_img },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(3), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.0 * (1.0 - 0.6), kb_frames: 10 },
        Knockback::default(),
        Hitbox { x: bx + 2.0, y: by + 4.0, w: 12.0, h: 14.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut statuses: ResMut<crate::app::status::Statuses>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<(&Player, &Hitbox), Without<RotHorror>>,
    mut horrors: Query<(&mut RotHorror, &mut Health, &mut Hitbox, &mut Transform, &mut Visibility), (Without<Player>, Without<RotPool>)>,
    mut pools: Query<(Entity, &mut RotPool, &mut Sprite), (Without<RotHorror>, Without<Player>)>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok((p, phb)) = players.single() else { return };
    let Ok((mut b, mut h, mut hb, mut tf, mut vis)) = horrors.single_mut() else { return };
    b.anim += 1;
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let (bcx, bcy) = (b.x + 8.0, b.y + 10.0);

    let frac = h.hp as f32 / h.max.max(1) as f32;
    let ph = if frac > 0.66 { 0 } else if frac > 0.33 { 1 } else { 2 };
    if ph > b.phase {
        b.phase = ph;
        h.flash = 16;
        h.invuln = h.invuln.max(18);
        b.spit_cd = 20;
        sfx.write(crate::app::sfx::Sfx("tink"));
    }
    let tempo = 1.0 + b.phase as f32 * 0.22;

    // --- Shamble (slow — the pools do the cornering). ---
    let s = 0.34 * tempo;
    b.x = (b.x + (pcx - bcx).signum() * s).clamp(6.0, PX_W as f32 - 22.0);
    b.y = (b.y + (pcy - bcy).signum() * s * 0.8).clamp(16.0, PX_H as f32 - 28.0);

    // --- SPREADING BLIGHT (signature): globs land as lingering poison pools. ---
    b.spit_cd -= 1;
    if b.spit_cd <= 0 {
        b.spit_cd = (150.0 / tempo) as i32;
        let n = 1 + b.phase as i32;
        for i in 0..=n {
            let (sx, sy) = if i == 0 {
                (pcx - 8.0, pcy - 5.0)
            } else {
                let a = b.anim as f32 * 1.1 + i as f32 * 2.0;
                ((pcx + a.cos() * 30.0 - 8.0).clamp(2.0, PX_W as f32 - 18.0), (pcy + a.sin() * 30.0 - 5.0).clamp(2.0, PX_H as f32 - 12.0))
            };
            commands.spawn((
                Sprite::from_image(b.pool_img.clone()),
                at(PLAY_X + sx, PLAY_Y + sy, 16.0, 10.0, 1.7),
                PIXEL_LAYER,
                RoomActor,
                RotPool { x: sx, y: sy, t: 520 },
            ));
        }
        sfx.write(crate::app::sfx::Sfx("splash"));
    }

    // --- POISON BOLTS: a slow gob that poisons on the touch. ---
    b.volley_cd -= 1;
    if b.volley_cd <= 0 {
        b.volley_cd = (130.0 / tempo) as i32;
        let base = (pcy - bcy).atan2(pcx - bcx);
        let n = 2 + b.phase as i32;
        for i in 0..n {
            let a = base + 0.3 * (i as f32 - (n - 1) as f32 / 2.0);
            let e = commands
                .spawn((
                    EBolt { x: bcx - 4.0, y: bcy, vx: a.cos() * 1.9, vy: a.sin() * 1.9, life: 150 },
                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                    HitOnce::default(),
                    Hitbox { x: bcx - 1.0, y: bcy + 3.0, w: 7.0, h: 7.0 },
                    Sprite::from_image(art.bolt(ROT, 0xd8ff60)),
                    at(PLAY_X + bcx - 5.0, PLAY_Y + bcy + 1.0, 8.0, 8.0, 8.6),
                    PIXEL_LAYER,
                    RoomActor,
                ))
                .id();
            commands.entity(e).insert(Afflicts("poison", 180));
        }
        sfx.write(crate::app::sfx::Sfx("tink"));
    }

    // --- Pools: cling poison + slow to whoever stands in the muck; fade over time. ---
    for (e, mut pool, mut pspr) in &mut pools {
        pool.t -= 1;
        if pool.t <= 0 {
            commands.entity(e).despawn();
            continue;
        }
        pspr.color = Color::srgba(1.0, 1.0, 1.0, (pool.t as f32 / 90.0).min(0.85));
        if overlap((phb.x, phb.y, phb.w, phb.h), (pool.x + 1.0, pool.y + 1.0, 14.0, 8.0)) {
            statuses.add("slow", 16);
            statuses.add("poison", 30);
        }
    }

    // --- Sync. ---
    *hb = Hitbox { x: b.x + 2.0, y: b.y + 4.0, w: 12.0, h: 14.0 };
    let bob = ((b.anim as f32) * 0.14).sin() * 1.2;
    *tf = at(PLAY_X + b.x, PLAY_Y + b.y + bob, 16.0, 20.0, actor_z(b.y + 20.0));
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

fn overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

/// The horror bursts; its pools dry up; the arena banks the reward.
#[allow(clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    horrors: Query<(Entity, &RotHorror, &Health)>,
    pools: Query<Entity, With<RotPool>>,
) {
    let Ok((e, b, h)) = horrors.single() else { return };
    if h.hp > 0 {
        return;
    }
    for pe in &pools {
        commands.entity(pe).despawn();
    }
    let (cx, cy) = (b.x + 8.0, b.y + 10.0);
    for i in 0..3 {
        let off = i as f32 * 7.0 - 7.0;
        spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.4), ROT, 12);
    }
    let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
    crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
    crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true);
    crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
    stats.bump("kills", 1.0);
    stats.bump_kill("boss");
    sfx.write(crate::app::sfx::Sfx("splash"));
    commands.entity(e).despawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids_rectangular() {
        for (i, r) in HORROR.iter().enumerate() {
            assert_eq!(r.chars().count(), 16, "horror row {i}");
        }
        for (i, r) in POOL.iter().enumerate() {
            assert_eq!(r.chars().count(), 16, "pool row {i}");
        }
    }
}
