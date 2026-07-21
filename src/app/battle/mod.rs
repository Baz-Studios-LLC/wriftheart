//! battle/ — the combat orchestration systems, split by lifecycle:
//! [`ai`] the brains (goblin + biome-mob think, knockback), [`projectiles`] every
//! flying thing (player swings, webs, rocks, arrows, bolts), [`deaths`] the fall +
//! loot recipes, [`fx`] bursts, blood and the sprite syncs. This file holds the
//! shared spine: the plugin, [`RoomActor`], [`GameRng`], spawn/despawn of a room's
//! cast, and the `not_sliding` freeze rule (the JS transition freezes the world).

mod ai;
mod deaths;
mod fx;
pub(crate) mod projectiles; // pub(crate): the boss pass fires EBolts too

pub use fx::spawn_burst;

use super::play::SlideActive;
use crate::actors::goblin::{goblin_bundle, GoblinKind};
use crate::actors::mobs::{self, mob_bundle};
use crate::combat::{resolve_combat, tick_health, HitLanded, Tinked};
use crate::gfx::PIXEL_LAYER;
use crate::worldgen::rng::Mulberry32;
use crate::worldgen::RoomEntity;
use bevy::prelude::*;

/// Marker: belongs to the current room's live cast — despawned wholesale on room change.
#[derive(Component)]
pub struct RoomActor;

/// Runtime (non-deterministic) RNG — the port of the JS's Math.random() call sites.
#[derive(Resource)]
pub struct GameRng(pub Mulberry32);

pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameRng(Mulberry32::new(0x5eed)))
            .add_message::<HitLanded>()
            .add_message::<Tinked>()
            .add_systems(Startup, |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
                commands.insert_resource(mobs::MobArtBank::build(&mut images));
            })
            .add_systems(
                FixedUpdate,
                (
                    water_mob_wake,
                    ai::goblin_ai,
                    ai::mobs_ai,
                    projectiles::attacks_tick,
                    projectiles::mob_projectiles_tick,
                    projectiles::enemy_shots_tick,
                    projectiles::block_shots_on_props,
                    resolve_combat,
                    ai::apply_knockback,
                    ai::apply_mob_knockback,
                    tick_health,
                    deaths::deaths,
                    deaths::mob_deaths,
                    fx::blood_fx,
                    fx::particles_tick,
                )
                    .chain()
                    .run_if(not_sliding)
                    .after(super::play::tick),
            )
            .add_systems(Update, (fx::sync_goblins, fx::sync_mobs, fx::sync_attacks));
        app.init_resource::<crate::actors::goblin::HumanArt>();
    }
}

pub fn not_sliding(
    slide: Res<SlideActive>,
    screen: Res<State<super::screen::Screen>>,
) -> bool {
    !slide.0 && *screen.get() == super::screen::Screen::Play
}

/// Spawn the room's mobs from its entity layout (positions are now prop-aware and
/// byte-parity with the JS — see worldgen/entities.rs).
#[allow(clippy::too_many_arguments)] // room composition needs the room context
pub fn spawn_room_mobs(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    rng: &mut GameRng,
    human_art: &mut crate::actors::goblin::HumanArt,
    world: &crate::worldgen::World,
    cleared: &super::encounters::ClearedEncounters,
    armed: &mut super::encounters::ArmedEncounter,
    ents: &[RoomEntity],
    room: (i32, i32),
) {
    // Safe havens never host foes (js noMobs): the start room and the burnt home
    // village (the castle grounds + player home join as they port).
    if room == (0, 0) || room == super::room_props::HOME_VILLAGE {
        return;
    }
    // A beaten encounter room stays PEACEFUL forever — no camp, no natural mobs (js).
    if cleared.0.contains(&room) {
        return;
    }
    // An encounter takes the room's mob slot: its curated roster replaces the natural
    // roll entirely (friendly ones spawn no foes at all).
    if let Some((def, _)) = super::encounters::for_room(world, room.0, room.1) {
        let scene = super::encounters::build(def, world, room.0, room.1);
        for (kind, x, y) in &scene.foes {
            let (x, y) = (*x, *y);
            if let Some(idx) = mobs::def_index(kind) {
                commands.spawn((mob_bundle(idx, x, y), RoomActor, PIXEL_LAYER, super::encounters::EncFoe));
            } else {
                // Humanoids without a MobDef ride the goblin chassis (AI + combat),
                // spear for slingers. BANDITS and CULTISTS wear people art over it —
                // a seeded villager in the kind's costume (Baz: "people in costumes").
                let gk = if *kind == "slinger" { GoblinKind::Spear } else { GoblinKind::Melee };
                let mut e = commands.spawn((goblin_bundle(gk, x, y), RoomActor, PIXEL_LAYER, super::encounters::EncFoe));
                e.insert(Sprite::default());
                if matches!(*kind, "bandit" | "cultist") {
                    let seed = (x as i32 as u32).wrapping_mul(2654435761) ^ (y as i32 as u32).wrapping_mul(97) ^ 0xba9d;
                    let frames = human_art.frames(kind, seed, images);
                    e.insert(crate::actors::goblin::HumanSkin { kind, seed, frames });
                }
            }
        }
        super::encounters::spawn_victims(commands, &scene);
        if !scene.foes.is_empty() {
            armed.0 = Some(room);
        }
        return;
    }
    for m in ents {
        let (x, y) = (m.x as f32, m.y as f32);
        // The Black Castle's gate guardians (js 'guard' rows) — until both fall.
        if m.kind == "guard" {
            continue; // darkknight.rs stands them up (guard_wake) — persistence lives there
        }
        // Biome mobs with a ported def spawn REAL; everything else (unported kinds, plain
        // goblins) falls back to the goblin placeholder — spear for slingers.
        if m.kind == "mob" && m.sub == "lootgoblin" {
            // The money goblin lives in the saved LootGob (app/lootgoblin.rs lootgob_load),
            // NOT the room roster — so it persists + relocates across rooms. Skip it here.
            continue;
        }
        let ent = if m.kind == "mob"
            && let Some(idx) = mobs::def_index(m.sub.as_str())
        {
            commands.spawn((mob_bundle(idx, x, y), RoomActor, PIXEL_LAYER)).id()
        } else {
            let kind = match m.kind {
                "goblin" => {
                    if m.sub == "spear" { GoblinKind::Spear } else { GoblinKind::Melee }
                }
                "mob" => GoblinKind::Melee, // an unported biome mob falls back to a goblin
                _ => continue,
            };
            let mut e = commands.spawn((goblin_bundle(kind, x, y), RoomActor, PIXEL_LAYER));
            e.insert(Sprite::default());
            e.id()
        };
        // The js promotion (makeChampion/makeElite): stats, affixes, and the aura.
        if m.elite {
            super::champions::promote(commands, images, ent, true, &mut || rng.0.next_f64());
        } else if m.champ {
            super::champions::promote(commands, images, ent, false, &mut || rng.0.next_f64());
        }
    }
}

/// Clear the previous room's cast (mobs, attacks, particles).
pub fn despawn_room_actors(commands: &mut Commands, actors: &Query<Entity, With<RoomActor>>) {
    for e in actors {
        commands.entity(e).despawn();
    }
}

/// THE WATERS' rare lurkers (Baz, past-js): at most ONE water mob per room, seeded
/// deterministically (~1 watery room in 8 — the same rooms every time, so they read
/// as authored haunts, never a chore). spitgill takes open water; tidewhip wants a
/// shore tile so its whip can reach a walking hero. The station_wake idiom: keyed
/// on the room, swept by the room's own RoomActor teardown.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn water_mob_wake(
    mut commands: Commands,
    cur: Res<super::play::CurRoom>,
    sliding: Res<super::play::SlideActive>,
    inside: Res<super::interior::Inside>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    world: Res<super::play::GameWorld>,
    grid: Res<super::play::CurGrid>,
    mut woke: Local<Option<(i32, i32)>>,
) {
    if sliding.0 || inside.0.is_some() || in_dungeon.0.is_some() {
        *woke = None;
        return;
    }
    let room = (cur.rx, cur.ry);
    if *woke == Some(room) {
        return;
    }
    *woke = Some(room);
    if room == (0, 0) || room == super::room_props::HOME_VILLAGE || world.0.is_town(room.0, room.1) {
        return;
    }
    let h = (room.0 as u32).wrapping_mul(2654435761) ^ (room.1 as u32).wrapping_mul(97) ^ world.0.seed;
    if !h.is_multiple_of(8) {
        return;
    }
    use crate::room::{COLS, ROWS};
    let mut water: Vec<(i32, i32, bool)> = Vec::new(); // (c, r, has land neighbour)
    for r in 1..ROWS - 1 {
        for c in 1..COLS - 1 {
            if grid.0.code_at(c, r) == '~' {
                let shore = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|(dc, dr)| grid.0.code_at(c + dc, r + dr) != '~' && !grid.0.box_hits_solid((c + dc) as f32 * 16.0 + 8.0, (r + dr) as f32 * 16.0 + 8.0, 1.0, 1.0));
                water.push((c, r, shore));
            }
        }
    }
    if water.len() < 8 {
        return; // a puddle earns no lurker
    }
    // The pair is the BIOME's (Baz: kin biomes share, strangers differ): the murk
    // partition (water_style) hosts the bog pair, the arctic its frost sniper,
    // everywhere temperate the originals.
    let biome = world.0.biome_key_at(room.0, room.1);
    let murk = world.0.water_style(room.0 * crate::room::COLS, room.1 * crate::room::ROWS) == "murk";
    let (spit_kind, whip_kind) = if murk {
        ("bogmaw", "mirelash")
    } else if biome == "arctic" {
        ("frostgill", "tidewhip")
    } else {
        ("spitgill", "tidewhip")
    };
    let whip = (h >> 9) & 1 == 0;
    let pool: Vec<&(i32, i32, bool)> = if whip {
        let shore: Vec<&(i32, i32, bool)> = water.iter().filter(|(_, _, s)| *s).collect();
        if shore.is_empty() { water.iter().collect() } else { shore }
    } else {
        water.iter().collect()
    };
    let &&(c, r, _) = &pool[(h >> 3) as usize % pool.len()];
    let kind = if whip { whip_kind } else { spit_kind };
    if let Some(idx) = crate::actors::mobs::def_index(kind) {
        commands.spawn((crate::actors::mobs::mob_bundle(idx, (c * 16) as f32, (r * 16) as f32), RoomActor, crate::gfx::PIXEL_LAYER));
    }
}
