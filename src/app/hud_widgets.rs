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
    /// Item gate: a gear FLAG that must be OWNED before the widget exists at all —
    /// gated widgets never appear in the ESC arranger until you have the item
    /// (Baz: the watch's CLOCK today, the compass MINIMAP to come).
    pub gate: Option<&'static str>,
}

/// Is this widget allowed to exist for this player right now?
pub fn unlocked(w: &WidgetDef, inv: &crate::inventory::PlayerInv) -> bool {
    w.gate.is_none_or(|flag| inv.owns_flagged(flag))
}

/// The registry — order here is the default TOP order and the arranger's fallback.
pub const WIDGETS: &[WidgetDef] = &[
    WidgetDef { id: "vitals", name: "CHARACTER", core: true, gate: None }, // (Baz) — core: movable, never hideable
    WidgetDef { id: "abilities", name: "ITEMS", core: false, gate: None }, // the sidebar header says ITEMS - the arranger matches (Baz)
    WidgetDef { id: "clock", name: "CLOCK", core: false, gate: Some("clock") },
    WidgetDef { id: "minimap", name: "MINIMAP", core: false, gate: Some("compass") },
    WidgetDef { id: "quests", name: "QUESTS", core: false, gate: None },
    WidgetDef { id: "buffs", name: "BUFFS/DEBUFFS", core: false, gate: None }, // it shows both kinds (Baz)
    WidgetDef { id: "shards", name: "SHARDS", core: false, gate: None },
    WidgetDef { id: "coins", name: "COIN", core: false, gate: None },
    WidgetDef { id: "hint", name: "HINTS", core: false, gate: None },
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
            row("minimap", 1, true),
            row("hint", 1, true),
            row("coins", 0, true),
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
        "minimap" => MM_BASE,
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
            "minimap" => inv.has_gear_flag("compass").then_some(MM_H),
            "hint" => Some(8.0),
            _ => None,
        }
    };
    let mut out: HashMap<&'static str, f32> = HashMap::new();
    // Bottom stack, REVERSED: the arranger lists this section top-to-bottom exactly
    // as it renders (Baz: it read upside down) — the LAST listed row hugs the edge.
    let mut bot = CANVAS_H as f32 - 4.0;
    for r in cfg.0.iter().filter(|r| r.on && r.pin == 1).rev() {
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
    label(&mut commands, &mut images, "COIN", PAD, COINS_BASE, 0xfcfcfc, HUD_Z + 1.0, (CoinHud, InWidget("coins")));
    // The coin pip + the purse, side by side (Baz: a proper coin widget).
    let img = images.add(crate::gfx::bake(crate::actors::items_art::COIN_ICON, &[]));
    commands.spawn((
        Sprite::from_image(img),
        crate::gfx::at(PAD, COINS_BASE + 7.0, 8.0, 8.0, HUD_Z + 1.0),
        crate::gfx::PIXEL_LAYER,
        CoinHud,
        InWidget("coins"),
    ));
    let amount = format!("{}", inv.money);
    label(&mut commands, &mut images, &amount, PAD + 10.0, COINS_BASE + 8.0, 0xfcd000, HUD_Z + 1.0, (CoinHud, InWidget("coins")));
}


// ---------------------------------------------------------------------------
// THE MINIMAP — the compass trinket's promise, kept the JS WAY (Baz: "go check
// the js again"): each room is its 1px-per-tile THUMBNAIL (the codex map's own
// baker + cache — water, paths, walls, forests all read), a 3x3 window around
// you, town/dungeon/home pips, and YOUR dot at your actual tile in the centre
// room. Owning the compass lists the widget; wearing it lights the map.

#[derive(Component)]
pub struct MinimapHud;

const MM_BASE: f32 = 120.0;
// The frame spans EXACTLY the ability-icon row (Baz): PAD to PAD + 64. Inner 62 =
// three 20px cells + two 1px seams; each 19-tile thumb stretches one px to fit.
const MM_W: f32 = crate::SIDEBAR_W - 2.0 * PAD;
const MM_TW: f32 = 20.0;
pub const MM_H: f32 = (crate::room::ROWS * 3 + 2) as f32 + 2.0;

fn rgb(c: u32) -> Color {
    Color::srgb_u8((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn minimap_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    inv: Res<crate::inventory::PlayerInv>,
    cur: Res<super::play::CurRoom>,
    world: Res<super::play::GameWorld>,
    visited: Res<super::play::Visited>,
    phouse: Res<super::home::PlayerHouse>,
    mut cache: ResMut<super::codex::map_tab::ThumbCache>,
    players: Query<&super::play::Player>,
    old: Query<Entity, With<MinimapHud>>,
    mut last: Local<Option<(i32, i32, bool, usize, i32, i32)>>,
) {
    use crate::room::{COLS, ROWS};
    let on = inv.has_gear_flag("compass");
    let (ptx, pty) = players
        .single()
        .map(|p| ((p.x / 16.0) as i32, (p.y / 16.0) as i32))
        .unwrap_or((COLS / 2, ROWS / 2));
    let key = Some((cur.rx, cur.ry, on, visited.0.len(), ptx, pty));
    if *last == key {
        return;
    }
    *last = key;
    for e in &old {
        commands.entity(e).despawn();
    }
    if !on {
        return;
    }
    let tag = || (MinimapHud, InWidget("minimap"));
    let (x0, y0) = (PAD, MM_BASE);
    // Slate backing + frame (unexplored rooms stay this dark).
    commands.spawn((
        Sprite::from_color(rgb(0x0a0a10), Vec2::new(MM_W, MM_H)),
        crate::gfx::at(x0, y0, MM_W, MM_H, HUD_Z + 0.3),
        crate::gfx::PIXEL_LAYER,
        tag(),
    ));
    for (sx, sy, sw, sh) in crate::ui::border_strips(x0, y0, MM_W, MM_H, 1.0) {
        commands.spawn((
            Sprite::from_color(rgb(0x4a4a58), Vec2::new(sw, sh)),
            crate::gfx::at(sx, sy, sw, sh, HUD_Z + 0.5),
            crate::gfx::PIXEL_LAYER,
            tag(),
        ));
    }
    let home = phouse.0.as_ref().map(|h| h.room);
    for dy in -1..=1i32 {
        for dx in -1..=1i32 {
            let (rx, ry) = (cur.rx + dx, cur.ry + dy);
            if !visited.0.contains(&(rx, ry)) {
                continue;
            }
            let cx = x0 + 1.0 + (dx + 1) as f32 * (MM_TW + 1.0);
            let cy = y0 + 1.0 + ((dy + 1) * (ROWS + 1)) as f32;
            // The codex map's own thumbnail, from the SHARED cache (bake once, ever).
            let img = cache
                .0
                .entry((rx, ry))
                .or_insert_with(|| images.add(super::codex::map_tab::room_thumb(&world.0, rx, ry)))
                .clone();
            let mut spr = Sprite::from_image(img);
            spr.custom_size = Some(Vec2::new(MM_TW, ROWS as f32));
            commands.spawn((
                spr,
                crate::gfx::at(cx, cy, MM_TW, ROWS as f32, HUD_Z + 0.35),
                crate::gfx::PIXEL_LAYER,
                tag(),
            ));
            let pip = if world.0.is_town(rx, ry) {
                Some(0xfcd000)
            } else if world.0.shard_dungeon_at(rx, ry).is_some() {
                Some(0xe23030)
            } else if home == Some((rx, ry)) {
                Some(0x7ee08a)
            } else {
                None
            };
            if let Some(pc) = pip {
                commands.spawn((
                    Sprite::from_color(rgb(pc), Vec2::new(2.0, 2.0)),
                    crate::gfx::at(cx + MM_TW - 3.0, cy + 1.0, 2.0, 2.0, HUD_Z + 0.4),
                    crate::gfx::PIXEL_LAYER,
                    tag(),
                ));
            }
        }
    }
    // YOU: a white dot at your ACTUAL TILE in the centre room (1px = 1 tile).
    let px = x0 + 1.0 + (MM_TW + 1.0) + ((ptx.clamp(0, COLS - 1) as f32 + 0.5) * MM_TW / COLS as f32).floor() - 1.0;
    let py = y0 + 1.0 + (ROWS + 1) as f32 + pty.clamp(0, ROWS - 1) as f32 - 0.5;
    commands.spawn((
        Sprite::from_color(rgb(0xffffff), Vec2::new(2.0, 2.0)),
        crate::gfx::at(px, py, 2.0, 2.0, HUD_Z + 0.45),
        crate::gfx::PIXEL_LAYER,
        tag(),
    ));
}
