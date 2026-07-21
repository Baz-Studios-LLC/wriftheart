//! stats_tab.rs — THE LEDGER OF DEEDS (js drawStatsDex): every absurd number the game
//! counts, as [label, value] rows under gold group banners, flowed down two columns.
//! Up/down scroll by two lines. The line list is the js's VERBATIM — counters whose
//! systems haven't ported print their zeros/fallbacks, exactly like a fresh js save.

use super::{hint_scaffold, CodexState, CodexUi, TabContent, CONTENT_Z};
use crate::app::gather::day_number;
use crate::app::rewards::Progress;
use crate::app::room_render::FrameClock;
use crate::app::stats::Stats;
use crate::gfx::font;
use crate::input::{Action, ActionState, Bindings};
use crate::ui::label;
use crate::{CANVAS_H, CANVAS_W};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct StatsView {
    pub scroll: usize,
}

#[derive(Component, Clone)]
pub struct StatsUi;

pub fn hint(bindings: &Bindings, pad: bool) -> String {
    let browse = if pad { "DPAD BROWSE" } else { "ARROWS BROWSE" };
    hint_scaffold(bindings, pad, browse)
}

enum Line {
    Banner(&'static str),
    Row(&'static str, String),
}

/// js statLines, line for line. `n()` rounds; text fallbacks match the js.
fn stat_lines(s: &Stats, progress: &Progress, clock: i64) -> Vec<Line> {
    let n = |k: &str| format!("{}", s.get(k).round() as i64);
    let _ = progress;
    // Favorite victim: the biggest kill_* counter.
    let fav = s
        .0
        .iter()
        .filter(|(k, v)| k.starts_with("kill_") && **v > 0.0)
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(k, _)| k[5..].to_uppercase());
    let secs_total = (s.get("frames") / 60.0).floor() as i64;
    let clock_str = format!("{}:{:02}:{:02}", secs_total / 3600, (secs_total % 3600) / 60, secs_total % 60);
    let days = day_number(clock);
    vec![
        Line::Banner("BLOOD AND BRUISES"),
        Line::Row("FOES FELLED", n("kills")),
        Line::Row("GOBLINS AMONG THEM", format!("{}", (s.get("kill_goblin") + s.get("kill_slinger")).round() as i64)),
        Line::Row("FAVORITE VICTIM", fav.unwrap_or_else(|| "NO ONE YET".into())),
        Line::Row("CHAMPIONS DOWN", n("champions")),
        Line::Row("ELITES DOWN", n("elites")),
        Line::Row("BOSSES FELLED", n("bosses")),
        Line::Row("HP LOST, LIFETIME", n("dmg")),
        Line::Row("TIMES ALL WENT DARK", n("deaths")),
        Line::Banner("THE LONG ROAD"),
        Line::Row("TILES WALKED", n("walk")),
        Line::Row("DAYS IN THE WORLD", format!("{days}")),
        Line::Row("TIME AT THE WHEEL", clock_str),
        Line::Row("NIGHTS SLEPT", n("sleeps")),
        Line::Row("WARPS RIDDEN", n("warps")),
        Line::Row("CHESTS CRACKED", n("chests")),
        Line::Row("HOLES DUG", n("digs")),
        Line::Row("COPPER OFF THE GROUND", n("coins")),
        Line::Banner("THE HOMESTEAD"),
        Line::Row("CROPS PULLED", n("crops")),
        Line::Row("EGGS FROM HAPPY HENS", n("eggs")),
        Line::Row("PAILS OF MILK", n("milk")),
        Line::Row("ANIMALS PETTED", n("pets")),
        Line::Row("FISH LANDED", n("fish")),
        Line::Row("BIGGEST CATCH", "STILL OUT THERE".into()),
        Line::Row("GRASS MOWN", n("grass")),
        Line::Row("TREES FELLED", n("trees")),
        Line::Row("STONES BROKEN", n("stones")),
        Line::Row("THINGS CRAFTED", n("crafts")),
        Line::Banner("THE FOLK"),
        Line::Row("HELLOS SAID", n("hellos")),
        Line::Row("GIFTS GIVEN", n("gifts")),
        Line::Row("SOULS KNOWN", "0".into()),
        Line::Row("HEARTS WON", "0".into()),
        Line::Row("FAIRS ATTENDED", n("festivals")),
        Line::Banner("THE DEEP PLACES"),
        Line::Row("RIFT GATES TAKEN", n("riftfloors")),
        Line::Row("DEEPEST RIFT", "UNTOUCHED".into()),
        Line::Row("SHARDS RECLAIMED", "0/10".into()),
        Line::Row("TOMES READ", "0".into()),
        Line::Row("TOWNS FOUND", "0".into()),
    ]
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn run(
    mut commands: Commands,
    state: Res<ActionState>,
    ptr: Res<crate::input::Pointer>,
    cx_state: Res<CodexState>,
    stats: Res<Stats>,
    progress: Res<Progress>,
    clock: Res<FrameClock>,
    mut view: ResMut<StatsView>,
    mut images: ResMut<Assets<Image>>,
    old: Query<Entity, With<StatsUi>>,
    mut seen_gen: Local<u32>,
) {
    let lines = stat_lines(&stats, &progress, clock.0);
    let y0 = super::dex::DEX_GY + 4.0;
    let rh = 9.0;
    let per_col = ((CANVAS_H as f32 - 14.0 - y0) / rh).floor() as usize;
    let max_scroll = lines.len().saturating_sub(per_col * 2);

    let mut dirty = *seen_gen != cx_state.generation;
    *seen_gen = cx_state.generation;
    if state.pressed(Action::Up) && view.scroll > 0 {
        view.scroll = view.scroll.saturating_sub(2);
        dirty = true;
    }
    if state.pressed(Action::Down) && view.scroll < max_scroll {
        view.scroll = (view.scroll + 2).min(max_scroll);
        dirty = true;
    }
    if ptr.wheel_steps != 0 {
        // Wheel scrolls the ledger (Baz: any scrollable list).
        view.scroll = (view.scroll as i32 - ptr.wheel_steps * 2).clamp(0, max_scroll as i32) as usize;
        dirty = true;
    }
    if !dirty {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let tag = || (CodexUi, TabContent, StatsUi);

    label(&mut commands, &mut images, "THE LEDGER OF DEEDS", 8.0, 16.0, 0xbfb9a0, CONTENT_Z + 0.1, tag());
    let col_w = ((CANVAS_W as f32 - 20.0) / 2.0).floor();
    for i in 0..per_col * 2 {
        let Some(ln) = lines.get(view.scroll + i) else { break };
        let col = (i / per_col) as f32;
        let x = 8.0 + col * (col_w + 8.0);
        let y = y0 + (i % per_col) as f32 * rh;
        match ln {
            Line::Banner(b) => {
                commands.spawn((
                    Sprite::from_color(Color::srgba(232.0 / 255.0, 200.0 / 255.0, 96.0 / 255.0, 0.08), Vec2::new(col_w, 8.0)),
                    crate::gfx::at(x - 2.0, y - 1.0, col_w, 8.0, CONTENT_Z),
                    crate::gfx::PIXEL_LAYER,
                    tag(),
                ));
                label(&mut commands, &mut images, b, x, y, 0xe8c860, CONTENT_Z + 0.1, tag());
            }
            Line::Row(k, v) => {
                label(&mut commands, &mut images, k, x, y, 0x8a8a92, CONTENT_Z + 0.1, tag());
                let vw = font::measure(v) as f32;
                label(&mut commands, &mut images, v, x + col_w - 6.0 - vw, y, 0xe8e8f0, CONTENT_Z + 0.1, tag());
            }
        }
    }
    if view.scroll > 0 {
        label(&mut commands, &mut images, "<", CANVAS_W as f32 - 12.0, super::dex::DEX_GY - 4.0, 0xe8c860, CONTENT_Z + 0.1, tag());
    }
    if view.scroll + per_col * 2 < lines.len() {
        label(&mut commands, &mut images, ">", CANVAS_W as f32 - 12.0, CANVAS_H as f32 - 24.0, 0xe8c860, CONTENT_Z + 0.1, tag());
    }
}
