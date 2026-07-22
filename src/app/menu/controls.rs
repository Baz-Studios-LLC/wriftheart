//! controls.rs — the CONTROLS rebind table (js drawControls): ACTION / KEY / PAD columns,
//! scrolling to keep the cursor in view, PRESS... while a capture is armed, and a
//! RESET DEFAULTS row at the bottom. The js list includes INTERACT — that action lands
//! with towns/NPCs and its row appears then.

use super::{Area, Draw, GOLD, MUTED, TEXT_Z};
use crate::input::{Action, Bindings};

/// Row order mirrors the js ACTIONS table (Sort is the js warpHome, "MENU HELPER").
const ROWS: [(Action, &str); 30] = [
    (Action::Up, "MOVE UP"),
    (Action::Down, "MOVE DOWN"),
    (Action::Left, "MOVE LEFT"),
    (Action::Right, "MOVE RIGHT"),
    (Action::Slot1, "ABILITY 1"),
    (Action::Slot2, "ABILITY 2"),
    (Action::Slot3, "ABILITY 3"),
    (Action::Slot4, "ABILITY 4"),
    (Action::Interact, "INTERACT"),
    (Action::Dodge, "DODGE"),
    (Action::Inventory, "INVENTORY"),
    (Action::SkillTree, "SKILL TREE"),
    (Action::TabPrev, "PREV TAB"),
    (Action::TabNext, "NEXT TAB"),
    (Action::Map, "MAP"),
    (Action::Calendar, "CALENDAR"),
    (Action::People, "PEOPLE"),
    (Action::Guilds, "GUILDS"),
    (Action::Mobs, "MOB DEX"),
    (Action::ItemsDex, "ITEM DEX"),
    (Action::Songs, "SONGS"),
    (Action::Awards, "AWARDS"),
    (Action::StatsTab, "STATS"),
    (Action::Lore, "LORE"),
    (Action::Wriftheart, "WRIFTHEART"),
    (Action::Craft, "CRAFTING"),
    (Action::StatusTab, "STATUS"),
    (Action::Sort, "MENU HELPER"),
    (Action::Trash, "TRASH"),
    (Action::Pause, "PAUSE"),
];

/// Row count including RESET DEFAULTS.
pub fn len() -> usize {
    ROWS.len() + 1
}

/// The action on a row (None on the RESET row).
pub fn action_at(index: usize) -> Option<Action> {
    ROWS.get(index).map(|(a, _)| *a)
}

pub fn draw(d: &mut Draw, a: &Area, sel: usize, capturing: bool, bindings: &Bindings) {
    let label_x = a.x + 12.0;
    // ONE key-or-mouse column (a key and a mouse button are mutually exclusive per action
    // now — Baz), plus the pad column. "PRESS..." accepts either a key or a mouse click.
    let key_r = a.x + a.w - 58.0;
    let pad_r = a.x + a.w;
    d.text("ACTION", label_x, a.y, 0x606060, TEXT_Z);
    d.text_right("KEY/MOUSE", key_r, a.y, 0x606060, TEXT_Z);
    d.text_right("PAD", pad_r, a.y, 0x606060, TEXT_Z);

    let rh = 10.0;
    let y0 = a.y + 11.0;
    let vis = (((a.y + a.h - y0) / rh).floor() as usize).max(1);
    let n = len();
    let scroll = if n > vis { (sel as i32 - (vis / 2) as i32).clamp(0, (n - vis) as i32) as usize } else { 0 };
    for vi in 0..vis.min(n - scroll) {
        let i = scroll + vi;
        let on = i == sel;
        let y = y0 + vi as f32 * rh;
        let col = if on { GOLD } else { MUTED };
        if on {
            d.text(">", label_x - 10.0, y, GOLD, TEXT_Z);
        }
        let Some((act, lbl)) = ROWS.get(i) else {
            d.text("RESET DEFAULTS", label_x, y, col, TEXT_Z);
            continue;
        };
        d.text(lbl, label_x, y, col, TEXT_Z);
        if capturing && on {
            d.text_right("PRESS...", key_r, y, 0xfc9838, TEXT_Z);
            continue;
        }
        d.text_right(&bindings.kbm_name(*act), key_r, y, col, TEXT_Z);
        // The four MOVE rows are also driven by the LEFT STICK (fixed — an axis, not a
        // rebindable button); show it so the analog input isn't invisible (Baz).
        let is_move = matches!(act, Action::Up | Action::Down | Action::Left | Action::Right);
        let pad = bindings.pad_names(*act);
        let pad = if is_move {
            if pad == "--" { "STICK".to_string() } else { format!("STICK/{pad}") }
        } else {
            pad
        };
        d.text_right(&pad, pad_r, y, col, TEXT_Z);
    }
    if scroll > 0 {
        d.text("▲", pad_r - 6.0, y0 - 1.0, 0x808080, TEXT_Z); // more above
    }
    if scroll + vis < n {
        d.text("▼", pad_r - 6.0, y0 + (vis - 1) as f32 * rh, 0x808080, TEXT_Z); // more below
    }
}
