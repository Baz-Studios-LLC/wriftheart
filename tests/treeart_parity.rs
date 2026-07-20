//! Tree-art parity: the ported seeded generators (build_oak/build_pine/build_cactus/
//! build_deadtree) must reproduce the JS builders' char grids across sampled seeds.
//! The generators mix integer hashing (bit-exact by contract) with f64 cos/sin/hypot —
//! if this ever fails by a pixel or two on some seed, suspect a libm ULP difference
//! at a threshold before suspecting the port.

use wriftheart::actors::props::tree_grid;

mod golden {
    include!("data/treeart_golden.rs");
}

#[test]
fn tree_grids_match_js() {
    for (kind, seed, want) in golden::TREES {
        let got = tree_grid(kind, *seed as i32);
        assert_eq!(got.len(), want.len(), "{kind} seed {seed}: row count");
        for (r, (g, w)) in got.iter().zip(want.iter()).enumerate() {
            assert_eq!(g, w, "{kind} seed {seed} row {r} drifted");
        }
    }
}
