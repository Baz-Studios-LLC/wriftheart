// GENERATED from js/items.js icon grids — do not edit.
//! items_art.rs — icons for the starter item set.

pub const WOOD_ICON: &[&str] = &["........", ".DDDDDD.", "DdDDDdDD", "DDDDDDDD", "DdDDDdDD", ".DDDDDD.", "........", "........"];
pub const STONE_ICON: &[&str] = &["........", "..aaa...", ".aAAaa..", "aAAaaaa.", "aaaaaaaa", ".aaaaaa.", "........", "........"];
pub const FIBER_ICON: &[&str] = &["........", "..l..l..", ".l.gg.l.", ".g.ll.g.", "..glg...", "..ggg...", "...g....", "........"];
pub const POTION_ICON: &[&str] = &["...dd...", "..KddK..", "..KroK..", "..KrrK..", "..KrrK..", "..KrrK..", "..KrrK..", "..KKKK.."];
pub const HERB_ICON: &[&str] = &["........", "...l....", "..lgl...", ".lgGgl..", ".llgll..", "...d....", "...d....", "........"];
pub const COPPER_ICON: &[&str] = &["........", "..ppp...", ".pYppp..", ".ppppY..", ".pYppp..", "..ppp...", "........", "........"];
// The square copper coin (js/entities.js COIN — matches the sidebar coin pips).
pub const COIN_ICON: &[&str] = &["........", ".KKKKKK.", ".KppppK.", ".KppppK.", ".KppppK.", ".KppppK.", ".KKKKKK.", "........"];
pub const ARROW_ICON: &[&str] = &["...A....", "..AAA...", "...D....", "...D....", "..WDW...", "...D....", "...D....", "........"];
pub const BANDAGE_ICON: &[&str] = &["........", "..WWWW..", ".WWrrWW.", ".WrWWrW.", ".WrWWrW.", ".WWrrWW.", "..WWWW..", "........"];
pub const GREATERPOTION_ICON: &[&str] = &["...dd...", "..KddK..", "..KwwK..", "..KVVK..", "..KVVK..", "..KVVK..", "..KVVK..", "..KKKK.."];
pub const ELIXIR_ICON: &[&str] = &["...P....", "..KPK...", ".KoooK..", ".KoooK..", ".KoPoK..", ".KoooK..", ".KoooK..", "..KKKK.."];
pub const LEATHER_ICON: &[&str] = &["........", ".SDDDS..", "SDDDDDS.", "SDDDDDS.", "SDDDDDS.", ".SDDDS..", "..SSS...", "........"];
pub const GEM_ICON: &[&str] = &["........", "..wWw...", ".wbbbw..", "wbBBbw..", ".wbBbw..", "..wbw...", "...w....", "........"];
pub const MEAT_ICON: &[&str] = &["........", ".W......", "WWrro...", ".orooor.", ".roooor.", ".rroorr.", "..rrrr..", "........"];
// A wound ball of string (js STRING_GRID; letters recolored via the def's icon_pal).
pub const STRING_ICON: &[&str] = &["..gggg..", ".gllllg.", "gllggllg", "glgllglg", "gllggllg", "gllllllg", ".gllllg.", "..gggg.."];
// The Pocket Watch (js I_WATCH): gold case, silver face, black hands at noon.
pub const WATCH_ICON: &[&str] = &["..PPPP..", ".PAAAAP.", "PAAKAAAP", "PAAKKAAP", "PAAAAAAP", ".PAAAAP.", "..PPPP..", "........"];
// Dungeon keys (js KEY_ICON / OKEY_ICON): plain iron, and the gilded gem-eyed ornate.
pub const KEY_ICON: &[&str] = &["..PPP...", ".P...P..", ".P...P..", "..PPP...", "...P....", "...PP...", "...P....", "...PP..."];
pub const OKEY_ICON: &[&str] = &["..PPP...", ".P.m.P..", ".P...P..", "..PPP...", "...P....", "..PPP...", "...P....", "..PP.PP."];
// A dungeon chest, closed + sprung (PORT-ORIGINAL stand-in art until the js chest
// entity draw ports; 16x12, feet on the tile floor).
pub const CHEST_ICON: &[&str] = &[
    ".KKKKKKKKKKKKK..", "KDDDDDDDDDDDDDK.", "KDPPPPPPPPPPPDK.", "KDDDDDDDDDDDDDK.",
    "KKKKKKKKKKKKKKK.", "KDDDDDPPPDDDDDK.", "KDDDDDPKPDDDDDK.", "KDDDDDPPPDDDDDK.",
    "KDDDDDDDDDDDDDK.", "KDDDDDDDDDDDDDK.", "KKKKKKKKKKKKKKK.", "................",
];
pub const CHEST_OPEN_ICON: &[&str] = &[
    ".KKKKKKKKKKKKK..", "KppppppppppppPK.", "KpKKKKKKKKKKKpK.", "KKKKKKKKKKKKKKK.",
    "KDDDDDDDDDDDDDK.", "KDKKKKKKKKKKKDK.", "KDKKKKKKKKKKKDK.", "KDKKKKKKKKKKKDK.",
    "KDDDDDDDDDDDDDK.", "KDDDDDDDDDDDDDK.", "KKKKKKKKKKKKKKK.", "................",
];
// The mimic's SPRUNG frames (its shut frame is CHEST_ICON verbatim — that IS the trick;
// the js S_MIMIC_SHUT was its own drawing, and Baz rightly called it obvious). Same
// 16x12 canvas as the chest. Bake overrides: 'R' maw red, 'T' tongue pink, 't' the
// tongue's darker root — the frog-tongue palette, so the lash and maw read as one flesh.
pub const MIMIC_OPEN_ICON: &[&str] = &[
    ".KKKKKKKKKKKKK..", "KDDDDDDDDDDDDDK.", "KDPPPPPPPPPPPDK.", "KKKKKKKKKKKKKKK.",
    "KRWRWRWRWRWRWRK.", "KRRRRRtTtRRRRRK.", "KRRtTTTTTTTtRRK.", "KWRTTTTTTTTTRWK.",
    "KDDDDDDDDDDDDDK.", "KDDDDDDDDDDDDDK.", "KKKKKKKKKKKKKKK.", "................",
];
pub const MIMIC_BITE_ICON: &[&str] = &[
    ".KKKKKKKKKKKKK..", "KDDDDDDDDDDDDDK.", "KDPPPPPPPPPPPDK.", "KKKKKKKKKKKKKKK.",
    "KWRWRWRWRWRWRWK.", "KRWRWTTTWRWRWRK.", "KDDDDDPPPDDDDDK.", "KDDDDDPKPDDDDDK.",
    "KDDDDDDDDDDDDDK.", "KDDDDDDDDDDDDDK.", "KKKKKKKKKKKKKKK.", "................",
];
// Fishing (js ROD_ICON / FISH_GRID / junk): the rod, the one fish silhouette every
// species recolors ('C'), and the river's consolation prizes.
pub const ROD_ICON: &[&str] = &[".......D", "......D.", ".....D..", "....DD..", "...D.W..", "..D..W..", ".D...W..", "D....W.."];
pub const FISH_GRID: &[&str] = &["........", "...CCCC.", ".K.CCCCC", "KKCCCKCC", ".K.CCCCC", "...CCCC.", "........", "........"];
pub const BOOT_ICON: &[&str] = &["........", ".DD.....", ".DD.....", ".DD.....", ".DDDDD..", ".DDDDDD.", ".dddddd.", "........"];
pub const WEED_ICON: &[&str] = &["..l..l..", ".l.l.l.l", ".l.ll.l.", "..lll...", "..l.l...", ".ll.l...", ".l..ll..", "........"];
pub const DRIFT_ICON: &[&str] = &["........", "......dd", "....ddd.", "..ddd...", ".dd.....", "dd......", "........", "........"];
// Farming (js HOE_ICON / CAN_ICON / PROD_GRID / SEED_GRID): the two farm tools, plus the
// shared produce + seed-packet silhouettes every crop recolors ('C').
pub const HOE_ICON: &[&str] = &[".....AAA", ".....A..", "....A...", "...d....", "..d.....", ".d......", "d.......", "........"];
pub const CAN_ICON: &[&str] = &["........", "..aa..A.", ".a..a.A.", ".aaaaaaA", ".aAAAAa.", ".aAAAAa.", ".aaaaaa.", "........"];
pub const PROD_GRID: &[&str] = &["...g....", "..gCCg..", ".CCCCCC.", "CCCCCCCC", "CCCCCCCC", ".CCCCCC.", "..CCCC..", "........"];
pub const SEED_GRID: &[&str] = &["........", ".DDDDDD.", ".DCCCCD.", ".DCCCCD.", ".DCCCCD.", ".DCCCCD.", ".DDDDDD.", "........"];
// The Windwood Flute (js FLUTE_ICON) — four notes of carved blossom-wood.
pub const FLUTE_ICON: &[&str] = &["........", "......dD", ".....dWd", "....dWd.", "...dWd..", "..dWd...", ".dDd....", "Dd......"];
