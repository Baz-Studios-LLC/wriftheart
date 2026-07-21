//! THE HIVE QUEEN — boss 3 of THE TEN (BOSSES.md): the Hive Hollow's guardian.
//!
//! She hovers untouchable while her BROOD COMBS live: four wax clusters on the
//! walls, each hatching wasp drones to defend her. Smash the combs — each loss
//! quickens her temper. Bare of brood, she is finally soft, but her ROYAL GUARD
//! remains: drones orbiting her in a ring with ONE rotating gap; your blows land
//! only through it (or thin the ring the hard way — guards die and stay dead).
//! All the while she dive-bombs, spits stinger fans, and drools HONEY SLICKS that
//! bog your boots (the shared Slowed rig).

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 40.0; // the js hivehollow pool (x HP_MUL)
const COMB_HP: i32 = 10;
const GUARD_HP: i32 = 4;
const GOLD: u32 = 0xf0b840;
const AMBER: u32 = 0xd89020;
const PAL: &[(char, u32)] = &[
    ('P', GOLD),     // body gold
    ('p', AMBER),    // shade
    ('A', 0x2a2320), // black bands
    ('W', 0xf8f4ff), // wing white
    ('E', 0xff5060), // queen's eyes
    ('C', 0xe8c060), // comb wax
    ('c', 0xb08830), // comb shade
    ('H', 0xffd870), // honey bright
    ('D', 0x503818), // comb dark cells
];

// --- Art ---
const QUEEN: [&str; 22] = [
    ".....W......W.....",
    "....WWW....WWW....",
    "...WWWWW..WWWWW...",
    "..WWWWWWKKWWWWWW..",
    "..WWWWKKPPKKWWWW..",
    "...WWKPPPPPPKWW...",
    "..KKKPPEPPEPPKKK..",
    ".KPPPPPEPPEPPPPPK.",
    ".KPpPPPPPPPPPPpPK.",
    "..KKAAAAAAAAAAKK..",
    ".KPPPPPPPPPPPPPPK.",
    ".KPpPPPPPPPPPPpPK.",
    "..KKAAAAAAAAAAKK..",
    ".KPPPPPPPPPPPPPPK.",
    "..KPpPPPPPPPPpPK..",
    "...KKAAAAAAAAKK...",
    "....KPPPPPPPPK....",
    ".....KPpPPpPK.....",
    "......KPPPPK......",
    ".......KPPK.......",
    "........KK........",
    "........K.........",
];
const COMB: [&str; 16] = [
    "...KKKKKKKKKK...",
    "..KCCcCCCCcCCK..",
    ".KCcDDcCCcDDcCK.",
    ".KCDDDDcCDDDDCK.",
    "KCcDDDDcCDDDDcCK",
    "KCCcDDcCCcDDcCCK",
    "KCcCCCCcCCCCCcCK",
    "KCDDcCCcCDDcCCCK",
    "KCDDDCcCCDDDcCCK",
    "KCcDDcCCcDDDCcCK",
    ".KCcCCcCCcDDcCK.",
    ".KCCcCCCCcCCCCK.",
    "..KCcCCcCCcCCK..",
    "...KCCCCCCCCK...",
    "....KKKKKKKK....",
    "................",
];
const GUARD: [&str; 10] = [
    "..W....W..",
    ".WWW..WWW.",
    "..KKPPKK..",
    ".KPPEPPPK.",
    ".KPpPPpPK.",
    "..KAAAAK..",
    ".KPPPPPPK.",
    "..KAAAAK..",
    "...KPPK...",
    "....KK....",
];
const HONEY: [&str; 10] = [
    "................",
    "....HHHHHHH.....",
    "..HHHHHHHHHHH...",
    ".HHHpHHHHHpHHH..",
    "HHHHHHHHHHHHHHH.",
    "HHpHHHHHpHHHHHH.",
    ".HHHHHHHHHHHHH..",
    "..HHHpHHHHHHH...",
    "....HHHHHHH.....",
    "................",
];

/// The four brood-comb wall spots (room px, sprite top-left).
const COMBS: [(f32, f32); 4] = [(40.0, 36.0), (248.0, 36.0), (40.0, 140.0), (248.0, 140.0)];
const GUARDS: usize = 4; // ring slots — slot 0 stays EMPTY (the rotating gap)

#[derive(Component)]
pub struct HiveQueen {
    x: f32,
    y: f32,
    anim: u32,
    combs_left: u8,
    /// Dive state: None = hover; Some((vx, vy, t, returning)).
    dive: Option<(f32, f32, i32, bool)>,
    dive_cd: i32,
    spit_cd: i32,
    honey_cd: i32,
    ring_angle: f32,
    guards: [Option<Entity>; GUARDS],
    honey_img: Handle<Image>,
}

#[derive(Component)]
pub struct BroodComb {
    idx: usize,
    hatch_cd: i32,
}

#[derive(Component)]
pub struct RoyalGuard {
    slot: usize,
}

/// A drone hatched from a comb (a REAL wasp mob wearing a marker so the deaths
/// system can cap the swarm; deliberately no DungeonFoe — hatches never bank).
#[derive(Component)]
pub struct HiveDrone;

#[derive(Component)]
pub struct HoneyPool {
    x: f32,
    y: f32,
    t: i32,
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let queen_img = images.add(crate::gfx::bake(&QUEEN, PAL));
    let comb_img = images.add(crate::gfx::bake(&COMB, PAL));
    let guard_img = images.add(crate::gfx::bake(&GUARD, PAL));
    let honey_img = images.add(crate::gfx::bake(&HONEY, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (qx, qy) = (143.0, 60.0);
    for (i, (cx, cy)) in COMBS.iter().enumerate() {
        commands.spawn((
            Sprite::from_image(comb_img.clone()),
            at(PLAY_X + cx, PLAY_Y + cy, 16.0, 16.0, actor_z(cy + 14.0)),
            PIXEL_LAYER,
            RoomActor,
            BroodComb { idx: i, hatch_cd: 140 + i as i32 * 45 },
            Combatant { team: Team::Enemy, hurt_team: None, damage: None, persistent: false, knock: 0.0 },
            Health { hp: COMB_HP, max: COMB_HP, defense: 0, invuln: 0, flash: 0 },
            HurtProfile { invuln: 8, flash: 6, kb_base: 0.0, kb_frames: 0 },
            Knockback::default(),
            Hitbox { x: *cx + 1.0, y: *cy + 1.0, w: 14.0, h: 14.0 },
        ));
    }
    let mut guards: [Option<Entity>; GUARDS] = [None; GUARDS];
    for (slot, g) in guards.iter_mut().enumerate().skip(1) {
        // slot 0 stays empty: the gap
        *g = Some(
            commands
                .spawn((
                    Sprite::from_image(guard_img.clone()),
                    at(PLAY_X + qx, PLAY_Y + qy, 10.0, 10.0, actor_z(qy + 30.0)),
                    PIXEL_LAYER,
                    RoomActor,
                    RoyalGuard { slot },
                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: true, knock: 0.0 },
                    Health { hp: GUARD_HP, max: GUARD_HP, defense: 0, invuln: 0, flash: 0 },
                    HurtProfile { invuln: 10, flash: 8, kb_base: 0.0, kb_frames: 0 },
                    Knockback::default(),
                    Hitbox { x: qx, y: qy, w: 10.0, h: 10.0 },
                ))
                .id(),
        );
    }
    commands.spawn((
        Sprite::from_image(queen_img),
        at(PLAY_X + qx, PLAY_Y + qy, 18.0, 22.0, actor_z(qy + 20.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE HIVE QUEEN"),
        crate::app::dungeon::DungeonBoss,
        HiveQueen {
            x: qx,
            y: qy,
            anim: 0,
            combs_left: 4,
            dive: None,
            dive_cd: 120,
            spit_cd: 90,
            honey_cd: 200,
            ring_angle: 0.0,
            guards,
            honey_img,
        },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2 * (1.0 - 0.85), kb_frames: 11 },
        Knockback::default(),
        Hitbox { x: qx + 2.0, y: qy + 4.0, w: 14.0, h: 14.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut statuses: ResMut<crate::app::status::Statuses>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<(&Player, &Hitbox), Without<HiveQueen>>,
    mut queens: Query<
        (&mut HiveQueen, &mut Health, &mut Hitbox, &mut Sprite, &mut Transform, &mut Visibility),
        (Without<Player>, Without<RoyalGuard>, Without<BroodComb>, Without<HoneyPool>),
    >,
    mut combs: Query<
        (&mut BroodComb, &mut Visibility, &Health),
        (Without<HiveQueen>, Without<RoyalGuard>, Without<HoneyPool>, Without<Player>),
    >,
    mut guards: Query<
        (&RoyalGuard, &mut Hitbox, &mut Transform, &mut Visibility, &Health),
        (Without<HiveQueen>, Without<BroodComb>, Without<HoneyPool>, Without<Player>),
    >,
    mut pools: Query<(Entity, &mut HoneyPool, &mut Sprite), (Without<HiveQueen>, Without<BroodComb>, Without<RoyalGuard>)>,
    drones: Query<(), With<HiveDrone>>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok((p, phb)) = players.single() else { return };
    let Ok((mut q, mut h, mut hb, _spr, mut tf, mut vis)) = queens.single_mut() else { return };
    q.anim += 1;
    let armored = q.combs_left > 0;
    if armored {
        h.invuln = h.invuln.max(2); // untouchable while her brood lives
    }
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let (qcx, qcy) = (q.x + 9.0, q.y + 11.0);
    let tempo = 1.0 + (4 - q.combs_left) as f32 * 0.18; // each lost comb quickens her

    // --- Movement: lazy figure-eight hover; dive-bombs on a timer. ---
    if let Some((vx, vy, mut t, returning)) = q.dive {
        t -= 1;
        q.x = (q.x + vx).clamp(8.0, PX_W as f32 - 26.0);
        q.y = (q.y + vy).clamp(20.0, PX_H as f32 - 30.0);
        if t <= 0 {
            if returning {
                q.dive = None;
                q.dive_cd = (150.0 / tempo) as i32;
            } else {
                // Swoop done — climb back to the hover band.
                let back = ((60.0 - q.y) * 0.06).clamp(-1.6, 1.6);
                q.dive = Some((0.0, back.min(-0.8), 26, true));
            }
        } else {
            q.dive = Some((vx, vy, t, returning));
        }
    } else {
        q.x += ((q.anim as f32) * 0.02).sin() * 0.8;
        q.y = 52.0 + ((q.anim as f32) * 0.04).sin() * 8.0;
        q.dive_cd -= 1;
        if q.dive_cd <= 0 {
            // Telegraphed dive: she rears (flash) then bombs THROUGH your position.
            let d = ((pcx - qcx).powi(2) + (pcy - qcy).powi(2)).sqrt().max(0.001);
            let sp = 2.6 * tempo;
            q.dive = Some(((pcx - qcx) / d * sp, (pcy - qcy) / d * sp, (d / sp) as i32 + 6, false));
            h.flash = 6;
            sfx.write(crate::app::sfx::Sfx("tink"));
        }
    }

    // --- Stinger fan + honey drip. ---
    q.spit_cd -= 1;
    if q.spit_cd <= 0 && q.dive.is_none() {
        q.spit_cd = (110.0 / tempo) as i32;
        let base = (pcy - qcy).atan2(pcx - qcx);
        for i in -1..=1i32 {
            let a = base + i as f32 * 0.3;
            commands.spawn((
                EBolt { x: qcx - 4.0, y: qcy, vx: a.cos() * 2.3, vy: a.sin() * 2.3, life: 120 },
                Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                HitOnce::default(),
                Hitbox { x: qcx - 1.0, y: qcy + 3.0, w: 7.0, h: 7.0 },
                Sprite::from_image(art.bolt(GOLD, 0xfff0c0)),
                at(PLAY_X + qcx - 3.0, PLAY_Y + qcy + 1.0, 8.0, 8.0, 8.6),
                PIXEL_LAYER,
                RoomActor,
            ));
        }
    }
    q.honey_cd -= 1;
    if q.honey_cd <= 0 {
        // A gob of honey splats where you STAND — the floor remembers for a while.
        q.honey_cd = (240.0 / tempo) as i32;
        commands.spawn((
            Sprite::from_image(q.honey_img.clone()),
            at(PLAY_X + pcx - 8.0, PLAY_Y + pcy - 5.0, 16.0, 10.0, 1.7),
            PIXEL_LAYER,
            RoomActor,
            HoneyPool { x: pcx - 8.0, y: pcy - 5.0, t: 480 },
        ));
        sfx.write(crate::app::sfx::Sfx("tink"));
    }

    // --- Honey pools: bog the boots that stand in them. ---
    for (e, mut pool, mut pspr) in &mut pools {
        pool.t -= 1;
        if pool.t <= 0 {
            commands.entity(e).despawn();
            continue;
        }
        let alpha = (pool.t as f32 / 120.0).min(1.0) * 0.9;
        pspr.color = Color::srgba(1.0, 1.0, 1.0, alpha);
        if overlap((phb.x, phb.y, phb.w, phb.h), (pool.x, pool.y, 16.0, 10.0)) {
            statuses.add("slow", 20); // honey clings a beat past the last sticky step
        }
    }

    // --- Brood combs: pulse and hatch (drone cap 3 across the hive). ---
    let drone_count = drones.iter().count();
    for (mut comb, mut cvis, chealth) in &mut combs {
        comb.hatch_cd -= 1;
        *cvis = if chealth.flash > 0 && (chealth.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
        if comb.hatch_cd <= 0 {
            comb.hatch_cd = 260 + comb.idx as i32 * 30;
            if drone_count < 3
                && let Some(idx) = crate::actors::mobs::def_index("wasp")
            {
                let (cx, cy) = (COMBS[comb.idx].0 + 2.0, COMBS[comb.idx].1 + 4.0);
                commands.spawn((
                    crate::actors::mobs::mob_bundle(idx, cx, cy),
                    RoomActor,
                    PIXEL_LAYER,
                    HiveDrone,
                ));
                sfx.write(crate::app::sfx::Sfx("tink"));
            }
        }
    }

    // --- The royal guard: an orbit with one gap, quickening as combs fall. ---
    q.ring_angle += 0.018 * tempo;
    for (g, mut ghb, mut gtf, mut gvis, ghealth) in &mut guards {
        let a = q.ring_angle + g.slot as f32 / GUARDS as f32 * std::f32::consts::TAU;
        let (gx, gy) = (qcx + a.cos() * 22.0 - 5.0, qcy + a.sin() * 22.0 - 5.0);
        *ghb = Hitbox { x: gx, y: gy, w: 10.0, h: 10.0 };
        *gtf = at(PLAY_X + gx, PLAY_Y + gy, 10.0, 10.0, actor_z(gy + 10.0));
        *gvis = if ghealth.flash > 0 && (ghealth.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
    }

    // --- Sync the queen. ---
    *hb = Hitbox { x: q.x + 2.0, y: q.y + 4.0, w: 14.0, h: 14.0 };
    let bob = ((q.anim as f32) * 0.2).sin() * 1.5;
    *tf = at(PLAY_X + q.x, PLAY_Y + q.y + bob, 18.0, 22.0, actor_z(q.y + 20.0));
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

fn overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

/// Smashed combs bare the queen; fallen guards thin the ring for good; the fallen
/// queen scatters her court.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    mut queens: Query<(Entity, &mut HiveQueen, &Health), (Without<BroodComb>, Without<RoyalGuard>)>,
    combs: Query<(Entity, &BroodComb, &Health), Without<HiveQueen>>,
    guards: Query<(Entity, &RoyalGuard, &Hitbox, &Health), Without<HiveQueen>>,
    pools: Query<Entity, With<HoneyPool>>,
) {
    let Ok((qe, mut q, qh)) = queens.single_mut() else { return };
    for (e, comb, ch) in &combs {
        if ch.hp > 0 {
            continue;
        }
        let (cx, cy) = (COMBS[comb.idx].0 + 8.0, COMBS[comb.idx].1 + 8.0);
        spawn_burst(&mut commands, &mut rng, Vec2::new(cx, cy), 0xe8c060, 12);
        commands.entity(e).despawn();
        q.combs_left = q.combs_left.saturating_sub(1);
        sfx.write(crate::app::sfx::Sfx("stone"));
    }
    for (e, g, ghb, gh) in &guards {
        if gh.hp > 0 {
            continue;
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(ghb.x + 5.0, ghb.y + 5.0), GOLD, 8);
        if let Some(slot) = q.guards.get_mut(g.slot) {
            *slot = None;
        }
        commands.entity(e).despawn();
    }
    if qh.hp <= 0 {
        for (e, ..) in &combs {
            commands.entity(e).despawn();
        }
        for (e, ..) in &guards {
            commands.entity(e).despawn();
        }
        for e in &pools {
            commands.entity(e).despawn();
        }
        let (cx, cy) = (q.x + 9.0, q.y + 11.0);
        for i in 0..3 {
            let off = i as f32 * 7.0 - 7.0;
            spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.5), GOLD, 12);
        }
        let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
        crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
        crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true, None);
        crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
        stats.bump("kills", 1.0);
        stats.bump_kill("boss");
        sfx.write(crate::app::sfx::Sfx("stone"));
        commands.entity(qe).despawn();
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
        check("queen", &QUEEN, 18);
        check("comb", &COMB, 16);
        check("guard", &GUARD, 10);
        check("honey", &HONEY, 16);
    }
}
