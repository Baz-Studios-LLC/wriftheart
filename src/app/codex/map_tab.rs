//! map_tab.rs — the explored-world map: every visited room drawn as a tiny picture of
//! itself (1 px per tile, cached), with integer zoom and held-key panning. Port of the
//! MAP tab in js/codex.js (roomThumb / mapBounds / drawMap).
//!
//! Deltas vs JS (arrive with their systems): no prop pixels on thumbs (trees/boulders need
//! the entities port), no town/dungeon/castle markers, no quest pins. Sides are full-bleed
//! to the canvas edge instead of the JS 6px clip margin — a deliberate small improvement.

use super::super::play::{CurRoom, GameWorld, Visited};
use super::{hint_scaffold, CodexUi, CodexState, TabContent, CONTENT_Z, FOOT_H, TOP_H};
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::ui::{frame_rect, label};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use crate::room::{COLS, ROWS};
use crate::{CANVAS_H, CANVAS_W};
use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const GAP: f32 = 2.0; // px between room cells (MAP_G)
const AX: f32 = 6.0;
const AY: f32 = TOP_H + 1.0;
const PAN_PX: f32 = 2.0; // canvas px per held tick — constant on-screen pan speed

/// Zoom (integer px per tile) + camera centre as a fraction of the full map.
#[derive(Resource)]
pub struct MapView {
    pub ts: i32,
    pub cx: f32,
    pub cy: f32,
}

impl Default for MapView {
    fn default() -> Self {
        Self { ts: 2, cx: 0.5, cy: 0.5 }
    }
}

/// Room-thumbnail cache — worldgen is deterministic, so a thumb never goes stale.
#[derive(Resource, Default)]
pub struct ThumbCache(HashMap<(i32, i32), Handle<Image>>);

#[derive(Component)]
pub struct MapRoot;

/// The RECENTER chip (bottom-right of the view) — snaps the camera back onto the
/// room you stand in. NOT a child of the map root (it must not pan).
#[derive(Component, Clone)]
pub struct RecenterBtn;

/// One geometry source for the chip's draw + click test (the tab_chips rule).
fn recenter_rect() -> (f32, f32, f32, f32) {
    let (vw, vh) = view_size();
    let w = font::measure("RECENTER") as f32 + 8.0;
    (AX + vw - w - 3.0, AY + vh - 14.0, w, 11.0)
}

pub fn hint(bindings: &Bindings, pad: bool) -> String {
    let zoom = format!(
        "{}/{} ZOOM",
        bindings.prompt(Action::Slot1, pad),
        bindings.prompt(Action::Slot3, pad)
    );
    let pan = if pad { "STICK PAN" } else { "ARROWS PAN" };
    let home = format!("{} RECENTER", bindings.prompt(Action::Slot4, pad));
    hint_scaffold(bindings, pad, &format!("{zoom} - {pan} - {home}"))
}

/// The MAP tab driver: (re)build on entry/zoom, pan the root every tick.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
#[allow(clippy::type_complexity)] // the marks tuple + the dungeon rebuild key
pub fn run(
    mut commands: Commands,
    state: Res<ActionState>,
    cx_state: Res<CodexState>,
    world: Res<GameWorld>,
    visited: Res<Visited>,
    cur: Res<CurRoom>,
    mut view: ResMut<MapView>,
    mut cache: ResMut<ThumbCache>,
    mut images: ResMut<Assets<Image>>,
    quests: Res<crate::app::quests::QuestLog>,
    tmaps: Res<crate::app::digging::TreasureMaps>,
    inv: Res<crate::inventory::PlayerInv>,
    mut root: Query<(Entity, &mut Transform), With<MapRoot>>,
    mut seen_gen: Local<u32>,
    // Tuple-nested (the 16-param cap): the marker layer's inputs — town names,
    // the player's built home, the shard count (the castle gate glows when whole),
    // and the hero's exact tile for the red position dot.
    marks: (
        Res<crate::app::banners::TownNames>,
        Res<crate::app::home::PlayerHouse>,
        Res<crate::app::dungeon::Relics>,
        Query<&crate::app::play::Player>,
        Res<crate::app::dungeon::InDungeon>,
        Res<crate::app::saltmaze::ChantClock>,
        // The MOUSE gear (Baz: drag pans, wheel zooms, RECENTER snaps home):
        // pointer, buttons, wheel, the drag anchor, the wheel accumulator, the chip.
        (
            Res<crate::input::Pointer>,
            Res<ButtonInput<MouseButton>>,
            MessageReader<MouseWheel>,
            Local<Option<Vec2>>,
            Local<f32>,
            Query<Entity, With<RecenterBtn>>,
        ),
        // Where you FELL + the clock that expires it with the day's room reset.
        (Res<crate::app::death::LastDeath>, Res<crate::app::room_render::FrameClock>),
    ),
    mut dmap_key: Local<Option<(i32, i32, i32, u32, i32)>>,
) {
    let (towns, phouse, relics_res, players_q, in_dungeon, chant, mouse, fell) = marks;
    let death_room = fell.0 .0.filter(|(_, day)| *day == crate::app::gather::farm_day(fell.1 .0)).map(|(r, _)| r);
    let (ptr, mbtn, mut wheels, mut drag, mut wacc, btns) = mouse;
    // UNDERGROUND: the DUNGEON FLOOR MAP replaces the world map (js drawDungeonMap) —
    // auto-fit, no zoom/pan, rebuilt when the room/floor moves (or the chant meter ticks).
    if let Some(drun) = &in_dungeon.0 {
        for e in &btns {
            commands.entity(e).despawn(); // no recentering a fixed floor map
        }
        let key = (drun.drx, drun.dry, drun.dungeon.floor as i32, cx_state.generation, chant.0 / 30);
        if *dmap_key != Some(key) {
            *dmap_key = Some(key);
            for (e, _) in &root {
                commands.entity(e).despawn();
            }
            spawn_dungeon_map(&mut commands, &mut images, drun, chant.0);
        }
        return;
    }
    if dmap_key.take().is_some() {
        // Back on the surface with the codex still open — trip a world-map rebuild.
        *seen_gen = cx_state.generation.wrapping_sub(1);
    }
    let pins = pin_rooms(&quests, &tmaps);
    let mut rebuild = false;
    if *seen_gen != cx_state.generation {
        *seen_gen = cx_state.generation;
        // Fresh open of this tab: reset to overview zoom, camera on the current room (js
        // mapOpenInit).
        *view = MapView::default();
        let b = bounds(&visited, cur.rx, cur.ry, &pins);
        let (cell_w, cell_h) = cell_size(view.ts);
        let full_w = b.cols as f32 * cell_w - GAP;
        let full_h = b.rows as f32 * cell_h - GAP;
        if full_w > 0.0 {
            view.cx = ((cur.rx - b.min_x) as f32 * cell_w + COLS as f32 * view.ts as f32 / 2.0) / full_w;
        }
        if full_h > 0.0 {
            view.cy = ((cur.ry - b.min_y) as f32 * cell_h + ROWS as f32 * view.ts as f32 / 2.0) / full_h;
        }
        rebuild = true;
    }
    if state.pressed(Action::Slot1) && view.ts < 16 {
        view.ts += 1;
        rebuild = true;
    }
    if state.pressed(Action::Slot3) && view.ts > 1 {
        view.ts -= 1;
        rebuild = true;
    }
    // Pan: held directions (the stick already feeds the dpad actions). The step is a
    // CONSTANT on-screen speed — cx/cy are fractions of the FULL map, so a flat fraction
    // per tick panned faster the more you'd explored (Baz: "too fast and hard to control").
    {
        let b = bounds(&visited, cur.rx, cur.ry, &pins);
        let (cell_w, cell_h) = cell_size(view.ts);
        let full_w = (b.cols as f32 * cell_w - GAP).max(1.0);
        let full_h = (b.rows as f32 * cell_h - GAP).max(1.0);
        if state.held(Action::Left) {
            view.cx -= PAN_PX / full_w;
        }
        if state.held(Action::Right) {
            view.cx += PAN_PX / full_w;
        }
        if state.held(Action::Up) {
            view.cy -= PAN_PX / full_h;
        }
        if state.held(Action::Down) {
            view.cy += PAN_PX / full_h;
        }
    }
    // MOUSE (Baz): hold-and-drag pans the map under the cursor, the wheel zooms,
    // and the RECENTER chip snaps the camera back onto the room you stand in.
    {
        let b = bounds(&visited, cur.rx, cur.ry, &pins);
        let (cell_w, cell_h) = cell_size(view.ts);
        let full_w = (b.cols as f32 * cell_w - GAP).max(1.0);
        let full_h = (b.rows as f32 * cell_h - GAP).max(1.0);
        let (vw, vh) = view_size();
        let (bx, by, bw, bh) = recenter_rect();
        for m in wheels.read() {
            *wacc += match m.unit {
                MouseScrollUnit::Line => m.y,
                MouseScrollUnit::Pixel => m.y / 24.0,
            };
        }
        while *wacc >= 1.0 {
            *wacc -= 1.0;
            if view.ts < 16 {
                view.ts += 1;
                rebuild = true;
            }
        }
        while *wacc <= -1.0 {
            *wacc += 1.0;
            if view.ts > 1 {
                view.ts -= 1;
                rebuild = true;
            }
        }
        if mbtn.pressed(MouseButton::Left) {
            if let Some(p) = ptr.pos {
                if let Some(last) = *drag {
                    // The map rides WITH the cursor: camera moves opposite the drag.
                    view.cx -= (p.x - last.x) / full_w;
                    view.cy -= (p.y - last.y) / full_h;
                    *drag = Some(p);
                } else if mbtn.just_pressed(MouseButton::Left)
                    && p.x >= AX
                    && p.x <= AX + vw
                    && p.y >= AY
                    && p.y <= AY + vh
                    && !ptr.over(bx, by, bw, bh)
                {
                    *drag = Some(p);
                }
            }
        } else {
            *drag = None;
        }
        // Click the chip OR press Slot4 (Baz: unreachable by pad otherwise).
        if (ptr.click && ptr.over(bx, by, bw, bh)) || state.pressed(Action::Slot4) {
            view.cx = ((cur.rx - b.min_x) as f32 * cell_w + COLS as f32 * view.ts as f32 / 2.0) / full_w;
            view.cy = ((cur.ry - b.min_y) as f32 * cell_h + ROWS as f32 * view.ts as f32 / 2.0) / full_h;
        }
        // Stand the chip up once (swept with the tab; rebuilt here if missing).
        if btns.is_empty() {
            let tag = (CodexUi, TabContent, RecenterBtn);
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0x14, 0x14, 0x1c), Vec2::new(bw, bh)),
                at(bx, by, bw, bh, CONTENT_Z + 0.55),
                PIXEL_LAYER,
                tag.clone(),
            ));
            frame_rect(&mut commands, bx, by, bw, bh, 0x4a4a58, CONTENT_Z + 0.56, tag.clone());
            label(&mut commands, &mut images, "RECENTER", bx + 4.0, by + 3.0, 0xcfd8e4, CONTENT_Z + 0.57, tag);
        }
    }
    view.cx = view.cx.clamp(0.0, 1.0);
    view.cy = view.cy.clamp(0.0, 1.0);

    if rebuild {
        for (e, _) in &root {
            commands.entity(e).despawn();
        }
        let relics_whole = relics_res.0.len() >= world.0.shard_biomes().len();
        let ppos = players_q.single().map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));
        spawn_map(
            &mut commands, &world, &visited, cur.as_ref(), &view, &mut cache, &mut images, &quests, &tmaps, &inv,
            &towns, &phouse, relics_whole, ppos, death_room,
        );
    } else if let Ok((_, mut tf)) = root.single_mut() {
        *tf = root_transform(&visited, cur.as_ref(), &view, &pins);
    }
}

/// Gold octant arrows (E, SE, S, SW, W, NW, N, NE) for the viewport-rim quest
/// compass — an in-progress marker off the visible map gets an arrow on the rim
/// pointing its way (Baz: "arrows on the edge of the map that point towards
/// quests"). 7x7 bakes, quest-gold.
const ARROW_PAL: &[(char, u32)] = &[('g', 0xffd34d)];
const OCTANT_ARROWS: [&[&str]; 8] = [
    &["g......", "gg.....", "ggg....", "gggg...", "ggg....", "gg.....", "g......"], // E
    &[".......", ".......", "....ggg", ".....gg", "....g.g", "...g..g", "..g...g"], // SE
    &[".......", ".......", ".......", "ggggggg", ".ggggg.", "..ggg..", "...g..."], // S
    &[".......", ".......", "ggg....", "gg.....", "g.g....", "g..g...", "g...g.."], // SW
    &["......g", ".....gg", "....ggg", "...gggg", "....ggg", ".....gg", "......g"], // W
    &["g...g..", "g..g...", "g.g....", "gg.....", "ggg....", ".......", "......."], // NW
    &["...g...", "..ggg..", ".ggggg.", "ggggggg", ".......", ".......", "......."], // N
    &["..g...g", "...g..g", "....g.g", ".....gg", "....ggg", ".......", "......."], // NE
];

/// The rim-arrow layer, rebuilt whenever the picture changes (pan, zoom, log).
#[derive(Component)]
pub struct EdgeArrow;

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn edge_arrows(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cx_state: Res<super::CodexState>,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    visited: Res<Visited>,
    cur: Res<CurRoom>,
    view: Res<MapView>,
    quests: Res<crate::app::quests::QuestLog>,
    tmaps: Res<crate::app::digging::TreasureMaps>,
    inv: Res<crate::inventory::PlayerInv>,
    old: Query<Entity, With<EdgeArrow>>,
    mut art: Local<Vec<Handle<Image>>>,
    mut last: Local<Option<String>>,
) {
    let mut wants: Vec<(f32, f32, usize)> = Vec::new();
    if in_dungeon.0.is_none() {
        let pins = pin_rooms(&quests, &tmaps);
        let b = bounds(&visited, cur.rx, cur.ry, &pins);
        let (cell_w, cell_h) = cell_size(view.ts);
        let full_w = (b.cols as f32 * cell_w - GAP).max(1.0);
        let full_h = (b.rows as f32 * cell_h - GAP).max(1.0);
        let (vw, vh) = view_size();
        let off = |full: f32, v: f32, c: f32| {
            if full <= v { -((v - full) / 2.0) } else { (c * full - v / 2.0).clamp(0.0, full - v) }
        };
        let (ox, oy) = (AX - off(full_w, vw, view.cx), AY - off(full_h, vh, view.cy));
        let (rw, rh) = ((COLS * view.ts) as f32, (ROWS * view.ts) as f32);
        for q in &quests.0 {
            if q.ready(&inv) {
                continue; // the '?' giver pin is a different errand — arrows track '!'s
            }
            let Some(m) = q.marker() else { continue };
            let sx = ox + (m.0 - b.min_x) as f32 * cell_w + rw / 2.0;
            let sy = oy + (m.1 - b.min_y) as f32 * cell_h + rh / 2.0;
            if sx >= AX && sx <= AX + vw && sy >= AY && sy <= AY + vh {
                continue; // on screen — the pin itself carries it
            }
            let ax = sx.clamp(AX + 3.0, AX + vw - 10.0);
            let ay = sy.clamp(AY + 3.0, AY + vh - 10.0);
            // Octant from the rim toward the marker (canvas y grows DOWN: 0=E, 1=SE ...).
            let ang = (sy - ay).atan2(sx - ax);
            let oct = ((((ang / std::f32::consts::FRAC_PI_4).round() as i32) % 8) + 8) % 8;
            wants.push((ax.round(), ay.round(), oct as usize));
        }
    }
    // Redraw only when the picture changes; the generation salt survives tab sweeps.
    let key = format!("{}|{wants:?}", cx_state.generation);
    if Some(&key) == last.as_ref() {
        return;
    }
    *last = Some(key);
    for e in &old {
        commands.entity(e).despawn();
    }
    if wants.is_empty() {
        return;
    }
    if art.is_empty() {
        *art = OCTANT_ARROWS.iter().map(|g| images.add(crate::gfx::bake(g, ARROW_PAL))).collect();
    }
    for (x, y, oct) in wants {
        commands.spawn((
            Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.6), Vec2::new(9.0, 9.0)),
            at(x - 1.0, y - 1.0, 9.0, 9.0, CONTENT_Z + 0.5),
            PIXEL_LAYER,
            CodexUi,
            TabContent,
            EdgeArrow,
        ));
        commands.spawn((
            Sprite::from_image(art[oct].clone()),
            at(x, y, 7.0, 7.0, CONTENT_Z + 0.51),
            PIXEL_LAYER,
            CodexUi,
            TabContent,
            EdgeArrow,
        ));
    }
}

/// Rooms the quest pins keep in frame (js codex fold: markers + giver rooms).
fn pin_rooms(quests: &crate::app::quests::QuestLog, tmaps: &crate::app::digging::TreasureMaps) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for m in &tmaps.0 {
        out.push((m.rx, m.ry)); // an X keeps its room in frame (js codex fold)
    }
    for q in &quests.0 {
        if let Some(m) = q.marker() {
            out.push(m);
        }
        out.push((q.giver_rx, q.giver_ry));
    }
    out
}

struct Bounds {
    min_x: i32,
    min_y: i32,
    cols: i32,
    rows: i32,
}

fn bounds(visited: &Visited, rx: i32, ry: i32, pins: &[(i32, i32)]) -> Bounds {
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (rx, rx, ry, ry);
    for (x, y) in visited.0.iter().copied().chain(pins.iter().copied()) {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    Bounds { min_x, min_y, cols: max_x - min_x + 1, rows: max_y - min_y + 1 }
}

fn cell_size(ts: i32) -> (f32, f32) {
    ((COLS * ts) as f32 + GAP, (ROWS * ts) as f32 + GAP)
}

fn view_size() -> (f32, f32) {
    (CANVAS_W as f32 - AX * 2.0, CANVAS_H as f32 - TOP_H - FOOT_H - 2.0)
}

/// Where the map's top-left cell lands on the canvas: centre the map if it fits the view,
/// else follow the camera fraction, clamped to the map edges (port of drawMap's offX/offY).
fn root_transform(visited: &Visited, cur: &CurRoom, view: &MapView, pins: &[(i32, i32)]) -> Transform {
    let b = bounds(visited, cur.rx, cur.ry, pins);
    let (cell_w, cell_h) = cell_size(view.ts);
    let full_w = b.cols as f32 * cell_w - GAP;
    let full_h = b.rows as f32 * cell_h - GAP;
    let (vw, vh) = view_size();
    let off = |full: f32, v: f32, c: f32| {
        if full <= v { -((v - full) / 2.0) } else { (c * full - v / 2.0).clamp(0.0, full - v) }
    };
    let mut t = at(AX - off(full_w, vw, view.cx), AY - off(full_h, vh, view.cy), 0.0, 0.0, CONTENT_Z);
    // WHOLE-PIXEL LAW: the camera fraction lands the root on half-pixels, which shears every
    // baked child at the integer upscale — the town names read garbled until a pan happened
    // to re-round them (Baz). Snap the root; the children sit at integer local offsets.
    t.translation.x = t.translation.x.round();
    t.translation.y = t.translation.y.round();
    t
}

/// Build the whole map as children of one root — panning is just moving the root.
#[allow(clippy::too_many_arguments)] // the map's whole context
fn spawn_map(
    commands: &mut Commands,
    world: &GameWorld,
    visited: &Visited,
    cur: &CurRoom,
    view: &MapView,
    cache: &mut ThumbCache,
    images: &mut Assets<Image>,
    quests: &crate::app::quests::QuestLog,
    tmaps: &crate::app::digging::TreasureMaps,
    inv: &crate::inventory::PlayerInv,
    names: &crate::app::banners::TownNames,
    house: &crate::app::home::PlayerHouse,
    relics_whole: bool,
    ppos: (f32, f32),
    death_room: Option<(i32, i32)>,
) {
    let b = bounds(visited, cur.rx, cur.ry, &pin_rooms(quests, tmaps));
    let (cell_w, cell_h) = cell_size(view.ts);
    let (rw, rh) = ((COLS * view.ts) as f32, (ROWS * view.ts) as f32);
    let root = commands
        .spawn((
            root_transform(visited, cur, view, &pin_rooms(quests, tmaps)),
            Visibility::default(),
            MapRoot,
            CodexUi,
            TabContent,
            PIXEL_LAYER,
        ))
        .id();
    // Child at map-space top-left (mx,my), size (w,h): local y flips (root is a canvas point).
    let local = |mx: f32, my: f32, w: f32, h: f32, dz: f32| Transform::from_xyz(mx + w / 2.0, -(my + h / 2.0), dz);
    // The map SHEET: a near-black panel + faint room-grid under everything. The void
    // was pure black, so a pan over unexplored fold didn't visibly move (Baz) — the
    // grid gives the camera something to slide, and hints where rooms could be.
    {
        let full_w = b.cols as f32 * cell_w - GAP;
        let full_h = b.rows as f32 * cell_h - GAP;
        let sheet = commands
            .spawn((
                Sprite::from_color(Color::srgb_u8(0x0b, 0x0b, 0x10), Vec2::new(full_w + 4.0, full_h + 4.0)),
                local(-2.0, -2.0, full_w + 4.0, full_h + 4.0, -0.1),
                PIXEL_LAYER,
            ))
            .id();
        commands.entity(root).add_child(sheet);
        let line = Color::srgb_u8(0x17, 0x17, 0x1f);
        for i in 1..b.cols {
            let e = commands
                .spawn((
                    Sprite::from_color(line, Vec2::new(1.0, full_h + 4.0)),
                    local(i as f32 * cell_w - 1.5, -2.0, 1.0, full_h + 4.0, -0.05),
                    PIXEL_LAYER,
                ))
                .id();
            commands.entity(root).add_child(e);
        }
        for j in 1..b.rows {
            let e = commands
                .spawn((
                    Sprite::from_color(line, Vec2::new(full_w + 4.0, 1.0)),
                    local(-2.0, j as f32 * cell_h - 1.5, full_w + 4.0, 1.0, -0.05),
                    PIXEL_LAYER,
                ))
                .id();
            commands.entity(root).add_child(e);
        }
    }
    for &(x, y) in &visited.0 {
        let mx = (x - b.min_x) as f32 * cell_w;
        let my = (y - b.min_y) as f32 * cell_h;
        let thumb = cache.0.entry((x, y)).or_insert_with(|| images.add(room_thumb(&world.0, x, y))).clone();
        // Dark backing 1px proud of the thumb = the JS per-room outline.
        let backing = commands
            .spawn((
                Sprite::from_color(Color::srgb_u8(0x10, 0x10, 0x10), Vec2::new(rw + 2.0, rh + 2.0)),
                local(mx - 1.0, my - 1.0, rw + 2.0, rh + 2.0, 0.0),
                PIXEL_LAYER,
            ))
            .id();
        let mut sprite = Sprite::from_image(thumb);
        sprite.custom_size = Some(Vec2::new(rw, rh));
        let cell = commands.spawn((sprite, local(mx, my, rw, rh, 0.1), PIXEL_LAYER)).id();
        commands.entity(root).add_child(backing).add_child(cell);
        if x == cur.rx && y == cur.ry {
            // Bright gold ring on the room you're in (js: 3px strokeRect) — shared border
            // geometry, spawned in root-local space because the map root pans.
            let gold = Color::srgb_u8(0xff, 0xd0, 0x00);
            for (sx, sy, sw, sh) in crate::ui::border_strips(mx, my, rw, rh, 2.0) {
                let strip = commands
                    .spawn((Sprite::from_color(gold, Vec2::new(sw, sh)), local(sx, sy, sw, sh, 0.2), PIXEL_LAYER))
                    .id();
                commands.entity(root).add_child(strip);
            }
            // The red player-position dot at the hero's exact tile (js codex 253).
            let ts = view.ts as f32;
            let (ptx, pty) = (((ppos.0 + 8.0) / 16.0).floor(), ((ppos.1 + 8.0) / 16.0).floor());
            let psz = ts.max(2.0);
            let dot = commands
                .spawn((
                    Sprite::from_color(Color::srgb_u8(0xfc, 0x20, 0x20), Vec2::splat(psz)),
                    local(mx + ptx * ts, my + pty * ts, psz, psz, 0.26),
                    PIXEL_LAYER,
                ))
                .id();
            commands.entity(root).add_child(dot);
        }
        // --- THE MARKER LAYER (js codex 194-258): every landmark reads at a glance. ---
        let mark = |commands: &mut Commands, images: &mut Assets<Image>, grid: &[&str], pal: &[(char, u32)]| {
            let img = images.add(crate::gfx::bake(grid, pal));
            let (iw, ih) = (grid[0].len() as f32, grid.len() as f32);
            let e = commands
                .spawn((
                    Sprite::from_image(img),
                    local((mx + rw / 2.0 - iw / 2.0).round(), (my + rh / 2.0 - ih / 2.0).round(), iw, ih, 0.25),
                    PIXEL_LAYER,
                ))
                .id();
            commands.entity(root).add_child(e);
        };
        // A name plate under a marker (town names / HOME), only when it fits the tile.
        let plate = |commands: &mut Commands, images: &mut Assets<Image>, text: &str, col: u32| {
            let tw = crate::gfx::font::measure(text);
            // Even-pad to match bake_text's image width — an odd width centres the sprite on a
            // half-pixel and the integer upscale shears the glyphs (the WINDVALE garble, Baz).
            let w = (tw + (tw & 1)) as f32;
            if w + 2.0 > rw {
                return; // hidden when zoomed out, like the js
            }
            let (img, _) = crate::gfx::font::bake_text(text, col, images);
            let (bx2, by2) = ((mx + rw / 2.0 - w / 2.0).round(), (my + rh / 2.0 + 6.0).round());
            let back = commands
                .spawn((
                    Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.78), Vec2::new(w + 2.0, 7.0)),
                    local(bx2 - 1.0, by2, w + 2.0, 7.0, 0.27),
                    PIXEL_LAYER,
                ))
                .id();
            let txt = commands
                .spawn((Sprite::from_image(img), local(bx2, by2 + 1.0, w, 6.0, 0.28), PIXEL_LAYER))
                .id();
            commands.entity(root).add_child(back).add_child(txt);
        };
        if (x, y) == (crate::worldgen::world::CASTLE_RX, crate::worldgen::world::CASTLE_RY) {
            // The Black Castle: a dark three-tower keep; the gate glows once the heart is whole.
            let gate = if relics_whole { 0xc878ff } else { 0x3a2410 };
            mark(commands, images, CASTLE_MARK, &[('k', 0x0c0a12), ('q', 0x46434f), ('Q', 0x5a5666), ('g', gate)]);
        }
        let ents = world.0.room_entities(x, y);
        let has = |k: &str| ents.iter().any(|e| e.kind == k);
        if has("dungeon") {
            mark(commands, images, SKULL_MARK, &[('b', 0x1a1208), ('W', 0xe4e0d4), ('K', 0x000000)]);
        }
        if has("rift") {
            mark(commands, images, RIFT_MARK, &[('v', 0x0e081a), ('p', 0x7a44c8), ('f', 0xe0b8ff)]);
        }
        if has("shop") {
            mark(commands, images, COIN_MARK, &[('P', 0xfcd000), ('Y', 0x7a5a00)]);
        }
        // Only the town CENTRE (market square) gets a marker — the house icon + the town's
        // NAME, which reads at a glance. The old per-district hollow-square dots peppered the
        // map with mystery boxes on every town-region room (Baz: "not sure why some tiles have
        // this square") and weren't in the JS marker set, so they're gone.
        if let Some(site) = world.0.town_site_of(x, y)
            && (site.tx, site.ty) == (x, y)
        {
            mark(commands, images, TOWN_MARK, &[('b', 0x1a1208), ('r', 0xc83828), ('w', 0xe8d8a0), ('d', 0x6a4a1c)]);
            if let Some(nm) = names.0.get(&format!("{x},{y}")) {
                plate(commands, images, &nm.to_uppercase(), 0xfce0a8);
            }
        }
        if house.0.as_ref().map(|h| h.room) == Some((x, y)) {
            // YOUR home: the marked cottage, star and all.
            mark(commands, images, HOME_MARK, &[('b', 0x1a1208), ('r', 0xc83028), ('w', 0xd8c0c8), ('d', 0x6a4a1c), ('*', 0xfcd000)]);
            plate(commands, images, "HOME", 0xfcd000);
        }
        if death_room == Some((x, y)) {
            // Where you FELL (today only — the corpse bag's window): a plain
            // gravestone, deliberately NOT the dungeon skull.
            mark(commands, images, GRAVE_MARK, &[('b', 0x1a1208), ('s', 0x9aa0ac), ('S', 0x5a6068), ('g', 0x4a8a3a)]);
        }
    }
    // Marked-but-unexplored rooms (quest targets, treasure X's): a dim slate cell so
    // the pin sits on a tile you can pan to, not in bare void (Baz). No terrain leak —
    // the room stays a mystery until you walk it.
    {
        let mut seen: std::collections::HashSet<(i32, i32)> = Default::default();
        for (x, y) in pin_rooms(quests, tmaps) {
            if visited.0.contains(&(x, y)) || !seen.insert((x, y)) {
                continue;
            }
            let mx = (x - b.min_x) as f32 * cell_w;
            let my = (y - b.min_y) as f32 * cell_h;
            let backing = commands
                .spawn((
                    Sprite::from_color(Color::srgb_u8(0x10, 0x10, 0x10), Vec2::new(rw + 2.0, rh + 2.0)),
                    local(mx - 1.0, my - 1.0, rw + 2.0, rh + 2.0, 0.0),
                    PIXEL_LAYER,
                ))
                .id();
            let slate = commands
                .spawn((
                    Sprite::from_color(Color::srgb_u8(0x22, 0x22, 0x2c), Vec2::new(rw, rh)),
                    local(mx, my, rw, rh, 0.05),
                    PIXEL_LAYER,
                ))
                .id();
            commands.entity(root).add_child(backing).add_child(slate);
        }
    }
    // Quest pins (js codex 271): '!' where the job is (until it's ready), '?' at the
    // giver — green the moment you can turn it in.
    let pin = |commands: &mut Commands, images: &mut Assets<Image>, room: (i32, i32), glyph: &str, col: u32| {
        let mx = (room.0 - b.min_x) as f32 * cell_w + rw / 2.0;
        let my = (room.1 - b.min_y) as f32 * cell_h + rh / 2.0;
        let (img, w) = crate::gfx::font::bake_text(glyph, col, images);
        let iw = (w + (w & 1)) as f32;
        let back = commands
            .spawn((
                Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.82), Vec2::new(iw + 4.0, 9.0)),
                local(mx - iw / 2.0 - 2.0, my - 4.5, iw + 4.0, 9.0, 0.3),
                PIXEL_LAYER,
            ))
            .id();
        let text = commands
            .spawn((Sprite::from_image(img), local(mx - iw / 2.0, my - 3.0, iw, 6.0, 0.31), PIXEL_LAYER))
            .id();
        commands.entity(root).add_child(back).add_child(text);
    };
    for q in &quests.0 {
        let ready = q.ready(inv);
        if let Some(m) = q.marker()
            && !ready
        {
            pin(commands, images, m, "!", 0xffd34d); // go here
        }
        // Turn in here (WoW-gold when ready) — story legs resolve in the field,
        // so no '?' waits at their giver.
        if !q.is_story() {
            pin(commands, images, (q.giver_rx, q.giver_ry), "?", if ready { 0xffd34d } else { 0x8aa0c0 });
        }
    }
    // Treasure X's (js codex: every undug chart keeps its mark).
    for m in &tmaps.0 {
        pin(commands, images, (m.rx, m.ry), "X", 0xa02020);
    }

}

/// The DUNGEON FLOOR MAP (js drawDungeonMap): an auto-fit grid of this floor's
/// VISITED rooms — no zoom, no pan. Rooms fill by type, doors are little nubs,
/// treasure/boss/stairs carry marks, the room you stand in gets the gold ring;
/// the theme's name + the floor label (1F / B1 / DEPTH n) head the page, and the
/// chant floor's hymn meter ticks top-right.
fn spawn_dungeon_map(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    run: &crate::app::dungeon::DungeonRun,
    chant: i32,
) {
    use crate::dungeon::{Dir, Door, RoomType};
    let root = commands
        .spawn((at(0.0, 0.0, 0.0, 0.0, CONTENT_Z), Visibility::default(), MapRoot, CodexUi, TabContent, PIXEL_LAYER))
        .id();
    let local = |mx: f32, my: f32, w: f32, h: f32, dz: f32| Transform::from_xyz(mx + w / 2.0, -(my + h / 2.0), dz);
    // Spawn one coloured quad AND parent it to the root (split spawn/attach trips E0499).
    #[allow(clippy::too_many_arguments)]
    fn quad(commands: &mut Commands, root: Entity, tf: Transform, w: f32, h: f32, col: u32, a: f32) {
        let c = Color::srgba(
            ((col >> 16) & 255) as f32 / 255.0,
            ((col >> 8) & 255) as f32 / 255.0,
            (col & 255) as f32 / 255.0,
            a,
        );
        let e = commands.spawn((Sprite::from_color(c, Vec2::new(w, h)), tf, PIXEL_LAYER)).id();
        commands.entity(root).add_child(e);
    }
    let put = |commands: &mut Commands, root: Entity, e: Entity| {
        commands.entity(root).add_child(e);
    };
    let (vw, vh) = view_size();
    // The dark stage (the js full-canvas 0.92 black, inside the codex chrome).
    quad(commands, root, local(AX - 6.0, AY - 1.0, vw + 12.0, vh + 2.0, 0.0), vw + 12.0, vh + 2.0, 0x000000, 0.92);
    // Header: theme name centred, the floor label right (1F / B1 / B2..; DEPTH n in rifts).
    // WHOLE-PIXEL LAW (ui::label): a text sprite renders at its EVEN padded image width;
    // centering with the odd `measure` width lands the quad on a half-pixel, and the
    // canvas upscale then shears every glyph — the wider the string, the worse the
    // garble (Baz: "THE VINE WARREN" unreadable). Pass the even width + floor the x.
    let title = run.dungeon.theme.name.to_uppercase();
    let (timg, tw) = crate::gfx::font::bake_text(&title, 0xfcfcfc, images);
    let tiw = (tw + (tw & 1)) as f32;
    let te = commands
        .spawn((Sprite::from_image(timg), local((AX + vw / 2.0 - tiw / 2.0).floor(), AY + 2.0, tiw, 6.0, 0.1), PIXEL_LAYER))
        .id();
    put(commands, root, te);
    let floor_label = if run.rift > 0 {
        Some(format!("DEPTH {}", run.rift))
    } else if run.dungeon.floors.len() > 1 {
        Some(if run.dungeon.floor == 0 { "1F".into() } else { format!("B{}", run.dungeon.floor) })
    } else {
        None
    };
    if let Some(fl) = floor_label {
        let (fimg, fw) = crate::gfx::font::bake_text(&fl, 0x9ad0ff, images);
        let fiw = (fw + (fw & 1)) as f32;
        let fe = commands
            .spawn((Sprite::from_image(fimg), local((AX + vw - fiw - 2.0).floor(), AY + 2.0, fiw, 6.0, 0.1), PIXEL_LAYER))
            .id();
        put(commands, root, fe);
    }
    // The chant floor's hymn meter — when it fills, a zealot answers (js).
    if run.dungeon.cur().gimmick == Some("chant") {
        use crate::app::saltmaze::CHANT_FRAMES;
        let (w, x0, y0) = (30.0, AX + vw - 32.0, AY + 12.0);
        quad(commands, root, local(x0 - 1.0, y0 - 1.0, w + 2.0, 5.0, 0.1), w + 2.0, 5.0, 0x000000, 0.6);
        let col = if chant > CHANT_FRAMES * 3 / 4 { 0xfc7460 } else { 0xe8dfa8 };
        let fill = (w * chant as f32 / CHANT_FRAMES as f32).round().clamp(0.0, w);
        if fill > 0.0 {
            quad(commands, root, local(x0, y0, fill, 3.0, 0.11), fill, 3.0, col, 1.0);
        }
    }
    // Bounds over the WHOLE floor (stable layout no matter what's explored) — minus
    // the hidden vaults, whose +100,100 keys would wreck the fit (an rs-only layout).
    let fl = run.dungeon.cur();
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for (&(x, y), r) in &fl.rooms {
        if r.vault {
            continue;
        }
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    if min_x > max_x {
        return; // an empty floor never happens, but never divide by it either
    }
    let (gw0, gh0) = ((max_x - min_x + 1) as f32, (max_y - min_y + 1) as f32);
    let (avail_w, avail_h) = (vw - 4.0, vh - 18.0);
    let step = ((avail_w / gw0).min(avail_h / gh0).floor()).clamp(7.0, 20.0);
    let gap = if step >= 14.0 { 4.0 } else if step >= 10.0 { 3.0 } else { 2.0 };
    let cell = step - gap;
    let hc = (cell / 2.0).floor();
    let (gw, gh) = (gw0 * step - gap, gh0 * step - gap);
    let ox = (AX + (vw - gw) / 2.0).round();
    let oy = (AY + 14.0 + (avail_h - gh) / 2.0).round();
    for (&(x, y), d) in &fl.rooms {
        if d.vault || !d.visited {
            continue;
        }
        let dx = ox + (x - min_x) as f32 * step;
        let dy = oy + (y - min_y) as f32 * step;
        let fill = match d.rtype {
            RoomType::Treasure => 0x7a5a18,
            RoomType::Boss => 0x5a2424,
            RoomType::Start | RoomType::Arrival => 0x244a6a,
            RoomType::Stairs => 0x2a4a4a,
            RoomType::Normal => 0x3a3f48,
        };
        quad(commands, root, local(dx, dy, cell, cell, 0.2), cell, cell, fill, 1.0);
        // Door nubs into the gaps between cells.
        for (dir, nx, ny, nw, nh) in [
            (Dir::E, dx + cell, dy + hc - 1.0, gap, 2.0),
            (Dir::W, dx - gap, dy + hc - 1.0, gap, 2.0),
            (Dir::S, dx + hc - 1.0, dy + cell, 2.0, gap),
            (Dir::N, dx + hc - 1.0, dy - gap, 2.0, gap),
        ] {
            if d.door(dir) != Door::None {
                quad(commands, root, local(nx, ny, nw, nh, 0.19), nw, nh, 0x6a6f78, 1.0);
            }
        }
        // Type marks (js): unlooted gold, the boss's red spot, the way out, the stairs.
        match d.rtype {
            RoomType::Treasure if !d.looted => {
                quad(commands, root, local(dx + hc - 2.0, dy + hc - 2.0, 4.0, 4.0, 0.25), 4.0, 4.0, 0xfcd000, 1.0);
            }
            RoomType::Boss => {
                let col = if d.cleared { 0x7a7a7a } else { 0xe23030 };
                quad(commands, root, local(dx + hc - 2.0, dy + hc - 2.0, 4.0, 4.0, 0.25), 4.0, 4.0, col, 1.0);
            }
            RoomType::Start => {
                quad(commands, root, local(dx + hc - 2.0, dy + hc - 3.0, 4.0, 6.0, 0.25), 4.0, 6.0, 0xc4cad4, 1.0);
            }
            _ => {}
        }
        if d.stairs_down.is_some() {
            // The cyan v: the way DEEPER.
            quad(commands, root, local(dx + hc - 2.0, dy + hc - 1.0, 5.0, 1.0, 0.25), 5.0, 1.0, 0x7fe0ff, 1.0);
            quad(commands, root, local(dx + hc - 1.0, dy + hc, 3.0, 1.0, 0.25), 3.0, 1.0, 0x7fe0ff, 1.0);
            quad(commands, root, local(dx + hc, dy + hc + 1.0, 1.0, 1.0, 0.25), 1.0, 1.0, 0x7fe0ff, 1.0);
        }
        if d.stairs_up.is_some() {
            // The green ^: the way BACK UP.
            quad(commands, root, local(dx + hc, dy + hc - 1.0, 1.0, 1.0, 0.25), 1.0, 1.0, 0x9aff9a, 1.0);
            quad(commands, root, local(dx + hc - 1.0, dy + hc, 3.0, 1.0, 0.25), 3.0, 1.0, 0x9aff9a, 1.0);
            quad(commands, root, local(dx + hc - 2.0, dy + hc + 1.0, 5.0, 1.0, 0.25), 5.0, 1.0, 0x9aff9a, 1.0);
        }
        // The outline: gold 2px where you stand, quiet 1px everywhere else.
        let here = (x, y) == (run.drx, run.dry);
        let (bcol, bw) = if here { (0xffd000, 2.0) } else { (0x101010, 1.0) };
        for (sx, sy, sw, sh) in crate::ui::border_strips(dx, dy, cell, cell, bw) {
            quad(commands, root, local(sx, sy, sw, sh, 0.3), sw, sh, bcol, 1.0);
        }
    }
}

// --- The landmark icons (the js fillRect stacks as char grids, same pixels) ---

/// Dungeon: a clear skull on a dark backing (js codex 201-207).
const SKULL_MARK: &[&str] = &[
    "bbbbbbbbbbb",
    "bWWWWWWWWWb",
    "bWWWWWWWWWb",
    "bWKKKWKKKWb",
    "bWKKKWKKKWb",
    "bWKKKWKKKWb",
    "bWWWWWWWWWb",
    "bbWWKWWKWWb",
    "bbWWKWWKWWb",
    "bbWWKWWKWWb",
    "bbbbbbbbbbb",
];

/// Rift spire: a violet tear on a void backing (js codex 209-213).
const RIFT_MARK: &[&str] = &[
    "vvvvvvvvvvv",
    "vvvvpppvvvv",
    "vvvvpfpvvvv",
    "vvpppfpppvv",
    "vvpppfpppvv",
    "vvpppfpppvv",
    "vvpppfpppvv",
    "vvvvpfpvvvv",
    "vvvvpfpvvvv",
    "vvvvpppvvvv",
    "vvvvvvvvvvv",
];

/// Shop: a small gold coin (js codex 215-218).
const COIN_MARK: &[&str] = &["PPPP", "PPPP", "PYYP", "PPPP"];

/// Town market square: roof + chimney, walls, door (js codex 227-230).
const TOWN_MARK: &[&str] = &[
    "bbbbrrbbbbb",
    "bbbbrrbbbbb",
    "rrrrrrrrrrr",
    "rrrrrrrrrrr",
    "rwwwwwwwwwr",
    "bwwwwwwwwwb",
    "bwwwddwwwwb",
    "bwwwddwwwwb",
    "bwwwddwwwwb",
    "bbbbbbbbbbb",
];

/// Where you fell today: a small gravestone over grass — the corpse-run pin.
#[rustfmt::skip]
const GRAVE_MARK: &[&str] = &[
    "...........",
    "...bbbbb...",
    "..bsssssb..",
    "..bssSssb..",
    "..bsSSSsb..",
    "..bssSssb..",
    "..bssSssb..",
    "..bsssssb..",
    "..bsssssb..",
    "..bsSsSsb..",
    "gbbsssssbbg",
    "ggbbbbbbbgg",
    "...........",
];

/// YOUR home: the cottage with the gold star (js codex 241-245).
const HOME_MARK: &[&str] = &[
    "....**.....",
    "bbbb**bbbbb",
    "bbbbbbbbbbb",
    "bbbbrrbbbbb",
    "bbbbrrbbbbb",
    "rrrrrrrrrrr",
    "rrrrrrrrrrr",
    "rwwwwwwwwwr",
    "bwwwwwwwwwb",
    "bwwwddwwwwb",
    "bwwwddwwwwb",
    "bwwwddwwwwb",
    "bbbbbbbbbbb",
];

/// The Black Castle: a dark three-tower keep with the gate (js codex 194-199).
const CASTLE_MARK: &[&str] = &[
    "kkkkkQQkkkkkk",
    "kkkkkQQkkkkkk",
    "kQQkkQQkkQQkk",
    "kQQkkQQkkQQkk",
    "kQQqqQQqqQQqk",
    "kQQqqQQqqQQqk",
    "kqqqqqqqqqqqk",
    "kqqqqggqqqqqk",
    "kqqqqggqqqqqk",
    "kqqqqggqqqqqk",
    "kqqqqggqqqqqk",
    "kkkkkkkkkkkkk",
];

// --- Room thumbnails: 1 px per tile, coloured so every biome reads correctly ---
// (port of GROUND_MINI / WALL_MINI / miniColor in js/codex.js)

const GROUND_MINI: &[(&str, u32)] = &[
    ("grass", 0x2e8a2a),
    ("dirt", 0x6e4a22),
    ("sand", 0xd8c088),
    ("bog", 0x3a5a32),
    ("mud", 0x5a4326),
    ("deadgrass", 0x6a6a3e),
    ("gravedirt", 0x46464a),
    ("snow", 0xdfeef7),
    ("ash", 0x45454a),
    ("spore", 0x5aa050),
    ("chaosground", 0x5a2f8a),
];

const WALL_MINI: &[(char, u32)] = &[
    ('T', 0x0a6a24),
    ('M', 0x808080),
    ('S', 0xc8a060),
    ('R', 0x8c8c8c),
    ('J', 0x2a5a32),
    ('X', 0x7a786e),
    ('I', 0xbfe1f0),
    ('H', 0x3a3a3e),
    ('U', 0x3f8e7c),
    ('Z', 0x7028a8),
];

fn mini_color(world: &crate::worldgen::World, code: char, gx: i32, gy: i32) -> u32 {
    match code {
        '~' => 0x2a6ad8, // water
        'B' => 0x7c4c1c, // bridge
        '_' => 0xe0c890, // town path (tan flagstone)
        '=' => 0x8a5a28, // wilderness dirt road
        c => WALL_MINI
            .iter()
            .find(|(w, _)| *w == c)
            .map(|(_, col)| *col)
            .unwrap_or_else(|| {
                let name = world.ground_name(gx, gy);
                GROUND_MINI.iter().find(|(n, _)| *n == name).map(|(_, col)| *col).unwrap_or(0x8a8a7a)
            }),
    }
}

fn room_thumb(world: &crate::worldgen::World, rx: i32, ry: i32) -> Image {
    let map = world.generate(rx, ry).map;
    let mut img = Image::new_fill(
        Extent3d { width: COLS as u32, height: ROWS as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    for ty in 0..ROWS {
        let row: Vec<char> = map[ty as usize].chars().collect();
        for tx in 0..COLS {
            let code = row.get(tx as usize).copied().unwrap_or('.');
            let hex = mini_color(world, code, rx * COLS + tx, ry * ROWS + ty);
            if let Ok(px) = img.pixel_bytes_mut(UVec3::new(tx as u32, ty as u32, 0)) {
                px.copy_from_slice(&[(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255]);
            }
        }
    }
    // Big-prop pixels so forests/rock fields read on the map (js roomThumb's entity loop).
    for e in world.room_entities(rx, ry) {
        let hex = match e.kind {
            "oak" | "pine" => 0x0a3f14,
            "cactus" => 0x2f7a2f,
            "bush" => 0x1c6a26,
            "boulder" => 0x6a6a6a,
            _ => continue,
        };
        let (tx, ty) = (e.x.div_euclid(16), e.y.div_euclid(16));
        if (0..COLS).contains(&tx)
            && (0..ROWS).contains(&ty)
            && let Ok(px) = img.pixel_bytes_mut(UVec3::new(tx as u32, ty as u32, 0))
        {
            px.copy_from_slice(&[(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255]);
        }
    }
    img
}
