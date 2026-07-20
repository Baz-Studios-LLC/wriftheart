//! room_cache.rs — the js roomCache: SAME-DAY EXACT RESTORE of a room's live layer.
//!
//! Step back into a room you left this morning and it is byte-for-byte what you left:
//! every surviving foe at its position with its health, every coin and item still on
//! the ground. At dawn the snapshot goes stale and the world refreshes as ever. The
//! STATIC layer — cut trees (TreeGrowth), mined rocks / picked bushes (GatherState's
//! daily stamps) — is already consistent via records and never lives here.
//!
//! IMPROVE-DON'T-COPY: the js snapshots on room-leave inside loadRoomEntities; here a
//! FixedPostUpdate system snapshots the CURRENT room every settled tick, so no leave
//! path (edge slide, door, death respawn) can ever miss one — the cache always holds
//! the room's last live state. In-memory only, like the js: loads regen from records.
//!
//! DEVIATIONS (flagged): mob AI timers reset on restore (the js re-seats the whole
//! object, aggro included — ours re-aggros on sight, imperceptible); ground drops
//! restore in TOWNS too (the js skips towns and loses them).

use super::battle::{spawn_room_mobs, RoomActor};
use super::gather::{farm_day, spawn_coin, spawn_pickup, Pickup, PickupKind};
use super::play::CurRoom;
use crate::actors::goblin::{goblin_bundle, Goblin, GoblinKind};
use crate::actors::mobs::{mob_bundle, Mob};
use crate::combat::Health;
use crate::gfx::PIXEL_LAYER;
use crate::worldgen::entities::RoomEntity;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// One live foe, as respawnable data (js cacheableMob — bosses/puppets join with
/// their systems; every current kind qualifies).
pub enum MobSnap {
    Mob { def: usize, x: f32, y: f32, hp: i32, max: i32 },
    /// `skin`: a humanoid rider on the goblin chassis (bandits) — (kind, look seed),
    /// so a restored bandit comes back the same PERSON, not a goblin.
    Goblin { kind: GoblinKind, x: f32, y: f32, hp: i32, max: i32, skin: Option<(&'static str, u32)> },
}

/// One loose ground drop (js `e.pickup` — placed items and tomes are record-driven
/// and respawn on their own).
pub struct DropSnap {
    pub kind: DropKind,
    pub x: f32,
    pub y: f32,
    pub life: u32,
}

pub enum DropKind {
    Item { id: &'static str, qty: i32 },
    Coin(i32),
}

pub struct RoomSnap {
    pub day: i64,
    pub mobs: Vec<MobSnap>,
    pub drops: Vec<DropSnap>,
}

/// "rx,ry" -> the room's live layer as of the last settled tick there (js roomCache).
#[derive(Resource, Default)]
pub struct RoomCache(pub HashMap<(i32, i32), RoomSnap>);

pub struct RoomCachePlugin;

impl Plugin for RoomCachePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoomCache>().add_systems(
            // FixedPostUpdate: the battle chain's spawns/deaths (and a death's bag
            // scatter) are flushed by now, so the snapshot sees the tick's true end
            // state. not_sliding: mid-slide `cur` already names the NEXT room — a
            // snapshot then would clobber that room's real snap with an empty one.
            bevy::app::FixedPostUpdate,
            snapshot_room.run_if(super::battle::not_sliding),
        );
    }
}

/// Record the current room's live layer (runs every settled play tick; cheap — a few
/// dozen small copies). Also drops yesterday's snapshots, the js dawn rule.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn snapshot_room(
    mut cache: ResMut<RoomCache>,
    cur: Res<CurRoom>,
    inside: Res<super::interior::Inside>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    clock: Res<super::room_render::FrameClock>,
    mobs: Query<(&Mob, &Health), Without<crate::app::quests::BountyTag>>,
    goblins: Query<(&Goblin, &Health, Option<&crate::actors::goblin::HumanSkin>), Without<crate::app::quests::BountyTag>>,
    drops: Query<&Pickup>,
) {
    if inside.0.is_some() || in_dungeon.0.is_some() {
        return; // an interior/dungeon floor is not the overworld room (js mode check)
    }
    let today = farm_day(clock.0); // rooms reset at DAWN (Baz) — one refresh with trees + crops
    cache.0.retain(|_, s| s.day == today);
    let mut snap = RoomSnap { day: today, mobs: Vec::new(), drops: Vec::new() };
    for (m, h) in &mobs {
        if h.hp > 0 {
            snap.mobs.push(MobSnap::Mob { def: m.def, x: m.x, y: m.y, hp: h.hp, max: h.max });
        }
    }
    for (g, h, skin) in &goblins {
        if h.hp > 0 {
            snap.mobs.push(MobSnap::Goblin {
                kind: g.kind,
                x: g.x,
                y: g.y,
                hp: h.hp,
                max: h.max,
                skin: skin.map(|s| (s.kind, s.seed)),
            });
        }
    }
    for p in &drops {
        if p.tile.is_some() {
            continue; // placed items live in GatherState.placed, forever
        }
        let kind = match p.kind {
            PickupKind::Item { id, qty } => DropKind::Item { id, qty },
            PickupKind::Coin(v) => DropKind::Coin(v),
            PickupKind::Book(_) => continue, // tomes respawn from their own records
        };
        snap.drops.push(DropSnap { kind, x: p.x, y: p.y, life: p.life });
    }
    cache.0.insert((cur.rx, cur.ry), snap);
}

/// Stand up a room's dynamic layer: a same-day snapshot re-seats EXACTLY what was
/// left (js model B); otherwise the fresh deterministic roll (spawn_room_mobs).
#[allow(clippy::too_many_arguments)] // room composition needs the room's whole context
pub fn spawn_or_restore(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    rng: &mut super::battle::GameRng,
    human_art: &mut crate::actors::goblin::HumanArt,
    cache: &RoomCache,
    world: &crate::worldgen::World,
    cleared: &super::encounters::ClearedEncounters,
    armed: &mut super::encounters::ArmedEncounter,
    ents: &[RoomEntity],
    room: (i32, i32),
    today: i64,
) {
    let Some(snap) = cache.0.get(&room).filter(|s| s.day == today) else {
        spawn_room_mobs(commands, images, rng, human_art, world, cleared, armed, ents, room);
        return;
    };
    // A same-day return to an encounter room: every restored foe is the camp's (the
    // encounter replaced the natural roll), so re-mark + re-arm — killing the last
    // survivor still clears the camp. Zero survivors restored = it clears next tick.
    let enc_room = !cleared.0.contains(&room)
        && super::encounters::for_room(world, room.0, room.1).is_some_and(|(d, _)| !d.friendly);
    if enc_room {
        armed.0 = Some(room);
    }
    for m in &snap.mobs {
        match *m {
            MobSnap::Mob { def, x, y, hp, max } => {
                let mut e = commands.spawn((mob_bundle(def, x, y), RoomActor, PIXEL_LAYER));
                if enc_room {
                    e.insert(super::encounters::EncFoe);
                }
                e.entry::<Health>().and_modify(move |mut h| {
                    h.hp = hp;
                    h.max = max;
                });
            }
            MobSnap::Goblin { kind, x, y, hp, max, skin } => {
                let mut e = commands.spawn((goblin_bundle(kind, x, y), RoomActor, PIXEL_LAYER));
                e.insert(Sprite::default());
                if let Some((skind, seed)) = skin {
                    let frames = human_art.frames(skind, seed, images);
                    e.insert(crate::actors::goblin::HumanSkin { kind: skind, seed, frames });
                }
                if enc_room {
                    e.insert(super::encounters::EncFoe);
                }
                e.entry::<Health>().and_modify(move |mut h| {
                    h.hp = hp;
                    h.max = max;
                });
            }
        }
    }
    for d in &snap.drops {
        let e = match d.kind {
            DropKind::Item { id, qty } => spawn_pickup(commands, images, id, qty, d.x, d.y, false),
            DropKind::Coin(v) => spawn_coin(commands, images, v, d.x, d.y),
        };
        // Re-seat as it lay: keep its remaining life, skip the spawn-pop arc.
        let life = d.life;
        commands.entity(e).entry::<Pickup>().and_modify(move |mut p| {
            p.life = life;
            p.t = 10;
            p.vy = 0.0;
        });
    }
}
