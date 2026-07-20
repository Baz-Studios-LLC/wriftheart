//! archery.rs — THE BOW (js items.js 'bow' + arrow()): the ranged weapon. A slot
//! press looses a player-team arrow along the 8-WAY AIM — the held movement keys
//! if any (diagonals!), the cardinal facing otherwise (js aimVec). Arrows fly at
//! 4.4 for 70 frames, hit for 2 x (1 + melee), die on the first foe they bite
//! (js oneShot), and sail OVER water — only walls and rocks stop them. No arrows
//! in the bag = the dry click (play.rs routes the press either way; the ammo is
//! consumed there so the fire here never vetoes).
//! Arrows carry the js crit fields (chance = the player's crit stat, x2 + critmult).

use bevy::prelude::*;

use super::battle::RoomActor;
use super::play::{CurGrid, Player};
use super::room_render::{PLAY_X, PLAY_Y};
use crate::combat::{Combatant, HitLanded, HitOnce, Hitbox, Team};
use crate::gfx::{at, bake, PIXEL_LAYER};
use crate::input::{Action, ActionState};

/// "Loose one" — play.rs consumed the ammo (or found the bag dry).
#[derive(Message)]
pub struct FireArrow {
    pub dry: bool,
}

/// An arrow in flight (js arrow()): straight, fast, one bite.
#[derive(Component)]
pub struct PlayerArrow {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: i32,
}

/// The js arrow, authored pointing RIGHT (rotated to the flight line): shaft,
/// bright head, twin fletching nubs.
const ARROW_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "..ff............",
    "...sssssssssWW..",
    "...sssssssssWW..",
    "..ff............",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
];

/// The 8-way aim (js aimVec): held movement keys wins, facing as the fallback.
fn aim_vec(state: &ActionState, p: &Player) -> (f32, f32) {
    let dx = (state.held(Action::Right) as i32 - state.held(Action::Left) as i32) as f32;
    let dy = (state.held(Action::Down) as i32 - state.held(Action::Up) as i32) as f32;
    if dx == 0.0 && dy == 0.0 {
        return match p.facing {
            crate::actors::hero::Facing::Up => (0.0, -1.0),
            crate::actors::hero::Facing::Down => (0.0, 1.0),
            crate::actors::hero::Facing::Left => (-1.0, 0.0),
            crate::actors::hero::Facing::Right => (1.0, 0.0),
        };
    }
    let m = dx.hypot(dy);
    (dx / m, dy / m)
}

/// Loose + fly + bite: spawns on FireArrow, walks every arrow, and retires any
/// that landed a hit this tick (js oneShot).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn arrow_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fires: MessageReader<FireArrow>,
    mut hits: MessageReader<HitLanded>,
    state: Res<ActionState>,
    grid: Res<CurGrid>,
    tstats: Res<super::slideout::TreeStats>,
    statuses: Res<super::status::Statuses>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&Player>,
    mut flying: Query<(Entity, &mut PlayerArrow, &mut Transform, &mut Hitbox)>,
) {
    if let Ok(p) = players.single() {
        for f in fires.read() {
            if f.dry {
                sfx.write(super::sfx::Sfx("tink")); // the empty-quiver click
                continue;
            }
            let (dx, dy) = aim_vec(&state, p);
            let sp = 4.4;
            // js: 2 x (1 + melee) — the tree + any meal buff, same term every swing uses.
            let dmg = ((2.0 * (1.0 + tstats.melee + statuses.sum(|m| m.melee))) + 0.5).floor().max(1.0) as i32;
            let (x, y) = (p.x + dx * 6.0, p.y + dy * 6.0);
            let img = images.add(bake(
                &ARROW_ART,
                &[('s', 0xcaa050), ('W', 0xe8e8e8), ('f', 0xd8d8d8)],
            ));
            let mut spr = Sprite::from_image(img);
            spr.custom_size = Some(Vec2::splat(16.0));
            let mut tf = at(PLAY_X + x, PLAY_Y + y, 16.0, 16.0, 8.6);
            tf.rotation = Quat::from_rotation_z(-dy.atan2(dx)); // js ctx.rotate, y-flipped
            commands.spawn((
                spr,
                tf,
                PIXEL_LAYER,
                RoomActor,
                PlayerArrow { x, y, vx: dx * sp, vy: dy * sp, life: 70 },
                Combatant { team: Team::Player, hurt_team: Some(Team::Enemy), damage: Some(dmg), persistent: true, knock: 1.0 },
                crate::combat::CritChance {
                    chance: tstats.crit + statuses.sum(|m| m.crit),
                    mult: 2.0 + tstats.critmult,
                },
                HitOnce::default(),
                Hitbox { x: x + 5.0, y: y + 5.0, w: 6.0, h: 6.0 },
            ));
            sfx.write(super::sfx::Sfx("swing"));
        }
    }
    // One bite each: an arrow that landed this tick is spent.
    let mut spent: Vec<Entity> = Vec::new();
    for hit in hits.read() {
        if flying.get(hit.attacker).is_ok() {
            spent.push(hit.attacker);
        }
    }
    for (e, mut a, mut tf, mut hb) in &mut flying {
        if spent.contains(&e) {
            commands.entity(e).despawn();
            continue;
        }
        a.x += a.vx;
        a.y += a.vy;
        a.life -= 1;
        // Arrows sail OVER water — only walls/rocks stop them (js codeAt '~' check).
        let (tc, tr) = (((a.x + 8.0) / 16.0).floor() as i32, ((a.y + 8.0) / 16.0).floor() as i32);
        let over_water = grid.0.code_at(tc, tr) == '~';
        if (!over_water && grid.0.box_hits_solid(a.x + 5.0, a.y + 5.0, 6.0, 6.0))
            || a.x < -16.0
            || a.x > crate::room::PX_W as f32
            || a.y < -16.0
            || a.y > crate::room::PX_H as f32
            || a.life <= 0
        {
            commands.entity(e).despawn();
            continue;
        }
        *hb = Hitbox { x: a.x + 5.0, y: a.y + 5.0, w: 6.0, h: 6.0 };
        let rot = tf.rotation;
        *tf = at(PLAY_X + a.x, PLAY_Y + a.y, 16.0, 16.0, 8.6);
        tf.rotation = rot;
    }
}

pub struct ArcheryPlugin;

impl Plugin for ArcheryPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FireArrow>().add_systems(
            bevy::app::FixedUpdate,
            arrow_tick
                .after(crate::combat::resolve_combat) // reads this tick's landed bites
                .before(super::play::EndTick)
                .run_if(super::screen::playing),
        );
    }
}
