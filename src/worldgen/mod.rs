//! worldgen — the procedural overworld: deterministic from (seed, room x, room y).
//!
//! `rng` is the arithmetic core (hash + PRNG + value noise) and must stay bit-identical to the
//! JS. On top of it: `biomes` (the rule table), `towns` (sites/footprints/roads), `doors`
//! (edge openings), `world` (the per-seed World with shard/saltmaze sites + terrain queries),
//! and `generate` (the room map builder). Parity with the JS is pinned by
//! `tests/worldgen_parity.rs` (arithmetic) and `tests/worldmap_parity.rs` (whole room maps).

pub mod biomes;
pub mod doors;
pub mod edges;
pub mod entities;
pub mod generate;
pub mod rng;
pub mod spawns;
pub mod town_entities;
pub mod towns;
pub mod world;

pub use entities::RoomEntity;
pub use generate::RoomMap;
pub use world::{World, COLS, ROWS};
