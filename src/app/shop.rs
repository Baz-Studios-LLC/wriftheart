//! shop.rs — the vendor BUY/SELL window (js drawShop/updateShop/buyItem/sellItem).
//!
//! Opened at a shop counter (services.rs). BUY lists the vendor's shelf — the stable,
//! location-seeded selection from stock.rs minus the sold-out ledgers ([`BoughtShop`]:
//! one-of-a-kinds gone forever, staples back at dawn). SELL lists your bag at 40% of
//! list price, one unit per confirm. LT/RT switch tabs; prices tint by coin metal, and
//! a befriended keeper's rates show as the FRIEND/CONFIDANT RATES tag.
//!
//! Its own `Screen::Shop` state (improve-don't-copy: js `shopOpen` flag) — the world
//! freezes underneath, the window owns the inputs, OnExit sweeps the panel.

use super::screen::Screen;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::inventory::PlayerInv;
use crate::stock::ShopEntry;
use crate::ui::{border_strips, label};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

const W: f32 = 264.0; // two panes: the list + the ware's details (Baz)
const H: f32 = 150.0;
const LIST_W: f32 = 158.0; // the left pane; the divider sits on its edge
const ROW: f32 = 13.0;
const Z: f32 = crate::gfx::layers::WINDOW;

/// The open window: the working shelf + cursor state (js shopStock/shopTab/shopCursor).
#[derive(Resource)]
pub struct ShopState {
    pub stock: Vec<ShopEntry>,
    pub key: String,
    pub discount: f64, // the keeper's rate (1.0 = list price) — drives the FRIEND RATES tag
    pub tab: usize, // 0 BUY / 1 SELL
    pub cursor: usize,
    pub scroll: usize,
}

impl Default for ShopState {
    fn default() -> Self {
        Self { stock: Vec::new(), key: String::new(), discount: 1.0, tab: 0, cursor: 0, scroll: 0 }
    }
}

/// Sold-out ledgers (both saved). DEVIATES from the js boughtShop (which never
/// restocks ANYTHING): one-of-a-kind wares (gear, trinkets, blueprints) stay sold out
/// FOREVER, but staples (stackable materials/consumables) RESTOCK AT DAWN with the
/// rest of the world (Baz, 2026-07-16).
#[derive(Resource, Default)]
pub struct BoughtShop {
    /// shop key -> one-of-a-kind ids bought there, gone for good.
    pub forever: HashMap<String, Vec<String>>,
    /// shop key -> staple ids bought there TODAY (stale entries ignore at dawn).
    pub today: HashMap<String, Vec<String>>,
    /// The dawn-day `today` belongs to.
    pub day: i64,
}

/// Does this ware come back tomorrow? Staples restock; one-of-a-kinds don't.
fn restocks(id: &str) -> bool {
    crate::items::get(id).is_some_and(|d| d.stackable && !d.unique)
}

#[derive(Component)]
struct ShopUi;

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShopState>()
            .init_resource::<BoughtShop>()
            .add_systems(
                bevy::app::FixedUpdate,
                shop_tick.run_if(in_state(Screen::Shop)).before(super::play::EndTick),
            )
            .add_systems(OnEnter(Screen::Shop), enter_shop)
            .add_systems(OnExit(Screen::Shop), close_shop);
    }
}

/// The KEEPER's goodwill (js keeperDiscount): FRIEND (3+ hearts) pays 95%, CONFIDANT
/// (7+) pays 85%. 1.0 = list price.
pub fn keeper_discount(inside: &super::interior::InsideState, people: &super::talk::PeopleLedger) -> f64 {
    let h = inside
        .keeper_key
        .as_ref()
        .and_then(|k| people.0.get(k))
        .map_or(0, |r| crate::people::hearts(r.pts));
    if h >= 7 {
        0.85
    } else if h >= 3 {
        0.95
    } else {
        1.0
    }
}

/// Build the shelf for the vendor the player is inside (js enterInterior's stock block):
/// the location-seeded selection minus this shop's sold-out ledgers, at the keeper's
/// rates. Call before setting `Screen::Shop`. Wild roadside storefronts carry a
/// scavenged cross-category haul.
#[allow(clippy::too_many_arguments)] // the shelf's full context
pub fn stock_up(
    shop: &mut ShopState,
    inside: &super::interior::InsideState,
    bought: &BoughtShop,
    people: &super::talk::PeopleLedger,
    rx: i32,
    ry: i32,
    today: i64,
) {
    let Some(key) = &inside.shop_key else { return };
    let zt = crate::worldgen::world::World::zone_tier(rx, ry);
    let mut stock = if inside.def.kind == "shop" {
        crate::stock::wild_stock(inside.iseed, zt, 0.0)
    } else {
        crate::stock::shop_stock(inside.def.stock, inside.iseed, zt)
    };
    let kd = keeper_discount(inside, people);
    if kd < 1.0 {
        for e in &mut stock {
            e.price = ((e.price as f64 * kd).ceil() as i32).max(1);
        }
    }
    let gone = bought.forever.get(key);
    let sold_today = bought.today.get(key).filter(|_| bought.day == today);
    shop.stock = stock
        .into_iter()
        .filter(|e| {
            !gone.is_some_and(|s| s.iter().any(|x| x == e.id))
                && !sold_today.is_some_and(|s| s.iter().any(|x| x == e.id))
        })
        .collect();
    shop.key = key.clone();
    shop.discount = kd;
    shop.tab = 0;
    shop.cursor = 0;
    shop.scroll = 0;
}

/// Open a roadside caravan's shelf (js caravanStock) straight into ShopState — no
/// interior, no keeper discount. Its sold-today ledger keys on the wagon's room.
pub fn open_caravan(shop: &mut ShopState, bought: &BoughtShop, rx: i32, ry: i32, seed: u32, today: i64) {
    let zt = crate::worldgen::world::World::zone_tier(rx, ry);
    let stock = crate::stock::caravan_stock(seed, zt);
    let key = format!("caravan:{rx},{ry}");
    let gone = bought.forever.get(&key);
    let sold_today = bought.today.get(&key).filter(|_| bought.day == today);
    shop.stock = stock
        .into_iter()
        .filter(|e| {
            !gone.is_some_and(|s| s.iter().any(|x| x == e.id))
                && !sold_today.is_some_and(|s| s.iter().any(|x| x == e.id))
        })
        .collect();
    shop.key = key;
    shop.discount = 1.0;
    shop.tab = 0;
    shop.cursor = 0;
    shop.scroll = 0;
}

/// One sellable bag row: (uid, id, qty, unit price) — js sellList (bag only; gear and
/// equipped slots stay off the table).
fn sell_list(inv: &PlayerInv, clock: i64, fish_mult: f32) -> Vec<(u32, &'static str, i32, i32)> {
    inv.bag
        .iter()
        .flatten()
        .filter_map(|uid| inv.entry(*uid))
        .map(|e| {
            let mut price = crate::items::sell_price_of(e.id) * super::festivals::sell_mult(e.id, clock);
            // The Anglers restored: their city's market pays extra for fish (js perk).
            if fish_mult > 1.0 && crate::items::get(e.id).is_some_and(|d| d.kind == "FISH") {
                price = (price as f32 * fish_mult).round() as i32;
            }
            (e.uid, e.id, e.qty, price)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn enter_shop(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut shop: ResMut<ShopState>,
    inv: Res<PlayerInv>,
    bindings: Res<Bindings>,
    state: Res<ActionState>,
    old: Query<Entity, With<ShopUi>>,
    clock: Res<super::room_render::FrameClock>,
    perks: Res<super::guildhall::CityPerks>,
) {
    redraw(&mut commands, &mut images, &mut shop, &inv, &bindings, &state, &old, clock.0, perks.fish_mult);
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
/// The window's top-left corner (js x/y) — the anchor for the mouse hit-tests, matching redraw.
fn shop_origin() -> (f32, f32) {
    use super::room_render::{PLAY_X, PLAY_Y};
    use crate::room::{PX_H, PX_W};
    (PLAY_X + ((PX_W as f32 - W) / 2.0).round(), PLAY_Y + ((PX_H as f32 - H) / 2.0).round())
}

/// The BUY/SELL tab chips as (index, x, y, w, h) — same geometry redraw draws.
fn shop_tabs() -> [(usize, f32, f32, f32, f32); 2] {
    let (x, y) = shop_origin();
    let mut tx = x + 6.0;
    let mut out = [(0usize, 0.0, 0.0, 0.0, 0.0); 2];
    for (i, name) in ["BUY", "SELL"].into_iter().enumerate() {
        let tw = font::measure(name) as f32 + 8.0;
        out[i] = (i, tx, y + 4.0, tw, 11.0);
        tx += tw + 3.0;
    }
    out
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn shop_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<ActionState>,
    bindings: Res<Bindings>,
    mut next: ResMut<NextState<Screen>>,
    mut shop: ResMut<ShopState>,
    mut inv: ResMut<PlayerInv>,
    mut bought: ResMut<BoughtShop>,
    mut log: ResMut<super::rewards::LootLog>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    clock: Res<super::room_render::FrameClock>,
    perks: Res<super::guildhall::CityPerks>,
    old: Query<Entity, With<ShopUi>>,
    ptr: Res<crate::input::Pointer>,
) {
    // The window owns the face buttons while open (the heldLatch rule).
    for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
        state.latch(a);
    }
    if state.pressed(Action::Slot2) || state.pressed(Action::Pause) {
        next.set(Screen::Play);
        return;
    }
    let mut dirty = false;
    if state.pressed(Action::TabNext) || state.pressed(Action::TabPrev) {
        shop.tab ^= 1;
        shop.cursor = 0;
        shop.scroll = 0;
        dirty = true;
    }
    // Mouse: click BUY/SELL to switch tabs (before `n`, which depends on the tab).
    if ptr.click
        && let Some((i, ..)) = shop_tabs().into_iter().find(|&(_, tx, ty, tw, th)| ptr.over(tx, ty, tw, th))
        && i != shop.tab
    {
        shop.tab = i;
        shop.cursor = 0;
        shop.scroll = 0;
        dirty = true;
    }
    let n = if shop.tab == 0 { shop.stock.len() } else { sell_list(&inv, clock.0, perks.fish_mult).len() };
    if state.pressed(Action::Up) && shop.cursor > 0 {
        shop.cursor -= 1;
        dirty = true;
    }
    if state.pressed(Action::Down) && shop.cursor + 1 < n.max(1) {
        shop.cursor += 1;
        dirty = true;
    }
    if ptr.wheel_steps != 0 {
        // Wheel walks the list (Baz: any scrollable list honours the wheel).
        shop.cursor = (shop.cursor as i32 - ptr.wheel_steps).clamp(0, n.max(1) as i32 - 1) as usize;
        dirty = true;
    }
    // Mouse: the list SCROLLS, so hover does nothing — a click selects a row, clicking
    // the selected row buys/sells it (no accidental purchases on the first click).
    let mut row_click = false;
    if ptr.click {
        let (ox, oy) = shop_origin();
        let vis = ((H - 32.0) / ROW) as usize; // == redraw's (y+H-12-top)/ROW, top = y+20
        let top = oy + 20.0;
        let cur = shop.cursor.min(n.saturating_sub(1));
        let scroll = shop.scroll.min(cur).max((cur + 1).saturating_sub(vis)).min(n.saturating_sub(vis));
        for v in 0..vis {
            if scroll + v >= n {
                break;
            }
            if ptr.over(ox + 4.0, top + v as f32 * ROW - 1.0, W - 8.0, ROW - 1.0) {
                if shop.cursor != scroll + v {
                    shop.cursor = scroll + v;
                    dirty = true;
                } else {
                    row_click = true;
                }
            }
        }
    }
    if (state.pressed(Action::Slot1) || row_click) && n > 0 {
        if shop.tab == 0 {
            dirty |= buy(&mut shop, &mut inv, &mut bought, &mut log, &mut saves, super::gather::farm_day(clock.0));
        } else {
            // js sellItem: one unit per confirm at the 40% rate.
            let (uid, _, _, price) = sell_list(&inv, clock.0, perks.fish_mult)[shop.cursor.min(n - 1)];
            inv.money += price as i64;
            inv.remove_entry(uid);
            log.add("sold", &format!("SOLD +{}", coin_str(price as i64)), 1, 0xfcd000, false, true);
            let left = sell_list(&inv, clock.0, perks.fish_mult).len();
            shop.cursor = shop.cursor.min(left.saturating_sub(1));
            saves.write(super::save::SaveRequest);
            dirty = true;
        }
    }
    if dirty {
        redraw(&mut commands, &mut images, &mut shop, &inv, &bindings, &state, &old, clock.0, perks.fish_mult);
    }
}

/// js buyItem: purse + unique + bag-room checks, then the ware moves, the toast rolls,
/// and the shelf slot is marked sold for good.
fn buy(
    shop: &mut ShopState,
    inv: &mut PlayerInv,
    bought: &mut BoughtShop,
    log: &mut super::rewards::LootLog,
    saves: &mut MessageWriter<super::save::SaveRequest>,
    today: i64,
) -> bool {
    let i = shop.cursor.min(shop.stock.len() - 1);
    let entry = shop.stock[i].clone();
    if inv.money < entry.price as i64 {
        return false;
    }
    let def = crate::items::get(entry.id);
    if def.is_some_and(|d| d.unique) && inv.has_item(entry.id) {
        log.add("owned", "ALREADY OWNED", 1, 0xfc6868, false, true);
        return true;
    }
    if !inv.can_add(entry.id) {
        log.add("bagfull", "BAG FULL", 1, 0xfc6868, false, true);
        return true;
    }
    inv.money -= entry.price as i64;
    inv.add_item(entry.id, 1);
    let name = def.map_or(entry.id, |d| d.name);
    log.add(entry.id, &name.to_uppercase(), 1, crate::items::rarity_of(entry.id).color(), false, false);
    // Sold out: off the shelf. Staples return at dawn; one-of-a-kinds never restock.
    if restocks(entry.id) {
        if bought.day != today {
            bought.today.clear();
            bought.day = today;
        }
        bought.today.entry(shop.key.clone()).or_default().push(entry.id.to_string());
    } else {
        bought.forever.entry(shop.key.clone()).or_default().push(entry.id.to_string());
    }
    shop.stock.remove(i);
    shop.cursor = shop.cursor.min(shop.stock.len().saturating_sub(1));
    saves.write(super::save::SaveRequest);
    true
}

fn close_shop(mut commands: Commands, old: Query<Entity, With<ShopUi>>) {
    for e in &old {
        commands.entity(e).despawn();
    }
}

// --- The purse string (js coinStr): 100C = 1S, 100S = 1G. ---

pub fn coin_str(n: i64) -> String {
    let n = n.max(0);
    let (g, s, c) = (n / 10000, (n % 10000) / 100, n % 100);
    let mut out = String::new();
    if g > 0 {
        out.push_str(&format!("{g}G "));
    }
    if s > 0 {
        out.push_str(&format!("{s}S "));
    }
    if c > 0 || out.is_empty() {
        out.push_str(&format!("{c}C"));
    }
    out.trim_end().to_string()
}

/// Coin metals — the G/S/C unit letters tint by metal so they read as denominations,
/// not digits (js COIN_COL; `tint` off paints everything `num_col` — the can't-afford red).
#[allow(clippy::too_many_arguments)] // a draw helper's arity
fn draw_coin_str(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    n: i64,
    x: f32,
    y: f32,
    num_col: u32,
    tint: bool,
    z: f32,
) {
    let s = coin_str(n);
    let mut cx = x;
    // Runs of one colour become one label (fewer sprites than per-char).
    let mut run = String::new();
    let mut run_col = num_col;
    let col_of = |ch: char| match ch {
        'G' if tint => 0xfcd000,
        'S' if tint => 0xcfd6df,
        'C' if tint => 0xd98a4c,
        _ => num_col,
    };
    for ch in s.chars() {
        let col = col_of(ch);
        if col != run_col && !run.is_empty() {
            label(commands, images, &run, cx, y, run_col, z, ShopUi);
            // measure() drops the trailing letter-gap — without the +1 every colour
            // boundary lost a pixel and the G/S/C letters crowded the digits (Baz:
            // "prices are getting mangled").
            cx += font::measure(&run) as f32 + 1.0;
            run.clear();
        }
        run_col = col;
        run.push(ch);
    }
    if !run.is_empty() {
        label(commands, images, &run, cx, y, run_col, z, ShopUi);
    }
}

fn fill(commands: &mut Commands, x: f32, y: f32, w: f32, h: f32, color: Color, z: f32) {
    commands.spawn((Sprite::from_color(color, Vec2::new(w, h)), at(x, y, w, h, z), PIXEL_LAYER, ShopUi));
}

/// The full window (js drawShop) — rebuilt whenever the tab/cursor/shelf changes.
#[allow(clippy::too_many_arguments)] // a full-window draw's arity
fn redraw(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    shop: &mut ShopState,
    inv: &PlayerInv,
    bindings: &Bindings,
    state: &ActionState,
    old: &Query<Entity, With<ShopUi>>,
    clock: i64,
    fish_mult: f32,
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
        fill(commands, sx, sy, sw, sh, Color::srgb_u8(0xfc, 0xe0, 0xa8), Z + 0.01);
    }
    // Tabs (top-left) + the player's purse (top-right).
    let mut tx = x + 6.0;
    for (i, name) in ["BUY", "SELL"].into_iter().enumerate() {
        let on = i == shop.tab;
        let tw = font::measure(name) as f32 + 8.0;
        let bg = if on { Color::srgb_u8(0x26, 0x26, 0x2e) } else { Color::srgb_u8(0x14, 0x14, 0x18) };
        fill(commands, tx, y + 4.0, tw, 11.0, bg, Z + 0.02);
        if on {
            fill(commands, tx, y + 4.0, tw, 1.0, Color::srgb_u8(0xfc, 0xe0, 0xa8), Z + 0.03);
        }
        label(commands, images, name, tx + 4.0, y + 6.0, if on { 0xfcfcfc } else { 0x6c6c74 }, Z + 0.04, ShopUi);
        tx += tw + 3.0;
    }
    let purse = coin_str(inv.money);
    draw_coin_str(commands, images, inv.money, x + W - 6.0 - font::measure(&purse) as f32, y + 6.0, 0xf0f0f0, true, Z + 0.04);
    // The keeper's goodwill, made visible (Baz): FRIEND rates green, CONFIDANT gold.
    if shop.discount < 1.0 {
        let (tag_text, col) = if shop.discount <= 0.86 { ("CONFIDANT RATES", 0xffd34d) } else { ("FRIEND RATES", 0x7ee08a) };
        let tw = font::measure(tag_text) as f32;
        label(commands, images, tag_text, x + W - 6.0 - tw, y + 14.0, col, Z + 0.04, ShopUi);
    }

    // The list: 9 visible rows, cursor-following scroll (js VIS/shopScroll).
    let selling = sell_list(inv, clock, fish_mult);
    let rows: Vec<(&'static str, i32, i32)> = if shop.tab == 0 {
        shop.stock.iter().map(|e| (e.id, 1, e.price)).collect()
    } else {
        selling.iter().map(|(_, id, qty, price)| (*id, *qty, *price)).collect()
    };
    let top = y + 20.0;
    let vis = ((y + H - 12.0 - top) / ROW) as usize;
    let cursor = shop.cursor.min(rows.len().saturating_sub(1));
    // The js scroll-follow: pull up to the cursor, push down to keep it visible, clamp.
    let scroll = shop
        .scroll
        .min(cursor)
        .max((cursor + 1).saturating_sub(vis))
        .min(rows.len().saturating_sub(vis));
    shop.cursor = cursor;
    shop.scroll = scroll;
    if rows.is_empty() {
        let msg = if shop.tab == 0 { "OUT OF STOCK" } else { "NOTHING TO SELL" };
        label(commands, images, msg, x + 8.0, top + 4.0, 0x787878, Z + 0.04, ShopUi);
    }
    for v in 0..vis {
        let Some((id, qty, price)) = rows.get(scroll + v).copied() else { break };
        let ry = top + v as f32 * ROW;
        if scroll + v == cursor {
            fill(commands, x + 4.0, ry - 1.0, LIST_W - 8.0, ROW - 1.0, Color::srgb_u8(0x1c, 0x1c, 0x24), Z + 0.02);
            for (sx, sy, sw, sh) in border_strips(x + 4.0, ry - 1.0, LIST_W - 8.0, ROW - 1.0, 1.0) {
                fill(commands, sx, sy, sw, sh, Color::srgb_u8(0xfc, 0xe0, 0xa8), Z + 0.03);
            }
        }
        if let Some(def) = crate::items::get(id) {
            let mut icon = Sprite::from_image(images.add(crate::gfx::bake(def.icon, def.icon_pal)));
            icon.custom_size = Some(Vec2::splat(10.0));
            commands.spawn((icon, at(x + 7.0, ry, 10.0, 10.0, Z + 0.04), PIXEL_LAYER, ShopUi));
        }
        let afford = shop.tab == 1 || inv.money >= price as i64;
        let pw = font::measure(&coin_str(price as i64)) as f32;
        let name = crate::items::get(id).map_or(id, |d| d.name).to_uppercase();
        let name = if qty > 1 { format!("{name} x{qty}") } else { name };
        // The name yields to the price: clip with '..' rather than collide.
        let name_max = (LIST_W - 20.0 - pw - 12.0) as i32;
        let name = if font::measure(&name) > name_max {
            let mut cut = name.clone();
            while !cut.is_empty() && font::measure(&format!("{cut}..")) > name_max {
                cut.pop();
            }
            format!("{cut}..")
        } else {
            name
        };
        label(commands, images, &name, x + 20.0, ry + 1.0, crate::items::rarity_of(id).color(), Z + 0.04, ShopUi);
        let (pc, tint) = if afford { (0xf0f0f0, true) } else { (0xa05050, false) };
        draw_coin_str(commands, images, price as i64, x + LIST_W - 8.0 - pw, ry + 1.0, pc, tint, Z + 0.04);
    }
    if rows.len() > vis {
        let track_h = vis as f32 * ROW;
        let th = (track_h * vis as f32 / rows.len() as f32).round().max(6.0);
        let ty = top + ((track_h - th) * (scroll as f32 / (rows.len() - vis) as f32)).round();
        fill(commands, x + LIST_W - 4.0, top, 1.0, track_h, Color::srgb_u8(0x20, 0x20, 0x28), Z + 0.02);
        fill(commands, x + LIST_W - 4.0, ty, 1.0, th, Color::srgb_u8(0x6c, 0x6c, 0x78), Z + 0.03);
    }
    // The DETAILS pane (Baz: a two-panel window): the selected ware, examined —
    // big icon, name, its class + rarity, the price, and the flavour line.
    let (pane_x, pane_w) = (x + LIST_W + 6.0, W - LIST_W - 12.0);
    fill(commands, x + LIST_W, top - 2.0, 1.0, y + H - 12.0 - (top - 2.0), Color::srgb_u8(0x2a, 0x2a, 0x34), Z + 0.02);
    if let Some((id, qty, price)) = rows.get(cursor).copied()
        && let Some(def) = crate::items::get(id)
    {
        let mut dy = top + 2.0;
        let mut icon = Sprite::from_image(images.add(crate::gfx::bake(def.icon, def.icon_pal)));
        icon.custom_size = Some(Vec2::splat(20.0));
        commands.spawn((icon, at((pane_x + (pane_w - 20.0) / 2.0).round(), dy, 20.0, 20.0, Z + 0.04), PIXEL_LAYER, ShopUi));
        dy += 24.0;
        let rar = crate::items::rarity_of(id);
        for line in font::wrap(&def.name.to_uppercase(), pane_w as i32 - 2).into_iter().take(2) {
            let lw = font::measure(&line) as f32;
            label(commands, images, &line, (pane_x + (pane_w - lw) / 2.0).round(), dy, rar.color(), Z + 0.04, ShopUi);
            dy += 8.0;
        }
        let meta = format!("{} {}", rar.name(), def.kind);
        let mw = font::measure(&meta) as f32;
        label(commands, images, &meta, (pane_x + (pane_w - mw) / 2.0).round(), dy, 0x6c7480, Z + 0.04, ShopUi);
        dy += 10.0;
        let tag = if shop.tab == 0 { "PRICE" } else if qty > 1 { "EACH" } else { "SELLS FOR" };
        let pline = format!("{tag} {}", coin_str(price as i64));
        let plw = font::measure(&pline) as f32;
        let px0 = (pane_x + (pane_w - plw) / 2.0).round();
        label(commands, images, tag, px0, dy, 0x9aa4b0, Z + 0.04, ShopUi);
        draw_coin_str(commands, images, price as i64, px0 + font::measure(tag) as f32 + 4.0, dy, 0xf0f0f0, true, Z + 0.04);
        dy += 11.0;
        if !def.desc.is_empty() {
            for line in font::wrap(&def.desc.to_uppercase(), pane_w as i32 - 2).into_iter().take(6) {
                if dy > y + H - 20.0 {
                    break;
                }
                label(commands, images, &line, pane_x, dy, 0x8a94a0, Z + 0.04, ShopUi);
                dy += 8.0;
            }
        }
    }
    // The bottom hint (js promptLine with ' - '), keys following the live bindings.
    let pad = state.pad_present;
    let hint = format!(
        "{} {} - {}/{} SWITCH - {} LEAVE",
        bindings.prompt(Action::Slot1, pad),
        if shop.tab == 0 { "BUY" } else { "SELL" },
        bindings.prompt(Action::TabPrev, pad),
        bindings.prompt(Action::TabNext, pad),
        bindings.prompt(Action::Slot2, pad),
    );
    let hw = font::measure(&hint) as f32;
    label(commands, images, &hint, x + ((W - hw) / 2.0).round(), y + H - 9.0, 0xa0a0a0, Z + 0.04, ShopUi);
}

#[cfg(test)]
mod tests {
    use super::{coin_str, restocks};

    #[test]
    fn staples_restock_but_one_of_a_kinds_do_not() {
        assert!(restocks("potion"), "stackable consumables return at dawn");
        assert!(restocks("herb"), "materials return at dawn");
        assert!(!restocks("sword"), "gear stays sold out");
        assert!(!restocks("nosuchitem"), "unknown ids never restock (safe default)");
    }

    #[test]
    fn coin_denominations() {
        assert_eq!(coin_str(0), "0C");
        assert_eq!(coin_str(45), "45C");
        assert_eq!(coin_str(2345), "23S 45C");
        assert_eq!(coin_str(12345), "1G 23S 45C");
        assert_eq!(coin_str(10000), "1G");
        assert_eq!(coin_str(100), "1S");
    }
}
