//! placing.rs — the GHOST-PLACEMENT mode (js startPlacement / updatePlacement /
//! ghostRect / confirmPlacement + drawPlacement): using a station kit or the House
//! opens a movable tile reticle instead of dropping the thing at your feet. Arrows
//! nudge the ghost a tile at a time, Slot3 spins a table's facing, Slot1/INTERACT
//! sets it down (paying the item), Slot2 backs out free. The ghost draws the REAL
//! sprite at half alpha under a green/red validity tint; the player stands rooted
//! (play.rs gates movement on `Placing`, the fluting idiom).

use bevy::prelude::*;

use super::battle::RoomActor;
use super::play::{CurRoom, Player};
use super::room_render::{PLAY_X, PLAY_Y};
use crate::actors::hero::Facing;
use crate::gfx::{at, font, layers, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::ui::label;

/// Tiles across / down the play area (the room grid).
const COLS: i32 = crate::room::PX_W / 16;
const ROWS: i32 = crate::room::PX_H / 16;

pub struct PlacingState {
    /// A STATION item id ("workbench", "cook", ...) or "house".
    pub kind: &'static str,
    /// The ghost's base tile (a station's LEFT tile; the house's DOOR tile).
    pub gx: i32,
    pub gy: i32,
    pub rot: u8,
    pub valid: bool,
    /// Placing INSIDE the player's house (js home tables) — interior-canvas coords,
    /// and the doorway mat must stay clear.
    pub home: bool,
}

/// The placement in flight (None = not placing). play.rs roots the hero while Some.
#[derive(Resource, Default)]
pub struct Placing(pub Option<PlacingState>);

#[derive(Component)]
struct GhostUi;

/// Does this kind rotate? (The cook fire + well are radially symmetric.)
fn rotates(kind: &str) -> bool {
    kind != "cook" && kind != "well" && kind != "house"
}

/// The footprint tinted for validity — the thing's blocker rect (js ghostRect).
fn foot_rect(kind: &str, x: f32, y: f32) -> (f32, f32, f32, f32) {
    if kind == "house" {
        (x - 12.0, y - 28.0, 40.0, 42.0)
    } else {
        (x, y, 32.0, 16.0)
    }
}

/// Start a placement — the kit/house use routes here (play.rs messages). The ghost
/// spawns two tiles in front of the hero's facing (js startPlacement).
#[allow(clippy::too_many_arguments)]
fn start_placement(
    mut kits: MessageReader<super::cooking::PlaceStation>,
    mut houses: MessageReader<super::home::PlaceHouse>,
    mut placing: ResMut<Placing>,
    cur: Res<CurRoom>,
    world: Res<super::play::GameWorld>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&Player>,
) {
    let mut want: Option<&'static str> = None;
    for super::cooking::PlaceStation(kind) in kits.read() {
        want = Some(kind);
    }
    for super::home::PlaceHouse in houses.read() {
        want = Some("house");
    }
    let Some(kind) = want else { return };
    let Ok(p) = players.single() else { return };
    // YOUR house welcomes stations (js home tables); every other roof — and a house
    // inside a house — says no.
    let in_house = inside.0.as_ref().is_some_and(|st| st.def.kind == "house") && kind != "house";
    if in_dungeon.0.is_some()
        || (inside.0.is_some() && !in_house)
        || (!in_house && crate::worldgen::towns::town_role(world.0.seed, cur.rx, cur.ry).is_some())
    {
        let msg = if kind == "house" { "NO PLACE FOR A HOME HERE" } else { "NO PLACE FOR A CAMP HERE" };
        log.add("place", msg, 1, 0xfc8868, false, true);
        sfx.write(super::sfx::Sfx("tink"));
        return;
    }
    let (dx, dy): (i32, i32) = match p.facing {
        Facing::Up => (0, -1),
        Facing::Down => (0, 1),
        Facing::Left => (-1, 0),
        Facing::Right => (1, 0),
    };
    let gx = ((p.x + 8.0) / 16.0).floor() as i32 + dx * 2 - 1;
    let gy = ((p.y + 8.0) / 16.0).floor() as i32 + dy * 2;
    placing.0 = Some(PlacingState { kind, gx, gy, rot: 0, valid: false, home: in_house });
    sfx.write(super::sfx::Sfx("open"));
}

/// The mode's fixed-tick driver: nudge / rotate / cancel / confirm (js updatePlacement).
#[allow(clippy::too_many_arguments)]
fn placement_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    mut placing: ResMut<Placing>,
    cur: Res<CurRoom>,
    grid: Res<super::play::CurGrid>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut ctx: PlaceCtx,
    active: Res<super::play::ActiveRoot>,
    inside: Res<super::interior::Inside>,
    players: Query<&Player>,
    live_house: Query<Entity, With<super::home::HouseSprite>>,
) {
    let Some(pl) = &mut placing.0 else { return };
    let Ok(p) = players.single() else { return };
    // The mode owns the face buttons (the heldLatch rule — a leftover hold can't swing).
    for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
        input.latch(a);
    }
    if input.pressed(Action::Up) {
        pl.gy -= 1;
    }
    if input.pressed(Action::Down) {
        pl.gy += 1;
    }
    if input.pressed(Action::Left) {
        pl.gx -= 1;
    }
    if input.pressed(Action::Right) {
        pl.gx += 1;
    }
    if rotates(pl.kind) && input.pressed(Action::Slot3) {
        pl.rot = (pl.rot + 1) % 4;
    }
    // Keep the whole footprint inside the walkable interior (also keeps room exits clear —
    // the js's separate doorway check).
    if pl.kind == "house" {
        pl.gx = pl.gx.clamp(2, COLS - 3);
        pl.gy = pl.gy.clamp(2, ROWS - 2);
    } else {
        pl.gx = pl.gx.clamp(1, COLS - 3);
        pl.gy = pl.gy.clamp(1, ROWS - 2);
    }
    let (x, y) = ((pl.gx * 16) as f32, (pl.gy * 16) as f32);
    // Validity (js ghostRect): clear ground + no entity blocker + not on the hero.
    let pbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    let over = |r: (f32, f32, f32, f32)| {
        pbox.0 < r.0 + r.2 && pbox.0 + pbox.2 > r.0 && pbox.1 < r.1 + r.3 && pbox.1 + pbox.3 > r.1
    };
    // Indoors, the doorway mat is sacred — a table there would wall you in with
    // your own furniture (the js separate-doorway check, home flavour).
    let on_exit = pl.home
        && inside.0.as_ref().is_some_and(|st| {
            let (ex, ey, ew, eh) = st.def.exit;
            x < (ex + ew) as f32 && x + 32.0 > ex as f32 && y < (ey + eh) as f32 && y + 16.0 > ey as f32
        });
    pl.valid = if pl.kind == "house" {
        !grid.0.box_hits_solid(x - 11.0, y - 27.0, 38.0, 34.0)
            && !ctx.blockers_overlap(&blockers, x - 12.0, y - 28.0, 40.0, 34.0)
            && !over((x - 12.0, y - 28.0, 40.0, 42.0))
    } else {
        (0..2).all(|i| !grid.0.box_hits_solid(x + 1.0 + i as f32 * 16.0, y + 1.0, 14.0, 14.0))
            && !ctx.blockers_overlap(&blockers, x, y, 32.0, 16.0)
            && !over((x, y, 32.0, 16.0))
            && !on_exit
    };
    if input.pressed(Action::Slot2) || input.pressed(Action::Pause) {
        input.consume(Action::Slot2);
        input.consume(Action::Pause);
        placing.0 = None;
        ctx.sfx.write(super::sfx::Sfx("open"));
        return;
    }
    if input.pressed(Action::Slot1) || input.pressed(Action::Interact) || input.pressed(Action::MenuConfirm) {
        input.consume(Action::Slot1);
        input.consume(Action::Interact);
        if !pl.valid {
            ctx.sfx.write(super::sfx::Sfx("tink"));
            return;
        }
        if pl.kind == "house" {
            ctx.inv.remove_one("house");
            super::home::confirm_house(
                &mut commands, &ctx.art, &mut blockers, &mut ctx.house, &live_house, active.0, (cur.rx, cur.ry), x, y,
            );
            ctx.log.add("home", "YOUR HOME STANDS", 1, 0xcfe0ff, false, true);
        } else {
            ctx.inv.remove_one(pl.kind);
            ctx.stations.0.push(super::cooking::StationRec {
                room: (cur.rx, cur.ry),
                x,
                y,
                kind: pl.kind.to_string(),
                rot: pl.rot,
                home: pl.home,
            });
            super::cooking::spawn_fire(&mut commands, &mut images, &mut blockers, x, y, pl.kind, pl.rot);
            let (line, col) = if pl.kind == "cook" {
                ("THE COOKING FIRE CRACKLES", 0xd0822a)
            } else {
                super::station_art::place_msg(pl.kind)
            };
            ctx.log.add("cook", line, 1, col, false, true);
        }
        ctx.sfx.write(super::sfx::Sfx("craft"));
        ctx.saves.write(super::save::SaveRequest);
        placing.0 = None;
    }
}

/// The confirm path's resource bundle (Bevy's 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub struct PlaceCtx<'w> {
    pub stations: ResMut<'w, super::cooking::PlacedStations>,
    pub house: ResMut<'w, super::home::PlayerHouse>,
    pub inv: ResMut<'w, crate::inventory::PlayerInv>,
    pub art: Res<'w, crate::actors::props::PropArt>,
    pub log: ResMut<'w, super::rewards::LootLog>,
    pub saves: MessageWriter<'w, super::save::SaveRequest>,
    pub sfx: MessageWriter<'w, super::sfx::Sfx>,
}

impl PlaceCtx<'_> {
    fn blockers_overlap(&self, blockers: &super::room_props::RoomBlockers, x: f32, y: f32, w: f32, h: f32) -> bool {
        blockers.0.iter().any(|b| x < b.0 + b.2 && x + w > b.0 && y < b.1 + b.3 && y + h > b.1)
    }
}

/// Rebuild the ghost (sprite + tint + hint) whenever it moves/spins/flips validity
/// (js drawPlacement: the real sprite at 0.55 alpha under the green/red wash).
#[allow(clippy::too_many_arguments)]
fn ghost_render(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    placing: Res<Placing>,
    art: Res<crate::actors::props::PropArt>,
    bindings: Res<Bindings>,
    state: Res<ActionState>,
    old: Query<Entity, With<GhostUi>>,
    mut last: Local<Option<(i32, i32, u8, bool)>>,
) {
    let Some(pl) = &placing.0 else {
        if last.is_some() {
            *last = None;
            for e in &old {
                commands.entity(e).despawn();
            }
        }
        return;
    };
    let key = (pl.gx, pl.gy, pl.rot, pl.valid);
    if *last == Some(key) {
        return;
    }
    *last = Some(key);
    for e in &old {
        commands.entity(e).despawn();
    }
    let (x, y) = ((pl.gx * 16) as f32, (pl.gy * 16) as f32);
    let z = layers::PROMPT - 0.2;
    // The real sprite, half-ghosted.
    let mut spr = if pl.kind == "house" {
        Some((Sprite::from_image(art.farmhouse.clone()), x - 16.0, y - 36.0, 48.0, 52.0))
    } else if pl.kind == "cook" || pl.kind == "well" {
        None // their grids live in cooking.rs — the tint alone reads fine for the symmetric pair
    } else {
        let img = super::station_art::station_image(pl.kind, pl.rot, &mut images);
        Some((
            Sprite::from_image(img),
            x,
            y - 8.0 - super::station_art::OY as f32,
            super::station_art::CANVAS.0 as f32,
            super::station_art::CANVAS.1 as f32,
        ))
    };
    if let Some((ref mut s, sx, sy, sw, sh)) = spr {
        s.color = Color::srgba(1.0, 1.0, 1.0, 0.55);
        commands.spawn((s.clone(), at(PLAY_X + sx, PLAY_Y + sy, sw, sh, z), PIXEL_LAYER, RoomActor, GhostUi));
    }
    // The validity wash over the footprint.
    let (fx, fy, fw, fh) = foot_rect(pl.kind, x, y);
    let tint = if pl.valid {
        Color::srgba(60.0 / 255.0, 220.0 / 255.0, 90.0 / 255.0, 0.30)
    } else {
        Color::srgba(230.0 / 255.0, 40.0 / 255.0, 40.0 / 255.0, 0.38)
    };
    commands.spawn((
        Sprite::from_color(tint, Vec2::new(fw, fh)),
        at(PLAY_X + fx, PLAY_Y + fy, fw, fh, z + 0.05),
        PIXEL_LAYER,
        RoomActor,
        GhostUi,
    ));
    // The hint line (js placement hint): PLACE / ROTATE / CANCEL, derived prompts.
    let pad = state.pad_present;
    let hint = if rotates(pl.kind) {
        format!(
            "{} PLACE - {} ROTATE - {} CANCEL",
            bindings.prompt(Action::Interact, pad),
            bindings.prompt(Action::Slot3, pad),
            bindings.prompt(Action::Slot2, pad)
        )
    } else {
        format!("{} PLACE - {} CANCEL", bindings.prompt(Action::Interact, pad), bindings.prompt(Action::Slot2, pad))
    };
    let w = font::measure(&hint) as f32;
    let bx = PLAY_X + (crate::room::PX_W as f32 - w) / 2.0;
    let by = PLAY_Y + crate::room::PX_H as f32 - 26.0;
    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.8), Vec2::new(w + 8.0, 11.0)),
        at(bx - 4.0, by - 2.0, w + 8.0, 11.0, layers::PROMPT),
        PIXEL_LAYER,
        GhostUi,
    ));
    label(&mut commands, &mut images, &hint, bx, by, 0xfce0a8, layers::PROMPT_TEXT, GhostUi);
}

pub struct PlacingPlugin;

impl Plugin for PlacingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Placing>()
            // start_placement runs UNGATED: a kit used from the BAG writes its message
            // while the slide-out screen is up — a playing-gate would let it expire
            // unread (the blueprint lesson). placement_tick stays play-only.
            .add_systems(
                bevy::app::FixedUpdate,
                (
                    start_placement,
                    placement_tick.after(start_placement).run_if(super::screen::playing),
                )
                    .before(super::play::EndTick),
            )
            .add_systems(Update, ghost_render);
    }
}
