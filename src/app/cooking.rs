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
    /// Facing 0 front / 1 right / 2 back / 3 left (js placedTables rot; default keeps
    /// old saves front-facing).
    #[serde(default)]
    pub rot: u8,
    /// Placed INSIDE the player's house (js home tables) — coords are interior-canvas,
    /// and the row only stands up while you're indoors.
    #[serde(default)]
    pub home: bool,
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
    if sliding.0 || in_dungeon.0.is_some() {
        *woke = None;
        return;
    }
    // Home stations stand up while inside YOUR house (js home tables); any other
    // interior has no stations at all. A sentinel key marks the indoor wake.
    let in_house = inside.0.as_ref().is_some_and(|st| st.def.kind == "house");
    if inside.0.is_some() && !in_house {
        *woke = None;
        return;
    }
    let key = if in_house { (i32::MIN, i32::MIN) } else { (cur.rx, cur.ry) };
    if *woke == Some(key) && !stations.is_changed() {
        return;
    }
    *woke = Some(key);
    for e in &live {
        commands.entity(e).despawn();
    }
    let stand = |r: &&StationRec| if in_house { r.home } else { !r.home && r.room == (cur.rx, cur.ry) };
    for rec in stations.0.iter().filter(stand) {
        spawn_fire(&mut commands, &mut images, &mut blockers, rec.x, rec.y, kind_static(&rec.kind), rec.rot);
    }
}

/// Re-intern a saved station kind (a String) back to the &'static id (js craftTable
/// kind), falling back to the cooking fire — every station item is a registered def.
fn kind_static(s: &str) -> &'static str {
    crate::items::get(s).map(|d| d.id).unwrap_or("cook")
}

pub(super) fn spawn_fire(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    blockers: &mut super::room_props::RoomBlockers,
    x: f32,
    y: f32,
    kind: &'static str,
    rot: u8,
) {
    // The cook fire + well keep their symmetric char grids; every table (and the forge)
    // bakes per-facing through the JS-ported vector renderer (station_art::station_image).
    let (img, top) = if kind == "cook" {
        (images.add(bake(&COOKFIRE[..], COOKFIRE_PAL)), y - 8.0)
    } else if kind == "well" {
        (images.add(bake(&super::station_art::WELL[..], super::station_art::WELL_PAL)), y - 8.0)
    } else {
        // Canvas row OY = the station's logical origin, which sits at world y - 8 (the old
        // art's top row) — so the 34-tall canvas draws at y - 8 - OY.
        (super::station_art::station_image(kind, rot, images), y - 8.0 - super::station_art::OY as f32)
    };
    let h = if kind == "cook" || kind == "well" { 22.0 } else { super::station_art::CANVAS.1 as f32 };
    commands.spawn((
        Sprite::from_image(img),
        // Depth-sort at the VISUAL base (the legs end ~y+19), not the js baseY y+24 — the
        // phantom 5px let the bench pop in FRONT of a hero standing at its south face for
        // a beat mid-swing (Baz: "my player's head clips through the workbench").
        at(PLAY_X + x, PLAY_Y + top, 32.0, h, actor_z(y + 19.0)),
        PIXEL_LAYER,
        RoomActor,
        StationSprite { x, y, kind },
    ));
    let blk = (x + 2.0, y + 6.0, 28.0, 9.0); // the js table-base hitbox, kit-anchored
    if !blockers.0.contains(&blk) {
        blockers.0.push(blk);
    }
}

/// The station's use zone (js craftTable useZone, kit-anchored) — shared by the prompt
/// and the interact.
fn use_zone(f: &StationSprite) -> (f32, f32, f32, f32) {
    (f.x - 6.0, f.y - 8.0, 44.0, 30.0)
}

fn in_zone(hitbox: (f32, f32, f32, f32), uz: (f32, f32, f32, f32)) -> bool {
    hitbox.0 < uz.0 + uz.2 && hitbox.0 + hitbox.2 > uz.0 && hitbox.1 < uz.1 + uz.3 && hitbox.1 + hitbox.3 > uz.1
}

#[derive(Component, Clone)]
pub struct StationPrompt;

/// The "CRAFT" prompt while you stand at a station (js 'A CRAFT' — Baz: "the workbench
/// should have a prompt"). Wells stay quiet (they only refill the can).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn station_prompt(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    bindings: Res<crate::input::Bindings>,
    input: Res<ActionState>,
    players: Query<&Player>,
    fires: Query<&StationSprite>,
    old: Query<Entity, With<StationPrompt>>,
    mut last: Local<Option<(i32, i32)>>,
) {
    let Ok(p) = players.single() else { return };
    let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    let near = fires.iter().any(|f| f.kind != "well" && in_zone(hitbox, use_zone(f)));
    // The shared by-the-character bubble (prompts.rs), re-anchored as the hero moves.
    let key = near.then_some((p.x as i32, p.y as i32));
    if *last == key {
        return;
    }
    *last = key;
    for e in &old {
        commands.entity(e).despawn();
    }
    if near {
        let text = format!("{} CRAFT", bindings.prompt(Action::Interact, input.pad_present));
        super::prompts::spawn_bubble(&mut commands, &mut images, &text, p.x + 8.0, p.y - 10.0, StationPrompt);
    }
}

/// PRESS beside a fire -> the CRAFT page opens in station mode (js craftStation).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn station_interact(
    mut commands: Commands,
    mut input: ResMut<ActionState>,
    mut craft: ResMut<super::slideout::craft_tab::CraftState>,
    mut so: ResMut<super::slideout::SlideOut>,
    mut next: ResMut<NextState<super::screen::Screen>>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&Player>,
    fires: Query<&StationSprite>,
    prompts: Query<Entity, With<StationPrompt>>,
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
        if in_zone(hitbox, use_zone(f)) {
            input.consume(Action::Interact);
            for e in &prompts {
                commands.entity(e).despawn(); // the window replaces the prompt
            }
            craft.station = Some(f.kind);
            craft.station_at = Some((f.x, f.y));
            craft.remove_requested = false;
            craft.cursor = 0;
            craft.scroll = 0;
            craft.rune_cursor = 0;
            so.tab = 0; // the station panel's OWN tab rows start at its CRAFT page
            // Build + slide the panel NOW (the keyboard openers' reset): without the dirty
            // flag the first SlideOut tick drew NOTHING until some input tripped a redraw
            // (Baz: "there is a delay when I open the workbench").
            so.dirty = true;
            so.held = None;
            so.hold_act = None;
            so.anim = 0.0;
            so.applied = 0.0;
            next.set(super::screen::Screen::SlideOut);
            sfx.write(super::sfx::Sfx("open"));
            return;
        }
    }
}

/// REMOVE TABLE (js removeTable): the craft window's Slot4 flagged it — tear the placed
/// station down, refund HALF each material (floor, js Math.floor(q/2)), and close back
/// to play. Runs under Screen::SlideOut (the window is open when the flag trips).
#[allow(clippy::too_many_arguments)]
fn station_remove(
    mut commands: Commands,
    mut craft: ResMut<super::slideout::craft_tab::CraftState>,
    cur: Res<CurRoom>,
    mut stations: ResMut<PlacedStations>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut next: ResMut<NextState<super::screen::Screen>>,
    inside: Res<super::interior::Inside>,
    fires: Query<(Entity, &StationSprite)>,
) {
    if !craft.remove_requested {
        return;
    }
    craft.remove_requested = false;
    let (Some(kind), Some((x, y))) = (craft.station, craft.station_at) else { return };
    // Which side of the door we're removing from (a home table can share coords with
    // an outdoor one in the same room — the flag keeps them distinct).
    let in_house = inside.0.as_ref().is_some_and(|st| st.def.kind == "house");
    // Half the build cost comes back (the station's own recipe, wherever it's crafted).
    if let Some(r) = crate::recipes_data::RECIPES.iter().find(|r| r.out == kind) {
        for (id, q) in r.cost {
            if q / 2 > 0 {
                inv.add_item(id, q / 2);
            }
        }
    }
    stations.0.retain(|s| !(s.room == (cur.rx, cur.ry) && s.x == x && s.y == y && s.home == in_house));
    for (e, f) in &fires {
        if f.x == x && f.y == y {
            commands.entity(e).despawn();
        }
    }
    blockers.0.retain(|b| *b != (x + 2.0, y + 6.0, 28.0, 9.0));
    craft.station = None;
    craft.station_at = None;
    log.add("cook", "REMOVED (HALF MATS BACK)", 1, 0xd0d0d0, false, true);
    sfx.write(super::sfx::Sfx("stone"));
    saves.write(super::save::SaveRequest);
    next.set(super::screen::Screen::Play);
}

pub struct CookingPlugin;

impl Plugin for CookingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacedStations>().add_message::<PlaceStation>().add_systems(
            bevy::app::FixedUpdate,
            (station_wake, station_prompt, station_interact.before(super::talk::talk_tick))
                .run_if(super::screen::playing),
        )
        .add_systems(
            bevy::app::FixedUpdate,
            station_remove
                .before(super::play::EndTick)
                .run_if(bevy::state::condition::in_state(super::screen::Screen::SlideOut)),
        );
    }
}
