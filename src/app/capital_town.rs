//! capital_town.rs — the capital's AUTHORED DRESSING (Baz: bespoke everything,
//! one room at a time). A wake system stands each room's props up from a table,
//! exactly the guild-wing idiom: wall pieces flat, floor pieces blocked + sorted.
//! First pass: THE SOUTH GATE — twin crenellated towers in the kingdom's livery,
//! the arch the traveler walks UNDER (canopy z), lampposts on the Royal Way.

use bevy::prelude::*;

use super::battle::RoomActor;
use super::play::{CurRoom, GameWorld, SlideState};
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::gfx::{at, PIXEL_LAYER};

const CAPITAL_PAL: &[(char, u32)] = &[
    ('K', 0x000000),
    ('A', 0x8a8a92), // stone
    ('a', 0xb0b4be), // stone lite
    ('d', 0x6a7078), // mortar
    ('s', 0x545a64), // base course
    ('b', 0x2a4a8a), // the kingdom's blue
    ('y', 0xe8c050), // the kingdom's gold
    ('D', 0x6a4a2a), // door oak
    ('i', 0x3a3e46), // door iron
    ('o', 0xf0a040), // window ember
];

const CP_TOWER: [&str; 48] = [
    ".KK..KK..KK..KK..KK.",
    ".AA..AA..AA..AA..AA.",
    "KAAAAAAAAAAAAAAAAAAK",
    "KaaaaaaaaaaaaaaaaaaK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KKKKKKKKKKKKKKKKKKKK",
    "KAKAAKAAKAAKAAKAAKAK",
    "KAaaaaaaaaaaaaaaaaAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAsAAAAsAAAAsAAAAsAK",
    "KAAAAAAAKKKKAAAAAAAK",
    "KAAAAAAAKKKKAAAAAAAK",
    "KAAAAAAAAKKAAAAAAAAK",
    "KAAAAAAAAKKAAAAAAAAK",
    "KAAAAAAAAKKAAAAAAAAK",
    "KAsAAAAsAKKAsAAAAsAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAKKKKKKKKAAAAAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAsAAAbbbbbbbbAAAsAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAAAAAbbyyyybbAAAAAK",
    "KAAAAAbbyyyybbAAAAAK",
    "KAAAAAbbbyybbbAAAAAK",
    "KAAAAAbbbyybbbAAAAAK",
    "KAsAAAbbbyybbbAAAsAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAsAAAbbbbbbbbAAAsAK",
    "KAAAAAKKKKKKKKAAAAAK",
    "KAAAAAbAbAbAbAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAsAAAAsAAAAsAAAAsAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KKKKKKKKKKKKKKKKKKKK",
    "KssssssssssssssssssK",
    "KssssssssssssssssssK",
    "KssssssssssssssssssK",
];
const CP_ARCH: [&str; 22] = [
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "AaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAKKKKKKAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAyyyyyyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAyybbyyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAAsAAAAAsAAAAAsAAAAAsAAAAAsAAAyybbyyAAsAAAAAsAAAAAsAAAAAsAAAAAsAAAA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAyyyyyyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "AAAAAAAAAK..s...s...s...s...s...s...s...s...s...s...s...s.KAAAAAAAAA",
    "AAAAAAAAAK..s...s...s...s...s...s...s...s...s...s...s...s.KAAAAAAAAA",
    "AAAAAAAAAK..s...s...s...s...s...s...s...s...s...s...s...s.KAAAAAAAAA",
    "AAAAAAAAAA..K...K...K...K...K...K...K...K...K...K...K...K.AAAAAAAAAA",
    "AAAAAAAAAA................................................AAAAAAAAAA",
    "AAAAAAAAAA................................................AAAAAAAAAA",
    "AAAAAA........................................................AAAAAA",
    "AAAAAA........................................................AAAAAA",
    "AAAAAA........................................................AAAAAA",
    "....................................................................",
    "....................................................................",
    "....................................................................",
    "....................................................................",
];
const CP_LAMP: [&str; 22] = [
    "..KKKK..",
    ".KKKKKK.",
    ".KyyyyK.",
    ".KyyyyK.",
    ".KyyyyK.",
    ".KKKKKK.",
    "...KK...",
    "..KKKK..",
    "..KKKK..",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    ".KKKKKK.",
    ".KssssK.",
    ".KKKKKK.",
];

const CP_KEEPDOOR: [&str; 56] = [
    "................................................................",
    "............................KAAAAAAK............................",
    "........................KAAAAAAAAAAAAAAK........................",
    "....................KAAAAAAAAAAAAAAAAAAAAAAK....................",
    "................KAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAK................",
    "............KAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAK............",
    "..........KAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAK..........",
    "........KAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAK........",
    "....ssssKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKssss....",
    "....ssssssssssssssssssssssssssssssssssssssssssssssssssssssss....",
    "....ssssssssssssssssssssssssssssssssssssssssssssssssssssssss....",
    "....ssssssssssssssssssssssssssssssssssssssssssssssssssssssss....",
    "....ssAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAss....",
    "....ssAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAss....",
    "....ssAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAss....",
    "....ssAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAss....",
    "....ssAAAAAAKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKiiiiiiiiiiiiiiiiiiKKiiiiiiiiiiiiiiiiiiKAAAAAAss....",
    "....ssAAAAAAKiiiiiiiiiiiiiiiiiiKKiiiiiiiiiiiiiiiiiiKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKiiiiiiiiiiiiiiiiiiKKiiiiiiiiiiiiiiiiiiKAAAAAAss....",
    "....ssAAAAAAKiiiiiiiiiiiiiiiiiiKKiiiiiiiiiiiiiiiiiiKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDyyyDDKKDDyyyDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDyyyDDKKDDyyyDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDyyyDDKKDDyyyDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDyDDDKKDDDyDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDyDDDKKDDDyDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDyDDDKKDDDyDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKiiiiiiiiiiiiiiiiiiKKiiiiiiiiiiiiiiiiiiKAAAAAAss....",
    "....ssAAAAAAKiiiiiiiiiiiiiiiiiiKKiiiiiiiiiiiiiiiiiiKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
    "....ssAAAAAAKDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDKAAAAAAss....",
];
const CP_WINDOW: [&str; 24] = [
    "............",
    ".....KK.....",
    "....KKKK....",
    "...KKKKKK...",
    "..KKKKKKKK..",
    "..KKKKKKKK..",
    "..KooKKooK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KKKKKKKK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KyyKKyyK..",
    "..KKKKKKKK..",
];
const CP_STANDARD: [&str; 60] = [
    "..KKKKKKKKKKKKKKKKKKKKKKKK..",
    "..KKKKKKKKKKKKKKKKKKKKKKKK..",
    "....KKKKKKKKKKKKKKKKKKKK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbyybbyybbyybbbbK....",
    "....KbbbbyybbyybbyybbbbK....",
    "....KbbbbyybbyybbyybbbbK....",
    "....KbbbbyyyyyyyyyybbbbK....",
    "....KbbbbyyyyyyyyyybbbbK....",
    "....KbbbbyyyyyyyyyybbbbK....",
    "....KbbbbbyyyyyyyybbbbbK....",
    "....KbbbbbyyyyyyyybbbbbK....",
    "....KbbbbbyyyyyyyybbbbbK....",
    "....KbbbbbyyyyyyyybbbbbK....",
    "....KbbbbbyyyyyyyybbbbbK....",
    "....KbbbbbyyyyyyyybbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbyyyyyyyyyyyybbbK....",
    "....KbbbyyyyyyyyyyyybbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbyybbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "....KbbbbbbbbbbbbbbbbbbK....",
    "......b...b...b...b...b.....",
    "......b...b...b...b...b.....",
    "............................",
    "............................",
    "............................",
    "............................",
    "............................",
];
const CP_PARAPET: [&str; 10] = [
    "KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........KKKKKKKK........",
    "KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........KAAAAAAK........",
    "AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........",
    "AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........",
    "AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........",
    "AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........AAAAAAAA........",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
];
const CP_TURRET: [&str; 56] = [
    "..KKK..KKK..KKK...",
    "..AAA..AAA..AAA...",
    "..AAA..AAA..AAA...",
    "..AAA..AAA..AAA...",
    ".KAAAAAAAAAAAAAAK.",
    ".aaaaaaaaaaaaaaaa.",
    ".KAAAAAAAAAAAAAAK.",
    ".KKKKKKKKKKKKKKKK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAKKAAAAAAK.",
    ".KAAAAAAKKAAAAAAK.",
    ".KAAAAAAKKAAAAAAK.",
    ".KAAAAAAKKAAAAAAK.",
    ".KAAAAAAKKAAAAAAK.",
    ".KAAAAAAKKAAAAAAK.",
    ".KAAAAAAKKAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAsAAAAsAAAAsAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAsAAAAsAAAAsAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAsAAAAsAAAAsAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAsAAAAsAAAAsAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KAAAAAAAAAAAAAAK.",
    ".KKKKKKKKKKKKKKKK.",
    ".ssssssssssssssss.",
    ".ssssssssssssssss.",
    ".ssssssssssssssss.",
];
const CP_SLIT: [&str; 14] = [
    "......",
    ".KKKK.",
    ".KKKK.",
    ".KssK.",
    ".KssK.",
    ".KssK.",
    ".KssK.",
    ".KssK.",
    ".KssK.",
    ".KssK.",
    ".KssK.",
    ".KKKK.",
    ".KKKK.",
    "......",
];

#[derive(Component)]
pub struct CapitalProp;

/// (art, x, y, canopy, blocker) per room — indexed by the room's (kx, ky).
type Dress = (&'static [&'static str], f32, f32, bool, Option<(f32, f32, f32, f32)>);

fn dressing(kx: i32, ky: i32) -> &'static [Dress] {
    match (kx, ky) {
        // THE SOUTH GATE (2,4), CENTRED on the mouth (x 128-192, axis 160): the
        // towers hug the jambs, one seamless arch spans the Royal Way, and the
        // lamp pair mirrors about the axis.
        (2, 4) => &[
            (&CP_TOWER, 108.0, 160.0, false, None), // stand ON the rampart (already solid)
            (&CP_TOWER, 192.0, 160.0, false, None),
            (&CP_ARCH, 126.0, 156.0, true, None),
            (&CP_LAMP, 116.0, 136.0, false, Some((118.0, 154.0, 4.0, 4.0))),
            (&CP_LAMP, 196.0, 136.0, false, Some((198.0, 154.0, 4.0, 4.0))),
        ],
        // THE KEEP FACE (2,0): the wall IS the castle (Baz) — its door built in
        // at the mouth, lancet windows lit, the ROYAL STANDARD over the arch,
        // parapet trim crowning the whole face.
        (2, 0) => &[
            (&CP_PARAPET, 0.0, -2.0, false, None),
            (&CP_KEEPDOOR, 128.0, 74.0, false, None),
            (&CP_STANDARD, 146.0, 10.0, false, None),
            (&CP_WINDOW, 46.0, 34.0, false, None),
            (&CP_WINDOW, 78.0, 34.0, false, None),
            (&CP_WINDOW, 214.0, 34.0, false, None),
            (&CP_WINDOW, 246.0, 34.0, false, None),
        ],
        // THE CURTAIN WINGS (1,0)/(3,0): engaged turrets at the keep junctions,
        // slits along the walk.
        (1, 0) => &[
            (&CP_TURRET, 284.0, 14.0, false, None),
            (&CP_SLIT, 80.0, 30.0, false, None),
            (&CP_SLIT, 170.0, 30.0, false, None),
        ],
        (3, 0) => &[
            (&CP_TURRET, 2.0, 14.0, false, None),
            (&CP_SLIT, 118.0, 30.0, false, None),
            (&CP_SLIT, 208.0, 30.0, false, None),
        ],
        _ => &[],
    }
}

/// Stand the room's authored props up (hall_wake idiom: slide-sparing sweep).
#[allow(clippy::too_many_arguments)]
pub fn capital_wake(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cur: Res<CurRoom>,
    slide: Res<SlideState>,
    world: Res<GameWorld>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut woke: Local<Option<(i32, i32)>>,
    props: Query<Entity, With<CapitalProp>>,
    parents: Query<&ChildOf>,
) {
    if in_dungeon.0.is_some() || inside.0.is_some() {
        *woke = None;
        return;
    }
    if *woke == Some((cur.rx, cur.ry)) {
        return;
    }
    *woke = Some((cur.rx, cur.ry));
    let outgoing = slide.outgoing_root();
    for e in &props {
        if outgoing.is_some() && parents.get(e).ok().map(|p| p.parent()) == outgoing {
            continue;
        }
        commands.entity(e).despawn();
    }
    let Some((kx, ky)) = world.0.capital_room(cur.rx, cur.ry) else { return };
    for (grid, x, y, canopy, blk) in dressing(kx, ky) {
        let img = images.add(crate::gfx::bake(grid, CAPITAL_PAL));
        let (w, h) = (grid[0].len() as f32, grid.len() as f32);
        if let Some(b) = blk {
            if !blockers.0.contains(b) {
                blockers.0.push(*b);
            }
        }
        // Canopy pieces (the arch) draw ABOVE the hero — you walk under them.
        let z = if *canopy { 8.5 } else { actor_z(y + h) };
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + *x, PLAY_Y + *y, w, h, z),
            PIXEL_LAYER,
            RoomActor,
            CapitalProp,
        ));
    }
}

pub struct CapitalTownPlugin;
impl Plugin for CapitalTownPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy::app::FixedUpdate,
            capital_wake.before(super::play::EndTick).run_if(super::screen::playing),
        );
    }
}
