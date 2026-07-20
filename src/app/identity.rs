//! identity.rs — WHO the hero is: the creator's record (name / gender / look / traits),
//! plus the day/night flag their quirk traits key on (js player.night). Saved per slot;
//! the creator writes it, the loader applies it, the HUD + traits read it.

use super::room_render::FrameClock;
use super::screen::playing;
use super::slideout::{skills_tab, TreeAlloc, TreeStats};
use crate::actors::hero::Look;
use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct HeroIdent {
    pub name: String,
    pub gender: String, // "M" | "F" — cosmetic flavor (dialogue later)
    pub look: Look,
    pub traits: Vec<String>, // trait keys (crate::traits)
}

impl Default for HeroIdent {
    fn default() -> Self {
        Self { name: "HERO".into(), gender: "M".into(), look: Look::default(), traits: vec![] }
    }
}

/// True after dusk (js: effectiveDarkness() > 0.5) — gates Night Owl-style quirks.
#[derive(Resource, Default, PartialEq)]
pub struct Night(pub bool);

pub struct IdentityPlugin;

impl Plugin for IdentityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroIdent>()
            .init_resource::<Night>()
            .add_systems(bevy::app::FixedUpdate, track_night.run_if(playing));
    }
}

/// Follow the day cycle and re-fold the derived stats when day flips to night (the js
/// recomputes Traits.stat live every query; one recompute per flip lands the same place).
fn track_night(
    clock: Res<FrameClock>,
    mut night: ResMut<Night>,
    ident: Res<HeroIdent>,
    alloc: Res<TreeAlloc>,
    inv: Res<crate::inventory::PlayerInv>,
    mut tstats: ResMut<TreeStats>,
) {
    let now = super::lighting::day_darkness(clock.0) > 0.5;
    if night.0 != now {
        night.0 = now;
        *tstats = skills_tab::recompute(&alloc, &ident.traits, now, &inv);
    }
}
