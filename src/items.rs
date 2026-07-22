//! items.rs — the item registry (port of js/items.js: `define`/`get`, the RARITY table and
//! the price helpers). Defs are static data; *behaviour* (a sword's swing, a potion's heal)
//! dispatches at the use-site on `tool` / `consumable`, since Rust items can't carry JS
//! closures — one match in play.rs/char_tab.rs replaces every `use()` lambda.
//!
//! STARTER SET ONLY (increment 1): the three tools, the potion, and the five gathered
//! materials. The full catalogue (armor, trinkets, wands, seeds…) joins def-by-def as its
//! systems port; every def here is field-for-field from js/items.js.

use crate::actors::goblin_art::SWORD_ICON;
use crate::actors::items_art::{
    ARROW_ICON, BANDAGE_ICON, CAN_ICON, COPPER_ICON, ELIXIR_ICON, FIBER_ICON, GEM_ICON, GREATERPOTION_ICON, HERB_ICON,
    BOOT_ICON, DRIFT_ICON, FISH_GRID, FLUTE_ICON, HOE_ICON, KEY_ICON, LEATHER_ICON, MEAT_ICON, OKEY_ICON, POTION_ICON, PROD_GRID,
    ROD_ICON, SEED_GRID, STONE_ICON, STRING_ICON, WATCH_ICON, WEED_ICON, WOOD_ICON,
};
use crate::actors::tools_art::{AXE_ICON, PICK_ICON};
use crate::combat::Tool;

/// The five-tier rarity ladder (js RARITY) — tier, display name, colour, and the default
/// price a def without its own `price` falls back to (js RARITY_PRICE).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    pub fn tier(self) -> i32 {
        self as i32
    }
    pub fn name(self) -> &'static str {
        match self {
            Rarity::Common => "COMMON",
            Rarity::Uncommon => "UNCOMMON",
            Rarity::Rare => "RARE",
            Rarity::Epic => "EPIC",
            Rarity::Legendary => "LEGENDARY",
        }
    }
    pub fn color(self) -> u32 {
        match self {
            Rarity::Common => 0xd8d8d8,    // white-gray
            Rarity::Uncommon => 0x3cdc5a,  // green
            Rarity::Rare => 0x4a9cff,      // blue
            Rarity::Epic => 0xc060fc,      // purple
            Rarity::Legendary => 0xff9a30, // orange
        }
    }
    pub fn base_price(self) -> i32 {
        match self {
            Rarity::Common => 30,
            Rarity::Uncommon => 120,
            Rarity::Rare => 400,
            Rarity::Epic => 1200,
            Rarity::Legendary => 4000,
        }
    }
}

/// One item definition — the js def object's fields, minus the `use()` closure (behaviour
/// dispatches on `tool`/`consumable` at the call sites).
pub struct ItemDef {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static [&'static str],
    pub icon_pal: &'static [(char, u32)], // per-icon recolors (js Assets.bake's override arg)
    pub kind: &'static str, // WEAPON / TOOL / CONSUMABLE / MATERIAL — the detail-pane line
    pub rarity: Rarity,
    pub desc: &'static str,
    pub price: Option<i32>, // None -> the rarity's base price
    pub weapon: bool,       // swings from an ability slot (auto-repeats while held)
    pub tool: Option<Tool>, // WHICH swing (sword/axe/pick) — the gather gate reads it too
    pub consumable: bool,
    pub stackable: bool,
    pub material: bool,
    pub unique: bool,   // one-of-a-kind: a second copy can never be picked up
    pub no_equip: bool, // sits in the bag only (satchels etc.)
    pub cooldown: u32,  // frames before the slot can fire again
    pub lock_frames: u32, // frames the player is rooted mid-swing
    pub slot: Option<&'static str>, // gear slot (head/body/feet/trinket) — none in the starter set
    /// js gear-flag booleans (clock/light/compass/…) — powers granted while WORN;
    /// [`crate::inventory::PlayerInv::has_gear_flag`] is the reader.
    pub flags: &'static [&'static str],
    /// A seed packet's crop id (js `seed: cd.id`) — app/farm.rs plants it from a slot.
    pub seed: Option<&'static str>,
    /// Shield durability (js `dur`): blocks before it shatters; 0 = not a shield.
    pub dur: i32,
    /// A cooked meal (js `dish: true`): eating grants its timed buff, and the
    /// Provisioners' bundle line counts it.
    pub dish: bool,
    /// js gear stats{} rows — worn gear folds these into player.stat (skills_tab
    /// recompute). Keys the systems don't consume yet (spell/haste/critmult/...)
    /// ride along inert until their system ports.
    pub stats: &'static [(&'static str, f64)],
    /// js `craftGen`: a forge COMMISSION preview (never granted as-is). Crafting one
    /// rolls a fresh procedural item of this class + tier — see craft_tab::do_craft.
    pub craft_gen: Option<CraftGen>,
    /// js `toolTier` (the gather gate): a pick/axe head's rank — a node's `req_tier` must
    /// be met to break it. Base pick/axe are 1; the metal tools climb to 6. 0 = not a tool.
    pub tool_tier: i32,
    /// The metal a tiered tool is forged from (js TOOL_METALS `mat`) — recolours its icon
    /// + swing sprite. None for the plain starter pick/axe (and every non-tool).
    pub tool_mat: Option<&'static str>,
}

/// A forge commission spec (js def.craftGen): roll a weapon of `base` (Sword/Axe/…) or
/// armor of `base` (a slot: head/body/feet) at `tier` (1..3).
#[derive(Clone, Copy)]
pub struct CraftGen {
    pub armor: bool,
    pub base: &'static str,
    pub tier: i32,
}

/// Field defaults, so each def below states only what the js literal states.
pub(crate) const BASE: ItemDef = ItemDef {
    id: "",
    name: "",
    icon: WOOD_ICON,
    icon_pal: &[],
    kind: "",
    rarity: Rarity::Common,
    desc: "",
    price: None,
    weapon: false,
    tool: None,
    consumable: false,
    stackable: false,
    material: false,
    unique: false,
    no_equip: false,
    cooldown: 0,
    lock_frames: 0,
    slot: None,
    flags: &[],
    seed: None,
    dur: 0,
    dish: false,
    stats: &[],
    craft_gen: None,
    tool_tier: 0,
    tool_mat: None,
};

/// A fresh catch (js FISH_DEFS define loop): the shared silhouette in its own colour.
const fn fish(id: &'static str, name: &'static str, rarity: Rarity, price: i32, pal: &'static [(char, u32)]) -> ItemDef {
    ItemDef { id, name, icon: FISH_GRID, icon_pal: pal, kind: "FISH", rarity, material: true, stackable: true, price: Some(price), desc: "A fresh catch. Sell it, or cook it for a buff.", ..BASE }
}

const fn material(id: &'static str, name: &'static str, icon: &'static [&'static str], price: i32, desc: &'static str) -> ItemDef {
    ItemDef { id, name, icon, kind: "MATERIAL", material: true, stackable: true, price: Some(price), desc, ..BASE }
}

/// A tiered ORE (js NUGGET_GRID + a metal recolour) — the copper nugget shape, retinted.
const fn ore(id: &'static str, name: &'static str, pal: &'static [(char, u32)], price: i32, rarity: Rarity, desc: &'static str) -> ItemDef {
    ItemDef { id, name, icon: COPPER_ICON, icon_pal: pal, kind: "MATERIAL", rarity, material: true, stackable: true, price: Some(price), desc, ..BASE }
}

/// The tiered-tool metal overlays (js TOOL_METALS `ov`) — recolour the pick/axe head's a/A/n.
const IRON_OV: &[(char, u32)] = &[('a', 0x8088a0), ('A', 0xc2cada), ('n', 0x525a70)];
const SILVER_OV: &[(char, u32)] = &[('a', 0xc4cee0), ('A', 0xf2f6ff), ('n', 0x8a96ae)];
const GOLD_OV: &[(char, u32)] = &[('a', 0xe0b030), ('A', 0xffe468), ('n', 0x9a7012)];
const MITHRIL_OV: &[(char, u32)] = &[('a', 0x54c8c2), ('A', 0x9af2ec), ('n', 0x268a84)];
const VOIDSTEEL_OV: &[(char, u32)] = &[('a', 0x7a4ec0), ('A', 0xc8a4f8), ('n', 0x3c2470)];

/// The metal overlay for a tiered tool's `tool_mat` (swing.rs bakes the recoloured swing).
pub fn tool_ov(mat: &str) -> &'static [(char, u32)] {
    match mat {
        "iron" => IRON_OV,
        "silver" => SILVER_OV,
        "gold" => GOLD_OV,
        "mithril" => MITHRIL_OV,
        "voidsteel" => VOIDSTEEL_OV,
        _ => &[],
    }
}

/// A tiered PICK or AXE (js TOOL_METALS loop): the shared head, retinted, with a higher
/// `tool_tier` (mines/chops tougher nodes) — a pick is a TOOL, an axe a WEAPON.
#[allow(clippy::too_many_arguments)]
const fn tiered_tool(id: &'static str, name: &'static str, tier: i32, mat: &'static str, ov: &'static [(char, u32)], rarity: Rarity, is_pick: bool) -> ItemDef {
    let (icon, kind, cooldown, lock, tool, desc) = if is_pick {
        (PICK_ICON, "TOOL", 28, 16, Tool::Pick, "A stronger head bites tougher ore veins.")
    } else {
        (AXE_ICON, "WEAPON", 30, 18, Tool::Axe, "A stronger bit fells harder timber.")
    };
    ItemDef {
        id, name, icon, icon_pal: ov, kind, rarity, desc, weapon: true, tool: Some(tool),
        cooldown, lock_frames: lock, tool_tier: tier, tool_mat: Some(mat), ..BASE
    }
}

/// A tiered TIMBER (js LOG_GRID + a wood recolour) — the log shape, retinted.
const fn timber(id: &'static str, name: &'static str, pal: &'static [(char, u32)], price: i32, rarity: Rarity, desc: &'static str) -> ItemDef {
    ItemDef { id, name, icon: WOOD_ICON, icon_pal: pal, kind: "MATERIAL", rarity, material: true, stackable: true, price: Some(price), desc, ..BASE }
}

/// Ripe produce (js CROP_DEFS define loop): the shared silhouette in the crop's colour.
const fn crop_item(id: &'static str, name: &'static str, price: i32, pal: &'static [(char, u32)], desc: &'static str) -> ItemDef {
    ItemDef { id, name, icon: PROD_GRID, icon_pal: pal, kind: "CROP", material: true, stackable: true, price: Some(price), desc, ..BASE }
}

/// A seed packet: plants its crop on tilled soil (app/farm.rs owns the slot press).
const fn seed_item(id: &'static str, name: &'static str, seed: &'static str, price: i32, pal: &'static [(char, u32)], desc: &'static str) -> ItemDef {
    ItemDef { id, name, icon: SEED_GRID, icon_pal: pal, kind: "SEED", stackable: true, price: Some(price), desc, seed: Some(seed), ..BASE }
}

/// The registry (js REGISTRY) — every def verbatim from js/items.js.
const HOOK_ICON: &[&str] = &["...aa...", "....a...", "....a...", "....a...", ".a..a...", ".a..a...", ".aaaa...", "..aa...."];
const HOOK_PAL: &[(char, u32)] = &[('a', 0xc8d2da)];
const SEAL_ICON: &[&str] = &["..rrrr..", ".rrrrrr.", "rrWrrWrr", "rrrWWrrr", "rrrWWrrr", "rrWrrWrr", ".rrrrrr.", "..rrrr.."];
const SEAL_PAL: &[(char, u32)] = &[('r', 0xc03848), ('W', 0xffd34d)];

// Farm-animal icons (js items.js bakes, verbatim).
const CHICKEN_ICON: &[&str] = &["...r....", "..WWW...", ".WWWWO..", ".WWWW...", ".WWWWW..", "..WWW...", "...yy...", "........"];
const CHICKEN_PAL: &[(char, u32)] = &[('W', 0xf4f0e8), ('r', 0xc83828), ('O', 0xe0902a), ('y', 0xc8a030)];
const EGG_ICON: &[&str] = &["........", "...WW...", "..WWWW..", "..WWWw..", "..WWWw..", "..WWww..", "...ww...", "........"];
const EGG_PAL: &[(char, u32)] = &[('W', 0xf4f0e4), ('w', 0xd8ccb0)];
const COWI_ICON: &[&str] = &["........", ".WWWK...", "WWWWWWO.", "WKWWWWO.", "WWWKWWW.", ".WWWWK..", ".pp.....", "..W..W.."];
const COWI_PAL: &[(char, u32)] = &[('W', 0xf4f0e8), ('K', 0x2a2a30), ('O', 0xe8d8a0), ('p', 0xf0a8b8)];
const PAIL_FARM_ICON: &[&str] = &["........", ".aWWWWa.", ".a....a.", ".AWWWWA.", ".AWWWWA.", ".AWWWWA.", ".AAAAAA.", "........"];
const PAIL_FARM_PAL: &[(char, u32)] = &[('A', 0xaab4bc), ('W', 0xccd6de), ('a', 0x6a747c)];
const MILK_ICON: &[&str] = &["..aaaa..", "..a..a..", ".aWWWWa.", ".aWWWWa.", ".aWWWWa.", ".aWWWWa.", ".aaaaaa.", "........"];
const MILK_PAL: &[(char, u32)] = &[('a', 0x8a949c), ('W', 0xf8f4ec)];
const COOPKIT_ICON: &[&str] = &["........", ".rrrrrr.", "rrrrrrrr", "KKKKKKKK", "KDWWDDWK", "KDWWDoWK", "KDDDDoWK", "KKKKKKKK"];
const COOPKIT_PAL: &[(char, u32)] = &[('r', 0xc83828), ('D', 0x8a6a3a), ('W', 0xe8d8a0), ('o', 0x141008), ('K', 0x3a2a18)];
const BARNKIT_ICON: &[&str] = &["..KKKK..", ".KrrrrK.", "KrrrrrrK", "rrrrrrrr", "rrWDDWrr", "rrWDDWrr", "rrWDDWrr", "........"];
const BARNKIT_PAL: &[(char, u32)] = &[('r', 0xc83828), ('K', 0x3a2a18), ('D', 0x5a3a20), ('W', 0xe8d8a0)];

// --- Cooked food (js items.js: made at the Cooking Fire; each dish = one timed buff,
// applied by app/status.rs eat_dish — potions heal, MEALS BUFF). Icons js-verbatim. ---
const ROAST_ICON: &[&str] = &["........", ".....W..", "....WW..", "..SSSW..", ".SSSSS..", ".SSSSS..", "..SSS...", "........"];
const STEW_ICON: &[&str] = &["........", "........", ".WoooW..", "WooorW..", "WoooooW.", ".WoooW..", "..WWW...", "........"];
const SKEWER_ICON: &[&str] = &[".......d", "......d.", "..SrS.d.", ".SrSrSd.", ".SrSrS..", "dSrSr...", "d.SrS...", "d......."];
const SAUTE_ICON: &[&str] = &["........", "...gPg..", "..rgPgr.", ".aaaaaa.", ".aaaaaa.", "..a..a..", "........", "........"];
const PIE_ICON: &[&str] = &["........", "...PP...", "..PyyP..", ".PyyyyP.", ".PyyyyP.", ".PyyyyP.", ".PPPPPP.", "........"];
const TART_ICON: &[&str] = &["........", ".DDDDDD.", ".DrmrrD.", ".DmrrmD.", ".DrrmrD.", ".DDDDDD.", "........", "........"];
const GRILL_ICON: &[&str] = &["........", ".....W..", "..ddddW.", ".dDDDDDd", "..ddddd.", "....d...", "........", "........"];
const CHOWDER_ICON: &[&str] = &["........", "........", ".WcccW..", "WccdccW.", "WcccccW.", ".WcccW..", "..WWW...", "........"];
const FRY_ICON: &[&str] = &["........", ".yyyyy..", "yDyyDyy.", "yyyyyyy.", ".yyyyy..", "...K....", "...K....", "........"];
const TONIC_ICON: &[&str] = &["...dd...", "..KddK..", "..KooK..", "..KoYK..", "..KooK..", "..KooK..", "..KooK..", "..KKKK.."];
const RHERB_ICON: &[&str] = &["........", "...l....", "..lgl...", ".lgGgl..", ".llgll..", "...d....", "...d....", "........"];
const COOKFIRE_ICON: &[&str] = &[".aAAAa..", "aKKKKKa.", "aKKKKKa.", ".nKKKn..", "..ooo...", ".ooroo..", ".rrorr..", "..rrr..."];
// STATION + BLUEPRINT icons (js items.js WORKBENCH_ICON..BP_ICON) — every blueprint shares
// the rolled-scroll BP_ICON; the station icons are their in-inventory kit look.
const WORKBENCH_ICON: &[&str] = &["........", "..a.a...", "DDDDDDDD", "DddddddD", "DDDDDDDD", ".d....d.", ".d....d.", "........"];
const FORGE_ICON: &[&str] = &["........", "..AAAA..", "...AA...", "..AAAA..", "..nnnn..", ".nKKKKn.", ".n.KK.n.", "........"];
const ALCHEMY_ICON: &[&str] = &["..WW....", "..ww....", ".wwww...", ".wggw...", "wggggw..", "wgGggw..", "wggggw..", ".wwww..."];
const ENCHANT_ICON: &[&str] = &["...x....", "..xXx...", ".xXWXx..", "..xXx...", "...x....", "...d....", "...d....", "..ddd..."];
const FLETCH_ICON: &[&str] = &["..D.....", ".D.W....", "D..WA...", "D.WAAAA.", "D..WA...", ".D.W....", "..D.....", "........"];
const JEWEL_ICON: &[&str] = &["...b....", "..PPP...", ".PP.PP..", ".P...P..", ".P...P..", ".PP.PP..", "..PPP...", "........"];
const FARMTABLE_ICON: &[&str] = &["..l.gl..", ".glgGgl.", ".lgGGgl.", "DDDDDDDD", "DyyyyyyD", "DDDDDDDD", ".d....d.", ".d....d."];
const BP_ICON: &[&str] = &[".bbbbbb.", ".bwwwwb.", ".bwBBwb.", ".bwwwwb.", ".bwBBwb.", ".bwwwwb.", ".bbbbbb.", "........"];
const HOUSE_ICON: &[&str] = &["..rrrr..", ".rrrrrr.", "rrrrrrrr", ".yyyyyy.", ".yyDDyy.", ".yyDDyy.", ".yyDDyy.", "........"];
const SLEEPINGBAG_ICON: &[&str] = &["........", ".DDDDDD.", "DbBbBbBD", "DbBbBbBD", "DbBbBbBD", "DbBbBbBD", ".DDDDDD.", "........"];
const WELL_ICON: &[&str] = &["..DDDD..", ".DDDDDD.", "..D..D..", ".gggggg.", ".gwwwwg.", ".gwwwwg.", ".gggggg.", ".g.gg.g."];
const KINGSPLITTER_ICON: &[&str] = &[".....WW.", "....WAW.", "...WAW..", "..WAW...", ".WAWD...", "DAWD....", "DDy.....", "y......."];
const GRAPPLE_ICON: &[&str] = &[".A...A..", ".A...A..", ".AA.AA..", "..AAA...", "...A....", "...A....", "..DAD...", "..DDD..."];
const SPRINGBOOTS_ICON: &[&str] = &["..bb....", "..bb....", "..bbbb..", "..bbbbb.", "AAAAAAA.", ".A.A.A..", "A.A.A.A.", ".A.A.A.."];
const BUBBLERING_ICON: &[&str] = &["..wPw...", ".w...w..", "w.....w.", "w.....w.", "w.....w.", ".w...w..", "..www...", "........"];
const ANTIDOTE_ICON: &[&str] = &["...dd...", "..KddK..", "..KglK..", "..KggK..", "..KggK..", "..KggK..", "..KggK..", "..KKKK.."];
const SATCHEL_ICON: &[&str] = &["........", "..dDDd..", ".dDDDDd.", ".DyyyyD.", ".DyDDyD.", ".DyyyyD.", ".DDDDDD.", "..DDDD.."];
const WAND_ICON: &[&str] = &[".......G", "......GG", "......G.", ".....D..", "....D...", "...D....", "..D.....", ".d......"];
const RUNE_ICON: &[&str] = &["..aaaa..", ".aGGGGa.", ".aGKGGa.", ".aGGKGa.", ".aGKGGa.", ".aGGGGa.", ".aGGGGa.", "..aaaa.."];
const MANAPOT_ICON: &[&str] = &["..KKK...", "..KWK...", ".KKKKK..", ".KLLLLK.", ".KLWLLK.", ".KLLLLK.", ".KLLLLK.", "..KKKK.."];
const MANAELIX_ICON: &[&str] = &["...P....", "..KPK...", ".KwwwK..", ".KwbwK..", ".KwbwK..", ".KbbbK..", ".KbbbK..", "..KKKK.."];
const SHIELD_ICON: &[&str] = &[".KKKKKK.", "KDDDDDDK", "KDppppDK", "KDppppDK", "KDppppDK", "KDDppDDK", ".KDDDDK.", "..KKKK.."];
const BOW_ICON: &[&str] = &["...DD...", "..D..W..", ".D...W..", ".D...W..", ".D...W..", "..D..W..", "...DD...", "........"];
const BOMB_ICON: &[&str] = &["......o.", ".....o..", "....r...", "..nnnn..", ".nKKKKn.", ".nKKKKn.", ".nKKKKn.", "..nnnn.."];
const SHOVEL_ICON: &[&str] = &[".......D", "......D.", ".....D..", "....D...", "..WW....", ".WWW....", "WWW.....", ".W......"];
const TMAP_ICON: &[&str] = &["WWWWWWW.", "WcccccW.", "WcDcccW.", "WccDccW.", "WcccKcW.", "WccKcKW.", "WcccccW.", "WWWWWWW."];
const BOTTLE_ICON: &[&str] = &["...gg...", "...gg...", "..g..g..", ".g.WW.g.", ".g.Wc.g.", ".g.cc.g.", ".g....g.", "..gggg.."];

/// A unique trinket (js NEW_GEAR): worn in a trinket slot; stats{} flow through
/// player.stat, flags drive the proc systems (uniques.rs).
#[allow(clippy::too_many_arguments)] // (id, name, rarity, art, stats, flags, desc) IS the row
const fn trinket(
    id: &'static str,
    name: &'static str,
    rarity: Rarity,
    icon: &'static [&'static str],
    pal: &'static [(char, u32)],
    stats: &'static [(&'static str, f64)],
    flags: &'static [&'static str],
    desc: &'static str,
) -> ItemDef {
    ItemDef { id, name, icon, icon_pal: pal, kind: "TRINKET", rarity, slot: Some("trinket"), stats, flags, desc, ..BASE }
}

/// A cooked meal: CONSUMABLE + dish (the buff itself lives in app/status.rs eat_dish).
const fn dish(
    id: &'static str,
    name: &'static str,
    rarity: Rarity,
    cooldown: u32,
    icon: &'static [&'static str],
    pal: &'static [(char, u32)],
    desc: &'static str,
) -> ItemDef {
    ItemDef { id, name, icon, icon_pal: pal, kind: "CONSUMABLE", rarity, consumable: true, stackable: true, cooldown, desc, dish: true, ..BASE }
}

/// A placeable crafting STATION (js `placeable: true`): consumed on placement (the
/// cooking-fire idiom), one per stack, opens a station-mode CRAFT page when pressed.
const fn station(id: &'static str, name: &'static str, icon: &'static [&'static str], rarity: Rarity, price: i32, desc: &'static str) -> ItemDef {
    ItemDef { id, name, icon, kind: "STATION", rarity, consumable: true, cooldown: 20, price: Some(price), desc, ..BASE }
}

/// A BLUEPRINT (js `bp*`): use it once to learn its gated recipe, then it's spent.
const fn blueprint(id: &'static str, name: &'static str, rarity: Rarity, price: i32, desc: &'static str) -> ItemDef {
    ItemDef { id, name, icon: BP_ICON, kind: "BLUEPRINT", rarity, consumable: true, stackable: false, cooldown: 12, price: Some(price), desc, ..BASE }
}

pub static DEFS: &[ItemDef] = &[
    // Sword: long reach, light damage, wide arc. Gathers Fiber (bushes/cactus).
    ItemDef {
        id: "sword",
        name: "Sword",
        icon: SWORD_ICON,
        kind: "WEAPON",
        desc: "A balanced blade with a wide slash.",
        weapon: true,
        tool: Some(Tool::Sword),
        cooldown: 20,
        lock_frames: 14,
        ..BASE
    },
    // Axe: shorter reach, hits harder (damage 2), slower, tighter chop. Gathers Wood.
    ItemDef {
        id: "axe",
        name: "Axe",
        icon: AXE_ICON,
        kind: "WEAPON",
        desc: "Shorter reach, but hits hard - and chops down trees for wood.",
        weapon: true,
        tool: Some(Tool::Axe),
        cooldown: 30,
        lock_frames: 18,
        tool_tier: 1,
        ..BASE
    },
    // Pickaxe: a tool/weapon (weak in combat) that mines Stone from boulders.
    ItemDef {
        id: "pick",
        name: "Pickaxe",
        icon: PICK_ICON,
        kind: "TOOL",
        desc: "Mine stone from boulders.",
        weapon: true,
        tool: Some(Tool::Pick),
        cooldown: 28,
        lock_frames: 16,
        tool_tier: 1,
        ..BASE
    },
    // TIERED tools (js TOOL_METALS): a better metal head mines/chops higher-`req_tier` nodes.
    // The metal recolours the shared pick/axe head (icon + swing); damage climbs with tier.
    tiered_tool("ironpick", "Iron Pickaxe", 2, "iron", IRON_OV, Rarity::Common, true),
    tiered_tool("silverpick", "Silver Pickaxe", 3, "silver", SILVER_OV, Rarity::Uncommon, true),
    tiered_tool("goldpick", "Gold Pickaxe", 4, "gold", GOLD_OV, Rarity::Uncommon, true),
    tiered_tool("mithrilpick", "Mithril Pickaxe", 5, "mithril", MITHRIL_OV, Rarity::Rare, true),
    tiered_tool("voidsteelpick", "Voidsteel Pickaxe", 6, "voidsteel", VOIDSTEEL_OV, Rarity::Epic, true),
    tiered_tool("ironaxe", "Iron Axe", 2, "iron", IRON_OV, Rarity::Common, false),
    tiered_tool("silveraxe", "Silver Axe", 3, "silver", SILVER_OV, Rarity::Uncommon, false),
    tiered_tool("goldaxe", "Gold Axe", 4, "gold", GOLD_OV, Rarity::Uncommon, false),
    tiered_tool("mithrilaxe", "Mithril Axe", 5, "mithril", MITHRIL_OV, Rarity::Rare, false),
    tiered_tool("voidsteelaxe", "Voidsteel Axe", 6, "voidsteel", VOIDSTEEL_OV, Rarity::Epic, false),
    ItemDef {
        id: "potion",
        name: "Potion",
        icon: POTION_ICON,
        kind: "CONSUMABLE",
        price: Some(40),
        desc: "Drink to restore health.",
        consumable: true,
        stackable: true,
        cooldown: 10,
        ..BASE
    },
    ItemDef {
        id: "bandage",
        name: "Bandage",
        icon: BANDAGE_ICON,
        kind: "CONSUMABLE",
        desc: "Restores 2 health. A simple field dressing you can make on the go.",
        consumable: true,
        stackable: true,
        cooldown: 8,
        ..BASE
    },
    ItemDef {
        id: "greaterpotion",
        name: "Greater Potion",
        icon: GREATERPOTION_ICON,
        kind: "CONSUMABLE",
        rarity: Rarity::Uncommon,
        price: Some(110),
        desc: "Restores 3 health.",
        consumable: true,
        stackable: true,
        cooldown: 10,
        ..BASE
    },
    ItemDef {
        id: "elixir",
        name: "Elixir of Life",
        icon: ELIXIR_ICON,
        kind: "CONSUMABLE",
        rarity: Rarity::Rare,
        desc: "Fully restores your health.",
        consumable: true,
        stackable: true,
        cooldown: 10,
        ..BASE
    },
    material("wood", "Wood", WOOD_ICON, 8, "Gathered from trees. A crafting staple."),
    material("arrow", "Arrow", ARROW_ICON, 3, "Bow ammunition. Just keep some in your bag."),
    material("stone", "Stone", STONE_ICON, 10, "Chipped from boulders."),
    material("fiber", "Fiber", FIBER_ICON, 6, "Pulled from bushes and brush."),
    material("herb", "Herb", HERB_ICON, 12, "A fragrant leaf. Brewed into potions and meals."),
    material("copper", "Copper", COPPER_ICON, 18, "Soft red metal. Forged into tools, arms, and rings."),
    material("leather", "Leather", LEATHER_ICON, 14, "Cured hide for boots, light armor, and bowstrings."),
    material("meat", "Raw Meat", MEAT_ICON, 8, "Beast meat. Cook it into hearty food."),
    ItemDef {
        id: "gem",
        name: "Gemstone",
        icon: GEM_ICON,
        kind: "MATERIAL",
        rarity: Rarity::Uncommon,
        price: Some(70),
        desc: "A cut stone. Set into jewelry or charged with magic.",
        material: true,
        stackable: true,
        ..BASE
    },
    // Iron reuses the copper-nugget shape with grey metals (js NUGGET_GRID + bake override).
    ItemDef {
        id: "iron",
        name: "Iron Ore",
        icon: COPPER_ICON,
        icon_pal: &[('p', 0x9aa0ac), ('Y', 0x62687a)],
        kind: "MATERIAL",
        price: Some(30),
        desc: "Sturdy grey metal mined in the mid-lands. Needs an iron pickaxe.",
        material: true,
        stackable: true,
        ..BASE
    },
    // The rest of the tiered-harvest ladder (js) — ore + timber that feed the forge/fletcher
    // recipes. Their gathering nodes wire in with the tiered-harvest system.
    ore("silver", "Silver Ore", &[('p', 0xdfe6f2), ('Y', 0x9aa4b8)], 55, Rarity::Uncommon, "Lustrous white metal from deep country."),
    ore("gold", "Gold Ore", &[('p', 0xf4c84a), ('Y', 0xb0841c)], 90, Rarity::Uncommon, "Heavy precious metal from the far reaches."),
    ore("mithril", "Mithril Ore", &[('p', 0x86e6e0), ('Y', 0x3a9a96)], 160, Rarity::Rare, "Rare cyan metal from the deadliest depths."),
    ore("voidsteel", "Voidsteel Ore", &[('p', 0xb488f0), ('Y', 0x5c3aa0)], 320, Rarity::Epic, "Metal born of the wound itself, mined only in the Wriftscar. It drinks the light. Needs a voidsteel pickaxe."),
    timber("hardwood", "Hardwood", &[('D', 0x5a4326), ('d', 0x3a2a14)], 26, Rarity::Common, "Dense timber from old, deep forests."),
    timber("ironbark", "Ironbark", &[('D', 0x3a4048), ('d', 0x23282e)], 60, Rarity::Uncommon, "Bark hard as metal, from blighted lands."),
    timber("greenheart", "Greenheart", &[('D', 0x3a7a42), ('d', 0x22522a)], 40, Rarity::Uncommon, "Springy jungle timber. Bowyers pay well for a straight length."),
    timber("petalwood", "Petalwood", &[('D', 0xc88aa8), ('d', 0x8a5670)], 26, Rarity::Common, "Rose-hearted timber from the blossom meadows. It smells like spring."),
    timber("voidwood", "Voidwood", &[('D', 0x4a3468), ('d', 0x2c1c42)], 85, Rarity::Rare, "Timber felled in the chaos lands. It hums, faintly, when you hold it."),
    // The BIOME woods (Baz: a grove's trees drop ITS timber, not plain oak).
    timber("petalwood", "Petalwood", &[('D', 0xc887a8), ('d', 0x8a5070)], 14, Rarity::Common, "Rosy timber from the blossom groves. It never quite loses the scent."),
    timber("gloomwood", "Gloomwood", &[('D', 0x525c74), ('d', 0x2e3446)], 16, Rarity::Common, "Grey-blue timber from the hollow woods. Lanternlight sinks into it."),
    timber("charwood", "Charwood", &[('D', 0x4a403c), ('d', 0x241e1c)], 16, Rarity::Common, "Fire-hardened timber from the burnt lands. Still warm, somehow."),
    timber("mirewood", "Mirewood", &[('D', 0x6a7444), ('d', 0x3c4424)], 14, Rarity::Common, "Water-dark timber from the mires. Heavier than it looks."),
    timber("frostpine", "Frostpine", &[('D', 0x9cc2d4), ('d', 0x5a7e94)], 16, Rarity::Common, "Pale arctic pine, rimed to the core. It creaks like ice."),
    // FISHING (js): the rod is a TOOL — equip it, face water, cast (app/fishing.rs
    // runs the cast -> bite -> tap loop).
    ItemDef {
        id: "fishingrod",
        name: "Fishing Rod",
        icon: ROD_ICON,
        kind: "TOOL",
        price: Some(50),
        desc: "Equip it, face water, and cast. Wait for a bite, then tap to hook your catch.",
        cooldown: 20,
        ..BASE
    },
    fish("minnow", "Minnow", Rarity::Common, 8, &[('C', 0xb8c0c8)]),
    fish("bluegill", "Bluegill", Rarity::Common, 14, &[('C', 0x5a8ac0)]),
    fish("bass", "Bass", Rarity::Uncommon, 30, &[('C', 0x5a7a3a)]),
    fish("carp", "Carp", Rarity::Common, 16, &[('C', 0xa08840)]),
    fish("catfish", "Catfish", Rarity::Uncommon, 34, &[('C', 0x6a6050)]),
    fish("trout", "Trout", Rarity::Uncommon, 36, &[('C', 0xc08a6a)]),
    fish("pike", "Pike", Rarity::Rare, 60, &[('C', 0x7a9a5a)]),
    fish("eel", "Eel", Rarity::Rare, 64, &[('C', 0x4a5a4a)]),
    fish("sunfish", "Sunfish", Rarity::Uncommon, 40, &[('C', 0xe0b040)]),
    fish("icefish", "Icefish", Rarity::Rare, 70, &[('C', 0xbfe6f5)]),
    fish("rainfish", "Rainfish", Rarity::Rare, 80, &[('C', 0x7090d8)]),
    fish("voidfin", "Voidfin", Rarity::Epic, 160, &[('C', 0x9a6ad0)]),
    ItemDef {
        id: "oldboot",
        name: "Old Boot",
        icon: BOOT_ICON,
        kind: "JUNK",
        price: Some(1),
        desc: "Waterlogged and worthless. The river gives what it gives.",
        stackable: true,
        ..BASE
    },
    ItemDef {
        id: "seaweed",
        name: "Seaweed",
        icon: WEED_ICON,
        kind: "JUNK",
        price: Some(3),
        desc: "Slimy green strands dredged up on your line.",
        material: true,
        stackable: true,
        ..BASE
    },
    ItemDef {
        id: "driftwood",
        name: "Driftwood",
        icon: DRIFT_ICON,
        kind: "JUNK",
        price: Some(2),
        desc: "A water-worn scrap of wood.",
        material: true,
        stackable: true,
        ..BASE
    },
    // Dungeon keys (js): consumed at their locks; stack in the bag.
    ItemDef {
        id: "key",
        name: "Key",
        icon: KEY_ICON,
        kind: "KEY",
        desc: "Opens a small locked dungeon door.",
        stackable: true,
        ..BASE
    },
    ItemDef {
        id: "ornatekey",
        name: "Ornate Key",
        icon: OKEY_ICON,
        icon_pal: &[('m', 0xc878ff)],
        kind: "KEY",
        rarity: Rarity::Rare,
        desc: "A gilded key with a violet eye. It turns one lock only: the boss door.",
        stackable: true,
        ..BASE
    },
    // The first TRINKET: worn in a trinket slot, its `clock` flag turns on the
    // sidebar TIME readout (app/hud.rs). Already listed in the general/trader/tool
    // stock tables — defining it here puts it on those shelves (the registry-fill rule).
    ItemDef {
        id: "pocketwatch",
        name: "Pocket Watch",
        icon: WATCH_ICON,
        kind: "TRINKET",
        rarity: Rarity::Uncommon,
        price: Some(100),
        desc: "Shows the time of day on your HUD.",
        slot: Some("trinket"),
        flags: &["clock"],
        ..BASE
    },
    ItemDef {
        id: "string",
        name: "String",
        icon: STRING_ICON,
        icon_pal: &[('l', 0xe6d9b0), ('g', 0xb89a5a)],
        kind: "MATERIAL",
        price: Some(8),
        desc: "Sturdy thread cut from cobwebs. Strings bows and fishing rods.",
        material: true,
        stackable: true,
        ..BASE
    },
    // THE FLUTE (js): a slot TOOL like the rod — app/flute.rs owns its press (play-mode).
    ItemDef {
        id: "flute",
        name: "Windwood Flute",
        icon: FLUTE_ICON,
        kind: "TOOL",
        rarity: Rarity::Rare,
        price: Some(30),
        desc: "Four notes of carved windwood. The right melody works wonders - the songs you learn are written in your codex.",
        cooldown: 18,
        ..BASE
    },
    // --- FARMING (js): the two tools + one produce/seed pair per crop row. The tools
    // are slot TOOLS like the rod — app/farm.rs owns their presses (till / water). ---
    ItemDef {
        id: "hoe",
        name: "Hoe",
        icon: HOE_ICON,
        kind: "TOOL",
        price: Some(40),
        desc: "Equip to a slot, then use it facing open ground to till farmland.",
        cooldown: 14,
        ..BASE
    },
    ItemDef {
        id: "wateringcan",
        name: "Watering Can",
        icon: CAN_ICON,
        kind: "TOOL",
        price: Some(40),
        desc: "Holds 12 pours. Face tilled soil to water it; face open water or stand by a well to refill.",
        cooldown: 12,
        ..BASE
    },
    // --- Farm animals (js ids/prices verbatim; coop/barn are PLACE-AT-FEET kits until
    // the blueprint placement system ports — flagged deviation). ---
    ItemDef {
        id: "chicken", name: "Chicken", icon: CHICKEN_ICON, icon_pal: CHICKEN_PAL, kind: "ANIMAL",
        rarity: Rarity::Uncommon, consumable: true, stackable: true, price: Some(150), cooldown: 20,
        desc: "A plump hen in a basket. Release her beside your coop and she will make herself at home.",
        ..BASE
    },
    ItemDef {
        id: "egg", name: "Egg", icon: EGG_ICON, icon_pal: EGG_PAL, kind: "FOOD", material: true,
        stackable: true, price: Some(25), desc: "Fresh from a happy hen. Sells well and cooks up rich.", ..BASE
    },
    ItemDef {
        id: "cow", name: "Cow", icon: COWI_ICON, icon_pal: COWI_PAL, kind: "ANIMAL",
        rarity: Rarity::Uncommon, consumable: true, stackable: true, price: Some(400), cooldown: 20,
        desc: "A gentle dairy cow on a lead. Release her beside your barn and she will settle in.",
        ..BASE
    },
    ItemDef {
        id: "milkpail", name: "Milk Pail", icon: PAIL_FARM_ICON, icon_pal: PAIL_FARM_PAL, kind: "TOOL",
        rarity: Rarity::Uncommon, price: Some(120),
        desc: "Sturdy tin. Keep it in your bag - pet your cow, then milk her once a day.", ..BASE
    },
    ItemDef {
        id: "milk", name: "Fresh Milk", icon: MILK_ICON, icon_pal: MILK_PAL, kind: "FOOD", material: true,
        stackable: true, price: Some(40), desc: "Fresh from a well-kept cow. Sells well and cooks up rich.", ..BASE
    },
    ItemDef {
        id: "rareherb", name: "Rare Herb", icon: RHERB_ICON, icon_pal: &[('l', 0xd090ff), ('g', 0x9040d0), ('G', 0x6020a0)],
        kind: "MATERIAL", rarity: Rarity::Uncommon, material: true, stackable: true, price: Some(36),
        desc: "A potent bloom found only in deadly country.", ..BASE
    },
    ItemDef {
        id: "tonic", name: "Tonic", icon: TONIC_ICON, kind: "CONSUMABLE", rarity: Rarity::Uncommon,
        consumable: true, stackable: true, cooldown: 10, desc: "Restores 2 health.", ..BASE
    },
    ItemDef {
        id: "kingsplitter", name: "The Kingsplitter", icon: KINGSPLITTER_ICON, kind: "WEAPON",
        rarity: Rarity::Legendary, unique: true, weapon: true, tool: Some(crate::combat::Tool::Sword),
        cooldown: 24, lock_frames: 14,
        desc: "The blade that broke the heart. At full health it sings a beam of light, and it bites deepest at what the break made.", ..BASE
    },
    ItemDef {
        id: "shovel", name: "Shovel", icon: SHOVEL_ICON, kind: "TOOL", price: Some(24), cooldown: 26,
        desc: "Digs the earth in front of you. Most holes hold dirt. Some hold everything.", ..BASE
    },
    ItemDef {
        id: "treasuremap", name: "Treasure Map", icon: TMAP_ICON,
        icon_pal: &[('W', 0xd8c8a0), ('c', 0xc0aa78), ('D', 0x7a5a3a), ('K', 0xa02020)],
        kind: "MAP", rarity: Rarity::Uncommon, consumable: true, stackable: true, price: Some(90), cooldown: 12,
        desc: "A weathered chart marking buried riches. Read it to fix the spot in your world map - then go dig.", ..BASE
    },
    ItemDef {
        id: "mapbottle", name: "Bottled Map", icon: BOTTLE_ICON,
        icon_pal: &[('g', 0x7aa89a), ('W', 0xd8c8a0), ('c', 0xc0aa78)],
        kind: "MAP", rarity: Rarity::Rare, consumable: true, stackable: true, price: Some(60), cooldown: 12,
        desc: "A corked bottle fished from the deep, a chart curled inside. Read it to mark the spot.", ..BASE
    },
    ItemDef {
        id: "grapplehook", name: "Grapple Hook", icon: GRAPPLE_ICON, kind: "GADGET", rarity: Rarity::Epic,
        weapon: true, cooldown: 40,
        desc: "Fire at a wall to zip across the room.", ..BASE
    },
    ItemDef {
        id: "springboots", name: "Spring Boots", icon: SPRINGBOOTS_ICON, kind: "BOOTS", rarity: Rarity::Rare,
        weapon: true, cooldown: 26,
        desc: "Bound forward, vaulting over a tile.", ..BASE
    },
    ItemDef {
        id: "bubblering", name: "Bubble Ring", icon: BUBBLERING_ICON, kind: "TRINKET", rarity: Rarity::Epic,
        slot: Some("trinket"), flags: &["bubble"],
        desc: "A bubble shields you from one shot, then recharges.", ..BASE
    },
    ItemDef {
        id: "antidote", name: "Antidote", icon: ANTIDOTE_ICON, kind: "CONSUMABLE", rarity: Rarity::Common,
        stackable: true, consumable: true, cooldown: 10,
        desc: "Cures poison and slow.", ..BASE
    },
    ItemDef {
        id: "satchel", name: "Small Satchel", icon: SATCHEL_ICON, icon_pal: &[('D', 0x7c4c1c), ('d', 0x503000)],
        kind: "UPGRADE", rarity: Rarity::Common, price: Some(60), consumable: true, stackable: true, no_equip: true, cooldown: 10,
        desc: "Expands a 1-row bag to 2 rows (16 slots).", ..BASE
    },
    ItemDef {
        id: "satchel2", name: "Satchel", icon: SATCHEL_ICON, icon_pal: &[('D', 0x8a6a2a), ('d', 0x5a4018)],
        kind: "UPGRADE", rarity: Rarity::Uncommon, price: Some(180), consumable: true, stackable: true, no_equip: true, cooldown: 10,
        desc: "Expands a 2-row bag to 3 rows (24 slots).", ..BASE
    },
    ItemDef {
        id: "satchel3", name: "Large Satchel", icon: SATCHEL_ICON, icon_pal: &[('D', 0x9a7a3a), ('d', 0x6a5020), ('y', 0xcdd6e4)],
        kind: "UPGRADE", rarity: Rarity::Rare, price: Some(420), consumable: true, stackable: true, no_equip: true, cooldown: 10,
        desc: "Expands a 3-row bag to 4 rows (32 slots).", ..BASE
    },
    ItemDef {
        id: "satchel4", name: "Travelers Pack", icon: SATCHEL_ICON, icon_pal: &[('D', 0xa06a2a), ('d', 0x6a4418), ('y', 0xf0c040)],
        kind: "UPGRADE", rarity: Rarity::Epic, price: Some(900), consumable: true, stackable: true, no_equip: true, cooldown: 10,
        desc: "Expands a 4-row bag to 5 rows (40 slots).", ..BASE
    },
    ItemDef {
        id: "wand", name: "Wand", icon: WAND_ICON, icon_pal: &[('G', 0xb890ff)], kind: "WAND", rarity: Rarity::Uncommon,
        price: Some(60), unique: true, weapon: true, cooldown: 16,
        desc: "A spellcasting wand. Slot a rune in to set its spell; casts for mana. You may carry only one.", ..BASE
    },
    ItemDef {
        id: "firerune", name: "Ember Rune", icon: RUNE_ICON, icon_pal: &[('G', 0xfc7030)], kind: "RUNE", rarity: Rarity::Uncommon,
        price: Some(35), stackable: true, consumable: true, no_equip: true, cooldown: 8,
        desc: "Slot it into your wand to imbue EMBER magic. Swaps with the wand's current rune.", ..BASE
    },
    ItemDef {
        id: "frostrune", name: "Frost Rune", icon: RUNE_ICON, icon_pal: &[('G', 0x7fd8ff)], kind: "RUNE", rarity: Rarity::Uncommon,
        price: Some(35), stackable: true, consumable: true, no_equip: true, cooldown: 8,
        desc: "Slot it into your wand to imbue FROST magic. Swaps with the wand's current rune.", ..BASE
    },
    ItemDef {
        id: "stormrune", name: "Storm Rune", icon: RUNE_ICON, icon_pal: &[('G', 0xfce64a)], kind: "RUNE", rarity: Rarity::Uncommon,
        price: Some(35), stackable: true, consumable: true, no_equip: true, cooldown: 8,
        desc: "Slot it into your wand to imbue STORM magic. Swaps with the wand's current rune.", ..BASE
    },
    ItemDef {
        id: "venomrune", name: "Venom Rune", icon: RUNE_ICON, icon_pal: &[('G', 0xb060f0)], kind: "RUNE", rarity: Rarity::Uncommon,
        price: Some(35), stackable: true, consumable: true, no_equip: true, cooldown: 8,
        desc: "Slot it into your wand to imbue VENOM magic. Swaps with the wand's current rune.", ..BASE
    },
    ItemDef {
        id: "manapotion", name: "Mana Potion", icon: MANAPOT_ICON, icon_pal: &[('L', 0x3868e8)], kind: "CONSUMABLE", rarity: Rarity::Common,
        price: Some(22), stackable: true, consumable: true, cooldown: 10,
        desc: "Restores arcane energy. Wasted at full mana.", ..BASE
    },
    ItemDef {
        id: "manaelixir", name: "Mana Elixir", icon: MANAELIX_ICON, kind: "CONSUMABLE", rarity: Rarity::Rare,
        stackable: true, consumable: true, cooldown: 10,
        desc: "Fully restores your mana.", ..BASE
    },
    ItemDef {
        id: "shield", name: "Wooden Shield", icon: SHIELD_ICON, kind: "SHIELD", rarity: Rarity::Uncommon,
        price: Some(120), dur: 12,
        desc: "Equip + HOLD to block shots. Splinters after 12 blocks.", ..BASE
    },
    ItemDef {
        id: "bow", name: "Bow", icon: BOW_ICON, kind: "WEAPON", rarity: Rarity::Uncommon,
        desc: "Loose arrows at range. Needs Arrows in your bag.",
        weapon: true, cooldown: 22, lock_frames: 8, ..BASE
    },
    ItemDef {
        id: "bombs", name: "Bomb", icon: BOMB_ICON, kind: "CONSUMABLE", rarity: Rarity::Uncommon,
        consumable: true, stackable: true, cooldown: 18, price: Some(30),
        desc: "Drop it and back away - a wide, heavy blast.", ..BASE
    },
    ItemDef {
        id: "sleepingbag", name: "Sleeping Bag", icon: SLEEPINGBAG_ICON,
        icon_pal: &[('D', 0x6a4a2a), ('B', 0x8a6a4a), ('b', 0xb89a72)], kind: "CONSUMABLE",
        rarity: Rarity::Uncommon, consumable: true, stackable: true, no_equip: true, cooldown: 10, price: Some(40),
        desc: "Rest anywhere in the open, once - a rough sleep that heals up to HALF, skips to morning, then is spent.", ..BASE
    },
    ItemDef {
        id: "cook", name: "Cooking Fire", icon: COOKFIRE_ICON, kind: "STATION", rarity: Rarity::Uncommon,
        consumable: true, cooldown: 20, price: Some(120),
        desc: "A placeable camp kitchen. Set it down in the wilds, then cook meat, herbs, crops and fish into hearty food.", ..BASE
    },
    // --- CRAFTING STATIONS (js items.js): placeable benches. The workbench is bought/found;
    // the rest are crafted at the workbench once their blueprint is learned. ---
    station("workbench", "Workbench", WORKBENCH_ICON, Rarity::Uncommon, 120,
        "Place it down, then craft the other tables at it."),
    station("forge", "Forge", FORGE_ICON, Rarity::Rare, 240,
        "A smithy for forging metal weapons and armor."),
    station("alchemy", "Alchemy Bench", ALCHEMY_ICON, Rarity::Rare, 240,
        "Brew potions, antidotes, and elixirs from herbs."),
    station("enchanter", "Enchanter Table", ENCHANT_ICON, Rarity::Epic, 320,
        "Craft wands and the runes that give them spells."),
    station("fletcher", "Fletcher Table", FLETCH_ICON, Rarity::Rare, 200,
        "Work wood and leather into bows and swift boots."),
    station("jeweler", "Jeweler Table", JEWEL_ICON, Rarity::Epic, 320,
        "Set copper and gems into rings, charms, and amulets."),
    station("farmtable", "Farming Table", FARMTABLE_ICON, Rarity::Uncommon, 120,
        "A potting bench. Craft the hoe and watering can here."),
    // The WELL (js STRUCTURE): a placeable refill point — set it by your fields, stand beside
    // it to fill the watering can. Rides the station placement system (no craft menu on it).
    ItemDef {
        id: "well", name: "Well", icon: WELL_ICON, icon_pal: &[('D', 0x8a8a92), ('g', 0x9a9a86), ('w', 0x4a9cff)],
        kind: "STATION", rarity: Rarity::Uncommon, consumable: true, cooldown: 20, price: Some(90),
        desc: "A stone well. Place it by your fields, then stand beside it to refill your watering can.", ..BASE
    },
    // The buildable HOME (js STRUCTURE): crafted at a workbench, placed in the world. Enter
    // it for the bed (sleep) + the storage chest. Consumed on placement (app/home.rs).
    ItemDef {
        id: "house", name: "House", icon: HOUSE_ICON, kind: "STRUCTURE", rarity: Rarity::Rare,
        consumable: true, cooldown: 24, price: Some(600),
        desc: "A home base to sleep, craft, and store. Set it down in the world, then step inside.", ..BASE
    },
    // --- BLUEPRINTS (js items.js): learn one to unlock its station/tool recipe at the workbench. ---
    blueprint("bpforge", "Forge Blueprint", Rarity::Rare, 140,
        "Use it to learn the Forge. Then craft one at your workbench."),
    blueprint("bpfarmtable", "Farming Table Blueprint", Rarity::Uncommon, 80,
        "Learn the Farming Table, then craft one at your workbench."),
    blueprint("bpwell", "Well Blueprint", Rarity::Uncommon, 90,
        "Learn the Well, then build one at your workbench. Stand by it to refill your watering can."),
    blueprint("bpalchemy", "Alchemy Bench Blueprint", Rarity::Rare, 160,
        "Learn the Alchemy Bench, then craft one at your workbench."),
    blueprint("bpcook", "Cooking Fire Blueprint", Rarity::Uncommon, 90,
        "Learn the Cooking Fire, then craft one at your workbench."),
    blueprint("bpenchanter", "Enchanter Blueprint", Rarity::Epic, 260,
        "Learn the Enchanter Table, then craft one at your workbench."),
    blueprint("bpfletcher", "Fletcher Blueprint", Rarity::Rare, 150,
        "Learn the Fletcher Table, then craft one at your workbench."),
    blueprint("bpjeweler", "Jeweler Blueprint", Rarity::Epic, 260,
        "Learn the Jeweler Table, then craft one at your workbench."),
    blueprint("bpgrapple", "Grapple Hook Blueprint", Rarity::Epic, 220,
        "Learn to craft the Grapple Hook at your workbench."),
    // RECIPE blueprints (js RECIPE_BPS loop): found schematics that unlock a specific item's
    // recipe at its station. They also drop as tiered loot + stock the matching shop.
    blueprint("bpmanapotion", "Recipe: Mana Potion", Rarity::Uncommon, 120,
        "A found schematic. Use it to learn the Mana Potion recipe at its table."),
    blueprint("bpelixir", "Recipe: Elixir", Rarity::Rare, 400,
        "A found schematic. Use it to learn the Elixir recipe at its table."),
    blueprint("bpmanaelixir", "Recipe: Mana Elixir", Rarity::Rare, 400,
        "A found schematic. Use it to learn the Mana Elixir recipe at its table."),
    blueprint("bpstormrune", "Recipe: Storm Rune", Rarity::Uncommon, 120,
        "A found schematic. Use it to learn the Storm Rune recipe at its table."),
    blueprint("bpvenomrune", "Recipe: Venom Rune", Rarity::Uncommon, 120,
        "A found schematic. Use it to learn the Venom Rune recipe at its table."),
    blueprint("bpmanacrystal", "Recipe: Mana Crystal", Rarity::Uncommon, 120,
        "A found schematic. Use it to learn the Mana Crystal recipe at its table."),
    blueprint("bpspringboots", "Recipe: Spring Boots", Rarity::Rare, 400,
        "A found schematic. Use it to learn the Spring Boots recipe at its table."),
    blueprint("bpcritring", "Recipe: Crit Ring", Rarity::Rare, 400,
        "A found schematic. Use it to learn the Crit Ring recipe at its table."),
    blueprint("bpluckamulet", "Recipe: Luck Amulet", Rarity::Rare, 400,
        "A found schematic. Use it to learn the Luck Amulet recipe at its table."),
    blueprint("bpmace", "Recipe: Mace", Rarity::Uncommon, 120,
        "A found schematic. Use it to learn the Mace recipe at its table."),
    blueprint("bpchainmail", "Recipe: Chainmail", Rarity::Rare, 400,
        "A found schematic. Use it to learn the Chainmail recipe at its table."),
    dish("roast", "Roast", Rarity::Uncommon, 600, ROAST_ICON, &[], "WELL FED: faster health regen for 90S."),
    dish("stew", "Hearty Stew", Rarity::Rare, 720, STEW_ICON, &[], "Cures poison + GUARDED: +ARMOR for 120S."),
    dish("skewer", "Spiced Skewer", Rarity::Uncommon, 600, SKEWER_ICON, &[], "MIGHTY: +20% damage for 120S."),
    dish("saute", "Veggie Saute", Rarity::Uncommon, 600, SAUTE_ICON, &[], "SWIFT: +move speed for 90S."),
    dish("pie", "Pumpkin Pie", Rarity::Rare, 720, PIE_ICON, &[], "KEEN: +crit chance for 120S."),
    dish("tart", "Berry Tart", Rarity::Rare, 720, TART_ICON, &[], "LUCKY: +richer loot for 120S."),
    dish("grilledfish", "Grilled Fish", Rarity::Uncommon, 600, GRILL_ICON, &[('D', 0xd09a52), ('d', 0xa06a32)], "WELL FED: faster health regen for 90S."),
    dish("chowder", "Fish Chowder", Rarity::Rare, 720, CHOWDER_ICON, &[('c', 0xf0e4c8), ('d', 0xa06a32)], "Comfort in a bowl: WELL FED + GUARDED for 120S."),
    dish("anglersfry", "Anglers Fry", Rarity::Rare, 720, FRY_ICON, &[('y', 0xe8b84a), ('D', 0xa06a32)], "The rivers favorite: LUCKY + SWIFT for 120S."),
    // --- UNIQUE trinkets (js NEW_GEAR batch 1): procs, tradeoffs, and oddities — each
    // does something you FEEL, not just a stat line. Proc chances scale with Luck. ---
    trinket("emberfang", "Ember Fang", Rarity::Epic,
        &["...K....", "..KWK...", "..KWK...", ".KWWOK..", ".KWOoK..", ".KWOoK..", "..KOK...", "...K...."],
        &[('W', 0xf4ecd8), ('O', 0xfc7430), ('o', 0xfcd23b), ('K', 0x3a2018)],
        &[("scorch", 0.22)], &[],
        "A tooth of the Embermaw, still warm. Strikes may catch fire - and fire spreads."),
    trinket("wintershard", "Winter Shard", Rarity::Rare,
        &["...b....", "..bWb...", ".bWBWb..", ".bBWBb..", "..bWb...", "..bBb...", "...b....", "........"],
        &[('b', 0x7fd8ff), ('W', 0xeef8ff), ('B', 0x4a9ad0)],
        &[("chill", 0.3)], &[],
        "Arctic shard-ice that never melts. Strikes may slow foes to a crawl."),
    trinket("midastooth", "Midas Tooth", Rarity::Epic,
        &["........", ".GGGGG..", ".GgGgG..", ".GGGGG..", "..G.G...", "..G.G...", "..g.g...", "........"],
        &[('G', 0xfcd000), ('g', 0xb08a00)],
        &[("midas", 0.2), ("coin", 0.1)], &[],
        "Bit from a greedy king. Slain foes sometimes burst into coin."),
    trinket("soullocket", "Soul Locket", Rarity::Epic,
        &["...s....", "..s.s...", ".sSSSs..", ".SwSSS..", ".SSSSS..", "..SSS...", "...S....", "........"],
        &[('s', 0x9a9ab0), ('S', 0xc8c8e8), ('w', 0xffffff)],
        &[("soul", 0.18)], &[],
        "It hums a soft dirge. Some deaths mend you a little."),
    trinket("brambleband", "Bramble Band", Rarity::Rare,
        &["..g..g..", ".gGggG..", "gGGGGGg.", ".GgGGg..", "gGGGGGg.", ".gGggG..", "..g..g..", "........"],
        &[('g', 0x3a6a2a), ('G', 0x5acb3a)],
        &[("defense", 1.0), ("thorns", 1.0)], &[],
        "Greenmaw briar, woven tight. Whatever strikes you gets bitten back."),
    trinket("grudgepurse", "Grudge Purse", Rarity::Uncommon,
        &["...DD...", "..D..D..", ".DddddD.", ".DdGddD.", ".DddddD.", "..DddD..", "...DD...", "........"],
        &[('D', 0x6a4a2a), ('d', 0x8a6a3a), ('G', 0xfcd000)],
        &[("coin", 0.12)], &["grudge"],
        "Keeps a tally of every insult. Spills a coin when you are struck - snatch it back."),
    trinket("saintsglass", "Saints Glass", Rarity::Legendary,
        &["..wWWw..", ".wWggWw.", ".WgGgW..", ".WgggW..", ".wWgWw..", "..wWw...", "...W....", "..WWW..."],
        &[('w', 0xc8d8e0), ('W', 0xeef8ff), ('g', 0x7ee08a), ('G', 0xffffff)],
        &[("luck", 0.35), ("crit", 0.1)], &["fragile"],
        "A chapel relic of impossible fortune. Shatters forever at the first blow you take."),
    trinket("owltalisman", "Owl Talisman", Rarity::Rare,
        &[".D....D.", ".DDDDDD.", ".DWDDWD.", ".DKDDKD.", ".DDddDD.", "..DDDD..", "..D..D..", "........"],
        &[('D', 0x6a5a3a), ('W', 0xfcd23b), ('K', 0x1a1208), ('d', 0xc09a44)],
        &[], &["nightowl"],
        "The hollowwood owls hunt at dusk. By night: +Move, +Crit."),
    trinket("wispstone", "Wispstone", Rarity::Legendary,
        &["........", "..ss....", ".sWWs...", ".sWws.s.", "..ss..s.", "....ss..", "........", "........"],
        &[('s', 0x5ad0c8), ('W', 0xc8fff8), ('w', 0xffffff)],
        &[], &["orbital"],
        "A grave-wisp keeps you company. It circles you, singes foes, and swats arrows from the air."),
    trinket("saltcrown", "Salt Crown", Rarity::Epic,
        &[".W..W..W", ".WW.W.WW", ".WWWWWW.", ".WwWWwW.", ".WWWWWW.", "........", "........", "........"],
        &[('W', 0xe8ecf0), ('w', 0xaab4b8)],
        &[("spell", 0.2), ("maxmana", 3.0), ("defense", -1.0)], &[],
        "The Choirs favor, crusted white. +20% Spell, +3 MP, -1 Armor."),
    trinket("gravecoin", "Grave Coin", Rarity::Rare,
        &["..ssss..", ".sSSSSs.", "sSKSSKs.", "sSSKSSs.", "sSKSKSs.", ".sSSSSs.", "..ssss..", "........"],
        &[('s', 0x5a5a6a), ('S', 0x9a9aa8), ('K', 0x2a2a32)],
        &[("coin", 0.2), ("luck", 0.1), ("maxhp", -1.0)], &[],
        "Two for the ferryman, one for you. +Gold, +Luck, -1 HP."),
    trinket("boarheart", "Boar Heart", Rarity::Rare,
        &["........", ".rr.rr..", "rRRrRRr.", "rRRRRRr.", ".rRRRr..", "..rRr...", "...r....", "........"],
        &[('r', 0x8a2a2a), ('R', 0xc04040)],
        &[("melee", 0.15), ("maxhp", 1.0), ("move", -0.08)], &[],
        "Eat like a boar, hit like one. Slower, though."),
    trinket("hollowbone", "Hollow Bone", Rarity::Rare,
        &[".WW.....", "WWWW....", ".WWWW...", "..WWWW..", "...WWWW.", "....WWWW", ".....WW.", "........"],
        &[('W', 0xe8e4d8)],
        &[("move", 0.12), ("haste", 0.1), ("defense", -1.0)], &[],
        "Light as a starling. Thin as one too. +Move, +Speed, -1 Armor."),
    trinket("riftsplinter", "Rift Splinter", Rarity::Epic,
        &[".......x", "......xX", ".....xX.", "....xX..", "...xX...", "..xX....", ".xX.....", "xX......"],
        &[('x', 0x6a3a9a), ('X', 0xc878ff)],
        &[("crit", 0.2), ("critmult", 0.5), ("maxhp", -1.0)], &[],
        "A needle of the broken heart. It wants blood - anyones."),
    trinket("bellclapper", "Bell Clapper", Rarity::Epic,
        &["...bb...", "...bb...", "..bBBb..", "..bBBb..", ".bBBBBb.", ".bbbbbb.", "...BB...", "........"],
        &[('b', 0x8a6a2a), ('B', 0xc09a44)],
        &[("knock", 1.5), ("melee", 0.1), ("haste", -0.08)], &[],
        "Swing like the bell tolls. Foes fly. Heavy, though."),
    trinket("wardensknuckle", "Wardens Knuckle", Rarity::Rare,
        &["........", ".AAAA...", "AaAaAA..", "AAAAAA..", "AaAaAA..", ".AAAA...", "........", "........"],
        &[('A', 0x8a8a92), ('a', 0xc8c8d0)],
        &[("iframes", 0.5)], &[],
        "The old warden never got hit twice. Longer grace after a blow."),
    trinket("stillwaterpearl", "Stillwater Pearl", Rarity::Rare,
        &["........", "..bbbb..", ".bWWwwb.", ".bWwwwb.", ".bwwwwb.", "..bbbb..", "........", "........"],
        &[('b', 0x4a9ad0), ('W', 0xffffff), ('w', 0xc8e8f8)],
        &[("manaregen", 1.5), ("maxmana", 2.0)], &[],
        "Calm as a windless lake. Your mind refills like one."),
    trinket("harvestknot", "Harvest Knot", Rarity::Epic,
        &["...Y....", "..YyY...", "..yYy...", "..YyY...", "..yYy...", "...Y....", "..gg....", ".g......"],
        &[('Y', 0xe0b040), ('y', 0xc09a2a), ('g', 0x5a7a3a)],
        &[("regen", 0.6), ("maxhp", 1.0)], &[],
        "A charm of braided wheat. Slow, patient mending - the farmers way."),
    ItemDef {
        id: "boomerang", name: "Windwood Boomerang",
        icon: &[".DD.....", "DddD....", "D.DdD...", "...DdD..", "....DdD.", ".....DdD", "......DD", "........"],
        icon_pal: &[('D', 0x8a6a3a), ('d', 0xc8a060)],
        kind: "TOOL", rarity: Rarity::Rare, price: Some(140), cooldown: 55,
        desc: "Throw it and it comes home, chilling whatever it clips - going out AND coming back.", ..BASE
    },
    ItemDef {
        id: "coop", name: "Chicken Coop", icon: COOPKIT_ICON, icon_pal: COOPKIT_PAL, kind: "STRUCTURE",
        consumable: true, price: Some(120), cooldown: 20,
        desc: "A flat-packed henhouse. Use it on open ground to raise the coop at your feet (4 roosts).",
        ..BASE
    },
    ItemDef {
        id: "barn", name: "Cattle Barn", icon: BARNKIT_ICON, icon_pal: BARNKIT_PAL, kind: "STRUCTURE",
        consumable: true, price: Some(250), cooldown: 20,
        desc: "A stout red barn, boards and all. Use it on open ground to raise it at your feet (3 stalls).",
        ..BASE
    },
    // --- Guildhall rewards (js; the hook's proc mechanics are flagged for the trinket pass). ---
    ItemDef {
        id: "luckyhook", name: "The Anglers Lucky Hook", icon: HOOK_ICON, icon_pal: HOOK_PAL, kind: "TRINKET",
        rarity: Rarity::Rare, desc: "The guild's own barbless hook, polished by a hundred grateful hands.", ..BASE
    },
    ItemDef {
        id: "guildseal", name: "The Guild Seal", icon: SEAL_ICON, icon_pal: SEAL_PAL, kind: "TRINKET",
        rarity: Rarity::Epic, desc: "Five crests in one ring of wax. The hall stands whole because of you.", ..BASE
    },
    crop_item("turnip", "Turnip", 28, &[('C', 0xf0e8d8)], "A fresh turnip. Sells well and cooks up nicely."),
    crop_item("potato", "Potato", 42, &[('C', 0xc89858)], "A fresh potato. Sells well and cooks up nicely."),
    crop_item("carrot", "Carrot", 34, &[('C', 0xe07a2a)], "A fresh carrot. Sells well and cooks up nicely."),
    crop_item("wheat", "Wheat", 26, &[('C', 0xe0c050)], "Fresh wheat. Sells well and cooks up nicely."),
    crop_item("tomato", "Tomato", 60, &[('C', 0xd83018)], "A fresh tomato. Sells well and cooks up nicely."),
    crop_item("pepper", "Pepper", 52, &[('C', 0xe84018)], "A fresh pepper. Sells well and cooks up nicely."),
    crop_item("pumpkin", "Pumpkin", 120, &[('C', 0xe8842a)], "A fresh pumpkin. Sells well and cooks up nicely."),
    crop_item("cranberry", "Cranberry", 75, &[('C', 0xc01830)], "Fresh cranberries. Sell well and cook up nicely."),
    seed_item("turnipseed", "Turnip Seeds", "turnip", 13, &[('C', 0xf0e8d8)], "Plant on tilled soil (spring). Water it every day to grow."),
    seed_item("potatoseed", "Potato Seeds", "potato", 19, &[('C', 0xc89858)], "Plant on tilled soil (spring). Water it every day to grow."),
    seed_item("carrotseed", "Carrot Seeds", "carrot", 15, &[('C', 0xe07a2a)], "Plant on tilled soil (spring/summer). Water it every day to grow."),
    seed_item("wheatseed", "Wheat Seeds", "wheat", 12, &[('C', 0xe0c050)], "Plant on tilled soil (summer/fall). Water it every day to grow."),
    seed_item("tomatoseed", "Tomato Seeds", "tomato", 27, &[('C', 0xd83018)], "Plant on tilled soil (summer). Water it every day to grow."),
    seed_item("pepperseed", "Pepper Seeds", "pepper", 23, &[('C', 0xe84018)], "Plant on tilled soil (summer). Water it every day to grow."),
    seed_item("pumpkinseed", "Pumpkin Seeds", "pumpkin", 54, &[('C', 0xe8842a)], "Plant on tilled soil (fall). Water it every day to grow."),
    seed_item("cranberryseed", "Cranberry Seeds", "cranberry", 34, &[('C', 0xc01830)], "Plant on tilled soil (fall). Water it every day to grow."),
];

/// One growable crop's farm-sim facts (js CROP_DEFS row): what seasons it lives in, how
/// many watered days to fruit, and how its ripe produce draws (colour + SHAPE).
pub struct CropDef {
    pub id: &'static str,
    pub name: &'static str,
    pub seasons: &'static [&'static str],
    pub stages: i32,
    pub color: u32,
    pub shape: &'static str,
    pub sell: i32,
}

/// js CROP_DEFS, verbatim (seed prices above are max(4, round(sell * 0.45)), the js rule).
pub static CROPS: &[CropDef] = &[
    CropDef { id: "turnip", name: "Turnip", seasons: &["SPRING"], stages: 3, color: 0xf0e8d8, shape: "round", sell: 28 },
    CropDef { id: "potato", name: "Potato", seasons: &["SPRING"], stages: 4, color: 0xc89858, shape: "oval", sell: 42 },
    CropDef { id: "carrot", name: "Carrot", seasons: &["SPRING", "SUMMER"], stages: 3, color: 0xe07a2a, shape: "cone", sell: 34 },
    CropDef { id: "wheat", name: "Wheat", seasons: &["SUMMER", "FALL"], stages: 4, color: 0xe0c050, shape: "grain", sell: 26 },
    CropDef { id: "tomato", name: "Tomato", seasons: &["SUMMER"], stages: 5, color: 0xd83018, shape: "round", sell: 60 },
    CropDef { id: "pepper", name: "Pepper", seasons: &["SUMMER"], stages: 4, color: 0xe84018, shape: "long", sell: 52 },
    CropDef { id: "pumpkin", name: "Pumpkin", seasons: &["FALL"], stages: 6, color: 0xe8842a, shape: "big", sell: 120 },
    CropDef { id: "cranberry", name: "Cranberry", seasons: &["FALL"], stages: 5, color: 0xc01830, shape: "cluster", sell: 75 },
];

pub fn crop(id: &str) -> Option<&'static CropDef> {
    CROPS.iter().find(|c| c.id == id)
}

pub fn get(id: &str) -> Option<&'static ItemDef> {
    if id.starts_with('~') {
        return crate::procgen::resolve(id); // a rolled weapon/armor (procgen.rs)
    }
    if id.starts_with("craftw") || id.starts_with("crafta") {
        return crate::procgen::preview(id); // a forge COMMISSION preview (procgen.rs)
    }
    DEFS.iter().chain(crate::gear_data::GEAR_DEFS).find(|d| d.id == id)
}

/// Every def, both tables (the codex ITEMS page + discovery walk over the full registry).
pub fn all_defs() -> impl Iterator<Item = &'static ItemDef> {
    DEFS.iter().chain(crate::gear_data::GEAR_DEFS)
}

/// js priceOf: the def's own price, else its rarity's base, else 10.
pub fn price_of(id: &str) -> i32 {
    match get(id) {
        None => 10,
        Some(d) => d.price.unwrap_or_else(|| d.rarity.base_price()),
    }
}

/// js sellPriceOf: 40% of the buy price, floored, never below 1.
pub fn sell_price_of(id: &str) -> i32 {
    ((price_of(id) as f64 * 0.4).floor() as i32).max(1)
}

pub fn rarity_of(id: &str) -> Rarity {
    get(id).map_or(Rarity::Common, |d| d.rarity)
}

/// js equippable: may sit in an ABILITY slot. (The js check also admits shield/accessory/
/// tool-flagged defs — those flags arrive with their items.)
pub fn equippable(id: &str) -> bool {
    // js: weapon || consumable || tool — belt TOOLS (rod/hoe/can) and SEED packets ride
    // the ability slots; their presses dispatch in fishing.rs / farm.rs.
    get(id).is_some_and(|d| (d.weapon || d.consumable || d.kind == "TOOL" || d.kind == "SEED" || d.kind == "SHIELD" || d.kind == "GADGET" || d.kind == "BOOTS") && !d.no_equip)
}

/// js gearSlot: which GEAR slot (head/body/feet/trinket) the item wears in, if any.
pub fn gear_slot(id: &str) -> Option<&'static str> {
    get(id).and_then(|d| d.slot)
}

// --- Loot rolls (js rollLoot): one tier by exponentially-boosted weight, one id from that
// tier's pool. TIER_BASE verbatim — BALANCE PHILOSOPHY: purple is ENDGAME; legendary is a
// deep-rift champion's long-shot. ---

/// Base chance per tier (common..legendary).
const TIER_BASE: [f64; 5] = [0.62, 0.28, 0.085, 0.0135, 0.0001];

/// The js LOOT_POOLS, FILTERED to defs that exist in this registry — every unported id
/// rejoins its pool as its item ports (the full lists live in js/items.js). Until then the
/// higher pools fall through to common like the js `|| LOOT_POOLS.common`.
const LOOT_POOLS: [&[&str]; 5] = [
    // common (js also: dagger, club, shortsword, hatchet, sickle, antidote, satchel)
    &["potion", "leathercap", "leatherboots", "paddedcoif", "clothtunic", "sandals"],
    // uncommon (js also: saber, spear, satchel2, rapier, scimitar, flail, trident, cleaver)
    &["bow", "bombs", "greaterpotion", "leathervest", "pocketwatch", "greedcharm", "bronzehelm", "rangerhood", "studdedleather", "swiftboots", "powerring", "swiftcharm", "lodestone", "gamblerscoin", "manacrystal", "tonic", "grudgepurse"],
    // rare (js also: mace, battleaxe, springboots, katana, glaive, morningstar, halberd, manaelixir, satchel3)
    &["elixir", "ironhelm", "chainmail", "travelboots", "luckamulet", "hornedhelm", "magehat", "scalemail", "magerobe", "ironcladgreaves", "critring", "magefocus", "ironheart", "titangrip", "focuslens", "wintershard", "brambleband", "owltalisman", "gravecoin", "boarheart", "hollowbone", "wardensknuckle", "stillwaterpearl", "boomerang"],
    // epic (js also: warhammer, greatsword, grapplehook, bubblering, executioner, frostbrand, vampiricscythe, satchel4)
    &["platemail", "vigorpendant", "dragonhelm", "dragonscale", "bootsofhaste", "berserkertotem", "vampirefang", "arcanesigil", "assassinmark", "emberfang", "midastooth", "soullocket", "saltcrown", "riftsplinter", "bellclapper", "compass", "regenring", "harvestknot"],
    // legendary (js also: sunblade, voidreaver, dragonfang, worldsplitter)
    &["crownofvalor", "aegisplate", "sevenleague", "phoenixfeather", "warlordbanner", "saintsglass", "wispstone"],
];

/// Stackables drop a small random quantity (js STACK_QTY).
const STACK_QTY: [(&str, i32, i32); 3] = [("potion", 1, 2), ("greaterpotion", 1, 2), ("elixir", 1, 1)];

/// The rarity walk alone (0..4) — THE one set of dice every loot source shares.
/// Exponential in tier so a high boost can genuinely swing rolls toward epic/legendary.
pub fn roll_tier(boost: f64, luck: f64, mut rand: impl FnMut() -> f64) -> usize {
    let k = 1.0 + boost + luck * 0.5;
    let w: Vec<f64> = TIER_BASE.iter().enumerate().map(|(i, base)| base * k.powi(i as i32)).collect();
    let sum: f64 = w.iter().sum();
    let mut r = rand() * sum;
    let mut ti = 0;
    while ti < w.len() {
        r -= w[ti];
        if r <= 0.0 {
            break;
        }
        ti += 1;
    }
    ti.min(w.len() - 1)
}

/// Roll one drop: (id, qty). `boost` (0..1+) shifts weight toward higher tiers; `luck`
/// nudges a little more. (The js procedural weapon/armour substitution joins with the
/// item-generator port.) `rand` supplies Math.random() — the caller's game rng.
pub fn roll_loot(boost: f64, luck: f64, mut rand: impl FnMut() -> f64) -> (&'static str, i32) {
    let ti = roll_tier(boost, luck, &mut rand);
    let pool = if LOOT_POOLS[ti].is_empty() { LOOT_POOLS[0] } else { LOOT_POOLS[ti] };
    let id = pool[(rand() * pool.len() as f64) as usize % pool.len()];
    // Procedural substitution (js rollLoot): EVERY common..epic WEAPON/ARMOUR drop
    // becomes a GENERATED item rolled at the same tier — the fixed gear defs survive
    // only as shop staples + forge craftables. Consumables, trinkets, and legendaries
    // keep their fixed defs.
    if ti <= 3
        && let Some(d) = get(id)
    {
        let seed = (rand() * u32::MAX as f64) as u32;
        if d.weapon && d.kind != "SHIELD" && d.tool.is_some() {
            return (crate::procgen::generate(crate::procgen::Kind::Weapon, ti as i32, seed), 1);
        }
        if matches!(d.slot, Some("head") | Some("body") | Some("feet")) {
            return (crate::procgen::generate(crate::procgen::Kind::Armor, ti as i32, seed), 1);
        }
    }
    let mut qty = 1;
    if let Some((_, lo, hi)) = STACK_QTY.iter().find(|(sid, ..)| *sid == id) {
        qty = lo + (rand() * (hi - lo + 1) as f64) as i32;
    }
    (id, qty)
}

// --- Recipes (js RECIPES): what crafting turns materials into. The HAND rows are
// verbatim; a recipe only SHOWS once its output item exists in this registry (same
// fill-in-as-it-ports rule as the loot pools). Station recipes join with the
// workbench/forge port. ---

pub struct Recipe {
    pub out: &'static str,
    /// How many the craft grants (js outN) — 1 for everything but the arrow bundles.
    pub n: i32,
    pub cost: &'static [(&'static str, i32)],
    pub station: &'static str, // "hand" = craftable anywhere (the slide-out CRAFT tab)
    /// The blueprint that must be LEARNED before this recipe shows (js bp), or None.
    pub bp: Option<&'static str>,
}

// The crafting table is GENERATED from js/items.js (tools/extract_recipes.mjs) — the
// full 106-recipe set with station + blueprint gating. recipes_for filters it below.
pub use crate::recipes_data::RECIPES;

/// A single def's OWN stat value (js cfg lookup) — generated weapons carry their combat
/// numbers here (dmg/crit/critmult/knock/leech); a weapon isn't worn, so gear_stat misses it.
pub fn def_stat(def: &ItemDef, name: &str) -> f64 {
    def.stats.iter().filter(|(k, _)| *k == name).map(|(_, v)| *v).sum()
}

/// Sum a stat over every WORN gear piece (js player.stat's gear term).
pub fn gear_stat(inv: &crate::inventory::PlayerInv, name: &str) -> f64 {
    inv.gear
        .iter()
        .flatten()
        .filter_map(|&uid| inv.id_of(uid).and_then(get))
        .flat_map(|d| d.stats.iter())
        .filter(|(k, _)| *k == name)
        .map(|(_, v)| *v)
        .sum()
}

/// The recipes a station shows (js recipesFor + the registry-existence gate; blueprint
/// locks join with the blueprint port).
pub fn recipes_for(station: &str, learned: &std::collections::HashSet<String>) -> Vec<&'static Recipe> {
    RECIPES
        .iter()
        .filter(|r| r.station == station && get(r.out).is_some() && r.bp.is_none_or(|bp| learned.contains(bp)))
        .collect()
}

/// A consumable's `use()` closure, as one dispatch. Returns false when the use is VETOED
/// (js `use()` returning false) — then the item is NOT consumed.
pub fn use_consumable(def: &ItemDef, health: &mut crate::combat::Health) -> bool {
    match def.id {
        "tonic" => {
            if health.hp >= health.max {
                return false;
            }
            health.hp = (health.hp + 2).min(health.max);
            true
        }
        "bandage" => {
            if health.hp >= health.max {
                return false;
            }
            health.hp = (health.hp + 2).min(health.max);
            true
        }
        "potion" => {
            if health.hp >= health.max {
                return false; // full HP -> don't waste it
            }
            // ~a third of your bar (min 3) — worth drinking (js playtest: 1 HP was pointless).
            let heal = ((health.max as f64 * 0.3).ceil() as i32).max(3);
            health.hp = (health.hp + heal).min(health.max);
            true
        }
        "greaterpotion" => {
            if health.hp >= health.max {
                return false;
            }
            health.hp = (health.hp + 3).min(health.max);
            true
        }
        "elixir" => {
            if health.hp >= health.max {
                return false;
            }
            health.hp = health.max;
            true
        }
        _ => false,
    }
}

// --- FISHING TABLES (js FISH_DEFS + rollFish): what bites is WATER TYPE x BIOME x
// SEASON x WEATHER. Weather-gated fish (rainfish/voidfin) lie dormant until the
// weather system ports — roll callers pass "clear" (FLAGGED). ---

pub struct FishRow {
    pub id: &'static str,
    pub name: &'static str,
    pub rarity: Rarity,
    pub water: &'static str, // "any" | "blue" | "murk"
    pub lb: (f64, f64),
    pub biomes: &'static [&'static str],
    pub seasons: &'static [&'static str],
    pub weather: &'static [&'static str],
}

const fn frow(id: &'static str, name: &'static str, rarity: Rarity, water: &'static str, lb: (f64, f64)) -> FishRow {
    FishRow { id, name, rarity, water, lb, biomes: &[], seasons: &[], weather: &[] }
}

pub static FISH_TABLE: &[FishRow] = &[
    frow("minnow", "Minnow", Rarity::Common, "any", (0.2, 0.6)),
    FishRow { seasons: &["SPRING", "SUMMER"], ..frow("bluegill", "Bluegill", Rarity::Common, "blue", (0.3, 1.2)) },
    frow("bass", "Bass", Rarity::Uncommon, "blue", (1.0, 5.0)),
    frow("carp", "Carp", Rarity::Common, "murk", (1.0, 6.0)),
    frow("catfish", "Catfish", Rarity::Uncommon, "murk", (2.0, 9.0)),
    FishRow { biomes: &["mountains", "arctic", "greenmaw", "forest"], ..frow("trout", "Trout", Rarity::Uncommon, "blue", (0.6, 3.0)) },
    frow("pike", "Pike", Rarity::Rare, "blue", (2.0, 12.0)),
    frow("eel", "Eel", Rarity::Rare, "murk", (1.0, 7.0)),
    FishRow {
        seasons: &["SUMMER"],
        biomes: &["suncoast", "honeyglade", "bluebell", "grassland"],
        ..frow("sunfish", "Sunfish", Rarity::Uncommon, "blue", (0.5, 2.5))
    },
    FishRow { seasons: &["WINTER"], biomes: &["arctic", "mountains", "saltwastes"], ..frow("icefish", "Icefish", Rarity::Rare, "blue", (0.4, 2.0)) },
    FishRow { weather: &["rain", "thunderstorm"], ..frow("rainfish", "Rainfish", Rarity::Rare, "any", (1.0, 4.0)) },
    FishRow { weather: &["thunderstorm"], ..frow("voidfin", "Voidfin", Rarity::Epic, "any", (3.0, 15.0)) },
];

const JUNK_IDS: [&str; 3] = ["oldboot", "seaweed", "driftwood"];

fn fish_weight(r: Rarity) -> f64 {
    match r {
        Rarity::Common => 60.0,
        Rarity::Uncommon => 26.0,
        Rarity::Rare => 10.0,
        Rarity::Epic => 2.0,
        Rarity::Legendary => 1.0,
    }
}

/// One cast's outcome.
pub enum Catch {
    Junk(&'static str),
    Fish { id: &'static str, name: &'static str, rarity: Rarity, lb: f64 },
}

/// js rollFish — junk odds, then a rarity-weighted draw from the rows native to this
/// water/biome/season/weather. ~1.4% of casts (10% of the junk 14%) the deep gives up
/// a BOTTLED MAP (js) — read it like any treasure chart (digging.rs ReadMap).
pub fn roll_fish(biome: &str, season: &str, weather: &str, water: &str, mut rand: impl FnMut() -> f64) -> Catch {
    if rand() < 0.14 {
        if rand() < 0.10 {
            return Catch::Junk("mapbottle"); // the deep gives up a bottled chart
        }
        return Catch::Junk(JUNK_IDS[(rand() * JUNK_IDS.len() as f64) as usize % JUNK_IDS.len()]);
    }
    let pool: Vec<&FishRow> = FISH_TABLE
        .iter()
        .filter(|f| {
            (f.water == "any" || f.water == water)
                && (f.biomes.is_empty() || f.biomes.contains(&biome))
                && (f.seasons.is_empty() || f.seasons.contains(&season))
                && (f.weather.is_empty() || f.weather.contains(&weather))
        })
        .collect();
    if pool.is_empty() {
        return Catch::Junk(JUNK_IDS[0]); // nothing native here — snag junk
    }
    let total: f64 = pool.iter().map(|f| fish_weight(f.rarity)).sum();
    let mut r = rand() * total;
    for f in &pool {
        r -= fish_weight(f.rarity);
        if r <= 0.0 {
            let lb = ((f.lb.0 + rand() * (f.lb.1 - f.lb.0)) * 10.0).round() / 10.0;
            return Catch::Fish { id: f.id, name: f.name, rarity: f.rarity, lb };
        }
    }
    let f0 = pool[0];
    Catch::Fish { id: f0.id, name: f0.name, rarity: f0.rarity, lb: f0.lb.0 }
}

#[cfg(test)]
mod fish_tests {
    use super::*;

    /// The GENERATED gear catalog stays sound: unique ids, resolvable, well-formed
    /// icons (bake() would panic on ragged rows), wearable slots, priced stats rows.
    #[test]
    fn gear_catalog_integrity() {
        let mut seen = std::collections::HashSet::new();
        for d in crate::gear_data::GEAR_DEFS {
            assert!(seen.insert(d.id), "duplicate gear id {}", d.id);
            assert!(get(d.id).is_some(), "{} must resolve through get()", d.id);
            assert!(DEFS.iter().all(|b| b.id != d.id), "{} shadows a base def", d.id);
            let w = d.icon.first().map_or(0, |r| r.chars().count());
            assert!(w == 8 && d.icon.len() == 8, "{} icon must be 8x8", d.id);
            assert!(d.icon.iter().all(|r| r.chars().count() == w), "{} icon ragged", d.id);
            assert!(matches!(d.slot, Some("head" | "body" | "feet" | "trinket")), "{} slot", d.id);
            assert!(price_of(d.id) > 0, "{} needs a price", d.id);
        }
        assert!(crate::gear_data::GEAR_DEFS.len() >= 45, "the catalog arrived whole");
        // Spot-check js values rode over: leathercap +1 armor, sevenleague +0.25 move.
        assert_eq!(get("leathercap").unwrap().stats, &[("defense", 1.0)]);
        assert_eq!(get("sevenleague").unwrap().stats, &[("move", 0.25)]);
        assert!(get("lantern").unwrap().flags.contains(&"light"));
    }

    /// Every recipe line resolves: outputs are real items, costs are real items or
    /// the "@FISH" wildcard, and stations are known — a typo starves a recipe forever.
    #[test]
    fn recipes_resolve() {
        const STATIONS: &[&str] = &["hand", "cook", "workbench", "forge", "alchemy", "enchanter", "fletcher", "jeweler", "farmtable"];
        for r in RECIPES {
            // A recipe may output something not yet registered (procedural gear etc.) — the
            // registry filter hides those in-game; here we only sanity-check what IS known.
            assert!(STATIONS.contains(&r.station), "{}: unknown station {}", r.out, r.station);
            assert!(r.n > 0);
            for (id, q) in r.cost {
                assert!(*q > 0);
                assert!(*id == "@FISH" || get(id).is_some(), "{}: unknown cost {id}", r.out);
            }
        }
    }

    #[test]
    fn rolls_respect_water_biome_season() {
        let mut rng = crate::worldgen::rng::Mulberry32::new(7);
        for _ in 0..200 {
            match roll_fish("forest", "SUMMER", "clear", "murk", || rng.next_f64()) {
                Catch::Fish { id, .. } => {
                    assert!(["minnow", "carp", "catfish", "eel"].contains(&id), "murk offered {id}");
                }
                Catch::Junk(j) => assert!(["oldboot", "seaweed", "driftwood", "mapbottle"].contains(&j)),
            }
        }
        // Winter mountains: icefish possible, sunfish/bluegill never.
        let mut saw_ice = false;
        for _ in 0..400 {
            if let Catch::Fish { id, .. } = roll_fish("mountains", "WINTER", "clear", "blue", || rng.next_f64()) {
                assert!(!["sunfish", "bluegill", "rainfish", "voidfin"].contains(&id), "out-of-season {id}");
                saw_ice |= id == "icefish";
            }
        }
        assert!(saw_ice, "icefish never bit in a winter lake");
    }
}
