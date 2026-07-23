//! mobs.rs — the shared biome-mob scaffold (port of js/enemies.js `mob()` + the shared
//! melee behaviors `approach`/`lunge`/`kite`) and the BASE ROSTER the starter biomes
//! spawn: boar, wasp, thornling, wolf, bear, spider, scorpion, burrower, vulture, golem,
//! bat, hurler — every stat, cadence and quirk lifted from its js factory.
//!
//! REDESIGNED per the improve-don't-copy rule: the js built each kind as a closure over
//! mob(); here [`MOB_DEFS`] is a data table and ONE ai system interprets [`Ai`] variants —
//! adding a mob is a def row (+ an Ai variant only for a genuinely new trick).
//!
//! Not here yet: afflictions on the player (web slow / scorpion venom — the status system),
//! downed/revive undead, burn/sleep/frost on mobs, champion auras, mob ground-shadows.

use crate::actors::mobs_art::{self, MobFrame};
use crate::combat::{Combatant, Health, Hitbox, HurtProfile, Knockback, Team};
use crate::gfx::bake;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

pub const HP_MUL: f64 = 1.5; // global mob-difficulty knob (js HP_MUL)
pub const AGGRO_R: f32 = 80.0; // idle until the player is ~5 tiles away (js AGGRO_R)

/// The melee-lunge state machine's numbers (js `lunge` cfg).
pub struct LungeCfg {
    pub range: f32,
    pub cd: i32,
    pub windup: i32, // 0 = skip the phase
    pub dash: i32,
    pub dash_spd: f32,
    pub recover: i32, // 0 = skip
    pub face_windup: bool,
}

/// One behavior per roster archetype — the js per-kind `ai` closures as data.
pub enum Ai {
    /// Shuffle toward the player (js `approach`, axis mode) while within `range`:
    /// bear/scorpion/golem (unbounded), icetroll/myconid (gated).
    Walker { spd: f32, range: f32 },
    /// Approach within `chase_r` + the lunge state machine: boar (axis) / wolf (vec).
    Chaser { spd: f32, chase_r: f32, vec: bool, refaces: bool, lunge: LungeCfg },
    /// Beeline with a sinusoidal weave, flying over everything: wasp/bat.
    Flyer { spd: f32, jamp: f32, jfreq: f32 },
    /// Sits disguised until the player is close, then walks (thornling).
    Dormant { wake_r: f32, spd: f32 },
    /// Kites to mid-range and spits a slowing web (spider).
    WebSpitter,
    /// Tunnels underground (harmless, invulnerable-ish), surfaces to bite (burrower).
    Burrow,
    /// Circles at range, then dives in a straight swoop (vulture).
    Swoop,
    /// Darts to a random spot at throwing range, lobs an arcing rock (hurler).
    Hurl,
    /// Kites at bow range and looses arrows, rooted while the bow is drawn (archer).
    Shooter { fire_r: f32, cd: i32, root: i32, wobble: f32, sp: f32, near: f32, spd: f32 },
    /// Ranged elemental: backs off inside `near`, closes beyond `far`, fires eBolts
    /// (frostwyrm/pyrewraith/riftlord; sporemother fans three).
    Caster {
        fire_r: f32,
        cd: i32,
        sp: f32,
        dmg: i32,
        color: u32,
        core: u32,
        life: i32,
        fan: i32,      // bolts per volley (1 or 3)
        spread: f32,   // fan angle step
        wobble: f32,   // random aim wobble
        near: f32,
        back_spd: f32,
        far: f32,
        fwd_spd: f32,
    },
    /// Hop-and-rest gel (slimes): moves only during the hop window of its 36-frame cycle.
    Hopper { spd: f32 },
    /// Teleports to a spot just behind the player, then walks in (voidling).
    Blinker { cd: i32, min: f32, max: f32, behind: f32, spd: f32 },
    /// Leaping bog frog. (Its js tongue-grab reel joins with the player-pull mechanic —
    /// TEMP: it leaps only, which is its close-range behaviour anyway.)
    FrogHop,
    /// Rooted crystal (prismshard): a turning cross of four bolts — the rotation
    /// advances an eighth-turn per volley, so no angle stays safe.
    CrossTurret { fire_r: f32, cd: i32, sp: f32, dmg: i32, color: u32, life: i32 },
    /// Blind hunter (deepcrawler): it hears FOOTFALLS — it moves only while the
    /// player does (last-seen position rides cvx/cvy).
    SoundHunter { spd: f32 },
    /// Walking bomb (emberling): chases, arms at arm_r (fuse frames), then detonates
    /// itself — its `volatile` death blast does the rest.
    Fuse { spd: f32, chase_r: f32, arm_r: f32, fuse: i32 },
    /// Rooted vent (ashgeyser): rains a spread of arcing rocks around the player.
    Vent { fire_r: f32, cd: i32, rocks: i32 },
    /// A radial bloom of bolts (mirefly 6-way venom / palehowler 8-way slow): fires n
    /// evenly-spaced bolts at close range, then backs off past `retreat_r`.
    RingBurst {
        fire_r: f32,
        cd: i32,
        n: i32,
        sp: f32,
        dmg: i32,
        color: u32,
        core: u32,
        life: i32,
        afflict: (&'static str, i32),
        retreat_r: f32,
        spd: f32,
    },
    /// On a clock (bellsnail sealed→out / boglight fade→solid): OUTSIDE [active_at, period)
    /// it is invulnerable + harmless (and, when `seal`, rooted); inside it chases at `spd`.
    PhaseClock { period: i32, active_at: i32, spd: f32, seal: bool, invert: bool },
    /// Only moves UNWATCHED (saltstatue): frozen + faced, it stalks fast the moment the
    /// player looks away.
    GazeStalker { spd: f32, range: f32 },
    /// WATER lurker (spitgill — Baz's Zora-ish sniper, past-js): rooted on its water
    /// tile, SUBMERGED (invulnerable, just a ripple) most of its clock; it surfaces
    /// for a beat, spits a bolt at the hero, and sinks again.
    WaterSpitter { period: i32, up_at: i32, fire_r: f32, sp: f32, dmg: i32, color: u32, core: u32, afflict: (&'static str, i32) },
    /// Shore squid (tidewhip, past-js): rooted in the shallows; a hero who strays
    /// near the water eats a tentacle WHIP (the frog-tongue lash, squid-inked).
    WaterWhip { lash_r: f32, cd: i32 },
    /// Raises the dead (gravewarden): summons up to `max` `kind` minions on a cooldown,
    /// keeping its distance.
    Summoner { fire_r: f32, cd: i32, kind: &'static str, max: i32, spd: f32 },
    /// Sidles AROUND the player then pincer-lunges (tidecrab): a strafing circler with a
    /// straight-line dash at close range.
    Strafer { orbit_r: f32, spd: f32, lunge_r: f32, lunge_spd: f32, lunge_t: i32, cd: i32 },
    /// Orbits the player, then DARTS when its back is turned (honeydrone): a flyer that
    /// keeps a ring until it can sting from behind.
    OrbitDart { orbit_r: f32, orbit_spd: f32, dart_r: f32, dart_spd: f32, dart_t: i32, cd: i32 },
    /// Rooted maw that DRAGS the player in (sandmaw): the sand slides you toward the teeth.
    Suction { pull_r: f32, min_r: f32, pull: f32 },
    /// Marks the ground under your feet + calls the sky (stormcaller): a telegraphed strike.
    SkyCaller { fire_r: f32, cd: i32, spd: f32 },
    /// Swaps places with the player on a long cooldown (switchshade): a slow spiral otherwise.
    Swapper { cd: i32, min_r: f32, max_r: f32, spd: f32 },
    /// Terrified light-sprite (glimmerling): flees, pops a spark burst when crowded, and
    /// locks a telegraphed light-beam at mid-range.
    Glimmer { flee_r: f32, burst_r: f32, burst_cd: i32, beam_r: f32, beam_cd: i32, spd: f32 },
    /// Self-mending flyer (witherheart): fires a slow HOMING orb, backs off, and regrows
    /// its wounds when left alone (the regen is applied in ai.rs, which owns Health).
    Drainer { fire_r: f32, cd: i32, spd: f32 },
}

/// A fallen mob's copper (js `o.coin`).
pub enum Coin {
    Default,          // 1 + rand(4)
    None,             // wasp/bat
    Range(i32, i32),  // base + rand(spread)
}

pub struct MobDef {
    pub kind: &'static str,
    pub hp: i32,
    pub damage: i32,
    pub xp: i32,
    pub blood: u32,
    pub defense: i32,
    pub knock_resist: f32,
    /// Lava-native (Baz): walks and stands in lava unburnt; everyone else avoids it.
    pub fireproof: bool,
    /// On-hit status for the player (js o.slow / o.poison / o.burn / o.shock): (id, frames).
    pub afflicts: (&'static str, i32),
    pub fly: bool,
    pub hb: (f32, f32, f32, f32), // ox, oy, w, h
    pub coin: Coin,
    pub potion: f64, // chance of a potion drop
    pub drops: Option<(&'static str, f64, i32, i32)>, // js matDrop: (mat, chance, min, spread)
    pub down_revive: i32, // js downRevive: collapse for n frames instead of dying (zombie)
    pub splits: bool,     // slain -> two small children (slimes)
    pub ghost: bool,      // drawn at 0.8 alpha (wraith)
    pub volatile: bool,   // explodes on ANY death (js e.volatile — the emberling's whole idea)
    /// Inherent draw scale (feet-anchored, hitbox follows) — the sandmaw's bulk.
    pub scale: f32,
    pub ai: Ai,
}

/// Field defaults so the def rows below state only what their js factory states.
pub(crate) const DEF_BASE: MobDef = MobDef {
    kind: "", hp: 1, damage: 1, xp: 1, blood: 0xd82800, defense: 0, knock_resist: 0.0, fireproof: false, afflicts: ("", 0),
    fly: false, hb: (3.0, 4.0, 10.0, 9.0), coin: Coin::Default, potion: 0.0, drops: None,
    down_revive: 0, splits: false, ghost: false, volatile: false, scale: 1.0, ai: Ai::Walker { spd: 0.5, range: 1e9 },
};

/// The base roster — stats verbatim from the js factories.
// The roster itself lives in mob_defs.rs (data-only; this file is the scaffold).
pub use super::mob_defs::MOB_DEFS;

/// The witherheart's stolen-life orb: a rot-green sphere round a pale sick core,
/// shedding a fleck as it drifts (it drained the colour out of something).
const ORB_A: &[&str] = &[
    "..kkkk....", ".kggggk...", "kggppggk..", "kgpccpgk..", "kgpccpgk..",
    "kggppggk..", ".kggggk...", "..kkkk....", "..........", "..........",
];
const ORB_B: &[&str] = &[
    "..kkkk....", ".kgppgk...", "kgpccpgk..", "kgccccgk..", "kgccccgk..",
    "kgpccpgk..", ".kgppgk...", "..kkkk..g.", ".........g", "..........",
];
const ORB_PAL: &[(char, u32)] = &[('k', 0x3a4020), ('g', 0x96a050), ('p', 0xb8c070), ('c', 0xe8f0b0)];


/// The uppercase display name for a mob kind (js BESTIARY[kind].name) — the elite name
/// tag's base; unknown kinds fall back. (Lives here, not the GENERATED mobs_art, so an
/// extractor regen can't drop it.)
pub fn bestiary_name(kind: &str) -> &'static str {
    super::mobs_art::BESTIARY_INFO
        .iter()
        .chain(super::mobs_art_extra::EXTRA_BESTIARY)
        .find(|(k, _, _)| *k == kind)
        .map(|(_, n, _)| *n)
        .unwrap_or("BEAST")
}

pub fn def_index(kind: &str) -> Option<usize> {
    MOB_DEFS.iter().position(|d| d.kind == kind)
}

/// A live biome mob. `st`/`t`/`cd`/`cvx`/`cvy` are the js state fields, one meaning per Ai.
#[derive(Component)]
pub struct Mob {
    pub def: usize,
    pub x: f32,
    pub y: f32,
    /// js speedMul (Swift affix, elites) — mob_step scales every stride by it.
    pub speed_mul: f32,
    /// js sizeMul (elites draw twice tall; the hitbox grows x1.7 centred).
    pub size_mul: f32,
    pub facing: usize, // 0 down / 1 up / 2 right / 3 left
    pub anim: u32,
    pub st: i32,
    pub t: i32,
    pub cd: i32,
    pub cvx: f32,
    pub cvy: f32,
    pub tx: f32,
    pub ty: f32,
    pub has_target: bool,
    pub dart_t: i32,
    pub aggro: bool,
    pub downed: bool, // collapsed, waiting to rise (js downRevive)
    pub down_t: i32,
    pub small: bool, // a slime split-child
    /// Lullaby frames left: asleep, no thinking — a hit wakes it (js applyStatus sleep).
    pub sleep: i32,
}

pub fn mob_bundle(def_idx: usize, x: f32, y: f32) -> impl Bundle {
    let d = &MOB_DEFS[def_idx];
    let hp = ((d.hp as f64) * HP_MUL).round() as i32;
    (
        Mob {
            def: def_idx, x, y, facing: 0, anim: 0, st: 0, t: 0, cd: 0,
            cvx: 0.0, cvy: 0.0, tx: 0.0, ty: 0.0, has_target: false, dart_t: 0, aggro: false,
            downed: false, down_t: 0, small: false, sleep: 0, speed_mul: 1.0, size_mul: d.scale,
        },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(d.damage), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: d.defense, invuln: 0, flash: 0 },
        // js mob onHurt: kb (2.2 + knock) * (1 - knockResist), a clear 11-frame flinch.
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2, kb_resist: d.knock_resist, kb_frames: 11 },
        Knockback::default(),
        Hitbox { x: x + d.hb.0, y: y + d.hb.1, w: d.hb.2, h: d.hb.3 },
        Sprite::default(),
        crate::combat::Afflicts(d.afflicts.0, d.afflicts.1),
    )
}

/// A slime split-child: 1 base hp (x HP_MUL), tiny hitbox, tagged `small` (js slime(sm)).
pub fn mob_bundle_small(def_idx: usize, x: f32, y: f32) -> impl Bundle {
    let d = &MOB_DEFS[def_idx];
    let hp = (1.0 * HP_MUL).round() as i32;
    (
        Mob {
            def: def_idx, x, y, facing: 0, anim: 0, st: 0, t: 0, cd: 0,
            cvx: 0.0, cvy: 0.0, tx: 0.0, ty: 0.0, has_target: false, dart_t: 0, aggro: true,
            downed: false, down_t: 0, small: true, sleep: 0, speed_mul: 1.0, size_mul: 1.0,
        },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(d.damage), persistent: true, knock: 0.0 },
        // Spawn i-frames so the killing blow's lingering hitbox can't instantly pop them (js).
        Health { hp, max: hp, defense: d.defense, invuln: 30, flash: 6 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2, kb_resist: d.knock_resist, kb_frames: 11 },
        Knockback::default(),
        Hitbox { x: x + 5.0, y: y + 9.0, w: 6.0, h: 5.0 },
        Sprite::default(),
    )
}

// --- Art bank ---

/// The burrower's dirt mound (js draws it as three fillRects; baked here once).
const MOUND: &[&str] = &[
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "......tttt......",
    "...mmmttttmm....",
    "...mmmmmmmmmm...",
    ".mmmmmmmmmmmmmm.",
    ".mmmmmmmmmmmmmm.",
    ".mmmmmmmmmmmmmm.",
    ".mmmmmmmmmmmmmm.",
];
const MOUND_PAL: &[(char, u32)] = &[('m', 0xc79b54), ('t', 0xb07a3a)];

/// The spider's web bolt (js: 5px #dfeee0 square with a #aebfb0 core).
const WEB: &[&str] = &["wwwww", "wcccw", "wcccw", "wcccw", "wwwww"];
const WEB_PAL: &[(char, u32)] = &[('w', 0xdfeee0), ('c', 0xaebfb0)];

/// The hurler's rock (js: 5px #9a9da3 square) + its landing shadow.
const ROCK: &[&str] = &["rrrrr", "rrrrr", "rrrrr", "rrrrr", "rrrrr"];
const ROCK_PAL: &[(char, u32)] = &[('r', 0x9a9da3)];

fn flip(grid: &[&str]) -> Vec<String> {
    grid.iter().map(|r| r.chars().rev().collect()).collect()
}

fn bake_frame(images: &mut Assets<Image>, f: &MobFrame, left: bool) -> Baked {
    let (w, h) = (f.grid.first().map_or(0, |r| r.len()) as f32, f.grid.len() as f32);
    let img = if left {
        let flipped = flip(f.grid);
        let refs: Vec<&str> = flipped.iter().map(|s| s.as_str()).collect();
        images.add(bake(&refs, f.pal))
    } else {
        images.add(bake(f.grid, f.pal))
    };
    (img, w, h)
}

/// (handle, native w, native h) — one baked animation frame.
pub type Baked = (Handle<Image>, f32, f32);

/// Baked frames per kind: [right-facing, left-facing] animation strips (+ specials).
#[derive(Resource)]
pub struct MobArtBank {
    pub frames: HashMap<&'static str, [Vec<Baked>; 2]>,
    pub wolf: [[Baked; 2]; 4], // down/up/right/left x 2 frames
    pub thorn_dorm: Baked,
    pub mound: Handle<Image>,
    /// The witherheart's drain orb, 2 pulse frames (it shipped as a flat square).
    pub drain_orb: [Handle<Image>; 2],
    pub web: Handle<Image>,
    pub rock: Handle<Image>,
    pub arrow: Handle<Image>,
    bolts: HashMap<(u32, u32), Handle<Image>>,
}

impl MobArtBank {
    pub fn build(images: &mut Assets<Image>) -> Self {
        let mut frames = HashMap::default();
        for (kind, set) in mobs_art::ALL_FRAMES.iter().chain(super::mobs_art_extra::EXTRA_FRAMES) {
            let right: Vec<_> = set.iter().map(|f| bake_frame(images, f, false)).collect();
            let left: Vec<_> = set.iter().map(|f| bake_frame(images, f, true)).collect();
            frames.insert(*kind, [right, left]);
        }
        // CULTISTS ARE PEOPLE (Baz): a person in the purple hood and robe, not the
        // retired js blob. The caster def keeps its AI; the LOOK is hero side-frames
        // in cultist gear, baked over the generated entry. (This bank also feeds the
        // bestiary page, so the codex shows the same hooded figure.)
        {
            let look = crate::actors::hero::build_frames_geared(
                &crate::actors::hero::random_look(0xba9d),
                &[
                    Some(&super::goblin::CULTIST_HOOD),
                    Some(&super::goblin::CULTIST_ROBE),
                    Some(&super::goblin::CULTIST_BOOTS),
                ],
                images,
            )
            .frames;
            let strip = |f: usize| -> Vec<Baked> { look[f].iter().map(|h| (h.clone(), 16.0, 16.0)).collect() };
            frames.insert("cultist", [strip(2), strip(3)]); // right, left
        }
        let mut wf = |set: &[MobFrame], left: bool| -> [Baked; 2] {
            [bake_frame(images, &set[0], left), bake_frame(images, &set[1], left)]
        };
        Self {
            frames,
            wolf: [
                wf(mobs_art::WOLF_DOWN, false),
                wf(mobs_art::WOLF_UP, false),
                wf(mobs_art::WOLF_RIGHT, false),
                wf(mobs_art::WOLF_RIGHT, true), // js: left = right flipped
            ],
            thorn_dorm: bake_frame(images, &mobs_art::THORN_DORM, false),
            mound: images.add(bake(MOUND, MOUND_PAL)),
            drain_orb: [images.add(bake(ORB_A, ORB_PAL)), images.add(bake(ORB_B, ORB_PAL))],
            web: images.add(bake(WEB, WEB_PAL)),
            rock: images.add(bake(ROCK, ROCK_PAL)),
            arrow: bake_frame(images, &mobs_art::ARROW_SPR, false).0,
            bolts: {
                // Pre-bake every caster's bolt (8px body + 3px core — js eBolt draw).
                let mut map = HashMap::default();
                for d in MOB_DEFS {
                    if let Ai::Caster { color, core, .. } = d.ai {
                        map.entry((color, core)).or_insert_with(|| {
                            let grid: &[&str] = &["bbbbbbbb", "bbbbbbbb", "bbcccbbb", "bbcccbbb", "bbcccbbb", "bbbbbbbb", "bbbbbbbb", "bbbbbbbb"];
                            images.add(bake(grid, &[('b', color), ('c', core)]))
                        });
                    }
                }
                map
            },
        }
    }

    /// The pre-baked bolt for a caster's colour pair.
    pub fn bolt(&self, color: u32, core: u32) -> Handle<Image> {
        self.bolts.get(&(color, core)).cloned().unwrap_or_else(|| self.web.clone())
    }
}

// --- Projectiles ---

/// The spider's web bolt (js `web` via `projectile`): straight flight, dies on walls.
/// (Its js `slow: 110` affliction joins with the status system.)
#[derive(Component)]
pub struct WebBolt {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: i32,
}

/// The hurler's lobbed rock (js `arcRock`): flies harmless on a fixed arc, then the
/// LANDING carries the damage for 5 frames.
#[derive(Component)]
pub struct ArcRock {
    pub sx: f32,
    pub sy: f32,
    pub tx: f32,
    pub ty: f32,
    pub t: i32,
    pub dur: i32,
}

/// The landing-telegraph shadow under an [`ArcRock`] — reaped with it.
#[derive(Component)]
pub struct ArcShadow(pub Entity);

// --- The js movement/facing helpers ---

pub fn face_from(dx: f32, dy: f32) -> usize {
    if dx.abs() > dy.abs() {
        if dx < 0.0 { 3 } else { 2 }
    } else if dy < 0.0 {
        1
    } else {
        0
    }
}

/// What a mob's brain asked the world to do this tick.
pub enum MobAct {
    Web { x: f32, y: f32, vx: f32, vy: f32 },
    Rock { sx: f32, sy: f32, tx: f32, ty: f32, dur: i32 },
    Arrow { x: f32, y: f32, vx: f32, vy: f32 },
    Bolts { x: f32, y: f32, angs: Vec<f32>, sp: f32, dmg: i32, color: u32, core: u32, life: i32, afflict: (&'static str, i32) },
    Blinked, // voidling teleported — the caller grants its brief i-frames
    /// A volley of arcing rocks (ashgeyser) — each (sx, sy, tx, ty, dur).
    Rocks(Vec<(f32, f32, f32, f32, i32)>),
    /// The fuse ran out (emberling): the caller zeroes its own health; the volatile
    /// death blast follows from deaths.rs.
    SelfDestruct,
    /// Raise a minion (gravewarden): a fresh `kind` mob at (x, y).
    Summon { kind: &'static str, x: f32, y: f32 },
    /// Drag the player toward (tx, ty) by `pull` px (sandmaw).
    PullPlayer { tx: f32, ty: f32, pull: f32 },
    /// A telegraphed sky-strike centred at (x, y) (stormcaller).
    SkyStrike { x: f32, y: f32 },
    /// Swap the mob and player positions (switchshade) — the mob teleports to `to`.
    SwapPlayer { to: Vec2 },
    /// A spark ring at (x, y) (glimmerling glimmerBurst).
    Burst { x: f32, y: f32 },
    /// A telegraphed light-beam locked from (x, y) toward (tx, ty) (glimmerBeam).
    Beam { x: f32, y: f32, tx: f32, ty: f32 },
    /// A slow homing drain-orb launched from (x, y) at (vx, vy) (witherheart drainOrb).
    DrainOrb { x: f32, y: f32, vx: f32, vy: f32 },
    /// A frog's lash-tongue VISUAL: a line + tip flung from the maw (ax, ay) along
    /// (ux, uy) out to `len` px (frogHop). The grab/reel itself rides PullPlayer — this
    /// is the visual twin of the mimic's tongue (dungeon.rs), animated self-contained.
    Tongue { ax: f32, ay: f32, ux: f32, uy: f32, len: f32 },
}

/// The full js `o.ai` dispatch, one tick (the caller already handled the aggro gate and
/// knockback). `contact` mirrors js `e.damage` toggling (the burrower is harmless while
/// underground); `hittable` goes false while it's tunnelling (js e.invuln=3 each frame).
// The AI interpreter itself lives in mob_think.rs (this file stays the data shapes,
// bundles and art bank).
pub use super::mob_think::{mob_step, mob_think};
