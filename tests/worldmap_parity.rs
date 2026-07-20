//! Whole-room map parity: `World::generate` must reproduce the live JS `World.getMap`
//! byte-for-byte across 299 golden rooms (3 seeds; a broad sweep around spawn covering towns,
//! roads, gates, lakes and rivers; the Black Castle and its approaches; a shard site; far-field
//! and negative-extreme rooms). One failing tile anywhere means some helper in the whole
//! biome/town/road/door/terrain/connectivity chain has drifted — this is the port's keystone test.

use wriftheart::worldgen::World;

mod golden {
    include!("data/worldmap_golden.rs");
}

#[test]
fn room_maps_match_js() {
    let mut world: Option<(u32, World)> = None;
    let mut checked = 0;
    for (seed, rx, ry, expected) in golden::MAPS {
        // Rebuild the World only when the seed changes (mirrors setSeed in the harness).
        if world.as_ref().map(|(s, _)| *s) != Some(*seed) {
            world = Some((*seed, World::new(*seed)));
        }
        let w = &world.as_ref().unwrap().1;
        let got = w.generate(*rx, *ry);
        for (row, want) in got.map.iter().zip(expected.iter()) {
            assert_eq!(
                row, want,
                "map drifted from JS at seed={seed} room=({rx},{ry})\n got: {:?}\nwant: {:?}",
                got.map, expected
            );
        }
        checked += 1;
    }
    assert_eq!(checked, golden::MAPS.len());
}
