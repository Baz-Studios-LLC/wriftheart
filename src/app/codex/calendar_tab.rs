//! calendar_tab.rs — the CALENDAR (js drawCalendar): the season's big title + year, the
//! day counter, and the 28-day month laid out 7 x 4 with weekday letters — today ringed
//! in the season's colour, past days dimmed.
//!
//! Not here yet (their systems haven't ported): festival pennants/countdown, friends'
//! birthdays, and the IN SEASON crop column — which therefore shows the js's own empty
//! state, "FIELDS LIE FALLOW."

use super::{hint_scaffold, CodexState, CodexUi, TabContent, CONTENT_Z};
use crate::app::gather::{day_number, DAY_LEN};
use crate::app::room_render::FrameClock;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::Bindings;
use crate::ui::{frame_rect, label};
use crate::CANVAS_H;
use bevy::prelude::*;

pub const SEASON_LEN: i64 = 28; // in-game days per season (a 28-day month)
pub const SEASONS: [&str; 4] = ["SPRING", "SUMMER", "FALL", "WINTER"];
const SEASON_COL: [u32; 4] = [0x7ee08a, 0xffd34d, 0xe0903a, 0xbfe0ff];

pub fn season_index(clock: i64) -> usize {
    ((day_number(clock) / SEASON_LEN) % 4) as usize
}
pub fn day_of_season(clock: i64) -> i64 {
    day_number(clock) % SEASON_LEN + 1 // 1..SEASON_LEN
}

#[derive(Component, Clone)]
pub struct CalendarUi;

pub fn hint(bindings: &Bindings, pad: bool) -> String {
    hint_scaffold(bindings, pad, "") // informational — no browsing
}

pub fn run(
    mut commands: Commands,
    cx_state: Res<CodexState>,
    clock: Res<FrameClock>,
    mut images: ResMut<Assets<Image>>,
    old: Query<Entity, With<CalendarUi>>,
    mut seen_gen: Local<u32>,
) {
    if *seen_gen == cx_state.generation {
        return; // static while open (the clock is frozen in menus)
    }
    *seen_gen = cx_state.generation;
    for e in &old {
        commands.entity(e).despawn();
    }
    let tag = || (CodexUi, TabContent, CalendarUi);

    let s_idx = season_index(clock.0);
    let s_col = SEASON_COL[s_idx];
    let year = day_number(clock.0) / (SEASON_LEN * 4) + 1;
    let cur = day_of_season(clock.0);
    let _ = DAY_LEN; // (day_number already owns the frame math)

    // Big season title (js scale 2) + the day counter.
    let title = format!("{} - YEAR {year}", SEASONS[s_idx]);
    let (img, tw) = font::bake_text(&title, s_col, &mut images);
    let iw = (tw + (tw & 1)) as f32;
    let mut big = Sprite::from_image(img);
    big.custom_size = Some(Vec2::new(iw * 2.0, 12.0));
    commands.spawn((big, at(12.0, 17.0, iw * 2.0, 12.0, CONTENT_Z + 0.1), PIXEL_LAYER, tag()));
    let day_lbl = format!("DAY {cur} OF {SEASON_LEN}");
    label(&mut commands, &mut images, &day_lbl, 12.0, 33.0, 0xa0a0a8, CONTENT_Z + 0.1, tag());
    // The season's fair, named + counted down beside the day counter (js).
    if let Some(f) = crate::app::festivals::LIST.iter().find(|f| f.season == s_idx) {
        let fest_lbl = if f.day == cur {
            format!("{} - TODAY!", f.name)
        } else if f.day > cur {
            format!("{} IN {} DAYS", f.name, f.day - cur)
        } else {
            format!("{} HAS PASSED", f.name)
        };
        let fw = crate::gfx::font::measure(&fest_lbl) as f32;
        label(&mut commands, &mut images, &fest_lbl, crate::CANVAS_W as f32 - 12.0 - fw, 33.0, f.color, CONTENT_Z + 0.1, tag());
    }

    // The month: 7 columns x 4 rows under weekday letters (js geometry verbatim).
    let (cols, gx, gy, cw) = (7i64, 8.0, 46.0, 36.0);
    let ch = ((CANVAS_H as f32 - 16.0 - (gy + 10.0)) / 4.0).floor();
    for (i, wk) in ["M", "T", "W", "T", "F", "S", "S"].iter().enumerate() {
        label(&mut commands, &mut images, wk, gx + i as f32 * cw + cw / 2.0 - 2.0, gy, 0x8a8a92, CONTENT_Z + 0.1, tag());
    }
    for d in 1..=SEASON_LEN {
        let idx = d - 1;
        let x = gx + (idx % cols) as f32 * cw;
        let y = gy + 10.0 + (idx / cols) as f32 * ch;
        let today = d == cur;
        let past = d < cur;
        let fill = if today {
            0x3a3a20
        } else if past {
            0x0e0e12
        } else {
            0x191920
        };
        commands.spawn((
            Sprite::from_color(Color::srgb_u8((fill >> 16) as u8, (fill >> 8) as u8, fill as u8), Vec2::new(cw - 3.0, ch - 3.0)),
            at(x + 1.0, y + 1.0, cw - 3.0, ch - 3.0, CONTENT_Z),
            PIXEL_LAYER,
            tag(),
        ));
        if today {
            frame_rect(&mut commands, x + 1.0, y + 1.0, cw - 3.0, ch - 3.0, s_col, CONTENT_Z + 0.05, tag());
        }
        let num = format!("{d}");
        let num_col = if today { 0xfcfcfc } else if past { 0x56565e } else { 0xb4b4bc };
        label(&mut commands, &mut images, &num, x + 4.0, y + 4.0, num_col, CONTENT_Z + 0.1, tag());
        // The fair's day wears its pennant colour (js festival pennants).
        if let Some(f) = crate::app::festivals::LIST.iter().find(|f| f.season == s_idx && f.day == d) {
            frame_rect(&mut commands, x + 1.0, y + 1.0, cw - 3.0, ch - 3.0, f.color, CONTENT_Z + 0.04, tag());
        }
        // (js: birthday hearts render per-day here once those port.)
    }

    // Right column: what's growing this season — no crops ported = the js fallow state.
    let ix = gx + cols as f32 * cw + 14.0;
    label(&mut commands, &mut images, "IN SEASON", ix, gy + 4.0, s_col, CONTENT_Z + 0.1, tag());
    label(&mut commands, &mut images, "FIELDS LIE", ix, gy + 18.0, 0x8a8a92, CONTENT_Z + 0.1, tag());
    label(&mut commands, &mut images, "FALLOW.", ix, gy + 28.0, 0x8a8a92, CONTENT_Z + 0.1, tag());
}
