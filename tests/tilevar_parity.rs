//! Procedural tile parity: thin_ground variants + water frames must match the JS
//! generators char-for-char (they share the seeds, so any drift is an arithmetic bug).

use wriftheart::gfx::tile_textures::{build_water_rows, thin_ground_rows};
use wriftheart::gfx::tiles_art::{ART, GROUND_DEFS};

mod golden {
    include!("data/tilevar_golden.rs");
}

fn art(name: &str) -> &'static [&'static str; 16] {
    &ART.iter().find(|(n, _)| *n == name).unwrap().1
}

#[test]
fn thin_ground_matches_js() {
    for (name, variant, want) in golden::THIN {
        let (_, base, speck, art_name) = GROUND_DEFS.iter().find(|(n, ..)| n == name).unwrap();
        let seed = variant * 131 + name.bytes().next().unwrap() as u32 * 17 + 1;
        let rows = thin_ground_rows(art(art_name), *base, *speck, 0.6, seed);
        assert_eq!(rows, want.to_vec(), "thin_ground({name}, v{variant}) drifted from JS");
    }
}

#[test]
fn water_frames_match_js() {
    for (phase, wave, base, want) in golden::WATER {
        assert_eq!(
            build_water_rows(*phase, *wave, *base),
            want.to_vec(),
            "build_water(phase={phase}) drifted from JS"
        );
    }
}
