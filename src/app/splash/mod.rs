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

const FADE_IN: u32 = 30;
const HOLD: u32 = 100; // the sting runs ~89 frames
const FADE_OUT: u32 = 25;
const Z: f32 = 30.0; // over everything — the card owns the boot

#[derive(Component)]
struct SplashUi;
#[derive(Component)]
struct SplashLogo;

#[derive(Resource, Default)]
struct SplashT(u32);

pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SplashT>()
            .add_systems(OnEnter(super::screen::Screen::Splash), enter)
            .add_systems(OnExit(super::screen::Screen::Splash), exit)
            .add_systems(Update, tick.run_if(in_state(super::screen::Screen::Splash)));
    }
}

fn enter(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut sfx: MessageWriter<super::sfx::Sfx>) {
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
    sfx.write(super::sfx::Sfx("ncelogo"));
}

fn tick(
    mut t: ResMut<SplashT>,
    state: Res<ActionState>,
    mut next: ResMut<NextState<super::screen::Screen>>,
    mut logos: Query<&mut Sprite, With<SplashLogo>>,
) {
    t.0 += 1;
    let a = if t.0 < FADE_IN {
        t.0 as f32 / FADE_IN as f32
    } else if t.0 < FADE_IN + HOLD {
        1.0
    } else {
        (1.0 - (t.0 - FADE_IN - HOLD) as f32 / FADE_OUT as f32).max(0.0)
    };
    for mut s in &mut logos {
        s.color = s.color.with_alpha(a);
    }
    let skip = state.pressed(Action::Slot1)
        || state.pressed(Action::MenuConfirm)
        || state.pressed(Action::Pause)
        || state.pressed(Action::Interact);
    if t.0 >= FADE_IN + HOLD + FADE_OUT || skip {
        next.set(super::screen::Screen::Title);
    }
}

fn exit(mut commands: Commands, ui: Query<Entity, With<SplashUi>>) {
    for e in &ui {
        commands.entity(e).despawn();
    }
}
