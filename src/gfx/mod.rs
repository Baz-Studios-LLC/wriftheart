//! gfx — the rendering foundation: the palette, the char-grid sprite baker, and the
//! pixel-perfect low-res canvas. Everything visual builds on these.

pub mod layers;
pub mod bake;
pub mod canvas;
pub mod edge_dressing;
pub mod font;
pub mod palette;
pub mod shadow_material;
pub mod weather_fx_material;
pub mod water_material;
pub mod tile_textures;
pub mod tiles_art;

pub use bake::{bake, flip_h};
pub use canvas::{at, PixelCanvasPlugin, PIXEL_LAYER};
pub use tile_textures::TileTextures;
