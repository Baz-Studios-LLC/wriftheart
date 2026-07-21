//! packup.rs — HOLD-Trash pack-up for your placed BUILDINGS (js packableAt /
//! doPackUp / drawPackPrompt): stand beside your house, coop, barn, or placed
//! well and hold Trash for ~1s (a filling bar) to take it back down.
//!
//! DEVIATION (Baz): only HALF the cost returns — the js refunded in full ("a
//! MOVE, not a tax"). Mats floor at q/2 (the craft window's REMOVE convention);
//! the coin-bought house refunds half its price in copper. Bag overflow drops
//! at your feet (js). The house's furniture records stay, so a re-placed house
//! brings the furniture back (js home tables); packing the house also clears a
//! respawn point set there, so death can't warp you to a home that's gone.
//! Craft tables keep their own craft-window REMOVE (cooking.rs station_remove).

use bevy::prelude::*;

use super::battle::RoomActor;
use super::cooking::{PlacedStations, StationSprite};
use super::farm_animals::{FarmYard, Livestock, YardWake};
use super::home::{HouseSprite, PlayerHouse, RespawnPoint};
use super::play::{CurRoom, Player};
use super::room_render::{PLAY_X, PLAY_Y};
use crate::gfx::{at, font, layers, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::ui::label;

const PACK_FRAMES: u32 = 60; // js: ~1s deliberate hold
const BAR_W: u32 = 26; // js drawPackPrompt bar width

/// What's beside the player (js packableAt): the building kind, its record
/// anchor, and where the prompt/bar hangs.
struct Packable {
    kind: &'static str,
    x: f32,
    y: f32,
    cx: f32,
    top: f32,
}

#[derive(Component)]
struct PackUi;

/// The nearest of YOUR buildings within reach (js packableAt's radii, re-anchored
/// to the rs record origins). Priority order matches the js: house first.
fn packable_at(
    cur: (i32, i32),
    p: (f32, f32),
    house: &PlayerHouse,
    stock: &Livestock,
    stations: &PlacedStations,
) -> Option<Packable> {
    let near = |cx: f32, cy: f32, r: f32| (p.0 - cx).hypot(p.1 - cy) < r;
    if let Some(h) = &house.0
        && h.room == cur
        && near(h.x + 8.0, h.y - 7.0, 42.0)
    {
        return Some(Packable { kind: "house", x: h.x, y: h.y, cx: h.x + 8.0, top: h.y - 32.0 });
    }
    for &(rx, ry, x, y) in &stock.coops {
        if (rx, ry) == cur && near(x + 16.0, y + 20.0, 40.0) {
            return Some(Packable { kind: "coop", x, y, cx: x + 16.0, top: y - 4.0 });
        }
    }
    for &(rx, ry, x, y) in &stock.barns {
        if (rx, ry) == cur && near(x + 24.0, y + 25.0, 48.0) {
            return Some(Packable { kind: "barn", x, y, cx: x + 24.0, top: y - 10.0 });
        }
    }
    for s in &stations.0 {
        if s.kind == "well" && !s.home && s.room == cur && near(s.x + 16.0, s.y + 12.0, 34.0) {
            return Some(Packable { kind: "well", x: s.x, y: s.y, cx: s.x + 16.0, top: s.y - 12.0 });
        }
    }
    None
}

/// The building-side resources (bundled for Bevy's 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub struct PackCtx<'w> {
    house: ResMut<'w, PlayerHouse>,
    respawn: ResMut<'w, RespawnPoint>,
    stock: ResMut<'w, Livestock>,
    stations: ResMut<'w, PlacedStations>,
    blockers: ResMut<'w, super::room_props::RoomBlockers>,
    inv: ResMut<'w, crate::inventory::PlayerInv>,
    log: ResMut<'w, super::rewards::LootLog>,
    saves: MessageWriter<'w, super::save::SaveRequest>,
    sfx: MessageWriter<'w, super::sfx::Sfx>,
    woke: ResMut<'w, YardWake>,
    inside: Res<'w, super::interior::Inside>,
    in_dungeon: Res<'w, super::dungeon::InDungeon>,
    placing: Res<'w, super::placing::Placing>,
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn packup_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    input: Res<ActionState>,
    bindings: Res<Bindings>,
    cur: Res<CurRoom>,
    players: Query<&Player>,
    houses: Query<Entity, With<HouseSprite>>,
    yards: Query<Entity, With<FarmYard>>,
    fires: Query<(Entity, &StationSprite)>,
    ui: Query<Entity, With<PackUi>>,
    mut hold: Local<Option<(String, u32)>>,
    mut last_ui: Local<Option<(String, u32)>>,
    mut ctx: PackCtx,
) {
    let clear_ui = |commands: &mut Commands| {
        for e in &ui {
            commands.entity(e).despawn();
        }
    };
    let gated = ctx.inside.0.is_some() || ctx.in_dungeon.0.is_some() || ctx.placing.0.is_some();
    let Ok(p) = players.single() else { return };
    let target = if gated {
        None
    } else {
        packable_at((cur.rx, cur.ry), (p.x + 8.0, p.y + 9.0), &ctx.house, &ctx.stock, &ctx.stations)
    };
    let Some(t) = target else {
        *hold = None;
        if last_ui.take().is_some() {
            clear_ui(&mut commands);
        }
        return;
    };

    let id = format!("{}:{}:{}", t.kind, t.x, t.y);
    if input.held(Action::Trash) {
        let n = match hold.take() {
            Some((k, n)) if k == id => n + 1,
            _ => 1,
        };
        if n >= PACK_FRAMES {
            do_pack(&mut commands, &mut images, &t, (cur.rx, cur.ry), &houses, &yards, &fires, &mut ctx, (p.x, p.y));
            *last_ui = None;
            clear_ui(&mut commands);
            return;
        }
        *hold = Some((id.clone(), n));
    } else {
        *hold = None;
    }

    // The hint when idle, the filling bar while holding (js drawPackPrompt) —
    // redrawn only when the whole-px fill (or the target) changes.
    let fill = hold.as_ref().map_or(0, |(_, n)| (n * BAR_W / PACK_FRAMES).min(BAR_W));
    let key = (id, fill + if hold.is_some() { 100 } else { 0 });
    if last_ui.as_ref() == Some(&key) {
        return;
    }
    *last_ui = Some(key);
    clear_ui(&mut commands);
    if hold.is_some() {
        let (w, x, y) = (BAR_W as f32, (t.cx - BAR_W as f32 / 2.0).round(), t.top - 6.0);
        commands.spawn((
            Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.6), Vec2::new(w + 2.0, 5.0)),
            at(PLAY_X + x - 1.0, PLAY_Y + y - 1.0, w + 2.0, 5.0, layers::PROMPT),
            PIXEL_LAYER,
            RoomActor,
            PackUi,
        ));
        if fill > 0 {
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0xe0, 0xb0, 0x40), Vec2::new(fill as f32, 3.0)),
                at(PLAY_X + x, PLAY_Y + y, fill as f32, 3.0, layers::PROMPT + 0.02),
                PIXEL_LAYER,
                RoomActor,
                PackUi,
            ));
        }
    } else {
        let hint = format!("HOLD {} PACK UP", bindings.prompt(Action::Trash, input.pad_present));
        let w = font::measure(&hint) as f32;
        let x = (t.cx - w / 2.0).round().clamp(2.0, crate::room::PX_W as f32 - w - 4.0);
        let y = t.top - 8.0;
        commands.spawn((
            Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.6), Vec2::new(w + 4.0, 7.0)),
            at(PLAY_X + x - 2.0, PLAY_Y + y - 1.0, w + 4.0, 7.0, layers::PROMPT),
            PIXEL_LAYER,
            RoomActor,
            PackUi,
        ));
        label(&mut commands, &mut images, &hint, PLAY_X + x, PLAY_Y + y, 0xcfe0ff, layers::PROMPT_TEXT, PackUi);
    }
}

/// Tear the building down + the half refund (js doPackUp, taxed).
#[allow(clippy::too_many_arguments)]
fn do_pack(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    t: &Packable,
    cur: (i32, i32),
    houses: &Query<Entity, With<HouseSprite>>,
    yards: &Query<Entity, With<FarmYard>>,
    fires: &Query<(Entity, &StationSprite)>,
    ctx: &mut PackCtx,
    ppos: (f32, f32),
) {
    match t.kind {
        "house" => {
            for e in houses {
                commands.entity(e).despawn();
            }
            ctx.blockers.0.retain(|b| *b != (t.x - 12.0, t.y - 28.0, 40.0, 42.0));
            ctx.house.0 = None;
            // A spawn point set at this home dies with it — death must not warp
            // to a house that no longer stands (the phantom-home lesson).
            if ctx.respawn.0.as_ref().is_some_and(|r| r.room == cur) {
                ctx.respawn.0 = None;
            }
            let back = crate::items::get("house").and_then(|d| d.price).unwrap_or(600) / 2;
            ctx.inv.money += back as i64;
            ctx.log.add("packup", &format!("HOME PACKED UP - {back} COPPER BACK"), 1, 0xcfe0ff, false, true);
        }
        "coop" | "barn" => {
            if t.kind == "coop" {
                ctx.stock.coops.retain(|c| !((c.0, c.1) == cur && c.2 == t.x && c.3 == t.y));
                ctx.blockers.0.retain(|b| *b != (t.x + 2.0, t.y + 10.0, 28.0, 20.0));
            } else {
                ctx.stock.barns.retain(|b| !((b.0, b.1) == cur && b.2 == t.x && b.3 == t.y));
                ctx.blockers.0.retain(|b| *b != (t.x + 3.0, t.y + 14.0, 42.0, 22.0));
            }
            // Sweep the yard + poke the wake tracker: survivors (and the animals,
            // who keep the yard regardless) re-stand from records next tick.
            for e in yards {
                commands.entity(e).despawn();
            }
            ctx.woke.0 = None;
            refund_half(t.kind, commands, images, ctx, ppos);
            ctx.log.add(
                "packup",
                if t.kind == "coop" { "COOP PACKED UP - HALF MATS BACK" } else { "BARN PACKED UP - HALF MATS BACK" },
                1,
                0xcfe0ff,
                false,
                true,
            );
        }
        _ => {
            // The well rides the station records (kind-gated in packable_at).
            ctx.stations.0.retain(|s| !(s.room == cur && s.x == t.x && s.y == t.y && !s.home));
            for (e, f) in fires {
                if f.x == t.x && f.y == t.y {
                    commands.entity(e).despawn();
                }
            }
            ctx.blockers.0.retain(|b| *b != (t.x + 2.0, t.y + 6.0, 28.0, 9.0));
            refund_half("well", commands, images, ctx, ppos);
            ctx.log.add("packup", "WELL PACKED UP - HALF MATS BACK", 1, 0xcfe0ff, false, true);
        }
    }
    ctx.sfx.write(super::sfx::Sfx("craft"));
    ctx.saves.write(super::save::SaveRequest);
}

/// Half of each recipe mat (floor — the craft-window REMOVE convention); a full
/// bag drops the remainder at your feet (js overflow).
fn refund_half(kind: &str, commands: &mut Commands, images: &mut Assets<Image>, ctx: &mut PackCtx, ppos: (f32, f32)) {
    if let Some(r) = crate::recipes_data::RECIPES.iter().find(|r| r.out == kind) {
        for (id, q) in r.cost {
            let half = q / 2;
            if half > 0 && !ctx.inv.add_item(id, half) {
                super::gather::spawn_pickup(commands, images, id, half, ppos.0 + 8.0, ppos.1 + 9.0, false, None);
            }
        }
    }
}

pub struct PackupPlugin;

impl Plugin for PackupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy::app::FixedUpdate,
            packup_tick.before(super::play::EndTick).run_if(super::screen::playing),
        );
    }
}
