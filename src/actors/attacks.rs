//! attacks.rs — swing/thrown attack entities: the player's sword arc (items.js `makeSwing`),
//! the goblin's axe swipe (`axeAttack`) and slung stone (`pebble`).
//!
//! Each is a short-lived entity whose system updates its hitbox window and visual each tick,
//! exactly on the JS timings. The blade/axe sprites pivot at the wielder's hand: canvas
//! `rotate()` is clockwise in a y-down world, Bevy rotation is CCW in y-up — same screen
//! direction, so angles carry over with a sign flip on the TRANSLATION mapping only.

use super::goblin_art::{GOB_AXE, PEBBLE, SWORD};
use super::tools_art::{AXE_SPRITE, PICK_SPRITE};
use crate::combat::{AttackTool, Combatant, HitOnce, Hitbox, Team, Tool};
use crate::gfx::bake;
use crate::room::{PX_H, PX_W};
use crate::room::RoomGrid;
use bevy::prelude::*;
use bevy::sprite::Anchor;

/// One tool's swing parameters — ports of SWORD_SWING / AXE_SWING / PICK_SWING (items.js).
pub struct SwingSpec {
    pub damage: i32,
    pub reach: f32,
    pub hit_box: f32,
    pub life: u32,
    pub arc: f32,
    pub grip_y: f32,
    pub cooldown: u32,
    pub lock: u32,
    /// The weapon's own crit chance on top of the player's (js cfg.crit).
    pub crit: f64,
}

pub fn swing_spec(tool: Tool) -> &'static SwingSpec {
    use std::f32::consts::PI;
    match tool {
        Tool::Sword => &SwingSpec { damage: 1, reach: 12.0, hit_box: 24.0, life: 14, arc: PI * 0.9, grip_y: -20.0, cooldown: 20, lock: 14, crit: 0.05 },
        Tool::Axe => &SwingSpec { damage: 2, reach: 8.0, hit_box: 16.0, life: 16, arc: PI * 0.7, grip_y: -14.0, cooldown: 30, lock: 18, crit: 0.05 },
        Tool::Pick => &SwingSpec { damage: 1, reach: 9.0, hit_box: 16.0, life: 16, arc: PI * 0.6, grip_y: -16.0, cooldown: 28, lock: 16, crit: 0.0 },
    }
}

/// Facing table for attacks — port of FACE in items.js (dx, dy, canvas angle, spin).
/// Indexed by `hero::Facing as usize` (down, up, right, left).
pub const FACE: [(f32, f32, f32, f32); 4] = [
    (0.0, 1.0, std::f32::consts::PI, 1.0),        // down
    (0.0, -1.0, 0.0, 1.0),                        // up
    (1.0, 0.0, std::f32::consts::FRAC_PI_2, 1.0), // right
    (-1.0, 0.0, -std::f32::consts::FRAC_PI_2, -1.0), // left
];

#[derive(Resource)]
pub struct AttackArt {
    pub sword: Handle<Image>,
    pub axe: Handle<Image>, // the GOBLIN's axe swipe
    pub pebble: Handle<Image>,
    pub sword_size: Vec2,
    pub axe_size: Vec2,
    // The player's gather tools (items.js AXE / PICK swing sprites).
    pub tool_axe: Handle<Image>,
    pub tool_axe_size: Vec2,
    pub tool_pick: Handle<Image>,
    pub tool_pick_size: Vec2,
    /// Metal-recoloured pick/axe swing sprites, keyed by the tiered tool's item id (js
    /// TOOL_METALS bakes G_PICK/G_AXE with `m.ov` per swing — baked once here instead).
    pub tiered: bevy::platform::collections::HashMap<&'static str, Handle<Image>>,
}

pub fn build_attack_art(images: &mut Assets<Image>) -> AttackArt {
    let sword_img = bake(SWORD, &[]);
    let axe_img = bake(GOB_AXE, &[]);
    let tool_axe_img = bake(AXE_SPRITE, &[]);
    let tool_pick_img = bake(PICK_SPRITE, &[]);
    let sword_size = sword_img.size().as_vec2();
    let axe_size = axe_img.size().as_vec2();
    let tool_axe_size = tool_axe_img.size().as_vec2();
    let tool_pick_size = tool_pick_img.size().as_vec2();
    // One recoloured swing sprite per tiered pick/axe (its metal overlay on the shared head).
    let mut tiered = bevy::platform::collections::HashMap::default();
    for d in crate::items::all_defs() {
        if let (Some(mat), Some(tool)) = (d.tool_mat, d.tool) {
            let grid = if tool == Tool::Pick { PICK_SPRITE } else { AXE_SPRITE };
            tiered.insert(d.id, images.add(bake(grid, crate::items::tool_ov(mat))));
        }
    }
    AttackArt {
        sword: images.add(sword_img),
        axe: images.add(axe_img),
        pebble: images.add(bake(PEBBLE, &[])),
        sword_size,
        axe_size,
        tool_axe: images.add(tool_axe_img),
        tool_axe_size,
        tool_pick: images.add(tool_pick_img),
        tool_pick_size,
        tiered,
    }
}

/// The player's swing arc (sword, axe or pick): hitbox rides the player at the tool's
/// reach, blade sweeps the tool's arc.
#[derive(Component)]
pub struct Swing {
    pub life: u32,
    pub facing: usize,
    pub tool: Tool,
    /// The head's rank (js toolTier): the harvest gate rejects a hit when a node's
    /// req_tier outranks it. Base sword 0, base pick/axe 1, metal tools 2..6.
    pub tool_tier: i32,
    /// Extra hitbox size for HOLD moves (cleave/shatter) — 0 for a plain tap swing.
    pub grow: f32,
}

/// The goblin's axe swipe: pivots on the goblin, active only after the wind-up.
#[derive(Component)]
pub struct AxeSwipe {
    pub life: u32,
    pub fx: f32,
    pub fy: f32,
    /// The swinging goblin — the JS closure read `g.x` LIVE, so the axe rides knockback.
    pub wielder: Entity,
    pub ox: f32, // last known wielder position (holds if the wielder dies mid-swing)
    pub oy: f32,
}
pub const AXE_LIFE: u32 = 16;

/// A slung stone: straight flight, dies on solids/borders/timeout (port of `pebble`).
#[derive(Component)]
pub struct Stone {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: u32,
}

/// A grip-anchored, rotating weapon visual — the shared piece for sword + axe (the reuse
/// rule: ONE function knows how a swung blade is drawn). Canvas drew the sprite at
/// (-4, grip_y) from the pivot; in Bevy 0.19 the pivot is the separate `Anchor` component,
/// centre-relative: sprite-pixel (4, -grip_y) from the top-left.
fn blade_visual(image: Handle<Image>, size: Vec2, grip_y: f32) -> (Sprite, Anchor) {
    // The pivot sits at image-pixel (4, -grip_y) from the TOP-LEFT. Bevy's Anchor is
    // centre-relative with +y UP, so: x = px/w - 0.5, y = 0.5 - py/h. (The first cut had
    // the y sign flipped, which hung the blade below the hand — the "backwards swing".)
    let (px, py) = (4.0, -grip_y);
    (
        Sprite::from_image(image),
        Anchor(Vec2::new(px / size.x - 0.5, 0.5 - py / size.y)),
    )
}

pub fn sword_visual(art: &AttackArt) -> (Sprite, Anchor) {
    blade_visual(art.sword.clone(), art.sword_size, swing_spec(Tool::Sword).grip_y)
}
fn tool_visual(tool: Tool, art: &AttackArt, tiered_img: Option<Handle<Image>>) -> (Sprite, Anchor) {
    let (img, size) = match tool {
        Tool::Sword => (art.sword.clone(), art.sword_size),
        Tool::Axe => (tiered_img.unwrap_or_else(|| art.tool_axe.clone()), art.tool_axe_size),
        Tool::Pick => (tiered_img.unwrap_or_else(|| art.tool_pick.clone()), art.tool_pick_size),
    };
    blade_visual(img, size, swing_spec(tool).grip_y)
}
pub fn axe_visual(art: &AttackArt) -> (Sprite, Anchor) {
    blade_visual(art.axe.clone(), art.axe_size, -14.0) // axe draws at (-4, -14)
}

/// Render layer for the player's swing, relative to the wielder's depth-band z: a blade
/// swung UPWARD tucks BEHIND the body, so the hero occludes the hilt instead of the sword
/// pasting over his face.
pub fn swing_z(facing: usize, wielder_z: f32) -> f32 {
    if facing == 1 { wielder_z - 0.005 } else { wielder_z + 0.005 } // 1 = Facing::Up
}
/// Same rule for the goblin's axe.
pub fn axe_z(fy: f32, wielder_z: f32) -> f32 {
    if fy < 0.0 { wielder_z - 0.005 } else { wielder_z + 0.005 }
}

pub fn swing_bundle(facing: usize, tool: Tool, damage: i32, tool_tier: i32, art: &AttackArt, tiered_img: Option<Handle<Image>>) -> impl Bundle {
    let spec = swing_spec(tool);
    (
        Swing { life: spec.life, facing, tool, tool_tier, grow: 0.0 },
        // hurt_team None: player swings hit foes AND resource nodes (js weapons set no
        // hurtTeam — the tool/team gates in resolve_combat do the sorting). `damage` is the
        // caller's final number (base spec x the tree's melee bonus).
        Combatant { team: Team::Player, hurt_team: None, damage: Some(damage), persistent: false, knock: 0.0 },
        AttackTool(tool, tool_tier),
        HitOnce::default(),
        Hitbox { x: -999.0, y: -999.0, w: spec.hit_box, h: spec.hit_box }, // placed on first tick
        tool_visual(tool, art, tiered_img),
    )
}

pub fn axe_bundle(fx: f32, fy: f32, wielder: Entity, ox: f32, oy: f32, art: &AttackArt) -> impl Bundle {
    (
        AxeSwipe { life: AXE_LIFE, fx, fy, wielder, ox, oy },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(1), persistent: false, knock: 0.0 },
        HitOnce::default(),
        Hitbox { x: -999.0, y: -999.0, w: 12.0, h: 12.0 },
        axe_visual(art),
    )
}

pub fn stone_bundle(x: f32, y: f32, angle: f32, art: &AttackArt) -> impl Bundle {
    let sp = 2.4;
    (
        Stone { x, y, vx: angle.cos() * sp, vy: angle.sin() * sp, life: 130 },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(1), persistent: false, knock: 0.0 },
        HitOnce::default(),
        Hitbox { x: x + 5.0, y: y + 5.0, w: 6.0, h: 6.0 },
        Sprite::from_image(art.pebble.clone()),
    )
}

/// Advance the sword swing: hitbox in front of the player at fixed reach across the arc.
/// Returns (hitbox, canvas rotation, pivot in room px, alive).
pub fn swing_tick(s: &mut Swing, px: f32, py: f32) -> (Hitbox, f32, Vec2, bool) {
    let spec = swing_spec(s.tool);
    s.life = s.life.saturating_sub(1);
    let (dx, dy, ang, spin) = FACE[s.facing];
    let prog = 1.0 - s.life as f32 / spec.life as f32;
    let cx = px + 8.0 + dx * spec.reach;
    let cy = py + 9.0 + dy * spec.reach;
    let g = s.grow;
    let hb = Hitbox { x: cx - (spec.hit_box + g) / 2.0, y: cy - (spec.hit_box + g) / 2.0, w: spec.hit_box + g, h: spec.hit_box + g };
    let rot = ang + spin * (-spec.arc / 2.0 + spec.arc * prog);
    (hb, rot, Vec2::new(px + 8.0, py + 9.0), s.life > 0)
}

/// Advance the goblin axe: wind back then swing through; hitbox active after the wind-up.
pub fn axe_tick(a: &mut AxeSwipe) -> (Option<Hitbox>, f32, Vec2, bool) {
    a.life = a.life.saturating_sub(1);
    let hb = if (a.life as f32) < AXE_LIFE as f32 * 0.55 {
        let cx = a.ox + 8.0 + a.fx * 9.0;
        let cy = a.oy + 9.0 + a.fy * 9.0;
        Some(Hitbox { x: cx - 6.0, y: cy - 6.0, w: 12.0, h: 12.0 })
    } else {
        None
    };
    let p = 1.0 - a.life as f32 / AXE_LIFE as f32;
    let base = a.fy.atan2(a.fx) + std::f32::consts::FRAC_PI_2;
    // DELIBERATE DEVIATION from the JS: it always swept clockwise, which chops top-to-bottom
    // facing right but reads as a bottom-up flick facing left. Mirror the sweep on left
    // swings so both sides chop downward — the sword's FACE spin column, applied here too.
    let spin = if a.fx < 0.0 { -1.0 } else { 1.0 };
    let rot = base + spin * (-1.0 + 1.9 * p);
    (hb, rot, Vec2::new(a.ox + 8.0, a.oy + 9.0), a.life > 0)
}

/// Advance a stone: straight flight; dead on solids, borders, or timeout.
pub fn stone_tick(st: &mut Stone, grid: &RoomGrid) -> bool {
    st.x += st.vx;
    st.y += st.vy;
    st.life = st.life.saturating_sub(1);
    !(grid.box_hits_solid(st.x + 5.0, st.y + 5.0, 6.0, 6.0)
        || st.x < -16.0
        || st.x > PX_W as f32
        || st.y < -16.0
        || st.y > PX_H as f32
        || st.life == 0)
}
