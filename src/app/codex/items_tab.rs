//! items_tab.rs — the ITEMS dex (js drawItemDex): every item with an icon, sorted rarity
//! then name, revealed once you've HELD it. Grid left, detail pane right (icon on the
//! plate, name, rarity line, description).
//!
//! Discovery: the js adds to `discoveredItems` on pickup. Here a watcher marks every id
//! currently in the inventory whenever it changes — one hook covers pickups, starting
//! gear, and every future acquisition path.

use super::{dex, hint_scaffold, CodexState, CodexUi, TabContent, CONTENT_Z};
use crate::gfx::bake;
use crate::input::{ActionState, Bindings};
use crate::inventory::PlayerInv;
use crate::items;
use crate::ui::label;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

/// Item ids the player has ever held (js discoveredItems — grows, never shrinks).
#[derive(Resource, Default)]
pub struct Discovered(pub HashSet<&'static str>);

/// Mark everything in the inventory as discovered whenever it changes.
pub fn track_discovered(inv: Res<PlayerInv>, mut seen: ResMut<Discovered>) {
    if !inv.is_changed() {
        return;
    }
    for e in &inv.entries {
        seen.0.insert(e.id);
    }
}

/// js itemDexKeys: all icon-bearing defs, rarity ASC then name.
fn keys() -> Vec<&'static items::ItemDef> {
    let mut v: Vec<_> = items::all_defs().collect();
    v.sort_by(|a, b| a.rarity.tier().cmp(&b.rarity.tier()).then(a.name.cmp(b.name)));
    v
}

#[derive(Resource, Default)]
pub struct ItemDex {
    pub cur: usize,
}

#[derive(Component, Clone)]
pub struct ItemsUi;

pub fn hint(bindings: &Bindings, pad: bool) -> String {
    let browse = if pad { "DPAD BROWSE" } else { "ARROWS BROWSE" };
    hint_scaffold(bindings, pad, browse)
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn run(
    mut commands: Commands,
    state: Res<ActionState>,
    cx_state: Res<CodexState>,
    seen: Res<Discovered>,
    mut dex_state: ResMut<ItemDex>,
    mut images: ResMut<Assets<Image>>,
    old: Query<Entity, With<ItemsUi>>,
    mut seen_gen: Local<u32>,
) {
    let ids = keys();
    let mut dirty = *seen_gen != cx_state.generation;
    *seen_gen = cx_state.generation;
    let cur = dex::dex_nav(&state, ids.len(), dex_state.cur, dex::DEX_COLS);
    if cur != dex_state.cur {
        dex_state.cur = cur;
        dirty = true;
    }
    if !dirty {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let tag = || (CodexUi, TabContent, ItemsUi);

    let found = ids.iter().filter(|d| seen.0.contains(d.id)).count();
    let hdr = format!("ITEMS  {found}/{}", ids.len());
    label(&mut commands, &mut images, &hdr, dex::DEX_AX, 16.0, 0xbfb9a0, CONTENT_Z + 0.1, tag());

    // Bake every icon up front (the grid closure can't reach `images` while it's borrowed).
    let icons: Vec<Handle<Image>> = ids.iter().map(|d| images.add(bake(d.icon, d.icon_pal))).collect();
    let unlocked = |i: usize| seen.0.contains(ids[i].id);
    dex::draw_grid(
        &mut commands,
        &mut images,
        ids.len(),
        dex_state.cur,
        dex::DEX_COLS,
        unlocked,
        |i| Some((icons[i].clone(), 8.0)),
        tag(),
    );
    let d = ids[dex_state.cur];
    let open = seen.0.contains(d.id);
    dex::draw_pane(
        &mut commands,
        &mut images,
        open,
        Some((icons[dex_state.cur].clone(), 8.0)),
        &d.name.to_uppercase(),
        Some((d.rarity.name(), d.rarity.color())),
        &d.desc.to_uppercase(),
        true,
        tag(),
    );
}
