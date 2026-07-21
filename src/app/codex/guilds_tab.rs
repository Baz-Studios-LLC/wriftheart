//! guilds_tab.rs — the GUILDS codex page (js drawGuildsDex): every city hall you've
//! stepped into, richest restoration first. Left: the cities — name, five crest pips
//! (lit in the wing's colour once restored), the wings-done tally. Right: the selected
//! city's five wings with progress and each perk — promised in gray, held in gold.
//! No halls found yet keeps the '? ? ?' mystery.

use super::dex::{center_label, wrap_text, DEX_GY, DEX_RX};
use super::{hint_scaffold, CodexState, CodexUi, TabContent, CONTENT_Z};
use crate::app::banners::TownNames;
use crate::app::guildhall::{GuildLedger, GuildState};
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::ui::{frame_rect, label};
use crate::{guildhall, CANVAS_H, CANVAS_W};
use bevy::prelude::*;

#[derive(Component, Clone)]
pub struct GuildsUi;

pub fn hint(bindings: &Bindings, pad: bool) -> String {
    let browse = format!(
        "{}/{} CITY",
        bindings.prompt(Action::Up, pad),
        bindings.prompt(Action::Down, pad)
    );
    hint_scaffold(bindings, pad, &browse)
}

/// The city list, richest restoration first, then by name (js guildCityRows).
fn city_rows<'a>(guilds: &'a GuildLedger, names: &TownNames) -> Vec<(String, &'a GuildState)> {
    let mut rows: Vec<(String, &GuildState)> = guilds
        .0
        .iter()
        .filter(|(k, _)| k.as_str() != "lost")
        .map(|(k, gh)| {
            let name = names.0.get(k).map_or("A FAR CITY".to_string(), |n| n.to_uppercase());
            (name, gh)
        })
        .collect();
    rows.sort_by(|a, b| b.1.done.len().cmp(&a.1.done.len()).then(a.0.cmp(&b.0)));
    rows
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn run(
    mut commands: Commands,
    cx_state: Res<CodexState>,
    mut images: ResMut<Assets<Image>>,
    guilds: Res<GuildLedger>,
    names: Res<TownNames>,
    mut state: ResMut<ActionState>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    ptr: Res<crate::input::Pointer>,
    old: Query<Entity, With<GuildsUi>>,
    mut cur: Local<usize>,
    mut seen_gen: Local<Option<u32>>,
) {
    let rows = city_rows(&guilds, &names);
    // Browse (js updateGuildsDex): up/down wraps through the cities.
    let mut moved = false;
    if !rows.is_empty() {
        *cur = (*cur).min(rows.len() - 1);
        if state.pressed(Action::Up) {
            *cur = (*cur + rows.len() - 1) % rows.len();
            state.consume(Action::Up);
            moved = true;
        }
        if state.pressed(Action::Down) {
            *cur = (*cur + 1) % rows.len();
            state.consume(Action::Down);
            moved = true;
        }
        if ptr.wheel_steps != 0 {
            // Wheel walks the halls, clamped (Baz: any scrollable list).
            *cur = (*cur as i32 - ptr.wheel_steps).clamp(0, rows.len() as i32 - 1) as usize;
            moved = true;
        }
        if moved {
            sfx.write(crate::app::sfx::Sfx("menuMove"));
        }
    }
    if *seen_gen == Some(cx_state.generation) && !moved {
        return;
    }
    *seen_gen = Some(cx_state.generation);
    for e in &old {
        commands.entity(e).despawn();
    }
    let tag = || (CodexUi, TabContent, GuildsUi);

    // Header (js): the tally of halls found.
    let head = match rows.len() {
        0 => "GUILDS".to_string(),
        1 => "GUILDS  1 HALL FOUND".to_string(),
        n => format!("GUILDS  {n} HALLS FOUND"),
    };
    label(&mut commands, &mut images, &head, 8.0, 15.0, 0xbfb9a0, CONTENT_Z + 0.1, tag());
    if rows.is_empty() {
        // Undiscovered — a mystery until you step into a city's hall.
        let (img, tw) = font::bake_text("? ? ?", 0x5a5a62, &mut images);
        let iw = (tw + (tw & 1)) as f32;
        let mut s = Sprite::from_image(img);
        s.custom_size = Some(Vec2::new(iw * 2.0, 12.0));
        let cx = CANVAS_W as f32 / 2.0;
        commands.spawn((s, at((cx - iw).round(), 88.0, iw * 2.0, 12.0, CONTENT_Z + 0.1), PIXEL_LAYER, tag()));
        return;
    }

    // --- Left: the city list (VIS rows of name + pips + tally). ---
    const VIS: usize = 8;
    let (y0, rh) = (DEX_GY + 2.0, 21.0);
    let lw = DEX_RX - 14.0;
    let scroll = (cur.saturating_sub(3)).min(rows.len().saturating_sub(VIS));
    for i in 0..VIS {
        let idx = scroll + i;
        let Some((name, gh)) = rows.get(idx) else { break };
        let y = y0 + i as f32 * rh;
        let on = idx == *cur;
        let row_col = if on {
            Color::srgba(0.988, 0.878, 0.659, 0.13)
        } else if i % 2 == 1 {
            Color::srgba(1.0, 1.0, 1.0, 0.03)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.25)
        };
        commands.spawn((Sprite::from_color(row_col, Vec2::new(lw, rh - 2.0)), at(6.0, y, lw, rh - 2.0, CONTENT_Z), PIXEL_LAYER, tag()));
        if on {
            frame_rect(&mut commands, 6.0, y, lw, rh - 2.0, 0xfce0a8, CONTENT_Z + 0.05, tag());
        }
        label(&mut commands, &mut images, name, 10.0, y + 3.0, if on { 0xfcfcfc } else { 0xd8d8e0 }, CONTENT_Z + 0.1, tag());
        // One pip per wing, lit in its crest colour once restored.
        for (p, wing) in guildhall::WINGS.iter().enumerate() {
            let col = if gh.done.iter().any(|d| d == wing.id) { wing.crest } else { 0x26262e };
            commands.spawn((
                Sprite::from_color(Color::srgb_u8((col >> 16) as u8, (col >> 8) as u8, col as u8), Vec2::splat(5.0)),
                at(10.0 + p as f32 * 8.0, y + 12.0, 5.0, 5.0, CONTENT_Z + 0.1),
                PIXEL_LAYER,
                tag(),
            ));
        }
        let tally = format!("{}/5", gh.done.len());
        let tw = font::measure(&tally) as f32;
        let tcol = if gh.done.len() >= 5 { 0xffd34d } else { 0x8a8a92 };
        label(&mut commands, &mut images, &tally, 6.0 + lw - 8.0 - tw, y + 3.0, tcol, CONTENT_Z + 0.1, tag());
    }
    if scroll > 0 {
        label(&mut commands, &mut images, "<", 6.0 + lw - 8.0, y0 - 8.0, 0xe8c860, CONTENT_Z + 0.1, tag());
    }
    if scroll + VIS < rows.len() {
        label(&mut commands, &mut images, ">", 6.0 + lw - 8.0, y0 + VIS as f32 * rh - 2.0, 0xe8c860, CONTENT_Z + 0.1, tag());
    }

    // --- Right: the selected city's five wings (the js custom pane). ---
    let (name, gh) = &rows[*cur];
    let (px, py) = (DEX_RX, DEX_GY);
    let pw = CANVAS_W as f32 - 6.0 - px;
    let ph = CANVAS_H as f32 - DEX_GY - 14.0;
    let cx2 = px + (pw / 2.0).round();
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x08, 0x08, 0x0c), Vec2::new(pw, ph)),
        at(px, py, pw, ph, CONTENT_Z),
        PIXEL_LAYER,
        tag(),
    ));
    frame_rect(&mut commands, px, py, pw, ph, 0x2c2c36, CONTENT_Z + 0.05, tag());
    center_label(&mut commands, &mut images, name, cx2, py + 8.0, 0xfcfcfc, CONTENT_Z + 0.1, tag());
    let nd = gh.done.len();
    let (sub, sub_col) = if nd >= 5 {
        ("THE HALL STANDS WHOLE".to_string(), 0xffd34d)
    } else {
        (format!("{nd} OF 5 WINGS RESTORED"), 0x8a8a92)
    };
    center_label(&mut commands, &mut images, &sub, cx2, py + 19.0, sub_col, CONTENT_Z + 0.1, tag());
    let mut yy = py + 34.0;
    for wing in &guildhall::WINGS {
        let done = gh.done.iter().any(|d| d == wing.id);
        let empty = Vec::new();
        let counts = gh.donated.get(wing.id).unwrap_or(&empty);
        let (have, need, _) = guildhall::wing_progress(wing, counts);
        let pip = if done { wing.crest } else { 0x26262e };
        commands.spawn((
            Sprite::from_color(Color::srgb_u8((pip >> 16) as u8, (pip >> 8) as u8, pip as u8), Vec2::splat(5.0)),
            at(px + 6.0, yy + 1.0, 5.0, 5.0, CONTENT_Z + 0.1),
            PIXEL_LAYER,
            tag(),
        ));
        label(&mut commands, &mut images, wing.name, px + 15.0, yy, if done { wing.crest } else { 0xb4b4bc }, CONTENT_Z + 0.1, tag());
        let (tag_txt, tag_col) = if done {
            ("RESTORED".to_string(), 0x7ee08a)
        } else {
            (format!("{have}/{need}"), if have > 0 { 0xffd34d } else { 0x5a5a62 })
        };
        let ttw = font::measure(&tag_txt) as f32;
        label(&mut commands, &mut images, &tag_txt, px + pw - 6.0 - ttw, yy, tag_col, CONTENT_Z + 0.1, tag());
        // The perk: promised in gray, held in gold (two wrapped lines max).
        for (li, ln) in wrap_text(wing.perk_desc, pw - 21.0).into_iter().take(2).enumerate() {
            label(&mut commands, &mut images, &ln, px + 15.0, yy + 9.0 + li as f32 * 8.0, if done { 0xb8a060 } else { 0x494952 }, CONTENT_Z + 0.1, tag());
        }
        yy += 27.0;
    }
}
