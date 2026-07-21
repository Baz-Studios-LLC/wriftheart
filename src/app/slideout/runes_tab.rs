//! runes_tab.rs — the ENCHANTER's second page: IMBUE THE WAND. The four elements
//! are rows — the bare arcane wand plus the three rune stones; picking one sockets
//! it and the ejected rune pops back into the bag (the js use-a-rune swap, given a
//! proper ceremony at the table that makes them). The socket happens INLINE, not
//! via WandMsg — the panel must redraw true on the same tick (the blueprint lesson).
//! Spell names/costs quote wands.rs SPELLS so the numbers can't drift.

use super::super::wands;
use super::craft_tab::CraftState;
use super::{SlideOutUi, PAD, PANEL_W, Z};
use crate::gfx::{at, bake, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings, Pointer};
use crate::inventory::PlayerInv;
use crate::ui::{frame_rect, label};
use crate::{CANVAS_H, SIDEBAR_W};
use bevy::prelude::*;

/// (element, rune item consumed — "" is the bare wand, row title, what the spell does).
const ROWS: &[(&str, &str, &str, &str)] = &[
    ("arcane", "", "BARE WAND", "ARCANE BOLT - 2 MANA"),
    ("fire", "firerune", "EMBER RUNE", "FIREBOLT - 3 MANA - SETS ALL ALIGHT"),
    ("frost", "frostrune", "FROST RUNE", "FROST BEAM - 3 MANA - FREEZES SOLID"),
    ("storm", "stormrune", "STORM RUNE", "SPARK BOLT - 4 MANA - PIERCES THROUGH"),
];

/// One geometry source for draw + mouse hit-testing (the tab_chips rule).
fn row_rect(i: usize) -> (f32, f32, f32, f32) {
    (SIDEBAR_W + PAD, 48.0 + i as f32 * 27.0, PANEL_W - PAD * 2.0, 26.0)
}

/// Cursor + imbue. A FLAT four-row list: hover highlights, click imbues (the menus
/// convention — only SCROLLING lists are click-only).
pub fn actions(
    state: &ActionState,
    ptr: &Pointer,
    inv: &mut PlayerInv,
    rune: &mut wands::WandRune,
    sfx: &mut MessageWriter<super::super::sfx::Sfx>,
    cs: &mut CraftState,
) -> bool {
    let mut dirty = false;
    if state.pressed(Action::Up) && cs.rune_cursor > 0 {
        cs.rune_cursor -= 1;
        dirty = true;
    }
    if state.pressed(Action::Down) && cs.rune_cursor + 1 < ROWS.len() {
        cs.rune_cursor += 1;
        dirty = true;
    }
    let mut go = false;
    for i in 0..ROWS.len() {
        let (x, y, w, h) = row_rect(i);
        if ptr.over(x, y, w, h) {
            if ptr.moved && cs.rune_cursor != i {
                cs.rune_cursor = i;
                dirty = true;
            }
            if ptr.click {
                cs.rune_cursor = i;
                go = true;
            }
        }
    }
    if state.pressed(Action::Slot1) || state.pressed(Action::MenuConfirm) || go {
        let (el, item, ..) = ROWS[cs.rune_cursor];
        if el == rune.0 || (!item.is_empty() && inv.count(item) == 0) {
            sfx.write(super::super::sfx::Sfx("tink")); // already imbued, or no stone to spend
        } else {
            if !item.is_empty() {
                inv.remove_one(item);
            }
            if let Some(old) = wands::element_rune(rune.0) {
                inv.add_item(old, 1); // the old rune pops back into the bag
            }
            rune.0 = el;
            sfx.write(super::super::sfx::Sfx("craft"));
        }
        dirty = true;
    }
    dirty
}

/// The page: header, the socketed element in its own light, the four element rows
/// (icon + name + spell blurb + owned count), the no-wand warning, derived hints.
#[allow(clippy::too_many_arguments)] // it IS the page's arity
pub fn draw(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    cs: &CraftState,
    inv: &PlayerInv,
    rune: &str,
    bindings: &Bindings,
    pad: bool,
) {
    let tag = || SlideOutUi;
    let ax = SIDEBAR_W + PAD;
    label(commands, images, "IMBUE THE WAND", ax, 24.0, 0xfcd000, Z + 1.0, tag());
    let cur = ROWS.iter().find(|r| r.0 == rune).unwrap_or(&ROWS[0]);
    label(commands, images, &format!("SOCKETED - {}", cur.2), ax, 34.0, wands::spell_for(rune).color, Z + 1.0, tag());
    for (i, (el, item, name, blurb)) in ROWS.iter().enumerate() {
        let (x, y, w, h) = row_rect(i);
        let socketed = *el == rune;
        let have = item.is_empty() || inv.count(item) > 0;
        if i == cs.rune_cursor {
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0x23, 0x23, 0x2c), Vec2::new(w, h)),
                at(x, y, w, h, Z + 1.0),
                PIXEL_LAYER,
                tag(),
            ));
            frame_rect(commands, x, y, w, h, 0xfce0a8, Z + 1.05, tag());
        }
        // The rune stone's own icon; the bare row shows the wand itself.
        if let Some(def) = crate::items::get(if item.is_empty() { "wand" } else { item }) {
            let mut icon = Sprite::from_image(images.add(bake(def.icon, def.icon_pal)));
            icon.custom_size = Some(Vec2::splat(16.0));
            if !have && !socketed {
                icon.color = Color::srgba(1.0, 1.0, 1.0, 0.35);
            }
            commands.spawn((icon, at(x + 4.0, y + 5.0, 16.0, 16.0, Z + 1.1), PIXEL_LAYER, tag()));
        }
        let ncol = if socketed || have { wands::spell_for(el).color } else { 0x606068 };
        label(commands, images, name, x + 26.0, y + 5.0, ncol, Z + 1.1, tag());
        label(commands, images, blurb, x + 26.0, y + 14.0, if have { 0x9aa0aa } else { 0x54545c }, Z + 1.1, tag());
        // Right edge: SOCKETED beats a count; a rune you lack reads NONE.
        let edge = if socketed {
            ("SOCKETED".to_string(), 0xcfeeb0)
        } else if item.is_empty() {
            (String::new(), 0)
        } else if inv.count(item) > 0 {
            (format!("X{}", inv.count(item)), 0xfce0a8)
        } else {
            ("NONE".to_string(), 0x54545c)
        };
        if !edge.0.is_empty() {
            let ew = font::measure(&edge.0) as f32;
            label(commands, images, &edge.0, x + w - ew - 4.0, y + 10.0, edge.1, Z + 1.1, tag());
        }
    }
    if !inv.has_item("wand") {
        let wy = row_rect(ROWS.len() - 1).1 + 32.0;
        label(commands, images, "YOU CARRY NO WAND - THIS TABLE CRAFTS ONE", ax, wy, 0xff9a7a, Z + 1.0, tag());
    }
    let hint = format!(
        "{}/{} TABS - {} IMBUE - {} CLOSE",
        bindings.prompt(Action::TabPrev, pad),
        bindings.prompt(Action::TabNext, pad),
        bindings.prompt(Action::Slot1, pad),
        bindings.prompt(Action::Inventory, pad)
    );
    label(commands, images, &hint, ax, CANVAS_H as f32 - 12.0, 0x707070, Z + 1.0, tag());
}
