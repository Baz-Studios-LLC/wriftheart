//! mobs_art_extra.rs — HAND-AUTHORED art for rs-original mobs (encounter bespokes
//! and friends). mobs_art.rs is GENERATED from js/enemies.js and a regen would drop
//! anything added there — originals live HERE, merged into the bank at build time.
//! Baz's encounter art rule: bespoke graphics make scenes feel special (but PEOPLE
//! always wear the villager sprite bank, never new human art).

use super::mobs_art::MobFrame;

/// THE ALPHA — one enormous, scarred, intelligent wolf (Ideas pdf). Side-facing
/// pair like any ground mob; the def adds scale on top of the bigger canvas.
pub static ALPHAWOLF_FRAMES: &[MobFrame] = &[
    MobFrame {
        grid: &[
            "......................",
            "............kk...kk...",
            "...........kddk.kddk..",
            "....k......kdddddddk..",
            "...kdk....kddddddddk..",
            "..kdddkkkkdddddddddk..",
            ".kdddddddddddDDddrdk..",
            ".kddsddddddddddddddk..",
            ".kdddddddddddddddtk...",
            "..kddddddddddddddk....",
            "...kddk..kddk..kdk....",
            "...kddk..kddk.kdk.....",
            "..kdk...kdk...kk......",
            "......................",
        ],
        pal: ALPHA_PAL,
    },
    MobFrame {
        grid: &[
            "......................",
            "............kk...kk...",
            "...........kddk.kddk..",
            "...k.......kdddddddk..",
            "..kdk.....kddddddddk..",
            "..kdddkkkkdddddddddk..",
            ".kdddddddddddDDddrdk..",
            ".kddsddddddddddddddk..",
            ".kdddddddddddddddtk...",
            "..kddddddddddddddk....",
            "....kddk..kddk.kdk....",
            "...kdk...kdk...kdk....",
            "...kk....kk.....kk....",
            "......................",
        ],
        pal: ALPHA_PAL,
    },
];
const ALPHA_PAL: &[(char, u32)] = &[
    ('k', 0x0e0e12), // near-black outline
    ('d', 0x2e2e36), // storm-dark fur
    ('D', 0x4a4a56), // shoulder highlight
    ('s', 0x8a8a94), // the old scar
    ('r', 0xd82020), // one burning eye
    ('t', 0xe8e8e0), // bared teeth
];

/// (kind, frames) — merged after the generated table in MobArtBank::build.
pub static EXTRA_FRAMES: &[(&str, &[MobFrame])] = &[("alphawolf", ALPHAWOLF_FRAMES)];

/// (kind, display name, bestiary line) — chained after the generated BESTIARY_INFO.
pub static EXTRA_BESTIARY: &[(&str, &str, &str)] = &[(
    "alphawolf",
    "THE ALPHA",
    "One enormous, scarred, and far too clever. The pack does what it says.",
)];
