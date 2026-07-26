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
    ('w', 0x7ac0e8), // fountain water
    ('W', 0xf4fbff), // spray
    ('e', 0xb08850), // bench oak lite
    ('r', 0xd05868), // urn blossom
    ('g', 0x58a058), // urn leaf
    ('l', 0x3e7a40), // topiary shade
    ('L', 0x79b25c), // topiary light
    ('m', 0xb888d8), // lavender bloom
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
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "AaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAKKKKKKAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAyyyyyyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAyybbyyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAAsAAAAAsAAAAAsAAAAAsAAAAAsAAAAAsAAAAAyybbyysAAAAAsAAAAAsAAAAAsAAAAAsAAAAAsAAAAAAAA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAyyyyyyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "AAAAAAAAAAAK...s...s...s...s...s...s...s...s...s...s...s...s...s...s....KAAAAAAAAAAA",
    "AAAAAAAAAAAK...s...s...s...s...s...s...s...s...s...s...s...s...s...s....KAAAAAAAAAAA",
    "AAAAAAAAAAAK...s...s...s...s...s...s...s...s...s...s...s...s...s...s....KAAAAAAAAAAA",
    "AAAAAAAAAAAA............................................................AAAAAAAAAAAA",
    "AAAAAAAAAAAA............................................................AAAAAAAAAAAA",
    "AAAAAAAAAAAA............................................................AAAAAAAAAAAA",
    "AAAAAAA......................................................................AAAAAAA",
    "AAAAAAA......................................................................AAAAAAA",
    "AAAAAAA......................................................................AAAAAAA",
    "....................................................................................",
    "....................................................................................",
    "....................................................................................",
    "....................................................................................",
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
    "................................................................................",
    "....................................KAAAAAAK....................................",
    "..............................KAAAAAAAAAAAAAAAAAAK..............................",
    "........................KAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAK........................",
    "..................KAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAK..................",
    ".............KAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAK.............",
    "..........KAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAK..........",
    "........KAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAK........",
    "....ssssKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKssss....",
    "....ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss....",
    "....ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss....",
    "....ssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssssss....",
    "....ssAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAss....",
    "....ssAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAss....",
    "....ssAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAss....",
    "....ssAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAss....",
    "....ssAAAAAAAAKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKiiiiiiiiiiiiiiiiiiiiiiiiKKiiiiiiiiiiiiiiiiiiiiiiiiKAAAAAAAAss....",
    "....ssAAAAAAAAKiiiiiiiiiiiiiiiiiiiiiiiiKKiiiiiiiiiiiiiiiiiiiiiiiiKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKiiiiiiiiiiiiiiiiiiiiiiiiKKiiiiiiiiiiiiiiiiiiiiiiiiKAAAAAAAAss....",
    "....ssAAAAAAAAKiiiiiiiiiiiiiiiiiiiiiiiiKKiiiiiiiiiiiiiiiiiiiiiiiiKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDyyyDDDKKDDDyyyDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDyyyDDDKKDDDyyyDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDyyyDDDKKDDDyyyDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDyDDDDKKDDDDyDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDyDDDDKKDDDDyDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDyDDDDKKDDDDyDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKiiiiiiiiiiiiiiiiiiiiiiiiKKiiiiiiiiiiiiiiiiiiiiiiiiKAAAAAAAAss....",
    "....ssAAAAAAAAKiiiiiiiiiiiiiiiiiiiiiiiiKKiiiiiiiiiiiiiiiiiiiiiiiiKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
    "....ssAAAAAAAAKDDDDDDDDDDDDDDDDDDDDDDDDKKDDDDDDDDDDDDDDDDDDDDDDDDKAAAAAAAAss....",
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

const CP_STALL: [&str; 28] = [
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "VVVVvvvvVVVVvvvvVVVVvvvvVVVVvvvvVV",
    "VVVVvvvvVVVVvvvvVVVVvvvvVVVVvvvvVV",
    "VVVVvvvvVVVVvvvvVVVVvvvvVVVVvvvvVV",
    "VVVVvvvvVVVVvvvvVVVVvvvvVVVVvvvvVV",
    "VVVVvvvvVVVVvvvvVVVVvvvvVVVVvvvvVV",
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    ".V...v...V...v...V...v...V...v...V",
    ".dd............................dd.",
    ".dd............................dd.",
    ".dd............................dd.",
    ".dd............................dd.",
    ".dd............................dd.",
    ".dd............................dd.",
    ".dd............................dd.",
    ".dd............................dd.",
    ".dd............................dd.",
    ".dd............................dd.",
    ".dd............................dd.",
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "DeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeD",
    "DDDDDgGgDDDDgGgDDDDgGgDDDDgGgDDDDD",
    "DDDDDgggDDDDgggDDDDgggDDDDgggDDDDD",
    "DDDDDgggDDDDgggDDDDgggDDDDgggDDDDD",
    "DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD",
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "KK..............................KK",
    "KK..............................KK",
];

/// A market stall: pressing its counter opens the trade's shelf.
#[derive(Component)]
pub struct CapitalStall {
    pub theme: usize,
    /// Which stall of this room (per-stall shop ledgers even for repeat trades).
    pub slot: usize,
    pub x: f32,
    pub y: f32,
}

/// Awning colors per trade: produce green, the catch blue, gear red, goods gold.
const STALL_AWNINGS: [(u32, u32); 4] =
    [(0x3f8f4f, 0x2a6634), (0x4a6ab0, 0x2a4a8a), (0xb0483a, 0x8a2a24), (0xd8b040, 0xa8842a)];

const CP_FOUNTAIN: [&str; 40] = [
    "............................................",
    "............................................",
    "............................................",
    "............................................",
    "............................................",
    ".................KKKKKKKKKK.................",
    ".............KKKKaaaaWaaaaaKKKK.............",
    "..........KKKaaaaaaaaWWaaaaaaaaKKK..........",
    ".........KaaaaaaaaaaaWWaaaaaaaaaaaK.........",
    ".......KKaaaaaaabbbbbWWbbbbbaaaaaaaKK.......",
    "......KaaaaaabbbbbbaaWWaabbbbbbaaaaaaK......",
    ".....KaaaaabbbbbaWaaaWWaaaaabbbbbaaaaaK.....",
    "....KaaaaabbbbbaaabbbbbbbWaaabbbbbaaaaaK....",
    "...KaaaabbbbbbAAAwwwwwwwwwwWAAbbbbbbaaaaK...",
    "..KaaaawwwwWwwAWAwwwwwwwwwwAAAwWwwwwwaaaaK..",
    "..KAAAAwwwwwwbAAAwwwwwwwwwwAAAbwwwwwwAAAAK..",
    "..KAAAwwwwwwbwAAAAwwwwwwwwAAAAwbwwwwwwAAAK..",
    ".KAAAAwwwwwbbwsAAAAAAAAAAAAAAswbbwwwwwAAAAK.",
    ".KAAAAwwwwwbwwssAAAAAAAAAAAAsswwbwwwwwAAAAK.",
    ".KAAAAwwwWwbwwwssssAAAAAAsssswwwbWwwwwAAAAK.",
    ".KAAAAwwwwwbWwbbssssssssssssbbWwbwwwwwAAAAK.",
    ".KAAAAwwwwwbbbbbbbbssssssbbbbbbbbwwwwwAAAAK.",
    ".KsAAAwwwwwwbbbbbbbbbbbbbbbbbbbbwwwwwwAAAsK.",
    ".KsAAAAwwwwWwbbbbbbbbbbbbbbbbbbWwwwwwAAAAsK.",
    ".KsAAAAwwwwwwwbbbbbbbbbbbbbbbbwwwwwwwAAAAsK.",
    ".KssAAAAwwwwwwwwbbwwwwwwwwbbwwwwwwwwAAAAssK.",
    "..KssAAAAAwwwwwwwWwbbbbbbWwwwwwwwwAAAAAssK..",
    "..KsssAAAAAwwwwwwwwwwwwwwwwwwwwwwAAAAAsssK..",
    "..KssssAAAAAAwwwwwwwwwwwwwwwwwwAAAAAAssssK..",
    "...KsssssAAAAAAAwwwwwwwwwwwwAAAAAAAsssssK...",
    "....KsssssAAAAAAAAAAAAAAAAAAAAAAAAsssssK....",
    ".....KsssssssAAAAAAAAAAAAAAAAAAsssssssK.....",
    "......KssssssssssAAAAAAAAAAssssssssssK......",
    ".......KKssssssssssssssssssssssssssKK.......",
    ".........KssssssssssssssssssssssssK.........",
    "..........KKKssssssssssssssssssKKK..........",
    ".............KKKKssssssssssKKKK.............",
    ".................KKKKKKKKKK.................",
    "............................................",
    "............................................",
];

const CP_FOUNTAIN_B: [&str; 40] = [
    "............................................",
    "............................................",
    "............................................",
    "............................................",
    "............................................",
    ".................KKKKKKKKKK.................",
    ".............KKKKaaaaaWaaaaKKKK.............",
    "..........KKKaaaaaaaaWWaaaaaaaaKKK..........",
    ".........KaaaaaaaaaaaWWaaaaaaaaaaaK.........",
    ".......KKaaaaaaabbbbbWWbbbbbaaaaaaaKK.......",
    "......KaaaaaabbbbbbaaWWaabbbbbbaaaaaaK......",
    ".....KaaaaabbbbbaaaaaWWaaaWabbbbbaaaaaK.....",
    "....KaaaaabbbbbaaaWbbbbbbbaaabbbbbaaaaaK....",
    "...KaaaabbbbbbAWAwwwwwwwwwwAAAbbbbbbaaaaK...",
    "..KaaaawwwwWwwAAAwwwwwwwwwwAAAwWwwwwwaaaaK..",
    "..KAAAAwwwwwwbAAAwwwwwwwwwwAWAbwwwwwwAAAAK..",
    "..KAAAwwwwwwbwAAAAwwwwwwwwAAAAwbwwwwwwAAAK..",
    ".KAAAAwwwwwbbwsAAAAAAAAAAAAAAswbbwwwwwAAAAK.",
    ".KAAAAwwwWwbwwssAAAAAAAAAAAAsswwbwwwwwAAAAK.",
    ".KAAAAwwwwwbWwwssssAAAAAAsssswWwbWwwwwAAAAK.",
    ".KAAAAwwwwwbwwbbssssssssssssbbwwbwwwwwAAAAK.",
    ".KAAAAwwwwwbbbbbbbbssssssbbbbbbbbwwwwwAAAAK.",
    ".KsAAAwwwwwwbbbbbbbbbbbbbbbbbbbbwwwwwwAAAsK.",
    ".KsAAAAwwwwWwbbbbbbbbbbbbbbbbbbWwwwwwAAAAsK.",
    ".KsAAAAwwwwwwwbbbbbbbbbbbbbbbbwwwwwwwAAAAsK.",
    ".KssAAAAwwwwwwwwbbwwwwwwwwbbwwwwwwwwAAAAssK.",
    "..KssAAAAAwwwwwwwWwbbbbbbWwwwwwwwwAAAAAssK..",
    "..KsssAAAAAwwwwwwwwwwwwwwwwwwwwwwAAAAAsssK..",
    "..KssssAAAAAAwwwwwwwwwwwwwwwwwwAAAAAAssssK..",
    "...KsssssAAAAAAAwwwwwwwwwwwwAAAAAAAsssssK...",
    "....KsssssAAAAAAAAAAAAAAAAAAAAAAAAsssssK....",
    ".....KsssssssAAAAAAAAAAAAAAAAAAsssssssK.....",
    "......KssssssssssAAAAAAAAAAssssssssssK......",
    ".......KKssssssssssssssssssssssssssKK.......",
    ".........KssssssssssssssssssssssssK.........",
    "..........KKKssssssssssssssssssKKK..........",
    ".............KKKKssssssssssKKKK.............",
    ".................KKKKKKKKKK.................",
    "............................................",
    "............................................",
];

const CP_FOUNTAIN_C: [&str; 40] = [
    "............................................",
    "............................................",
    "............................................",
    "............................................",
    "............................................",
    ".................KKKKKKKKKK.................",
    ".............KKKKaaaaWaaaaaKKKK.............",
    "..........KKKaaaaaaaaWWaaaaaaaaKKK..........",
    ".........KaaaaaaaaaaaWWaaaaaaaaaaaK.........",
    ".......KKaaaaaaabbbbbWWbbbbbaaaaaaaKK.......",
    "......KaaaaaabbbbbbaaWWaabbbbbbaaaaaaK......",
    ".....KaaaaabbbbbWaaaaWWaaaaabbbbbaaaaaK.....",
    "....KaaaaabbbbbaaabbbbbbbbaWabbbbbaaaaaK....",
    "...KaaaabbbbbbAAAwwwwwwwWwwAAAbbbbbbaaaaK...",
    "..KaaaawwwwWwwAAAwwwwwwwwwwAAAwWwwwwwaaaaK..",
    "..KAAAAwwwwwwbWAAwwwwwwwwwwAAAbwwwwwwAAAAK..",
    "..KAAAwwwwwwbwAAAAwwwwwwwwAAAAwbwwwwwwAAAK..",
    ".KAAAAwwwwwbbwsAAAAAAAAAAAAAAswbbwwwwwAAAAK.",
    ".KAAAAwwwWwbwwssAAAAAAAAAAAAsswwbwwwwwAAAAK.",
    ".KAAAAwwwwwbwwwssssAAAAAAsssswwwbWwwwwAAAAK.",
    ".KAAAAwwwwwbWwbbssssssssssssbbWwbwwwwwAAAAK.",
    ".KAAAAwwwwwbbbbbbbbssssssbbbbbbbbwwwwwAAAAK.",
    ".KsAAAwwwwwwbbbbbbbbbbbbbbbbbbbbwwwwwwAAAsK.",
    ".KsAAAAwwwwWwbbbbbbbbbbbbbbbbbbWwwwwwAAAAsK.",
    ".KsAAAAwwwwwwwbbbbbbbbbbbbbbbbwwwwwwwAAAAsK.",
    ".KssAAAAwwwwwwwwbbwwwwwwwwbbwwwwwwwwAAAAssK.",
    "..KssAAAAAwwwwwwwWwbbbbbbWwwwwwwwwAAAAAssK..",
    "..KsssAAAAAwwwwwwwwwwwwwwwwwwwwwwAAAAAsssK..",
    "..KssssAAAAAAwwwwwwwwwwwwwwwwwwAAAAAAssssK..",
    "...KsssssAAAAAAAwwwwwwwwwwwwAAAAAAAsssssK...",
    "....KsssssAAAAAAAAAAAAAAAAAAAAAAAAsssssK....",
    ".....KsssssssAAAAAAAAAAAAAAAAAAsssssssK.....",
    "......KssssssssssAAAAAAAAAAssssssssssK......",
    ".......KKssssssssssssssssssssssssssKK.......",
    ".........KssssssssssssssssssssssssK.........",
    "..........KKKssssssssssssssssssKKK..........",
    ".............KKKKssssssssssKKKK.............",
    ".................KKKKKKKKKK.................",
    "............................................",
    "............................................",
];
const CP_BENCH: [&str; 10] = [
    "....................",
    ".eeeeeeeeeeeeeeeeee.",
    ".DDDDDDDDDDDDDDDDDD.",
    "ii................ii",
    "iieeeeeeeeeeeeeeeeii",
    "iiDDDDDDDDDDDDDDDDii",
    "iiDDDDDDDDDDDDDDDDii",
    "iKKKKKKKKKKKKKKKKKKi",
    "ii................ii",
    "ii................ii",
];
const CP_URN: [&str; 16] = [
    "...r..r.....",
    "....gr..r...",
    "..r..g.g.r..",
    "...g..g.g...",
    "..KKKKKKKK..",
    "..aaaaaaaa..",
    "...AAAAAA...",
    "...AAAAAA...",
    "...AAAAAA...",
    "...AAAAAA...",
    "...AAAAAA...",
    "...AAAAAA...",
    "....ssss....",
    "..AAAAAAAA..",
    "..KKKKKKKK..",
    "............",
];

const CP_TOPIARY: [&str; 28] = [
    ".......LLLL.......",
    ".......gggg.......",
    ".......gggg.......",
    ".......llll.......",
    "........dd........",
    "........dd........",
    ".....LLLLLLLL.....",
    ".....gggggggg.....",
    ".....gggggggg.....",
    ".....llllllll.....",
    "........dd........",
    "........dd........",
    "...LLLLLLLLLLLL...",
    "...gggggggggggg...",
    "...gggggggggggg...",
    "...llllllllllll...",
    "...llllllllllll...",
    "........dd........",
    "........dd........",
    "........dd........",
    "........dd........",
    ".....ssssssss.....",
    "..KKKKKKKKKKKKKK..",
    "..aaaaaaaaaaaaaa..",
    "...AAAAAAAAAAAA...",
    "...AAAAAAAAAAAA...",
    "...ssssssssssss...",
    "..KKKKKKKKKKKKKK..",
];
const CP_BANNERPOLE: [&str; 40] = [
    "....yyyy....",
    "....yyyy....",
    ".....ii.....",
    ".iiiiiiiiii.",
    "..KKKKKKKK..",
    "..KbbbbbbK..",
    "..KbbbbbbK..",
    "..KbbbbbbK..",
    "..KbybbybK..",
    "..KbybbybK..",
    "..KbyyyybK..",
    "..KbyyyybK..",
    "..KbbbbbbK..",
    "..KbyyyybK..",
    "..KbyyyybK..",
    "..KbyyyybK..",
    "..KbbbbbbK..",
    "..KbbbbbbK..",
    "..KbbbbbbK..",
    "..KbbbbbbK..",
    "..KbbbbbbK..",
    "..KbbbbbbK..",
    "..KbbbbbbK..",
    "..KbbbbbbK..",
    "..KbbbbbbK..",
    "..KbbbbbbK..",
    "..KbbbbbbK..",
    "...b.bib....",
    "...b.bib....",
    ".....ii.....",
    ".....ii.....",
    ".....ii.....",
    ".....ii.....",
    ".....ii.....",
    ".....ii.....",
    ".....ii.....",
    "...AAAAAA...",
    "...AAAAAA...",
    "...ssssss...",
    "..KKKKKKKK..",
];
const CP_STATUE: [&str; 40] = [
    "......................",
    "......................",
    "......................",
    "......................",
    "........KKKKKK........",
    "........aaaaaa........",
    "........aKKKKa........",
    "........aaaaaa........",
    "........aaaaaa........",
    "......................",
    "....KKKKKKKKKKKKKK....",
    "....aaaaaaaaaaaaaa....",
    "....aaaaaaaaaaaaaa....",
    "....AAssAAAAAAAAAA....",
    "....AAssAAAAAAAAAA....",
    "....AAssAAAAAAAAAA....",
    "....AAssAAAAAAAAAA....",
    "....AAssAAAAAAAAAA....",
    "....AAssAAAAAAAAAA....",
    "....AAssAAAAAAAAAA....",
    "......ssAyyyyAAA......",
    "......ssAyyyyAAA......",
    "......ssAAaaAAAA......",
    ".......AAAaaAAA.......",
    ".......AAAaaAAA.......",
    ".......AAAaaAAA.......",
    ".......AAAaaAAA.......",
    ".......AAAaaAAA.......",
    ".......AAAaaAAA.......",
    ".......sssaasss.......",
    ".KKKKKKKKKKKKKKKKKKKK.",
    ".aaaaaaaaaaaaaaaaaaaa.",
    ".aaaaaaaaaaaaaaaaaaaa.",
    ".aaaaaaaaaaaaaaaaaaaa.",
    "AAAAAAAAAAAAAAAAAAAAAA",
    "AAAAAAAAAAAAAAAAAAAAAA",
    "AAAAAAAAAAAAAAAAAAAAAA",
    "ssssssssssssssssssssss",
    "ssssssssssssssssssssss",
    "KKKKKKKKKKKKKKKKKKKKKK",
];

const CP_BED_V: [&str; 24] = [
    "aaaaaaaaaa",
    "aDDDDDDDDa",
    "aDrrDDmmDa",
    "aDglDDglDa",
    "aDDDDDDDDa",
    "aDDDDDDDDa",
    "aDyyDDWWDa",
    "aDglDDglDa",
    "aDDDDDDDDa",
    "aDDDDDDDDa",
    "aDmmDDrrDa",
    "aDglDDglDa",
    "aDDDDDDDDa",
    "aDDDDDDDDa",
    "aDWWDDyyDa",
    "aDglDDglDa",
    "aDDDDDDDDa",
    "aDDDDDDDDa",
    "aDrrDDmmDa",
    "aDglDDglDa",
    "aDDDDDDDDa",
    "aDDDDDDDDa",
    "aDDDDDDDDa",
    "ssssssssss",
];
const CP_BED_H: [&str; 10] = [
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDa",
    "aDDrrDDDyyDDDmmDDDWWDDDrrDDDyyDa",
    "aDDglDDDglDDDglDDDglDDDglDDDglDa",
    "aDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDa",
    "aDDDmmDWWDDDDDrrDyyDDDDDmmDWWDDa",
    "aDDDglDglDDDDDglDglDDDDDglDglDDa",
    "aDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDa",
    "aDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDa",
    "ssssssssssssssssssssssssssssssss",
];

const CP_ROSE: [&str; 20] = [
    "........KKKK........",
    ".....KKKKKKKKKK.....",
    "....KKKbbyybbKKK....",
    "...KKbbbbyybbbbKK...",
    "..KKbbbbbyybbbbbKK..",
    ".KKbbbbbbyybbbbbbKK.",
    ".KKbbbbyyyyyybbbbKK.",
    ".KbbbbyybyybyybbbbK.",
    "KKbbbbybbyybbybbbbKK",
    "KKyyyyyyyyyyyyyyyyKK",
    "KKyyyyyyyyyyyyyyyyKK",
    "KKbbbbybbyybbybbbbKK",
    ".KbbbbyybyybyybbbbK.",
    ".KKbbbbyyyyyybbbbKK.",
    ".KKbbbbbbyybbbbbbKK.",
    "..KKbbbbbyybbbbbKK..",
    "...KKbbbbyybbbbKK...",
    "....KKKbbyybbKKK....",
    ".....KKKKKKKKKK.....",
    "........KKKK........",
];
const CP_CATHDOOR: [&str; 28] = [
    "..........KAAK..........",
    "........KAAAAAAK........",
    "......KAAAAAAAAAAK......",
    "....KAAAAAAAAAAAAAAK....",
    "..KAAAAAAAAAAAAAAAAAAK..",
    ".KAAAAAAAAAAAAAAAAAAAAK.",
    ".ssssssssssssssssssssss.",
    ".ssssssssssssssssssssss.",
    ".sAAAAAAAAAAAAAAAAAAAAs.",
    ".sAAAAAAAAAAAAAAAAAAAAs.",
    ".sAAKKKKKKKKKKKKKKKKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
    ".sAAKiiiiiiKKiiiiiiKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
    ".sAAKDDDDyDKKDyDDDDKAAs.",
    ".sAAKDDDDyDKKDyDDDDKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
    ".sAAKiiiiiiKKiiiiiiKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
    ".sAAKDDDDDDKKDDDDDDKAAs.",
];
const CP_CROSSTOP: [&str; 16] = [
    "....KK....",
    "....yy....",
    "....yy....",
    ".KyyyyyyK.",
    ".yyyyyyyy.",
    "....yy....",
    "....yy....",
    "....yy....",
    "....yy....",
    "....yy....",
    "....yy....",
    "....yy....",
    "....yy....",
    "..AAAAAA..",
    "..AAAAAA..",
    "..ssssss..",
];
const CP_HEADSTONE: [&str; 12] = [
    "...KaaK...",
    "..KAAAAK..",
    "..KAAAAK..",
    "..KAAAAK..",
    "..KAssAK..",
    "..KAAAAK..",
    "..KAssAK..",
    "..KAAAAK..",
    "..KAAAAK..",
    "..KAAAAK..",
    ".aaaaaaaa.",
    ".ssssssss.",
];

const CP_ROWHOUSE: [&str; 40] = [
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAKKKKKKKKAAAAAAAAAAAAAAAAAAKKKKKKKKAAAaa",
    "aaAAAKooKKooKAAAAAAAAAAAAAAAAAAKooKKooKAAAaa",
    "aaAAAKooKKooKAAAAAAAAAAAAAAAAAAKooKKooKAAAaa",
    "aaAAAKooKKooKAAAAAAAAAAAAAAAAAAKooKKooKAAAaa",
    "aaAAAKKKKKKKKAAAAAAAAAAAAAAAAAAKKKKKKKKAAAaa",
    "aaAAAKooKKooKAAAAAAAAAAAAAAAAAAKooKKooKAAAaa",
    "aaAAAKooKKooKAAAAAKKKKKKKKAAAAAKooKKooKAAAaa",
    "aaAAAKooKKooKAAAAAKDDDDDDKAAAAAKooKKooKAAAaa",
    "aaAAAKooKKooKAAAAAKDDDDDDKAAAAAKooKKooKAAAaa",
    "aaAAAKKKKKKKKAAAAAKDDDDDDKAAAAAKKKKKKKKAAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDyDKAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAaa",
    "ssssssssssssssssssssssssssssssssssssssssssss",
    "ssssssssssssssssssssssssssssssssssssssssssss",
];

/// BAZ'S CRAZY IDEA: citizens ROAM THE WHOLE CITY. The sim walks them across
/// all 25 rooms in capital-local pixel space (a room is 304x208; gx/304, gy/208
/// give the room); only the current room's walkers wear a sprite. They keep to
/// the ways, in lanes that pass clear of (or neatly behind) the fountain.
pub struct Citizen {
    pub gx: f32,
    pub gy: f32,
    pub wx: f32,
    pub wy: f32,
    pub pause: f32,
    pub seed: u32,
    /// Last tick's step (drives facing + walk cycle on the shown body).
    pub sx: f32,
    pub sy: f32,
}

#[derive(Resource, Default)]
pub struct Citizens(pub Vec<Citizen>, pub u32);

#[derive(Component)]
pub struct CitizenIdx(pub usize);

fn croll(s: &mut u32) -> f32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*s >> 8) as f32 / 16_777_216.0
}

/// A destination somewhere on the ways: a lane of the Royal Way (any distance
/// gate-to-castle) or a lane of the Cross Way (gate-to-gate).
fn waypoint(s: &mut u32) -> (f32, f32) {
    let r = croll(s);
    if r < 0.4 {
        let lane = if croll(s) < 0.5 { 726.0 + croll(s) * 6.0 } else { 780.0 + croll(s) * 4.0 };
        (lane, 84.0 + croll(s) * 932.0)
    } else if r < 0.75 {
        let lane = if croll(s) < 0.5 { 498.0 + croll(s) * 4.0 } else { 524.0 + croll(s) * 4.0 };
        (24.0 + croll(s) * 1456.0, lane)
    } else {
        // THE HIGH STREET (shop-district row), between the garden squares
        (350.0 + croll(s) * 900.0, 722.0 + croll(s) * 12.0)
    }
}

/// March the whole city's walkers (cheap: a handful of points, no collision —
/// the ways are open by construction).
fn citizens_sim(time: Res<bevy::prelude::Time>, mut cz: ResMut<Citizens>) {
    let Citizens(list, seed) = &mut *cz;
    if list.is_empty() {
        *seed = 0xC17_15EE;
        for _ in 0..18 {
            let (gx, gy) = waypoint(seed);
            let (wx, wy) = waypoint(seed);
            let pause = croll(seed) * 6.0;
            let cseed = (*seed).max(1);
            list.push(Citizen { gx, gy, wx, wy, pause, seed: cseed, sx: 0.0, sy: 0.0 });
        }
    }
    let dt = time.delta_secs().min(0.1);
    let step = 26.0 * dt;
    for c in list.iter_mut() {
        if c.pause > 0.0 {
            c.pause -= dt;
            c.sx = 0.0;
            c.sy = 0.0;
            continue;
        }
        let (ox, oy) = (c.gx, c.gy);
        // To reach the Royal Way, settle x first (walk the Cross to the column,
        // then turn); to reach the Cross Way, settle y first. Every turn lands
        // on road, never a lawn.
        let to_royal = c.wx > 712.0 && c.wx < 800.0 && !(c.wy > 490.0 && c.wy < 550.0);
        let (dx, dy) = (c.wx - c.gx, c.wy - c.gy);
        let on_royal_x = c.gx > 712.0 && c.gx < 800.0;
        if !to_royal && !on_royal_x && dy.abs() > 40.0 {
            // changing horizontals: walk this road to the Royal Way first
            let sx = 760.0 - c.gx;
            if sx.abs() > step { c.gx += step * sx.signum() } else { c.gx = 760.0 }
            c.sx = c.gx - ox;
            c.sy = 0.0;
            continue;
        }
        if to_royal {
            if dx.abs() > step {
                c.gx += step * dx.signum();
            } else {
                c.gx = c.wx;
                if dy.abs() > step { c.gy += step * dy.signum() } else { c.gy = c.wy }
            }
        } else if dy.abs() > step {
            c.gy += step * dy.signum();
        } else {
            c.gy = c.wy;
            if dx.abs() > step { c.gx += step * dx.signum() } else { c.gx = c.wx }
        }
        c.sx = c.gx - ox;
        c.sy = c.gy - oy;
        if (c.gx - c.wx).abs() < 0.5 && (c.gy - c.wy).abs() < 0.5 {
            c.pause = 1.5 + croll(seed) * 5.0;
            let (wx, wy) = waypoint(seed);
            c.wx = wx;
            c.wy = wy;
        }
    }
}

/// Dress the current room's walkers in villager bodies; strip the ones who left.
fn citizens_show(
    mut commands: Commands,
    cur: Res<CurRoom>,
    world: Res<GameWorld>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    cz: Res<Citizens>,
    mut shown: Query<(Entity, &CitizenIdx, &mut crate::actors::villager::Villager)>,
    players: Query<&super::play::Player>,
) {
    let cap = if in_dungeon.0.is_none() && inside.0.is_none() {
        world.0.capital_room(cur.rx, cur.ry)
    } else {
        None
    };
    let here = |i: usize| {
        cap.is_some_and(|(kx, ky)| {
            let c = &cz.0[i];
            (c.gx / 304.0).floor() as i32 == kx && (c.gy / 208.0).floor() as i32 == ky
        })
    };
    let mut present = [false; 32];
    for (e, idx, mut v) in &mut shown {
        if !here(idx.0) {
            commands.entity(e).despawn();
            continue;
        }
        present[idx.0] = true;
        let c = &cz.0[idx.0];
        v.x = c.gx % 304.0;
        v.y = c.gy % 208.0;
        if c.sx != 0.0 || c.sy != 0.0 {
            v.stride(c.sx, c.sy);
        } else if let Ok(p) = players.single() {
            let (dx, dy) = (p.x - v.x, p.y - v.y);
            v.face_point(dx, dy, dx.hypot(dy) < 48.0);
        }
    }
    let Some(_) = cap else { return };
    for (i, c) in cz.0.iter().enumerate() {
        if present[i] || !here(i) {
            continue;
        }
        let (lx, ly) = (c.gx % 304.0, c.gy % 208.0);
        let mut v = crate::actors::villager::Villager::new(lx, ly, c.seed, String::new());
        v.roaming = true;
        commands.spawn((
            Sprite::default(),
            at(PLAY_X + lx, PLAY_Y + ly, 16.0, 16.0, actor_z(ly + 16.0)),
            PIXEL_LAYER,
            RoomActor,
            CapitalProp,
            CitizenIdx(i),
            v,
        ));
    }
}

const CP_BASKET: [&str; 10] = [
    "...rr.r..r..",
    "..r..r.rr...",
    "KDDDDDDDDDDK",
    "KeeeeeeeeeeK",
    "KDDDDDDDDDDK",
    "KeeeeeeeeeeK",
    "KDDDDDDDDDDK",
    "KeeeeeeeeeeK",
    ".KKKKKKKKKK.",
    "............",
];

const CP_HOUSE_TALL: [&str; 48] = [
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAKKKKKKKKAAAAAAKKKKKKKKAAAaa",
    "aaAAAKooKKooKAAAAAAKooKKooKAAAaa",
    "aaAAAKooKKooKAAAAAAKooKKooKAAAaa",
    "aaAAAKooKKooKAAAAAAKooKKooKAAAaa",
    "aaAAAKKKKKKKKAAAAAAKKKKKKKKAAAaa",
    "aaAAAKooKKooKAAAAAAKooKKooKAAAaa",
    "aaAAAKooKKooKAAAAAAKooKKooKAAAaa",
    "aaAAAKooKKooKAAAAAAKooKKooKAAAaa",
    "aaAAAKKKKKKKKAAAAAAKKKKKKKKAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAKKKKKKKKAAAAAAAAAAAAAAAAAaa",
    "aaAAAKooKKooKAAAAAAAAAAAAAAAAAaa",
    "aaAAAKooKKooKAAAAAAAAAAAAAAAAAaa",
    "aaAAAKooKKooKAAAAAKKKKKKKKKKAAaa",
    "aaAAAKKKKKKKKAAAAAKDDDDDDDDKAAaa",
    "aaAAAKooKKooKAAAAAKDDDDDDDDKAAaa",
    "aaAAAKooKKooKAAAAAKDDDDDDDDKAAaa",
    "aaAAAKooKKooKAAAAAKDDDDDDDDKAAaa",
    "aaAAAKKKKKKKKAAAAAKDDDDDDDDKAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDDDKAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDyDKAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDDDKAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDDDKAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDDDKAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDDDKAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDDDKAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDDDKAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDDDKAAaa",
    "aaAAAAAAAAAAAAAAAAKDDDDDDDDKAAaa",
    "ssssssssssssssssssssssssssssssss",
    "ssssssssssssssssssssssssssssssss",
];
const CP_HOUSE_WIDE: [&str; 36] = [
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbK",
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAKKKKKKKKKKKKAAAAAAAAAAAAAAAAKKKKKKKKKKKKAAAAaa",
    "aaAAAAKooooKKooooKAAAAAAAAAAAAAAAAKooooKKooooKAAAAaa",
    "aaAAAAKooooKKooooKAAAAAAAAAAAAAAAAKooooKKooooKAAAAaa",
    "aaAAAAKooooKKooooKAAAAKKKKKKKKAAAAKooooKKooooKAAAAaa",
    "aaAAAAKooooKKooooKAAAAKDDDDDDKAAAAKooooKKooooKAAAAaa",
    "aaAAAAKKKKKKKKKKKKAAAAKDDDDDDKAAAAKKKKKKKKKKKKAAAAaa",
    "aaAAAAKooooKKooooKAAAAKDDDDDDKAAAAKooooKKooooKAAAAaa",
    "aaAAAAKooooKKooooKAAAAKDDDDDDKAAAAKooooKKooooKAAAAaa",
    "aaAAAAKooooKKooooKAAAAKDDDDDDKAAAAKooooKKooooKAAAAaa",
    "aaAAAAKooooKKooooKAAAAKDDDDDDKAAAAKooooKKooooKAAAAaa",
    "aaAAAAKKKKKKKKKKKKAAAAKDDDDyDKAAAAKKKKKKKKKKKKAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAAAAAaa",
    "aaAAAAAAAAAAAAAAAAAAAAKDDDDDDKAAAAAAAAAAAAAAAAAAAAaa",
    "ssssssssssssssssssssssssssssssssssssssssssssssssssss",
    "ssssssssssssssssssssssssssssssssssssssssssssssssssss",
];

/// The residential shapes (art, w, h) — the row house plus two new silhouettes.
const HOUSE_ARTS: [(&[&str], f32, f32); 3] =
    [(&CP_ROWHOUSE, 44.0, 40.0), (&CP_HOUSE_TALL, 32.0, 48.0), (&CP_HOUSE_WIDE, 52.0, 36.0)];

/// Four liveries: slate/grey, red/cream, green/tan, charcoal/blue-grey.
const HOUSE_PALS: [&[(char, u32)]; 4] = [
    &[('b', 0x2a4a8a), ('i', 0x1c3564), ('A', 0x9aa0a8), ('a', 0xb4bac2), ('D', 0x6a4a2a), ('o', 0xf0c060), ('K', 0x14161c), ('y', 0xe8c050), ('s', 0x545a64)],
    &[('b', 0x8a3226), ('i', 0x611f18), ('A', 0xcabb9a), ('a', 0xdacfae), ('D', 0x5a3a22), ('o', 0xf0c060), ('K', 0x14161c), ('y', 0xe8c050), ('s', 0x6a5a48)],
    &[('b', 0x3f6a35), ('i', 0x2a4a24), ('A', 0xb09a78), ('a', 0xc2ae8c), ('D', 0x4a3018), ('o', 0xf0c060), ('K', 0x14161c), ('y', 0xe8c050), ('s', 0x5c5040)],
    &[('b', 0x3a3e46), ('i', 0x272a30), ('A', 0x8e98a6), ('a', 0xa6b0bc), ('D', 0x5a3a22), ('o', 0xf0c060), ('K', 0x14161c), ('y', 0xe8c050), ('s', 0x4a505a)],
];

/// (shape, livery, x, y) — two dense terraces either side of the lanes, no two
/// neighbours sharing both shape and colour.
const HOUSE_SPOTS: [(usize, usize, f32, f32); 8] = [
    (1, 1, 20.0, 0.0),
    (0, 2, 60.0, 8.0),
    (0, 3, 180.0, 8.0),
    (1, 2, 232.0, 0.0),
    (2, 3, 24.0, 92.0),
    (0, 0, 84.0, 88.0),
    (0, 1, 180.0, 88.0),
    (2, 2, 232.0, 92.0),
];

const CP_MKCROSS: [&str; 36] = [
    "............KyyK............",
    "............yyyy............",
    ".........KyyyyyyyyK.........",
    ".........yyyyyyyyyy.........",
    "............yyyy............",
    "............yyyy............",
    "............yyyy............",
    "............yyyy............",
    "............yyyy............",
    "...........KKKKKK...........",
    "...........AAAAAA...........",
    "............aAAs............",
    "............aAAs............",
    "............aAAs............",
    "............aAAs............",
    "............aAAs............",
    "............aAAs............",
    "............aAAs............",
    "............aAAs............",
    "............aAAs............",
    "............aAAs............",
    "............aAAs............",
    ".........aaaaaaaaaa.........",
    ".........AAAAAAAAAA.........",
    ".........AAAAAAAAAA.........",
    ".........ssssssssss.........",
    ".....aaaaaaaaaaaaaaaaaa.....",
    ".....AAAAAAAAAAAAAAAAAA.....",
    ".....AAAAAAAAAAAAAAAAAA.....",
    ".....ssssssssssssssssss.....",
    ".aaaaaaaaaaaaaaaaaaaaaaaaaa.",
    ".AAAAAAAAAAAAAAAAAAAAAAAAAA.",
    ".AAAAAAAAAAAAAAAAAAAAAAAAAA.",
    ".ssssssssssssssssssssssssss.",
    ".ssssssssssssssssssssssssss.",
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKK",
];

/// The fountains run a little water loop (baked frames, pool idiom).
#[derive(Component)]
pub struct CapFx {
    frames: [Handle<Image>; 3],
    t: u32,
}

fn fountain_tick(mut q: Query<(&mut CapFx, &mut Sprite)>) {
    for (mut fx, mut sp) in &mut q {
        fx.t = fx.t.wrapping_add(1);
        let i = ((fx.t / 9) % 3) as usize;
        sp.image = fx.frames[i].clone();
    }
}

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
            (&CP_TOWER, 92.0, 160.0, false, None), // stand ON the rampart (already solid)
            (&CP_TOWER, 192.0, 160.0, false, None),
            (&CP_ARCH, 110.0, 156.0, true, None),
            (&CP_BED_V, 100.0, 88.0, false, Some((100.0, 89.0, 10.0, 22.0))),
            (&CP_BED_V, 194.0, 88.0, false, Some((194.0, 89.0, 10.0, 22.0))),
            (&CP_LAMP, 100.0, 100.0, false, Some((102.0, 118.0, 4.0, 4.0))),
            (&CP_LAMP, 196.0, 100.0, false, Some((198.0, 118.0, 4.0, 4.0))),
        ],
        // THE GRAND PLAZA (2,2): the fountain roundabout at the crossing of the
        // ways — traffic flows around it — benches and blossom urns at the
        // corners, lamplight on every approach.
        (2, 2) => &[
            (&CP_FOUNTAIN, 130.0, 84.0, false, Some((134.0, 102.0, 36.0, 16.0))),
            (&CP_BED_H, 136.0, 70.0, false, Some((137.0, 71.0, 30.0, 8.0))),
            (&CP_BED_H, 136.0, 128.0, false, Some((137.0, 129.0, 30.0, 8.0))),
            (&CP_BENCH, 56.0, 52.0, false, Some((57.0, 56.0, 18.0, 4.0))),
            (&CP_BENCH, 228.0, 52.0, false, Some((229.0, 56.0, 18.0, 4.0))),
            (&CP_BENCH, 56.0, 146.0, false, Some((57.0, 150.0, 18.0, 4.0))),
            (&CP_BENCH, 228.0, 146.0, false, Some((229.0, 150.0, 18.0, 4.0))),
            (&CP_URN, 100.0, 52.0, false, Some((102.0, 60.0, 8.0, 5.0))),
            (&CP_URN, 192.0, 52.0, false, Some((194.0, 60.0, 8.0, 5.0))),
            (&CP_URN, 100.0, 140.0, false, Some((102.0, 148.0, 8.0, 5.0))),
            (&CP_URN, 192.0, 140.0, false, Some((194.0, 148.0, 8.0, 5.0))),
            (&CP_LAMP, 84.0, 56.0, false, Some((86.0, 74.0, 4.0, 4.0))),
            (&CP_LAMP, 212.0, 56.0, false, Some((214.0, 74.0, 4.0, 4.0))),
            (&CP_LAMP, 84.0, 128.0, false, Some((86.0, 146.0, 4.0, 4.0))),
            (&CP_LAMP, 212.0, 128.0, false, Some((214.0, 146.0, 4.0, 4.0))),
        ],
        // THE KEEP FACE (2,0): the wall IS the castle (Baz) — its door built in
        // at the mouth, lancet windows lit, the ROYAL STANDARD over the arch,
        // parapet trim crowning the whole face.
        (2, 0) => &[
            (&CP_PARAPET, 0.0, -2.0, false, None),
            (&CP_KEEPDOOR, 112.0, 74.0, false, None),
            (&CP_STANDARD, 138.0, 10.0, false, None),
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
        // THE APPROACH (2,3): the tree-lined avenue from the S gate — clipped
        // topiary ranks with lamplight between, mirrored on the Way.
        (2, 3) => &[
            (&CP_TOPIARY, 82.0, 12.0, false, Some((84.0, 34.0, 14.0, 6.0))),
            (&CP_TOPIARY, 204.0, 12.0, false, Some((206.0, 34.0, 14.0, 6.0))),
            (&CP_TOPIARY, 82.0, 48.0, false, Some((84.0, 70.0, 14.0, 6.0))),
            (&CP_TOPIARY, 204.0, 48.0, false, Some((206.0, 70.0, 14.0, 6.0))),
            (&CP_TOPIARY, 82.0, 132.0, false, Some((84.0, 154.0, 14.0, 6.0))),
            (&CP_TOPIARY, 204.0, 132.0, false, Some((206.0, 154.0, 14.0, 6.0))),
            (&CP_TOPIARY, 82.0, 168.0, false, Some((84.0, 190.0, 14.0, 6.0))),
            (&CP_TOPIARY, 204.0, 168.0, false, Some((206.0, 190.0, 14.0, 6.0))),
            (&CP_BED_V, 100.0, 16.0, false, Some((100.0, 17.0, 10.0, 22.0))),
            (&CP_BED_V, 194.0, 16.0, false, Some((194.0, 17.0, 10.0, 22.0))),
            (&CP_BED_V, 100.0, 52.0, false, Some((100.0, 53.0, 10.0, 22.0))),
            (&CP_BED_V, 194.0, 52.0, false, Some((194.0, 53.0, 10.0, 22.0))),
            (&CP_BED_V, 100.0, 136.0, false, Some((100.0, 137.0, 10.0, 22.0))),
            (&CP_BED_V, 194.0, 136.0, false, Some((194.0, 137.0, 10.0, 22.0))),
            (&CP_BED_V, 100.0, 172.0, false, Some((100.0, 173.0, 10.0, 22.0))),
            (&CP_BED_V, 194.0, 172.0, false, Some((194.0, 173.0, 10.0, 22.0))),
            (&CP_LAMP, 64.0, 56.0, false, Some((66.0, 74.0, 4.0, 4.0))),
            (&CP_LAMP, 232.0, 56.0, false, Some((234.0, 74.0, 4.0, 4.0))),
            (&CP_LAMP, 64.0, 130.0, false, Some((66.0, 148.0, 4.0, 4.0))),
            (&CP_LAMP, 232.0, 130.0, false, Some((234.0, 148.0, 4.0, 4.0))),
        ],
        // THE PROCESSION (2,1): the last stretch before the keep — sentinel
        // statues and the kingdom's banners, mirrored on the Way.
        (2, 1) => &[
            (&CP_STATUE, 76.0, 24.0, false, Some((77.0, 56.0, 20.0, 8.0))),
            (&CP_STATUE, 206.0, 24.0, false, Some((207.0, 56.0, 20.0, 8.0))),
            (&CP_STATUE, 76.0, 120.0, false, Some((77.0, 152.0, 20.0, 8.0))),
            (&CP_STATUE, 206.0, 120.0, false, Some((207.0, 152.0, 20.0, 8.0))),
            (&CP_BANNERPOLE, 98.0, 8.0, false, Some((101.0, 44.0, 6.0, 4.0))),
            (&CP_BANNERPOLE, 194.0, 8.0, false, Some((197.0, 44.0, 6.0, 4.0))),
            (&CP_BANNERPOLE, 98.0, 144.0, false, Some((101.0, 180.0, 6.0, 4.0))),
            (&CP_BANNERPOLE, 194.0, 144.0, false, Some((197.0, 180.0, 6.0, 4.0))),
            (&CP_BED_V, 100.0, 40.0, false, Some((100.0, 41.0, 10.0, 22.0))),
            (&CP_BED_V, 194.0, 40.0, false, Some((194.0, 41.0, 10.0, 22.0))),
        ],
        // THE W GATE (0,2): twin towers on the rampart above and below the
        // mouth, lamplight and blossoms inside — mirrored on the Cross Way.
        (0, 2) => &[
            (&CP_TOWER, 8.0, 32.0, false, None),
            (&CP_TOWER, 8.0, 128.0, false, None),
            (&CP_LAMP, 36.0, 76.0, false, Some((38.0, 94.0, 4.0, 4.0))),
            (&CP_LAMP, 36.0, 110.0, false, Some((38.0, 128.0, 4.0, 4.0))),
            (&CP_BED_V, 40.0, 40.0, false, Some((40.0, 41.0, 10.0, 22.0))),
            (&CP_BED_V, 40.0, 144.0, false, Some((40.0, 145.0, 10.0, 22.0))),
        ],
        // THE E GATE (4,2): the same face, mirrored.
        (4, 2) => &[
            (&CP_TOWER, 276.0, 32.0, false, None),
            (&CP_TOWER, 276.0, 128.0, false, None),
            (&CP_LAMP, 260.0, 76.0, false, Some((262.0, 94.0, 4.0, 4.0))),
            (&CP_LAMP, 260.0, 110.0, false, Some((262.0, 128.0, 4.0, 4.0))),
            (&CP_BED_V, 254.0, 40.0, false, Some((254.0, 41.0, 10.0, 22.0))),
            (&CP_BED_V, 254.0, 144.0, false, Some((254.0, 145.0, 10.0, 22.0))),
        ],
        // THE PARTERRE GARDENS (0,1)/(4,1): benches on the walk's axis, urns
        // between the hedge quads.
        (0, 1) | (4, 1) => &[
            (&CP_BENCH, 142.0, 180.0, false, Some((143.0, 184.0, 18.0, 4.0))),
            (&CP_URN, 36.0, 76.0, false, Some((38.0, 84.0, 8.0, 5.0))),
            (&CP_URN, 256.0, 76.0, false, Some((258.0, 84.0, 8.0, 5.0))),
        ],
        // THE POND PARKS (0,0)/(4,0): a stone-rimmed pond in the green —
        // topiaries at the corners, benches facing the water, urns on the axis.
        (0, 0) => &[
            (&CP_TOPIARY, 72.0, 32.0, false, Some((74.0, 54.0, 14.0, 6.0))),
            (&CP_TOPIARY, 246.0, 32.0, false, Some((248.0, 54.0, 14.0, 6.0))),
            (&CP_TOPIARY, 72.0, 180.0, false, Some((74.0, 202.0, 14.0, 6.0))),
            (&CP_TOPIARY, 246.0, 180.0, false, Some((248.0, 202.0, 14.0, 6.0))),
            (&CP_BENCH, 36.0, 115.0, false, Some((37.0, 119.0, 18.0, 4.0))),
            (&CP_BENCH, 280.0, 115.0, false, Some((281.0, 119.0, 18.0, 4.0))),
            (&CP_URN, 162.0, 32.0, false, Some((164.0, 40.0, 8.0, 5.0))),
            (&CP_URN, 162.0, 192.0, false, Some((164.0, 200.0, 8.0, 5.0))),
            (&CP_LAMP, 100.0, 50.0, false, Some((102.0, 68.0, 4.0, 4.0))),
            (&CP_LAMP, 228.0, 50.0, false, Some((230.0, 68.0, 4.0, 4.0))),
            (&CP_LAMP, 100.0, 168.0, false, Some((102.0, 186.0, 4.0, 4.0))),
            (&CP_LAMP, 228.0, 168.0, false, Some((230.0, 186.0, 4.0, 4.0))),
        ],
        (4, 0) => &[
            (&CP_TOPIARY, 40.0, 32.0, false, Some((42.0, 54.0, 14.0, 6.0))),
            (&CP_TOPIARY, 214.0, 32.0, false, Some((216.0, 54.0, 14.0, 6.0))),
            (&CP_TOPIARY, 40.0, 180.0, false, Some((42.0, 202.0, 14.0, 6.0))),
            (&CP_TOPIARY, 214.0, 180.0, false, Some((216.0, 202.0, 14.0, 6.0))),
            (&CP_BENCH, 4.0, 115.0, false, Some((5.0, 119.0, 18.0, 4.0))),
            (&CP_BENCH, 248.0, 115.0, false, Some((249.0, 119.0, 18.0, 4.0))),
            (&CP_URN, 130.0, 32.0, false, Some((132.0, 40.0, 8.0, 5.0))),
            (&CP_URN, 130.0, 192.0, false, Some((132.0, 200.0, 8.0, 5.0))),
            (&CP_LAMP, 68.0, 50.0, false, Some((70.0, 68.0, 4.0, 4.0))),
            (&CP_LAMP, 196.0, 50.0, false, Some((198.0, 68.0, 4.0, 4.0))),
            (&CP_LAMP, 68.0, 168.0, false, Some((70.0, 186.0, 4.0, 4.0))),
            (&CP_LAMP, 196.0, 168.0, false, Some((198.0, 186.0, 4.0, 4.0))),
        ],
        // THE CATHEDRAL QUARTER (3,4): the nave face — rose window over the
        // pointed doors, lancets, the gold cross on the roofline — a lamplit
        // courtyard, and the hedge-walled graveyard in ordered ranks.
        (3, 4) => &[
            (&CP_ROSE, 166.0, 24.0, false, None),
            (&CP_CATHDOOR, 164.0, 52.0, false, None),
            (&CP_CROSSTOP, 172.0, 2.0, false, None),
            (&CP_WINDOW, 120.0, 28.0, false, None),
            (&CP_WINDOW, 220.0, 28.0, false, None),
            (&CP_LAMP, 140.0, 82.0, false, Some((142.0, 100.0, 4.0, 4.0))),
            (&CP_LAMP, 204.0, 82.0, false, Some((206.0, 100.0, 4.0, 4.0))),
            (&CP_URN, 100.0, 84.0, false, Some((102.0, 92.0, 8.0, 5.0))),
            (&CP_URN, 240.0, 84.0, false, Some((242.0, 92.0, 8.0, 5.0))),
            (&CP_URN, 100.0, 124.0, false, Some((102.0, 132.0, 8.0, 5.0))),
            (&CP_URN, 240.0, 124.0, false, Some((242.0, 132.0, 8.0, 5.0))),
            (&CP_BENCH, 120.0, 146.0, false, Some((121.0, 150.0, 18.0, 4.0))),
            (&CP_BENCH, 212.0, 146.0, false, Some((213.0, 150.0, 18.0, 4.0))),
            (&CP_HEADSTONE, 40.0, 36.0, false, Some((41.0, 44.0, 8.0, 4.0))),
            (&CP_HEADSTONE, 60.0, 36.0, false, Some((61.0, 44.0, 8.0, 4.0))),
            (&CP_HEADSTONE, 40.0, 68.0, false, Some((41.0, 76.0, 8.0, 4.0))),
            (&CP_HEADSTONE, 60.0, 68.0, false, Some((61.0, 76.0, 8.0, 4.0))),
        ],
        // THE SHOP DISTRICT (1,3)/(3,3): shopfronts on a cobbled high street,
        // awnings out, lamps at the ends. Doors open the trades' shelves.
        (1, 3) | (3, 3) => &[
            (&CP_LAMP, 20.0, 100.0, false, Some((22.0, 118.0, 4.0, 4.0))),
            (&CP_LAMP, 276.0, 100.0, false, Some((278.0, 118.0, 4.0, 4.0))),
        ],
        // THE RESIDENTIAL DISTRICT (1,4): row houses on cobbled lanes.
        (1, 4) => &[
            (&CP_LAMP, 76.0, 64.0, false, Some((78.0, 82.0, 4.0, 4.0))),
            (&CP_LAMP, 220.0, 64.0, false, Some((222.0, 82.0, 4.0, 4.0))),
        ],
        // THE ORCHARDS (0,4)/(4,4): fruit trees in working rows along the dirt
        // lanes, harvest baskets set out between them.
        (0, 4) | (4, 4) => &[
            (&CP_BASKET, 74.0, 60.0, false, Some((75.0, 64.0, 10.0, 5.0))),
            (&CP_BASKET, 218.0, 60.0, false, Some((219.0, 64.0, 10.0, 5.0))),
            (&CP_BASKET, 146.0, 124.0, false, Some((147.0, 128.0, 10.0, 5.0))),
        ],
        // THE STATUE GARDEN (0,3): a sentinel ringed by topiary on the west square.
        (0, 3) => &[
            (&CP_STATUE, 149.0, 66.0, false, Some((150.0, 98.0, 20.0, 8.0))),
            (&CP_TOPIARY, 100.0, 24.0, false, Some((102.0, 46.0, 14.0, 6.0))),
            (&CP_TOPIARY, 202.0, 24.0, false, Some((204.0, 46.0, 14.0, 6.0))),
            (&CP_TOPIARY, 100.0, 116.0, false, Some((102.0, 138.0, 14.0, 6.0))),
            (&CP_TOPIARY, 202.0, 116.0, false, Some((204.0, 138.0, 14.0, 6.0))),
            (&CP_BENCH, 106.0, 92.0, false, Some((107.0, 96.0, 18.0, 4.0))),
            (&CP_BENCH, 194.0, 92.0, false, Some((195.0, 96.0, 18.0, 4.0))),
            (&CP_URN, 154.0, 44.0, false, Some((156.0, 52.0, 8.0, 5.0))),
            (&CP_URN, 154.0, 148.0, false, Some((156.0, 156.0, 8.0, 5.0))),
        ],
        // THE COMMONS GREEN (4,3): a second fountain square east, benches all round.
        (4, 3) => &[
            (&CP_FOUNTAIN, 122.0, 76.0, false, Some((126.0, 94.0, 36.0, 16.0))),
            (&CP_BENCH, 134.0, 40.0, false, Some((135.0, 44.0, 18.0, 4.0))),
            (&CP_BENCH, 134.0, 150.0, false, Some((135.0, 154.0, 18.0, 4.0))),
            (&CP_BENCH, 84.0, 88.0, false, Some((85.0, 92.0, 18.0, 4.0))),
            (&CP_BENCH, 184.0, 88.0, false, Some((185.0, 92.0, 18.0, 4.0))),
            (&CP_URN, 84.0, 44.0, false, Some((86.0, 52.0, 8.0, 5.0))),
            (&CP_URN, 192.0, 44.0, false, Some((194.0, 52.0, 8.0, 5.0))),
            (&CP_URN, 84.0, 144.0, false, Some((86.0, 152.0, 8.0, 5.0))),
            (&CP_URN, 192.0, 144.0, false, Some((194.0, 152.0, 8.0, 5.0))),
            (&CP_LAMP, 48.0, 60.0, false, Some((50.0, 78.0, 4.0, 4.0))),
            (&CP_LAMP, 232.0, 60.0, false, Some((234.0, 78.0, 4.0, 4.0))),
            (&CP_LAMP, 48.0, 124.0, false, Some((50.0, 142.0, 4.0, 4.0))),
            (&CP_LAMP, 232.0, 124.0, false, Some((234.0, 142.0, 4.0, 4.0))),
        ],
        // THE GUILDHALL FORECOURT (1,1): sentinels and benches on the hall's
        // green, lamplight at the pad corners.
        (1, 1) => &[
            (&CP_STATUE, 50.0, 96.0, false, Some((51.0, 128.0, 20.0, 8.0))),
            (&CP_STATUE, 232.0, 96.0, false, Some((233.0, 128.0, 20.0, 8.0))),
            (&CP_URN, 122.0, 68.0, false, Some((124.0, 76.0, 8.0, 5.0))),
            (&CP_URN, 170.0, 68.0, false, Some((172.0, 76.0, 8.0, 5.0))),
            (&CP_BED_H, 100.0, 132.0, false, Some((101.0, 133.0, 30.0, 8.0))),
            (&CP_BED_H, 172.0, 132.0, false, Some((173.0, 133.0, 30.0, 8.0))),
            (&CP_BENCH, 106.0, 162.0, false, Some((107.0, 166.0, 18.0, 4.0))),
            (&CP_BENCH, 178.0, 162.0, false, Some((179.0, 166.0, 18.0, 4.0))),
            (&CP_LAMP, 84.0, 32.0, false, Some((86.0, 50.0, 4.0, 4.0))),
            (&CP_LAMP, 212.0, 32.0, false, Some((214.0, 50.0, 4.0, 4.0))),
            (&CP_LAMP, 84.0, 112.0, false, Some((86.0, 130.0, 4.0, 4.0))),
            (&CP_LAMP, 212.0, 112.0, false, Some((214.0, 130.0, 4.0, 4.0))),
        ],
        // THE MARKET SQUARES (1,2)/(3,2): lamplit corners, benches on the south
        // rail, baskets and blossom strips between the stalls (self-symmetric
        // about x152, so one arm serves both mirrored rooms).
        (1, 2) | (3, 2) => &[
            (&CP_MKCROSS, 138.0, 62.0, false, Some((142.0, 90.0, 20.0, 8.0))),
            (&CP_BED_H, 88.0, 16.0, false, Some((89.0, 17.0, 30.0, 8.0))),
            (&CP_BED_H, 184.0, 16.0, false, Some((185.0, 17.0, 30.0, 8.0))),
            (&CP_BASKET, 104.0, 44.0, false, Some((105.0, 48.0, 10.0, 5.0))),
            (&CP_BASKET, 190.0, 44.0, false, Some((191.0, 48.0, 10.0, 5.0))),
            (&CP_BASKET, 104.0, 144.0, false, Some((105.0, 148.0, 10.0, 5.0))),
            (&CP_BASKET, 190.0, 144.0, false, Some((191.0, 148.0, 10.0, 5.0))),
            (&CP_LAMP, 52.0, 24.0, false, Some((54.0, 42.0, 4.0, 4.0))),
            (&CP_LAMP, 244.0, 24.0, false, Some((246.0, 42.0, 4.0, 4.0))),
            (&CP_LAMP, 52.0, 160.0, false, Some((54.0, 178.0, 4.0, 4.0))),
            (&CP_LAMP, 244.0, 160.0, false, Some((246.0, 178.0, 4.0, 4.0))),
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
    // THE MARKET (1,2)/(3,2): stalls with their vendors at the counters (Baz: the
    // market lives OUTSIDE). Themes: west = produce + the catch; east = gear + goods.
    let stalls: &[(usize, f32, f32)] = match (kx, ky) {
        (1, 2) => &[(0, 64.0, 28.0), (1, 135.0, 28.0), (3, 206.0, 28.0), (1, 64.0, 128.0), (3, 135.0, 128.0), (0, 206.0, 128.0)],
        (3, 2) => &[(2, 64.0, 28.0), (3, 135.0, 28.0), (0, 206.0, 28.0), (1, 64.0, 128.0), (2, 135.0, 128.0), (3, 206.0, 128.0)],
        _ => &[],
    };
    for (slot, &(theme, sx, sy)) in stalls.iter().enumerate() {
        let (hi, lo) = STALL_AWNINGS[theme];
        let pal: &[(char, u32)] = &[
            ('K', 0x000000),
            ('V', hi),
            ('v', lo),
            ('d', 0x6a4a2a),
            ('D', 0x8a6a3a),
            ('e', 0xb08850),
            ('g', 0x9a9aa2),
            ('G', 0xc8ccd8),
        ];
        let img = images.add(crate::gfx::bake(&CP_STALL, pal));
        let blk = (sx + 1.0, sy + 20.0, 32.0, 7.0);
        if !blockers.0.contains(&blk) {
            blockers.0.push(blk);
        }
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + sx, PLAY_Y + sy, 34.0, 28.0, actor_z(sy + 26.0)),
            PIXEL_LAYER,
            RoomActor,
            CapitalProp,
            CapitalStall { theme, slot, x: sx, y: sy },
            super::shadows::CastsShadow { left: sx + 1.0, top: sy + 24.0, w: 32, a: 0.85 },
        ));
        // The vendor, key-less at their counter (press = SHOP, not chat).
        let seed = 0xcab1u32 ^ (kx as u32 * 31 + ky as u32 * 7 + theme as u32).wrapping_mul(0x9e37_79b9);
        let (vx, vy) = (sx + 9.0, sy + 4.0);
        let mut v = crate::actors::villager::Villager::new(vx, vy, seed, String::new());
        v.hold_post();
        commands.spawn((
            Sprite::default(),
            at(PLAY_X + vx, PLAY_Y + vy, 16.0, 16.0, actor_z(vy + 16.0)),
            PIXEL_LAYER,
            RoomActor,
            CapitalProp,
            v,
        ));
    }
    // THE RESIDENTIAL DISTRICT (1,4): dense varied homes, baked per livery.
    if (kx, ky) == (1, 4) {
        for &(shape, pal, hx, hy) in &HOUSE_SPOTS {
            let (art, w, h) = HOUSE_ARTS[shape];
            let img = images.add(crate::gfx::bake(art, HOUSE_PALS[pal]));
            let blk = (hx + 2.0, hy + h - 24.0, w - 4.0, 22.0);
            if !blockers.0.contains(&blk) {
                blockers.0.push(blk);
            }
            commands.spawn((
                Sprite::from_image(img),
                at(PLAY_X + hx, PLAY_Y + hy, w, h, actor_z(hy + h - 2.0)),
                PIXEL_LAYER,
                RoomActor,
                CapitalProp,
                super::shadows::CastsShadow { left: hx + 2.0, top: hy + h - 4.0, w: (w - 4.0) as u32, a: 0.9 },
            ));
        }
    }
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
        let e = commands
            .spawn((
                Sprite::from_image(img.clone()),
                at(PLAY_X + *x, PLAY_Y + *y, w, h, z),
                PIXEL_LAYER,
                RoomActor,
                CapitalProp,
            ))
            .id();
        // Freestanding pieces opt into the shader shadow system (the same
        // silhouette-sampled, sun-sheared shadows the trees wear).
        let feet: Option<u32> = [
            (CP_LAMP.as_ptr(), 6u32),
            (CP_STATUE.as_ptr(), 20),
            (CP_TOPIARY.as_ptr(), 14),
            (CP_BANNERPOLE.as_ptr(), 6),
            (CP_URN.as_ptr(), 10),
            (CP_BENCH.as_ptr(), 18),
            (CP_BASKET.as_ptr(), 10),
            (CP_MKCROSS.as_ptr(), 24),
        ]
        .iter()
        .find(|(p, _)| *p == grid.as_ptr())
        .map(|(_, fw)| *fw);
        if let Some(fw) = feet {
            commands.entity(e).insert(super::shadows::CastsShadow {
                left: *x + (w - fw as f32) / 2.0,
                top: *y + h - 4.0,
                w: fw,
                a: 0.85,
            });
        }
        // The fountain animates: two more frames, cycled by fountain_tick.
        if std::ptr::eq(grid.as_ptr(), CP_FOUNTAIN.as_ptr()) {
            let frames = [
                img,
                images.add(crate::gfx::bake(&CP_FOUNTAIN_B, CAPITAL_PAL)),
                images.add(crate::gfx::bake(&CP_FOUNTAIN_C, CAPITAL_PAL)),
            ];
            commands.entity(e).insert(CapFx { frames, t: 0 });
        }
    }
}

/// PRESS at a stall counter: the trade's shelf opens (specialist idiom).
#[allow(clippy::too_many_arguments)]
fn stall_interact(
    mut input: ResMut<crate::input::ActionState>,
    mut shop: ResMut<super::shop::ShopState>,
    bought: Res<super::shop::BoughtShop>,
    mut next: ResMut<NextState<super::screen::Screen>>,
    cur: Res<CurRoom>,
    clock: Res<super::room_render::FrameClock>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&super::play::Player>,
    stalls: Query<&CapitalStall>,
) {
    use crate::input::Action;
    if !input.pressed(Action::Interact) {
        return;
    }
    let Ok(p) = players.single() else { return };
    let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    for st in &stalls {
        let zone = (st.x - 2.0, st.y + 18.0, 38.0, 16.0);
        if !(hitbox.0 < zone.0 + zone.2 && hitbox.0 + hitbox.2 > zone.0 && hitbox.1 < zone.1 + zone.3 && hitbox.1 + hitbox.3 > zone.1) {
            continue;
        }
        input.consume(Action::Interact);
        super::shop::open_capital_stall(
            &mut shop,
            &bought,
            st.theme,
            st.slot,
            cur.rx,
            cur.ry,
            super::gather::farm_day(clock.0),
        );
        next.set(super::screen::Screen::Shop);
        sfx.write(super::sfx::Sfx("open"));
        return;
    }
}

pub struct CapitalTownPlugin;
impl Plugin for CapitalTownPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Citizens>();
        app.add_systems(
            bevy::app::FixedUpdate,
            (
                capital_wake,
                stall_interact.after(capital_wake).before(super::talk::talk_tick),
                citizens_sim,
                citizens_show.after(citizens_sim).after(capital_wake),
                fountain_tick,
            )
                .before(super::play::EndTick)
                .run_if(super::screen::playing),
        );
    }
}
