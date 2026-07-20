//! wriftheart_tab.rs — the WRIFTHEART codex page (js drawWriftheart): the assembling
//! heart of the main quest. A dark crystal heart, CPU-rastered from the js bezier
//! path: facet web joining the ten shard sockets, the WOUND — a jagged fissure that
//! knits closed as shards return — veins linking what you've recovered, gold
//! reliquary studs, and the ten sockets themselves: a held shard glows in its land's
//! colour, an empty one is a dim diamond already hinting at the colour it waits for.
//! Arrows cycle the shards; the right pane tells the held shard's chapter (draw_pane,
//! the standard reliquary frame). The story runs beneath — INTRO while broken,
//! FINALE once whole.
//! DEVIATION (flagged): the js scene is live canvas — god-rays, radial auras, the
//! jewelry shine-sweep, drifting motes. This raster keeps the body, the wound, the
//! web, veins, studs and sockets, at a fixed mid pulse; the light-show layers join
//! a later polish pass.

use super::dex::{draw_pane, wrap_text, DEX_AX, DEX_GY};
use super::{hint_scaffold, CodexState, CodexUi, TabContent, CONTENT_Z};
use crate::app::dungeon::Relics;
use crate::app::play::GameWorld;
use crate::gfx::{at, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::relics_data::{self, Relic};
use crate::ui::{frame_rect, label};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

#[derive(Component, Clone)]
pub struct WriftheartUi;

pub fn hint(bindings: &Bindings, pad: bool) -> String {
    let browse = format!(
        "{}/{} SHARD",
        bindings.prompt(Action::Left, pad),
        bindings.prompt(Action::Right, pad)
    );
    hint_scaffold(bindings, pad, &browse)
}

/// The ten shard sockets in js heart-space (HEART_PTS; SC=1.45 scales them out).
const HEART_PTS: [(f32, f32); 10] = [
    (-14.0, -15.0),
    (14.0, -15.0),
    (-25.0, -2.0),
    (0.0, -9.0),
    (25.0, -2.0),
    (-16.0, 8.0),
    (16.0, 8.0),
    (-8.0, 20.0),
    (8.0, 20.0),
    (0.0, 32.0),
];
const SC: f32 = 1.45;

/// The heart raster's canvas (heart-space -32..+32 x, -60..+58 y at hw 66/hh 92).
const IMG_W: u32 = 160;
const IMG_H: u32 = 128;
const OX: f32 = 80.0; // heart-centre inside the raster
const OY: f32 = 62.0;

/// Sample the js heartPath (two cubic beziers) into a polygon.
fn heart_poly(w: f32, h: f32) -> Vec<(f32, f32)> {
    let cubic = |p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), t: f32| {
        let u = 1.0 - t;
        (
            u * u * u * p0.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * p3.0,
            u * u * u * p0.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * p3.1,
        )
    };
    let bottom = (0.0, h * 0.58);
    let dip = (0.0, -h * 0.08);
    let mut pts = Vec::with_capacity(96);
    for i in 0..48 {
        let t = i as f32 / 47.0;
        pts.push(cubic(bottom, (-w, h * 0.04), (-w * 0.62, -h * 0.62), dip, t));
    }
    for i in 0..48 {
        let t = i as f32 / 47.0;
        pts.push(cubic(dip, (w * 0.62, -h * 0.62), (w, h * 0.04), bottom, t));
    }
    pts
}

/// Even-odd point-in-polygon (scanline crossing count).
fn inside(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut hit = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if ((yi > y) != (yj > y)) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            hit = !hit;
        }
        j = i;
    }
    hit
}

struct Raster(Image);

impl Raster {
    fn new() -> Self {
        Self(Image::new_fill(
            Extent3d { width: IMG_W, height: IMG_H, depth_or_array_layers: 1 },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        ))
    }
    /// Blend a heart-space pixel (source-over at alpha `a`).
    fn put(&mut self, hx: f32, hy: f32, hex: u32, a: f32) {
        let (x, y) = ((hx + OX) as i32, (hy + OY) as i32);
        if x < 0 || y < 0 || x >= IMG_W as i32 || y >= IMG_H as i32 {
            return;
        }
        let Ok(px) = self.0.pixel_bytes_mut(UVec3::new(x as u32, y as u32, 0)) else { return };
        let src = [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8];
        if px[3] == 0 {
            px.copy_from_slice(&[src[0], src[1], src[2], (a * 255.0) as u8]);
            return;
        }
        for i in 0..3 {
            px[i] = (src[i] as f32 * a + px[i] as f32 * (1.0 - a)) as u8;
        }
        px[3] = px[3].max((a * 255.0) as u8);
    }
    /// A 1px line in heart-space, optionally clipped to the heart body.
    #[allow(clippy::too_many_arguments)] // two endpoints + paint + clip — a plotting call
    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, hex: u32, a: f32, clip: Option<&[(f32, f32)]>) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let (x, y) = (x0 + dx * t, y0 + dy * t);
            if clip.is_none_or(|p| inside(p, x, y)) {
                self.put(x.round(), y.round(), hex, a);
            }
        }
    }
}

/// Bake the heart at this progress (0..=1) — body, rims, web, wound, veins, studs.
fn bake_heart(relics: &Relics, shards: &[&'static Relic]) -> Image {
    let n = shards.len().max(1) as f32;
    let got = shards.iter().filter(|r| relics.0.contains(r.biome)).count() as f32;
    let prog = got / n;
    let pulse = 0.5; // the js breathes at 0.5 +/- 0.5; the raster holds mid-beat (flagged)
    let (hw, hh) = (66.0, 92.0);
    let poly = heart_poly(hw, hh);
    let mut r = Raster::new();

    // A soft halo standing in for the js radial aura (brighter as it fills).
    let halo_a = 0.05 + prog * 0.20;
    for y in -60..56 {
        for x in -70..70 {
            let (fx, fy) = (x as f32, y as f32);
            let d = (fx * fx / 1.6 + fy * fy).sqrt();
            if d < 58.0 && !inside(&poly, fx, fy) {
                r.put(fx, fy, 0xc878ff, halo_a * (1.0 - d / 58.0) * 0.5);
            }
        }
    }
    // The body: dark crystal fill.
    for y in -60..56 {
        for x in -70..70 {
            let (fx, fy) = (x as f32, y as f32);
            if inside(&poly, fx, fy) {
                r.put(fx, fy, 0x170c24, 1.0);
            }
        }
    }
    // The glowing rim (2px walk of the sampled outline) + the inner bevel echo.
    let rim_a = if prog >= 1.0 { 0.7 + 0.3 * pulse } else { 0.3 + prog * 0.5 };
    let rim_col = if prog >= 1.0 { 0xe1a0ff } else { 0x965ad2 };
    for w in [(1.0, rim_a), (0.9, 0.10 + 0.22 * prog)] {
        let (scale, a) = w;
        let col = if scale >= 1.0 { rim_col } else { 0xe2beff };
        for i in 0..poly.len() {
            let (x0, y0) = poly[i];
            let (x1, y1) = poly[(i + 1) % poly.len()];
            r.line(x0 * scale, y0 * scale + (1.0 - scale) * 2.0, x1 * scale, y1 * scale + (1.0 - scale) * 2.0, col, a, None);
        }
    }
    // Facet web: every socket pair faintly joined — cut crystal, even while empty.
    for (i, &(ax, ay)) in HEART_PTS.iter().enumerate() {
        for &(bx, by) in HEART_PTS.iter().skip(i + 1) {
            let (dx, dy) = (ax - bx, ay - by);
            if dx * dx + dy * dy < 26.0 * 26.0 {
                r.line(ax * SC, ay * SC, bx * SC, by * SC, 0x9664d2, 0.10, Some(&poly));
            }
        }
    }
    // The WOUND: the jagged fissure, knitting closed as shards return.
    if prog < 1.0 {
        let ca = 1.0 - prog;
        let crack: [(f32, f32); 7] = [(1.0, -12.0), (6.0, -1.0), (-4.0, 12.0), (4.0, 23.0), (-3.0, 33.0), (3.0, 44.0), (0.0, 54.0)];
        for k in 0..crack.len() - 1 {
            r.line(crack[k].0, crack[k].1, crack[k + 1].0, crack[k + 1].1, 0x05020a, 0.9 * ca, Some(&poly));
            r.line(crack[k].0 + 1.0, crack[k].1, crack[k + 1].0 + 1.0, crack[k + 1].1, 0x05020a, 0.9 * ca, Some(&poly));
        }
        let light: [(f32, f32); 7] = [(4.0, -10.0), (9.0, 0.0), (-1.0, 13.0), (7.0, 24.0), (0.0, 34.0), (6.0, 45.0), (3.0, 54.0)];
        for k in 0..light.len() - 1 {
            r.line(light[k].0, light[k].1, light[k + 1].0, light[k + 1].1, 0xbe78ff, 0.30 * ca, Some(&poly));
        }
        // The two hairline side-cracks off the wound.
        for seg in [[(-1.0, 3.0), (-16.0, 10.0), (-22.0, 20.0)], [(4.0, 23.0), (17.0, 29.0), (20.0, 39.0)]] {
            r.line(seg[0].0, seg[0].1, seg[1].0, seg[1].1, 0x05020a, 0.7 * ca, Some(&poly));
            r.line(seg[1].0, seg[1].1, seg[2].0, seg[2].1, 0x05020a, 0.7 * ca, Some(&poly));
        }
    }
    // Specular gleam pixels on the upper-left lobe.
    r.put(-38.0, -41.0, 0xebd7ff, 0.32);
    r.put(-37.0, -41.0, 0xebd7ff, 0.32);
    r.put(-32.0, -35.0, 0xebd7ff, 0.25);
    // Veins linking collected shards (brighter with progress).
    let vein_a = 0.10 + 0.25 * prog;
    for i in 0..HEART_PTS.len().min(shards.len()) {
        if !relics.0.contains(shards[i].biome) {
            continue;
        }
        for j in (i + 1)..HEART_PTS.len().min(shards.len()) {
            if !relics.0.contains(shards[j].biome) {
                continue;
            }
            let (ax, ay) = HEART_PTS[i];
            let (bx, by) = HEART_PTS[j];
            let (dx, dy) = (ax - bx, ay - by);
            if dx * dx + dy * dy < 24.0 * 24.0 {
                r.line(ax * SC, ay * SC, bx * SC, by * SC, 0xbe82ff, vein_a, Some(&poly));
            }
        }
    }
    // Reliquary studs: gold prongs at the dip, tip, and both shoulders.
    for (sx, sy) in [(0.0, -(hh * 0.08) - 1.0), (0.0, hh * 0.58), (-hw + 2.0, hh * 0.04), (hw - 2.0, hh * 0.04)] {
        for dy in -2..2i32 {
            for dx in -2..2i32 {
                r.put(sx + dx as f32, sy + dy as f32, 0x8a6a20, 1.0);
            }
        }
        for dy in -2..1i32 {
            r.put(sx - 1.0, sy + dy as f32, 0xe8c860, 1.0);
            r.put(sx, sy + dy as f32, 0xe8c860, 1.0);
        }
        r.put(-1.0 + sx, sy - 2.0, 0xfff4c0, 1.0);
    }
    // The empty sockets: dim diamonds already hinting at their shard's colour.
    for (i, rl) in shards.iter().enumerate() {
        if relics.0.contains(rl.biome) {
            continue; // held shards render as sprites over the raster
        }
        let (sx, sy) = (HEART_PTS[i].0 * SC, HEART_PTS[i].1 * SC);
        // The dark diamond (js: moveTo top, corners at +/-5 x and +/-7 y), scanline-filled.
        for dy in -7..=7i32 {
            let w = 5.0 * (1.0 - dy.abs() as f32 / 7.0);
            for dx in (-w.floor() as i32)..=(w.floor() as i32) {
                r.put(sx + dx as f32, sy + dy as f32, 0x0c0814, 1.0);
            }
        }
        // The waiting colour: the diamond's rim + a glint.
        r.line(sx, sy - 7.0, sx + 5.0, sy, rl.col, 0.5, None);
        r.line(sx + 5.0, sy, sx, sy + 7.0, rl.col, 0.5, None);
        r.line(sx, sy + 7.0, sx - 5.0, sy, rl.col, 0.5, None);
        r.line(sx - 5.0, sy, sx, sy - 7.0, rl.col, 0.5, None);
        r.put(sx - 3.0, sy - 3.0, rl.col, 0.55);
        r.put(sx - 2.0, sy - 4.0, rl.col, 0.55);
    }
    r.0
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn run(
    mut commands: Commands,
    cx_state: Res<CodexState>,
    mut images: ResMut<Assets<Image>>,
    relics: Res<Relics>,
    world: Res<GameWorld>,
    mut state: ResMut<ActionState>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    old: Query<Entity, With<WriftheartUi>>,
    mut cur: Local<usize>,
    mut seen_gen: Local<Option<u32>>,
) {
    // This world's ten shards, in the relic table's order (js worldShards()).
    let world_biomes = world.0.shard_biomes();
    let shards: Vec<&'static Relic> =
        relics_data::LIST.iter().filter(|r| world_biomes.contains(&r.biome)).collect();
    let n = shards.len();
    if n == 0 {
        return;
    }
    // Browse (js updateWriftheartDex): left/up back, right/down forward, wrapping.
    let mut moved = false;
    *cur = (*cur).min(n - 1);
    if state.pressed(Action::Left) || state.pressed(Action::Up) {
        *cur = (*cur + n - 1) % n;
        state.consume(Action::Left);
        state.consume(Action::Up);
        moved = true;
    }
    if state.pressed(Action::Right) || state.pressed(Action::Down) {
        *cur = (*cur + 1) % n;
        state.consume(Action::Right);
        state.consume(Action::Down);
        moved = true;
    }
    if moved {
        sfx.write(crate::app::sfx::Sfx("menuMove"));
    }
    if *seen_gen == Some(cx_state.generation) && !moved {
        return;
    }
    *seen_gen = Some(cx_state.generation);
    for e in &old {
        commands.entity(e).despawn();
    }
    let tag = || (CodexUi, TabContent, WriftheartUi);

    let got = shards.iter().filter(|r| relics.0.contains(r.biome)).count();
    let done = got >= n;
    let head = format!("THE WRIFTHEART  {got} / {n}{}", if done { "  -  WHOLE" } else { "" });
    label(&mut commands, &mut images, &head, DEX_AX, 15.0, if done { 0xd0a0ff } else { 0xbfb9a0 }, CONTENT_Z + 0.1, tag());

    // The heart raster, centred where the js paints it (cx = DEX_AX+97, cy = DEX_GY+68).
    let (hcx, hcy) = (DEX_AX + 97.0, DEX_GY + 68.0);
    let img = images.add(bake_heart(&relics, &shards));
    commands.spawn((
        Sprite::from_image(img),
        at(hcx - OX, hcy - OY, IMG_W as f32, IMG_H as f32, CONTENT_Z + 0.05),
        PIXEL_LAYER,
        tag(),
    ));
    // Held shards sit over their sockets in the land's colour (js dexBlit + glow).
    for (i, rl) in shards.iter().enumerate() {
        let (sx, sy) = (hcx + HEART_PTS[i].0 * SC, hcy + HEART_PTS[i].1 * SC);
        if relics.0.contains(rl.biome) {
            let icon = crate::app::dungeon::shard_image(&mut images, rl.col);
            let mut s = Sprite::from_image(icon);
            s.custom_size = Some(Vec2::splat(12.0));
            commands.spawn((s, at(sx - 6.0, sy - 6.0, 12.0, 12.0, CONTENT_Z + 0.1), PIXEL_LAYER, tag()));
        }
        if i == *cur {
            frame_rect(&mut commands, sx - 8.0, sy - 8.0, 17.0, 17.0, 0xffd34d, CONTENT_Z + 0.15, tag());
        }
    }
    // The story beneath the heart — INTRO while broken, FINALE once whole.
    let story = if done { relics_data::FINALE } else { relics_data::INTRO };
    let mut sy = hcy + 72.0;
    for ln in wrap_text(story, 196.0) {
        label(&mut commands, &mut images, &ln, DEX_AX, sy, if done { 0xc8a8f0 } else { 0x8a8a94 }, CONTENT_Z + 0.1, tag());
        sy += 8.0;
    }
    // Right pane: the selected shard's chapter (the standard reliquary frame).
    let rl = shards[*cur];
    let open = relics.0.contains(rl.biome);
    let big = open.then(|| (crate::app::dungeon::shard_image(&mut images, rl.col), 16.0));
    draw_pane(
        &mut commands,
        &mut images,
        open,
        big,
        rl.name,
        Some((&rl.biome.to_uppercase(), rl.col)),
        rl.lore,
        true,
        tag(),
    );
}
