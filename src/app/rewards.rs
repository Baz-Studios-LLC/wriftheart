//! rewards.rs — the kill-reward loop's state and feedback UX, three ports in one place:
//! [`Progress`] (js player.xp/level/gainXP: level*10 curve, +1 skill point per level),
//! the LOOT FEED (js addLoot/drawLootLog: pickup toasts bottom-right of the play field,
//! rapid same-item runs merge, six lines max), and the LEVEL-UP flourish (js drawLevelUp:
//! golden flash + shockwave rings + glow + a pop-scaled "LEVEL UP!").

use super::battle::not_sliding;
use super::room_render::{PLAY_X, PLAY_Y};
use super::screen::playing;
use super::slideout::TreeAlloc;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::items;
use crate::room::{PX_H, PX_W};
use crate::{CANVAS_H, CANVAS_W};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// XP needed to clear `level` (js xpForLevel).
pub fn xp_for_level(level: i32) -> i32 {
    level * 10
}

/// The hero's levelling state (js player.level/xp/levelFlash).
#[derive(Resource)]
pub struct Progress {
    pub level: i32,
    pub xp: i32,
    pub flash: u32, // drives the level-up flourish (js levelFlash, 120 frames)
}

impl Default for Progress {
    fn default() -> Self {
        Self { level: 1, xp: 0, flash: 0 }
    }
}

const FLASH_MAX: u32 = 120;

/// js gainXP: bank the XP, loop level-ups (each grants ONE skill point — max HP grows only
/// via the tree, so combat stays meaningful for hundreds of levels).
pub fn gain_xp(progress: &mut Progress, alloc: &mut TreeAlloc, amount: i32) {
    progress.xp += amount;
    while progress.xp >= xp_for_level(progress.level) {
        progress.xp -= xp_for_level(progress.level);
        progress.level += 1;
        alloc.points += 1;
        progress.flash = FLASH_MAX;
    }
}

// --- The loot feed (js lootLog) ---

const LOOT_LIFE: u32 = 200; // frames a loot line lingers

/// Toast text colour per item id (js LOOT_COL — ported ids only; rarity colour covers the
/// rest, exactly the js fallback).
fn loot_col(id: &str) -> Option<u32> {
    Some(match id {
        "coin" => 0xfcd000,
        "potion" => 0xfc6868,
        "wood" => 0xc8915a,
        "stone" => 0xc4c4c4,
        "fiber" => 0x9ad06a,
        "axe" | "pick" => 0xbcbcbc,
        "sword" => 0xcfe0ff,
        _ => return None,
    })
}

/// js collectPickup's colour rule: rare+ tints to rarity, commons use the LOOT_COL accent.
pub fn toast_color(id: &str) -> u32 {
    if items::rarity_of(id).tier() > 0 {
        items::rarity_of(id).color()
    } else {
        loot_col(id).unwrap_or_else(|| items::rarity_of(id).color())
    }
}

pub struct Loot {
    key: String,
    name: String,
    qty: i32,
    color: u32,
    coin: bool, // renders as "+N COPPER"
    raw: bool,  // pre-formatted line (never merged, no "xN")
    life: u32,
}

#[derive(Resource, Default)]
pub struct LootLog(pub Vec<Loot>);

impl LootLog {
    /// js addLoot: a rapid run of the same key merges into the top line; six lines max.
    pub fn add(&mut self, key: &str, name: &str, qty: i32, color: u32, coin: bool, raw: bool) {
        if let Some(top) = self.0.last_mut()
            && !raw
            && top.key == key
            && top.life > 70
        {
            top.qty += qty;
            top.life = LOOT_LIFE;
            return;
        }
        self.0.push(Loot { key: key.into(), name: name.into(), qty, color, coin, raw, life: LOOT_LIFE });
        if self.0.len() > 6 {
            self.0.remove(0);
        }
    }
}

#[derive(Component)]
struct LootUi;
#[derive(Component)]
struct FlashUi;

pub struct RewardsPlugin;

impl Plugin for RewardsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Progress>()
            .init_resource::<LootLog>()
            .add_systems(Startup, |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
                commands.insert_resource(FlashArt::build(&mut images));
            })
            .add_systems(FixedUpdate, rewards_tick.run_if(playing).run_if(not_sliding))
            // The flourish never pauses (Baz, 2026-07-16): it keeps playing through the
            // slide-out, the codex, the pause menu — every screen — and draws OVER them
            // (see Z in levelup_fx). Unconditional so it can never linger frozen either.
            .add_systems(FixedUpdate, flash_tick)
            .add_systems(Update, (lootlog_draw, levelup_fx));
    }
}

/// Age the level-up flourish (120 -> 0).
fn flash_tick(mut progress: ResMut<Progress>) {
    if progress.flash > 0 {
        progress.flash -= 1;
    }
}

/// Age the toast feed on the fixed clock, only while the world runs (js ticks it
/// inside the frozen-by-menus update pass).
fn rewards_tick(mut log: ResMut<LootLog>) {
    for l in &mut log.0 {
        l.life -= 1;
    }
    log.0.retain(|l| l.life > 0);
}

/// js drawLootLog: right-aligned toast pills above the play field's bottom edge, newest at
/// the bottom; each slides in from the right (~8 frames) and fades out over its last ~50.
fn lootlog_draw(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    log: Res<LootLog>,
    old: Query<Entity, With<LootUi>>,
    mut was_empty: Local<bool>,
) {
    if log.0.is_empty() && *was_empty {
        return; // nothing to animate and nothing to clear
    }
    *was_empty = log.0.is_empty();
    for e in &old {
        commands.entity(e).despawn();
    }
    const Z: f32 = crate::gfx::layers::LOOT_FEED;
    let right = PLAY_X + PX_W as f32 - 3.0;
    let mut y = PLAY_Y + PX_H as f32 - 10.0;
    for l in log.0.iter().rev() {
        let age = (LOOT_LIFE - l.life) as f32;
        let pop = (age / 8.0).min(1.0); // slide + fade IN over ~8 frames
        let a = pop.min((l.life as f32 / 50.0).min(1.0)); // ...fade OUT over the last ~0.8s
        let xo = ((1.0 - pop) * 12.0).round();
        let txt = if l.raw {
            l.name.clone()
        } else if l.coin {
            format!("+{} COPPER", l.qty)
        } else {
            format!("{} X{}", l.name, l.qty)
        };
        let w = font::measure(&txt) as f32 + 7.0;
        let x = right - w + xo;
        // Toast pill (js rgba(0,0,0,0.5); black dimming needs MORE alpha in linear).
        commands.spawn((
            Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.6 * a), Vec2::new(w, 9.0)),
            at(x, y - 2.0, w, 9.0, Z),
            PIXEL_LAYER,
            LootUi,
        ));
        // Coloured accent edge (screen side).
        let [r, g, b] = [(l.color >> 16) as u8, (l.color >> 8) as u8, l.color as u8];
        commands.spawn((
            Sprite::from_color(Color::srgba_u8(r, g, b, (a * 255.0) as u8), Vec2::new(1.0, 9.0)),
            at(x + w - 1.0, y - 2.0, 1.0, 9.0, Z + 0.1),
            PIXEL_LAYER,
            LootUi,
        ));
        let (img, tw) = font::bake_text(&txt, l.color, &mut images);
        let iw = (tw + (tw & 1)) as f32; // the baked texture is padded to even width
        let mut text = Sprite::from_image(img);
        text.color = Color::srgba(1.0, 1.0, 1.0, a);
        commands.spawn((text, at(x + 3.0, y, iw, 6.0, Z + 0.1), PIXEL_LAYER, LootUi));
        y -= 11.0;
    }
}

// --- The level-up flourish (js drawLevelUp) ---

/// Pre-baked radial textures for the flourish: an expanding ring and the soft glow behind
/// the text. (js draws these additively; we alpha-blend at roughly half intensity per the
/// linear-blending rule, and the ring thickness stays fixed as it scales — a motion-effect
/// approximation.)
#[derive(Resource)]
struct FlashArt {
    ring: Handle<Image>,
    glow: Handle<Image>,
}

impl FlashArt {
    fn build(images: &mut Assets<Image>) -> Self {
        Self {
            ring: radial(images, 88.0, |d, r| {
                let t = (d - (r - 3.0)).abs();
                (1.5 - t).clamp(0.0, 1.0) // a ~3px band at the rim, soft-edged
            }),
            glow: radial(images, 105.0, |d, r| (1.0 - d / r).max(0.0)),
        }
    }
}

/// Bake a radial falloff into an even-sized RGBA texture (white, alpha = f(dist)).
fn radial(images: &mut Assets<Image>, r: f32, f: impl Fn(f32, f32) -> f32) -> Handle<Image> {
    let s = (r as u32) * 2;
    let mut img = Image::new_fill(
        Extent3d { width: s, height: s, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    for y in 0..s {
        for x in 0..s {
            let d = (x as f32 + 0.5 - r).hypot(y as f32 + 0.5 - r);
            let a = f(d, r).clamp(0.0, 1.0);
            if a > 0.0
                && let Ok(px) = img.pixel_bytes_mut(UVec3::new(x, y, 0))
            {
                px.copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
            }
        }
    }
    images.add(img)
}

/// Golden full-screen flash + expanding shockwave rings + glow + "LEVEL UP!" with a
/// pop-scale overshoot — rebuilt each frame from `Progress.flash` (120 -> 0).
fn levelup_fx(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    progress: Res<Progress>,
    art: Res<FlashArt>,
    old: Query<Entity, With<FlashUi>>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let t = progress.flash;
    if t == 0 {
        return;
    }
    const Z: f32 = crate::gfx::layers::FLOURISH;
    let p = 1.0 - t as f32 / FLASH_MAX as f32; // 0 -> 1 over the effect
    let (cw, ch) = (CANVAS_W as f32, CANVAS_H as f32);
    let (cx, cy) = (cw / 2.0, ch / 2.0);
    let gold = |a: f32| Color::srgba(1.0, 226.0 / 255.0, 150.0 / 255.0, a);

    // 1) Bright golden full-screen flash, fading over the first quarter (js 0.55; halved).
    let flash_a = (1.0 - p / 0.25).max(0.0);
    if flash_a > 0.0 {
        commands.spawn((
            Sprite::from_color(gold(0.28 * flash_a), Vec2::new(cw, ch)),
            at(0.0, 0.0, cw, ch, Z),
            PIXEL_LAYER,
            FlashUi,
        ));
    }
    // 2) Expanding shockwave rings.
    for k in 0..2 {
        let rp = p * 1.5 - k as f32 * 0.18;
        if rp <= 0.0 || rp >= 1.0 {
            continue;
        }
        let radius = rp * 175.0;
        let mut ring = Sprite::from_image(art.ring.clone());
        ring.custom_size = Some(Vec2::splat(radius * 2.0));
        ring.color = Color::srgba(1.0, 222.0 / 255.0, 140.0 / 255.0, (1.0 - rp) * 0.28);
        commands.spawn((ring, at(cx - radius, cy - radius, radius * 2.0, radius * 2.0, Z + 0.02), PIXEL_LAYER, FlashUi));
    }
    // 3) Radial glow behind the text (rise & fall).
    let glow_a = ((p / 0.7).min(1.0) * std::f32::consts::PI).sin() * 0.3;
    if glow_a > 0.01 {
        let gr = 105.0;
        let mut glow = Sprite::from_image(art.glow.clone());
        glow.color = Color::srgba(1.0, 212.0 / 255.0, 120.0 / 255.0, glow_a);
        commands.spawn((glow, at(cx - gr, cy - gr, gr * 2.0, gr * 2.0, Z + 0.04), PIXEL_LAYER, FlashUi));
    }
    // 4) "LEVEL UP!" — pop-scale with overshoot, drift up, fade out at the end.
    let fade = if p < 0.8 { 1.0 } else { (1.0 - (p - 0.8) / 0.2).max(0.0) };
    let scale = if p < 0.16 {
        1.5 + (p / 0.16) * 2.0
    } else if p < 0.28 {
        3.5 - ((p - 0.16) / 0.12) * 0.5
    } else {
        3.0
    };
    let rise = -(p * 10.0).round();
    center_text(&mut commands, &mut images, "LEVEL UP!", cx, cy + rise, scale, fade, 0xffe070, Z + 0.06);
    center_text(&mut commands, &mut images, &format!("LVL {}", progress.level), cx, cy + rise + 18.0, 1.0, fade, 0xfce0a8, Z + 0.06);
}

/// js centerText: scaled text centred on a point, with a dark drop-shadow.
#[allow(clippy::too_many_arguments)] // it IS a draw call
fn center_text(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    text: &str,
    cx: f32,
    cy_mid: f32,
    scale: f32,
    alpha: f32,
    color: u32,
    z: f32,
) {
    let w = font::measure(text) as f32 * scale;
    let h = 5.0 * scale;
    let x = (cx - w / 2.0).round();
    let y = (cy_mid - h / 2.0).round();
    let off = (scale / 2.0).round().max(1.0);
    for (dx, col, dz) in [(off, 0x5a3a00u32, 0.0), (0.0, color, 0.01)] {
        let (img, tw) = font::bake_text(text, col, images);
        let iw = (tw + (tw & 1)) as f32; // texture is padded-even x 6 — scale that shape
        let mut s = Sprite::from_image(img);
        s.custom_size = Some(Vec2::new(iw * scale, 6.0 * scale));
        s.color = Color::srgba(1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0));
        commands.spawn((s, at(x + dx, y + dx, iw * scale, 6.0 * scale, z + dz), PIXEL_LAYER, FlashUi));
    }
}
