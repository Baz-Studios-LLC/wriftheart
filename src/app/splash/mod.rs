//! splash/ — the NEW CITY ENTERTAINMENT boot card (Baz: Wingman opens on it, so
//! does WriftHeart): black screen, the studio mark breathing in over its sting,
//! then the title. Any press skips. Both assets EMBED in the binary — the game
//! stays one file (the logo pre-scaled to canvas size, the sting a mono WAV).

use crate::gfx::{at, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use crate::{CANVAS_H, CANVAS_W};
use bevy::prelude::*;

static LOGO_PNG: &[u8] = include_bytes!("nce_logo.png");
/// Registered into the SfxBank at audio startup (pre-encoded — no bake needed).
pub static LOGO_WAV: &[u8] = include_bytes!("nce_logo.wav");

/// Pure black before the fade even starts: the macOS window spends its first
/// ~second invisible (Metal warm-up) while updates run — without this buffer the
/// whole fade-in elapsed off-screen and the logo POPPED into the first visible
/// frame (Baz). The pre-hold soaks that up; on a fast boot it reads as a beat.
const PRE_HOLD: u32 = 45;
const FADE_IN: u32 = 30;
const HOLD: u32 = 100; // the sting runs ~89 frames
const FADE_OUT: u32 = 25;
const Z: f32 = 30.0; // over everything — the card owns the boot

#[derive(Component)]
struct SplashUi;
#[derive(Component)]
struct SplashLogo;

/// The hand-off veil: black over the title, lifting — the splash FADES INTO the
/// title instead of cutting (Baz). Spawned by exit(), reaped when it clears.
#[derive(Component)]
struct TitleFade(u32);

const TITLE_FADE: u32 = 30;

#[derive(Resource, Default)]
struct SplashT(u32);

pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SplashT>()
            .add_systems(OnEnter(super::screen::Screen::Splash), enter)
            .add_systems(OnExit(super::screen::Screen::Splash), exit)
            // FixedUpdate: frame counts mean 60ths of a second on ANY display —
            // in Update a 120hz panel halved the whole card (Baz's ProMotion).
            .add_systems(bevy::app::FixedUpdate, tick.run_if(in_state(super::screen::Screen::Splash)))
            .add_systems(bevy::app::FixedUpdate, title_fade);
    }
}

fn enter(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.spawn((
        Sprite::from_color(Color::BLACK, Vec2::new(CANVAS_W as f32, CANVAS_H as f32)),
        at(0.0, 0.0, CANVAS_W as f32, CANVAS_H as f32, Z),
        PIXEL_LAYER,
        SplashUi,
    ));
    let img = Image::from_buffer(
        LOGO_PNG,
        bevy::image::ImageType::Extension("png"),
        bevy::image::CompressedImageFormats::NONE,
        true,
        bevy::image::ImageSampler::linear(),
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
    .expect("embedded splash png decodes");
    let (w, h) = (img.size().x as f32, img.size().y as f32);
    let mut spr = Sprite::from_image(images.add(img));
    spr.color = Color::srgba(1.0, 1.0, 1.0, 0.0); // fades in from black
    commands.spawn((
        spr,
        at(((CANVAS_W as f32 - w) / 2.0).floor(), ((CANVAS_H as f32 - h) / 2.0).floor(), w, h, Z + 0.1),
        PIXEL_LAYER,
        SplashUi,
        SplashLogo,
    ));
    // (The sting fires from tick() when the logo LANDS — playing it here meant
    // sound over a still-black window while the renderer warmed up.)
}

#[allow(clippy::too_many_arguments)]
fn tick(
    mut t: ResMut<SplashT>,
    state: Res<ActionState>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut next: ResMut<NextState<super::screen::Screen>>,
    mut logos: Query<&mut Sprite, With<SplashLogo>>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    frames: Res<bevy::diagnostic::FrameCount>,
    mut last_frame: Local<u32>,
) {
    // One timeline step per RENDERED frame: fixed-tick catch-up after the boot
    // stall was bursting straight through the fade-in (logo popped, sting early).
    if frames.0 == *last_frame {
        return;
    }
    *last_frame = frames.0;
    t.0 += 1;
    if t.0 == PRE_HOLD + FADE_IN {
        // The logo has landed — NOW the sting.
        sfx.write(super::sfx::Sfx("ncelogo"));
    }
    // ANY key, click, or bound pad button skips (Baz) — by jumping to the fade,
    // never a hard cut. (`pressed`/held, not just_pressed: edge flags can slip
    // between fixed ticks on high-refresh panels.)
    let skip = keys.get_pressed().next().is_some()
        || mouse.get_pressed().next().is_some()
        || state.pressed(Action::Slot1)
        || state.pressed(Action::MenuConfirm)
        || state.pressed(Action::Pause)
        || state.pressed(Action::Interact);
    if skip && t.0 < PRE_HOLD + FADE_IN + HOLD {
        t.0 = PRE_HOLD + FADE_IN + HOLD;
    }
    let a = if t.0 < PRE_HOLD {
        0.0
    } else if t.0 < PRE_HOLD + FADE_IN {
        (t.0 - PRE_HOLD) as f32 / FADE_IN as f32
    } else if t.0 < PRE_HOLD + FADE_IN + HOLD {
        1.0
    } else {
        (1.0 - (t.0 - PRE_HOLD - FADE_IN - HOLD) as f32 / FADE_OUT as f32).max(0.0)
    };
    for mut s in &mut logos {
        s.color = s.color.with_alpha(a);
    }
    if t.0 >= PRE_HOLD + FADE_IN + HOLD + FADE_OUT {
        next.set(super::screen::Screen::Title);
    }
}

fn exit(mut commands: Commands, ui: Query<Entity, With<SplashUi>>) {
    for e in &ui {
        commands.entity(e).despawn();
    }
    // Leave the veil behind: the title starts under black and surfaces.
    commands.spawn((
        Sprite::from_color(Color::BLACK, Vec2::new(CANVAS_W as f32, CANVAS_H as f32)),
        at(0.0, 0.0, CANVAS_W as f32, CANVAS_H as f32, Z),
        PIXEL_LAYER,
        TitleFade(TITLE_FADE),
    ));
}

/// Lift the hand-off veil, then reap it (runs everywhere; the query is empty
/// except the few frames after the splash).
fn title_fade(mut commands: Commands, mut veils: Query<(Entity, &mut TitleFade, &mut Sprite)>) {
    for (e, mut f, mut s) in &mut veils {
        f.0 = f.0.saturating_sub(1);
        if f.0 == 0 {
            commands.entity(e).despawn();
        } else {
            s.color = Color::BLACK.with_alpha(f.0 as f32 / TITLE_FADE as f32);
        }
    }
}
