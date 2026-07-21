//! home.rs — the BUILDABLE HOME (js playerHouse + placeHouse + Entities.house): craft the
//! House at a workbench, then set it down in the overworld. ONE home per save (placing a
//! second RELOCATES it). Standing at its door and pressing INTERACT enters the "house"
//! interior — the one with the BED (sleep) and the STORAGE CHEST (storage.rs). Its room +
//! spot ride the save (PlayerHouse); it re-spawns whenever you walk into that room.
//!
//! Placement goes through the shared GHOST mode (placing.rs): a movable validity-tinted
//! reticle, confirmed like the js. The world sprite is the bespoke FARMHOUSE
//! (buildings_art::FARMHOUSE). Still deferred: house pack-up + the respawn-at-home warp.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::battle::RoomActor;
use super::play::CurRoom;
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::gfx::{at, PIXEL_LAYER};

/// The one placed home (js playerHouse), saved. None until you build one.
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct PlayerHouse(pub Option<HouseRec>);

#[derive(Clone, Serialize, Deserialize)]
pub struct HouseRec {
    pub room: (i32, i32),
    pub x: f32,
    pub y: f32,
}

/// The chosen death-respawn point (SET SPAWN at your bed or an inn), saved. None =
/// the start room, the old behavior.
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct RespawnPoint(pub Option<RespawnRec>);

#[derive(Clone, Serialize, Deserialize)]
pub struct RespawnRec {
    pub room: (i32, i32),
    pub x: f32,
    pub y: f32,
}

/// play.rs slot-use -> validate + place here (the PlaceStation idiom).
#[derive(Message)]
pub struct PlaceHouse;

#[derive(Component)]
pub struct HouseSprite;

/// The player-house door zone (town-building convention: a 24x18 strip just below the
/// base). interior.rs adds this to its door candidates so entry reuses the shared path.
pub fn door_zone(x: f32, y: f32) -> (f32, f32, f32, f32) {
    (x - 4.0, y + 8.0, 24.0, 18.0)
}

/// Re-stand the home when the player walks into its room (the station_wake idiom).
#[allow(clippy::too_many_arguments)]
pub fn house_wake(
    mut commands: Commands,
    cur: Res<CurRoom>,
    house: Res<PlayerHouse>,
    art: Res<super::super::actors::props::PropArt>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    active: Res<super::play::ActiveRoot>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut woke: Local<Option<(i32, i32)>>,
    live: Query<Entity, With<HouseSprite>>,
) {
    if in_dungeon.0.is_some() || inside.0.is_some() {
        *woke = None;
        return;
    }
    if *woke == Some((cur.rx, cur.ry)) && !house.is_changed() {
        return;
    }
    *woke = Some((cur.rx, cur.ry));
    for e in &live {
        commands.entity(e).despawn();
    }
    let Some(rec) = &house.0 else { return };
    if rec.room != (cur.rx, cur.ry) {
        return;
    }
    spawn_house(&mut commands, &art, &mut blockers, active.0, rec.x, rec.y);
}

fn spawn_house(commands: &mut Commands, art: &super::super::actors::props::PropArt, blockers: &mut super::room_props::RoomBlockers, root: Entity, x: f32, y: f32) {
    // The bespoke FARMHOUSE (48x52 — taller than the old town "home" front it replaced,
    // so it anchors 4px higher; the base tile, blocker and door zone are unchanged).
    // A CHILD of the room root, so an edge slide carries it in with its room (it used to
    // spawn parked at its final spot mid-scroll — Baz: "the house doesn't transition").
    let e = commands
        .spawn((
            Sprite::from_image(art.farmhouse.clone()),
            at(PLAY_X + x - 16.0, PLAY_Y + y - 36.0, 48.0, 52.0, actor_z(y + 16.0)),
            PIXEL_LAYER,
            RoomActor,
            HouseSprite,
        ))
        .id();
    commands.entity(root).add_child(e);
    let blk = (x - 12.0, y - 28.0, 40.0, 42.0);
    if !blockers.0.contains(&blk) {
        blockers.0.push(blk);
    }
}

/// Set the home down at (x, y) — the ghost-placement confirm (placing.rs) has already
/// validated the ground, refused towns, and paid the kit. One home per save: placing a
/// second RELOCATES it (the old blocker + sprite come down first).
#[allow(clippy::too_many_arguments)]
pub(super) fn confirm_house(
    commands: &mut Commands,
    art: &super::super::actors::props::PropArt,
    blockers: &mut super::room_props::RoomBlockers,
    house: &mut PlayerHouse,
    live: &Query<Entity, With<HouseSprite>>,
    root: Entity,
    room: (i32, i32),
    x: f32,
    y: f32,
) {
    if let Some(old) = &house.0 {
        let ob = (old.x - 12.0, old.y - 28.0, 40.0, 42.0);
        blockers.0.retain(|b| *b != ob);
    }
    for e in live {
        commands.entity(e).despawn();
    }
    house.0 = Some(HouseRec { room, x, y });
    spawn_house(commands, art, blockers, root, x, y);
}

pub struct HomePlugin;

impl Plugin for HomePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerHouse>()
            .init_resource::<RespawnPoint>()
            .add_message::<PlaceHouse>()
            .add_systems(bevy::app::FixedUpdate, house_wake.run_if(super::screen::playing));
    }
}
