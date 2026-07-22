//! debug_shot.rs — headless visual verification: `WRIFT_SHOT=<scene> cargo run` freezes the
//! game, stages a scene, screenshots the window to `WRIFT_SHOT_PATH` (default
//! `wriftshot.png`), and exits. Rotation, anchor, layering, and layout bugs get EYEBALLED
//! instead of debated on paper.
//!
//! Scenes: `swings` (or `1`) = every sword/axe facing at three tinted sweep phases;
//! `codex` = the codex MAP tab over a seeded visited set; `mobs` = the codex MOBS tab;
//! `pause` = the pause menu (WRIFT_TAB picks its tab); `title` = the start menu;
//! `newgame` = drives NEW GAME from the title through the LoadSlot world swap (the
//! loader's smoke test — a clean exit is the pass).
//!
//! Inert unless the env var is set — the plugin adds nothing to a normal run.

use super::battle::RoomActor;
use super::codex::{self, CodexChrome, CodexState, TabId, TABS};
use super::play::{HeroArt, Visited};
use super::screen::Screen;
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::actors::attacks::{
    axe_tick, axe_visual, axe_z, sword_visual, swing_tick, swing_z, AttackArt, AxeSwipe, Swing,
    FACE,
};
use crate::actors::goblin::GoblinArt;
use crate::gfx::{at, PIXEL_LAYER};
use crate::combat::Tool;
use crate::input::{ActionState, Bindings};
use crate::ui::label;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

pub struct DebugShotPlugin;

impl Plugin for DebugShotPlugin {
    fn build(&self, app: &mut App) {
        let Ok(scene) = std::env::var("WRIFT_SHOT") else {
            return;
        };
        // The game BOOTS into Screen::Title now — every play-side scene forces Play first
        // (codex/stages/roster/tableau set their own screen state and don't need it).
        match scene.as_str() {
            "title" => &mut *app, // the default state IS the title
            "codex" | "mobs" => app.add_systems(PostStartup, (super::title::cleanup_title, open_codex_shot).chain()),
            "stages" => app.add_systems(PostStartup, (super::title::cleanup_title, spawn_stages).chain()),
            "slideout" | "skills" | "craft" => app.add_systems(PostStartup, (set_play, open_slideout_shot).chain()),
            "drops" => app.add_systems(PostStartup, (set_play, spawn_drops).chain()),
            "roster" => app.add_systems(PostStartup, (super::title::cleanup_title, spawn_roster).chain()),
            "pause" => app.add_systems(PostStartup, set_play).add_systems(Update, pause_menu_stage),
            // The DUNGEON FLOOR MAP: a generated dungeon with a walked trail, codex open.
            "dmap" => app.add_systems(PostStartup, set_play).add_systems(Update, dmap_stage),
            "newgame" => app.add_systems(Update, newgame_stage),
            "creator" => app.add_systems(Update, creator_stage),
            "death" => app.add_systems(PostStartup, set_play).add_systems(Update, death_stage),
            "town" => app.add_systems(PostStartup, set_play).add_systems(Update, town_stage),
            "interior" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, interior_stage)),
            "shop" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, shop_stage)),
            "talk" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, talk_stage)),
            // Inside a shard dungeon: town_stage lands at a WRIFT_TOWN shard site, then
            // dungeon_stage walks to the mouth + presses INTERACT.
            "dungeon" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, dungeon_stage)),
            "descent" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, descent_stage)),
            "fanfare" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, fanfare_stage)),
            // Inside a city's guildhall: town_stage lands at a CITY (WRIFT_TOWN), this walks
            // the hero through its doors; WRIFT_WALK strolls the wings ("up,400" reaches the
            // provisioners altar), WRIFT_DONATE=1 then presses at it for the checklist shot.
            "guildhall" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, hall_stage)),
            // Mid-cast at a lakeside: rod to a slot, face the water, cast (bobber + prompt).
            "fish" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, fish_stage)),
            // A staged plot beside the hero: dry/wet beds, every growth stage, the hoe
            // reticle (point WRIFT_TOWN at a wild grass room).
            "farm" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, farm_stage)),
            // Unique trinkets: the wispstone's orbiting grave-wisp + a boomerang
            // caught mid-flight (wild grass room).
            "uniq" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, uniq_stage)),
            // Treasure digging (wild grass room): reads a chart (X + toast), plants an
            // X in THIS room, digs the mound up. "digmap" opens the codex map's X pins.
            "dig" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, dig_stage)),
            "digmap" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, dig_stage, codex_after_stage)),
            // Magic: a firebolt mid-flight, a bush ablaze, the wildfire creeping.
            "magic" => app.add_systems(PostStartup, set_play).add_systems(Update, magic_stage),
            // Caravan: the roadside wagon + shopkeeper standing by the hero.
            "caravan" => app.add_systems(PostStartup, set_play).add_systems(Update, caravan_stage),
            // Procgen: a rolled weapon slotted + a full rolled armor set worn.
            "procgen" => app.add_systems(PostStartup, set_play).add_systems(Update, procgen_stage),
            // Loot goblin: the gold goblin mid-flee near the hero.
            "lootgob" => app.add_systems(PostStartup, set_play).add_systems(Update, lootgob_stage),
            // Newly-ported roster mobs, lined up (art + spawn check).
            "newmobs" => app.add_systems(PostStartup, set_play).add_systems(Update, newmobs_stage),
            // Elites: two promoted mobs with auras + name tags, lined up.
            "elite" => app.add_systems(PostStartup, set_play).add_systems(Update, elite_stage),
            // Grapple: the claw in flight with its taut rope back to the hero.
            "grapple" => app.add_systems(PostStartup, set_play).add_systems(Update, grapple_stage),
            // Worn armor: the hero in crested helm, plate, and greaves.
            "worn" => app.add_systems(PostStartup, set_play).add_systems(Update, worn_stage),
            // The shield: guard raised right as an archer's arrow streaks in.
            "shield" => app.add_systems(PostStartup, set_play).add_systems(Update, shield_stage),
            // The bow: four arrows loosed in four facings, caught mid-flight.
            "bow" => app.add_systems(PostStartup, set_play).add_systems(Update, bow_stage),
            // The opening cinematic (WRIFT_CUT_T jumps the clock to a phase).
            "cutscene" => app.add_systems(PostStartup, set_play).add_systems(Update, cutscene_stage),
            // The hidden side-view chamber (gravity + ladders): drops straight in.
            "side" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, side_stage)),
            // The singing stone (point WRIFT_TOWN at one, e.g. "1,-9" on seed 1337):
            // shows it standing; WRIFT_SING=1 plays the Song of Opening at it.
            "song" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, song_stage)),
            // The cracked-wall loop (point WRIFT_TOWN at a room with a crack, e.g.
            // "2,-9" c4r0 on seed 1337): bombs the fissure, the cave door is carved;
            // WRIFT_CAVE_STAY=1 holds outside for the door shot, else PRESS descends.
            "cave" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, cave_stage)),
            // The Cooking Fire loop (point WRIFT_TOWN at wild grass): seeds a larder,
            // places the fire, opens its station window; WRIFT_COOKCRAFT=1 crafts a
            // roast; WRIFT_EAT=1 eats one instead (the HUD buff row is the proof).
            "cook" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, cook_stage)),
            // A seeded 3-slot quest log (sidebar list; open the codex MAP for its pins).
            "quest" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, quest_stage)),
            "questmap" => {
                app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, quest_stage, codex_after_stage))
            }
            // Mid-song: the compass, motes, and the replay banner (SONG OF RETURNING).
            "flute" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, flute_stage)),
            "dev" => app.add_systems(PostStartup, set_play).add_systems(Update, (town_stage, dev_stage)),
            _ => app.add_systems(PostStartup, (super::title::cleanup_title, spawn_tableau).chain()),
        };
        app.add_systems(Update, capture);
    }
}

/// Stage the codex: seed a plausible explored region, then open straight onto the tab.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn open_codex_shot(
    mut commands: Commands,
    mut next: ResMut<NextState<Screen>>,
    mut cx: ResMut<CodexState>,
    chrome: Query<Entity, With<CodexChrome>>,
    bindings: Res<Bindings>,
    state: Res<ActionState>,
    mut images: ResMut<Assets<Image>>,
    mut visited: ResMut<Visited>,
    mut gather: ResMut<super::gather::GatherState>,
    mut people: ResMut<super::talk::PeopleLedger>,
    mut people_dex: ResMut<super::codex::people_tab::PeopleDex>,
    mut learned: ResMut<super::flute::LearnedSongs>,
    mut guilds: ResMut<super::guildhall::GuildLedger>,
    mut names: ResMut<super::banners::TownNames>,
    mut relics: ResMut<super::dungeon::Relics>,
) {
    // GUILDS shots: two halls found — one three wings deep, one barely begun.
    let mut a = super::guildhall::GuildState::default();
    for w in crate::guildhall::WINGS.iter().take(3) {
        a.done.push(w.id.to_string());
    }
    let w3 = &crate::guildhall::WINGS[3];
    a.donated.insert(w3.id.to_string(), vec![w3.reqs[0].n - 1]);
    guilds.0.insert("10,6".into(), a);
    guilds.0.insert("-4,12".into(), super::guildhall::GuildState::default());
    names.0.insert("10,6".into(), "Brightmoor".into());
    names.0.insert("-4,12".into(), "Coldwell".into());
    // WRIFTHEART shots: half the shards home.
    for r in crate::relics_data::LIST.iter().take(6) {
        relics.0.insert(r.biome.to_string());
    }
    // SONGS shots: half the songbook learned, half still rumours.
    for id in ["returning", "stormcall", "greensong", "canticle"] {
        learned.0.insert(id);
    }
    // WRIFT_ROW=<n> pre-sets the PEOPLE roster cursor (row 1 = the first person).
    if let Ok(n) = std::env::var("WRIFT_ROW").map(|v| v.parse().unwrap_or(0)) {
        people_dex.cur = n;
    }
    for x in -3..=3 {
        for y in -2..=2 {
            visited.0.insert((x, y));
        }
    }
    // LORE shots: a browsable shelf needs some found tomes.
    for b in crate::lore_books::BOOKS.iter().take(9) {
        gather.tomes.insert(b.id);
    }
    // PEOPLE shots: a small circle of friends across two towns + a wanderer.
    for (i, (town, pts, know)) in [
        (Some("SILVERVALE"), 780, true),
        (Some("SILVERVALE"), 350, true),
        (Some("SILVERVALE"), 40, false),
        (Some("OAKDALE"), 210, false),
        (Some("OAKDALE"), 0, false),
        (None, 120, false),
    ]
    .into_iter()
    .enumerate()
    {
        let seed = 0x5eed_0001u32.wrapping_mul(i as u32 + 1);
        people.0.insert(
            format!("shot:{i}"),
            super::talk::PersonRec {
                pts,
                last_chat: if pts > 0 { 2 } else { -1 },
                name: if i == 0 { crate::people::title_for(seed, "inn") } else { crate::people::name_for(seed).to_string() },
                seed,
                town: town.map(String::from),
                know_bday: know,
                know_love: know,
                ..Default::default()
            },
        );
    }
    // WRIFT_TAB=<title> opens any codex tab by name (ITEMS, STATS, ...); the legacy
    // WRIFT_SHOT=mobs shorthand still works.
    let tab = std::env::var("WRIFT_TAB")
        .ok()
        .and_then(|t| TABS.iter().position(|d| d.title.eq_ignore_ascii_case(&t)))
        .or_else(|| {
            let want = match std::env::var("WRIFT_TAB").as_deref() {
                Ok("guilds") => TabId::Guilds,
                Ok("wriftheart") => TabId::Wriftheart,
                _ if std::env::var("WRIFT_SHOT").as_deref() == Ok("mobs") => TabId::Mobs,
                _ => TabId::Map,
            };
            TABS.iter().position(|t| t.id == want)
        });
    codex::open(&mut commands, &mut next, &mut cx, &chrome, &bindings, &state, &mut images, tab);
}

/// Stage the slide-out over a stocked bag (opens fully slid-in on frame 1).
fn open_slideout_shot(
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut state: ResMut<crate::input::ActionState>,
    mut alloc: ResMut<super::slideout::TreeAlloc>,
    mut so: ResMut<super::slideout::SlideOut>,
) {
    if std::env::var("WRIFT_SHOT").as_deref() == Ok("craft") {
        so.tab = 1; // the CRAFT page
    }
    for (id, n) in [("wood", 14), ("stone", 6), ("fiber", 9), ("herb", 2), ("copper", 1), ("potion", 3)] {
        inv.add_item(id, n);
    }
    if std::env::var("WRIFT_SHOT").as_deref() == Ok("skills") {
        // Pre-allocate a short war-branch path so lit links/halos show in the shot.
        for id in ["warm1", "warm2", "warn1"] {
            if let Some(i) = crate::skilltree::nodes().iter().position(|n| n.id == id) {
                alloc.taken.insert(i);
            }
        }
        alloc.points = 4;
        state.press_for_test(crate::input::Action::SkillTree);
    } else {
        state.press_for_test(crate::input::Action::Inventory);
    }
}

/// Skip the title straight into play (scenes stage the WORLD, not the start menu).
fn set_play(mut next: ResMut<NextState<Screen>>) {
    next.set(Screen::Play);
}

/// Stage the DUNGEON FLOOR MAP: generate a castle-sized dungeon, mark a walked trail
/// visited, stand the run up, then open the codex MAP tab over it.
fn dmap_stage(
    mut in_dungeon: ResMut<super::dungeon::InDungeon>,
    mut state: ResMut<ActionState>,
    clock: Res<super::room_render::FrameClock>,
    mut done: Local<bool>,
) {
    if !*done && clock.0 >= 20 {
        *done = true;
        let theme = std::env::var("WRIFT_DTHEME").unwrap_or_else(|_| "castle".into());
        let mut d = crate::dungeon::generate(0xd00d, &theme, &crate::dungeon::GenOpts { floors: Some(4), ..Default::default() });
        // Walk a plausible trail: the start room + half its floor stands visited.
        let keys: Vec<(i32, i32)> = d.cur().order.clone();
        let fl = d.cur_mut();
        for (i, k) in keys.iter().enumerate() {
            if (i % 2 == 0 || i < 3)
                && let Some(r) = fl.rooms.get_mut(k)
            {
                r.visited = true;
            }
        }
        let start = keys.first().copied().unwrap_or((0, 0));
        in_dungeon.0 = Some(super::dungeon::DungeonRun {
            dungeon: d,
            drx: start.0,
            dry: start.1,
            return_pos: (0, 0, 144.0, 120.0),
            entrance_key: "shot".into(),
            biome: None,
            is_final: false,
            arena: None,
            mini: None,
            rift: 0,
            rift_base: 0,
        });
    }
    if *done && clock.0 == 30 {
        state.press_for_test(crate::input::Action::Map); // open the codex on the MAP tab
    }
}

/// Stock the bag and strike the hero dead — the YOU DIED sequence takes it from there.
/// Once the menu is up (t>=52), keep confirming CONTINUE: a clean respawn back to Play
/// (no panic, world re-stood) is the smoke-test pass.
fn death_stage(
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut state: ResMut<ActionState>,
    screen: Res<State<Screen>>,
    mut players: Query<&mut crate::combat::Health, With<super::play::Player>>,
    mut done: Local<bool>,
    mut cool: Local<u32>,
) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    if *screen.get() == Screen::Dead {
        state.press_for_test(crate::input::Action::Slot1);
        *cool = if std::env::var("WRIFT_HOLD").is_ok() { u32::MAX } else { 30 }; // WRIFT_HOLD: freeze on the menu for the shot
        return;
    }
    if *done {
        return;
    }
    let Ok(mut h) = players.single_mut() else { return };
    for (id, n) in [("wood", 5), ("stone", 3), ("herb", 2)] {
        inv.add_item(id, n);
    }
    h.hp = 0;
    *done = true;
}

/// Swap the world onto a town room (WRIFT_TOWN="rx,ry" overrides; default a seed-1337
/// market) so the townEntities dressing gets eyeballed.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn town_stage(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut ctx: super::save::SaveCtx,
    mut swap: super::title::loader::SwapCtx,
    caves: Res<super::caves::CrackCaves>,
    songs_opened: Res<super::caves::OpenedSongstones>,
    actors: Query<Entity, With<RoomActor>>,
    mut players: Query<&mut super::play::Player>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *done = true;
    let (rx, ry) = std::env::var("WRIFT_TOWN")
        .ok()
        .and_then(|s| {
            let (a, b) = s.split_once(',')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        })
        .unwrap_or((-4, -10));
    super::title::loader::swap_world_room(&mut commands, &mut images, &mut swap, &mut ctx, &caves, &songs_opened, &actors, rx, ry, None);
    // WRIFT_CLOCK=frames pins time-of-day (dusk/night shadow + lighting shots).
    if let Some(c) = std::env::var("WRIFT_CLOCK").ok().and_then(|s| s.parse().ok()) {
        ctx.clock.0 = c;
    }
    // Announce like a real arrival (the swap itself anchors silently).
    let world_ref = &swap.world.0;
    let (mut banners, mut names) = (swap.banners, ctx.town_names);
    banners.room_entered(world_ref, &mut names, rx, ry);
    if let Ok(mut p) = players.single_mut() {
        // WRIFT_POS="x,y" pins the hero (water-bank reflection shots etc).
        let pos = std::env::var("WRIFT_POS").ok().and_then(|s| {
            let (a, b) = s.split_once(',')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        });
        let (px, py) = pos.unwrap_or(((crate::room::PX_W / 2 - 8) as f32, (crate::room::PX_H - 40) as f32));
        p.x = px;
        p.y = py;
    }
}

/// After town_stage lands the room, stand on the first building's doorstep and press
/// INTERACT — the shot captures whatever interior WRIFT_TOWN's first building opens.
fn interior_stage(
    mut state: ResMut<ActionState>,
    inside: Res<super::interior::Inside>,
    world: Res<super::play::GameWorld>,
    cur: Res<super::play::CurRoom>,
    mut players: Query<&mut super::play::Player>,
    mut cool: Local<u32>,
) {
    if inside.0.is_some() {
        return;
    }
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    *cool = 5;
    let Some(b) = world.0.room_entities(cur.rx, cur.ry).into_iter().find(|e| e.kind == "town") else { return };
    if let Ok(mut p) = players.single_mut() {
        p.x = (b.x - 4 + 12 - 8) as f32; // door-zone centre
        p.y = (b.y + 8) as f32;
        state.press_for_test(crate::input::Action::Interact);
    }
}

/// After town_stage lands the room, walk into the first VENDOR building, stand at its
/// counter and press INTERACT — the shot captures the BUY window over the scene.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn shop_stage(
    mut state: ResMut<ActionState>,
    screen: Res<State<Screen>>,
    inside: Res<super::interior::Inside>,
    world: Res<super::play::GameWorld>,
    cur: Res<super::play::CurRoom>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut players: Query<&mut super::play::Player>,
    mut cool: Local<u32>,
) {
    if *screen.get() == Screen::Shop {
        return; // staged — the window is up
    }
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    *cool = 5;
    let Ok(mut p) = players.single_mut() else { return };
    match &inside.0 {
        None => {
            // A storefront whose interior actually sells (skip plain houses).
            let vendor = world.0.room_entities(cur.rx, cur.ry).into_iter().find(|e| {
                e.kind == "town"
                    && crate::actors::interiors_art::INTERIORS.iter().any(|d| d.kind == e.sub && !d.stock.is_empty())
            });
            let Some(b) = vendor else { return };
            p.x = (b.x - 4 + 12 - 8) as f32; // door-zone centre
            p.y = (b.y + 8) as f32;
            state.press_for_test(crate::input::Action::Interact);
        }
        Some(st) => {
            inv.money = 500; // a shot-worthy purse: some prices green, some red
            let Some((_, _, zx, zy, zw, zh)) = st.def.interact.iter().find(|(k, ..)| *k == "shop") else { return };
            p.x = (zx + zw / 2 - 8) as f32;
            p.y = (zy + zh / 2 - 8) as f32;
            state.press_for_test(crate::input::Action::Interact);
        }
    }
}

/// After town_stage lands the room, sidle up to the first named villager and press
/// INTERACT — the shot catches the TALK/GIVE chooser. WRIFT_GIFT=1 drives on into the
/// gift picker (stocks the bag, arrows to GIVE, confirms).
/// Stand at the monument's mouth and press INTERACT (the enter flow does the rest).
/// WRIFT_WALK="dir,frames" then walks inside (e.g. "up,90" crosses into the next room).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn dungeon_stage(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<ActionState>,
    dungeon: Res<super::dungeon::InDungeon>,
    mut players: Query<&mut super::play::Player>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut cool: Local<u32>,
    mut walked: Local<u32>,
    mut mimicked: Local<bool>,
) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    if dungeon.0.is_none() {
        *cool = 4;
        let Ok(mut p) = players.single_mut() else { return };
        p.x = 144.0; // the mouth's door zone (monument tile 9,4 -> door at 140,72)
        p.y = 78.0;
        state.press_for_test(crate::input::Action::Interact);
        return;
    }
    // WRIFT_MIMIC=1: plant a fake chest up the start room — pair with WRIFT_WALK="up,N"
    // to stroll into its spring radius and shoot the chomp. WRIFT_MIMIC=sprung plants it
    // already-sprung at mid-range, so the tongue lash fires at the hero on arrival.
    if !*mimicked && let Ok(v) = std::env::var("WRIFT_MIMIC") {
        *mimicked = true;
        super::dungeon::spawn_mimic(&mut commands, &mut images, 9 * 16, 7 * 16, v == "sprung");
    }
    // WRIFT_BOSS=<theme>: drop that theme's authored boss into the start room —
    // the fight (and the boss bar) without walking the whole dungeon.
    if !*mimicked && let Ok(theme) = std::env::var("WRIFT_BOSS") {
        *mimicked = true;
        if theme == "wriftheart" {
            super::boss::wriftheart::spawn(&mut commands, &mut images);
        } else {
            super::boss::spawn_authored(&mut commands, &mut images, &mut blockers, &theme);
        }
    }
    // WRIFT_MOB="kind,kind,...": line those mobs up across the start room — the
    // cheap way to eyeball a new mob's art + idle without hunting its biome.
    if !*mimicked && let Ok(kinds) = std::env::var("WRIFT_MOB") {
        *mimicked = true;
        for (i, kind) in kinds.split(',').enumerate() {
            if kind.trim() == "ogre" {
                super::ogre::spawn_ogre(&mut commands, 48.0 + i as f32 * 48.0, 5.0 * 16.0);
                continue;
            }
            // "elite:kind" (or "champ:kind") promotes it — eyeball the aura + name tag.
            let (rank, name) = kind.trim().split_once(':').map_or(("", kind.trim()), |(r, k)| (r, k));
            if let Some(idx) = crate::actors::mobs::def_index(name) {
                let ent = commands
                    .spawn((
                        crate::actors::mobs::mob_bundle(idx, 48.0 + i as f32 * 60.0, 5.0 * 16.0),
                        super::battle::RoomActor,
                        crate::gfx::PIXEL_LAYER,
                        super::dungeon::DungeonFoe(crate::actors::mobs::MOB_DEFS[idx].kind),
                    ))
                    .id();
                if rank == "elite" || rank == "champ" {
                    let mut seed = 1234u64 + i as u64 * 77;
                    let mut rng = move || {
                        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                        ((seed >> 33) as f64) / (1u64 << 31) as f64
                    };
                    super::champions::promote(&mut commands, &mut images, ent, rank == "elite", &mut rng);
                }
            }
        }
    }
    if let Some((dir, frames)) = std::env::var("WRIFT_WALK").ok().and_then(|s| {
        let (a, b) = s.split_once(',')?;
        Some((a.to_string(), b.parse::<u32>().ok()?))
    }) && *walked < frames
    {
        *walked += 1;
        let action = match dir.as_str() {
            "down" => crate::input::Action::Down,
            "left" => crate::input::Action::Left,
            "right" => crate::input::Action::Right,
            _ => crate::input::Action::Up,
        };
        state.hold_for_test(action);
    }
}

/// WRIFT_SHOT=fanfare: fire the item-get cutscene (default a KEY; WRIFT_FAN=ornatekey|
/// sword|shield picks another). Capture mid-raise (~frame 30) for the prize aloft.
fn fanfare_stage(mut fanfare: ResMut<super::fanfare::Fanfare>, mut done: Local<bool>) {
    if *done {
        return;
    }
    *done = true;
    let id: &'static str = match std::env::var("WRIFT_FAN").as_deref() {
        Ok("ornatekey") => "ornatekey",
        Ok("sword") => "sword",
        Ok("shield") => "shield",
        _ => "key",
    };
    super::fanfare::begin(&mut fanfare, id);
}

/// WRIFT_SHOT=descent: park the stair-walk fade over the room so the DESCENDING /
/// CLIMBING wash + word can be eyeballed. InDungeon stays empty, so navigate never
/// ticks it — the fade holds for a stable capture. WRIFT_DESCEND=up shows CLIMBING.
fn descent_stage(mut descending: ResMut<super::dungeon::Descending>, mut done: Local<bool>) {
    if *done {
        return;
    }
    *done = true;
    let dir = if std::env::var("WRIFT_DESCEND").as_deref() == Ok("up") { -1 } else { 1 };
    descending.0 = Some(super::dungeon::DescendFx::staged(dir));
}

/// Equip the rod, face down at the bank, cast — capture mid-wait (or mid-bite with a
/// Wear the wispstone, throw the boomerang — the shot catches both in the air.
fn uniq_stage(
    mut state: ResMut<ActionState>,
    clock: Res<super::room_render::FrameClock>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut step: Local<u32>,
    mut cool: Local<u32>,
) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    if clock.0 < 30 {
        return;
    }
    *cool = 30;
    match *step {
        0 => {
            inv.add_item("wispstone", 1);
            inv.auto_equip("wispstone");
            inv.add_item("boomerang", 1);
            if let Some(uid) = inv.entries.iter().find(|e| e.id == "boomerang").map(|e| e.uid) {
                inv.slots[3] = Some(uid);
            }
            *step = 1;
        }
        1 => {
            state.press_for_test(crate::input::Action::Slot4); // let it fly
            *step = 2;
            *cool = 12; // catch it outbound
        }
        _ => {}
    }
}

/// Drive the treasure loop: kit up, read a chart (a REAL far-off X), plant a
/// second X in THIS room, then dig its mound open.
#[allow(clippy::too_many_arguments)] // shot-stage plumbing
fn dig_stage(
    mut state: ResMut<ActionState>,
    clock: Res<super::room_render::FrameClock>,
    cur: Res<super::play::CurRoom>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut maps: ResMut<super::digging::TreasureMaps>,
    mut players: Query<&mut super::play::Player>,
    mut step: Local<u32>,
    mut cool: Local<u32>,
) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    if clock.0 < 30 {
        return;
    }
    let Ok(mut p) = players.single_mut() else { return };
    *cool = 24;
    match *step {
        0 => {
            inv.add_item("shovel", 1);
            inv.add_item("treasuremap", 2);
            for (slot, id) in [(2, "shovel"), (3, "treasuremap")] {
                if let Some(uid) = inv.entries.iter().find(|e| e.id == id).map(|e| e.uid) {
                    inv.slots[slot] = Some(uid);
                }
            }
            *step = 1;
        }
        1 => {
            state.press_for_test(crate::input::Action::Slot4); // read the chart
            *step = 2;
        }
        2 => {
            // Plant an X in THIS room beside the hero (mound_wake stands the mound).
            let (c, r) = (((p.x + 8.0) / 16.0) as i32 + 2, ((p.y + 12.0) / 16.0) as i32);
            maps.0.push(super::digging::TMap { rx: cur.rx, ry: cur.ry, c, r, tier: 2 });
            *step = 3;
        }
        3 => {
            let Some(tm) = maps.0.iter().find(|m| m.rx == cur.rx && m.ry == cur.ry) else { return };
            p.x = (tm.c * 16 - 18) as f32; // stand west of the mound, facing it
            p.y = (tm.r * 16 - 4) as f32;
            p.facing = crate::actors::hero::Facing::Right;
            *step = 4;
        }
        4 => {
            state.press_for_test(crate::input::Action::Slot3); // dig
            *step = 9;
        }
        _ => {}
    }
}

/// Wand in hand: socket fire, loose casts, and put a torch to the brush.
#[allow(clippy::too_many_arguments)]
fn magic_stage(
    mut commands: Commands,
    clock: Res<super::room_render::FrameClock>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut rune: ResMut<super::wands::WandRune>,
    mut casts: MessageWriter<super::wands::WandMsg>,
    mut players: Query<&mut super::play::Player>,
    nodes: Query<(Entity, &super::gather::GatherNode), Without<super::fire::Burning>>,
    mut step: Local<u32>,
) {
    let Ok(mut p) = players.single_mut() else { return };
    if *step == 0 && clock.0 >= 40 {
        *step = 1;
        inv.add_item("wand", 1);
        inv.auto_equip("wand");
        rune.0 = "fire";
        p.x = 60.0;
        p.y = 150.0;
        p.facing = crate::actors::hero::Facing::Right;
    }
    if *step == 1 && clock.0 >= 70 {
        *step = 2;
        casts.write(super::wands::WandMsg::Cast);
        // ...and set the nearest bush alight for the burn + spread check.
        if let Some((e, n)) = nodes.iter().find(|(_, n)| n.kind == "bush") {
            super::fire::ignite(&mut commands, e, n.kind, false);
        }
    }
    if *step == 2 && clock.0 >= 82 {
        *step = 3;
        casts.write(super::wands::WandMsg::Cast);
    }
}

/// Stand up the trade wagon + a shopkeeper beside the hero (the art check).
fn caravan_stage(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut art: ResMut<crate::actors::villager::VillagerArt>,
    clock: Res<super::room_render::FrameClock>,
    mut players: Query<&mut super::play::Player>,
    mut done: Local<bool>,
) {
    if *done || clock.0 < 25 {
        return;
    }
    *done = true;
    let (wx, wy) = (120.0f32, 90.0f32);
    let img = images.add(crate::gfx::bake(crate::actors::encounter_art::WAGON, &[]));
    let mut spr = Sprite::from_image(img);
    spr.custom_size = Some(bevy::math::Vec2::new(32.0, 21.0));
    commands.spawn((
        spr,
        crate::gfx::at(super::room_render::PLAY_X + wx, super::room_render::PLAY_Y + wy, 32.0, 21.0, super::room_render::actor_z(wy + 20.0)),
        crate::gfx::PIXEL_LAYER,
        super::battle::RoomActor,
    ));
    let frames = art.frames(0x51a7, &mut images);
    let mut ks = Sprite::from_image(frames.frames[0][0].clone());
    ks.custom_size = Some(bevy::math::Vec2::splat(16.0));
    commands.spawn((
        ks,
        crate::gfx::at(super::room_render::PLAY_X + wx + 18.0, super::room_render::PLAY_Y + wy + 2.0, 16.0, 16.0, super::room_render::actor_z(wy + 18.0)),
        crate::gfx::PIXEL_LAYER,
        super::battle::RoomActor,
    ));
    if let Ok(mut p) = players.single_mut() {
        p.x = wx + 4.0;
        p.y = wy + 20.0;
    }
}

/// Grant rolled gear (WRIFT_GEN_SEED sets the roll) and equip it — eyeball the
/// material-tinted icons + the generated armor worn on the hero.
fn procgen_stage(
    clock: Res<super::room_render::FrameClock>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut players: Query<&mut super::play::Player>,
    mut done: Local<bool>,
) {
    if *done || clock.0 < 30 {
        return;
    }
    *done = true;
    let base = std::env::var("WRIFT_GEN_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(0xd00du32);
    // An epic weapon + a full epic armor set (each a distinct roll).
    let w = crate::procgen::generate(crate::procgen::Kind::Weapon, 3, base);
    inv.add_item(w, 1);
    inv.auto_equip(w);
    for (i, _slot) in ["head", "body", "feet"].iter().enumerate() {
        let a = crate::procgen::generate(crate::procgen::Kind::Armor, 3, base ^ (i as u32 + 1).wrapping_mul(0x9e3779b1));
        // Re-roll until the class matches the slot we want to fill (js generate opts.slot).
        let mut id = a;
        for t in 0..40u32 {
            if crate::items::get(id).and_then(|d| d.slot) == Some(["head", "body", "feet"][i]) {
                break;
            }
            id = crate::procgen::generate(crate::procgen::Kind::Armor, 3, base ^ (i as u32 + 1).wrapping_mul(0x9e3779b1) ^ t.wrapping_mul(2654435761));
        }
        inv.add_item(id, 1);
        inv.auto_equip(id);
    }
    if let Ok(mut p) = players.single_mut() {
        p.x = 90.0;
        p.y = 150.0;
    }
}

/// Spawn the loot goblin next to the hero so it's spooked + fleeing at capture.
fn lootgob_stage(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    clock: Res<super::room_render::FrameClock>,
    mut done: Local<bool>,
) {
    if *done || clock.0 < 20 {
        return;
    }
    *done = true;
    super::lootgoblin::spawn_lootgoblin(&mut commands, &mut images, 130.0, 90.0, 10);
}

/// Line up the newly-ported roster mobs (WRIFT_MOB="a,b,c" overrides the default set).
fn newmobs_stage(
    mut commands: Commands,
    clock: Res<super::room_render::FrameClock>,
    mut done: Local<bool>,
) {
    if *done || clock.0 < 20 {
        return;
    }
    *done = true;
    let default = "cultist,mirefly,palehowler,bellsnail,boglight,saltstatue,gravewarden,vinesnare";
    let list = std::env::var("WRIFT_MOB").unwrap_or_else(|_| default.to_string());
    for (i, kind) in list.split(',').enumerate() {
        if let Some(idx) = crate::actors::mobs::def_index(kind.trim()) {
            commands.spawn((
                crate::actors::mobs::mob_bundle(idx, 34.0 + i as f32 * 34.0, 5.0 * 16.0),
                super::battle::RoomActor,
                crate::gfx::PIXEL_LAYER,
                super::dungeon::DungeonFoe(crate::actors::mobs::MOB_DEFS[idx].kind),
            ));
        }
    }
}

/// Stand up two elite mobs (aura + name tag) in the start room for a clean shot.
fn elite_stage(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    clock: Res<super::room_render::FrameClock>,
    mut done: Local<bool>,
) {
    if *done || clock.0 < 20 {
        return;
    }
    *done = true;
    for (i, kind) in ["archer", "spider"].iter().enumerate() {
        if let Some(idx) = crate::actors::mobs::def_index(kind) {
            let ent = commands
                .spawn((
                    crate::actors::mobs::mob_bundle(idx, 70.0 + i as f32 * 90.0, 100.0),
                    super::battle::RoomActor,
                    crate::gfx::PIXEL_LAYER,
                    super::dungeon::DungeonFoe(crate::actors::mobs::MOB_DEFS[idx].kind),
                ))
                .id();
            let mut seed = 4242u64 + i as u64 * 131;
            let mut rng = move || {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                ((seed >> 33) as f64) / (1u64 << 31) as f64
            };
            super::champions::promote(&mut commands, &mut images, ent, true, &mut rng);
        }
    }
}

/// Fire the grapple hook rightward so the rope + claw are mid-flight when captured.
fn grapple_stage(
    clock: Res<super::room_render::FrameClock>,
    mut fires: MessageWriter<super::traversal::FireHook>,
    mut players: Query<&mut super::play::Player>,
    mut moved: Local<bool>,
    mut fired: Local<bool>,
) {
    // Reposition early; fire once the sim is warm so the rope is mid-flight at capture.
    if !*moved && clock.0 >= 10 {
        *moved = true;
        if let Ok(mut p) = players.single_mut() {
            p.x = 40.0;
            p.y = 150.0;
            p.facing = crate::actors::hero::Facing::Right;
        }
    }
    if *fired || clock.0 < 40 {
        return;
    }
    *fired = true;
    let Ok(p) = players.single() else { return };
    fires.write(super::traversal::FireHook { dx: 1.0, dy: 0.0, sx: p.x + 8.0, sy: p.y + 9.0 });
}

/// Dress the hero in a full visible set (helm crest + plate seam + knee plates).
fn worn_stage(
    clock: Res<super::room_render::FrameClock>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut players: Query<&mut super::play::Player>,
    mut done: Local<bool>,
) {
    if *done || clock.0 < 40 {
        return;
    }
    *done = true;
    let Ok(mut p) = players.single_mut() else { return };
    for id in ["dragonhelm", "platemail", "ironcladgreaves"] {
        inv.add_item(id, 1);
        inv.auto_equip(id);
    }
    p.x = 60.0;
    p.y = 150.0;
}

/// Grant + raise the shield (held via the injected test hold), then fly an enemy
/// arrow at the hero's face — the block should spark it away.
#[allow(clippy::too_many_arguments)]
fn shield_stage(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    clock: Res<super::room_render::FrameClock>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut state: ResMut<crate::input::ActionState>,
    mut players: Query<&mut super::play::Player>,
    mut granted: Local<bool>,
    mut fired: Local<bool>,
) {
    use crate::input::Action;
    let Ok(mut p) = players.single_mut() else { return };
    if clock.0 < 40 {
        return;
    }
    if !*granted {
        *granted = true;
        inv.add_item("shield", 1);
        inv.auto_equip("shield");
        p.facing = crate::actors::hero::Facing::Right;
        p.x = 60.0; // out from under the start room's centre tree
        p.y = 150.0;
    }
    // Hold the shield's slot every polled frame — the raise is level-triggered.
    if let Some(i) = inv.slots.iter().position(|u| u.and_then(|u| inv.id_of(u)) == Some("shield")) {
        state.hold_for_test([Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4][i]);
    }
    if clock.0 >= 70 && !*fired {
        *fired = true;
        let (x, y) = (p.x + 60.0, p.y);
        let art = images.add(crate::gfx::bake(
            &["................", "................", "................", "................", "................", "................", "..ff............", "...sssssssssWW..", "...sssssssssWW..", "..ff............", "................", "................", "................", "................", "................", "................"],
            &[('s', 0xcaa050), ('W', 0xe8e8e8), ('f', 0xd8d8d8)],
        ));
        let mut tf = crate::gfx::at(super::room_render::PLAY_X + x, super::room_render::PLAY_Y + y, 16.0, 16.0, 8.6);
        tf.rotation = Quat::from_rotation_z(std::f32::consts::PI);
        commands.spawn((
            super::battle::projectiles::EnemyArrow { x, y, vx: -3.0, vy: 0.0, life: 42 },
            crate::combat::Combatant { team: crate::combat::Team::Enemy, hurt_team: Some(crate::combat::Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
            crate::combat::HitOnce::default(),
            crate::combat::Hitbox { x: x + 5.0, y: y + 5.0, w: 6.0, h: 6.0 },
            Sprite::from_image(art),
            tf,
            crate::gfx::PIXEL_LAYER,
            super::battle::RoomActor,
        ));
    }
}

/// Loose an arrow in each facing a few ticks apart (rotation + flight check).
fn bow_stage(
    clock: Res<super::room_render::FrameClock>,
    mut fires: MessageWriter<super::archery::FireArrow>,
    mut players: Query<&mut super::play::Player>,
    mut step: Local<u32>,
) {
    use crate::actors::hero::Facing;
    let Ok(mut p) = players.single_mut() else { return };
    let shots = [(60, Facing::Right), (63, Facing::Up), (66, Facing::Left), (69, Facing::Down)];
    if let Some((_, f)) = shots.get(*step as usize).filter(|(at, _)| clock.0 >= *at) {
        p.facing = *f;
        fires.write(super::archery::FireArrow { dry: false });
        *step += 1;
    }
}

/// Play the opening from WRIFT_CUT_T (default 0).
fn cutscene_stage(
    clock: Res<super::room_render::FrameClock>,
    mut cs: ResMut<super::cinematic::Cutscene>,
    mut done: Local<bool>,
) {
    if *done || clock.0 < 30 {
        return;
    }
    *done = true;
    let t = std::env::var("WRIFT_CUT_T").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    cs.0 = Some(t);
}

/// Drop straight into the hidden side-view chamber (js DBG_SIDE).
#[allow(clippy::too_many_arguments)] // shot-stage plumbing
fn side_stage(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    clock: Res<super::room_render::FrameClock>,
    root: Res<super::play::ActiveRoot>,
    looted: Res<super::sidescroll::SideLooted>,
    mut side: ResMut<super::sidescroll::SideScroll>,
    actors: Query<Entity, With<RoomActor>>,
    mut players: Query<&mut super::play::Player>,
    mut done: Local<bool>,
) {
    if *done || clock.0 < 40 {
        return;
    }
    *done = true;
    commands.entity(root.0).despawn();
    for a in &actors {
        commands.entity(a).despawn();
    }
    let st = super::sidescroll::enter_side(&mut commands, &mut images, &looted, "shot".into());
    if let Ok(mut p) = players.single_mut() {
        p.x = (st.spawn.0 * 16) as f32;
        p.y = (st.spawn.1 * 16) as f32;
        p.facing = crate::actors::hero::Facing::Right;
    }
    side.0 = Some(st);
}

/// Stand by the singing stone; WRIFT_SING=1 rings the Song of Opening at it.
fn song_stage(
    clock: Res<super::room_render::FrameClock>,
    mut openings: MessageWriter<super::caves::OpeningSung>,
    stones: Query<&super::caves::Songstone>,
    mut players: Query<&mut super::play::Player>,
    mut done: Local<bool>,
) {
    if *done || clock.0 < 40 {
        return;
    }
    let Some(st) = stones.iter().next() else { return };
    *done = true;
    if let Ok(mut p) = players.single_mut() {
        p.x = st.x - 4.0;
        p.y = st.y + 20.0;
        p.facing = crate::actors::hero::Facing::Up;
    }
    if std::env::var("WRIFT_SING").is_ok() {
        openings.write(super::caves::OpeningSung { mana: 0 });
    }
}

/// Drive the cracked-wall loop: stand under the crack, bomb it, then (unless
/// WRIFT_CAVE_STAY) press at the carved door and descend.
fn cave_stage(
    mut state: ResMut<ActionState>,
    clock: Res<super::room_render::FrameClock>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    nodes: Query<&super::gather::GatherNode>,
    mut players: Query<&mut super::play::Player>,
    mut step: Local<u32>,
    mut cool: Local<u32>,
) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    if clock.0 < 30 {
        return;
    }
    let Ok(mut p) = players.single_mut() else { return };
    match *step {
        0 => {
            // Find the room's crack and stand on the floor tile beside it.
            let Some(n) = nodes.iter().find(|n| n.kind == "crackedrock") else { return };
            let (dc, dr) = if n.r == 0 { (0, 1) } else if n.c == 0 { (1, 0) } else if n.c > 9 { (-1, 0) } else { (0, -1) };
            p.x = ((n.c + dc) * 16 - 2) as f32;
            p.y = ((n.r + dr) * 16) as f32;
            inv.add_item("bombs", 3);
            if let Some(uid) = inv.entries.iter().find(|e| e.id == "bombs").map(|e| e.uid) {
                inv.slots[3] = Some(uid);
            }
            *step = 1;
            *cool = 10;
        }
        1 => {
            state.press_for_test(crate::input::Action::Slot4); // drop the bomb...
            *step = 2;
            *cool = 6;
        }
        2 => {
            p.y += 56.0; // ...and BACK AWAY (the blast is friend and foe alike)
            *step = 3;
            *cool = 190; // fuse 75 FIXED ticks = 150 update frames + blast + settle
        }
        3 => {
            p.y -= 56.0; // back to the carved mouth
            *step = 4;
            *cool = 10;
        }
        4 => {
            if std::env::var("WRIFT_CAVE_STAY").is_err() {
                state.press_for_test(crate::input::Action::Interact); // into the dark
            }
            *step = 5;
        }
        _ => {}
    }
}

/// Drive the Cooking Fire loop: larder -> place at feet -> PRESS -> station window
/// (WRIFT_COOKCRAFT=1 then crafts; WRIFT_EAT=1 eats a roast instead).
fn cook_stage(
    mut state: ResMut<ActionState>,
    clock: Res<super::room_render::FrameClock>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut step: Local<u32>,
    mut cool: Local<u32>,
) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    if clock.0 < 30 {
        return; // let town_stage's warp settle first
    }
    *cool = 20;
    match *step {
        0 => {
            for (id, n) in [
                ("cook", 1), ("meat", 4), ("herb", 6), ("fiber", 2), ("milk", 1), ("potato", 1), ("pumpkin", 1),
                ("wheat", 3), ("cranberry", 2), ("pepper", 1), ("carrot", 1), ("tomato", 1), ("egg", 1),
                ("rareherb", 1), ("minnow", 3),
            ] {
                inv.add_item(id, n);
            }
            if let Some(uid) = inv.entries.iter().find(|e| e.id == "cook").map(|e| e.uid) {
                inv.slots[3] = Some(uid);
            }
            *step = 1;
        }
        1 => {
            state.press_for_test(crate::input::Action::Slot4); // place the fire
            *step = 2;
        }
        2 => {
            if std::env::var("WRIFT_EAT").is_ok() {
                inv.add_item("roast", 1);
                if let Some(uid) = inv.entries.iter().find(|e| e.id == "roast").map(|e| e.uid) {
                    inv.slots[3] = Some(uid);
                }
                *step = 3;
            } else {
                state.press_for_test(crate::input::Action::Interact); // open the window
                *step = 4;
            }
        }
        3 => {
            state.press_for_test(crate::input::Action::Slot4); // eat it
            *step = 9;
        }
        4 => {
            if std::env::var("WRIFT_COOKCRAFT").is_ok() {
                state.press_for_test(crate::input::Action::Slot1); // craft the cursor row
            }
            *step = 9;
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)] // shot-stage plumbing
/// Walk the hero into the city's guildhall (scene "guildhall"): outside, teleport onto
/// the hall's door zone and press; inside, run WRIFT_WALK, then WRIFT_DONATE=1 keeps
/// pressing so the shot catches a wing's donation checklist.
fn hall_stage(
    mut state: ResMut<ActionState>,
    dungeon: Res<super::dungeon::InDungeon>,
    world: Res<super::play::GameWorld>,
    cur: Res<super::play::CurRoom>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut players: Query<&mut super::play::Player>,
    mut cool: Local<u32>,
    mut walked: Local<u32>,
    mut pressed: Local<usize>,
    mut bagged: Local<bool>,
) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    if dungeon.0.is_none() {
        *cool = 4;
        let Ok(mut p) = players.single_mut() else { return };
        let Some(e) = world.0.room_entities(cur.rx, cur.ry).into_iter().find(|e| e.kind == "guildhall") else { return };
        p.x = (e.x + 50) as f32; // the js door zone (x+46, y+16, 20, 14)
        p.y = (e.y + 18) as f32;
        state.press_for_test(crate::input::Action::Interact);
        return;
    }
    if let Some((dir, frames)) = std::env::var("WRIFT_WALK").ok().and_then(|s| {
        let (a, b) = s.split_once(',')?;
        Some((a.to_string(), b.parse::<u32>().ok()?))
    }) {
        if *walked < frames {
            *walked += 1;
            let action = match dir.as_str() {
                "down" => crate::input::Action::Down,
                "left" => crate::input::Action::Left,
                "right" => crate::input::Action::Right,
                _ => crate::input::Action::Up,
            };
            state.hold_for_test(action);
            return;
        }
        // WRIFT_BAG="milk:2,meat:2" seeds the bag once inside; WRIFT_DONATE=1 then
        // plays the whole provisioners bundle (open, 2 gives, down, 2, down, 2) so a
        // single shot proves donate -> wing-whole -> reward.
        if !*bagged && let Ok(bag) = std::env::var("WRIFT_BAG") {
            *bagged = true;
            for part in bag.split(',') {
                if let Some((id, n)) = part.split_once(':')
                    && let Some(def) = crate::items::get(id)
                {
                    inv.add_item(def.id, n.parse().unwrap_or(1));
                }
            }
        }
        if std::env::var("WRIFT_DONATE").is_ok() {
            use crate::input::Action::{Down, Interact};
            const SEQ: [crate::input::Action; 9] = [Interact, Interact, Interact, Down, Interact, Interact, Down, Interact, Interact];
            *cool = 30;
            if let Some(a) = SEQ.get(*pressed) {
                *pressed += 1;
                state.press_for_test(*a);
            }
        }
    }
}

/// short WRIFT_CLOCK-independent delay; the bobber + prompt bar are the proof).
fn fish_stage(
    mut state: ResMut<ActionState>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    fishing: Res<super::fishing::Fishing>,
    mut players: Query<&mut super::play::Player>,
    mut cool: Local<u32>,
) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    *cool = 6;
    if fishing.0.is_some() {
        return; // cast away — hold for the capture
    }
    let Ok(mut p) = players.single_mut() else { return };
    if !inv.has_item("fishingrod") {
        inv.add_item("fishingrod", 1);
        inv.auto_equip("fishingrod");
    }
    p.facing = crate::actors::hero::Facing::Down;
    let slot = (0..4).find(|&i| inv.slots[i].and_then(|u| inv.id_of(u)) == Some("fishingrod"));
    if let Some(i) = slot {
        state.press_for_test([crate::input::Action::Slot1, crate::input::Action::Slot2, crate::input::Action::Slot3, crate::input::Action::Slot4][i]);
    }
}

/// Stage a farm plot: 4 beds left-to-right at seed / young / grown / RIPE (last two
/// watered today), farm tools + seeds on the belt, hero facing up at fresh grass so
/// the hoe reticle reads green. Data is written straight into FarmTiles — the render
/// path (sync_farm_sprites) is exactly what a live till would exercise.
#[allow(clippy::too_many_arguments, clippy::type_complexity)] // shot-stage plumbing
fn farm_stage(
    mut commands: Commands,
    mut farm: ResMut<super::farm::FarmTiles>,
    mut dirty: ResMut<super::farm::FarmDirty>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    clock: Res<super::room_render::FrameClock>,
    cur: Res<super::play::CurRoom>,
    mut players: Query<&mut super::play::Player>,
    veg: Query<
        (Entity, Option<&super::gather::GatherNode>, Option<&super::farm::GroundVeg>),
        Or<(With<super::gather::GatherNode>, With<super::farm::GroundVeg>)>,
    >,
    mut done: Local<bool>,
) {
    let Ok(mut p) = players.single_mut() else { return };
    if *done {
        p.facing = crate::actors::hero::Facing::Up; // hold the pose for the reticle
        return;
    }
    if clock.0 < 30 {
        return; // let town_stage's warp settle first
    }
    *done = true;
    for id in ["hoe", "wateringcan", "turnipseed"] {
        inv.add_item(id, 1);
    }
    // The farm tools ride the belt like a live game (slot 4 = the hoe -> reticle).
    if let Some(uid) = inv.entries.iter().find(|e| e.id == "hoe").map(|e| e.uid) {
        inv.slots[3] = Some(uid);
    }
    let room = (cur.rx, cur.ry);
    let day = super::gather::farm_day(clock.0);
    let (pc, pr) = (((p.x + 8.0) / 16.0) as i32, ((p.y + 12.0) / 16.0) as i32);
    for (i, stage) in [0, 1, 2, 3].into_iter().enumerate() {
        let (c, r) = (pc - 2 + i as i32, pr - 2);
        farm.till(room, c, r, false, day);
        // The live till strips grass + cosmetics off the tile — mirror it here.
        for (e, node, gv) in &veg {
            let tile = node.filter(|n| n.kind == "grass").map(|n| (n.c, n.r)).or(gv.map(|g| (g.c, g.r)));
            if tile == Some((c, r)) {
                commands.entity(e).despawn();
            }
        }
        farm.plant(room, c, r, "turnip", day);
        if let Some(t) = farm.0.get_mut(&room).and_then(|m| m.get_mut(&(c, r))) {
            if let Some(cr) = &mut t.crop {
                cr.stage = stage; // turnip ripens at 3
            }
            t.watered = if i >= 2 { day } else { -1 }; // wet vs dry beds
        }
    }
    dirty.0 = true;
}

/// Seed a plausible quest log straight into the resources: a slay mid-count, a fetch
/// nearly done, and a bounty with a map marker two rooms east.
fn quest_stage(
    mut log: ResMut<super::quests::QuestLog>,
    mut counter: ResMut<super::quests::QuestCounter>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    cur: Res<super::play::CurRoom>,
    clock: Res<super::room_render::FrameClock>,
    mut done: Local<bool>,
) {
    use super::quests::{Quest, QuestKind, Reward};
    if *done || clock.0 < 30 {
        return;
    }
    *done = true;
    inv.add_item("wood", 4); // the fetch quest below reads READY -> the gold '?' shows
    let mk = |id: u32, kind: QuestKind, title: &str, goal: &str, desc: &str, coin: i32, xp: i32| Quest {
        id,
        kind,
        done: false,
        title: title.into(),
        goal: goal.into(),
        desc: desc.into(),
        reward: Reward { coin, xp, item: None },
        giver_key: format!("{},{},{id}", cur.rx, cur.ry),
        giver_rx: cur.rx,
        giver_ry: cur.ry,
    };
    counter.0 = 3;
    log.0 = vec![
        mk(1, QuestKind::Slay { kind: "wolf".into(), need: 6, have: 2 }, "CULL THE DIRE WOLF",
            "Slay 6 dire wolfs", "Dire wolfs have been a menace. Slay 6 of them, wherever you find them.", 52, 41),
        mk(2, QuestKind::Fetch { item: "wood".into(), need: 4 }, "GATHER WOOD",
            "Bring 4 wood", "I need 4 wood. Gather it from the wilds and bring it back.", 38, 30),
        mk(3, QuestKind::Bounty { rx: cur.rx + 2, ry: cur.ry, kind: "bear".into(), name: "Bloodfang".into() },
            "BOUNTY: BLOODFANG", "Slay Bloodfang",
            "Bloodfang, a monstrous forest bear, lairs to the east. Put it down.", 92, 74),
    ];
}

/// After the quest log seeds, hop into the codex on the MAP tab (pin verification).
fn codex_after_stage(
    mut state: ResMut<ActionState>,
    screen: Res<State<Screen>>,
    clock: Res<super::room_render::FrameClock>,
    mut cool: Local<u32>,
) {
    if clock.0 < 40 || *screen.get() == Screen::Codex {
        return;
    }
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    *cool = 6;
    state.press_for_test(crate::input::Action::Map);
}

/// Stage the flute mid-REPLAY: the banner + lit notes + drifting motes, no input needed.
fn flute_stage(
    mut fluting: ResMut<super::flute::Fluting>,
    mut learned: ResMut<super::flute::LearnedSongs>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut clock: ResMut<super::room_render::FrameClock>,
    mut state: ResMut<ActionState>,
    mut weather: ResMut<super::weather::WeatherState>,
    mut done: Local<bool>,
) {
    use super::flute::{FluteState, Mote, Phase};
    let play_mode = std::env::var("WRIFT_FLUTE").as_deref() == Ok("play");
    // Play mode: a real note press every few frames — the live spark/glow/light path,
    // not staged state — so the capture lands with the rose mid-song.
    if play_mode && *done && clock.0 % 9 == 0 {
        state.press_for_test(crate::input::Action::Up);
    }
    if *done || clock.0 < 30 {
        return;
    }
    *done = true;
    inv.add_item("flute", 1);
    inv.auto_equip("flute");
    learned.0.insert("returning");
    // WRIFT_FLUTE=play: the PLAY-phase compass rose (staff band + diamond + played tail)
    // — jumped to MIDNIGHT so the flute's pool of light shows against the dark.
    if play_mode {
        clock.0 += super::gather::DAY_LEN / 2;
        weather.force(Some("rain".into())); // prove the rose rides ABOVE the rain
        fluting.0 = Some(FluteState {
            phase: Phase::Play,
            t: 30,
            seq: "ULDRUL".into(),
            glow: [0, 0, 10, 0], // L mid-ring
            song: None,
            armed: None,
            ri: 0,
            rt: 0,
            flash: 0,
            motes: vec![],
            sparks: vec![],
            held: [false; 4],
            dests: vec![],
            di: 0,
            wt: 0,
            wspin: 0.0,
            whp: 0,
            wdest: (0, 0),
            whome: false,
        });
        return;
    }
    let motes = vec![
        Mote { x: 150.0, y: 95.0, t: 6, ltr: 'U', col: 0xb48ae8 },
        Mote { x: 156.0, y: 96.0, t: 14, ltr: 'R', col: 0xe87a9a },
        Mote { x: 148.0, y: 94.0, t: 22, ltr: 'D', col: 0xe8a84a },
    ];
    fluting.0 = Some(FluteState {
        phase: Phase::Replay,
        t: 40,
        seq: String::new(),
        glow: [12, 0, 0, 8],
        song: crate::songs::get("returning"),
        armed: None,
        ri: 3,
        rt: 900, // hold mid-replay for the capture (no cast fires)
        flash: 10,
        motes,
        sparks: vec![],
        held: [false; 4],
        dests: vec![],
        di: 0,
        wt: 0,
        wspin: 0.0,
        whp: 0,
        wdest: (0, 0),
        whome: false,
    });
}

/// Pop the dev console (WRIFT_TAB picks a category by index).
fn dev_stage(mut state: ResMut<ActionState>, screen: Res<State<Screen>>, mut cool: Local<u32>) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    *cool = 10;
    if *screen.get() == Screen::Play {
        state.press_for_test(crate::input::Action::DevPanel);
    }
}

fn talk_stage(
    mut state: ResMut<ActionState>,
    screen: Res<State<Screen>>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut players: Query<&mut super::play::Player>,
    villagers: Query<&crate::actors::villager::Villager>,
    mut cool: Local<u32>,
    mut step: Local<u32>,
) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    *cool = 8;
    match screen.get() {
        Screen::Play if *step == 0 => {
            let Some(v) = villagers.iter().find(|v| v.pkey.is_some()) else { return };
            if let Ok(mut p) = players.single_mut() {
                p.x = v.x + 14.0;
                p.y = v.y;
                state.press_for_test(crate::input::Action::Interact);
            }
        }
        Screen::Dialog if std::env::var("WRIFT_GIFT").is_ok() => match *step {
            0 => {
                for (id, n) in [("potion", 3), ("herb", 5), ("wood", 8), ("gem", 1)] {
                    inv.add_item(id, n);
                }
                state.press_for_test(crate::input::Action::Down); // onto GIVE
                *step = 1;
            }
            1 => {
                state.press_for_test(crate::input::Action::Slot1); // open the picker
                *step = 2;
            }
            _ => {}
        },
        _ => {}
    }
}

/// Open the creator and stay there (NEW GAME from the empty title, then hands off).
fn creator_stage(mut state: ResMut<ActionState>, screen: Res<State<Screen>>, mut cool: Local<u32>) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    if *screen.get() == Screen::Title {
        state.press_for_test(crate::input::Action::Slot1);
        *cool = 5;
    }
}

/// Drive a NEW GAME from the title through the real input path: under WRIFT_SHOT the
/// slots scan empty, so NEW GAME is the first row — confirm opens the CREATOR, Up wraps
/// the cursor onto START ADVENTURE, confirm launches. Exercises the whole creator +
/// LoadSlot fresh-world swap; a clean run (no panic) is the pass.
fn newgame_stage(mut state: ResMut<ActionState>, screen: Res<State<Screen>>, mut cool: Local<u32>, mut upped: Local<bool>) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    match screen.get() {
        Screen::Title => state.press_for_test(crate::input::Action::Slot1),
        Screen::Creator if !*upped => {
            state.press_for_test(crate::input::Action::Up); // NAME row wraps up to START
            *upped = true;
        }
        Screen::Creator => state.press_for_test(crate::input::Action::Slot1),
        _ => return,
    }
    *cool = 5;
}

/// Stage the pause menu by pressing the real keys: Pause to open, then TabNext until the
/// WRIFT_TAB menu tab (GAME/VIDEO/SOUND/CONTROLS) is front. Presses are spaced a few
/// frames apart — a state transition lags its `next.set` by a frame, and an eager second
/// Pause press would toggle the menu straight back shut.
fn pause_menu_stage(
    mut state: ResMut<ActionState>,
    screen: Res<State<Screen>>,
    menu: Res<super::menu::MenuState>,
    mut cool: Local<u32>,
) {
    if *cool > 0 {
        *cool -= 1;
        return;
    }
    let want = std::env::var("WRIFT_TAB").ok().and_then(|t| super::menu::tab_index(&t)).unwrap_or(0);
    match screen.get() {
        Screen::Play => state.press_for_test(crate::input::Action::Pause),
        Screen::Pause if menu.tab != want => state.press_for_test(crate::input::Action::TabNext),
        _ => return,
    }
    *cool = 5;
}

/// A scatter of ground drops (one per starter item) so the icon + glow + bob get eyeballed.
fn spawn_drops(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    for (i, id) in ["wood", "stone", "fiber", "herb", "copper", "potion"].into_iter().enumerate() {
        let (col, row) = (i % 3, i / 3);
        super::gather::spawn_pickup(
            &mut commands,
            &mut images,
            id,
            1,
            90.0 + col as f32 * 40.0,
            70.0 + row as f32 * 36.0,
            false,
        None);
    }
}

/// The biome-mob roster, one of each on a labelled grid (frozen — AI only runs in Play).
fn spawn_roster(
    mut commands: Commands,
    mut next: ResMut<NextState<Screen>>,
    mut images: ResMut<Assets<Image>>,
    cast: Query<Entity, With<RoomActor>>,
) {
    next.set(Screen::Pause);
    for e in &cast {
        commands.entity(e).despawn();
    }
    for (i, def) in crate::actors::mobs::MOB_DEFS.iter().enumerate() {
        let (col, row) = (i % 7, i / 7);
        let (x, y) = (10.0 + col as f32 * 42.0, 14.0 + row as f32 * 33.0);
        commands.spawn((crate::actors::mobs::mob_bundle(i, x, y), RoomActor, PIXEL_LAYER));
        label(&mut commands, &mut images, def.kind, PLAY_X + x - 4.0, PLAY_Y + y + 17.0, 0xfcd000, 15.0, ());
    }
}

/// Tree growth-stage gallery: stump / sapling / young / full for four kinds.
fn spawn_stages(
    mut commands: Commands,
    mut next: ResMut<NextState<Screen>>,
    mut images: ResMut<Assets<Image>>,
    mut art: ResMut<crate::actors::props::PropArt>,
    cast: Query<Entity, With<RoomActor>>,
) {
    next.set(Screen::Pause);
    for e in &cast {
        commands.entity(e).despawn();
    }
    for (row, kind) in ["oak", "pine", "deadtree", "shroom"].into_iter().enumerate() {
        let y = 26.0 + row as f32 * 46.0;
        label(&mut commands, &mut images, kind, PLAY_X + 4.0, y + 26.0, 0xfcd000, 15.0, ());
        for stage in 0..3u8 {
            let img = art.stage(kind, stage, &mut images);
            let x = 60.0 + stage as f32 * 60.0;
            commands.spawn((
                Sprite::from_image(img),
                at(PLAY_X + x - 16.0, PLAY_Y + y - 56.0 + 20.0, 48.0, 72.0, 14.0),
                PIXEL_LAYER,
            ));
        }
        let full = art.tree(kind, (row as i32) * 160, 0, &mut images);
        let size = images.get(&full).map(|i| i.size().as_vec2()).unwrap_or(Vec2::new(48.0, 72.0));
        commands.spawn((
            Sprite::from_image(full),
            at(PLAY_X + 240.0 - 16.0, PLAY_Y + y - 56.0 + 20.0, size.x, size.y, 14.0),
            PIXEL_LAYER,
        ));
    }
}

const COL_X: [f32; 4] = [18.0, 90.0, 162.0, 234.0]; // room-px column per facing
const FACING_NAME: [&str; 4] = ["DOWN", "UP", "RIGHT", "LEFT"];
/// Sweep-phase tints: RED = start of the swing, white = middle, CYAN = end — so the shot
/// shows the direction of travel, not just the fan.
const PHASE_TINT: [Color; 3] = [
    Color::srgb(1.0, 0.3, 0.3),
    Color::WHITE,
    Color::srgb(0.3, 1.0, 1.0),
];

/// Freeze the sim, clear the room's cast, and lay out the tableau: one body per facing with
/// three blade phases around it — swords on the top row, goblin axes on the bottom.
fn spawn_tableau(
    mut commands: Commands,
    mut next: ResMut<NextState<Screen>>,
    mut images: ResMut<Assets<Image>>,
    hero: Res<HeroArt>,
    goblins: Res<GoblinArt>,
    art: Res<AttackArt>,
    cast: Query<Entity, With<RoomActor>>,
) {
    next.set(Screen::Pause); // gates play::tick and the whole battle chain — nothing moves
    for e in &cast {
        commands.entity(e).despawn();
    }

    for f in 0..4 {
        let x = COL_X[f];
        label(&mut commands, &mut images, FACING_NAME[f], PLAY_X + x, 8.0, 0xfcd000, 15.0, ());

        // Sword row: hero body at the real player layer, three sweep phases around it.
        let (hx, hy) = (x, 40.0);
        commands.spawn((
            Sprite::from_image(hero.0.frames[f][0].clone()),
            at(PLAY_X + hx, PLAY_Y + hy, 16.0, 16.0, actor_z(hy + 16.0)),
            PIXEL_LAYER,
        ));
        for (i, life) in [12, 7, 2].into_iter().enumerate() {
            let mut s = Swing { life: life + 1, facing: f, tool: Tool::Sword, tool_tier: 0, grow: 0.0, chop: false }; // tick decrements first
            let (_, rot, pivot, _) = swing_tick(&mut s, hx, hy);
            let mut tf = at(PLAY_X + pivot.x, PLAY_Y + pivot.y, 0.0, 0.0, swing_z(f, actor_z(hy + 16.0)));
            tf.rotation = Quat::from_rotation_z(-rot);
            let (mut sprite, anchor) = sword_visual(&art);
            sprite.color = PHASE_TINT[i];
            commands.spawn((sprite, anchor, tf, PIXEL_LAYER));
        }

        // Axe row: goblin body, three sweep phases of its swipe.
        let (gx, gy) = (x, 120.0);
        let (fx, fy, _, _) = FACE[f];
        commands.spawn((
            Sprite::from_image(goblins.0[0][f][0].clone()),
            at(PLAY_X + gx, PLAY_Y + gy, 16.0, 16.0, actor_z(gy + 16.0)),
            PIXEL_LAYER,
        ));
        for (i, life) in [14, 8, 2].into_iter().enumerate() {
            let mut a =
                AxeSwipe { life: life + 1, fx, fy, wielder: Entity::PLACEHOLDER, ox: gx, oy: gy };
            let (_, rot, pivot, _) = axe_tick(&mut a);
            let mut tf = at(PLAY_X + pivot.x, PLAY_Y + pivot.y, 0.0, 0.0, axe_z(fy, actor_z(gy + 16.0)));
            tf.rotation = Quat::from_rotation_z(-rot);
            let (mut sprite, anchor) = axe_visual(&art);
            sprite.color = PHASE_TINT[i];
            commands.spawn((sprite, anchor, tf, PIXEL_LAYER));
        }
    }
}

/// Shoot after the renderer has warmed up, then exit once the save has had time.
/// (Frame 90, not sooner: on the FIRST launch after a fresh compile, Metal is still
/// compiling pipelines around frame 40 and the capture comes back solid black. When the
/// machine is in one of its black-frame moods, WRIFT_SHOT_FRAME pushes the capture later.)
fn capture(mut commands: Commands, mut frames: Local<u32>, mut exit: MessageWriter<AppExit>) {
    *frames += 1;
    let shoot: u32 = std::env::var("WRIFT_SHOT_FRAME").ok().and_then(|v| v.parse().ok()).unwrap_or(90);
    if *frames == shoot {
        let path = std::env::var("WRIFT_SHOT_PATH").unwrap_or_else(|_| "wriftshot.png".into());
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
    if *frames == shoot + 80 {
        exit.write(AppExit::Success);
    }
}
