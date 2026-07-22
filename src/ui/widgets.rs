//! widgets.rs — the shared UI primitives. THE REUSE RULE MADE FLESH: "a window is a function
//! you call." Every screen composes these; no screen ever hand-draws its own chrome, text,
//! or bars. If a new screen needs something twice, it becomes a primitive here first.

use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use bevy::prelude::*;
use bevy::sprite::Anchor;

/// Worn by every speech/prompt bubble — prompt systems yield while one is live
/// (see prompts.rs; defined here so speech_bubble can insert it itself).
#[derive(Component)]
pub struct AnyBubble;

/// One label = one baked-text sprite. THE text helper — every string on screen goes through
/// here. Returns the entity (despawn + respawn to retext).
#[allow(clippy::too_many_arguments)] // (text, pos, colour, z, marker) IS the label's arity
pub fn label<M: Bundle>(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    text: &str,
    x: f32,
    y: f32,
    color: u32,
    z: f32,
    marker: M,
) -> Entity {
    let (img, w) = font::bake_text(text, color, images);
    let iw = (w + (w & 1)) as f32;
    // Whole-pixel law: centred callers (x = (w - measure)/2) can land on a half-pixel,
    // and the canvas upscale then shears every glyph — floor keeps the font crisp.
    commands.spawn((Sprite::from_image(img), at(x.floor(), y.floor(), iw, 6.0, z), PIXEL_LAYER, marker)).id()
}

/// The four strip rectangles that outline (x,y,w,h) at thickness `t` — the ONE place
/// border geometry lives. Pure geometry: callers spawn the strips in whatever transform
/// space they're in (canvas `at()`, or child-local under a moving root).
pub fn border_strips(x: f32, y: f32, w: f32, h: f32, t: f32) -> [(f32, f32, f32, f32); 4] {
    [
        (x, y, w, t),
        (x, y + h - t, w, t),
        (x, y, t, h),
        (x + w - t, y, t, h),
    ]
}

/// Outline a rect with 1px border strips in canvas space — bars, slots, and dex cells all
/// stroke their boxes through here.
#[allow(clippy::too_many_arguments)] // (rect, color, z, marker) IS the outline's arity
pub fn frame_rect(commands: &mut Commands, x: f32, y: f32, w: f32, h: f32, color: u32, z: f32, marker: impl Bundle + Clone) {
    let c = Color::srgb_u8((color >> 16) as u8, (color >> 8) as u8, color as u8);
    for (sx, sy, sw, sh) in border_strips(x, y, w, h, 1.0) {
        commands.spawn((Sprite::from_color(c, Vec2::new(sw, sh)), at(sx, sy, sw, sh, z), PIXEL_LAYER, marker.clone()));
    }
}

/// A bordered square cell with an optional fill and an optional centred icon — the shape
/// shared by the HUD ability slots and the codex dex grids.
#[allow(clippy::too_many_arguments)] // (pos, size, fill, border, icon, z, marker) IS the cell's arity
pub fn cell(
    commands: &mut Commands,
    x: f32,
    y: f32,
    size: f32,
    fill: Option<u32>,
    border: u32,
    icon: Option<(Handle<Image>, f32)>,
    z: f32,
    marker: impl Bundle + Clone,
) {
    if let Some(fill) = fill {
        let c = Color::srgb_u8((fill >> 16) as u8, (fill >> 8) as u8, fill as u8);
        commands.spawn((Sprite::from_color(c, Vec2::new(size, size)), at(x, y, size, size, z), PIXEL_LAYER, marker.clone()));
    }
    frame_rect(commands, x, y, size, size, border, z + 0.2, marker.clone());
    if let Some((img, is)) = icon {
        let off = ((size - is) / 2.0).round();
        commands.spawn((Sprite::from_image(img), at(x + off, y + off, is, is, z + 0.1), PIXEL_LAYER, marker));
    }
}

/// A window panel: dark fill + 1px border — the ONE panel chrome (port of the JS dialog
/// style: dark backing, light stroke). Returns the backing entity.
#[allow(clippy::too_many_arguments)] // (rect, z, border, marker) IS the panel's arity
pub fn panel<M: Bundle>(
    commands: &mut Commands,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    z: f32,
    border: u32,
    marker: M,
) -> Entity {
    let fill = commands
        .spawn((
            Sprite::from_color(Color::srgb_u8(0x06, 0x08, 0x0e), Vec2::new(w, h)),
            at(x, y, w, h, z),
            PIXEL_LAYER,
            marker,
        ))
        .id();
    // Border: four 1px strips (children so they despawn with the panel).
    let bc = Color::srgb_u8((border >> 16) as u8, (border >> 8) as u8, border as u8);
    for (sx, sy, sw, sh) in border_strips(x, y, w, h, 1.0) {
        let s = commands
            .spawn((Sprite::from_color(bc, Vec2::new(sw, sh)), at(sx, sy, sw, sh, z + 0.1), PIXEL_LAYER))
            .id();
        commands.entity(fill).add_child(s);
        // Children inherit the parent transform; re-anchor the strip relative to the panel.
        commands.entity(s).insert(Transform::from_xyz(
            (sx + sw / 2.0) - (x + w / 2.0),
            (y + h / 2.0) - (sy + sh / 2.0),
            0.1,
        ));
    }
    fill
}

/// A labelled stat bar — port of the sidebar `bar()` helper (label left, trough + fill +
/// border, centred value text). Returns the FILL entity; resize it with [`set_bar`].
pub struct BarSpec<'a> {
    pub label: &'a str,
    pub x: f32,
    pub y: f32,
    pub w: f32, // trough width (the JS lblBw)
    pub h: f32,
    pub fill: u32,
    pub border: u32,
    pub z: f32, // base layer: trough z, fill z+0.2, border z+0.3, label z+1
}

pub fn bar<M: Bundle>(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    spec: &BarSpec,
    frac: f32,
    marker: M,
) -> Entity {
    label(commands, images, spec.label, spec.x, spec.y + ((spec.h - 5.0) / 2.0).round(), 0x9a9a9a, spec.z + 1.0, ());
    let bx = spec.x + 13.0;
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x0e, 0x0e, 0x12), Vec2::new(spec.w, spec.h)),
        at(bx, spec.y, spec.w, spec.h, spec.z),
        PIXEL_LAYER,
    ));
    frame_rect(commands, bx, spec.y, spec.w, spec.h, spec.border, spec.z + 0.3, ());
    let fc = Color::srgb_u8((spec.fill >> 16) as u8, (spec.fill >> 8) as u8, spec.fill as u8);
    commands
        .spawn((
            Sprite::from_color(fc, Vec2::new((spec.w - 2.0) * frac.clamp(0.0, 1.0), spec.h - 2.0)),
            Anchor(Vec2::new(-0.5, 0.0)), // grow from the left edge
            at(bx + 1.0, spec.y + 1.0, 0.0, spec.h - 2.0, spec.z + 0.2),
            PIXEL_LAYER,
            marker,
        ))
        .id()
}

/// Resize a bar's fill to a new fraction (the update half of [`bar`]).
pub fn set_bar(sprite: &mut Sprite, spec_w: f32, spec_h: f32, frac: f32) {
    sprite.custom_size = Some(Vec2::new((spec_w - 2.0) * frac.clamp(0.0, 1.0), spec_h - 2.0));
}

/// A marker-carrying draw context: commands + images + the screen's UI marker, so window
/// code writes `pen.text(...)` instead of threading three arguments through every helper.
/// THE way a screen lays down text and fills (the pause menu and title both draw with it).
pub struct Pen<'a, 'w, 's, M: Component + Clone> {
    pub commands: &'a mut Commands<'w, 's>,
    pub images: &'a mut Assets<Image>,
    pub marker: M,
}

impl<M: Component + Clone> Pen<'_, '_, '_, M> {
    pub fn text(&mut self, s: &str, x: f32, y: f32, color: u32, z: f32) {
        label(self.commands, self.images, s, x, y, color, z, self.marker.clone());
    }
    /// Right-aligned: the text ENDS at x.
    pub fn text_right(&mut self, s: &str, x: f32, y: f32, color: u32, z: f32) {
        let w = crate::gfx::font::measure(s) as f32;
        self.text(s, x - w, y, color, z);
    }
    pub fn text_center(&mut self, s: &str, cx: f32, y: f32, color: u32, z: f32) {
        let x = (cx - crate::gfx::font::measure(s) as f32 / 2.0).round();
        self.text(s, x, y, color, z);
    }
    /// Integer-scaled text (js Font.draw's scale arg): baked once, upscaled by the sprite
    /// (nearest sampling keeps it crisp). Positions by the TOP-LEFT like text().
    pub fn text_scaled(&mut self, s: &str, x: f32, y: f32, color: u32, z: f32, scale: f32) {
        let (img, w) = crate::gfx::font::bake_text(s, color, self.images);
        let iw = (w + (w & 1)) as f32;
        self.commands.spawn((
            Sprite { image: img, custom_size: Some(Vec2::new(iw * scale, 6.0 * scale)), ..default() },
            crate::gfx::at(x, y, iw * scale, 6.0 * scale, z),
            crate::gfx::PIXEL_LAYER,
            self.marker.clone(),
        ));
    }
    pub fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, color: u32, z: f32) {
        let c = Color::srgb_u8((color >> 16) as u8, (color >> 8) as u8, color as u8);
        self.fill_rgba(x, y, w, h, c, z);
    }
    /// A fill with alpha (dim layers, soft panels). Remember: Bevy blends in LINEAR space,
    /// so a js rgba alpha needs a bump (~+0.1) to read the same on screen.
    pub fn fill_rgba(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color, z: f32) {
        self.commands
            .spawn((Sprite::from_color(c, Vec2::new(w, h)), crate::gfx::at(x, y, w, h, z), crate::gfx::PIXEL_LAYER, self.marker.clone()));
    }
}

/// Shared vertical-list selection: up/down presses move it (wrapping). Returns true when the
/// selection changed — the ONE list-navigation implementation every menu uses.
pub struct ListNav {
    pub sel: usize,
    pub len: usize,
}

impl ListNav {
    pub fn new(len: usize) -> Self {
        Self { sel: 0, len }
    }
    pub fn tick(&mut self, state: &ActionState) -> bool {
        if self.len == 0 {
            return false;
        }
        if state.pressed(Action::Up) {
            self.sel = (self.sel + self.len - 1) % self.len;
            true
        } else if state.pressed(Action::Down) {
            self.sel = (self.sel + 1) % self.len;
            true
        } else {
            false
        }
    }
}

/// A complete selectable-list WINDOW: centred panel + gold title + list with a `>` cursor +
/// a bottom hint. THE window — the pause menu, pickers, and confirm dialogs all call this
/// with their own marker (tagging every spawned entity for wholesale despawn on close/redraw).
///
/// `x`/`y`/`w` frame the window; height derives from the item count so callers can't
/// mis-size it. Selected row is bright with the cursor; the rest are muted.
pub struct ListWindow<'a> {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub title: &'a str,
    pub items: &'a [&'a str],
    pub sel: usize,
    pub hint: &'a str,
    pub z: f32,
}

impl ListWindow<'_> {
    /// The derived window height: title band + rows + hint band.
    pub fn height(&self) -> f32 {
        20.0 + self.items.len() as f32 * 12.0 + 16.0
    }
}

pub fn list_window<M: Component + Clone>(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    win: &ListWindow,
    marker: M,
) {
    let h = win.height();
    panel(commands, win.x, win.y, win.w, h, win.z, 0x8a92a2, marker.clone());
    let tz = win.z + 1.0;
    label(
        commands,
        images,
        win.title,
        win.x + ((win.w - font::measure(win.title) as f32) / 2.0).round(),
        win.y + 6.0,
        0xfcd000,
        tz,
        marker.clone(),
    );
    for (i, item) in win.items.iter().enumerate() {
        let iy = win.y + 20.0 + i as f32 * 12.0;
        let sel = i == win.sel;
        if sel {
            label(commands, images, ">", win.x + 12.0, iy, 0xfcd000, tz, marker.clone());
        }
        label(commands, images, item, win.x + 20.0, iy, if sel { 0xfcfcfc } else { 0x9aa0aa }, tz, marker.clone());
    }
    label(
        commands,
        images,
        win.hint,
        win.x + ((win.w - font::measure(win.hint) as f32) / 2.0).round(),
        win.y + h - 12.0,
        0x606060,
        tz,
        marker.clone(),
    );
}

/// THE speech bubble — one recipe for every talker (town chat, wilderness shouts):
/// dark backing, pale-blue border, light centred line, built as a PARENTED bundle
/// so one transform moves the whole thing (despawn the parent, the bubble goes).
/// Returns (parent, bubble width) — the caller positions + marks the parent.
pub fn speech_bubble(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    text: &str,
    x: f32,
    y: f32,
    z: f32,
) -> (Entity, f32) {
    let (img, w) = crate::gfx::font::bake_text(text, 0xe8f0ff, images);
    let iw = (w + (w & 1)) as f32;
    let bw = iw + 8.0;
    // OPAQUE plate: at 0.85 whatever sat beneath ghosted through the line (the
    // town garble) — a live bubble hides its ground completely.
    let parent = commands
        .spawn((
            Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 1.0), Vec2::new(bw, 11.0)),
            crate::gfx::at(x, y, bw, 11.0, z),
            crate::gfx::PIXEL_LAYER,
            AnyBubble,
        ))
        .id();
    let child_tf =
        |cx: f32, cy: f32, w2: f32, h2: f32, dz: f32| Transform::from_xyz(cx + w2 / 2.0 - bw / 2.0, 11.0 / 2.0 - cy - h2 / 2.0, dz);
    for (sx, sy, sw, sh) in border_strips(0.0, 0.0, bw, 11.0, 1.0) {
        let c = commands
            .spawn((Sprite::from_color(Color::srgb_u8(0x9a, 0xb8, 0xe0), Vec2::new(sw, sh)), child_tf(sx, sy, sw, sh, 0.03), crate::gfx::PIXEL_LAYER))
            .id();
        commands.entity(parent).add_child(c);
    }
    let t = commands.spawn((Sprite::from_image(img), child_tf(4.0, 2.5, iw, 6.0, 0.05), crate::gfx::PIXEL_LAYER)).id();
    commands.entity(parent).add_child(t);
    (parent, bw)
}
