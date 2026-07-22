//! encounter_art.rs — the set-piece decor grids from js/encounters.js, verbatim: camp
//! fires (2 flicker frames), the overturned wagon, ritual circles, tents, war banners,
//! treasure, webs, ice pillars, captive stakes, and the fallen. Default palette letters
//! except the per-call recolours (banner cloth 'r', crystal body 'x').

pub const CAMP_A: &[&str] = &[
    "................", ".......o........", "......ooo.......", "......oro.......",
    ".....orPro......", ".....rPPPr......", ".....rrPrr......", "....DdDdDdD.....",
    "...dDdDdDdDd....", "................",
];
pub const CAMP_B: &[&str] = &[
    "................", "......o.o.......", "......ooo.......", ".....oroo.......",
    ".....orPro......", ".....rPPrr......", ".....rrPrr......", "....DdDdDdD.....",
    "...dDdDdDdDd....", "................",
];
pub const CORPSE: &[&str] = &[
    "................", "................", "....KK....KK....", "...KssK..KSSK...",
    "...KsSSSSSSsK...", "..KSSSSSSSSSSK..", "...KK..KK..KK...", "................",
];
// The trader's COVERED WAGON, 32x21 (Baz: the old 16x12 handcart "really is too
// small lol"): a ribbed canvas top over a plank bed on two big spoked wheels.
// Shared by the roadside caravan AND encounter wagons — one wagon in this world.
pub const WAGON: &[&str] = &[
    "....KKKKKKKKKKKKKKKKKKKKKKKK....",
    "...KyyyyyyyyyyyyyyyyyyyyyyyyK...",
    "..KyyYyyyyyYyyyyyYyyyyyYyyyyyK..",
    "..KyyYyyyyyYyyyyyYyyyyyYyyyyyK..",
    "..KyyYyyyyyYyyyyyYyyyyyYyyyyyK..",
    "..KyyYyyyyyYyyyyyYyyyyyYyyyyyK..",
    "..KyyYyyyyyYyyyyyYyyyyyYyyyyyK..",
    "..KYYYYYYYYYYYYYYYYYYYYYYYYYYK..",
    "..KKKKKKKKKKKKKKKKKKKKKKKKKKKK..",
    ".KDDDDDDDDDDDDDDDDDDDDDDDDDDDDK.",
    ".KDdDDDdDDDdDDDdDDDdDDDdDDDdDDK.",
    ".KDDDDDDDDDDDDDDDDDDDDDDDDDDDDK.",
    ".KddddddddddddddddddddddddddddK.",
    ".KKKKKKKKKKKKKKKKKKKKKKKKKKKKKK.",
    "....KKK..................KKK....",
    "...KnnnK................KnnnK...",
    "..KnnSnnK..............KnnSnnK..",
    "..KnSdSnK..............KnSdSnK..",
    "..KnnSnnK..............KnnSnnK..",
    "...KnnnK................KnnnK...",
    "....KKK..................KKK....",
];

#[cfg(test)]
mod wagon_tests {
    #[test]
    fn wagon_rows_are_uniform() {
        // bake()'s width check is debug_assert-only; a ragged row would just render
        // truncated in release. Pin it here.
        assert!(super::WAGON.iter().all(|r| r.len() == 32), "WAGON rows must all be 32 chars");
        assert_eq!(super::WAGON.len(), 21);
    }
}
pub const RITUAL: &[&str] = &[
    "................", "....xXXXXXx.....", "..xX.......Xx...", ".xX...mmm...Xx..",
    ".X...m...m...X..", "X...m.....m...X.", "X..m.......m..X.", "X..m.......m..X.",
    "X...m.....m...X.", ".X...m...m...X..", ".xX...mmm...Xx..", "..xX.......Xx...",
    "....xXXXXXx.....", "................",
];
pub const BONES: &[&str] = &[
    "................", "................", "....W...WW......", "...WAW...W......",
    "....W..WAW......", "......WW...W....", "................", "................",
];
pub const CRATE: &[&str] = &[
    "................", "................", "...KKKKKKK......", "...KDdDdDK......",
    "...KdDDDdK......", "...KDdKdDK......", "...KdDDDdK......", "...KKKKKKK......",
    "................",
];
pub const TENT: &[&str] = &[
    ".......K........", "......KDK.......", ".....KDDDK......", "....KDDDDDK.....",
    "...KDDDDDDDK....", "..KDDDDDDDDDK...", ".KDDDDKKKDDDDK..", ".KDDDKnnnKDDDK..",
    "KDDDDKnnnKDDDDK.", "KDDDDKnnnKDDDDK.", "KDDDDKnnnKDDDDK.", "KKKKKKKKKKKKKKK.",
    "................",
];
pub const BANNER_ART: &[&str] = &[
    "....P...........", "...PKP..........", "....rK..........", "...KrrK.........",
    "...KrrrK........", "...KrrrrK.......", "...KrrrK........", "...KrrK.........",
    "....d...........", "....d...........", "....d...........", "....d...........",
    "...ddd..........", "................",
];
pub const GOLD: &[&str] = &[
    "................", "................", "................", "......PP........",
    "....PPpPPr......", "...PpPPpPPPb....", "..PPPpPPpPPPP...", "..pPPPPpPPPpP...",
    "................",
];
pub const CRYSTAL_ART: &[&str] = &[
    ".......K........", "......KxK.......", "......KxK.......", ".....KxxxK......",
    ".....KxWxK......", ".....KxxxK......", "....KxxxxxK.....", "....KxXxxXK.....",
    "....KxxxxxK.....", ".....KKKKK......", "................",
];
pub const WEB: &[&str] = &[
    "A.A.A.A.A.A.A...", ".A.A.A.A.A.A....", "A.A.A.A.A.A.A...", ".A.A.A.A.A.A....",
    "A.A.A.A.A.A.A...", ".A.A.A.A.A.A....", "................", "................",
];
pub const ICE: &[&str] = &[
    ".......W........", "......WfW.......", "......ffF.......", ".....WffFW......",
    ".....fffFf......", ".....FffFF......", "....FfffffF.....", "....FFfffFF.....",
    "....FFFFFFF.....", ".....FFFFF......", "................",
];
pub const STAKE: &[&str] = &[
    "......KdK.......", "......KdK.......", ".....AAdAA......", "......KdK.......",
    "......KdK.......", ".....AAdAA......", "......KdK.......", "......KdK.......",
    ".....KdddK......", "................",
];
/// A pool of spilt blood (js Entities.bloodPool — the flat splat under the fallen).
/// 'q' recolours to dried-blood red at bake time (the palette's q is goblin green).
pub const BLOOD_POOL: &[&str] = &[
    "................", "................", "......qq........", "....qqqqqq......",
    "...qqqqqqqqq....", "....qqqqqqq.....", "......qqq.......", "................",
];
pub const BLOOD_PAL: &[(char, u32)] = &[('q', 0x7c0800)];

// --- rs-original bespoke props (Baz: encounters lean on bespoke graphics) --------

/// THE FALSE MERCHANT's roadside stall: striped awning over a laden counter.
pub const STALL: &[&str] = &[
    "..........................",
    ".kkkkkkkkkkkkkkkkkkkkkkk..",
    ".kaAaaAaaAaaAaaAaaAaaAak..",
    ".kAaaAaaAaaAaaAaaAaaAaAk..",
    ".kkkkkkkkkkkkkkkkkkkkkkk..",
    "..kW..................Wk..",
    "..kW..................Wk..",
    "..kW...gg...yy....gg..Wk..",
    ".kkkkkkkkkkkkkkkkkkkkkkk..",
    ".kWwwwWwwwWwwwWwwwWwwwWk..",
    ".kwwwwwwwwwwwwwwwwwwwwwk..",
    ".kkkkkkkkkkkkkkkkkkkkkkk..",
    "..kw.................wk...",
    "..kw.................wk...",
    "..kk.................kk...",
    "..........................",
];
pub const STALL_PAL: &[(char, u32)] = &[
    ('k', 0x14100c),
    ('a', 0xb02828), // awning red
    ('A', 0xe8dcc0), // awning cream
    ('w', 0x6a4a26), // counter wood
    ('W', 0x8a6636), // wood light
    ('g', 0x5aa04a), // wares
    ('y', 0xd8b040), // wares
];

/// THE WHISPERING WELL: a mossy ancient ring over a mouth of pure dark.
pub const OLDWELL: &[&str] = &[
    "....................",
    "......kkkkkkkk......",
    "....kkSSSSSSSSkk....",
    "...kSSSmSSSSSSSk....",
    "..kSSkkkkkkkkkSSk...",
    "..kSkDDDDDDDDDkSk...",
    ".kSSkDDDDDDDDDkSSk..",
    ".kSmkDDDDDDDDDkSSk..",
    ".kSSkDDDDDDDDDkmSk..",
    ".kSSSkkkkkkkkkSSSk..",
    "..kSSSSSSmSSSSSSk...",
    "..kkSSSSSSSSSSkk....",
    "....kkSSmSSSkk......",
    "......kkkkkk........",
    "....................",
];
pub const OLDWELL_PAL: &[(char, u32)] = &[
    ('k', 0x0c0e10),
    ('S', 0x6a7078), // old stone
    ('m', 0x4a7a3a), // moss
    ('D', 0x05060a), // the dark below
];
