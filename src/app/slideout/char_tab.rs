//! char_tab.rs — the CHARACTER page (port of updateGear/drawGear in js/inventory.js):
//! ONE unified cell list — gear column (HD/BD/FT), trinkets (T1-T3), the A/B/X/Y ability
//! row, and the bag grid — walked by the same spatial cursor (nearest cell the way you
//! pressed, `along + perp * 2.5`), every offset lifted from the JS `gearLayout`.
//!
//! The CARRY model: bag cells, ability slots and gear slots all behave alike. A picks an
//! item up from any cell and places it in any cell that accepts it (reorder / equip /
//! unequip / swap); X uses the bag item under the cursor; Y drops (tap one, hold the
//! stack); T trashes; H sorts. Cells hold UIDS (per-instance ids), never item-ids.

use super::{SlideOut, SlideOutUi, PANEL_W, Z};
use crate::actors::hero::Facing;
use crate::app::gather::spawn_pickup;
use crate::app::play::{HeroArt, Player};
use crate::combat::Health;
use crate::gfx::{at, bake, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::inventory::PlayerInv;
use crate::items::{self, ItemDef};
use crate::skilltree;
use crate::ui::{cell as ui_cell, frame_rect, label};
use crate::{CANVAS_H, SIDEBAR_W};
use bevy::prelude::*;

// The content area under the tab bar (the JS `a` rect): everything below is a-relative.
const A_X: f32 = 8.0; // panel-relative
const A_Y: f32 = 18.0;
const A_W: f32 = PANEL_W - 16.0;
const SL: f32 = 18.0; // cell size

pub const BAG_COLS: usize = 8;
pub const BAG_ROWS: usize = 5; // rows past the current cap are locked (unlock via Satchels)

const HOLD_FRAMES: u32 = 40; // ~0.65s: a held drop/trash flips from act-on-ONE to the STACK

/// A little padlock for the locked bag rows.
const PADLOCK: &[&str] = &[
    "..aaaa..",
    ".a....a.",
    ".a....a.",
    "aaaaaaaa",
    "aAAAAAAa",
    "aAAnnAAa",
    "aAAnnAAa",
    "aaaaaaaa",
];

/// Cell labels for the six gear slots — indexes match [`crate::inventory::GEAR_KEYS`].
const GEAR_LABELS: [&str; 6] = ["HD", "BD", "FT", "T1", "T2", "T3"];

#[derive(Clone, Copy, PartialEq)]
pub enum CellKind {
    Gear(usize),    // index into GEAR_KEYS / GEAR_LABELS
    Ability(usize), // slot index 0..4
    Bag(usize),     // bag index
}

pub struct Cell {
    pub kind: CellKind,
    pub rx: f32, // a-relative
    pub ry: f32,
    pub locked: bool,
}

/// The unified cell list — offsets verbatim from the JS `gearLayout`.
pub fn cells(inv: &PlayerInv) -> Vec<Cell> {
    let mut out = Vec::new();
    for (gi, (rx, ry)) in [(11.0, 6.0), (11.0, 30.0), (11.0, 54.0), (93.0, 6.0), (93.0, 30.0), (93.0, 54.0)]
        .into_iter()
        .enumerate()
    {
        out.push(Cell { kind: CellKind::Gear(gi), rx, ry, locked: false });
    }
    for (slot, rx) in [(0usize, 11.0), (1, 38.0), (2, 66.0), (3, 93.0)] {
        out.push(Cell { kind: CellKind::Ability(slot), rx, ry: 82.0, locked: false });
    }
    for i in 0..BAG_COLS * BAG_ROWS {
        let (c, r) = (i % BAG_COLS, i / BAG_COLS);
        out.push(Cell {
            kind: CellKind::Bag(i),
            rx: 130.0 + c as f32 * (SL + 2.0),
            ry: 11.0 + r as f32 * (SL + 2.0),
            locked: i >= inv.bag_cap(),
        });
    }
    out
}

/// The cursor's home when the page opens: the first bag slot (js `charEntry`).
pub fn home_cell() -> usize {
    10 // 6 gear + 4 ability cells precede bag0
}

/// Spatial cursor move — port of `navGear`: nearest cell in the pressed direction,
/// cost = distance-along + 2.5x perpendicular drift; locked cells can't be landed on.
fn nav_gear(cells: &[Cell], cur: usize, dx: f32, dy: f32) -> Option<usize> {
    let c = &cells[cur];
    let (cx, cy) = (c.rx + SL / 2.0, c.ry + SL / 2.0);
    let mut best = None;
    let mut best_cost = f32::INFINITY;
    for (i, e) in cells.iter().enumerate() {
        if i == cur || e.locked {
            continue;
        }
        let (ex, ey) = (e.rx + SL / 2.0, e.ry + SL / 2.0);
        let along = (ex - cx) * dx + (ey - cy) * dy;
        if along <= 1.0 {
            continue;
        }
        let perp = if dx != 0.0 { (ey - cy).abs() } else { (ex - cx).abs() };
        let cost = along + perp * 2.5;
        if cost < best_cost {
            best_cost = cost;
            best = Some(i);
        }
    }
    best
}

/// Direction presses drive the cursor (called from slideout_tick while CHAR is active).
pub fn nav(so: &mut SlideOut, state: &ActionState, inv: &PlayerInv) -> bool {
    let cells = cells(inv);
    let mut moved = false;
    for (a, dx, dy) in [
        (Action::Up, 0.0, -1.0),
        (Action::Down, 0.0, 1.0),
        (Action::Left, -1.0, 0.0),
        (Action::Right, 1.0, 0.0),
    ] {
        if state.pressed(a)
            && let Some(next) = nav_gear(&cells, so.gear_cursor.min(cells.len() - 1), dx, dy)
        {
            so.gear_cursor = next;
            moved = true;
        }
    }
    moved
}

// --- The carry model: which uid a cell holds, and what it may hold (js cellGet/cellSet/
// cellAccepts). ---

fn cell_get(inv: &PlayerInv, c: &Cell) -> Option<u32> {
    match c.kind {
        CellKind::Bag(i) => inv.bag.get(i).copied().flatten(),
        CellKind::Ability(s) => inv.slots[s],
        CellKind::Gear(g) => inv.gear[g],
    }
}

fn cell_set(inv: &mut PlayerInv, c: &Cell, uid: Option<u32>) {
    match c.kind {
        CellKind::Bag(i) => {
            if i < inv.bag.len() {
                inv.bag[i] = uid;
            }
        }
        CellKind::Ability(s) => inv.slots[s] = uid,
        CellKind::Gear(g) => inv.gear[g] = uid,
    }
}

fn cell_accepts(inv: &PlayerInv, c: &Cell, uid: Option<u32>) -> bool {
    let Some(uid) = uid else { return true };
    match c.kind {
        CellKind::Bag(_) => true, // the bag holds anything
        // (js also checks noEquip here — our equippable() folds it in.)
        CellKind::Ability(_) => inv.id_of(uid).is_some_and(items::equippable),
        CellKind::Gear(g) => {
            // Gear slot: the gear type must match (any trinket fits a trinket* slot).
            let Some(gs) = inv.id_of(uid).and_then(items::gear_slot) else { return false };
            let key = crate::inventory::GEAR_KEYS[g];
            if key.starts_with("trinket") { gs == "trinket" } else { gs == key }
        }
    }
}

/// Move the item from one cell to another, swapping any occupant back if it fits.
fn move_item(inv: &mut PlayerInv, from: &Cell, to: &Cell) -> bool {
    let Some(a) = cell_get(inv, from) else { return false };
    if !cell_accepts(inv, to, Some(a)) {
        return false;
    }
    let b = cell_get(inv, to);
    if b.is_some() && !cell_accepts(inv, from, b) {
        return false; // occupant can't swap back -> reject
    }
    cell_set(inv, from, b);
    cell_set(inv, to, Some(a));
    // (js: gear changes restat + re-skin the hero — hooks up when armor items port.)
    true
}

/// X on a bag cell: use the item (js useBagAt — blueprints join with the crafting port).
fn use_bag_at(inv: &mut PlayerInv, i: usize, health: &mut Health) {
    let Some(uid) = inv.bag.get(i).copied().flatten() else { return };
    let Some(def) = inv.def_of(uid) else { return };
    if !def.consumable {
        return;
    }
    if items::use_consumable(def, health) {
        inv.remove_entry(uid); // use() may veto (potion at full HP) — then keep it
    }
}

/// The drop point: one tile in front of the hero (js FACE offset arithmetic verbatim).
fn drop_pos(player: &Player) -> (f32, f32) {
    let f: (f32, f32) = match player.facing {
        Facing::Up => (0.0, -1.0),
        Facing::Down => (0.0, 1.0),
        Facing::Left => (-1.0, 0.0),
        Facing::Right => (1.0, 0.0),
    };
    (player.x + 8.0 + f.0 * 18.0 - 8.0, player.y + 9.0 + f.1 * 18.0 - 8.0)
}

/// Y on a bag cell: drop ONE in front of you (a ground pickup with no magnet).
fn drop_bag_at(
    inv: &mut PlayerInv,
    i: usize,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    player: &Player,
) {
    let Some(uid) = inv.bag.get(i).copied().flatten() else { return };
    let Some(id) = inv.id_of(uid) else { return };
    let (x, y) = drop_pos(player);
    spawn_pickup(commands, images, id, 1, x, y, false);
    inv.remove_entry(uid);
}

/// Drop the WHOLE stack in front of you (one pickup carrying the full qty).
fn drop_stack_at(
    inv: &mut PlayerInv,
    i: usize,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    player: &Player,
) {
    let Some(uid) = inv.bag.get(i).copied().flatten() else { return };
    let Some(id) = inv.id_of(uid) else { return };
    let (x, y) = drop_pos(player);
    let q = inv.remove_entry_all(uid);
    if q > 0 {
        spawn_pickup(commands, images, id, q, x, y, false);
    }
}

/// Trash = destroy (no ground pickup): one, or the whole stack.
fn trash_bag_at(inv: &mut PlayerInv, i: usize, stack: bool) {
    let Some(uid) = inv.bag.get(i).copied().flatten() else { return };
    if stack {
        inv.remove_entry_all(uid);
    } else {
        inv.remove_entry(uid);
    }
}

/// A held drop/trash in flight (js holdAct): release early = act on ONE, hold to the
/// threshold = act on the WHOLE STACK; `done` swallows the rest of the hold.
pub struct HoldAct {
    pub idx: usize, // bag index acted on
    pub trash: bool,
    pub act: Action, // whichever button armed it (Slot4 or Trash)
    pub t: u32,
    pub done: bool,
}

/// Is the carried cell this bag index? (js `held === 'bag' + idx`.)
fn held_is_bag(so: &SlideOut, cells: &[Cell], idx: usize) -> bool {
    so.held.is_some_and(|h| matches!(cells.get(h), Some(c) if c.kind == CellKind::Bag(idx)))
}

/// The CHAR page's button handling — port of updateGear minus the nav (which already ran).
/// Returns true when anything changed (the panel redraws).
#[allow(clippy::too_many_arguments)] // it IS the page's arity
pub fn actions(
    so: &mut SlideOut,
    state: &ActionState,
    shift: bool,
    inv: &mut PlayerInv,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    player: &Player,
    health: &mut Health,
) -> bool {
    let cells = cells(inv);
    let cur_i = so.gear_cursor.min(cells.len() - 1);
    let cur = &cells[cur_i];

    // A held drop/trash button resolves tap-vs-hold first — the hold owns its button
    // until release; other buttons wait a beat.
    if let Some(mut act) = so.hold_act.take() {
        if !state.held(act.act) {
            if !act.done && act.t < HOLD_FRAMES {
                if act.trash {
                    trash_bag_at(inv, act.idx, false);
                    // Trashed the last one you were carrying -> stop carrying it.
                    if held_is_bag(so, &cells, act.idx) && inv.bag.get(act.idx).copied().flatten().is_none() {
                        so.held = None;
                    }
                } else {
                    drop_bag_at(inv, act.idx, commands, images, player);
                }
            }
        } else {
            if !act.done {
                act.t += 1;
                if act.t >= HOLD_FRAMES {
                    if act.trash {
                        trash_bag_at(inv, act.idx, true);
                        if held_is_bag(so, &cells, act.idx) {
                            so.held = None;
                        }
                    } else {
                        drop_stack_at(inv, act.idx, commands, images, player);
                    }
                    act.done = true; // swallow the rest of the hold
                }
            }
            so.hold_act = Some(act);
        }
        return true; // redraw every tick — the progress bar is animating
    }

    if state.pressed(Action::Slot1) {
        // A: pick up / place / cancel.
        if cur.locked {
            // tink
        } else if so.held.is_none() {
            if cell_get(inv, cur).is_some() {
                so.held = Some(cur_i);
            }
        } else if so.held == Some(cur_i) {
            so.held = None; // tap the held cell again to cancel
        } else {
            let from = &cells[so.held.unwrap()];
            if move_item(inv, from, cur) {
                so.held = None;
            }
        }
        true
    } else if state.pressed(Action::Slot3) && matches!(cur.kind, CellKind::Bag(_)) {
        if let CellKind::Bag(i) = cur.kind {
            use_bag_at(inv, i, health);
        }
        true
    } else if state.pressed(Action::Slot4) {
        if so.held.is_none() {
            // Browsing: DROP (tap one / hold stack / SHIFT instant stack).
            if let CellKind::Bag(i) = cur.kind
                && cell_get(inv, cur).is_some()
            {
                if shift {
                    drop_stack_at(inv, i, commands, images, player);
                } else {
                    so.hold_act = Some(HoldAct { idx: i, trash: false, act: Action::Slot4, t: 0, done: false });
                }
            }
        } else {
            // Carrying: TRASH the carried item (tap one / hold all / SHIFT instant all) —
            // equipped gear can't be trashed directly; place it in the bag first.
            let hc = &cells[so.held.unwrap()];
            if let CellKind::Bag(i) = hc.kind
                && cell_get(inv, hc).is_some()
            {
                if shift {
                    trash_bag_at(inv, i, true);
                    so.held = None;
                } else {
                    so.hold_act = Some(HoldAct { idx: i, trash: true, act: Action::Slot4, t: 0, done: false });
                }
            }
        }
        true
    } else if state.pressed(Action::Trash) {
        // T / R3: TRASH the cursor cell (tap one / hold stack / SHIFT instant stack).
        if let CellKind::Bag(i) = cur.kind
            && cell_get(inv, cur).is_some()
        {
            if shift {
                trash_bag_at(inv, i, true);
                if held_is_bag(so, &cells, i) {
                    so.held = None;
                }
            } else {
                so.hold_act = Some(HoldAct { idx: i, trash: true, act: Action::Trash, t: 0, done: false });
            }
        }
        true
    } else if state.pressed(Action::Sort) {
        // H / L3: SORT the bag (drop any half-made move — positions just changed).
        inv.sort_bag();
        so.held = None;
        true
    } else {
        false
    }
}

/// Word-wrap `text` to `width` px using the real font metrics (js wrapText).
pub fn wrap_text(text: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let probe = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if font::measure(&probe) as f32 <= width || line.is_empty() {
            line = probe;
        } else {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Draw the whole CHAR page (called from the slide-out redraw).
#[allow(clippy::too_many_arguments)] // it IS the page's arity
pub fn draw(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    so: &SlideOut,
    bindings: &Bindings,
    pad: bool,
    inv: &PlayerInv,
    hero: &HeroArt,
    alloc: &super::TreeAlloc,
    health: &Health,
) {
    let ax = SIDEBAR_W + A_X;
    let ay = A_Y;
    let tag = || SlideOutUi;

    // Hero portrait, centred in the 64px gap between the columns (3x = integer scale).
    let mut doll = Sprite::from_image(hero.0.frames[0][0].clone());
    doll.custom_size = Some(Vec2::splat(48.0));
    commands.spawn((doll, at(ax + 37.0, ay + 14.0, 48.0, 48.0, Z + 1.2), PIXEL_LAYER, tag()));

    let lock = images.add(bake(PADLOCK, &[]));

    let all = cells(inv);
    let cursor = so.gear_cursor.min(all.len() - 1);
    let mut sel_def: Option<&'static ItemDef> = None;
    for (i, c) in all.iter().enumerate() {
        let (x, y) = (ax + c.rx, ay + c.ry);
        let sel = i == cursor;
        if c.locked {
            ui_cell(commands, x, y, SL, Some(0x0c0c0e), 0x26262c, Some((lock.clone(), 8.0)), Z + 1.0, tag());
            continue;
        }
        let uid = cell_get(inv, c);
        let entry = uid.and_then(|u| inv.entry(u));
        let def = entry.and_then(|e| items::get(e.id));
        if sel {
            sel_def = def;
        }
        let is_held = so.held == Some(i); // the item currently picked up to move
        // Border: carried cyan, cursor gold, else the item's rarity tint, else empty grey.
        let border = if is_held {
            0x7fe0ff
        } else if sel {
            0xfce0a8
        } else if let Some(def) = def {
            def.rarity.color()
        } else {
            0x454552
        };
        let fill = if matches!(c.kind, CellKind::Bag(_)) { 0x202024 } else { 0x14141e };
        ui_cell(commands, x, y, SL, Some(fill), border, None, Z + 1.0, tag());
        if is_held || sel {
            // js lineWidth 2: the stroke straddles the cell edge — add the outer ring.
            frame_rect(commands, x - 1.0, y - 1.0, SL + 2.0, SL + 2.0, border, Z + 1.25, tag());
        }
        // The item's icon, 16x16 (2x the 8px grid) centred in the 18px cell (js drawImage).
        if let Some(def) = def {
            let mut icon = Sprite::from_image(images.add(bake(def.icon, def.icon_pal)));
            icon.custom_size = Some(Vec2::splat(16.0));
            commands.spawn((icon, at(x + 1.0, y + 1.0, 16.0, 16.0, Z + 1.3), PIXEL_LAYER, tag()));
        } else if let CellKind::Gear(g) = c.kind {
            // An empty gear slot names itself.
            label(commands, images, GEAR_LABELS[g], x + 3.0, y + SL - 7.0, 0x50505c, Z + 1.4, tag());
        }
        if let CellKind::Ability(s) = c.kind {
            // The trigger button, always shown top-left over a dark backing (js).
            let lab = bindings.prompt([Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4][s], pad);
            let lw = font::measure(lab) as f32;
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0x08, 0x08, 0x0a), Vec2::new(lw + 2.0, 7.0)),
                at(x + 1.0, y + 1.0, lw + 2.0, 7.0, Z + 1.35),
                PIXEL_LAYER,
                tag(),
            ));
            label(commands, images, lab, x + 2.0, y + 2.0, 0xfcdf8a, Z + 1.4, tag());
        }
        if let Some(e) = entry
            && e.qty > 1
        {
            let qty = format!("{}", e.qty);
            let qw = font::measure(&qty) as f32;
            label(commands, images, &qty, x + SL - 1.0 - qw, y + SL - 7.0, 0xfcfcfc, Z + 1.45, tag());
        }
        // Hold-to-stack progress: amber = drop, red = trash (js holdAct bar).
        if let Some(actn) = &so.hold_act
            && !actn.done
            && c.kind == CellKind::Bag(actn.idx)
        {
            let w = ((actn.t as f32 / HOLD_FRAMES as f32) * (SL - 2.0)).round();
            if w > 0.0 {
                let col = if actn.trash { 0xe04040 } else { 0xfcd000 };
                commands.spawn((
                    Sprite::from_color(Color::srgb_u8((col >> 16) as u8, (col >> 8) as u8, col as u8), Vec2::new(w, 2.0)),
                    at(x + 1.0, y + SL - 3.0, w, 2.0, Z + 1.45),
                    PIXEL_LAYER,
                    tag(),
                ));
            }
        }
    }

    // Bag fullness header (red when full) + the coin pips, flush right (js drawGear).
    let (used, cap) = (inv.bag_used(), inv.bag_cap());
    let bag_hdr = format!("BAG {used}/{cap}");
    label(commands, images, &bag_hdr, ax + 130.0, ay + 2.0, if used >= cap { 0xfc6868 } else { 0x8a8a92 }, Z + 1.0, tag());
    {
        // Coin — gold / silver / copper pips, right-aligned, level with the BAG header.
        let m = inv.money;
        let coins: [(u32, String); 3] = [
            (0xfcd000, format!("{}", m / 10000)),
            (0xcfd0d2, format!("{}", (m % 10000) / 100)),
            (0xc87838, format!("{}", m % 100)),
        ];
        let width: f32 = coins.iter().map(|(_, v)| 7.0 + font::measure(v) as f32 + 6.0).sum::<f32>() - 6.0;
        let mut cx = ax + A_W - width;
        for (col, val) in coins {
            commands.spawn((
                Sprite::from_color(Color::srgb_u8((col >> 16) as u8, (col >> 8) as u8, col as u8), Vec2::splat(5.0)),
                at(cx, ay + 2.0, 5.0, 5.0, Z + 1.0),
                PIXEL_LAYER,
                tag(),
            ));
            // The pip's dark rim (js: strokeRect(cx-0.5, py-0.5, 6, 6) in #202020 — a 1px
            // near-black border hugging the 5px coin).
            frame_rect(commands, cx - 1.0, ay + 1.0, 7.0, 7.0, 0x202020, Z + 1.0, tag());
            label(commands, images, &val, cx + 7.0, ay + 2.0, 0xfcfcfc, Z + 1.0, tag());
            cx += 7.0 + font::measure(&val) as f32 + 6.0;
        }
    }

    // Detail + compact derived stats along the bottom (js: divider at a.y + 118).
    let dy = ay + 118.0;
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x2a, 0x2a, 0x30), Vec2::new(A_W - 3.0, 1.0)),
        at(ax, dy, A_W - 3.0, 1.0, Z + 1.0),
        PIXEL_LAYER,
        tag(),
    ));
    match sel_def {
        Some(d) => {
            label(commands, images, &d.name.to_uppercase(), ax, dy + 6.0, d.rarity.color(), Z + 1.0, tag());
            let kind_line = if d.kind.is_empty() {
                d.rarity.name().to_string()
            } else {
                format!("{}  {}", d.rarity.name(), d.kind)
            };
            label(commands, images, &kind_line, ax, dy + 15.0, 0x7c84a0, Z + 1.0, tag());
            // The item's own stats (js wstats/stats line) — generated + fixed gear carry them.
            let sline = item_stats_line(d);
            let desc_y = if sline.is_empty() {
                dy + 25.0
            } else {
                label(commands, images, &sline, ax, dy + 24.0, 0xe8d090, Z + 1.0, tag());
                dy + 33.0
            };
            for (i, ln) in wrap_text(d.desc, 144.0).iter().take(2).enumerate() {
                label(commands, images, &ln.to_uppercase(), ax, desc_y + i as f32 * 8.0, 0x9a9aa0, Z + 1.0, tag());
            }
        }
        None => {
            label(commands, images, "- EMPTY -", ax, dy + 6.0, 0x5a5a5a, Z + 1.0, tag());
        }
    }
    // Derived stats — live values (the % lines read the tree like js player.stat).
    let sgn = |v: f64| format!("{}{}%", if v >= 0.0 { "+" } else { "" }, (v * 100.0 + 0.5).floor() as i64);
    let stats: [(&str, String); 6] = [
        ("ARMOR", format!("{}", health.defense)),
        ("HP", format!("{}", health.max)),
        ("DMG", sgn(skilltree::stat(&alloc.taken, "melee"))),
        ("MOVE", sgn(skilltree::stat(&alloc.taken, "move") / 1.25)),
        ("LUCK", sgn(skilltree::stat(&alloc.taken, "luck"))),
        ("GOLD", sgn(skilltree::stat(&alloc.taken, "coin"))),
    ];
    for (i, (name, value)) in stats.iter().enumerate() {
        let y = dy + 6.0 + i as f32 * 9.0;
        label(commands, images, name, ax + 150.0, y, 0xa0a0a0, Z + 1.0, tag());
        let vw = font::measure(value) as f32;
        label(commands, images, value, ax + A_W - 4.0 - vw, y, 0xfcfcfc, Z + 1.0, tag());
    }

    // The control hint, centred along the bottom (js drawGear): the carry state swaps it.
    let p = |a: Action| bindings.prompt(a, pad);
    let tabs = format!(" - {}/{} TABS", p(Action::TabPrev), p(Action::TabNext));
    let hint = if so.held.is_some() {
        // Carrying: Y trashes the carried item — the pad's deliberate two-step destroy.
        format!("{} PLACE - {} CANCEL - {} TRASH (HOLD: ALL){tabs}", p(Action::Slot1), p(Action::Slot2), p(Action::Slot4))
    } else if !pad {
        // Keyboard gets instant T-trash + SHIFT; a pad taps/holds Y instead.
        format!(
            "{} EQUIP - {} USE - {} DROP - {} TRASH | SHIFT: STACK - {} SORT{tabs}",
            p(Action::Slot1),
            p(Action::Slot3),
            p(Action::Slot4),
            p(Action::Trash),
            p(Action::Sort)
        )
    } else {
        format!(
            "{} EQUIP - {} USE - {} DROP - {} TRASH (HOLD: STACK) - {} SORT{tabs}",
            p(Action::Slot1),
            p(Action::Slot3),
            p(Action::Slot4),
            p(Action::Trash),
            p(Action::Sort)
        )
    };
    let hw = font::measure(&hint) as f32;
    label(commands, images, &hint, (ax + A_W / 2.0 - hw / 2.0).round(), CANVAS_H as f32 - 8.0, 0x909098, Z + 1.0, tag());
}


/// The item's own stats as a compact line (js wstats for weapons, stats for armour):
/// "DMG 4  CRIT 10%  KNOCK 2" or "+2 ARMOR  +1 HP". Empty for items with no stats.
fn item_stats_line(d: &crate::items::ItemDef) -> String {
    let get = |k: &str| d.stats.iter().find(|(sk, _)| *sk == k).map(|(_, v)| *v);
    let pct = |v: f64| format!("{}%", (v * 100.0 + 0.5).floor() as i64);
    let mut parts: Vec<String> = Vec::new();
    if d.weapon {
        if let Some(v) = get("dmg") {
            parts.push(format!("DMG {}", v as i64));
        }
        if let Some(v) = get("crit").filter(|v| *v > 0.0) {
            parts.push(format!("CRIT {}", pct(v)));
        }
        if let Some(v) = get("critmult").filter(|v| *v > 0.0) {
            parts.push(format!("CRITDMG {}", pct(v)));
        }
        if let Some(v) = get("leech").filter(|v| *v > 0.0) {
            parts.push(format!("LEECH {}", pct(v)));
        }
        if let Some(v) = get("knock").filter(|v| *v > 0.0) {
            parts.push(format!("KNOCK {}", v as i64));
        }
    } else {
        // Armour + trinkets: flat +N for whole stats, +N% for fractional.
        const LABELS: &[(&str, &str)] = &[
            ("defense", "ARMOR"), ("maxhp", "HP"), ("maxmana", "MP"), ("regen", "REGEN"),
            ("move", "MOVE"), ("luck", "LUCK"), ("coin", "GOLD"), ("melee", "DMG"), ("crit", "CRIT"),
        ];
        for (k, name) in LABELS {
            if let Some(v) = get(k).filter(|v| *v != 0.0) {
                if v.fract() == 0.0 {
                    parts.push(format!("+{} {}", v as i64, name));
                } else {
                    parts.push(format!("+{} {}", pct(v), name));
                }
            }
        }
    }
    parts.join("  ")
}