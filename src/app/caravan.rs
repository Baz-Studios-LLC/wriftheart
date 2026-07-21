//! caravan.rs — THE TRAVELLING TRADESMAN'S WAGON (js tradewagon + caravanStock): a rare
//! roadside merchant that deals in MATERIALS, sold in small stacks — the finer stock only
//! rides with caravans out in the deeper lands. Worldgen seeds a `tradewagon` entity on a
//! wide clear roadside patch (~1.5% of non-start rooms); this stands it up (the wagon + a
//! shopkeeper), blocks the cart, and — walk up and INTERACT — opens its shelf straight into
//! the shop screen (shop::open_caravan populates ShopState; no interior, no keeper).
//! The wake idiom (per-room Local + RoomActor sweep) mirrors mound_wake / guard_wake.

use bevy::prelude::*;

use super::battle::RoomActor;
use super::play::{CurRoom, GameWorld, Player};
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::actors::encounter_art::WAGON;
use crate::gfx::{at, bake, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};

/// A standing caravan (the merchant's cart) — its room-pixel anchor.
#[derive(Component)]
pub struct CaravanWagon {
    x: f32,
    y: f32,
}

/// The proximity prompt above the wagon (rebuilt only when it appears/vanishes).
#[derive(Component, Clone)]
struct CaravanPrompt;

/// The wagon's per-site seed (stable per room), for the stock roll.
fn wagon_seed(rx: i32, ry: i32) -> u32 {
    (rx.wrapping_mul(73_856_093) ^ ry.wrapping_mul(19_349_663)) as u32 ^ 0x2b1d_e300
}

/// Stand the wagon + shopkeeper up when its room arrives (the wake idiom); block the cart.
#[allow(clippy::too_many_arguments)]
fn wagon_wake(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut art: ResMut<crate::actors::villager::VillagerArt>,
    cur: Res<CurRoom>,
    sliding: Res<super::play::SlideActive>,
    world: Res<GameWorld>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut woke: Local<Option<(i32, i32)>>,
    live: Query<Entity, With<CaravanWagon>>,
) {
    if sliding.0 || in_dungeon.0.is_some() || inside.0.is_some() {
        *woke = None;
        return;
    }
    if *woke == Some((cur.rx, cur.ry)) {
        return;
    }
    *woke = Some((cur.rx, cur.ry));
    for e in &live {
        commands.entity(e).despawn();
    }
    for e in world.0.room_entities(cur.rx, cur.ry) {
        if e.kind != "tradewagon" {
            continue;
        }
        // The wagon anchor is the entity tile; the 32x21 covered wagon centres on it
        // (the old 16x12 handcart sat flush — Baz: "really is too small lol").
        let (x, y) = (e.x as f32 - 8.0, e.y as f32 - 6.0);
        let img = images.add(bake(WAGON, &[]));
        let mut spr = Sprite::from_image(img);
        spr.custom_size = Some(Vec2::new(32.0, 21.0));
        commands.spawn((
            spr,
            at(PLAY_X + x, PLAY_Y + y, 32.0, 21.0, actor_z(y + 20.0)),
            PIXEL_LAYER,
            RoomActor,
            CaravanWagon { x, y },
        ));
        // The shopkeeper stands a step to the right, facing down (a seeded villager look).
        let keeper_seed = wagon_seed(cur.rx, cur.ry) ^ 0x5178;
        let frames = art.frames(keeper_seed, &mut images);
        let kimg = frames.frames[0][0].clone();
        let mut ks = Sprite::from_image(kimg);
        ks.custom_size = Some(Vec2::splat(16.0));
        commands.spawn((
            ks,
            at(PLAY_X + x + 34.0, PLAY_Y + y + 6.0, 16.0, 16.0, actor_z(y + 22.0)),
            PIXEL_LAYER,
            RoomActor,
        ));
        // Solid cart: a blocker under the body + wheels (rebuilt on the next room swap).
        blockers.0.push((x + 2.0, y + 9.0, 28.0, 11.0));
    }
}

/// Show the trade prompt when the hero is near the wagon; INTERACT opens the shelf.
#[allow(clippy::too_many_arguments)]
fn caravan_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    bindings: Res<Bindings>,
    cur: Res<CurRoom>,
    mut shop: ResMut<super::shop::ShopState>,
    bought: Res<super::shop::BoughtShop>,
    clock: Res<super::room_render::FrameClock>,
    mut next: ResMut<NextState<super::screen::Screen>>,
    players: Query<&Player>,
    wagons: Query<&CaravanWagon>,
    old: Query<Entity, With<CaravanPrompt>>,
    mut shown: Local<Option<(i32, i32)>>,
) {
    let Ok(p) = players.single() else { return };
    let near = wagons.iter().any(|w| (w.x + 16.0 - (p.x + 8.0)).hypot(w.y + 10.0 - (p.y + 8.0)) < 34.0);
    // The shared by-the-character bubble (prompts.rs), re-anchored as the hero moves.
    let key = near.then_some((p.x as i32, p.y as i32));
    if key != *shown {
        *shown = key;
        for e in &old {
            commands.entity(e).despawn();
        }
        if near {
            let text = format!("{} TRADE", bindings.prompt(Action::Interact, false));
            super::prompts::spawn_bubble(&mut commands, &mut images, &text, p.x + 8.0, p.y - 10.0, CaravanPrompt);
        }
    }
    if near && input.pressed(Action::Interact) {
        input.consume(Action::Interact);
        let seed = wagon_seed(cur.rx, cur.ry);
        super::shop::open_caravan(&mut shop, &bought, cur.rx, cur.ry, seed, super::gather::farm_day(clock.0));
        next.set(super::screen::Screen::Shop);
    }
}

pub struct CaravanPlugin;

impl Plugin for CaravanPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy::app::FixedUpdate,
            (
                wagon_wake,
                caravan_tick
                    .after(super::prompts::prompt_tick)
                    .after(super::services::interact_tick)
                    .before(super::play::EndTick),
            )
                .run_if(super::screen::playing),
        );
    }
}
