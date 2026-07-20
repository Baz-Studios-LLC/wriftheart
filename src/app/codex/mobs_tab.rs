//! mobs_tab.rs — the BESTIARY (js drawMobDex): grid of every mob kind, unlocked once
//! you've slain one (the js `bestiary` set — fed by battle.rs deaths), detail pane right.
//! The roster grows as biome mobs port.

use super::{dex, hint_scaffold, CodexState, CodexUi, TabContent, CONTENT_Z};
use crate::actors::goblin::GoblinArt;
use crate::actors::mobs::MobArtBank;
use crate::actors::mobs_art::BESTIARY_INFO;
use crate::input::{ActionState, Bindings};
use crate::ui::label;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

/// Mob kinds the player has slain at least once (js `bestiary`, saved with the game).
#[derive(Resource, Default)]
pub struct Bestiary(pub HashSet<&'static str>);

/// The dex roster: the goblins (bespoke art) + every ported biome mob (js BESTIARY text).
type Row = (&'static str, &'static str, &'static str, crate::actors::mobs::Baked);

fn roster(goblins: &GoblinArt, mobs: &MobArtBank) -> Vec<Row> {
    let mut out: Vec<Row> = vec![
        ("goblin", "GOBLIN", "Wretched raiders; some sling stones.", (goblins.0[0][0][0].clone(), 16.0, 16.0)),
        ("slinger", "SPEAR GOBLIN", "Keeps its distance and slings stones.", (goblins.0[1][0][0].clone(), 16.0, 16.0)),
    ];
    for (kind, name, desc) in BESTIARY_INFO {
        let art = if *kind == "wolf" { mobs.wolf[0][0].clone() } else { mobs.frames[kind][0][0].clone() };
        out.push((kind, name, desc, art));
    }
    out
}

#[derive(Resource, Default)]
pub struct MobDex {
    pub cur: usize,
}

#[derive(Component, Clone)]
pub struct MobsUi;

pub fn hint(bindings: &Bindings, pad: bool) -> String {
    let browse = if pad { "DPAD BROWSE" } else { "ARROWS BROWSE" };
    hint_scaffold(bindings, pad, browse)
}

/// The MOBS tab driver: dpad browses the grid, redraw on entry or cursor move.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn run(
    mut commands: Commands,
    state: Res<ActionState>,
    cx_state: Res<CodexState>,
    goblins: Res<GoblinArt>,
    mob_art: Res<MobArtBank>,
    bestiary: Res<Bestiary>,
    mut dex_state: ResMut<MobDex>,
    mut images: ResMut<Assets<Image>>,
    old: Query<Entity, With<MobsUi>>,
    mut seen_gen: Local<u32>,
) {
    let entries = roster(&goblins, &mob_art);
    let mut dirty = *seen_gen != cx_state.generation;
    *seen_gen = cx_state.generation;
    let cur = dex::dex_nav(&state, entries.len(), dex_state.cur, dex::DEX_COLS);
    if cur != dex_state.cur {
        dex_state.cur = cur;
        dirty = true;
    }
    if !dirty {
        return;
    }
    for e in &old {
        commands.entity(e).despawn();
    }
    let tag = || (CodexUi, TabContent, MobsUi);

    let slain = entries.iter().filter(|e| bestiary.0.contains(e.0)).count();
    let hdr = format!("BESTIARY  {slain}/{}", entries.len());
    label(&mut commands, &mut images, &hdr, dex::DEX_AX, 16.0, 0xbfb9a0, CONTENT_Z + 0.1, tag());

    dex::draw_grid(
        &mut commands,
        &mut images,
        entries.len(),
        dex_state.cur,
        dex::DEX_COLS,
        |i| bestiary.0.contains(entries[i].0),
        |i| Some((entries[i].3 .0.clone(), entries[i].3 .1.max(entries[i].3 .2))),
        tag(),
    );
    let e = &entries[dex_state.cur];
    dex::draw_pane(
        &mut commands,
        &mut images,
        bestiary.0.contains(e.0),
        Some((e.3 .0.clone(), e.3 .1.max(e.3 .2))),
        e.1,
        None,
        e.2,
        true,
        tag(),
    );
}
