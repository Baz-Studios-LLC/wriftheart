//! hall_exterior.rs — the city guildhall's town-side face (js Entities.guildhall):
//! a grand stone hall with a gabled slate roof, drawn at its restoration stage
//! (wings done, 0-5): boarded -> scaffolding (2-3) -> repaired stone (4) -> lit,
//! pennant flying (5). Every restored guild hangs its crest pennant over the door.
//! The js painted this per frame with canvas fills; here the whole face bakes once
//! per (stage, crests) into a static image — DEVIATION (flagged): the pennant sway,
//! window-glow pulse and doorway spill are frozen at their mid-phase.
//! Stood up by hall_wake (the yard_wake idiom — no room_props threading), which
//! also lays the js-verbatim body blocker and the plaque lettering.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::battle::RoomActor;
use super::play::CurRoom;
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::ui::label;

/// Everything hall_wake stands up (swept as one on room change).
#[derive(Component)]
pub struct HallFace;

/// The baked face: image coords run (dx + OX, dy + OY) for js-relative (dx, dy).
const W: i32 = 112; // js building width
const OX: i32 = 6; //  left overhang (scaffold pole at -6)
const OY: i32 = 66; // headroom (pennant tip at -66)
const IMG_W: u32 = 124;
const IMG_H: u32 = 88;

struct Painter(Image);

impl Painter {
    fn new() -> Self {
        Self(Image::new_fill(
            Extent3d { width: IMG_W, height: IMG_H, depth_or_array_layers: 1 },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        ))
    }

    /// Blend one js-relative pixel (alpha 1.0 = overwrite, else source-over).
    fn put(&mut self, dx: i32, dy: i32, hex: u32, a: f32) {
        let (x, y) = (dx + OX, dy + OY);
        if x < 0 || y < 0 || x >= IMG_W as i32 || y >= IMG_H as i32 {
            return;
        }
        let Ok(px) = self.0.pixel_bytes_mut(UVec3::new(x as u32, y as u32, 0)) else { return };
        let src = [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8];
        if a >= 1.0 || px[3] == 0 {
            let out = if a >= 1.0 { 255 } else { (a * 255.0) as u8 };
            px.copy_from_slice(&[src[0], src[1], src[2], if px[3] == 0 { out } else { 255 }]);
            return;
        }
        for i in 0..3 {
            px[i] = (src[i] as f32 * a + px[i] as f32 * (1.0 - a)) as u8;
        }
    }

    fn rect(&mut self, dx: i32, dy: i32, w: i32, h: i32, hex: u32) {
        self.rect_a(dx, dy, w, h, hex, 1.0);
    }

    fn rect_a(&mut self, dx: i32, dy: i32, w: i32, h: i32, hex: u32, a: f32) {
        for y in dy..dy + h {
            for x in dx..dx + w {
                self.put(x, y, hex, a);
            }
        }
    }

    /// Isoceles gable: apex (ax, ay) down to base y `by` spanning [bl, br].
    fn gable(&mut self, ax: i32, ay: i32, by: i32, bl: i32, br: i32, hex: u32) {
        for y in ay..=by {
            let p = (y - ay) as f32 / (by - ay) as f32;
            let l = ax as f32 + (bl - ax) as f32 * p;
            let r = ax as f32 + (br - ax) as f32 * p;
            for x in (l.ceil() as i32)..=(r.floor() as i32) {
                self.put(x, y, hex, 1.0);
            }
        }
    }
}

/// Bake the hall face at a restoration stage — the js draw, fill for fill.
pub fn bake_hall(stage: usize, crests: &[u32]) -> Image {
    let st = stage as i32;
    let (fixed, whole) = (st >= 4, st >= 5);
    let mut p = Painter::new();
    // Gabled slate roof (brighter, whole slates once repaired).
    p.gable(W / 2, -56, -30, -4, W + 4, if fixed { 0x3a4250 } else { 0x32363e });
    p.gable(W / 2, -52, -30, 2, W - 2, if fixed { 0x525c6e } else { 0x464b56 });
    if st >= 2 && !fixed {
        // Fresh slate patches mid-repair.
        p.rect(28, -44, 12, 6, 0x5a6478);
        p.rect(66, -40, 14, 6, 0x5a6478);
    }
    if whole {
        // The rooftop pennant flies again (sway frozen mid-wave).
        p.rect(W / 2 - 1, -66, 2, 14, 0x8a6a3a);
        for y in -66..=-60i32 {
            let t = (y + 63).abs() as f32 / 3.0;
            let r = 57.0 + 10.0 * (1.0 - t);
            for x in 57..=(r as i32) {
                p.put(x, y, 0xe8c860, 1.0);
            }
        }
    }
    // Stone body: weathered grey -> clean warm stone; shadow courses top + skirt.
    p.rect(0, -30, W, 46, if fixed { 0x7e7e88 } else { 0x6a6a74 });
    let course = if fixed { 0x6e6e78 } else { 0x5a5a64 };
    p.rect(0, -30, W, 4, course);
    p.rect(0, 8, W, 8, course);
    if !fixed {
        // Worn blocks fade with the repair.
        for i in 0..6 {
            p.rect(6 + i * 18, -22 + (i % 2) * 14, 10, 3, 0x7a7a84);
        }
    }
    // Flanking windows: boarded -> opened dark -> aglow when the hall lives again.
    for wx in [14, W - 26] {
        p.rect(wx, -18, 12, 14, 0x1a1a20);
        if whole {
            p.rect_a(wx + 1, -17, 10, 12, 0xffd27a, 0.78); // glow frozen mid-pulse
            p.rect(wx + 5, -17, 1, 12, 0x3a2c14);
            p.rect(wx + 1, -12, 10, 1, 0x3a2c14);
        } else if st >= 2 {
            p.rect(wx + 5, -17, 1, 12, 0x2c2c38); // opened, dark inside
            p.rect(wx + 1, -12, 10, 1, 0x2c2c38);
        } else {
            p.rect(wx - 1, -15, 14, 3, 0x6a4a2a); // boarded shut
            p.rect(wx - 1, -9, 14, 3, 0x6a4a2a);
        }
    }
    // Grand double door: planked over until the first guild returns.
    let dx2 = W / 2 - 12;
    p.rect(dx2, -12, 24, 28, 0x241a10);
    p.rect(dx2 + 2, -10, 20, 26, if fixed { 0x5a4226 } else { 0x4a3620 });
    if fixed {
        // Proper double doors + ring pulls.
        p.rect(dx2 + 11, -10, 2, 26, 0x2e2010);
        p.rect(dx2 + 7, 2, 2, 2, 0xc8a04a);
        p.rect(dx2 + 15, 2, 2, 2, 0xc8a04a);
    }
    if st < 1 {
        p.rect(dx2 - 2, -6, 28, 4, 0x8a6a3a); // cross boards
        p.rect(dx2 - 2, 4, 28, 4, 0x8a6a3a);
    }
    if whole {
        p.rect_a(dx2 - 3, -13, 30, 32, 0xffd27a, 0.14); // warm spill from the doorway
    }
    // Scaffolding while the works are up (stages 2-3).
    if st >= 2 && !fixed {
        for (x, y, w, h) in [(-6, -34, 2, 48), (20, -34, 2, 48), (-6, -26, 28, 2), (-6, -8, 28, 2)] {
            p.rect(x, y, w, h, 0x7a5c30);
        }
        if st >= 3 {
            for (x, y, w, h) in [(W - 22, -34, 2, 48), (W + 4, -34, 2, 48), (W - 22, -20, 28, 2)] {
                p.rect(x, y, w, h, 0x7a5c30);
            }
        }
    }
    // Steps.
    p.rect(dx2 - 4, 16, 32, 3, if fixed { 0x6e6e78 } else { 0x5a5a64 });
    p.rect(dx2 - 6, 19, 36, 3, if fixed { 0x5e5e68 } else { 0x4a4a54 });
    // The plaque band (lettering rides as a label so the shared font draws it).
    let nw = font::measure("GUILDHALL");
    p.rect((W - nw) / 2 - 3, -28, nw + 6, 9, 0x2a2a32);
    // Banner poles by the door; every restored guild hangs its crest on the line.
    let pole = if whole { 0x5a4226 } else { 0x3a3228 };
    p.rect(dx2 - 10, -20, 2, 36, pole);
    p.rect(dx2 + 32, -20, 2, 36, pole);
    if !crests.is_empty() {
        for x in (dx2 - 9)..=(dx2 + 33) {
            let t = (x - (dx2 - 9)) as f32 / 42.0;
            let y = -19.0 + 4.0 * t * (1.0 - t) * 2.0; // the sagging line
            p.put(x, y.round() as i32, 0xd8ccb0, 1.0);
        }
        for (i, &crest) in crests.iter().take(5).enumerate() {
            let bx = dx2 - 6 + i as i32 * 7;
            p.rect(bx, -18, 5, 5, crest);
            for y in -13..=-10i32 {
                let t = (y + 13) as f32 / 3.0;
                let l = bx as f32 + 2.5 * t;
                let r = bx as f32 + 5.0 - 2.5 * t;
                for x in (l.ceil() as i32)..=(r.floor() as i32) {
                    p.put(x, y, crest, 1.0); // swallowtail tip
                }
            }
            p.put(bx + 2, -17, 0xf4f0e4, 1.0); // pale emblem dot
        }
    }
    p.0
}

/// Stand the hall face up when a room holding one arrives (the yard_wake idiom:
/// sweep-and-restand keyed on the room; blockers lay the js-verbatim body box).
#[allow(clippy::too_many_arguments)]
pub fn hall_wake(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cur: Res<CurRoom>,
    slide: Res<super::play::SlideState>,
    world: Res<super::play::GameWorld>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    ledger: Res<super::guildhall::GuildLedger>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut woke: Local<Option<(i32, i32)>>,
    face: Query<Entity, With<HallFace>>,
    parents: Query<&ChildOf>,
) {
    if in_dungeon.0.is_some() || inside.0.is_some() {
        *woke = None; // interiors re-stand the face on the way back out
        return;
    }
    // MID-SLIDE the hall stands up EARLY for the incoming room (cur already names
    // it) and adopt_room_cast puts it on the sliding new root, so it rides in with
    // its streets (Baz: it popped at settle). The sweep spares the OUTGOING hall —
    // a child of the departing root, leaving with it.
    if *woke == Some((cur.rx, cur.ry)) && !ledger.is_changed() {
        return;
    }
    *woke = Some((cur.rx, cur.ry));
    let outgoing = slide.outgoing_root();
    for e in &face {
        if outgoing.is_some() && parents.get(e).ok().map(|p| p.parent()) == outgoing {
            continue;
        }
        commands.entity(e).despawn();
    }
    let Some(e) = world.0.room_entities(cur.rx, cur.ry).into_iter().find(|e| e.kind == "guildhall") else {
        return;
    };
    let (fx, fy) = (e.x as f32, e.y as f32);
    // Restoration stage + crest string for THIS city (js game.js:1046).
    let done = super::guildhall::city_key(&world.0, cur.rx, cur.ry)
        .and_then(|k| ledger.0.get(&k).map(|g| g.done.clone()))
        .unwrap_or_default();
    let crests: Vec<u32> =
        crate::guildhall::WINGS.iter().filter(|w| done.iter().any(|d| d == w.id)).map(|w| w.crest).collect();
    let img = images.add(bake_hall(crests.len(), &crests));
    let z = actor_z(fy + 16.0); // js depthSort anchor
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + fx - OX as f32, PLAY_Y + fy - OY as f32, IMG_W as f32, IMG_H as f32, z),
        PIXEL_LAYER,
        RoomActor, // the actor sweep clears the face on room/hall swaps
        HallFace,
    ));
    // The plaque (gold once the first guild is home).
    let nw = font::measure("GUILDHALL") as f32;
    let lx = PLAY_X + fx + ((W as f32 - nw) / 2.0).round();
    let plaque = label(
        &mut commands,
        &mut images,
        "GUILDHALL",
        lx,
        PLAY_Y + fy - 26.0,
        if crests.is_empty() { 0x8a8060 } else { 0xe8c860 },
        z + 0.01,
        (RoomActor, HallFace),
    );
    let _ = plaque;
    let blk = (fx + 2.0, fy - 10.0, 108.0, 26.0); // js hitbox
    if !blockers.0.contains(&blk) {
        blockers.0.push(blk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every stage bakes inside the canvas (a stray fill would silently clip).
    #[test]
    fn stages_bake() {
        for st in 0..=5usize {
            let crests: Vec<u32> = crate::guildhall::WINGS.iter().take(st).map(|w| w.crest).collect();
            let img = bake_hall(st, &crests);
            assert_eq!(img.size(), bevy::math::UVec2::new(IMG_W, IMG_H));
        }
    }
}
