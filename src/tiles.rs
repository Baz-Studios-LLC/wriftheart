//! tiles.rs — tile codes: solidity + placeholder colours.
//!
//! Solidity is the REAL contract (port of tiles.js TABLE `solid` flags) — collision and
//! connectivity depend on it. The colours are PLACEHOLDERS: flat fills per tile so the world
//! is visible and navigable; texture parity (speckled grounds, animated water, wall art,
//! shoreline rounding) is a later milestone and replaces `tile_color`, not the callers.

/// Is this tile code solid? Port of `Tiles.isSolid` — walls + water block; ground, paths,
/// roads and bridges walk. Unknown codes fall back to ground (walkable), like the JS TABLE.
pub fn is_solid(code: char) -> bool {
    matches!(
        code,
        'T' | 'R' | '~' | 'M' | 'S' | 'J' | 'X' | 'I' | 'H' | 'U' | 'Z' | 'O' | 'N' | 'G' | 'Y' | 'C'
    )
}

/// Flat base colour for a ground-type name — port of tiles.js GROUND_BASE (each ground's
/// `base` palette char, resolved through the shared palette).
pub fn ground_base(name: &str) -> u32 {
    let ch = match name {
        "sand" => 'y',
        "snow" => 'W',
        "ice" => 'F',
        "basalt" => 'n',
        "lava" => 'r',
        "rotleaf" => 'd',
        "ash" => 'n',
        "spore" => 'l',
        "chaosground" => 'X',
        "dirt" => 'D',
        "grass" => 'G',
        "voidglass" => 'z',
        "bog" => 'E',
        "mud" => 'd',
        "deadgrass" => 'Q',
        "gravedirt" => 'n',
        "jungle" => 'G',
        "crystalground" => 'X',
        "caverock" => 'n',
        "meadow" => 'g',
        "bluemeadow" => 'l',
        "wetsand" => 'Y',
        "stormrock" => 'n',
        "tar" => 'K',
        "steppe" => 'Q',
        "salt" => 'W',
        "blight" => 'Q',
        _ => 'y', // JS: GROUND_BASE[name] || GROUND_BASE.sand
    };
    crate::gfx::palette::palette(ch).unwrap_or(0xfce0a8)
}

/// Water base colour per style — port of `Tiles.waterColor`.
pub fn water_color(style: &str) -> u32 {
    if style == "murk" { 0x357066 } else { 0x0070ec }
}

/// PLACEHOLDER flat colour for a non-ground tile code (walls, paths, bridges). Chosen to read
/// like the real art's dominant tone so rooms are legible until tile-art parity lands.
pub fn overlay_color(code: char) -> u32 {
    match code {
        'T' | 'G' => 0x0c3014, // leafy walls (LEAF_WALL_DARK dome)
        'R' | 'M' => 0x808080, // rock walls
        'S' => 0xe0a060,       // sandstone
        'J' => 0x0a5a14,       // swamp thicket
        'X' => 0x585858,       // ruin wall
        'I' => 0x7fb8e0,       // ice wall
        'H' => 0x404040,       // charred wall
        'U' => 0xb0487a,       // fungal wall
        'Z' => 0x7028a8,       // chaos wall
        'O' => 0x1a1a22,       // obsidian
        'N' => 0x503000,       // gnarlwood
        'Y' => 0xb388ff,       // crystal wall
        'C' => 0x2a2a32,       // cavern wall
        '_' => 0xfce0a8,       // town street
        '=' => 0x7c4c1c,       // dirt road
        'p' => 0x585858,       // flagstone path
        'B' => 0x7c4c1c,       // bridge deck
        _ => 0xff00ff,         // unmapped: loud magenta so it's noticed
    }
}
