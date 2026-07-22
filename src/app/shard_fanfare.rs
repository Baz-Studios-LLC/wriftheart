//! shard_fanfare.rs — THE SHARD RITE: claiming a piece of the Wriftheart is the
//! game's central act, and it gets its own cutscene, a tier above the item-get
//! fanfare. The world freezes and darkens; the shard rises from where it lay on a
//! pillar of light, the room pulses to a HEARTBEAT (rings ripple out on every
//! beat), motes stream upward across the whole room, and a typewriter banner
//! declares THE <SHARD> IS YOURS with the running count beneath. ~4.5s, skippable
//! once the banner is up. Rebuild-every-frame draw, same pattern as fanfare.rs.

use crate::gfx::{at, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use crate::room::{PX_H, PX_W};
use super::room_render::{PLAY_X, PLAY_Y};
use bevy::prelude::*;
use std::f32::consts::TAU;

/// ~4.5s at 60fps.
const DUR: u32 = 270;
/// One heartbeat cycle: lub-dub, then rest.
const BEAT: f32 = 60.0;
/// Same celebration band as the item fanfare (above weather 13.2, below banners
/// 15.5) — only one of the two can ever run at once.
const Z: f32 = 13.5;

pub struct ShardRite {
    pub name: String,
    pub col: u32,
    pub have: usize,
    pub goal: usize,
    /// Room coords of the shard where it lay (the rite grows out of that spot).
    pub x: f32,
    pub y: f32,
    t: u32,
}

#[derive(Resource, Default)]
pub struct ShardFanfare(pub Option<ShardRite>);

impl ShardFanfare {
    pub fn begin(&mut self, name: String, col: u32, have: usize, goal: usize, x: f32, y: f32) {
        if self.0.is_none() {
            self.0 = Some(ShardRite { name, col, have, goal, x, y, t: 0 });
        }
    }
}

#[derive(Component)]
struct ShardRiteUi;

pub struct ShardFanfarePlugin;
impl Plugin for ShardFanfarePlugin {
    fn build(&self, app: &mut App) {
        // Outside `playing` — the rite OWNS the freeze (folded into screen::playing).
        app.init_resource::<ShardFanfare>().add_systems(Update, (rite_tick, rite_draw).chain());
    }
}

/// The two-bump lub-dub curve for heartbeat phase `ph` (0..BEAT).
fn beat_curve(ph: f32) -> f32 {
    let bump = |c: f32| (1.0 - (ph - c).abs() / 7.0).max(0.0);
    bump(0.0).max(bump(11.0) * 0.65)
}

fn rite_tick(mut rite: ResMut<ShardFanfare>, state: Res<ActionState>, mut sfx: MessageWriter<super::sfx::Sfx>) {
    let Some(fx) = &mut rite.0 else { return };
    fx.t += 1;
    match fx.t {
        1 => {
            sfx.write(super::sfx::Sfx("heartbeat")); // the wound answers first
        }
        35 => {
            sfx.write(super::sfx::Sfx("shardget")); // the rite's own jingle, as the shard crests
        }
        t if t > 35 && (t - 1) % BEAT as u32 == 0 => {
            sfx.write(super::sfx::Sfx("heartbeat")); // the pulse carries on under the shimmer
        }
        _ => {}
    }
    let skip = fx.t > 70
        && (state.pressed(Action::Interact)
            || state.pressed(Action::MenuConfirm)
            || state.pressed(Action::Slot2)
            || state.pressed(Action::Inventory));
    if fx.t >= DUR || skip {
        rite.0 = None;
    }
}

/// Radial glow disc (squared falloff — soft core, no wide bright wash).
fn bake_glow(images: &mut Assets<Image>, r: f32) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let d = (r * 2.0) as u32;
    let mut img = Image::new_fill(
        Extent3d { width: d, height: d, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let c = r - 0.5;
    for y in 0..d {
        for x in 0..d {
            let dist = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
            let a = (1.0 - dist / r).max(0.0).powi(2);
            if let Ok(px) = img.pixel_bytes_mut(UVec3::new(x, y, 0)) {
                px.copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
            }
        }
    }
    images.add(img)
}

/// A thin soft-edged circle outline — the heartbeat shockwave (scaled up as it runs).
fn bake_ring(images: &mut Assets<Image>) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const D: u32 = 64;
    let mut img = Image::new_fill(
        Extent3d { width: D, height: D, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let c = D as f32 / 2.0 - 0.5;
    for y in 0..D {
        for x in 0..D {
            let dist = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
            let a = (1.0 - (dist - 28.0).abs() / 2.5).max(0.0);
            if let Ok(px) = img.pixel_bytes_mut(UVec3::new(x, y, 0)) {
                px.copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
            }
        }
    }
    images.add(img)
}

/// The pillar of light: soft horizontal falloff, brightest at the shard end
/// (drawn bottom), thinning toward the sky.
fn bake_pillar(images: &mut Assets<Image>) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const W: u32 = 16;
    const H: u32 = 96;
    let mut img = Image::new_fill(
        Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    for y in 0..H {
        let vf = 0.30 + 0.70 * (y as f32 / H as f32); // top dim -> bottom bright
        for x in 0..W {
            let hf = (1.0 - ((x as f32 - 7.5).abs() / 8.0)).max(0.0).powi(2);
            if let Ok(px) = img.pixel_bytes_mut(UVec3::new(x, y, 0)) {
                px.copy_from_slice(&[255, 255, 255, (hf * vf * 255.0) as u8]);
            }
        }
    }
    images.add(img)
}

fn rgb(col: u32) -> Color {
    Color::srgb_u8((col >> 16) as u8, (col >> 8) as u8, col as u8)
}

fn lighten(col: u32, f: f32) -> u32 {
    let ch = |i: u32| ((((col >> i) & 255) as f32 * f).min(255.0)) as u32;
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

#[allow(clippy::too_many_arguments)]
fn rite_draw(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    rite: Res<ShardFanfare>,
    old: Query<Entity, With<ShardRiteUi>>,
    mut glow_img: Local<Option<Handle<Image>>>,
    mut ring_img: Local<Option<Handle<Image>>>,
    mut pillar_img: Local<Option<Handle<Image>>>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(fx) = &rite.0 else { return };
    let t = fx.t as f32;
    // Master fade: everything eases in over the first half second and back out over
    // the last 24 frames, so the freeze never pops.
    let m = (t / 30.0).min(1.0) * ((DUR as f32 - t) / 24.0).clamp(0.0, 1.0);
    let ph = (t - 1.0).rem_euclid(BEAT);
    let beat = beat_curve(ph);
    let col = rgb(fx.col);
    let col_lit = rgb(lighten(fx.col, 1.6));

    // The dark falls — deeper than the item fanfare; this is a rite, not a pickup.
    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.72 * m), Vec2::new(PX_W as f32, PX_H as f32)),
        at(PLAY_X, PLAY_Y, PX_W as f32, PX_H as f32, Z),
        PIXEL_LAYER,
        ShardRiteUi,
    ));

    // The claim flash — one hard white frame decaying fast.
    if t < 14.0 {
        commands.spawn((
            Sprite::from_color(Color::srgba(1.0, 1.0, 1.0, 0.85 * (1.0 - t / 14.0)), Vec2::new(PX_W as f32, PX_H as f32)),
            at(PLAY_X, PLAY_Y, PX_W as f32, PX_H as f32, Z + 0.3),
            PIXEL_LAYER,
            ShardRiteUi,
        ));
    }

    // The shard rises from where it lay, growing as it goes, then hovers on a bob.
    let rise = ((t - 6.0) / 45.0).clamp(0.0, 1.0);
    let rise = rise * rise * (3.0 - 2.0 * rise); // smoothstep
    let size = 8.0 + 16.0 * rise;
    let sx = PLAY_X + fx.x + 8.0;
    let sy = PLAY_Y + fx.y + 8.0 - rise * 34.0 + if rise >= 1.0 { (t / 9.0).sin() * 1.5 } else { 0.0 };

    // The pillar of light from the shard up out of the world.
    let pillar = pillar_img.get_or_insert_with(|| bake_pillar(&mut images)).clone();
    let ph_h = (sy - PLAY_Y).max(1.0);
    let pw = 10.0 + 6.0 * beat;
    let mut ps = Sprite::from_image(pillar);
    ps.custom_size = Some(Vec2::new(pw, ph_h));
    ps.color = col_lit.with_alpha(0.34 * m * (0.7 + 0.3 * beat) * rise);
    commands.spawn((ps, at(sx - pw / 2.0, PLAY_Y, pw, ph_h, Z + 0.04), PIXEL_LAYER, ShardRiteUi));

    // Rotating rays fanning from the shard (long/short alternating spokes).
    for k in 0..10 {
        let a = t * 0.006 + k as f32 * (TAU / 10.0);
        let len = if k % 2 == 0 { 72.0 } else { 46.0 };
        let cx = sx + a.cos() * len * 0.5;
        let cy = sy + a.sin() * len * 0.5;
        let mut tr = at(cx - len / 2.0, cy - 1.0, len, 2.0, Z + 0.05);
        tr.rotation = Quat::from_rotation_z(-a); // canvas y flips in at()
        let mut rs = Sprite::from_color(col_lit.with_alpha((0.10 + 0.10 * beat) * m * rise), Vec2::new(len, 2.0));
        rs.custom_size = Some(Vec2::new(len, 2.0));
        commands.spawn((rs, tr, PIXEL_LAYER, ShardRiteUi));
    }

    // The heartbeat shockwave — a ring swells out of the shard on every beat.
    if ph < 26.0 && t > 6.0 {
        let ring = ring_img.get_or_insert_with(|| bake_ring(&mut images)).clone();
        let d = 20.0 + ph * 7.0;
        let mut rs = Sprite::from_image(ring);
        rs.custom_size = Some(Vec2::splat(d));
        rs.color = col_lit.with_alpha(0.5 * (1.0 - ph / 26.0) * m);
        commands.spawn((rs, at(sx - d / 2.0, sy - d / 2.0, d, d, Z + 0.06), PIXEL_LAYER, ShardRiteUi));
    }

    // The glow behind the shard, breathing with the pulse.
    let gr = 40.0;
    let glow = glow_img.get_or_insert_with(|| bake_glow(&mut images, gr)).clone();
    let gd = gr * 2.0 * (0.8 + 0.25 * beat) * (0.4 + 0.6 * rise);
    let mut gs = Sprite::from_image(glow);
    gs.custom_size = Some(Vec2::splat(gd));
    gs.color = col.with_alpha(0.44 * m);
    commands.spawn((gs, at(sx - gd / 2.0, sy - gd / 2.0, gd, gd, Z + 0.07), PIXEL_LAYER, ShardRiteUi));

    // The shard itself.
    let img = super::dungeon::shard_image(&mut images, fx.col);
    let mut is = Sprite::from_image(img);
    is.custom_size = Some(Vec2::splat(size));
    commands.spawn((is, at(sx - size / 2.0, sy - size / 2.0, size, size, Z + 0.12), PIXEL_LAYER, ShardRiteUi));

    // Motes stream upward across the WHOLE room — the land itself answering.
    for i in 0u32..40 {
        let h = i.wrapping_mul(2654435761);
        let mx = PLAY_X + ((h >> 8) % PX_W as u32) as f32 + (t / 22.0 + i as f32).sin() * 4.0;
        let speed = 0.35 + ((h >> 16) % 100) as f32 / 160.0;
        let total = PX_H as f32 + 24.0;
        let yy = (((h >> 4) % PX_H as u32) as f32 + t * speed).rem_euclid(total);
        let my = PLAY_Y + PX_H as f32 - yy;
        let a = (yy / 28.0).min(1.0) * ((total - yy) / 40.0).min(1.0) * 0.8 * m;
        let sz = if i % 3 == 0 { 2.0 } else { 1.0 };
        let mc = if i % 2 == 0 { Color::WHITE } else { col_lit };
        commands.spawn((
            Sprite::from_color(mc.with_alpha(a), Vec2::splat(sz)),
            at(mx.round(), my.round(), sz, sz, Z + 0.08),
            PIXEL_LAYER,
            ShardRiteUi,
        ));
    }

    // Twinkling sparks orbiting the shard on two rings.
    for k in 0..12 {
        let (r0, w) = if k < 7 { (15.0, 1.0) } else { (25.0, -0.7) };
        let a = t / 16.0 * w + k as f32 * (TAU / if k < 7 { 7.0 } else { 5.0 });
        let tw = ((t / 6.0).floor() as i32 + k) % 2 == 0;
        let r2 = r0 + beat * 3.0 + if tw { 2.0 } else { 0.0 };
        let px = (sx + a.cos() * r2).round();
        let py = (sy + a.sin() * r2).round();
        let sz = if tw { 2.0 } else { 1.0 };
        let c = if tw { Color::WHITE } else { col_lit };
        commands.spawn((
            Sprite::from_color(c.with_alpha(m), Vec2::splat(sz)),
            at(px, py, sz, sz, Z + 0.14),
            PIXEL_LAYER,
            ShardRiteUi,
        ));
    }

    // The banner: THE <SHARD> IS YOURS, revealed letter by letter, count beneath.
    if t >= 70.0 {
        let msg = format!("THE {} IS YOURS", fx.name.to_uppercase());
        let shown = (((t - 70.0) / 2.0) as usize).min(msg.len());
        let (_, full_w) = crate::gfx::font::bake_text(&msg, fx.col, images.as_mut());
        let bw = full_w as f32 * 2.0 + 18.0;
        let bh = 30.0;
        let bx = PLAY_X + ((PX_W as f32 - bw) / 2.0).floor();
        let by = PLAY_Y + 20.0;
        commands.spawn((
            Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.78), Vec2::new(bw, bh)),
            at(bx, by, bw, bh, Z + 0.2),
            PIXEL_LAYER,
            ShardRiteUi,
        ));
        for (sx4, sy4, sw4, sh4) in crate::ui::border_strips(bx, by, bw, bh, 1.0) {
            commands.spawn((Sprite::from_color(col, Vec2::new(sw4, sh4)), at(sx4, sy4, sw4, sh4, Z + 0.22), PIXEL_LAYER, ShardRiteUi));
        }
        if shown > 0 {
            let (timg, tw) = crate::gfx::font::bake_text(&msg[..shown], lighten(fx.col, 1.4), images.as_mut());
            let iw = (tw + (tw & 1)) as f32 * 2.0;
            let mut ts = Sprite::from_image(timg);
            ts.custom_size = Some(Vec2::new(iw, 12.0));
            // Grows from the same left edge the full line will fill — a typewriter.
            let tx = (bx + (bw - full_w as f32 * 2.0) / 2.0).floor();
            commands.spawn((ts, at(tx, (by + 5.0).floor(), iw, 12.0, Z + 0.24), PIXEL_LAYER, ShardRiteUi));
        }
        if t >= 110.0 {
            let count = format!("{} OF {} SHARDS", fx.have, fx.goal);
            let (cimg, cw) = crate::gfx::font::bake_text(&count, 0xe0b8ff, images.as_mut());
            let iw = (cw + (cw & 1)) as f32;
            commands.spawn((
                Sprite::from_image(cimg),
                at((bx + (bw - cw as f32) / 2.0).floor(), (by + 20.0).floor(), iw, 6.0, Z + 0.24),
                PIXEL_LAYER,
                ShardRiteUi,
            ));
        }
    }
}
