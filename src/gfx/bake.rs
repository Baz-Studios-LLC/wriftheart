//! bake.rs — turn a char grid into a texture (port of `bake()` in js/assets.js).

use super::palette::{palette, rgba};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Bake a char grid into a texture.
///
/// `overrides` recolors specific chars for this bake only — the port of `bake(grid, override)`'s
/// second argument (hero outfits, ore-vein metals, wood grades, …). Later entries win. Every row
/// must be the same length; rows run top-to-bottom, matching the JS authoring.
pub fn bake(grid: &[&str], overrides: &[(char, u32)]) -> Image {
    let h = grid.len() as u32;
    let w = grid.first().map_or(0, |r| r.chars().count()) as u32;
    debug_assert!(
        grid.iter().all(|r| r.chars().count() as u32 == w),
        "bake(): every row must be the same width"
    );

    // Start fully transparent, then write the opaque pixels (JS leaves alpha 0 for '.').
    let mut img = Image::new_fill(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    for (y, row) in grid.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            // An override beats the base palette; an unmapped char stays transparent.
            let hex = overrides
                .iter()
                .rev()
                .find(|(c, _)| *c == ch)
                .map(|(_, hex)| *hex)
                .or_else(|| palette(ch));
            let Some(hex) = hex else { continue };
            if let Ok(px) = img.pixel_bytes_mut(UVec3::new(x as u32, y as u32, 0)) {
                px.copy_from_slice(&rgba(hex));
            }
        }
    }
    img
}

/// Mirror a grid horizontally — the port of `Assets.flipH` (left facings are the right ones
/// flipped, so only three directions are authored).
pub fn flip_h(grid: &[&str]) -> Vec<String> {
    grid.iter().map(|r| r.chars().rev().collect()).collect()
}
