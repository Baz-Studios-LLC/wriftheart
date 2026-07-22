//! prompts.rs — the floating interact bubble (the js on-object prompt, game.js ~5235):
//! a small dark bar with the DERIVED interact key — "F ENTER" over the player at a
//! doorway, "F TAKE" over a lore tome at your feet — and the press-to-take for books
//! (doors take theirs in interior.rs). Doors outrank books, the js priority.

use super::gather::{GatherState, Pickup, PickupKind};
use super::interior::Inside;
use super::play::{CurRoom, GameWorld, Player};
use super::room_render::{PLAY_X, PLAY_Y};
use super::screen::playing;
use crate::gfx::{at, font, layers, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::ui::label;
use bevy::prelude::*;

/// The tome-take working set (grouped under Bevy's 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct TakeCtx<'w, 's> {
    pub gathered: ResMut<'w, GatherState>,
    pub log: ResMut<'w, super::rewards::LootLog>,
    pub farm: Res<'w, super::farm::FarmTiles>,
    pub inv: Res<'w, crate::inventory::PlayerInv>,
    pub learned: ResMut<'w, super::flute::LearnedSongs>,
    pub villagers: Query<'w, 's, &'static crate::actors::villager::Villager>,
    pub wanderers: Query<'w, 's, &'static super::encounters::Wanderer>,
    pub other_bubbles: Query<'w, 's, (), (With<AnyBubble>, Without<PromptUi>)>,
}

#[derive(Component, Clone)]
pub(crate) struct PromptUi;

/// Worn by EVERY bubble — spawn_bubble's AND ui::speech_bubble's (the town chat
/// path missed it and the TALK plate overprinted a live speech line — Baz) —
/// the presence check that keeps two prompt systems from stacking their bubbles
/// on the same head. Lives in ui::widgets so the shared recipe can wear it.
pub use crate::ui::AnyBubble;

/// What the bubble said last tick (rebuild only when it changes; position rides along).
#[derive(Default)]
pub struct LastPrompt(Option<(String, i32, i32)>);

/// The current room's cached door zones (rects re-derived on room change).
pub(crate) type DoorCache = Option<((i32, i32), Vec<(f32, f32, f32, f32)>)>;

pub struct PromptsPlugin;

impl Plugin for PromptsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy::app::FixedUpdate,
            prompt_tick.before(super::play::EndTick).run_if(playing),
        );
    }
}

fn overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub(crate) fn prompt_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    bindings: Res<Bindings>,
    world: Res<GameWorld>,
    cur: Res<CurRoom>,
    inside: Res<Inside>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    mut tk: TakeCtx,
    players: Query<&Player>,
    books: Query<(Entity, &Pickup)>,
    wild: Query<&super::farm::WildCrop>,
    old: Query<Entity, With<PromptUi>>,
    mut last: Local<LastPrompt>,
    mut doors: Local<DoorCache>,
    house: Res<super::home::PlayerHouse>,
) {
    let Ok(p) = players.single() else { return };
    let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);

    // Door zones, cached per room (derived from the entity layout, like door_enter).
    // Suppressed underground: in a dungeon, cur.rx/ry still names the OVERWORLD room that
    // holds the entrance, so its "dungeon" entity would raise a phantom ENTER prompt on
    // whatever dungeon tile happens to sit in the zone (Baz: "an enter prompt in the boss
    // room"). The overworld's doors aren't reachable from underground.
    let at_door = if inside.0.is_some() || in_dungeon.0.is_some() {
        false
    } else {
        if doors.as_ref().map(|(room, _)| *room) != Some((cur.rx, cur.ry)) {
            // Every "press to enter" landmark gets a prompt, each with the SAME zone
            // enter_dungeon.rs checks (Baz: "the dungeon entrance didn't have a prompt").
            let zones = world
                .0
                .room_entities(cur.rx, cur.ry)
                .iter()
                .filter_map(|e| match e.kind {
                    "town" | "shop" | "dungeon" => Some(((e.x - 4) as f32, (e.y + 8) as f32, 24.0, 18.0)),
                    "rift" => Some(((e.x - 2) as f32, (e.y + 18) as f32, 20.0, 12.0)), // the js maw door
                    "castle" => Some(((e.x - 16) as f32, (e.y + 5) as f32, 32.0, 33.0)),
                    "guildhall" => Some(((e.x + 46) as f32, (e.y + 16) as f32, 20.0, 14.0)),
                    _ => None,
                })
                .collect();
            *doors = Some(((cur.rx, cur.ry), zones));
        }
        // The PLAYER'S OWN house door too (its zone lives outside the worldgen entity
        // layout, so the cache never sees it — Baz: "the house should have an ENTER prompt").
        doors.as_ref().is_some_and(|(_, zones)| zones.iter().any(|z| overlap(hitbox, *z)))
            || house.0.as_ref().is_some_and(|h| {
                h.room == (cur.rx, cur.ry) && overlap(hitbox, super::home::door_zone(h.x, h.y))
            })
    };

    // A lore tome at your feet (any mode) — its generous js zone reaches the floor
    // beside the furniture it sits on. Doors outrank it.
    let near_book = (!at_door)
        .then(|| {
            books.iter().find(|(_, pk)| {
                matches!(pk.kind, PickupKind::Book(_))
                    && overlap(hitbox, (pk.x - 8.0, pk.y - 8.0, 32.0, 32.0))
            })
        })
        .flatten();

    // PRESS to take the tome (the ledger + toast; the glow reaps with the pickup).
    // The press is CONSUMED — the counter/villager underneath stays quiet (js skipAction).
    if let Some((e, pk)) = near_book
        && input.pressed(Action::Interact)
        && let PickupKind::Book(id) = pk.kind
    {
        input.consume(Action::Interact);
        tk.gathered.tomes.insert(id);
        let title = crate::lore_books::get(id).map_or(id, |b| b.title);
        tk.log.add("tome", &format!("NEW TOME: {title}"), 1, 0xd8b8ff, false, true);
        // A songbook writes its tune into your fingers — if you have a flute to play.
        if let Some(song) = crate::lore_books::get(id).and_then(|b| b.teaches) {
            super::flute::learn_song(&mut tk.learned, &tk.inv, &mut tk.log, song, false);
        }
        commands.entity(e).despawn();
    }

    // A crop to PICK where you stand: a wild forage plant, or a ripe planted crop
    // (js 5218 — doors and tomes outrank it).
    let on_crop = if at_door || near_book.is_some() || inside.0.is_some() {
        false
    } else {
        let (pc, pr) = (((p.x + 8.0) / 16.0).floor() as i32, ((p.y + 12.0) / 16.0).floor() as i32);
        tk.farm.ready_at((cur.rx, cur.ry), pc, pr).is_some()
            || wild.iter().any(|w| {
                overlap(hitbox, ((w.c * 16 + 3) as f32, (w.r * 16 + 3) as f32, 10.0, 11.0))
            })
    };

    // A named villager OR a wilds wanderer in arm's reach — the js TALK label
    // (game.js 5228-5231: every fixture prompt outranks it; both talk systems
    // use the same 26px circle).
    let near_npc = !at_door
        && near_book.is_none()
        && !on_crop
        && tk.other_bubbles.is_empty() // a counter/station/wagon prompt owns the head
        && (tk.villagers.iter().any(|v| {
            v.pkey.is_some() && ((v.x + 8.0) - (p.x + 8.0)).hypot((v.y + 8.0) - (p.y + 8.0)) < 26.0
        }) || tk.wanderers.iter().any(|w| {
            ((w.x + 8.0) - (p.x + 8.0)).hypot((w.y + 8.0) - (p.y + 8.0)) < 26.0
        }));

    // The bubble: label + anchor (book prompts hover the BOOK, door prompts the player).
    let key = bindings.prompt(Action::Interact, input.pad_present);
    let want: Option<(String, i32, i32)> = if at_door {
        Some((format!("{key} ENTER"), p.x as i32 + 8, p.y as i32 - 10))
    } else if let Some((_, pk)) = near_book {
        Some((format!("{key} TAKE"), pk.x as i32 + 8, pk.y as i32 - 8))
    } else if on_crop {
        Some((format!("{key} PICK"), p.x as i32 + 8, p.y as i32 - 10))
    } else if near_npc {
        Some((format!("{key} TALK"), p.x as i32 + 8, p.y as i32 - 10))
    } else {
        None
    };
    if want == last.0 {
        return;
    }
    last.0 = want.clone();
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some((text, cx, by)) = want else { return };
    spawn_bubble(&mut commands, &mut images, &text, cx as f32, by as f32, PromptUi);
}

/// THE interact bubble — dark backing + gold label, centred on (cx, by) in room px.
/// Every interact prompt in the game speaks this one language, anchored by the character
/// (Baz: "some prompts were at the bottom, some by the character — I like them by the
/// character").
pub fn spawn_bubble(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    text: &str,
    cx: f32,
    by: f32,
    marker: impl Bundle + Clone,
) {
    let w = font::measure(text) as f32;
    let bx = (cx - w / 2.0).round();
    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.95), Vec2::new(w + 4.0, 9.0)),
        at(PLAY_X + bx - 2.0, PLAY_Y + by - 2.0, w + 4.0, 9.0, layers::PROMPT),
        PIXEL_LAYER,
        AnyBubble,
        marker.clone(),
    ));
    let le = label(commands, images, text, PLAY_X + bx, PLAY_Y + by, 0xfce0a8, layers::PROMPT_TEXT, marker);
    commands.entity(le).insert(AnyBubble);
}
