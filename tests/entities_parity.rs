//! Entity-layout parity: `World::room_entities` must reproduce the live JS
//! `World.getEntities` descriptor-for-descriptor (type, kind/dest payload, position, champ/
//! elite flags, ORDER) across the golden rooms. This is the prop-layer keystone: the streams
//! share a `used`-occupancy set, so ONE drifted rng call shifts every placement after it —
//! including the mob roster.

use wriftheart::worldgen::World;

mod golden {
    include!("data/entities_golden.rs");
}

#[test]
fn entity_layouts_match_js() {
    let mut world: Option<(u32, World)> = None;
    let mut checked = 0usize;
    for (seed, rx, ry, want) in golden::ENTITIES {
        if world.as_ref().map(|(s, _)| *s) != Some(*seed) {
            world = Some((*seed, World::new(*seed)));
        }
        let w = &world.as_ref().unwrap().1;
        let got = w.room_entities(*rx, *ry);
        assert_eq!(
            got.len(),
            want.len(),
            "entity COUNT drifted in room ({rx},{ry}): got {} want {}\n got: {:?}",
            got.len(),
            want.len(),
            got.iter().map(|e| (e.kind, e.sub.as_str(), e.x, e.y)).collect::<Vec<_>>()
        );
        for (i, (g, (kind, sub, x, y, champ, elite))) in got.iter().zip(want.iter()).enumerate() {
            assert_eq!(
                (g.kind, g.sub.as_str(), g.x, g.y, g.champ, g.elite),
                (*kind, *sub, *x, *y, *champ, *elite),
                "entity {i} drifted in room ({rx},{ry})"
            );
        }
        checked += got.len();
    }
    assert!(checked > 1000, "suspiciously few entities checked: {checked}");
}
