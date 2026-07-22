//! hud.rs — the left sidebar (port of the drawSidebar layout in js/game.js): name + level,
//! HP/MP/XP bars, and the four ability slots with their bound-button labels DERIVED from the
//! live bindings (they flip to pad glyphs the moment a controller connects).
//!
//! MP/XP show static placeholders until mana + levelling port; the compass minimap is TODO.

use super::play::Player;
use crate::combat::Health;
use crate::gfx::{at, bake, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::inventory::PlayerInv;
use crate::ui::{bar, cell, label, set_bar, BarSpec};
use crate::{CANVAS_H, SIDEBAR_W};
use bevy::prelude::*;

// The sidebar draws ABOVE the entire play-field stack (tiles 1, props 3.x, actors 4-8,
// FX 12): the JS painted the sidebar last every frame, so nothing sliding between rooms —
// tree canopies included — may ever cross over it.
use super::hud_widgets::InWidget;

pub(crate) const HUD_Z: f32 = 17.2; // above the slide-out (16.x) so its tree can never cross the sidebar

pub(crate) const PAD: f32 = 8.0;
const INNER_W: f32 = SIDEBAR_W - PAD * 2.0; // 64
const BAR_W: f32 = INNER_W - 13.0; // trough width right of the 2-char label
const SLOT: f32 = 13.0;
const HP_H: f32 = 9.0;

// The sidebar layout, as NAMED rows chained off each other — the single source of truth.
// Every system that places or replaces a sidebar widget reads these; nobody re-derives a
// y-position by hand (re-deriving is how the JS sidebar drifted apart).
const NAME_Y: f32 = PAD;
const HP_Y: f32 = NAME_Y + 8.0;
const MP_Y: f32 = HP_Y + HP_H + 1.0;
const XP_Y: f32 = MP_Y + 10.0;
const ITEMS_HDR_Y: f32 = XP_Y + 8.0 + 5.0;
const SLOTS_Y: f32 = ITEMS_HDR_Y + 8.0;
const PROMPTS_Y: f32 = SLOTS_Y + SLOT + 2.0;
const TIME_HDR_Y: f32 = PROMPTS_Y + 8.0 + 5.0; // same section rhythm as ITEMS
const TIME_ROW_Y: f32 = TIME_HDR_Y + 8.0;
const QUESTS_HDR_Y: f32 = TIME_ROW_Y + 8.0 + 5.0;

// THE SIDEBAR WIDGET SYSTEM lives in hud_widgets.rs (Baz: a true system —
// registry, saved arrangement, top/bottom pinning, the ESC arranger). Every
// section here spawns at its fixed BASELINE tagged InWidget("id"); the engine
// moves whole widgets and hides the off/unavailable ones.

/// Content systems that spawn/refresh sidebar entities — the widget engine's
/// adopt/place pass runs after this set so fresh spawns land placed, same frame.
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HudContent;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        use super::hud_widgets as hw;
        app.add_systems(Startup, setup_hud)
            .init_resource::<hw::HudConfig>()
            .init_resource::<hw::WidgetLayout>()
            .add_systems(
                Update,
                (
                    (hud_hp, hud_prompts, hud_slots, hud_progress, hud_name, hud_time, hud_quests, hud_mana, hw::shards_tick, hw::coins_tick).in_set(HudContent),
                    hw::layout_tick,
                    (hw::adopt, hw::place).chain().after(HudContent).after(hw::layout_tick),
                ),
            );
    }
}

#[derive(Component)]
struct HpFill;
#[derive(Component)]
struct HpValue;
#[derive(Component)]
struct SlotPromptLabel; // the A/B/X/Y row — re-baked when a pad connects/disconnects
#[derive(Component)]
struct HintLabel; // the bottom control hint — same
#[derive(Component, Clone)]
struct SlotCell; // the four ability-slot cells — re-baked when the inventory changes
#[derive(Component)]
struct XpFill; // the XP bar fill — follows Progress
#[derive(Component, Clone)]
struct MpFill; // the MP bar fill — follows Mana (flute.rs)
#[derive(Component)]
struct LvlLabel; // the "LVL n" plate — re-baked on level-up

fn setup_hud(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    bindings: Res<Bindings>,
) {
    // Panel + divider against the play field.
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x0c, 0x0c, 0x0c), Vec2::new(SIDEBAR_W, CANVAS_H as f32)),
        at(0.0, 0.0, SIDEBAR_W, CANVAS_H as f32, HUD_Z),
        PIXEL_LAYER,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x2a, 0x2a, 0x2a), Vec2::new(1.0, CANVAS_H as f32)),
        at(SIDEBAR_W - 1.0, 0.0, 1.0, CANVAS_H as f32, HUD_Z + 0.05),
        PIXEL_LAYER,
    ));

    // ===== Vitals: NAME + LVL, then HP / MP / XP — every row a named layout const =====
    // The name IS the character widget's title — same style as every section title.
    // (DELIBERATE DEVIATION: the JS drew the name grey while ITEMS went through its white
    // hdr() helper; Baz wants one title format, so both route through section_title.)
    // Live via hud_name — the creator/loader put the real hero name in HeroIdent.
    label(&mut commands, &mut images, "HERO", PAD, NAME_Y, 0xfcfcfc, HUD_Z + 1.0, (NameLabel, InWidget("vitals")));
    // LVL, right-aligned: secondary info, dimmer than the title (live via hud_progress).
    let lvl = "LVL 1";
    let w = crate::gfx::font::measure(lvl) as f32;
    label(&mut commands, &mut images, lvl, PAD + INNER_W - w, NAME_Y, 0xdcdce0, HUD_Z + 1.0, (LvlLabel, InWidget("vitals")));
    bar(
        &mut commands,
        &mut images,
        &BarSpec { label: "HP", x: PAD, y: HP_Y, w: BAR_W, h: HP_H, fill: 0x00a800, border: 0x787878, z: HUD_Z + 0.1 },
        1.0,
        HpFill,
        InWidget("vitals"),
    );
    label(&mut commands, &mut images, "3/3", PAD + 13.0 + 20.0, HP_Y + 2.0, 0xfcfcfc, HUD_Z + 1.5, (HpValue, InWidget("vitals")));
    // MP: live via hud_mana (songs are mana's first consumer). XP: live via hud_progress.
    bar(&mut commands, &mut images, &BarSpec { label: "MP", x: PAD, y: MP_Y, w: BAR_W, h: 9.0, fill: 0x3868e8, border: 0x5a6a9a, z: HUD_Z + 0.1 }, 1.0, MpFill, InWidget("vitals"));
    bar(&mut commands, &mut images, &BarSpec { label: "XP", x: PAD, y: XP_Y, w: BAR_W, h: 7.0, fill: 0x5cc0fc, border: 0x5a6a9a, z: HUD_Z + 0.1 }, 0.0, XpFill, InWidget("vitals"));

    // ===== Items: the four ability slots (cells live-drawn by hud_slots) =====
    section_title(&mut commands, &mut images, "ITEMS", ITEMS_HDR_Y, InWidget("abilities"));
    spawn_slot_prompts(&mut commands, &mut images, &bindings, false, PROMPTS_Y);

    // Bottom control hint (also derived; re-baked on pad connect).
    spawn_hint(&mut commands, &mut images, &bindings, false);
}

/// ONE style for every sidebar widget title (the name plate, ITEMS, and every section to
/// come) — the port of the JS `hdr()` helper. If a title ever changes look, it changes here.
fn section_title(commands: &mut Commands, images: &mut Assets<Image>, text: &str, y: f32, tag: impl Bundle) {
    label(commands, images, text, PAD, y, 0xfcfcfc, HUD_Z + 1.0, tag);
}

/// The hero's name plate (the section title that isn't static text).
#[derive(Component)]
struct NameLabel;

/// Re-bake the name plate whenever the hero changes (a load, a fresh creator hero).
fn hud_name(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    ident: Res<crate::app::identity::HeroIdent>,
    old: Query<Entity, With<NameLabel>>,
) {
    if !ident.is_changed() {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let name = ident.name.to_uppercase();
    label(&mut commands, &mut images, &name, PAD, NAME_Y, 0xfcfcfc, HUD_Z + 1.0, (NameLabel, InWidget("vitals")));
}

/// The A/B/X/Y (or E/X/C/V) labels under the slots — one bake per slot, centred.
fn spawn_slot_prompts(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    bindings: &Bindings,
    pad: bool,
    y: f32,
) {
    let gap = ((INNER_W - 4.0 * SLOT) / 3.0).floor();
    for (i, a) in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4].into_iter().enumerate() {
        let x = PAD + i as f32 * (SLOT + gap);
        let text = bindings.prompt(a, pad);
        let w = crate::gfx::font::measure(text) as f32;
        label(commands, images, text, x + ((SLOT - w) / 2.0).round(), y, 0x8a8a92, HUD_Z + 1.0, (SlotPromptLabel, InWidget("abilities")));
    }
}

fn spawn_hint(commands: &mut Commands, images: &mut Assets<Image>, bindings: &Bindings, pad: bool) {
    let hint = format!(
        "{} MOVE {} ATTACK",
        bindings.prompt(Action::Up, pad),
        bindings.prompt(Action::Slot1, pad)
    );
    label(commands, images, &hint, PAD, CANVAS_H as f32 - 12.0, 0x606060, HUD_Z + 1.0, (HintLabel, InWidget("hint")));
}

/// The four ability slots, live from the inventory (js drawSidebar's ITEMS row): each
/// cell's border tints to its item's rarity — dimmed to half while that slot recharges —
/// the item's icon sits centred, and a stack shows its count bottom-right.
fn hud_slots(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    inv: Res<PlayerInv>,
    players: Query<&Player>,
    old: Query<Entity, With<SlotCell>>,
    mut last_cd: Local<Option<[bool; 4]>>,
) {
    let cooling = players
        .single()
        .map(|p| [p.cooldowns[0] > 0, p.cooldowns[1] > 0, p.cooldowns[2] > 0, p.cooldowns[3] > 0])
        .unwrap_or([false; 4]);
    if !inv.is_changed() && *last_cd == Some(cooling) {
        return;
    }
    *last_cd = Some(cooling);
    for e in &old {
        commands.entity(e).despawn();
    }
    let gap = ((INNER_W - 4.0 * SLOT) / 3.0).floor();
    let darken = |c: u32| (c >> 1) & 0x7f7f7f; // js darken(rar, 0.5)
    for (i, cooling) in cooling.into_iter().enumerate() {
        let x = PAD + i as f32 * (SLOT + gap);
        let entry = inv.slots[i].and_then(|uid| inv.entry(uid));
        let def = entry.and_then(|e| crate::items::get(e.id));
        // Border: the item's rarity tint (empty = neutral gray), dim while recharging.
        let mut border = def.map_or(0x5a5a5a, |d| d.rarity.color());
        if cooling && def.is_some() {
            border = darken(border);
        }
        let icon = def.map(|d| (images.add(bake(d.icon, d.icon_pal)), 8.0));
        cell(&mut commands, x, SLOTS_Y, SLOT, None, border, icon, HUD_Z + 0.4, (SlotCell, InWidget("abilities")));
        if let Some(e) = entry
            && e.qty > 1
        {
            let q = format!("{}", e.qty);
            let qw = font::measure(&q) as f32;
            label(&mut commands, &mut images, &q, x + SLOT - 1.0 - qw, SLOTS_Y + SLOT - 6.0, 0xfcfcfc, HUD_Z + 0.6, (SlotCell, InWidget("abilities")));
        }
    }
}

/// XP bar + LVL plate, refreshed as XP flows in (js: bar('XP', xp/xpToNext) + the LVL text).
fn hud_progress(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    progress: Res<super::rewards::Progress>,
    mut fill: Query<&mut Sprite, With<XpFill>>,
    old_lvl: Query<Entity, With<LvlLabel>>,
    mut last: Local<(i32, i32)>,
) {
    if !progress.is_changed() || *last == (progress.level, progress.xp) {
        return;
    }
    *last = (progress.level, progress.xp);
    if let Ok(mut s) = fill.single_mut() {
        let frac = progress.xp as f32 / super::rewards::xp_for_level(progress.level).max(1) as f32;
        set_bar(&mut s, BAR_W, 7.0, frac.clamp(0.0, 1.0));
    }
    for e in &old_lvl {
        commands.entity(e).despawn();
    }
    let lvl = format!("LVL {}", progress.level);
    let w = crate::gfx::font::measure(&lvl) as f32;
    label(&mut commands, &mut images, &lvl, PAD + INNER_W - w, NAME_Y, 0xdcdce0, HUD_Z + 1.0, (LvlLabel, InWidget("vitals")));
}

/// The TIME section (js: "only with a clock trinket" — the Pocket Watch): a sun/moon
/// pip and HH:MM below the ability slots. Frame 0 boots at NOON, so 12:00 + tod * 24h.
#[derive(Component)]
struct TimeWidget; // header + pip + readout — rebaked when the minute (or the flag) flips

fn hud_time(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    inv: Res<PlayerInv>,
    clock: Res<super::room_render::FrameClock>,
    old: Query<Entity, With<TimeWidget>>,
    mut last: Local<Option<(bool, u32, u32, bool)>>,
) {
    let on = inv.has_gear_flag("clock");
    let day_len = super::gather::DAY_LEN;
    let tod = clock.0.rem_euclid(day_len) as f64 / day_len as f64; // 0 = noon
    let h24 = (12.0 + tod * 24.0) % 24.0;
    let (hh, mm) = (h24 as u32, ((h24 - h24.floor()) * 60.0) as u32);
    let night = super::lighting::ambient_alpha(clock.0) > 0.5;
    if *last == Some((on, hh, mm, night)) {
        return;
    }
    *last = Some((on, hh, mm, night));
    for e in &old {
        commands.entity(e).despawn();
    }
    if !on {
        return;
    }
    label(&mut commands, &mut images, "TIME", PAD, TIME_HDR_Y, 0xfcfcfc, HUD_Z + 1.0, (TimeWidget, InWidget("clock")));
    // The pip: a 7px sun (gold) that turns moon-blue once the dark is winning (js colours).
    let pip = if night { Color::srgb_u8(0x9a, 0xb0, 0xe0) } else { Color::srgb_u8(0xfc, 0xd2, 0x3b) };
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x20, 0x20, 0x20), Vec2::new(9.0, 9.0)),
        at(PAD - 1.0, TIME_ROW_Y - 1.0, 9.0, 9.0, HUD_Z + 0.4),
        PIXEL_LAYER,
        TimeWidget,
        InWidget("clock"),
    ));
    commands.spawn((
        Sprite::from_color(pip, Vec2::new(7.0, 7.0)),
        at(PAD, TIME_ROW_Y, 7.0, 7.0, HUD_Z + 0.5),
        PIXEL_LAYER,
        TimeWidget,
        InWidget("clock"),
    ));
    let t = format!("{hh:02}:{mm:02}");
    label(&mut commands, &mut images, &t, PAD + 11.0, TIME_ROW_Y + 1.0, 0xfcfcfc, HUD_Z + 1.0, (TimeWidget, InWidget("clock")));
}

/// HP bar + value, refreshed when the player's health changes.
/// The sidebar QUESTS list (js drawSidebar's quest block): one row per active quest —
/// a status bullet (green = ready to turn in) + a clipped name, counts kept visible.
#[derive(Component)]
struct QuestHud;

/// The MP value text, sibling of HpValue.
#[derive(Component)]
struct MpValue;

/// The MP bar tracks the mana pool (js drawSidebar's mana trough) — bar, flash,
/// and the centred value text (HP's read, same recipe).
fn hud_mana(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fills: Query<&mut Sprite, With<MpFill>>,
    mana: Res<super::flute::Mana>,
    old_value: Query<Entity, With<MpValue>>,
    mut last: Local<(i32, i32)>,
) {
    if !mana.is_changed() {
        return;
    }
    let frac = if mana.max > 0 { (mana.cur as f32 / mana.max as f32).clamp(0.0, 1.0) } else { 0.0 };
    // A fizzled cast flashes the bar red (js manaFlash).
    let flash = mana.flash > 0 && (mana.flash / 2) % 2 == 0;
    for mut f in &mut fills {
        f.color = if flash { Color::srgb_u8(0xd8, 0x30, 0x28) } else { Color::srgb_u8(0x48, 0x6c, 0xd8) };
    }
    if let Ok(mut s) = fills.single_mut() {
        set_bar(&mut s, BAR_W, 9.0, frac);
    }
    if *last == (mana.cur, mana.max) {
        return; // a flash tick, not a pool change — keep the baked text
    }
    *last = (mana.cur, mana.max);
    for e in &old_value {
        commands.entity(e).despawn();
    }
    if mana.max > 0 {
        let text = format!("{}/{}", mana.cur.max(0), mana.max);
        let tw = font::measure(&text) as f32;
        label(&mut commands, &mut images, &text, PAD + 13.0 + ((BAR_W - tw) / 2.0).round(), MP_Y + 2.0, 0xfcfcfc, HUD_Z + 1.5, (MpValue, InWidget("vitals")));
    }
}

fn hud_quests(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    log: Res<super::quests::QuestLog>,
    inv: Res<PlayerInv>,
    old: Query<Entity, With<QuestHud>>,
    mut last: Local<Option<String>>,
) {
    use super::quests::QuestKind;
    // Rebuild only when the picture changes (ready flips + live counts included).
    let key = log
        .0
        .iter()
        .map(|q| format!("{}:{}:{}", q.id, q.ready(&inv), q.have(&inv)))
        .collect::<Vec<_>>()
        .join("|");
    if Some(&key) == last.as_ref() {
        return;
    }
    *last = Some(key);
    for e in &old {
        commands.entity(e).despawn();
    }
    if log.0.is_empty() {
        return;
    }
    label(&mut commands, &mut images, "QUESTS", PAD, QUESTS_HDR_Y, 0xfcfcfc, HUD_Z + 1.0, (QuestHud, InWidget("quests")));
    for (i, q) in log.0.iter().enumerate() {
        let y = QUESTS_HDR_Y + 8.0 + i as f32 * 8.0;
        let ready = q.ready(&inv);
        let (mut name, suffix) = match &q.kind {
            // The short noun (e.g. WOLF) keeps the count visible (js split(' ').pop()).
            QuestKind::Slay { kind, need, .. } => (
                super::quests::kind_name(kind).split(' ').next_back().unwrap_or("FOE").to_string(),
                format!(" {}/{}", q.have(&inv).min(*need), need),
            ),
            QuestKind::Fetch { item, need } => (
                crate::items::get(item).map_or(item.as_str(), |d| d.name).to_uppercase(),
                format!(" {}/{}", q.have(&inv).min(*need), need),
            ),
            QuestKind::Bounty { name, .. } => (format!("Hunt {name}"), String::new()),
            QuestKind::Clear { enc_name, .. } => (enc_name.clone(), String::new()),
            QuestKind::Story { .. } => (q.title.clone(), String::new()),
        };
        // Clip the name, never the count (js clip()).
        let max_w = (INNER_W - 6.0) as i32 - font::measure(&suffix);
        while name.len() > 1 && font::measure(&name) > max_w {
            name.pop();
        }
        if ready {
            // Ready to turn in: the WoW gold '?' replaces the bullet (Baz).
            label(&mut commands, &mut images, "?", PAD, y, 0xffd34d, HUD_Z + 1.0, (QuestHud, InWidget("quests")));
        } else {
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0xff, 0xd3, 0x4d), Vec2::new(3.0, 3.0)),
                at(PAD, y + 1.0, 3.0, 3.0, HUD_Z + 1.0),
                PIXEL_LAYER,
                QuestHud,
                InWidget("quests"),
            ));
        }
        let text = format!("{name}{suffix}");
        label(&mut commands, &mut images, &text, PAD + 5.0, y, if ready { 0x7ee08a } else { 0xd8d8e0 }, HUD_Z + 1.0, (QuestHud, InWidget("quests")));
    }
}

fn hud_hp(
    mut commands: Commands,
    changed: Query<&Health, (With<Player>, Changed<Health>)>,
    old_value: Query<Entity, With<HpValue>>,
    mut fill: Query<&mut Sprite, With<HpFill>>,
    mut images: ResMut<Assets<Image>>,
    mut last: Local<(i32, i32)>,
) {
    let Ok(h) = changed.single() else { return };
    if *last == (h.hp, h.max) {
        return; // an invuln/flash tick, not an HP change
    }
    *last = (h.hp, h.max);
    let frac = h.hp.max(0) as f32 / h.max.max(1) as f32;
    if let Ok(mut s) = fill.single_mut() {
        set_bar(&mut s, BAR_W, HP_H, frac);
        // HP colour thresholds, port of the JS: green > 50%, amber > 25%, red below.
        s.color = if frac > 0.5 {
            Color::srgb_u8(0x00, 0xa8, 0x00)
        } else if frac > 0.25 {
            Color::srgb_u8(0xfc, 0x98, 0x38)
        } else {
            Color::srgb_u8(0xd8, 0x28, 0x00)
        };
    }
    for e in &old_value {
        commands.entity(e).despawn();
    }
    let text = format!("{}/{}", h.hp.max(0), h.max);
    let tw = font::measure(&text) as f32;
    label(&mut commands, &mut images, &text, PAD + 13.0 + ((BAR_W - tw) / 2.0).round(), HP_Y + 2.0, 0xfcfcfc, HUD_Z + 1.5, (HpValue, InWidget("vitals")));
}

/// Re-bake every derived prompt when a controller connects or disconnects — THE payoff of
/// the prompts-are-derived rule: one system, and every label in the game flips.
fn hud_prompts(
    mut commands: Commands,
    state: Res<ActionState>,
    bindings: Res<Bindings>,
    old_slots: Query<Entity, With<SlotPromptLabel>>,
    old_hint: Query<Entity, With<HintLabel>>,
    mut images: ResMut<Assets<Image>>,
    mut last_pad: Local<Option<bool>>,
) {
    // Rebuild when a pad connects/disconnects OR the player rebinds anything — every
    // on-screen prompt is DERIVED, so a rebind must reflow the labels instantly.
    if *last_pad == Some(state.pad_present) && !bindings.is_changed() {
        return;
    }
    *last_pad = Some(state.pad_present);
    for e in old_slots.iter().chain(old_hint.iter()) {
        commands.entity(e).despawn();
    }
    spawn_slot_prompts(&mut commands, &mut images, &bindings, state.pad_present, PROMPTS_Y);
    spawn_hint(&mut commands, &mut images, &bindings, state.pad_present);
}
