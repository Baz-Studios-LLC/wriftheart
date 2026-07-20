//! themes.rs — the dungeon theme palettes + per-theme enemy rosters (js/dungeon.js
//! THEMES + ENEMY_POOL, field-for-field; hex colours as 0xRRGGBB).

/// Wall/floor drawing style (js `style` — absent means brick).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Brick,
    Cave,
    Hall,
}

pub struct Theme {
    pub key: &'static str,
    pub name: &'static str,
    pub style: Style,
    pub floor: u32,
    pub floor_alt: u32,
    pub wall: u32,
    pub wall_top: u32,
    pub grout: u32,
    /// Cave pools ([still, ripple] — lava tubes pool MOLTEN, frost caverns freeze).
    pub pool: Option<[u32; 2]>,
    pub tint: [u8; 3],
    /// A theme can opt out of dungeon-dark (the guildhall is a lit building).
    pub amb_alpha: Option<f32>,
}

#[allow(clippy::too_many_arguments)] // a palette row is colours all the way down
const fn brick(key: &'static str, name: &'static str, floor: u32, floor_alt: u32, wall: u32, wall_top: u32, grout: u32, tint: [u8; 3]) -> Theme {
    Theme { key, name, style: Style::Brick, floor, floor_alt, wall, wall_top, grout, pool: None, tint, amb_alpha: None }
}
#[allow(clippy::too_many_arguments)] // a palette row is colours all the way down
const fn cave(key: &'static str, name: &'static str, floor: u32, floor_alt: u32, wall: u32, wall_top: u32, grout: u32, pool: [u32; 2], tint: [u8; 3]) -> Theme {
    Theme { key, name, style: Style::Cave, floor, floor_alt, wall, wall_top, grout, pool: Some(pool), tint, amb_alpha: None }
}

pub static THEMES: &[Theme] = &[
    cave("cave", "CAVE", 0x453626, 0x3a2d1f, 0x57534d, 0x6e6a62, 0x241f18, [0x1c3448, 0x2c5068], [14, 12, 18]),
    brick("crypt", "CRYPT", 0x2f3238, 0x282b31, 0x4a4f57, 0x5d636c, 0x1b1d22, [12, 16, 28]),
    brick("ruins", "RUINS", 0x33382e, 0x2d3228, 0x566048, 0x6b765a, 0x1f2418, [14, 22, 14]),
    brick("tomb", "TOMB", 0x4a4030, 0x43392b, 0x7c6a48, 0x96825a, 0x2c2516, [26, 20, 10]),
    brick("castle", "CASTLE", 0x34323c, 0x2e2c36, 0x5b5868, 0x716d80, 0x1e1d26, [18, 14, 30]),
    brick("bog", "BOG", 0x2e3a30, 0x28332b, 0x3c4a3a, 0x4e6048, 0x19211a, [12, 26, 18]),
    // Underground-biome caves (cracked walls + cave doors).
    cave("crystalcave", "CRYSTAL CAVERN", 0x2a2440, 0x241f38, 0x4a3f7a, 0x6a5aa0, 0x1a1630, [0x221c3c, 0x4a3f88], [22, 14, 36]),
    cave("fungal", "FUNGAL GROTTO", 0x283832, 0x22322c, 0x3a5a4a, 0x4e7a64, 0x16241e, [0x1e3428, 0x2c5040], [14, 30, 22]),
    cave("lavatube", "LAVA TUBE", 0x3a2420, 0x321e1a, 0x5a3028, 0x7a4030, 0x220f0c, [0x8a2808, 0xfc6020], [34, 12, 6]),
    cave("darkdepths", "THE DARK DEPTHS", 0x1e1c24, 0x1a1820, 0x33303c, 0x46424f, 0x100e14, [0x0e1420, 0x1c2c44], [10, 8, 16]),
    cave("frostcavern", "FROST CAVERN", 0x2a343e, 0x243038, 0x45596e, 0x5e7896, 0x161e26, [0x8ab8dc, 0xc8e8f8], [12, 18, 30]),
    // Land themes: every shard dungeon wears its land's interior (game.rs THEME_BY_BIOME).
    brick("ossuary", "THE OSSUARY", 0x3e3a32, 0x37342c, 0x8a8474, 0xa8a292, 0x26221c, [18, 16, 12]),
    brick("charhall", "THE CHAR HALLS", 0x2e2a28, 0x282422, 0x4a3a30, 0x5e4a3a, 0x1a1512, [24, 14, 8]),
    brick("riftvault", "THE RIFT VAULT", 0x2c2438, 0x261f30, 0x5a3a8a, 0x7a54b0, 0x181026, [26, 12, 36]),
    cave("wriftvault", "THE WRIFT VAULT", 0x241a34, 0x1e152c, 0x4a2e76, 0x6a44a8, 0x140c22, [0x2a1c48, 0x5c3ac0], [30, 14, 44]),
    brick("petalhall", "THE PETAL HALLS", 0x3c3028, 0x352a22, 0x8a4a6a, 0xb06a8a, 0x241a14, [28, 16, 22]),
    brick("hivehollow", "THE HIVE HOLLOW", 0x443622, 0x3c301e, 0x8a6a2a, 0xb08a3a, 0x2a2012, [30, 22, 8]),
    brick("bellbarrow", "THE BELL BARROW", 0x2e3240, 0x282c38, 0x4a5a9a, 0x6274bc, 0x1a1d28, [14, 18, 32]),
    brick("vinewarren", "THE VINE WARREN", 0x2a3426, 0x242e20, 0x3a5a2e, 0x4e783c, 0x161f12, [12, 26, 12]),
    brick("searuin", "THE DROWNED HALLS", 0x2e3a3a, 0x283434, 0x4a7a72, 0x5e9a8e, 0x182422, [10, 26, 26]),
    brick("stormspire", "THE STORM SPIRE", 0x30343c, 0x2a2e36, 0x525e74, 0x6a7a96, 0x1c1f26, [14, 18, 30]),
    brick("tarpit", "THE TAR WARRENS", 0x26241e, 0x211f1a, 0x3c3a2e, 0x4e4a3a, 0x141310, [16, 16, 8]),
    brick("windbarrow", "THE WIND BARROW", 0x3c382c, 0x353126, 0x6e6448, 0x8a7e5a, 0x242014, [24, 20, 10]),
    brick("saltmine", "THE SALT MINE", 0x3a3e40, 0x34383a, 0x9aa4a8, 0xbcc6ca, 0x26292b, [18, 20, 22]),
    brick("hollowroot", "THE HOLLOW ROOT", 0x32322a, 0x2c2c24, 0x5c5a48, 0x74705a, 0x1e1e18, [18, 18, 14]),
    brick("blightvault", "THE BLIGHT VAULT", 0x32362a, 0x2c3024, 0x565e3a, 0x6e7a4a, 0x1e2216, [18, 24, 10]),
    brick("saltmaze", "THE SALTMAZE", 0x3e4244, 0x383c3e, 0xaab4b8, 0xccd6da, 0x2a2d2f, [16, 18, 20]),
    // A city's great hall: honey planks + timber panels, lit like a building (NOT a dungeon).
    Theme {
        key: "guildhall",
        name: "THE GUILDHALL",
        style: Style::Hall,
        floor: 0x9a8154,
        floor_alt: 0xa88e60,
        wall: 0x4e3a28,
        wall_top: 0x66503a,
        grout: 0x32241a,
        pool: None,
        tint: [42, 30, 18],
        amb_alpha: Some(0.28),
    },
];

pub fn theme(key: &str) -> &'static Theme {
    THEMES.iter().find(|t| t.key == key).unwrap_or(&THEMES[0])
}

/// js ENEMY_POOL — each theme's roster (duplicates spawn more often).
pub static ENEMY_POOL: &[(&str, [&str; 5])] = &[
    ("cave", ["bat", "bat", "hurler", "slime", "skeleton"]),
    ("crypt", ["zombie", "skeleton", "archer", "ghoul", "gravewarden"]),
    ("ruins", ["spider", "spider", "thornling", "slime", "gnat"]),
    ("tomb", ["skeleton", "archer", "scorpion", "burrower", "sandmaw"]),
    ("castle", ["skeleton", "archer", "goblin", "slinger", "redgoblin"]), // js also lists a 6th (hurler) — see pool()
    ("bog", ["frog", "frog", "leech", "mirefly", "lurker"]),
    ("crystalcave", ["golem", "bat", "slime", "prismshard", "prismshard"]),
    ("fungal", ["sporeling", "sporeling", "slime", "spider", "myconid"]),
    ("lavatube", ["cinderhound", "cinderhound", "emberling", "charbrute", "ashgeyser"]),
    ("darkdepths", ["bat", "spider", "deepcrawler", "deepcrawler", "wraith"]),
    ("frostcavern", ["frostmite", "frostmite", "bat", "icetroll", "slime"]),
    ("ossuary", ["skeleton", "archer", "zombie", "ghoul", "gravewarden"]),
    ("charhall", ["cinderhound", "emberling", "bat", "pyrewraith", "charbrute"]),
    ("riftvault", ["chaoswisp", "voidling", "voidling", "switchshade", "wraith"]),
    ("wriftvault", ["voidling", "voidling", "switchshade", "chaoswisp", "riftlord"]),
    ("petalhall", ["wasp", "wasp", "thornling", "bellsnail", "slime"]),
    ("hivehollow", ["wasp", "honeydrone", "honeydrone", "gnat", "thornling"]),
    ("bellbarrow", ["wasp", "bellsnail", "bellsnail", "honeydrone", "spider"]),
    ("vinewarren", ["spider", "spider", "vinesnare", "vinesnare", "lurker"]),
    ("searuin", ["frog", "leech", "tidecrab", "tidecrab", "mirefly"]),
    ("stormspire", ["hurler", "stormcaller", "stormcaller", "golem", "slinger"]),
    ("tarpit", ["leech", "leech", "lurker", "boglight", "boglight"]),
    ("windbarrow", ["vulture", "hurler", "stormcaller", "golem", "slinger"]),
    ("saltmine", ["scorpion", "burrower", "hurler", "saltstatue", "sandmaw"]),
    ("hollowroot", ["wraith", "ghoul", "palehowler", "gravewarden", "zombie"]),
    ("blightvault", ["revenant", "wraith", "ghoul", "voidling", "witherheart"]),
    ("saltmaze", ["cultist", "cultist", "saltstatue", "wraith", "slinger"]),
    ("guildhall", ["skeleton", "skeleton", "skeleton", "skeleton", "skeleton"]), // never used (the hall spawns no foes)
];

/// A theme's roster. NOTE the js `castle` pool has SIX entries (the array type here holds
/// the first five) — pool() special-cases it so the roll `floor(r()*len)` still spans all six.
pub fn pool(theme_key: &str) -> &'static [&'static str] {
    static CASTLE6: [&str; 6] = ["skeleton", "archer", "goblin", "slinger", "redgoblin", "hurler"];
    if theme_key == "castle" {
        return &CASTLE6;
    }
    ENEMY_POOL
        .iter()
        .find(|(k, _)| *k == theme_key)
        .map(|(_, p)| p.as_slice())
        .unwrap_or_else(|| ENEMY_POOL[0].1.as_slice())
}

/// Re-intern a saved roster kind to its &'static str (rosters only ever hold names
/// from these tables, so a miss means a stale save — the caller skips it).
pub fn intern_kind(s: &str) -> Option<&'static str> {
    const EXTRA: [&str; 5] = ["ogre", "golem", "revenant", "charbrute", "mimic"]; // maze/stairs guards
    if let Some(k) = EXTRA.iter().find(|k| **k == s) {
        return Some(k);
    }
    for (_, p) in ENEMY_POOL {
        if let Some(k) = p.iter().find(|k| **k == s) {
            return Some(k);
        }
    }
    pool("castle").iter().find(|k| **k == s).copied()
}
