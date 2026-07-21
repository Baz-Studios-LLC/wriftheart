//! lootgoblin.rs — THE LOOT GOBLIN (js enemies.js lootgoblin): a goblin made of money,
//! super-rare, harmless, and TERRIFIED. It bolts from you, sheds a spray of coins every
//! time you land a hit (rarely a trinket), and if you corner it before it slips away the
//! kill is a jackpot. Its flee is evasive: it juks around the OPEN MIDDLE of the room for
//! a ~7s grace (steering off whatever edge it nears, so it circles you instead of
//! escaping straight off), and only after the grace does it bolt for an exit. A gold-
//! recoloured goblin (js GOLDPAL).
//! CROSS-ROOM (js lootGob): it doesn't die at the edge — it RELOCATES to the neighbour room
//! ON THE RUN with a deadline. Follow it there (fresh chase window) before the deadline or
//! it's gone for good. The whole roam lives in the saved [`LootGob`] (not the room roster),
//! managed by [`lootgob_load`]; a slain/escaped origin banks into [`LootGobCleared`] so it
//! never re-rolls there. See save.rs (loot_gob / loot_gob_cleared).

use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::battle::{spawn_burst, GameRng, RoomActor};
use super::play::{CurRoom, GameWorld, Player};
use super::room_render::{actor_z, FrameClock, PLAY_X, PLAY_Y};
use crate::actors::goblin_art::GOBLIN_FRAMES;
use crate::combat::{Blood, Combatant, Health, HitLanded, HurtProfile, Hitbox, Knockback, Team};
use crate::gfx::{at, bake, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

/// The gold goblin (js GOLDPAL remaps its skin q/Q to gold).
const GOLD_PAL: &[(char, u32)] = &[('q', 0xfcd000), ('Q', 0xc87838)];

/// The js LG grids: the loot goblin's OWN back/side frames wearing the stuffed
/// BACKPACK (leather D/d, gold buckle P). The pack reads differently per facing —
/// whole on the back (up), a bulge behind the shoulder (side) — and the front is
/// the plain gold body: the pack hides behind him (js: straps read badly small).
#[rustfmt::skip]
const LG_UP: [&str; 16] = [
    "................", "...K......K.....", "...Kq....qK.....", "...KqDDDDqK.....",
    "...KqDDDDqK.....", "...KqDPPDqK.....", "...KqDDDDqK.....", "..KKqDDDDqKK....",
    "..KqqDDDDqqK....", "..KqqDDDDqqK....", "...KddddddK.....", "...KDddddDK.....",
    "...Kqq..qqK.....", "...Kq....qK.....", "...Kd....dK.....", "................",
];
#[rustfmt::skip]
const LG_SIDE_A: [&str; 16] = [
    "................", "..K.............", "..KqqqqqK.......", ".DKqqqqqqK......",
    "dDKqqqqrqK......", "dDKqqqqqqK......", "dPKqqqqqK.......", "dDKqqqqqqK......",
    "dDqqqqqqqqK.....", "dDqqqqqqqqK.....", "..KddddddK......", "..KDddddDK......",
    "..Kqq..qqK......", "..Kq....qK......", "..Kd....dK......", "................",
];
#[rustfmt::skip]
const LG_SIDE_B: [&str; 16] = [
    "................", "..K.............", "..KqqqqqK.......", ".DKqqqqqqK......",
    "dDKqqqqrqK......", "dDKqqqqqqK......", "dPKqqqqqK.......", "dDKqqqqqqK......",
    "dDqqqqqqqqK.....", "dDqqqqqqqqK.....", "..KddddddK......", "..KDddddDK......",
    "..Kqq..qqK......", "..Kq....qK......", "..K.d..d.K......", "................",
];

/// The four facings x two frames, gold-baked once at startup: the plain front
/// (js GOLD_FRAMES.down), the packed back + sides, left = the side flipped.
#[derive(Resource)]
pub struct LootGoblinArt(pub [[Handle<Image>; 2]; 4]);

fn gold(images: &mut Assets<Image>, grid: &[&str]) -> Handle<Image> {
    images.add(bake(grid, GOLD_PAL))
}

fn gold_flipped(images: &mut Assets<Image>, grid: &[&str]) -> Handle<Image> {
    let v = crate::gfx::flip_h(grid);
    gold(images, &v.iter().map(|s| s.as_str()).collect::<Vec<_>>())
}

impl LootGoblinArt {
    fn build(images: &mut Assets<Image>) -> Self {
        let (_, down) = GOBLIN_FRAMES.iter().find(|(f, _)| *f == "down").unwrap();
        LootGoblinArt([
            [gold(images, &down[0]), gold(images, &down[1])],
            [gold(images, &LG_UP), gold(images, &LG_UP)], // up has no leg-swap frame (js)
            [gold(images, &LG_SIDE_A), gold(images, &LG_SIDE_B)],
            [gold_flipped(images, &LG_SIDE_A), gold_flipped(images, &LG_SIDE_B)],
        ])
    }
}

/// The loot goblin's CROSS-ROOM state (js lootGob), saved: it lives here, not in the room
/// roster, so an escaped goblin persists and RELOCATES to the next room to be chased.
#[derive(Clone, Serialize, Deserialize)]
pub struct LootGobRec {
    pub room: (i32, i32),
    pub x: f32,
    pub y: f32,
    pub hp: i32,
    /// Some(frame) once it's ON THE RUN in a room you're NOT in — reach it before this or
    /// it's gone. Cleared the moment you arrive in its room (you're chasing now).
    pub deadline: Option<i64>,
    /// The room it was first rolled in — banked to `cleared` on death/escape so it never
    /// re-rolls there.
    pub origin: (i32, i32),
}

/// The one live loot goblin across the whole world (js lootGob, saved).
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct LootGob(pub Option<LootGobRec>);

/// Origin rooms whose goblin was slain or escaped — never re-rolls (js lootGobCleared, saved).
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct LootGobCleared(pub HashSet<(i32, i32)>);

const LOOT_RUN: i64 = 1000; // frames on-the-run before it's gone for good (~17s at 60fps)

/// The fleeing money-goblin (js lootgoblin state).
#[derive(Component)]
pub struct LootGoblin {
    x: f32,
    y: f32,
    facing: usize,
    anim: u32,
    spooked: bool,
    spook_t: i32,
    last_hp: i32,
    /// It slipped out an exit — despawn WITHOUT the jackpot (js banished).
    banished: bool,
}

/// The walking hoard's golden glint (js collectLights 'lootgoblin': glow [255,210,70],
/// glowR 17, gi pulsing on sin(frame/10)) — a child sprite, so it rides him for free.
#[derive(Component)]
pub struct LootGlow;

/// Spawn the goblin at `hp` (a re-entered/relocated goblin keeps its wounds; 10 = fresh).
pub fn spawn_lootgoblin(commands: &mut Commands, images: &mut Assets<Image>, x: f32, y: f32, hp: i32) -> Entity {
    let glow = crate::gfx::radial_glow_tex(images, 34);
    commands
        .spawn((
            Sprite::default(), // art applied per tick from the bank
            at(PLAY_X + x, PLAY_Y + y, 16.0, 16.0, actor_z(y + 16.0)),
            PIXEL_LAYER,
            RoomActor,
            LootGoblin { x, y, facing: 0, anim: 0, spooked: false, spook_t: 0, last_hp: hp, banished: false },
            // Harmless (js damage: null) — it never hurts you, it only runs.
            Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: None, persistent: true, knock: 0.0 },
            Health { hp, max: 10, defense: 0, invuln: 0, flash: 0 },
            HurtProfile { invuln: 8, flash: 8, kb_base: 2.6, kb_frames: 12 }, // knockback closes the gap
            Knockback::default(),
            Blood(0xfcd000),
            Hitbox { x: x + 3.0, y: y + 4.0, w: 10.0, h: 11.0 },
        ))
        .with_children(|p| {
            p.spawn((
                LootGlow,
                Sprite { image: glow, color: Color::srgba(1.0, 0.82, 0.27, 0.28), custom_size: Some(Vec2::splat(34.0)), ..default() },
                Transform::from_xyz(0.0, 0.0, -0.005), // just under the body, over the ground
                PIXEL_LAYER,
            ));
        })
        .id()
}

/// The flee AI + the shed-coins-on-hit spray + the per-tick art/hitbox.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn lootgoblin_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    art: Res<LootGoblinArt>,
    mut hits: MessageReader<HitLanded>,
    mut lootgob: ResMut<LootGob>,
    cur: Res<CurRoom>,
    clock: Res<FrameClock>,
    grid: Res<super::play::CurGrid>,
    players: Query<&Player>,
    mut gobs: Query<(Entity, &mut LootGoblin, &mut Transform, &mut Hitbox, &Health, &mut Sprite)>,
    mut glows: Query<&mut Sprite, (With<LootGlow>, Without<LootGoblin>)>,
) {
    let _ = hits.read().count(); // drain the reader; coins key on the hp-drop below
    // The hoard glints — js gi = 0.28 + 0.07 sin(frame/10), the pulsing gold halo.
    for mut g in &mut glows {
        g.color = g.color.with_alpha(0.28 + 0.07 * (clock.0 as f32 / 10.0).sin());
    }
    let Ok(p) = players.single() else { return };
    let (w, h) = (PX_W as f32, PX_H as f32);
    for (e, mut g, mut tf, mut hb, health, mut spr) in &mut gobs {
        // Struck: bleed a spray of coins (mostly money, rarely a trinket) — js e.lastHp.
        if health.hp < g.last_hp {
            g.last_hp = health.hp;
            let n = 3 + (rng.0.next_f64() * 3.0) as i32;
            for _ in 0..n {
                let cv = 2 + (rng.0.next_f64() * 4.0) as i32;
                let (cx, cy) = (g.x + 4.0 + rng.0.next_f64() as f32 * 10.0 - 5.0, g.y + 8.0 + rng.0.next_f64() as f32 * 8.0 - 4.0);
                super::gather::spawn_coin(&mut commands, &mut images, cv, cx, cy);
            }
            if rng.0.next_f64() < 0.12 {
                let (id, qty) = crate::items::roll_loot(0.8, 0.0, || rng.0.next_f64());
                super::gather::spawn_pickup(&mut commands, &mut images, id, qty, g.x + 4.0, g.y + 6.0, true, None);
            }
        }
        // The flee: react once you're within 150 (or already spooked).
        let (dx, dy) = (p.x - g.x, p.y - g.y);
        let d = (dx * dx + dy * dy).sqrt().max(0.001);
        if d < 150.0 || g.spooked {
            if !g.spooked {
                g.spooked = true;
                g.spook_t = 0;
            }
            g.spook_t += 1;
            let grace = g.spook_t < 420; // ~7s juking before it bolts for an exit
            let (mut vx, mut vy) = (-dx / d, -dy / d); // away from the player (fleeVec)
            if grace {
                // Steer off whatever edge it nears so it circles the OPEN MIDDLE.
                const MRG: f32 = 30.0;
                if g.x < MRG {
                    vx += 0.9;
                } else if g.x > w - 16.0 - MRG {
                    vx -= 0.9;
                }
                if g.y < MRG {
                    vy += 0.9;
                } else if g.y > h - 16.0 - MRG {
                    vy -= 0.9;
                }
                let m = (vx * vx + vy * vy).sqrt().max(0.001);
                vx /= m;
                vy /= m;
            }
            let sp = 1.7;
            let wob = (g.anim as f32 * 0.3).sin() * 0.35;
            // Per-axis collision (the mob_step rule): it JUKES, it doesn't GHOST —
            // no more sprinting across lakes or through boulders (Baz, laughing).
            let try_x = (g.x + vx * sp + wob).clamp(0.0, w - 16.0);
            if !grid.0.box_hits_solid(try_x + 3.0, g.y + 5.0, 10.0, 9.0) {
                g.x = try_x;
            }
            let try_y = (g.y + vy * sp - wob).clamp(0.0, h - 16.0);
            if !grid.0.box_hits_solid(g.x + 3.0, try_y + 5.0, 10.0, 9.0) {
                g.y = try_y;
            }
            g.facing = face_from(vx, vy);
            // Only AFTER the grace does it slip out an edge — it RELOCATES to the neighbour
            // room ON THE RUN (js relocate): follow it before the deadline or it's gone.
            if !grace && (g.x < 3.0 || g.x > w - 19.0 || g.y < 3.0 || g.y > h - 19.0) {
                let (nrx, nry, ex, ey) = if g.x < 3.0 {
                    (cur.rx - 1, cur.ry, w - 20.0, g.y)
                } else if g.x > w - 19.0 {
                    (cur.rx + 1, cur.ry, 4.0, g.y)
                } else if g.y < 3.0 {
                    (cur.rx, cur.ry - 1, g.x, h - 20.0)
                } else {
                    (cur.rx, cur.ry + 1, g.x, 4.0)
                };
                let origin = lootgob.0.as_ref().map_or((cur.rx, cur.ry), |lg| lg.origin);
                lootgob.0 = Some(LootGobRec { room: (nrx, nry), x: ex, y: ey, hp: health.hp, deadline: Some(clock.0 + LOOT_RUN), origin });
                spawn_burst(&mut commands, &mut rng, Vec2::new(g.x + 8.0, g.y + 8.0), 0xffd34d, 8);
                commands.entity(e).despawn();
                continue;
            }
        }
        // Mirror the live goblin into the saved state so leaving the room ANY way preserves
        // its wounds + spot (js keepLootGobMirrored).
        if let Some(lg) = lootgob.0.as_mut()
            && lg.room == (cur.rx, cur.ry)
        {
            lg.x = g.x;
            lg.y = g.y;
            lg.hp = health.hp;
        }
        g.anim += 1;
        // Art + hitbox follow.
        let frame = (g.anim / 6) as usize % 2;
        if health.flash > 0 && (health.flash & 1) == 1 {
            *spr = Sprite::default(); // blink on the hurt frame
        } else {
            *spr = Sprite::from_image(art.0[g.facing][frame].clone());
            spr.custom_size = Some(Vec2::splat(16.0));
        }
        *hb = Hitbox { x: g.x + 3.0, y: g.y + 4.0, w: 10.0, h: 11.0 };
        *tf = at(PLAY_X + g.x.round(), PLAY_Y + g.y.round(), 16.0, 16.0, actor_z(g.y.round() + 16.0));
    }
}

/// The jackpot on a real kill (js drops): a burst of gold + a decent loot roll — unless
/// it fled (banished), in which case it just vanishes with whatever it had.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn lootgoblin_deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<super::rewards::Progress>,
    mut alloc: ResMut<super::slideout::TreeAlloc>,
    tstats: Res<super::slideout::TreeStats>,
    mut stats: ResMut<super::stats::Stats>,
    mut bestiary: ResMut<super::codex::mobs_tab::Bestiary>,
    mut lootgob: ResMut<LootGob>,
    mut cleared: ResMut<LootGobCleared>,
    q: Query<(Entity, &LootGoblin, &Health)>,
) {
    for (e, g, h) in &q {
        if h.hp > 0 {
            continue;
        }
        bestiary.0.insert("lootgoblin");
        // Cornered at last — bank its origin so it never re-rolls, and end the roam.
        if let Some(lg) = lootgob.0.take() {
            cleared.0.insert(lg.origin);
        }
        if !g.banished {
            // The jackpot: 45-99 copper + a scatter of coins + a rich loot roll.
            let jackpot = 45 + (rng.0.next_f64() * 55.0) as i32;
            super::gather::spawn_coin(&mut commands, &mut images, jackpot, g.x + 4.0, g.y + 4.0);
            for i in 0..4 {
                let cv = 3 + (rng.0.next_f64() * 6.0) as i32;
                super::gather::spawn_coin(&mut commands, &mut images, cv, g.x + 2.0 + i as f32 * 3.0, g.y + 6.0);
            }
            if rng.0.next_f64() < 0.5 * (1.0 + tstats.luck) {
                let (id, qty) = crate::items::roll_loot(1.6, tstats.luck, || rng.0.next_f64());
                super::gather::spawn_pickup(&mut commands, &mut images, id, qty, g.x + 4.0, g.y + 6.0, true, None);
            }
            spawn_burst(&mut commands, &mut rng, Vec2::new(g.x + 8.0, g.y + 8.0), 0xfcd000, 14);
            super::rewards::gain_xp(&mut progress, &mut alloc, 20);
            stats.bump("kills", 1.0);
        }
        commands.entity(e).despawn();
    }
}

/// Facing index from a velocity (js faceFrom): down/up/right/left.
fn face_from(vx: f32, vy: f32) -> usize {
    if vx.abs() > vy.abs() {
        if vx > 0.0 {
            2
        } else {
            3
        }
    } else if vy > 0.0 {
        0
    } else {
        1
    }
}

/// Cross-room plumbing (js loot-goblin block): (re)spawn the roaming goblin when you enter
/// its room, mint a fresh one on the super-rare worldgen roll, and let one that's on the run
/// slip away for good once its deadline passes. The loot goblin lives in [`LootGob`], NOT the
/// room roster — so it persists across rooms.
#[allow(clippy::too_many_arguments)]
fn lootgob_load(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    world: Res<GameWorld>,
    cur: Res<CurRoom>,
    clock: Res<FrameClock>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    mut lootgob: ResMut<LootGob>,
    mut cleared: ResMut<LootGobCleared>,
    existing: Query<(), With<LootGoblin>>,
) {
    // On the run in a room you never reached — the deadline lapses, it's gone with the gold.
    if let Some(lg) = &lootgob.0
        && let Some(dl) = lg.deadline
        && lg.room != (cur.rx, cur.ry)
        && clock.0 > dl
    {
        cleared.0.insert(lg.origin);
        lootgob.0 = None;
    }
    // The loot goblin is an OVERWORLD creature; only (re)spawn when none is already up.
    if in_dungeon.0.is_some() || inside.0.is_some() || !existing.is_empty() {
        return;
    }
    // ARRIVE / RE-SPAWN: the roaming goblin is (or fled to) this room.
    if let Some(lg) = lootgob.0.as_mut()
        && lg.room == (cur.rx, cur.ry)
    {
        if lg.deadline.is_some_and(|dl| clock.0 > dl) {
            cleared.0.insert(lg.origin); // you got here, but too late
            lootgob.0 = None;
            return;
        }
        let (x, y, hp) = (lg.x, lg.y, lg.hp);
        lg.deadline = None; // you're here now — it isn't "getting away" while you chase
        spawn_lootgoblin(&mut commands, &mut images, x, y, hp);
        return;
    }
    // FRESH ROLL: the worldgen seeded a super-rare one here, never cleared, none roaming yet.
    if lootgob.0.is_none()
        && !cleared.0.contains(&(cur.rx, cur.ry))
        && let Some(e) = world.0.room_entities(cur.rx, cur.ry).iter().find(|e| e.kind == "mob" && e.sub == "lootgoblin")
    {
        let (x, y) = (e.x as f32, e.y as f32);
        spawn_lootgoblin(&mut commands, &mut images, x, y, 10);
        lootgob.0 = Some(LootGobRec { room: (cur.rx, cur.ry), x, y, hp: 10, deadline: None, origin: (cur.rx, cur.ry) });
    }
}

pub struct LootGoblinPlugin;

impl Plugin for LootGoblinPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LootGob>()
            .init_resource::<LootGobCleared>()
            .add_systems(Startup, |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
                commands.insert_resource(LootGoblinArt::build(&mut images));
            })
            .add_systems(
                bevy::app::FixedUpdate,
                (lootgob_load, lootgoblin_tick, lootgoblin_deaths.after(crate::combat::resolve_combat))
                    .before(super::play::EndTick)
                    .run_if(super::screen::playing),
            );
    }
}
