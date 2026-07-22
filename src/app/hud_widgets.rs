//! hud_widgets.rs — THE SIDEBAR WIDGET SYSTEM (Baz: "a true system for what a
//! widget is"). Every sidebar section is a WIDGET: a registry entry with a saved
//! row (order + pin TOP/BOTTOM + shown), a live height, and an availability rule.
//! One layout pass stacks the shown, available widgets — top group down from the
//! pad, bottom group up from the edge — and moves each widget's entities as a
//! unit. The ESC menu's WIDGETS tab rearranges them (menu/widgets_tab.rs).
//!
//! HOW A WIDGET MOVES: content systems keep spawning at their widget's fixed
//! BASELINE y (the old hand-laid consts), tagging every entity `InWidget(id)`.
//! `adopt` captures each new entity's spawn translation once (`BaseTy`), and
//! `place` shifts the whole widget by the layout's delta — no respawns, so the
//! change-detection caches in every hud system stay honest.
//!
//! ADDING A WIDGET: a WidgetDef row + a baseline + a height arm in layout_tick
//! (+ availability if it isn't always-on), then spawn your content tagged
//! `InWidget("yourid")` at the baseline. It appears in the ESC arranger free.

use crate::ui::label;
use crate::CANVAS_H;
use bevy::prelude::*;
use std::collections::HashMap;

use super::hud::{HUD_Z, PAD};

/// One registered widget. `core` widgets can be moved and pinned but never hidden
/// (a hero with no HP bar is a support ticket, not a build).
pub struct WidgetDef {
    pub id: &'static str,
    pub name: &'static str,
    pub core: bool,
}

/// The registry — order here is the default TOP order and the arranger's fallback.
pub const WIDGETS: &[WidgetDef] = &[
    WidgetDef { id: "vitals", name: "VITALS", core: true },
    WidgetDef { id: "abilities", name: "ABILITIES", core: false },
    WidgetDef { id: "clock", name: "CLOCK", core: false },
    WidgetDef { id: "quests", name: "QUESTS", core: false },
    WidgetDef { id: "buffs", name: "BUFFS", core: false },
    WidgetDef { id: "shards", name: "SHARDS", core: false },
    WidgetDef { id: "coins", name: "COPPER", core: false },
    WidgetDef { id: "hint", name: "HINTS", core: false },
];

pub fn def(id: &str) -> Option<&'static WidgetDef> {
    WIDGETS.iter().find(|w| w.id == id)
}

/// One arranged row: pin 0 = top stack, 1 = bottom stack; `on` = shown.
#[derive(Clone)]
pub struct HudRow {
    pub id: String,
    pub pin: u8,
    pub on: bool,
}

/// The player's arrangement, in display order (normalized: top, bottom, hidden).
/// SAVED with the run; menu/widgets_tab.rs edits it live.
#[derive(Resource)]
pub struct HudConfig(pub Vec<HudRow>);

impl Default for HudConfig {
    fn default() -> Self {
        let row = |id: &str, pin: u8, on: bool| HudRow { id: id.into(), pin, on };
        HudConfig(vec![
            row("vitals", 0, true),
            row("abilities", 0, true),
            row("clock", 0, true),
            row("quests", 0, true),
            row("buffs", 0, true),
            row("shards", 0, true),
            row("hint", 1, true),
            row("coins", 0, false),
        ])
    }
}

impl HudConfig {
    /// The save-file shape.
    pub fn rows_for_save(&self) -> Vec<(String, u8, bool)> {
        self.0.iter().map(|r| (r.id.clone(), r.pin, r.on)).collect()
    }
    /// Rebuild from a save: keep known ids in their saved order, append widgets the
    /// save predates (new builds grow the registry), drop ids that no longer exist,
    /// and re-assert the core rule. An empty save (legacy) = the defaults.
    pub fn load(&mut self, saved: &[(String, u8, bool)]) {
        if saved.is_empty() {
            *self = HudConfig::default();
            return;
        }
        let mut rows: Vec<HudRow> = saved
            .iter()
            .filter(|(id, ..)| def(id).is_some())
            .map(|(id, pin, on)| HudRow {
                id: id.clone(),
                pin: (*pin).min(1),
                on: *on || def(id).is_some_and(|d| d.core),
            })
            .collect();
        for d in WIDGETS {
            if !rows.iter().any(|r| r.id == d.id) {
                let dflt = HudConfig::default();
                let fallback = dflt.0.iter().find(|r| r.id == d.id).cloned();
                rows.push(fallback.unwrap_or(HudRow { id: d.id.into(), pin: 0, on: false }));
            }
        }
        self.0 = rows;
    }
}

/// Membership tag: this entity belongs to widget `id` and rides its layout.
#[derive(Component, Clone)]
pub struct InWidget(pub &'static str);

/// The entity's spawn translation.y, captured once — `place` shifts from here.
#[derive(Component)]
pub struct BaseTy(f32);

/// id -> downward delta from its baseline. A widget absent from the map is hidden
/// (turned off, unavailable, or overflowed off the sidebar).
#[derive(Resource, Default)]
pub struct WidgetLayout(pub HashMap<&'static str, f32>);

/// Where each widget's content spawns (the old hand-laid consts) — deltas are
/// measured against these, so the default arrangement is a zero-shift.
fn baseline(id: &str) -> f32 {
    match id {
        "vitals" => 8.0,
        "abilities" => 47.0,
        "clock" => 83.0,
        "quests" => 104.0,
        "buffs" => BUFFS_BASE,
        "shards" => SHARDS_BASE,
        "coins" => COINS_BASE,
        _ => CANVAS_H as f32 - 12.0, // hint
    }
}
pub const BUFFS_BASE: f32 = 160.0;
const SHARDS_BASE: f32 = 175.0;
const COINS_BASE: f32 = 190.0;
const GAP: f32 = 5.0;

/// Stack the arrangement: bottom group up from the edge first, then the top group
/// down from the pad; a top widget that would collide with the bottom stack is
/// dropped (better a hidden low-priority widget than an overlap).
#[allow(clippy::too_many_arguments)] // the layout reads every height source
pub fn layout_tick(
    cfg: Res<HudConfig>,
    log: Res<super::quests::QuestLog>,
    statuses: Res<super::status::Statuses>,
    inv: Res<crate::inventory::PlayerInv>,
    relics: Res<super::dungeon::Relics>,
    mut layout: ResMut<WidgetLayout>,
) {
    let quests_n = log.0.len();
    let buffs_n = super::status::DEFS.iter().filter(|d| statuses.has(d.id)).count().min(8);
    let height = |id: &str| -> Option<f32> {
        match id {
            "vitals" => Some(34.0),
            "abilities" => Some(31.0),
            "clock" => inv.has_gear_flag("clock").then_some(16.0),
            "quests" => (quests_n > 0).then(|| 8.0 + quests_n as f32 * 8.0),
            "buffs" => (buffs_n > 0).then(|| buffs_n.div_ceil(4) as f32 * 13.0),
            "shards" => (!relics.0.is_empty()).then_some(16.0),
            "coins" => Some(16.0),
            "hint" => Some(8.0),
            _ => None,
        }
    };
    let mut out: HashMap<&'static str, f32> = HashMap::new();
    // Bottom stack: first row sits nearest the edge, the next above it.
    let mut bot = CANVAS_H as f32 - 4.0;
    for r in cfg.0.iter().filter(|r| r.on && r.pin == 1) {
        let (Some(d), Some(h)) = (def(&r.id), height(&r.id)) else { continue };
        bot -= h;
        out.insert(d.id, bot - baseline(d.id));
        bot -= GAP;
    }
    // Top stack, stopped by the bottom stack's ceiling.
    let mut y = PAD;
    for r in cfg.0.iter().filter(|r| r.on && r.pin == 0) {
        let (Some(d), Some(h)) = (def(&r.id), height(&r.id)) else { continue };
        if y + h > bot {
            break; // out of sidebar — this and everything below it hides
        }
        out.insert(d.id, y - baseline(d.id));
        y += h + GAP;
    }
    if layout.0 != out {
        layout.0 = out;
    }
}

/// Capture each tagged entity's spawn translation once, the frame it appears.
pub fn adopt(mut commands: Commands, q: Query<(Entity, &Transform), (With<InWidget>, Without<BaseTy>)>) {
    for (e, tf) in &q {
        commands.entity(e).insert(BaseTy(tf.translation.y));
    }
}

/// Apply the layout: shift every widget entity by its delta, hide the hidden.
pub fn place(
    layout: Res<WidgetLayout>,
    fresh: Query<(), Added<BaseTy>>,
    mut q: Query<(&InWidget, &BaseTy, &mut Transform, &mut Visibility)>,
) {
    if !layout.is_changed() && fresh.is_empty() {
        return;
    }
    for (w, base, mut tf, mut vis) in &mut q {
        match layout.0.get(w.0) {
            Some(delta) => {
                // Canvas y runs down, world y up — a downward delta subtracts.
                tf.translation.y = base.0 - delta;
                *vis = Visibility::Inherited;
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

// ---------------------------------------------------------------------------
// The two widgets born WITH the system: the shard tally and the coin purse.

#[derive(Component)]
pub struct ShardHud;

/// SHARDS: N OF M in the endgame violet — hidden until the first shard is yours.
pub fn shards_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    relics: Res<super::dungeon::Relics>,
    world: Res<super::play::GameWorld>,
    old: Query<Entity, With<ShardHud>>,
    mut last: Local<Option<usize>>,
) {
    let have = relics.0.len();
    if *last == Some(have) {
        return;
    }
    *last = Some(have);
    for e in &old {
        commands.entity(e).despawn();
    }
    if have == 0 {
        return;
    }
    let goal = world.0.shard_biomes().len();
    label(&mut commands, &mut images, "SHARDS", PAD, SHARDS_BASE, 0xfcfcfc, HUD_Z + 1.0, (ShardHud, InWidget("shards")));
    let line = format!("{have} OF {goal}");
    label(&mut commands, &mut images, &line, PAD, SHARDS_BASE + 8.0, 0xe0b8ff, HUD_Z + 1.0, (ShardHud, InWidget("shards")));
}

#[derive(Component)]
pub struct CoinHud;

/// COPPER: the purse, live — for players who want money on the sidebar.
pub fn coins_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    inv: Res<crate::inventory::PlayerInv>,
    old: Query<Entity, With<CoinHud>>,
    mut last: Local<Option<i64>>,
) {
    if *last == Some(inv.money) {
        return;
    }
    *last = Some(inv.money);
    for e in &old {
        commands.entity(e).despawn();
    }
    label(&mut commands, &mut images, "COPPER", PAD, COINS_BASE, 0xfcfcfc, HUD_Z + 1.0, (CoinHud, InWidget("coins")));
    let amount = format!("{}", inv.money);
    label(&mut commands, &mut images, &amount, PAD, COINS_BASE + 8.0, 0xfcd000, HUD_Z + 1.0, (CoinHud, InWidget("coins")));
}
