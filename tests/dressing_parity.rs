//! Edge-dressing parity: `dressing_rects` must reproduce the live JS room.js draw()'s
//! fillRect stream rect-for-rect — same order, geometry, and colours — across 27 golden
//! rooms (a 5x5 sweep around spawn with coastlines, roads, towns and tree lines, plus two
//! far-field rooms exercising the scallop hash on negative world-pixel coordinates).
//! One drifted rect means the corner-nook logic, the kind grid, a flat colour table, or the
//! scallop hash (`rnd`/`edge_h`) has diverged from the JS.

use wriftheart::gfx::edge_dressing::dressing_rects;
use wriftheart::room::RoomGrid;
use wriftheart::worldgen::World;

mod golden {
    include!("data/dressing_golden.rs");
}

#[test]
fn edge_dressing_matches_js() {
    let mut world: Option<(u32, World)> = None;
    let mut checked = 0usize;
    for (seed, rx, ry, want) in golden::DRESSING {
        if world.as_ref().map(|(s, _)| *s) != Some(*seed) {
            world = Some((*seed, World::new(*seed)));
        }
        let w = &world.as_ref().unwrap().1;
        let grid = RoomGrid::from_map(&w.generate(*rx, *ry));
        let got = dressing_rects(&grid, w, *rx, *ry);
        assert_eq!(
            got.len(),
            want.len(),
            "rect COUNT drifted from JS in room ({rx},{ry}): got {} want {}",
            got.len(),
            want.len()
        );
        for (i, (g, expect)) in got.iter().zip(want.iter()).enumerate() {
            assert_eq!(g, expect, "rect {i} drifted from JS in room ({rx},{ry})");
        }
        checked += got.len();
    }
    assert!(checked > 10_000, "suspiciously few rects checked: {checked}");
}
