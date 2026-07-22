// Release builds on Windows are GUI apps — don't pop a console window behind the game.
// (Debug keeps the console so dev logs / WRIFT_SHOT output stay visible.)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! WriftHeart — Rust/Bevy port (binary entry point).
//!
//! Thin by design: it wires the Bevy app; rendering setup lives in `gfx::PixelCanvasPlugin`
//! and the playable scene in `app::PlayPlugin`. All real logic is library modules (lib.rs).

use bevy::prelude::*;
use wriftheart::app::{BattlePlugin, CodexPlugin, DebugShotPlugin, EncountersPlugin, FarmPlugin, FlutePlugin, GatherPlugin, HudPlugin, MenuPlugin, PlayPlugin, QuestsPlugin, SlideOutPlugin};
use wriftheart::gfx::PixelCanvasPlugin;
use wriftheart::{CANVAS_H, CANVAS_W};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest()) // pixel art: never smooth it
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "WriftHeart".into(),
                        resolution: (CANVAS_W * 3, CANVAS_H * 3).into(), // 3x — physical px
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(PixelCanvasPlugin)
        .add_plugins(wriftheart::settings::SettingsPlugin)
        .add_plugins(PlayPlugin)
        .add_plugins(BattlePlugin)
        .add_plugins(HudPlugin)
        .add_plugins(MenuPlugin)
        .add_plugins(CodexPlugin)
        .add_plugins(GatherPlugin)
        .add_plugins(FarmPlugin)
        .add_plugins(EncountersPlugin)
        .add_plugins(QuestsPlugin)
        .add_plugins(FlutePlugin)
        .add_plugins(wriftheart::app::LightingPlugin)
        .add_plugins(wriftheart::app::RewardsPlugin)
        .add_plugins(wriftheart::app::SavePlugin)
        .add_plugins(wriftheart::app::StatsPlugin)
        .add_plugins(wriftheart::app::TitlePlugin)
        .add_plugins(wriftheart::app::IdentityPlugin)
        .add_plugins(wriftheart::app::CreatorPlugin)
        .add_plugins(wriftheart::app::DeathPlugin)
        .add_plugins(wriftheart::app::InteriorPlugin)
        .add_plugins(wriftheart::app::BannersPlugin)
        .add_plugins(wriftheart::app::PromptsPlugin)
        .add_plugins(wriftheart::app::ShopPlugin)
        .add_plugins(wriftheart::app::StoragePlugin)
        .add_plugins(wriftheart::app::HomePlugin)
        .add_plugins(wriftheart::app::ServicesPlugin)
        .add_plugins(wriftheart::app::fanfare::FanfarePlugin)
        .add_plugins(wriftheart::app::TalkPlugin)
        .add_plugins(wriftheart::app::DialogPlugin)
        .add_plugins(wriftheart::app::DevPlugin)
        .add_plugins(wriftheart::app::DungeonPlugin)
        .add_plugins(wriftheart::app::BossPlugin)
        .add_plugins(wriftheart::app::GameAudioPlugin)
        .add_plugins(wriftheart::app::StatusPlugin)
        .add_plugins(wriftheart::app::FestivalsPlugin)
        .add_plugins(wriftheart::app::FarmAnimalsPlugin)
        .add_plugins(wriftheart::app::GuildhallPlugin)
        .add_plugins(wriftheart::app::cooking::CookingPlugin)
        .add_plugins(wriftheart::app::packup::PackupPlugin)
        .add_plugins(wriftheart::app::story::StoryPlugin)
        .add_plugins(wriftheart::app::riftspire::RiftSpirePlugin)
        .add_plugins(wriftheart::app::placing::PlacingPlugin)
        .add_plugins(wriftheart::app::caves::CavesPlugin)
        .add_plugins(wriftheart::app::digging::DiggingPlugin)
        .add_plugins(wriftheart::app::uniques::UniquesPlugin)
        .add_plugins(wriftheart::app::champions::ChampionsPlugin)
        .add_plugins(wriftheart::app::ogre::OgrePlugin)
        .add_plugins(wriftheart::app::saltmaze::SaltmazePlugin)
        .add_plugins(wriftheart::app::sidescroll::SideScrollPlugin)
        .add_plugins(wriftheart::app::darkknight::DarkKnightPlugin)
        .add_plugins(wriftheart::app::cinematic::CinematicPlugin)
        .add_plugins(wriftheart::app::archery::ArcheryPlugin)
        .add_plugins(wriftheart::app::shield::ShieldPlugin)
        .add_plugins(wriftheart::app::skystrike::SkyStrikePlugin)
        .add_plugins(wriftheart::app::mobfx::MobFxPlugin)
        .add_plugins(wriftheart::app::blueprints::BlueprintsPlugin)
        .add_plugins(wriftheart::app::wands::WandsPlugin)
        .add_plugins(wriftheart::app::traversal::TraversalPlugin)
        .add_plugins(wriftheart::app::lootgoblin::LootGoblinPlugin)
        .add_plugins(wriftheart::app::caravan::CaravanPlugin)
        .add_plugins(wriftheart::app::fire::FirePlugin)
        .add_plugins(wriftheart::app::FishingPlugin)
        .add_plugins(wriftheart::app::RoomCachePlugin)
        .add_plugins(wriftheart::actors::critters::CritterPlugin)
        .add_plugins(wriftheart::app::SfxPlugin)
        .add_plugins(wriftheart::gfx::shadow_material::ShadowMaterialPlugin)
        .add_plugins(wriftheart::gfx::water_material::WaterMaterialPlugin)
        .add_plugins(wriftheart::gfx::weather_fx_material::WeatherFxMaterialPlugin)
        .add_plugins(wriftheart::app::WeatherPlugin)
        .add_plugins(wriftheart::app::WaterPlugin)
        .add_plugins(wriftheart::app::ShadowsPlugin)
        .add_plugins(SlideOutPlugin)
        .add_plugins(DebugShotPlugin) // inert unless WRIFT_SHOT is set
        .run();
}
