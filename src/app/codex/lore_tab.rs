//! lore_tab.rs — the LORE codex tab (js updateLoreDex/drawLoreDex/drawBookPane): a
//! two-column study — the tome shelf (6-col grid) on the left, the open book's pages on
//! the right. Browsing previews page 1; confirm picks a found book UP (arrows/confirm
//! then turn pages; off the last page it sets the book down). Sort/Inventory flip pages
//! in either mode. Unfound tomes are '?' spines with an UNDISCOVERED pane.

use super::{dex, CodexState, CodexUi, TabContent, CONTENT_Z};
use crate::app::gather::GatherState;
use crate::gfx::bake;
use crate::input::{Action, ActionState, Bindings};
use crate::lore_books::{Book, BOOKS, BOOK_GRID};
use crate::ui::label;
use crate::CANVAS_H;
use bevy::prelude::*;

const TOME_COLS: usize = 6; // a narrower shelf leaves room to read
const TOME_RX: f32 = dex::DEX_AX + TOME_COLS as f32 * dex::DEX_CELL + 10.0;

/// Shelf cursor + the reader (page, and whether a book is picked UP).
#[derive(Resource, Default)]
pub struct LoreDex {
    pub cur: usize,
    pub page: usize,
    pub reading: Option<&'static str>,
}

#[derive(Component, Clone)]
pub struct LoreUi;

pub fn hint(bindings: &Bindings, pad: bool) -> String {
    format!(
        "{} READ - {}/{} PAGE",
        bindings.prompt(Action::Slot1, pad),
        bindings.prompt(Action::Sort, pad),
        bindings.prompt(Action::Inventory, pad),
    )
}

/// Split a book into pages of pane-wrapped lines (None = paragraph gap) — js paginateBook.
fn paginate(b: &Book) -> Vec<Vec<Option<String>>> {
    let w = crate::CANVAS_W as f32 - 6.0 - TOME_RX;
    let mut flat: Vec<Option<String>> = Vec::new();
    for para in b.text.split("\n\n") {
        for ln in dex::wrap_text(para, w) {
            flat.push(Some(ln));
        }
        flat.push(None);
    }
    let cap = CANVAS_H as f32 - dex::DEX_GY - 14.0 - 27.0 - 12.0;
    let mut pages: Vec<Vec<Option<String>>> = Vec::new();
    let mut cur: Vec<Option<String>> = Vec::new();
    let mut y = 0.0;
    for ln in flat {
        if ln.is_some() && y + 9.0 > cap {
            pages.push(std::mem::take(&mut cur));
            y = 0.0;
        }
        if ln.is_none() && cur.is_empty() {
            continue; // never lead a page with a gap
        }
        y += if ln.is_none() { 5.0 } else { 9.0 };
        cur.push(ln);
    }
    if !cur.is_empty() {
        pages.push(cur);
    }
    if pages.is_empty() {
        pages.push(vec![]);
    }
    pages
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn run(
    mut commands: Commands,
    state: Res<ActionState>,
    cx_state: Res<CodexState>,
    gather: Res<GatherState>,
    mut lx: ResMut<LoreDex>,
    ptr: Res<crate::input::Pointer>,
    mut images: ResMut<Assets<Image>>,
    old: Query<Entity, With<LoreUi>>,
    mut seen_gen: Local<u32>,
) {
    let mut dirty = *seen_gen != cx_state.generation;
    *seen_gen = cx_state.generation;
    let found = |id: &str| gather.tomes.contains(id);

    // The shown book (picked up, or the found shelf pick) turns pages on Sort/Inventory.
    let shown: Option<&'static Book> = lx
        .reading
        .and_then(crate::lore_books::get)
        .or_else(|| BOOKS.get(lx.cur).filter(|b| found(b.id)));
    if let Some(b) = shown {
        let pages = paginate(b);
        if state.pressed(Action::Sort) && lx.page > 0 {
            lx.page -= 1;
            dirty = true;
        }
        if state.pressed(Action::Inventory) && lx.page < pages.len() - 1 {
            lx.page += 1;
            dirty = true;
        }
        if lx.reading.is_some() {
            if state.pressed(Action::Left) && lx.page > 0 {
                lx.page -= 1;
                dirty = true;
            }
            if state.pressed(Action::Right) && lx.page < pages.len() - 1 {
                lx.page += 1;
                dirty = true;
            }
            if state.pressed(Action::Slot1) {
                // Confirm turns the page; off the last page it sets the book down.
                if lx.page < pages.len() - 1 {
                    lx.page += 1;
                } else {
                    lx.reading = None;
                }
                dirty = true;
            }
            if state.pressed(Action::Slot2) {
                lx.reading = None; // cancel sets the book down (B backs out one layer)
                dirty = true;
            }
            if dirty {
                redraw(&mut commands, &mut images, &old, &lx, &gather);
            }
            return; // the open book owns the inputs (no shelf browsing)
        }
    }

    let cur = dex::dex_nav(&state, BOOKS.len(), lx.cur, TOME_COLS);
    let cur = dex::dex_click(&ptr, BOOKS.len(), cur, TOME_COLS).unwrap_or(cur);
    if cur != lx.cur {
        lx.cur = cur;
        lx.page = 0; // a new shelf pick previews from page 1
        dirty = true;
    }
    if state.pressed(Action::Slot1) && found(BOOKS[lx.cur].id) {
        lx.reading = Some(BOOKS[lx.cur].id);
        lx.page = 0;
        dirty = true;
    }
    if dirty {
        redraw(&mut commands, &mut images, &old, &lx, &gather);
    }
}

fn redraw(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    old: &Query<Entity, With<LoreUi>>,
    lx: &LoreDex,
    gather: &GatherState,
) {
    for e in old {
        commands.entity(e).despawn();
    }
    let tag = || (CodexUi, TabContent, LoreUi);
    let found_n = BOOKS.iter().filter(|b| gather.tomes.contains(b.id)).count();
    let hdr = format!("TOMES  {found_n} / {}", BOOKS.len());
    label(commands, images, &hdr, dex::DEX_AX, 16.0, 0xc8b0e8, CONTENT_Z + 0.1, tag());

    let icons: Vec<Handle<Image>> = BOOKS
        .iter()
        .map(|b| images.add(bake(BOOK_GRID, &[('C', b.col), ('W', 0xf4ecd0)])))
        .collect();
    dex::draw_grid(
        commands,
        images,
        BOOKS.len(),
        lx.cur,
        TOME_COLS,
        |i| gather.tomes.contains(BOOKS[i].id),
        |i| Some((icons[i].clone(), 8.0)),
        tag(),
    );
    // Divider between the shelf and the reading pane.
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x2a, 0x2a, 0x30), Vec2::new(1.0, CANVAS_H as f32 - dex::DEX_GY - 14.0)),
        crate::gfx::at(TOME_RX - 6.0, dex::DEX_GY, 1.0, CANVAS_H as f32 - dex::DEX_GY - 14.0, CONTENT_Z),
        crate::gfx::PIXEL_LAYER,
        tag(),
    ));
    book_pane(commands, images, lx, gather, &BOOKS[lx.cur], tag());
}

/// The reading pane (js drawBookPane): backing, header, the page's lines, the footer.
fn book_pane(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    lx: &LoreDex,
    gather: &GatherState,
    b: &'static Book,
    tag: impl Bundle + Clone,
) {
    let (x, top) = (TOME_RX, dex::DEX_GY);
    let w = crate::CANVAS_W as f32 - 6.0 - x;
    let bot = CANVAS_H as f32 - 14.0;
    commands.spawn((
        Sprite::from_color(Color::srgba(0.086, 0.063, 0.11, 0.86), Vec2::new(w + 4.0, bot - top + 2.0)),
        crate::gfx::at(x - 2.0, top - 2.0, w + 4.0, bot - top + 2.0, CONTENT_Z - 0.05),
        crate::gfx::PIXEL_LAYER,
        tag.clone(),
    ));
    if lx.reading.is_some() {
        // Picked UP: the gold frame around the pane.
        for (sx, sy, sw, sh) in crate::ui::border_strips(x - 2.0, top - 2.0, w + 4.0, bot - top + 2.0, 1.0) {
            commands.spawn((
                Sprite::from_color(Color::srgb_u8(0xfc, 0xe0, 0xa8), Vec2::new(sw, sh)),
                crate::gfx::at(sx, sy, sw, sh, CONTENT_Z - 0.02),
                crate::gfx::PIXEL_LAYER,
                tag.clone(),
            ));
        }
    }
    if !gather.tomes.contains(b.id) {
        let cx = x + (w / 2.0).round();
        dex::center_label(commands, images, "?", cx, top + 28.0, 0x505058, CONTENT_Z + 0.1, tag.clone());
        dex::center_label(commands, images, "UNDISCOVERED", cx, top + 56.0, 0x606060, CONTENT_Z + 0.1, tag.clone());
        dex::center_label(commands, images, b.cat, cx, top + 66.0, 0x4a4a52, CONTENT_Z + 0.1, tag);
        return;
    }
    let pages = paginate(b);
    let page = lx.page.min(pages.len() - 1);
    label(commands, images, b.title, x, top + 2.0, 0xf0e4c0, CONTENT_Z + 0.1, tag.clone());
    let byline = format!("BY {}  -  {}", b.by, b.cat);
    label(commands, images, &byline, x, top + 12.0, b.col, CONTENT_Z + 0.1, tag.clone());
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x43, 0x36, 0x3a), Vec2::new(w, 1.0)),
        crate::gfx::at(x, top + 21.0, w, 1.0, CONTENT_Z),
        crate::gfx::PIXEL_LAYER,
        tag.clone(),
    ));
    let mut sy = top + 27.0;
    for ln in &pages[page] {
        match ln {
            None => sy += 5.0,
            Some(text) => {
                label(commands, images, text, x, sy, 0xcfc6b4, CONTENT_Z + 0.1, tag.clone());
                sy += 9.0;
            }
        }
    }
    let pg = format!(
        "{}PAGE {} / {}{}",
        if page > 0 { "< " } else { "  " },
        page + 1,
        pages.len(),
        if page < pages.len() - 1 { " >" } else { "  " },
    );
    dex::center_label(commands, images, &pg, x + w / 2.0, bot - 8.0, 0x8a8090, CONTENT_Z + 0.1, tag);
}

/// Setting the codex down closes any open book (fresh shelf next visit).
pub fn reset(mut lx: ResMut<LoreDex>) {
    lx.reading = None;
    lx.page = 0;
}
