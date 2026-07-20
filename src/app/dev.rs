//! dev.rs — the DEV PANEL, rebuilt from the ground up (Baz: the js overlay was "a
//! garbled mess"). One clean console instead of a scrolling edge strip:
//!
//!   - a live INFO STRIP (seed / room / biome / weather / day + clock),
//!   - CATEGORIES down the left (WORLD / TRAVEL / HERO / ITEMS / QUEST),
//!   - the selected category's commands on the right — label left, LIVE VALUE right,
//!   - cycle rows (WEATHER, SHARD SITE) adjust with left/right, everything else runs
//!     on INTERACT, and every action toasts through the loot feed so you see it land.
//!
//! Backquote opens/closes from play (world freezes, codex-style). Q/R switch category
//! (the codex tab keys; E is select).

use super::battle::RoomActor;
use super::gather::DAY_LEN;
use super::play::Player;
use super::rewards::{gain_xp, xp_for_level, LootLog};
use super::screen::Screen;
use crate::combat::Health;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use crate::ui::label;
use crate::{CANVAS_H, CANVAS_W};
use bevy::prelude::*;

const Z: f32 = 19.4; // over the play field + HUD, under pause (19.85) + flourish
const CATS: [&str; 5] = ["WORLD", "TRAVEL", "HERO", "ITEMS", "QUEST"];
const WEATHERS: [&str; 10] =
    ["natural", "clear", "overcast", "windy", "fog", "rain", "thunderstorm", "snow", "blizzard", "sandstorm"];

/// One command row: what it says, and what pressing it does.
#[derive(Clone, Copy, PartialEq)]
enum Cmd {
    TimeHour,
    NextDawn,
    SeasonNext,
    Weather, // cycle row
    WarpHome,
    WarpCastle,
    WarpShard, // cycle row
    Heal,
    Coins,
    LevelUp,
    FishKit,
    BowKit,
    MageKit,
    TravelKit,
    ShieldKit,
    KeyRing,
    Potions,
    ShardNext,
    ShardAll,
    ShardClear,
    God, // cycle-ish row: shows ON/OFF, INTERACT toggles
}

fn rows(cat: usize) -> &'static [(&'static str, Cmd)] {
    match cat {
        0 => &[
            ("TIME +1 HOUR", Cmd::TimeHour),
            ("SKIP TO DAWN", Cmd::NextDawn),
            ("SEASON +1", Cmd::SeasonNext),
            ("WEATHER", Cmd::Weather),
        ],
        1 => &[("WARP HOME", Cmd::WarpHome), ("WARP TO CASTLE", Cmd::WarpCastle), ("WARP TO SHARD SITE", Cmd::WarpShard)],
        2 => &[("FULL HEAL", Cmd::Heal), ("+100 COPPER", Cmd::Coins), ("LEVEL UP", Cmd::LevelUp), ("GOD MODE", Cmd::God)],
        3 => &[("FISHING KIT", Cmd::FishKit), ("ARCHER KIT", Cmd::BowKit), ("MAGE KIT", Cmd::MageKit), ("TRAVEL KIT", Cmd::TravelKit), ("SHIELD", Cmd::ShieldKit), ("KEY RING", Cmd::KeyRing), ("POTION PACK", Cmd::Potions)],
        _ => &[("GRANT NEXT SHARD", Cmd::ShardNext), ("GRANT ALL SHARDS", Cmd::ShardAll), ("CLEAR SHARDS", Cmd::ShardClear)],
    }
}

/// Invulnerability (dev): hits can't land, the bar stays full. G toggles from play;
/// the HERO category mirrors it with a live value.
#[derive(Resource, Default)]
pub struct GodMode(pub bool);

#[derive(Resource, Default)]
pub struct DevState {
    cat: usize,
    row: usize,
    weather_idx: usize,
    shard_idx: usize,
    dirty: bool,
}

#[derive(Component)]
struct DevUi;

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        // Input-reading systems live in FixedUpdate BEFORE EndTick (clear_pressed
        // consumes the press edges at the end of every fixed tick — an Update reader
        // only catches the odd frame with no tick in between, which felt like the
        // panel ignoring keys). redraw stays in Update: it's pure visuals.
        app.init_resource::<DevState>()
            .init_resource::<GodMode>()
            .add_systems(
                bevy::app::FixedUpdate,
                (god_toggle, god_tick).before(super::play::EndTick).run_if(super::screen::playing),
            )
            .add_systems(bevy::app::FixedUpdate, toggle.before(super::play::EndTick))
            .add_systems(
                bevy::app::FixedUpdate,
                drive.before(super::play::EndTick).run_if(in_state(Screen::Dev)),
            )
            .add_systems(Update, redraw.run_if(in_state(Screen::Dev)))
            // One tear-down for EVERY way out of the panel (backquote, PAUSE/SLOT2, and
            // any WARP row — all of which just `next.set(Play)`). Without this the opaque
            // DevUi backdrop outlived the state and the game looked frozen behind it.
            .add_systems(OnExit(Screen::Dev), close_dev);
    }
}

/// Backquote from play opens the console; backquote (or pause) inside closes it.
/// (Tear-down of the panel entities is `close_dev` on `OnExit(Screen::Dev)` — every
/// close route flows through it, so this just flips the state.)
fn toggle(
    mut input: ResMut<ActionState>,
    screen: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
    mut state: ResMut<DevState>,
) {
    if !input.pressed(Action::DevPanel) {
        return;
    }
    match screen.get() {
        Screen::Play => {
            input.consume(Action::DevPanel);
            state.dirty = true;
            next.set(Screen::Dev);
        }
        Screen::Dev => {
            input.consume(Action::DevPanel);
            next.set(Screen::Play);
        }
        _ => {}
    }
}

/// Despawn the whole panel — fires on OnExit(Screen::Dev) no matter HOW we left.
fn close_dev(mut commands: Commands, ui: Query<Entity, With<DevUi>>) {
    for e in &ui {
        commands.entity(e).despawn();
    }
}

/// Navigation + execution (the world stands frozen under us).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn drive(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    mut state: ResMut<DevState>,
    mut ctx: super::save::SaveCtx,
    mut swap: super::title::loader::SwapCtx,
    mut weather: ResMut<super::weather::WeatherState>,
    mut log: ResMut<LootLog>,
    caves: Res<super::caves::CrackCaves>,
    songs_opened: Res<super::caves::OpenedSongstones>,
    actors: Query<Entity, With<RoomActor>>,
    mut players: Query<(&mut Player, &mut Health)>,
    mut next: ResMut<NextState<Screen>>,
    mut god: ResMut<GodMode>,
    badges: Query<Entity, With<GodBadge>>,
) {
    let nrows = rows(state.cat).len();
    if input.pressed(Action::Pause) || input.pressed(Action::Slot2) {
        input.consume(Action::Pause);
        input.consume(Action::Slot2);
        next.set(Screen::Play);
        state.dirty = true; // (redraw skipped — toggle's despawn path handles reopen)
        return;
    }
    if input.pressed(Action::Down) {
        input.consume(Action::Down);
        state.row = (state.row + 1) % nrows;
        state.dirty = true;
    }
    if input.pressed(Action::Up) {
        input.consume(Action::Up);
        state.row = (state.row + nrows - 1) % nrows;
        state.dirty = true;
    }
    if input.pressed(Action::TabNext) {
        input.consume(Action::TabNext);
        state.cat = (state.cat + 1) % CATS.len();
        state.row = 0;
        state.dirty = true;
    }
    if input.pressed(Action::TabPrev) {
        input.consume(Action::TabPrev);
        state.cat = (state.cat + CATS.len() - 1) % CATS.len();
        state.row = 0;
        state.dirty = true;
    }
    let (_, cmd) = rows(state.cat)[state.row];
    // Cycle rows adjust with left/right (and left/right does nothing elsewhere).
    let dir = i32::from(input.pressed(Action::Right)) - i32::from(input.pressed(Action::Left));
    if dir != 0 {
        input.consume(Action::Left);
        input.consume(Action::Right);
        match cmd {
            Cmd::Weather => {
                let n = WEATHERS.len() as i32;
                state.weather_idx = ((state.weather_idx as i32 + dir + n) % n) as usize;
                let pick = WEATHERS[state.weather_idx];
                weather.force((pick != "natural").then(|| pick.to_string()));
                state.dirty = true;
            }
            Cmd::WarpShard => {
                let n = swap.world.0.shard_sites().len().max(1) as i32;
                state.shard_idx = ((state.shard_idx as i32 + dir + n) % n) as usize;
                state.dirty = true;
            }
            _ => {}
        }
    }
    if !input.pressed(Action::Interact) && !input.pressed(Action::Slot1) {
        return;
    }
    input.consume(Action::Interact);
    input.consume(Action::Slot1);
    let Ok((mut p, mut health)) = players.single_mut() else { return };
    let mut warp = |rx: i32, ry: i32, commands: &mut Commands, images: &mut Assets<Image>, swap: &mut super::title::loader::SwapCtx, ctx: &mut super::save::SaveCtx| {
        super::title::loader::swap_world_room(commands, images, swap, ctx, &caves, &songs_opened, &actors, rx, ry);
        p.x = 144.0;
        p.y = 120.0;
        p.facing = crate::actors::hero::Facing::Down;
        health.invuln = 30;
        next.set(Screen::Play);
    };
    match cmd {
        Cmd::TimeHour => {
            ctx.clock.0 += DAY_LEN / 24;
            log.add("dev", "AN HOUR PASSES", 1, 0xa8e0ff, false, true);
        }
        Cmd::NextDawn => {
            ctx.clock.0 = (ctx.clock.0 + DAY_LEN).div_euclid(DAY_LEN) * DAY_LEN;
            log.add("dev", "DAWN BREAKS", 1, 0xffd34d, false, true);
        }
        Cmd::SeasonNext => {
            ctx.clock.0 += 28 * DAY_LEN; // one SEASON_LEN of days
            log.add("dev", "THE SEASON TURNS", 1, 0xa8e0ff, false, true);
        }
        Cmd::Weather => {} // the value IS the action (left/right)
        Cmd::WarpHome => warp(0, 0, &mut commands, &mut images, &mut swap, &mut ctx),
        Cmd::WarpCastle => warp(crate::worldgen::world::CASTLE_RX, crate::worldgen::world::CASTLE_RY, &mut commands, &mut images, &mut swap, &mut ctx),
        Cmd::WarpShard => {
            if let Some(&(_, (rx, ry))) = swap.world.0.shard_sites().get(state.shard_idx) {
                warp(rx, ry, &mut commands, &mut images, &mut swap, &mut ctx);
            }
        }
        Cmd::Heal => {
            health.hp = health.max;
            log.add("dev", "HALE AND WHOLE", 1, 0x3cdc5a, false, true);
        }
        Cmd::Coins => {
            ctx.inv.money += 100;
            log.add("dev", "+100 COPPER", 1, 0xfcd000, true, true);
        }
        Cmd::LevelUp => {
            let need = xp_for_level(ctx.progress.level) - ctx.progress.xp;
            gain_xp(&mut ctx.progress, &mut ctx.alloc, need.max(1));
            log.add("dev", "LEVEL UP", 1, 0x5cc0fc, false, true);
        }
        Cmd::FishKit => {
            ctx.inv.add_item("fishingrod", 1);
            ctx.inv.auto_equip("fishingrod");
            log.add("dev", "FISHING KIT", 1, 0xa8e0ff, false, true);
        }
        Cmd::BowKit => {
            ctx.inv.add_item("bow", 1);
            ctx.inv.add_item("arrow", 20);
            ctx.inv.auto_equip("bow");
            log.add("dev", "ARCHER KIT", 1, 0xa8e0ff, false, true);
        }
        Cmd::MageKit => {
            for id in ["wand", "firerune", "frostrune", "stormrune"] {
                ctx.inv.add_item(id, 1);
            }
            ctx.inv.add_item("manapotion", 5);
            ctx.inv.auto_equip("wand");
            log.add("dev", "MAGE KIT", 1, 0xb890ff, false, true);
        }
        Cmd::TravelKit => {
            for id in ["grapplehook", "springboots", "bubblering"] {
                ctx.inv.add_item(id, 1);
            }
            ctx.inv.auto_equip("grapplehook");
            ctx.inv.auto_equip("springboots");
            ctx.inv.auto_equip("bubblering");
            log.add("dev", "TRAVEL KIT", 1, 0x8ad0ff, false, true);
        }
        Cmd::ShieldKit => {
            ctx.inv.add_item("shield", 1);
            ctx.inv.auto_equip("shield");
            log.add("dev", "SHIELD", 1, 0xc8a060, false, true);
        }
        Cmd::KeyRing => {
            ctx.inv.add_item("key", 3);
            ctx.inv.add_item("ornatekey", 1);
            log.add("dev", "KEY RING", 1, 0xfcd000, false, true);
        }
        Cmd::Potions => {
            ctx.inv.add_item("potion", 5);
            ctx.inv.add_item("greaterpotion", 2);
            log.add("dev", "POTION PACK", 1, 0xfc6868, false, true);
        }
        Cmd::ShardNext => {
            let next_biome = swap
                .world
                .0
                .shard_biomes()
                .iter()
                .find(|b| !ctx.social.relics.0.contains(**b))
                .copied();
            if let Some(b) = next_biome {
                ctx.social.relics.0.insert(b.to_string());
                let name = crate::relics_data::by_biome(b).map(|r| r.name.to_uppercase()).unwrap_or_default();
                log.add("dev", &format!("THE {name} IS YOURS"), 1, 0xc882ff, false, true);
            } else {
                log.add("dev", "THE WRIFTHEART IS ALREADY WHOLE", 1, 0xc882ff, false, true);
            }
        }
        Cmd::ShardAll => {
            for b in swap.world.0.shard_biomes().to_vec() {
                ctx.social.relics.0.insert(b.to_string());
            }
            log.add("dev", "THE WRIFTHEART IS WHOLE", 1, 0xc882ff, false, true);
        }
        Cmd::God => {
            god.0 = !god.0;
            set_badge(&mut commands, &mut images, god.0, &badges);
            log.add("dev", if god.0 { "GOD MODE ON" } else { "GOD MODE OFF" }, 1, 0xffd34d, false, true);
        }
        Cmd::ShardClear => {
            ctx.social.relics.0.clear();
            log.add("dev", "THE SHARDS SCATTER", 1, 0x9a9aa0, false, true);
        }
    }
    state.dirty = true;
}

/// Rebuild the panel whenever the cursor or the world state it reports changes.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn redraw(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<DevState>,
    old: Query<Entity, With<DevUi>>,
    clock: Res<super::room_render::FrameClock>,
    cur: Res<super::play::CurRoom>,
    world: Res<super::play::GameWorld>,
    weather: Res<super::weather::WeatherState>,
    relics: Res<super::dungeon::Relics>,
    bindings: Res<crate::input::Bindings>,
    input: Res<ActionState>,
    god: Res<GodMode>,
) {
    if !state.dirty {
        return;
    }
    state.dirty = false;
    for e in &old {
        commands.entity(e).despawn();
    }
    let (w, h) = (CANVAS_W as f32, CANVAS_H as f32);
    // The backdrop + frame.
    commands.spawn((
        Sprite::from_color(Color::srgb(0.024, 0.024, 0.05), Vec2::new(w, h)), // OPAQUE — nothing garbles through
        at(0.0, 0.0, w, h, Z),
        PIXEL_LAYER,
        DevUi,
    ));
    let line = |commands: &mut Commands, y: f32| {
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(0x2a, 0x2a, 0x3a), Vec2::new(w - 16.0, 1.0)),
            at(8.0, y, w - 16.0, 1.0, Z + 0.1),
            PIXEL_LAYER,
            DevUi,
        ));
    };
    // Header: title + the live info strip.
    label(&mut commands, &mut images, "DEV", 10.0, 8.0, 0xffd34d, Z + 0.2, DevUi);
    let day = clock.0.div_euclid(DAY_LEN) + 1;
    let tod = (clock.0.rem_euclid(DAY_LEN)) as f64 / DAY_LEN as f64;
    let h24 = (12.0 + tod * 24.0) % 24.0;
    let info = format!(
        "SEED {}  ROOM {},{}  DAY {day}  {:02}:{:02}",
        world.0.seed,
        cur.rx,
        cur.ry,
        h24 as u32,
        ((h24 - h24.floor()) * 60.0) as u32
    );
    let iw = font::measure(&info) as f32;
    label(&mut commands, &mut images, &info, w - 10.0 - iw, 8.0, 0x8a8a92, Z + 0.2, DevUi);
    let info2 = format!(
        "{}  -  {}  -  {} OF {} SHARDS",
        world.0.biome_key_at(cur.rx, cur.ry).to_uppercase(),
        crate::weather::get(weather.cur).label,
        relics.0.len(),
        world.0.shard_biomes().len()
    );
    label(&mut commands, &mut images, &info2, 10.0, 18.0, 0x8a8a92, Z + 0.2, DevUi);
    line(&mut commands, 28.0);

    // Categories, left column.
    for (i, cat) in CATS.iter().enumerate() {
        let sel = i == state.cat;
        let col = if sel { 0xfcfcfc } else { 0x5a5a66 };
        if sel {
            label(&mut commands, &mut images, ">", 10.0, 38.0 + i as f32 * 12.0, 0xffd34d, Z + 0.2, DevUi);
        }
        label(&mut commands, &mut images, cat, 18.0, 38.0 + i as f32 * 12.0, col, Z + 0.2, DevUi);
    }
    // The column divider.
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x2a, 0x2a, 0x3a), Vec2::new(1.0, h - 76.0)),
        at(78.0, 34.0, 1.0, h - 76.0, Z + 0.1),
        PIXEL_LAYER,
        DevUi,
    ));

    // Commands, right pane: label left, live value right, cursor row highlighted.
    for (i, (name, cmd)) in rows(state.cat).iter().enumerate() {
        let y = 38.0 + i as f32 * 12.0;
        let sel = i == state.row;
        if sel {
            commands.spawn((
                Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.07), Vec2::new(w - 100.0, 10.0)),
                at(86.0, y - 2.0, w - 100.0, 10.0, Z + 0.15),
                PIXEL_LAYER,
                DevUi,
            ));
            label(&mut commands, &mut images, ">", 88.0, y, 0xffd34d, Z + 0.2, DevUi);
        }
        label(&mut commands, &mut images, name, 96.0, y, if sel { 0xfcfcfc } else { 0x9a9aa2 }, Z + 0.2, DevUi);
        let value: Option<String> = match cmd {
            Cmd::Weather => Some(format!("< {} >", WEATHERS[state.weather_idx].to_uppercase())),
            Cmd::WarpShard => world
                .0
                .shard_sites()
                .get(state.shard_idx)
                .map(|(b, (rx, ry))| format!("< {} {rx},{ry} >", b.to_uppercase())),
            Cmd::God => Some(if god.0 { "ON".into() } else { "OFF".into() }),
            _ => None,
        };
        if let Some(v) = value {
            let vw = font::measure(&v) as f32;
            label(&mut commands, &mut images, &v, w - 12.0 - vw, y, if sel { 0xa8e0ff } else { 0x6a7a8a }, Z + 0.2, DevUi);
        }
    }
    line(&mut commands, h - 16.0);
    // EVERY prompt derives from the LIVE bindings (and flips to pad glyphs with a
    // controller) — rebind in CONTROLS and this line keeps up.
    let pad = input.pad_present;
    let hint = format!(
        "{}/{} MOVE - {}/{} CATEGORY - {} RUN - {}/{} ADJUST - {} CLOSE",
        bindings.prompt(Action::Up, pad),
        bindings.prompt(Action::Down, pad),
        bindings.prompt(Action::TabPrev, pad),
        bindings.prompt(Action::TabNext, pad),
        bindings.prompt(Action::Interact, pad),
        bindings.prompt(Action::Left, pad),
        bindings.prompt(Action::Right, pad),
        bindings.prompt(Action::DevPanel, pad),
    );
    label(&mut commands, &mut images, &hint, 10.0, h - 11.0, 0x5a5a66, Z + 0.2, DevUi);
}


/// The badge in the top-right while the walls are down.
#[derive(Component)]
struct GodBadge;

/// G flips it from play (the dev panel's HERO row mirrors it).
fn god_toggle(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    mut god: ResMut<GodMode>,
    mut log: ResMut<LootLog>,
    badges: Query<Entity, With<GodBadge>>,
) {
    if !input.pressed(Action::God) {
        return;
    }
    input.consume(Action::God);
    god.0 = !god.0;
    set_badge(&mut commands, &mut images, god.0, &badges);
    log.add("dev", if god.0 { "GOD MODE ON" } else { "GOD MODE OFF" }, 1, 0xffd34d, false, true);
}

/// While on: hits can't land (a standing invuln floor) and the bar stays full.
fn god_tick(god: Res<GodMode>, mut players: Query<&mut Health, With<Player>>) {
    if !god.0 {
        return;
    }
    for mut h in &mut players {
        h.hp = h.max;
        h.invuln = h.invuln.max(2);
    }
}

fn set_badge(commands: &mut Commands, images: &mut Assets<Image>, on: bool, badges: &Query<Entity, With<GodBadge>>) {
    for e in badges {
        commands.entity(e).despawn();
    }
    if on {
        let text = "GOD MODE";
        let w = font::measure(text) as f32;
        label(commands, images, text, CANVAS_W as f32 - 6.0 - w, 4.0, 0xffd34d, 17.3, GodBadge);
    }
}
