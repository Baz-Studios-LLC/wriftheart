//! slideout.rs — the game's MAIN MENU (port of js/inventory.js): a panel that SLIDES in
//! from the right over the play area (never the sidebar), freezes the world, and hosts a
//! TAB REGISTRY — new gameplay systems (gear, craft, skills) become one more row here,
//! exactly like the codex's reference tabs. Open/close with the Inventory button (I / RB);
//! Q/R (LT/RT) switch tabs.
//!
//! Tabs are the JS set 1-for-1 — CHAR (the full carry model over the real inventory),
//! CRAFT (hand recipes), SKILLS (the passive constellation), STATUS (its true empty state
//! until effects exist). Slide speed = the JS 0.2/frame.

mod char_tab;
pub mod craft_tab;
pub mod skills_tab;

pub use skills_tab::{TreeAlloc, TreeStats};

use super::play::{EndTick, HeroArt, Player};
use super::screen::Screen;
use crate::combat::Health;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::inventory::PlayerInv;
use crate::room::PX_W;
use crate::ui::label;
use crate::{CANVAS_H, SIDEBAR_W};
use bevy::prelude::*;

pub(super) const PANEL_W: f32 = PX_W as f32; // fills the play area (js PANEL_W = 304)
const SLIDE: f32 = 0.2; // slide progress per tick (js SLIDE)
pub(super) const PAD: f32 = 8.0;
pub(super) const Z: f32 = 16.0; // above the HUD band (13-14), below the codex (17.8+)

/// The slide-out's tab registry — the JS TABS rows, same keys, same order.
const TABS: &[&str] = &["CHAR", "CRAFT", "SKILLS", "STATUS"];

#[derive(Resource, Default)]
pub struct SlideOut {
    pub tab: usize,     // persists across opens (js activeTab)
    pub anim: f32,      // 0..1 visual slide progress
    pub applied: f32,   // x-offset currently applied to the spawned entities
    pub gear_cursor: usize, // unified CHAR-page cell cursor (js: gearCursor)
    pub held: Option<usize>, // CHAR: cell index picked up to move (js: held), carry-and-place
    pub hold_act: Option<char_tab::HoldAct>, // CHAR: a drop/trash hold in flight (js: holdAct)
    pub dirty: bool,    // a tab's input system changed state -> redraw this tick
}

/// Marker on every slide-out entity (widgets place absolutely; the slide system shifts
/// them all by the animation delta each frame).
#[derive(Component, Clone)]
pub struct SlideOutUi;

/// The world-dim behind the panel (js: rgba(0,0,0,0.45*ease) over the whole canvas). It
/// does NOT slide with the panel; its alpha follows the ease instead.
#[derive(Component)]
pub struct DimLayer;

pub struct SlideOutPlugin;

impl Plugin for SlideOutPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SlideOut>()
            .init_resource::<TreeAlloc>()
            .init_resource::<TreeStats>()
            .init_resource::<skills_tab::SkillsState>()
            .init_resource::<craft_tab::CraftState>()
            .add_systems(Startup, |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
                commands.insert_resource(skills_tab::SkillArt::build(&mut images));
            })
            .add_systems(
                FixedUpdate,
                (
                    skills_tab::gear_refresh,
                    skills_tab::skills_input.run_if(skills_tab::active),
                    slideout_tick.after(super::menu::menu_tick),
                )
                    .chain()
                    .before(EndTick),
            )
            .add_systems(Update, (slide_anim, skills_tab::skills_anim.run_if(skills_tab::active)))
            .add_systems(OnExit(Screen::SlideOut), close_slideout);
    }
}

/// The tick's resource bundle (Bevy caps systems at 16 params — see play.rs RoomCtx).
#[derive(bevy::ecs::system::SystemParam)]
struct SlideCtx<'w> {
    inv: ResMut<'w, PlayerInv>,
    craft: ResMut<'w, craft_tab::CraftState>,
    stats: ResMut<'w, super::stats::Stats>,
    rng: ResMut<'w, super::battle::GameRng>,
    hero: Res<'w, HeroArt>,
    skill_art: Res<'w, skills_tab::SkillArt>,
    skills: Res<'w, skills_tab::SkillsState>,
    alloc: Res<'w, TreeAlloc>,
    learned: Res<'w, super::blueprints::LearnedBlueprints>,
    stash: ResMut<'w, super::storage::PlayerStash>,
    inside: Res<'w, super::interior::Inside>,
}

/// Open from play, close from inside, switch tabs — on the fixed clock like every menu.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn slideout_tick(
    mut commands: Commands,
    mut state: ResMut<ActionState>,
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    screen: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
    mut so: ResMut<SlideOut>,
    mut sc: SlideCtx,
    mut players: Query<(&Player, &mut Health)>,
    old: Query<Entity, With<SlideOutUi>>,
    mut images: ResMut<Assets<Image>>,
) {
    let SlideCtx { ref mut inv, ref mut craft, ref mut stats, ref mut rng, ref hero, ref skill_art, ref skills, ref alloc, ref learned, ref mut stash, ref inside } = sc;
    // At home (inside your built house), crafting also draws from the storage chest.
    let home = inside.0.as_ref().is_some_and(|st| st.def.kind == "house");
    let Ok((player, mut health)) = players.single_mut() else { return };
    match screen.get() {
        Screen::Play => {
            // EVERY opener jumps to ITS page — Inventory (I) to CHAR, SkillTree (K) to
            // SKILLS, and each page's quick-access action (unbound by default, all in
            // CONTROLS). (Inventory used to reopen the LAST tab — pressing I after
            // browsing skills landed you back on SKILLS; Baz: it should be the bag.)
            let jump = [
                (Action::Inventory, "CHAR"),
                (Action::SkillTree, "SKILLS"),
                (Action::Craft, "CRAFT"),
                (Action::StatusTab, "STATUS"),
            ]
            .into_iter()
            .find(|(a, _)| state.pressed(*a));
            if let Some((_, page)) = jump {
                so.tab = TABS.iter().position(|t| *t == page).unwrap_or(0);
                if TABS[so.tab] == "CHAR" {
                    so.gear_cursor = char_tab::home_cell(); // js charEntry()
                }
                so.held = None;
                so.hold_act = None;
                so.anim = 0.0;
                so.applied = 0.0;
                next.set(Screen::SlideOut);
                let ctx = RedrawCtx { inv, hero, skill_art, skills, alloc, learned: &learned.0, stash, home };
                redraw(&mut commands, &old, &so, &bindings, &state, &ctx, player, &health, craft, &mut images);
            }
        }
        Screen::SlideOut => {
            // Global BACK/close — but mid-move (carrying an item), B just cancels the carry.
            if state.pressed(Action::Slot2) && so.held.is_some() {
                so.held = None;
                so.dirty = true;
            } else if state.pressed(Action::Slot2) || state.pressed(Action::Pause) {
                next.set(Screen::Play); // OnExit latches + clears
                return;
            } else if let Some((_, page)) = [
                // An opener key while OPEN is a TOGGLE (Baz: "K only opens the skills
                // tree — it should close it as well"): its own page closes the drawer,
                // another page jumps straight to it.
                (Action::Inventory, "CHAR"),
                (Action::SkillTree, "SKILLS"),
                (Action::Craft, "CRAFT"),
                (Action::StatusTab, "STATUS"),
            ]
            .into_iter()
            .find(|(a, _)| state.pressed(*a))
            {
                let idx = TABS.iter().position(|t| *t == page).unwrap_or(0);
                if idx == so.tab {
                    next.set(Screen::Play); // already there — close (the I-toggle feel)
                    return;
                }
                so.tab = idx;
                if TABS[idx] == "CHAR" {
                    so.gear_cursor = char_tab::home_cell();
                }
                so.held = None; // a carry can't survive its page going away
                so.hold_act = None;
                so.dirty = true;
            }
            // The menu owns the face buttons while open (the heldLatch rule).
            for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
                state.latch(a);
            }
            let mut dirty = false;
            let n = TABS.len();
            let mut step = 0;
            if state.pressed(Action::TabNext) {
                step += 1;
            }
            if state.pressed(Action::TabPrev) {
                step += n - 1;
            }
            if step % n != 0 {
                so.tab = (so.tab + step) % n;
                so.hold_act = None; // a hold can't survive its page going away
                dirty = true;
            }
            // The CHAR page's unified cursor walks gear + trinkets + ability slots + bag
            // (the js gearCursor). Entering the tab parks it on the first bag slot.
            if TABS[so.tab] == "CHAR" {
                if dirty {
                    so.gear_cursor = char_tab::home_cell(); // js charEntry()
                }
                if char_tab::nav(&mut so, &state, inv) {
                    dirty = true;
                }
                // A/X/Y/T/H — the carry model (SHIFT = the keyboard's instant-stack mod).
                let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
                if char_tab::actions(&mut so, &state, shift, inv, &mut commands, &mut images, player, &mut health) {
                    dirty = true;
                }
            }
            if TABS[so.tab] == "CRAFT" {
                let mut roll = || rng.0.next_f64();
                if craft_tab::actions(&state, inv, stash, home, stats, alloc, &mut roll, craft, &learned.0) {
                    dirty = true;
                }
            }
            if dirty || so.dirty {
                so.dirty = false;
                let ctx = RedrawCtx { inv, hero, skill_art, skills, alloc, learned: &learned.0, stash, home };
                redraw(&mut commands, &old, &so, &bindings, &state, &ctx, player, &health, craft, &mut images);
            }
        }
        _ => {}
    }
}

/// The per-tab data redraw needs, bundled so the call sites stay readable.
struct RedrawCtx<'a> {
    inv: &'a PlayerInv,
    hero: &'a HeroArt,
    skill_art: &'a skills_tab::SkillArt,
    skills: &'a skills_tab::SkillsState,
    alloc: &'a TreeAlloc,
    learned: &'a std::collections::HashSet<String>,
    stash: &'a super::storage::PlayerStash,
    home: bool,
}

/// Ease the panel in from the right with the JS smoothstep (anim^2 * (3 - 2*anim)); every
/// slide-out entity shifts by the animation delta (widgets spawn at their FINAL positions).
/// The world-dim behind stays put and fades in with the ease instead.
fn slide_anim(
    screen: Res<State<Screen>>,
    mut so: ResMut<SlideOut>,
    mut q: Query<&mut Transform, (With<SlideOutUi>, Without<DimLayer>)>,
    mut dim: Query<&mut Sprite, With<DimLayer>>,
) {
    if *screen.get() != Screen::SlideOut {
        return;
    }
    so.anim = (so.anim + SLIDE).min(1.0);
    let ease = so.anim * so.anim * (3.0 - 2.0 * so.anim);
    if let Ok(mut sprite) = dim.single_mut() {
        // js 0.45; darkening needs MORE alpha under linear blending to read the same.
        sprite.color = sprite.color.with_alpha(0.55 * ease);
    }
    let target = (1.0 - ease) * PANEL_W;
    let delta = target - so.applied;
    if delta == 0.0 {
        return;
    }
    so.applied = target;
    for mut tf in &mut q {
        tf.translation.x += delta;
    }
}

/// Leaving: clear the panel and swallow held face buttons.
fn close_slideout(
    mut commands: Commands,
    mut state: ResMut<ActionState>,
    mut craft: ResMut<craft_tab::CraftState>,
    ui: Query<Entity, With<SlideOutUi>>,
) {
    craft.station = None; // walking away from a station ends its session
    for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
        state.latch(a);
    }
    for e in &ui {
        commands.entity(e).despawn();
    }
}

/// (Re)build the panel at its FINAL position; the anim system shifts it into place.
#[allow(clippy::too_many_arguments)] // it IS the redraw's arity
fn redraw(
    commands: &mut Commands,
    old: &Query<Entity, With<SlideOutUi>>,
    so: &SlideOut,
    bindings: &Bindings,
    state: &ActionState,
    ctx: &RedrawCtx,
    _player: &Player,
    health: &Health,
    craft: &mut craft_tab::CraftState,
    images: &mut Assets<Image>,
) {
    for e in old {
        commands.entity(e).despawn();
    }
    let x0 = SIDEBAR_W;
    let h = CANVAS_H as f32;
    // Dim the frozen world behind the panel — whole canvas, sidebar included (js).
    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::new(crate::CANVAS_W as f32, h)),
        at(0.0, 0.0, crate::CANVAS_W as f32, h, 18.5), // above the HUD; alpha rides the ease
        PIXEL_LAYER,
        SlideOutUi,
        DimLayer,
    ));
    // Panel body + left divider — NO border box (js: #0c0c10 fill, 1px #2a2a30 edge).
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x0c, 0x0c, 0x10), Vec2::new(PANEL_W, h)),
        at(x0, 0.0, PANEL_W, h, Z),
        PIXEL_LAYER,
        SlideOutUi,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x2a, 0x2a, 0x30), Vec2::new(1.0, h)),
        at(x0, 0.0, 1.0, h, Z + 0.05),
        PIXEL_LAYER,
        SlideOutUi,
    ));
    // Tab strip (same look as the codex: lit bg + gold rule on the active tab).
    let mut tx = x0 + 6.0;
    for (i, title) in TABS.iter().enumerate() {
        let on = i == so.tab;
        let tw = font::measure(title) as f32 + 8.0;
        let bg = if on { Color::srgb_u8(0x2a, 0x2a, 0x18) } else { Color::srgb_u8(0x14, 0x14, 0x18) };
        commands.spawn((Sprite::from_color(bg, Vec2::new(tw, 11.0)), at(tx, 4.0, tw, 11.0, Z + 1.0), PIXEL_LAYER, SlideOutUi));
        if on {
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0xff, 0xd3, 0x4d), Vec2::new(tw, 1.0)),
                at(tx, 4.0, tw, 1.0, Z + 1.1),
                PIXEL_LAYER,
                SlideOutUi,
            ));
        }
        label(commands, images, title, tx + 4.0, 6.0, if on { 0xfcfcfc } else { 0x6c6c74 }, Z + 1.1, SlideOutUi);
        tx += tw + 2.0;
    }
    // Content area below the tab bar.
    let cy = 22.0;
    match TABS[so.tab] {
        "CHAR" => {
            char_tab::draw(commands, images, so, bindings, state.pad_present, ctx.inv, ctx.hero, ctx.alloc, health);
        }
        "SKILLS" => {
            skills_tab::draw(commands, images, ctx.skill_art, ctx.skills, ctx.alloc, bindings, state.pad_present);
        }
        "CRAFT" => {
            craft_tab::draw(commands, images, craft, ctx.inv, bindings, state.pad_present, so, ctx.learned, ctx.stash, ctx.home);
        }
        _ => {
            // STATUS: the true empty state — no effect system means no active effects.
            label(commands, images, "STATUS EFFECTS", x0 + PAD, cy + 4.0, 0xfcd000, Z + 1.0, SlideOutUi);
            label(commands, images, "NO ACTIVE EFFECTS", x0 + PAD, cy + 18.0, 0x9aa0aa, Z + 1.0, SlideOutUi);
        }
    }
    // Footer hint — derived prompts only (the CHAR and SKILLS pages draw their own
    // bottom bands).
    if TABS[so.tab] == "SKILLS" || TABS[so.tab] == "CHAR" || TABS[so.tab] == "CRAFT" {
        return;
    }
    let hint = format!(
        "{}/{} TABS - {} CLOSE",
        bindings.prompt(Action::TabPrev, state.pad_present),
        bindings.prompt(Action::TabNext, state.pad_present),
        bindings.prompt(Action::Inventory, state.pad_present)
    );
    label(commands, images, &hint, x0 + PAD, h - 12.0, 0x606060, Z + 1.0, SlideOutUi);
}
