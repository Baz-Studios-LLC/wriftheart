//! canvas.rs — the low-resolution render target + the JS->Bevy coordinate bridge.
//!
//! The JS original paints immediate-mode into a 384x216 canvas (ours is 384x208 — see
//! CANVAS_H's deviation note). Here that canvas is a render
//! target: gameplay draws at 1 texture pixel = 1 game pixel on [`PIXEL_LAYER`], then a second
//! camera blits it to the window at an INTEGER scale so nothing lands on a half-pixel. Pattern:
//! bevy's `examples/2d/pixel_grid_snap.rs`.

use crate::{CANVAS_H, CANVAS_W};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::window::{MonitorSelection, WindowMode};

/// Everything drawn at game resolution.
pub const PIXEL_LAYER: RenderLayers = RenderLayers::layer(0);
/// The upscaled canvas itself (and anything that wants real window pixels).
pub const HIGH_RES_LAYER: RenderLayers = RenderLayers::layer(1);

/// Blits the canvas to the window at an integer scale.
#[derive(Component)]
struct OuterCamera;

/// Drop-in Bevy plugin: sets up the two-camera pixel-perfect canvas and keeps it integer-scaled.
pub struct PixelCanvasPlugin;

impl Plugin for PixelCanvasPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, (fit_canvas, apply_fullscreen));
    }
}

/// JS screen-space -> canvas-space.
///
/// THE PORT'S SHARPEST EDGE: the JS canvas has its origin at the TOP-LEFT with +Y pointing
/// DOWN, drawing sprites from their top-left corner. Bevy 2D has its origin at the CENTRE with
/// +Y pointing UP, anchoring sprites at their CENTRE. Every ported coordinate goes through here
/// — pass the JS x/y and the sprite size, get a Bevy transform back.
pub fn at(x: f32, y: f32, w: f32, h: f32, z: f32) -> Transform {
    Transform::from_xyz(
        x - CANVAS_W as f32 / 2.0 + w / 2.0,
        CANVAS_H as f32 / 2.0 - y - h / 2.0,
        z,
    )
}

fn setup_camera(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size = Extent3d { width: CANVAS_W, height: CANVAS_H, ..default() };

    let mut canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("wriftheart_canvas"),
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    canvas.resize(size);
    let handle = images.add(canvas);

    // Pass 1: the game, at game resolution, into the canvas texture.
    // (Msaa stays OFF: Sample4 on a render-target camera renders black intermittently on
    // Metal — soft edges for rotated quads come from linearly-sampled edge-fade textures
    // instead; see skills_tab's line strip.)
    //
    // NO BLOOM either — tried and reverted (2026-07-16): bevy's first bloom downsample
    // karis-averages (firefly suppression) BEFORE the prefilter threshold, crushing every
    // value toward <=1 — so a threshold >=1.0 passes nothing, and the tiny emissive
    // sparkles we wanted are exactly the "fireflies" it exists to kill. Selective
    // per-sprite glow via Bloom is a structural dead end at 384x216; glows are BAKED
    // radial sprites instead (gather.rs).
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(Color::srgb_u8(0x0a, 0x0a, 0x0e)),
            ..default()
        },
        RenderTarget::Image(handle.clone().into()),
        Msaa::Off,
        PIXEL_LAYER,
    ));

    // Pass 2: the canvas sprite, upscaled to the window.
    commands.spawn((Sprite::from_image(handle), HIGH_RES_LAYER));
    commands.spawn((Camera2d, Msaa::Off, OuterCamera, HIGH_RES_LAYER));
}

/// Scale the canvas to the window per the PIXEL PERFECT setting (js game.js:6070):
/// ON = the largest whole multiple that still FITS (floor — rounding up would crop),
/// OFF = fill the tight axis edge-to-edge, fractional scale allowed (the js default).
/// Runs every frame (a compare + rare write) so a menu toggle applies instantly.
fn fit_canvas(
    windows: Query<&Window>,
    settings: Res<crate::settings::Settings>,
    mut projection: Single<&mut Projection, With<OuterCamera>>,
) {
    let Ok(window) = windows.single() else { return };
    let Projection::Orthographic(projection) = &mut **projection else {
        return;
    };
    let fit = (window.width() / CANVAS_W as f32).min(window.height() / CANVAS_H as f32);
    let scale = if settings.pixel { fit.floor().max(1.0) } else { fit.max(0.01) };
    if (projection.scale - 1.0 / scale).abs() > 1e-6 {
        projection.scale = 1.0 / scale;
    }
}

/// Apply the menu's FULLSCREEN toggle to the window. (DEVIATION from the js, with Baz's
/// improve-don't-copy blessing pending: the js could only PRINT the OS fullscreen shortcut
/// — its webview couldn't act; a native window can, so the row really toggles.)
fn apply_fullscreen(settings: Res<crate::settings::Settings>, mut windows: Query<&mut Window>) {
    if !settings.is_changed() {
        return;
    }
    let want = if settings.fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
    for mut w in &mut windows {
        if w.mode != want {
            w.mode = want;
        }
    }
}
