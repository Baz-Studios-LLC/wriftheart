//! dex.rs — the codex's shared two-pane collection viewer (js drawDexGrid / drawDexPane /
//! dexNav / dexBlit): a scrollable cell grid on the LEFT ('?' for undiscovered entries),
//! a detail pane on the RIGHT (plate + big preview + centred name/sub + wrapped lore).
//! MOBS, ITEMS (and later AWARDS/LORE) all draw through here.

use super::CONTENT_Z;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use crate::ui::label;
use crate::{CANVAS_H, CANVAS_W};
use bevy::prelude::*;

pub const DEX_COLS: usize = 9;
pub const DEX_CELL: f32 = 22.0;
pub const DEX_AX: f32 = 8.0;
pub const DEX_GY: f32 = 24.0;
/// Right detail-pane x (js DEX_RX).
pub const DEX_RX: f32 = DEX_AX + DEX_COLS as f32 * DEX_CELL + 8.0;

/// js dexNav: left/right step 1, up/down step a row, clamped. Returns the new cursor.
pub fn dex_nav(state: &ActionState, count: usize, cur: usize, cols: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let mut c = cur as i32;
    if state.pressed(Action::Left) {
        c -= 1;
    }
    if state.pressed(Action::Right) {
        c += 1;
    }
    if state.pressed(Action::Up) {
        c -= cols as i32;
    }
    if state.pressed(Action::Down) {
        c += cols as i32;
    }
    c.clamp(0, count as i32 - 1) as usize
}

/// Mouse: the entry under a click in the grid (Baz: click a cell to select it) —
/// the same scroll window draw_grid shows. Returns the clicked entry, or None.
pub fn dex_click(ptr: &crate::input::Pointer, count: usize, cur: usize, cols: usize) -> Option<usize> {
    if !ptr.click || count == 0 {
        return None;
    }
    let p = ptr.pos?;
    let (ax, ay, cell) = (DEX_AX, DEX_GY, DEX_CELL);
    let rows = count.div_ceil(cols);
    let vis_rows = (((CANVAS_H as f32 - ay - 14.0) / cell).floor() as usize).max(1);
    let scroll = (cur / cols).saturating_sub(vis_rows / 2).min(rows.saturating_sub(vis_rows));
    let c = ((p.x - ax) / cell).floor();
    let r = ((p.y - ay) / cell).floor();
    if c < 0.0 || c >= cols as f32 || r < 0.0 || r >= vis_rows as f32 {
        return None;
    }
    let i = (scroll + r as usize) * cols + c as usize;
    (i < count).then_some(i)
}

/// js dexBlit's scale rule: oversized art shrinks to FIT `target`; smaller art
/// integer-upscales for crispness. Returns the drawn size for an art of `native` px.
pub fn blit_size(native: f32, target: f32) -> f32 {
    if native > target {
        target
    } else {
        native * (target / native).floor().max(1.0)
    }
}

/// Spawn `img` centred on (cx, cy) at the dexBlit scale.
#[allow(clippy::too_many_arguments)] // it IS a draw call
pub fn blit(
    commands: &mut Commands,
    img: Handle<Image>,
    native: f32,
    cx: f32,
    cy: f32,
    target: f32,
    z: f32,
    marker: impl Bundle + Clone,
) {
    let s = blit_size(native, target);
    let mut sp = Sprite::from_image(img);
    sp.custom_size = Some(Vec2::splat(s));
    commands.spawn((sp, at((cx - s / 2.0).round(), (cy - s / 2.0).round(), s, s, z), PIXEL_LAYER, marker));
}

/// js drawDexGrid: the scrollable cell grid + scrollbar. `icon(i)` yields the entry's art
/// (handle + native px) — drawn when `unlocked(i)`, else the '?' mystery glyph.
#[allow(clippy::too_many_arguments)] // it IS the grid's arity
pub fn draw_grid(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    count: usize,
    cur: usize,
    cols: usize,
    unlocked: impl Fn(usize) -> bool,
    icon: impl Fn(usize) -> Option<(Handle<Image>, f32)>,
    marker: impl Bundle + Clone,
) {
    let (ax, ay, cell) = (DEX_AX, DEX_GY, DEX_CELL);
    let rows = count.div_ceil(cols);
    let vis_rows = (((CANVAS_H as f32 - ay - 14.0) / cell).floor() as usize).max(1);
    let scroll = (cur / cols)
        .saturating_sub(vis_rows / 2)
        .min(rows.saturating_sub(vis_rows));
    for i in 0..count {
        let r = (i / cols) as i32 - scroll as i32;
        if r < 0 || r >= vis_rows as i32 {
            continue;
        }
        let x = ax + (i % cols) as f32 * cell;
        let y = ay + r as f32 * cell;
        let open = unlocked(i);
        // Open cells lighter so near-black icons read (js note).
        let fill = if open { 0x34343e } else { 0x14141a };
        let border = if i == cur { 0xffd34d } else { 0x2c2c36 };
        crate::ui::cell(commands, x, y, cell - 2.0, Some(fill), border, None, CONTENT_Z, marker.clone());
        if open {
            if let Some((img, native)) = icon(i) {
                blit(commands, img, native, x + cell / 2.0 - 1.0, y + cell / 2.0 - 1.0, cell - 4.0, CONTENT_Z + 0.1, marker.clone());
            }
        } else {
            // js dexQ: the mystery glyph.
            label(commands, images, "?", x + cell / 2.0 - 2.0, y + cell / 2.0 - 4.0, 0x54545e, CONTENT_Z + 0.1, marker.clone());
        }
    }
    if rows > vis_rows {
        let sh = CANVAS_H as f32 - ay - 14.0;
        let sx = ax + cols as f32 * cell;
        let th = (sh * vis_rows as f32 / rows as f32).max(8.0);
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(0x26, 0x26, 0x2e), Vec2::new(2.0, sh)),
            at(sx, ay, 2.0, sh, CONTENT_Z),
            PIXEL_LAYER,
            marker.clone(),
        ));
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(0x7a, 0x7a, 0x86), Vec2::new(2.0, th.round())),
            at(sx, (ay + sh * scroll as f32 / rows as f32).round(), 2.0, th.round(), CONTENT_Z + 0.1),
            PIXEL_LAYER,
            marker,
        ));
    }
}

/// A centred label (js centerText at scale 1, no shadow — the dex pane's text style).
#[allow(clippy::too_many_arguments)] // it IS a draw call
pub fn center_label(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    text: &str,
    cx: f32,
    y: f32,
    color: u32,
    z: f32,
    marker: impl Bundle + Clone,
) {
    let w = font::measure(text) as f32;
    label(commands, images, text, (cx - w / 2.0).round(), y, color, z, marker);
}

/// js drawDexPane: the right detail pane — 0.5-black backing + border; '?' when locked;
/// else optional plate + big art, centred name, sub line, and wrapped lore text.
#[allow(clippy::too_many_arguments)] // it IS the pane's arity
pub fn draw_pane(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    open: bool,
    big: Option<(Handle<Image>, f32)>,
    name: &str,
    sub: Option<(&str, u32)>,
    lore: &str,
    plate: bool,
    marker: impl Bundle + Clone,
) {
    let (px, py) = (DEX_RX, DEX_GY);
    let pw = CANVAS_W as f32 - 6.0 - px;
    let ph = CANVAS_H as f32 - DEX_GY - 14.0;
    let cx = px + (pw / 2.0).round();
    // rgba(0,0,0,0.5) over the near-black overlay — solid dark reads the same.
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x08, 0x08, 0x0c), Vec2::new(pw, ph)),
        at(px, py, pw, ph, CONTENT_Z),
        PIXEL_LAYER,
        marker.clone(),
    ));
    crate::ui::frame_rect(commands, px, py, pw, ph, 0x2c2c36, CONTENT_Z + 0.05, marker.clone());
    if !open {
        // The undiscovered mystery: a big '?' + the label (js centerText scale 3).
        let (img, tw) = font::bake_text("?", 0x5a5a64, images);
        let iw = (tw + (tw & 1)) as f32;
        let mut s = Sprite::from_image(img);
        s.custom_size = Some(Vec2::new(iw * 3.0, 18.0));
        commands.spawn((s, at((cx - iw * 3.0 / 2.0).round(), py + 32.0, iw * 3.0, 18.0, CONTENT_Z + 0.1), PIXEL_LAYER, marker.clone()));
        center_label(commands, images, "UNDISCOVERED", cx, py + 71.0, 0x54545c, CONTENT_Z + 0.1, marker);
        return;
    }
    if let Some((img, native)) = big {
        if plate {
            let ps = 52.0;
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0x3a, 0x3a, 0x46), Vec2::splat(ps)),
                at(cx - ps / 2.0, py + 38.0 - ps / 2.0, ps, ps, CONTENT_Z + 0.05),
                PIXEL_LAYER,
                marker.clone(),
            ));
            crate::ui::frame_rect(commands, cx - ps / 2.0, py + 38.0 - ps / 2.0, ps, ps, 0x4c4c5a, CONTENT_Z + 0.06, marker.clone());
        }
        blit(commands, img, native, cx, py + 38.0, 44.0, CONTENT_Z + 0.1, marker.clone());
    }
    center_label(commands, images, name, cx, py + 73.0, 0xfcfcfc, CONTENT_Z + 0.1, marker.clone());
    if let Some((sub, sub_color)) = sub {
        center_label(commands, images, sub, cx, py + 85.0, sub_color, CONTENT_Z + 0.1, marker.clone());
    }
    let mut yy = py + 106.0; // js: extra breathing room under the rarity line
    for ln in wrap_text(lore, pw - 10.0) {
        label(commands, images, &ln, px + 5.0, yy, 0x9a9aa2, CONTENT_Z + 0.1, marker.clone());
        yy += 8.0;
    }
}

/// Word-wrap on the real font metrics (js wrapText).
pub fn wrap_text(text: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let probe = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if font::measure(&probe) as f32 <= width || line.is_empty() {
            line = probe;
        } else {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
