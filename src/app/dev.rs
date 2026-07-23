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
const CATS: [&str; 8] = ["WORLD", "TRAVEL", "HERO", "KITS", "MATS", "LEARN", "SPAWN", "QUEST"];

/// The category chips as (index, x, y, w, h) — the SAME top chip-tab bar every other
/// menu uses (Baz: the old side column was the odd one out). One geometry source for
/// the draw and the mouse hit-test.
fn tab_chips() -> Vec<(usize, f32, f32, f32, f32)> {
    let mut tx = 8.0;
    CATS.iter()
        .enumerate()
        .map(|(i, title)| {
            let tw = font::measure(title) as f32 + 8.0;
            let chip = (i, tx, 32.0, tw, 11.0);
            tx += tw + 2.0;
            chip
        })
        .collect()
}
const WEATHERS: [&str; 10] =
    ["natural", "clear", "overcast", "windy", "fog", "rain", "thunderstorm", "snow", "blizzard", "sandstorm"];

/// One command row: what it says, and what pressing it does.
#[derive(Clone, Copy, PartialEq)]
enum Cmd {
    TimeHour,
    NextDawn,
    SeasonNext,
    Weather, // cycle row
    Strip,   // pin the status line to the viewport top
    WarpHome,
    WarpCastle,
    WarpShard, // cycle row
    Heal,
    Coins,
    Coins1000,
    LevelUp,
    Level5,
    FishKit,
    BowKit,
    MageKit,
    RuneKit,
    FarmKit,
    TableKit,
    TravelKit,
    ShieldKit,
    KeyRing,
    Potions,
    SongsAll,
    BlueprintsAll,
    LootRoll,
    OreKit,
    GemKit,
    WoodKit,
    MatsKit,
    SpawnMob, // cycle row: left/right picks the kind, press spawns it
    SpawnGoblin,
    SpawnSpear,
    ClearRoom,
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
            ("STATUS STRIP", Cmd::Strip),
        ],
        1 => &[("WARP HOME", Cmd::WarpHome), ("WARP TO CASTLE", Cmd::WarpCastle), ("WARP TO SHARD SITE", Cmd::WarpShard)],
        2 => &[
            ("FULL HEAL", Cmd::Heal),
            ("+100 COPPER", Cmd::Coins),
            ("+1000 COPPER", Cmd::Coins1000),
            ("LEVEL UP", Cmd::LevelUp),
            ("LEVEL +5", Cmd::Level5),
            ("GOD MODE", Cmd::God),
        ],
        // Ten rows is the panel's comfortable max (row 13 ran into the footer, Baz) —
        // the learn-alls + the loot dice live on their own LEARN page instead.
        3 => &[
            ("FISHING KIT", Cmd::FishKit),
            ("ARCHER KIT", Cmd::BowKit),
            ("MAGE KIT", Cmd::MageKit),
            ("RUNE KIT", Cmd::RuneKit),
            ("FARM KIT", Cmd::FarmKit),
            ("TABLE KIT", Cmd::TableKit),
            ("TRAVEL KIT", Cmd::TravelKit),
            ("SHIELD", Cmd::ShieldKit),
            ("KEY RING", Cmd::KeyRing),
            ("POTION PACK", Cmd::Potions),
        ],
        4 => &[
            ("ORE KIT", Cmd::OreKit),
            ("GEM KIT", Cmd::GemKit),
            ("WOOD KIT", Cmd::WoodKit),
            ("MATS KIT", Cmd::MatsKit),
        ],
        5 => &[
            ("LEARN ALL SONGS", Cmd::SongsAll),
            ("LEARN ALL BLUEPRINTS", Cmd::BlueprintsAll),
            ("LOOT ROLL", Cmd::LootRoll),
        ],
        6 => &[
            ("SPAWN MOB", Cmd::SpawnMob),
            ("SPAWN GOBLIN", Cmd::SpawnGoblin),
            ("SPAWN SPEAR GOBLIN", Cmd::SpawnSpear),
            ("CLEAR ROOM", Cmd::ClearRoom),
        ],
        _ => &[("GRANT NEXT SHARD", Cmd::ShardNext), ("GRANT ALL SHARDS", Cmd::ShardAll), ("CLEAR SHARDS", Cmd::ShardClear)],
    }
}

/// Invulnerability (dev): hits can't land, the bar stays full. G toggles from play;
/// the HERO category mirrors it with a live value.
#[derive(Resource, Default)]
pub struct GodMode(pub bool);

/// Dev toggle: pin the panel's status line (room/biome/tiers/weather/shards) to the
/// top of the viewport while playing (Baz).
#[derive(Resource, Default)]
pub struct StatusStrip(pub bool);

#[derive(Component)]
struct StripUi;

/// The one status-line builder — the dev panel and the strip must never drift.
fn status_line(
    world: &super::play::GameWorld,
    cur: &super::play::CurRoom,
    weather: &super::weather::WeatherState,
    relics: &super::dungeon::Relics,
) -> String {
    format!(
        "ROOM {},{}  -  {}  -  ZONE {}  -  THREAT {}  -  {}  -  {} OF {} SHARDS",
        cur.rx,
        cur.ry,
        world.0.biome_key_at(cur.rx, cur.ry).to_uppercase(),
        crate::worldgen::World::zone_tier(cur.rx, cur.ry),
        crate::worldgen::World::threat_tier(cur.rx, cur.ry),
        crate::weather::get(weather.cur).label,
        relics.0.len(),
        world.0.shard_biomes().len()
    )
}

/// Rebuild the strip when its text flips (room move, weather, a shard) or the toggle.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn strip_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    strip: Res<StatusStrip>,
    world: Res<super::play::GameWorld>,
    cur: Res<super::play::CurRoom>,
    weather: Res<super::weather::WeatherState>,
    relics: Res<super::dungeon::Relics>,
    old: Query<Entity, With<StripUi>>,
    mut last: Local<Option<String>>,
) {
    let want = strip.0.then(|| status_line(&world, &cur, &weather, &relics));
    if want == *last {
        return;
    }
    *last = want.clone();
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(text) = want else { return };
    use super::room_render::{PLAY_X, PLAY_Y};
    use crate::room::PX_W;
    commands.spawn((
        Sprite::from_color(bevy::color::Color::srgba(0.0, 0.0, 0.0, 0.55), bevy::math::Vec2::new(PX_W as f32, 10.0)),
        crate::gfx::at(PLAY_X, PLAY_Y, PX_W as f32, 10.0, 15.35), // under the banners (15.5)
        crate::gfx::PIXEL_LAYER,
        StripUi,
    ));
    let tw = crate::gfx::font::measure(&text) as f32;
    crate::ui::label(&mut commands, &mut images, &text, PLAY_X + ((PX_W as f32 - tw) / 2.0).round(), PLAY_Y + 2.0, 0x8a8a92, 15.4, StripUi);
}

#[derive(Resource, Default)]
pub struct DevState {
    cat: usize,
    row: usize,
    weather_idx: usize,
    shard_idx: usize,
    mob_idx: usize,
    dirty: bool,
}

#[derive(Component)]
struct DevUi;

/// Click a chip to jump to its category (the shared tab-bar behaviour).
fn dev_tab_click(ptr: Res<crate::input::Pointer>, mut state: ResMut<DevState>) {
    if !ptr.click {
        return;
    }
    if let Some((i, ..)) = tab_chips().into_iter().find(|&(_, x, y, w2, h2)| ptr.over(x, y, w2, h2))
        && state.cat != i
    {
        state.cat = i;
        state.row = 0;
        state.dirty = true;
    }
}

/// GOD MODE's purse: bottomless — anything spent refills instantly. The MORTAL purse is
/// remembered the tick god mode comes on and restored the tick it goes off (Baz: no
/// keeping the divine allowance). The stash rides the SAVED stats ledger under a hidden
/// key, so it survives save/quit — and since god mode itself never persists, a load that
/// finds the key restores the honest purse on the first tick.
fn god_money(god: Res<GodMode>, mut stats: ResMut<super::stats::Stats>, mut inv: ResMut<crate::inventory::PlayerInv>) {
    if god.0 {
        if !stats.0.contains_key("godpurse") {
            stats.0.insert("godpurse".into(), inv.money as f64); // capture BEFORE the pin
        }
        if inv.money < 999_999 {
            inv.money = 999_999;
        }
    } else if let Some(m) = stats.0.remove("godpurse") {
        inv.money = m as i64;
    }
}

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        // Input-reading systems live in FixedUpdate BEFORE EndTick (clear_pressed
        // consumes the press edges at the end of every fixed tick — an Update reader
        // only catches the odd frame with no tick in between, which felt like the
        // panel ignoring keys). redraw stays in Update: it's pure visuals.
        app.init_resource::<DevState>()
            .init_resource::<GodMode>()
            .init_resource::<StatusStrip>()
            .add_systems(Update, strip_tick)
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
            .add_systems(bevy::app::FixedUpdate, dev_tab_click.before(super::play::EndTick).run_if(in_state(Screen::Dev)))
            .add_systems(bevy::app::FixedUpdate, god_money)
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

/// The odds and ends the dev rows reach for (bundled — `drive` sits at Bevy's
/// 16-param cap, so new resources nest here).
#[allow(clippy::type_complexity)] // CLEAR ROOM's Or-filter (mobs AND goblinkind) is the point
#[derive(bevy::ecs::system::SystemParam)]
struct DevRefs<'w, 's> {
    house: Res<'w, super::home::PlayerHouse>,
    // NO songs / rng here — drive already reaches them via ctx.social.songs and
    // swap.rng; a duplicate ResMut is a B0002 PANIC AT BOOT, not a compile error.
    bps: ResMut<'w, super::blueprints::LearnedBlueprints>,
    keys: ResMut<'w, super::dungeon::DungeonKeys>,
    ptr: Res<'w, crate::input::Pointer>,
    foes: Query<'w, 's, Entity, Or<(With<crate::actors::mobs::Mob>, With<crate::actors::goblin::Goblin>)>>,
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
    toggles: (ResMut<GodMode>, ResMut<StatusStrip>), // tuple: drive sits AT the 16-param cap
    badges: Query<Entity, With<GodBadge>>,
    mut refs: DevRefs,
) {
    let (mut god, mut strip) = toggles;
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
    if refs.ptr.wheel_steps != 0 {
        // Wheel walks the rows, clamped (Baz: any scrollable list).
        state.row = (state.row as i32 - refs.ptr.wheel_steps).clamp(0, nrows as i32 - 1) as usize;
        state.dirty = true;
    }
    // Mouse on the rows (Baz: "the options aren't clickable"): hover selects (a
    // flat menu), a click RUNS the row — except the cycle rows, whose value IS the
    // action: a click steps them right. Injected as presses so the key handlers
    // below treat mouse and keyboard identically. Same geometry as redraw (52 + i*12).
    if let Some(pp) = refs.ptr.pos {
        for i in 0..nrows {
            let ry = 52.0 + i as f32 * 12.0;
            if pp.x >= 8.0 && pp.x < CANVAS_W as f32 - 8.0 && pp.y >= ry - 2.0 && pp.y < ry + 10.0 {
                if refs.ptr.moved && state.row != i {
                    state.row = i;
                    state.dirty = true;
                }
                if refs.ptr.click {
                    state.row = i;
                    // The `< value >` side cycles; the label side RUNS (SpawnMob and
                    // WarpShard have both; Weather's value is its only action).
                    let on_value = pp.x > CANVAS_W as f32 * 0.62;
                    match rows(state.cat)[i].1 {
                        Cmd::Weather => input.press(Action::Right),
                        Cmd::WarpShard | Cmd::SpawnMob if on_value => input.press(Action::Right),
                        _ => input.press(Action::Interact),
                    }
                }
            }
        }
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
            Cmd::SpawnMob => {
                let n = crate::actors::mobs::MOB_DEFS.len() as i32;
                state.mob_idx = ((state.mob_idx as i32 + dir + n) % n) as usize;
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
    let mut warp = |rx: i32, ry: i32, px: f32, py: f32, commands: &mut Commands, images: &mut Assets<Image>, swap: &mut super::title::loader::SwapCtx, ctx: &mut super::save::SaveCtx| {
        super::title::loader::swap_world_room(commands, images, swap, ctx, &caves, &songs_opened, &actors, rx, ry, None); // dev warp: no home-safe gate
        p.x = px;
        p.y = py;
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
        Cmd::WarpHome => {
            // HOME means your HOUSE — doorstep landing, just clear of the door mat so
            // the warp doesn't suck you straight inside. No house yet -> refuse and
            // say why; the old bed/origin fallbacks dumped you somewhere that READ
            // as a conjured home (Baz).
            if let Some(h) = refs.house.0.as_ref() {
                let ((rx, ry), (px, py)) = (h.room, (h.x + 3.0, (h.y + 28.0).min(crate::room::PX_H as f32 - 24.0)));
                warp(rx, ry, px, py, &mut commands, &mut images, &mut swap, &mut ctx);
            } else {
                log.add("dev", "NO HOME YET - BUILD A HOUSE FIRST", 1, 0xff9a66, false, true);
            }
        }
        Cmd::WarpCastle => warp(crate::worldgen::world::CASTLE_RX, crate::worldgen::world::CASTLE_RY, 144.0, 120.0, &mut commands, &mut images, &mut swap, &mut ctx),
        Cmd::WarpShard => {
            if let Some(&(_, (rx, ry))) = swap.world.0.shard_sites().get(state.shard_idx) {
                warp(rx, ry, 144.0, 120.0, &mut commands, &mut images, &mut swap, &mut ctx);
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
            for id in ["wand", "firerune", "frostrune", "stormrune", "venomrune"] {
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
            // The keys ride the DUNGEON RING now, not the bag (the key HUD reads it).
            refs.keys.small += 3;
            refs.keys.ornate += 1;
            log.add("dev", "KEY RING", 1, 0xfcd000, false, true);
        }
        Cmd::Potions => {
            ctx.inv.add_item("potion", 5);
            ctx.inv.add_item("greaterpotion", 2);
            log.add("dev", "POTION PACK", 1, 0xfc6868, false, true);
        }
        Cmd::Coins1000 => {
            ctx.inv.money += 1000;
            log.add("dev", "+1000 COPPER", 1, 0xfcd000, true, true);
        }
        Cmd::Level5 => {
            for _ in 0..5 {
                let need = xp_for_level(ctx.progress.level) - ctx.progress.xp;
                gain_xp(&mut ctx.progress, &mut ctx.alloc, need.max(1));
            }
            log.add("dev", "FIVE LEVELS AT A STROKE", 1, 0x5cc0fc, false, true);
        }
        Cmd::RuneKit => {
            for id in ["firerune", "frostrune", "stormrune", "venomrune"] {
                ctx.inv.add_item(id, 3);
            }
            log.add("dev", "RUNE KIT", 1, 0xb890ff, false, true);
        }
        Cmd::FarmKit => {
            ctx.inv.add_item("hoe", 1);
            ctx.inv.add_item("wateringcan", 1);
            for id in ["turnipseed", "carrotseed", "potatoseed", "tomatoseed", "wheatseed", "pumpkinseed", "pepperseed", "cranberryseed"] {
                ctx.inv.add_item(id, 5);
            }
            log.add("dev", "FARM KIT", 1, 0x8ac850, false, true);
        }
        Cmd::TableKit => {
            for id in ["cook", "workbench", "forge", "alchemy", "enchanter", "fletcher", "jeweler", "farmtable", "well", "house"] {
                ctx.inv.add_item(id, 1);
            }
            log.add("dev", "EVERY TABLE IN THE LAND", 1, 0xd0a860, false, true);
        }
        Cmd::SongsAll => {
            ctx.inv.add_item("flute", 1);
            for s in crate::songs::LIST {
                super::flute::learn_song(&mut ctx.social.songs, &ctx.inv, &mut log, s.id, true);
            }
            log.add("dev", "EVERY SONG KNOWN", 1, 0xb8a0d8, false, true);
        }
        Cmd::BlueprintsAll => {
            for r in crate::recipes_data::RECIPES {
                if let Some(bp) = r.bp {
                    refs.bps.0.insert(bp.to_string());
                }
            }
            log.add("dev", "EVERY SCHEMATIC KNOWN", 1, 0x9ad0ff, false, true);
        }
        Cmd::LootRoll => {
            let (id, qty) = crate::items::roll_loot(1.2, 0.5, || swap.rng.0.next_f64());
            ctx.inv.add_item(id, qty);
            let name = crate::items::get(id).map(|d| d.name).unwrap_or(id);
            log.add("dev", &format!("THE DICE GIVE {}", name.to_uppercase()), 1, 0xffd34d, false, true);
        }
        Cmd::OreKit => {
            for id in ["copper", "iron", "silver", "gold", "mithril", "voidsteel"] {
                ctx.inv.add_item(id, 20);
            }
            log.add("dev", "ORE KIT - EVERY TIER", 1, 0xc8b088, false, true);
        }
        Cmd::GemKit => {
            ctx.inv.add_item("gem", 10);
            log.add("dev", "GEM KIT", 1, 0xb060f0, false, true);
        }
        Cmd::WoodKit => {
            for id in ["wood", "hardwood", "ironbark", "voidwood", "petalwood", "gloomwood", "charwood", "mirewood", "frostpine"] {
                ctx.inv.add_item(id, 20);
            }
            log.add("dev", "WOOD KIT - EVERY TIER", 1, 0xa07040, false, true);
        }
        Cmd::MatsKit => {
            for id in ["stone", "fiber", "herb", "leather", "meat", "arrow"] {
                ctx.inv.add_item(id, 20);
            }
            log.add("dev", "MATS KIT", 1, 0x9aa0aa, false, true);
        }
        Cmd::SpawnMob => {
            let (fdx, fdy) = p.facing.offset();
            let sx = (p.x + fdx * 40.0).clamp(16.0, crate::room::PX_W as f32 - 32.0);
            let sy = (p.y + fdy * 40.0).clamp(16.0, crate::room::PX_H as f32 - 32.0);
            let d = &crate::actors::mobs::MOB_DEFS[state.mob_idx];
            commands.spawn((crate::actors::mobs::mob_bundle(state.mob_idx, sx, sy), super::battle::RoomActor, PIXEL_LAYER));
            log.add("dev", &format!("A {} ANSWERS THE CALL", d.kind.to_uppercase()), 1, 0xfc8868, false, true);
        }
        Cmd::SpawnGoblin | Cmd::SpawnSpear => {
            let (fdx, fdy) = p.facing.offset();
            let sx = (p.x + fdx * 40.0).clamp(16.0, crate::room::PX_W as f32 - 32.0);
            let sy = (p.y + fdy * 40.0).clamp(16.0, crate::room::PX_H as f32 - 32.0);
            let gk = if cmd == Cmd::SpawnSpear { crate::actors::goblin::GoblinKind::Spear } else { crate::actors::goblin::GoblinKind::Melee };
            let mut e = commands.spawn((crate::actors::goblin::goblin_bundle(gk, sx, sy), super::battle::RoomActor, PIXEL_LAYER));
            e.insert(Sprite::default());
            log.add("dev", "A GOBLIN ANSWERS THE CALL", 1, 0xfc8868, false, true);
        }
        Cmd::ClearRoom => {
            let mut n = 0;
            for e in &refs.foes {
                commands.entity(e).despawn();
                n += 1;
            }
            log.add("dev", &format!("{n} FOES SWEPT AWAY"), 1, 0x9aa0aa, false, true);
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
        Cmd::Strip => {
            strip.0 = !strip.0;
            log.add("dev", if strip.0 { "STATUS STRIP ON" } else { "STATUS STRIP OFF" }, 1, 0xffd34d, false, true);
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
    strip: Res<StatusStrip>,
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
    let info2 = status_line(&world, &cur, &weather, &relics);
    label(&mut commands, &mut images, &info2, 10.0, 18.0, 0x8a8a92, Z + 0.2, DevUi);
    line(&mut commands, 28.0);

    // Category chips — the standard top tab bar (codex/pause/slide-out look: lit bg +
    // gold rule on the active chip).
    for (i, tx, ty, tw, th) in tab_chips() {
        let on = i == state.cat;
        let bg = if on { Color::srgb_u8(0x2a, 0x2a, 0x18) } else { Color::srgb_u8(0x14, 0x14, 0x18) };
        commands.spawn((Sprite::from_color(bg, Vec2::new(tw, th)), at(tx, ty, tw, th, Z + 0.15), PIXEL_LAYER, DevUi));
        if on {
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0xff, 0xd3, 0x4d), Vec2::new(tw, 1.0)),
                at(tx, ty, tw, 1.0, Z + 0.2),
                PIXEL_LAYER,
                DevUi,
            ));
        }
        label(&mut commands, &mut images, CATS[i], tx + 4.0, ty + 2.0, if on { 0xfcfcfc } else { 0x6c6c74 }, Z + 0.2, DevUi);
    }
    line(&mut commands, 45.0);

    // Commands, full width under the chips: label left, live value right, cursor row lit.
    for (i, (name, cmd)) in rows(state.cat).iter().enumerate() {
        let y = 52.0 + i as f32 * 12.0;
        let sel = i == state.row;
        if sel {
            commands.spawn((
                Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.07), Vec2::new(w - 16.0, 10.0)),
                at(8.0, y - 2.0, w - 16.0, 10.0, Z + 0.15),
                PIXEL_LAYER,
                DevUi,
            ));
            label(&mut commands, &mut images, ">", 10.0, y, 0xffd34d, Z + 0.2, DevUi);
        }
        label(&mut commands, &mut images, name, 18.0, y, if sel { 0xfcfcfc } else { 0x9a9aa2 }, Z + 0.2, DevUi);
        let value: Option<String> = match cmd {
            Cmd::Weather => Some(format!("< {} >", WEATHERS[state.weather_idx].to_uppercase())),
            Cmd::WarpShard => world
                .0
                .shard_sites()
                .get(state.shard_idx)
                .map(|(b, (rx, ry))| format!("< {} {rx},{ry} >", b.to_uppercase())),
            Cmd::God => Some(if god.0 { "ON".into() } else { "OFF".into() }),
            Cmd::Strip => Some(if strip.0 { "ON".into() } else { "OFF".into() }),
            Cmd::SpawnMob => {
                Some(format!("< {} >", crate::actors::mobs::MOB_DEFS[state.mob_idx].kind.to_uppercase()))
            }
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
fn god_tick(god: Res<GodMode>, mut players: Query<&mut Health, With<Player>>, mut mana: ResMut<super::flute::Mana>) {
    if !god.0 {
        return;
    }
    for mut h in &mut players {
        h.hp = h.max;
        h.invuln = h.invuln.max(2);
    }
    mana.cur = mana.max; // the divine well never runs dry
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
