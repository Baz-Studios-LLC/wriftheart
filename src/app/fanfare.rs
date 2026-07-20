//! fanfare.rs — the "got it!" item-get cutscene (js fanfare / startFanfare / drawFanfare):
//! the FIRST time you ever collect a notable item (gear, tool, KEY, upgrade, or rare
//! tier>=2 loot) the world FREEZES and the hero holds the prize aloft — a soft warm
//! glow, twinkling sparkles, a GOT <NAME>! banner, and the itemget jingle. ~1.3s and
//! skippable. A repeat of an already-seen item just chimes (discovery is once-ever, so
//! e.g. only the FIRST dungeon key gives the moment).
//!
//! DEVIATION (flagged, minor): the js re-draws the hero at 100% over a 0.34-dimmed
//! scene; here one wash dims the frozen scene INCLUDING the hero (still clearly lit at
//! ~66%) — the prize glows above it. This dodges re-drawing the hero (which would drop
//! his worn armour for the pose). The prize is the star either way.

use crate::gfx::{at, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use crate::room::{PX_H, PX_W};
use super::play::Player;
use super::room_render::{PLAY_X, PLAY_Y};
use bevy::prelude::*;

/// ~1.3s at 60fps (js fanfare.dur).
const DUR: u32 = 80;
/// The prize drawn 2x (js S).
const S: f32 = 16.0;
/// Layer band: above the weather pass (13.2), below the banners (15.5) — a celebration
/// that the night/rain never subdues.
const Z: f32 = 13.5;

pub struct FanfareFx {
    pub id: &'static str,
    t: u32,
}

#[derive(Resource, Default)]
pub struct Fanfare(pub Option<FanfareFx>);

/// js fanfareWorthy: gear / tools / keys / upgrades / rare (tier>=2) earn the moment;
/// plain materials and common drops don't interrupt play.
pub fn worthy(id: &str) -> bool {
    let Some(def) = crate::items::get(id) else { return false };
    !def.material
        && (def.weapon
            || def.kind == "SHIELD"
            || def.slot.is_some() // worn gear / trinket (js accessory)
            || def.kind == "TOOL"
            || def.kind == "KEY"
            || def.kind == "UPGRADE"
            || def.rarity.tier() >= 2)
}

/// Should collecting `id` play the cutscene? KEYS ring out EVERY time — the classic
/// hold-aloft on each dungeon key (Baz: keys must always fanfare; `discovered` is SAVED,
/// so first-discovery-only silences them forever once you've ever held one). Everything
/// else fires once-ever, on first discovery.
pub fn should_play(id: &str, discovered: &super::codex::items_tab::Discovered) -> bool {
    worthy(id) && (crate::items::get(id).is_some_and(|d| d.kind == "KEY") || !discovered.0.contains(id))
}

/// Start the cutscene for `id` unless one is already playing (js startFanfare's `!fanfare`).
pub fn begin(fanfare: &mut Fanfare, id: &'static str) {
    if fanfare.0.is_none() {
        fanfare.0 = Some(FanfareFx { id, t: 0 });
    }
}

#[derive(Component)]
struct FanfareUi;

pub struct FanfarePlugin;
impl Plugin for FanfarePlugin {
    fn build(&self, app: &mut App) {
        // Both run OUTSIDE `playing` — the cutscene OWNS the freeze (Fanfare is folded
        // into screen::playing, so every gameplay system holds while it plays).
        app.init_resource::<Fanfare>().add_systems(Update, (fanfare_tick, fanfare_draw).chain());
    }
}

/// Advance the timer + handle the skip (js: t++ then confirm/cancel/inventory after
/// t>16, else auto-end at DUR). Fires the itemget jingle on the opening frame.
fn fanfare_tick(mut fanfare: ResMut<Fanfare>, state: Res<ActionState>, mut sfx: MessageWriter<super::sfx::Sfx>) {
    let Some(fx) = &mut fanfare.0 else { return };
    fx.t += 1;
    if fx.t == 1 {
        sfx.write(super::sfx::Sfx("itemget")); // the jingle
    }
    let skip = fx.t > 16
        && (state.pressed(Action::Interact)
            || state.pressed(Action::MenuConfirm)
            || state.pressed(Action::Slot2) // js cancel
            || state.pressed(Action::Inventory));
    if fx.t >= DUR || skip {
        fanfare.0 = None;
    }
}

/// Rebuild the overlay each frame (js drawFanfare): a soft wash over the frozen scene,
/// the prize held above the hero's head with a warm glow + orbiting sparkles, and a
/// GOT <NAME>! banner. Cheap (< 10 sprites) so a full despawn/respawn is simplest.
#[allow(clippy::too_many_arguments)]
fn fanfare_draw(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    fanfare: Res<Fanfare>,
    players: Query<&Player>,
    old: Query<Entity, With<FanfareUi>>,
    mut glow_img: Local<Option<Handle<Image>>>,
) {
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(fx) = &fanfare.0 else { return };
    let Ok(p) = players.single() else { return };
    let def = match crate::items::get(fx.id) {
        Some(d) => d,
        None => return,
    };
    let t = fx.t as f32;

    // Soften the frozen scene. js is 0.34, but it also re-draws the hero at 100% over
    // the dim; since we dim the hero WITH the scene (no re-draw), a touch more (0.5)
    // keeps the glowing prize the clear focus.
    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.5), Vec2::new(PX_W as f32, PX_H as f32)),
        at(PLAY_X, PLAY_Y, PX_W as f32, PX_H as f32, Z),
        PIXEL_LAYER,
        FanfareUi,
    ));

    // The prize rises over the hero's head and bobs (js raise + sin).
    let sx = PLAY_X + p.x.round() + 8.0; // hero centre x
    let raise = (t / 12.0).min(1.0);
    let iy = PLAY_Y + p.y.round() - 6.0 - (raise * 16.0).round() + (t / 7.0).sin().round();

    // A warm radial glow behind it (js 'lighter' gradient — a white disc tinted warm).
    let gr = 22.0;
    let glow = glow_img.get_or_insert_with(|| {
        use bevy::asset::RenderAssetUsages;
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        let d = (gr * 2.0) as u32;
        let mut img = Image::new_fill(
            Extent3d { width: d, height: d, depth_or_array_layers: 1 },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        let c = gr - 0.5;
        for y in 0..d {
            for x in 0..d {
                let dist = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
                // Squared falloff — a soft core that fades fast, not a wide bright disc
                // (Baz: "the glow was entirely too bright", worst against a dark dungeon).
                let a = (1.0 - dist / gr).max(0.0).powi(2);
                if let Ok(px) = img.pixel_bytes_mut(UVec3::new(x, y, 0)) {
                    px.copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
                }
            }
        }
        images.add(img)
    });
    let mut gs = Sprite::from_image(glow.clone());
    gs.custom_size = Some(Vec2::splat(gr * 2.0));
    gs.color = Color::srgba(1.0, 244.0 / 255.0, 200.0 / 255.0, 0.38);
    commands.spawn((gs, at(sx - gr, iy + S / 2.0 - gr, gr * 2.0, gr * 2.0, Z + 0.05), PIXEL_LAYER, FanfareUi));

    // The prize itself, 2x.
    let mut is = Sprite::from_image(images.add(crate::gfx::bake(def.icon, def.icon_pal)));
    is.custom_size = Some(Vec2::splat(S));
    commands.spawn((is, at(sx - S / 2.0, iy, S, S, Z + 0.1), PIXEL_LAYER, FanfareUi));

    // Five twinkling sparkles orbiting the prize (js loop).
    for k in 0..5 {
        let a = t / 14.0 + k as f32 * (std::f32::consts::TAU / 5.0);
        let tw = ((t / 6.0).floor() as i32 + k) % 2 == 0;
        let r2 = 12.0 + if tw { 2.0 } else { 0.0 };
        let px = (sx + a.cos() * r2).round();
        let py = (iy + S / 2.0 + a.sin() * r2).round();
        let sz = if tw { 2.0 } else { 1.0 };
        let col = if tw { Color::srgb_u8(0xff, 0xff, 0xff) } else { Color::srgb_u8(0xfc, 0xe0, 0xa8) };
        commands.spawn((Sprite::from_color(col, Vec2::splat(sz)), at(px, py, sz, sz, Z + 0.15), PIXEL_LAYER, FanfareUi));
    }

    // GOT <NAME>! banner near the top (js), a dark plate with a rarity-coloured border.
    let msg = format!("GOT {}!", def.name.to_uppercase());
    let (timg, tw) = crate::gfx::font::bake_text(&msg, def.rarity.color(), images.as_mut());
    let bw = (tw + 10) as f32;
    let bx = PLAY_X + ((PX_W as f32 - bw) / 2.0).floor();
    let by = PLAY_Y + 18.0;
    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.72), Vec2::new(bw, 12.0)),
        at(bx, by, bw, 12.0, Z + 0.2),
        PIXEL_LAYER,
        FanfareUi,
    ));
    let border = def.rarity.color();
    let bc = Color::srgb_u8((border >> 16) as u8, (border >> 8) as u8, border as u8);
    for (sx4, sy4, sw4, sh4) in crate::ui::border_strips(bx, by, bw, 12.0, 1.0) {
        commands.spawn((Sprite::from_color(bc, Vec2::new(sw4, sh4)), at(sx4, sy4, sw4, sh4, Z + 0.22), PIXEL_LAYER, FanfareUi));
    }
    let iw = (tw + (tw & 1)) as f32;
    commands.spawn((
        Sprite::from_image(timg),
        at((bx + 5.0).floor(), (by + 4.0).floor(), iw, 6.0, Z + 0.24),
        PIXEL_LAYER,
        FanfareUi,
    ));
}
