//! mob_think.rs — the ONE biome-mob AI interpreter (split from mobs.rs): walks the
//! [`Ai`] table per tick — approach/lunge/kite, hoppers, burrowers, casters, the
//! hurler's dart — and the shared per-axis step every mover and knockback uses.

use super::mobs::{face_from, Ai, Mob, MobAct, MobDef, MOB_DEFS};
use crate::combat::Hitbox;
use crate::room::{PX_H, PX_W};
use bevy::prelude::*;

#[allow(clippy::too_many_arguments)] // the js update(ctx)'s full context, flattened
pub fn mob_think(
    m: &mut Mob,
    grid: &crate::room::RoomGrid,
    blockers: &crate::app::room_props::RoomBlockers,
    ppos: Vec2,
    pface: Vec2,
    pbox: &Hitbox,
    rand: &mut impl FnMut() -> f32,
    contact: &mut Option<i32>,
    hittable: &mut bool,
) -> Option<MobAct> {
    let d = &MOB_DEFS[m.def];
    let (dx, dy) = (ppos.x - m.x, ppos.y - m.y);
    let dist = dx.hypot(dy).max(1.0);
    let (ux, uy) = (dx / dist, dy / dist);
    match &d.ai {
        Ai::Walker { spd, range } => {
            if dist < *range {
                mob_step(m, d, grid, blockers, pbox, dx.signum() * spd, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, dy.signum() * spd);
                m.facing = face_from(dx, dy);
            }
            m.anim += 1;
        }
        Ai::Chaser { spd, chase_r, vec, refaces, lunge } => {
            if *refaces {
                m.facing = face_from(dx, dy);
            }
            // The js lunge state machine (st: 0 idle / 1 wind-up / 2 dash / 3 recover).
            if m.st == 3 {
                m.t -= 1;
                if m.t <= 0 {
                    m.st = 0;
                }
                return None;
            }
            if m.st == 2 {
                let moved = mob_step(m, d, grid, blockers, pbox, m.cvx, m.cvy);
                m.t -= 1;
                if !moved || m.t <= 0 {
                    m.st = if lunge.recover > 0 { 3 } else { 0 };
                    m.t = lunge.recover;
                }
                return None;
            }
            if m.st == 1 {
                if lunge.face_windup {
                    m.facing = face_from(dx, dy);
                }
                m.t -= 1;
                if m.t <= 0 {
                    m.cvx = ux * lunge.dash_spd;
                    m.cvy = uy * lunge.dash_spd;
                    m.st = 2;
                    m.t = lunge.dash;
                    m.facing = face_from(dx, dy);
                }
                return None;
            }
            if dist < lunge.range && m.cd <= 0 {
                m.cd = lunge.cd;
                m.facing = face_from(dx, dy);
                if lunge.windup > 0 {
                    m.st = 1;
                    m.t = lunge.windup;
                } else {
                    m.cvx = ux * lunge.dash_spd;
                    m.cvy = uy * lunge.dash_spd;
                    m.st = 2;
                    m.t = lunge.dash;
                }
                return None;
            }
            if dist < *chase_r {
                let (ax, ay) = if *vec { (ux, uy) } else { (dx.signum(), dy.signum()) };
                mob_step(m, d, grid, blockers, pbox, ax * spd, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, ay * spd);
                m.facing = face_from(dx, dy);
            }
            m.anim += 1;
        }
        Ai::Flyer { spd, jamp, jfreq } => {
            let jx = ((m.anim as f32 + m.x) * jfreq).sin() * jamp;
            let jy = ((m.anim as f32 + m.y) * jfreq).cos() * jamp;
            mob_step(m, d, grid, blockers, pbox, ux * spd + jx, 0.0);
            mob_step(m, d, grid, blockers, pbox, 0.0, uy * spd + jy);
            m.facing = face_from(dx, dy);
            m.anim += 1;
        }
        Ai::Dormant { wake_r, spd } => {
            if m.st == 0 {
                if dist < *wake_r {
                    m.st = 1;
                }
                return None;
            }
            mob_step(m, d, grid, blockers, pbox, dx.signum() * spd, 0.0);
            mob_step(m, d, grid, blockers, pbox, 0.0, dy.signum() * spd);
            m.facing = face_from(dx, dy);
            m.anim += 1;
        }
        Ai::WebSpitter => {
            m.facing = face_from(dx, dy);
            if m.st == 1 {
                m.t -= 1;
                if m.t <= 0 {
                    m.st = 0;
                }
                return None;
            }
            if dist < 120.0 && m.cd <= 0 {
                m.st = 1;
                m.t = 10;
                m.cd = 120;
                return Some(MobAct::Web { x: m.x, y: m.y, vx: ux * 2.4, vy: uy * 2.4 });
            }
            // js kite: back off inside 55, close beyond 120, axis mode.
            let dir = if dist < 55.0 { -1.0 } else if dist > 120.0 { 1.0 } else { 0.0 };
            if dir != 0.0 {
                mob_step(m, d, grid, blockers, pbox, dx.signum() * 0.7 * dir, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, dy.signum() * 0.7 * dir);
            }
            m.anim += 1;
        }
        Ai::Burrow => {
            m.t += 1;
            if m.st == 0 {
                // Underground: unhittable, harmless, homing under the floor.
                *hittable = false;
                *contact = None;
                mob_step(m, d, grid, blockers, pbox, dx.signum() * 1.0, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, dy.signum() * 1.0);
                if dist < 24.0 || m.t > 200 {
                    m.st = 1;
                    m.t = 0;
                }
            } else {
                *contact = Some(2);
                if m.t > 90 {
                    m.st = 0;
                    m.t = 0;
                }
            }
            m.anim += 1;
        }
        Ai::Swoop => {
            m.facing = face_from(dx, dy);
            if m.st == 1 {
                let moved = mob_step(m, d, grid, blockers, pbox, m.cvx, m.cvy);
                m.t -= 1;
                if !moved || m.t <= 0 {
                    m.st = 2;
                    m.t = 26;
                }
                return None;
            }
            if m.st == 2 {
                mob_step(m, d, grid, blockers, pbox, -ux * 1.4, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, -uy * 1.4);
                m.t -= 1;
                if m.t <= 0 {
                    m.st = 0;
                }
                return None;
            }
            if dist > 72.0 {
                mob_step(m, d, grid, blockers, pbox, ux * 0.8, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, uy * 0.8);
            } else if m.cd <= 0 {
                m.cvx = ux * 2.9;
                m.cvy = uy * 2.9;
                m.st = 1;
                m.t = 16;
                m.cd = 95;
            }
            m.anim += 1;
        }
        Ai::CrossTurret { fire_r, cd, sp, dmg, color, life } => {
            m.anim += 1;
            if dist < *fire_r && m.cd <= 0 {
                m.cd = *cd;
                m.cvx += std::f32::consts::PI / 8.0; // the cross turns an eighth per volley
                let angs = (0..4).map(|q| m.cvx + q as f32 * std::f32::consts::FRAC_PI_2).collect();
                return Some(MobAct::Bolts { x: m.x, y: m.y, angs, sp: *sp, dmg: *dmg, color: *color, core: 0xffffff, life: *life, afflict: ("", 0) });
            }
        }
        Ai::SoundHunter { spd } => {
            // Blind, but not deaf (js): it tracks your FOOTFALLS — stand still and it
            // loses you. cvx/cvy remember where you last stood.
            let moved = m.st == 1 && (ppos.x - m.cvx).hypot(ppos.y - m.cvy) > 0.4;
            m.cvx = ppos.x;
            m.cvy = ppos.y;
            m.st = 1;
            if moved {
                mob_step(m, d, grid, blockers, pbox, (dx / dist) * *spd, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, (dy / dist) * *spd);
                m.facing = face_from(dx, dy);
                m.anim += 1;
            }
        }
        Ai::Fuse { spd, chase_r, arm_r, fuse } => {
            m.facing = face_from(dx, dy);
            m.anim += 1;
            if m.st == 1 {
                m.t -= 1;
                if m.t <= 0 {
                    return Some(MobAct::SelfDestruct); // the fuse ends loudly
                }
                return None;
            }
            if dist < *arm_r {
                m.st = 1;
                m.t = *fuse;
                return None;
            }
            if dist < *chase_r {
                mob_step(m, d, grid, blockers, pbox, (dx / dist) * *spd, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, (dy / dist) * *spd);
            }
        }
        Ai::Vent { fire_r, cd, rocks } => {
            m.anim += 1;
            if dist < *fire_r && m.cd <= 0 {
                m.cd = *cd;
                let list = (0..*rocks)
                    .map(|_| {
                        let tx = ppos.x + rand() * 56.0 - 28.0;
                        let ty = ppos.y + rand() * 40.0 - 20.0;
                        (m.x + 8.0, m.y + 6.0, tx, ty, 38)
                    })
                    .collect();
                return Some(MobAct::Rocks(list));
            }
        }
        Ai::RingBurst { fire_r, cd, n, sp, dmg, color, core, life, afflict, retreat_r, spd } => {
            m.anim += 1;
            m.facing = face_from(dx, dy);
            if dist < *fire_r && m.cd <= 0 {
                m.cd = *cd;
                let angs: Vec<f32> = (0..*n).map(|q| q as f32 * std::f32::consts::TAU / *n as f32 + 0.26).collect();
                return Some(MobAct::Bolts { x: m.x, y: m.y, angs, sp: *sp, dmg: *dmg, color: *color, core: *core, life: *life, afflict: *afflict });
            }
            // Drift toward the player, then back off after a bloom (cd still high).
            let want = if m.cd > *cd / 2 { -1.0 } else { 1.0 };
            if dist < retreat_r * 2.0 {
                mob_step(m, d, grid, blockers, pbox, ux * spd * want, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, uy * spd * want);
            }
        }
        Ai::PhaseClock { period, active_at, spd, seal, invert } => {
            m.anim += 1;
            let ph = (m.anim as i32) % period;
            let active = if *invert { ph < *active_at } else { ph >= *active_at };
            if !active {
                *hittable = false; // sealed / faded — harm passes through
                *contact = None;
                if *seal {
                    return None; // rooted in its shell
                }
            }
            if dist < 170.0 {
                mob_step(m, d, grid, blockers, pbox, ux * spd, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, uy * spd);
                m.facing = face_from(dx, dy);
            }
        }
        Ai::GazeStalker { spd, range } => {
            // Watched = the player faces roughly toward the statue (dot > 0.35).
            let watched = pface.x * ux + pface.y * uy > 0.35;
            m.facing = face_from(dx, dy);
            if watched {
                m.st = 0; // stone
            } else if dist < *range {
                m.st = 1; // walking
                mob_step(m, d, grid, blockers, pbox, ux * spd, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, uy * spd);
                m.anim += 1;
            }
        }
        Ai::Summoner { fire_r, cd, kind, max, spd } => {
            m.anim += 1;
            m.facing = face_from(dx, dy);
            if dist < *fire_r && m.cd <= 0 {
                // js caps live minions at 2 via e.raised; the rs interpreter can't count the
                // room, so the long cd paces it instead (FLAGGED as an approximation).
                let _ = max;
                m.cd = *cd;
                let side = if rand() < 0.5 { -18.0 } else { 18.0 };
                return Some(MobAct::Summon { kind, x: m.x + side, y: m.y + 8.0 });
            }
            if dist < 90.0 {
                mob_step(m, d, grid, blockers, pbox, -ux * spd, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, -uy * spd);
            }
        }
        Ai::Strafer { orbit_r, spd, lunge_r, lunge_spd, lunge_t, cd } => {
            m.facing = face_from(dx, dy);
            // st 1 = mid-lunge dash.
            if m.st == 1 {
                let moved = mob_step(m, d, grid, blockers, pbox, m.cvx, m.cvy);
                m.t -= 1;
                if !moved || m.t <= 0 {
                    m.st = 0;
                }
                m.anim += 1;
                return None;
            }
            if dist < *lunge_r && m.cd <= 0 {
                m.cvx = ux * lunge_spd;
                m.cvy = uy * lunge_spd;
                m.st = 1;
                m.t = *lunge_t;
                m.cd = *cd;
                return None;
            }
            if dist < *orbit_r {
                // Circle the player: a perpendicular strafe flipping direction slowly.
                let flip = if (m.anim / 70).is_multiple_of(2) { 1.0 } else { -1.0 };
                mob_step(m, d, grid, blockers, pbox, -uy * flip * spd + ux * 0.18, 0.0);
                mob_step(m, d, grid, blockers, pbox, ux * flip * spd + uy * 0.18, 0.0);
            }
            m.anim += 1;
        }
        Ai::OrbitDart { orbit_r, orbit_spd, dart_r, dart_spd, dart_t, cd } => {
            m.facing = face_from(dx, dy);
            if m.st == 1 {
                let moved = mob_step(m, d, grid, blockers, pbox, m.cvx, m.cvy);
                m.t -= 1;
                if !moved || m.t <= 0 {
                    m.st = 0;
                    m.cd = *cd;
                }
                m.anim += 1;
                return None;
            }
            // Sting when the player faces AWAY (dot < 0.35) and we're in range.
            let watched = pface.x * ux + pface.y * uy > 0.35;
            if !watched && dist < *dart_r && m.cd <= 0 {
                m.cvx = ux * dart_spd;
                m.cvy = uy * dart_spd;
                m.st = 1;
                m.t = *dart_t;
                return None;
            }
            // Otherwise orbit at radius, always advancing.
            let ang = (-uy).atan2(-ux) + 0.045;
            let (gx, gy) = (ppos.x + ang.cos() * orbit_r - m.x, ppos.y + ang.sin() * orbit_r - m.y);
            let mm = gx.hypot(gy).max(1.0);
            let step = (mm * 0.1).min(*orbit_spd);
            mob_step(m, d, grid, blockers, pbox, gx / mm * step, 0.0);
            mob_step(m, d, grid, blockers, pbox, 0.0, gy / mm * step);
            m.anim += 1;
        }
        Ai::Suction { pull_r, min_r, pull } => {
            m.anim += 1;
            if dist > *min_r && dist < *pull_r {
                return Some(MobAct::PullPlayer { tx: m.x, ty: m.y, pull: *pull });
            }
        }
        Ai::SkyCaller { fire_r, cd, spd } => {
            m.anim += 1;
            m.facing = face_from(dx, dy);
            let mut act = None;
            if dist < *fire_r && m.cd <= 0 {
                m.cd = *cd;
                act = Some(MobAct::SkyStrike { x: ppos.x, y: ppos.y });
            }
            if dist < 80.0 {
                mob_step(m, d, grid, blockers, pbox, -ux * spd, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, -uy * spd);
            }
            return act;
        }
        Ai::Swapper { cd, min_r, max_r, spd } => {
            m.anim += 1;
            m.facing = face_from(dx, dy);
            if m.cd <= 0 && dist > *min_r && dist < *max_r {
                m.cd = *cd;
                let mob_old = Vec2::new(m.x, m.y);
                m.x = ppos.x; // the mob takes the player's spot...
                m.y = ppos.y;
                return Some(MobAct::SwapPlayer { to: mob_old }); // ...the player takes the mob's
            }
            // A slow spiral otherwise (js orb sway).
            let orb = (m.anim as f32 * 0.05).sin();
            mob_step(m, d, grid, blockers, pbox, ux * spd * 0.4 - uy * orb * 0.8, 0.0);
            mob_step(m, d, grid, blockers, pbox, 0.0, uy * spd * 0.4 + ux * orb * 0.8);
        }
        Ai::Glimmer { flee_r, burst_r, burst_cd, beam_r, beam_cd, spd } => {
            m.anim += 1;
            m.facing = face_from(dx, dy);
            // Rooted mid-cast (st = beam-lock timer): the shimmer is your cue to move.
            if m.st > 0 {
                m.st -= 1;
                return None;
            }
            // Crowded: a close spark burst (cd rides m.cd).
            if dist < *burst_r && m.cd <= 0 {
                m.cd = *burst_cd;
                return Some(MobAct::Burst { x: m.x, y: m.y });
            }
            // Mid-range: lock a light-beam down the line (cd rides m.t as a second timer).
            if dist > 44.0 && dist < *beam_r && m.t <= 0 {
                m.t = *beam_cd;
                m.st = 36; // hold still through the shimmer + flash
                return Some(MobAct::Beam { x: m.x, y: m.y, tx: ppos.x, ty: ppos.y });
            }
            if m.t > 0 {
                m.t -= 1;
            }
            // Otherwise flee toward the open room (fleeVec + a light jink).
            if dist < *flee_r {
                let wob = (m.anim as f32 * 0.2).sin() * 0.4;
                mob_step(m, d, grid, blockers, pbox, -ux * spd + wob, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, -uy * spd - wob);
            }
        }
        Ai::Drainer { fire_r, cd, spd } => {
            m.anim += 1;
            m.facing = face_from(dx, dy);
            let mut act = None;
            if dist < *fire_r && m.cd <= 0 {
                m.cd = *cd;
                act = Some(MobAct::DrainOrb { x: m.x, y: m.y, vx: ux * 1.1, vy: uy * 1.1 });
            }
            if dist < 90.0 {
                mob_step(m, d, grid, blockers, pbox, -ux * spd, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, -uy * spd);
            }
            return act;
        }
        Ai::Hurl => {
            m.facing = face_from(dx, dy);
            m.anim += 1;
            if m.st == 1 {
                m.t -= 1;
                if m.t <= 0 {
                    m.st = 0;
                    hurler_dart(m, ppos, rand);
                }
                return None;
            }
            if !m.has_target {
                hurler_dart(m, ppos, rand);
            }
            let (tdx, tdy) = (m.tx - m.x, m.ty - m.y);
            let td = tdx.hypot(tdy).max(1.0);
            let jit = ((m.anim as f32) * 0.55 + m.x).sin() * 0.55;
            mob_step(m, d, grid, blockers, pbox, (tdx / td) * 1.45 + jit, 0.0);
            mob_step(m, d, grid, blockers, pbox, 0.0, (tdy / td) * 1.45 - jit);
            m.dart_t -= 1;
            if m.dart_t <= 0 || td < 12.0 {
                if dist < 150.0 {
                    m.st = 1;
                    m.t = 12;
                    m.has_target = false;
                    return Some(MobAct::Rock { sx: m.x + 4.0, sy: m.y, tx: ppos.x, ty: ppos.y, dur: 22 });
                }
                hurler_dart(m, ppos, rand); // player out of range: reposition instead
            }
        }
        // Batch-2 archetypes live in mob_think_2.
        _ => return mob_think_2(m, grid, blockers, ppos, pbox, rand),
    }
    None
}

/// The batch-2 archetypes, split out to keep mob_think readable.
#[allow(clippy::too_many_arguments)]
fn mob_think_2(
    m: &mut Mob,
    grid: &crate::room::RoomGrid,
    blockers: &crate::app::room_props::RoomBlockers,
    ppos: Vec2,
    pbox: &Hitbox,
    rand: &mut impl FnMut() -> f32,
) -> Option<MobAct> {
    let d = &MOB_DEFS[m.def];
    let (dx, dy) = (ppos.x - m.x, ppos.y - m.y);
    let dist = dx.hypot(dy).max(1.0);
    let (ux, uy) = (dx / dist, dy / dist);
    match &d.ai {
        Ai::Shooter { fire_r, cd, root, wobble, sp, near, spd } => {
            m.facing = face_from(dx, dy);
            if m.st == 1 {
                m.t -= 1;
                if m.t <= 0 {
                    m.st = 0;
                }
                return None; // rooted while the bow is drawn
            }
            if dist < *fire_r && m.cd <= 0 {
                let ang = dy.atan2(dx) + (rand() - 0.5) * wobble;
                m.cd = *cd;
                m.st = 1;
                m.t = *root;
                return Some(MobAct::Arrow { x: m.x + 3.0, y: m.y + 3.0, vx: ang.cos() * sp, vy: ang.sin() * sp });
            }
            // js kite (vec): back away inside `near`, close in beyond `fire_r`.
            let dir = if dist < *near { -1.0 } else if dist > *fire_r { 1.0 } else { 0.0 };
            if dir != 0.0 {
                mob_step(m, d, grid, blockers, pbox, ux * spd * dir, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, uy * spd * dir);
            }
            m.anim += 1;
        }
        Ai::Caster { fire_r, cd, sp, dmg, color, core, life, fan, spread, wobble, near, back_spd, far, fwd_spd } => {
            m.facing = face_from(dx, dy);
            if dist < *fire_r && m.cd <= 0 {
                m.cd = *cd;
                let ang = dy.atan2(dx) + (rand() - 0.5) * wobble;
                // A volley: `fan` bolts around the aim line (sporemother's 3-fan).
                let n = *fan;
                let first = ang - spread * ((n - 1) as f32) / 2.0;
                return Some(MobAct::Bolts {
                    x: m.x,
                    y: m.y,
                    angs: (0..n).map(|i| first + spread * i as f32).collect(),
                    sp: *sp,
                    dmg: *dmg,
                    color: *color,
                    core: *core,
                    life: *life,
                    afflict: d.afflicts,
                });
            }
            if dist < *near {
                mob_step(m, d, grid, blockers, pbox, -ux * back_spd, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, -uy * back_spd);
            } else if dist > *far {
                mob_step(m, d, grid, blockers, pbox, ux * fwd_spd, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, uy * fwd_spd);
            }
            m.anim += 1;
        }
        Ai::Hopper { spd } => {
            m.t += 1;
            if (m.t % 36) < 10 {
                let s = if m.small { 1.3 } else { *spd };
                mob_step(m, d, grid, blockers, pbox, ux * s, 0.0);
                mob_step(m, d, grid, blockers, pbox, 0.0, uy * s);
            }
            m.facing = face_from(dx, dy);
            m.anim += 1;
        }
        Ai::Blinker { cd, min, max, behind, spd } => {
            m.facing = face_from(dx, dy);
            if m.cd <= 0 && dist > *min && dist < *max {
                // Blink to a spot just behind the player.
                m.cd = *cd;
                let ang = dy.atan2(dx);
                let bx = (ppos.x - ang.cos() * behind).clamp(8.0, PX_W as f32 - 24.0);
                let by = (ppos.y - ang.sin() * behind).clamp(8.0, PX_H as f32 - 24.0);
                if !grid.box_hits_solid(bx + 3.0, by + 4.0, 10.0, 9.0) {
                    m.x = bx;
                    m.y = by;
                    return Some(MobAct::Blinked);
                }
            }
            mob_step(m, d, grid, blockers, pbox, ux * spd, 0.0);
            mob_step(m, d, grid, blockers, pbox, 0.0, uy * spd);
            m.anim += 1;
        }
        Ai::FrogHop => {
            m.facing = face_from(dx, dy);
            if m.st == 1 {
                // Mid-leap.
                let moved = mob_step(m, d, grid, blockers, pbox, m.cvx, m.cvy);
                m.t -= 1;
                if !moved || m.t <= 0 {
                    m.st = 0;
                    m.cd = 24 + (rand() * 30.0) as i32;
                }
                return None;
            }
            if m.st == 2 {
                // TONGUE LASH (js frogTongue): the tongue snaps out, then REELS you in over
                // the back half of the flick. (The tongue SPRITE is a flagged polish item —
                // the grab + reel is the mechanic.)
                m.t -= 1;
                m.anim += 1;
                if m.t <= 0 {
                    m.st = 0;
                    m.cd = 40 + (rand() * 30.0) as i32;
                    return None;
                }
                if m.t < 12 {
                    return Some(MobAct::PullPlayer { tx: m.x + 8.0, ty: m.y + 6.0, pull: 2.4 });
                }
                return None;
            }
            if m.cd > 0 {
                m.cd -= 1; // (the frog's own cooldown counts only while grounded — js)
                return None;
            }
            // Grounded + off cooldown: mid-range, it FLICKS ITS TONGUE 55% of the time (js);
            // otherwise it leaps. The grab+reel rides the shared player-pull rig.
            if (36.0..112.0).contains(&dist) && rand() < 0.55 {
                m.st = 2;
                m.t = 18;
                m.anim += 1;
                // Fling the tongue toward where the hero STANDS (direction locked at launch,
                // js frogTongue). The reel follows from the PullPlayer above once it lands.
                return Some(MobAct::Tongue { ax: m.x + 8.0, ay: m.y + 6.0, ux, uy, len: dist.min(62.0) });
            }
            let sp = if dist < 70.0 { 3.1 } else { 2.0 };
            m.cvx = ux * sp;
            m.cvy = uy * sp;
            m.st = 1;
            m.t = 12;
            m.anim += 1;
        }
        _ => unreachable!("mob_think handles the batch-1 archetypes"),
    }
    None
}

/// js hurlerDart: a point at throwing range from the PLAYER at a random angle, clamped
/// well off the walls so it never corners itself.
fn hurler_dart(m: &mut Mob, ppos: Vec2, rand: &mut impl FnMut() -> f32) {
    const MARGIN: f32 = 26.0;
    let r = 68.0 + rand() * 46.0;
    let ang = rand() * std::f32::consts::TAU;
    m.tx = (ppos.x + ang.cos() * r).clamp(MARGIN, PX_W as f32 - 16.0 - MARGIN);
    m.ty = (ppos.y + ang.sin() * r).clamp(MARGIN, PX_H as f32 - 16.0 - MARGIN);
    m.dart_t = 34 + (rand() * 26.0) as i32;
    m.has_target = true;
}

/// js e.step(): one axis move, blocked by solids/blockers/player body (fliers skip all).
#[allow(clippy::too_many_arguments)]
pub fn mob_step(
    m: &mut Mob,
    d: &MobDef,
    grid: &crate::room::RoomGrid,
    blockers: &crate::app::room_props::RoomBlockers,
    pbox: &Hitbox,
    dx: f32,
    dy: f32,
) -> bool {
    // js speedMul (Swift affix / elites) scales every stride.
    let (dx, dy) = (dx * m.speed_mul, dy * m.speed_mul);
    let (nx, ny) = (m.x + dx, m.y + dy);
    if !nx.is_finite() || !ny.is_finite() {
        return false;
    }
    let (bx, by, bw, bh) = (nx + d.hb.0, ny + d.hb.1, d.hb.2, d.hb.3);
    if !d.fly {
        if grid.box_hits_solid(bx, by, bw, bh) {
            return false;
        }
        if blockers.blocks((m.x + d.hb.0, m.y + d.hb.1, bw, bh), (bx, by, bw, bh)) {
            return false;
        }
        // Don't walk through the player's body (contact damage handles the touch).
        if bx < pbox.x + pbox.w && bx + bw > pbox.x && by < pbox.y + pbox.h && by + bh > pbox.y {
            return false;
        }
    }
    if nx < -2.0 || ny < -2.0 || nx > (PX_W - 14) as f32 || ny > (PX_H - 14) as f32 {
        return false;
    }
    m.x = nx;
    m.y = ny;
    true
}
