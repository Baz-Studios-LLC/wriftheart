//! biomes.rs — the biome rule table (port of BIOMES in js/world.js).
//!
//! ORDER MATTERS: `BIOME_KEYS` must keep the JS object's declaration order — biome picking
//! (`biome_key_at`) and the shard-dungeon shuffle both walk it in order, and any reorder
//! changes every world. Keep this table byte-equivalent to the JS.

/// One biome's generation rules. Field meanings (from the JS):
/// `grass` = threshold for grass fields (HIGHER => less grass; >1 = none).
/// `water` = lake threshold (HIGHER => more lakes). Densities feed entity placement (later).
pub struct Biome {
    pub tier: i32,
    pub wall: char,
    pub water: f64,
    pub river: bool,
    pub grass: f64,
    pub ground: &'static str,
    pub alt: &'static str,
    pub alt_lvl: f64,
    pub tree_kind: Option<&'static str>,
    pub trees: i32,
    pub bushes: i32,
    pub boulders: i32,
    pub cacti: i32,
    pub mobs: i32,
    pub dungeon: f64,
    pub mob_kinds: &'static [&'static str],
}

macro_rules! biome {
    ($tier:expr, $wall:expr, $water:expr, $river:expr, $grass:expr, $ground:expr, $alt:expr,
     $alt_lvl:expr, $tree:expr, $trees:expr, $bushes:expr, $boulders:expr, $cacti:expr,
     $mobs:expr, $dungeon:expr, $kinds:expr) => {
        Biome {
            tier: $tier, wall: $wall, water: $water, river: $river, grass: $grass,
            ground: $ground, alt: $alt, alt_lvl: $alt_lvl, tree_kind: $tree, trees: $trees,
            bushes: $bushes, boulders: $boulders, cacti: $cacti, mobs: $mobs, dungeon: $dungeon,
            mob_kinds: $kinds,
        }
    };
}

/// Declaration order — the JS `Object.keys(BIOMES)` order. DO NOT REORDER.
pub const BIOME_KEYS: [&str; 28] = [
    "grassland", "forest", "desert", "mountains", "petalwood", "swamp", "graveyard", "arctic",
    "burnt", "hollowwood", "mushroom", "chaos", "embermaw", "greenmaw", "prismwastes",
    "blackdeep", "honeyglade", "bluebell", "suncoast", "stormreach", "tarmire", "galewind",
    "saltwastes", "witherlands", "wriftscar", "emberscar", "gloammoor", "starhollow",
];

/// Look up a biome's rules by key. Panics on an unknown key — every key in play comes from
/// `BIOME_KEYS`, so a miss is a port bug, not a data condition.
pub fn biome(key: &str) -> &'static Biome {
    match key {
        "grassland" => &biome!(0, 'T', 0.30, true, 0.50, "grass", "dirt", 0.66, Some("oak"), 1, 2, 2, 0, 2, 0.03, &["boar", "wasp", "wasp", "thornling", "glimmerling"]),
        "forest" => &biome!(0, 'T', 0.27, true, 0.42, "grass", "dirt", 0.62, Some("mix"), 4, 1, 1, 0, 2, 0.05, &["wolf", "wolf", "spider", "spider", "bear"]),
        "desert" => &biome!(1, 'S', 0.14, false, 2.0, "sand", "dirt", 0.60, None, 0, 1, 2, 3, 1, 0.04, &["scorpion", "scorpion", "vulture", "burrower", "sandmaw"]),
        "mountains" => &biome!(1, 'M', 0.22, true, 0.80, "dirt", "sand", 0.55, Some("pine"), 1, 0, 4, 0, 2, 0.07, &["bat", "bat", "hurler", "hurler", "golem"]),
        "petalwood" => &biome!(1, 'T', 0.25, true, 0.38, "grass", "dirt", 0.64, Some("blossom"), 4, 3, 1, 0, 1, 0.04, &["boar", "wasp", "thornling", "bellsnail", "glimmerling"]),
        "swamp" => &biome!(2, 'J', 0.40, true, 2.0, "bog", "mud", 0.48, Some("deadtree"), 3, 0, 0, 0, 2, 0.05, &["frog", "frog", "leech", "gnat", "lurker", "toxicslime", "mirefly", "boglight"]),
        "graveyard" => &biome!(3, 'X', 0.16, false, 2.0, "deadgrass", "gravedirt", 0.5, Some("deadtree"), 2, 0, 1, 0, 2, 0.11, &["skeleton", "archer", "zombie", "ghoul", "wraith", "revenant", "gravewarden"]),
        "arctic" => &biome!(2, 'I', 0.28, true, 0.90, "snow", "ice", 0.80, Some("pine"), 2, 0, 2, 0, 2, 0.05, &["frostmite", "frostmite", "frostslime", "icetroll", "frostwyrm"]),
        "burnt" => &biome!(4, 'H', 0.12, false, 2.0, "ash", "gravedirt", 0.5, Some("burnttree"), 3, 0, 1, 0, 2, 0.07, &["cinderhound", "cinderhound", "charbrute", "emberslime", "pyrewraith", "emberling"]),
        "hollowwood" => &biome!(4, 'N', 0.18, true, 0.85, "rotleaf", "gravedirt", 0.55, Some("deadtree"), 4, 0, 1, 0, 2, 0.07, &["wraith", "ghoul", "skeleton", "archer", "palehowler", "gravewarden"]),
        "mushroom" => &biome!(3, 'U', 0.26, true, 0.55, "spore", "grass", 0.62, Some("shroom"), 3, 2, 0, 0, 2, 0.05, &["sporeling", "sporeling", "myconid", "sporemother"]),
        "chaos" => &biome!(5, 'Z', 0.20, true, 0.70, "chaosground", "gravedirt", 0.5, Some("chaosmix"), 3, 1, 2, 0, 3, 0.10, &["chaoswisp", "voidling", "voidling", "riftlord", "switchshade"]),
        "embermaw" => &biome!(5, 'O', 0.0, false, 2.0, "basalt", "lava", 0.74, None, 0, 0, 3, 0, 2, 0.08, &["cinderhound", "charbrute", "pyrewraith", "emberslime", "emberling", "ashgeyser"]),
        "greenmaw" => &biome!(1, 'G', 0.30, true, 0.45, "jungle", "mud", 0.55, Some("jungletree"), 5, 3, 1, 0, 2, 0.05, &["boar", "spider", "wasp", "vinesnare", "vinesnare"]),
        "prismwastes" => &biome!(2, 'Y', 0.16, false, 2.0, "crystalground", "dirt", 0.55, Some("crystalspire"), 3, 0, 2, 0, 2, 0.06, &["golem", "bat", "slime", "prismshard", "prismshard"]),
        "blackdeep" => &biome!(5, 'C', 0.10, false, 2.0, "caverock", "gravedirt", 0.5, Some("stalagmite"), 5, 0, 3, 0, 2, 0.08, &["bat", "spider", "golem", "deepcrawler", "deepcrawler"]),
        "honeyglade" => &biome!(0, 'T', 0.28, true, 0.40, "meadow", "dirt", 0.66, Some("giantflower"), 4, 2, 1, 0, 1, 0.03, &["boar", "wasp", "honeydrone", "honeydrone", "glimmerling"]),
        "bluebell" => &biome!(0, 'T', 0.30, true, 0.45, "bluemeadow", "dirt", 0.64, Some("bluebloom"), 4, 2, 1, 0, 1, 0.03, &["boar", "wasp", "bellsnail", "honeydrone", "glimmerling"]),
        "suncoast" => &biome!(2, 'M', 0.42, false, 1.2, "wetsand", "sand", 0.50, None, 0, 1, 2, 0, 2, 0.05, &["frog", "leech", "tidecrab", "tidecrab", "mirefly"]),
        "stormreach" => &biome!(3, 'M', 0.15, false, 1.4, "stormrock", "dirt", 0.55, None, 0, 0, 4, 0, 2, 0.07, &["hurler", "bat", "sparkslime", "golem", "stormcaller", "stormcaller"]),
        "tarmire" => &biome!(3, 'J', 0.30, true, 2.0, "tar", "mud", 0.50, Some("deadtree"), 3, 0, 0, 0, 2, 0.06, &["leech", "lurker", "ghoul", "toxicslime", "boglight", "mirefly"]),
        "galewind" => &biome!(4, 'M', 0.18, true, 0.90, "steppe", "dirt", 0.58, Some("pine"), 1, 1, 2, 0, 2, 0.07, &["vulture", "hurler", "golem", "stormcaller"]),
        "saltwastes" => &biome!(4, 'S', 0.10, false, 2.0, "salt", "sand", 0.50, None, 0, 0, 3, 1, 1, 0.06, &["scorpion", "burrower", "hurler", "saltstatue", "sandmaw"]),
        "witherlands" => &biome!(5, 'X', 0.14, false, 1.6, "blight", "gravedirt", 0.50, Some("deadtree"), 3, 0, 1, 0, 2, 0.09, &["revenant", "wraith", "ghoul", "voidling", "witherheart"]),
        // TIER 6: THE WRIFTSCAR band — the wound's raw edge (the Black Castle sits just past it).
        "wriftscar" => &biome!(6, 'Z', 0.16, true, 0.75, "voidglass", "gravedirt", 0.5, Some("chaosmix"), 3, 1, 2, 0, 3, 0.11, &["voidling", "voidling", "chaoswisp", "switchshade", "riftlord"]),
        "emberscar" => &biome!(6, 'O', 0.0, false, 2.0, "basalt", "lava", 0.72, None, 0, 0, 3, 0, 3, 0.09, &["charbrute", "pyrewraith", "emberling", "emberslime", "cinderhound", "ashgeyser"]),
        "gloammoor" => &biome!(6, 'X', 0.22, true, 1.6, "blight", "mud", 0.5, Some("deadtree"), 3, 0, 1, 0, 3, 0.09, &["revenant", "wraith", "witherheart", "ghoul", "voidling"]),
        "starhollow" => &biome!(6, 'C', 0.12, false, 2.0, "caverock", "gravedirt", 0.5, Some("stalagmite"), 4, 0, 3, 0, 3, 0.08, &["deepcrawler", "deepcrawler", "golem", "voidling", "bat"]),
        other => panic!("unknown biome key: {other}"),
    }
}
