//! World-gen determinism parity: the Rust `worldgen::rng` must reproduce the live JS
//! (`js/world.js`) bit-for-bit. The golden vectors in `data/worldgen_golden.rs` are
//! machine-generated from the JS source; if this test ever fails, world generation has
//! drifted and every existing world would change. Regenerate the data ONLY when the JS
//! reference itself intentionally changes.

use wriftheart::worldgen::rng::{hash, value_noise, Mulberry32};

mod golden {
    include!("data/worldgen_golden.rs");
}

#[test]
fn hash_matches_js() {
    for &(seed, x, y, salt, expected) in golden::HASH {
        assert_eq!(
            hash(seed, x, y, salt),
            expected,
            "hash(seed={seed}, x={x}, y={y}, salt={salt}) drifted from JS"
        );
    }
}

#[test]
fn mulberry32_stream_matches_js() {
    for &(seed, expected) in golden::RNG {
        let mut rng = Mulberry32::new(seed);
        for (i, &want) in expected.iter().enumerate() {
            let got = rng.next_f64();
            // u32/2^32 is exact in f64, so the whole stream must be bit-identical, not merely close.
            assert_eq!(got.to_bits(), want.to_bits(), "rng seed={seed} draw #{i} drifted from JS");
        }
    }
}

#[test]
fn value_noise_matches_js() {
    for &(seed, x, y, salt, expected) in golden::NOISE {
        let got = value_noise(seed, x, y, salt);
        assert_eq!(
            got.to_bits(),
            expected.to_bits(),
            "value_noise(seed={seed}, x={x}, y={y}, salt={salt}) drifted from JS"
        );
    }
}
