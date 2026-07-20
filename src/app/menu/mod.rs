//! menu/ — the pause menu (port of js/menu.js): a tabbed centered panel over the dimmed,
//! frozen world. GAME (resume/save/autosave/exit) · VIDEO · SOUND · CONTROLS (rebind).
//! Tab switch = TabPrev/TabNext, move = up/down, confirm = slot1, close = pause/slot2 —
//! the js contract. CO-OP is the one js tab NOT here (co-op is post-parity scope).
//!
//! DEVIATIONS (on-screen): GAME has EXIT GAME where the js has QUIT TO TITLE + a slot
//! picker (both arrive with the title screen / saves increment 2 — SAVE writes the one
//! autosave slot and flashes SAVED!); VIDEO has a real FULLSCREEN toggle where the js
//! could only print an OS shortcut hint its webview couldn't act on.

mod controls;
mod tabs;

use super::save::SaveRequest;
use super::screen::Screen;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{key_bindable, mouse_bindable, Action, ActionState, Bindings};
use crate::settings::{store, Settings};
use crate::ui::frame_rect;
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use crate::{CANVAS_H, CANVAS_W};

// js: PW=280, PH=180 — centered leaves a dimmed margin all round.
const PW: f32 = 280.0;
const PH: f32 = 180.0;
pub(crate) const GOLD: u32 = 0xfce0a8; // the js pause-menu accent
pub(crate) const MUTED: u32 = 0xa0a0a0; // unselected row text
const DIM_Z: f32 = 19.85; // over the codex band (…19.8), under nothing — Pause owns the screen
const Z: f32 = 19.9;
const TEXT_Z: f32 = 20.1;

#[derive(Resource, Default)]
pub struct MenuState {
    pub(crate) tab: usize, // index into the VISIBLE tabs (settings-only hides GAME)
    pub(crate) index: usize,
    saved_flash: u32,     // SAVE row shows SAVED! while > 0
    exit_in: Option<u8>,  // EXIT GAME countdown: lets the queued SaveRequest flush first
    title_in: Option<u8>, // QUIT TO TITLE countdown — same flush rule
    settings_only: bool,  // opened from the TITLE (js openSettings): no GAME tab
}

impl MenuState {
    /// Visible tab index -> the real TITLES index (settings-only starts at VIDEO).
    fn real_tab(&self) -> usize {
        self.tab + if self.settings_only { 1 } else { 0 }
    }
    fn n_tabs(&self) -> usize {
        tabs::TITLES.len() - if self.settings_only { 1 } else { 0 }
    }
}

/// WRIFT_SHOT staging: the index of a menu tab by title (GAME, VIDEO, SOUND, CONTROLS).
pub fn tab_index(name: &str) -> Option<usize> {
    tabs::TITLES.iter().position(|t| t.eq_ignore_ascii_case(name))
}

/// Raw next-input capture for the CONTROLS rebind flow. A render-frame system fills `got`
/// (the fixed clock can't see just_pressed reliably); menu_tick consumes it.
#[derive(Resource, Default)]
pub struct Capture {
    active: bool,
    fresh: bool, // skip the remainder of the starting frame, so the CONFIRM press itself isn't captured
    got: Option<Captured>,
}

enum Captured {
    Key(KeyCode),
    Pad(GamepadButton),
    Mouse(MouseButton),
}

#[derive(Component, Clone)]
pub struct MenuUi;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuState>()
            .init_resource::<Capture>()
            .add_systems(FixedUpdate, menu_tick)
            .add_systems(Update, capture_raw)
            .add_systems(OnEnter(Screen::TitleOptions), open_title_options)
            .add_systems(OnExit(Screen::Pause), close_menu)
            .add_systems(OnExit(Screen::TitleOptions), close_menu);
    }
}

/// The title's OPTIONS row: the same panel, GAME tab hidden (js Menu.openSettings —
/// there's no game running to save or quit).
fn open_title_options(
    mut commands: Commands,
    mut menu: ResMut<MenuState>,
    settings: Res<Settings>,
    bindings: Res<Bindings>,
    state: Res<ActionState>,
    ui: Query<Entity, With<MenuUi>>,
    mut images: ResMut<Assets<Image>>,
) {
    *menu = MenuState { settings_only: true, ..default() };
    redraw(&mut commands, &ui, &mut images, &menu, &settings, &bindings, &state, false);
}

/// Grab the next raw key, mouse button, or pad button while a rebind capture is armed (js
/// Input.capture). A captured mouse click is consumed here, so it doesn't also click a menu row.
fn capture_raw(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    pads: Query<&Gamepad>,
    mut cap: ResMut<Capture>,
) {
    if !cap.active || cap.got.is_some() {
        return;
    }
    if cap.fresh {
        cap.fresh = false;
        return;
    }
    // Only keys the font can print a prompt for; Escape passes through as the cancel.
    if let Some(k) = keys.get_just_pressed().find(|k| key_bindable(**k)) {
        cap.got = Some(Captured::Key(*k));
        return;
    }
    if let Some(b) = mouse.get_just_pressed().find(|b| mouse_bindable(**b)) {
        cap.got = Some(Captured::Mouse(*b));
        return;
    }
    for g in &pads {
        if let Some(b) = g.get_just_pressed().next() {
            cap.got = Some(Captured::Pad(*b));
            return;
        }
    }
}

/// Swallow any face button still held when leaving the menu, so it can't act the instant
/// gameplay resumes (the JS heldLatch rule) — and clear the window. OnExit fires for every
/// way out (RESUME, Start-close), so the rule can't be forgotten on a new path.
fn close_menu(
    mut commands: Commands,
    mut state: ResMut<ActionState>,
    mut capture: ResMut<Capture>,
    ui: Query<Entity, With<MenuUi>>,
) {
    for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
        state.latch(a);
    }
    capture.active = false;
    capture.got = None;
    for e in &ui {
        commands.entity(e).despawn();
    }
}

/// The menu's draw context — the shared [`Pen`] carrying the MenuUi marker.
pub(crate) type Draw<'a, 'w, 's> = crate::ui::Pen<'a, 'w, 's, MenuUi>;

/// The tab content rect (js `a` in Menu.draw).
pub(crate) struct Area {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Toggle + drive the menu on the fixed clock (same consume-once press semantics as play).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn menu_tick(
    mut commands: Commands,
    mut state: ResMut<ActionState>,
    mut bindings: ResMut<Bindings>,
    screen: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
    mut menu: ResMut<MenuState>,
    mut settings: ResMut<Settings>,
    mut capture: ResMut<Capture>,
    ui: Query<Entity, With<MenuUi>>,
    mut images: ResMut<Assets<Image>>,
    mut exit: MessageWriter<AppExit>,
    mut saves: MessageWriter<SaveRequest>,
    ptr: Res<crate::input::Pointer>,
) {
    match screen.get() {
        Screen::Play => {
            if state.pressed(Action::Pause) {
                *menu = MenuState::default();
                next.set(Screen::Pause);
                redraw(&mut commands, &ui, &mut images, &menu, &settings, &bindings, &state, false);
            }
        }
        Screen::Pause | Screen::TitleOptions => {
            let back = if menu.settings_only { Screen::Title } else { Screen::Play };
            // EXIT GAME / QUIT TO TITLE queued a SaveRequest last tick; give
            // save_on_request a tick to flush it before moving on.
            if let Some(n) = menu.exit_in {
                match n {
                    0 => {
                        exit.write(AppExit::Success);
                    }
                    _ => menu.exit_in = Some(n - 1),
                }
                return;
            }
            if let Some(n) = menu.title_in {
                match n {
                    0 => {
                        menu.title_in = None;
                        next.set(Screen::Title);
                    }
                    _ => menu.title_in = Some(n - 1),
                }
                return;
            }
            // The menu owns the face buttons while open (heldLatch, so holds can't leak).
            for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
                state.latch(a);
            }
            // Rebind capture freezes the menu until an input lands (js: update() bails).
            if capture.active {
                if let Some(got) = capture.got.take() {
                    capture.active = false;
                    match got {
                        Captured::Key(KeyCode::Escape) => {} // js: Escape aborts the capture
                        Captured::Key(k) => {
                            if let Some(a) = controls::action_at(menu.index) {
                                bindings.rebind_key(a, k);
                                store(&mut settings, &bindings);
                            }
                        }
                        Captured::Pad(b) => {
                            if let Some(a) = controls::action_at(menu.index) {
                                bindings.rebind_pad(a, b);
                                store(&mut settings, &bindings);
                            }
                        }
                        Captured::Mouse(b) => {
                            if let Some(a) = controls::action_at(menu.index) {
                                bindings.rebind_mouse(a, b);
                                store(&mut settings, &bindings);
                            }
                        }
                    }
                    redraw(&mut commands, &ui, &mut images, &menu, &settings, &bindings, &state, false);
                }
                return;
            }
            if menu.saved_flash > 0 {
                menu.saved_flash -= 1;
                if menu.saved_flash == 0 {
                    redraw(&mut commands, &ui, &mut images, &menu, &settings, &bindings, &state, false);
                }
            }
            if state.pressed(Action::Pause) || state.pressed(Action::Slot2) {
                next.set(back); // OnExit latches + clears
                return;
            }
            let tabs_n = menu.n_tabs();
            let mut dirty = false;
            if state.pressed(Action::TabNext) {
                menu.tab = (menu.tab + 1) % tabs_n;
                menu.index = 0;
                dirty = true;
            } else if state.pressed(Action::TabPrev) {
                menu.tab = (menu.tab + tabs_n - 1) % tabs_n;
                menu.index = 0;
                dirty = true;
            } else {
                let rows = rows_len(menu.real_tab(), &settings, &menu);
                if state.pressed(Action::Up) {
                    menu.index = (menu.index + rows - 1) % rows;
                    dirty = true;
                }
                if state.pressed(Action::Down) {
                    menu.index = (menu.index + 1) % rows;
                    dirty = true;
                }
                // Slot1 OR Enter — Slot1 now defaults to LMB (which no longer fires in menus),
                // so ENTER is the keyboard confirm; a bound pad/key Slot1 still works.
                if state.pressed(Action::Slot1) || state.pressed(Action::MenuConfirm) {
                    dirty |= confirm(
                        &mut menu, &mut settings, &mut bindings, &mut capture, &mut next, &mut saves,
                    );
                }
            }
            // --- Mouse (Baz's request): hover highlights a row, LMB selects it, and clicking a
            // chip switches tabs. Rebind capture consumed its own clicks above and returned, so
            // this only runs for a live (non-capturing) menu. ---
            let off = tabs::TITLES.len() - tabs_n;
            let hover_tab = tab_chips(&menu)
                .into_iter()
                .find(|&(_, x, y, w, h)| ptr.over(x, y, w, h))
                .map(|(i, ..)| i);
            if let Some(abs) = hover_tab {
                if ptr.click && abs - off != menu.tab {
                    menu.tab = abs - off;
                    menu.index = 0;
                    dirty = true;
                }
            } else if let Some(r) =
                ptr.pos.and_then(|p| row_at(menu.real_tab(), &content_area(), p, &settings, &menu))
            {
                if ptr.moved && menu.index != r {
                    menu.index = r;
                    dirty = true;
                }
                if ptr.click {
                    menu.index = r;
                    dirty |= confirm(
                        &mut menu, &mut settings, &mut bindings, &mut capture, &mut next, &mut saves,
                    );
                }
            }
            if dirty {
                redraw(&mut commands, &ui, &mut images, &menu, &settings, &bindings, &state, capture.active);
            }
        }
        _ => {} // codex / slide-out own their own inputs
    }
}

fn rows_len(tab: usize, settings: &Settings, menu: &MenuState) -> usize {
    match tab {
        3 => controls::len(),
        t => tabs::list_rows(t, settings, menu.saved_flash).len(),
    }
}

/// The selected row was confirmed — act on it. Returns whether a redraw is due.
fn confirm(
    menu: &mut MenuState,
    settings: &mut Settings,
    bindings: &mut Bindings,
    capture: &mut Capture,
    next: &mut NextState<Screen>,
    saves: &mut MessageWriter<SaveRequest>,
) -> bool {
    match menu.real_tab() {
        0 => match menu.index {
            0 => {
                next.set(Screen::Play);
                false
            }
            1 => {
                saves.write(SaveRequest);
                menu.saved_flash = 90;
                true
            }
            2 => {
                settings.autosave = !settings.autosave;
                store(settings, bindings);
                true
            }
            // Both exits save first and move on two ticks later (js quits save-first).
            3 => {
                saves.write(SaveRequest);
                menu.title_in = Some(2);
                false
            }
            _ => {
                saves.write(SaveRequest);
                menu.exit_in = Some(2);
                false
            }
        },
        3 => {
            if menu.index == controls::len() - 1 {
                bindings.reset();
                store(settings, bindings);
            } else {
                capture.active = true;
                capture.fresh = true;
                capture.got = None;
            }
            true
        }
        t => {
            tabs::confirm_setting(t, menu.index, settings);
            store(settings, bindings);
            true
        }
    }
}

/// The panel's top-left corner (js px/py) — the anchor every other rect derives from.
fn panel_xy() -> (f32, f32) {
    (
        ((CANVAS_W as f32 - PW) / 2.0).round(),
        ((CANVAS_H as f32 - PH) / 2.0).round(),
    )
}

/// The visible tab chips as (absolute tab index, x, y, w, h) — ONE geometry source shared by
/// `redraw` (drawing) and `menu_tick` (mouse hit-testing), so the two can never drift.
fn tab_chips(menu: &MenuState) -> Vec<(usize, f32, f32, f32, f32)> {
    let (px, py) = panel_xy();
    let (ix, tab_h, tab_y) = (px + 12.0, 11.0, py + 9.0);
    let off = tabs::TITLES.len() - menu.n_tabs();
    let mut tx = ix;
    tabs::TITLES
        .iter()
        .enumerate()
        .skip(off)
        .map(|(i, title)| {
            let cw = font::measure(title) as f32 + 8.0;
            let chip = (i, tx, tab_y, cw, tab_h);
            tx += cw + 3.0;
            chip
        })
        .collect()
}

/// The tab content rect (js `a` in Menu.draw) — same single source for draw + hit-test.
fn content_area() -> Area {
    let (px, py) = panel_xy();
    let (ix, iw) = (px + 12.0, PW - 24.0);
    let ay = py + 9.0 + 11.0 + 9.0; // tab_y + tab_h + gap
    Area { x: ix, y: ay, w: iw, h: py + PH - ay - 15.0 }
}

/// The absolute row index under a canvas point, or None (over a gap / header / outside). Mirrors
/// the two row layouts: the scrolling CONTROLS table (tab 3, controls::draw) and the centred
/// settings list (tabs::draw_list). Kept next to `content_area` so the geometry stays paired.
fn row_at(tab: usize, a: &Area, p: Vec2, settings: &Settings, menu: &MenuState) -> Option<usize> {
    if p.x < a.x || p.x >= a.x + a.w || p.y < a.y {
        return None;
    }
    if tab == 3 {
        let (rh, y0) = (10.0, a.y + 11.0);
        let vis = (((a.y + a.h - y0) / rh).floor() as usize).max(1);
        let n = controls::len();
        let scroll = if n > vis {
            (menu.index as i32 - (vis / 2) as i32).clamp(0, (n - vis) as i32) as usize
        } else {
            0
        };
        let vi = ((p.y - y0) / rh).floor();
        if vi < 0.0 {
            return None;
        }
        let vi = vi as usize;
        (vi < vis.min(n - scroll)).then_some(scroll + vi)
    } else {
        let rows = rows_len(tab, settings, menu);
        let rh = 18.0;
        let y0 = a.y + (((a.h - rows as f32 * rh) / 2.0).round()).max(0.0);
        if p.y < y0 {
            return None;
        }
        let ri = ((p.y - y0) / rh).floor() as usize;
        (ri < rows).then_some(ri)
    }
}

/// (Re)build the whole panel — js Menu.draw, chrome verbatim (280x180, chip tabs, hint).
#[allow(clippy::too_many_arguments)] // one redraw = the full window state
fn redraw(
    commands: &mut Commands,
    old: &Query<Entity, With<MenuUi>>,
    images: &mut Assets<Image>,
    menu: &MenuState,
    settings: &Settings,
    bindings: &Bindings,
    state: &ActionState,
    capturing: bool,
) {
    for e in old {
        commands.entity(e).despawn();
    }
    let mut d = Draw { commands, images, marker: MenuUi };
    let (w, h) = (CANVAS_W as f32, CANVAS_H as f32);
    // js dims rgba(0,0,0,0.6); Bevy blends in LINEAR space, so 0.6 reads too thin — 0.72
    // lands the same on screen (see the codex overlay note).
    d.commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.72), Vec2::new(w, h)),
        at(0.0, 0.0, w, h, DIM_Z),
        PIXEL_LAYER,
        MenuUi,
    ));
    let px = ((w - PW) / 2.0).round();
    let py = ((h - PH) / 2.0).round();
    d.fill(px, py, PW, PH, 0x0c0c10, Z); // panel body
    frame_rect(d.commands, px, py, PW, PH, 0x2a2a30, Z + 0.05, MenuUi);
    d.fill(px, py, PW, 1.0, GOLD, Z + 0.1); // gold top accent

    let pad = 12.0;
    let (ix, iw, cx) = (px + pad, PW - pad * 2.0, px + PW / 2.0);

    // Tab bar — codex-style chips, left-aligned from the panel's inner edge (only the
    // visible tabs: settings-only hides GAME). Geometry from tab_chips so mouse hit-testing
    // lands on exactly what's drawn.
    for (i, tx, tab_y, cw, tab_h) in tab_chips(menu) {
        let on = i == menu.real_tab();
        d.fill(tx, tab_y, cw, tab_h, if on { 0x26262e } else { 0x141418 }, Z + 0.1);
        if on {
            d.fill(tx, tab_y, cw, 1.0, GOLD, Z + 0.15); // active accent
        }
        d.text(tabs::TITLES[i], tx + 4.0, tab_y + 4.0, if on { 0xfcfcfc } else { 0x6c6c74 }, TEXT_Z);
    }
    d.fill(ix, py + 9.0 + 11.0 + 2.0, iw, 1.0, 0x2a2a30, Z + 0.1); // underline

    let a = content_area();
    match menu.real_tab() {
        3 => controls::draw(&mut d, &a, menu.index, capturing, bindings),
        t => tabs::draw_list(&mut d, &a, &tabs::list_rows(t, settings, menu.saved_flash), menu.index),
    }

    let hint = format!(
        "{} SELECT - {}/{} TABS - {} CLOSE",
        bindings.prompt(Action::Slot1, state.pad_present),
        bindings.prompt(Action::TabPrev, state.pad_present),
        bindings.prompt(Action::TabNext, state.pad_present),
        bindings.prompt(Action::Pause, state.pad_present),
    );
    d.text_center(&hint, cx, py + PH - 11.0, 0x606060, TEXT_Z);
}
