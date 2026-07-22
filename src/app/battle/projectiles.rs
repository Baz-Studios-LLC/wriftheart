//! Every flying thing: the player's swing/axe/stone lifetimes, and the mobs'
//! webs, arc rocks, arrows and elemental bolts (split from battle.rs).

use crate::app::play::{CurGrid, Player};
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::actors::attacks::{axe_tick, axe_z, stone_tick, swing_tick, swing_z, AxeSwipe, Stone, Swing};
use crate::actors::goblin::Goblin;
use crate::actors::mobs::{ArcRock, ArcShadow, WebBolt};
use crate::combat::{Combatant, Hitbox};
use crate::gfx::at;
use bevy::prelude::*;

/// Advance swings/axes/stones: lifetimes, hitbox windows, expiry.
#[allow(clippy::type_complexity)] // ECS system queries are wide by nature
pub(super) fn attacks_tick(
    mut commands: Commands,
    grid: Res<CurGrid>,
    players: Query<&Player>,
    mut swings: Query<(Entity, &mut Swing, &mut Hitbox, &mut Transform), (Without<AxeSwipe>, Without<Stone>)>,
    mut axes: Query<(Entity, &mut AxeSwipe, &mut Hitbox, &mut Transform, &mut Combatant), (Without<Swing>, Without<Stone>)>,
    mut stones: Query<(Entity, &mut Stone, &mut Hitbox, &mut Transform), (Without<Swing>, Without<AxeSwipe>)>,
    wielders: Query<&Goblin>,
) {
    let ppos = players.single().map(|p| Vec2::new(p.x, p.y)).unwrap_or_default();
    for (e, mut s, mut hb, mut tf) in &mut swings {
        let (nhb, rot, pivot, alive) = swing_tick(&mut s, ppos.x, ppos.y);
        *hb = nhb;
        // ROUNDED y, matching sync_player_sprite's own z: the +-0.5 subpixel gap is
        // nearly twice swing_z's 0.005 tuck, so an unrounded base let an up-swing's
        // blade paste over the hero's face at half the standing spots (Baz).
        *tf = at(PLAY_X + pivot.x, PLAY_Y + pivot.y, 0.0, 0.0, swing_z(s.facing, actor_z(ppos.y.round() + 16.0)));
        tf.rotation = Quat::from_rotation_z(-rot); // canvas cw -> bevy ccw
        if !alive {
            commands.entity(e).despawn();
        }
    }
    for (e, mut a, mut hb, mut tf, mut cb) in &mut axes {
        // The axe pivots on the goblin LIVE (the JS closure read g.x each frame) — it rides
        // knockback mid-swing and only freezes if the wielder is already gone.
        if let Ok(g) = wielders.get(a.wielder) {
            a.ox = g.x;
            a.oy = g.y;
        }
        let (nhb, rot, pivot, alive) = axe_tick(&mut a);
        // The axe only bites after the wind-up: gate its damage by the hitbox window.
        cb.damage = nhb.is_some().then_some(1);
        if let Some(nhb) = nhb {
            *hb = nhb;
        }
        *tf = at(PLAY_X + pivot.x, PLAY_Y + pivot.y, 0.0, 0.0, axe_z(a.fy, actor_z(a.oy.round() + 16.0))); // rounded like the goblin's own z
        tf.rotation = Quat::from_rotation_z(-rot);
        if !alive {
            commands.entity(e).despawn();
        }
    }
    for (e, mut st, mut hb, mut tf) in &mut stones {
        if !stone_tick(&mut st, &grid.0) {
            commands.entity(e).despawn();
            continue;
        }
        *hb = Hitbox { x: st.x + 5.0, y: st.y + 5.0, w: 6.0, h: 6.0 };
        *tf = at(PLAY_X + st.x + 4.0, PLAY_Y + st.y + 4.0, 8.0, 8.0, actor_z(st.y + 8.0));
    }
}

/// The skeleton archer's arrow — straight flight, rotated to face it (js `arrow`).
#[derive(Component)]
pub struct EnemyArrow {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: i32,
}

/// A caster's elemental bolt (js `eBolt`): coloured square with a bright core.
#[derive(Component)]
pub struct EBolt {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: i32,
}

/// Advance webs (straight flight, dies on walls) and arc rocks (harmless flight on a
/// parabola; the landing hurts for 5 frames).
#[allow(clippy::type_complexity)]
pub(crate) fn mob_projectiles_tick(
    mut commands: Commands,
    grid: Res<CurGrid>,
    mut webs: Query<(Entity, &mut WebBolt, &mut Transform, &mut Hitbox), (Without<ArcRock>, Without<ArcShadow>)>,
    mut rocks: Query<(Entity, &mut ArcRock, &mut Transform, &mut Hitbox, &mut Combatant), Without<WebBolt>>,
    shadows: Query<(Entity, &ArcShadow)>,
) {
    for (e, mut w, mut tf, mut hb) in &mut webs {
        w.x += w.vx;
        w.y += w.vy;
        w.life -= 1;
        *hb = Hitbox { x: w.x + 5.0, y: w.y + 5.0, w: 6.0, h: 6.0 };
        *tf = at(PLAY_X + w.x + 5.0, PLAY_Y + w.y + 5.0, 5.0, 5.0, 8.6);
        // Webs share the shot rule: water doesn't stop them, walls do (see
        // enemy_shots_tick's over-water note).
        let (tc, tr) = (((w.x + 8.0) / 16.0).floor() as i32, ((w.y + 8.0) / 16.0).floor() as i32);
        if (grid.0.code_at(tc, tr) != '~' && grid.0.box_hits_solid(w.x + 5.0, w.y + 5.0, 6.0, 6.0))
            || w.x < -16.0
            || w.x > crate::room::PX_W as f32
            || w.y < -16.0
            || w.y > crate::room::PX_H as f32
            || w.life <= 0
        {
            commands.entity(e).despawn();
        }
    }
    for (e, mut r, mut tf, mut hb, mut cb) in &mut rocks {
        r.t += 1;
        let p = (r.t as f32 / r.dur as f32).min(1.0);
        let x = r.sx + (r.tx - r.sx) * p;
        let y = r.sy + (r.ty - r.sy) * p;
        let arc = (p * std::f32::consts::PI).sin() * 18.0;
        *tf = at(PLAY_X + x + 6.0, PLAY_Y + y + 6.0 - arc, 5.0, 5.0, 11.5);
        if r.t >= r.dur {
            cb.damage = Some(2);
            *hb = Hitbox { x: r.tx + 4.0, y: r.ty + 5.0, w: 12.0, h: 11.0 };
            if r.t >= r.dur + 5 {
                commands.entity(e).despawn();
            }
        }
    }
    for (se, sh) in &shadows {
        if rocks.get(sh.0).is_err() {
            commands.entity(se).despawn();
        }
    }
}

/// Advance arrows + elemental bolts: straight flight, dead on walls/bounds/expiry.
pub(crate) fn enemy_shots_tick(
    mut commands: Commands,
    grid: Res<CurGrid>,
    mut arrows: Query<(Entity, &mut EnemyArrow, &mut Transform, &mut Hitbox), Without<EBolt>>,
    mut bolts: Query<(Entity, &mut EBolt, &mut Transform, &mut Hitbox), Without<EnemyArrow>>,
) {
    let (w, h) = (crate::room::PX_W as f32, crate::room::PX_H as f32);
    // Enemy shots sail OVER water exactly like the player's (archery.rs) — without
    // this, a water sniper's bolt died on its OWN spawn tile ('~' is solid) and the
    // spitgill just bobbed there looking harmless (Baz).
    let over_water = |x: f32, y: f32| {
        let (tc, tr) = (((x + 8.0) / 16.0).floor() as i32, ((y + 8.0) / 16.0).floor() as i32);
        grid.0.code_at(tc, tr) == '~'
    };
    for (e, mut a, mut tf, mut hb) in &mut arrows {
        a.x += a.vx;
        a.y += a.vy;
        a.life -= 1;
        *hb = Hitbox { x: a.x + 5.0, y: a.y + 5.0, w: 6.0, h: 6.0 };
        let rot = tf.rotation;
        *tf = at(PLAY_X + a.x, PLAY_Y + a.y, 16.0, 16.0, 8.6);
        tf.rotation = rot;
        if (!over_water(a.x, a.y) && grid.0.box_hits_solid(a.x + 5.0, a.y + 5.0, 6.0, 6.0))
            || a.x < -16.0
            || a.x > w
            || a.y < -16.0
            || a.y > h
            || a.life <= 0
        {
            commands.entity(e).despawn();
        }
    }
    for (e, mut b, mut tf, mut hb) in &mut bolts {
        b.x += b.vx;
        b.y += b.vy;
        b.life -= 1;
        *hb = Hitbox { x: b.x + 3.0, y: b.y + 3.0, w: 7.0, h: 7.0 };
        *tf = at(PLAY_X + b.x + 1.0, PLAY_Y + b.y + 1.0, 8.0, 8.0, 8.6);
        if (!over_water(b.x, b.y) && grid.0.box_hits_solid(b.x + 3.0, b.y + 3.0, 7.0, 7.0))
            || b.x < -16.0
            || b.x > w
            || b.y < -16.0
            || b.y > h
            || b.life <= 0
        {
            commands.entity(e).despawn();
        }
    }
}

/// blockShotsOnProps (js game.js): a shot dies when it hits a SOLID PROP — the same
/// rocks/trees/bushes/buildings that block your feet (RoomBlockers). Applies to every
/// straight shot, player OR enemy: arrows, spell bolts, caster bolts, webs. FIRE bolts
/// are the exception — app/fire.rs stops them ON brush (igniting it first), so they're
/// filtered out here to avoid killing them a tick before they can light the world.
/// (The boomerang, grapple claw, and Kingsplitter beam are hand-flighted weapons, not
/// `projectile` shots in the js sense — they keep their own stop rules.)
#[allow(clippy::type_complexity)]
pub(crate) fn block_shots_on_props(
    mut commands: Commands,
    blockers: Res<super::super::room_props::RoomBlockers>,
    arrows: Query<(Entity, &Hitbox), With<super::super::archery::PlayerArrow>>,
    bolts: Query<(Entity, &Hitbox, &super::super::wands::SpellBolt)>,
    enemy_arrows: Query<(Entity, &Hitbox), With<EnemyArrow>>,
    ebolts: Query<(Entity, &Hitbox), With<EBolt>>,
    webs: Query<(Entity, &Hitbox), With<crate::actors::mobs::WebBolt>>,
) {
    if blockers.0.is_empty() {
        return;
    }
    let hits = |hb: &Hitbox| {
        blockers.0.iter().any(|b| hb.x < b.0 + b.2 && hb.x + hb.w > b.0 && hb.y < b.1 + b.3 && hb.y + hb.h > b.1)
    };
    for (e, hb) in &arrows {
        if hits(hb) {
            commands.entity(e).despawn();
        }
    }
    for (e, hb, bolt) in &bolts {
        if !bolt.fire && hits(hb) {
            commands.entity(e).despawn();
        }
    }
    for (e, hb) in &enemy_arrows {
        if hits(hb) {
            commands.entity(e).despawn();
        }
    }
    for (e, hb) in &ebolts {
        if hits(hb) {
            commands.entity(e).despawn();
        }
    }
    for (e, hb) in &webs {
        if hits(hb) {
            commands.entity(e).despawn();
        }
    }
}
