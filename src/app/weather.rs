//! weather.rs — the LIVE sky (js/weather.js's cosmetic half): the two-layer crossfade
//! over the deterministic front rolls (src/weather), the wind, the lightning, and the
//! shader quad that rains it all down (gfx/weather_fx.wgsl). WRIFT_WEATHER=<id> pins
//! the sky for shots/testing (js debugForce).
//!
//! The 18 wind-blown leaves stay ECS sprites (tiny squares suit entities); everything
//! wetter is the shader's. Ambient mood (sky darkness + tint + strike flash) feeds
//! lighting.rs; storm intensity feeds the water shader; cloud cover dims the sun's
//! shadows (PORT-ORIGINAL tie-ins — the js couldn't afford them).

use super::play::{CurRoom, GameWorld};
use super::room_render::{FrameClock, PLAY_X, PLAY_Y};
use crate::gfx::weather_fx_material::{WeatherFxMaterial, WeatherFxParams};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};
use crate::weather::{get, Kind};
use crate::worldgen::rng::Mulberry32;
use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

const EASE: f32 = 0.014; // a full change takes ~70 frames — gentle, like the js

#[derive(Resource)]
pub struct WeatherState {
    pub cur: &'static str,
    pub vis: f32,
    pub prev: &'static str,
    pub prev_vis: f32,
    pub wind: f32,
    /// Accumulated wind travel (px-weight seconds) — the shader's displacement input.
    pub windx: f32,
    pub flash: f32,
    flash_cd: i32,
    /// A ground strike this frame + where it hit (room px) — the darkness overlay
    /// punches a light hole there for the flash's duration.
    pub strike: Option<(f32, f32)>,
    forced: Option<String>,
    /// A song-commanded sky: (weather id, frames left) — outranks the sim, yields to
    /// the WRIFT pin (js commandWeather).
    commanded: Option<(String, i64)>,
    rng: Mulberry32,
}

impl Default for WeatherState {
    fn default() -> Self {
        WeatherState {
            cur: "clear",
            vis: 0.0,
            prev: "clear",
            prev_vis: 0.0,
            wind: 0.0,
            windx: 0.0,
            flash: 0.0,
            flash_cd: 90,
            strike: None,
            forced: std::env::var("WRIFT_WEATHER").ok(),
            commanded: None,
            rng: Mulberry32::new(0x5107),
        }
    }
}

impl WeatherState {
    /// The mood both layers add to the ambient light (js applyAmbient): extra darkness
    /// + a tint pull, and the strike's momentary lift.
    pub fn ambient(&self, reduce_flash: bool) -> (f32, Option<([f32; 3], f32)>) {
        let mut alpha = 0.0;
        let mut tint: Option<([f32; 3], f32)> = None;
        for (id, v) in [(self.prev, self.prev_vis), (self.cur, self.vis)] {
            let d = get(id);
            if v <= 0.001 {
                continue;
            }
            alpha += v * d.sky;
            if let Some(t) = d.tint {
                tint = Some(([t[0] as f32, t[1] as f32, t[2] as f32], v * 0.6));
            }
        }
        let lift = if reduce_flash { 0.12 } else { 0.5 }; // photosensitivity: flicker, not strobe
        (alpha - self.flash * lift, tint)
    }

    /// Pin (or free) the sky — the dev panel's WEATHER row (js debugForce).
    pub fn force(&mut self, id: Option<String>) {
        self.forced = id;
    }
    /// What the sky is pinned to, if anything.
    pub fn forced_id(&self) -> Option<&str> {
        self.forced.as_deref()
    }
    /// A song-commanded sky for `frames` (js commandWeather) — Stormcall's channel.
    pub fn command(&mut self, id: &str, frames: i64) {
        self.commanded = Some((id.to_string(), frames));
    }

    /// Cloud cover 0..1 — how much of the sun the sky is hiding (drives shadow fade).
    pub fn cloud(&self) -> f32 {
        let sky = get(self.prev).sky * self.prev_vis + get(self.cur).sky * self.vis;
        (sky / 0.28).min(1.0)
    }

    /// A whiteout / dust storm you have to TRUDGE through (js slows): heavy snow or dust
    /// once it's really come in (vis > 0.5).
    pub fn slows(&self) -> bool {
        let d = get(self.cur);
        d.heavy && matches!(d.kind, Kind::Snow | Kind::Dust) && self.vis > 0.5
    }

    /// Precipitation intensity 0..1 (drives the water shader's chop).
    pub fn storm(&self) -> f32 {
        let mut s = 0.0;
        for (id, v) in [(self.prev, self.prev_vis), (self.cur, self.vis)] {
            let d = get(id);
            if d.kind == Kind::Rain {
                s += v * if d.heavy { 1.0 } else { 0.55 };
            }
        }
        s.min(1.0)
    }
}

/// The full-screen shader quad.
#[derive(Resource)]
struct WeatherQuad(Handle<WeatherFxMaterial>);

/// Marker on the shader quad — lets `weather_vis` hide the whole weather pass whenever
/// a menu is up (the quad sits at z 13.2, under every menu, but a translucent pause
/// backdrop or the side-drawn slide-out still let it PEEK; menus should own the screen).
#[derive(Component)]
struct WeatherFx;

/// One wind-blown leaf (js `leaves` pool — tiny squares riding the gusts).
#[derive(Component)]
struct Leaf {
    x: f32, // room px (the transform derives from these each tick)
    y: f32,
    ph: f32,
    s: f32,
}

/// Trudging through a blizzard / sandstorm keeps the SLOW status topped up (js game.js:
/// `if (Weather.slows()) player.addStatus('slow', 6)`) — indoors and underground you're
/// sheltered. The movement code already halves a slowed hero, so this is the whole effect.
fn weather_slow(
    weather: Res<WeatherState>,
    inside: Res<super::interior::Inside>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    mut statuses: ResMut<super::status::Statuses>,
) {
    if inside.0.is_none() && in_dungeon.0.is_none() && weather.slows() {
        statuses.add("slow", 6);
    }
}

/// Menus own the screen: the shader quad + every wind-leaf blink OFF whenever we leave
/// gameplay, so nothing weather-y peeks through a translucent pause backdrop or beside
/// the side-drawn slide-out (Trello: "UI menus should layer on top of weather effects").
fn weather_vis(
    screen: Res<State<super::screen::Screen>>,
    mut fx: Query<&mut Visibility, With<WeatherFx>>,
) {
    let want = if *screen.get() == super::screen::Screen::Play {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut fx {
        if *v != want {
            *v = want;
        }
    }
}

pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeatherState>()
            .add_systems(Startup, setup)
            // Gated on PLAY: menus freeze the sky whole (clock, crossfade, wind) —
            // an Update-running tick under a frozen clock is how the lurch snuck in.
            .add_systems(Update, (tick, leaves_tick).run_if(super::screen::playing))
            .add_systems(Update, weather_vis)
            .add_systems(bevy::app::FixedUpdate, weather_slow.run_if(super::screen::playing));
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WeatherFxMaterial>>,
) {
    let mat = materials.add(WeatherFxMaterial { params: WeatherFxParams::default() });
    let mut tf = at(PLAY_X, PLAY_Y, PX_W as f32, PX_H as f32, 13.2); // over the lit scene (13.0), under the HUD
    tf.scale = Vec3::new(PX_W as f32, PX_H as f32, 1.0);
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial2d(mat.clone()),
        tf,
        PIXEL_LAYER,
        WeatherFx,
    ));
    commands.insert_resource(WeatherQuad(mat));
}

fn kind_id(k: Kind) -> f32 {
    match k {
        Kind::None | Kind::Wind => 0.0, // leaves are sprites, not a shader pass
        Kind::Rain => 1.0,
        Kind::Snow => 2.0,
        Kind::Dust => 3.0,
        Kind::Fog => 4.0,
    }
}

/// The js update(): pick the target sky, crossfade, ease the wind, roll the lightning,
/// and push the uniforms.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn tick(
    mut state: ResMut<WeatherState>,
    quad: Res<WeatherQuad>,
    mut materials: ResMut<Assets<WeatherFxMaterial>>,
    clock: Res<FrameClock>,
    cur_room: Res<CurRoom>,
    world: Res<GameWorld>,
    inside: Res<super::interior::Inside>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    screen: Res<State<super::screen::Screen>>,
    settings: Res<crate::settings::Settings>,
    time: Res<Time>,
) {
    let outdoors = *screen.get() == super::screen::Screen::Play
        && inside.0.is_none()
        && in_dungeon.0.is_none();
    // A commanded sky counts down in play-time and lapses back to the sim.
    if let Some((_, left)) = &mut state.commanded {
        *left -= 1;
        if *left <= 0 {
            state.commanded = None;
        }
    }
    let want: &'static str = if !outdoors {
        "clear"
    } else if let Some(f) = &state.forced {
        crate::weather::DEFS.iter().find(|d| d.id == *f).map(|d| d.id).unwrap_or("clear")
    } else if let Some((id, _)) = &state.commanded {
        crate::weather::DEFS.iter().find(|d| d.id == *id).map(|d| d.id).unwrap_or("clear")
    } else {
        let season = ["SPRING", "SUMMER", "FALL", "WINTER"][super::codex::calendar_tab::season_index(clock.0) % 4];
        let period = clock.0.div_euclid((super::gather::DAY_LEN / 3).max(1));
        crate::weather::weather_for(world.0.biome_key_at(cur_room.rx, cur_room.ry), season, period, world.0.seed)
    };
    if want != state.cur {
        state.prev = state.cur;
        state.prev_vis = state.prev_vis.max(state.vis);
        state.cur = want;
        state.vis = 0.0;
    }
    state.vis = (state.vis + EASE).min(1.0);
    state.prev_vis = (state.prev_vis - EASE).max(0.0);
    if state.prev == state.cur {
        state.prev_vis = 0.0;
    }
    // Wind follows the INCOMING weather; everything else calms toward 0 (js targets).
    let target = match state.cur {
        "sandstorm" => 1.0,
        "windy" | "blizzard" | "thunderstorm" => 0.6,
        "snow" => 0.12,
        _ => 0.0,
    } * state.vis;
    state.wind += (target - state.wind) * 0.04;
    state.windx += state.wind * time.delta_secs();
    // Lightning (js: ~1.2% per frame past visibility 0.55; ~45% strike the ground).
    if get(state.cur).lightning && state.vis > 0.55 {
        if state.flash_cd > 0 {
            state.flash_cd -= 1;
        } else if state.rng.next_f64() < 0.012 {
            state.flash = 1.0;
            if state.rng.next_f64() < 0.45 {
                let (sx, sy) = (state.rng.next_f64() as f32 * PX_W as f32, state.rng.next_f64() as f32 * PX_H as f32);
                state.strike = Some((sx, sy));
            }
            state.flash_cd = 70 + (state.rng.next_f64() * 150.0) as i32;
        }
    }
    state.flash = (state.flash - 0.08).max(0.0);
    if state.flash <= 0.0 {
        state.strike = None; // the bolt's light dies with the flash
    }

    // Push the uniforms (REDUCE FLASHING pre-scales the whiteout).
    if let Some(mut m) = materials.get_mut(&quad.0) {
        let (a, b) = ((state.cur, state.vis), (state.prev, state.prev_vis));
        let lay = |(id, v): (&str, f32)| {
            let d = get(id);
            Vec4::new(kind_id(d.kind), v, if d.heavy { 1.0 } else { 0.0 }, 0.0)
        };
        m.params = WeatherFxParams {
            layer_a: lay(a),
            layer_b: lay(b),
            time: clock.0 as f32 / 60.0,
            wind: state.wind,
            flash: state.flash * if settings.flash { 0.24 } else { 1.0 },
            windx: state.windx,
        };
    }
}

/// The leaves on the wind (js 'wind' kind): up to 18 little squares riding the gusts.
fn leaves_tick(
    mut commands: Commands,
    state: Res<WeatherState>,
    mut leaves: Query<(Entity, &mut Leaf, &mut Transform)>,
    mut rng: Local<Option<Mulberry32>>,
) {
    let rng = rng.get_or_insert_with(|| Mulberry32::new(0x1eaf));
    let windy = (state.cur == "windy" && state.vis > 0.05) || (state.prev == "windy" && state.prev_vis > 0.05);
    let want = if windy { 18 } else { 0 };
    let have = leaves.iter().count();
    for _ in have..want {
        let (x, y) = (rng.next_f64() as f32 * PX_W as f32, rng.next_f64() as f32 * PX_H as f32);
        let col = if rng.next_f64() < 0.5 { Color::srgb_u8(0x6a, 0x8a, 0x3a) } else { Color::srgb_u8(0x9a, 0x7a, 0x3a) };
        commands.spawn((
            Sprite::from_color(col, Vec2::new(2.0, 2.0)),
            at(PLAY_X + x, PLAY_Y + y, 2.0, 2.0, 13.15),
            PIXEL_LAYER,
            WeatherFx, // hidden with the shader quad when a menu takes the screen
            Leaf { x, y, ph: rng.next_f64() as f32 * std::f32::consts::TAU, s: 2.0 + rng.next_f64() as f32 * 2.5 },
        ));
    }
    let mut extra = have as i32 - want as i32;
    for (e, mut leaf, mut tf) in &mut leaves {
        if extra > 0 {
            commands.entity(e).despawn();
            extra -= 1;
            continue;
        }
        leaf.ph += 0.15;
        leaf.x += leaf.s + state.wind * 5.0 + 1.5;
        leaf.y += leaf.ph.sin() * 1.2;
        if leaf.x > PX_W as f32 + 6.0 {
            leaf.x = -6.0;
        }
        if leaf.y > PX_H as f32 {
            leaf.y = 0.0;
        }
        if leaf.y < 0.0 {
            leaf.y = PX_H as f32;
        }
        *tf = at(PLAY_X + leaf.x, PLAY_Y + leaf.y, 2.0, 2.0, 13.15);
    }
}
