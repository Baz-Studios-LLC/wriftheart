//! cooking.rs — the COOKING FIRE, the port's first placeable crafting station (js
//! craftTable kind 'cook' + placedTables): use the kit from a slot and the fire
//! stands at your feet (the coop/barn place-at-feet idiom — js's ghost-placement
//! mode is flagged for the crafting overhaul, along with the workbench + blueprint
//! chain). Placed fires persist per room (PlacedStations rides the save), block
//! like furniture, and PRESS beside one opens the slide-out CRAFT page in station
//! mode — the cook recipes (meat/herbs/crops/@FISH) instead of the hand list.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::battle::RoomActor;
use super::play::{CurRoom, Player, SlideActive};
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::gfx::{at, bake, PIXEL_LAYER};
use crate::input::{Action, ActionState};

/// One placed station (js placedTables rows) — kind stays a String so later
/// stations (forge, alchemy...) ride the same list without a save migration.
#[derive(Clone, Serialize, Deserialize)]
pub struct StationRec {
    pub room: (i32, i32),
    pub x: f32,
    pub y: f32,
    pub kind: String,
}

/// Every placed station, saved (js placedTables).
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct PlacedStations(pub Vec<StationRec>);

/// play.rs slot-use -> validate + place here (the UseFarmItem idiom).
#[derive(Message)]
pub struct PlaceStation(pub &'static str);

#[derive(Component)]
pub struct StationSprite {
    pub x: f32,
    pub y: f32,
    pub kind: &'static str,
}

/// The camp kitchen, hand-drawn to the js cook-table look: a dark oak work table
/// (top 6b4a2a / lite 86603a / legs 3f2a14) bearing the stew pot on its trivet,
/// embers glowing underneath, steam curling off the lid.
const COOKFIRE: [&str; 22] = [
    "..............s.................",
    "..........t...s.................",
    "..........t.....t...............",
    "..........t.....t...............",
    "................t...............",
    "............RRRRRRRR............",
    "...........RKKKKKKKKR...........",
    "...........RKKKKKKKKR...........",
    "...........RKKKKKKKKR...........",
    "..........rrrrrrrrrrrr..........",
    "............EEFFFFEE............",
    ".............EFFFFE.............",
    "LLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLL",
    "TTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTT",
    "TLTTTTTTTTTTTTTTTTTTTTTTTTTTTLTT",
    "TTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTT",
    "DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD",
    ".DD..........................DD.",
    ".DD..........................DD.",
    ".DD..........................DD.",
    ".dd..........................dd.",
    "................................",
];

const COOKFIRE_PAL: &[(char, u32)] = &[
    ('s', 0xbfbfbf), // steam, bright wisp
    ('t', 0x9f9f9f), // steam, faint wisp
    ('R', 0x4a4a4a), // pot rim
    ('K', 0x2a2a2a), // pot body
    ('r', 0x3a3a3a), // pot base ring
    ('E', 0xb5651d), // embers
    ('F', 0xd0822a), // embers, hot core
    ('L', 0x86603a), // table top, lit edge
    ('T', 0x6b4a2a), // table top
    ('D', 0x3f2a14), // legs
    ('d', 0x2a1c0e), // feet shadow
];

/// Stand the current room's fires up (the yard_wake idiom: sweep-and-restand keyed
/// on the room; RoomActor rides them so room swaps sweep them too).
#[allow(clippy::too_many_arguments)]
pub fn station_wake(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cur: Res<CurRoom>,
    sliding: Res<SlideActive>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    stations: Res<PlacedStations>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut woke: Local<Option<(i32, i32)>>,
    live: Query<Entity, With<StationSprite>>,
) {
    if sliding.0 || in_dungeon.0.is_some() || inside.0.is_some() {
        *woke = None;
        return;
    }
    if *woke == Some((cur.rx, cur.ry)) && !stations.is_changed() {
        return;
    }
    *woke = Some((cur.rx, cur.ry));
    for e in &live {
        commands.entity(e).despawn();
    }
    for rec in stations.0.iter().filter(|r| r.room == (cur.rx, cur.ry)) {
        spawn_fire(&mut commands, &mut images, &mut blockers, rec.x, rec.y, kind_static(&rec.kind));
    }
}

/// Re-intern a saved station kind (a String) back to the &'static id (js craftTable
/// kind), falling back to the cooking fire — every station item is a registered def.
fn kind_static(s: &str) -> &'static str {
    crate::items::get(s).map(|d| d.id).unwrap_or("cook")
}

fn spawn_fire(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    blockers: &mut super::room_props::RoomBlockers,
    x: f32,
    y: f32,
    kind: &'static str,
) {
    let (grid, pal) = if kind == "cook" {
        (&COOKFIRE[..], COOKFIRE_PAL)
    } else {
        super::station_art::station_art(kind)
    };
    let img = images.add(bake(grid, pal));
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + x, PLAY_Y + y - 8.0, 32.0, 22.0, actor_z(y + 24.0)),
        PIXEL_LAYER,
        RoomActor,
        StationSprite { x, y, kind },
    ));
    let blk = (x + 2.0, y + 6.0, 28.0, 9.0); // the js table-base hitbox, kit-anchored
    if !blockers.0.contains(&blk) {
        blockers.0.push(blk);
    }
}

/// Using the kit places the fire at your feet — overworld wilds only (js tables
/// refuse towns; interiors and dungeons have no ground to claim).
#[allow(clippy::too_many_arguments)]
pub fn place_station(
    mut uses: MessageReader<PlaceStation>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cur: Res<CurRoom>,
    world: Res<super::play::GameWorld>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    grid: Res<super::play::CurGrid>,
    mut stations: ResMut<PlacedStations>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&Player>,
) {
    for PlaceStation(kind) in uses.read() {
        let Ok(p) = players.single() else { continue };
        if in_dungeon.0.is_some()
            || inside.0.is_some()
            || crate::worldgen::towns::town_role(world.0.seed, cur.rx, cur.ry).is_some()
        {
            log.add("cook", "NO PLACE FOR A CAMP HERE", 1, 0xfc8868, false, true);
            sfx.write(super::sfx::Sfx("tink"));
            continue;
        }
        // Snap the 2x1 base to the tile under the hero's feet (the coop idiom).
        let c = ((p.x + 8.0) / 16.0).round() as i32 - 1;
        let r = ((p.y + 20.0) / 16.0).floor() as i32;
        let (x, y) = ((c * 16) as f32, (r * 16) as f32);
        let clear = (0..2).all(|i| !grid.0.box_hits_solid(x + 1.0 + i as f32 * 16.0, y + 1.0, 14.0, 14.0))
            && !blockers.0.iter().any(|b| x < b.0 + b.2 && x + 32.0 > b.0 && y < b.1 + b.3 && y + 16.0 > b.1);
        if !clear {
            log.add("cook", "NO ROOM TO SET THE FIRE", 1, 0xfc8868, false, true);
            sfx.write(super::sfx::Sfx("tink"));
            continue;
        }
        inv.remove_one(kind);
        stations.0.push(StationRec { room: (cur.rx, cur.ry), x, y, kind: kind.to_string() });
        spawn_fire(&mut commands, &mut images, &mut blockers, x, y, kind);
        let (line, col) = if *kind == "cook" {
            ("THE COOKING FIRE CRACKLES", 0xd0822a)
        } else {
            super::station_art::place_msg(kind)
        };
        log.add("cook", line, 1, col, false, true);
        sfx.write(super::sfx::Sfx("craft"));
        saves.write(super::save::SaveRequest);
    }
}

/// PRESS beside a fire -> the CRAFT page opens in station mode (js craftStation).
pub fn station_interact(
    mut input: ResMut<ActionState>,
    mut craft: ResMut<super::slideout::craft_tab::CraftState>,
    mut so: ResMut<super::slideout::SlideOut>,
    mut next: ResMut<NextState<super::screen::Screen>>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&Player>,
    fires: Query<&StationSprite>,
) {
    if !input.pressed(Action::Interact) {
        return;
    }
    let Ok(p) = players.single() else { return };
    let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    for f in &fires {
        if f.kind == "well" {
            continue; // a well has no craft menu — it just refills the can (farm.rs)
        }
        let uz = (f.x - 6.0, f.y - 8.0, 44.0, 30.0); // the js useZone, kit-anchored
        if hitbox.0 < uz.0 + uz.2 && hitbox.0 + hitbox.2 > uz.0 && hitbox.1 < uz.1 + uz.3 && hitbox.1 + hitbox.3 > uz.1 {
            input.consume(Action::Interact);
            craft.station = Some(f.kind);
            craft.cursor = 0;
            craft.scroll = 0;
            so.tab = 1; // the CRAFT page
            next.set(super::screen::Screen::SlideOut);
            sfx.write(super::sfx::Sfx("open"));
            return;
        }
    }
}

pub struct CookingPlugin;

impl Plugin for CookingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacedStations>().add_message::<PlaceStation>().add_systems(
            bevy::app::FixedUpdate,
            (station_wake, place_station.after(station_wake), station_interact.before(super::talk::talk_tick))
                .run_if(super::screen::playing),
        );
    }
}
