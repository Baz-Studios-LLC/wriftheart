//! skystrike.rs — the STORMCALLER's telegraphed bolt (js skyStrike): it marks the ground
//! under your feet, then the sky answers. A warning ring pulses for ~34 frames (harmless),
//! then a brief bright bolt lands there for a few frames — stand in it and it bites.

use bevy::prelude::*;

use super::battle::RoomActor;
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, HitOnce, Hitbox, Team};
use crate::gfx::{at, PIXEL_LAYER};

/// A pending sky-strike: telegraph while `t` > 0, then a short live-damage window.
#[derive(Component)]
pub struct SkyStrike {
    pub t: i32,
    pub struck: i32,
}

const TELEGRAPH: i32 = 34;
const STRIKE: i32 = 6;

/// Spawn a strike centred on (x, y) in room pixels (no image — a colour ring/bolt).
pub fn spawn(commands: &mut Commands, x: f32, y: f32) {
    commands.spawn((
        Sprite::from_color(Color::srgba(0.5, 0.6, 0.85, 0.35), Vec2::new(20.0, 20.0)),
        at(PLAY_X + x - 2.0, PLAY_Y + y - 2.0, 20.0, 20.0, actor_z(y + 16.0) - 0.2),
        PIXEL_LAYER,
        RoomActor,
        SkyStrike { t: TELEGRAPH, struck: 0 },
        // Harmless while telegraphing; the tick arms it at the flash.
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: None, persistent: true, knock: 0.0 },
        HitOnce::default(),
        Hitbox { x: x + 2.0, y: y + 2.0, w: 12.0, h: 12.0 },
    ));
}

fn skystrike_tick(
    mut commands: Commands,
    mut q: Query<(Entity, &mut SkyStrike, &mut Sprite, &mut Combatant)>,
) {
    for (e, mut s, mut spr, mut cb) in &mut q {
        if s.t > 0 {
            s.t -= 1;
            // Pulse the warning ring brighter as the strike nears.
            let a = 0.25 + 0.5 * (1.0 - s.t as f32 / TELEGRAPH as f32);
            spr.color = Color::srgba(0.55, 0.65, 0.9, a);
            if s.t == 0 {
                // The bolt lands: bright + live for STRIKE frames.
                spr.color = Color::srgba(0.9, 0.95, 1.0, 0.9);
                cb.damage = Some(2);
                s.struck = STRIKE;
            }
        } else {
            s.struck -= 1;
            if s.struck <= 0 {
                commands.entity(e).despawn();
            }
        }
    }
}

pub struct SkyStrikePlugin;

impl Plugin for SkyStrikePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy::app::FixedUpdate,
            skystrike_tick.before(crate::combat::resolve_combat).run_if(super::screen::playing),
        );
    }
}
