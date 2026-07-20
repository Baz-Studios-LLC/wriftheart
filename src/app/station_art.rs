//! station_art.rs — the placed-station sprites (js Interiors.stationSprite / TABLE_STYLES).
//! The js draws every table with a rotation-aware vector renderer (woodTable + per-kind
//! deco/props, forge as a stone `special`); the port places stations at your feet
//! front-facing only (the cooking-fire idiom — ghost-placement + rot + home-interior
//! placement are the flagged crafting-overhaul deviations), so we bake ONE front view per
//! kind: a shared oak-table body under a themed centrepiece, matched to the js palettes.
//! Adding a station = one grid + one palette + a `station_art` arm.

/// The shared 2-tile table body (rows 12-21): lit top edge, slab, four legs, foot
/// shadow. Every table-style station reuses this shape; only the T/L/D/d palette
/// entries change per kind. Centrepieces stand on rows 4-11 (above the slab).
macro_rules! table_body {
    ($($top:literal),+ $(,)?) => {
        [
            $($top),+,
            "LLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLL",
            "TTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTT",
            "TLTTTTTTTTTTTTTTTTTTTTTTTTTTTLTT",
            "TTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTT",
            "DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD",
            ".DD..........................DD.",
            ".DD..........................DD.",
            ".DD..........................DD.",
            ".dd..........................dd.",
            "................................",
        ]
    };
}

// Cook fire lives in cooking.rs (the first station, hand-authored there). The rest:
const WORKBENCH: [&str; 22] = table_body!(
    "................................",
    "................................",
    "................................",
    "................................",
    "................................",
    ".........GGGGGGGGGG.............",
    ".........GGGGGGGGGG.............", // a planed board
    "....SSSSSS......................",
    "...S.....W.....nn...............", // a saw + two nails
    "...........G...nn...............",
    "................................",
    "................................",
);
const WORKBENCH_PAL: &[(char, u32)] = &[
    ('L', 0x9a6a36), ('T', 0x7c5226), ('D', 0x43301a), ('d', 0x2a1c0e),
    ('G', 0x9a6a36), ('S', 0xbcbcbc), ('W', 0x8a5a2c), ('n', 0x8a8a8a),
];

const ALCHEMY: [&str; 22] = table_body!(
    "................................",
    "................................",
    "................................",
    ".............w..................", // flask neck
    ".............w..................",
    "............www.....v...........", // flask body + a little vial
    "...........wwGww....v...........",
    "...........wwwww....V...........",
    "...........wwwww....c...........",
    "................................",
    "................................",
    "................................",
);
const ALCHEMY_PAL: &[(char, u32)] = &[
    ('L', 0x3a8270), ('T', 0x27514a), ('D', 0x143029), ('d', 0x0c201b),
    ('w', 0x2e7d5a), ('G', 0x46c98a), ('v', 0x7a4ab0), ('V', 0xb890e8), ('c', 0xcaa84a),
];

const ENCHANTER: [&str; 22] = table_body!(
    "................................",
    "................................",
    "...............C................", // a floating crystal
    "..............CCC...............",
    ".............CCWCC..............",
    "..............CCC...............",
    "...............C................",
    ".........r...........r..........", // two runes
    "........rRr.........rRr.........",
    ".........r...........r..........",
    "................................",
    "................................",
);
const ENCHANTER_PAL: &[(char, u32)] = &[
    ('L', 0x6a557e), ('T', 0x4a3a5e), ('D', 0x2a2038), ('d', 0x1a1426),
    ('C', 0x9a7ad8), ('W', 0xe0d0ff), ('r', 0x8f6ad0), ('R', 0xd8c4ff),
];

const FLETCHER: [&str; 22] = table_body!(
    "................................",
    "................................",
    "...........b....................", // a bow (limbs + string)
    "..........b.s...................",
    ".........b..s...................",
    ".........b..s....AAAAAAA........", // an arrow
    ".........b..s...H.......n.......",
    ".........b..s....AAAAAAA........",
    "..........b.s...................",
    "...........b....................",
    "................................",
    "................................",
);
const FLETCHER_PAL: &[(char, u32)] = &[
    ('L', 0x7a5e38), ('T', 0x5a4326), ('D', 0x33240f), ('d', 0x201608),
    ('b', 0x6b4a2a), ('s', 0xcaa84a), ('A', 0xbcbcbc), ('H', 0xa0703a), ('n', 0x8a8a8a),
];

const JEWELER: [&str; 22] = table_body!(
    "................................",
    "................................",
    "................................",
    "................................",
    "...........b...r....gg..........", // blue gem, red gem, gold ring
    "..........bBb.rRr..g..g.........",
    "...........b...r...g..g.........",
    "...................gg...........",
    "................................",
    "................................",
    "................................",
    "................................",
);
const JEWELER_PAL: &[(char, u32)] = &[
    ('L', 0x52526a), ('T', 0x3a3a4e), ('D', 0x1f1f2c), ('d', 0x121218),
    ('b', 0x4a9cff), ('B', 0xbcdcff), ('r', 0xd82800), ('R', 0xfc7460), ('g', 0xfcd000),
];

const FARMTABLE: [&str; 22] = table_body!(
    "................................",
    "................................",
    "................................",
    "..............l.................", // seedling
    ".............lGl....cccc........", // watering can
    "...........l.l.l..cc....c.......",
    "..............l..cccccc.c.......",
    "..........pppppp.cccccc.........",
    "..........pppppp................", // pot
    "................................",
    "................................",
    "................................",
);
const FARMTABLE_PAL: &[(char, u32)] = &[
    ('L', 0x8c6236), ('T', 0x6e4a26), ('D', 0x3c2814), ('d', 0x261a0c),
    ('l', 0x3a8a3a), ('G', 0x56b056), ('p', 0x5a3a1c), ('c', 0x9aa0a6),
];

/// The forge — a stone furnace, not a table (js `special`): stacked stone with a
/// glowing arched mouth and a smoking chimney.
const FORGE: [&str; 22] = [
    "................................",
    ".........m......................",
    "........m.m.....................",
    ".........m......................",
    ".....SSSSSSSSSSSSSSSSSSSS........", // chimney cap
    ".....S..............S...........",
    ".....SSSSSSSSSSSSSSSSSSSS........",
    "....SSSSSSSSSSSSSSSSSSSSSS.......",
    "....SsSSSsSSSSsSSSsSSSSsSS.......", // stone courses
    "....SSSSSSSSSSSSSSSSSSSSSS.......",
    "....SSSS..........SSSSSSSS.......", // arched mouth
    "....SSS.EEEEEEEEEE.SSSsSSS.......",
    "....SSS.EFFFFFFFFE.SSSSSSS.......",
    "....SsS.EFFFFFFFFE.SSSSSSS.......", // glowing coals
    "....SSS.EEEEEEEEEE.SSSsSSS.......",
    "....SSSSSSSSSSSSSSSSSSSSSS.......",
    "....SSSsSSSSSSSSsSSSSSSSSS.......",
    "....SSSSSSSSSSSSSSSSSSSSSS.......",
    "....dddddddddddddddddddddd.......",
    "................................",
    "................................",
    "................................",
];
const FORGE_PAL: &[(char, u32)] = &[
    ('m', 0x9f9f9f), // smoke
    ('S', 0x6a6a72), // stone
    ('s', 0x53535a), // stone, darker course
    ('E', 0xb5651d), // coal glow, rim
    ('F', 0xfcae40), // coal glow, hot core
    ('d', 0x2a2a2e), // ground shadow
];

/// The WELL (js STRUCTURE, placed like a station): a stone shaft under a little roof,
/// water glinting far below — the watering-can refill point (farm.rs detects it).
const WELL: [&str; 22] = [
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
const WELL_PAL: &[(char, u32)] = &[
    ('D', 0x6b4a2a), // roof wood
    ('d', 0x3f2a14), // posts
    ('A', 0x9a9a86), // stone light
    ('a', 0x6a6a5a), // stone edge
    ('n', 0x53535a), // stone seam / shadow
    ('K', 0x2a2a30), // shaft dark
    ('w', 0x3a6ea0), // water far below
];

/// Grid + palette for a placed station kind (falls back to the workbench look).
pub fn station_art(kind: &str) -> (&'static [&'static str], &'static [(char, u32)]) {
    match kind {
        "well" => (&WELL, WELL_PAL),
        "forge" => (&FORGE, FORGE_PAL),
        "alchemy" => (&ALCHEMY, ALCHEMY_PAL),
        "enchanter" => (&ENCHANTER, ENCHANTER_PAL),
        "fletcher" => (&FLETCHER, FLETCHER_PAL),
        "jeweler" => (&JEWELER, JEWELER_PAL),
        "farmtable" => (&FARMTABLE, FARMTABLE_PAL),
        _ => (&WORKBENCH, WORKBENCH_PAL),
    }
}

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
