//! crawl.rs — the title's attract mode (js drawCrawl): sit idle ~7s and the WRIFTHEART
//! story scrolls up over the flyover, looping, until any input returns to the menu.
//! The text is the js Relics.STORY, verbatim.

use super::{Pen, TitleState, TitleUi};
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::{CANVAS_H, CANVAS_W};
use bevy::prelude::*;

pub(super) const CRAWL_SPEED: f32 = 0.18; // px/frame — slow, readable
pub(super) const IDLE_MAX: u32 = 420; // ~7s of stillness starts the story

const TITLE_SC: f32 = 3.0;
const BODY_SC: f32 = 2.0;

/// js Relics.STORY — the opening lore, shown here and nowhere else until relics port.
const STORY: [&str; 6] = [
    "Long ago, the world was whole.",
    "It was held together by the Wriftheart, a single gem at the heart of all creation.",
    "Then it shattered. From the wound bled Chaos, warping the farthest lands and twisting their beasts into corrupted things.",
    "But the Wriftheart did not die. It broke into ten shards, and each sank into the heart of a different land, taking on its nature.",
    "The old maps whisper that one who gathers all ten shards may mend the Wriftheart... or claim the power that remains within it.",
    "Seek out the shattered pieces of the Wriftheart, and face what waits beyond the wound.",
];

/// One scrolling line: its centred x, cumulative offset down the scroll, and scale.
#[derive(Component)]
pub struct CrawlLine {
    x: f32,
    off: f32,
    scale: f32,
}

/// Word-wrap at a pixel width for a given text scale (js wrap()).
fn wrap(text: &str, max_w: f32, scale: f32) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for w in text.split(' ') {
        let try_line = if line.is_empty() { w.to_string() } else { format!("{line} {w}") };
        if font::measure(&try_line) as f32 * scale > max_w && !line.is_empty() {
            out.push(line);
            line = w.to_string();
        } else {
            line = try_line;
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

fn line_h(text: &str, scale: f32) -> f32 {
    if text.is_empty() { 8.0 } else { 5.0 * scale + 5.0 }
}

/// Build the scroll: dim layer + every line as a sprite the scroller repositions.
/// Sets `state.crawl_h` (the loop length, js crawlH).
pub(super) fn spawn(pen: &mut Pen, state: &mut TitleState) {
    let (w, h) = (CANVAS_W as f32, CANVAS_H as f32);
    // Deepen the backdrop for readability (js rgba(2,4,8,0.55); +linear bump).
    pen.fill_rgba(0.0, 0.0, w, h, Color::srgba(0.008, 0.016, 0.03, 0.66), super::CRAWL_DIM_Z);
    pen.text_center("PRESS ANY KEY", w / 2.0, h - 12.0, 0x5a6a5a, super::TEXT_Z);

    let mut items: Vec<(String, f32)> = vec![("WRIFTHEART".into(), TITLE_SC), (String::new(), BODY_SC)];
    for para in STORY {
        for ln in wrap(para, w - 36.0, BODY_SC) {
            items.push((ln, BODY_SC));
        }
        items.push((String::new(), BODY_SC));
    }
    let mut off = 0.0;
    for (text, scale) in &items {
        if !text.is_empty() {
            let lw = font::measure(text) as f32 * scale;
            let x = ((w - lw) / 2.0).round();
            let color = if *scale >= TITLE_SC { 0xfce0a8 } else { 0xdfeae2 };
            let (img, bw) = font::bake_text(text, color, pen.images);
            let iw = (bw + (bw & 1)) as f32;
            pen.commands.spawn((
                Sprite { image: img, custom_size: Some(Vec2::new(iw * scale, 6.0 * scale)), ..default() },
                at(x, h + off, iw * scale, 6.0 * scale, super::TEXT_Z),
                PIXEL_LAYER,
                Visibility::Hidden,
                CrawlLine { x, off, scale: *scale },
                TitleUi,
            ));
        }
        off += line_h(text, *scale);
    }
    state.crawl_h = off + h; // full scroll distance before the loop restarts
    state.crawl_t = 0.0;
}

/// Scroll + edge-fade every line from the shared timer (runs each fixed tick while
/// crawling; js recomputes y and alpha the same way per frame).
pub(super) fn scroll(state: &TitleState, mut lines: Query<(&CrawlLine, &mut Transform, &mut Sprite, &mut Visibility)>) {
    let h = CANVAS_H as f32;
    for (line, mut tf, mut sprite, mut vis) in &mut lines {
        let y = h - state.crawl_t * CRAWL_SPEED + line.off;
        let lh = 6.0 * line.scale;
        if y < -lh || y > h {
            *vis = Visibility::Hidden;
            continue;
        }
        let a = ((y - 14.0).min(h - 24.0 - y) / 28.0).clamp(0.0, 1.0);
        if a <= 0.01 {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;
        sprite.color = Color::srgba(1.0, 1.0, 1.0, a);
        let w = sprite.custom_size.map_or(0.0, |s| s.x);
        *tf = at(line.x, y.round(), w, lh, super::TEXT_Z);
    }
}
