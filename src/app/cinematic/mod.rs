//! cinematic/ — THE OPENING (js drawCutscene), now the js's FULL CANVAS PAINTINGS:
//! every new game starts with the story, six scenes over ~24 seconds, skippable
//! with a key. The Whole Age and the heart that held it; the Sundering — the
//! silhouette, the blade, the shatter into TEN shards; ten shards sinking into
//! ten lands; TONIGHT, the Choir on the hill calling the ember down; the cottage
//! — asleep, jolted awake, up and OUT; and EMBERFALL ABLAZE — fleeing villagers
//! screaming in word bubbles while one grey-robed figure stands perfectly still.
//! Painted per-frame into one CPU canvas (paint.rs) exactly like the js
//! immediate-mode ctx — same gradients, frames, sfx cues, era cards and fades.

// Public so examples/cutscene_frames.rs can render frames HEADLESSLY (the WRIFT_SHOT
// window capture blacks out under macOS occlusion whenever another app holds focus).
pub mod paint;
pub mod scenes;

use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use paint::Cv;

use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState};

pub const LEN: u32 = 1480; // js CUTSCENE_LEN
pub const W: i32 = crate::CANVAS_W as i32;
pub const H: i32 = crate::CANVAS_H as i32;
const Z: f32 = 20.3; // above play, below the win overlay

/// Some(t) while the opening plays — the world underneath waits.
#[derive(Resource, Default)]
pub struct Cutscene(pub Option<u32>);

/// A fleeing villager (js cutscene.npcs).
pub struct Npc {
    pub x: f32,
    pub base_y: f32,
    pub spd: f32,
    pub col: u32,
    pub line: &'static str,
    pub dir: f32,
    pub always: bool,
    pub ph: f32,
}

/// Everything staged at cutscene start (js initCutscene): the canvas + its sprite,
/// the star field, the TEN shards, the fleeing villagers, and every pre-baked text.
pub struct CsInner {
    pub canvas: Handle<Image>,
    pub sprite: Entity,
    pub stars: Vec<(f32, f32, f32)>,
    /// Exactly one per biome relic: (colour, fling angle, fling speed) — TEN, never more.
    pub shards: Vec<(u32, f32, f32)>,
    pub npcs: Vec<Npc>,
    pub texts: HashMap<String, (Handle<Image>, i32)>,
    pub hero: Handle<Image>,
    /// REAL seeded villager sprites for the fleeing townsfolk (one walk-frame set
    /// each) — the js drew little abstract runners, but on the dark blazing ground
    /// they read as blobs, not people (Baz). Real folk read as folk.
    pub folk: Vec<[[Handle<Image>; 4]; 4]>,
    pub well: Handle<Image>,
    pub torch: [Handle<Image>; 2],
    /// (kind, x, y, front art) — the js town layout, the hero's cottage last.
    pub fronts: Vec<(&'static str, i32, i32, Option<Handle<Image>>)>,
    pub skip_hint: String,
}

#[derive(Resource, Default)]
pub struct CsState(pub Option<CsInner>);

/// The era cards (js csEraCard) + the closing line.
const CARDS: [(&str, u32); 4] = [
    ("LONG AGO - THE WHOLE AGE", 16),
    ("THE SUNDERING - 312 YEARS AGO", 278),
    ("TEN SHARDS - TEN LANDS", 526),
    ("PRESENT DAY - THE YEAR 312 A.S.", 736),
];
const FINAL_LINE: &str = "THE WRIFTHEART MUST BE MADE WHOLE";
const NPC_LINES: [&str; 6] = ["THE OLD DOOM COMES AGAIN!", "RUN!", "FIRE!", "FLEE!", "THE SKY TORE OPEN!", "HELP US!"];

fn teardown(commands: &mut Commands, images: &mut Assets<Image>, state: &mut CsState) {
    if let Some(inner) = state.0.take() {
        commands.entity(inner.sprite).despawn();
        images.remove(&inner.canvas);
        for (_, (h, _)) in inner.texts {
            images.remove(&h);
        }
        for set in inner.folk {
            for row in set {
                for h in row {
                    images.remove(&h);
                }
            }
        }
    }
}

/// The frame driver: paints the current scene into the canvas, uploads it, rides
/// the js sfx cues, shakes on the js frames, and skips on any key.
#[allow(clippy::too_many_arguments)]
fn cutscene_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut cs: ResMut<Cutscene>,
    mut state: ResMut<CsState>,
    mut input: ResMut<ActionState>,
    prop_art: Res<crate::actors::props::PropArt>,
    hero_art: Res<crate::app::play::HeroArt>,
    world: Res<crate::app::play::GameWorld>,
    settings: Res<crate::settings::Settings>,
    bindings: Res<crate::input::Bindings>,
    mut rng: ResMut<super::battle::GameRng>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut sprites: Query<&mut Transform, With<CsCanvas>>,
) {
    let Some(t) = &mut cs.0 else {
        // Something else dropped the cutscene (quit-to-title) — sweep our stage too.
        if state.0.is_some() {
            teardown(&mut commands, &mut images, &mut state);
        }
        return;
    };
    let tv = *t;
    *t += 1;

    // --- Skip / finish (js: ANY key skips; consumed so the press can't open a menu). ---
    const SKIP_KEYS: [Action; 6] =
        [Action::Slot2, Action::Interact, Action::Pause, Action::Slot1, Action::Map, Action::Inventory];
    if SKIP_KEYS.iter().any(|a| input.pressed(*a)) || tv >= LEN {
        for a in SKIP_KEYS {
            input.consume(a);
        }
        cs.0 = None;
        teardown(&mut commands, &mut images, &mut state);
        sfx.write(super::sfx::Sfx("open"));
        return;
    }

    // --- The js sound cues, frame for frame. ---
    if let Some(c) = match tv {
        10 | 50 | 90 | 130 | 170 | 210 => Some("heartbeat"), // the Whole Age, alive and easy
        270 | 309 | 348 => Some("heartbeat"),                // the last beats, on the fateful night
        400 | 406 => Some("hurt"),                           // the shatter
        750 | 852 => Some("warpCharge"),                     // the hymn swells
        928 => Some("hurt"),                                 // the ember, called down
        1010 => Some("thunder"),                             // the boom that jolts the hero awake
        _ => None,
    } {
        sfx.write(super::sfx::Sfx(c));
    }

    // --- First tick: stage the canvas + the js initCutscene state. ---
    if state.0.is_none() {
        let canvas = images.add(Image::new(
            Extent3d { width: W as u32, height: H as u32, depth_or_array_layers: 1 },
            TextureDimension::D2,
            vec![0; (W * H * 4) as usize],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        let sprite = commands
            .spawn((Sprite::from_image(canvas.clone()), at(0.0, 0.0, W as f32, H as f32, Z), PIXEL_LAYER, CsCanvas))
            .id();
        let mut stars = Vec::new();
        for _ in 0..70 {
            stars.push((
                rng.0.next_f64() as f32 * W as f32,
                rng.0.next_f64() as f32 * (H as f32 * 0.72),
                0.25 + rng.0.next_f64() as f32 * 0.7,
            ));
        }
        // The TEN shards — js worldShards(): THIS world's ten shard biomes, their relic
        // colours. (Iterating the full relics CATALOG here was the old "a hundred
        // shards" bug — the catalog holds every biome's relic, not the world's ten.)
        let biomes = world.0.shard_biomes();
        let n = biomes.len().max(1);
        let shards: Vec<(u32, f32, f32)> = biomes
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let col = crate::relics_data::by_biome(b).map_or(0xb060f0, |r| r.col);
                (col, (i as f32 / n as f32) * std::f32::consts::TAU + 0.3, 1.4 + (i % 4) as f32 * 0.4)
            })
            .collect();
        let looks = [0x3cdc5au32, 0x4a9cff, 0xc060fc, 0xfc7460, 0xe0c040, 0x50c0a0];
        let npcs = (0..6)
            .map(|i| Npc {
                x: 24.0 + i as f32 * 60.0,
                base_y: 104.0 + (i % 3) as f32 * 30.0,
                spd: 0.8 + rng.0.next_f64() as f32 * 0.9,
                col: looks[i % looks.len()],
                line: NPC_LINES[i % NPC_LINES.len()],
                dir: if i % 2 == 1 { 1.0 } else { -1.0 },
                always: i < 2,
                ph: i as f32 * 4.0,
            })
            .collect();
        // Pre-bake every string the scenes draw (blitted later with live alpha).
        let mut texts = HashMap::default();
        let mut bake = |s: &str, col: u32, images: &mut Assets<Image>| {
            let (h, w) = font::bake_text(s, col, images);
            texts.insert(s.to_string(), (h, w));
        };
        for (card, _) in CARDS {
            bake(card, 0xd8dce8, &mut images);
        }
        bake(FINAL_LINE, 0xffd8c8, &mut images);
        for line in NPC_LINES {
            bake(line, 0xffd0d0, &mut images);
        }
        bake("Z Z Z", 0x8a9ab0, &mut images);
        bake("!", 0xffd0c0, &mut images);
        let skip_hint = format!("{} SKIP", bindings.prompt(Action::Slot2, input.pad_present));
        bake(&skip_hint, 0x6a6a72, &mut images);
        // The js town layout, back rows first; the hero's cottage stands nearest.
        let fronts = [
            ("inn", 52, 68),
            ("store", 150, 58),
            ("blacksmith", 252, 68),
            ("tavern", 96, 150),
            ("bakery", 330, 150),
            ("cottage", 206, 176),
        ]
        .into_iter()
        .map(|(k, x, y)| (k, x, y, prop_art.fronts.get(k).cloned()))
        .collect();
        // Six real townsfolk, seeded looks, full walk frames.
        let folk = (0..6u32)
            .map(|i| crate::actors::hero::build_frames(&crate::actors::hero::random_look(i * 7919 + 3), &mut images).frames)
            .collect();
        state.0 = Some(CsInner {
            canvas,
            sprite,
            stars,
            shards,
            npcs,
            texts,
            hero: hero_art.0.frames[0][0].clone(),
            folk,
            well: prop_art.well.clone(),
            torch: prop_art.torch.clone(),
            fronts,
            skip_hint,
        });
    }
    let st = state.0.as_ref().expect("staged above");

    // --- Paint this frame (the js drawCutscene dispatch, y-down canvas coords). ---
    let mut cv = Cv::new(W, H);
    let tf = tv as f32;
    let hero_img = images.get(&st.hero);
    match tv {
        0..=259 => scenes::whole_age(&mut cv, tf),
        260..=499 => scenes::sky(&mut cv, st, tf - 260.0),
        500..=719 => scenes::scatter(&mut cv, st, tf - 500.0),
        720..=959 => scenes::choir(&mut cv, st, tf - 720.0),
        960..=1159 => {
            if let Some(hero) = hero_img {
                scenes::interior(&mut cv, hero, tf - 960.0);
            }
            // The js 'z z z' / '!' over the bed (scale-2 bang, exactly the js frames).
            let lt = tf - 960.0;
            let (bx, by) = (40 + 2 * 16 + 22, 4 + 2 * 16 + 12);
            if lt < 54.0 {
                scenes::text(&mut cv, &images, st, "Z Z Z", bx + 14, by - 12, 0.5 + 0.3 * (lt * 0.1).sin(), 1);
            } else if lt < 100.0 {
                scenes::text(&mut cv, &images, st, "!", bx + 13, by - 18, 1.0, 2);
            }
        }
        _ => scenes::town(&mut cv, &images, st, hero_img, tf - 1160.0),
    }
    // Era cards fade in, hold, fade out (js csEraCard); the closing line rises last.
    for (card, t0) in CARDS {
        let lt = tv as i32 - t0 as i32;
        if (0..=170).contains(&lt) {
            let a = (lt as f32 / 30.0).min(1.0) * ((170 - lt) as f32 / 30.0).min(1.0);
            scenes::text_c(&mut cv, &images, st, card, W / 2, 26, a, 1);
        }
    }
    if tv > 1300 {
        scenes::text_c(&mut cv, &images, st, FINAL_LINE, W / 2, 22, ((tv - 1300) as f32 / 40.0).min(1.0), 1);
    }
    if tv > LEN - 24 {
        cv.rect(0, 0, W, H, 0x000000, (tv - (LEN - 24)) as f32 / 24.0); // fade to play
    }
    let hint_w = st.texts.get(&st.skip_hint).map_or(0, |(_, w)| *w);
    let hint = st.skip_hint.clone();
    scenes::text(&mut cv, &images, st, &hint, W - 4 - hint_w, H - 12, 1.0, 1);

    // Upload + the js screen shakes (whole-canvas: the sprite jitters).
    let canvas_handle = st.canvas.clone();
    if let Some(mut img) = images.get_mut(&canvas_handle) {
        img.data = Some(cv.px);
    }
    let shake = match tv {
        400..=435 => ((tv as f32 * 1.9).sin() * 5.0 * (1.0 - (tv - 400) as f32 / 36.0) * settings.shake_mul()).round(), // the shatter
        1010..=1051 => ((tv as f32 * 1.6).sin() * (4.5 - (tv - 1010) as f32 * 0.11).max(1.0) * settings.shake_mul()).round(), // the boom
        1160.. => ((tv as f32 * 0.8).sin() * 1.4 * settings.shake_mul()).round(), // the blaze, rumbling on
        _ => 0.0,
    };
    if let Ok(mut tfm) = sprites.single_mut() {
        let base = at(0.0, 0.0, W as f32, H as f32, Z);
        tfm.translation = base.translation + Vec3::new(shake, -(shake / 2.0).floor(), 0.0);
    }
}

/// Marker on the one full-screen canvas sprite.
#[derive(Component)]
struct CsCanvas;

pub struct CinematicPlugin;

impl Plugin for CinematicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Cutscene>().init_resource::<CsState>().add_systems(
            bevy::app::FixedUpdate,
            cutscene_tick
                .before(super::play::EndTick)
                .before(super::menu::menu_tick) // a skip press must be consumed before the openers see it
                .run_if(super::screen::playing),
        );
    }
}
