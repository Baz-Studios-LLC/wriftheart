//! widgets_tab.rs — the WIDGETS tab of the pause menu (Baz: rearrange, pin, and
//! show/hide the sidebar widgets — pad and KBM alike). Three sections mirror the
//! config's normalized order: PINNED TOP, PINNED BOTTOM, HIDDEN. One verb does
//! everything: CONFIRM grabs the selected widget, UP/DOWN carries it (crossing a
//! section header re-pins or hides it), CONFIRM drops. SLOT3 is the quick
//! show/hide. The sidebar updates LIVE behind the dimmed panel.

use super::{Area, Draw, GOLD, MUTED, TEXT_Z};
use crate::app::hud_widgets::{def, unlocked, HudConfig, HudRow};
use crate::inventory::PlayerInv;
use crate::input::{Action, Bindings};
use bevy::prelude::Vec2;

const RH: f32 = 10.0;
const HEADERS: [&str; 3] = ["PINNED TOP", "PINNED BOTTOM", "HIDDEN"];

/// Display rank: 0 top, 1 bottom, 2 hidden — the section a row lives in.
fn rank(r: &HudRow) -> usize {
    if !r.on { 2 } else { r.pin as usize }
}

fn set_rank(r: &mut HudRow, rk: usize) {
    match rk {
        0 => { r.pin = 0; r.on = true; }
        1 => { r.pin = 1; r.on = true; }
        _ => r.on = false,
    }
}

/// Keep cfg.0 in display order (top block, bottom block, hidden block) — every
/// mutation runs through this, so a row's index IS its cursor position.
pub fn normalize(cfg: &mut HudConfig) {
    cfg.0.sort_by_key(rank); // stable: order within a section is preserved
}

/// The rows the menu may LIST: normalized cfg indices whose widget is unlocked —
/// item-gated widgets (the watch's CLOCK, the compass MINIMAP to come) never
/// appear before you own the item (Baz).
fn listed(cfg: &HudConfig, inv: &PlayerInv) -> Vec<usize> {
    (0..cfg.0.len()).filter(|&i| def(&cfg.0[i].id).is_some_and(|d| unlocked(d, inv))).collect()
}

/// Cursor domain = one row per LISTED widget (headers aren't selectable).
pub fn len(cfg: &HudConfig, inv: &PlayerInv) -> usize {
    listed(cfg, inv).len()
}

/// Carry the grabbed widget one step: swap within its section, or cross the
/// boundary into the next/previous section (re-pin / hide). Core widgets refuse
/// to enter HIDDEN.
pub fn shift(cfg: &mut HudConfig, inv: &PlayerInv, index: &mut usize, down: bool) {
    normalize(cfg);
    let vis = listed(cfg, inv);
    if vis.is_empty() {
        return;
    }
    let sel = (*index).min(vis.len() - 1);
    let k = vis[sel];
    let r = rank(&cfg.0[k]);
    let core = def(&cfg.0[k].id).is_some_and(|d| d.core);
    if down {
        if let Some(&j) = vis.get(sel + 1).filter(|&&j| rank(&cfg.0[j]) == r) {
            // Hop PAST any locked rows in between: remove + reinsert just after the
            // next visible neighbour (their hidden order doesn't matter).
            let row = cfg.0.remove(k);
            cfg.0.insert(j, row);
            *index = sel + 1;
        } else if r < 2 && !(core && r == 1) {
            set_rank(&mut cfg.0[k], r + 1); // head of the next section (stable sort keeps it first)
            *index = sel;
        }
    } else if sel > 0 && rank(&cfg.0[vis[sel - 1]]) == r {
        let j = vis[sel - 1];
        let row = cfg.0.remove(k);
        cfg.0.insert(j, row);
        *index = sel - 1;
    } else if r > 0 {
        set_rank(&mut cfg.0[k], r - 1); // tail of the previous section
        *index = sel;
    }
    normalize(cfg);
}

/// The SLOT3 quick-toggle: show/hide in place (core refuses). Returns redraw-due.
pub fn toggle(cfg: &mut HudConfig, inv: &PlayerInv, index: &mut usize) -> bool {
    normalize(cfg);
    let vis = listed(cfg, inv);
    if vis.is_empty() {
        return false;
    }
    let sel = (*index).min(vis.len() - 1);
    let k = vis[sel];
    if def(&cfg.0[k].id).is_some_and(|d| d.core) {
        return false;
    }
    let id = cfg.0[k].id.clone();
    let on = cfg.0[k].on;
    cfg.0[k].on = !on;
    normalize(cfg);
    *index = listed(cfg, inv).iter().position(|&i| cfg.0[i].id == id).unwrap_or(sel);
    true
}

/// Display rows: every section header, then its widgets — mirrored by draw and
/// the mouse hit-test so the two can never drift.
fn display_rows(cfg: &HudConfig, inv: &PlayerInv) -> Vec<Option<usize>> {
    let vis = listed(cfg, inv);
    let mut rows = Vec::new();
    for rk in 0..3 {
        rows.push(None); // the section header line
        for &i in &vis {
            if rank(&cfg.0[i]) == rk {
                rows.push(Some(i));
            }
        }
    }
    rows
}

/// The widget-row index under a canvas point (headers and gaps miss).
pub fn row_at(a: &Area, p: Vec2, cfg: &HudConfig, inv: &PlayerInv) -> Option<usize> {
    if p.x < a.x || p.x >= a.x + a.w || p.y < a.y {
        return None;
    }
    let vi = ((p.y - a.y - 2.0) / RH).floor();
    if vi < 0.0 {
        return None;
    }
    // The mouse reports the CURSOR index (position among listed rows), not the cfg index.
    let hit = display_rows(cfg, inv).get(vi as usize).copied().flatten()?;
    listed(cfg, inv).iter().position(|&i| i == hit)
}

pub fn draw(d: &mut Draw, a: &Area, cfg: &HudConfig, inv: &PlayerInv, sel: usize, grab: bool, bindings: &Bindings, pad: bool) {
    let vis = listed(cfg, inv);
    let mut y = a.y + 2.0;
    let mut next_header = 0usize;
    for row in display_rows(cfg, inv) {
        match row {
            None => {
                d.text(HEADERS[next_header], a.x + 2.0, y + 2.0, 0x5a5a64, TEXT_Z);
                next_header += 1;
            }
            Some(i) => {
                let r = &cfg.0[i];
                let name = def(&r.id).map_or(r.id.as_str(), |w| w.name);
                let on = vis.iter().position(|&v| v == i) == Some(sel);
                let x = a.x + 14.0;
                if on {
                    d.text(if grab { "=" } else { ">" }, x - 8.0, y + 2.0, GOLD, TEXT_Z);
                }
                let col = if on && grab { 0xfcfcfc } else if on { GOLD } else { MUTED };
                d.text(name, x, y + 2.0, col, TEXT_Z);
                // (Core widgets carry no marker — the hide toggle simply refusing
                // is clearer than an unexplained star was — Baz.)
            }
        }
        y += RH;
    }
    let hint = if grab {
        format!(
            "{}/{} MOVE - {} DROP",
            bindings.prompt(Action::Up, pad),
            bindings.prompt(Action::Down, pad),
            bindings.prompt(Action::Slot1, pad),
        )
    } else {
        format!(
            "{} GRAB - {} SHOW/HIDE",
            bindings.prompt(Action::Slot1, pad),
            bindings.prompt(Action::Slot3, pad),
        )
    };
    d.text(&hint, a.x + 2.0, a.y + a.h - 8.0, 0x8a8a92, TEXT_Z);
}
