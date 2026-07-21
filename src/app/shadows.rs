//! shadows.rs — cast shadows under everything that stands, walks or flies
//! (PORT-ORIGINAL, Baz 2026-07-16: the js draws none; the early flat-blob mode and
//! its VIDEO toggle were retired once the shader look won).
//!
//! Every shadow is a [`Mesh2d`] quad running gfx/shadow.wgsl over the OWNER's live
//! sprite art: flipped at the feet, gaussian-blurred, and SHEARED so the silhouette
//! leans with the sun — long west at dawn, tight at noon, long east at dusk, faint
//! moon shadow after dark — while the feet stay planted. Because the material samples
//! the live texture, walk cycles, wing flaps and tree growth stages shadow for free.
//!
//! Each shadow-bearing type (player, villagers, goblins, biome mobs, critters, and
//! the gather props — trees/bushes/boulders) is a query line here — attach on sight,
//! follow every frame, reap when the owner goes. The same anchor pass drives the
//! water pass's REFLECTIONS: a second quad per owner, mirrored onto the water mask.
//! Fliers cast smaller/fainter; the player's hides with the body on death; fireflies
//! are LIGHTS — no shadow.

use super::battle::RoomActor;
use super::play::Player;
use super::room_render::{PLAY_X, PLAY_Y};
use crate::actors::critters::Critter;
use crate::actors::goblin::Goblin;
use crate::actors::mobs::{Mob, MOB_DEFS};
use crate::actors::villager::Villager;
use crate::app::gather::{GatherNode, Shake};
use crate::combat::Health;
use crate::gfx::shadow_material::{ShadowMaterial, ShadowParams, ShadowQuad};
use crate::gfx::water_material::{ReflectionMaterial, ReflectionParams};
use crate::gfx::{at, layers, PIXEL_LAYER};
use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

/// The quad is wider than the art by this factor so a sheared silhouette never clips
/// (shadow.wgsl maps uv through the same margin — keep them in lockstep).
pub const CAST_MARGIN: f32 = 1.5;

/// On the OWNER: its shadow entity (attach-once marker).
#[derive(Component)]
pub struct Shadowed(Entity);

/// On the SHADOW quad: its owner (the follow + reap key).
#[derive(Component)]
pub struct ShadowOf(Entity);

/// On the OWNER: its water reflection (attach-once marker; the water pass).
#[derive(Component)]
pub struct Reflected(Entity);

/// On the REFLECTION quad: its owner.
#[derive(Component)]
pub struct Reflection(Entity);

/// Opt-in for plain sprites that aren't live actors or gather nodes (growth-stage
/// stumps/saplings): a static feet anchor in room pixels. The silhouette still
/// comes from the live sprite, so each growth stage casts its own shape.
#[derive(Component)]
pub struct CastsShadow {
    pub left: f32,
    pub top: f32,
    pub w: u32,
    pub a: f32,
}

/// Where an owner grounds this frame.
#[derive(Clone)]
struct Anchor {
    left: f32, // the FEET band's rect (its centre anchors both quads)
    top: f32,
    w: u32,
    a: f32,     // opacity (fliers fainter)
    hide: bool, // dead player etc
    /// The owner's live sprite image + drawn size (walk cycle shadows for free).
    sil: Option<(Handle<Image>, Vec2)>,
}

/// People (player/villager/goblin) share the 16px sprite box; feet band at +12.
fn person_anchor(x: f32, y: f32, hide: bool) -> Anchor {
    Anchor { left: x.round() + 2.0, top: y.round() + 12.0, w: 12, a: 1.0, hide, sil: None }
}

pub struct ShadowsPlugin;

impl Plugin for ShadowsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_shadows);
    }
}

/// The pass's resource side, bundled (Bevy's 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
struct FxCtx<'w> {
    images: Res<'w, Assets<Image>>,
    materials: ResMut<'w, Assets<ShadowMaterial>>,
    refl_materials: ResMut<'w, Assets<ReflectionMaterial>>,
    quad: Res<'w, ShadowQuad>,
    clock: Res<'w, super::room_render::FrameClock>,
    water: Res<'w, super::water::WaterMask>,
    weather: Res<'w, super::weather::WeatherState>,
}

/// Attach + follow + reap: shadows and water reflections off one anchor pass.
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // ECS system params are wide by nature
fn sync_shadows(
    mut commands: Commands,
    mut fx: FxCtx,
    players: Query<(Entity, &Player, &Health, &Sprite, Option<&Shadowed>, Option<&Reflected>), Without<ShadowOf>>,
    villagers: Query<(Entity, &Villager, &Sprite, Option<&Shadowed>, Option<&Reflected>), Without<ShadowOf>>,
    goblins: Query<(Entity, &Goblin, &Health, &Sprite, Option<&Shadowed>, Option<&Reflected>), (Without<Player>, Without<ShadowOf>)>,
    mobs: Query<(Entity, &Mob, &Health, &Sprite, Option<&Shadowed>, Option<&Reflected>), Without<ShadowOf>>,
    critters: Query<(Entity, &Critter, &Sprite, Option<&Shadowed>, Option<&Reflected>), Without<ShadowOf>>,
    nodes: Query<
        (Entity, &GatherNode, &Sprite, Option<&Shake>, Option<&Shadowed>, Option<&Reflected>),
        Without<ShadowOf>,
    >,
    casters: Query<(Entity, &CastsShadow, &Sprite, Option<&Shadowed>, Option<&Reflected>), Without<ShadowOf>>,
    room_actors: Query<(), With<RoomActor>>,
    parents: Query<&ChildOf>,
    mut shadows: Query<
        (Entity, &ShadowOf, &mut Transform, &MeshMaterial2d<ShadowMaterial>, &mut Visibility),
        Without<Reflection>,
    >,
    mut refls: Query<
        (Entity, &Reflection, &mut Transform, &MeshMaterial2d<ReflectionMaterial>, &mut Visibility),
        Without<ShadowOf>,
    >,
    owners_alive: Query<
        (),
        Or<(
            With<Player>,
            With<Villager>,
            With<Goblin>,
            With<Mob>,
            With<Critter>,
            With<GatherNode>,
            With<CastsShadow>,
        )>,
    >,
) {
    let FxCtx { ref images, ref mut materials, ref mut refl_materials, ref quad, ref clock, ref water, ref weather } = fx;

    let sil_of = |spr: &Sprite| -> Option<(Handle<Image>, Vec2)> {
        let size = spr
            .custom_size
            .or_else(|| images.get(&spr.image).map(|i| i.size().as_vec2()))?;
        Some((spr.image.clone(), size))
    };

    // --- Gather this frame's anchors (owner, anchor, shadow entity, reflection entity). ---
    let mut anchors: Vec<(Entity, Anchor, Option<Entity>, Option<Entity>)> = Vec::new();
    for (e, p, h, spr, sh, rf) in &players {
        let mut a = person_anchor(p.x, p.y, h.hp <= 0);
        a.sil = sil_of(spr);
        anchors.push((e, a, sh.map(|s| s.0), rf.map(|r| r.0)));
    }
    for (e, v, spr, sh, rf) in &villagers {
        let mut a = person_anchor(v.x, v.y, false);
        a.sil = sil_of(spr);
        anchors.push((e, a, sh.map(|s| s.0), rf.map(|r| r.0)));
    }
    for (e, g, h, spr, sh, rf) in &goblins {
        let mut a = person_anchor(g.x, g.y, h.hp <= 0);
        a.sil = sil_of(spr);
        anchors.push((e, a, sh.map(|s| s.0), rf.map(|r| r.0)));
    }
    for (e, m, h, spr, sh, rf) in &mobs {
        let d = &MOB_DEFS[m.def];
        // The hitbox is the FEET box: centre on it; fliers hover, so fainter.
        let w = if m.small { 8 } else { (d.hb.2 as u32 + 4).clamp(10, 16) & !1 };
        let w = if d.fly { (w.saturating_sub(4)).max(6) & !1 } else { w };
        let cx = (m.x + d.hb.0 + d.hb.2 / 2.0).round();
        let top = (m.y + d.hb.1 + d.hb.3).round() - 3.0;
        let a = if d.fly { 0.55 } else { 1.0 };
        anchors.push((
            e,
            Anchor {
                left: cx - (w / 2) as f32,
                top,
                w,
                a,
                hide: h.hp <= 0 && !m.downed,
                sil: sil_of(spr),
            },
            sh.map(|s| s.0),
            rf.map(|r| r.0),
        ));
    }
    for (e, c, spr, sh, rf) in &critters {
        // Critters own their rect (per-kind sprite sizes live there).
        let (left, top, w, _h, a) = c.shadow_rect();
        anchors.push((
            e,
            Anchor { left, top, w, a, hide: false, sil: sil_of(spr) },
            sh.map(|s| s.0),
            rf.map(|r| r.0),
        ));
    }
    // Props too (Baz: "it looks weird that they are on the players and not those") —
    // trees, bushes, boulders and their variants: the canopy silhouettes lean with
    // the sun, and lakeside trees mirror in the water.
    for (e, n, spr, shake, sh, rf) in &nodes {
        let (fx_, fy) = ((n.c * 16) as f32, (n.r * 16) as f32);
        let (w, top) = if n.tree { (14u32, fy + 12.0) } else { (12u32, fy + 11.0) };
        // Axe-hit wobble: the shadow swings with the trunk (same formula apply_shake
        // runs on the sprite, so the two stay pixel-locked).
        let wob = shake.map(|s| ((s.t as f32) * 1.7).sin().round() * 2.0).unwrap_or(0.0);
        anchors.push((
            e,
            Anchor {
                left: fx_ + ((16 - w as i32) / 2) as f32 + wob,
                top,
                w,
                a: 0.9,
                hide: false,
                sil: sil_of(spr),
            },
            sh.map(|s| s.0),
            rf.map(|r| r.0),
        ));
    }
    // Static opt-ins (growth-stage stumps/saplings) carry their own anchor.
    for (e, c, spr, sh, rf) in &casters {
        anchors.push((
            e,
            Anchor { left: c.left, top: c.top, w: c.w, a: c.a, hide: false, sil: sil_of(spr) },
            sh.map(|s| s.0),
            rf.map(|r| r.0),
        ));
    }

    // --- Attach: a shadow + a reflection for every newly seen owner (each rides the
    // RoomActor sweep iff its owner does — the player's cross rooms with him). ---
    for (owner, anchor, shadow, refl) in &anchors {
        let owned_by_room = room_actors.get(*owner).is_ok();
        // Root children (props, villagers) SCROLL with their room during an edge
        // slide — their quads must ride the same parent or they sit parked while
        // the owner slides in (the old tree transition bug, re-earned by shadows).
        let parent = parents.get(*owner).ok().map(|p| p.parent());
        let tex = anchor.sil.as_ref().map(|(h, _)| h.clone()).unwrap_or_default();
        if shadow.is_none() {
            let mat = materials.add(ShadowMaterial {
                texture: tex.clone(),
                mask: water.image.clone(),
                params: ShadowParams::default(),
            });
            let mut ec = commands.spawn((
                ShadowOf(*owner),
                Mesh2d(quad.0.clone()),
                MeshMaterial2d(mat),
                at(PLAY_X + anchor.left, PLAY_Y + anchor.top, anchor.w as f32, 4.0, layers::SHADOW),
                PIXEL_LAYER,
            ));
            if owned_by_room {
                ec.insert(RoomActor);
            }
            if let Some(p) = parent {
                ec.insert(ChildOf(p));
            }
            let se = ec.id();
            // try_insert: the owner can die THIS frame (a room swap despawning its root
            // between the anchor pass and command apply) — the orphan quad reaps next tick.
            commands.entity(*owner).try_insert(Shadowed(se));
        }
        if refl.is_none() {
            // The mirror in the water (hidden until it overlaps the mask).
            let mat = refl_materials.add(ReflectionMaterial {
                texture: tex,
                mask: water.image.clone(),
                params: ReflectionParams { rect: Vec4::ZERO, time: 0.0, opacity: 0.38, ripple: 0.9, _pad: 0.0 },
            });
            let mut ec = commands.spawn((
                Reflection(*owner),
                Mesh2d(quad.0.clone()),
                MeshMaterial2d(mat),
                at(PLAY_X + anchor.left, PLAY_Y + anchor.top, 1.0, 1.0, layers::REFLECTION),
                PIXEL_LAYER,
                Visibility::Hidden,
            ));
            if owned_by_room {
                ec.insert(RoomActor);
            }
            if let Some(p) = parent {
                ec.insert(ChildOf(p));
            }
            let re = ec.id();
            commands.entity(*owner).try_insert(Reflected(re)); // same same-frame-death tolerance
        }
    }

    // The sun, shared by every shadow this frame.
    let t = (clock.0.rem_euclid(super::gather::DAY_LEN)) as f32 / super::gather::DAY_LEN as f32;
    let elev = (t * std::f32::consts::TAU).cos(); // 1 noon .. -1 midnight
    let stretch = 0.45 + 0.3 * (1.0 - elev.max(0.0)); // noon tight, low sun long
    let shear = (t * std::f32::consts::TAU).sin() * 0.3; // west at dawn, east at dusk
    // The sun's strength IS the shadow's: full-dark at noon, long and fading through
    // dusk to nothing, then a faint moon shadow rising for the small hours, and back
    // with the dawn. powf(0.6) keeps daytime shadows solid until the sun is low.
    // Darker than before (Baz: shadows were too faint to read) — sun 0.38 -> 0.55, moon
    // 0.10 -> 0.18, and clouds soften a little less (0.65 -> 0.5) so overcast doesn't wash
    // them out entirely.
    let sun_a = 0.55 * elev.max(0.0).powf(0.6);
    let moon_a = 0.18 * (-elev).max(0.0).powf(0.6);
    // Clouds hide the sun (PORT-ORIGINAL tie-in): overcast/storm skies soften every
    // shadow toward nothing — one multiply, priced in by the shader stack.
    let day_a = (sun_a + moon_a) * (1.0 - 0.5 * weather.cloud());
    let rtime = clock.0 as f32 / 60.0;

    // --- Shadows: follow + reap. ---
    for (se, shadow, mut tf, mat, mut vis) in &mut shadows {
        let Some((_, anchor, ..)) = anchors.iter().find(|(o, ..)| *o == shadow.0) else {
            if owners_alive.get(shadow.0).is_err() {
                commands.entity(se).despawn();
            }
            continue;
        };
        let Some((img, size)) = &anchor.sil else { continue };
        let (sw, sh) = (size.x, (size.y * stretch).round().max(2.0));
        let quad_w = sw * CAST_MARGIN; // shear headroom (shadow.wgsl maps it back)
        let centre = anchor.left + anchor.w as f32 / 2.0;
        // Feet contact: the quad's top edge tucks 2px up into the boots/base (the
        // owner draws over the seam — shadow band < actor band). The same room-px
        // rect goes to the shader, which clips fragments landing on water.
        let rect = Vec4::new((centre - quad_w / 2.0).round(), anchor.top - 2.0, quad_w, sh);
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.texture = img.clone();
            m.mask = water.image.clone();
            m.params = ShadowParams { rect, shear, blur: 1.1, opacity: day_a * anchor.a, flip_x: 0.0 };
        }
        let mut t = at(PLAY_X + rect.x, PLAY_Y + rect.y, quad_w, sh, layers::SHADOW);
        t.scale = Vec3::new(quad_w, sh, 1.0); // the unit quad takes its size from scale
        *tf = t;
        *vis = if anchor.hide { Visibility::Hidden } else { Visibility::Inherited };
    }

    // --- Reflections: follow + reap (the water pass). Hidden in dry rooms; on water
    // they mirror the live art below the feet, rippled + clipped by the shader. ---
    for (re, refl, mut tf, mat, mut vis) in &mut refls {
        let Some((_, anchor, ..)) = anchors.iter().find(|(o, ..)| *o == refl.0) else {
            if owners_alive.get(refl.0).is_err() {
                commands.entity(re).despawn();
            }
            continue;
        };
        let Some((img, size)) = &anchor.sil else { continue };
        if !water.any || anchor.hide {
            *vis = Visibility::Hidden;
            continue;
        }
        let (rw, rh) = (size.x, (size.y * 0.9).round().max(2.0));
        let centre = anchor.left + anchor.w as f32 / 2.0;
        // Room-pixel rect (feet contact tucks 2px up, like the shadow).
        let rect = Vec4::new((centre - rw / 2.0).round(), anchor.top - 2.0, rw, rh);
        if let Some(mut m) = refl_materials.get_mut(&mat.0) {
            m.texture = img.clone();
            m.mask = water.image.clone();
            m.params = ReflectionParams { rect, time: rtime, opacity: 0.38 * anchor.a, ripple: 0.9, _pad: 0.0 };
        }
        let mut t = at(PLAY_X + rect.x, PLAY_Y + rect.y, rw, rh, layers::REFLECTION);
        t.scale = Vec3::new(rw, rh, 1.0);
        *tf = t;
        *vis = Visibility::Inherited;
    }
}
