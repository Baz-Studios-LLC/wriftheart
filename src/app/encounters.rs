//! encounters.rs — hand-authored SET-PIECE encounters (port of js/encounters.js): ~10%
//! of eligible wild rooms deterministically host a curated scene — a goblin raid on an
//! overturned wagon, an ogre encampment, a dark ritual — instead of the natural mob
//! roll. Decor rebuilds identically on every visit; foes spawn fresh each day until the
//! room is CLEARED (roster wiped), after which it reverts to a peaceful room forever
//! (js clearedEncounters, saved).
//!
//! INC 1 (this file): the full ENCOUNTERS table (24 defs — determinism-complete, so ids
//! never shift), decor + foes + cleared tracking + save. NOT YET (flagged in PORT.md):
//! friendly WANDERER boons, the threat banner, campfire/crystal light.

use super::battle::RoomActor;
use super::play::CurRoom;
use super::room_render::{actor_z, child, FrameClock, PLAY_X, PLAY_Y};
use super::screen::playing;
use crate::actors::encounter_art as art;
use crate::actors::props::PropArt;
use crate::gfx::{at, bake, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};
use crate::worldgen::rng::hash;
use crate::worldgen::World;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

const SALT: u32 = 0x9e3d71b1;
/// ~10% of ELIGIBLE rooms host an encounter (js BASE_CHANCE).
pub const BASE_CHANCE: f64 = 0.10;
const CX: f32 = (PX_W / 2) as f32;
const CY: f32 = (PX_H / 2) as f32;

/// One authored encounter (js ENCOUNTERS row).
pub struct EncDef {
    pub id: &'static str,
    pub name: &'static str,
    pub biomes: Option<&'static [&'static str]>,
    pub min_tier: i32,
    pub max_tier: Option<i32>,
    pub weight: i32,
    /// Gate to seasons (indices into calendar SEASONS). Gated defs are EVENTS:
    /// they roam daily instead of holding a stable tenancy.
    pub seasons: Option<&'static [usize]>,
    /// Gate to the night (Some(true)) or the day (Some(false)). Also event-making.
    pub night: Option<bool>,
    /// Force EVENT-hood without a time gate: rolls fresh dice daily, any hour.
    pub roaming: bool,
    /// No foes — peaceful strangers take the room's slot (no threat, never "cleared").
    pub friendly: bool,
    pub place: fn(&mut Scene),
}

/// A staged scene: what `place` dropped, ready to spawn (js build()'s {decor, mobs}).
pub struct Scene {
    pub cx: f32,
    pub cy: f32,
    pub biome: &'static str,
    pub tier: i32,
    pub seed: u32,
    pub decor: Vec<Decor>,
    pub foes: Vec<(&'static str, f32, f32)>,
    /// Frightened civilians caught in the scene (spawn with the foes, fresh rooms only).
    pub victims: Vec<(f32, f32)>,
    /// Friendly strangers (decor-persistent): (x, y, role, title).
    pub wanderers: Vec<(f32, f32, &'static str, &'static str)>,
    /// Free pickings placed with the scene (item id, x, y) — each waits until
    /// taken, then stays gone for good (the ruined-village stones' rule).
    pub loot: Vec<(&'static str, f32, f32)>,
    /// Foes that spawn ASLEEP (sprawled, dreaming) — a hit or a close footstep
    /// wakes them. The slumbering-guardian scenes' whole gamble.
    pub sleepers: Vec<(&'static str, f32, f32)>,
}

pub struct Decor {
    pub kind: &'static str,
    pub x: f32,
    pub y: f32,
    pub color: u32, // banner cloth / crystal body recolour (0 = the kind's default)
}

impl Scene {
    fn clamp(x: f32, y: f32) -> (f32, f32) {
        (x.round().clamp(8.0, PX_W as f32 - 24.0), y.round().clamp(8.0, PX_H as f32 - 24.0))
    }
    fn put(&mut self, kind: &'static str, x: f32, y: f32, color: u32) {
        let (x, y) = Self::clamp(x, y);
        self.decor.push(Decor { kind, x, y, color });
    }
    pub fn foe(&mut self, kind: &'static str, x: f32, y: f32) {
        let (x, y) = Self::clamp(x, y);
        self.foes.push((kind, x, y));
    }
    pub fn victim(&mut self, x: f32, y: f32) {
        let (x, y) = Self::clamp(x, y);
        self.victims.push((x, y));
    }
    pub fn loot(&mut self, id: &'static str, x: f32, y: f32) {
        self.loot.push((id, x, y));
    }
    pub fn sleeper(&mut self, kind: &'static str, x: f32, y: f32) {
        self.sleepers.push((kind, x, y));
    }
    pub fn wanderer(&mut self, x: f32, y: f32, role: &'static str, title: &'static str) {
        let (x, y) = Self::clamp(x, y);
        self.wanderers.push((x, y, role, title));
    }
    pub fn campfire(&mut self, x: f32, y: f32) { self.put("campfire", x, y, 0) }
    pub fn corpse(&mut self, x: f32, y: f32) { self.put("corpse", x, y, 0) }
    pub fn blood(&mut self, x: f32, y: f32) { self.put("blood", x, y, 0) }
    pub fn wagon(&mut self, x: f32, y: f32) { self.put("wagon", x, y, 0) }
    pub fn ritual(&mut self, x: f32, y: f32) { self.put("ritual", x, y, 0) }
    pub fn bones(&mut self, x: f32, y: f32) { self.put("bones", x, y, 0) }
    pub fn crate_(&mut self, x: f32, y: f32) { self.put("crate", x, y, 0) }
    pub fn tent(&mut self, x: f32, y: f32) { self.put("tent", x, y, 0) }
    pub fn banner(&mut self, x: f32, y: f32, color: u32) { self.put("banner", x, y, color) }
    pub fn gold(&mut self, x: f32, y: f32) { self.put("gold", x, y, 0) }
    pub fn crystal(&mut self, x: f32, y: f32, color: u32) { self.put("crystal", x, y, color) }
    pub fn web(&mut self, x: f32, y: f32) { self.put("web", x, y, 0) }
    pub fn ice(&mut self, x: f32, y: f32) { self.put("ice", x, y, 0) }
    pub fn stake(&mut self, x: f32, y: f32) { self.put("stake", x, y, 0) }
    pub fn torch(&mut self, x: f32, y: f32) { self.put("torch", x, y, 0) }
    pub fn mushroom(&mut self, x: f32, y: f32) { self.put("mushroom", x, y, 0) }
    pub fn flower(&mut self, x: f32, y: f32) { self.put("flower", x, y, 0) }
    pub fn clutter(&mut self, sub: &'static str, x: f32, y: f32) { self.put(sub, x, y, u32::MAX) }
}

/// The authored table — order + weights are PARITY-LOAD-BEARING (the weighted pick walks
/// the eligible list in table order; adding an encounter appends, never reorders).
pub static ENCOUNTERS: &[EncDef] = &[
    EncDef { id: "goblinRaid", name: "GOBLIN RAID", biomes: None, min_tier: 0, max_tier: None, weight: 3, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.wagon(cx - 8.0, cy - 18.0); a.crate_(cx + 30.0, cy - 2.0); a.blood(cx - 22.0, cy + 22.0); a.bones(cx + 38.0, cy + 18.0);
            for (i, k) in ["goblin", "goblin", "slinger", "goblin", "slinger"].into_iter().enumerate() { a.foe(k, cx - 56.0 + i as f32 * 28.0, cy + 20.0 + (i % 2) as f32 * 12.0); }
            a.victim(cx + 44.0, cy - 8.0); a.victim(cx + 56.0, cy + 14.0); a.victim(cx + 34.0, cy + 32.0); } },
    EncDef { id: "banditAmbush", name: "BANDIT AMBUSH", biomes: None, min_tier: 1, max_tier: None, weight: 3, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.campfire(cx + 22.0, cy + 12.0); a.crate_(cx - 32.0, cy - 10.0); a.crate_(cx - 22.0, cy + 18.0); a.bones(cx + 6.0, cy - 28.0);
            for i in 0..4 { a.foe("bandit", cx - 44.0 + i as f32 * 30.0, cy - 22.0 + (i % 2) as f32 * 30.0); }
            a.victim(cx + 40.0, cy - 14.0); a.victim(cx + 48.0, cy + 4.0); } },
    EncDef { id: "ogreCamp", name: "OGRE ENCAMPMENT", biomes: None, min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.campfire(cx - 4.0, cy - 6.0);
            a.corpse(cx + 32.0, cy + 20.0); a.blood(cx + 34.0, cy + 24.0); a.corpse(cx - 40.0, cy - 6.0); a.blood(cx - 38.0, cy - 2.0);
            a.bones(cx - 30.0, cy + 22.0); a.bones(cx + 26.0, cy - 26.0); a.crate_(cx + 46.0, cy - 18.0); a.crate_(cx - 52.0, cy + 12.0);
            a.foe("ogre", cx - 36.0, cy - 24.0); a.foe("ogre", cx + 40.0, cy - 14.0); a.foe("ogre", cx + 4.0, cy + 32.0); } },
    EncDef { id: "evilRitual", name: "DARK RITUAL", biomes: None, min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.ritual(cx - 7.0, cy - 7.0); a.corpse(cx - 2.0, cy - 2.0); a.blood(cx, cy);
            a.bones(cx - 44.0, cy + 22.0); a.bones(cx + 32.0, cy - 28.0);
            a.foe("cultist", cx, cy - 36.0); a.foe("cultist", cx - 40.0, cy + 12.0); a.foe("cultist", cx + 40.0, cy + 12.0); } },
    EncDef { id: "wolfPack", name: "WOLF PACK", biomes: Some(&["forest", "grassland", "mountains"]), min_tier: 1, max_tier: None, weight: 3, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.corpse(cx - 4.0, cy + 4.0); a.blood(cx - 2.0, cy + 8.0); a.blood(cx + 8.0, cy + 2.0); a.bones(cx + 28.0, cy + 18.0);
            for i in 0..5 { let ang = (i as f32 / 5.0) * std::f32::consts::TAU; a.foe("wolf", cx + ang.cos() * 48.0, cy + ang.sin() * 36.0); } } },
    EncDef { id: "undeadVigil", name: "UNDEAD VIGIL", biomes: Some(&["graveyard"]), min_tier: 1, max_tier: None, weight: 3, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.bones(cx - 6.0, cy); a.bones(cx + 32.0, cy + 20.0); a.bones(cx - 36.0, cy - 16.0);
            a.foe("skeleton", cx - 42.0, cy - 10.0); a.foe("skeleton", cx + 40.0, cy - 6.0); a.foe("zombie", cx - 20.0, cy + 26.0); a.foe("zombie", cx + 18.0, cy + 28.0);
            if a.tier >= 3 { a.foe("revenant", cx, cy - 22.0); } } },
    EncDef { id: "frozenCamp", name: "FROZEN WARCAMP", biomes: Some(&["arctic"]), min_tier: 2, max_tier: None, weight: 3, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.campfire(cx, cy - 2.0); a.crate_(cx + 32.0, cy + 12.0); a.bones(cx - 30.0, cy + 18.0);
            a.foe("icetroll", cx - 32.0, cy - 16.0); a.foe("icetroll", cx + 34.0, cy - 12.0); a.foe("frostmite", cx - 14.0, cy + 24.0); a.foe("frostmite", cx + 16.0, cy + 26.0); } },
    EncDef { id: "sporeBloom", name: "SPORE BLOOM", biomes: Some(&["mushroom"]), min_tier: 2, max_tier: None, weight: 3, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.foe("sporemother", cx, cy - 6.0);
            for i in 0..4 { let ang = (i as f32 / 4.0) * std::f32::consts::TAU; a.foe("sporeling", cx + ang.cos() * 42.0, cy + ang.sin() * 30.0); }
            a.foe("myconid", cx - 46.0, cy + 14.0); } },
    EncDef { id: "warband", name: "WARBAND CAMP", biomes: None, min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.tent(cx - 40.0, cy - 18.0); a.tent(cx + 30.0, cy - 16.0);
            a.banner(cx - 12.0, cy - 26.0, 0xb01818); a.banner(cx + 14.0, cy - 26.0, 0x1a1a1a); a.campfire(cx, cy + 2.0);
            a.crate_(cx - 56.0, cy + 16.0); a.crate_(cx + 52.0, cy + 18.0); a.bones(cx + 4.0, cy + 30.0);
            a.foe("ogre", cx, cy - 18.0);
            a.foe("goblin", cx - 48.0, cy + 2.0); a.foe("goblin", cx + 46.0, cy + 4.0); a.foe("goblin", cx - 24.0, cy + 30.0);
            a.foe("slinger", cx + 26.0, cy + 32.0); a.foe("slinger", cx + 60.0, cy - 6.0); } },
    EncDef { id: "plunderedCaravan", name: "PLUNDERED CARAVAN", biomes: None, min_tier: 1, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.wagon(cx - 30.0, cy - 14.0); a.wagon(cx + 22.0, cy + 6.0); a.gold(cx, cy + 2.0);
            a.crate_(cx + 44.0, cy - 14.0); a.crate_(cx - 50.0, cy + 18.0); a.blood(cx - 8.0, cy + 18.0); a.corpse(cx - 14.0, cy + 16.0);
            a.foe("bandit", cx - 24.0, cy - 2.0); a.foe("bandit", cx + 8.0, cy - 8.0); a.foe("bandit", cx + 34.0, cy + 22.0);
            a.victim(cx + 54.0, cy + 6.0); a.victim(cx - 56.0, cy - 8.0); } },
    EncDef { id: "arcaneExperiment", name: "ARCANE EXPERIMENT", biomes: None, min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.ritual(cx - 7.0, cy + 4.0); a.crystal(cx - 34.0, cy - 16.0, 0x4a9cff); a.crystal(cx + 30.0, cy - 16.0, 0x4a9cff);
            a.corpse(cx + 30.0, cy + 22.0); a.blood(cx + 32.0, cy + 26.0); a.crate_(cx - 48.0, cy + 16.0);
            a.foe("cultist", cx - 30.0, cy - 4.0); a.foe("cultist", cx + 30.0, cy - 2.0); a.foe("ogre", cx, cy + 16.0); } },
    EncDef { id: "lastStand", name: "LAST STAND", biomes: None, min_tier: 1, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.campfire(cx, cy - 4.0); a.crate_(cx - 18.0, cy + 6.0); a.crate_(cx + 14.0, cy + 8.0);
            a.corpse(cx - 8.0, cy + 20.0); a.blood(cx - 6.0, cy + 24.0); a.bones(cx + 30.0, cy - 18.0);
            a.victim(cx - 4.0, cy); a.victim(cx + 8.0, cy + 2.0);
            for i in 0..5 { let ang = (i as f32 / 5.0) * std::f32::consts::TAU + 0.3; a.foe(if i % 2 == 1 { "bandit" } else { "goblin" }, cx + ang.cos() * 64.0, cy + ang.sin() * 44.0); } } },
    EncDef { id: "guardedHoard", name: "ANCIENT HOARD", biomes: Some(&["mountains", "desert"]), min_tier: 3, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.gold(cx - 10.0, cy); a.gold(cx + 8.0, cy + 4.0); a.gold(cx, cy - 12.0);
            a.crystal(cx, cy - 24.0, 0xfcd000); a.clutter("pillar", cx - 46.0, cy - 18.0); a.clutter("pillar", cx + 42.0, cy - 18.0); a.bones(cx - 30.0, cy + 22.0);
            a.foe("golem", cx - 30.0, cy + 4.0); a.foe("golem", cx + 30.0, cy + 4.0); } },
    EncDef { id: "spiderLair", name: "SPIDER LAIR", biomes: Some(&["forest", "swamp", "mushroom"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.web(cx - 40.0, cy - 20.0); a.web(cx + 24.0, cy - 16.0); a.web(cx - 8.0, cy + 16.0); a.web(cx + 36.0, cy + 12.0);
            a.corpse(cx, cy - 6.0); a.blood(cx + 2.0, cy - 2.0); a.bones(cx - 34.0, cy + 18.0);
            for i in 0..5 { let ang = (i as f32 / 5.0) * std::f32::consts::TAU; a.foe("spider", cx + ang.cos() * 50.0, cy + ang.sin() * 36.0); } } },
    EncDef { id: "barrowRising", name: "THE DEAD RISE", biomes: Some(&["graveyard"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("gravestone", cx - 30.0, cy - 14.0); a.clutter("gravestone", cx + 26.0, cy - 12.0); a.clutter("gravestone", cx, cy + 18.0);
            a.clutter("pillar", cx + 44.0, cy - 18.0); a.bones(cx - 10.0, cy + 2.0); a.bones(cx + 14.0, cy + 24.0);
            a.foe("revenant", cx, cy - 10.0); a.foe("skeleton", cx - 36.0, cy + 8.0); a.foe("skeleton", cx + 34.0, cy + 10.0);
            a.foe("zombie", cx - 18.0, cy + 28.0); a.foe("zombie", cx + 20.0, cy + 30.0); } },
    EncDef { id: "graveRobbers", name: "GRAVE ROBBERS", biomes: Some(&["graveyard"]), min_tier: 1, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("gravestone", cx - 24.0, cy - 10.0); a.clutter("gravestone", cx + 22.0, cy - 8.0);
            a.torch(cx - 4.0, cy - 18.0); a.crate_(cx + 30.0, cy + 14.0); a.bones(cx - 8.0, cy + 10.0); a.blood(cx + 10.0, cy + 16.0);
            a.foe("bandit", cx - 16.0, cy + 2.0); a.foe("bandit", cx + 12.0, cy + 4.0); a.foe("bandit", cx, cy + 24.0);
            a.foe("skeleton", cx - 40.0, cy - 6.0); a.foe("skeleton", cx + 38.0, cy + 22.0); } },
    EncDef { id: "moltenForge", name: "THE MOLTEN FORGE", biomes: Some(&["embermaw"]), min_tier: 3, max_tier: None, weight: 3, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.crystal(cx - 34.0, cy - 14.0, 0xfc4030); a.crystal(cx + 30.0, cy - 16.0, 0xfc7430);
            a.clutter("emberpile", cx - 4.0, cy + 18.0); a.clutter("lavarock", cx + 22.0, cy + 8.0); a.clutter("obsidianshard", cx - 30.0, cy + 14.0); a.bones(cx + 6.0, cy - 26.0);
            a.foe("charbrute", cx, cy + 2.0); a.foe("pyrewraith", cx + 2.0, cy - 18.0); a.foe("cinderhound", cx - 28.0, cy + 8.0); a.foe("cinderhound", cx + 26.0, cy - 4.0); } },
    EncDef { id: "infernalGate", name: "INFERNAL GATE", biomes: Some(&["burnt", "chaos", "embermaw"]), min_tier: 3, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.ritual(cx - 7.0, cy - 4.0); a.crystal(cx - 32.0, cy - 18.0, 0xfc4030); a.crystal(cx + 28.0, cy - 18.0, 0xfc4030);
            a.clutter("embers", cx - 18.0, cy + 18.0); a.clutter("embers", cx + 16.0, cy + 16.0); a.clutter("charredlog", cx + 36.0, cy + 8.0); a.corpse(cx, cy + 22.0);
            a.foe("cultist", cx - 28.0, cy); a.foe("cultist", cx + 26.0, cy + 2.0); a.foe("pyrewraith", cx - 6.0, cy - 18.0);
            let boss = if a.biome == "chaos" { "riftlord" } else { "charbrute" }; a.foe(boss, cx, cy + 6.0); } },
    EncDef { id: "frozenColossus", name: "FROZEN COLOSSUS", biomes: Some(&["arctic"]), min_tier: 3, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.ice(cx - 36.0, cy - 12.0); a.ice(cx + 34.0, cy - 12.0); a.ice(cx - 14.0, cy + 18.0); a.ice(cx + 18.0, cy + 20.0);
            a.bones(cx, cy + 6.0); a.crate_(cx + 48.0, cy + 14.0);
            a.foe("frostwyrm", cx, cy - 14.0); a.foe("icetroll", cx - 30.0, cy + 6.0); a.foe("icetroll", cx + 30.0, cy + 6.0); } },
    EncDef { id: "fungalNexus", name: "FUNGAL NEXUS", biomes: Some(&["mushroom"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.crystal(cx, cy - 8.0, 0xff5cae); a.mushroom(cx - 30.0, cy + 12.0); a.mushroom(cx + 26.0, cy + 10.0); a.mushroom(cx - 12.0, cy - 14.0);
            a.clutter("toadstool", cx + 14.0, cy - 16.0); a.flower(cx - 44.0, cy + 18.0);
            a.foe("sporemother", cx, cy + 8.0); a.foe("myconid", cx - 38.0, cy - 4.0); a.foe("myconid", cx + 36.0, cy - 2.0);
            for i in 0..3 { a.foe("sporeling", cx - 24.0 + i as f32 * 24.0, cy + 28.0); } } },
    EncDef { id: "scorchedConvoy", name: "SCORCHED CONVOY", biomes: Some(&["burnt"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.wagon(cx - 24.0, cy - 12.0); a.clutter("charredlog", cx + 20.0, cy + 4.0); a.clutter("ashpile", cx - 4.0, cy + 18.0);
            a.clutter("embers", cx + 6.0, cy - 8.0); a.corpse(cx - 12.0, cy + 14.0); a.blood(cx - 10.0, cy + 18.0); a.bones(cx + 36.0, cy + 16.0);
            a.foe("charbrute", cx + 6.0, cy - 6.0); a.foe("cinderhound", cx - 34.0, cy + 6.0); a.foe("cinderhound", cx + 34.0, cy + 8.0); a.foe("cinderhound", cx, cy + 28.0); } },
    EncDef { id: "lostTraveler", name: "A LOST TRAVELER", biomes: None, min_tier: 0, max_tier: Some(3), weight: 4, seasons: None, night: None, roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.crate_(cx + 22.0, cy + 8.0); a.clutter("pebble", cx - 26.0, cy + 14.0); a.flower(cx - 18.0, cy + 20.0);
            a.wanderer(cx, cy, "lost", "TRAVELER"); } },
    EncDef { id: "wanderingMinstrel", name: "A WANDERING MINSTREL", biomes: None, min_tier: 0, max_tier: Some(3), weight: 3, seasons: None, night: None, roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.campfire(cx + 4.0, cy + 18.0); a.clutter("twig", cx - 22.0, cy + 16.0); a.flower(cx + 30.0, cy + 6.0);
            a.wanderer(cx - 6.0, cy - 4.0, "minstrel", "MINSTREL"); } },
    EncDef { id: "hurtWayfarer", name: "AN INJURED WAYFARER", biomes: None, min_tier: 0, max_tier: Some(3), weight: 3, seasons: None, night: None, roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.campfire(cx + 22.0, cy + 14.0); a.clutter("pebble", cx - 28.0, cy + 8.0); a.bones(cx + 34.0, cy + 18.0);
            a.wanderer(cx - 4.0, cy + 2.0, "hurt", "WAYFARER"); } },
    EncDef { id: "wanderingHerbalist", name: "A WANDERING HERBALIST", biomes: None, min_tier: 0, max_tier: Some(3), weight: 3, seasons: None, night: None, roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.crate_(cx + 24.0, cy + 6.0); a.mushroom(cx - 26.0, cy + 16.0); a.flower(cx - 14.0, cy + 20.0); a.flower(cx + 12.0, cy + 22.0);
            a.wanderer(cx, cy - 2.0, "herbalist", "HERBALIST"); } },
    // --- EVENTS (Baz): gated to a season and/or the night. They ROAM — fresh
    // dice each day — visiting rooms that hold no standing camp, and vanish when
    // their window closes (night events die at dawn with the day's cache).
    EncDef { id: "restlessDead", name: "THE RESTLESS DEAD", biomes: None, min_tier: 1, max_tier: None, weight: 3, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.bones(cx - 8.0, cy + 2.0); a.bones(cx + 30.0, cy - 16.0); a.bones(cx - 36.0, cy + 20.0);
            a.foe("skeleton", cx - 38.0, cy - 8.0); a.foe("skeleton", cx + 36.0, cy - 4.0); a.foe("skeleton", cx, cy - 26.0);
            a.foe("zombie", cx - 16.0, cy + 24.0); a.foe("zombie", cx + 18.0, cy + 26.0); } },
    EncDef { id: "moonlitRite", name: "MOONLIT RITE", biomes: None, min_tier: 2, max_tier: None, weight: 2, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.ritual(cx - 7.0, cy - 7.0); a.torch(cx - 30.0, cy - 20.0); a.torch(cx + 26.0, cy - 20.0);
            a.foe("cultist", cx, cy - 34.0); a.foe("cultist", cx - 38.0, cy + 10.0); a.foe("cultist", cx + 38.0, cy + 10.0); a.foe("cultist", cx, cy + 30.0); } },
    EncDef { id: "nightHunt", name: "THE NIGHT HUNT", biomes: Some(&["forest", "grassland", "mountains"]), min_tier: 1, max_tier: None, weight: 3, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.corpse(cx, cy + 2.0); a.blood(cx + 4.0, cy + 6.0);
            for i in 0..6 { let ang = (i as f32 / 6.0) * std::f32::consts::TAU; a.foe("wolf", cx + ang.cos() * 52.0, cy + ang.sin() * 38.0); } } },
    EncDef { id: "nightfire", name: "A FIRE IN THE DARK", biomes: None, min_tier: 0, max_tier: Some(4), weight: 2, seasons: None, night: Some(true), roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.campfire(cx + 6.0, cy + 12.0); a.clutter("twig", cx - 20.0, cy + 18.0);
            a.wanderer(cx - 6.0, cy - 2.0, "minstrel", "MINSTREL"); } },
    EncDef { id: "springtide", name: "SPRINGTIDE SWARM", biomes: None, min_tier: 1, max_tier: None, weight: 3, seasons: Some(&[0]), night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.mushroom(cx - 24.0, cy + 10.0); a.mushroom(cx + 22.0, cy - 12.0); a.flower(cx - 8.0, cy - 20.0); a.flower(cx + 36.0, cy + 16.0);
            for i in 0..6 { let ang = (i as f32 / 6.0) * std::f32::consts::TAU + 0.5; a.foe("sporeling", cx + ang.cos() * 44.0, cy + ang.sin() * 32.0); } } },
    EncDef { id: "noonMuster", name: "NOON MUSTER", biomes: None, min_tier: 2, max_tier: None, weight: 3, seasons: Some(&[1]), night: Some(false), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.tent(cx - 36.0, cy - 16.0); a.banner(cx - 6.0, cy - 24.0, 0xb01818); a.crate_(cx + 40.0, cy + 10.0);
            for i in 0..5 { a.foe("bandit", cx - 48.0 + i as f32 * 24.0, cy + 14.0 + (i % 2) as f32 * 16.0); } } },
    EncDef { id: "harvestThieves", name: "HARVEST THIEVES", biomes: None, min_tier: 0, max_tier: None, weight: 3, seasons: Some(&[2]), night: Some(false), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.wagon(cx + 18.0, cy - 12.0); a.crate_(cx - 28.0, cy + 8.0); a.crate_(cx - 12.0, cy + 20.0);
            a.foe("goblin", cx - 40.0, cy - 8.0); a.foe("goblin", cx + 2.0, cy + 30.0); a.foe("goblin", cx + 44.0, cy + 8.0); a.foe("slinger", cx - 8.0, cy - 26.0);
            a.victim(cx + 50.0, cy - 18.0); } },
    EncDef { id: "creepingCold", name: "THE CREEPING COLD", biomes: None, min_tier: 1, max_tier: None, weight: 3, seasons: Some(&[3]), night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.ice(cx - 30.0, cy - 10.0); a.ice(cx + 28.0, cy - 8.0); a.ice(cx - 4.0, cy + 18.0);
            a.foe("frostmite", cx - 34.0, cy + 8.0); a.foe("frostmite", cx + 32.0, cy + 10.0); a.foe("frostmite", cx - 8.0, cy - 22.0); a.foe("frostmite", cx + 10.0, cy + 30.0); } },
    // --- BATCH 2 (Baz: "build a lot more out"): stable set-pieces first ---
    EncDef { id: "boarWallow", name: "BOAR WALLOW", biomes: Some(&["forest", "grassland"]), min_tier: 0, max_tier: None, weight: 3, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("pebble", cx - 20.0, cy + 12.0); a.flower(cx + 30.0, cy - 8.0); a.bones(cx + 8.0, cy + 22.0);
            for i in 0..4 { let ang = (i as f32 / 4.0) * std::f32::consts::TAU + 0.4; a.foe("boar", cx + ang.cos() * 38.0, cy + ang.sin() * 28.0); } } },
    EncDef { id: "bearDen", name: "THE BEARS DEN", biomes: Some(&["forest", "mountains"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.bones(cx - 8.0, cy + 4.0); a.bones(cx + 26.0, cy - 14.0); a.corpse(cx + 10.0, cy + 18.0); a.blood(cx + 12.0, cy + 22.0);
            a.foe("bear", cx - 20.0, cy - 8.0); a.foe("bear", cx + 24.0, cy + 6.0); } },
    EncDef { id: "paperNest", name: "THE PAPER NEST", biomes: Some(&["forest", "petalwood", "honeyglade"]), min_tier: 1, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.web(cx - 6.0, cy - 10.0); a.flower(cx - 30.0, cy + 14.0); a.flower(cx + 28.0, cy + 10.0);
            for i in 0..5 { let ang = (i as f32 / 5.0) * std::f32::consts::TAU; a.foe("wasp", cx + ang.cos() * 40.0, cy + ang.sin() * 30.0); } } },
    EncDef { id: "tollGate", name: "THE TOLL", biomes: None, min_tier: 1, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.banner(cx, cy - 24.0, 0x8a5a1a); a.crate_(cx - 26.0, cy + 6.0); a.crate_(cx + 22.0, cy + 8.0); a.torch(cx - 8.0, cy - 16.0);
            a.foe("bandit", cx - 30.0, cy - 6.0); a.foe("bandit", cx + 28.0, cy - 4.0); a.foe("archer", cx, cy + 24.0);
            a.victim(cx + 48.0, cy + 16.0); } },
    EncDef { id: "blackProcession", name: "THE BLACK PROCESSION", biomes: None, min_tier: 3, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.torch(cx - 24.0, cy - 18.0); a.torch(cx + 20.0, cy - 18.0); a.banner(cx - 2.0, cy - 26.0, 0x1a1a1a);
            a.foe("cultist", cx - 36.0, cy); a.foe("cultist", cx - 12.0, cy + 8.0); a.foe("cultist", cx + 12.0, cy + 8.0); a.foe("cultist", cx + 36.0, cy);
            a.foe("wraith", cx, cy - 16.0); } },
    EncDef { id: "sleepingSentinels", name: "SLEEPING SENTINELS", biomes: Some(&["desert", "mountains", "saltwastes"]), min_tier: 3, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("pillar", cx - 40.0, cy - 16.0); a.clutter("pillar", cx + 36.0, cy - 16.0); a.gold(cx, cy + 2.0);
            a.foe("golem", cx - 24.0, cy + 6.0); a.foe("golem", cx + 24.0, cy + 6.0); a.foe("golem", cx, cy - 20.0); } },
    EncDef { id: "boneOrchard", name: "THE BONE ORCHARD", biomes: Some(&["desert", "suncoast", "saltwastes"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); for (bx, by) in [(-30, -12), (26, -16), (-10, 6), (34, 12), (-38, 20)] { a.bones(cx + bx as f32, cy + by as f32); }
            a.foe("skeleton", cx - 24.0, cy - 4.0); a.foe("skeleton", cx + 20.0, cy + 2.0); a.foe("skeleton", cx, cy + 22.0);
            a.foe("vulture", cx - 44.0, cy - 20.0); a.foe("vulture", cx + 42.0, cy - 18.0); } },
    EncDef { id: "standingStones", name: "THE STANDING STONES", biomes: None, min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.crystal(cx, cy - 20.0, 0xd8d8e8);
            for i in 0..4 { let ang = (i as f32 / 4.0) * std::f32::consts::TAU + 0.78; a.foe("saltstatue", cx + ang.cos() * 42.0, cy + ang.sin() * 30.0); } } },
    EncDef { id: "slimeSpill", name: "SLIME SPILL", biomes: Some(&["swamp", "mushroom", "tarmire"]), min_tier: 1, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.mushroom(cx - 26.0, cy + 8.0); a.mushroom(cx + 24.0, cy - 10.0);
            for i in 0..5 { let ang = (i as f32 / 5.0) * std::f32::consts::TAU + 0.2; a.foe("slime", cx + ang.cos() * 36.0, cy + ang.sin() * 28.0); } } },
    EncDef { id: "cinderNest", name: "CINDER NEST", biomes: Some(&["embermaw", "burnt", "emberscar"]), min_tier: 3, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("embers", cx - 16.0, cy + 12.0); a.clutter("lavarock", cx + 20.0, cy + 6.0); a.bones(cx - 32.0, cy - 10.0);
            a.foe("cinderhound", cx - 28.0, cy - 2.0); a.foe("cinderhound", cx + 26.0, cy); a.foe("cinderhound", cx, cy + 24.0);
            a.foe("emberling", cx - 10.0, cy - 20.0); a.foe("emberling", cx + 12.0, cy - 18.0); } },
    EncDef { id: "voidLeak", name: "WHERE THE VOID LEAKS", biomes: Some(&["chaos", "wriftscar", "starhollow"]), min_tier: 4, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.crystal(cx, cy - 12.0, 0x8a5ae0); a.clutter("obsidianshard", cx - 30.0, cy + 10.0);
            a.foe("voidling", cx - 34.0, cy - 6.0); a.foe("voidling", cx + 32.0, cy - 4.0); a.foe("voidling", cx - 12.0, cy + 22.0); a.foe("voidling", cx + 14.0, cy + 24.0);
            a.foe("chaoswisp", cx - 20.0, cy - 24.0); a.foe("chaoswisp", cx + 22.0, cy - 22.0); } },
    EncDef { id: "coldCampsite", name: "A COLD CAMPSITE", biomes: None, min_tier: 1, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.tent(cx - 16.0, cy - 14.0); a.crate_(cx + 18.0, cy + 4.0); a.bones(cx + 34.0, cy + 16.0); a.gold(cx - 2.0, cy + 10.0);
            // Nobody home — whatever happened here, the pickings are free.
        } },
    EncDef { id: "tidePools", name: "THE TIDE POOLS", biomes: Some(&["suncoast"]), min_tier: 1, max_tier: None, weight: 3, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("pebble", cx - 22.0, cy + 10.0); a.clutter("scree", cx + 26.0, cy - 8.0); a.bones(cx + 6.0, cy + 20.0);
            a.foe("tidecrab", cx - 30.0, cy - 6.0); a.foe("tidecrab", cx + 28.0, cy - 2.0); a.foe("tidecrab", cx - 8.0, cy + 24.0); a.foe("tidecrab", cx + 12.0, cy - 22.0); } },
    EncDef { id: "honeyHollow", name: "THE HONEY HOLLOW", biomes: Some(&["honeyglade", "petalwood", "bluebell"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.flower(cx - 28.0, cy + 12.0); a.flower(cx + 24.0, cy + 8.0); a.flower(cx - 6.0, cy - 18.0); a.gold(cx + 2.0, cy + 2.0);
            for i in 0..4 { let ang = (i as f32 / 4.0) * std::f32::consts::TAU + 0.6; a.foe("honeydrone", cx + ang.cos() * 40.0, cy + ang.sin() * 30.0); } } },
    // --- BATCH 2: events (season / time-of-day gated, roaming) ---
    EncDef { id: "graveMarch", name: "THE GRAVE MARCH", biomes: None, min_tier: 2, max_tier: None, weight: 2, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.bones(cx - 20.0, cy + 8.0); a.bones(cx + 24.0, cy - 10.0);
            a.foe("gravewarden", cx, cy - 14.0); a.foe("skeleton", cx - 32.0, cy + 4.0); a.foe("skeleton", cx + 30.0, cy + 6.0); a.foe("ghoul", cx - 8.0, cy + 26.0); } },
    EncDef { id: "witchingHour", name: "THE WITCHING HOUR", biomes: None, min_tier: 3, max_tier: None, weight: 1, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.ritual(cx - 7.0, cy - 4.0); a.crystal(cx - 30.0, cy - 16.0, 0x8a5ae0); a.crystal(cx + 26.0, cy - 16.0, 0x8a5ae0);
            a.foe("cultist", cx - 28.0, cy + 6.0); a.foe("cultist", cx + 26.0, cy + 8.0); a.foe("cultist", cx, cy + 26.0);
            a.foe("wraith", cx - 12.0, cy - 20.0); a.foe("wraith", cx + 14.0, cy - 18.0); } },
    EncDef { id: "batCloud", name: "THE BAT CLOUD", biomes: None, min_tier: 0, max_tier: None, weight: 3, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy);
            for i in 0..6 { let ang = (i as f32 / 6.0) * std::f32::consts::TAU; a.foe("bat", cx + ang.cos() * 46.0, cy + ang.sin() * 34.0); } } },
    EncDef { id: "paleHowl", name: "THE PALE HOWL", biomes: Some(&["gloammoor", "graveyard", "forest", "witherlands"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.bones(cx + 10.0, cy + 14.0);
            a.foe("palehowler", cx - 26.0, cy - 6.0); a.foe("palehowler", cx + 24.0, cy - 2.0); a.foe("palehowler", cx, cy + 22.0); } },
    EncDef { id: "frostMoon", name: "THE FROST MOON", biomes: None, min_tier: 1, max_tier: None, weight: 2, seasons: Some(&[3]), night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.ice(cx - 24.0, cy - 8.0); a.ice(cx + 22.0, cy + 10.0);
            a.foe("icetroll", cx - 28.0, cy + 2.0); a.foe("icetroll", cx + 26.0, cy - 2.0); a.foe("frostmite", cx - 6.0, cy + 22.0); a.foe("frostmite", cx + 8.0, cy - 24.0); a.foe("frostmite", cx, cy) ; } },
    EncDef { id: "midsummerSwelter", name: "MIDSUMMER SWELTER", biomes: None, min_tier: 2, max_tier: None, weight: 2, seasons: Some(&[1]), night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("embers", cx - 14.0, cy + 12.0); a.clutter("embers", cx + 16.0, cy - 8.0);
            a.foe("emberling", cx - 26.0, cy - 4.0); a.foe("emberling", cx + 24.0, cy); a.foe("emberling", cx, cy + 20.0); a.foe("ashgeyser", cx, cy - 18.0); } },
    EncDef { id: "leanMonths", name: "THE LEAN MONTHS", biomes: None, min_tier: 1, max_tier: None, weight: 3, seasons: Some(&[2]), night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.corpse(cx + 4.0, cy + 4.0); a.blood(cx + 8.0, cy + 8.0);
            for i in 0..4 { let ang = (i as f32 / 4.0) * std::f32::consts::TAU + 0.3; a.foe("wolf", cx + ang.cos() * 44.0, cy + ang.sin() * 32.0); }
            a.foe("boar", cx, cy - 26.0); } },
    EncDef { id: "springGrowth", name: "GROWTH RUN WILD", biomes: None, min_tier: 1, max_tier: None, weight: 2, seasons: Some(&[0]), night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.flower(cx - 30.0, cy + 10.0); a.flower(cx + 28.0, cy + 6.0); a.flower(cx - 8.0, cy - 18.0); a.flower(cx + 10.0, cy + 24.0);
            a.foe("thornling", cx - 24.0, cy - 8.0); a.foe("thornling", cx + 22.0, cy - 4.0); a.foe("thornling", cx, cy + 18.0);
            a.foe("vinesnare", cx - 12.0, cy + 30.0); a.foe("vinesnare", cx + 34.0, cy + 16.0); } },
    EncDef { id: "starfall", name: "STARFALL", biomes: None, min_tier: 2, max_tier: None, weight: 1, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.crystal(cx, cy - 8.0, 0xfcd000); a.gold(cx - 12.0, cy + 8.0); a.gold(cx + 12.0, cy + 10.0);
            a.foe("glimmerling", cx - 24.0, cy - 4.0); a.foe("glimmerling", cx + 24.0, cy - 2.0); a.foe("prismshard", cx, cy + 20.0); } },
    EncDef { id: "pilgrimDawn", name: "A PILGRIM AT PRAYER", biomes: None, min_tier: 0, max_tier: Some(4), weight: 2, seasons: None, night: Some(false), roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("pillar", cx + 20.0, cy - 12.0); a.flower(cx - 18.0, cy + 12.0); a.flower(cx + 6.0, cy + 18.0);
            a.wanderer(cx - 4.0, cy, "pilgrim", "PILGRIM"); } },
    EncDef { id: "trappersFire", name: "THE TRAPPERS FIRE", biomes: None, min_tier: 1, max_tier: Some(4), weight: 2, seasons: None, night: Some(true), roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.campfire(cx + 8.0, cy + 12.0); a.crate_(cx - 22.0, cy + 6.0); a.bones(cx + 30.0, cy + 18.0);
            a.wanderer(cx - 6.0, cy - 2.0, "trapper", "TRAPPER"); } },
    // --- BATCH 3 (Baz: "the amount of encounters we build should be staggering" —
    // you should never know what the next room holds). HORDES first: -------------
    EncDef { id: "zombieHorde", name: "THE HUNGRY DEAD", biomes: None, min_tier: 2, max_tier: None, weight: 2, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.corpse(cx, cy); a.blood(cx + 4.0, cy + 4.0); a.bones(cx - 30.0, cy + 16.0);
            for i in 0..22 { let ang = (i as f32 / 22.0) * std::f32::consts::TAU; let r = 34.0 + ((i * 7) % 3) as f32 * 22.0;
                a.foe("zombie", cx + ang.cos() * r, cy + ang.sin() * r * 0.7); } } },
    EncDef { id: "marchingLegion", name: "THE MARCHING LEGION", biomes: Some(&["graveyard", "witherlands", "gloammoor"]), min_tier: 3, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.banner(cx, cy - 28.0, 0x2a2a34); a.bones(cx - 20.0, cy + 10.0); a.bones(cx + 24.0, cy + 14.0);
            for i in 0..14 { let col = (i % 7) as f32; let row = (i / 7) as f32; a.foe("skeleton", cx - 66.0 + col * 22.0, cy - 10.0 + row * 26.0); }
            for i in 0..4 { a.foe("archer", cx - 42.0 + i as f32 * 28.0, cy + 44.0); }
            a.foe("gravewarden", cx, cy - 14.0); } },
    EncDef { id: "gnatStorm", name: "THE GNAT STORM", biomes: Some(&["swamp", "tarmire", "greenmaw"]), min_tier: 1, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.mushroom(cx - 10.0, cy + 12.0);
            for i in 0..16 { let ang = (i as f32 / 16.0) * std::f32::consts::TAU; let r = 26.0 + ((i * 5) % 4) as f32 * 14.0;
                a.foe("gnat", cx + ang.cos() * r, cy + ang.sin() * r * 0.8); } } },
    EncDef { id: "screechingSky", name: "THE SCREECHING SKY", biomes: None, min_tier: 2, max_tier: None, weight: 2, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy);
            for i in 0..12 { let ang = (i as f32 / 12.0) * std::f32::consts::TAU; let r = 30.0 + ((i * 3) % 3) as f32 * 18.0;
                a.foe("bat", cx + ang.cos() * r, cy + ang.sin() * r * 0.75); } } },
    EncDef { id: "sporeTide", name: "THE SPORE TIDE", biomes: Some(&["mushroom"]), min_tier: 3, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.mushroom(cx - 30.0, cy + 8.0); a.mushroom(cx + 26.0, cy - 12.0); a.clutter("toadstool", cx + 6.0, cy + 20.0);
            a.foe("sporemother", cx - 18.0, cy - 8.0); a.foe("sporemother", cx + 20.0, cy + 4.0);
            for i in 0..15 { let ang = (i as f32 / 15.0) * std::f32::consts::TAU; let r = 30.0 + ((i * 11) % 3) as f32 * 16.0;
                a.foe("sporeling", cx + ang.cos() * r, cy + ang.sin() * r * 0.8); } } },
    EncDef { id: "slimeFlood", name: "THE SLIME FLOOD", biomes: Some(&["swamp", "tarmire", "greenmaw"]), min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy);
            for i in 0..10 { let ang = (i as f32 / 10.0) * std::f32::consts::TAU + 0.3; let r = 28.0 + ((i * 7) % 3) as f32 * 16.0;
                a.foe("slime", cx + ang.cos() * r, cy + ang.sin() * r * 0.8); }
            a.foe("toxicslime", cx - 10.0, cy - 6.0); a.foe("toxicslime", cx + 12.0, cy + 8.0); } },
    EncDef { id: "warhost", name: "THE WARHOST", biomes: None, min_tier: 3, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.tent(cx - 52.0, cy - 20.0); a.tent(cx + 44.0, cy - 18.0); a.tent(cx - 4.0, cy - 30.0);
            a.banner(cx - 24.0, cy - 26.0, 0xb01818); a.banner(cx + 20.0, cy - 26.0, 0x1a1a1a); a.campfire(cx, cy + 2.0); a.crate_(cx + 58.0, cy + 14.0);
            for i in 0..10 { let col = (i % 5) as f32; let row = (i / 5) as f32; a.foe("goblin", cx - 52.0 + col * 26.0, cy + 16.0 + row * 22.0); }
            for i in 0..5 { a.foe("slinger", cx - 44.0 + i as f32 * 22.0, cy - 8.0); }
            a.foe("ogre", cx - 16.0, cy - 18.0); a.foe("ogre", cx + 18.0, cy - 16.0); } },
    EncDef { id: "starvingPack", name: "THE STARVING PACK", biomes: None, min_tier: 2, max_tier: None, weight: 2, seasons: Some(&[3]), night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.bones(cx + 6.0, cy + 6.0); a.ice(cx - 26.0, cy - 10.0);
            for i in 0..8 { let ang = (i as f32 / 8.0) * std::f32::consts::TAU; a.foe("wolf", cx + ang.cos() * 46.0, cy + ang.sin() * 34.0); } } },
    EncDef { id: "emberTide", name: "THE EMBER TIDE", biomes: Some(&["embermaw", "emberscar"]), min_tier: 4, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("lavarock", cx - 20.0, cy + 10.0); a.clutter("embers", cx + 18.0, cy - 8.0);
            for i in 0..10 { let ang = (i as f32 / 10.0) * std::f32::consts::TAU; let r = 28.0 + ((i * 3) % 3) as f32 * 16.0;
                a.foe("emberling", cx + ang.cos() * r, cy + ang.sin() * r * 0.8); }
            a.foe("ashgeyser", cx - 12.0, cy - 4.0); a.foe("ashgeyser", cx + 14.0, cy + 6.0); } },
    EncDef { id: "molting", name: "THE MOLTING", biomes: Some(&["suncoast"]), min_tier: 1, max_tier: None, weight: 2, seasons: Some(&[0]), night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("pebble", cx - 18.0, cy + 8.0); a.clutter("scree", cx + 22.0, cy - 6.0);
            for i in 0..10 { let ang = (i as f32 / 10.0) * std::f32::consts::TAU + 0.5; let r = 26.0 + ((i * 7) % 3) as f32 * 16.0;
                a.foe("tidecrab", cx + ang.cos() * r, cy + ang.sin() * r * 0.8); } } },
    EncDef { id: "conclave", name: "THE CONCLAVE", biomes: None, min_tier: 4, max_tier: None, weight: 1, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.ritual(cx - 7.0, cy - 6.0); a.crystal(cx - 34.0, cy - 18.0, 0x8a5ae0); a.crystal(cx + 30.0, cy - 18.0, 0x8a5ae0); a.torch(cx - 4.0, cy - 26.0);
            for i in 0..8 { let ang = (i as f32 / 8.0) * std::f32::consts::TAU; a.foe("cultist", cx + ang.cos() * 44.0, cy + ang.sin() * 32.0); }
            a.foe("wraith", cx - 14.0, cy - 2.0); a.foe("wraith", cx + 16.0, cy) ; } },
    // --- LIFE IN THE WORLD: friendly vignettes -----------------------------------
    EncDef { id: "storyCircle", name: "TALES BY FIRELIGHT", biomes: None, min_tier: 0, max_tier: Some(4), weight: 2, seasons: None, night: Some(true), roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.campfire(cx, cy + 2.0); a.tent(cx - 40.0, cy - 18.0); a.tent(cx + 34.0, cy - 16.0); a.crate_(cx + 20.0, cy + 22.0);
            a.wanderer(cx - 16.0, cy - 8.0, "storyteller", "STORYTELLER"); a.wanderer(cx + 14.0, cy - 6.0, "minstrel", "MINSTREL");
            a.wanderer(cx - 22.0, cy + 12.0, "trapper", "TRAPPER"); a.wanderer(cx + 20.0, cy + 10.0, "lost", "TRAVELER");
            a.wanderer(cx - 2.0, cy + 18.0, "pilgrim", "PILGRIM"); } },
    EncDef { id: "pilgrimRoad", name: "THE PILGRIM ROAD", biomes: None, min_tier: 0, max_tier: Some(4), weight: 2, seasons: None, night: Some(false), roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.banner(cx + 24.0, cy - 20.0, 0xd8c060); a.flower(cx - 20.0, cy + 14.0);
            a.wanderer(cx - 18.0, cy - 4.0, "pilgrim", "PILGRIM"); a.wanderer(cx + 4.0, cy + 2.0, "pilgrim", "PILGRIM"); a.wanderer(cx + 24.0, cy + 8.0, "pilgrim", "PILGRIM"); } },
    EncDef { id: "huntersRest", name: "HUNTERS AT REST", biomes: Some(&["forest", "mountains", "grassland", "hollowwood"]), min_tier: 1, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.campfire(cx + 4.0, cy + 8.0); a.bones(cx + 28.0, cy + 16.0); a.crate_(cx - 24.0, cy + 4.0);
            a.wanderer(cx - 8.0, cy - 6.0, "trapper", "TRAPPER"); a.wanderer(cx + 18.0, cy - 2.0, "hurt", "WAYFARER"); } },
    EncDef { id: "countryWedding", name: "A COUNTRY WEDDING", biomes: Some(&["petalwood", "bluebell", "grassland", "honeyglade"]), min_tier: 0, max_tier: Some(3), weight: 1, seasons: None, night: Some(false), roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); for (fx, fy) in [(-34, -10), (30, -12), (-14, 18), (16, 20), (-2, -22), (38, 8)] { a.flower(cx + fx as f32, cy + fy as f32); }
            a.banner(cx - 20.0, cy - 24.0, 0xe89ac0); a.banner(cx + 16.0, cy - 24.0, 0xe8d060); a.gold(cx + 2.0, cy + 8.0);
            a.wanderer(cx - 10.0, cy - 4.0, "dancer", "BRIDE"); a.wanderer(cx + 8.0, cy - 4.0, "dancer", "GROOM");
            a.wanderer(cx - 26.0, cy + 8.0, "minstrel", "MINSTREL"); a.wanderer(cx + 26.0, cy + 10.0, "dancer", "DANCER"); } },
    EncDef { id: "mourning", name: "THE MOURNING", biomes: Some(&["graveyard"]), min_tier: 1, max_tier: None, weight: 1, seasons: None, night: Some(false), roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("gravestone", cx, cy - 10.0); a.flower(cx - 8.0, cy + 2.0); a.flower(cx + 8.0, cy + 2.0);
            a.wanderer(cx - 14.0, cy + 10.0, "mourner", "MOURNER"); a.wanderer(cx + 12.0, cy + 12.0, "mourner", "MOURNER"); } },
    EncDef { id: "foragers", name: "THE FORAGERS", biomes: Some(&["mushroom", "forest", "swamp"]), min_tier: 0, max_tier: Some(4), weight: 2, seasons: None, night: Some(false), roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.mushroom(cx - 22.0, cy + 10.0); a.clutter("toadstool", cx + 18.0, cy + 14.0); a.crate_(cx + 30.0, cy - 8.0);
            a.wanderer(cx - 6.0, cy - 4.0, "herbalist", "HERBALIST"); a.wanderer(cx + 14.0, cy + 4.0, "herbalist", "FORAGER"); } },
    EncDef { id: "stargazers", name: "THE STARGAZERS", biomes: None, min_tier: 0, max_tier: Some(4), weight: 1, seasons: None, night: Some(true), roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.campfire(cx - 18.0, cy + 12.0); a.crystal(cx + 14.0, cy - 8.0, 0xd8d8f0);
            a.wanderer(cx - 4.0, cy - 2.0, "stargazer", "STARGAZER"); a.wanderer(cx + 16.0, cy + 6.0, "stargazer", "SCHOLAR"); } },
    // --- CREEP AND WONDER: atmosphere, ambush, and the deep lands ----------------
    EncDef { id: "shallowGraves", name: "THE SHALLOW GRAVES", biomes: Some(&["graveyard", "witherlands"]), min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); for (bx, by) in [(-32, -8), (-6, -16), (22, -10), (-20, 14), (10, 18), (36, 10), (-42, 22), (28, 28)] { a.bones(cx + bx as f32, cy + by as f32); }
            a.foe("ghoul", cx - 18.0, cy); a.foe("ghoul", cx + 16.0, cy + 4.0); a.foe("ghoul", cx, cy + 24.0); } },
    EncDef { id: "abandonedRite", name: "THE ABANDONED RITE", biomes: None, min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.ritual(cx - 7.0, cy - 4.0); a.torch(cx - 26.0, cy - 16.0); a.torch(cx + 22.0, cy - 16.0);
            a.bones(cx + 6.0, cy + 14.0); a.blood(cx - 4.0, cy + 2.0);
            // Cold candles, a stain, and nobody to ask. Whatever was called here, came.
        } },
    EncDef { id: "frozenCaravan", name: "THE FROZEN CARAVAN", biomes: Some(&["arctic"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.wagon(cx - 20.0, cy - 12.0); a.crate_(cx + 22.0, cy + 4.0); a.ice(cx - 38.0, cy + 10.0); a.ice(cx + 8.0, cy - 22.0); a.corpse(cx - 2.0, cy + 10.0);
            a.foe("frostmite", cx - 28.0, cy + 2.0); a.foe("frostmite", cx + 30.0, cy - 4.0); a.foe("frostmite", cx + 6.0, cy + 24.0); } },
    EncDef { id: "larder", name: "THE LARDER", biomes: Some(&["forest", "swamp", "hollowwood"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); for (wx, wy) in [(-36, -14), (-8, -20), (24, -12), (-24, 10), (8, 16), (34, 8)] { a.web(cx + wx as f32, cy + wy as f32); }
            a.corpse(cx - 4.0, cy - 2.0); a.corpse(cx + 18.0, cy + 6.0);
            a.foe("spider", cx - 26.0, cy - 4.0); a.foe("spider", cx + 28.0, cy); a.foe("spider", cx + 2.0, cy + 22.0); } },
    EncDef { id: "witheredGarden", name: "THE WITHERED GARDEN", biomes: Some(&["witherlands", "gloammoor"]), min_tier: 3, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.flower(cx - 26.0, cy + 8.0); a.flower(cx + 22.0, cy + 12.0); a.flower(cx - 4.0, cy - 16.0);
            a.foe("witherheart", cx - 14.0, cy - 4.0); a.foe("witherheart", cx + 16.0, cy + 2.0);
            a.foe("thornling", cx - 32.0, cy + 18.0); a.foe("thornling", cx + 30.0, cy - 12.0); a.foe("thornling", cx, cy + 26.0); } },
    EncDef { id: "sinkingSand", name: "THE SINKING SAND", biomes: Some(&["desert", "saltwastes"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.bones(cx - 22.0, cy + 6.0); a.bones(cx + 18.0, cy - 8.0); a.bones(cx + 2.0, cy + 20.0);
            a.foe("sandmaw", cx - 14.0, cy); a.foe("sandmaw", cx + 16.0, cy + 6.0); a.foe("burrower", cx - 30.0, cy - 10.0); a.foe("burrower", cx + 32.0, cy + 14.0); } },
    EncDef { id: "stormNest", name: "THE STORM NEST", biomes: Some(&["stormreach", "galewind"]), min_tier: 3, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.crystal(cx, cy - 14.0, 0x78c8ff); a.clutter("scree", cx - 24.0, cy + 10.0);
            a.foe("stormcaller", cx - 22.0, cy - 4.0); a.foe("stormcaller", cx + 22.0, cy); a.foe("stormcaller", cx, cy + 20.0);
            a.foe("sparkslime", cx - 10.0, cy + 28.0); a.foe("sparkslime", cx + 34.0, cy + 12.0); a.foe("sparkslime", cx - 36.0, cy + 14.0); } },
    EncDef { id: "deepBreach", name: "THE DEEP BREACH", biomes: Some(&["blackdeep", "mountains"]), min_tier: 3, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.crystal(cx, cy - 10.0, 0x4a9cff); a.clutter("obsidianshard", cx + 24.0, cy + 8.0); a.bones(cx - 26.0, cy + 12.0);
            a.foe("deepcrawler", cx - 28.0, cy - 6.0); a.foe("deepcrawler", cx + 26.0, cy - 2.0); a.foe("deepcrawler", cx - 8.0, cy + 22.0); a.foe("deepcrawler", cx + 12.0, cy - 24.0);
            a.foe("lurker", cx - 40.0, cy + 4.0); a.foe("lurker", cx + 40.0, cy + 10.0); } },
    EncDef { id: "shiftingShades", name: "THE SHIFTING SHADES", biomes: Some(&["gloammoor", "chaos", "starhollow"]), min_tier: 4, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.crystal(cx, cy - 8.0, 0x8a5ae0);
            a.foe("switchshade", cx - 26.0, cy - 4.0); a.foe("switchshade", cx + 24.0, cy); a.foe("switchshade", cx, cy + 20.0);
            a.foe("wraith", cx - 12.0, cy - 22.0); a.foe("wraith", cx + 14.0, cy - 20.0); } },
    EncDef { id: "leechPool", name: "THE LEECH POOL", biomes: Some(&["swamp", "tarmire"]), min_tier: 1, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.blood(cx - 8.0, cy + 4.0); a.blood(cx + 10.0, cy + 8.0); a.corpse(cx, cy);
            for i in 0..5 { let ang = (i as f32 / 5.0) * std::f32::consts::TAU; a.foe("leech", cx + ang.cos() * 30.0, cy + ang.sin() * 24.0); } } },
    EncDef { id: "underEveryStone", name: "UNDER EVERY STONE", biomes: Some(&["desert", "mountains", "saltwastes"]), min_tier: 1, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("pebble", cx - 24.0, cy + 8.0); a.clutter("scree", cx + 20.0, cy - 6.0); a.clutter("scree", cx - 4.0, cy + 18.0);
            for i in 0..5 { let ang = (i as f32 / 5.0) * std::f32::consts::TAU + 0.4; a.foe("scorpion", cx + ang.cos() * 36.0, cy + ang.sin() * 28.0); } } },
    EncDef { id: "rockslingers", name: "THE ROCKSLINGERS", biomes: Some(&["mountains"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.clutter("scree", cx - 28.0, cy + 10.0); a.clutter("pebble", cx + 24.0, cy + 14.0);
            a.foe("hurler", cx - 34.0, cy - 8.0); a.foe("hurler", cx + 32.0, cy - 6.0); a.foe("hurler", cx - 8.0, cy - 24.0); a.foe("hurler", cx + 10.0, cy + 24.0); } },
    EncDef { id: "glassGarden", name: "THE GLASS GARDEN", biomes: Some(&["prismwastes"]), min_tier: 3, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.crystal(cx - 28.0, cy - 10.0, 0xb060f0); a.crystal(cx + 24.0, cy - 12.0, 0x78c8ff); a.crystal(cx - 2.0, cy + 16.0, 0xff5cae);
            a.foe("prismshard", cx - 20.0, cy + 4.0); a.foe("prismshard", cx + 18.0, cy + 6.0); a.foe("prismshard", cx, cy - 20.0); a.foe("prismshard", cx + 38.0, cy + 18.0);
            a.foe("glimmerling", cx - 38.0, cy + 14.0); a.foe("glimmerling", cx + 6.0, cy + 30.0); } },
    EncDef { id: "beckoningLights", name: "THE BECKONING LIGHTS", biomes: Some(&["swamp", "tarmire", "gloammoor"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.mushroom(cx + 14.0, cy + 12.0);
            a.foe("boglight", cx - 26.0, cy - 8.0); a.foe("boglight", cx + 24.0, cy - 4.0); a.foe("boglight", cx - 6.0, cy + 18.0); a.foe("boglight", cx + 36.0, cy + 10.0);
            a.foe("mirefly", cx - 36.0, cy + 6.0); a.foe("mirefly", cx + 8.0, cy - 22.0); a.foe("mirefly", cx - 14.0, cy + 30.0); a.foe("mirefly", cx + 44.0, cy - 10.0); } },
    EncDef { id: "bellToll", name: "WHERE THE BELLS TOLL", biomes: Some(&["bluebell", "hollowwood"]), min_tier: 1, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.flower(cx - 22.0, cy + 8.0); a.flower(cx + 18.0, cy + 12.0); a.flower(cx - 2.0, cy - 14.0);
            a.foe("bellsnail", cx - 18.0, cy - 2.0); a.foe("bellsnail", cx + 16.0, cy + 2.0); a.foe("bellsnail", cx, cy + 20.0); } },
    EncDef { id: "ghoulFeast", name: "THE FEAST", biomes: None, min_tier: 2, max_tier: None, weight: 2, seasons: None, night: Some(true), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.corpse(cx - 12.0, cy); a.corpse(cx + 10.0, cy + 6.0); a.corpse(cx - 2.0, cy + 16.0); a.blood(cx, cy + 8.0); a.blood(cx - 14.0, cy + 4.0);
            a.foe("ghoul", cx - 24.0, cy - 8.0); a.foe("ghoul", cx + 22.0, cy - 4.0); a.foe("ghoul", cx - 6.0, cy + 28.0); a.foe("ghoul", cx + 30.0, cy + 16.0); } },
    EncDef { id: "vultureWake", name: "THE WAKE", biomes: Some(&["desert", "suncoast", "saltwastes"]), min_tier: 1, max_tier: None, weight: 2, seasons: None, night: Some(false), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.corpse(cx, cy + 2.0); a.bones(cx - 20.0, cy + 12.0); a.bones(cx + 22.0, cy + 8.0);
            for i in 0..6 { let ang = (i as f32 / 6.0) * std::f32::consts::TAU; a.foe("vulture", cx + ang.cos() * 40.0, cy + ang.sin() * 28.0); } } },
    EncDef { id: "woundedBeast", name: "A WOUNDED BEAST", biomes: Some(&["forest", "mountains", "hollowwood"]), min_tier: 2, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.blood(cx - 2.0, cy + 2.0); a.blood(cx + 8.0, cy - 4.0);
            a.foe("bear", cx, cy - 6.0);
            a.foe("wolf", cx - 34.0, cy - 10.0); a.foe("wolf", cx + 32.0, cy - 8.0); a.foe("wolf", cx - 4.0, cy + 26.0); } },
    EncDef { id: "tooQuiet", name: "TOO QUIET", biomes: None, min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.campfire(cx, cy); a.gold(cx + 10.0, cy + 8.0); a.crate_(cx - 18.0, cy + 4.0);
            // A warm fire, coin in the open, and no one in sight. Of course.
            a.foe("bandit", cx - 52.0, cy - 22.0); a.foe("bandit", cx + 48.0, cy - 20.0); a.foe("bandit", cx - 44.0, cy + 30.0); a.foe("bandit", cx + 42.0, cy + 32.0);
            a.foe("lurker", cx + 2.0, cy - 30.0); } },
    EncDef { id: "runawayCart", name: "THE RUNAWAY CART", biomes: Some(&["grassland", "forest"]), min_tier: 1, max_tier: None, weight: 2, seasons: None, night: Some(false), roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.wagon(cx - 8.0, cy - 10.0); a.crate_(cx - 30.0, cy + 8.0); a.crate_(cx + 22.0, cy + 4.0); a.crate_(cx + 6.0, cy + 20.0); a.gold(cx - 12.0, cy + 14.0);
            a.foe("goblin", cx - 24.0, cy - 4.0); a.foe("goblin", cx + 20.0, cy - 6.0); a.foe("goblin", cx + 2.0, cy + 30.0);
            a.victim(cx + 44.0, cy - 14.0); } },
    EncDef { id: "greenMaw", name: "THE GREEN MAW", biomes: Some(&["greenmaw"]), min_tier: 4, max_tier: None, weight: 2, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.mushroom(cx - 24.0, cy + 10.0); a.flower(cx + 20.0, cy + 14.0); a.flower(cx - 6.0, cy - 16.0);
            a.foe("vinesnare", cx - 28.0, cy - 6.0); a.foe("vinesnare", cx + 26.0, cy - 2.0); a.foe("vinesnare", cx - 8.0, cy + 24.0); a.foe("vinesnare", cx + 12.0, cy - 26.0);
            a.foe("thornling", cx - 40.0, cy + 12.0); a.foe("thornling", cx + 38.0, cy + 8.0); a.foe("thornling", cx + 2.0, cy + 38.0); a.foe("thornling", cx - 16.0, cy - 34.0); } },
    EncDef { id: "banditCourt", name: "THE BANDIT COURT", biomes: None, min_tier: 4, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.tent(cx - 48.0, cy - 20.0); a.tent(cx + 40.0, cy - 18.0); a.tent(cx - 4.0, cy - 32.0);
            a.banner(cx - 22.0, cy - 28.0, 0x8a5a1a); a.banner(cx + 18.0, cy - 28.0, 0x8a5a1a); a.campfire(cx, cy); a.gold(cx - 12.0, cy + 10.0); a.gold(cx + 10.0, cy + 12.0); a.gold(cx, cy + 20.0);
            for i in 0..8 { let ang = (i as f32 / 8.0) * std::f32::consts::TAU + 0.2; a.foe("bandit", cx + ang.cos() * 48.0, cy + ang.sin() * 34.0); }
            a.foe("archer", cx - 24.0, cy - 14.0); a.foe("archer", cx + 22.0, cy - 12.0);
            a.foe("ogre", cx, cy - 18.0); } },
    // --- REWARD SCENES (Baz): encounters that HAND you something ------------------
    EncDef { id: "lastKnight", name: "THE LAST KNIGHT", biomes: None, min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: true, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.banner(cx + 22.0, cy - 20.0, 0x4a5a8a); a.corpse(cx - 26.0, cy + 8.0); a.corpse(cx + 30.0, cy + 14.0);
            a.blood(cx - 2.0, cy + 6.0); a.blood(cx - 22.0, cy + 12.0); a.bones(cx + 8.0, cy + 24.0);
            a.wanderer(cx - 4.0, cy, "knight", "DYING KNIGHT"); } },
    EncDef { id: "crater", name: "THE CRATER", biomes: None, min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy);
            // The star itself: a scorched ember heart ringed in thrown stone.
            a.crystal(cx, cy - 6.0, 0xff8040);
            a.clutter("obsidianshard", cx - 26.0, cy + 6.0); a.clutter("obsidianshard", cx + 24.0, cy + 8.0);
            a.clutter("embers", cx - 12.0, cy + 16.0); a.clutter("embers", cx + 10.0, cy - 18.0);
            a.clutter("scree", cx - 38.0, cy - 8.0); a.clutter("scree", cx + 36.0, cy - 10.0); a.clutter("pebble", cx + 2.0, cy + 28.0);
            // Star-metal, scattered where it fell — richer the deeper the land.
            // a.tier is THREAT (ring/4, uncapped) — thresholds sit on zone-equivalents:
            // voidsteel ~zone 5+, mithril ~zone 4, gold ~zone 3 (purple is endgame).
            let ore: &'static str = match a.tier { t if t >= 9 => "voidsteel", t if t >= 7 => "mithril", t if t >= 5 => "gold", _ => "silver" };
            for (ox, oy) in [(-18, -4), (16, -8), (-6, 12), (22, 18), (-30, 16), (8, -22), (34, 2), (-40, -14)] {
                a.loot(ore, cx + ox as f32, cy + oy as f32);
            }
            a.loot("iron", cx - 14.0, cy + 24.0); a.loot("iron", cx + 40.0, cy - 18.0); } },
    EncDef { id: "theAlpha", name: "THE ALPHA", biomes: Some(&["forest", "grassland", "mountains", "hollowwood"]), min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.corpse(cx + 8.0, cy + 10.0); a.blood(cx + 12.0, cy + 14.0); a.bones(cx - 26.0, cy + 18.0);
            a.foe("alphawolf", cx - 4.0, cy - 8.0);
            a.foe("wolf", cx - 40.0, cy + 6.0); a.foe("wolf", cx + 38.0, cy + 8.0); } },
    EncDef { id: "slumberingBeast", name: "THE SLUMBERING BEAST", biomes: Some(&["forest", "mountains", "hollowwood"]), min_tier: 2, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.gold(cx - 10.0, cy + 8.0); a.gold(cx + 10.0, cy + 10.0); a.gold(cx, cy + 18.0); a.bones(cx - 28.0, cy - 8.0); a.bones(cx + 26.0, cy - 10.0);
            // Take the gold quietly... or take one step too close.
            a.sleeper("bear", cx - 6.0, cy - 12.0); } },
    EncDef { id: "slumberingTroll", name: "THE SLUMBERING TROLL", biomes: Some(&["arctic", "blackdeep"]), min_tier: 3, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: false,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.ice(cx - 28.0, cy - 10.0); a.ice(cx + 26.0, cy - 8.0); a.ice(cx + 2.0, cy - 24.0);
            a.gold(cx - 8.0, cy + 10.0); a.gold(cx + 10.0, cy + 12.0); a.crate_(cx + 32.0, cy + 16.0);
            a.sleeper("icetroll", cx - 6.0, cy - 8.0); a.sleeper("icetroll", cx + 18.0, cy - 2.0); } },
    EncDef { id: "falseMerchant", name: "A ROADSIDE STALL", biomes: None, min_tier: 1, max_tier: None, weight: 2, seasons: None, night: None, roaming: true, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.put("stall", cx - 13.0, cy - 22.0, 0); a.crate_(cx + 24.0, cy + 6.0);
            a.wanderer(cx - 2.0, cy + 2.0, "merchant", "MERCHANT"); } },
    EncDef { id: "whisperingWell", name: "THE WHISPERING WELL", biomes: None, min_tier: 1, max_tier: None, weight: 1, seasons: None, night: None, roaming: false, friendly: true,
        place: |a| { let (cx, cy) = (a.cx, a.cy); a.put("oldwell", cx - 10.0, cy - 14.0, 0); a.flower(cx - 24.0, cy + 12.0); a.clutter("pebble", cx + 20.0, cy + 10.0);
            // Stand at its mouth with a coin. Listen.
        } },
];

/// Camps the hero has wiped, and WHEN (room -> (day of last clear, clears so
/// far)) — saved. Beaten ground lies FALLOW for half a season, then the world
/// moves on: the room re-rolls a fresh tenancy with new dice — maybe a different
/// camp, maybe quiet wilderness (Baz retired the js "cleared forever": rooms
/// should feel alive, like things keep happening out there).
#[derive(Resource, Default)]
pub struct ClearedEncounters(pub bevy::platform::collections::HashMap<(i32, i32), (i64, u32)>);

/// How long beaten ground stays quiet before the world moves on (half a season).
pub const FALLOW_DAYS: i64 = 14;

impl ClearedEncounters {
    /// Still quiet from the last clear?
    pub fn fallow(&self, room: (i32, i32), today: i64) -> bool {
        matches!(self.0.get(&room), Some((day, _)) if today < day + FALLOW_DAYS)
    }
    /// The room's tenancy index (0 = the original scene, then 1, 2, ... as camps
    /// come and go), or None while the ground lies fallow.
    pub fn tenancy(&self, room: (i32, i32), today: i64) -> Option<u32> {
        match self.0.get(&room) {
            None => Some(0),
            Some((day, n)) => (today >= *day + FALLOW_DAYS).then_some(*n),
        }
    }
    /// Bank a wipe: quiet from today; the NEXT tenancy rolls new dice.
    pub fn record(&mut self, room: (i32, i32), today: i64) {
        let n = self.0.get(&room).map_or(1, |(_, n)| n + 1);
        self.0.insert(room, (today, n));
    }
    /// Save round-trip. Legacy saves stored bare rooms ("cleared forever") —
    /// they thaw FALLOW_DAYS after the day they load.
    pub fn from_save(led: &[(i32, i32, i64, u32)], legacy: &[(i32, i32)], today: i64) -> Self {
        let mut m = bevy::platform::collections::HashMap::default();
        if led.is_empty() {
            for &(x, y) in legacy {
                m.insert((x, y), (today, 1));
            }
        } else {
            for &(x, y, d, n) in led {
                m.insert((x, y), (d, n));
            }
        }
        Self(m)
    }
}

/// Armed when encounter foes spawn in the current room; the clear watcher retires it.
#[derive(Resource, Default)]
pub struct ArmedEncounter(pub Option<(i32, i32)>);

/// Marks an encounter's foes so the clear watcher can count the survivors.
#[derive(Component)]
pub struct EncFoe;

/// A campfire's two flicker frames (js: swap on (clock >> 3) & 1).
#[derive(Component)]
pub struct Campfire {
    pub frames: [Handle<Image>; 2],
}

/// Encounters stage across the room's middle blind to terrain — the host room must be
/// (almost) dry: tolerate a corner pond (<= ~8% of interior tiles), reject lakes.
fn dry_enough(world: &World, rx: i32, ry: i32) -> bool {
    let (c, r) = (crate::room::COLS, crate::room::ROWS);
    let mut wet = 0;
    for row in 1..r - 1 {
        for col in 1..c - 1 {
            if world.is_water(rx * c + col, ry * r + row) {
                wet += 1;
            }
        }
    }
    wet as f64 <= ((c - 2) * (r - 2)) as f64 * 0.08
}

/// Which encounter (if any) owns this room — deterministic from seed + coords
/// (js forRoom). None on shard grounds, towns, dry-fail, or the 90% quiet rooms.
/// The moment's face: the day index, the season, and whether it's night —
/// everything a time-gated event needs. Build one with [`Now::at`].
#[derive(Clone, Copy)]
pub struct Now {
    pub today: i64,
    pub season: usize,
    pub night: bool,
}

impl Now {
    pub fn at(clock: i64) -> Self {
        Now {
            today: super::gather::farm_day(clock),
            season: super::codex::calendar_tab::season_index(clock),
            night: super::lighting::day_darkness(clock) > 0.6, // evening through pre-dawn
        }
    }
}

/// The room/tier/biome eligibility every roll shares.
fn fits(d: &EncDef, biome: &'static str, tier: i32) -> bool {
    d.min_tier <= tier && d.max_tier.is_none_or(|m| tier <= m) && d.biomes.is_none_or(|bs| bs.contains(&biome))
}

/// The shared weighted pick off a roll's hash.
fn weighted(list: &[&'static EncDef], h: u32) -> &'static EncDef {
    let total: i32 = list.iter().map(|d| d.weight).sum();
    let mut r = (((h >> 10) % 100000) as f64 / 100000.0) * total as f64;
    for d in list {
        r -= d.weight as f64;
        if r < 0.0 {
            return d;
        }
    }
    list[0]
}

pub fn for_room(world: &World, rx: i32, ry: i32, cycle: u32) -> Option<(&'static EncDef, u32)> {
    if world.shard_dungeon_at(rx, ry).is_some() || world.is_town(rx, ry) {
        return None;
    }
    let tier = World::threat_tier(rx, ry);
    let biome = world.biome_key_at(rx, ry);
    // STABLE tenancies only — gated defs are roaming events (event_at's business).
    let list: Vec<&'static EncDef> = ENCOUNTERS
        .iter()
        .filter(|d| d.seasons.is_none() && d.night.is_none() && !d.roaming && fits(d, biome, tier))
        .collect();
    if list.is_empty() {
        return None;
    }
    // Each tenancy salts the dice — the camp that moves in later may differ.
    let h = hash(world.seed ^ cycle.wrapping_mul(0x9E37_79B9), rx, ry, SALT);
    if (h % 1000) as f64 / 1000.0 >= BASE_CHANCE {
        return None;
    }
    if !dry_enough(world, rx, ry) {
        return None;
    }
    Some((weighted(&list, h), h))
}

const SALT_EVENT: u32 = 0x51e7_a3d9;
/// Events visit ~7% of quiet eligible rooms while their window is open.
pub const EVENT_CHANCE: f64 = 0.07;

/// A roaming EVENT for this room right now: season/night-gated defs roll fresh
/// dice each day, so they wander the world instead of holding ground.
fn event_at(world: &World, rx: i32, ry: i32, now: Now) -> Option<(&'static EncDef, u32)> {
    if world.shard_dungeon_at(rx, ry).is_some() || world.is_town(rx, ry) {
        return None;
    }
    let tier = World::threat_tier(rx, ry);
    let biome = world.biome_key_at(rx, ry);
    let list: Vec<&'static EncDef> = ENCOUNTERS
        .iter()
        .filter(|d| {
            (d.seasons.is_some() || d.night.is_some() || d.roaming)
                && d.seasons.is_none_or(|ss| ss.contains(&now.season))
                && d.night.is_none_or(|n| n == now.night)
                && fits(d, biome, tier)
        })
        .collect();
    if list.is_empty() {
        return None;
    }
    let h = hash(world.seed ^ (now.today as u32).wrapping_mul(0x2545_F491), rx, ry, SALT_EVENT);
    if (h % 1000) as f64 / 1000.0 >= EVENT_CHANCE {
        return None;
    }
    if !dry_enough(world, rx, ry) {
        return None;
    }
    Some((weighted(&list, h), h))
}

/// The room's STABLE encounter today (fallow ground is quiet; each tenancy rolls
/// its own dice). Quest targeting rides THIS — events are too fleeting to chase.
pub fn stable_at(
    world: &World,
    cleared: &ClearedEncounters,
    rx: i32,
    ry: i32,
    today: i64,
) -> Option<(&'static EncDef, u32)> {
    for_room(world, rx, ry, cleared.tenancy((rx, ry), today)?)
}

/// EVERYTHING here right now — the standing camp if one holds the room, else any
/// event whose window is open. THE entry point for spawn/dress/banner/lights;
/// callers should not pair for_room with their own cleared checks.
pub fn live_at(
    world: &World,
    cleared: &ClearedEncounters,
    rx: i32,
    ry: i32,
    now: Now,
) -> Option<(&'static EncDef, u32)> {
    let cycle = cleared.tenancy((rx, ry), now.today)?;
    for_room(world, rx, ry, cycle).or_else(|| event_at(world, rx, ry, now))
}

/// Stage the def into a concrete scene (js build — decor + foe list, clamped in-room).
pub fn build(def: &'static EncDef, world: &World, rx: i32, ry: i32, seed: u32) -> Scene {
    let mut s = Scene {
        cx: CX,
        cy: CY,
        biome: world.biome_key_at(rx, ry),
        tier: World::threat_tier(rx, ry),
        seed,
        decor: vec![],
        foes: vec![],
        victims: vec![],
        wanderers: vec![],
        loot: vec![],
        sleepers: vec![],
    };
    (def.place)(&mut s);
    s
}

/// (grid, height-of-base, hitbox, flat?) per decor kind — the js entity shapes.
struct DecorSpec {
    grid: &'static [&'static str],
    base_y: f32,
    hitbox: Option<(f32, f32, f32, f32)>,
}

fn spec(kind: &str) -> Option<DecorSpec> {
    Some(match kind {
        "campfire" => DecorSpec { grid: art::CAMP_A, base_y: 9.0, hitbox: Some((5.0, 6.0, 6.0, 3.0)) },
        "corpse" => DecorSpec { grid: art::CORPSE, base_y: 0.0, hitbox: None },
        "blood" => DecorSpec { grid: art::BLOOD_POOL, base_y: 0.0, hitbox: None },
        "wagon" => DecorSpec { grid: art::WAGON, base_y: 20.0, hitbox: Some((2.0, 9.0, 28.0, 11.0)) },
        "ritual" => DecorSpec { grid: art::RITUAL, base_y: 0.0, hitbox: None },
        "bones" => DecorSpec { grid: art::BONES, base_y: 0.0, hitbox: None },
        "crate" => DecorSpec { grid: art::CRATE, base_y: 8.0, hitbox: Some((3.0, 3.0, 6.0, 5.0)) },
        "tent" => DecorSpec { grid: art::TENT, base_y: 25.0, hitbox: Some((2.0, 16.0, 30.0, 9.0)) }, // HUMAN scale (Baz: the hero would never fit that door)
        "banner" => DecorSpec { grid: art::BANNER_ART, base_y: 27.0, hitbox: Some((4.0, 24.0, 4.0, 3.0)) }, // a war standard, not a toothpick
        "gold" => DecorSpec { grid: art::GOLD, base_y: 0.0, hitbox: None },
        "crystal" => DecorSpec { grid: art::CRYSTAL_ART, base_y: 10.0, hitbox: Some((4.0, 6.0, 8.0, 4.0)) },
        "web" => DecorSpec { grid: art::WEB, base_y: 0.0, hitbox: None },
        "ice" => DecorSpec { grid: art::ICE, base_y: 10.0, hitbox: Some((4.0, 6.0, 8.0, 4.0)) },
        "stake" => DecorSpec { grid: art::STAKE, base_y: 9.0, hitbox: Some((6.0, 7.0, 4.0, 2.0)) },
        "stall" => DecorSpec { grid: art::STALL, base_y: 14.0, hitbox: Some((2.0, 8.0, 22.0, 6.0)) },
        "oldwell" => DecorSpec { grid: art::OLDWELL, base_y: 12.0, hitbox: Some((3.0, 5.0, 14.0, 8.0)) },
        _ => return None,
    })
}

/// Spawn a scene's decor as room-root children (rebuilt identically every visit);
/// solid pieces feed the blocker list. Clutter/flower/mushroom passthroughs pull from
/// the shared PropArt banks like the natural props do.
pub fn spawn_decor(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    art_bank: &mut PropArt,
    root: Entity,
    scene: &Scene,
    blockers: &mut Vec<(f32, f32, f32, f32)>,
) {
    for d in &scene.decor {
        let (x, y) = (d.x, d.y);
        if let Some(sp) = spec(d.kind) {
            let pal: &[(char, u32)] = match d.kind {
                "banner" => &[('r', 0)],  // placeholder — replaced below
                "crystal" => &[('x', 0)], // (bake needs a concrete slice per call)
                "blood" => art::BLOOD_PAL,
                "stall" => art::STALL_PAL,
                "oldwell" => art::OLDWELL_PAL,
                _ => &[],
            };
            // Recolours can't borrow a temp slice through the match — bake directly.
            let img = match d.kind {
                "banner" => images.add(bake(sp.grid, &[('r', if d.color == 0 { 0xb01818 } else { d.color })])),
                "crystal" => images.add(bake(sp.grid, &[('x', if d.color == 0 { 0xb060f0 } else { d.color })])),
                _ => images.add(bake(sp.grid, pal)),
            };
            let (w, h) = (sp.grid[0].len() as f32, sp.grid.len() as f32);
            let z = if sp.base_y > 0.0 { actor_z(y + sp.base_y) } else { 3.05 };
            let e = child(commands, root, Sprite::from_image(img), at(PLAY_X + x, PLAY_Y + y, w, h, z));
            if d.kind == "oldwell" {
                commands.entity(e).insert(WhisperWell { x, y });
            }
            if d.kind == "web" {
                // Encounter webs are REAL webs (Baz): the same sword-cut string node
                // the dungeons and spider nests use, not set dressing.
                let hb = crate::combat::Hitbox { x: x + 1.0, y, w: 13.0, h: 8.0 };
                commands.entity(e).insert(super::room_props::node_bundle(
                    "cobweb", (x / 16.0) as i32, (y / 16.0) as i32,
                    crate::combat::Tool::Sword, 2, hb, None, 0xe8e8f0, false, 0, 0,
                ));
            }
            if d.kind == "campfire" {
                commands.entity(e).insert(Campfire {
                    frames: [images.add(bake(art::CAMP_A, &[])), images.add(bake(art::CAMP_B, &[]))],
                });
            }
            if let Some((hx, hy, hw, hh)) = sp.hitbox {
                blockers.push((x + hx, y + hy, hw, hh));
            }
            continue;
        }
        // Passthroughs to the shared prop banks (js a.clutter / a.flower / a.mushroom / a.torch).
        let img = match d.kind {
            "flower" => Some(art_bank.flowers[0].clone()),
            "mushroom" | "toadstool" => art_bank.clutter.get("toadstool").cloned(),
            "torch" => Some(art_bank.torch[0].clone()),
            sub => art_bank.clutter.get(sub).cloned(),
        };
        if let Some(img) = img {
            child(commands, root, Sprite::from_image(img), at(PLAY_X + x, PLAY_Y + y, 16.0, 16.0, 3.0));
        }
    }
}

/// Campfires flicker on the shared clock (js: frame = (clock >> 3) & 1).
fn campfire_flicker(clock: Res<FrameClock>, mut fires: Query<(&Campfire, &mut Sprite)>, mut last: Local<i64>) {
    let phase = (clock.0 >> 3) & 1;
    if phase == *last {
        return;
    }
    *last = phase;
    for (f, mut s) in &mut fires {
        s.image = f.frames[phase as usize].clone();
    }
}

/// The clear watcher: the room's armed encounter is beaten the moment its last foe
/// falls — recorded forever (js clearedEncounters + onEncounterCleared).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn encounter_clear_tick(
    cur: Res<CurRoom>,
    clock: Res<FrameClock>,
    sliding: Res<super::play::SlideActive>,
    mut armed: ResMut<ArmedEncounter>,
    mut cleared: ResMut<ClearedEncounters>,
    mut stats: ResMut<super::stats::Stats>,
    mut log: ResMut<super::rewards::LootLog>,
    mut quests: ResMut<super::quests::QuestLog>,
    foes: Query<Entity, (With<EncFoe>, With<RoomActor>)>,
) {
    let Some(room) = armed.0 else { return };
    if sliding.0 {
        return;
    }
    if room != (cur.rx, cur.ry) {
        armed.0 = None; // walked away mid-fight — survivors re-arm on the next visit
        return;
    }
    if foes.is_empty() {
        armed.0 = None;
        cleared.record(room, super::gather::farm_day(clock.0));
        stats.bump("encounters", 1.0);
        log.add("encounter", "AREA CLEARED", 1, 0x7ee08a, false, true);
        // A clear quest pointed here is now READY (js onEncounterCleared).
        for q in &mut quests.0 {
            if !q.done && matches!(&q.kind, super::quests::QuestKind::Clear { rx, ry, .. } if (*rx, *ry) == room) {
                q.done = true;
                log.add("quest", &format!("QUEST READY: {}", q.goal.to_uppercase()), 1, 0xffd34d, false, true);
            }
        }
    }
}

pub struct EncountersPlugin;

impl Plugin for EncountersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClearedEncounters>()
            .init_resource::<ArmedEncounter>()
            .init_resource::<MetWanderers>()
            .add_systems(
                bevy::app::FixedUpdate,
                (
                    campfire_flicker,
                    encounter_clear_tick,
                    victim_tick.run_if(super::battle::not_sliding),
                    victim_deaths.after(crate::combat::resolve_combat),
                    wanderer_shout_tick,
                    threat_banner_tick,
                    whisper_well_tick
                        .after(super::prompts::prompt_tick)
                        .before(super::play::EndTick),
                    wanderer_talk
                        .after(super::prompts::prompt_tick)
                        .after(super::services::interact_tick)
                        .after(super::interior::door_enter)
                        .before(super::talk::talk_tick)
                        .before(super::play::EndTick),
                )
                    .run_if(playing),
            )
            .add_systems(Update, (sync_enc_people, shout_labels).run_if(playing));
    }
}

// --- INC 2: the people of the encounters — fleeing victims, friendly wanderers. ---

/// A frightened civilian caught in a hostile scene (js victim): bolts away from the
/// nearest foe, yells for help while threatened, thanks you the instant it's over.
/// They are MORTAL (js victim health 8): a foe that catches one cuts it down, leaving a
/// corpse in a pool of blood — Team::Player makes enemies target them while your own
/// swings pass harmlessly through.
#[derive(Component)]
pub struct Victim {
    pub x: f32,
    pub y: f32,
    pub seed: u32,
    pub facing: usize,
    pub anim: u32,
    pub move_t: i32,
    pub dir: (f32, f32),
    pub shout: Option<(String, i32)>,
    pub shout_t: i32,
    pub was_danger: bool,
    pub thanked: bool,
}

/// A staged friendly stranger (js wanderer): TALK for a one-time boon by role, then
/// they just chat on later visits.
#[derive(Component)]
pub struct Wanderer {
    pub x: f32,
    pub y: f32,
    pub role: &'static str,
    pub title: &'static str,
    pub seed: u32,
    pub facing: usize,
    pub shout: Option<(String, i32)>,
}

/// Rooms whose wanderer boon you've claimed (js metWanderers, saved).
#[derive(Resource, Default)]
pub struct MetWanderers(pub HashSet<(i32, i32)>);

/// The floating speech label above a victim/wanderer (the glyph-rig pattern).
#[derive(Component)]
pub struct ShoutLabel;

const PANIC: [&str; 7] =
    ["HELP!", "AAAHH!", "SOMEBODY HELP!", "PLEASE, HELP!", "GET IT AWAY!", "NO! NO!", "SAVE ME!"];
const THANKS: [&str; 5] =
    ["THANK YOU!", "THAT WAS CLOSE!", "YOU SAVED ME!", "BLESS YOU, HERO!", "I OWE YOU MY LIFE!"];

/// A slain victim (js deathEffect): a body in a pool of blood, then it's gone. The
/// blood + corpse ride RoomActor, so they clear when you leave the room (js entity reset).
fn victim_deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<super::battle::GameRng>,
    victims: Query<(Entity, &Victim, &crate::combat::Health)>,
) {
    for (e, v, h) in &victims {
        if h.hp > 0 {
            continue;
        }
        super::battle::spawn_burst(&mut commands, &mut rng, Vec2::new(v.x + 8.0, v.y + 8.0), 0xd82800, 10);
        for (kind, pal) in [("blood", art::BLOOD_PAL), ("corpse", &[][..])] {
            if let Some(sp) = spec(kind) {
                let img = images.add(bake(sp.grid, pal));
                let (w, hh) = (sp.grid[0].len() as f32, sp.grid.len() as f32);
                commands.spawn((
                    Sprite::from_image(img),
                    at(PLAY_X + v.x, PLAY_Y + v.y, w, hh, 3.02),
                    PIXEL_LAYER,
                    RoomActor,
                ));
            }
        }
        commands.entity(e).despawn();
    }
}

/// Spawn a scene's victims with the foes (fresh rooms only — js pushes them to `mobs`).
pub fn spawn_victims(commands: &mut Commands, scene: &Scene) {
    for (x, y) in &scene.victims {
        let look_seed = scene.seed ^ ((*x as i32).wrapping_mul(131) + (*y as i32).wrapping_mul(17)) as u32;
        commands.spawn((
            Victim {
                x: *x,
                y: *y,
                seed: look_seed,
                facing: 0,
                anim: 0,
                move_t: 0,
                dir: (0.0, 0.0),
                shout: None,
                shout_t: 8,
                was_danger: false,
                thanked: false,
            },
            Sprite::default(),
            at(PLAY_X + x, PLAY_Y + y, 16.0, 16.0, actor_z(y + 16.0)),
            PIXEL_LAYER,
            RoomActor,
            // MORTAL (js victim health 8): Team::Player so enemy attacks hit it and
            // YOUR swings (hurt_team Enemy) skip it — you protect it, never harm it.
            // Generous i-frames (js invuln 48) so a swarm can't delete it instantly.
            crate::combat::Combatant { team: crate::combat::Team::Player, hurt_team: None, damage: None, persistent: false, knock: 0.0 },
            crate::combat::Health { hp: 8, max: 8, defense: 0, invuln: 0, flash: 0 },
            crate::combat::HurtProfile { invuln: 48, flash: 10, kb_base: 0.0, kb_resist: 0.0, kb_frames: 0 },
            crate::combat::Blood(0xd82800),
            crate::combat::Hitbox { x: *x + 3.0, y: *y + 4.0, w: 10.0, h: 10.0 },
        ));
    }
}

/// Spawn a scene's wanderers with the DECOR (they persist every visit; the boon ledger
/// keeps them from paying twice).
pub fn spawn_wanderers(commands: &mut Commands, root: Entity, scene: &Scene) {
    for (x, y, role, title) in &scene.wanderers {
        let look_seed = scene
            .seed
            .wrapping_add(0) // (kept explicit: the js xors an imul mix — close enough for a LOOK)
            ^ (((*x as i32).wrapping_mul(131) + (*y as i32).wrapping_mul(17) + role.len() as i32) as u32)
                .wrapping_mul(2654435761);
        let e = child(
            commands,
            root,
            Sprite::default(),
            at(PLAY_X + x, PLAY_Y + y, 16.0, 16.0, actor_z(y + 16.0)),
        );
        commands
            .entity(e)
            .insert(Wanderer { x: *x, y: *y, role, title, seed: look_seed, facing: 0, shout: None });
    }
}

/// js faceFrom, in villager frame indices (0 down / 1 up / 2 right / 3 left).
fn face_from(dx: f32, dy: f32) -> usize {
    if dx.abs() > dy.abs() {
        if dx < 0.0 { 3 } else { 2 }
    } else if dy < 0.0 {
        1
    } else {
        0
    }
}

/// The victims' whole little lives: flee the nearest foe, mill about when safe, panic
/// on a timer, thank the hero the moment the danger ends (js victim.update).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn victim_tick(
    grid: Res<super::play::CurGrid>,
    blockers: Res<super::room_props::RoomBlockers>,
    clock: Res<FrameClock>,
    sliding: Res<super::play::SlideActive>,
    players: Query<&super::play::Player>,
    mobs: Query<&crate::actors::mobs::Mob>,
    goblins: Query<&crate::actors::goblin::Goblin>,
    mut victims: Query<(&mut Victim, &mut crate::combat::Hitbox)>,
) {
    if sliding.0 {
        return;
    }
    let Ok(p) = players.single() else { return };
    // The nearest live foe (danger radius: anywhere in the room counts, like the js —
    // its fleeFrom is simply the closest foe while any live).
    let mut foes: Vec<(f32, f32)> = mobs.iter().map(|m| (m.x, m.y)).collect();
    foes.extend(goblins.iter().map(|g| (g.x, g.y)));
    // A stateless per-victim rand off the clock + seed (no GameRng churn needed).
    let rnd = |seed: u32, t: i64| {
        let mut h = seed ^ (t as u32).wrapping_mul(0x9e3779b1);
        h ^= h >> 15;
        h = h.wrapping_mul(0x2c1b3c6d);
        (h >> 8) as f32 / 16777216.0
    };
    for (mut v, mut hb) in &mut victims {
        let danger = !foes.is_empty();
        let nearest = foes
            .iter()
            .min_by(|a, b| {
                let da = (a.0 - v.x).hypot(a.1 - v.y);
                let db = (b.0 - v.x).hypot(b.1 - v.y);
                da.total_cmp(&db)
            })
            .copied();
        let step = |v: &mut Victim, mx: f32, my: f32, grid: &crate::room::RoomGrid, blk: &super::room_props::RoomBlockers| {
            let (nx, ny) = (v.x + mx, v.y + my);
            let b = (nx + 3.0, ny + 8.0, 10.0, 6.0);
            if grid.box_hits_solid(b.0, b.1, b.2, b.3)
                || blk.blocks((v.x + 3.0, v.y + 8.0, 10.0, 6.0), b)
                || nx < 4.0
                || ny < 4.0
                || nx > (PX_W - 18) as f32
                || ny > (PX_H - 18) as f32
            {
                return;
            }
            v.x = nx;
            v.y = ny;
        };
        if let (true, Some((fx, fy))) = (danger, nearest) {
            // Bolt directly away from the foe.
            let (dx, dy) = (v.x - fx, v.y - fy);
            let d = dx.hypot(dy).max(1.0);
            let s = 1.3;
            step(&mut v, dx / d * s, 0.0, &grid.0, &blockers);
            step(&mut v, 0.0, dy / d * s, &grid.0, &blockers);
            v.facing = face_from(dx, dy);
            v.anim = v.anim.wrapping_add(2);
        } else {
            // Safe: face the hero when near, else mill about calmly.
            let (pdx, pdy) = (p.x - v.x, p.y - v.y);
            if pdx.hypot(pdy) < 36.0 {
                v.facing = face_from(pdx, pdy);
            } else {
                v.move_t -= 1;
                if v.move_t <= 0 {
                    v.move_t = 40 + (rnd(v.seed, clock.0) * 60.0) as i32;
                    let r = (rnd(v.seed ^ 7, clock.0) * 5.0) as i32;
                    v.dir = match r {
                        0 => (-1.0, 0.0),
                        1 => (1.0, 0.0),
                        2 => (0.0, -1.0),
                        3 => (0.0, 1.0),
                        _ => (0.0, 0.0),
                    };
                }
                let (mx, my) = (v.dir.0 * 0.35, v.dir.1 * 0.35);
                step(&mut v, mx, my, &grid.0, &blockers);
                if v.dir != (0.0, 0.0) {
                    v.facing = face_from(v.dir.0, v.dir.1);
                    v.anim = v.anim.wrapping_add(1);
                }
            }
        }
        // Speech: periodic panicked yells in danger; one grateful line the moment it ends.
        if danger {
            v.shout_t -= 1;
            if v.shout_t <= 0 {
                let line = PANIC[(rnd(v.seed ^ 13, clock.0) * PANIC.len() as f32) as usize % PANIC.len()];
                v.shout = Some((line.to_string(), 64));
                v.shout_t = 70 + (rnd(v.seed ^ 29, clock.0) * 70.0) as i32;
            }
        } else if v.was_danger && !v.thanked {
            let line = THANKS[(rnd(v.seed ^ 31, clock.0) * THANKS.len() as f32) as usize % THANKS.len()];
            v.shout = Some((line.to_string(), 150));
            v.thanked = true;
        }
        v.was_danger = danger;
        if let Some((_, t)) = &mut v.shout {
            *t -= 1;
            if *t <= 0 {
                v.shout = None;
            }
        }
        hb.x = v.x + 3.0;
        hb.y = v.y + 4.0;
    }
}

/// TALK to a wanderer: their one-time boon by role, tracked per room so it can't be
/// farmed (js talkWanderer); afterwards a friendly idle line. DEVIATION (flagged): the
/// minstrel's waysong speed-buff awaits the player status system — he refills your
/// MANA instead ("a tune for the road").
/// THE WHISPERING WELL (Ideas pdf): an ancient ring over a mouth of dark. Toss a
/// coin (one per well per day) and the water answers — a whisper, a blessing, a
/// potion splashed up, the path to a treasure... and very rarely, YOUR NAME.
#[derive(Component)]
pub struct WhisperWell {
    pub x: f32,
    pub y: f32,
}

#[derive(Component, Clone)]
struct WellPromptUi;

/// The well's toss-a-coin working set (grouped under the 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct WellCtx<'w> {
    inv: ResMut<'w, crate::inventory::PlayerInv>,
    statuses: ResMut<'w, super::status::Statuses>,
    mana: ResMut<'w, super::flute::Mana>,
    log: ResMut<'w, super::rewards::LootLog>,
    banners: ResMut<'w, super::banners::Banners>,
    ident: Res<'w, super::identity::HeroIdent>,
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn whisper_well_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<crate::input::ActionState>,
    bindings: Res<crate::input::Bindings>,
    world: Res<super::play::GameWorld>,
    cur: Res<CurRoom>,
    clock: Res<FrameClock>,
    players: Query<&super::play::Player>,
    wells: Query<&WhisperWell>,
    mut cx: WellCtx,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    old: Query<Entity, With<WellPromptUi>>,
    mut shown: Local<bool>,
    mut drank: Local<bevy::platform::collections::HashMap<(i32, i32), i64>>,
) {
    let Ok(p) = players.single() else { return };
    let today = super::gather::farm_day(clock.0);
    let room = (cur.rx, cur.ry);
    let near = wells.iter().next().filter(|w| {
        ((p.x + 8.0) - (w.x + 10.0)).abs() < 16.0 && (p.y + 8.0) - w.y > 4.0 && (p.y + 8.0) - w.y < 30.0
    });
    let can = near.is_some() && drank.get(&room) != Some(&today);
    if can != *shown {
        *shown = can;
        for e in &old {
            commands.entity(e).despawn();
        }
        if can {
            let key = bindings.prompt(crate::input::Action::Interact, input.pad_present);
            super::prompts::spawn_bubble(&mut commands, &mut images, &format!("{key} TOSS A COIN"), p.x + 8.0, p.y - 10.0, WellPromptUi);
        }
    }
    let Some(w) = near else { return };
    if !input.pressed(crate::input::Action::Interact) {
        return;
    }
    input.consume(crate::input::Action::Interact);
    if drank.get(&room) == Some(&today) {
        cx.log.add("well", "THE WELL IS SILENT NOW", 1, 0x8a8a92, false, true);
        return;
    }
    if cx.inv.money < 1 {
        cx.log.add("well", "IT WANTS A COIN", 1, 0x8a8a92, false, true);
        sfx.write(super::sfx::Sfx("tink"));
        return;
    }
    cx.inv.money -= 1;
    drank.insert(room, today);
    let h = hash(world.0.seed ^ (today as u32).wrapping_mul(0x9e37_79b9), room.0, room.1, 0x77e1_15e1);
    let tier = World::threat_tier(room.0, room.1);
    match h % 100 {
        0..=1 => {
            // It knows you. The water remembers.
            let name = cx.ident.name.to_uppercase();
            cx.banners.note(&name, "- THE WATER REMEMBERS -");
            cx.statuses.add("waysong", 5400);
            cx.mana.cur = cx.mana.max;
            if cx.inv.can_add("potion") {
                cx.inv.add_item("potion", 1);
            }
            sfx.write(super::sfx::Sfx("songmatch"));
        }
        2..=9 => {
            if cx.inv.can_add("treasuremap") {
                cx.inv.add_item("treasuremap", 1);
                cx.log.add("treasuremap", "THE WATER SHOWS YOU A PLACE", 1, 0xd8b8ff, false, true);
            } else {
                spawn_pickup_at(&mut commands, &mut images, "treasuremap", 1, w.x + 4.0, w.y + 18.0);
            }
            sfx.write(super::sfx::Sfx("itemget"));
        }
        10..=29 => {
            if cx.inv.can_add("potion") {
                cx.inv.add_item("potion", 1);
                cx.log.add("potion", "A POTION SPLASHES UP", 1, 0xd83060, false, true);
            } else {
                spawn_pickup_at(&mut commands, &mut images, "potion", 1, w.x + 4.0, w.y + 18.0);
            }
            sfx.write(super::sfx::Sfx("itemget"));
        }
        30..=49 => {
            cx.statuses.add("waysong", 3600);
            cx.mana.cur = cx.mana.max;
            cx.log.add("well", "THE WELL HUMS KINDLY", 1, 0xa8e0ff, false, true);
            sfx.write(super::sfx::Sfx("songmatch"));
        }
        _ => {
            let whispers = [
                "THE WATER SAYS... NOT YET",
                "A NAME YOU DO NOT KNOW... YET",
                "IT COUNTS YOUR COINS BACK TO YOU",
                "SOMETHING TURNS OVER, FAR BELOW",
                "THE DEEP LANDS HOLD WHAT YOU SEEK",
                "IT HUMS A TUNE YOU ALMOST REMEMBER",
            ];
            let line = whispers[((h >> 8) as usize + tier as usize) % whispers.len()];
            cx.log.add("well", line, 1, 0xcfc9a8, false, true);
            sfx.write(super::sfx::Sfx("open"));
        }
    }
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn wanderer_talk(
    mut statuses: ResMut<super::status::Statuses>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<crate::input::ActionState>,
    cur: Res<CurRoom>,
    mut met: ResMut<MetWanderers>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut mana: ResMut<super::flute::Mana>,
    mut log: ResMut<super::rewards::LootLog>,
    mut stats: ResMut<super::stats::Stats>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut rng: ResMut<super::battle::GameRng>,
    players: Query<&super::play::Player>,
    mut human_art: ResMut<crate::actors::goblin::HumanArt>,
    mut wanderers: Query<(&mut Wanderer, Entity)>,
) {
    let Ok(p) = players.single() else { return };
    if !input.pressed(crate::input::Action::Interact) {
        return;
    }
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let Some((mut w, _)) = wanderers
        .iter_mut()
        .find(|(w, _)| ((w.x + 8.0) - pcx).hypot((w.y + 8.0) - pcy) < 26.0)
    else {
        return;
    };
    input.consume(crate::input::Action::Interact);
    w.facing = face_from(pcx - (w.x + 8.0), pcy - (w.y + 8.0));
    let room = (cur.rx, cur.ry);
    let tier = World::zone_tier(cur.rx, cur.ry);
    if met.0.contains(&room) {
        let idle = match w.role {
            "lost" => "OFF TO TOWN NOW. FARE WELL!",
            "minstrel" => "LA LA LAAA... HELLO AGAIN!",
            "hurt" => "MUCH BETTER, THANKS TO YOU.",
            "herbalist" => "THE WILDS ARE GENEROUS.",
            "pilgrim" => "WALK IN THE LIGHT, FRIEND.",
            "trapper" => "MIND THE SNARES OUT THERE.",
            "storyteller" => "COME BACK ANYTIME. THE FIRE IS WARM.",
            "dancer" => "WHAT A DAY! WHAT A DAY!",
            "stargazer" => "THE SKY REMEMBERS EVERYTHING.",
            "mourner" => "THANK YOU FOR STANDING WITH US.",
            "knight" => "GO... TAKE IT AND GO. LET ME REST.",
            "merchant" => "SOLD OUT, FRIEND. SOLD OUT.",
            _ => "SAFE TRAVELS!",
        };
        w.shout = Some((idle.to_string(), 220));
        sfx.write(super::sfx::Sfx("menuMove"));
        return;
    }
    let grant_coins = |n: i32, inv: &mut crate::inventory::PlayerInv, log: &mut super::rewards::LootLog, stats: &mut super::stats::Stats| {
        inv.money += n as i64;
        stats.bump("coins", n as f64);
        log.add("coin", &format!("+{n} COIN"), n, 0xfcd000, true, false);
    };
    match w.role {
        "hurt" => {
            // They need a dressing from YOUR bag — return with one and they'll still be here.
            let heal = if inv.has_item("bandage") {
                Some("bandage")
            } else if inv.has_item("potion") {
                Some("potion")
            } else {
                None
            };
            let Some(heal) = heal else {
                w.shout = Some(("IF ONLY I HAD A BANDAGE...".to_string(), 220));
                sfx.write(super::sfx::Sfx("tink"));
                return;
            };
            inv.remove_one(heal);
            grant_coins(30 + tier * 15, &mut inv, &mut log, &mut stats);
            let (id, qty) = crate::items::roll_loot(0.2 + tier as f64 * 0.12, 0.0, || rng.0.next_f64());
            if inv.can_add(id) {
                inv.add_item(id, qty);
                let name = crate::items::get(id).map_or(id, |d| d.name).to_uppercase();
                log.add(id, &name, qty, super::rewards::toast_color(id), false, false);
            } else {
                spawn_pickup_at(&mut commands, &mut images, id, qty, w.x, w.y + 12.0);
            }
            w.shout = Some(("BLESS YOU! TAKE THIS.".to_string(), 220));
            sfx.write(super::sfx::Sfx("itemget"));
        }
        "lost" => {
            grant_coins(20 + tier * 12, &mut inv, &mut log, &mut stats);
            w.shout = Some(("TOWN IS NEAR? BLESS YOU!".to_string(), 220));
            sfx.write(super::sfx::Sfx("itemget"));
        }
        "minstrel" => {
            statuses.add("waysong", 3600); // the true waysong: +move, gentle mending
            mana.cur = mana.max; // and a refilled songwell besides
            w.shout = Some(("A TUNE FOR THE ROAD!".to_string(), 220));
            log.add("song", "THE TUNE RESTORES YOUR SPIRIT", 1, 0xcfc9a8, false, true);
            sfx.write(super::sfx::Sfx("itemget"));
        }
        "herbalist" => {
            let n = 2 + tier;
            inv.add_item("herb", n);
            log.add("herb", "HERB", n, 0x9ad06a, false, false);
            if inv.can_add("potion") {
                inv.add_item("potion", 1);
                log.add("potion", "POTION", 1, 0xd83060, false, false);
            }
            w.shout = Some(("THE WILDS PROVIDE.".to_string(), 220));
            sfx.write(super::sfx::Sfx("itemget"));
        }
        "pilgrim" => {
            // A blessing for the road: a potion, a dressing, and a kind word.
            if inv.can_add("potion") {
                inv.add_item("potion", 1);
                log.add("potion", "POTION", 1, 0xd83060, false, false);
            }
            if inv.can_add("bandage") {
                inv.add_item("bandage", 1);
                log.add("bandage", "BANDAGE", 1, 0xe8e0d0, false, false);
            }
            w.shout = Some(("THE ROAD KEEP YOU.".to_string(), 220));
            sfx.write(super::sfx::Sfx("itemget"));
        }
        "merchant" => {
            // THE FALSE MERCHANT (Ideas pdf): the same stall, four fates — the
            // room's seed decides who he really is. Sales that can't close leave
            // the offer OPEN (no met-mark), so you can come back with coin.
            match w.seed % 4 {
                0 => {
                    // Genuine: a fair price on a real find.
                    if inv.money < 25 {
                        w.shout = Some(("FINE GOODS, 25 COIN. COME BACK.".to_string(), 220));
                        sfx.write(super::sfx::Sfx("tink"));
                        return;
                    }
                    inv.money -= 25;
                    let (id, qty) = crate::items::roll_loot(0.45 + tier as f64 * 0.1, 0.0, || rng.0.next_f64());
                    if inv.can_add(id) {
                        inv.add_item(id, qty);
                        let name = crate::items::get(id).map_or(id, |d| d.name).to_uppercase();
                        log.add(id, &name, qty, super::rewards::toast_color(id), false, false);
                    } else {
                        spawn_pickup_at(&mut commands, &mut images, id, qty, w.x, w.y + 12.0);
                    }
                    w.shout = Some(("A FAIR PRICE, FRIEND.".to_string(), 220));
                    sfx.write(super::sfx::Sfx("itemget"));
                }
                1 => {
                    // The trap: his friends were behind the stall all along.
                    w.shout = Some(("NOTHING PERSONAL, FRIEND.".to_string(), 260));
                    for (bx, by) in [(-42.0, -20.0), (44.0, -16.0), (0.0, 38.0)] {
                        let (x, y) = (w.x + bx, w.y + by);
                        let seed = (x as i32 as u32).wrapping_mul(2654435761) ^ (y as i32 as u32).wrapping_mul(97) ^ 0xfa15e;
                        let frames = human_art.frames("bandit", seed, &mut images);
                        commands.spawn((
                            crate::actors::goblin::goblin_bundle(crate::actors::goblin::GoblinKind::Melee, x, y),
                            Sprite::default(),
                            crate::actors::goblin::HumanSkin { kind: "bandit", seed, frames },
                            RoomActor,
                            crate::gfx::PIXEL_LAYER,
                            EncFoe,
                        ));
                    }
                    sfx.write(super::sfx::Sfx("swing"));
                }
                2 => {
                    // Black market: pricey, but the chart is real.
                    if inv.money < 60 {
                        w.shout = Some(("THE GOOD STUFF IS 60. COME BACK RICH.".to_string(), 220));
                        sfx.write(super::sfx::Sfx("tink"));
                        return;
                    }
                    inv.money -= 60;
                    if inv.can_add("treasuremap") {
                        inv.add_item("treasuremap", 1);
                        log.add("treasuremap", "TREASURE MAP", 1, 0xd8b8ff, false, false);
                    } else {
                        spawn_pickup_at(&mut commands, &mut images, "treasuremap", 1, w.x, w.y + 12.0);
                    }
                    w.shout = Some(("DONT ASK WHERE I GOT IT.".to_string(), 220));
                    sfx.write(super::sfx::Sfx("itemget"));
                }
                _ => {
                    // Already robbed — he presses his last stock on you.
                    if inv.can_add("potion") {
                        inv.add_item("potion", 1);
                        log.add("potion", "POTION", 1, 0xd83060, false, false);
                    }
                    w.shout = Some(("BANDITS TOOK THE REST. TAKE IT.".to_string(), 260));
                    sfx.write(super::sfx::Sfx("itemget"));
                }
            }
        }
        "knight" => {
            // His last tale, and his blade — rolled by the forge that made him
            // (procgen weapon, scaled to the land that killed him).
            let tales = [
                "WE WERE TWELVE AGAINST THE DARK... I AM WHAT IS LEFT.",
                "THE CAMP PAST THE RIDGE... DO NOT GO UNARMED, FRIEND.",
                "I HELD THE LINE SO THE OTHERS COULD RUN. WORTH IT.",
                "MY ORDER IS ASHES. LET MY BLADE SERVE SOMEONE TRUE.",
            ];
            w.shout = Some((tales[(w.seed as usize) % tales.len()].to_string(), 300));
            // The blade rolls the SAME rarity dice as any drop (Baz: a guaranteed
            // epic here made hour one endgame — purple is ENDGAME). Threat deepens
            // the odds; the floor is uncommon (a knight's blade is never junk), the
            // cap is epic and it stays a story you tell.
            let mut dice = crate::worldgen::rng::Mulberry32::new(w.seed ^ 0x5eed_b1ad);
            let ti = crate::items::roll_tier(0.15 + tier as f64 * 0.10, 0.0, || dice.next_f64()).clamp(1, 3);
            let id = crate::procgen::generate(crate::procgen::Kind::Weapon, ti as i32, w.seed ^ 0x5eed_b1ad);
            let name = crate::procgen::resolve(id).map_or("A BLADE", |d| d.name).to_uppercase();
            if inv.can_add(id) {
                inv.add_item(id, 1);
                log.add(id, &name, 1, 0xd8b8ff, false, false);
            } else {
                spawn_pickup_at(&mut commands, &mut images, id, 1, w.x, w.y + 12.0);
            }
            log.add("tale", "THE KNIGHTS LAST GIFT", 1, 0xcfc9a8, false, true);
            sfx.write(super::sfx::Sfx("itemget"));
        }
        "storyteller" => {
            // The tale IS the gift — one of the fire's stories, picked by the room.
            let tales = [
                "THEY SAY THE WRIFT SINGS IF YOU LISTEN LONG ENOUGH.",
                "A KING SPLIT THE HEART, AND THE HEART SPLIT HIM BACK.",
                "NEVER TRUST A QUIET CAMP WITH COIN IN THE OPEN.",
                "THE SPIRE OUT PAST THE DEEP LANDS? IT HUMS AT NIGHT.",
                "MY GRANDMOTHER TRADED A SONG TO A STONE. IT PAID.",
            ];
            w.shout = Some((tales[(w.seed as usize) % tales.len()].to_string(), 300));
            log.add("tale", "A TALE FOR THE ROAD", 1, 0xcfc9a8, false, true);
            sfx.write(super::sfx::Sfx("songmatch"));
        }
        "trapper" => {
            let n = 2 + tier;
            inv.add_item("leather", n);
            log.add("leather", "LEATHER", n, 0xb08a5a, false, false);
            inv.add_item("meat", 2);
            log.add("meat", "RAW MEAT", 2, 0xd87878, false, false);
            w.shout = Some(("SPARE HIDES FOR A FRIEND.".to_string(), 220));
            sfx.write(super::sfx::Sfx("itemget"));
        }
        _ => {
            w.shout = Some(("WELL MET.".to_string(), 220));
            sfx.write(super::sfx::Sfx("menuMove"));
        }
    }
    met.0.insert(room);
    saves.write(super::save::SaveRequest);
}

fn spawn_pickup_at(commands: &mut Commands, images: &mut Assets<Image>, id: &'static str, qty: i32, x: f32, y: f32) {
    super::gather::spawn_pickup(commands, images, id, qty, x, y, true, None);
}

/// Dress the victims + wanderers each frame from the shared villager sprite bank
/// (their look keys off their seed, like every villager).
pub fn sync_enc_people(
    mut art: ResMut<crate::actors::villager::VillagerArt>,
    mut images: ResMut<Assets<Image>>,
    mut victims: Query<(&Victim, &crate::combat::Health, &mut Sprite, &mut Visibility, &mut Transform), Without<Wanderer>>,
    mut wanderers: Query<(&Wanderer, &mut Sprite, &mut Transform), Without<Victim>>,
) {
    for (v, health, mut sprite, mut vis, mut tf) in &mut victims {
        let frames = art.frames(v.seed, &mut images);
        let fi = ((v.anim / 8) % 4) as usize;
        let img = &frames.frames[v.facing][fi];
        if sprite.image != *img {
            sprite.image = img.clone();
        }
        // Blink on the hurt frame (js: flash & 1 -> skip the draw).
        *vis = if health.flash > 0 && (health.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
        *tf = at(PLAY_X + v.x.round(), PLAY_Y + v.y.round(), 16.0, 16.0, actor_z(v.y.round() + 16.0));
    }
    for (w, mut sprite, mut tf) in &mut wanderers {
        let frames = art.frames(w.seed, &mut images);
        let img = &frames.frames[w.facing][0];
        if sprite.image != *img {
            sprite.image = img.clone();
        }
        // Root children keep room-local coords (they ride slides like villagers).
        *tf = at(PLAY_X + w.x.round(), PLAY_Y + w.y.round(), 16.0, 16.0, actor_z(w.y.round() + 16.0));
    }
}

/// Speech bubbles over the encounter folk — the floating-label rig (bake on change).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
#[allow(clippy::type_complexity)] // the chat-bubble peek needs its Without wall
pub fn shout_labels(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    sliding: Res<super::play::SlideActive>,
    victims: Query<(Entity, &Victim)>,
    wanderers: Query<(Entity, &Wanderer)>,
    mut labels: Query<(&mut Transform, &mut Visibility), With<ShoutLabel>>,
    chat: Query<(&Transform, &Sprite), (With<crate::app::talk::ChatUi>, Without<ShoutLabel>)>,
    mut live: Local<bevy::platform::collections::HashMap<Entity, (String, Entity, f32, f32)>>,
) {
    let mut seen: Vec<Entity> = Vec::new();
    // Bubbles NEVER overlap (Baz): every placed rect this frame — seeded with the
    // town chat bubble (its 11-tall backing; border strips and text are thinner) —
    // and each shout hops ABOVE whatever it would cover.
    let mut placed: Vec<(f32, f32, f32)> = chat
        .iter()
        .filter(|(_, s)| s.custom_size.is_some_and(|v| (v.y - 11.0).abs() < 0.5))
        .map(|(t, s)| {
            let w = s.custom_size.unwrap().x;
            (t.translation.x + crate::CANVAS_W as f32 / 2.0 - w / 2.0,
             crate::CANVAS_H as f32 / 2.0 - t.translation.y - 5.5,
             w)
        })
        .collect();
    let mut place = |commands: &mut Commands,
                     images: &mut Assets<Image>,
                     labels: &mut Query<(&mut Transform, &mut Visibility), With<ShoutLabel>>,
                     live: &mut bevy::platform::collections::HashMap<Entity, (String, Entity, f32, f32)>,
                     owner: Entity,
                     shout: Option<&str>,
                     x: f32,
                     y: f32| {
        let Some(text) = shout else {
            if let Some((_, old, ..)) = live.remove(&owner) {
                commands.entity(old).despawn();
            }
            return;
        };
        // The bubble floats where the town bubbles float (talk.rs by = y - 13),
        // clamped into the play field so a shore-side shout never clips off —
        // then HOPS UPWARD past any bubble it would cover (Baz: never overlap).
        let mw = crate::gfx::font::measure(text);
        let bw = (mw + (mw & 1)) as f32 + 8.0;
        let bx = (PLAY_X + x + 8.0 - bw / 2.0).round().clamp(PLAY_X + 2.0, PLAY_X + crate::room::PX_W as f32 - bw - 2.0);
        let mut by = (PLAY_Y + y - 13.0).round();
        while placed.iter().any(|&(ox, oy, ow)| bx < ox + ow && bx + bw > ox && by < oy + 11.0 && by + 11.0 > oy) {
            by -= 13.0;
        }
        placed.push((bx, by, bw));
        // Spawn AT the final spot and respawn on any change — the town-chat rule.
        // (The old spawn-then-reposition path is what garbled the knight's line.)
        let fresh = match live.get(&owner) {
            Some((t, _, ox, oy)) => t != text || *ox != bx || *oy != by,
            None => true,
        };
        if fresh {
            if let Some((_, old, ..)) = live.remove(&owner) {
                commands.entity(old).despawn();
            }
            let (e, _) = crate::ui::speech_bubble(commands, images, text, bx, by, crate::gfx::layers::CHAT);
            commands.entity(e).insert(ShoutLabel);
            live.insert(owner, (text.to_string(), e, bx, by));
        }
        if let Some((_, e, ..)) = live.get(&owner)
            && let Ok((_, mut vis)) = labels.get_mut(*e)
        {
            *vis = if sliding.0 { Visibility::Hidden } else { Visibility::Inherited };
        }
        seen.push(owner);
    };
    for (e, v) in &victims {
        place(&mut commands, &mut images, &mut labels, &mut live, e, v.shout.as_ref().map(|(t, _)| t.as_str()), v.x, v.y);
    }
    for (e, w) in &wanderers {
        place(&mut commands, &mut images, &mut labels, &mut live, e, w.shout.as_ref().map(|(t, _)| t.as_str()), w.x, w.y);
    }
    live.retain(|owner, (_, e, ..)| {
        if seen.contains(owner) {
            true
        } else {
            commands.entity(*e).despawn();
            false
        }
    });
}

/// A wanderer's shout runs down like a victim's (their tick is the talk handler, so
/// the countdown lives here).
pub fn wanderer_shout_tick(mut wanderers: Query<&mut Wanderer>) {
    for mut w in &mut wanderers {
        if let Some((_, t)) = &mut w.shout {
            *t -= 1;
            if *t <= 0 {
                w.shout = None;
            }
        }
    }
}

/// Entering an un-cleared hostile camp announces it (the js threat banner).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn threat_banner_tick(
    cur: Res<CurRoom>,
    sliding: Res<super::play::SlideActive>,
    inside: Res<super::interior::Inside>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    world: Res<super::play::GameWorld>,
    cleared: Res<ClearedEncounters>,
    clock: Res<FrameClock>,
    mut banners: ResMut<super::banners::Banners>,
    mut last: Local<Option<(i32, i32)>>,
) {
    if sliding.0 || inside.0.is_some() || in_dungeon.0.is_some() {
        return;
    }
    let room = (cur.rx, cur.ry);
    if *last == Some(room) {
        return;
    }
    *last = Some(room);
    if let Some((def, _)) = live_at(&world.0, &cleared, room.0, room.1, Now::at(clock.0))
        && !def.friendly
    {
        banners.threat(def.name);
    }
}
