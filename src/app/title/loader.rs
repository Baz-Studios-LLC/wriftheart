//! loader.rs — the title's slot loader: swap the LIVE world onto another save (or a fresh
//! one) without relaunching. The world boots into the newest slot (save.rs); CONTINUE,
//! LOAD and NEW all funnel through one [`LoadSlot`] message so there is exactly one
//! "make the world match this save" path.

use crate::actors::props::PropArt;
use crate::app::battle::{despawn_room_actors, RoomActor};
use crate::app::gather::GatherState;
use crate::app::play::{
    ActiveRoot, CurGrid, CurRoom, GameWorld, Player, SlideActive, SlideState, HP_BASE,
};
use crate::app::room_props::RoomBlockers;
use crate::app::room_render::spawn_room_root;
use crate::app::rewards::Progress;
use crate::app::save::{apply_to, read_slot, scan_metas, write_save, SaveCtx, SlotMetas};
use crate::app::screen::Screen;
use crate::app::slideout::{skills_tab, TreeAlloc};
use crate::combat::{Health, Hitbox};
use crate::gfx::TileTextures;
use crate::inventory::PlayerInv;
use crate::room::{RoomGrid, PX_H, PX_W};
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

/// "Make the live world match slot N" (fresh = ignore the file, start over in it —
/// with `seed` as the new world's seed; a load uses the seed saved in the slot).
#[derive(Message)]
pub struct LoadSlot {
    pub slot: u32,
    pub fresh: bool,
    pub seed: Option<u32>,
}

/// The room-swap resources, grouped under Bevy's 16-param cap.
#[derive(SystemParam)]
pub struct SwapCtx<'w> {
    pub(crate) world: ResMut<'w, GameWorld>,
    pub(crate) tex: Res<'w, TileTextures>,
    pub(crate) prop_art: ResMut<'w, PropArt>,
    pub(crate) active: ResMut<'w, ActiveRoot>,
    pub(crate) grid: ResMut<'w, CurGrid>,
    pub(crate) blockers: ResMut<'w, RoomBlockers>,
    pub(crate) slide: ResMut<'w, SlideState>,
    pub(crate) sliding: ResMut<'w, SlideActive>,
    pub(crate) inside: ResMut<'w, crate::app::interior::Inside>,
    pub(crate) in_dungeon: ResMut<'w, crate::app::dungeon::InDungeon>,
    pub(crate) dungeon_lights: ResMut<'w, crate::app::dungeon::DungeonLights>,
    pub(crate) banners: ResMut<'w, crate::app::banners::Banners>,
    pub(crate) room_cache: ResMut<'w, crate::app::room_cache::RoomCache>,
    pub(crate) armed: ResMut<'w, crate::app::encounters::ArmedEncounter>,
    pub(crate) rng: ResMut<'w, crate::app::battle::GameRng>,
    pub(crate) human_art: ResMut<'w, crate::actors::goblin::HumanArt>,
}

/// Song of Returning: carry the hero to a remembered town's room centre (the js warp
/// charge animation is flagged for later — this is the landing half).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn handle_warp(
    mut reqs: MessageReader<crate::app::flute::WarpTo>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut ctx: SaveCtx,
    mut swap: SwapCtx,
    caves: Res<super::super::caves::CrackCaves>,
    songs_opened: Res<super::super::caves::OpenedSongstones>,
    actors: Query<Entity, With<RoomActor>>,
    house: Res<crate::app::home::PlayerHouse>,
    mut players: Query<(&mut Player, &mut Health)>,
) {
    let Some(req) = reqs.read().last() else { return };
    swap_world_room(&mut commands, &mut images, &mut swap, &mut ctx, &caves, &songs_opened, &actors, req.rx, req.ry, house.0.as_ref().map(|h| h.room));
    if let Ok((mut p, mut h)) = players.single_mut() {
        if req.home && let Some(hs) = house.0.as_ref() {
            // js placeAtHouseDoor: centred on the door, a step below it.
            p.x = hs.x + 3.0;
            p.y = (hs.y + 28.0).min(PX_H as f32 - 24.0);
            p.facing = crate::actors::hero::Facing::Up;
        } else {
            p.x = (PX_W / 2 - 8) as f32;
            p.y = (PX_H / 2 - 8) as f32;
            p.facing = crate::actors::hero::Facing::Down;
        }
        h.invuln = 50; // js: warp landings arrive with a longer mercy window
    }
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn handle_load_slot(
    mut reqs: MessageReader<LoadSlot>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut ctx: SaveCtx,
    mut extras: super::super::save::SaveExtras,
    mut swap: SwapCtx,
    mut metas: ResMut<SlotMetas>,
    actors: Query<Entity, With<RoomActor>>,
    mut players: Query<(&mut Player, &mut Health, &mut Hitbox, &mut crate::combat::Knockback)>,
    mut hero_art: ResMut<crate::app::play::HeroArt>,
    mut cutscene: ResMut<crate::app::cinematic::Cutscene>,
    mut next: ResMut<NextState<Screen>>,
) {
    let Some(req) = reqs.read().last() else { return };
    ctx.social.dungeon_ledger.0.clear(); // a slot switch starts from ITS OWN history (restored below on load)
    ctx.social.relics.0.clear();
    let data = if req.fresh { None } else { read_slot(req.slot) };
    if !req.fresh && data.is_none() {
        return; // the picker only offers occupied slots; a vanished file just no-ops
    }
    ctx.active.0 = req.slot;
    swap.room_cache.0.clear(); // js: same-day snapshots don't cross a load — regen from records

    // --- Resources: the saved state, or a clean slate (the fresh-boot defaults). ---
    // A FRESH start keeps whatever HeroIdent the creator just wrote; a load restores the
    // slot's own hero (apply_to).
    match &data {
        Some(d) => apply_to(d, &mut ctx, &mut extras),
        None => {
            extras.guilds.0.clear();
            extras.stations.0.clear();
            extras.caves.0.clear();
            extras.songs.0.clear();
            extras.tmaps.0.clear();
            extras.side_looted.0.clear();
            extras.guards.0 = false;
            extras.victory.won = false;
            extras.rune.0 = "arcane";
            // The rest of SaveExtras leaked the OLD slot into a new game (Baz died
            // and respawned at a house he never built): a fresh start owns NONE of
            // the last life's house, spawn point, stash, blueprints, pins, or goblin.
            *extras.house = Default::default();
            *extras.respawn = Default::default();
            *extras.blueprints = Default::default();
            *extras.stash = Default::default();
            *extras.pins = Default::default();
            *extras.loot_gob = Default::default();
            *extras.loot_gob_cleared = Default::default();
            *ctx.inv = PlayerInv::default();
            *ctx.progress = Progress::default();
            *ctx.alloc = TreeAlloc::default();
            *ctx.tstats = skills_tab::recompute(&ctx.alloc, &ctx.ident.traits, false, &ctx.inv);
            ctx.stats.0.clear();
            ctx.bestiary.0.clear();
            ctx.discovered.0.clear();
            *ctx.gather = GatherState::default();
            ctx.growth.0.clear();
            ctx.visited.0 = HashSet::from([(0, 0)]);
            ctx.clock.0 = 0;
            ctx.town_names.0.clear();
            ctx.social.bought.forever.clear();
            ctx.social.bought.today.clear();
            ctx.social.people.0.clear();
            *ctx.social.farm = crate::app::farm::FarmTiles::default();
            *ctx.social.can_water = crate::app::farm::CanWater::default();
            *ctx.social.farm_day = crate::app::farm::LastFarmDay::default();
            ctx.social.cleared.0.clear();
            ctx.social.quests.0.clear();
            ctx.social.giver_done.0.clear();
            ctx.social.quest_counter.0 = 0;
            ctx.social.songs.0.clear();
            ctx.social.awards.0.clear();
            ctx.social.met_wanderers.0.clear();
            *ctx.social.fest = crate::app::festivals::FestivalLedger::default();
            *ctx.social.livestock = Default::default();
        }
    }
    // Re-bake the hero's sprite bank in this save's look (hair colours, style, the works).
    *hero_art = crate::app::play::HeroArt(crate::actors::hero::build_frames(&ctx.ident.look, &mut images));

    // --- The world seed: a new game rolls its own; a load restores the slot's. A change
    // rebuilds the World (its caches key on the seed; room spawns below regenerate). ---
    let want_seed = match (&data, req.seed) {
        (Some(d), _) if d.seed != 0 => d.seed,
        (None, Some(s)) => s,
        _ => 1337, // pre-seed saves stored 0 (serde default)
    };
    if swap.world.0.seed != want_seed {
        swap.world.0 = crate::worldgen::World::new(want_seed);
    }

    // --- Room swap: the old room (props ride its root), cast and ground loot leave. ---
    let (rx, ry) = data.as_ref().map_or((0, 0), |d| (d.rx, d.ry));
    swap_world_room(&mut commands, &mut images, &mut swap, &mut ctx, &extras.caves, &extras.songs, &actors, rx, ry, extras.house.0.as_ref().map(|h| h.room));

    // --- The hero lands where the save left him (or room centre, facing down). ---
    let Ok((mut p, mut h, mut hb, mut kb)) = players.single_mut() else { return };
    kb.timer = 0; // no shove survives a world swap (js: every teleport clears knockTimer)
    p.grapple = None; // ...nor any reel/leap in flight (js clears p.grapple/p.hop on load)
    p.hop = None;
    p.dash = None;
    p.dash_cd = 0;
    p.hop_z = 0.0;
    let (px, py) = data
        .as_ref()
        .map_or(((PX_W / 2 - 8) as f32, (PX_H / 2 - 8) as f32), |d| (d.px, d.py));
    p.x = px;
    p.y = py;
    p.facing = crate::actors::hero::Facing::Down;
    p.anim_frame = 0;
    p.anim_timer = 0;
    p.moving = false;
    p.cooldowns = [0; 4];
    p.lock_timer = 0;
    *h = data.as_ref().map_or(
        Health { hp: HP_BASE, max: HP_BASE, defense: 0, invuln: 0, flash: 0 },
        |d| Health { hp: d.hp.max(1), max: d.max_hp.max(HP_BASE), defense: d.defense, invuln: 0, flash: 0 },
    );
    *hb = Hitbox { x: px + 3.0, y: py + 2.0, w: 10.0, h: 13.0 };

    // Claim the slot NOW (a new game's card shows up when you next quit to the title;
    // a load refreshes its timestamp so CONTINUE keeps resuming it).
    write_save(&ctx, &extras, &p, &h, swap.world.0.seed);
    *metas = scan_metas();
    if req.fresh {
        cutscene.0 = Some(0); // every new game opens on the story (js gameState 'cutscene')
    }
    next.set(Screen::Play);
}

/// Tear down the live room (root, cast, ground loot, any slide in flight) and stand up
/// room (rx, ry) in its place — shared by the slot loader and the death respawn.
/// (Ground pickups + their glows carry RoomActor, so the actor sweep takes them too.)
#[allow(clippy::too_many_arguments)] // the world-swap touches every room-scoped store
pub(crate) fn swap_world_room(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    swap: &mut SwapCtx,
    ctx: &mut SaveCtx,
    caves: &super::super::caves::CrackCaves,
    songs_opened: &super::super::caves::OpenedSongstones,
    actors: &Query<Entity, With<RoomActor>>,
    rx: i32,
    ry: i32,
    home_room: Option<(i32, i32)>,
) {
    commands.entity(swap.active.0).despawn();
    despawn_room_actors(commands, actors);
    swap.slide.0 = None;
    swap.sliding.0 = false;
    swap.inside.0 = None; // standing up an OUTDOOR room always ends any interior visit
    swap.in_dungeon.0 = None; // ...and any dungeon run (death + load land outdoors too)
    let grid = RoomGrid::from_map(&swap.world.0.generate(rx, ry));
    let ents = swap.world.0.room_entities(rx, ry);
    swap.armed.0 = None; // no half-fought camp survives a world swap
    let (root, blockers) = spawn_room_root(
        commands, images, &swap.tex, &mut swap.prop_art, &swap.world.0, &grid, &ents,
        &ctx.gather, &mut ctx.growth, &ctx.social.farm, &ctx.social.cleared, caves, songs_opened, rx, ry, Vec2::ZERO, ctx.clock.0,
    );
    swap.active.0 = root;
    swap.grid.0 = grid;
    swap.blockers.0 = blockers;
    *ctx.cur = CurRoom { rx, ry };
    ctx.visited.0.insert((rx, ry));
    // A same-day snapshot re-seats exactly what was left (death respawns + door exits
    // walk back into the world they remember; loads cleared the cache above).
    crate::app::room_cache::spawn_or_restore(
        commands,
        images,
        &mut swap.rng,
        &mut swap.human_art,
        &swap.room_cache,
        &swap.world.0,
        &ctx.social.cleared,
        &mut swap.armed,
        &ents,
        (rx, ry),
        crate::app::gather::farm_day(ctx.clock.0),
        home_room == Some((rx, ry)), // the home room is a mob-free safe zone (Baz)
    );
    swap.banners.anchor(&swap.world.0, rx, ry); // silent arrival — no announcement
}
