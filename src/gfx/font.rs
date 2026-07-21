//! font.rs — the 3x5 bitmap font (port of js/font.js).
//!
//! Canvas `fillText` antialiased and turned blurry when the low-res canvas scaled up, so the
//! JS drew glyphs as solid pixel rects. Here a whole string bakes into ONE tiny texture (crisp
//! by construction, one sprite per label). Glyphs are 3 wide x 5 tall except the wide ones
//! (M/N/W and the d-pad triangles, 5 wide); advance is width+1, space is 2.
//!
//! `measure` is pinned to the JS by `tests/font_parity.rs`.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const SPACE_ADV: i32 = 2;

/// Glyph rows ('1' = pixel) — port of `G` in js/font.js, verbatim.
fn glyph(ch: char) -> Option<&'static [&'static str]> {
    Some(match ch {
        'A' => &["010", "101", "111", "101", "101"],
        'B' => &["110", "101", "110", "101", "110"],
        'C' => &["011", "100", "100", "100", "011"],
        'D' => &["110", "101", "101", "101", "110"],
        'E' => &["111", "100", "110", "100", "111"],
        'F' => &["111", "100", "110", "100", "100"],
        'G' => &["011", "100", "101", "101", "011"],
        'H' => &["101", "101", "111", "101", "101"],
        'I' => &["111", "010", "010", "010", "111"],
        'J' => &["001", "001", "001", "101", "010"],
        'K' => &["101", "101", "110", "101", "101"],
        'L' => &["100", "100", "100", "100", "111"],
        'M' => &["10001", "11011", "10101", "10001", "10001"],
        'N' => &["10001", "11001", "10101", "10011", "10001"],
        'O' => &["111", "101", "101", "101", "111"],
        'P' => &["111", "101", "111", "100", "100"],
        'Q' => &["111", "101", "101", "111", "001"],
        'R' => &["110", "101", "110", "101", "101"],
        'S' => &["011", "100", "010", "001", "110"],
        'T' => &["111", "010", "010", "010", "010"],
        'U' => &["101", "101", "101", "101", "111"],
        'V' => &["101", "101", "101", "101", "010"],
        'W' => &["10001", "10001", "10101", "10101", "01010"],
        'X' => &["101", "101", "010", "101", "101"],
        'Y' => &["101", "101", "010", "010", "010"],
        'Z' => &["111", "001", "010", "100", "111"],
        '0' => &["111", "101", "101", "101", "111"],
        '1' => &["010", "110", "010", "010", "111"],
        '2' => &["111", "001", "111", "100", "111"],
        '3' => &["111", "001", "111", "001", "111"],
        '4' => &["101", "101", "111", "001", "001"],
        '5' => &["111", "100", "111", "001", "111"],
        '6' => &["111", "100", "111", "101", "111"],
        '7' => &["111", "001", "010", "010", "010"],
        '8' => &["111", "101", "111", "101", "111"],
        '9' => &["111", "101", "111", "001", "111"],
        '.' => &["000", "000", "000", "000", "010"],
        ',' => &["000", "000", "000", "010", "100"],
        '-' => &["000", "000", "111", "000", "000"],
        '+' => &["000", "010", "111", "010", "000"],
        '%' => &["101", "001", "010", "100", "101"],
        ':' => &["000", "010", "000", "010", "000"],
        ';' => &["000", "010", "000", "010", "100"],
        '\'' => &["010", "010", "000", "000", "000"],
        '’' => &["010", "010", "000", "000", "000"],
        '—' => &["000", "000", "111", "000", "000"],
        '|' => &["010", "010", "010", "010", "010"],
        '▲' => &["00000", "00100", "01110", "11111", "00000"],
        '▼' => &["00000", "11111", "01110", "00100", "00000"],
        '◀' => &["00010", "00110", "01110", "00110", "00010"],
        '▶' => &["01000", "01100", "01110", "01100", "01000"],
        '!' => &["010", "010", "010", "000", "010"],
        '/' => &["001", "001", "010", "100", "100"],
        '>' => &["100", "010", "001", "010", "100"],
        '<' => &["001", "010", "100", "010", "001"],
        '(' => &["001", "010", "010", "010", "001"],
        ')' => &["100", "010", "010", "010", "100"],
        '?' => &["111", "001", "011", "000", "010"],
        '*' => &["000", "101", "010", "101", "000"],
        _ => return None,
    })
}

fn glyph_w(ch: char) -> i32 {
    glyph(ch).map_or(3, |g| g[0].len() as i32)
}

/// Pixel width of `text` — port of `Font.measure` (uppercases, drops the trailing 1px gap).
pub fn measure(text: &str) -> i32 {
    let mut w = 0;
    for ch in text.to_uppercase().chars() {
        w += if ch == ' ' { SPACE_ADV } else { glyph_w(ch) + 1 };
    }
    if w > 0 { w - 1 } else { 0 }
}

/// Word-wrap at a pixel width (scale-1) — shared by any panel that flows prose.
pub fn wrap(text: &str, max_w: i32) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for w in text.split(' ') {
        let try_line = if line.is_empty() { w.to_string() } else { format!("{line} {w}") };
        if measure(&try_line) > max_w && !line.is_empty() {
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

/// Bake a whole string into one texture in `color` (0xRRGGBB) — the sprite-friendly analog of
/// `Font.draw`. The image is padded to even dimensions so a centre-anchored sprite still lands
/// on whole pixels. Unknown glyphs draw as '?', exactly like the JS.
pub fn bake_text(text: &str, color: u32, images: &mut Assets<Image>) -> (Handle<Image>, i32) {
    let up = text.to_uppercase();
    let w = measure(&up).max(1);
    let img_w = (w + (w & 1)) as u32; // pad odd widths
    let img_h = 6u32; // 5 rows + 1 pad row
    let mut img = Image::new_fill(
        Extent3d { width: img_w, height: img_h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let rgba = [(color >> 16) as u8, (color >> 8) as u8, color as u8, 0xff];
    let mut cx: i32 = 0;
    for ch in up.chars() {
        if ch == ' ' {
            cx += SPACE_ADV;
            continue;
        }
        let g = glyph(ch).or_else(|| glyph('?')).unwrap();
        let gw = g[0].len() as i32;
        for (r, row) in g.iter().enumerate() {
            for (c, bit) in row.bytes().enumerate() {
                if bit == b'1'
                    && let Ok(px) =
                        img.pixel_bytes_mut(UVec3::new((cx + c as i32) as u32, r as u32, 0))
                {
                    px.copy_from_slice(&rgba);
                }
            }
        }
        cx += gw + 1;
    }
    (images.add(img), w)
}
