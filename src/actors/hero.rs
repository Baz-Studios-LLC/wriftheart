//! hero.rs — the hero's sprite data + appearance recolor (port of the top of js/player.js).
//!
//! Three authored facings (down / up / right; LEFT is right flipped) x three leg poses,
//! played as a 4-frame gait [stand, step, stand, other-step]. Hairstyles are HAIR_EDITS
//! whole-row swaps applied before baking; worn-armor overlays come later.

use crate::gfx::bake::{bake, flip_h};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// A hero's chosen colours — port of `DEFAULT_LOOK` / the creator's appearance record.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Look {
    pub outfit_light: u32,
    pub outfit_dark: u32,
    pub hair_light: u32,
    pub hair_dark: u32,
    pub skin: u32,
    pub eye: u32,
    pub hair_style: String, // creator style id ('short'/'bangs'/…; 'bald' = the BAZ egg)
}

impl Default for Look {
    /// `DEFAULT_LOOK` (js/player.js): blue tunic, brown hair, light skin.
    fn default() -> Self {
        Self {
            outfit_light: 0x0058f8,
            outfit_dark: 0x0030a0,
            hair_light: 0x8a5a2a,
            hair_dark: 0x5a3a18,
            skin: 0xfcb888,
            eye: 0x20202c,
            hair_style: "short".into(),
        }
    }
}

// js NPC look pools (player.js): outfit/hair pairs, skins, styles — villagers roll theirs
// from a per-seed LCG so the same person always looks the same.
const NPC_OUTFIT: [(u32, u32); 9] = [
    (0x2f6fe0, 0x163f9c), (0xe23a2a, 0x8c1810), (0x3cba4a, 0x1c6e28), (0x8a5a2a, 0x5a3a18),
    (0x8890a0, 0x4a5260), (0xa64fe0, 0x5e2496), (0x2fc0b0, 0x147068), (0xfc8a30, 0xa8480f),
    (0xf0c030, 0x9a7410),
];
const NPC_HAIR: [(u32, u32); 5] = [
    (0x8a5a2a, 0x5a3a18), (0x4a4450, 0x26222c), (0xf0d070, 0xb89030), (0xc85a2a, 0x8c3416),
    (0xe0e0e8, 0xa0a0ac),
];
const NPC_SKIN: [u32; 5] = [0xfcd0a0, 0xf0b890, 0xd89860, 0xa06838, 0x6a4428];
const NPC_STYLE: [&str; 8] = ["short", "bangs", "parted", "mohawk", "long", "ponytail", "spiky", "topknot"];

/// A villager's stable appearance from their identity seed (js Player.randomLook — the
/// LCG draw order is part of the identity, don't reorder).
pub fn random_look(seed: u32) -> Look {
    let mut s = if seed == 0 { 1 } else { seed };
    let mut r = |n: usize| -> usize {
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        (s % n as u32) as usize
    };
    let o = NPC_OUTFIT[r(NPC_OUTFIT.len())];
    let h = NPC_HAIR[r(NPC_HAIR.len())];
    Look {
        outfit_light: o.0,
        outfit_dark: o.1,
        hair_light: h.0,
        hair_dark: h.1,
        skin: NPC_SKIN[r(NPC_SKIN.len())],
        eye: 0x20202c,
        hair_style: NPC_STYLE[r(NPC_STYLE.len())].into(),
    }
}

/// Hairstyle row swaps (js HAIR_EDITS, verbatim): (style, facing, row, replacement).
/// 'j'/'J' are OFF-HEAD hair (tails, manes, spikes, buns) a helmet never re-lids.
const HAIR_EDITS: &[(&str, &str, usize, &str)] = &[
    // bangs — a fringe over the forehead (back view unchanged)
    ("bangs", "down", 3, "....KhhsshhK...."),
    ("bangs", "right", 3, "..KhhsshhK......"),
    // parted — a centre part; side parts toward the front; a hint at the back crown
    ("parted", "down", 1, ".....KhHHhK....."),
    ("parted", "down", 2, "....KhhHHhhK...."),
    ("parted", "right", 1, "...KhHhhK......."),
    ("parted", "right", 2, "..KhhHhhhK......"),
    ("parted", "up", 3, "....KhhHHhhK...."),
    // mohawk — a strip of hair, shaved sides; the crest stands above the crown
    ("mohawk", "down", 0, ".......jj......."),
    ("mohawk", "down", 1, ".....KshhsK....."),
    ("mohawk", "down", 2, "....KsshhssK...."),
    ("mohawk", "up", 0, ".......jj......."),
    ("mohawk", "up", 1, ".....KsHHsK....."),
    ("mohawk", "up", 2, "....KsshhssK...."),
    ("mohawk", "up", 3, "....KsshhssK...."),
    ("mohawk", "up", 4, "....KsshhssK...."),
    ("mohawk", "up", 5, "....KsshhssK...."),
    ("mohawk", "right", 0, "....jjjj........"),
    ("mohawk", "right", 2, "..KshhhhsK......"),
    // long — a full mane: frames the face, spills over the shoulders, curtains the back
    ("long", "down", 2, "...KhhhhhhhhK..."),
    ("long", "down", 3, "...KjssssssjK..."),
    ("long", "down", 4, "...KjsIssIsjK..."),
    ("long", "down", 5, "...KjssssssjK..."),
    ("long", "down", 6, "...KJGggggGJK..."),
    ("long", "up", 6, "....KJJJJJJK...."),
    ("long", "up", 7, "...KGJJJJJJGK..."),
    ("long", "up", 8, "...KsgJJJJgsK..."),
    ("long", "right", 2, ".KjhhhhhhK......"),
    ("long", "right", 3, ".KjssssssK......"),
    ("long", "right", 4, ".KjssssIsK......"),
    ("long", "right", 5, ".KjssssssK......"),
    ("long", "right", 6, ".KJGgggggK......"),
    // ponytail — tidy front; a tied tail hangs down the back, pokes out in profile
    ("ponytail", "up", 5, "....KhhJJhhK...."),
    ("ponytail", "up", 6, "....KGgJJgGK...."),
    ("ponytail", "up", 7, "...KGggJJggGK..."),
    ("ponytail", "up", 8, "...KsggJJggsK..."),
    ("ponytail", "right", 2, ".jKhhhhhhK......"),
    ("ponytail", "right", 3, ".jKssssssK......"),
    ("ponytail", "right", 4, ".jKssssIsK......"),
    // spiky — spikes break the crown silhouette on every facing
    ("spiky", "down", 0, ".....j.jj.j....."),
    ("spiky", "up", 0, ".....j.jj.j....."),
    ("spiky", "right", 0, "...j.j.j........"),
    // topknot — a bun on the crown; sits toward the back in profile
    ("topknot", "down", 0, "......KjjK......"),
    ("topknot", "down", 1, ".....KhJJhK....."),
    ("topknot", "up", 0, "......KjjK......"),
    ("topknot", "up", 1, ".....KHJJHK....."),
    ("topknot", "right", 0, "..jjKKKK........"),
    ("topknot", "right", 1, "..jKhhhhK......."),
];

/// Re-shape a head grid for a hairstyle (js styleHead): whole-row swaps per facing;
/// 'bald' turns every hair pixel to skin (the BAZ easter egg).
fn style_head(grid: &[&str], style: &str, facing: &str) -> Vec<String> {
    if style.is_empty() || style == "short" {
        return grid.iter().map(|r| r.to_string()).collect();
    }
    if style == "bald" {
        return grid.iter().map(|r| r.replace(['h', 'H'], "s")).collect();
    }
    grid.iter()
        .enumerate()
        .map(|(i, row)| {
            HAIR_EDITS
                .iter()
                .find(|(s, f, r, _)| *s == style && *f == facing && *r == i)
                .map_or_else(|| row.to_string(), |(.., t)| t.to_string())
        })
        .collect()
}

// --- Frame grids, char-for-char from js/player.js -------------------------------------------

pub const DOWN_N: [&str; 16] = [
    "......KKKK......", ".....KhhhhK.....", "....KhhhhhhK....", "....KssssssK....",
    "....KsIssIsK....", "....KssssssK....", "....KGggggGK....", "...KGggggggGK...",
    "...KsggggggsK...", "....KggggggK....", "....KGGGGGGK....", "....KggggggK....",
    ".....dd..dd.....", ".....dd..dd.....", ".....ee..ee.....", "................",
];
const DOWN_L: [&str; 16] = [
    "......KKKK......", ".....KhhhhK.....", "....KhhhhhhK....", "....KssssssK....",
    "....KsIssIsK....", "....KssssssK....", "....KGggggGK....", "...KGggggggGK...",
    "...KsggggggsK...", "....KggggggK....", "....KGGGGGGK....", "....KggggggK....",
    ".....dd..dd.....", ".....dd..ee.....", ".....ee.........", "................",
];
const DOWN_R: [&str; 16] = [
    "......KKKK......", ".....KhhhhK.....", "....KhhhhhhK....", "....KssssssK....",
    "....KsIssIsK....", "....KssssssK....", "....KGggggGK....", "...KGggggggGK...",
    "...KsggggggsK...", "....KggggggK....", "....KGGGGGGK....", "....KggggggK....",
    ".....dd..dd.....", ".....ee..dd.....", ".........ee.....", "................",
];
const UP_N: [&str; 16] = [
    "......KKKK......", ".....KHHHHK.....", "....KHhhhhHK....", "....KhhhhhhK....",
    "....KhhhhhhK....", "....KhhhhhhK....", "....KGggggGK....", "...KGggggggGK...",
    "...KsggggggsK...", "....KggggggK....", "....KGGGGGGK....", "....KggggggK....",
    ".....dd..dd.....", ".....dd..dd.....", ".....ee..ee.....", "................",
];
const UP_L: [&str; 16] = [
    "......KKKK......", ".....KHHHHK.....", "....KHhhhhHK....", "....KhhhhhhK....",
    "....KhhhhhhK....", "....KhhhhhhK....", "....KGggggGK....", "...KGggggggGK...",
    "...KsggggggsK...", "....KggggggK....", "....KGGGGGGK....", "....KggggggK....",
    ".....dd..dd.....", ".....dd..ee.....", ".....ee.........", "................",
];
const UP_R: [&str; 16] = [
    "......KKKK......", ".....KHHHHK.....", "....KHhhhhHK....", "....KhhhhhhK....",
    "....KhhhhhhK....", "....KhhhhhhK....", "....KGggggGK....", "...KGggggggGK...",
    "...KsggggggsK...", "....KggggggK....", "....KGGGGGGK....", "....KggggggK....",
    ".....dd..dd.....", ".....ee..dd.....", ".........ee.....", "................",
];
const RIGHT_N: [&str; 16] = [
    "....KKKK........", "...KhhhhK.......", "..KhhhhhhK......", "..KssssssK......",
    "..KssssIsK......", "..KssssssK......", "..KGgggggK......", "..KgggggGK......",
    "..KgggggGK......", "..KgggggsK......", "..KGGGGGGK......", "..KggggggK......",
    "...dd..dd.......", "...dd..dd.......", "...ee..ee.......", "................",
];
const RIGHT_A2: [&str; 16] = [
    "....KKKK........", "...KhhhhK.......", "..KhhhhhhK......", "..KssssssK......",
    "..KssssIsK......", "..KssssssK......", "..KGgggggK......", "..KgggggGK......",
    "..KgggggGK......", "..KgggggsK......", "..KGGGGGGK......", "..KggggggK......",
    "...dd..dd.......", "...ee..dd.......", "........ee......", "................",
];
const RIGHT_B2: [&str; 16] = [
    "....KKKK........", "...KhhhhK.......", "..KhhhhhhK......", "..KssssssK......",
    "..KssssIsK......", "..KssssssK......", "..KGgggggK......", "..KgggggGK......",
    "..KgggggGK......", "..KgggggsK......", "..KGGGGGGK......", "..KggggggK......",
    "...dd..dd.......", "...dd..ee.......", "..ee............", "................",
];

/// Bake one hero frame in `look`'s colours — port of `bakeHero`.
///
/// The char contract (js/player.js): `g`/`G` outfit, `h`/`H` head hair, `j`/`J` OFF-head hair
/// (tails and manes a helmet must never re-lid), `s` skin, `I` eye, `e` shoe leather.
pub fn bake_hero(grid: &[&str], look: &Look) -> Image {
    bake(
        grid,
        &[
            ('g', look.outfit_light),
            ('G', look.outfit_dark),
            ('h', look.hair_light),
            ('H', look.hair_dark),
            ('j', look.hair_light),
            ('J', look.hair_dark),
            ('s', look.skin),
            ('I', look.eye),
            ('e', 0x3a2410), // shoe leather (boots recolor it)
        ],
    )
}

/// Facing index into [`HeroFrames`]: matches the JS `FRAMES` table's down/up/right/left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    Down = 0,
    Up = 1,
    Right = 2,
    Left = 3,
}

impl Facing {
    /// The unit step this facing points along (screen space, +y down) — THE one
    /// copy of the match that used to be pasted at every "in front of the hero".
    pub fn offset(self) -> (f32, f32) {
        match self {
            Facing::Up => (0.0, -1.0),
            Facing::Down => (0.0, 1.0),
            Facing::Left => (-1.0, 0.0),
            Facing::Right => (1.0, 0.0),
        }
    }
}

/// The 4-frame gait for all four facings — port of `buildFrames`.
/// Frame order per facing: [stand, step, stand, other step] (frames 0 and 2 share art).
pub struct HeroFrames {
    pub frames: [[Handle<Image>; 4]; 4],
}

pub fn build_frames(look: &Look, images: &mut Assets<Image>) -> HeroFrames {
    let style = look.hair_style.as_str();
    // Style the head rows per facing, THEN bake (js: styleHead before bakeHero).
    let baked = |g: &[&str], facing: &str, images: &mut Assets<Image>| {
        let styled = style_head(g, style, facing);
        let rows: Vec<&str> = styled.iter().map(|s| s.as_str()).collect();
        images.add(bake_hero(&rows, look))
    };
    // One gait: bake N/A/N/B (the JS reuses the N bitmap for frames 0 and 2 — a shared
    // handle preserves that).
    let gait = |n: &[&str], a: &[&str], b: &[&str], fc: &str, images: &mut Assets<Image>| {
        let hn = baked(n, fc, images);
        let ha = baked(a, fc, images);
        let hb = baked(b, fc, images);
        [hn.clone(), ha, hn, hb]
    };
    let down = gait(&DOWN_N, &DOWN_L, &DOWN_R, "down", images);
    let up = gait(&UP_N, &UP_L, &UP_R, "up", images);
    let right = gait(&RIGHT_N, &RIGHT_A2, &RIGHT_B2, "right", images);
    // LEFT = the styled RIGHT flipped, like Assets.flipH on the baked frames.
    let flip = |g: &[&str], images: &mut Assets<Image>| {
        let styled = style_head(g, style, "right");
        let refs: Vec<&str> = styled.iter().map(|s| s.as_str()).collect();
        let flipped = flip_h(&refs);
        let rows: Vec<&str> = flipped.iter().map(|s| s.as_str()).collect();
        images.add(bake_hero(&rows, look))
    };
    let ln = flip(&RIGHT_N, images);
    let la = flip(&RIGHT_A2, images);
    let lb = flip(&RIGHT_B2, images);
    let left = [ln.clone(), la, ln, lb];
    HeroFrames { frames: [down, up, right, left] }
}

// --- WORN ARMOR (js ARMOR_LOOK + bakeGeared + drawAccents): equipped head/body/feet
// pieces render ON the hero — armor recolors the placeholder pixels (outfit, hair
// dome, shoe leather) and each style paints its small accents (helm brow, miner
// lamp, crest ridge, crown points, plate seam, mail rings, rivets, belts, hems).
// DEVIATION (flagged): procedural armor pieces (it.armorLook) join the generator.

/// One armor id's worn look (js ARMOR_LOOK row).
pub struct ArmorLook {
    pub style: &'static str,
    pub lite: u32,
    pub dark: u32,
}

/// js ARMOR_LOOK, verbatim colours.
pub static ARMOR_LOOK: &[(&str, ArmorLook)] = &[
    // head
    ("leathercap", ArmorLook { style: "cap", lite: 0x9a6a32, dark: 0x5a3a18 }),
    ("paddedcoif", ArmorLook { style: "cap", lite: 0xaab0b8, dark: 0x5a5e66 }),
    ("rangerhood", ArmorLook { style: "hood", lite: 0x3c7040, dark: 0x1c4420 }),
    ("ironhelm", ArmorLook { style: "helm", lite: 0xc0c4cc, dark: 0x6a6e78 }),
    ("bronzehelm", ArmorLook { style: "helm", lite: 0xcc9648, dark: 0x7a5424 }),
    ("minerhelm", ArmorLook { style: "lamp", lite: 0x9aa0aa, dark: 0x545a64 }), // grey steel, not leather-brown (Baz)
    ("hornedhelm", ArmorLook { style: "horned", lite: 0x9094a0, dark: 0x42464e }),
    ("dragonhelm", ArmorLook { style: "crest", lite: 0xc0442e, dark: 0x5a1a12 }),
    ("magehat", ArmorLook { style: "hat", lite: 0x6450b0, dark: 0x2e2456 }),
    ("crownofvalor", ArmorLook { style: "crown", lite: 0xffd21e, dark: 0xa87c10 }),
    // body
    ("clothtunic", ArmorLook { style: "tunic", lite: 0xb4ac92, dark: 0x7a7460 }),
    ("leathervest", ArmorLook { style: "vest", lite: 0x92602e, dark: 0x5a3a18 }),
    ("studdedleather", ArmorLook { style: "studs", lite: 0x6e4e2c, dark: 0x3a2614 }),
    ("chainmail", ArmorLook { style: "mail", lite: 0xaab0b8, dark: 0x6a6e78 }),
    ("scalemail", ArmorLook { style: "mail", lite: 0x8e9aac, dark: 0x525e70 }),
    ("platemail", ArmorLook { style: "plate", lite: 0xc6cad2, dark: 0x7a7e88 }),
    ("magerobe", ArmorLook { style: "robe", lite: 0x6442a0, dark: 0x2e1c4a }),
    ("dragonscale", ArmorLook { style: "plate", lite: 0xc0442e, dark: 0x5a1a12 }),
    ("aegisplate", ArmorLook { style: "plate", lite: 0xe6d69a, dark: 0x9a7c40 }),
    // feet
    ("leatherboots", ArmorLook { style: "boots", lite: 0x854f1e, dark: 0x4a3018 }),
    ("travelboots", ArmorLook { style: "boots", lite: 0x946e3c, dark: 0x5a4220 }),
    ("sandals", ArmorLook { style: "boots", lite: 0xa87c42, dark: 0x6a4e28 }),
    ("swiftboots", ArmorLook { style: "boots", lite: 0x3c6e90, dark: 0x1c3a52 }),
    ("ironcladgreaves", ArmorLook { style: "greaves", lite: 0xb0b6be, dark: 0x5a5e66 }),
    ("bootsofhaste", ArmorLook { style: "boots", lite: 0xcaa84a, dark: 0x7a5e1e }),
    ("sevenleague", ArmorLook { style: "boots", lite: 0x7a3a8a, dark: 0x4a1c5a }),
];

/// The three worn looks (head, body, feet), by armor id.
pub type WornArm = [Option<&'static ArmorLook>; 3];

pub fn armor_look(id: &str) -> Option<&'static ArmorLook> {
    ARMOR_LOOK.iter().find(|(k, _)| *k == id).map(|(_, l)| l)
}

/// Styles whose helmet covers the head dome (recolour the hair); a crown sits on top.
fn head_covers(style: &str) -> bool {
    matches!(style, "cap" | "hood" | "helm" | "horned" | "crest" | "hat" | "lamp")
}

/// bakeGeared: the base hero bake with the armor's colours swapped over the
/// placeholder chars — body over outfit, a covering helm over the hair DOME only
/// (off-head j/J tails stay hair), boots over the shoe leather + trouser shaft.
fn bake_hero_geared(grid: &[&str], look: &Look, arm: &WornArm) -> Image {
    let mut pal: Vec<(char, u32)> = vec![
        ('g', look.outfit_light),
        ('G', look.outfit_dark),
        ('h', look.hair_light),
        ('H', look.hair_dark),
        ('j', look.hair_light),
        ('J', look.hair_dark),
        ('s', look.skin),
        ('I', look.eye),
        ('e', 0x3a2410),
    ];
    if let Some(b) = arm[1] {
        pal.push(('g', b.lite));
        pal.push(('G', b.dark));
    }
    if let Some(h) = arm[0].filter(|h| head_covers(h.style)) {
        pal.push(('h', h.lite));
        pal.push(('H', h.dark));
    }
    if let Some(f) = arm[2] {
        pal.push(('d', f.lite)); // boot shaft over the trousers
        pal.push(('e', f.dark)); // boot leather on the feet — tracks every walk frame
    }
    bake(grid, &pal)
}

/// drawAccents: small style marks poked over a baked 16x16 frame (js fillRects).
fn accent_pokes(img: &mut Image, arm: &WornArm, facing: &str) {
    let side = facing == "right" || facing == "left";
    let mut rect = |x: i32, y: i32, w: i32, h: i32, col: u32| {
        for yy in y..y + h {
            for xx in x..x + w {
                if xx < 0 || yy < 0 || xx >= 16 || yy >= 16 {
                    continue;
                }
                if let Ok(px) = img.pixel_bytes_mut(bevy::math::UVec3::new(xx as u32, yy as u32, 0)) {
                    px.copy_from_slice(&[(col >> 16) as u8, (col >> 8) as u8, col as u8, 255]);
                }
            }
        }
    };
    let (hx, hw) = (if side { 2 } else { 4 }, 8);
    let (bx, bw) = (if side { 2 } else { 4 }, if side { 9 } else { 8 });
    if let Some(h) = arm[0] {
        if matches!(h.style, "helm" | "horned" | "crest" | "lamp") {
            rect(hx, 3, hw, 1, h.dark); // brow
            rect(hx + 2, 1, hw - 4, 1, h.lite); // dome glint
        }
        if h.style == "lamp" {
            // the miner's headlamp, readable from EVERY facing (Baz): a bold lamp
            // front and in profile, the strap + peeking lamp tip from behind.
            if facing == "down" {
                rect(hx + hw / 2 - 2, 1, 3, 2, 0xffe27a);
                rect(hx + hw / 2 - 1, 1, 1, 1, 0xfff8d0);
            } else if side {
                rect(hx + hw - 2, 1, 1, 2, 0x2a2a30);
                rect(hx + hw - 1, 0, 1, 3, 0xffe27a);
                rect(hx + hw - 1, 1, 1, 1, 0xfff8d0);
            } else {
                rect(hx, 2, hw, 1, h.dark); // the strap across the crown
                rect(hx + hw / 2 - 1, 0, 2, 1, 0xffe27a); // the lamp tip peeking over
            }
        }
        if h.style == "horned" {
            rect(hx - if side { 0 } else { 1 }, 0, 1, 2, h.lite);
            rect(hx + hw - 1 + if side { 0 } else { 1 }, 0, 1, 2, h.lite);
        }
        if h.style == "crest" {
            for r in 0..3 {
                rect(hx + hw / 2 - 1 + r % 2, r, 1, 1, h.lite);
            }
        }
        if h.style == "hat" {
            rect(hx, 2, hw, 1, h.dark);
            rect(hx + hw / 2 - 1, 0, 2, 1, h.lite);
        }
        if h.style == "crown" {
            rect(hx, 2, hw, 1, h.lite);
            rect(hx + 1, 1, 1, 1, h.lite);
            rect(hx + hw / 2, 0, 1, 2, h.lite);
            rect(hx + hw - 2, 1, 1, 1, h.lite);
        }
    }
    if let Some(b) = arm[1] {
        let (by, bh) = (6, 6);
        match b.style {
            "plate" => {
                rect(bx + bw / 2, by + 1, 1, bh - 1, b.dark); // centre seam
                rect(bx, by + 3, bw, 1, b.dark); // plate line
                rect(bx, by, 2, 1, b.lite); // pauldrons
                rect(bx + bw - 2, by, 2, 1, b.lite);
            }
            "mail" => {
                let mut r = by + 1;
                while r < by + bh {
                    let mut cc = bx + if r & 1 == 1 { 1 } else { 0 };
                    while cc < bx + bw {
                        rect(cc, r, 1, 1, b.dark);
                        cc += 2;
                    }
                    r += 2;
                }
            }
            "studs" => {
                let mut cc = bx + 1;
                while cc < bx + bw {
                    rect(cc, by + 1, 1, 1, b.lite);
                    rect(cc, by + 4, 1, 1, b.lite);
                    cc += 2;
                }
            }
            "vest" | "tunic" => rect(bx, by + 3, bw, 1, b.dark),
            "robe" => {
                rect(bx + bw / 2, by + 1, 1, bh, b.dark); // tie
                rect(if side { 2 } else { 5 }, 12, 6, 1, b.lite); // hem over the legs
            }
            _ => {}
        }
    }
    if let Some(f) = arm[2]
        && f.style == "greaves"
    {
        rect(if side { 3 } else { 5 }, 12, 6, 1, f.lite); // knee plate over the trouser row
    }
}

/// Mirror a baked 16x16 frame (js F.left = F.right.map(flipH) AFTER the accents —
/// asymmetric marks like the miner lamp stay on the leading edge).
fn flip_image(img: &Image) -> Image {
    let mut out = img.clone();
    for y in 0..16u32 {
        for x in 0..16u32 {
            let src = img.pixel_bytes(bevy::math::UVec3::new(15 - x, y, 0)).unwrap().to_vec();
            if let Ok(px) = out.pixel_bytes_mut(bevy::math::UVec3::new(x, y, 0)) {
                px.copy_from_slice(&src);
            }
        }
    }
    out
}

/// buildGearedFrames: the full facing set in this gear (recoloured + accented).
pub fn build_frames_geared(look: &Look, arm: &WornArm, images: &mut Assets<Image>) -> HeroFrames {
    if arm.iter().all(|a| a.is_none()) {
        return build_frames(look, images); // bare: the plain bank (shared handles)
    }
    // BALD + covering helm: fall back to 'short' so the helm has dome pixels to paint.
    let covered = arm[0].is_some_and(|h| head_covers(h.style));
    let style = if look.hair_style == "bald" && covered { "short" } else { look.hair_style.as_str() };
    let mk = |g: &[&str], facing: &str| {
        let styled = style_head(g, style, facing);
        let rows: Vec<&str> = styled.iter().map(|s| s.as_str()).collect();
        let mut img = bake_hero_geared(&rows, look, arm);
        accent_pokes(&mut img, arm, facing);
        img
    };
    let gait = |n: &[&str], a: &[&str], b: &[&str], fc: &str, images: &mut Assets<Image>| {
        let hn = images.add(mk(n, fc));
        let ha = images.add(mk(a, fc));
        let hb = images.add(mk(b, fc));
        [hn.clone(), ha, hn, hb]
    };
    let down = gait(&DOWN_N, &DOWN_L, &DOWN_R, "down", images);
    let up = gait(&UP_N, &UP_L, &UP_R, "up", images);
    let right_imgs: Vec<Image> =
        [&RIGHT_N[..], &RIGHT_A2[..], &RIGHT_B2[..]].into_iter().map(|g| mk(g, "right")).collect();
    let left: Vec<Handle<Image>> = right_imgs.iter().map(|i| images.add(flip_image(i))).collect();
    let right: Vec<Handle<Image>> = right_imgs.into_iter().map(|i| images.add(i)).collect();
    HeroFrames {
        frames: [
            down,
            up,
            [right[0].clone(), right[1].clone(), right[0].clone(), right[2].clone()],
            [left[0].clone(), left[1].clone(), left[0].clone(), left[2].clone()],
        ],
    }
}
