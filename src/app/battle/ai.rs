//! The brains: the goblin think loop, the one biome-mob AI interpreting MOB_DEFS,
//! and the knockback slides that own a struck body (split from battle.rs).

use super::{GameRng, RoomActor};
use super::projectiles::{EBolt, EnemyArrow};
use crate::app::play::{CurGrid, Player};
use crate::app::room_props::RoomBlockers;
use crate::app::room_render::{PLAY_X, PLAY_Y};
use crate::actors::attacks::{axe_bundle, stone_bundle, AttackArt};
use crate::actors::goblin::{goblin_hitbox, goblin_think, Goblin, GoblinAct};
use crate::actors::mobs::{self, mob_step, mob_think, ArcRock, ArcShadow, Mob, MobAct, MobArtBank, WebBolt};
use crate::combat::{Combatant, Health, Hitbox, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use bevy::prelude::*;

/// The goblin brain + its attack spawns.
pub(super) fn goblin_ai(
    mut commands: Commands,
    grid: Res<CurGrid>,
    blockers: Res<RoomBlockers>,
    art: Res<AttackArt>,
    mut rng: ResMut<GameRng>,
    players: Query<(&Player, &Hitbox), Without<Goblin>>,
    mut goblins: Query<(Entity, &mut Goblin, &mut Hitbox, &Knockback, &Health), Without<Player>>,
) {
    let Ok((p, pbox)) = players.single() else { return };
    let ppos = Vec2::new(p.x, p.y);
    let mut rand = || rng.0.next_f64() as f32;
    for (ent, mut g, mut hb, kb, health) in &mut goblins {
        if health.hp <= 0 {
            continue;
        }
        let act = goblin_think(&mut g, kb, &grid.0, &blockers.0, ppos, pbox, &mut rand);
        *hb = goblin_hitbox(&g);
        match act {
            Some(GoblinAct::Axe { fx, fy, x, y }) => {
                commands.spawn((axe_bundle(fx, fy, ent, x, y, &art), RoomActor, PIXEL_LAYER));
            }
            Some(GoblinAct::Stone { x, y, angle }) => {
                commands.spawn((stone_bundle(x, y, angle, &art), RoomActor, PIXEL_LAYER));
            }
            None => {}
        }
    }
}

/// The biome-mob brain (js mob().update): aggro gate, then the per-kind Ai — one system
/// interpreting the MOB_DEFS table. A struck mob wakes at once (js onHurt).
#[allow(clippy::type_complexity)]
pub(super) fn mobs_ai(
    mut commands: Commands,
    grid: Res<CurGrid>,
    blockers: Res<RoomBlockers>,
    art: Res<MobArtBank>,
    mut rng: ResMut<GameRng>,
    mut players: Query<(&mut Player, &Hitbox), Without<Mob>>,
    mut q: Query<(&mut Mob, &mut Hitbox, &mut Combatant, &mut Health, &Knockback, Option<&crate::app::uniques::MobAfflictions>), Without<Player>>,
) {
    let (ppos, pface, pbox_val) = {
        let Ok((p, pbox)) = players.single() else { return };
        let pface = match p.facing {
            crate::actors::hero::Facing::Down => Vec2::new(0.0, 1.0),
            crate::actors::hero::Facing::Up => Vec2::new(0.0, -1.0),
            crate::actors::hero::Facing::Right => Vec2::new(1.0, 0.0),
            crate::actors::hero::Facing::Left => Vec2::new(-1.0, 0.0),
        };
        (Vec2::new(p.x, p.y), pface, *pbox)
    };
    let pbox = &pbox_val;
    // Deferred player effects (a mob can't hold &mut Player mid-loop): applied after.
    let mut pull: Option<(f32, f32, f32)> = None; // (tx, ty, strength)
    let mut swap_to: Option<Vec2> = None;
    let mut rand = || rng.0.next_f64() as f32;
    for (mut m, mut hb, mut cb, mut health, kb, afflictions) in &mut q {
        if health.hp <= 0 {
            continue;
        }
        if m.cd > 0 {
            m.cd -= 1;
        }
        let d = &mobs::MOB_DEFS[m.def];
        // Collapsed (js downRevive): lies still and harmless, then rises restored.
        if m.downed {
            m.down_t -= 1;
            if m.down_t <= 0 {
                m.downed = false;
                cb.damage = Some(d.damage);
                health.hp = health.max;
                health.invuln = 14;
                health.flash = 12;
            }
            let hs = if m.size_mul > 1.0 { 1.7 } else { 1.0 };
        let (gw, gh) = (d.hb.2 * hs, d.hb.3 * hs);
        *hb = Hitbox { x: m.x + d.hb.0 - (gw - d.hb.2) / 2.0, y: m.y + d.hb.1 - (gh - d.hb.3) / 2.0, w: gw, h: gh };
            continue;
        }
        // The Lullaby: asleep foes stand dreaming — no aggro, no thinking (flute.rs
        // wakes them the instant they're struck).
        if m.sleep > 0 {
            m.sleep -= 1;
            let hs = if m.size_mul > 1.0 { 1.7 } else { 1.0 };
        let (gw, gh) = (d.hb.2 * hs, d.hb.3 * hs);
        *hb = Hitbox { x: m.x + d.hb.0 - (gw - d.hb.2) / 2.0, y: m.y + d.hb.1 - (gh - d.hb.3) / 2.0, w: gw, h: gh };
            continue;
        }
        // Knockback owns the body this tick (apply_mob_knockback moves it).
        if kb.timer == 0 {
            // Aggro gate: idle until the player is close; a struck foe jolts awake.
            if !m.aggro {
                let (adx, ady) = ((ppos.x + 8.0) - (m.x + 8.0), (ppos.y + 9.0) - (m.y + 8.0));
                if adx * adx + ady * ady <= mobs::AGGRO_R * mobs::AGGRO_R || health.hp < health.max {
                    m.aggro = true;
                }
            }
            if m.aggro {
                // CHILLED (Winter Shard / boomerang): the foe crawls — every other
                // think tick skips (js slowT halves speed; the cadence-skip also slows
                // its attacks a touch — flagged).
                if let Some(aff) = afflictions
                    && aff.chill > 0
                    && (aff.chill & 1) == 1
                {
                    m.anim += 1;
                    let hs = if m.size_mul > 1.0 { 1.7 } else { 1.0 };
        let (gw, gh) = (d.hb.2 * hs, d.hb.3 * hs);
        *hb = Hitbox { x: m.x + d.hb.0 - (gw - d.hb.2) / 2.0, y: m.y + d.hb.1 - (gh - d.hb.3) / 2.0, w: gw, h: gh };
                    continue;
                }
                let mut hittable = true;
                let act = mob_think(&mut m, &grid.0, &blockers, ppos, pface, pbox, &mut rand, &mut cb.damage, &mut hittable);
                if !hittable {
                    health.invuln = health.invuln.max(3); // js: the tunnelling burrower can't be hit
                }
                if matches!(d.ai, mobs::Ai::Drainer { .. }) {
                    // Self-mending (js witherheart regenT): pain resets the timer; after ~90
                    // untroubled frames it regrows 1 HP every 30 (m.st = timer, m.tx = last hp).
                    if health.hp < m.tx as i32 {
                        m.st = 0;
                    }
                    m.tx = health.hp as f32;
                    m.st += 1;
                    if m.st > 90 && health.hp < health.max && m.anim.is_multiple_of(30) {
                        health.hp += 1;
                    }
                }
                match act {
                    Some(MobAct::Web { x, y, vx, vy }) => {
                        commands.spawn((
                            WebBolt { x, y, vx, vy, life: 120 },
                            crate::combat::Afflicts("slow", 110),
                            Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(1), persistent: false, knock: 0.0 },
                            crate::combat::HitOnce::default(),
                            Hitbox { x: x + 5.0, y: y + 5.0, w: 6.0, h: 6.0 },
                            Sprite::from_image(art.web.clone()),
                            at(PLAY_X + x + 5.0, PLAY_Y + y + 5.0, 5.0, 5.0, 8.6),
                            PIXEL_LAYER,
                            RoomActor,
                        ));
                    }
                    Some(MobAct::Rock { sx, sy, tx, ty, dur }) => {
                        // The landing telegraph (js: a 30%-black 8x4 pool at the target).
                        let rock = commands
                            .spawn((
                                ArcRock { sx, sy, tx, ty, t: 0, dur },
                                Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: None, persistent: false, knock: 0.0 },
                                crate::combat::HitOnce::default(),
                                Hitbox { x: -99.0, y: -99.0, w: 0.0, h: 0.0 },
                                Sprite::from_image(art.rock.clone()),
                                at(PLAY_X + sx + 6.0, PLAY_Y + sy + 6.0, 5.0, 5.0, 11.5),
                                PIXEL_LAYER,
                                RoomActor,
                            ))
                            .id();
                        commands.spawn((
                            ArcShadow(rock),
                            Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.35), Vec2::new(8.0, 4.0)),
                            at(PLAY_X + tx + 5.0, PLAY_Y + ty + 11.0, 8.0, 4.0, 3.5),
                            PIXEL_LAYER,
                            RoomActor,
                        ));
                    }
                    Some(MobAct::Arrow { x, y, vx, vy }) => {
                        let ang = vy.atan2(vx);
                        let mut tf = at(PLAY_X + x, PLAY_Y + y, 16.0, 16.0, 8.6);
                        tf.rotation = Quat::from_rotation_z(-ang);
                        commands.spawn((
                            EnemyArrow { x, y, vx, vy, life: 42 },
                            Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                            crate::combat::HitOnce::default(),
                            Hitbox { x: x + 5.0, y: y + 5.0, w: 6.0, h: 6.0 },
                            Sprite::from_image(art.arrow.clone()),
                            tf,
                            PIXEL_LAYER,
                            RoomActor,
                        ));
                    }
                    Some(MobAct::Bolts { x, y, angs, sp, dmg, color, core, life, afflict }) => {
                        for ang in angs {
                            let bolt = art.bolt(color, core);
                            let e = commands
                                .spawn((
                                    EBolt { x, y, vx: ang.cos() * sp, vy: ang.sin() * sp, life },
                                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(dmg), persistent: false, knock: 0.0 },
                                    crate::combat::HitOnce::default(),
                                    Hitbox { x: x + 3.0, y: y + 3.0, w: 7.0, h: 7.0 },
                                    Sprite::from_image(bolt),
                                    at(PLAY_X + x + 1.0, PLAY_Y + y + 1.0, 8.0, 8.0, 8.6),
                                    PIXEL_LAYER,
                                    RoomActor,
                                ))
                                .id();
                            if !afflict.0.is_empty() {
                                // A venom/frost bolt clings its status to the player it strikes.
                                commands.entity(e).insert(crate::combat::Afflicts(afflict.0, afflict.1));
                            }
                        }
                    }
                    Some(MobAct::Rocks(list)) => {
                        for (sx, sy, tx, ty, dur) in list {
                            let rock = commands
                                .spawn((
                                    ArcRock { sx, sy, tx, ty, t: 0, dur },
                                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: None, persistent: false, knock: 0.0 },
                                    crate::combat::HitOnce::default(),
                                    Hitbox { x: -99.0, y: -99.0, w: 0.0, h: 0.0 },
                                    Sprite::from_image(art.rock.clone()),
                                    at(PLAY_X + sx + 6.0, PLAY_Y + sy + 6.0, 5.0, 5.0, 11.5),
                                    PIXEL_LAYER,
                                    RoomActor,
                                ))
                                .id();
                            commands.spawn((
                                ArcShadow(rock),
                                Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.35), Vec2::new(8.0, 4.0)),
                                at(PLAY_X + tx + 5.0, PLAY_Y + ty + 11.0, 8.0, 4.0, 3.5),
                                PIXEL_LAYER,
                                RoomActor,
                            ));
                        }
                    }
                    Some(MobAct::SelfDestruct) => {
                        health.hp = 0; // the emberling ends on its own terms (volatile blast follows)
                    }
                    Some(MobAct::Blinked) => {
                        // js voidling blink: brief i-frames + a flash at the new spot.
                        health.invuln = health.invuln.max(8);
                        health.flash = health.flash.max(8);
                    }
                    Some(MobAct::PullPlayer { tx, ty, pull: str }) => {
                        pull = Some((tx, ty, str)); // one drag at a time (last mob wins)
                    }
                    Some(MobAct::SwapPlayer { to }) => {
                        swap_to = Some(to);
                    }
                    Some(MobAct::SkyStrike { x, y }) => {
                        crate::app::skystrike::spawn(&mut commands, x, y); // telegraphed strike
                    }
                    Some(MobAct::Burst { x, y }) => {
                        crate::app::mobfx::spawn_burst_ring(&mut commands, x, y);
                    }
                    Some(MobAct::Beam { x, y, tx, ty }) => {
                        crate::app::mobfx::spawn_beam(&mut commands, x, y, tx, ty);
                    }
                    Some(MobAct::DrainOrb { x, y, vx, vy }) => {
                        crate::app::mobfx::spawn_drain_orb(&mut commands, x, y, vx, vy);
                    }
                    Some(MobAct::Tongue { ax, ay, ux, uy, len }) => {
                        crate::app::mobfx::spawn_frog_tongue(&mut commands, ax, ay, ux, uy, len);
                    }
                    Some(MobAct::Summon { kind, x, y }) => {
                        // The gravewarden raises a minion (js make + spawn).
                        if let Some(idx) = mobs::def_index(kind) {
                            commands.spawn((
                                mobs::mob_bundle(idx, x, y),
                                RoomActor,
                                PIXEL_LAYER,
                                crate::app::dungeon::DungeonFoe(mobs::MOB_DEFS[idx].kind),
                            ));
                        }
                    }
                    None => {}
                }
            } else {
                m.anim += 1;
                m.t += 1; // dormant: idle anims keep ticking (js)
            }
        }
        let hs = if m.size_mul > 1.0 { 1.7 } else { 1.0 };
        let (gw, gh) = (d.hb.2 * hs, d.hb.3 * hs);
        *hb = Hitbox { x: m.x + d.hb.0 - (gw - d.hb.2) / 2.0, y: m.y + d.hb.1 - (gh - d.hb.3) / 2.0, w: gw, h: gh };
    }
    // Apply the deferred player effects now that the mob loop released &mut Player.
    if (pull.is_some() || swap_to.is_some())
        && let Ok((mut pl, _)) = players.single_mut()
    {
        {
            if let Some((tx, ty, str)) = pull {
                // The sand drags the hero toward the maw (js sandmaw), stopping on walls.
                let (dx, dy) = (tx - pl.x, ty - pl.y);
                let dd = dx.hypot(dy).max(1.0);
                let nx = pl.x + dx / dd * str;
                let ny = pl.y + dy / dd * str;
                if !grid.0.box_hits_solid(nx + 3.0, ny + 2.0, 10.0, 13.0) {
                    pl.x = nx;
                    pl.y = ny;
                }
            }
            if let Some(to) = swap_to {
                // The switchshade traded places (its own x/y already moved in mob_think).
                pl.x = to.x;
                pl.y = to.y;
            }
        }
    }
}


/// Knockback rides every combatant with a body (goblins; the player's is applied in play.rs
/// because the player slides against tiles with his own feet box).
pub(super) fn apply_knockback(
    grid: Res<CurGrid>,
    blockers: Res<RoomBlockers>,
    mut q: Query<(&mut Goblin, &mut Knockback)>,
) {
    for (mut g, mut kb) in &mut q {
        if kb.timer == 0 {
            continue;
        }
        kb.timer -= 1;
        // Same per-axis slide as the goblin's own move (grid-only; JS also blocked on entities).
        let (kx, ky) = (kb.kx, kb.ky);
        for (dx, dy) in [(kx, 0.0), (0.0, ky)] {
            let nx = g.x + dx;
            let ny = g.y + dy;
            if !grid.0.box_hits_solid(nx + 3.0, ny + 8.0, 10.0, 6.0)
                && !blockers.blocks((g.x + 3.0, g.y + 8.0, 10.0, 6.0), (nx + 3.0, ny + 8.0, 10.0, 6.0))
            {
                g.x = nx;
                g.y = ny;
            }
        }
    }
}

/// Knockback slide for biome mobs (the goblin version reads Goblin; same shape).
pub(super) fn apply_mob_knockback(
    grid: Res<CurGrid>,
    blockers: Res<RoomBlockers>,
    players: Query<&Hitbox, With<Player>>,
    mut q: Query<(&mut Mob, &mut Knockback), Without<Player>>,
) {
    let Ok(pbox) = players.single() else { return };
    for (mut m, mut kb) in &mut q {
        if kb.timer == 0 {
            continue;
        }
        kb.timer -= 1;
        let d = &mobs::MOB_DEFS[m.def];
        let (kx, ky) = (kb.kx, kb.ky);
        mob_step(&mut m, d, &grid.0, &blockers, pbox, kx, 0.0);
        mob_step(&mut m, d, &grid.0, &blockers, pbox, 0.0, ky);
    }
}
