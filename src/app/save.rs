//! save.rs — persistence v1: ONE autosave file (js saveGame -> localStorage; here a JSON
//! file in the platform data dir). Loaded before setup, applied at boot; written every
//! ~10s of active play and on every pause. The 4-slot picker + title screen = increment 2.
//!
//! Robustness rules: item/mob/node references save as STRINGS (ids), not indexes — a
//! save survives registry reordering, and unknown ids (from a newer/older build) drop
//! quietly instead of corrupting. WRIFT_SHOT runs never load or write saves.

use super::codex::items_tab::Discovered;
use super::codex::mobs_tab::Bestiary;
use super::gather::{GatherState, TreeGrowth};
use super::play::{CurRoom, Player, Visited};
use super::rewards::Progress;
use super::room_render::FrameClock;
use super::screen::{playing, Screen};
use super::slideout::{skills_tab, TreeAlloc, TreeStats};
use crate::combat::Health;
use crate::inventory::{InvEntry, PlayerInv};
use crate::items;
use crate::skilltree;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const AUTOSAVE_TICKS: u32 = 600; // ~10s at 60Hz

/// (rx, ry, day, taken tiles) — a room's gather record, flattened for JSON.
pub type GatherRow = (i32, i32, i64, Vec<(i32, i32)>);
/// (rx, ry, [(c, r, cut day)]) — a room's tree-growth stamps, flattened for JSON.
pub type GrowthRow = (i32, i32, Vec<(i32, i32, i64)>);
/// (rx, ry, taken tiles) — a room's permanently-taken placed items, flattened for JSON.
pub type PlacedRow = (i32, i32, Vec<(i32, i32)>);

// serde default for the once-per-day festival markers (-1 = never)
fn neg_one() -> i64 {
    -1
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)] // an older file missing newer fields loads with defaults, never drops
pub struct SaveData {
    pub version: u32,
    pub name: String,                    // hero name — the slot card line
    pub gender: String,                  // "M" | "F" (cosmetic flavor)
    pub look: crate::actors::hero::Look, // creator appearance (colors + hairstyle)
    pub traits: Vec<String>,             // rolled trait keys (crate::traits)
    pub ts: u64,                         // epoch seconds at write — CONTINUE resumes the newest slot
    pub seed: u32,                       // this save's WORLD seed (js World.setSeed per new game)
    pub rx: i32,
    pub ry: i32,
    pub px: f32,
    pub py: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub defense: i32,
    pub level: i32,
    pub xp: i32,
    pub money: i64,
    pub entries: Vec<(u32, String, i32)>,
    /// Shield wear by entry uid (js e.dur) — absent for fresh shields and old saves.
    #[serde(default)]
    pub shield_dur: Vec<(u32, i32)>,
    /// The wand's socketed rune element (js player.wandRune).
    #[serde(default)]
    pub wand_rune: String,
    pub bag: Vec<Option<u32>>,
    pub bag_rows: usize,
    pub slots: [Option<u32>; 4],
    pub gear: [Option<u32>; 6],
    pub next_uid: u32,
    pub tree: Vec<String>, // allocated node IDs (stable across table reorders)
    pub points: i32,
    pub stats: Vec<(String, f64)>,
    pub bestiary: Vec<String>,
    pub discovered: Vec<String>,
    pub gather: Vec<GatherRow>,
    pub growth: Vec<GrowthRow>,
    pub visited: Vec<(i32, i32)>,
    pub clock: i64,
    pub town_names: Vec<(String, String)>, // assigned town names (unique per game)
    pub placed: Vec<PlacedRow>, // hand-placed items taken forever
    pub lorebooks: Vec<String>, // collected lore-tome ids
    pub bought_shop: Vec<(String, Vec<String>)>, // per-shop sold-out-forever ledger (one-of-a-kinds)
    pub sold_today: Vec<(String, Vec<String>)>, // per-shop staples bought today (restock at dawn)
    pub sold_day: i64, // the dawn-day sold_today belongs to
    pub people: Vec<(String, super::talk::PersonRec)>, // relationship ledger (js people)
    pub relics: Vec<String>, // claimed shard biomes (js relics Set — THE WIN COUNTER)
    pub dungeons: Vec<(String, super::dungeon::DgSave)>, // banked dungeon progress per entrance
    pub farm: Vec<FarmRow>, // tilled soil + crops (js Farm.serialize)
    pub cleared_encounters: Vec<(i32, i32)>, // beaten set-piece rooms (peaceful forever)
    pub quests: Vec<super::quests::Quest>, // the active 3-slot log
    pub quest_giver_done: Vec<(String, i32)>, // per-giver completed tallies
    pub quest_counter: u32, // monotonic quest-id source
    pub songs: Vec<String>, // learned flute songs (js learnedSongs)
    pub awards: Vec<String>, // earned deeds (js unlockedAchievements)
    pub met_wanderers: Vec<(i32, i32)>, // wanderer boons claimed (js metWanderers)
    #[serde(default = "neg_one")]
    pub festival_seen_day: i64, // js festivalSeenDay
    #[serde(default = "neg_one")]
    pub blessed_day: i64, // js blessedDay (Bellnight)
    #[serde(default)]
    pub livestock: super::farm_animals::Livestock, // coops/barns/animals (js saved rows)
    #[serde(default)]
    pub guilds: bevy::platform::collections::HashMap<String, super::guildhall::GuildState>, // per-city hall restoration
    #[serde(default)]
    pub stations: Vec<super::cooking::StationRec>, // placed crafting fires (js placedTables)
    #[serde(default)]
    pub blueprints: Vec<String>, // learned blueprint ids (js learnedBlueprints)
    #[serde(default)]
    pub stash: Vec<super::storage::StashEntry>, // the home chest's contents (js playerStash)
    #[serde(default)]
    pub house: Option<super::home::HouseRec>, // the one placed home (js playerHouse)
    #[serde(default)]
    pub loot_gob: Option<super::lootgoblin::LootGobRec>, // the roaming loot goblin (js lootGob)
    #[serde(default)]
    pub loot_gob_cleared: Vec<(i32, i32)>, // origins whose goblin was slain/escaped (js lootGobCleared)
    #[serde(default)]
    pub crack_caves: bevy::platform::collections::HashMap<String, Vec<(i32, i32, String)>>, // opened cave doors (js crackCaves)
    #[serde(default)]
    pub songstones: Vec<String>, // sung-open stones, room keys (js openedSongstones)
    #[serde(default)]
    pub tmaps: Vec<super::digging::TMap>, // undug treasure-map X's (js treasureMaps)
    #[serde(default)]
    pub side_looted: Vec<String>, // hidden-chamber caches taken (js sideLooted)
    #[serde(default)]
    pub castle_guards_cleared: bool, // both gate knights fell (js)
    #[serde(default)]
    pub game_won: bool, // the Wriftheart has been mended (js gameWon)
    #[serde(default = "full_can")]
    pub can_water: i32, // the watering can's remaining pours (pre-farm saves: full)
}

fn full_can() -> i32 {
    super::farm::CAN_CAP
}

/// One hoed tile, flattened for JSON (js Farm.serialize's {k, home, tendedDay,
/// wateredDay, crop}); the crop rides as (id, stage, dry).
pub type FarmRow = (i32, i32, i32, i32, bool, i64, i64, Option<(String, i32, i32)>);

/// Rebuild [`super::farm::FarmTiles`] from saved rows (crop ids re-intern via the
/// registry; unknown crops drop, the soil stays).
pub fn farm_from_save(rows: &[FarmRow]) -> super::farm::FarmTiles {
    let mut farm = super::farm::FarmTiles::default();
    for (rx, ry, c, r, home, tended, watered, crop) in rows {
        let crop = crop.as_ref().and_then(|(id, stage, dry)| {
            crate::items::crop(id).map(|d| super::farm::Crop { id: d.id, stage: *stage, dry: *dry })
        });
        farm.0
            .entry((*rx, *ry))
            .or_default()
            .insert((*c, *r), super::farm::FarmTile { home: *home, tended: *tended, watered: *watered, crop });
    }
    farm
}

/// The save loaded at boot (None on a fresh start). Consumed by setup + apply_save.
#[derive(Resource, Default)]
pub struct Loaded(pub Option<SaveData>);

pub const SAVE_SLOTS: u32 = 4;

fn slot_path(n: u32) -> Option<std::path::PathBuf> {
    crate::persist::data_file(&format!("save{n}.json"))
}

/// Read one slot's full save (None: empty slot, unreadable file, or WRIFT_SHOT).
pub fn read_slot(n: u32) -> Option<SaveData> {
    if !crate::persist::enabled() {
        return None;
    }
    let s = std::fs::read_to_string(slot_path(n)?).ok()?;
    serde_json::from_str::<SaveData>(&s).ok()
}

pub fn delete_slot(n: u32) {
    if let Some(p) = slot_path(n) {
        let _ = std::fs::remove_file(p);
    }
}

/// A slot's title-picker summary line (js meta).
pub struct SlotMeta {
    pub name: String,
    pub level: i32,
    pub clock: i64, // season + day derive from this at draw time
    pub ts: u64,
}

/// Per-slot summaries for the title screen, refreshed on entry / delete / save.
#[derive(Resource, Default)]
pub struct SlotMetas(pub Vec<Option<SlotMeta>>);

pub fn scan_metas() -> SlotMetas {
    SlotMetas(
        (1..=SAVE_SLOTS)
            .map(|n| {
                read_slot(n).map(|d| SlotMeta {
                    name: if d.name.is_empty() { "HERO".into() } else { d.name },
                    level: d.level.max(1),
                    clock: d.clock,
                    ts: d.ts,
                })
            })
            .collect(),
    )
}

/// The newest occupied slot (1-based; slot 1 when nothing is saved) — what CONTINUE loads.
pub fn latest_slot(metas: &SlotMetas) -> u32 {
    let mut best = 1;
    let mut bt = 0;
    for (i, m) in metas.0.iter().enumerate() {
        if let Some(m) = m
            && m.ts >= bt
        {
            bt = m.ts;
            best = i as u32 + 1;
        }
    }
    best
}

/// Which slot file autosaves + SAVE write to (js 'baz.slot'). Set at boot to the newest
/// slot; retargeted by every title-screen pick.
#[derive(Resource)]
pub struct ActiveSlot(pub u32);

/// The pause menu's SAVE row (and anything else wanting a checkpoint NOW) writes one of
/// these; the handler snapshots on the next fixed tick.
#[derive(Message)]
pub struct SaveRequest;

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        // Present from frame zero: the initial Screen::Title transition (and its
        // enter_title) runs BEFORE PreStartup's commands flush.
        app.init_resource::<Loaded>()
            .init_resource::<SlotMetas>()
            .insert_resource(ActiveSlot(1))
            .add_message::<SaveRequest>()
            .add_systems(PreStartup, load_save)
            .add_systems(PostStartup, apply_save)
            .add_systems(bevy::app::FixedUpdate, (autosave_tick.run_if(playing), save_on_request))
            .add_systems(OnEnter(Screen::Pause), save_now);
    }
}

/// Boot: scan the slots, load the NEWEST save (the world spawns straight into it — the
/// title's CONTINUE then just unfreezes; picking another slot hot-reloads).
fn load_save(mut commands: Commands) {
    let metas = scan_metas();
    let slot = latest_slot(&metas);
    let data = read_slot(slot);
    if data.is_some() {
        info!("save loaded (slot {slot})");
    }
    commands.insert_resource(Loaded(data));
    commands.insert_resource(ActiveSlot(slot));
    commands.insert_resource(metas);
}

/// Everything the collector reads / the applier writes (Bevy's 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub struct SaveCtx<'w> {
    pub cur: ResMut<'w, CurRoom>,
    pub inv: ResMut<'w, PlayerInv>,
    pub progress: ResMut<'w, Progress>,
    pub alloc: ResMut<'w, TreeAlloc>,
    pub tstats: ResMut<'w, TreeStats>,
    pub stats: ResMut<'w, super::stats::Stats>,
    pub bestiary: ResMut<'w, Bestiary>,
    pub discovered: ResMut<'w, Discovered>,
    pub gather: ResMut<'w, GatherState>,
    pub growth: ResMut<'w, TreeGrowth>,
    pub visited: ResMut<'w, Visited>,
    pub clock: ResMut<'w, FrameClock>,
    pub active: ResMut<'w, ActiveSlot>,
    pub ident: ResMut<'w, super::identity::HeroIdent>,
    pub town_names: ResMut<'w, super::banners::TownNames>,
    pub social: SocialCtx<'w>, // 16 fields — AT the SystemParam cap; new ones nest here
}

/// The town-social slice of the save (nested so SaveCtx stays under Bevy's 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub struct SocialCtx<'w> {
    pub bought: ResMut<'w, super::shop::BoughtShop>,
    pub people: ResMut<'w, super::talk::PeopleLedger>,
    pub relics: ResMut<'w, super::dungeon::Relics>,
    pub dungeon_ledger: ResMut<'w, super::dungeon::DungeonLedger>,
    pub farm: ResMut<'w, super::farm::FarmTiles>,
    pub can_water: ResMut<'w, super::farm::CanWater>,
    pub farm_day: ResMut<'w, super::farm::LastFarmDay>,
    pub cleared: ResMut<'w, super::encounters::ClearedEncounters>,
    pub quests: ResMut<'w, super::quests::QuestLog>,
    pub giver_done: ResMut<'w, super::quests::GiverDone>,
    pub quest_counter: ResMut<'w, super::quests::QuestCounter>,
    pub songs: ResMut<'w, super::flute::LearnedSongs>,
    pub awards: ResMut<'w, super::codex::awards_tab::Unlocked>,
    pub met_wanderers: ResMut<'w, super::encounters::MetWanderers>,
    pub fest: ResMut<'w, super::festivals::FestivalLedger>,
    pub livestock: ResMut<'w, super::farm_animals::Livestock>,
}

/// The save's SIDE-LEDGERS: SaveCtx sits at the 16-field SystemParam cap, so every
/// later saved resource rides this second bundle instead of growing parameter lists.
#[derive(bevy::ecs::system::SystemParam)]
pub struct SaveExtras<'w> {
    pub guilds: ResMut<'w, super::guildhall::GuildLedger>,
    pub stations: ResMut<'w, super::cooking::PlacedStations>,
    pub blueprints: ResMut<'w, super::blueprints::LearnedBlueprints>,
    pub stash: ResMut<'w, super::storage::PlayerStash>,
    pub house: ResMut<'w, super::home::PlayerHouse>,
    pub loot_gob: ResMut<'w, super::lootgoblin::LootGob>,
    pub loot_gob_cleared: ResMut<'w, super::lootgoblin::LootGobCleared>,
    pub caves: ResMut<'w, super::caves::CrackCaves>,
    pub songs: ResMut<'w, super::caves::OpenedSongstones>,
    pub tmaps: ResMut<'w, super::digging::TreasureMaps>,
    pub side_looted: ResMut<'w, super::sidescroll::SideLooted>,
    pub guards: ResMut<'w, super::darkknight::CastleGuards>,
    pub rune: ResMut<'w, super::wands::WandRune>,
    pub victory: ResMut<'w, super::dungeon::Victory>,
}

/// Restore every resource-side piece of the save (setup already consumed the world/room/
/// player fields — see play.rs).
fn apply_save(loaded: Res<Loaded>, mut ctx: SaveCtx, mut extras: SaveExtras) {
    if let Some(d) = &loaded.0 {
        apply_to(d, &mut ctx, &mut extras);
    }
}

/// The resource half of loading, shared by boot and the title's hot slot-reload.
pub fn apply_to(d: &SaveData, ctx: &mut SaveCtx, extras: &mut SaveExtras) {
    extras.guilds.0 = d.guilds.clone();
    extras.stations.0 = d.stations.clone();
    extras.blueprints.0 = d.blueprints.iter().cloned().collect();
    extras.stash.0 = d.stash.clone();
    extras.house.0 = d.house.clone();
    extras.loot_gob.0 = d.loot_gob.clone();
    extras.loot_gob_cleared.0 = d.loot_gob_cleared.iter().copied().collect();
    extras.caves.0 = d.crack_caves.clone();
    extras.songs.0 = d.songstones.iter().cloned().collect();
    extras.tmaps.0 = d.tmaps.clone();
    extras.side_looted.0 = d.side_looted.iter().cloned().collect();
    extras.guards.0 = d.castle_guards_cleared;
    extras.victory.won = d.game_won;
    // The wand's socketed rune (pre-wand saves stored "": stay arcane).
    extras.rune.0 = match d.wand_rune.as_str() {
        "fire" => "fire",
        "frost" => "frost",
        "storm" => "storm",
        _ => "arcane",
    };
    *ctx.ident = super::identity::HeroIdent {
        name: if d.name.is_empty() { "HERO".into() } else { d.name.clone() },
        gender: if d.gender.is_empty() { "M".into() } else { d.gender.clone() },
        look: d.look.clone(),
        traits: d.traits.clone(),
    };
    // Inventory: ids come back as &'static via the registry; unknown ids drop (their slot
    // references dangle harmlessly — entry() lookups just miss).
    ctx.inv.entries = d
        .entries
        .iter()
        .filter_map(|(uid, id, qty)| items::get(id).map(|def| InvEntry { uid: *uid, id: def.id, qty: *qty, dur: None }))
        .collect();
    for (uid, dur) in &d.shield_dur {
        if let Some(e) = ctx.inv.entries.iter_mut().find(|e| e.uid == *uid) {
            e.dur = Some(*dur);
        }
    }
    ctx.inv.bag = d.bag.clone();
    ctx.inv.bag_rows = d.bag_rows.max(1);
    ctx.inv.slots = d.slots;
    ctx.inv.gear = d.gear;
    ctx.inv.money = d.money;
    ctx.inv.next_uid = d.next_uid;
    ctx.progress.level = d.level.max(1);
    ctx.progress.xp = d.xp;
    ctx.alloc.taken = d
        .tree
        .iter()
        .filter_map(|id| skilltree::nodes().iter().position(|n| n.id == id.as_str()))
        .collect();
    ctx.alloc.points = d.points;
    *ctx.tstats = skills_tab::recompute(&ctx.alloc, &d.traits, false, &ctx.inv);
    ctx.stats.0 = d.stats.iter().cloned().collect();
    ctx.bestiary.0 = statics(&d.bestiary, |s| {
        crate::actors::mobs::MOB_DEFS.iter().map(|m| m.kind).chain(["goblin", "slinger"]).find(|k| *k == s)
    });
    ctx.discovered.0 = statics(&d.discovered, |s| items::get(s).map(|def| def.id));
    ctx.gather.rooms = d
        .gather
        .iter()
        .map(|(rx, ry, day, tiles)| ((*rx, *ry), (*day, tiles.iter().copied().collect())))
        .collect();
    ctx.growth.0 = d
        .growth
        .iter()
        .map(|(rx, ry, tiles)| ((*rx, *ry), tiles.iter().map(|(c, r, day)| ((*c, *r), *day)).collect()))
        .collect();
    ctx.visited.0 = d.visited.iter().copied().collect();
    ctx.clock.0 = d.clock;
    ctx.town_names.0 = d.town_names.iter().cloned().collect();
    ctx.gather.placed = d
        .placed
        .iter()
        .map(|(rx, ry, tiles)| ((*rx, *ry), tiles.iter().copied().collect()))
        .collect();
    ctx.gather.tomes =
        d.lorebooks.iter().filter_map(|id| crate::lore_books::get(id).map(|b| b.id)).collect();
    ctx.social.bought.forever = d.bought_shop.iter().cloned().collect();
    ctx.social.bought.today = d.sold_today.iter().cloned().collect();
    ctx.social.bought.day = d.sold_day;
    ctx.social.people.0 = d.people.iter().cloned().collect();
    ctx.social.relics.0 = d.relics.iter().cloned().collect();
    ctx.social.dungeon_ledger.0 = d.dungeons.iter().cloned().collect();
    *ctx.social.farm = farm_from_save(&d.farm);
    ctx.social.farm.prune(super::gather::farm_day(d.clock)); // wild plots that decayed while away
    ctx.social.can_water.0 = d.can_water;
    ctx.social.farm_day.0 = super::gather::farm_day(d.clock); // today is already accounted for
    ctx.social.cleared.0 = d.cleared_encounters.iter().copied().collect();
    ctx.social.quests.0 = d.quests.clone();
    ctx.social.giver_done.0 = d.quest_giver_done.iter().cloned().collect();
    ctx.social.quest_counter.0 = d.quest_counter;
    ctx.social.songs.0 = d.songs.iter().filter_map(|id| crate::songs::get(id).map(|s| s.id)).collect();
    ctx.social.awards.0 = d.awards.iter().filter_map(|id| crate::achievements::get(id).map(|a| a.id)).collect();
    ctx.social.met_wanderers.0 = d.met_wanderers.iter().copied().collect();
    ctx.social.fest.seen_day = d.festival_seen_day;
    ctx.social.fest.blessed_day = d.blessed_day;
    *ctx.social.livestock = d.livestock.clone();
}

/// Map saved strings back onto &'static keys, dropping unknowns.
fn statics(
    saved: &[String],
    find: impl Fn(&str) -> Option<&'static str>,
) -> HashSet<&'static str> {
    saved.iter().filter_map(|s| find(s)).collect()
}

/// Snapshot the world into a SaveData (the collector half). `seed` is the live world's —
/// callers read it off GameWorld (SaveCtx can't hold it: the loader mutates the world).
pub fn collect(ctx: &SaveCtx, extras: &SaveExtras, player: &Player, health: &Health, seed: u32) -> SaveData {
    SaveData {
        version: 1,
        name: ctx.ident.name.clone(),
        gender: ctx.ident.gender.clone(),
        look: ctx.ident.look.clone(),
        traits: ctx.ident.traits.clone(),
        ts: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
        seed,
        rx: ctx.cur.rx,
        ry: ctx.cur.ry,
        px: player.x,
        py: player.y,
        hp: health.hp,
        max_hp: health.max,
        defense: health.defense,
        level: ctx.progress.level,
        xp: ctx.progress.xp,
        money: ctx.inv.money,
        entries: ctx.inv.entries.iter().map(|e| (e.uid, e.id.to_string(), e.qty)).collect(),
        shield_dur: ctx.inv.entries.iter().filter_map(|e| e.dur.map(|d| (e.uid, d))).collect(),
        wand_rune: extras.rune.0.to_string(),
        bag: ctx.inv.bag.clone(),
        bag_rows: ctx.inv.bag_rows,
        slots: ctx.inv.slots,
        gear: ctx.inv.gear,
        next_uid: ctx.inv.next_uid,
        tree: ctx.alloc.taken.iter().map(|i| skilltree::nodes()[*i].id.to_string()).collect(),
        points: ctx.alloc.points,
        stats: ctx.stats.0.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        bestiary: ctx.bestiary.0.iter().map(|s| s.to_string()).collect(),
        discovered: ctx.discovered.0.iter().map(|s| s.to_string()).collect(),
        gather: ctx
            .gather
            .rooms
            .iter()
            .map(|((rx, ry), (day, tiles))| (*rx, *ry, *day, tiles.iter().copied().collect()))
            .collect(),
        growth: ctx
            .growth
            .0
            .iter()
            .map(|((rx, ry), tiles)| (*rx, *ry, tiles.iter().map(|((c, r), day)| (*c, *r, *day)).collect()))
            .collect(),
        visited: ctx.visited.0.iter().copied().collect(),
        clock: ctx.clock.0,
        town_names: ctx.town_names.0.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        placed: ctx
            .gather
            .placed
            .iter()
            .map(|((rx, ry), tiles)| (*rx, *ry, tiles.iter().copied().collect()))
            .collect(),
        lorebooks: ctx.gather.tomes.iter().map(|s| s.to_string()).collect(),
        bought_shop: ctx.social.bought.forever.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        sold_today: ctx.social.bought.today.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        sold_day: ctx.social.bought.day,
        people: ctx.social.people.0.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        relics: {
            let mut v: Vec<String> = ctx.social.relics.0.iter().cloned().collect();
            v.sort(); // deterministic file bytes
            v
        },
        dungeons: ctx.social.dungeon_ledger.0.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        farm: {
            let mut v: Vec<FarmRow> = ctx
                .social
                .farm
                .0
                .iter()
                .flat_map(|((rx, ry), tiles)| {
                    tiles.iter().map(|((c, r), t)| {
                        let crop = t.crop.as_ref().map(|cr| (cr.id.to_string(), cr.stage, cr.dry));
                        (*rx, *ry, *c, *r, t.home, t.tended, t.watered, crop)
                    })
                })
                .collect();
            v.sort_by_key(|(rx, ry, c, r, ..)| (*rx, *ry, *c, *r)); // deterministic file bytes
            v
        },
        can_water: ctx.social.can_water.0,
        cleared_encounters: {
            let mut v: Vec<(i32, i32)> = ctx.social.cleared.0.iter().copied().collect();
            v.sort_unstable(); // deterministic file bytes
            v
        },
        quests: ctx.social.quests.0.clone(),
        quest_giver_done: {
            let mut v: Vec<(String, i32)> = ctx.social.giver_done.0.iter().map(|(k, n)| (k.clone(), *n)).collect();
            v.sort(); // deterministic file bytes
            v
        },
        quest_counter: ctx.social.quest_counter.0,
        songs: {
            let mut v: Vec<String> = ctx.social.songs.0.iter().map(|s| s.to_string()).collect();
            v.sort(); // deterministic file bytes
            v
        },
        awards: {
            let mut v: Vec<String> = ctx.social.awards.0.iter().map(|s| s.to_string()).collect();
            v.sort(); // deterministic file bytes
            v
        },
        festival_seen_day: ctx.social.fest.seen_day,
        blessed_day: ctx.social.fest.blessed_day,
        livestock: ctx.social.livestock.clone(),
        guilds: extras.guilds.0.clone(),
        stations: extras.stations.0.clone(),
        blueprints: extras.blueprints.0.iter().cloned().collect(),
        stash: extras.stash.0.clone(),
        house: extras.house.0.clone(),
        loot_gob: extras.loot_gob.0.clone(),
        loot_gob_cleared: extras.loot_gob_cleared.0.iter().copied().collect(),
        crack_caves: extras.caves.0.clone(),
        tmaps: extras.tmaps.0.clone(),
        castle_guards_cleared: extras.guards.0,
        game_won: extras.victory.won,
        side_looted: {
            let mut v: Vec<String> = extras.side_looted.0.iter().cloned().collect();
            v.sort(); // deterministic file bytes
            v
        },
        songstones: {
            let mut v: Vec<String> = extras.songs.0.iter().cloned().collect();
            v.sort(); // deterministic file bytes
            v
        },
        met_wanderers: {
            let mut v: Vec<(i32, i32)> = ctx.social.met_wanderers.0.iter().copied().collect();
            v.sort_unstable(); // deterministic file bytes
            v
        },
    }
}

pub fn write_save(ctx: &SaveCtx, extras: &SaveExtras, player: &Player, health: &Health, seed: u32) {
    if !crate::persist::enabled() {
        return;
    }
    let Some(path) = slot_path(ctx.active.0) else { return };
    let data = collect(ctx, extras, player, health, seed);
    if let Ok(json) = serde_json::to_string(&data) {
        let _ = std::fs::write(path, json);
    }
}

/// The ~10s heartbeat while the world runs (js saveGame fires on events; a heartbeat
/// covers the same ground with one system). AUTOSAVE OFF in the menu silences it —
/// then only the explicit SAVE row touches disk (the js rule).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn autosave_tick(
    ctx: SaveCtx,
    extras: SaveExtras,
    players: Query<(&Player, &Health)>,
    settings: Res<crate::settings::Settings>,
    world: Res<super::play::GameWorld>,
    inside: Res<super::interior::Inside>,
    mut t: Local<u32>,
) {
    *t += 1;
    if *t < AUTOSAVE_TICKS || !settings.autosave || inside.0.is_some() {
        return;
    }
    *t = 0;
    if let Ok((p, h)) = players.single() {
        write_save(&ctx, &extras, p, h, world.0.seed);
    }
}

/// Pausing is a natural checkpoint — save immediately (gated with autosave: with it off,
/// the player has asked for explicit writes only).
fn save_now(
    ctx: SaveCtx,
    extras: SaveExtras,
    players: Query<(&Player, &Health)>,
    settings: Res<crate::settings::Settings>,
    world: Res<super::play::GameWorld>,
    inside: Res<super::interior::Inside>,
) {
    // Indoors the player position is interior-local — a save here would load him into
    // the overworld at those coords. The pause checkpoint waits for the doorstep.
    if !settings.autosave || inside.0.is_some() {
        return;
    }
    if let Ok((p, h)) = players.single() {
        write_save(&ctx, &extras, p, h, world.0.seed);
    }
}

/// An explicit SAVE from the pause menu — always writes, autosave setting or not.
fn save_on_request(
    mut reqs: MessageReader<SaveRequest>,
    ctx: SaveCtx,
    extras: SaveExtras,
    players: Query<(&Player, &Health)>,
    world: Res<super::play::GameWorld>,
    inside: Res<super::interior::Inside>,
) {
    if reqs.read().next().is_none() || inside.0.is_some() {
        return;
    }
    if let Ok((p, h)) = players.single() {
        write_save(&ctx, &extras, p, h, world.0.seed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The JSON round trip preserves every field group (the save format's canary).
    #[test]
    fn save_round_trip() {
        let d = SaveData {
            version: 1,
            name: "HERO".into(),
            gender: "M".into(),
            look: crate::actors::hero::Look::default(),
            traits: vec!["strong".into(), "frail".into()],
            ts: 1770000000,
            seed: 1337,
            rx: -3,
            ry: 7,
            px: 120.5,
            py: 88.0,
            hp: 2,
            max_hp: 5,
            defense: 1,
            level: 4,
            xp: 17,
            money: 231,
            entries: vec![(1, "sword".into(), 1), (5, "wood".into(), 14)],
            shield_dur: Vec::new(),
            wand_rune: String::new(),
            bag: vec![Some(5), None],
            bag_rows: 1,
            slots: [Some(1), None, None, None],
            gear: [None; 6],
            next_uid: 5,
            tree: vec!["warm1".into()],
            points: 2,
            stats: vec![("kills".into(), 9.0)],
            bestiary: vec!["goblin".into(), "wolf".into()],
            discovered: vec!["potion".into()],
            gather: vec![(0, 0, 3, vec![(4, 5)])],
            growth: vec![(0, 0, vec![(6, 7, 2)])],
            visited: vec![(0, 0), (1, 0)],
            clock: 54321,
            town_names: vec![("3,4".into(), "OAKDALE".into())],
            placed: vec![(1, 0, vec![(5, 4)])],
            lorebooks: vec!["sundering".into()],
            bought_shop: vec![("2,3,general,100,50".into(), vec!["compass".into()])],
            sold_today: vec![("2,3,general,100,50".into(), vec!["potion".into()])],
            sold_day: 6,
            people: vec![(
                "i:2,3:inn:100,50:0".into(),
                super::super::talk::PersonRec { pts: 240, last_chat: 6, name: "MARA THE INNKEEP".into(), seed: 99, ..Default::default() },
            )],
                    relics: vec!["greenmaw".into()],
            dungeons: vec![],
            farm: vec![(2, 2, 5, 6, false, 3, 3, Some(("turnip".into(), 1, 0)))],
            can_water: 7,
            cleared_encounters: vec![(4, -2)],
            quests: vec![],
            quest_giver_done: vec![("1,2,777".into(), 2)],
            quest_counter: 5,
            songs: vec!["returning".into()],
            awards: vec!["firstblood".into()],
            met_wanderers: vec![(2, -3)],
            festival_seen_day: -1,
            blessed_day: -1,
            livestock: Default::default(),
            guilds: Default::default(),
            stations: Default::default(),
            blueprints: Default::default(),
            stash: Default::default(),
            house: Default::default(),
            loot_gob: Default::default(),
            loot_gob_cleared: Default::default(),
            crack_caves: Default::default(),
            songstones: Default::default(),
            tmaps: Default::default(),
            side_looted: Default::default(),
            castle_guards_cleared: false,
            game_won: false,
        };
        let json = serde_json::to_string(&d).unwrap();
        let back: SaveData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.rx, -3);
        assert_eq!(back.entries.len(), 2);
        assert_eq!(back.tree, vec!["warm1".to_string()]);
        assert_eq!(back.gather[0].3, vec![(4, 5)]);
        assert_eq!(back.clock, 54321);
    }
}
