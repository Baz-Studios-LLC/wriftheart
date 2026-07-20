//! weather_fx_material.rs — the weather pass's Material2d plumbing (the shadow-material
//! pattern): one screen quad over the lit play field running gfx/weather_fx.wgsl.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

#[derive(ShaderType, Debug, Clone, Copy, Default)]
pub struct WeatherFxParams {
    /// (kind, visibility, heavy, unused) — kinds 0 none / 1 rain / 2 snow / 3 dust / 4 fog.
    pub layer_a: Vec4,
    pub layer_b: Vec4,
    pub time: f32,
    pub wind: f32,
    pub flash: f32,
    /// Accumulated wind TRAVEL (the integral of wind over play-time) — never derive
    /// displacement as time x current wind: when wind changes, the product jumps by
    /// the whole elapsed time and the field lurches (Baz caught it in three menus).
    pub windx: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WeatherFxMaterial {
    #[uniform(0)]
    pub params: WeatherFxParams,
}

impl Material2d for WeatherFxMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://wriftheart/gfx/weather_fx.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

pub struct WeatherFxMaterialPlugin;

impl Plugin for WeatherFxMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "weather_fx.wgsl");
        app.add_plugins(Material2dPlugin::<WeatherFxMaterial>::default());
    }
}
