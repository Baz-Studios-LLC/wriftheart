//! slots.rs — the title's slot picker (js drawSlots + the 'slots' input arm): one card
//! per save with its hero summary. Destructive picks (delete, overwrite) demand a second
//! press, with the warning shown IN the card so there are no surprise deletions.

use super::{ArmKind, Pen, SlotMode, TitleState, View, TEXT_Z};
use crate::app::codex::calendar_tab::{day_of_season, season_index, SEASONS};
use crate::app::save::{delete_slot, scan_metas, SlotMetas, SAVE_SLOTS};
use crate::input::{Action, ActionState, Bindings};
use crate::{CANVAS_H, CANVAS_W};
use bevy::prelude::*;

/// What the picker resolved to this tick.
pub(super) enum SlotAct {
    None,
    Dirty,
    Load(u32),
    New(u32),
}

/// The card fill/hit rect (bx, y, bw, h) for slot index i — ONE geometry source for the draw
/// and the mouse hit-test.
pub(super) fn card_rect(i: usize) -> (f32, f32, f32, f32) {
    // y0 clears the word-art logo (10..95) AND its heading line at 100.
    let (y0, rh, bw) = (112.0, 21.0, 232.0);
    let bx = ((CANVAS_W as f32 - bw) / 2.0).round();
    (bx, y0 + i as f32 * rh - 3.0, bw, 17.0)
}

/// The card's DELETE chip (LOAD mode, occupied slots): a small X on the right edge —
/// click once to arm, click again to delete (the same two-step the key uses).
fn del_rect(i: usize) -> (f32, f32, f32, f32) {
    let (bx, cy, bw, _) = card_rect(i);
    (bx + bw - 14.0, cy + 3.0, 11.0, 11.0)
}

pub(super) fn tick(
    st: &mut TitleState,
    input: &ActionState,
    metas: &mut SlotMetas,
    ptr: &crate::input::Pointer,
) -> SlotAct {
    if input.pressed(Action::Slot2) {
        st.view = View::Main;
        st.armed = None;
        return SlotAct::Dirty;
    }
    let n_slots = SAVE_SLOTS as usize;
    let mut dirty = false;
    if input.pressed(Action::Up) {
        st.slot_sel = (st.slot_sel + n_slots - 1) % n_slots;
        st.armed = None;
        dirty = true;
    }
    if input.pressed(Action::Down) {
        st.slot_sel = (st.slot_sel + 1) % n_slots;
        st.armed = None;
        dirty = true;
    }
    // Mouse: hovering a different card selects it (and disarms, like the arrow keys); a click
    // is folded into the confirm below. A move WITHIN the armed card must not disarm, so the
    // hover only fires when the slot index actually changes.
    let mut clicked = false;
    let mut del_clicked = false;
    for i in 0..n_slots {
        let (rx, ry, rw, rh) = card_rect(i);
        if ptr.over(rx, ry, rw, rh) {
            if ptr.moved && st.slot_sel != i {
                st.slot_sel = i;
                st.armed = None;
                dirty = true;
            }
            if ptr.click {
                st.slot_sel = i;
                let (dx, dy, dw, dh) = del_rect(i);
                if st.slot_mode == SlotMode::Load && ptr.over(dx, dy, dw, dh) {
                    del_clicked = true; // the X chip: arm, then delete (never a load)
                } else {
                    clicked = true;
                }
            }
        }
    }
    let n = st.slot_sel as u32 + 1;
    let occupied = metas.0.get(st.slot_sel).is_some_and(|m| m.is_some());
    // Delete (LOAD mode, js slot3 'C' or the card's X chip): armed on the first
    // press, gone on the second.
    if (input.pressed(Action::Slot3) || del_clicked) && st.slot_mode == SlotMode::Load && occupied {
        if st.armed == Some((ArmKind::Delete, n)) {
            delete_slot(n);
            *metas = scan_metas();
            st.armed = None;
            if metas.0.iter().all(|m| m.is_none()) {
                st.view = View::Main;
                st.sel = 0;
            }
        } else {
            st.armed = Some((ArmKind::Delete, n));
        }
        return SlotAct::Dirty;
    }
    // Select = INTERACT or ENTER (js Input.confirm), with Slot1/Pause and a card click accepted.
    if clicked || input.pressed(Action::Interact) || input.pressed(Action::MenuConfirm) || input.pressed(Action::Slot1) || input.pressed(Action::Pause) {
        match st.slot_mode {
            SlotMode::Load => {
                if occupied {
                    return SlotAct::Load(n);
                }
            }
            SlotMode::New => {
                // Starting over an occupied slot asks twice (the overwrite guard).
                if occupied && st.armed != Some((ArmKind::Overwrite, n)) {
                    st.armed = Some((ArmKind::Overwrite, n));
                    return SlotAct::Dirty;
                }
                return SlotAct::New(n);
            }
        }
    }
    if dirty { SlotAct::Dirty } else { SlotAct::None }
}

pub(super) fn draw(pen: &mut Pen, st: &TitleState, metas: &SlotMetas, bindings: &Bindings, pad: bool) {
    let (w, h) = (CANVAS_W as f32, CANVAS_H as f32);
    let cx = w / 2.0;
    let hdr = if st.slot_mode == SlotMode::Load { "LOAD GAME" } else { "NEW GAME - PICK A SLOT" };
    pen.text_center(hdr, cx, 100.0, 0xfce0a8, TEXT_Z); // just under the logo's flourish
    for i in 0..SAVE_SLOTS as usize {
        let m = metas.0.get(i).and_then(|m| m.as_ref());
        let on = i == st.slot_sel;
        let (bx, cy, bw, ch) = card_rect(i);
        let y = cy + 3.0; // text sits 3px below the card top (was y0 + i*rh)
        // Card backing (js rgba fills; alpha bumped for the linear-blend gotcha).
        let back = if on { Color::srgba(0.988, 0.878, 0.659, 0.20) } else { Color::srgba(0.0, 0.0, 0.0, 0.52) };
        pen.fill_rgba(bx, cy, bw, ch, back, TEXT_Z - 0.02);
        if on {
            for (sx, sy, sw, sh) in crate::ui::border_strips(bx, cy, bw, ch, 1.0) {
                pen.fill(sx, sy, sw, sh, 0xfce0a8, TEXT_Z - 0.01);
            }
        }
        pen.text(&format!("SLOT {}", i + 1), bx + 4.0, y + 2.0, if on { 0xfce0a8 } else { 0x7a7a7a }, TEXT_Z);
        let (txt, col) = match (st.armed, m) {
            (Some((kind, an)), _) if an == i as u32 + 1 && on => (
                if kind == ArmKind::Delete { "PRESS AGAIN TO DELETE".into() } else { "PRESS AGAIN TO OVERWRITE".into() },
                0xfc7460,
            ),
            (_, Some(m)) => (
                format!(
                    "{}  LVL {}  {} D{}",
                    m.name.to_uppercase(),
                    m.level,
                    SEASONS[season_index(m.clock)],
                    day_of_season(m.clock),
                ),
                if on { 0xfcfcfc } else { 0x9a9aa0 },
            ),
            _ => ("- EMPTY -".into(), if on { 0xb8b8b8 } else { 0x5a5a60 }),
        };
        pen.text(&txt, bx + 40.0, y + 2.0, col, TEXT_Z);
        // The DELETE chip (LOAD mode, occupied): a small X the mouse can reach.
        if st.slot_mode == SlotMode::Load && m.is_some() {
            let (dx, dy, dw, dh) = del_rect(i);
            pen.fill_rgba(dx, dy, dw, dh, Color::srgba(0.0, 0.0, 0.0, 0.55), TEXT_Z - 0.01);
            pen.text("X", dx + 3.0, dy + 3.0, if on { 0xfc7460 } else { 0x8a5a5a }, TEXT_Z);
        }
    }
    let mut help = format!(
        "{} SELECT - {} BACK",
        bindings.prompt(Action::Interact, pad),
        bindings.prompt(Action::Slot2, pad)
    );
    if st.slot_mode == SlotMode::Load {
        help += &format!(" - {} DELETE", bindings.prompt(Action::Slot3, pad));
    }
    pen.text_center(&help, cx, h - 16.0, 0x5a6a5a, TEXT_Z);
}
