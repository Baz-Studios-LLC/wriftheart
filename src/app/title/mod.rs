//! title/ — the start menu (port of js/titlescreen.js). The game BOOTS here: the world
//! spawns frozen underneath (already loaded from the newest slot) and the title covers it
//! with the flyover backdrop. CONTINUE / LOAD / NEW all funnel through loader.rs's
//! LoadSlot message; OPTIONS opens the settings-only pause panel (Screen::TitleOptions);
//! idling ~7s starts the attract-mode story crawl (crawl.rs).
//!
//! NOT here (flagged): JOIN A FRIEND (co-op is post-parity) and the character creator
//! (its own milestone — NEW GAME starts a default HERO directly).

mod crawl;
mod flyover;
pub mod loader;
mod slots;

use super::save::{latest_slot, scan_metas, SlotMetas};
use super::screen::Screen;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings, ACTIONS};
use crate::{CANVAS_H, CANVAS_W};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use loader::LoadSlot;

// The z ladder: everything sits over the frozen play world + HUD (<= 18.7) and under the
// options panel (19.85+), so OPTIONS renders over the title like any modal.
const BLACK_Z: f32 = 18.72;
const GRAD_Z: f32 = 18.78;
const CRAWL_DIM_Z: f32 = 18.79;
const PANEL_Z: f32 = 18.95;
const TEXT_Z: f32 = 19.0;

/// Rebuilt-on-change UI (menu rows, slot cards, crawl lines).
#[derive(Component, Clone)]
pub struct TitleUi;

/// The session-long backdrop (black base, flyover rooms, darkening gradient).
#[derive(Component, Clone)]
pub struct TitleBackdrop;

#[derive(PartialEq, Eq, Clone, Copy, Default)]
enum View {
    #[default]
    Main,
    Slots,
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
enum SlotMode {
    #[default]
    Load,
    New,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum ArmKind {
    Delete,
    Overwrite,
}

#[derive(Resource, Default)]
pub struct TitleState {
    view: View,
    sel: usize,
    slot_sel: usize,
    slot_mode: SlotMode,
    armed: Option<(ArmKind, u32)>, // destructive picks need a second press
    idle: u32,
    crawling: bool,
    crawl_t: f32,
    crawl_h: f32,
}

/// The title's draw context — the shared [`crate::ui::Pen`] carrying the TitleUi marker.
pub(crate) type Pen<'a, 'w, 's> = crate::ui::Pen<'a, 'w, 's, TitleUi>;

/// The main-menu rows (js opts(): a slot picker only earns its place with 2+ saves; we're
/// always a native shell, so EXIT GAME is always offered).
#[derive(Clone, Copy, PartialEq)]
enum Opt {
    Continue,
    Load,
    New,
    Options,
    Exit,
}

fn opts(metas: &SlotMetas) -> Vec<Opt> {
    let saved = metas.0.iter().filter(|m| m.is_some()).count();
    let mut v = Vec::new();
    if saved > 0 {
        v.push(Opt::Continue);
        // LOAD earns its row with ANY save (it was 2+): it's also where a save
        // gets DELETED, and that path must always exist (Baz).
        v.push(Opt::Load);
    }
    v.extend([Opt::New, Opt::Options, Opt::Exit]);
    v
}

fn opt_label(o: Opt) -> &'static str {
    match o {
        Opt::Continue => "CONTINUE",
        Opt::Load => "LOAD GAME",
        Opt::New => "NEW GAME",
        Opt::Options => "OPTIONS",
        Opt::Exit => "EXIT GAME",
    }
}

const MENU_Y0: f32 = 116.0; // first row baseline
const MENU_ROW_H: f32 = 14.0;

/// The main-menu panel rect (px, py, pw, ph) for an option list — ONE geometry source for the
/// draw and the mouse hit-test, so row rects match what's on screen.
fn menu_geom(o: &[Opt]) -> (f32, f32, f32, f32) {
    let w = CANVAS_W as f32;
    let max_w = o.iter().map(|k| font::measure(opt_label(*k))).max().unwrap_or(0) as f32;
    let (pw, ph) = (max_w + 40.0, o.len() as f32 * MENU_ROW_H + 8.0);
    let px = ((w - pw) / 2.0).round();
    (px, MENU_Y0 - 9.0, pw, ph)
}

pub struct TitlePlugin;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TitleState>()
            .init_resource::<flyover::Flyover>()
            .add_message::<LoadSlot>()
            .add_systems(OnEnter(Screen::Title), enter_title)
            .add_systems(OnEnter(Screen::Play), cleanup_title)
            .add_systems(
                FixedUpdate,
                (
                    (flyover::flyover_tick, title_tick.before(super::play::EndTick), crawl_tick)
                        .run_if(in_state(Screen::Title)),
                    loader::handle_load_slot,
                    loader::handle_warp,
                ),
            );
    }
}

/// Entering the title: on a FRESH entry (boot / quit-to-title) reset the menu, rescan the
/// slot cards and raise the backdrop; coming back from OPTIONS keeps everything in place.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn enter_title(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut st: ResMut<TitleState>,
    mut metas: ResMut<SlotMetas>,
    fly: Res<flyover::Flyover>,
    bindings: Res<Bindings>,
    state: Res<ActionState>,
    ui: Query<Entity, With<TitleUi>>,
) {
    if !fly.active() {
        *st = TitleState::default();
        *metas = scan_metas();
        let (w, h) = (CANVAS_W as f32, CANVAS_H as f32);
        commands.spawn((
            Sprite::from_color(Color::BLACK, Vec2::new(w, h)),
            at(0.0, 0.0, w, h, BLACK_Z),
            PIXEL_LAYER,
            TitleBackdrop,
        ));
        // The js contrast gradient: rgba(4,8,12,0.55) -> rgba(2,5,8,0.8) top to bottom
        // (alphas bumped for Bevy's linear blending).
        let grad = images.add(gradient_image());
        commands.spawn((
            Sprite { image: grad, custom_size: Some(Vec2::new(w, h)), ..default() },
            at(0.0, 0.0, w, h, GRAD_Z),
            PIXEL_LAYER,
            TitleBackdrop,
        ));
    }
    redraw(&mut commands, &ui, &mut images, &st, &metas, &bindings, &state);
}

/// Every entity the title owns (UI layer + backdrop) — the cleanup sweep's filter.
type AnyTitleEntity = Or<(With<TitleUi>, With<TitleBackdrop>)>;

/// Leaving for play (a LoadSlot landed): drop every title entity + baked flyover image,
/// and swallow held buttons so the confirming press can't swing a sword (js startGuard).
/// (Pub for the WRIFT_SHOT scenes that jump straight from the title to another screen.)
pub fn cleanup_title(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fly: ResMut<flyover::Flyover>,
    mut input: ResMut<ActionState>,
    ui: Query<Entity, AnyTitleEntity>,
) {
    for e in &ui {
        commands.entity(e).despawn();
    }
    fly.clear_images(&mut images);
    for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4, Action::Interact, Action::MenuConfirm] {
        input.latch(a);
    }
}

/// The title's fixed-tick input driver (js Title.update).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn title_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    state: Res<ActionState>,
    bindings: Res<Bindings>,
    mut st: ResMut<TitleState>,
    mut metas: ResMut<SlotMetas>,
    ui: Query<Entity, With<TitleUi>>,
    mut next: ResMut<NextState<Screen>>,
    mut loads: MessageWriter<LoadSlot>,
    mut exit: MessageWriter<AppExit>,
    mut creator: ResMut<super::creator::CreatorState>,
    ptr: Res<crate::input::Pointer>,
) {
    // Mouse motion or a click counts as activity too, so the cursor exits attract mode and
    // keeps it from starting while the player is actively mousing.
    let any = ACTIONS.iter().any(|a| state.pressed(*a)) || ptr.click || ptr.moved;

    // Attract mode: any key returns to the menu, else keep (and loop) the story scroll.
    if st.crawling {
        if any {
            st.crawling = false;
            st.idle = 0;
            redraw(&mut commands, &ui, &mut images, &st, &metas, &bindings, &state);
        } else {
            st.crawl_t += 1.0;
            if st.crawl_h > 0.0 && st.crawl_t * crawl::CRAWL_SPEED > st.crawl_h {
                st.crawl_t = 0.0;
            }
        }
        return;
    }
    st.idle = if any { 0 } else { st.idle + 1 };
    if st.idle > crawl::IDLE_MAX && st.view == View::Main {
        st.crawling = true;
        for e in &ui {
            commands.entity(e).despawn();
        }
        let mut pen = Pen { commands: &mut commands, images: &mut images, marker: TitleUi };
        crawl::spawn(&mut pen, &mut st);
        return;
    }

    if st.view == View::Slots {
        match slots::tick(&mut st, &state, &mut metas, &ptr) {
            slots::SlotAct::Dirty => redraw(&mut commands, &ui, &mut images, &st, &metas, &bindings, &state),
            slots::SlotAct::Load(n) => {
                loads.write(LoadSlot { slot: n, fresh: false, seed: None });
            }
            slots::SlotAct::New(n) => {
                creator.slot = n;
                st.view = View::Main; // come back to a sane title if the creator backs out
                next.set(Screen::Creator);
            }
            slots::SlotAct::None => {}
        }
        return;
    }

    let o = opts(&metas);
    if st.sel >= o.len() {
        st.sel = o.len() - 1;
    }
    let mut dirty = false;
    if state.pressed(Action::Up) {
        st.sel = (st.sel + o.len() - 1) % o.len();
        dirty = true;
    }
    if state.pressed(Action::Down) {
        st.sel = (st.sel + 1) % o.len();
        dirty = true;
    }
    // Select = INTERACT or ENTER (js Input.confirm), with Slot1/Pause still accepted.
    let mut confirm = state.pressed(Action::Interact) || state.pressed(Action::MenuConfirm) || state.pressed(Action::Slot1) || state.pressed(Action::Pause);
    // Mouse: hover a row highlights it, a click selects it (same confirm path as the keys).
    let (px, _, pw, _) = menu_geom(&o);
    for i in 0..o.len() {
        if ptr.over(px, MENU_Y0 - 4.0 + i as f32 * MENU_ROW_H, pw, MENU_ROW_H) {
            if ptr.moved {
                st.sel = i;
                dirty = true;
            }
            if ptr.click {
                st.sel = i;
                confirm = true;
            }
        }
    }
    if confirm {
        match o[st.sel] {
            Opt::Continue => {
                loads.write(LoadSlot { slot: latest_slot(&metas), fresh: false, seed: None });
            }
            // First ever game — no picker needed (js: straight into slot 1).
            Opt::New if metas.0.iter().all(|m| m.is_none()) => {
                creator.slot = 1;
                next.set(Screen::Creator);
            }
            Opt::New | Opt::Load => {
                st.slot_mode = if o[st.sel] == Opt::Load { SlotMode::Load } else { SlotMode::New };
                st.view = View::Slots;
                st.slot_sel = 0;
                st.armed = None;
                dirty = true;
            }
            Opt::Options => {
                next.set(Screen::TitleOptions);
            }
            Opt::Exit => {
                exit.write(AppExit::Success);
            }
        }
    }
    if dirty {
        redraw(&mut commands, &ui, &mut images, &st, &metas, &bindings, &state);
    }
}

/// Move the crawl lines while the story runs (split out: scroll wants its own query).
fn crawl_tick(st: Res<TitleState>, lines: Query<(&crawl::CrawlLine, &mut Transform, &mut Sprite, &mut Visibility)>) {
    if st.crawling {
        crawl::scroll(&st, lines);
    }
}

/// Rebuild the title's UI layer (js Title.draw minus the backdrop, which is retained).
fn redraw(
    commands: &mut Commands,
    old: &Query<Entity, With<TitleUi>>,
    images: &mut Assets<Image>,
    st: &TitleState,
    metas: &SlotMetas,
    bindings: &Bindings,
    state: &ActionState,
) {
    for e in old {
        commands.entity(e).despawn();
    }
    let mut pen = Pen { commands, images, marker: TitleUi };
    let (w, h) = (CANVAS_W as f32, CANVAS_H as f32);
    let cx = w / 2.0;

    // The wordmark, scale 2 with a 1px drop shadow (js draws shadow then gold offset -1,-1).
    let title = "WRIFTHEART";
    let tw = font::measure(title) as f32 * 2.0;
    let tx = ((w - tw) / 2.0).round();
    pen.text_scaled(title, tx, 54.0, 0x0a0a0a, TEXT_Z - 0.01, 2.0);
    pen.text_scaled(title, tx - 1.0, 53.0, 0xfce0a8, TEXT_Z, 2.0);

    if st.view == View::Slots {
        slots::draw(&mut pen, st, metas, bindings, state.pad_present);
        return;
    }

    // Menu options on a soft rounded panel (js roundRect rgba(9,12,16,0.62)).
    let o = opts(metas);
    let (px, py, pw, ph) = menu_geom(&o);
    let panel = pen.images.add(rounded_panel(pw as u32, ph as u32));
    pen.commands.spawn((
        Sprite::from_image(panel),
        at(px, py, pw, ph, PANEL_Z),
        PIXEL_LAYER,
        TitleUi,
    ));
    for (i, k) in o.iter().enumerate() {
        let lab = opt_label(*k);
        let on = i == st.sel;
        let lx = ((w - font::measure(lab) as f32) / 2.0).round();
        let y = MENU_Y0 + i as f32 * MENU_ROW_H;
        if on {
            pen.text(">", lx - 9.0, y, 0xfce0a8, TEXT_Z);
        }
        pen.text(lab, lx, y, if on { 0xfcfcfc } else { 0x9a9a9a }, TEXT_Z);
    }

    let help = if state.pad_present {
        "D-PAD  +  A / START".to_string()
    } else {
        // The js line, LIVE: the interact binding selects (E was the stale js bug too),
        // and ENTER always works (Action::MenuConfirm).
        format!("ARROWS  +  {} / ENTER", bindings.prompt(Action::Interact, false))
    };
    pen.text_center(&help, cx, h - 16.0, 0x5a6a5a, TEXT_Z);
    let ver = concat!("V", env!("CARGO_PKG_VERSION"));
    pen.text_right(ver, w - 6.0, h - 12.0, 0x5a6a5a, TEXT_Z);
}

/// The js linear contrast gradient, baked as a 1px-wide column the sprite stretches.
fn gradient_image() -> Image {
    let h = CANVAS_H as usize;
    let mut buf = Vec::with_capacity(h * 4);
    for y in 0..h {
        let t = y as f32 / (h - 1) as f32;
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        buf.extend([
            lerp(4.0, 2.0) as u8,
            lerp(8.0, 5.0) as u8,
            lerp(12.0, 8.0) as u8,
            (lerp(0.65, 0.88) * 255.0) as u8, // js 0.55 -> 0.8, bumped for linear blending
        ]);
    }
    Image::new(
        Extent3d { width: 1, height: h as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        buf,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// The js roundRect menu panel (radius 6, soft fill + faint border), CPU-baked.
fn rounded_panel(w: u32, h: u32) -> Image {
    const R: f32 = 6.0;
    let mut buf = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            // Distance outside the rounded-corner radius decides in/out; the 1px rim is
            // the border (js rgba(150,165,185,0.14), fill rgba(9,12,16,0.62) — bumped).
            let dx = (R - 1.0 - x as f32).max(x as f32 - (w as f32 - R)).max(0.0);
            let dy = (R - 1.0 - y as f32).max(y as f32 - (h as f32 - R)).max(0.0);
            let d = (dx * dx + dy * dy).sqrt();
            if d > R {
                continue;
            }
            let i = ((y * w + x) * 4) as usize;
            let edge = x == 0 || y == 0 || x == w - 1 || y == h - 1 || d > R - 1.5;
            if edge {
                buf[i..i + 4].copy_from_slice(&[150, 165, 185, 56]);
            } else {
                buf[i..i + 4].copy_from_slice(&[9, 12, 16, 184]);
            }
        }
    }
    Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        buf,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
