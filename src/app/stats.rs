//! stats.rs — the LEDGER OF DEEDS counters (js game.js `stats` + the bump() sites).
//!
//! One [`Stats`] map, keyed exactly like the js save (`kills`, `kill_goblin`, `walk`,
//! `frames`, ...). Counters whose systems haven't ported yet simply stay at zero — the
//! STATS codex tab renders the FULL js line list either way (the js also prints zeros).
//!
//! Bump sites live where the deed happens (battle deaths, pickup collection, node
//! deaths); the cross-cutting ones that would bloat those systems — playtime, tiles
//! walked, HP lost, deaths — accrue here in one observer system.

use super::battle::not_sliding;
use super::play::Player;
use super::screen::playing;
use crate::combat::{Health, HitLanded};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Stats(pub HashMap<String, f64>);

impl Stats {
    pub fn bump(&mut self, key: &str, n: f64) {
        *self.0.entry(key.to_string()).or_insert(0.0) += n;
    }
    /// The per-victim tally (js bump('kill_' + type)) — feeds FAVORITE VICTIM.
    pub fn bump_kill(&mut self, kind: &str) {
        self.bump(&format!("kill_{kind}"), 1.0);
    }
    pub fn get(&self, key: &str) -> f64 {
        self.0.get(key).copied().unwrap_or(0.0)
    }
}

pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Stats>().add_systems(
            FixedUpdate,
            stats_tick
                .run_if(playing)
                .run_if(not_sliding)
                .after(crate::combat::resolve_combat)
                .before(super::play::EndTick),
        );
    }
}

/// Playtime, tiles walked, HP lost, and deaths — read from the world each active tick.
fn stats_tick(
    mut stats: ResMut<Stats>,
    mut hits: MessageReader<HitLanded>,
    players: Query<(Entity, &Player, &Health)>,
    mut prev: Local<Option<(f32, f32, i32)>>,
) {
    stats.bump("frames", 1.0);
    let Ok((pe, p, h)) = players.single() else { return };
    // HP lost, lifetime (js bump('dmg') on every player hurt).
    for hit in hits.read() {
        if hit.target == pe {
            stats.bump("dmg", hit.dealt as f64);
            // Deaths: this hit put the hero down (play.rs respawns before the next tick,
            // so the moment is only visible here).
            if h.hp <= 0 {
                stats.bump("deaths", 1.0);
            }
        }
    }
    // Tiles walked (js accrues it as a float: distance / tile).
    if let Some((px, py, _)) = *prev {
        let d = (p.x - px).abs() + (p.y - py).abs();
        if d > 0.0 && d < 32.0 {
            // (room slides teleport the hero a screen over — not a walk)
            stats.bump("walk", (d / 16.0) as f64);
        }
    }
    *prev = Some((p.x, p.y, h.hp));
}
