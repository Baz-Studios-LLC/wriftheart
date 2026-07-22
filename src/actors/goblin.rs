//! goblin.rs — the first ported enemy (js/enemies.js `goblin`): melee (axe) + slinger (spear
//! frames, thrown stones). Red goblins join once their spear art lands.
//!
//! JS numbers kept exactly: speed 0.55, chase 86, melee range 28 / throw 90 (keep-away 52),
//! attack cds 72 / 230, hp 2 * HP_MUL(1.5) = 3, contact damage 1, i-frames 10, knockback
//! 2.2 base for 11 frames. Wander repicks every 35-80 frames; walk anim flips every 10.

use super::goblin_art::{GOBLIN_FRAMES, SPEAR_FRAMES};
use crate::actors::hero::Facing;
use crate::combat::{Blood, Combatant, Health, Hitbox, HurtProfile, Knockback, Team};
use crate::gfx::bake;
use crate::room::RoomGrid;
use bevy::prelude::*;

const SPEED: f32 = 0.55;
const CHASE_DIST: f32 = 86.0;
const ATTACK_RANGE: f32 = 28.0;
const THROW_RANGE: f32 = 90.0;
const KEEP_DIST: f32 = 52.0;
const HP_MUL: f32 = 1.5;

/// Which goblin — decides frames, range, cadence (port of the `kind` argument).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GoblinKind {
    Melee,
    Spear,
}

#[derive(Component)]
pub struct Goblin {
    pub kind: GoblinKind,
    pub x: f32,
    pub y: f32,
    pub facing: Facing,
    dirx: f32,
    diry: f32,
    move_timer: i32,
    anim_timer: u32,
    pub frame: usize,
    attack_cd: u32,
    pub attack_timer: u32,
}

/// Baked goblin frames: [kind][facing][frame].
#[derive(Resource)]
pub struct GoblinArt(pub [[[Handle<Image>; 2]; 4]; 2]);

/// A HUMANOID riding the goblin chassis wears PEOPLE art (Baz: "bandits should look
/// like people in bandit costumes"): the sync swaps its sprite for these seeded
/// hero-pipeline frames, and deaths credit `kind` instead of "goblin".
#[derive(Component)]
pub struct HumanSkin {
    pub kind: &'static str,
    /// The look's seed — the room cache re-dresses a restored bandit from it.
    pub seed: u32,
    pub frames: [[Handle<Image>; 4]; 4],
}

// The bandit costume: charcoal hood, dark-red studded vest, worn leather boots —
// a masked raider (js S_BANDIT's silhouette) over a seeded villager look.
static BANDIT_HOOD: crate::actors::hero::ArmorLook =
    crate::actors::hero::ArmorLook { style: "hood", lite: 0x3a3a44, dark: 0x1c1c24 };
static BANDIT_VEST: crate::actors::hero::ArmorLook =
    crate::actors::hero::ArmorLook { style: "studs", lite: 0x6e2a24, dark: 0x3a1410 };
static BANDIT_BOOTS: crate::actors::hero::ArmorLook =
    crate::actors::hero::ArmorLook { style: "boots", lite: 0x4a3018, dark: 0x2a1c10 };
// The cultist costume: the CHOIR'S grey (the opening's hilltop singers and the still
// watcher above burning Emberfall wear exactly these) — hood + long grey robe.
pub static CULTIST_HOOD: crate::actors::hero::ArmorLook =
    crate::actors::hero::ArmorLook { style: "hood", lite: 0x767b84, dark: 0x42464e };
pub static CULTIST_ROBE: crate::actors::hero::ArmorLook =
    crate::actors::hero::ArmorLook { style: "robe", lite: 0x8a8f98, dark: 0x525660 };
pub static CULTIST_BOOTS: crate::actors::hero::ArmorLook =
    crate::actors::hero::ArmorLook { style: "boots", lite: 0x3a3e46, dark: 0x22262c };

/// The humanoid WARDROBE: seeded people-in-costume frames by (kind, look seed),
/// cached — every camp is a gang of DIFFERENT people in that kind's colours (and
/// re-entering a room reuses its images instead of leaking them).
#[derive(Resource, Default)]
pub struct HumanArt(pub bevy::platform::collections::HashMap<(&'static str, u32), [[Handle<Image>; 4]; 4]>);

impl HumanArt {
    pub fn frames(&mut self, kind: &'static str, seed: u32, images: &mut Assets<Image>) -> [[Handle<Image>; 4]; 4] {
        self.0
            .entry((kind, seed))
            .or_insert_with(|| {
                let arm: crate::actors::hero::WornArm = match kind {
                    "cultist" => [Some(&CULTIST_HOOD), Some(&CULTIST_ROBE), Some(&CULTIST_BOOTS)],
                    _ => [Some(&BANDIT_HOOD), Some(&BANDIT_VEST), Some(&BANDIT_BOOTS)],
                };
                crate::actors::hero::build_frames_geared(&crate::actors::hero::random_look(seed), &arm, images).frames
            })
            .clone()
    }
}

pub fn build_goblin_art(images: &mut Assets<Image>) -> GoblinArt {
    let mut make = |table: &[(&str, [[&str; 16]; 2])]| -> [[Handle<Image>; 2]; 4] {
        // Facing order matches hero::Facing: down, up, right, left.
        std::array::from_fn(|fi| {
            let want = ["down", "up", "right", "left"][fi];
            let (_, frames) = table.iter().find(|(f, _)| *f == want).unwrap();
            std::array::from_fn(|i| images.add(bake(&frames[i], &[])))
        })
    };
    GoblinArt([make(GOBLIN_FRAMES), make(SPEAR_FRAMES)])
}

/// Spawn one goblin at room-pixel (x, y) — the component side of the JS constructor.
pub fn goblin_bundle(kind: GoblinKind, x: f32, y: f32) -> impl Bundle {
    (
        Goblin {
            kind,
            x,
            y,
            facing: Facing::Down,
            dirx: 0.0,
            diry: 0.0,
            move_timer: 0,
            anim_timer: 0,
            frame: 0,
            attack_cd: 0,
            attack_timer: 0,
        },
        Combatant {
            team: Team::Enemy,
            hurt_team: Some(Team::Player),
            damage: Some(1), // bumping into a goblin hurts (contact damage)
            persistent: true,
            knock: 0.0,
        },
        Health { hp: (2.0 * HP_MUL) as i32, max: (2.0 * HP_MUL) as i32, defense: 0, invuln: 0, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2, kb_frames: 11 },
        Knockback::default(),
        Blood(0xd82800),
        Hitbox { x: x + 3.0, y: y + 4.0, w: 10.0, h: 10.0 },
    )
}

/// What a goblin decided to do this tick that needs the world to act (spawned attacks).
pub enum GoblinAct {
    Axe { fx: f32, fy: f32, x: f32, y: f32 },
    Stone { x: f32, y: f32, angle: f32 },
}

/// The goblin brain — port of the JS update() body. Returns attack intents for the caller to
/// spawn (systems can't spawn mid-iteration without commands; play.rs applies them).
#[allow(clippy::too_many_arguments)]
pub fn goblin_think(
    g: &mut Goblin,
    kb: &Knockback,
    grid: &RoomGrid,
    blockers: &[(f32, f32, f32, f32)],
    player_pos: Vec2,
    player_box: &Hitbox,
    rand: &mut impl FnMut() -> f32,
) -> Option<GoblinAct> {
    let ranged = g.kind == GoblinKind::Spear;
    let atk_range = if ranged { THROW_RANGE } else { ATTACK_RANGE };
    let atk_cd = if ranged { 230 } else { 72 };

    if g.attack_cd > 0 {
        g.attack_cd -= 1;
    }
    // Knockback in flight: the shared knockback system moves the body; AI is suspended
    // (that suspension is what makes hits actually interrupt attacks — playtest note in JS).
    if kb.timer > 0 {
        return None;
    }
    if g.attack_timer > 0 {
        g.attack_timer -= 1; // rooted mid-attack
        return None;
    }

    let pdx = player_pos.x - g.x;
    let pdy = player_pos.y - g.y;
    let dist = pdx.hypot(pdy);
    if dist < atk_range && g.attack_cd == 0 {
        let (mut fx, mut fy) = (0.0f32, 0.0f32);
        if pdx.abs() > pdy.abs() {
            fx = if pdx < 0.0 { -1.0 } else { 1.0 };
        } else {
            fy = if pdy < 0.0 { -1.0 } else { 1.0 };
        }
        g.facing = face_of(fx, fy);
        g.attack_cd = atk_cd;
        return Some(if ranged {
            g.attack_timer = 12;
            let angle = pdy.atan2(pdx) + (rand() - 0.5) * 0.38; // ±11° wobble
            GoblinAct::Stone { x: g.x, y: g.y, angle }
        } else {
            g.attack_timer = 16;
            GoblinAct::Axe { fx, fy, x: g.x, y: g.y }
        });
    }

    g.move_timer -= 1;
    if g.move_timer <= 0 {
        g.move_timer = 35 + (rand() * 45.0).floor() as i32;
        if ranged {
            if dist < KEEP_DIST {
                g.dirx = -pdx.signum();
                g.diry = -pdy.signum();
            } else if dist > atk_range {
                g.dirx = pdx.signum();
                g.diry = pdy.signum();
            } else {
                g.dirx = 0.0;
                g.diry = 0.0;
            }
        } else if dist < CHASE_DIST {
            g.dirx = pdx.signum();
            g.diry = pdy.signum();
        } else {
            let r = (rand() * 5.0).floor() as i32;
            g.dirx = 0.0;
            g.diry = 0.0;
            match r {
                0 => g.dirx = -1.0,
                1 => g.dirx = 1.0,
                2 => g.diry = -1.0,
                3 => g.diry = 1.0,
                _ => {}
            }
        }
    }
    gob_move(g, grid, blockers, player_box, g.dirx * SPEED, 0.0);
    gob_move(g, grid, blockers, player_box, 0.0, g.diry * SPEED);
    // Face the DOMINANT axis on diagonals (the JS fix for side-on-only goblins).
    if g.dirx != 0.0 && g.diry != 0.0 {
        g.facing = if pdx.abs() > pdy.abs() { face_of(g.dirx, 0.0) } else { face_of(0.0, g.diry) };
    } else if g.dirx != 0.0 {
        g.facing = face_of(g.dirx, 0.0);
    } else if g.diry != 0.0 {
        g.facing = face_of(0.0, g.diry);
    }
    if g.dirx != 0.0 || g.diry != 0.0 {
        g.anim_timer += 1;
        if g.anim_timer >= 10 {
            g.anim_timer = 0;
            g.frame ^= 1;
        }
    } else {
        g.frame = 0;
    }
    None
}

fn face_of(fx: f32, fy: f32) -> Facing {
    if fx < 0.0 {
        Facing::Left
    } else if fx > 0.0 {
        Facing::Right
    } else if fy < 0.0 {
        Facing::Up
    } else {
        Facing::Down
    }
}

/// Port of the goblin's `move()`: grid solids + solid props + never walking through the player.
fn gob_move(
    g: &mut Goblin,
    grid: &RoomGrid,
    blockers: &[(f32, f32, f32, f32)],
    player_box: &Hitbox,
    dx: f32,
    dy: f32,
) {
    if dx == 0.0 && dy == 0.0 {
        return;
    }
    let nx = g.x + dx;
    let ny = g.y + dy;
    let (bx, by, bw, bh) = (nx + 3.0, ny + 8.0, 10.0, 6.0);
    if grid.box_hits_solid(bx, by, bw, bh)
        || crate::room::blockers_block(blockers, (g.x + 3.0, g.y + 8.0, 10.0, 6.0), (bx, by, bw, bh))
    {
        return;
    }
    let feet = Hitbox { x: bx, y: by, w: bw, h: bh };
    if feet.overlaps(player_box) {
        return;
    }
    g.x = nx;
    g.y = ny;
}

/// A goblin's live hitbox from its position (the JS per-tick hitbox update).
pub fn goblin_hitbox(g: &Goblin) -> Hitbox {
    Hitbox { x: g.x + 3.0, y: g.y + 4.0, w: 10.0, h: 10.0 }
}
