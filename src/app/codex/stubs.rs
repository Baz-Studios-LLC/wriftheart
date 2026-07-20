//! stubs.rs — the codex tabs whose systems haven't ported: each shows its js HEADER and
//! its js TRUE EMPTY STATE (what a fresh save shows before you've found anything), so the
//! full tab strip exists 1:1 today and every tab upgrades in place when its system lands.
//!
//! PEOPLE: "NO ONE MET YET" (no villagers). GUILDS: the '? ? ?' mystery (no halls found).
//! SONGS: the songbook's '? ? ?' (no flute). AWARDS / LORE / WRIFTHEART: their headers
//! over the mystery glyph until achievements / lore books / the shard quest port.

use super::{dex::center_label, hint_scaffold, CodexState, CodexUi, TabContent, CONTENT_Z};
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::Bindings;
use crate::ui::label;
use crate::CANVAS_W;
use bevy::prelude::*;

#[derive(Component, Clone)]
pub struct StubUi;

pub fn hint(bindings: &Bindings, pad: bool) -> String {
    hint_scaffold(bindings, pad, "")
}

/// The big '? ? ?' mystery (js centerText scale 2).
fn mystery(commands: &mut Commands, images: &mut Assets<Image>, y: f32, color: u32, tag: impl Bundle + Clone) {
    let (img, tw) = font::bake_text("? ? ?", color, images);
    let iw = (tw + (tw & 1)) as f32;
    let mut s = Sprite::from_image(img);
    s.custom_size = Some(Vec2::new(iw * 2.0, 12.0));
    let cx = CANVAS_W as f32 / 2.0;
    commands.spawn((s, at((cx - iw).round(), y, iw * 2.0, 12.0, CONTENT_Z + 0.1), PIXEL_LAYER, tag));
}

/// One system per stub tab — tiny closures over the shared drawer.
macro_rules! stub_tab {
    ($fn_name:ident, $draw:expr) => {
        pub fn $fn_name(
            mut commands: Commands,
            cx_state: Res<CodexState>,
            mut images: ResMut<Assets<Image>>,
            old: Query<Entity, With<StubUi>>,
            mut seen_gen: Local<u32>,
        ) {
            if *seen_gen == cx_state.generation {
                return;
            }
            *seen_gen = cx_state.generation;
            for e in &old {
                commands.entity(e).despawn();
            }
            #[allow(clippy::type_complexity)]
            let draw: fn(&mut Commands, &mut Assets<Image>) = $draw;
            draw(&mut commands, &mut images);
        }
    };
}

const TAG: (CodexUi, TabContent, StubUi) = (CodexUi, TabContent, StubUi);

stub_tab!(people, |c, i| {
    // js drawPeopleDex empty state.
    label(c, i, "PEOPLE  0 MET", 8.0, 16.0, 0xbfb9a0, CONTENT_Z + 0.1, TAG);
    center_label(c, i, "NO ONE MET YET", CANVAS_W as f32 / 2.0, 96.0, 0x8a8a92, CONTENT_Z + 0.1, TAG);
    center_label(c, i, "GO SAY HELLO IN TOWN", CANVAS_W as f32 / 2.0, 108.0, 0x5a5a62, CONTENT_Z + 0.1, TAG);
});

stub_tab!(guilds, |c, i| {
    // js drawGuildsDex undiscovered state — a mystery until you step into a city's hall.
    label(c, i, "GUILDS", 8.0, 16.0, 0xbfb9a0, CONTENT_Z + 0.1, TAG);
    mystery(c, i, 88.0, 0x5a5a62, TAG);
});

stub_tab!(songs, |c, i| {
    // js drawSongsDex: the illuminated title + the flute-less mystery.
    let mid = CANVAS_W as f32 / 2.0;
    center_label(c, i, "THE SONGBOOK", mid, 16.0, 0xe8c860, CONTENT_Z + 0.1, TAG);
    let tw = font::measure("THE SONGBOOK") as f32;
    for sgn in [-1.0f32, 1.0] {
        let fx = mid + sgn * (tw / 2.0 + 12.0);
        c.spawn((
            Sprite::from_color(Color::srgb_u8(0xe8, 0xc8, 0x60), Vec2::new(3.0, 1.0)),
            at(fx - 1.0, 20.0, 3.0, 1.0, CONTENT_Z + 0.1),
            PIXEL_LAYER,
            TAG,
        ));
        c.spawn((
            Sprite::from_color(Color::srgb_u8(0xe8, 0xc8, 0x60), Vec2::new(1.0, 3.0)),
            at(fx, 19.0, 1.0, 3.0, CONTENT_Z + 0.1),
            PIXEL_LAYER,
            TAG,
        ));
        c.spawn((
            Sprite::from_color(Color::srgba(232.0 / 255.0, 200.0 / 255.0, 96.0 / 255.0, 0.3), Vec2::new(56.0, 1.0)),
            at(if sgn < 0.0 { fx - 60.0 } else { fx + 4.0 }, 20.0, 56.0, 1.0, CONTENT_Z + 0.1),
            PIXEL_LAYER,
            TAG,
        ));
    }
    mystery(c, i, 90.0, 0x5a5e6a, TAG);
});

stub_tab!(awards, |c, i| {
    // The HALL OF DEEDS opens with the achievements port.
    label(c, i, "THE HALL OF DEEDS", 8.0, 16.0, 0xe8c860, CONTENT_Z + 0.1, TAG);
    mystery(c, i, 88.0, 0x5a5a62, TAG);
});

stub_tab!(lore, |c, i| {
    // The tome shelf fills when the lore books port.
    label(c, i, "TOMES  0 / 100", 8.0, 16.0, 0xc8b0e8, CONTENT_Z + 0.1, TAG);
    mystery(c, i, 88.0, 0x5a5a62, TAG);
});

stub_tab!(wriftheart, |c, i| {
    // The assembling heart — the main quest's page; shards port with the relic system.
    center_label(c, i, "THE WRIFTHEART", CANVAS_W as f32 / 2.0, 16.0, 0xc060fc, CONTENT_Z + 0.1, TAG);
    mystery(c, i, 88.0, 0x5a5a62, TAG);
    center_label(c, i, "0 OF 10 SHARDS RECLAIMED", CANVAS_W as f32 / 2.0, 120.0, 0x8a8a92, CONTENT_Z + 0.1, TAG);
});
