//! storage.rs — the HOME STORAGE CHEST (js drawStorage/updateStorage + playerStash):
//! a two-pane bank you open at the chest in your house. Left pane = your BAG, right =
//! the CHEST; A moves the selected WHOLE stack across, LT/RT (or left/right) switch
//! sides, B closes + saves. Unique items can't be stored; stackables merge; the chest
//! caps at STASH_CAP slots. The stash is saved and — at home — feeds the CRAFT page's
//! material pool (craft_tab counts + consumes it too).
//!
//! Its own `Screen::Storage` (like shop.rs): the world freezes underneath, the window
//! owns the face buttons, OnExit sweeps the panel.

use super::screen::Screen;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::inventory::PlayerInv;
use crate::ui::{border_strips, label};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

const W: f32 = 240.0;
const H: f32 = 150.0;
const ROW: f32 = 12.0;
const Z: f32 = crate::gfx::layers::WINDOW;

/// The home chest holds at most this many slots (js STASH_CAP).
pub const STASH_CAP: usize = 24;

/// One stored stack (js playerStash row). `id` is a String so procedural/`~` ids ride
/// the save; it re-resolves to a &'static str on retrieval via items::get.
#[derive(Clone, Serialize, Deserialize)]
pub struct StashEntry {
    pub id: String,
    pub qty: i32,
}

/// The home chest's contents (js playerStash, saved).
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct PlayerStash(pub Vec<StashEntry>);

impl PlayerStash {
    /// Add `qty` of `id` to the chest, merging into a stackable slot when one exists.
    /// Returns false only when a new slot is needed but the chest is full.
    pub fn add(&mut self, id: &str, qty: i32) -> bool {
        let stackable = crate::items::get(id).is_some_and(|d| d.stackable);
        if stackable
            && let Some(e) = self.0.iter_mut().find(|e| e.id == id)
        {
            e.qty += qty;
            return true;
        }
        if self.0.len() >= STASH_CAP {
            return false;
        }
        self.0.push(StashEntry { id: id.to_string(), qty });
        true
    }

    /// Remove ONE of `id` (home crafting draws from the chest). Prunes an emptied slot.
    pub fn remove_one(&mut self, id: &str) -> bool {
        let Some(i) = self.0.iter().position(|e| e.id == id && e.qty > 0) else { return false };
        self.0[i].qty -= 1;
        if self.0[i].qty <= 0 {
            self.0.remove(i);
        }
        true
    }

    /// Total of `id` in the chest (home crafting's have-count).
    pub fn count(&self, id: &str) -> i32 {
        self.0.iter().filter(|e| e.id == id).map(|e| e.qty).sum()
    }
}

/// The open window's cursor state (js stSide/stCur/stScroll).
#[derive(Resource, Default)]
pub struct StorageState {
    pub side: usize,       // 0 = bag, 1 = chest
    pub cursor: [usize; 2],
    pub scroll: [usize; 2],
}

#[derive(Component)]
struct StorageUi;

pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerStash>()
            .init_resource::<StorageState>()
            .add_systems(
                bevy::app::FixedUpdate,
                storage_tick.run_if(in_state(Screen::Storage)).before(super::play::EndTick),
            )
            .add_systems(OnEnter(Screen::Storage), enter_storage)
            .add_systems(OnExit(Screen::Storage), close_storage);
    }
}

/// One BAG row: (id, uid, qty) in slot order (js bagList).
fn bag_list(inv: &PlayerInv) -> Vec<(&'static str, u32, i32)> {
    inv.bag.iter().flatten().filter_map(|uid| inv.entry(*uid)).map(|e| (e.id, e.uid, e.qty)).collect()
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn enter_storage(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut st: ResMut<StorageState>,
    stash: Res<PlayerStash>,
    inv: Res<PlayerInv>,
    bindings: Res<Bindings>,
    state: Res<ActionState>,
    old: Query<Entity, With<StorageUi>>,
) {
    // Open fresh on the bag side (js: stSide=0, stCur=[0,0], stScroll=[0,0]).
    *st = StorageState::default();
    redraw(&mut commands, &mut images, &st, &stash, &inv, &bindings, &state, &old);
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn storage_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<ActionState>,
    bindings: Res<Bindings>,
    mut next: ResMut<NextState<Screen>>,
    mut st: ResMut<StorageState>,
    mut stash: ResMut<PlayerStash>,
    mut inv: ResMut<PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    old: Query<Entity, With<StorageUi>>,
    ptr: Res<crate::input::Pointer>,
) {
    // The window owns the face buttons while open (the heldLatch rule).
    for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
        state.latch(a);
    }
    if state.pressed(Action::Slot2) || state.pressed(Action::Pause) {
        sfx.write(super::sfx::Sfx("open"));
        saves.write(super::save::SaveRequest);
        next.set(Screen::Play);
        return;
    }
    let mut dirty = false;
    if state.pressed(Action::TabNext) || state.pressed(Action::TabPrev) || state.pressed(Action::Left) || state.pressed(Action::Right) {
        st.side ^= 1;
        sfx.write(super::sfx::Sfx("menuMove"));
        dirty = true;
    }
    let side = st.side;
    let n = if side == 0 { bag_list(&inv).len() } else { stash.0.len() };
    if state.pressed(Action::Up) && st.cursor[side] > 0 {
        st.cursor[side] -= 1;
        dirty = true;
    }
    if state.pressed(Action::Down) && st.cursor[side] + 1 < n.max(1) {
        st.cursor[side] += 1;
        dirty = true;
    }
    if ptr.wheel_steps != 0 {
        // Wheel walks the active pane (Baz: any scrollable list honours the wheel).
        st.cursor[side] = (st.cursor[side] as i32 - ptr.wheel_steps).clamp(0, n.max(1) as i32 - 1) as usize;
        dirty = true;
    }
    // Mouse: the panes SCROLL, so hover does nothing — a click selects a cell (switching
    // sides if needed), clicking the selected cell transfers the stack.
    let mut cell_click = false;
    if ptr.click {
        use super::room_render::{PLAY_X, PLAY_Y};
        use crate::room::{PX_H, PX_W};
        let x = PLAY_X + ((PX_W as f32 - W) / 2.0).round();
        let y = PLAY_Y + ((PX_H as f32 - H) / 2.0).round();
        let col_w = ((W - 18.0) / 2.0).floor();
        let top = y + 28.0;
        let vis = (((y + H - 12.0 - top) / ROW) as usize).max(1);
        let lens = [bag_list(&inv).len(), stash.0.len()];
        for (s, &pane_len) in lens.iter().enumerate() {
            let cx = x + 6.0 + s as f32 * (col_w + 6.0);
            let cur = st.cursor[s].min(pane_len.saturating_sub(1));
            let scroll = st.scroll[s].min(cur).max((cur + 1).saturating_sub(vis)).min(pane_len.saturating_sub(vis));
            for v in 0..vis {
                if scroll + v >= pane_len {
                    break;
                }
                if ptr.over(cx - 1.0, top + v as f32 * ROW - 1.0, col_w, ROW - 1.0) {
                    if st.side != s || st.cursor[s] != scroll + v {
                        st.side = s;
                        st.cursor[s] = scroll + v;
                        dirty = true;
                    } else {
                        cell_click = true;
                    }
                }
            }
        }
    }
    let cur_n = if st.side == 0 { bag_list(&inv).len() } else { stash.0.len() };
    if (state.pressed(Action::Slot1) || cell_click) && cur_n > 0 {
        dirty |= transfer(&mut st, &mut stash, &mut inv, &mut log, &mut sfx);
        saves.write(super::save::SaveRequest);
    }
    if dirty {
        redraw(&mut commands, &mut images, &st, &stash, &inv, &bindings, &state, &old);
    }
}

/// Move the selected WHOLE stack across (js updateStorage's confirm branch).
fn transfer(
    st: &mut StorageState,
    stash: &mut PlayerStash,
    inv: &mut PlayerInv,
    log: &mut super::rewards::LootLog,
    sfx: &mut MessageWriter<super::sfx::Sfx>,
) -> bool {
    let side = st.side;
    if side == 0 {
        // BAG -> CHEST (whole stack).
        let bag = bag_list(inv);
        let Some(&(id, _, _)) = bag.get(st.cursor[0]) else {
            sfx.write(super::sfx::Sfx("tink"));
            return false;
        };
        if crate::items::get(id).is_some_and(|d| d.unique) {
            log.add("store", "CAN'T STORE", 1, 0xfc6868, false, true);
            sfx.write(super::sfx::Sfx("tink"));
            return false;
        }
        let stackable = crate::items::get(id).is_some_and(|d| d.stackable);
        let room = (stackable && stash.0.iter().any(|s| s.id == id)) || stash.0.len() < STASH_CAP;
        if !room {
            log.add("store", "CHEST FULL", 1, 0xfc6868, false, true);
            sfx.write(super::sfx::Sfx("tink"));
            return false;
        }
        let qty = inv.remove_stack(id);
        stash.add(id, qty);
        sfx.write(super::sfx::Sfx("craft"));
    } else {
        // CHEST -> BAG (whole stack).
        let Some(e) = stash.0.get(st.cursor[1]).cloned() else {
            sfx.write(super::sfx::Sfx("tink"));
            return false;
        };
        let Some(sid) = crate::items::get(&e.id).map(|d| d.id) else {
            // An id that no longer resolves (a since-removed item) — drop the dead slot.
            stash.0.remove(st.cursor[1]);
            return true;
        };
        if inv.add_item(sid, e.qty) {
            stash.0.remove(st.cursor[1]);
            sfx.write(super::sfx::Sfx("craft"));
        } else {
            log.add("store", "BAG FULL", 1, 0xfc6868, false, true);
            sfx.write(super::sfx::Sfx("tink"));
            return false;
        }
    }
    let m = if side == 0 { bag_list(inv).len() } else { stash.0.len() };
    st.cursor[side] = st.cursor[side].min(m.saturating_sub(1));
    true
}

fn close_storage(mut commands: Commands, old: Query<Entity, With<StorageUi>>) {
    for e in &old {
        commands.entity(e).despawn();
    }
}

fn fill(commands: &mut Commands, x: f32, y: f32, w: f32, h: f32, color: Color, z: f32) {
    commands.spawn((Sprite::from_color(color, Vec2::new(w, h)), at(x, y, w, h, z), PIXEL_LAYER, StorageUi));
}

/// The full two-pane window (js drawStorage), rebuilt whenever the side/cursor/lists change.
#[allow(clippy::too_many_arguments)] // a full-window draw's arity
fn redraw(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    st: &StorageState,
    stash: &PlayerStash,
    inv: &PlayerInv,
    bindings: &Bindings,
    state: &ActionState,
    old: &Query<Entity, With<StorageUi>>,
) {
    use super::room_render::{PLAY_X, PLAY_Y};
    use crate::room::{PX_H, PX_W};
    for e in old {
        commands.entity(e).despawn();
    }
    let x = PLAY_X + ((PX_W as f32 - W) / 2.0).round();
    let y = PLAY_Y + ((PX_H as f32 - H) / 2.0).round();
    fill(commands, x, y, W, H, Color::srgba(0.0, 0.0, 0.0, 0.93), Z);
    for (sx, sy, sw, sh) in border_strips(x, y, W, H, 1.0) {
        fill(commands, sx, sy, sw, sh, Color::srgb_u8(0xca, 0xa8, 0x4a), Z + 0.01);
    }
    label(commands, images, "STORAGE CHEST", x + 6.0, y + 5.0, 0xfce0a8, Z + 0.04, StorageUi);

    let bag = bag_list(inv);
    // (id, qty) per pane.
    let panes: [Vec<(&'static str, i32)>; 2] = [
        bag.iter().map(|(id, _, q)| (*id, *q)).collect(),
        stash.0.iter().filter_map(|e| crate::items::get(&e.id).map(|d| (d.id, e.qty))).collect(),
    ];
    let titles = ["BAG", "CHEST"];
    let col_w = ((W - 18.0) / 2.0).floor();
    let top = y + 28.0;
    let vis = (((y + H - 12.0 - top) / ROW) as usize).max(1);

    for s in 0..2 {
        let cx = x + 6.0 + s as f32 * (col_w + 6.0);
        let list = &panes[s];
        label(commands, images, titles[s], cx, y + 17.0, if s == st.side { 0xfce0a8 } else { 0x8a8a8a }, Z + 0.04, StorageUi);
        let cursor = st.cursor[s].min(list.len().saturating_sub(1));
        let scroll = st.scroll[s].min(cursor).max((cursor + 1).saturating_sub(vis)).min(list.len().saturating_sub(vis));
        if list.is_empty() {
            label(commands, images, "EMPTY", cx + 2.0, top + 2.0, 0x666666, Z + 0.04, StorageUi);
        }
        for v in 0..vis {
            let Some(&(id, qty)) = list.get(scroll + v) else { break };
            let ry = top + v as f32 * ROW;
            if s == st.side && scroll + v == cursor {
                fill(commands, cx - 1.0, ry - 1.0, col_w, ROW - 1.0, Color::srgb_u8(0x1c, 0x1c, 0x24), Z + 0.02);
                for (bx, by, bw, bh) in border_strips(cx - 1.0, ry - 1.0, col_w, ROW - 1.0, 1.0) {
                    fill(commands, bx, by, bw, bh, Color::srgb_u8(0xfc, 0xe0, 0xa8), Z + 0.03);
                }
            }
            if let Some(def) = crate::items::get(id) {
                let mut icon = Sprite::from_image(images.add(crate::gfx::bake(def.icon, def.icon_pal)));
                icon.custom_size = Some(Vec2::splat(10.0));
                commands.spawn((icon, at(cx + 1.0, ry, 10.0, 10.0, Z + 0.04), PIXEL_LAYER, StorageUi));
            }
            let mut nm = crate::items::get(id).map_or(id, |d| d.name).to_uppercase();
            if nm.len() > 11 {
                nm.truncate(11);
            }
            label(commands, images, &nm, cx + 13.0, ry + 1.0, 0xdcdce0, Z + 0.04, StorageUi);
            if qty > 1 {
                let q = format!("x{qty}");
                let qw = font::measure(&q) as f32;
                label(commands, images, &q, cx + col_w - 2.0 - qw, ry + 1.0, 0xfcfcfc, Z + 0.04, StorageUi);
            }
        }
    }
    let pad = state.pad_present;
    let hint = format!(
        "{} MOVE - {}/{} SIDE - {} CLOSE",
        bindings.prompt(Action::Slot1, pad),
        bindings.prompt(Action::TabPrev, pad),
        bindings.prompt(Action::TabNext, pad),
        bindings.prompt(Action::Slot2, pad),
    );
    let hw = font::measure(&hint) as f32;
    label(commands, images, &hint, x + ((W - hw) / 2.0).round(), y + H - 9.0, 0xa0a0a0, Z + 0.04, StorageUi);
}
