//! home.rs — the BUILDABLE HOME (js playerHouse + placeHouse + Entities.house): craft the
//! House at a workbench, then set it down in the overworld. ONE home per save (placing a
//! second RELOCATES it). Standing at its door and pressing INTERACT enters the "house"
//! interior — the one with the BED (sleep) and the STORAGE CHEST (storage.rs). Its room +
//! spot ride the save (PlayerHouse); it re-spawns whenever you walk into that room.
//!
//! Placement mirrors the cooking-fire idiom (place-at-feet) — the js ghost-placement +
//! rotation stay the flagged crafting-overhaul deviation. The world sprite reuses the
//! town "home" front (PropArt.fronts) so the built home matches the cottages you visit.
//! v1 defers PACK-UP (removing it for a mat refund) and the respawn-at-home warp.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::battle::RoomActor;
use super::play::{CurRoom, Player};
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
    spawn_house(&mut commands, &art, &mut blockers, rec.x, rec.y);
}

fn spawn_house(commands: &mut Commands, art: &super::super::actors::props::PropArt, blockers: &mut super::room_props::RoomBlockers, x: f32, y: f32) {
    // The town "home" front (48x48), anchored the js townBuilding way: sprite up-left of
    // the base tile, depth-sorted at y+16, the cabin body blocking (the doorway opens).
    if let Some(img) = art.fronts.get("home") {
        commands.spawn((
            Sprite::from_image(img.clone()),
            at(PLAY_X + x - 16.0, PLAY_Y + y - 32.0, 48.0, 48.0, actor_z(y + 16.0)),
            PIXEL_LAYER,
            RoomActor,
            HouseSprite,
        ));
    }
    let blk = (x - 12.0, y - 28.0, 40.0, 42.0);
    if !blockers.0.contains(&blk) {
        blockers.0.push(blk);
    }
}

/// Using the House kit sets it down at your feet — overworld wilds/rooms only (js placeHouse
/// refuses towns; interiors + dungeons have no ground to claim). One home per save.
#[allow(clippy::too_many_arguments)]
pub fn place_house(
    mut uses: MessageReader<PlaceHouse>,
    mut commands: Commands,
    cur: Res<CurRoom>,
    world: Res<super::play::GameWorld>,
    art: Res<super::super::actors::props::PropArt>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    grid: Res<super::play::CurGrid>,
    mut house: ResMut<PlayerHouse>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    live: Query<Entity, With<HouseSprite>>,
    players: Query<&Player>,
) {
    for PlaceHouse in uses.read() {
        let Ok(p) = players.single() else { continue };
        if in_dungeon.0.is_some()
            || inside.0.is_some()
            || crate::worldgen::towns::town_role(world.0.seed, cur.rx, cur.ry).is_some()
        {
            log.add("home", "NO PLACE FOR A HOME HERE", 1, 0xfc8868, false, true);
            sfx.write(super::sfx::Sfx("tink"));
            continue;
        }
        // Snap the door to the tile under the hero's feet (the coop idiom).
        let c = ((p.x + 8.0) / 16.0).round() as i32;
        let r = ((p.y + 24.0) / 16.0).floor() as i32;
        let (x, y) = ((c * 16) as f32, (r * 16) as f32);
        // The cabin body must land on clear ground (its blocker footprint, minus the door row).
        let clear = !grid.0.box_hits_solid(x - 11.0, y - 27.0, 38.0, 34.0)
            && !blockers.0.iter().any(|b| x - 12.0 < b.0 + b.2 && x + 28.0 > b.0 && y - 28.0 < b.1 + b.3 && y + 6.0 > b.1);
        if !clear {
            log.add("home", "NO ROOM TO RAISE A HOME", 1, 0xfc8868, false, true);
            sfx.write(super::sfx::Sfx("tink"));
            continue;
        }
        // One home per save: pull down the old one (its blocker + sprite) before moving.
        if let Some(old) = &house.0 {
            let ob = (old.x - 12.0, old.y - 28.0, 40.0, 42.0);
            blockers.0.retain(|b| *b != ob);
        }
        for e in &live {
            commands.entity(e).despawn();
        }
        inv.remove_one("house");
        house.0 = Some(HouseRec { room: (cur.rx, cur.ry), x, y });
        if rec_here(&house, cur.rx, cur.ry) {
            spawn_house(&mut commands, &art, &mut blockers, x, y);
        }
        log.add("home", "YOUR HOME STANDS", 1, 0xcfe0ff, false, true);
        sfx.write(super::sfx::Sfx("craft"));
        saves.write(super::save::SaveRequest);
    }
}

fn rec_here(house: &PlayerHouse, rx: i32, ry: i32) -> bool {
    house.0.as_ref().is_some_and(|h| h.room == (rx, ry))
}

pub struct HomePlugin;

impl Plugin for HomePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerHouse>().add_message::<PlaceHouse>().add_systems(
            bevy::app::FixedUpdate,
            (house_wake, place_house.after(house_wake)).run_if(super::screen::playing),
        );
    }
}
