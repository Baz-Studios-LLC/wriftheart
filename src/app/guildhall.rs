//! app/guildhall.rs — the city guildhall's GAME layer (js game.js increments 1-3):
//! enter the boarded hall (a lit, peaceful guildhall "dungeon" — gen has carried
//! its floors and wing tags all along), find each wing's DONATION ALTAR under its
//! crest banner, and fill the bundle line by line straight from your bag. A filled
//! wing brings its guild home: a toast, a one-time reward, and a city-wide perk.
//! All five home = THE GUILDHALL STANDS WHOLE (the guild seal). Progress is
//! per-city (keyed by the town centre) and rides the save.
//! WIRED PERKS: the Anglers (fish sell x1.5 in their city's market) and the
//! Provisioners (the inn rests you free). FLAGGED: tillers stall / smiths stock /
//! scholars discount perks, the hall steward + desk, wing-room dressing.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::battle::RoomActor;
use super::play::{CurRoom, Player};
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::gfx::{at, PIXEL_LAYER};
use crate::guildhall::{bundle_progress, req_matches, WINGS};
use crate::input::{Action, ActionState};
use crate::ui::label;

/// One city's restoration (js guildhalls[key]).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct GuildState {
    pub donated: HashMap<String, Vec<i32>>,
    pub done: Vec<String>,
}

/// Every city's hall, keyed "tx,ty" (the town centre) — saved.
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct GuildLedger(pub HashMap<String, GuildState>);

/// Which city's hall you're inside (js currentHallKey; transient).
#[derive(Resource, Default)]
pub struct CurrentHall(pub Option<String>);

/// The open donation window (js guildDonate; transient).
#[derive(Resource, Default)]
pub struct DonateState(pub Option<(usize, Option<usize>, usize)>); // (wing, open bundle, cursor)

/// The current city's live perks, refreshed on room change — shops/inns read this.
#[derive(Resource, Default)]
pub struct CityPerks {
    pub fish_mult: f32,
    pub free_inn: bool,
    pub smith_stock: bool,   // smiths home: gear vendors add ceiling-tier bonus wares
    pub tome_half: bool,     // scholars home: the library's tome shelf (next increment)
    pub produce_stall: bool, // tillers home: the produce stall opens (next increment)
}

/// A wing's donation altar under its crest banner.
#[derive(Component)]
pub struct GuildAltar {
    pub wing: usize,
    pub x: f32,
    pub y: f32,
}

const ALTAR: [&str; 18] = [
    ".KKKKKKKKKK.",
    ".KCCCCCCCCK.",
    ".KCcCCCCcCK.",
    ".KCCWWWWCCK.",
    ".KCcWCCWcCK.",
    ".KCCWWWWCCK.",
    ".KCcCCCCcCK.",
    ".KCCCCCCCCK.",
    "..KCCCCCCK..",
    "...KCCCCK...",
    "....KCCK....",
    ".....KK.....",
    "..KKKKKKKK..",
    ".KDDDDDDDDK.",
    ".KDdDDDDdDK.",
    ".KDDDDDDDDK.",
    "KKKKKKKKKKKK",
    "............",
];

const DESK: [&str; 10] = [
    "KKKKKKKKKKKKKKKKKKKKKKKK",
    "KDDDDDDDDDDDDDDDDDDDDDDK",
    "KDdDDDDWWWDDDDDbDDDDDdDK",
    "KDDDDDDWWWDDDDDDDDDDDDDK",
    "KDDDDDDDDDDDDDDDDDDDDDDK",
    "KddddddddddddddddddddddK",
    "KDDDDDDDDDDDDDDDDDDDDDDK",
    "KDDDDDDDDDDDDDDDDDDDDDDK",
    ".KddddddddddddddddddddK.",
    "..KKKKKKKKKKKKKKKKKKKK..",
];
const DESK_PAL: &[(char, u32)] = &[
    ('K', 0x000000),
    ('D', 0x8a6a3a),
    ('d', 0x6a4a2a),
    ('W', 0xf0ead0), // the open ledger
    ('b', 0xd8b040), // the counter bell
];

/// The hall's PEOPLE and furniture (steward, desk, wing guild members) — swept and
/// re-stood with each hall room, exactly like the altars.
#[derive(Component)]
struct HallCast;

/// The wing ceremony's light burst: a crest-colored star that swells, spins
/// slowly, and fades (Baz: everything guild-touched should feel epic).
#[derive(Component)]
struct WingBurst(u32);

fn spawn_wing_burst(commands: &mut Commands, crest: u32, x: f32, y: f32) {
    let [r, g, b] = [(crest >> 16) as u8, (crest >> 8) as u8, crest as u8];
    let mut tf = at(PLAY_X + x - 5.0, PLAY_Y + y - 5.0, 10.0, 10.0, 12.0);
    tf.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
    commands.spawn((
        Sprite::from_color(Color::srgba_u8(r, g, b, 200), Vec2::new(10.0, 10.0)),
        tf,
        PIXEL_LAYER,
        RoomActor,
        WingBurst(0),
    ));
}

fn burst_tick(mut commands: Commands, mut q: Query<(Entity, &mut WingBurst, &mut Sprite, &mut Transform)>) {
    for (e, mut b, mut spr, mut tf) in &mut q {
        b.0 += 1;
        let t = b.0 as f32;
        let s = 1.0 + t * 0.3;
        tf.scale = Vec3::new(s, s, 1.0);
        tf.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4 + t * 0.04);
        spr.color = spr.color.with_alpha(((1.0 - t / 55.0).max(0.0)) * 0.8);
        if b.0 >= 55 {
            commands.entity(e).despawn();
        }
    }
}

/// A restored wing hangs its crest along the walls (the busy-guild dressing).
const WING_BANNER: [&str; 12] = [
    "KKKKKKKK",
    "KCCCCCCK",
    "KCcCCcCK",
    "KCCCCCCK",
    "KCCWWCCK",
    "KCCWWCCK",
    "KCCCCCCK",
    "KCcCCcCK",
    "KCCCCCCK",
    ".KCCCCK.",
    "..KCCK..",
    "...KK...",
];

/// What the guild members of a THRIVING wing say (Baz: restored wings bustle).
/// Indexed [wing][member] — every line font-safe, busy-guild voice.
const GUILD_CHAT: [[&str; 3]; 5] = [
    [
        "THE SOIL HERE REMEMBERS US.",
        "SEED STOCK COMES BACK ROOM BY ROOM.",
        "SMELL THAT? FRESH EARTH IN A STONE HALL.",
    ],
    [
        "FIRST CATCH IN YEARS HANGS ON THAT WALL.",
        "THE RIVERS KNEW OUR LINES ONCE. THEY WILL AGAIN.",
        "WE OWE THIS WING TO YOU, FRIEND.",
    ],
    [
        "HEAR THE HAMMERS? MUSIC.",
        "THE FORGE DREW BREATH AGAIN THIS MORNING.",
        "BRING US ORE AND WE WILL BRING IT TO LIFE.",
    ],
    [
        "EVERY SHELF REFILLS A LITTLE MORE EACH WEEK.",
        "WHAT WAS WRITTEN IS BEING FOUND AGAIN.",
        "QUIET, PLEASE - HISTORY IS LISTENING.",
    ],
    [
        "THE LONG TABLE FEEDS ALL COMERS AGAIN.",
        "STEW'S ON. IT IS ALWAYS ON NOW.",
        "A FED CITY IS A STANDING CITY.",
    ],
];

/// A stable u32 off the city key string (the steward's identity survives forever).
fn key_seed(key: &str) -> u32 {
    key.bytes().fold(0x57e3_a4d1u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32))
}

pub fn city_key(world: &crate::worldgen::World, rx: i32, ry: i32) -> Option<String> {
    crate::worldgen::towns::town_site_of(world.seed, rx, ry).map(|s| format!("{},{}", s.tx, s.ty))
}

/// Refresh the current city's perks whenever the room changes.
fn perks_tick(
    cur: Res<CurRoom>,
    world: Res<super::play::GameWorld>,
    ledger: Res<GuildLedger>,
    mut perks: ResMut<CityPerks>,
) {
    if !cur.is_changed() && !ledger.is_changed() {
        return;
    }
    let done = city_key(&world.0, cur.rx, cur.ry)
        .and_then(|k| ledger.0.get(&k).map(|g| g.done.clone()))
        .unwrap_or_default();
    let home = |id: &str| crate::guildhall::home_by_id(&done, id);
    perks.fish_mult = if home("anglers") { 1.5 } else { 1.0 };
    perks.free_inn = home("provisioners");
    perks.smith_stock = home("smiths");
    perks.tome_half = home("scholars");
    perks.produce_stall = home("tillers");
}

/// Wing altars stand up with the room (called from spawn_room_chests' wake path).
pub(crate) fn spawn_room_altar(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    blockers: &mut super::room_props::RoomBlockers,
    room: &crate::dungeon::DRoom,
    ledger: &GuildLedger,
    hall: &CurrentHall,
) {
    let Some(gw) = room.gwing else { return };
    let Some(widx) = WINGS.iter().position(|w| w.id == gw) else { return };
    let w = &WINGS[widx];
    let restored = hall
        .0
        .as_ref()
        .and_then(|k| ledger.0.get(k))
        .map(|g| crate::guildhall::wing_home(&g.done, w))
        .unwrap_or(false);
    let pal: &[(char, u32)] = &[
        ('C', if restored { w.crest } else { 0x4a4a52 }),
        ('c', if restored { 0xffffff } else { 0x6a6a72 }),
        ('D', 0x8a6a3a),
        ('d', 0x6a4a2a),
    ];
    let img = images.add(crate::gfx::bake(&ALTAR, pal));
    let (x, y) = (8.0 * 16.0 + 8.0, 2.0 * 16.0);
    let blk = (x - 1.0, y + 2.0, 14.0, 14.0);
    if !blockers.0.contains(&blk) {
        blockers.0.push(blk);
    }
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + x, PLAY_Y + y, 12.0, 18.0, actor_z(y + 16.0)),
        PIXEL_LAYER,
        RoomActor,
        GuildAltar { wing: widx, x, y },
    ));
}

/// Stand each wing room's altar up when the hall's room changes (self-contained
/// watcher — no wake-site churn); clears CurrentHall once you're back outside.
#[allow(clippy::too_many_arguments)]
fn altar_wake(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    ledger: Res<GuildLedger>,
    mut hall: ResMut<CurrentHall>,
    mut donate: ResMut<DonateState>,
    mut woke: Local<Option<(i32, i32, usize)>>,
    altars: Query<Entity, With<GuildAltar>>,
    cast: Query<Entity, With<HallCast>>,
) {
    let Some(run) = &in_dungeon.0 else {
        if hall.0.is_some() {
            hall.0 = None;
            donate.0 = None;
        }
        *woke = None;
        return;
    };
    if run.dungeon.theme.key != "guildhall" {
        return;
    }
    let key = (run.drx, run.dry, run.dungeon.floor);
    if *woke == Some(key) {
        return;
    }
    *woke = Some(key);
    donate.0 = None;
    for e in altars.iter().chain(cast.iter()) {
        commands.entity(e).despawn();
    }
    if let Some(room) = run.dungeon.cur().room(run.drx, run.dry) {
        spawn_room_altar(&mut commands, &mut images, &mut blockers, room, &ledger, &hall);
    }
    let key_s = hall.0.clone().unwrap_or_default();
    let done: Vec<String> = ledger.0.get(&key_s).map(|g| g.done.clone()).unwrap_or_default();
    // THE STEWARD keeps the entry hall (js inc-2 port): one per city forever, at
    // his reception desk, his greeting tracking the restoration.
    if run.drx == 0 && run.dry == 0 {
        let (dx, dy) = (118.0, 58.0);
        let desk = images.add(crate::gfx::bake(&DESK, DESK_PAL));
        let blk = (dx, dy + 2.0, 24.0, 8.0);
        if !blockers.0.contains(&blk) {
            blockers.0.push(blk);
        }
        commands.spawn((
            Sprite::from_image(desk),
            at(PLAY_X + dx, PLAY_Y + dy, 24.0, 10.0, actor_z(dy + 10.0)),
            PIXEL_LAYER,
            RoomActor,
            HallCast,
        ));
        let seed = key_seed(&key_s);
        let line = match crate::guildhall::wings_home(&done) {
            0 => "THE GUILDS ARE SCATTERED. BRING WHAT THE WINGS ASK AND THEY COME HOME.".to_string(),
            5 => "THE HALL STANDS WHOLE. YOU DID THIS. EVERY GUILD REMEMBERS.".to_string(),
            n => format!(
                "{} OF THE FIVE {} HOME. THE OTHER WINGS STILL WAIT ON YOU.",
                ["ONE", "TWO", "THREE", "FOUR"][(n - 1).min(3)],
                if n == 1 { "IS" } else { "ARE" }
            ),
        };
        let (sx, sy) = (dx + 4.0, dy - 12.0); // pressed to the counter's back — talkable across it
        let mut v = crate::actors::villager::Villager::new(sx, sy, seed, line);
        v.identify(format!("g:{key_s}"), crate::people::title_for(seed, "ghall"));
        v.hold_post();
        commands.spawn((
            Sprite::default(),
            at(PLAY_X + sx, PLAY_Y + sy, 16.0, 16.0, actor_z(sy + 16.0)),
            PIXEL_LAYER,
            RoomActor,
            HallCast,
            v,
        ));
    }
    // A THRIVING WING (Baz): once a guild is home, its room BUSTLES — three named
    // members of the order, wandering their restored hall, talking shop.
    if let Some(gw) = run.dungeon.cur().room(run.drx, run.dry).and_then(|r| r.gwing) {
        if let Some(widx) = WINGS.iter().position(|w| w.id == gw) {
            if crate::guildhall::wing_home(&done, &WINGS[widx]) {
                // The guild's livery on every wall (Baz: super fancy, like a busy guild).
                let pal: &[(char, u32)] = &[
                    ('K', 0x000000),
                    ('C', WINGS[widx].crest),
                    ('c', 0xffffff),
                    ('W', 0xf0ead0),
                ];
                let bimg = images.add(crate::gfx::bake(&WING_BANNER, pal));
                for bxp in [56.0, 104.0, 184.0, 232.0] {
                    commands.spawn((
                        Sprite::from_image(bimg.clone()),
                        at(PLAY_X + bxp, PLAY_Y + 18.0, 8.0, 12.0, 3.3),
                        PIXEL_LAYER,
                        RoomActor,
                        HallCast,
                    ));
                }
                for i in 0..3usize {
                    let seed = key_seed(&key_s) ^ (widx as u32 + 1).wrapping_mul(0x9e37_79b9) ^ (i as u32 + 1).wrapping_mul(0x85eb_ca6b);
                    let (mx, my) = (70.0 + i as f32 * 62.0, 88.0 + ((i * 37) % 40) as f32);
                    let mut v = crate::actors::villager::Villager::new(mx, my, seed, GUILD_CHAT[widx][i].to_string());
                    v.identify(format!("gw:{key_s}:{gw}:{i}"), crate::people::name_for(seed).to_string());
                    commands.spawn((
                        Sprite::default(),
                        at(PLAY_X + mx, PLAY_Y + my, 16.0, 16.0, actor_z(my + 16.0)),
                        PIXEL_LAYER,
                        RoomActor,
                        HallCast,
                        v,
                    ));
                }
            }
        }
    }
}

/// PRESS at an altar -> the wing's checklist opens.
fn altar_interact(
    mut input: ResMut<ActionState>,
    mut donate: ResMut<DonateState>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&Player>,
    altars: Query<&GuildAltar>,
) {
    if donate.0.is_some() || !input.pressed(Action::Interact) {
        return;
    }
    let Ok(p) = players.single() else { return };
    let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    for a in &altars {
        let ab = (a.x - 6.0, a.y + 4.0, 24.0, 22.0);
        if hitbox.0 < ab.0 + ab.2 && hitbox.0 + hitbox.2 > ab.0 && hitbox.1 < ab.1 + ab.3 && hitbox.1 + hitbox.3 > ab.1 {
            input.consume(Action::Interact);
            donate.0 = Some((a.wing, None, 0));
            sfx.write(super::sfx::Sfx("open"));
            return;
        }
    }
}

#[derive(Component)]
struct GuildUi;

/// The wing's BUNDLE BOOK (Baz: Coral Island museum scale): the altar opens the
/// wing's list of named bundles; pick one for its checklist. Every filled bundle
/// pays its reward on the spot; the LAST one brings the guild home - perk, wing
/// capstone, and (at five wings) the seal.
#[allow(clippy::too_many_arguments)]
fn donate_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    mut donate: ResMut<DonateState>,
    mut ledger: ResMut<GuildLedger>,
    hall: Res<CurrentHall>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut alloc: ResMut<super::slideout::TreeAlloc>,
    mut rng: ResMut<super::battle::GameRng>,
    mut log: ResMut<super::rewards::LootLog>,
    mut banners: ResMut<super::banners::Banners>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut altars: Query<(&GuildAltar, &mut Sprite)>,
    old_ui: Query<Entity, With<GuildUi>>,
    // Tuple-nested (the flat list sits at Bevy's 16-param cap): the live bindings
    // for the prompts, the player query, and the mouse pointer.
    extras: (Res<crate::input::Bindings>, Query<&Player>, Res<crate::input::Pointer>),
) {
    let Some((widx, bsel, mut cur)) = donate.0 else {
        for e in &old_ui {
            commands.entity(e).despawn();
        }
        return;
    };
    let w = &WINGS[widx];
    let key = hall.0.clone().unwrap_or_else(|| "lost".into());
    let mut dirty = donate.is_changed();
    // Back out one level: checklist -> bundle book -> closed.
    if input.pressed(Action::Slot2) || input.pressed(Action::Pause) {
        input.consume(Action::Slot2);
        input.consume(Action::Pause);
        donate.0 = if bsel.is_some() { Some((widx, None, 0)) } else { None };
        sfx.write(super::sfx::Sfx("open"));
        return;
    }
    // Pad A (Slot1) is the menus' CONFIRM (the shop rule) — read it before the
    // consume sweep eats it. Keyboard ignores it: LMB is Slot1 there, and stray
    // clicks must not donate (row_click handles the mouse properly).
    let a_press = input.pad_present && input.pressed(Action::Slot1);
    // The window OWNS the buttons while open (the menus rule).
    for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
        input.consume(a);
    }
    let rows = match bsel {
        None => w.bundles.len(),
        Some(bi) => w.bundles[bi].reqs.len(),
    };
    if input.pressed(Action::Up) {
        cur = (cur + rows - 1) % rows;
        sfx.write(super::sfx::Sfx("menuMove"));
        dirty = true;
    }
    if input.pressed(Action::Down) {
        cur = (cur + 1) % rows;
        sfx.write(super::sfx::Sfx("menuMove"));
        dirty = true;
    }
    // Shared window geometry (draw + mouse agree by construction).
    let (bw, bh) = (250.0, 64.0 + rows as f32 * 16.0 + if bsel.is_some() { 12.0 } else { 0.0 });
    let bx = PLAY_X + (crate::room::PX_W as f32 - bw) / 2.0;
    let by = PLAY_Y + (crate::room::PX_H as f32 - bh) / 2.0;
    let mut row_click = false;
    for i in 0..rows {
        if extras.2.over(bx + 6.0, by + 30.0 + i as f32 * 16.0 - 2.0, bw - 12.0, 14.0) {
            if extras.2.moved && cur != i {
                cur = i;
                sfx.write(super::sfx::Sfx("menuMove"));
                dirty = true;
            }
            if extras.2.click {
                cur = i;
                row_click = true;
            }
        }
    }
    if donate.0 != Some((widx, bsel, cur)) {
        donate.0 = Some((widx, bsel, cur));
    }
    let confirm = input.pressed(Action::Interact) || input.pressed(Action::MenuConfirm) || a_press || row_click;
    if confirm {
        input.consume(Action::Interact);
        input.consume(Action::MenuConfirm);
    }
    match bsel {
        None => {
            // The book: pick a bundle.
            if confirm && rows > 0 {
                donate.0 = Some((widx, Some(cur), 0));
                sfx.write(super::sfx::Sfx("open"));
                return;
            }
        }
        Some(bi) => {
            let b = &w.bundles[bi];
            let gh = ledger.0.entry(key.clone()).or_default();
            let bdone = gh.done.iter().any(|d| d == b.id);
            if confirm && !bdone {
                let counts = gh.donated.entry(b.id.to_string()).or_insert_with(|| vec![0; b.reqs.len()]);
                let req = &b.reqs[cur];
                if counts[cur] >= req.n {
                    sfx.write(super::sfx::Sfx("tink"));
                } else if let Some(id) = inv
                    .bag
                    .iter()
                    .flatten()
                    .filter_map(|uid| inv.entry(*uid))
                    .map(|e| e.id)
                    .find(|id| req_matches(req.matches, id))
                {
                    inv.remove_one(id);
                    counts[cur] += 1;
                    sfx.write(super::sfx::Sfx("craft"));
                    dirty = true;
                    let (_, _, whole) = bundle_progress(b, counts);
                    if whole {
                        // THE BUNDLE IS FILLED - it pays on the spot.
                        gh.done.push(b.id.to_string());
                        log.add("gh", &format!("{} - COMPLETE", b.name), 1, w.crest, false, true);
                        sfx.write(super::sfx::Sfx("itemget"));
                        let (px, py) = extras.1.single().map(|p| (p.x, p.y)).unwrap_or((144.0, 100.0));
                        if !inv.add_item(b.reward.0, b.reward.1) {
                            super::gather::spawn_pickup(&mut commands, &mut images, b.reward.0, b.reward.1, px + 4.0, py + 18.0, false, None);
                        }
                        if crate::guildhall::wing_home(&gh.done, w) {
                            // THE LAST BUNDLE - the guild comes home. CEREMONY:
                            // the big banner, the fanfare, light bursting off the altar.
                            log.add("gh", &format!("{} RETURN TO THE CITY", w.name), 1, w.crest, false, true);
                            banners.interior(&format!("{} RETURN TO THE CITY", w.name));
                            sfx.write(super::sfx::Sfx("levelup"));
                            for (a, mut spr) in &mut altars {
                                if a.wing == widx {
                                    spr.color = Color::WHITE; // rebake shortcut: the banner lights on re-entry
                                }
                            }
                            if let Some((a, _)) = altars.iter().find(|(a, _)| a.wing == widx) {
                                spawn_wing_burst(&mut commands, w.crest, a.x + 6.0, a.y + 12.0);
                            }
                            grant_loot(w.id, &mut commands, &mut images, &mut inv, &mut alloc, &mut rng, &mut log, &mut sfx, extras.1.single().ok());
                            if crate::guildhall::wings_home(&gh.done) >= WINGS.len() {
                                // THE CAPSTONE: every guild home.
                                if let Some(def) = crate::items::get("guildseal") {
                                    inv.add_item(def.id, 1);
                                }
                                banners.interior("THE GUILDHALL STANDS WHOLE");
                                sfx.write(super::sfx::Sfx("levelup"));
                            }
                            donate.0 = Some((widx, None, 0)); // back out to the finished book
                        }
                    }
                    saves.write(super::save::SaveRequest);
                } else {
                    log.add("gh", "NOTHING IN YOUR BAG FITS THAT LINE", 1, 0xfc8868, false, true);
                    sfx.write(super::sfx::Sfx("tink"));
                }
            } else if confirm {
                sfx.write(super::sfx::Sfx("tink"));
            }
        }
    }
    if !dirty {
        return;
    }
    // --- Redraw (the shop-window idiom: quads + labels under a GuildUi sweep). ---
    for e in &old_ui {
        commands.entity(e).despawn();
    }
    let gh_r = ledger.0.get(&key);
    let done_list: Vec<String> = gh_r.map(|g| g.done.clone()).unwrap_or_default();
    const Z: f32 = crate::gfx::layers::WINDOW;
    let quad = |commands: &mut Commands, c: Color, x: f32, y: f32, qw: f32, qh: f32, z: f32| {
        commands.spawn((Sprite::from_color(c, Vec2::new(qw, qh)), at(x, y, qw, qh, z), PIXEL_LAYER, GuildUi));
    };
    quad(&mut commands, Color::srgba(0.016, 0.024, 0.04, 0.94), bx, by, bw, bh, Z);
    let [cr, cg, cb] = [(w.crest >> 16) as u8, (w.crest >> 8) as u8, w.crest as u8];
    let crest_c = Color::srgb_u8(cr, cg, cb);
    let gold = Color::srgb_u8(0xff, 0xd3, 0x4d);
    // The hall's livery: the guild crest over old gold, rails down the sides,
    // gold ticks in the corners (Baz: the window should feel like it MATTERS).
    quad(&mut commands, crest_c, bx, by, bw, 1.0, Z + 0.01);
    quad(&mut commands, gold, bx + 4.0, by + 2.0, bw - 8.0, 1.0, Z + 0.01);
    quad(&mut commands, crest_c, bx, by + bh - 1.0, bw, 1.0, Z + 0.01);
    quad(&mut commands, gold, bx + 4.0, by + bh - 3.0, bw - 8.0, 1.0, Z + 0.01);
    quad(&mut commands, Color::srgba_u8(cr, cg, cb, 120), bx, by, 1.0, bh, Z + 0.01);
    quad(&mut commands, Color::srgba_u8(cr, cg, cb, 120), bx + bw - 1.0, by, 1.0, bh, Z + 0.01);
    for (tx, ty) in [
        (bx + 2.0, by + 5.0),
        (bx + bw - 5.0, by + 5.0),
        (bx + 2.0, by + bh - 6.0),
        (bx + bw - 5.0, by + bh - 6.0),
    ] {
        quad(&mut commands, gold, tx, ty, 3.0, 1.0, Z + 0.01);
    }
    // Heraldic diamonds flank whichever title each view draws at by+6.
    let medallions = |commands: &mut Commands, tx0: f32, tw: f32| {
        for mx in [tx0 - 12.0, tx0 + tw + 7.0] {
            let mut mtf = at(mx, by + 7.0, 5.0, 5.0, Z + 0.02);
            mtf.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
            commands.spawn((Sprite::from_color(crest_c, Vec2::new(5.0, 5.0)), mtf, PIXEL_LAYER, GuildUi));
        }
    };
    let pad = input.pad_present;
    // On the pad, Interact IS D-pad up — but the book turns the D-pad into arrows
    // (set_dpad_dirs), so GIVE/OPEN prompt the confirm button there instead.
    let give_act = if pad { Action::Slot1 } else { Action::Interact };
    match bsel {
        None => {
            // THE BUNDLE BOOK: the wing's named sets, each with its running tally.
            let title_w = crate::gfx::font::measure(w.name) as f32;
            let tx0 = (bx + (bw - title_w) / 2.0).floor();
            label(&mut commands, &mut images, w.name, tx0, by + 6.0, w.crest, Z + 0.02, GuildUi);
            medallions(&mut commands, tx0, title_w);
            // Bundle pips: one per set, lit as each fills.
            let nb = w.bundles.iter().filter(|b| done_list.iter().any(|d| d == b.id)).count();
            let pips_w = w.bundles.len() as f32 * 8.0 - 3.0;
            for i in 0..w.bundles.len() {
                let px2 = (bx + (bw - pips_w) / 2.0 + i as f32 * 8.0).floor();
                let c = if i < nb { crest_c } else { Color::srgb_u8(0x2a, 0x2a, 0x32) };
                quad(&mut commands, c, px2, by + 25.0, 5.0, 3.0, Z + 0.02);
            }
            let home = crate::guildhall::wing_home(&done_list, w);
            let sub = if home { "THE WING IS RESTORED" } else { w.desc };
            let sub_w = crate::gfx::font::measure(sub) as f32;
            label(&mut commands, &mut images, sub, (bx + (bw - sub_w) / 2.0).floor(), by + 16.0, if home { 0x7ee08a } else { 0x8a8a92 }, Z + 0.02, GuildUi);
            for (i, b) in w.bundles.iter().enumerate() {
                let y = by + 30.0 + i as f32 * 16.0;
                let bdone = done_list.iter().any(|d| d == b.id);
                let on = i == cur;
                if on {
                    quad(&mut commands, Color::srgba(0.98, 0.88, 0.66, 0.12), bx + 6.0, y - 2.0, bw - 12.0, 14.0, Z + 0.015);
                }
                let col = if bdone { 0x7ee08a } else if on { 0xfcfcfc } else { 0xb4b4bc };
                label(&mut commands, &mut images, b.name, bx + 12.0, y + 1.0, col, Z + 0.02, GuildUi);
                let counts = gh_r.and_then(|g| g.donated.get(b.id)).cloned().unwrap_or_default();
                let (have, need, _) = bundle_progress(b, &counts);
                let tag = if bdone { "COMPLETE".to_string() } else { format!("{have}/{need}") };
                let tw = crate::gfx::font::measure(&tag) as f32;
                let tcol = if bdone { 0x7ee08a } else if have > 0 { 0xffd34d } else { 0x5a5a62 };
                label(&mut commands, &mut images, &tag, bx + bw - 12.0 - tw, y + 1.0, tcol, Z + 0.02, GuildUi);
            }
            let hint = format!(
                "{} OPEN   {} CLOSE",
                extras.0.prompt(give_act, pad),
                extras.0.prompt(Action::Slot2, pad)
            );
            let hw = crate::gfx::font::measure(&hint) as f32;
            label(&mut commands, &mut images, &hint, (bx + (bw - hw) / 2.0).floor(), by + bh - 14.0, 0x6c6c74, Z + 0.02, GuildUi);
        }
        Some(bi) => {
            let b = &w.bundles[bi];
            let bdone = done_list.iter().any(|d| d == b.id);
            let counts2 = gh_r.and_then(|g| g.donated.get(b.id)).cloned().unwrap_or_else(|| vec![0; b.reqs.len()]);
            let title_w = crate::gfx::font::measure(b.name) as f32;
            let tx0 = (bx + (bw - title_w) / 2.0).floor();
            label(&mut commands, &mut images, b.name, tx0, by + 6.0, w.crest, Z + 0.02, GuildUi);
            medallions(&mut commands, tx0, title_w);
            let sub = if bdone { "THE BUNDLE IS FILLED" } else { w.name };
            let sub_w = crate::gfx::font::measure(sub) as f32;
            label(&mut commands, &mut images, sub, (bx + (bw - sub_w) / 2.0).floor(), by + 16.0, if bdone { 0x7ee08a } else { 0x8a8a92 }, Z + 0.02, GuildUi);
            for (i, req) in b.reqs.iter().enumerate() {
                let y = by + 30.0 + i as f32 * 16.0;
                let full = counts2.get(i).copied().unwrap_or(0) >= req.n;
                let on = i == cur && !bdone;
                if on {
                    quad(&mut commands, Color::srgba(0.98, 0.88, 0.66, 0.12), bx + 6.0, y - 2.0, bw - 12.0, 14.0, Z + 0.015);
                }
                let col = if full { 0x7ee08a } else if on { 0xfcfcfc } else { 0xb4b4bc };
                label(&mut commands, &mut images, req.label, bx + 12.0, y + 1.0, col, Z + 0.02, GuildUi);
                let has = !full
                    && inv
                        .bag
                        .iter()
                        .flatten()
                        .filter_map(|uid| inv.entry(*uid))
                        .any(|e| req_matches(req.matches, e.id));
                let give_key = extras.0.prompt(give_act, pad);
                let tag = format!(
                    "{}/{}{}",
                    counts2.get(i).copied().unwrap_or(0),
                    req.n,
                    if !full && has { format!("  {give_key} GIVE") } else { String::new() }
                );
                let tw = crate::gfx::font::measure(&tag) as f32;
                let tcol = if full { 0x7ee08a } else if has { 0xffd34d } else { 0x5a5a62 };
                label(&mut commands, &mut images, &tag, bx + bw - 12.0 - tw, y + 1.0, tcol, Z + 0.02, GuildUi);
            }
            if let Some(def) = crate::items::get(b.reward.0) {
                let rline = format!("REWARD: {} X{}", def.name.to_uppercase(), b.reward.1);
                let rw2 = crate::gfx::font::measure(&rline) as f32;
                label(&mut commands, &mut images, &rline, (bx + (bw - rw2) / 2.0).floor(), by + bh - 26.0, if bdone { 0x4a4a52 } else { 0xffd34d }, Z + 0.02, GuildUi);
            }
            let hint = format!(
                "{} GIVE   {} BACK",
                extras.0.prompt(give_act, pad),
                extras.0.prompt(Action::Slot2, pad)
            );
            let hw = crate::gfx::font::measure(&hint) as f32;
            label(&mut commands, &mut images, &hint, (bx + (bw - hw) / 2.0).floor(), by + bh - 14.0, 0x6c6c74, Z + 0.02, GuildUi);
        }
    }
}

/// The guild's thank-you (js grantGuildLoot; smiths use the loot roll until the
/// procedural forge ports, provisioners feed you potions until cooking lands).
#[allow(clippy::too_many_arguments)]
fn grant_loot(
    id: &str,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    inv: &mut crate::inventory::PlayerInv,
    alloc: &mut super::slideout::TreeAlloc,
    rng: &mut super::battle::GameRng,
    log: &mut super::rewards::LootLog,
    sfx: &mut MessageWriter<super::sfx::Sfx>,
    player: Option<&Player>,
) {
    let (px, py) = player.map(|p| (p.x, p.y)).unwrap_or((144.0, 100.0));
    let drop = |iid: &'static str, qty: i32, commands: &mut Commands, images: &mut Assets<Image>, inv: &mut crate::inventory::PlayerInv| {
        if !inv.add_item(iid, qty) {
            super::gather::spawn_pickup(commands, images, iid, qty, px + 4.0, py + 18.0, false, None);
        }
    };
    match id {
        "tillers" => {
            drop("pumpkinseed", 2, commands, images, inv);
            drop("cranberryseed", 2, commands, images, inv);
            drop("tomatoseed", 2, commands, images, inv);
            log.add("gh", "A PACKET OF RARE SEEDS", 1, 0x7ee08a, false, true);
        }
        "anglers" => {
            drop("luckyhook", 1, commands, images, inv);
            log.add("gh", "THE ANGLERS LUCKY HOOK", 1, 0x7090d8, false, true);
        }
        "smiths" => {
            let (iid, qty) = crate::items::roll_loot(1.6, 0.0, || rng.0.next_f64());
            drop(iid, qty, commands, images, inv);
            log.add("gh", "A MASTERWORK PRIZE", 1, 0xe0903a, false, true);
        }
        "scholars" => {
            alloc.points += 1;
            log.add("gh", "A LESSON LEARNED - +1 SKILL POINT", 1, 0xc878ff, false, true);
            sfx.write(super::sfx::Sfx("levelup"));
        }
        "provisioners" => {
            drop("potion", 2, commands, images, inv);
            drop("greaterpotion", 1, commands, images, inv);
            log.add("gh", "A FEAST FOR THE ROAD", 1, 0xffd34d, false, true);
        }
        _ => {}
    }
}

/// The tillers' PRODUCE STALL — a dynamic storefront in the city's market room.
/// door_enter and prompt_tick treat it like any town doorway (its x,y is the door).
#[derive(Component)]
pub struct ProduceStall {
    pub x: f32,
    pub y: f32,
}

/// THE PRODUCE STALL OPENS (tillers perk, the js stallspot idea): once the Tillers
/// are home, the city's Market room grows the farmstall storefront — worldgen
/// untouched, the stall literally opens with the guild. Deterministic spot: the
/// first 3x3 patch of open ground with breathing room from the laid-out market.
#[allow(clippy::too_many_arguments)]
fn stall_wake(
    mut commands: Commands,
    cur: Res<CurRoom>,
    slide: Res<super::play::SlideState>,
    world: Res<super::play::GameWorld>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    grid: Res<super::play::CurGrid>,
    ledger: Res<GuildLedger>,
    art: Res<crate::actors::props::PropArt>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut woke: Local<Option<(i32, i32)>>,
    stalls: Query<Entity, With<ProduceStall>>,
    parents: Query<&ChildOf>,
) {
    if in_dungeon.0.is_some() || inside.0.is_some() {
        *woke = None; // interiors re-stand the stall on the way back out
        return;
    }
    if *woke == Some((cur.rx, cur.ry)) && !ledger.is_changed() {
        return;
    }
    *woke = Some((cur.rx, cur.ry));
    let outgoing = slide.outgoing_root();
    for e in &stalls {
        if outgoing.is_some() && parents.get(e).ok().map(|p| p.parent()) == outgoing {
            continue;
        }
        commands.entity(e).despawn();
    }
    if !matches!(world.0.town_role(cur.rx, cur.ry), Some(crate::worldgen::towns::TownRole::Market)) {
        return;
    }
    let open = city_key(&world.0, cur.rx, cur.ry)
        .and_then(|k| ledger.0.get(&k).map(|g| crate::guildhall::home_by_id(&g.done, "tillers")))
        .unwrap_or(false);
    if !open {
        return;
    }
    let Some(front) = art.fronts.get("farmstall").cloned() else { return };
    let ents = world.0.room_entities(cur.rx, cur.ry);
    let mut spot: Option<(i32, i32)> = None;
    'scan: for r in 4..crate::room::ROWS - 4 {
        for c in 2..crate::room::COLS - 4 {
            if !(0..3).all(|dr| (0..3).all(|dc| grid.0.code_at(c + dc, r + dr) == '.')) {
                continue;
            }
            let (px, py) = ((c * crate::room::TILE + 16) as f32, (r * crate::room::TILE + 32) as f32);
            if ents.iter().all(|e| {
                let (dx, dy) = (e.x as f32 - px, e.y as f32 - py);
                dx * dx + dy * dy >= 44.0 * 44.0
            }) {
                spot = Some((c, r));
                break 'scan;
            }
        }
    }
    let Some((c, r)) = spot else { return };
    // (x, y) is the DOOR anchor, exactly like a "town" entity's.
    let (x, y) = ((c * crate::room::TILE + 16) as f32, (r * crate::room::TILE + 32) as f32);
    let blk = (x - 12.0, y - 28.0, 40.0, 42.0);
    if !blockers.0.contains(&blk) {
        blockers.0.push(blk);
    }
    commands.spawn((
        Sprite::from_image(front),
        at(PLAY_X + x - 16.0, PLAY_Y + y - 32.0, 48.0, 48.0, actor_z(y + 16.0)),
        PIXEL_LAYER,
        RoomActor,
        ProduceStall { x, y },
    ));
}

pub struct GuildhallPlugin;
impl Plugin for GuildhallPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuildLedger>()
            .init_resource::<CurrentHall>()
            .init_resource::<DonateState>()
            .init_resource::<CityPerks>()
            .add_systems(
                bevy::app::FixedUpdate,
                (perks_tick, burst_tick, super::hall_exterior::hall_wake, stall_wake, altar_wake, altar_interact.before(super::talk::talk_tick).after(altar_wake), donate_tick.after(altar_interact).before(super::flute::flute_tick))
                    .before(super::play::EndTick)
                    .run_if(super::screen::playing),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn altar_is_rectangular() {
        for r in ALTAR {
            assert_eq!(r.chars().count(), 12, "altar row width");
        }
    }
}
