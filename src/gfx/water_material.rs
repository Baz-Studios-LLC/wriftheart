//! water_material.rs — the water pass's two materials (the shadow-material pattern,
//! second and third users): [`WaterMaterial`] animates the surface over the baked
//! tiles, [`ReflectionMaterial`] mirrors an actor's live sprite onto the water. Both
//! clip to the per-room WATER MASK (app/water.rs bakes it: r = water, a = depth).

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct WaterParams {
    pub time: f32,
    pub strength: f32,
    pub _p0: f32, // `storm` in the shader — rain agitation 0..1
    pub _p1: f32,
    /// The style palette (Baz: water rebuilt FROM THE GROUND UP, shader-painted —
    /// no more tile sprite underneath): shallow / deep / wave-light, rgb in xyz.
    pub shallow: Vec4,
    pub deep: Vec4,
    pub wave: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub mask: Handle<Image>,
    #[uniform(2)]
    pub params: WaterParams,
}

impl Material2d for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://wriftheart/gfx/water.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct ReflectionParams {
    /// The quad in ROOM pixels (x, y, w, h) — maps fragments onto the mask.
    pub rect: Vec4,
    pub time: f32,
    pub opacity: f32,
    /// X wobble amplitude, in texels.
    pub ripple: f32,
    pub _pad: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ReflectionMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    #[texture(2)]
    #[sampler(3)]
    pub mask: Handle<Image>,
    #[uniform(4)]
    pub params: ReflectionParams,
}

impl Material2d for ReflectionMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://wriftheart/gfx/reflection.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

pub struct WaterMaterialPlugin;

impl Plugin for WaterMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "water.wgsl");
        embedded_asset!(app, "reflection.wgsl");
        app.add_plugins(Material2dPlugin::<WaterMaterial>::default())
            .add_plugins(Material2dPlugin::<ReflectionMaterial>::default());
    }
}
