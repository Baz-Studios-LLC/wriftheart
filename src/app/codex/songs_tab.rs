//! songs_tab.rs — the SONGS codex page (js drawSongsPage): the ONLY place a song's
//! notes are ever written down. Learned songs show their name, the four note-arrows in
//! the notes' own colours, and what the song does; unlearned ones keep the mystery —
//! a grey '? ? ?' and the rumour of where the tune might be found.

use super::{hint_scaffold, CodexState, CodexUi, TabContent, CONTENT_Z};
use crate::app::flute::LearnedSongs;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::Bindings;
use crate::songs;
use crate::ui::label;
use bevy::prelude::*;

#[derive(Component, Clone)]
pub struct SongsUi;

pub fn hint(bindings: &Bindings, pad: bool) -> String {
    hint_scaffold(bindings, pad, "") // informational — no browsing
}

pub fn run(
    mut commands: Commands,
    cx_state: Res<CodexState>,
    learned: Res<LearnedSongs>,
    mut images: ResMut<Assets<Image>>,
    old: Query<Entity, With<SongsUi>>,
    mut seen_gen: Local<u32>,
) {
    if *seen_gen == cx_state.generation {
        return; // static while open (nothing teaches songs from inside the codex)
    }
    *seen_gen = cx_state.generation;
    for e in &old {
        commands.entity(e).despawn();
    }
    let tag = || (CodexUi, TabContent, SongsUi);

    label(&mut commands, &mut images, "THE SONGBOOK", 12.0, 17.0, 0xd8b8ff, CONTENT_Z + 0.1, tag());
    let known = learned.0.len();
    let count = format!("{known}/{} LEARNED", songs::LIST.len());
    let cw = font::measure(&count) as f32;
    label(&mut commands, &mut images, &count, crate::CANVAS_W as f32 - 12.0 - cw, 17.0, 0x8a8a92, CONTENT_Z + 0.1, tag());

    // One white arrow bake per direction, tinted per note at spawn.
    let mk = |ltr: char, images: &mut Assets<Image>| {
        use bevy::asset::RenderAssetUsages;
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        let mut img = Image::new_fill(
            Extent3d { width: 7, height: 7, depth_or_array_layers: 1 },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        for (x, y) in songs::arrow_cells(ltr) {
            if let Ok(px) = img.pixel_bytes_mut(UVec3::new(x as u32, y as u32, 0)) {
                px.copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        images.add(img)
    };
    let arrows = [mk('U', &mut images), mk('D', &mut images), mk('L', &mut images), mk('R', &mut images)];

    let (x0, y0, row_h) = (12.0, 30.0, 21.0);
    for (i, s) in songs::LIST.iter().enumerate() {
        let y = y0 + i as f32 * row_h;
        if learned.0.contains(s.id) {
            label(&mut commands, &mut images, s.name, x0, y, s.col, CONTENT_Z + 0.1, tag());
            // The notes, written down at last — each arrow in its note's colour.
            for (j, ltr) in s.notes.chars().enumerate() {
                let n = &songs::NOTES[songs::note_idx(ltr)];
                let mut spr = Sprite::from_image(arrows[songs::note_idx(ltr)].clone());
                spr.color = Color::srgb_u8((n.col >> 16) as u8, (n.col >> 8) as u8, n.col as u8);
                commands.spawn((
                    spr,
                    at(crate::CANVAS_W as f32 - 12.0 - (4.0 - j as f32) * 10.0, y - 1.0, 7.0, 7.0, CONTENT_Z + 0.1),
                    PIXEL_LAYER,
                    tag(),
                ));
            }
            label(&mut commands, &mut images, s.desc, x0 + 6.0, y + 9.0, 0x9aa0aa, CONTENT_Z + 0.1, tag());
        } else {
            label(&mut commands, &mut images, "? ? ?", x0, y, 0x5a5a62, CONTENT_Z + 0.1, tag());
            label(&mut commands, &mut images, s.hint, x0 + 6.0, y + 9.0, 0x50505a, CONTENT_Z + 0.1, tag());
        }
    }
}
