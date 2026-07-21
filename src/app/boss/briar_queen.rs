//! THE BRIAR QUEEN — boss 6 of THE TEN (BOSSES.md): the Petal Hall's guardian.
//!
//! A rose-monarch rooted at the hall's heart, untouchable while she BLOOMS — and
//! blooming, she fills the air with slow spirals of razor petals while THORN
//! HEDGES grow and reshape the arena around you (smashable, and they wilt on
//! their own — the maze never locks). Her tell: she must DRAW from her ROOTS.
//! Gnarled root-loops surface one by one across the floor; smash one and the
//! bloom is INTERRUPTED — she wilts, soft, for a long window. Ignore them and
//! they retract fed, and the spiral only thickens.

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};

const HP: f64 = 46.0;
const PINK: u32 = 0xff9ad0;
const PAL: &[(char, u32)] = &[
    ('R', PINK),     // petal pink
    ('r', 0xd0609a), // petal deep
    ('G', 0x3a7a3a), // stem green
    ('g', 0x5aa04a), // leaf light
    ('T', 0x7a5c30), // thorn brown
    ('E', 0xffe080), // face glow
    ('W', 0xffffff),
];

const QUEEN_BLOOM: [&str; 24] = [
    "....R....RR....R....",
    "..RRrR..RrrR..RrRR..",
    ".RrRRrRRrRRrRRrRRr..",
    ".RrRRrrRRRRrrRRrRr..",
    "..RRrRRRRRRRRrRRr...",
    ".RrRRWWRRRRWWRRrRR..",
    ".RrRWEEWRRWEEWRrRR..",
    ".RrRRWWRRRRWWRRrRR..",
    "..RrRRRWWWWRRRrRR...",
    "..RRrRRRRRRRRrRR....",
    "...RRrrRRRRrrRR.....",
    "....KGGgGGgGGK......",
    ".....KGgGGgGK.......",
    "....gKGGTGGKg.......",
    "...gGKGgGGgKGg......",
    "....KGGTGGTGGK......",
    ".....KGgGGgGK.......",
    "....TKGGgGGKT.......",
    ".....KGgGGgK........",
    "....gKGGTGGKg.......",
    ".....KGgGGgK........",
    "......KGGGK.........",
    ".......KKK..........",
    "....................",
];
const QUEEN_WILT: [&str; 24] = [
    "....................",
    "....................",
    "..r....rr.....r.....",
    ".rrr..rrrr...rrr....",
    "..rrr.rrrrr.rrr.....",
    "...rrrWWrrWWrrr.....",
    "...rrWEEWWEEWrr.....",
    "...rrrWWrrWWrrr.....",
    "....rrrrWWrrrr......",
    ".....rrrrrrrr.......",
    "......rrrrrr........",
    "....KGGgGGgGGK......",
    ".....KGgGGgGK.......",
    "....gKGGTGGKg.......",
    "...gGKGgGGgKGg......",
    "....KGGTGGTGGK......",
    ".....KGgGGgGK.......",
    "....TKGGgGGKT.......",
    ".....KGgGGgK........",
    "....gKGGTGGKg.......",
    ".....KGgGGgK........",
    "......KGGGK.........",
    ".......KKK..........",
    "....................",
];
const ROOT: [&str; 10] = [
    "....KKKK....",
    "..KKTgTKK...",
    ".KTgTRRTgK..",
    ".KgTRrRTTK..",
    "KTgRrrrRgTK.",
    "KgTRrrrRTgK.",
    ".KTgRRRTgK..",
    ".KKTgTgTKK..",
    "...KKTKK....",
    "............",
];
const HEDGE: [&str; 14] = [
    "..K..KK..K......",
    ".KTK.KTK.KTK....",
    "KTgTKTgTKTgTK...",
    "KgGgTgGgTgGgKK..",
    "KTGGgGGGgGGTgK..",
    ".KgGGgTGGgGGTK..",
    "KTgGGGgGGGgGgK..",
    "KgGTgGGgTGGgTK..",
    "KTgGGgGGGgGGgK..",
    ".KgGgTgGgTgGK...",
    "KTgGGgGGgGGTK...",
    ".KTgTKTgTKTgK...",
    "..KK..KK..KK....",
    "................",
];

/// Hedge growth spots — a loose lattice clear of doors and the queen's court.
const HEDGE_SPOTS: [(f32, f32); 8] =
    [(64.0, 64.0), (208.0, 64.0), (96.0, 112.0), (176.0, 112.0), (56.0, 152.0), (216.0, 152.0), (136.0, 132.0), (136.0, 56.0)];

#[derive(Component)]
pub struct BriarQueen {
    x: f32,
    y: f32,
    anim: u32,
    /// Some(frames left) = wilted (soft); None = blooming (untouchable).
    wilt: Option<i32>,
    spiral: f32,
    spiral_cd: i32,
    root_cd: i32,
    hedge_cd: i32,
    hedge_slots: [bool; 8],
    bloom_img: Handle<Image>,
    wilt_img: Handle<Image>,
    root_img: Handle<Image>,
    hedge_img: Handle<Image>,
}

#[derive(Component)]
pub struct BriarRoot {
    t: i32,
    x: f32,
    y: f32,
}

#[derive(Component)]
pub struct ThornHedge {
    slot: usize,
    t: i32,
    blocker: (f32, f32, f32, f32),
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let bloom_img = images.add(crate::gfx::bake(&QUEEN_BLOOM, PAL));
    let wilt_img = images.add(crate::gfx::bake(&QUEEN_WILT, PAL));
    let root_img = images.add(crate::gfx::bake(&ROOT, PAL));
    let hedge_img = images.add(crate::gfx::bake(&HEDGE, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (qx, qy) = (142.0, 44.0);
    commands.spawn((
        Sprite::from_image(bloom_img.clone()),
        at(PLAY_X + qx, PLAY_Y + qy, 20.0, 24.0, actor_z(qy + 22.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE BRIAR QUEEN"),
        crate::app::dungeon::DungeonBoss,
        BriarQueen {
            x: qx,
            y: qy,
            anim: 0,
            wilt: None,
            spiral: 0.0,
            spiral_cd: 60,
            root_cd: 120,
            hedge_cd: 90,
            hedge_slots: [false; 8],
            bloom_img,
            wilt_img,
            root_img,
            hedge_img,
        },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 0.0, kb_frames: 0 }, // rooted
        Knockback::default(),
        Hitbox { x: qx + 3.0, y: qy + 2.0, w: 14.0, h: 12.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut rng: ResMut<GameRng>,
    mut blockers: ResMut<crate::app::room_props::RoomBlockers>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player>,
    mut queens: Query<
        (&mut BriarQueen, &mut Health, &mut Sprite, &mut Transform, &mut Visibility),
        (Without<BriarRoot>, Without<ThornHedge>, Without<Player>),
    >,
    mut roots: Query<(Entity, &mut BriarRoot, &mut Sprite), (Without<BriarQueen>, Without<ThornHedge>)>,
    mut hedges: Query<(Entity, &mut ThornHedge, &mut Sprite), (Without<BriarQueen>, Without<BriarRoot>)>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut q, mut h, mut spr, mut tf, mut vis)) = queens.single_mut() else { return };
    q.anim += 1;
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let (qcx, qcy) = (q.x + 10.0, q.y + 8.0);

    match q.wilt {
        None => {
            h.invuln = h.invuln.max(2); // in full bloom nothing touches her
            // --- The petal spiral: two arms, forever turning. ---
            q.spiral += 0.19;
            q.spiral_cd -= 1;
            if q.spiral_cd <= 0 {
                q.spiral_cd = 9;
                for arm in 0..2 {
                    let a = q.spiral + arm as f32 * std::f32::consts::PI;
                    commands.spawn((
                        EBolt { x: qcx - 4.0, y: qcy - 4.0, vx: a.cos() * 1.6, vy: a.sin() * 1.6, life: 160 },
                        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                        HitOnce::default(),
                        Hitbox { x: qcx - 1.0, y: qcy - 1.0, w: 7.0, h: 7.0 },
                        Sprite::from_image(art.bolt(PINK, 0xffe0f0)),
                        at(PLAY_X + qcx - 3.0, PLAY_Y + qcy - 3.0, 8.0, 8.0, 8.6),
                        PIXEL_LAYER,
                        RoomActor,
                    ));
                }
            }
            // --- Roots surface (cap 3): each is the bloom's throat, bared. ---
            q.root_cd -= 1;
            if q.root_cd <= 0 && roots.iter().count() < 3 {
                q.root_cd = 180;
                let rx = 48.0 + rng.0.next_f64() as f32 * 190.0;
                let ry = 70.0 + rng.0.next_f64() as f32 * 110.0;
                commands.spawn((
                    Sprite::from_image(q.root_img.clone()),
                    at(PLAY_X + rx, PLAY_Y + ry, 12.0, 10.0, actor_z(ry + 8.0)),
                    PIXEL_LAYER,
                    RoomActor,
                    BriarRoot { t: 220, x: rx, y: ry },
                    Combatant { team: Team::Enemy, hurt_team: None, damage: None, persistent: false, knock: 0.0 },
                    Health { hp: 6, max: 6, defense: 0, invuln: 10, flash: 0 },
                    HurtProfile { invuln: 6, flash: 6, kb_base: 0.0, kb_frames: 0 },
                    Knockback::default(),
                    Hitbox { x: rx + 1.0, y: ry + 1.0, w: 10.0, h: 8.0 },
                ));
                spawn_burst(&mut commands, &mut rng, Vec2::new(rx + 6.0, ry + 5.0), 0x5aa04a, 6);
            }
            // --- Hedges grow and reshape the hall (cap 4 alive; they wilt on their own). ---
            q.hedge_cd -= 1;
            if q.hedge_cd <= 0 && hedges.iter().count() < 4 {
                q.hedge_cd = 260;
                let free: Vec<usize> = (0..8).filter(|&i| !q.hedge_slots[i]).collect();
                if !free.is_empty() {
                    let slot = free[(rng.0.next_f64() * free.len() as f64) as usize % free.len()];
                    let (hx, hy) = HEDGE_SPOTS[slot];
                    // Never grow a wall onto the hero's boots.
                    if ((pcx - hx - 8.0).abs() > 20.0 || (pcy - hy - 7.0).abs() > 18.0)
                        && let blocker = (hx + 1.0, hy + 3.0, 13.0, 9.0)
                    {
                        q.hedge_slots[slot] = true;
                        blockers.0.push(blocker);
                        commands.spawn((
                            Sprite::from_image(q.hedge_img.clone()),
                            at(PLAY_X + hx, PLAY_Y + hy, 16.0, 14.0, actor_z(hy + 12.0)),
                            PIXEL_LAYER,
                            RoomActor,
                            ThornHedge { slot, t: 720, blocker },
                            Combatant { team: Team::Enemy, hurt_team: None, damage: None, persistent: false, knock: 0.0 },
                            Health { hp: 6, max: 6, defense: 0, invuln: 0, flash: 0 },
                            HurtProfile { invuln: 6, flash: 6, kb_base: 0.0, kb_frames: 0 },
                            Knockback::default(),
                            Hitbox { x: hx + 1.0, y: hy + 2.0, w: 13.0, h: 11.0 },
                        ));
                        sfx.write(crate::app::sfx::Sfx("tink"));
                    }
                }
            }
        }
        Some(mut t) => {
            // Wilted: soft, spent — a thin drizzle is all she has.
            t -= 1;
            if q.anim.is_multiple_of(45) {
                let base = (pcy - qcy).atan2(pcx - qcx);
                commands.spawn((
                    EBolt { x: qcx - 4.0, y: qcy - 4.0, vx: base.cos() * 1.8, vy: base.sin() * 1.8, life: 130 },
                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(1), persistent: false, knock: 0.0 },
                    HitOnce::default(),
                    Hitbox { x: qcx - 1.0, y: qcy - 1.0, w: 7.0, h: 7.0 },
                    Sprite::from_image(art.bolt(0xd0609a, 0xffe0f0)),
                    at(PLAY_X + qcx - 3.0, PLAY_Y + qcy - 3.0, 8.0, 8.0, 8.6),
                    PIXEL_LAYER,
                    RoomActor,
                ));
            }
            if t <= 0 {
                q.wilt = None;
                spr.image = q.bloom_img.clone();
                h.flash = 8;
                sfx.write(crate::app::sfx::Sfx("tink"));
            } else {
                q.wilt = Some(t);
            }
        }
    }

    // --- Roots retract fed; hedges wilt on their own. ---
    for (e, mut root, mut rspr) in &mut roots {
        root.t -= 1;
        rspr.color = Color::srgba(1.0, 1.0, 1.0, 0.65 + 0.35 * ((root.t as f32) * 0.2).sin());
        if root.t <= 0 {
            commands.entity(e).despawn();
        }
    }
    for (e, mut hedge, mut hspr) in &mut hedges {
        hedge.t -= 1;
        if hedge.t < 90 {
            hspr.color = Color::srgba(1.0, 1.0, 1.0, hedge.t as f32 / 90.0);
        }
        if hedge.t <= 0 {
            blockers.0.retain(|r| *r != hedge.blocker);
            q.hedge_slots[hedge.slot] = false;
            commands.entity(e).despawn();
        }
    }

    // --- Sync: the bloom sways; wilted she droops. ---
    let sway = ((q.anim as f32) * 0.06).sin() * 1.2;
    let droop = if q.wilt.is_some() { 2.0 } else { 0.0 };
    *tf = at(PLAY_X + q.x + sway, PLAY_Y + q.y + droop, 20.0, 24.0, actor_z(q.y + 22.0));
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

/// A smashed root interrupts the bloom (the whole point); hedges just fall; the
/// fallen queen takes her garden with her.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut blockers: ResMut<crate::app::room_props::RoomBlockers>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    mut queens: Query<(Entity, &mut BriarQueen, &mut Health, &mut Sprite), (Without<BriarRoot>, Without<ThornHedge>)>,
    roots: Query<(Entity, &BriarRoot, &Health), Without<BriarQueen>>,
    hedges: Query<(Entity, &ThornHedge, &Health), Without<BriarQueen>>,
) {
    let Ok((qe, mut q, mut qh, mut qspr)) = queens.single_mut() else { return };
    for (e, root, rh) in &roots {
        if rh.hp > 0 {
            continue;
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(root.x + 6.0, root.y + 5.0), PINK, 12);
        commands.entity(e).despawn();
        if q.wilt.is_none() {
            // THE INTERRUPT: the bloom collapses — she is soft for a long window.
            q.wilt = Some(300);
            qspr.image = q.wilt_img.clone();
            qh.flash = 12;
            sfx.write(crate::app::sfx::Sfx("warpCharge"));
        }
    }
    for (e, hedge, hh) in &hedges {
        if hh.hp > 0 {
            continue;
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(hedge.blocker.0 + 6.0, hedge.blocker.1 + 4.0), 0x3a7a3a, 8);
        blockers.0.retain(|r| *r != hedge.blocker);
        q.hedge_slots[hedge.slot] = false;
        commands.entity(e).despawn();
    }
    if qh.hp <= 0 {
        for (e, _, _) in &roots {
            commands.entity(e).despawn();
        }
        for (e, hedge, _) in &hedges {
            blockers.0.retain(|r| *r != hedge.blocker);
            commands.entity(e).despawn();
        }
        let (cx, cy) = (q.x + 10.0, q.y + 10.0);
        for i in 0..3 {
            let off = i as f32 * 7.0 - 7.0;
            spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.5), PINK, 12);
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
        check("bloom", &QUEEN_BLOOM, 20);
        check("wilt", &QUEEN_WILT, 20);
        check("root", &ROOT, 12);
        check("hedge", &HEDGE, 16);
    }
}
