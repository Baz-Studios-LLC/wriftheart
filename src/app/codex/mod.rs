//! codex — the tabbed reference screen (the Map button's world). Port of js/codex.js,
//! REDESIGNED per the improve-don't-copy rule:
//!
//! * The JS dispatched 11 tabs through hardcoded index chains (`codexTab === 7`) that broke
//!   every time a tab was inserted. Here [`TABS`] is a registry of [`TabDef`] rows with
//!   stable [`TabId`]s — adding a tab is one row + one module + one `add_systems` line.
//! * The JS fed codex.js a ~60-variable api snapshot every frame. Here each tab is a Bevy
//!   system that queries exactly what it shows.
//!
//! All 11 js tabs are present in js order — MAP / CALENDAR / PEOPLE / GUILDS / MOBS /
//! ITEMS / SONGS / AWARDS / STATS / LORE / WRIFTHEART. MAP, MOBS, ITEMS, CALENDAR and
//! STATS render their systems; the rest show their js true-empty states (stubs.rs) and
//! upgrade in place as villagers / guildhalls / songs / achievements / lore books / the
//! shard quest port.

pub mod calendar_tab;
pub mod dex;
pub mod items_tab;
pub mod lore_tab;
pub mod map_tab;
pub mod mobs_tab;
pub mod awards_tab;
pub mod songs_tab;
pub mod guilds_tab;
pub mod stats_tab;
pub mod wriftheart_tab;
pub mod people_tab;
pub mod stubs;

use super::play::EndTick;
use super::screen::Screen;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::ui::label;
use crate::{CANVAS_H, CANVAS_W};
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TabId {
    Map,
    Calendar,
    People,
    Guilds,
    Mobs,
    Items,
    Songs,
    Awards,
    Stats,
    Lore,
    Wriftheart,
}

pub struct TabDef {
    pub id: TabId,
    pub title: &'static str,
    /// Builds the tab's footer hint (derived prompts only — never hardcode a key name).
    pub hint: fn(&Bindings, bool) -> String,
}

/// THE tab registry — the js CODEX_TABS list, same order. Nothing indexes it by number.
pub const TABS: &[TabDef] = &[
    TabDef { id: TabId::Map, title: "MAP", hint: map_tab::hint },
    TabDef { id: TabId::Calendar, title: "CALENDAR", hint: calendar_tab::hint },
    TabDef { id: TabId::People, title: "PEOPLE", hint: people_tab::hint },
    TabDef { id: TabId::Guilds, title: "GUILDS", hint: guilds_tab::hint },
    TabDef { id: TabId::Mobs, title: "MOBS", hint: mobs_tab::hint },
    TabDef { id: TabId::Items, title: "ITEMS", hint: items_tab::hint },
    TabDef { id: TabId::Songs, title: "SONGS", hint: songs_tab::hint },
    TabDef { id: TabId::Awards, title: "AWARDS", hint: awards_tab::hint },
    TabDef { id: TabId::Stats, title: "STATS", hint: stats_tab::hint },
    TabDef { id: TabId::Lore, title: "LORE", hint: lore_tab::hint },
    TabDef { id: TabId::Wriftheart, title: "WRIFTHEART", hint: wriftheart_tab::hint },
];

/// Codex state: which tab, and a generation counter tab systems watch to know "you just
/// became visible — rebuild your entities" (bumped on open and on every tab switch).
#[derive(Resource)]
pub struct CodexState {
    pub tab: usize, // index into TABS; persists across opens (JS behaviour)
    pub generation: u32,
}

/// Marker on EVERY codex entity (frame + tab content) — wholesale despawn on close.
#[derive(Component, Clone)]
pub struct CodexUi;
/// Marker on the frame chrome only (tab strip + hint), redrawn on tab change.
#[derive(Component)]
pub struct CodexChrome;
/// Marker on tab-owned content, cleared on tab change (the owner rebuilds via generation).
#[derive(Component, Clone)]
pub struct TabContent;

/// Run condition for a tab's systems: codex open AND this tab active.
pub fn tab_active(id: TabId) -> impl Fn(Res<State<Screen>>, Res<CodexState>) -> bool {
    move |screen, cx| *screen.get() == Screen::Codex && TABS[cx.tab].id == id
}

pub struct CodexPlugin;

impl Plugin for CodexPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CodexState { tab: 0, generation: 0 })
            .init_resource::<map_tab::MapView>()
            .init_resource::<map_tab::ThumbCache>()
            .init_resource::<mobs_tab::MobDex>()
            .init_resource::<mobs_tab::Bestiary>()
            .init_resource::<items_tab::ItemDex>()
            .init_resource::<items_tab::Discovered>()
            .init_resource::<stats_tab::StatsView>()
            .init_resource::<lore_tab::LoreDex>()
            .init_resource::<people_tab::PeopleDex>()
            .init_resource::<awards_tab::Unlocked>()
            .init_resource::<awards_tab::AwardsDex>()
            .add_systems(
                FixedUpdate,
                (
                    codex_tick.after(super::menu::menu_tick),
                    map_tab::run.run_if(tab_active(TabId::Map)).after(codex_tick),
                    map_tab::edge_arrows.run_if(tab_active(TabId::Map)).after(map_tab::run),
                    map_tab::void_backdrop.run_if(tab_active(TabId::Map)).after(map_tab::run),
                    calendar_tab::run.run_if(tab_active(TabId::Calendar)).after(codex_tick),
                    mobs_tab::run.run_if(tab_active(TabId::Mobs)).after(codex_tick),
                    items_tab::run.run_if(tab_active(TabId::Items)).after(codex_tick),
                    stats_tab::run.run_if(tab_active(TabId::Stats)).after(codex_tick),
                    people_tab::run.run_if(tab_active(TabId::People)).after(codex_tick),
                    guilds_tab::run.run_if(tab_active(TabId::Guilds)).after(codex_tick),
                    songs_tab::run.run_if(tab_active(TabId::Songs)).after(codex_tick),
                    awards_tab::run.run_if(tab_active(TabId::Awards)).after(codex_tick),
                    lore_tab::run.run_if(tab_active(TabId::Lore)).after(codex_tick),
                    wriftheart_tab::run.run_if(tab_active(TabId::Wriftheart)).after(codex_tick),
                )
                    .before(EndTick),
            )
            .add_systems(bevy::app::FixedUpdate, awards_tab::award_ticker.run_if(crate::app::screen::playing))
            .add_systems(Update, items_tab::track_discovered)
            .add_systems(OnExit(Screen::Codex), (close_codex, lore_tab::reset, people_tab::reset));
    }
}

/// Open (Map from play), close (Map/Pause inside), and cycle tabs (TabPrev/TabNext).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn codex_tick(
    mut commands: Commands,
    state: Res<ActionState>,
    bindings: Res<Bindings>,
    screen: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
    mut cx: ResMut<CodexState>,
    chrome: Query<Entity, With<CodexChrome>>,
    content: Query<Entity, With<TabContent>>,
    mut images: ResMut<Assets<Image>>,
    lore: Res<lore_tab::LoreDex>,
    ptr: Res<crate::input::Pointer>,
) {
    match screen.get() {
        Screen::Play => {
            // EVERY opener jumps to ITS page — Map included (Baz: "M should always
            // open the map"; it used to reopen the last-viewed tab). Each tab also
            // has its own quick-access action (unbound by default; CONTROLS offers all).
            const OPENERS: [(Action, TabId); 11] = [
                (Action::Map, TabId::Map),
                (Action::Calendar, TabId::Calendar),
                (Action::People, TabId::People),
                (Action::Guilds, TabId::Guilds),
                (Action::Mobs, TabId::Mobs),
                (Action::ItemsDex, TabId::Items),
                (Action::Songs, TabId::Songs),
                (Action::Awards, TabId::Awards),
                (Action::StatsTab, TabId::Stats),
                (Action::Lore, TabId::Lore),
                (Action::Wriftheart, TabId::Wriftheart),
            ];
            for (action, tab) in OPENERS {
                if state.pressed(action) {
                    let idx = TABS.iter().position(|t| t.id == tab);
                    open(&mut commands, &mut next, &mut cx, &chrome, &bindings, &state, &mut images, idx);
                    break;
                }
            }
        }
        Screen::Codex => {
            // CANCEL (pad B / X key) backs out one LAYER: an open tome sets down first
            // (lore_tab handles that press), then the codex itself closes.
            let cancel = state.pressed(Action::Slot2) && lore.reading.is_none();
            if cancel || state.pressed(Action::Map) || state.pressed(Action::Pause) {
                next.set(Screen::Play); // OnExit clears every CodexUi entity
                return;
            }
            let n = TABS.len();
            let mut target = cx.tab;
            if state.pressed(Action::TabNext) {
                target = (target + 1) % n;
            }
            if state.pressed(Action::TabPrev) {
                target = (target + n - 1) % n; // -1, wrapped
            }
            // Mouse: click a tab chip to jump straight to it.
            if ptr.click
                && let Some((i, ..)) = tab_chips().into_iter().find(|&(_, x, y, w, h)| ptr.over(x, y, w, h))
            {
                target = i;
            }
            if target != cx.tab {
                cx.tab = target;
                cx.generation += 1;
                for e in &content {
                    commands.entity(e).despawn(); // the incoming tab rebuilds via generation
                }
                draw_frame(&mut commands, &chrome, &cx, &bindings, &state, &mut images);
            }
        }
        _ => {} // pause menu / slide-out own their own inputs
    }
}

/// Programmatic open — shared by the Map-press path and the WRIFT_SHOT harness.
/// `tab` picks a specific tab (by TABS index) or keeps the last one.
#[allow(clippy::too_many_arguments)] // it IS the open call's arity
pub fn open(
    commands: &mut Commands,
    next: &mut NextState<Screen>,
    cx: &mut CodexState,
    chrome: &Query<Entity, With<CodexChrome>>,
    bindings: &Bindings,
    state: &ActionState,
    images: &mut Assets<Image>,
    tab: Option<usize>,
) {
    if let Some(t) = tab {
        cx.tab = t;
    }
    next.set(Screen::Codex);
    cx.generation += 1; // tells the active tab's system "you just became visible"
    draw_frame(commands, chrome, cx, bindings, state, images);
}

/// Leave: clear the whole codex and swallow held face buttons (zoom uses Slot1/Slot3 —
/// the heldLatch rule keeps a held zoom key from swinging the sword on close).
fn close_codex(
    mut commands: Commands,
    mut state: ResMut<ActionState>,
    ui: Query<Entity, With<CodexUi>>,
) {
    for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
        state.latch(a);
    }
    for e in &ui {
        commands.entity(e).despawn();
    }
}

// Frame geometry shared with the tabs: content sits between the tab strip and the footer.
pub const TOP_H: f32 = 15.0; // tab strip band
pub const FOOT_H: f32 = 13.0; // hint band
// The codex band sits ABOVE the HUD's ceiling (sidebar text tops out at ~18.7 since the
// tree-spill z-hoist) and below the settings menu (20) — the js drew the codex over the
// whole frame, sidebar included.
const OVERLAY_Z: f32 = 18.9;
pub const CONTENT_Z: f32 = 19.0; // tabs draw their content in [19, 19.6]
const BAND_Z: f32 = 19.7; // opaque top/footer bands "clip" overtall content
const TEXT_Z: f32 = 19.8;

/// The codex frame: near-black overlay, opaque top/footer bands, the tab strip (gold top
/// rule on the active tab), and the active tab's derived-prompt hint.
/// The tab chips as (index, x, y, w, h) — ONE geometry source for `draw_frame` (drawing) and
/// `codex_tick` (mouse hit-testing), so a click lands on exactly the chip that's drawn.
fn tab_chips() -> Vec<(usize, f32, f32, f32, f32)> {
    let mut tx = 6.0;
    TABS.iter()
        .enumerate()
        .map(|(i, tab)| {
            let tw = font::measure(tab.title) as f32 + 8.0;
            let chip = (i, tx, 2.0, tw, 11.0);
            tx += tw + 2.0;
            chip
        })
        .collect()
}

fn draw_frame(
    commands: &mut Commands,
    old: &Query<Entity, With<CodexChrome>>,
    cx: &CodexState,
    bindings: &Bindings,
    state: &ActionState,
    images: &mut Assets<Image>,
) {
    for e in old {
        commands.entity(e).despawn();
    }
    let (w, h) = (CANVAS_W as f32, CANVAS_H as f32);
    // Overlay across the whole canvas, sidebar included. The JS used rgba(0,0,0,0.92);
    // Bevy alpha-blends in LINEAR space so 0.92 reads far too see-through — opaque
    // near-black matches the JS look on screen.
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x05, 0x05, 0x08), Vec2::new(w, h)),
        at(0.0, 0.0, w, h, OVERLAY_Z),
        PIXEL_LAYER,
        CodexUi,
        CodexChrome,
    ));
    // Opaque bands: content that pans/zooms past its area vanishes under them.
    let band = Color::srgb_u8(0x06, 0x08, 0x0e);
    for (y, bh) in [(0.0, TOP_H), (h - FOOT_H, FOOT_H)] {
        commands.spawn((
            Sprite::from_color(band, Vec2::new(w, bh)),
            at(0.0, y, w, bh, BAND_Z),
            PIXEL_LAYER,
            CodexUi,
            CodexChrome,
        ));
    }
    // Tab strip (port of drawCodexTabs: variable-width tabs, active = lit bg + gold rule).
    for (i, tx, ty, tw, th) in tab_chips() {
        let on = i == cx.tab;
        let bg = if on { Color::srgb_u8(0x2a, 0x2a, 0x18) } else { Color::srgb_u8(0x14, 0x14, 0x18) };
        commands.spawn((
            Sprite::from_color(bg, Vec2::new(tw, th)),
            at(tx, ty, tw, th, TEXT_Z),
            PIXEL_LAYER,
            CodexUi,
            CodexChrome,
        ));
        if on {
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0xff, 0xd3, 0x4d), Vec2::new(tw, 1.0)),
                at(tx, ty, tw, 1.0, TEXT_Z + 0.1),
                PIXEL_LAYER,
                CodexUi,
                CodexChrome,
            ));
        }
        let color = if on { 0xfcfcfc } else { 0x6c6c74 };
        label(commands, images, TABS[i].title, tx + 4.0, ty + 2.0, color, TEXT_Z + 0.1, (CodexUi, CodexChrome));
    }
    // Footer hint, right-aligned (derived prompts — the tab builds its own).
    let hint = (TABS[cx.tab].hint)(bindings, state.pad_present);
    let hx = w - 6.0 - font::measure(&hint) as f32;
    label(commands, images, &hint, hx, h - 10.0, 0x606060, TEXT_Z, (CodexUi, CodexChrome));
}

/// The shared "Q/R TABS - ... - M CLOSE" scaffold every tab hint starts and ends with.
pub fn hint_scaffold(bindings: &Bindings, pad: bool, middle: &str) -> String {
    let tabs = format!(
        "{}/{} TABS",
        bindings.prompt(Action::TabPrev, pad),
        bindings.prompt(Action::TabNext, pad)
    );
    let close = format!("{} CLOSE", bindings.prompt(Action::Map, pad));
    if middle.is_empty() {
        format!("{tabs} - {close}")
    } else {
        format!("{tabs} - {middle} - {close}")
    }
}
