//! station_art.rs — placed crafting stations, ported 1:1 from the JS interiors renderer
//! (interiors.js workTable / TABLE_STYLES / forge / stationSprite): ONE shared 2-tile
//! wooden body drawn per facing (0 front / 1 right / 2 back; LEFT = the right view
//! mirrored), dressed per kind by a palette + a decoration — a custom drawer (workbench,
//! wand) or a data `props` cluster that auto-anchors on the facing's surface rect.
//!
//! Baked per (kind, rot) into a 32x34 canvas: the station's tile-origin sits at row
//! [`OY`] so chimneys/crystals rise above the tile. The WELL keeps its char grid (it is
//! radially symmetric, rotation is meaningless); the cooking fire lives in cooking.rs.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const T: i32 = 16;
/// Canvas rows above the station's logical origin (the JS forge chimney tops at y-5).
pub const OY: i32 = 5;
/// The bake canvas (w, h): 2 tiles wide, OY headroom + the legs' reach below.
pub const CANVAS: (i32, i32) = (32, 34);

/// A tiny fill-rect canvas with the two mirror transforms the JS uses (canvas
/// scale(-1,1)/(1,-1) about a pivot). Pivots are DOUBLED so they stay integral (the
/// back-deco pivot is y+10.5).
struct Pen {
    buf: Vec<u8>,
    flip_x: Option<i32>, // doubled pivot: x' = p2 - x - w
    flip_y: Option<i32>, // doubled pivot: y' = p2 - y - h
}

impl Pen {
    fn new() -> Self {
        Pen { buf: vec![0u8; (CANVAS.0 * CANVAS.1 * 4) as usize], flip_x: None, flip_y: None }
    }
    fn rect(&mut self, col: u32, x: i32, y: i32, w: i32, h: i32) {
        let (mut x, mut y) = (x, y);
        if let Some(p2) = self.flip_x {
            x = p2 - x - w;
        }
        if let Some(p2) = self.flip_y {
            y = p2 - y - h;
        }
        let rgba = [(col >> 16) as u8, (col >> 8) as u8, col as u8, 0xff];
        for yy in y.max(0)..(y + h).min(CANVAS.1) {
            for xx in x.max(0)..(x + w).min(CANVAS.0) {
                let i = ((yy * CANVAS.0 + xx) * 4) as usize;
                self.buf[i..i + 4].copy_from_slice(&rgba);
            }
        }
    }
}

/// The facing's work surface — decorations anchor onto it (js workTable's return).
#[derive(Clone, Copy)]
struct Surf {
    sx: i32,
    sy: i32,
    sw: i32,
    sh: i32,
}

/// The shared 2-tile table body for a facing (js workTable, verbatim rects).
fn work_table(p: &mut Pen, x: i32, y: i32, top: u32, lite: u32, dark: u32, rot: u8) -> Surf {
    let w = 2 * T;
    const FOOT: u32 = 0x241810;
    if rot == 1 {
        // RIGHT-facing: top as a tall slab, work edge toward us (left).
        p.rect(dark, x + 9, y + T + 1, 3, 7);
        p.rect(dark, x + 20, y + T + 1, 3, 7);
        p.rect(FOOT, x + 9, y + T + 8, 3, 3);
        p.rect(FOOT, x + 20, y + T + 8, 3, 3);
        p.rect(top, x + 7, y + 1, 18, T + 1);
        p.rect(lite, x + 7, y + 1, 3, T + 1);
        p.rect(dark, x + 22, y + 1, 3, T + 1);
        p.rect(dark, x + 7, y + T + 1, 18, 2);
        let mut i = y + 5;
        while i < y + T - 1 {
            p.rect(dark, x + 11, i, 11, 1);
            i += 5;
        }
        return Surf { sx: x + 10, sy: y + 3, sw: 12, sh: T - 2 };
    }
    if rot == 2 {
        // BACK: same legs as the front (gravity!) — only the surface faces away.
        p.rect(dark, x + 3, y + T, 3, 7);
        p.rect(dark, x + w - 6, y + T, 3, 7);
        p.rect(FOOT, x + 3, y + T + 7, 3, 3);
        p.rect(FOOT, x + w - 6, y + T + 7, 3, 3);
        p.rect(top, x + 1, y + 3, w - 2, T - 1);
        p.rect(dark, x + 1, y + 3, w - 2, 2);
        p.rect(lite, x + 1, y + T, w - 2, 3);
        let mut i = x + 5;
        while i < x + w - 3 {
            p.rect(dark, i, y + 6, 1, T - 5);
            i += 6;
        }
        return Surf { sx: x + 1, sy: y + 5, sw: w - 2, sh: 6 };
    }
    // FRONT.
    p.rect(dark, x + 3, y + T, 3, 7);
    p.rect(dark, x + w - 6, y + T, 3, 7);
    p.rect(FOOT, x + 3, y + T + 7, 3, 3);
    p.rect(FOOT, x + w - 6, y + T + 7, 3, 3);
    p.rect(top, x + 1, y + 3, w - 2, T - 1);
    p.rect(lite, x + 1, y + 3, w - 2, 3);
    p.rect(dark, x + 1, y + T + 1, w - 2, 2);
    let mut i = x + 5;
    while i < x + w - 3 {
        p.rect(dark, i, y + 7, 1, T - 6);
        i += 6;
    }
    Surf { sx: x + 1, sy: y + 4, sw: w - 2, sh: T - 3 }
}

/// Workbench dressing: hammer, saw, vise, scattered nails (js workbenchDeco).
fn workbench_deco(p: &mut Pen, x: i32, y: i32, rot: u8) {
    if rot == 1 {
        p.rect(0x5a3a1c, x + 13, y + 3, 2, 7);
        p.rect(0x9a9a9a, x + 11, y + 3, 6, 2);
        p.rect(0xbcbcbc, x + 11, y + 9, 10, 2);
        p.rect(0x8a8a8a, x + 11, y + 10, 10, 1);
        p.rect(0x7c4c1c, x + 11, y + 9, 2, 4);
        p.rect(0x6a6a6a, x + 13, y + 13, 6, 4);
        p.rect(0x9a9a9a, x + 13, y + 13, 6, 1);
        p.rect(0xcaa84a, x + 17, y + 6, 1, 1);
        p.rect(0xcaa84a, x + 19, y + 12, 1, 1);
        return;
    }
    p.rect(0x9a9a9a, x + 4, y + 5, 7, 2);
    p.rect(0x5a3a1c, x + 4, y + 5, 2, 7);
    p.rect(0x7c4c1c, x + 13, y + 5, 2, 8);
    p.rect(0xbcbcbc, x + 12, y + 4, 9, 2);
    p.rect(0x8a8a8a, x + 12, y + 5, 9, 1);
    p.rect(0x6a6a6a, x + 23, y + 6, 6, 5);
    p.rect(0x9a9a9a, x + 23, y + 6, 6, 1);
    p.rect(0x3a3a3a, x + 25, y + 5, 2, 1);
    p.rect(0xcaa84a, x + 18, y + 11, 1, 1);
    p.rect(0xcaa84a, x + 20, y + 10, 1, 1);
    p.rect(0xcaa84a, x + 9, y + 11, 1, 1);
}

/// Arcane dressing: hovering rune-crystal, fire/frost runes, a candle (js wandDeco).
fn wand_deco(p: &mut Pen, x: i32, y: i32, rot: u8) {
    if rot == 1 {
        let cx = x + 16;
        p.rect(0xc0a0ff, cx - 3, y + 5, 6, 7);
        p.rect(0xe8d8ff, cx - 2, y + 3, 4, 3);
        p.rect(0xffffff, cx - 1, y + 7, 2, 2);
        p.rect(0x8a6ab0, cx - 4, y + 12, 8, 1);
        p.rect(0xfc7030, x + 12, y + 4, 4, 3);
        p.rect(0xfcae40, x + 13, y + 4, 2, 1);
        p.rect(0x7fd8ff, x + 12, y + 13, 4, 3);
        p.rect(0xcdeeff, x + 13, y + 13, 2, 1);
        return;
    }
    let cx = x + T;
    p.rect(0xc0a0ff, cx - 3, y, 6, 7);
    p.rect(0xe8d8ff, cx - 2, y - 2, 4, 3);
    p.rect(0xffffff, cx - 1, y + 2, 2, 2);
    p.rect(0x8a6ab0, cx - 4, y + 7, 8, 1);
    p.rect(0xfc7030, x + 4, y + 6, 4, 4);
    p.rect(0xfcae40, x + 5, y + 6, 2, 1);
    p.rect(0x7fd8ff, x + 24, y + 6, 4, 4);
    p.rect(0xcdeeff, x + 25, y + 6, 2, 1);
    p.rect(0xe8d8a0, x + 14, y + 9, 4, 3);
    p.rect(0xfcae40, x + 15, y + 7, 2, 2);
}

/// The forge: a stone furnace whose fire mouth, anvil and chimney move to the facing
/// (js forge, verbatim).
fn forge(p: &mut Pen, x: i32, y: i32, rot: u8) {
    const ST: u32 = 0x5a5660;
    const STL: u32 = 0x74707c;
    const DK: u32 = 0x26242c;
    const BLK: u32 = 0x181620;
    p.rect(ST, x + 2, y + 6, 28, 18);
    p.rect(STL, x + 2, y + 6, 28, 2);
    p.rect(STL, x + 2, y + 6, 2, 18);
    p.rect(DK, x + 28, y + 6, 2, 18);
    let mut yy = y + 11;
    while yy < y + 23 {
        p.rect(DK, x + 3, yy, 26, 1);
        yy += 5;
    }
    if rot == 2 {
        // BACK — solid rear wall, a tall chimney + a stacked coal pile.
        p.rect(DK, x + 12, y - 5, 8, 11);
        p.rect(ST, x + 12, y - 5, 8, 2);
        p.rect(0x3a3a44, x + 13, y - 4, 6, 2);
        p.rect(0x1a1a1a, x + 7, y + 13, 7, 7);
        p.rect(0x242424, x + 16, y + 14, 7, 6);
        p.rect(0x4a4a52, x + 9, y + 14, 2, 2);
        p.rect(0x4a4a52, x + 18, y + 15, 2, 2);
        return;
    }
    if rot == 1 {
        // RIGHT — fire mouth on the right face, anvil to the left.
        p.rect(BLK, x + 17, y + 11, 10, 11);
        p.rect(0xd83018, x + 18, y + 14, 8, 7);
        p.rect(0xfc7030, x + 19, y + 16, 6, 5);
        p.rect(0xffd040, x + 20, y + 17, 4, 4);
        p.rect(0x9a9aa2, x + 4, y + 1, 9, 2);
        p.rect(0x6a6a72, x + 6, y + 3, 5, 2);
        p.rect(0x52525a, x + 4, y + 5, 9, 1);
        p.rect(DK, x + 15, y - 1, 6, 7);
        p.rect(ST, x + 15, y - 1, 6, 2);
        return;
    }
    // FRONT — fire mouth centre, anvil on top, chimney hood right.
    p.rect(BLK, x + 11, y + 12, 10, 11);
    p.rect(0xd83018, x + 12, y + 15, 8, 7);
    p.rect(0xfc7030, x + 13, y + 17, 6, 5);
    p.rect(0xffd040, x + 14, y + 18, 4, 4);
    p.rect(0x9a9aa2, x + 4, y + 1, 9, 2);
    p.rect(0x6a6a72, x + 6, y + 3, 5, 2);
    p.rect(0x52525a, x + 4, y + 5, 9, 1);
    p.rect(DK, x + 22, y - 1, 6, 7);
    p.rect(ST, x + 22, y - 1, 6, 2);
}

/// A little colour-box cluster anchored at normalised (u, v) on the facing's surface —
/// the data path decorations rotate for free (js placeProp).
struct Prop {
    u: f32,
    v: f32,
    boxes: &'static [(i32, i32, i32, i32, u32)],
}

fn place_prop(p: &mut Pen, surf: Surf, pr: &Prop) {
    let ax = (surf.sx as f32 + pr.u * surf.sw as f32).round() as i32;
    let ay = (surf.sy as f32 + pr.v * surf.sh as f32).round() as i32;
    for &(dx, dy, w, h, col) in pr.boxes {
        p.rect(col, ax + dx, ay + dy, w, h);
    }
}

/// A station's dressing (js TABLE_STYLES deco/props split).
enum Deco {
    Workbench,
    Wand,
    Props(&'static [Prop]),
}

/// (top, lite, dark, dressing) per table kind (js TABLE_STYLES, palettes verbatim).
fn style(kind: &str) -> (u32, u32, u32, Deco) {
    match kind {
        "alchemy" => (0x27514a, 0x3a8270, 0x143029, Deco::Props(&[
            Prop { u: 0.42, v: 0.5, boxes: &[(-3, -1, 6, 6, 0x2e7d5a), (-2, 0, 4, 4, 0x46c98a), (-1, -6, 2, 5, 0xcfeede), (-2, 0, 4, 1, 0x8fe8b8), (0, -3, 1, 1, 0xd8ffe8)] },
            Prop { u: 0.76, v: 0.55, boxes: &[(-1, -2, 3, 5, 0x7a4ab0), (-1, -2, 3, 1, 0xb890e8), (0, -4, 1, 2, 0xcaa84a)] },
        ])),
        "enchanter" => (0x4a3a5e, 0x6a557e, 0x2a2038, Deco::Wand),
        "fletcher" => (0x5a4326, 0x7a5e38, 0x33240f, Deco::Props(&[
            Prop { u: 0.4, v: 0.5, boxes: &[(-1, -6, 2, 12, 0x6b4a2a), (-2, -6, 3, 1, 0x4a3018), (-2, 5, 3, 1, 0x4a3018), (0, -3, 1, 6, 0xcaa84a)] },
            Prop { u: 0.72, v: 0.45, boxes: &[(-3, 0, 6, 1, 0xbcbcbc), (2, -1, 2, 3, 0x8a8a8a), (-4, -1, 1, 3, 0xa0703a)] },
        ])),
        "jeweler" => (0x3a3a4e, 0x52526a, 0x1f1f2c, Deco::Props(&[
            Prop { u: 0.36, v: 0.5, boxes: &[(-2, -2, 4, 4, 0x4a9cff), (-1, -3, 2, 1, 0xbcdcff)] },
            Prop { u: 0.6, v: 0.55, boxes: &[(-2, -1, 4, 4, 0xd82800), (-1, -2, 2, 1, 0xfc7460)] },
            Prop { u: 0.82, v: 0.5, boxes: &[(-1, -2, 3, 1, 0xfcd000), (-2, -1, 1, 3, 0xfcd000), (2, -1, 1, 3, 0xfcd000), (-1, 2, 3, 1, 0xfcd000)] },
        ])),
        "farmtable" => (0x6e4a26, 0x8c6236, 0x3c2814, Deco::Props(&[
            Prop { u: 0.34, v: 0.55, boxes: &[(-3, 0, 6, 4, 0x5a3a1c), (-3, 0, 6, 1, 0x3a2410), (-1, -6, 2, 6, 0x3a8a3a), (-3, -4, 2, 1, 0x56b056), (1, -5, 2, 1, 0x56b056)] },
            Prop { u: 0.72, v: 0.5, boxes: &[(-2, -2, 5, 5, 0x9aa0a6), (-2, -2, 5, 1, 0xc2c8ce), (3, -1, 2, 1, 0x9aa0a6), (-4, -3, 2, 1, 0xc2c8ce), (-1, -5, 1, 3, 0x7e848a)] },
        ])),
        _ => (0x7c5226, 0x9a6a36, 0x43301a, Deco::Workbench), // workbench (the fallback)
    }
}

/// Body + dressing for a facing; the BACK mirrors the FRONT dressing top-to-bottom so
/// the tools sit on the far edge (js woodTable).
fn wood_table(p: &mut Pen, x: i32, y: i32, rot: u8, kind: &str) {
    let (top, lite, dark, deco) = style(kind);
    let surf = work_table(p, x, y, top, lite, dark, rot);
    let draw = |p: &mut Pen, r: u8, s: Surf| match &deco {
        Deco::Workbench => workbench_deco(p, x, y, r),
        Deco::Wand => wand_deco(p, x, y, r),
        Deco::Props(list) => {
            for pr in *list {
                place_prop(p, s, pr);
            }
        }
    };
    if rot == 2 {
        // Mirror the FRONT dressing about the slab centre (js cy = y + 10.5, doubled).
        let front = Surf { sx: x + 1, sy: y + 4, sw: 2 * T - 2, sh: T - 3 };
        p.flip_y = Some(2 * y + 21);
        draw(p, 0, front);
        p.flip_y = None;
        return;
    }
    draw(p, rot, surf);
}

/// Bake the station sprite for (kind, rot 0..=3) — left = the right view mirrored about
/// the 2-tile cell centre (js stationSprite). Draw the handle at (x, y - OY).
pub fn station_image(kind: &str, rot: u8, images: &mut Assets<Image>) -> Handle<Image> {
    let mut p = Pen::new();
    let (x, y) = (0, OY);
    let rot = rot % 4;
    let paint = |p: &mut Pen, r: u8| {
        if kind == "forge" {
            forge(p, x, y, r);
        } else {
            wood_table(p, x, y, r, kind);
        }
    };
    if rot == 3 {
        p.flip_x = Some(2 * (x + T));
        paint(&mut p, 1);
        p.flip_x = None;
    } else {
        paint(&mut p, rot);
    }
    images.add(Image::new(
        Extent3d { width: CANVAS.0 as u32, height: CANVAS.1 as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        p.buf,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ))
}

/// The WELL (js STRUCTURE, placed like a station): a stone shaft under a little roof,
/// water glinting far below — the watering-can refill point (farm.rs detects it).
/// Radially symmetric, so it keeps its char grid; rotation is a no-op.
pub const WELL: [&str; 22] = [
    "..........DDDDDDDDDD............",
    ".........DDDDDDDDDDDD...........",
    "........DDDDDDDDDDDDDD..........",
    "..........d........d...........",
    "..........d........d...........",
    "..........d........d...........",
    "..........d........d...........",
    ".......aAAAAAAAAAAAAAAAAa.......",
    ".......aAnnnnnnnnnnnnnnAa.......",
    ".......aAnKKKKKKKKKKKKnAa.......",
    ".......aAnKwwwwwwwwwwKnAa.......",
    ".......aAnKwwwwwwwwwwKnAa.......",
    ".......aAAAAAAAAAAAAAAAAa.......",
    ".......aAnAAnAAnAAnAAnAa........",
    ".......aAAAAAAAAAAAAAAAa........",
    ".......anAAnAAnAAnAAnAAa........",
    ".......aAAAAAAAAAAAAAAAa........",
    "........nnnnnnnnnnnnnn..........",
    "................................",
    "................................",
    "................................",
    "................................",
];
pub const WELL_PAL: &[(char, u32)] = &[
    ('D', 0x6b4a2a), // roof wood
    ('d', 0x3f2a14), // posts
    ('A', 0x9a9a86), // stone light
    ('a', 0x6a6a5a), // stone edge
    ('n', 0x53535a), // stone seam / shadow
    ('K', 0x2a2a30), // shaft dark
    ('w', 0x3a6ea0), // water far below
];

/// The line logged when a station is set down (js craftTable place messages).
pub fn place_msg(kind: &str) -> (&'static str, u32) {
    match kind {
        "well" => ("THE WELL IS DUG - THE CAN FILLS HERE", 0x4a9cff),
        "forge" => ("THE FORGE SETTLES ON ITS STONES", 0xd0822a),
        "alchemy" => ("THE ALCHEMY BENCH BUBBLES", 0x46c98a),
        "enchanter" => ("THE ENCHANTER'S TABLE HUMS", 0x9a7ad8),
        "fletcher" => ("THE FLETCHER'S BENCH IS READY", 0xcaa84a),
        "jeweler" => ("THE JEWELER'S BENCH GLINTS", 0x4a9cff),
        "farmtable" => ("THE FARM BENCH IS READY", 0x56b056),
        _ => ("THE WORKBENCH STANDS READY", 0x9a6a36),
    }
}
