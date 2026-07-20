//! WriftHeart — Rust/Bevy port (library root).
//!
//! The crate is split lib + bin on purpose: every subsystem lives behind a module here so it
//! can be unit- and integration-tested in isolation (`main.rs` is only the app wiring). See
//! PORT.md for the module map and the enforced rules — no file grows past ~500 lines, and
//! repeated patterns become shared functions before the third copy exists.

pub mod actors;
pub mod app;
pub mod combat;
pub mod deathlines;
pub mod dungeon;
pub mod gfx;
pub mod input;
pub mod inventory;
pub mod guildhall;
pub mod gear_data;
pub mod procgen;
pub mod recipes_data;
pub mod items;
pub mod lore_books;
pub mod people;
pub mod people_data;
pub mod persist;
pub mod relics_data;
pub mod room;
pub mod settings;
pub mod achievements;
pub mod skilltree;
pub mod songs;
pub mod stock;
pub mod stock_tables;
pub mod tiles;
pub mod weather;
pub mod traits;
pub mod ui;
pub mod worldgen;

/// Native game resolution — 1080p/5, 16:9. Every layout constant in the JS assumes it.
pub const CANVAS_W: u32 = 384;
// 208, not the js 216: the room is exactly 13 tiles (208px) tall, so a 216-high canvas
// left 4px dead bands above and below the play field. DELIBERATE DEVIATION (Baz, 2026-07-16):
// the canvas now hugs the content; the window's integer scaler letterboxes outside it.
pub const CANVAS_H: u32 = 208;

/// The left HUD strip: title, 4 ability slots, HP/MP bars. The room renders to its right.
pub const SIDEBAR_W: f32 = 80.0;
