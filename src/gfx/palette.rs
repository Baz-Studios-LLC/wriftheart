//! palette.rs — the shared sprite palette (port of PALETTE in js/assets.js).
//!
//! All art is authored as grids of these chars. Keep this byte-identical to the JS PALETTE;
//! every sprite in the game depends on it.

/// Look up a palette char's colour as `0xRRGGBB`. `'.'` and any unmapped char are transparent.
pub fn palette(ch: char) -> Option<u32> {
    Some(match ch {
        'K' => 0x000000, // black (outlines)
        'W' => 0xfcfcfc, // white
        'g' => 0x00a800, // green (tunic)
        'G' => 0x007800, // dark green
        'E' => 0x0a5a14, // darkest green (foliage shadow)
        'l' => 0x74d07d, // light green (bushes)
        's' => 0xfcb888, // skin
        'S' => 0xd07840, // skin shadow / brown
        'b' => 0x0058f8, // blue (tunic)
        'B' => 0x0030a0, // dark blue (tunic shadow)
        'y' => 0xfce0a8, // sand / tan ground
        'Y' => 0xe0a060, // darker tan
        'r' => 0xd82800, // red
        'o' => 0xfc7460, // light red
        'q' => 0x7fa838, // olive green (goblin skin)
        'Q' => 0x5c7c24, // dark olive (goblin shadow)
        'u' => 0x4f9e8c, // teal-green (spear-goblin skin)
        'U' => 0x357066, // dark teal (spear-goblin shadow)
        'p' => 0xc87838, // copper
        'P' => 0xfcd000, // gold (coin glint)
        'd' => 0x503000, // dark brown (trunks, dirt)
        'D' => 0x7c4c1c, // brown
        'a' => 0x808080, // gray (rock)
        'A' => 0xbcbcbc, // light gray
        'n' => 0x585858, // dark gray (rock shadow)
        'w' => 0x3cbcfc, // water light
        'V' => 0x0070ec, // water dark
        'f' => 0xcfeeff, // frost light (arctic ice)
        'F' => 0x7fb8e0, // frost mid
        'x' => 0xb060f0, // chaos purple
        'X' => 0x7028a8, // chaos dark purple
        'm' => 0xff5cae, // whimsy pink / magenta (mushrooms)
        'c' => 0x6fe6e0, // crystal cyan (prismwastes)
        'v' => 0xb388ff, // crystal violet (prismwastes / blackdeep gleam)
        'z' => 0x241436, // deep void purple-black (Wriftscar void-glass, tier 6)
        _ => return None,
    })
}

/// Split `0xRRGGBB` into an opaque RGBA byte array.
pub fn rgba(hex: u32) -> [u8; 4] {
    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 0xff]
}
