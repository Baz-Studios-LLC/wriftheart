//! town_entities_parity.rs — the Rust townEntities port reproduces the live JS
//! descriptor-for-descriptor: kinds, positions, villager seeds and chatter lines, across
//! 12 real town rooms in 3 seeds (golden vectors from tools/extract_towns.mjs).

use wriftheart::worldgen::World;

mod data {
    include!("data/town_entities_golden.rs");
}

#[test]
fn town_entities_match_js() {
    let mut checked = 0;
    let mut world = World::new(0);
    for (seed, rx, ry, expected) in data::TOWN_GOLDEN {
        if world.seed != *seed {
            world = World::new(*seed);
        }
        let got = world.room_entities(*rx, *ry);
        assert_eq!(got.len(), expected.len(), "seed {seed} room ({rx},{ry}): count");
        for (g, (kind, sub, x, y, eseed, line)) in got.iter().zip(expected.iter()) {
            assert_eq!(g.kind, *kind, "seed {seed} ({rx},{ry}): kind");
            // The Rust port carries the js `kind` payload for buildings and the chatter
            // `line` for villagers in the one `sub` field.
            let want_sub = if *kind == "npc" { line } else { sub };
            assert_eq!(g.sub, *want_sub, "seed {seed} ({rx},{ry}) {kind}: sub");
            assert_eq!((g.x, g.y), (*x, *y), "seed {seed} ({rx},{ry}) {kind}: pos");
            assert_eq!(g.seed, *eseed, "seed {seed} ({rx},{ry}) {kind}: npc seed");
            checked += 1;
        }
    }
    assert!(checked > 200, "golden vectors look truncated: {checked}");
}
