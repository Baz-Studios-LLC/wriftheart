//! blueprints.rs — LEARNED BLUEPRINTS (js learnedBlueprints): using a `bp*` blueprint item
//! LEARNS it forever, unlocking its gated recipe (crafting the station at the workbench, or
//! the grapple/well). Saved. The gate itself lives in items::recipes_for.

use bevy::prelude::*;
use std::collections::HashSet;

/// Blueprint ids you've learned (js learnedBlueprints, saved).
#[derive(Resource, Default)]
pub struct LearnedBlueprints(pub HashSet<String>);

/// "Learn this blueprint" (a `bp*` item was used from a slot).
#[derive(Message)]
pub struct LearnBlueprint(pub &'static str);

fn learn_blueprint(
    mut msgs: MessageReader<LearnBlueprint>,
    mut learned: ResMut<LearnedBlueprints>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut saves: MessageWriter<super::save::SaveRequest>,
) {
    for LearnBlueprint(bp) in msgs.read() {
        if learned.0.contains(*bp) {
            sfx.write(super::sfx::Sfx("tink")); // already known — don't waste it
            continue;
        }
        learned.0.insert(bp.to_string());
        inv.remove_one(bp);
        let name = crate::items::get(bp).map(|d| d.name).unwrap_or(bp).to_uppercase();
        log.add("bp", &format!("LEARNED: {name}"), 1, 0x9ad0ff, false, true);
        sfx.write(super::sfx::Sfx("craft"));
        saves.write(super::save::SaveRequest);
    }
}

pub struct BlueprintsPlugin;

impl Plugin for BlueprintsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LearnedBlueprints>()
            .add_message::<LearnBlueprint>()
            .add_systems(bevy::app::FixedUpdate, learn_blueprint.run_if(super::screen::playing));
    }
}
