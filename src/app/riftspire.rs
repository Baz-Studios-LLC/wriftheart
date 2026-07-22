//! riftspire.rs — THE RIFT SPIRE, painted exactly as the js drew it (entities.js
//! riftSpire): three beveled tiers under a jagged crown, violet crack seams
//! breathing up the faces, lit slit windows, the MAW arch with its flickering
//! swirl and bright fissure cross, ground cracks at the base, three shards of
//! the world orbiting the crown, and a cold halo bleeding off the whole spire.
//!
//! The js repainted every frame with time-varying alphas; here each animated
//! element is baked ONCE into its own layer and a tick system pulses the layer
//! alphas (slow breath sin(t/26), fast flicker sin(t/7)) and flies the shards.
//! Geometry, colours, and phase offsets are copied from the js line for line.

use bevy::prelude::*;

use super::room_render::{actor_z, FrameClock, PLAY_X, PLAY_Y};
use crate::gfx::{at, PIXEL_LAYER};

/// A pulsing layer of the spire; `t0` is the js per-spire phase ((x*7+y*13)%60).
#[derive(Component)]
pub struct RiftFx {
    pub kind: RiftFxKind,
    pub t0: i64,
}

pub enum RiftFxKind {
    Cracks,  // ground cracks: 0.16 + 0.12p
    Seams,   // face seams:    0.35 + 0.35p
    Windows, // lit slits:     0.50 + 0.40p
    Swirl,   // maw swirl:     0.25 + 0.35fl (the fast flicker)
    Fissure, // bright cross:  0.50 + 0.50p
    Rim,     // maw rim:       0.50 + 0.40p
    Halo,    // cold halo:     js 0.05 + 0.05p, bumped for linear blending
}

/// One of the three shards of the world orbiting the crown, "slow and wrong".
#[derive(Component)]
pub struct RiftShard {
    pub i: usize,
    pub t0: i64,
    /// The spire's entity anchor (room px) — orbits are computed from it.
    pub fx: f32,
    pub fy: f32,
}

// --- A tiny CPU painter (bake-time only) --------------------------------------------

struct Buf {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Buf {
    fn new(w: usize, h: usize) -> Self {
        Buf { w, h, px: vec![0; w * h * 4] }
    }
    fn put(&mut self, x: i32, y: i32, c: u32) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let i = (y as usize * self.w + x as usize) * 4;
        self.px[i] = (c >> 16) as u8;
        self.px[i + 1] = (c >> 8) as u8;
        self.px[i + 2] = c as u8;
        self.px[i + 3] = 255;
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.put(xx, yy, c);
            }
        }
    }
    /// 1px line, Bresenham — the js ctx.stroke() at lineWidth 1.
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
        let (mut x, mut y) = (x0, y0);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.put(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }
    /// Filled polygon (even-odd, pixel centres) — the js ctx.fill() paths.
    fn poly(&mut self, pts: &[(f32, f32)], c: u32) {
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for (x, y) in pts {
            min_x = min_x.min(*x);
            max_x = max_x.max(*x);
            min_y = min_y.min(*y);
            max_y = max_y.max(*y);
        }
        for y in min_y.floor() as i32..=max_y.ceil() as i32 {
            for x in min_x.floor() as i32..=max_x.ceil() as i32 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let mut inside = false;
                let mut j = pts.len() - 1;
                for i in 0..pts.len() {
                    let (xi, yi) = pts[i];
                    let (xj, yj) = pts[j];
                    if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
                        inside = !inside;
                    }
                    j = i;
                }
                if inside {
                    self.put(x, y, c);
                }
            }
        }
    }
    fn circle(&mut self, cx: f32, cy: f32, r: f32, c: u32) {
        for y in (cy - r).floor() as i32..=(cy + r).ceil() as i32 {
            for x in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                if dx * dx + dy * dy <= r * r {
                    self.put(x, y, c);
                }
            }
        }
    }
    fn image(self) -> Image {
        use bevy::asset::RenderAssetUsages;
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        Image::new(
            Extent3d { width: self.w as u32, height: self.h as u32, depth_or_array_layers: 1 },
            TextureDimension::D2,
            self.px,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        )
    }
}

// --- The layers (all on one 72x140 canvas: cx=36, ground line gy=120) ----------------

const CW: usize = 72;
const CH: usize = 140;
const CX: i32 = 36;
const GY: i32 = 120;

/// The static mass: three beveled tiers, the crown, and the maw's dark arch.
fn bake_body() -> Image {
    let mut b = Buf::new(CW, CH);
    // js tier(): dark mass, 2px lite left+top bevel, 2px darker right edge.
    let mut tier = |hw: i32, y0: i32, y1: i32, base: u32, lite: u32| {
        b.rect(CX - hw, y0, hw * 2, y1 - y0, base);
        b.rect(CX - hw, y0, 2, y1 - y0, lite);
        b.rect(CX - hw, y0, hw * 2, 2, lite);
        b.rect(CX + hw - 2, y0, 2, y1 - y0, 0x0c0a12);
    };
    tier(34, GY - 40, GY, 0x16141e, 0x242032);
    tier(25, GY - 72, GY - 40, 0x191622, 0x282436);
    tier(16, GY - 98, GY - 72, 0x16141e, 0x242032);
    // The jagged crown.
    let g = |x: i32, y: i32| ((x + CX) as f32, (y + GY) as f32);
    b.poly(&[g(-16, -98), g(-10, -112), g(-5, -100), g(0, -118), g(5, -100), g(10, -110), g(16, -98)], 0x191622);
    // The MAW: a pointed arch of pure rift.
    b.poly(&[g(-11, 0), g(-11, -18), g(0, -28), g(11, -18), g(11, 0)], 0x0a0812);
    b.image()
}

fn bake_cracks() -> Image {
    let mut b = Buf::new(CW, CH);
    for (x0, y0, x1, y1) in [(-20, 4, -34, 10), (18, 5, 32, 12), (-6, 8, -12, 16), (8, 9, 15, 17)] {
        b.line(CX + x0, GY + y0, CX + x1, GY + y1, 0xa06eff);
    }
    b.image()
}

fn bake_seams() -> Image {
    let mut b = Buf::new(CW, CH);
    let paths: [&[(i32, i32)]; 3] = [
        &[(-18, -4), (-14, -18), (-19, -30), (-15, -44), (-18, -58)],
        &[(16, -10), (20, -26), (15, -38), (18, -54)],
        &[(-4, -76), (-1, -88), (-5, -96)],
    ];
    for path in paths {
        for w in path.windows(2) {
            b.line(CX + w[0].0, GY + w[0].1, CX + w[1].0, GY + w[1].1, 0xc8a0ff);
        }
    }
    b.image()
}

fn bake_windows() -> Image {
    let mut b = Buf::new(CW, CH);
    for (wx, wy) in [(-12, -52), (10, -50), (-8, -84), (6, -86), (-2, -106)] {
        b.rect(CX + wx, GY + wy, 2, 5, 0xb088ff);
    }
    b.image()
}

fn bake_swirl() -> Image {
    let mut b = Buf::new(CW, CH);
    let g = |x: i32, y: i32| ((x + CX) as f32, (y + GY) as f32);
    b.poly(&[g(-8, 0), g(-8, -16), g(0, -24), g(8, -16), g(8, 0)], 0x6a3ab0);
    b.image()
}

fn bake_fissure() -> Image {
    let mut b = Buf::new(CW, CH);
    b.rect(CX - 1, GY - 20, 2, 14, 0xc8a0ff);
    b.rect(CX - 5, GY - 12, 10, 2, 0xc8a0ff);
    b.image()
}

fn bake_rim() -> Image {
    let mut b = Buf::new(CW, CH);
    let pts = [(-11, 0), (-11, -18), (0, -28), (11, -18), (11, 0)];
    for w in pts.windows(2) {
        b.line(CX + w[0].0, GY + w[0].1, CX + w[1].0, GY + w[1].1, 0xc8a0ff);
    }
    b.image()
}

/// The halo gets its own wider canvas (r=56 spills past the tower's 72px).
const HALO_S: usize = 116;
fn bake_halo() -> Image {
    let mut b = Buf::new(HALO_S, HALO_S);
    b.circle(58.0, 58.0, 56.0, 0x8a5ae0);
    b.image()
}

// --- Spawn + tick --------------------------------------------------------------------

/// Raise the spire as children of the room root (rides slides, swept per room).
/// `fx, fy` is the worldgen entity anchor; the js drew from cx = x+8, gy = y+16.
pub fn spawn(commands: &mut Commands, images: &mut Assets<Image>, root: Entity, fx: f32, fy: f32) {
    let t0 = ((fx as i64) * 7 + (fy as i64) * 13) % 60; // the js per-spire phase
    let z = actor_z(fy + 16.0);
    let (bx, by) = (PLAY_X + fx + 8.0 - CX as f32, PLAY_Y + fy + 16.0 - GY as f32);
    let layer = |commands: &mut Commands, images: &mut Assets<Image>, img: Image, dz: f32, kind: Option<RiftFxKind>| {
        let e = commands
            .spawn((Sprite::from_image(images.add(img)), at(bx, by, CW as f32, CH as f32, z + dz), PIXEL_LAYER))
            .id();
        if let Some(kind) = kind {
            commands.entity(e).insert(RiftFx { kind, t0 });
        }
        commands.entity(root).add_child(e);
    };
    layer(commands, images, bake_cracks(), -0.01, Some(RiftFxKind::Cracks));
    layer(commands, images, bake_body(), 0.0, None);
    layer(commands, images, bake_seams(), 0.01, Some(RiftFxKind::Seams));
    layer(commands, images, bake_windows(), 0.01, Some(RiftFxKind::Windows));
    layer(commands, images, bake_swirl(), 0.01, Some(RiftFxKind::Swirl));
    layer(commands, images, bake_fissure(), 0.02, Some(RiftFxKind::Fissure));
    layer(commands, images, bake_rim(), 0.03, Some(RiftFxKind::Rim));
    // The halo bleeds over everything (the js drew it last).
    let halo = commands
        .spawn((
            Sprite::from_image(images.add(bake_halo())),
            at(PLAY_X + fx + 8.0 - 58.0, PLAY_Y + fy + 16.0 - 50.0 - 58.0, HALO_S as f32, HALO_S as f32, z + 0.04),
            PIXEL_LAYER,
            RiftFx { kind: RiftFxKind::Halo, t0 },
        ))
        .id();
    commands.entity(root).add_child(halo);
    // Three shards of the world orbit the crown, slow and wrong.
    for i in 0..3usize {
        let size = if i == 0 { 2.0 } else { 3.0 };
        let col = if i == 0 { 0xb0_88_ff } else { 0x4a_3a_6a };
        let mut b = Buf::new(size as usize, size as usize);
        b.rect(0, 0, size as i32, size as i32, col);
        let e = commands
            .spawn((
                Sprite::from_image(images.add(b.image())),
                at(PLAY_X + fx + 8.0, PLAY_Y + fy + 16.0 - 108.0, size, size, z + 0.05),
                PIXEL_LAYER,
                RiftShard { i, t0, fx, fy },
            ))
            .id();
        commands.entity(root).add_child(e);
    }
}

/// The breath, the flicker, and the orbit — the js per-frame alphas, live.
fn rift_fx_tick(
    clock: Res<FrameClock>,
    mut layers: Query<(&RiftFx, &mut Sprite)>,
    mut shards: Query<(&RiftShard, &mut Transform), Without<RiftFx>>,
) {
    for (fx, mut s) in &mut layers {
        let t = (clock.0 + fx.t0) as f32;
        let p = 0.5 + 0.5 * (t / 26.0).sin();
        let fl = 0.5 + 0.5 * (t / 7.0).sin();
        let a = match fx.kind {
            RiftFxKind::Cracks => 0.16 + 0.12 * p,
            RiftFxKind::Seams => 0.35 + 0.35 * p,
            RiftFxKind::Windows => 0.5 + 0.4 * p,
            RiftFxKind::Swirl => 0.25 + 0.35 * fl,
            RiftFxKind::Fissure => 0.5 + 0.5 * p,
            RiftFxKind::Rim => 0.5 + 0.4 * p,
            // js 0.05+0.05p — bumped for Bevy's linear-space blending (the fill_rgba rule).
            RiftFxKind::Halo => 0.12 + 0.08 * p,
        };
        s.color = Color::srgba(1.0, 1.0, 1.0, a);
    }
    for (sh, mut tf) in &mut shards {
        let t = (clock.0 + sh.t0) as f32;
        let a = t / 40.0 + sh.i as f32 * 2.1;
        let r = 22.0 + 3.0 * (t / 30.0 + sh.i as f32).sin();
        let size = if sh.i == 0 { 2.0 } else { 3.0 };
        let x = sh.fx + 8.0 + (a.cos() * r).round() - 1.0;
        let y = sh.fy + 16.0 - 108.0 + (a.sin() * 7.0).round() - 1.0;
        let z = tf.translation.z;
        *tf = at(PLAY_X + x, PLAY_Y + y, size, size, z);
    }
}

pub struct RiftSpirePlugin;

impl Plugin for RiftSpirePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, rift_fx_tick.run_if(super::screen::playing));
    }
}
