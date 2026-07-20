//! play.rs — the playable overworld: JS-exact player movement, combat wiring, and the
//! Zelda room-slide.
//!
//! Port notes (js/player.js + game.js):
//! * SPEED 1.25 px/frame at 60Hz; diagonals x sqrt(1/2); per-axis collision (walls slide you).
//! * Feet box (2, 8) 12x8; full-body hitbox (3, 2) 10x13 for combat.
//! * Sword: cooldown 20, lock 14 (move at 55% mid-swing), swing entity carries the damage.
//! * Hurt: 72 i-frames + blink, knockback 2.6 for 8 frames (movement yields to it).
//! * Crossing: pushing an edge with the body centre within 12px slides both rooms over PX/8
//!   frames; the previous room's cast despawns at slide start, the new roster spawns on land.

use super::battle::{despawn_room_actors, spawn_room_mobs, RoomActor};
use super::gather::{GatherState, TreeGrowth};
use super::slideout::TreeStats;
use super::room_props::{sway_grass, RoomBlockers};
use super::room_render::{actor_z, animate_water, spawn_room_root, FrameClock, PLAY_X, PLAY_Y};
use crate::actors::props::PropArt;
use crate::actors::attacks::{build_attack_art, swing_bundle, swing_spec, AttackArt};
use crate::actors::goblin::build_goblin_art;
use crate::actors::hero::{self, Facing, HeroFrames};
use crate::combat::{Combatant, Health, Hitbox, HurtProfile, Knockback, Team};
use crate::gfx::{at, TileTextures, PIXEL_LAYER};
use crate::ui::label;
use crate::input::{clear_pressed, poll_input, Action, ActionState, Bindings};
use crate::room::{RoomGrid, COLS, PX_H, PX_W, ROWS};
use crate::worldgen::World;
use crate::CANVAS_H;
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

const SPEED: f32 = 1.25;
const ANIM_TICKS: u32 = 8;
const BOX_OX: f32 = 2.0; // feet collision box, sprite-local
const BOX_OY: f32 = 8.0;
const BOX_W: f32 = 12.0;
const BOX_H: f32 = 8.0;
const EDGE_REACH: f32 = 12.0;
pub const HP_BASE: i32 = 3; // max HP at level 1 with no Vitality (js/player.js)

pub struct PlayPlugin;

impl Plugin for PlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<super::screen::Screen>()
            .init_resource::<Bindings>()
            .init_resource::<ActionState>()
            .init_resource::<FrameClock>()
            .init_resource::<Pulled>()
            .init_resource::<Hexed>()
            .init_resource::<crate::inventory::PlayerInv>()
            .insert_resource(Visited(HashSet::from([(0, 0)])))
            .init_resource::<RoomBlockers>()
            .insert_resource(SlideActive(false))
            .insert_resource(Time::<Fixed>::from_hz(60.0)) // the JS runs a fixed 60Hz update
            .add_systems(Startup, setup)
            .init_resource::<crate::input::DpadDirs>()
            .add_systems(PreUpdate, (set_dpad_dirs, poll_input).chain())
            .add_systems(
                FixedUpdate,
                (
                    tick.run_if(super::screen::playing),
                    crate::actors::villager::villager_tick
                        .run_if(super::screen::playing)
                        .run_if(super::battle::not_sliding),
                    // Presses are consumed ONCE per fixed tick (the JS endFrame contract).
                    // Every UI system that reads presses must be ordered before this.
                    clear_pressed.after(tick).after(super::menu::menu_tick).in_set(EndTick),
                ),
            )
            .init_resource::<crate::actors::villager::VillagerArt>()
            .add_systems(
                Update,
                (
                    animate_water,
                    sway_grass,
                    super::room_props::animate_torches,
                    crate::actors::villager::sync_villagers,
                    // The death scene hides the body behind its corpse sprite — the sync
                    // must not flip visibility back on.
                    sync_player_sprite
                        .run_if(not(in_state(super::screen::Screen::Dead)))
                        .run_if(|p: Res<super::dungeon::PitFalling>| p.0.is_none()),
                    relabel_coords,
                    worn_refresh,
                    apply_tree_hp,
                ),
            );
    }
}

/// The D-pad is arrows in ANY non-free-roam screen (js dpadDirs) — menus, codex, the
/// title, the death choice — and the shortcut cluster only in open play.
fn set_dpad_dirs(
    screen: Res<State<super::screen::Screen>>,
    fluting: Res<super::flute::Fluting>,
    mut dirs: ResMut<crate::input::DpadDirs>,
) {
    // Mid-song the D-pad is the four notes, not the shortcut cluster (js dpadDirs).
    dirs.0 = *screen.get() != super::screen::Screen::Play || fluting.0.is_some();
}

/// The press-consumption boundary of each fixed tick: `clear_pressed` lives here; systems
/// that read presses order themselves `.before(EndTick)`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EndTick;

/// Every overworld room the player has entered — the codex MAP tab draws exactly this set.
/// (Persists with the save once saves port.)
#[derive(Resource)]
pub struct Visited(pub HashSet<(i32, i32)>);

#[derive(Resource)]
pub struct GameWorld(pub World);

#[derive(Resource, PartialEq, Eq, Clone, Copy)]
pub struct CurRoom {
    pub rx: i32,
    pub ry: i32,
}

#[derive(Resource)]
pub struct CurGrid(pub RoomGrid);

#[derive(Resource)]
pub struct HeroArt(pub HeroFrames);

/// True while a room-slide is in flight — battle systems freeze on it (the JS transition
/// freezes the world).
#[derive(Resource)]
pub struct SlideActive(pub bool);

/// The active room's tile root (a slide scrolls it out and replaces it).
#[derive(Resource)]
pub struct ActiveRoot(pub Entity);

/// A room-slide in flight — port of the JS `transition` state.
pub struct Slide {
    frame: u32,
    total: u32,
    start: Vec2,
    end: Vec2,
    delta: Vec2,
    old_root: Entity,
    new_root: Entity,
}

#[derive(Resource, Default)]
pub struct SlideState(pub Option<Slide>);

impl SlideState {
    /// The INCOMING room's current scroll offset in SCREEN px (top-left space), while a
    /// slide is in flight. Free sprites/overlays anchored to room-local coords add this
    /// to ride the slide exactly like root children do (the torch-light fix — Baz).
    pub fn incoming_offset(&self) -> Option<(f32, f32)> {
        self.0.as_ref().map(|s| {
            let t = (s.frame as f32 / s.total as f32).min(1.0);
            (s.delta.x * (1.0 - t), -s.delta.y * (1.0 - t)) // Bevy y-up -> screen y-down
        })
    }
}

/// Begin a room slide — the DUNGEON walk drives the same machinery (play.rs owns the
/// Slide fields; tick's in-flight branch does the rest for either world).
#[allow(clippy::too_many_arguments)] // a slide is its whole geometry
pub(crate) fn start_slide(
    slide: &mut SlideState,
    sliding: &mut SlideActive,
    old_root: Entity,
    new_root: Entity,
    start: Vec2,
    end: Vec2,
    delta: Vec2,
    horizontal: bool,
) {
    slide.0 = Some(Slide {
        frame: 0,
        total: if horizontal { (PX_W / 8) as u32 } else { (PX_H / 8) as u32 },
        start,
        end,
        delta,
        old_root,
        new_root,
    });
    sliding.0 = true;
}

#[derive(Component)]
pub struct Player {
    pub x: f32,
    pub y: f32,
    pub facing: Facing,
    pub anim_frame: usize,
    pub anim_timer: u32,
    pub moving: bool,
    pub cooldowns: [u32; 4], // per-ability-slot cooldown timers (js p.cooldowns)
    pub lock_timer: u32, // rooted-swing frames: move at 55%, facing free (the walk-and-attack rule)
    pub blocking: bool, // a shield is raised (js p.blocking — recomputed every free tick)
    pub block_uid: Option<u32>, // WHICH shield (durability wears on this entry)
    pub grapple: Option<Grapple>, // reeled toward a lodged hook (js p.grapple)
    pub hop: Option<Hop>,         // a spring-boots leap in flight (js p.hop)
    pub hop_z: f32,               // the leap's draw-height offset (js p.hopZ)
    pub vx: f32,                  // carried velocity (only meaningful on slippery ice, js p.vx/vy)
    pub vy: f32,
}

/// The reel toward a lodged grapple hook (js p.grapple {tx,ty,t}).
#[derive(Clone, Copy)]
pub struct Grapple {
    pub tx: f32,
    pub ty: f32,
    pub t: i32,
}

/// A spring-boots leap: a lerp arc from start to target (js p.hop).
#[derive(Clone, Copy)]
pub struct Hop {
    pub sx: f32,
    pub sy: f32,
    pub tx: f32,
    pub ty: f32,
    pub t: i32,
    pub total: i32,
}

#[derive(Component)]
struct CoordsLabel;

/// The non-overworld modes (interior / dungeon), nested (RoomCtx sits AT the 16 cap).
#[derive(SystemParam)]
pub struct ModeCtx<'w> {
    pub inside: Res<'w, super::interior::Inside>,
    pub dungeon: Res<'w, super::dungeon::InDungeon>,
    pub relics: Res<'w, super::dungeon::Relics>,
    pub fishing: Res<'w, super::fishing::Fishing>,
    pub fluting: Res<'w, super::flute::Fluting>,
    /// Read-only for the slide-in room build (farm.rs owns the mutations).
    pub farm: Res<'w, super::farm::FarmTiles>,
    pub cleared: Res<'w, super::encounters::ClearedEncounters>,
    pub pit: Res<'w, super::dungeon::PitFalling>,
    /// Mutable: tick applies the drag AND clears it (arrival/timeout/wedge/hit).
    pub pulled: ResMut<'w, Pulled>,
    /// Read-only: status effects scale movement (slow x0.5, shock x0.3, +move buffs).
    pub statuses: Res<'w, super::status::Statuses>,
    /// Mutable: tick burns the hex down while applying it.
    pub hexed: ResMut<'w, Hexed>,
    /// Read-only: an open guild checklist owns the keys (js guildDonate freeze).
    pub donate: Res<'w, super::guildhall::DonateState>,
    /// Read-only: opened cave doors re-stand with their room (caves.rs).
    pub caves: Res<'w, super::caves::CrackCaves>,
    /// Read-only: sung-open songstones re-stand as doors (caves.rs).
    pub songs_opened: Res<'w, super::caves::OpenedSongstones>,
    /// Read-only: the hidden side-view chamber owns movement while it's up.
    pub side: Res<'w, super::sidescroll::SideScroll>,
    /// Read-only: the opening cinematic owns the whole frame (ModeCtx is AT the cap).
    pub cutscene: Res<'w, super::cinematic::Cutscene>,
}

/// The hero, HEXED (the Unmaker's rule-theft): held directions are MIRRORED while
/// frames remain — left is right, up is down. Burned down by tick like Slowed.
#[derive(Resource, Default)]
pub struct Hexed(pub i32);

/// The consumable branch's hand-off writers (js each item's use() env), bundled —
/// tick sits at the 16-param cap, and every new usable item was costing a slot.
#[derive(bevy::ecs::system::SystemParam)]
pub struct UseRoutes<'w> {
    pub farm: MessageWriter<'w, super::farm_animals::UseFarmItem>,
    pub eats: MessageWriter<'w, super::status::EatDish>,
    pub stations: MessageWriter<'w, super::cooking::PlaceStation>,
    pub houses: MessageWriter<'w, super::home::PlaceHouse>,
    pub sleep: MessageWriter<'w, super::services::SleepRequest>,
    pub blueprints: MessageWriter<'w, super::blueprints::LearnBlueprint>,
    pub bombs: MessageWriter<'w, super::caves::DropBomb>,
    pub maps: MessageWriter<'w, super::digging::ReadMap>,
    pub boomerangs: MessageWriter<'w, super::uniques::ThrowBoomerang>,
    pub arrows: MessageWriter<'w, super::archery::FireArrow>,
    pub wands: MessageWriter<'w, super::wands::WandMsg>,
    pub cures: MessageWriter<'w, super::status::CureStatus>,
    pub hooks: MessageWriter<'w, super::traversal::FireHook>,
    /// The proc rolls (swing chill/scorch) share the battle rng.
    pub rng: ResMut<'w, super::battle::GameRng>,
    /// Seeded people-in-costume frames for humanoid foes (bandits) — room spawns.
    pub human_art: ResMut<'w, crate::actors::goblin::HumanArt>,
    pub beams: MessageWriter<'w, super::saltmaze::FireBeam>,
    pub sfx: MessageWriter<'w, super::sfx::Sfx>,
}

impl UseRoutes<'_> {
    /// The dry click a vetoed use plays (js Sound.sfx('tink')).
    fn sfx_tink(&mut self) {
        self.sfx.write(super::sfx::Sfx("tink"));
    }
}

/// The 8-way aim for fired traversal gadgets (js aimVec) — held movement wins,
/// facing as the fallback. Takes facing by value (the hook only needs the vector).
pub(crate) fn traversal_aim(state: &ActionState, facing: crate::actors::hero::Facing) -> (f32, f32) {
    use crate::actors::hero::Facing;
    let dx = (state.held(Action::Right) as i32 - state.held(Action::Left) as i32) as f32;
    let dy = (state.held(Action::Down) as i32 - state.held(Action::Up) as i32) as f32;
    if dx == 0.0 && dy == 0.0 {
        return match facing {
            Facing::Up => (0.0, -1.0),
            Facing::Down => (0.0, 1.0),
            Facing::Left => (-1.0, 0.0),
            Facing::Right => (1.0, 0.0),
        };
    }
    let m = dx.hypot(dy);
    (dx / m, dy / m)
}

/// The bag-row a satchel tier grows the bag TO (js SATCHELS rows), or None.
fn satchel_target(id: &str) -> Option<usize> {
    match id {
        "satchel" => Some(2),
        "satchel2" => Some(3),
        "satchel3" => Some(4),
        "satchel4" => Some(5),
        _ => None,
    }
}

/// The hero, reeled in by a tongue (js p.pulled {tx,ty,t} — built for the mimic;
/// the frog's deferred lash can ride the same rig later). While Some, walking is
/// overridden by the drag — swinging still works, so you can fight the reel.
#[derive(Resource, Default)]
pub struct Pulled(pub Option<Pull>);
pub struct Pull {
    pub tx: f32,
    pub ty: f32,
    pub t: i32,
}

/// The room-state resources `tick` juggles, grouped so the system stays under Bevy's
/// 16-parameter limit (destructured at the top of `tick` back into their old names).
#[derive(SystemParam)]
pub struct RoomCtx<'w> {
    world: Res<'w, GameWorld>,
    tex: Res<'w, TileTextures>,
    cur: ResMut<'w, CurRoom>,
    grid: ResMut<'w, CurGrid>,
    slide: ResMut<'w, SlideState>,
    sliding: ResMut<'w, SlideActive>,
    active: ResMut<'w, ActiveRoot>,
    visited: ResMut<'w, Visited>,
    prop_art: ResMut<'w, PropArt>,
    blockers: ResMut<'w, RoomBlockers>,
    gather: Res<'w, GatherState>,
    growth: ResMut<'w, TreeGrowth>,
    modes: ModeCtx<'w>,
    banners: ResMut<'w, super::banners::Banners>,
    town_names: ResMut<'w, super::banners::TownNames>,
    room_cache: Res<'w, super::room_cache::RoomCache>, // 16 fields — AT the SystemParam cap
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    loaded: Res<super::save::Loaded>,
    mut armed: ResMut<super::encounters::ArmedEncounter>,
    mut rng: ResMut<super::battle::GameRng>,
) {
    let tex = TileTextures::build(&mut images);
    // The save's own world seed (js World.setSeed) — 1337 for fresh boots and pre-seed
    // saves (whose serde default is 0).
    let seed = loaded.0.as_ref().map_or(1337, |d| if d.seed == 0 { 1337 } else { d.seed });
    let world = World::new(seed);
    // A save resumes in ITS room with ITS gather/growth stamps (they shape the room spawn);
    // apply_save restores the rest of the resources right after startup.
    let (rx, ry) = loaded.0.as_ref().map_or((0, 0), |d| (d.rx, d.ry));
    let grid = RoomGrid::from_map(&world.generate(rx, ry));

    let mut prop_art = PropArt::build(&mut images);
    let ents = world.room_entities(rx, ry);
    let gather = loaded.0.as_ref().map_or_else(GatherState::default, |d| GatherState {
        rooms: d.gather.iter().map(|(gx, gy, day, tiles)| ((*gx, *gy), (*day, tiles.iter().copied().collect()))).collect(),
        placed: d.placed.iter().map(|(gx, gy, tiles)| ((*gx, *gy), tiles.iter().copied().collect())).collect(),
        tomes: d.lorebooks.iter().filter_map(|id| crate::lore_books::get(id).map(|b| b.id)).collect(),
    });
    let mut growth = loaded.0.as_ref().map_or_else(TreeGrowth::default, |d| {
        TreeGrowth(d.growth.iter().map(|(gx, gy, tiles)| ((*gx, *gy), tiles.iter().map(|(c, r, day)| ((*c, *r), *day)).collect())).collect())
    });
    // The save's hoed soil shapes the room spawn too (nothing natural on tilled tiles).
    let farm = loaded
        .0
        .as_ref()
        .map_or_else(super::farm::FarmTiles::default, |d| super::save::farm_from_save(&d.farm));
    // ...and its beaten camps stay beaten (encounter decor + rosters skip them).
    let cleared = super::encounters::ClearedEncounters(
        loaded.0.as_ref().map_or_else(Default::default, |d| d.cleared_encounters.iter().copied().collect()),
    );
    let caves = super::caves::CrackCaves(
        loaded.0.as_ref().map_or_else(Default::default, |d| d.crack_caves.clone()),
    );
    let songs_opened = super::caves::OpenedSongstones(
        loaded.0.as_ref().map_or_else(Default::default, |d| d.songstones.iter().cloned().collect()),
    );
    let (root, blockers) = spawn_room_root(
        &mut commands, &mut images, &tex, &mut prop_art, &world, &grid, &ents, &gather, &mut growth, &farm, &cleared, &caves, &songs_opened, rx, ry, Vec2::ZERO, loaded.0.as_ref().map_or(0, |d| d.clock),
    );
    commands.insert_resource(caves);
    commands.insert_resource(songs_opened);
    commands.insert_resource(gather);
    commands.insert_resource(growth);
    commands.insert_resource(farm);
    commands.insert_resource(RoomBlockers(blockers));
    commands.insert_resource(prop_art);
    commands.insert_resource(ActiveRoot(root));
    commands.insert_resource(CurRoom { rx, ry });
    commands.insert_resource(SlideState::default());

    // Art banks. The hero bakes in HIS saved look (apply_save fills HeroIdent right after
    // startup; the sprite bank needs the look NOW).
    let look = loaded.0.as_ref().map(|d| d.look.clone()).unwrap_or_default();
    let art = hero::build_frames(&look, &mut images);
    let first = art.frames[Facing::Down as usize][0].clone();
    commands.insert_resource(HeroArt(art));
    commands.insert_resource(build_goblin_art(&mut images));
    commands.insert_resource(build_attack_art(&mut images));

    // The hero, centred in the spawn room (or where the save left him), with his combat
    // side (js: hitbox (3,2) 10x13, 72 i-frames, knockback 2.6/8 — see p.onHurt).
    let (px, py) = loaded
        .0
        .as_ref()
        .map_or(((PX_W / 2 - 8) as f32, (PX_H / 2 - 8) as f32), |d| (d.px, d.py));
    commands.spawn((
        Player {
            x: px,
            y: py,
            facing: Facing::Down,
            anim_frame: 0,
            anim_timer: 0,
            moving: false,
            cooldowns: [0; 4],
            lock_timer: 0,
            blocking: false,
            block_uid: None,
            grapple: None,
            hop: None,
            hop_z: 0.0,
            vx: 0.0,
            vy: 0.0,
        },
        Combatant { team: Team::Player, hurt_team: None, damage: None, persistent: false, knock: 0.0 },
        {
            let (hp, max, def) = loaded.0.as_ref().map_or((HP_BASE, HP_BASE, 0), |d| (d.hp.max(1), d.max_hp.max(HP_BASE), d.defense));
            Health { hp, max, defense: def, invuln: 0, flash: 0 }
        },
        HurtProfile { invuln: 72, flash: 72, kb_base: 2.6, kb_frames: 8 },
        Knockback::default(),
        Hitbox { x: px + 3.0, y: py + 2.0, w: 10.0, h: 13.0 },
        Sprite::from_image(first),
        at(PLAY_X + px, PLAY_Y + py, 16.0, 16.0, 10.0),
        PIXEL_LAYER,
    ));

    // Room coords readout (the rest of the sidebar lives in hud.rs).
    let coords = format!("{rx},{ry}");
    label(&mut commands, &mut images, &coords, 8.0, CANVAS_H as f32 - 22.0, 0xa0a0a0, 18.2, CoordsLabel);

    // First room's cast.
    let world = GameWorld(world);
    let grid = CurGrid(grid);
    let mut human_art = crate::actors::goblin::HumanArt::default();
    spawn_room_mobs(&mut commands, &mut images, &mut rng, &mut human_art, &world.0, &cleared, &mut armed, &ents, (rx, ry));
    commands.insert_resource(human_art); // seeds the session cache (replaces the plugin default)
    commands.insert_resource(cleared);
    commands.insert_resource(world);
    commands.insert_resource(grid);
    commands.insert_resource(tex);
}

/// Port of `safeEntry`: keep the lane if clear, else slide along the entry edge.
fn safe_entry(grid: &RoomGrid, vertical: bool, ex: f32, ey: f32) -> Vec2 {
    let hits = |x: f32, y: f32| grid.box_hits_solid(x + BOX_OX, y + BOX_OY, BOX_W, BOX_H);
    if !hits(ex, ey) {
        return Vec2::new(ex, ey);
    }
    for d in 1..PX_W.max(PX_H) {
        let d = d as f32;
        if vertical {
            if ex + d <= (PX_W - 16) as f32 && !hits(ex + d, ey) {
                return Vec2::new(ex + d, ey);
            }
            if ex - d >= 0.0 && !hits(ex - d, ey) {
                return Vec2::new(ex - d, ey);
            }
        } else {
            if ey + d <= (PX_H - 16) as f32 && !hits(ex, ey + d) {
                return Vec2::new(ex, ey + d);
            }
            if ey - d >= 0.0 && !hits(ex, ey - d) {
                return Vec2::new(ex, ey - d);
            }
        }
    }
    Vec2::new(ex, ey)
}

/// The fixed-60Hz player/world tick: clock, slide-or-move, attack input, edge crossings.
#[allow(clippy::too_many_arguments)]
pub fn tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    state: Res<ActionState>,
    attack_art: Res<AttackArt>,
    mut clock: ResMut<FrameClock>,
    tstats: Res<TreeStats>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    ctx: RoomCtx,
    mut armed: ResMut<super::encounters::ArmedEncounter>,
    mut roots: Query<&mut Transform, With<super::room_render::RoomRoot>>,
    actors: Query<Entity, With<RoomActor>>,
    mut q: Query<(&mut Player, &mut Knockback, &mut Hitbox, &mut Health)>,
    mut uses: UseRoutes,
    descending: Res<super::dungeon::Descending>,
) {
    let RoomCtx {
        world,
        tex,
        mut cur,
        mut grid,
        mut slide,
        mut sliding,
        mut active,
        mut visited,
        mut prop_art,
        blockers: mut room_blockers,
        gather,
        mut growth,
        mut modes,
        mut banners,
        mut town_names,
        room_cache,
    } = ctx;
    clock.0 += 1;
    let Ok((mut p, mut kb, mut hitbox, mut health)) = q.single_mut() else { return };

    // --- A slide in flight: scroll the roots, lerp the player, land at the end. ---
    if let Some(s) = &mut slide.0 {
        s.frame += 1;
        let t = (s.frame as f32 / s.total as f32).min(1.0);
        p.x = s.start.x + (s.end.x - s.start.x) * t;
        p.y = s.start.y + (s.end.y - s.start.y) * t;
        p.anim_timer += 1; // the JS walks the gait at 6 ticks during a slide
        if p.anim_timer >= 6 {
            p.anim_timer = 0;
            p.anim_frame = (p.anim_frame + 1) & 3;
        }
        if let Ok(mut tf) = roots.get_mut(s.old_root) {
            tf.translation.x = -s.delta.x * t;
            tf.translation.y = -s.delta.y * t;
        }
        if let Ok(mut tf) = roots.get_mut(s.new_root) {
            tf.translation.x = s.delta.x * (1.0 - t);
            tf.translation.y = s.delta.y * (1.0 - t);
        }
        if s.frame >= s.total {
            commands.entity(s.old_root).despawn();
            active.0 = s.new_root;
            slide.0 = None;
            sliding.0 = false;
            if let Some(run) = &modes.dungeon.0 {
                // A dungeon walk landed: wake the room's roster (bank_room wrote the
                // survivors back at slide start — kills stay killed within the run).
                if let Some(droom) = run.dungeon.cur().room(run.drx, run.dry) {
                    super::dungeon::spawn_room_foes(&mut commands, droom, run.rift);
                    super::dungeon::spawn_room_dprops(&mut commands, &mut images, droom, run.dungeon.theme, &mut room_blockers);
                    super::dungeon::spawn_room_chests(&mut commands, &mut images, droom);
                    super::dungeon::spawn_room_secret(&mut commands, &mut images, droom, &mut room_blockers);
                    super::dungeon::spawn_room_boss(&mut commands, &mut images, &mut room_blockers, run.rift, run.mini, inv.has_item("kingsplitter"), droom, run.dungeon.theme.key, run.biome.as_deref(), run.is_final, &modes.relics);
                }
            } else {
                visited.0.insert((cur.rx, cur.ry));
                // The new room wakes — a same-day snapshot re-seats exactly what was left.
                super::room_cache::spawn_or_restore(
                    &mut commands,
                    &mut images,
                    &mut uses.rng,
                    &mut uses.human_art,
                    &room_cache,
                    &world.0,
                    &modes.cleared,
                    &mut armed,
                    &world.0.room_entities(cur.rx, cur.ry),
                    (cur.rx, cur.ry),
                    super::gather::farm_day(clock.0),
                );
                banners.room_entered(&world.0, &mut town_names, cur.rx, cur.ry); // announce towns/regions
            }
        }
        return;
    }

    // --- Timers ---
    for cd in &mut p.cooldowns {
        if *cd > 0 {
            *cd -= 1;
        }
    }
    if p.lock_timer > 0 {
        p.lock_timer -= 1;
    }

    // Death is death.rs's: check_death sees hp <= 0 this same tick and takes the screen.

    // Rooted mid-cast (js frozen: !!fishing): fishing.rs owns the inputs; the world —
    // and whatever is hunting you — keeps running. Mid-song the same — the move keys
    // are NOTES (flute.rs owns them), and the foes keep coming.
    if modes.fishing.0.is_some()
        || modes.fluting.0.is_some()
        || modes.pit.0.is_some()
        || modes.donate.0.is_some()
        || modes.side.0.is_some()
        || modes.cutscene.0.is_some()
        || descending.0.is_some()
    {
        p.moving = false; // tumbling into a pit / walking the stairs locks control (js pitFalling / descending)
        return;
    }

    let mut l = state.held(Action::Left);
    let mut r = state.held(Action::Right);
    let mut u = state.held(Action::Up);
    let mut d = state.held(Action::Down);
    if modes.hexed.0 > 0 {
        // The Unmaker's hex: the world answers backwards.
        modes.hexed.0 -= 1;
        (l, r, u, d) = (r, l, d, u);
    }

    // Facing follows the most recently pressed direction (runs mid-swing too — turning while
    // holding attack aims the next swing).
    if state.pressed(Action::Right) {
        p.facing = Facing::Right;
    } else if state.pressed(Action::Left) {
        p.facing = Facing::Left;
    } else if state.pressed(Action::Up) {
        p.facing = Facing::Up;
    } else if state.pressed(Action::Down) {
        p.facing = Facing::Down;
    } else {
        let facing_held = match p.facing {
            Facing::Right => r,
            Facing::Left => l,
            Facing::Up => u,
            Facing::Down => d,
        };
        if !facing_held {
            if r { p.facing = Facing::Right } else if l { p.facing = Facing::Left }
            else if u { p.facing = Facing::Up } else if d { p.facing = Facing::Down }
        }
    }

    // --- Ability slots: each face button triggers the item INSTANCE equipped in that slot
    // (js useSlot). Weapons AUTO-REPEAT while held — gated by their own slot cooldown and
    // the swing lock, and at most ONE weapon fires per tick (holding all four buttons
    // doesn't unleash four attacks). Consumables fire on the press edge, ungated by the
    // swing lock.
    // The guard first (js): HOLD a slotted shield's button to raise it. Raised = half
    // speed + no swings; the deflection itself lives in app/shield.rs.
    p.blocking = false;
    p.block_uid = None;
    for action in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
        let i = action as usize - Action::Slot1 as usize;
        if let Some(uid) = inv.slots[i]
            && inv.def_of(uid).is_some_and(|d| d.kind == "SHIELD")
            && state.held(action)
        {
            p.blocking = true;
            p.block_uid = Some(uid);
            break;
        }
    }
    let mut weapon_fired = false;
    for (i, action) in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4].into_iter().enumerate() {
        let Some(uid) = inv.slots[i] else { continue };
        let Some(def) = inv.def_of(uid) else { continue };
        if def.weapon {
            // A raised shield holds every swing (js: `!p.blocking` gates the attack block).
            if p.blocking || weapon_fired || p.lock_timer > 0 || p.cooldowns[i] > 0 || !state.held_unlatched(action) {
                continue;
            }
            if def.id == "bow" && state.pressed(action) && p.cooldowns[i] == 0 {
                // js use(): the quiver pays first; a dry bag is just the click (no
                // cooldown, no lock — the js returns false and charges nothing).
                if inv.has_item("arrow") {
                    inv.remove_one("arrow");
                    uses.arrows.write(super::archery::FireArrow { dry: false });
                    p.lock_timer = p.lock_timer.max(def.lock_frames);
                    p.cooldowns[i] = def.cooldown;
                    weapon_fired = true;
                } else {
                    uses.arrows.write(super::archery::FireArrow { dry: true });
                }
                continue;
            }
            if def.id == "grapplehook" && state.pressed(action) && p.cooldowns[i] == 0 {
                // js use(): no new hook while a reel or leap is in flight.
                if p.grapple.is_none() && p.hop.is_none() {
                    let (dx, dy) = traversal_aim(&state, p.facing);
                    uses.hooks.write(super::traversal::FireHook { dx, dy, sx: p.x + 8.0, sy: p.y + 9.0 });
                    uses.sfx.write(super::sfx::Sfx("swing"));
                    p.cooldowns[i] = def.cooldown;
                    weapon_fired = true;
                }
                continue;
            }
            if def.id == "springboots" && state.pressed(action) && p.cooldowns[i] == 0 {
                // A forward bound over a tile (js): only if it LANDS somewhere clear.
                if p.grapple.is_none() && p.hop.is_none() {
                    let (fdx, fdy) = match p.facing {
                        crate::actors::hero::Facing::Up => (0.0, -1.0),
                        crate::actors::hero::Facing::Down => (0.0, 1.0),
                        crate::actors::hero::Facing::Left => (-1.0, 0.0),
                        crate::actors::hero::Facing::Right => (1.0, 0.0),
                    };
                    let (tx, ty) = (p.x + fdx * 30.0, p.y + fdy * 30.0);
                    if grid.0.box_hits_solid(tx + 2.0, ty + 8.0, 12.0, 8.0) {
                        uses.sfx.write(super::sfx::Sfx("tink")); // would land in a wall — scuffs
                    } else {
                        p.hop = Some(Hop { sx: p.x, sy: p.y, tx, ty, t: 0, total: 13 });
                        uses.sfx.write(super::sfx::Sfx("swing"));
                        weapon_fired = true;
                    }
                    p.cooldowns[i] = def.cooldown;
                }
                continue;
            }
            if def.id == "wand" {
                // Casts auto-repeat while held (weapon: true); mana pays inside wands.rs.
                uses.wands.write(super::wands::WandMsg::Cast);
                p.cooldowns[i] = def.cooldown;
                weapon_fired = true;
                continue;
            }
            if def.id == "boomerang" && state.pressed(action) && p.cooldowns[i] == 0 {
                // js use(): lock 6 + the out-and-back throw (uniques.rs flies it).
                uses.boomerangs.write(super::uniques::ThrowBoomerang);
                p.lock_timer = p.lock_timer.max(6);
                p.cooldowns[i] = def.cooldown;
                continue;
            }
            let Some(tool) = def.tool else { continue };
            let spec = swing_spec(tool);
            // The tree's melee bonus scales every swing (js: weapon dmg x (1 + stat.melee)).
            // The Kingsplitter swings heavier than the spec (js KINGSPLITTER_SWING dmg 4).
            // A GENERATED weapon (procgen.rs) carries its own combat numbers in `stats`;
            // a fixed weapon uses the tool spec (Kingsplitter swings heavier).
            let is_gen = def.id.starts_with('~');
            let base_dmg = if is_gen {
                crate::items::def_stat(def, "dmg")
            } else if def.id == "kingsplitter" {
                4.0
            } else {
                spec.damage as f64
            };
            let dmg = ((base_dmg * (1.0 + tstats.melee + modes.statuses.sum(|m| m.melee))) + 0.5).floor().max(1.0) as i32;
            // A tiered pick/axe swings its metal-recoloured head at its own toolTier (the harvest
            // gate); only tiered tools are in the map, so a plain pick/axe/sword gets None.
            let tier_img = attack_art.tiered.get(def.id).cloned();
            let swing = commands.spawn((swing_bundle(p.facing as usize, tool, dmg, def.tool_tier, &attack_art, tier_img), RoomActor, PIXEL_LAYER)).id();
            // The strike may land critically (js: st('crit') + cfg.crit, x2 + critmult) — the
            // fixed weapon's intrinsic is the tool spec; a generated one's is its own roll.
            let wcrit = if is_gen { crate::items::def_stat(def, "crit") } else { spec.crit };
            let wcritmult = if is_gen { crate::items::def_stat(def, "critmult") } else { 0.0 };
            commands.entity(swing).insert(crate::combat::CritChance {
                chance: wcrit + tstats.crit + modes.statuses.sum(|m| m.crit),
                mult: 2.0 + tstats.critmult + wcritmult,
            });
            // Bonus knockback (generated "knock" affixes) + lifesteal on the strike
            // (weapon leech + worn gear leech — js atk.leech).
            let wknock = if is_gen { crate::items::def_stat(def, "knock") } else { 0.0 };
            let leech = tstats.leech + if is_gen { crate::items::def_stat(def, "leech") } else { 0.0 };
            if wknock > 0.0 || leech > 0.0 {
                commands.entity(swing).insert(super::uniques::SwingBonus { knock: wknock as f32, leech });
            }
            if wknock > 0.0 {
                // The extra shove rides the swing's Combatant.knock (resolve_combat's kb term).
                commands.entity(swing).entry::<crate::combat::Combatant>().and_modify(move |mut c| {
                    c.knock += wknock as f32;
                });
            }
            // Ember Fang / Winter Shard: the swing may carry a proc (luck-scaled roll).
            super::uniques::roll_swing_procs(&mut commands, swing, &inv, tstats.luck, &mut || uses.rng.0.next_f64());
            if def.id == "kingsplitter" {
                commands.entity(swing).insert(super::boss::wriftheart::Wriftbane);
                if health.hp >= health.max {
                    uses.beams.write(super::saltmaze::FireBeam); // hale: the blade sings
                }
            }
            p.cooldowns[i] = def.cooldown;
            p.lock_timer = def.lock_frames;
            weapon_fired = true;
        } else if def.consumable && state.pressed(action) && p.cooldowns[i] == 0 {
            if matches!(def.id, "chicken" | "cow" | "coop" | "barn") {
                // Farm items validate + consume in their own handler (js use() veto).
                uses.farm.write(super::farm_animals::UseFarmItem(def.id));
                p.cooldowns[i] = def.cooldown;
                continue;
            }
            if def.dish {
                // A cooked meal never vetoes: eat it, wear its buff (app/status.rs).
                uses.eats.write(super::status::EatDish(def.id));
                inv.remove_entry(uid);
                p.cooldowns[i] = def.cooldown;
                continue;
            }
            if def.kind == "RUNE" {
                uses.wands.write(super::wands::WandMsg::Socket(def.id));
                p.cooldowns[i] = def.cooldown;
                continue; // wands.rs consumes on a real socket (a matching rune vetoes)
            }
            if matches!(def.id, "manapotion" | "manaelixir") {
                let amt = if def.id == "manapotion" { 8 } else { i32::MAX };
                uses.wands.write(super::wands::WandMsg::Potion { id: def.id, amt });
                p.cooldowns[i] = def.cooldown;
                continue; // consumed there only if it actually restored
            }
            if matches!(def.id, "treasuremap" | "mapbottle") {
                // Reading validates + consumes in its own handler (js use() veto).
                uses.maps.write(super::digging::ReadMap(def.id));
                p.cooldowns[i] = def.cooldown;
                continue;
            }
            if def.id == "bombs" {
                // js use(): drop at the feet, back away — always consumed.
                uses.bombs.write(super::caves::DropBomb(p.x.round(), p.y.round()));
                inv.remove_entry(uid);
                p.cooldowns[i] = def.cooldown;
                continue;
            }
            if def.kind == "STATION" {
                // Any placeable station (cook/workbench/forge/…) validates + consumes in
                // the cooking handler (js placing) — set it at your feet.
                uses.stations.write(super::cooking::PlaceStation(def.id));
                p.cooldowns[i] = def.cooldown;
                continue;
            }
            if def.kind == "STRUCTURE" {
                // The buildable home validates + consumes in the home handler (js placeHouse).
                uses.houses.write(super::home::PlaceHouse);
                p.cooldowns[i] = def.cooldown;
                continue;
            }
            if def.id == "sleepingbag" {
                // The sleep handler validates (open world, no foes) + consumes (js use() veto).
                uses.sleep.write(super::services::SleepRequest);
                p.cooldowns[i] = def.cooldown;
                continue;
            }
            if def.kind == "BLUEPRINT" {
                // A blueprint teaches its recipe(s) once, then is spent — the handler
                // validates + consumes (js use() veto: an already-known one isn't wasted).
                uses.blueprints.write(super::blueprints::LearnBlueprint(def.id));
                p.cooldowns[i] = def.cooldown;
                continue;
            }
            if def.id == "antidote" {
                // Cures poison + slow; consumed only if you were actually sick (js veto).
                if modes.statuses.has("poison") || modes.statuses.has("slow") {
                    uses.cures.write(super::status::CureStatus);
                    inv.remove_entry(uid);
                } else {
                    uses.sfx_tink(); // nothing to cure
                }
                p.cooldowns[i] = def.cooldown;
                continue;
            }
            if let Some(target) = satchel_target(def.id) {
                // STRICT tiers (js): only a bag exactly one row short grows — you must
                // use the lower packs first (can't skip to a bigger one).
                if inv.bag_rows == target - 1 && inv.expand_bag() {
                    inv.remove_entry(uid);
                } else {
                    uses.sfx_tink();
                }
                p.cooldowns[i] = def.cooldown;
                continue;
            }
            // js useSlot: use() may veto (potion at full HP) — then nothing is consumed,
            // but the cooldown starts either way.
            if crate::items::use_consumable(def, &mut health) {
                inv.remove_entry(uid);
            }
            p.cooldowns[i] = def.cooldown;
        }
    }

    let move_axis = |p: &mut Player, mx: f32, my: f32, grid: &RoomGrid, blk: &RoomBlockers| {
        let nx = p.x + mx;
        let ny = p.y + my;
        let feet = |x: f32, y: f32| (x + BOX_OX, y + BOX_OY, BOX_W, BOX_H);
        if grid.box_hits_solid(nx + BOX_OX, ny + BOX_OY, BOX_W, BOX_H)
            || blk.blocks(feet(p.x, p.y), feet(nx, ny))
        {
            return false;
        }
        p.x = nx;
        p.y = ny;
        true
    };

    if kb.timer > 0 {
        // Knocked: the hit's shove owns movement this tick (js p.knockTimer branch).
        kb.timer -= 1;
        let (kx, ky) = (kb.kx, kb.ky);
        move_axis(&mut p, kx, 0.0, &grid.0, &room_blockers);
        move_axis(&mut p, 0.0, ky, &grid.0, &room_blockers);
        p.moving = false;
        modes.pulled.0 = None; // a hit snaps any tongue's grip (js onHurt clears p.pulled)
        p.grapple = None;
        p.hop = None;
    } else if let Some(mut g) = p.grapple {
        // Reeled in by a lodged grapple hook (js p.grapple): dragged FAST toward the
        // anchor (sp 5, near 4) until he arrives, the timer dies, or he wedges.
        let (dx, dy) = (g.tx - p.x, g.ty - p.y);
        let dist = (dx * dx + dy * dy).sqrt();
        g.t -= 1;
        let done = if dist < 4.0 || g.t <= 0 {
            true
        } else {
            let sp = 5.0_f32.min(dist);
            let mx = move_axis(&mut p, dx / dist * sp, 0.0, &grid.0, &room_blockers);
            let my = move_axis(&mut p, 0.0, dy / dist * sp, &grid.0, &room_blockers);
            !mx && !my // wedged against a wall — done
        };
        p.grapple = if done { None } else { Some(g) };
        p.moving = false;
    } else if let Some(mut h) = p.hop {
        // Spring-boots leap (js p.hop): a lerp arc clearing whatever's between; hop_z
        // gives the draw its jump height.
        h.t += 1;
        let t = (h.t as f32 / h.total as f32).min(1.0);
        p.x = h.sx + (h.tx - h.sx) * t;
        p.y = h.sy + (h.ty - h.sy) * t;
        p.hop_z = (t * std::f32::consts::PI).sin() * 10.0;
        p.moving = true;
        p.anim_timer += 1;
        if p.anim_timer >= ANIM_TICKS {
            p.anim_timer = 0;
            p.anim_frame = (p.anim_frame + 1) & 3;
        }
        if t >= 1.0 {
            p.hop = None;
            p.hop_z = 0.0;
        } else {
            p.hop = Some(h);
        }
    } else if modes.pulled.0.is_some() {
        // Reeled in by a tongue (js p.pulled): dragged at 3.2 toward the maw until he
        // arrives (near 10), the timer dies, or he wedges against a wall. Only outside
        // a dungeon it can't happen — a room change mid-reel clears with the mode.
        let done = modes.dungeon.0.is_none() || {
            let g = modes.pulled.0.as_mut().unwrap();
            let (dx, dy) = (g.tx - p.x, g.ty - p.y);
            let dist = (dx * dx + dy * dy).sqrt();
            g.t -= 1;
            if dist < 10.0 || g.t <= 0 {
                true
            } else {
                let sp = 3.2_f32.min(dist);
                let mx = move_axis(&mut p, dx / dist * sp, 0.0, &grid.0, &room_blockers);
                let my = move_axis(&mut p, 0.0, dy / dist * sp, &grid.0, &room_blockers);
                !mx && !my // wedged — the tongue loses its grip (js)
            }
        };
        if done {
            modes.pulled.0 = None;
        }
        p.moving = false;
    } else {
        let mut dx = (r as i32 - l as i32) as f32;
        let mut dy = (d as i32 - u as i32) as f32;
        if dx != 0.0 && dy != 0.0 {
            dx *= std::f32::consts::FRAC_1_SQRT_2;
            dy *= std::f32::consts::FRAC_1_SQRT_2;
        }
        // Mid-swing slow (the walk-and-attack rule): 55% while the swing plays out.
        // Statuses ride the same line (js): SLOWED halves you, SHOCKED nearly roots
        // you, move buffs quicken you — floored so nothing can freeze you solid.
        let mv = tstats.move_bonus + modes.statuses.sum(|m| m.mv);
        let mut spd = SPEED * (1.0 + mv as f32) * if p.lock_timer > 0 { 0.55 } else { 1.0 };
        if p.blocking {
            spd *= 0.5; // guard up, feet slow (js)
        }
        if modes.statuses.has("slow") {
            spd *= 0.5;
        }
        if modes.statuses.has("shock") {
            spd *= 0.3;
        }
        spd = spd.max(0.4);
        let (tvx, tvy) = (dx * spd, dy * spd); // the velocity the input asks for
        // Slippery ICE (js footG=='ice' && !paved): build toward the input slowly (low grip)
        // and coast to a stop when released (low friction) — momentum instead of stop-on-a-dime.
        // A road/processional/street/bridge deck laid over ice gives sure footing.
        let on_ice = modes.inside.0.is_none()
            && modes.dungeon.0.is_none()
            && {
                let col = ((p.x + 8.0) / 16.0).floor() as i32;
                let row = ((p.y + 13.0) / 16.0).floor() as i32;
                world.0.ground_name(cur.rx * COLS + col, cur.ry * ROWS + row) == "ice"
                    && !matches!(grid.0.code_at(col, row), '=' | 'p' | '_' | 'B')
            };
        if on_ice {
            let k = if dx != 0.0 || dy != 0.0 { 0.09 } else { 0.025 };
            p.vx += (tvx - p.vx) * k;
            p.vy += (tvy - p.vy) * k;
            if p.vx.abs() < 0.04 && dx == 0.0 {
                p.vx = 0.0;
            }
            if p.vy.abs() < 0.04 && dy == 0.0 {
                p.vy = 0.0;
            }
        } else {
            p.vx = tvx; // grippy ground: instant
            p.vy = tvy;
        }
        let (vx, vy) = (p.vx, p.vy);
        if vx != 0.0 && !move_axis(&mut p, vx, 0.0, &grid.0, &room_blockers) {
            p.vx = 0.0; // a wall kills the slide on that axis
        }
        if vy != 0.0 && !move_axis(&mut p, 0.0, vy, &grid.0, &room_blockers) {
            p.vy = 0.0;
        }
        p.moving = p.vx != 0.0 || p.vy != 0.0;
    }

    if p.moving {
        p.anim_timer += 1;
        if p.anim_timer >= ANIM_TICKS {
            p.anim_timer = 0;
            p.anim_frame = (p.anim_frame + 1) & 3;
        }
    } else {
        p.anim_frame = 0;
        p.anim_timer = 0;
    }
    // The full-body combat hitbox rides the position (js: p.hitbox = (x+3, y+2, 10, 13)).
    *hitbox = Hitbox { x: p.x + 3.0, y: p.y + 2.0, w: 10.0, h: 13.0 };

    // --- Edge crossing -> start a slide (JS check order: right, left, down, up).
    // No slides indoors or underground: the interior owns its exits (the doorway mat),
    // the dungeon owns its own room walk (app/dungeon.rs navigate). ---
    if modes.inside.0.is_some() || modes.dungeon.0.is_some() {
        return;
    }
    let cx = p.x + 8.0;
    let cy = p.y + 8.0;
    let dir: Option<(i32, i32, Facing)> = if r && cx >= (PX_W as f32 - EDGE_REACH) {
        Some((1, 0, Facing::Right))
    } else if l && cx <= EDGE_REACH {
        Some((-1, 0, Facing::Left))
    } else if d && cy >= (PX_H as f32 - EDGE_REACH) {
        Some((0, 1, Facing::Down))
    } else if u && cy <= EDGE_REACH {
        Some((0, -1, Facing::Up))
    } else {
        None
    };
    let Some((ddx, ddy, face)) = dir else { return };

    let nrx = cur.rx + ddx;
    let nry = cur.ry + ddy;
    let new_grid = RoomGrid::from_map(&world.0.generate(nrx, nry));
    let (mut ex, mut ey) = (p.x, p.y);
    if ddx == 1 {
        ex = 2.0;
    } else if ddx == -1 {
        ex = (PX_W - 16 - 2) as f32;
    } else if ddy == 1 {
        ey = 2.0;
    } else {
        ey = (PX_H - 16 - 2) as f32;
    }
    let end = safe_entry(&new_grid, ddy != 0, ex, ey);
    let delta = Vec2::new((ddx * PX_W) as f32, (-ddy * PX_H) as f32);
    let new_ents = world.0.room_entities(nrx, nry);
    let (new_root, blockers) = spawn_room_root(
        &mut commands, &mut images, &tex, &mut prop_art, &world.0, &new_grid, &new_ents, &gather, &mut growth,
        &modes.farm, &modes.cleared, &modes.caves, &modes.songs_opened, nrx, nry, delta, clock.0,
    );
    room_blockers.0 = blockers; // movement is frozen mid-slide; ready when we land
    despawn_room_actors(&mut commands, &actors); // the old room's cast leaves with it
    slide.0 = Some(Slide {
        frame: 0,
        total: if ddx != 0 { (PX_W / 8) as u32 } else { (PX_H / 8) as u32 },
        start: Vec2::new(p.x, p.y),
        end,
        delta,
        old_root: active.0,
        new_root,
    });
    sliding.0 = true;
    p.facing = face;
    grid.0 = new_grid;
    cur.rx = nrx;
    cur.ry = nry;
}

/// Worn armor changed -> re-bake the hero's sprite bank in the new gear (js
/// refreshSprite on equipGear). The anim tick reads HeroArt every frame, so the
/// swap re-skins instantly.
pub fn worn_refresh(
    mut images: ResMut<Assets<Image>>,
    inv: Res<crate::inventory::PlayerInv>,
    ident: Res<super::identity::HeroIdent>,
    mut hero_art: ResMut<HeroArt>,
    mut last: Local<Option<[Option<&'static str>; 3]>>,
) {
    if !inv.is_changed() && last.is_some() {
        return;
    }
    let look_of = |id: &'static str| crate::actors::hero::armor_look(id).or_else(|| crate::procgen::armor_look(id));
    let worn: [Option<&'static str>; 3] =
        [0, 1, 2].map(|g| inv.gear[g].and_then(|uid| inv.id_of(uid)).filter(|id| look_of(id).is_some()));
    if *last == Some(worn) {
        return;
    }
    *last = Some(worn);
    let arm: crate::actors::hero::WornArm = worn.map(|id| id.and_then(look_of));
    *hero_art = HeroArt(crate::actors::hero::build_frames_geared(&ident.look, &arm, &mut images));
}

/// Re-bake the sidebar coords label when the room changes.
fn relabel_coords(
    mut commands: Commands,
    cur: Res<CurRoom>,
    old: Query<Entity, With<CoordsLabel>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !cur.is_changed() {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let text = format!("{},{}", cur.rx, cur.ry);
    label(&mut commands, &mut images, &text, 8.0, CANVAS_H as f32 - 22.0, 0xa0a0a0, 18.2, CoordsLabel);
}

/// Push the Player's room-pixel position + gait frame into its sprite each render frame.
/// The i-frame blink hides the body on alternating 4-frame windows (js hurtFlash >> 2).
fn sync_player_sprite(
    art: Res<HeroArt>,
    mut q: Query<(&Player, &Health, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let Ok((p, h, mut sprite, mut tf, mut vis)) = q.single_mut() else { return };
    sprite.image = art.0.frames[p.facing as usize][p.anim_frame].clone();
    let bob = if p.moving && (p.anim_frame & 1) == 1 { 1.0 } else { 0.0 };
    // Spring-boots leap lifts the sprite (js hopZ) — a shadow-anchored bounce.
    *tf = at(PLAY_X + p.x.round(), PLAY_Y + p.y.round() - bob - p.hop_z.round(), 16.0, 16.0, actor_z(p.y.round() + 16.0));
    // DEAD hides the body outright (the death scene's corpse sprite stands in) — keyed
    // off HP, not the screen state: the state transition lags a frame and this sync
    // would win that race and re-show the standing hero beside his own corpse.
    *vis = if h.hp <= 0 || (h.flash > 0 && ((h.flash >> 2) % 2) == 0) {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
}

/// The tree's Max HP total lands on the player whenever allocations change (no free heal:
/// current HP only clamps down, like the JS refresh).
fn apply_tree_hp(tstats: Res<TreeStats>, mut q: Query<&mut Health, With<Player>>) {
    if !tstats.is_changed() {
        return;
    }
    let Ok(mut h) = q.single_mut() else { return };
    // Max HP never folds below HP_BASE (Baz, 2026-07-16): a bad trait can't start you
    // on 1 heart — its penalty only bites once tree Vitality lifts you above the floor.
    h.max = ((HP_BASE as f64 + tstats.maxhp).round() as i32).max(HP_BASE);
    h.hp = h.hp.min(h.max);
}
