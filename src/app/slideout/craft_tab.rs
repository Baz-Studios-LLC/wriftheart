//! craft_tab.rs — the slide-out CRAFT page (js inventory.js drawCraft/updateCraft/doCraft
//! over the shared drawCraftWindow): the HAND recipes — a scrollable list on the left, the
//! selected recipe's detail on the right (icon, description, materials with have/need
//! counts, the A CRAFT button), and the floating "+1 NAME" / "BAG FULL" banner.
//!
//! Not here yet: recipe PINS, the home-chest material pool, blueprint locks, station
//! windows (workbench/forge) — each joins with its system.

use super::{SlideOut, SlideOutUi, PANEL_W, PAD, Z};
use crate::app::stats::Stats;
use crate::gfx::{at, bake, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::inventory::PlayerInv;
use super::super::storage::PlayerStash;
use crate::items::{self, Recipe};
use crate::skilltree;
use crate::ui::{frame_rect, label};
use crate::{CANVAS_H, SIDEBAR_W};
use bevy::prelude::*;

/// Recipe output-ids pinned to the top of the craft list (js pinnedRecipes, saved).
#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct PinnedRecipes(pub std::collections::HashSet<String>);

/// The craft list as SHOWN: pinned recipes float to the top (js shownRecipes — the
/// stable sort keeps registry order within each group).
pub fn shown(station: &str, learned: &std::collections::HashSet<String>, pins: &PinnedRecipes) -> Vec<&'static Recipe> {
    let mut list = items::recipes_for(station, learned);
    list.sort_by_key(|r| !pins.0.contains(r.out));
    list
}

/// Page state (js craftCursor/craftScroll/craftFlash).
#[derive(Resource, Default)]
pub struct CraftState {
    pub cursor: usize,
    pub scroll: usize,
    pub flash: u32,
    pub flash_msg: String,
    /// Some(station) while the page is opened AT a placed station (js craftStation) —
    /// the list swaps to that station's recipes; cleared when the slide-out closes.
    pub station: Option<&'static str>,
    /// The opened station's room-px anchor — the REMOVE action needs to know WHICH one.
    pub station_at: Option<(f32, f32)>,
    /// Slot4 pressed in station mode: cooking.rs's station_remove tears it down for half
    /// the materials back (js removeTable).
    pub remove_requested: bool,
    /// The enchanter's RUNES page cursor (runes_tab.rs).
    pub rune_cursor: usize,
}

#[derive(Component, Clone)]
pub struct CraftUi;

/// js canCraft: every cost line covered by the bag (+ the home chest, when crafting
/// AT HOME — js homeCraft draws from playerStash too).
fn can_craft(inv: &PlayerInv, stash: &PlayerStash, home: bool, god: bool, r: &Recipe) -> bool {
    god || r.cost.iter().all(|(id, q)| have_of(inv, stash, home, id) >= *q)
}

/// A cost line's have-count — bag, plus the home chest when at home. "@FISH" is the
/// any-fish wildcard (js cook recipes; the chest doesn't feed the wildcard).
fn have_of(inv: &PlayerInv, stash: &PlayerStash, home: bool, id: &str) -> i32 {
    if id == "@FISH" {
        inv.count_fish()
    } else {
        inv.count(id) + if home { stash.count(id) } else { 0 }
    }
}

/// js doCraft: consume the materials (the tree's CRAFT stat may spare some), add the
/// output, raise the banner.
#[allow(clippy::too_many_arguments)] // the craft context is wide
fn do_craft(
    inv: &mut PlayerInv,
    stash: &mut PlayerStash,
    home: bool,
    stats: &mut Stats,
    alloc: &super::TreeAlloc,
    rng: &mut impl FnMut() -> f64,
    cs: &mut CraftState,
    god: bool,
    r: &Recipe,
) {
    if !can_craft(inv, stash, home, god, r) {
        return;
    }
    // A forge COMMISSION (js craftGen): the preview isn't granted — roll a fresh
    // procedural item of that class + tier and grant THAT instead.
    let out_id: &'static str = match items::get(r.out).and_then(|d| d.craft_gen) {
        Some(cg) => {
            let kind = if cg.armor { crate::procgen::Kind::Armor } else { crate::procgen::Kind::Weapon };
            let entropy = (rng() * 4_294_967_296.0) as u32;
            crate::procgen::generate_pinned(kind, cg.base, cg.tier, entropy)
        }
        None => r.out,
    };
    let out = items::get(out_id);
    if !inv.can_add(out_id) {
        cs.flash = 72;
        cs.flash_msg = if out.is_some_and(|d| d.unique) && inv.has_item(out_id) {
            "YOU ALREADY CARRY ONE".into()
        } else {
            "BAG FULL".into()
        };
        return;
    }
    // Crafting tree: a chance to NOT consume each material; spared ones come back.
    // GOD MODE pays nothing (Baz).
    let craft_save = skilltree::stat(&alloc.taken, "craft");
    let mut saved: Vec<&'static str> = Vec::new();
    for (id, q) in r.cost.iter().filter(|_| !god) {
        for _ in 0..*q {
            if *id == "@FISH" {
                // The wildcard eats the cheapest fish — and the CRAFT save can't
                // return "a fish", so wildcard lines are never spared.
                inv.remove_cheapest_fish();
                continue;
            }
            // Bag first, then the home chest for the remainder (js homeCraft/stashRemoveOne).
            if inv.remove_one(id) {
                if craft_save > 0.0 && rng() < craft_save {
                    saved.push(id);
                }
            } else if home {
                stash.remove_one(id); // chest draws are never spared (js can't return a stashed unit)
            }
        }
    }
    for id in saved {
        inv.add_item(id, 1);
    }
    stats.bump("crafts", 1.0); // the ledger + (later) the Maker awards
    inv.add_item(out_id, r.n);
    cs.flash = 72;
    cs.flash_msg = format!("+{} {}", r.n, out.map_or(out_id, |d| d.name).to_uppercase());
}

/// Button handling for the CRAFT page (js updateCraft, minus pins). Returns dirty.
#[allow(clippy::too_many_arguments)] // the craft page's context is wide
pub fn actions(
    state: &ActionState,
    inv: &mut PlayerInv,
    stash: &mut PlayerStash,
    home: bool,
    god: bool,
    stats: &mut Stats,
    alloc: &super::TreeAlloc,
    rng: &mut impl FnMut() -> f64,
    cs: &mut CraftState,
    learned: &std::collections::HashSet<String>,
    pins: &mut PinnedRecipes,
    ptr: &crate::input::Pointer,
) -> bool {
    let mut dirty = false;
    if cs.flash > 0 {
        cs.flash -= 1;
        dirty = true; // the banner floats + fades every tick
    }
    let recipes = shown(cs.station.unwrap_or("hand"), learned, pins);
    if recipes.is_empty() {
        return dirty;
    }
    if cs.cursor >= recipes.len() {
        cs.cursor = recipes.len() - 1;
    }
    if state.pressed(Action::Up) {
        cs.cursor = (cs.cursor + recipes.len() - 1) % recipes.len();
        dirty = true;
    }
    if state.pressed(Action::Down) {
        cs.cursor = (cs.cursor + 1) % recipes.len();
        dirty = true;
    }
    // Slot3 = PIN/unpin the cursor's recipe (js): the cursor FOLLOWS it as the list
    // re-sorts, so pinning doesn't teleport your selection.
    if state.pressed(Action::Slot3) {
        let out = recipes[cs.cursor].out;
        if !pins.0.remove(out) {
            pins.0.insert(out.to_string());
        }
        let after = shown(cs.station.unwrap_or("hand"), learned, pins);
        cs.cursor = after.iter().position(|r| r.out == out).unwrap_or(0);
        dirty = true;
    }
    // Mouse: the list SCROLLS, so hover does nothing — a CLICK selects a recipe; the CRAFT
    // button is the explicit act. Rects mirror `draw`; scroll is draw's last clamp.
    let (ax, ay, aw) = (SIDEBAR_W + PAD, 20.0, PANEL_W - PAD * 2.0);
    let ah = CANVAS_H as f32 - ay - 4.0;
    let lw = (aw * 0.52).round().min(154.0);
    let row = 14.0;
    let vis = (((ah - 12.0) / row).floor() as usize).max(1);
    let sc = cs.scroll;
    for v in 0..vis {
        if sc + v >= recipes.len() {
            break;
        }
        if ptr.click && ptr.over(ax, ay + 1.0 + v as f32 * row, lw, row - 1.0) && cs.cursor != sc + v {
            cs.cursor = sc + v;
            dirty = true;
        }
    }
    let (dx, bh) = (ax + lw + 8.0, 13.0);
    let dw = ax + aw - dx;
    let craft_click = ptr.click && ptr.over(dx, ay + ah - bh - 2.0, dw, bh);
    if state.pressed(Action::Slot1) || craft_click {
        do_craft(inv, stash, home, stats, alloc, rng, cs, god, recipes[cs.cursor]);
        dirty = true;
    }
    // Slot4 at a PLACED station = REMOVE TABLE (js inventory slot4 -> removeTable):
    // cooking.rs's station_remove tears it down + refunds half the mats.
    if cs.station.is_some() && cs.station_at.is_some() && state.pressed(Action::Slot4) {
        cs.remove_requested = true;
        dirty = true;
    }
    dirty
}

/// The shared crafting window (js drawCraftWindow), drawn for the 'hand' station.
#[allow(clippy::too_many_arguments)] // it IS the page's arity
pub fn draw(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    cs: &mut CraftState,
    inv: &PlayerInv,
    bindings: &Bindings,
    pad: bool,
    _so: &SlideOut,
    learned: &std::collections::HashSet<String>,
    pins: &PinnedRecipes,
    stash: &PlayerStash,
    home: bool,
    god: bool,
) {
    let tag = || (SlideOutUi, CraftUi);
    // The content rect (js `a`): below the tab strip, panel-padded.
    let ax = SIDEBAR_W + PAD;
    let ay = 20.0;
    let aw = PANEL_W - PAD * 2.0;
    let ah = CANVAS_H as f32 - ay - 4.0;

    let recipes = shown(cs.station.unwrap_or("hand"), learned, pins);
    let lw = (aw * 0.52).round().min(154.0); // left-list width (js LW)
    let row = 14.0;
    let list_h = ah - 12.0;
    let vis = ((list_h / row).floor() as usize).max(1);
    // Keep the selection in the window (js scroll clamp).
    let mut sc = cs.scroll;
    if cs.cursor < sc {
        sc = cs.cursor;
    }
    if cs.cursor >= sc + vis {
        sc = cs.cursor - vis + 1;
    }
    sc = sc.min(recipes.len().saturating_sub(vis));
    cs.scroll = sc;

    // --- LEFT: the recipe list ---
    for v in 0..vis {
        let Some(r) = recipes.get(sc + v) else { break };
        let ry = ay + 1.0 + v as f32 * row;
        let sel = sc + v == cs.cursor;
        let ok = can_craft(inv, stash, home, god, r);
        let out = items::get(r.out);
        if sel {
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0x23, 0x23, 0x2c), Vec2::new(lw, row - 1.0)),
                at(ax, ry, lw, row - 1.0, Z + 1.0),
                PIXEL_LAYER,
                tag(),
            ));
            frame_rect(commands, ax, ry, lw, row - 1.0, 0xfce0a8, Z + 1.05, tag());
        }
        if let Some(out) = out {
            let mut icon = Sprite::from_image(images.add(bake(out.icon, out.icon_pal)));
            icon.custom_size = Some(Vec2::splat(11.0));
            commands.spawn((icon, at(ax + 2.0, ry + 1.0, 11.0, 11.0, Z + 1.1), PIXEL_LAYER, tag()));
        }
        let color = if sel { 0xfcfcfc } else if ok { 0xbcbcbc } else { 0x787878 };
        label(commands, images, &out.map_or(r.out, |d| d.name).to_uppercase(), ax + 16.0, ry + 3.0, color, Z + 1.1, tag());
        if pins.0.contains(r.out) {
            // The little cyan tack (js pin marker): head + point.
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0x7e, 0xc8, 0xe0), Vec2::new(4.0, 3.0)),
                at(ax + lw - 8.0, ry + 3.0, 4.0, 3.0, Z + 1.15),
                PIXEL_LAYER,
                tag(),
            ));
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0x7e, 0xc8, 0xe0), Vec2::new(2.0, 3.0)),
                at(ax + lw - 7.0, ry + 6.0, 2.0, 3.0, Z + 1.15),
                PIXEL_LAYER,
                tag(),
            ));
        }
    }
    if recipes.len() > vis {
        // Scrollbar in the list gutter.
        let th = (list_h * vis as f32 / recipes.len() as f32).max(6.0);
        let ty = ay + 1.0 + ((list_h - th) * sc as f32 / (recipes.len() - vis) as f32).round();
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(0x20, 0x20, 0x28), Vec2::new(1.0, list_h)),
            at(ax + lw - 2.0, ay + 1.0, 1.0, list_h, Z + 1.0),
            PIXEL_LAYER,
            tag(),
        ));
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(0x6c, 0x6c, 0x78), Vec2::new(1.0, th)),
            at(ax + lw - 2.0, ty, 1.0, th, Z + 1.05),
            PIXEL_LAYER,
            tag(),
        ));
    }
    // Divider.
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x26, 0x26, 0x2e), Vec2::new(1.0, ah - 4.0)),
        at(ax + lw + 2.0, ay, 1.0, ah - 4.0, Z + 1.0),
        PIXEL_LAYER,
        tag(),
    ));

    // --- RIGHT: the selected recipe's detail ---
    let dx = ax + lw + 8.0;
    let dw = ax + aw - dx;
    let Some(r) = recipes.get(cs.cursor) else {
        label(commands, images, "NOTHING TO CRAFT", dx, ay + 20.0, 0x787878, Z + 1.1, tag());
        return;
    };
    let out = items::get(r.out);
    let ok = can_craft(inv, stash, home, god, r);
    if let Some(out) = out {
        let mut icon = Sprite::from_image(images.add(bake(out.icon, out.icon_pal)));
        icon.custom_size = Some(Vec2::splat(24.0));
        commands.spawn((icon, at(dx, ay + 3.0, 24.0, 24.0, Z + 1.1), PIXEL_LAYER, tag()));
        for (i, ln) in super::char_tab::wrap_text(&out.name.to_uppercase(), dw - 28.0).iter().take(2).enumerate() {
            label(commands, images, ln, dx + 28.0, ay + 6.0 + i as f32 * 9.0, out.rarity.color(), Z + 1.1, tag());
        }
    }
    let mut yy = ay + 36.0;
    if let Some(out) = out {
        let lines = super::char_tab::wrap_text(&out.desc.to_uppercase(), dw);
        for (i, ln) in lines.iter().take(3).enumerate() {
            label(commands, images, ln, dx, yy + i as f32 * 8.0, 0x9a9aa0, Z + 1.1, tag());
        }
        yy += lines.len().min(3) as f32 * 8.0;
    }
    yy += 14.0;
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x26, 0x26, 0x2e), Vec2::new(dw, 1.0)),
        at(dx, yy - 7.0, dw, 1.0, Z + 1.0),
        PIXEL_LAYER,
        tag(),
    ));
    label(commands, images, "MATERIALS", dx, yy, 0x9a9a9a, Z + 1.1, tag());
    yy += 14.0;
    for (id, q) in r.cost {
        // "@FISH" shows the shared silhouette + the whole bag's fish count.
        let def = if *id == "@FISH" { items::get("minnow") } else { items::get(id) };
        let have = have_of(inv, stash, home, id);
        if let Some(def) = def {
            let mut icon = Sprite::from_image(images.add(bake(def.icon, def.icon_pal)));
            icon.custom_size = Some(Vec2::splat(11.0));
            commands.spawn((icon, at(dx, yy - 1.0, 11.0, 11.0, Z + 1.1), PIXEL_LAYER, tag()));
        }
        let cost_name = if *id == "@FISH" { "ANY FISH" } else { def.map_or(*id, |d| d.name) };
        label(commands, images, &cost_name.to_uppercase(), dx + 14.0, yy + 1.0, 0xc0c0c0, Z + 1.1, tag());
        let cnt = format!("{have} / {q}");
        let cw = font::measure(&cnt) as f32;
        label(commands, images, &cnt, dx + dw - cw, yy + 1.0, if have >= *q { 0x7ec850 } else { 0xc85050 }, Z + 1.1, tag());
        yy += 13.0;
    }
    // The craft button, bottom of the pane.
    let bh = 13.0;
    let by = ay + ah - bh - 2.0;
    let (fill, border, text_col) = if ok {
        (0x243a1a, 0x7ec850, 0xcfeeb0)
    } else {
        (0x241c1c, 0x5a4444, 0x8a7070)
    };
    commands.spawn((
        Sprite::from_color(Color::srgb_u8((fill >> 16) as u8, (fill >> 8) as u8, fill as u8), Vec2::new(dw, bh)),
        at(dx, by, dw, bh, Z + 1.0),
        PIXEL_LAYER,
        tag(),
    ));
    frame_rect(commands, dx, by, dw, bh, border, Z + 1.05, tag());
    let lbl = format!("{} CRAFT", bindings.prompt(Action::Slot1, pad));
    let lw2 = font::measure(&lbl) as f32;
    label(commands, images, &lbl, (dx + dw / 2.0 - lw2 / 2.0).round(), by + 4.0, text_col, Z + 1.1, tag());
    // The floating confirmation banner (js craftFlash: rises + fades above the button).
    if cs.flash > 0 {
        let age = (72 - cs.flash) as f32;
        let rise = (age * 0.45).min(9.0).round();
        let warn = cs.flash_msg == "BAG FULL" || cs.flash_msg == "YOU ALREADY CARRY ONE";
        let a = (cs.flash as f32 / 30.0).min(1.0);
        let tw = font::measure(&cs.flash_msg) as f32;
        let pw = tw + 10.0;
        let px = (dx + (dw - pw) / 2.0).round();
        let py = by - 15.0 - rise;
        commands.spawn((
            Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.65 * a), Vec2::new(pw, 11.0)),
            at(px, py - 2.0, pw, 11.0, Z + 1.2),
            PIXEL_LAYER,
            tag(),
        ));
        let (img, bw) = font::bake_text(&cs.flash_msg, if warn { 0xff9a7a } else { 0xcfeeb0 }, images);
        let iw = (bw + (bw & 1)) as f32;
        let mut text = Sprite::from_image(img);
        text.color = Color::srgba(1.0, 1.0, 1.0, a);
        commands.spawn((text, at(px + 5.0, py, iw, 6.0, Z + 1.25), PIXEL_LAYER, tag()));
    }
    // Footer hint (js drawCraft, minus PIN until pins port). At a placed station, Slot4
    // offers the js REMOVE TABLE (half mats back).
    let hint = if cs.station.is_some() && cs.station_at.is_some() {
        format!(
            "{}/{} TABS - {} PIN - {} REMOVE - {} CLOSE",
            bindings.prompt(Action::TabPrev, pad),
            bindings.prompt(Action::TabNext, pad),
            bindings.prompt(Action::Slot3, pad),
            bindings.prompt(Action::Slot4, pad),
            bindings.prompt(Action::Inventory, pad)
        )
    } else {
        format!(
            "{}/{} TABS - {} PIN - {} CLOSE",
            bindings.prompt(Action::TabPrev, pad),
            bindings.prompt(Action::TabNext, pad),
            bindings.prompt(Action::Slot3, pad),
            bindings.prompt(Action::Inventory, pad)
        )
    };
    label(commands, images, &hint, ax, CANVAS_H as f32 - 12.0, 0x707070, Z + 1.0, tag());
}
