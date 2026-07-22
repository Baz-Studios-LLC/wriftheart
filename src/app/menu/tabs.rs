//! tabs.rs — the LIST tabs (GAME / VIDEO / SOUND): short centered rows, toggles show
//! their state in the label (js drawList + the game/video/sound arms of confirm()).

use super::{Area, Draw, GOLD, MUTED, TEXT_Z};
use crate::gfx::font;
use crate::settings::Settings;

pub const TITLES: [&str; 5] = ["GAME", "WIDGETS", "VIDEO", "SOUND", "CONTROLS"];

fn on_off(v: bool) -> &'static str {
    if v { "ON" } else { "OFF" }
}

/// The tab's row labels, live state baked in (js rows() + the drawList relabeling).
pub fn list_rows(tab: usize, s: &Settings, saved_flash: u32) -> Vec<String> {
    match tab {
        0 => vec![
            "RESUME".into(),
            if saved_flash > 0 { "SAVED!".into() } else { "SAVE".into() },
            format!("AUTOSAVE: {}", on_off(s.autosave)),
            "QUIT TO TITLE".into(),
            "EXIT GAME".into(),
        ],
        2 => vec![
            format!("PIXEL PERFECT: {}", on_off(s.pixel)),
            format!("SCREEN SHAKE: {}", s.shake_label()),
            format!("BRIGHTNESS: {}", s.bright_label()),
            format!("REDUCE FLASHING: {}", on_off(s.flash)),
            format!("FULLSCREEN: {}", on_off(s.fullscreen)),
        ],
        _ => vec![format!("SOUND: {}", on_off(s.sound))],
    }
}

/// Flip the selected VIDEO/SOUND setting (js confirm(), the Settings.set arms — GAME's
/// rows need screen/save access and live in mod.rs).
pub fn confirm_setting(tab: usize, index: usize, s: &mut Settings) {
    match (tab, index) {
        (2, 0) => s.pixel = !s.pixel,
        (2, 1) => s.shake = (s.shake + 1) % 3, // OFF -> LOW -> FULL
        (2, 2) => s.bright = (s.bright + 1) % 5, // DEFAULT -> +1..+4
        (2, 3) => s.flash = !s.flash,
        (2, 4) => s.fullscreen = !s.fullscreen,
        (3, 0) => s.sound = !s.sound,
        _ => {}
    }
}

/// A short, vertically-centered list; selected row gets the `>` cursor + gold (js drawList).
pub fn draw_list(d: &mut Draw, a: &Area, rows: &[String], sel: usize) {
    let rh = 18.0;
    let cx = a.x + a.w / 2.0;
    let y0 = a.y + (((a.h - rows.len() as f32 * rh) / 2.0).round()).max(0.0);
    for (i, lbl) in rows.iter().enumerate() {
        let on = i == sel;
        let y = y0 + i as f32 * rh;
        let lx = (cx - font::measure(lbl) as f32 / 2.0).round();
        if on {
            d.text(">", lx - 12.0, y, GOLD, TEXT_Z);
        }
        d.text(lbl, lx, y, if on { GOLD } else { MUTED }, TEXT_Z);
    }
}
