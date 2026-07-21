//! lighting.rs — the day/night cycle + the DARKNESS pass of js/lighting.js.
//!
//! The js rasterizes darkness into an offscreen canvas every frame: fill the ambient tint
//! at the time-of-day alpha, then cut a radial hole per light (destination-out, gradient
//! stops 1.0 / 0.7 @ 55% / 0.0) and blit it over the play field. We do the SAME on CPU
//! into a play-field-sized Image (83k pixels — sub-millisecond) — byte-faithful gradient
//! math, no custom shaders, no render-pipeline risk (see the MSAA/Bloom gotchas).
//!
//! Step 2 of the js pass (the additive glow bloom around lights) is NOT here yet — the
//! baked pickup glows cover today's content; a custom additive material picks it up when
//! torches/dungeons port.
//!
//! Time of day: `dayDarkness = (1 - cos(t*2pi)) / 2` over DAY_LEN (frame 0 = noon), scaled
//! into ambient alpha [DAY_MIN..NIGHT_MAX], deep-blue tint. Debug: WRIFT_TIME=0..1 pins
//! the clock phase (0 = noon, 0.5 = midnight) for eyeballing either extreme.

use super::gather::{Pickup, DAY_LEN};
use super::room_render::{FrameClock, PLAY_X, PLAY_Y};
use crate::gfx::{at, PIXEL_LAYER};
use crate::{CANVAS_H, CANVAS_W, SIDEBAR_W};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

// The overlay covers everything RIGHT of the sidebar — a hair wider than the js's exact
// play rect, so a tree canopy spilling past the field's top edge can't stay day-bright
// against the night.
const OV_X: f32 = SIDEBAR_W;
const OV_W: i32 = (CANVAS_W as i32) - (SIDEBAR_W as i32);
const OV_H: i32 = CANVAS_H as i32;

pub const DAY_MIN: f32 = 0.16; // a touch of blue overlay even at high noon
pub const NIGHT_MAX: f32 = 0.8; // darkness alpha at midnight
// The overworld night tint. DEVIATION (Baz: "night should have a slight bluish tint"):
// the js's [10, 14, 38] is so deep it composites to near-black; the lifted blue channel
// pulls the whole night toward moonlit blue while the alpha keeps it just as dark.
const TINT: [u8; 3] = [12, 18, 58];

/// Linear-blend correction for the js's sRGB-composited darkness (the PORT.md alpha
/// gotcha): the GPU blends this overlay in LINEAR space, the js canvas composited the
/// same alpha in sRGB bytes, so raw js alphas read a shade too bright ("night doesnt
/// get very dark" — Baz). Matching the js midnight (0.8) needs an effective ~0.90 on
/// midtones / ~0.95 on bright grass; 1.16 splits it across the whole curve. Applies
/// to every js-authored ambient alpha that lands here (dungeon 0.94 will clamp to 1).
const DARK_GAIN: f32 = 1.16;

const LIGHT_Z: f32 = 13.0; // over every play-field layer (FX 12.x), under the HUD (17.2)

/// 0 at noon -> 1 at midnight -> 0 (js dayDarkness — frame 0 boots at noon).
pub fn day_darkness(clock: i64) -> f32 {
    let t = clock.rem_euclid(DAY_LEN) as f32 / DAY_LEN as f32;
    (1.0 - (t * std::f32::consts::TAU).cos()) / 2.0
}

/// The current ambient darkness alpha (js: DAY_MIN + dayDarkness * (NIGHT_MAX - DAY_MIN)).
pub fn ambient_alpha(clock: i64) -> f32 {
    (DAY_MIN + day_darkness(clock) * (NIGHT_MAX - DAY_MIN)) * DARK_GAIN
}

#[derive(Resource)]
struct LightOverlay(Handle<Image>);

/// WRIFT_TIME=0..1 pins the cycle phase for testing (0 = noon, 0.5 = midnight).
#[derive(Resource)]
struct PinnedTime(Option<f32>);

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_overlay).add_systems(Update, update_overlay);
    }
}

fn setup_overlay(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let img = Image::new_fill(
        Extent3d { width: OV_W as u32, height: OV_H as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(img);
    commands.spawn((
        Sprite::from_image(handle.clone()),
        at(OV_X, 0.0, OV_W as f32, OV_H as f32, LIGHT_Z),
        PIXEL_LAYER,
    ));
    commands.insert_resource(LightOverlay(handle));
    commands.insert_resource(PinnedTime(
        std::env::var("WRIFT_TIME").ok().and_then(|v| v.parse().ok()),
    ));
}

/// A light hole's erase factor at gradient position t in [0..1] — the js destination-out
/// radial stops: 1.0 at the centre, 0.7 at 55%, 0.0 at the rim (linear between).
fn hole_cut(t: f32) -> f32 {
    if t < 0.55 {
        1.0 - 0.3 * (t / 0.55)
    } else {
        0.7 * (1.0 - (t - 0.55) / 0.45).max(0.0)
    }
}

/// (ambient alpha bits, light list) — the rebuild key: skip the raster when it's unchanged.
type OverlayKey = (u32, Vec<(i32, i32, i32)>, Vec<(i32, i32, i32, i32)>);
/// Per-room encounter camp lights: (room, [(x, y, r)]).
type CampCache = Option<((i32, i32), Vec<(i32, i32, i32)>)>;

/// The room-scene inputs (grouped under Bevy's 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
struct SceneCtx<'w, 's> {
    cur: Res<'w, super::play::CurRoom>,
    world: Res<'w, super::play::GameWorld>,
    slide: Res<'w, super::play::SlideState>,
    fluting: Res<'w, super::flute::Fluting>,
    /// The walking hoard's light (js collectLights 'lootgoblin') — nested here,
    /// update_overlay sits AT the 16-param cap.
    lootgobs: Query<'w, 's, &'static crate::combat::Hitbox, With<super::lootgoblin::LootGoblin>>,
}

/// Rebuild the darkness buffer: ambient fill, then every light multiplies the alpha down
/// inside its radius (sequential destination-out, exactly the js compositing).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn update_overlay(
    clock: Res<FrameClock>,
    pinned: Res<PinnedTime>,
    overlay: Res<LightOverlay>,
    settings: Res<crate::settings::Settings>,
    inside: Res<super::interior::Inside>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    dlights: Res<super::dungeon::DungeonLights>,
    inv: Res<crate::inventory::PlayerInv>,
    players: Query<&super::play::Player>,
    mut images: ResMut<Assets<Image>>,
    pickups: Query<&Pickup>,
    weather: Res<super::weather::WeatherState>,
    scene: SceneCtx,
    burning: Res<super::fire::BurningLights>,
    mut last: Local<Option<OverlayKey>>,
    mut camp_cache: Local<CampCache>,
) {
    let (cur, world, slide) = (&scene.cur, &scene.world, &scene.slide);
    let clock_val = match pinned.0 {
        Some(t) => (t * DAY_LEN as f32) as i64,
        None => clock.0,
    };
    // The menu's BRIGHTNESS lift subtracts straight off the ambient darkness (js dimAmbient).
    // Indoors reads as daylit until the warm-light pass ports (interiors carry lights).
    // UNDERGROUND ignores the sun: theme ambience (0.78; ambAlpha themes opt out; 'dark'
    // hymn floors 0.94), through the same DARK_GAIN linear-blend correction.
    let dungeon_amb = in_dungeon.0.as_ref().map(|run| {
        let base = if run.dungeon.cur().gimmick == Some("dark") { 0.94 } else { run.dungeon.theme.amb_alpha.unwrap_or(0.78) };
        ((base * DARK_GAIN).min(1.0), run.dungeon.theme.tint)
    });
    let mut alpha = if inside.0.is_some() {
        0.0
    } else if let Some((amb, _)) = dungeon_amb {
        (amb - settings.bright_lift()).clamp(0.0, 1.0)
    } else {
        (ambient_alpha(clock_val) - settings.bright_lift()).clamp(0.0, 1.0)
    };
    let mut tint = dungeon_amb.map(|(_, t)| t).unwrap_or(TINT);
    // The SKY'S MOOD (js applyAmbient): weather darkness + tint pull + the strike's
    // momentary lift — outdoors only (interiors/dungeons keep their own light).
    if inside.0.is_none() && dungeon_amb.is_none() {
        let (wa, wt) = weather.ambient(settings.flash);
        alpha = (alpha + wa).clamp(0.0, 0.92);
        if let Some((tw, k)) = wt {
            let lerp = |a: u8, b: f32| (a as f32 + (b - a as f32) * k).round() as u8;
            tint = [lerp(tint[0], tw[0]), lerp(tint[1], tw[1]), lerp(tint[2], tw[2])];
        }
    }

    // The light list (js collectLights, the ported subset): every ground pickup keeps a
    // small pool of visibility around itself (item/coin/potion: r 16). The hero carries NO
    // overworld light — that's lantern gear (hasGearFlag('light')); dungeons come later.
    // Positions land in overlay space (play-field coords + the play rect's offset).
    let (ox, oy) = ((PLAY_X - OV_X) as i32, PLAY_Y as i32);
    let mut lights: Vec<(i32, i32, i32)> = pickups
        .iter()
        .map(|p| (ox + (p.x + 2.0) as i32, oy + (p.y + 2.0) as i32, 16))
        .collect();
    // A walking hoard, glinting gold (js collectLights 'lootgoblin': r 30).
    for hb in &scene.lootgobs {
        lights.push((ox + (hb.x + 5.0) as i32, oy + (hb.y + 4.0) as i32, 30));
    }
    // A blaze lights the area (js collectLights' .burning arm, r36).
    for &(x, y) in &burning.0 {
        lights.push((ox + x, oy + y, 36));
    }
    // The tune GLOWS (past-js flourish): while the flute is up, the hero holds a warm
    // breathing pool of light — each pressed note kicks it wider, a song catch flares
    // it. Radius quantized to 3px steps so the raster only rebuilds on real changes.
    if let (Some(f), Ok(p)) = (&scene.fluting.0, players.single()) {
        let gmax = f.glow.iter().copied().max().unwrap_or(0) as f32;
        let r = if f.phase == super::flute::Phase::Warp {
            // The channelling glow brightens the whole area as the warp charges
            // (js collectLights' warp arm: r 34 -> 80).
            34.0 + (f.wt as f32 / super::flute::WARP_CHARGE as f32).min(1.0) * 46.0
        } else {
            40.0 + (f.t as f32 * 0.1).sin() * 6.0 + gmax * 1.6 + if f.flash > 0 { 14.0 } else { 0.0 }
        };
        lights.push((ox + p.x as i32 + 8, oy + p.y as i32 + 9, ((r / 3.0).round() * 3.0) as i32));
    }
    // Fixed flames in the room hold their ground against the night (js collectLights:
    // torch r40, the graveyard's cursed brazier r44, the monument's breathing maw r44,
    // wisps r20). Derived from the room's entity layout — no live state needed.
    if inside.0.is_none() && in_dungeon.0.is_none() {
        // Mid-slide, `cur` is already the INCOMING room but its root is still scrolling —
        // shift its flames by the live slide offset so each pool rides with its torch
        // (the same class of bug the trees had; light positions sit in the cache key,
        // so the moving offset re-renders every slide frame).
        let (sox, soy) = slide.incoming_offset().map_or((0, 0), |(x, y)| (x.round() as i32, y.round() as i32));
        for e in world.0.room_entities(cur.rx, cur.ry) {
            let (x, y) = (e.x + sox, e.y + soy);
            match e.kind {
                "torch" => lights.push((ox + x + 8, oy + y + 4, 40)),
                "gravebrazier" => lights.push((ox + x + 8, oy + y + 4, 44)),
                "dungeon" => lights.push((ox + x + 8, oy + y - 6, 44)),
                "wisp" => lights.push((ox + x, oy + y, 20)),
                _ => {}
            }
        }
        // Encounter camp light (js collectLights: campfire r44, crystal r30) — the
        // scene is deterministic, so it bakes once per room into a local cache.
        if camp_cache.as_ref().map(|(room, _)| *room) != Some((cur.rx, cur.ry)) {
            let mut v = Vec::new();
            if let Some((def, _)) = super::encounters::for_room(&world.0, cur.rx, cur.ry) {
                let scene = super::encounters::build(def, &world.0, cur.rx, cur.ry);
                for d in &scene.decor {
                    match d.kind {
                        "campfire" => v.push((d.x as i32 + 8, d.y as i32 + 4, 44)),
                        "crystal" => v.push((d.x as i32 + 8, d.y as i32 + 6, 30)),
                        _ => {}
                    }
                }
            }
            *camp_cache = Some(((cur.rx, cur.ry), v));
        }
        if let Some((_, camps)) = camp_cache.as_ref() {
            for &(x, y, r) in camps {
                lights.push((ox + x + sox, oy + y + soy, r));
            }
        }
    }
    // A GROUND STRIKE punches a light hole where the bolt lands (PORT-ORIGINAL — the
    // js only flashed the whole screen), fading with the flash.
    if dungeon_amb.is_none()
        && inside.0.is_none()
        && let Some((sx, sy)) = weather.strike
        && weather.flash > 0.05
    {
        lights.push((ox + sx as i32, oy + sy as i32, (46.0 * weather.flash) as i32));
    }
    if dungeon_amb.is_some() {
        // js playerLight: underground the hero always carries a pool of light (wider
        // with lantern gear), and the room's torches + lit decor hold the walls.
        if let Ok(p) = players.single() {
            let r = if inv.has_gear_flag("light") { 84 } else { 54 };
            lights.push((ox + (p.x + 8.0) as i32, oy + (p.y + 9.0) as i32, r));
        }
        // Mid-slide the dungeon room's root is still scrolling in, but dlights sit at the
        // landed positions — so the torch glow detached from the torch during transitions
        // (Baz). Ride the same incoming-slide offset the overworld flames use (line ~204).
        let (dox, doy) = slide.incoming_offset().map_or((0, 0), |(x, y)| (x.round() as i32, y.round() as i32));
        for &(lx, ly, r) in &dlights.0 {
            lights.push((ox + lx + dox, oy + ly + doy, r));
        }
    }

    // The MINER'S HELMET (js coneLight): a bright cone thrown from the forehead lamp
    // along the facing — worth casting only underground or in real dark. The beam
    // starts at the LAMP (head height, nudged toward facing), not the body centre.
    let mut cones: Vec<(i32, i32, i32, i32)> = Vec::new(); // (x, y, dir milli-rad, r)
    if inv.has_gear_flag("conelight")
        && (dungeon_amb.is_some() || alpha > 0.12)
        && let Ok(p) = players.single()
    {
        use crate::actors::hero::Facing;
        let dir: f32 = match p.facing {
            Facing::Right => 0.0,
            Facing::Down => std::f32::consts::FRAC_PI_2,
            Facing::Left => std::f32::consts::PI,
            Facing::Up => -std::f32::consts::FRAC_PI_2,
        };
        let horiz = matches!(p.facing, Facing::Left | Facing::Right);
        let x = ox + p.x as i32 + 8 + (dir.cos() * 3.0).round() as i32;
        let y = oy + p.y as i32 + if horiz { 2 } else { 4 } + (dir.sin() * 2.0).round() as i32;
        lights.push((x, y, 24)); // the lamp's own small pool (js r 24)
        cones.push((x, y, (dir * 1000.0) as i32, 122));
    }

    // Skip the rebuild when nothing moved and the clock's alpha hasn't visibly changed.
    let key = ((alpha * 4096.0) as u32 ^ (u32::from(tint[0]) << 16) ^ (u32::from(tint[2]) << 8), lights.clone(), cones.clone());
    if last.as_ref() == Some(&key) {
        return;
    }
    *last = Some(key);

    let Some(mut img) = images.get_mut(&overlay.0) else { return };
    let Some(data) = img.data.as_mut() else { return };
    // Ambient fill.
    let base_a = (alpha * 255.0).round() as u8;
    for px in data.chunks_exact_mut(4) {
        px.copy_from_slice(&[tint[0], tint[1], tint[2], base_a]);
    }
    // Cut each light's hole (multiplicative within its bounding rect).
    let (w, h) = (OV_W, OV_H);
    for &(lx, ly, r) in &lights {
        let rf = r as f32;
        for y in (ly - r).max(0)..(ly + r).min(h) {
            for x in (lx - r).max(0)..(lx + r).min(w) {
                let d = (((x - lx).pow(2) + (y - ly).pow(2)) as f32).sqrt();
                if d >= rf {
                    continue;
                }
                let i = ((y * w + x) * 4 + 3) as usize;
                data[i] = (data[i] as f32 * (1.0 - hole_cut(d / rf))).round() as u8;
            }
        }
    }
    // The helmet's CONE (js cone: {dir, half 0.52, r 122}): a wedge of light along
    // the facing, feathered at its angular edges so the beam's rim reads soft.
    for &(cx2, cy2, dmill, r) in &cones {
        let (dirf, rf, half) = (dmill as f32 / 1000.0, r as f32, 0.52f32);
        for y in (cy2 - r).max(0)..(cy2 + r).min(h) {
            for x in (cx2 - r).max(0)..(cx2 + r).min(w) {
                let (dx, dy) = ((x - cx2) as f32, (y - cy2) as f32);
                let d = (dx * dx + dy * dy).sqrt();
                if d >= rf || d < 0.5 {
                    continue;
                }
                let mut adiff = (dy.atan2(dx) - dirf).rem_euclid(std::f32::consts::TAU);
                if adiff > std::f32::consts::PI {
                    adiff = std::f32::consts::TAU - adiff;
                }
                if adiff >= half {
                    continue;
                }
                let feather = ((half - adiff) / 0.14).clamp(0.0, 1.0);
                let i = ((y * w + x) * 4 + 3) as usize;
                data[i] = (data[i] as f32 * (1.0 - hole_cut(d / rf) * feather)).round() as u8;
            }
        }
    }
}
