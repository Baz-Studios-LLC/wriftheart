//! shadow_material.rs — the port's FIRST custom shader: the cast-shadow material
//! (shadow.wgsl). A [`Mesh2d`] quad textured with the OWNER's live sprite art,
//! flipped/sheared/blurred/blackened on the GPU — real gaussian softness and a real
//! sun lean, which plain sprites can't do (no shear, and scaling blurs).
//!
//! The plumbing here is deliberately generic — the pending additive glow pass
//! (PORT.md lighting step 2) reuses the same pattern with a different WGSL file and
//! `AlphaMode2d::Add`.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

/// The uniform block (shadow.wgsl `ShadowParams`).
#[derive(ShaderType, Debug, Clone, Copy)]
pub struct ShadowParams {
    /// The quad in ROOM pixels (x, y, w, h) — maps fragments to the water mask
    /// (shadows drown on water; the reflection shows there instead).
    pub rect: Vec4,
    /// Far-end x lean in quad widths (+ = east); the feet edge stays planted.
    pub shear: f32,
    /// Gaussian radius in texels (0 = crisp).
    pub blur: f32,
    /// Final shadow darkness.
    pub opacity: f32,
    /// Reserved (a flipped owner sprite later).
    pub flip_x: f32,
}

impl Default for ShadowParams {
    fn default() -> Self {
        Self { rect: Vec4::ZERO, shear: 0.0, blur: 1.2, opacity: 0.4, flip_x: 0.0 }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ShadowMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    /// The room's water mask (app/water.rs bake; r = water).
    #[texture(3)]
    #[sampler(4)]
    pub mask: Handle<Image>,
    #[uniform(2)]
    pub params: ShadowParams,
}

impl Material2d for ShadowMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://wriftheart/gfx/shadow.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// The shared unit quad every shadow scales from (size rides in the Transform).
#[derive(Resource)]
pub struct ShadowQuad(pub Handle<Mesh>);

pub struct ShadowMaterialPlugin;

impl Plugin for ShadowMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shadow.wgsl");
        app.add_plugins(Material2dPlugin::<ShadowMaterial>::default())
            .add_systems(Startup, |mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>| {
                commands.insert_resource(ShadowQuad(meshes.add(Rectangle::new(1.0, 1.0))));
            });
    }
}
