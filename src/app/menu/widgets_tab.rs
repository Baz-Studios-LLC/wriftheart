//! widgets_tab.rs — the WIDGETS tab of the pause menu (Baz: rearrange, pin, and
//! show/hide the sidebar widgets — pad and KBM alike). Three sections mirror the
//! config's normalized order: PINNED TOP, PINNED BOTTOM, HIDDEN. One verb does
//! everything: CONFIRM grabs the selected widget, UP/DOWN carries it (crossing a
//! section header re-pins or hides it), CONFIRM drops. SLOT3 is the quick
//! show/hide. The sidebar updates LIVE behind the dimmed panel.

use super::{Area, Draw, GOLD, MUTED, TEXT_Z};
use crate::app::hud_widgets::{def, HudConfig, HudRow};
use crate::gfx::font;
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

/// Cursor domain = one row per widget (headers aren't selectable).
pub fn len(cfg: &HudConfig) -> usize {
    cfg.0.len()
}

/// Carry the grabbed widget one step: swap within its section, or cross the
/// boundary into the next/previous section (re-pin / hide). Core widgets refuse
/// to enter HIDDEN.
pub fn shift(cfg: &mut HudConfig, index: &mut usize, down: bool) {
    normalize(cfg);
    let n = cfg.0.len();
    if n == 0 {
        return;
    }
    let k = (*index).min(n - 1);
    let r = rank(&cfg.0[k]);
    let core = def(&cfg.0[k].id).is_some_and(|d| d.core);
    if down {
        if k + 1 < n && rank(&cfg.0[k + 1]) == r {
            cfg.0.swap(k, k + 1);
            *index = k + 1;
        } else if r < 2 && !(core && r == 1) {
            set_rank(&mut cfg.0[k], r + 1); // head of the next section (stable sort keeps it first)
            *index = k;
        }
    } else if k > 0 && rank(&cfg.0[k - 1]) == r {
        cfg.0.swap(k - 1, k);
        *index = k - 1;
    } else if r > 0 {
        set_rank(&mut cfg.0[k], r - 1); // tail of the previous section
        *index = k;
    }
    normalize(cfg);
}

/// The SLOT3 quick-toggle: show/hide in place (core refuses). Returns redraw-due.
pub fn toggle(cfg: &mut HudConfig, index: &mut usize) -> bool {
    normalize(cfg);
    let n = cfg.0.len();
    if n == 0 {
        return false;
    }
    let k = (*index).min(n - 1);
    if def(&cfg.0[k].id).is_some_and(|d| d.core) {
        return false;
    }
    let id = cfg.0[k].id.clone();
    let on = cfg.0[k].on;
    cfg.0[k].on = !on;
    normalize(cfg);
    *index = cfg.0.iter().position(|r| r.id == id).unwrap_or(k);
    true
}

/// Display rows: every section header, then its widgets — mirrored by draw and
/// the mouse hit-test so the two can never drift.
fn display_rows(cfg: &HudConfig) -> Vec<Option<usize>> {
    let mut rows = Vec::new();
    for rk in 0..3 {
        rows.push(None); // the section header line
        for (i, r) in cfg.0.iter().enumerate() {
            if rank(r) == rk {
                rows.push(Some(i));
            }
        }
    }
    rows
}

/// The widget-row index under a canvas point (headers and gaps miss).
pub fn row_at(a: &Area, p: Vec2, cfg: &HudConfig) -> Option<usize> {
    if p.x < a.x || p.x >= a.x + a.w || p.y < a.y {
        return None;
    }
    let vi = ((p.y - a.y - 2.0) / RH).floor();
    if vi < 0.0 {
        return None;
    }
    display_rows(cfg).get(vi as usize).copied().flatten()
}

pub fn draw(d: &mut Draw, a: &Area, cfg: &HudConfig, sel: usize, grab: bool, bindings: &Bindings, pad: bool) {
    let mut y = a.y + 2.0;
    let mut next_header = 0usize;
    for row in display_rows(cfg) {
        match row {
            None => {
                d.text(HEADERS[next_header], a.x + 2.0, y + 2.0, 0x5a5a64, TEXT_Z);
                next_header += 1;
            }
            Some(i) => {
                let r = &cfg.0[i];
                let name = def(&r.id).map_or(r.id.as_str(), |w| w.name);
                let on = i == sel;
                let x = a.x + 14.0;
                if on {
                    d.text(if grab { "=" } else { ">" }, x - 8.0, y + 2.0, GOLD, TEXT_Z);
                }
                let col = if on && grab { 0xfcfcfc } else if on { GOLD } else { MUTED };
                d.text(name, x, y + 2.0, col, TEXT_Z);
                if def(&r.id).is_some_and(|w| w.core) {
                    let nw = font::measure(name) as f32;
                    d.text("*", x + nw + 4.0, y + 2.0, 0x5a5a64, TEXT_Z); // core: always shown
                }
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
