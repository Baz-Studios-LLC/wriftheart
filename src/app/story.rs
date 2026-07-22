//! story.rs — THE FIRST-HOUR THREAD: a real two-leg quest line from the ashes of
//! Emberfall to the first shard, then it bows out forever.
//!
//! Leg 1 (THE LAST EMBER): WREN, the burnt village's last voice, offers it through
//! the normal QUEST window — reach the nearest living town and speak to its keeper.
//! It resolves at the keeper: the first AXE + PICK, and leg 2 (THE FIRST SHARD)
//! takes its place — claim a relic from the marked den, paying out on pickup.
//! Both legs live in the quest log (gold '!' pins, reopenable at their givers)
//! but are SLOT-EXEMPT: they never eat one of the 3 side-quest slots, never show
//! a hand-in '?', and abandoning is allowed (leg 1 re-offers; leg 2 lets go).
//! Nothing is gated behind the thread: skip it and the world plays as before.

use bevy::prelude::*;

use super::quests::{giver_key, Quest, QuestKind, QuestLog, Reward};
use super::screen::playing;
use crate::actors::villager::Villager;

/// Where the hero stands on the thread (saved): 0 fresh, 1 leg 1 accepted,
/// 2 tools gifted + leg 2 active, 3 retired.
#[derive(Resource, Default)]
pub struct StoryThread(pub u8);

/// The lone survivor in Emberfall (spawned by room_props' ruined village).
#[derive(Component)]
pub struct StorySurvivor;

/// The marked town keeper who arms the hero (picked live on town entry).
#[derive(Component)]
pub struct StoryElder;

/// Her identity seed — fixed so she is the same person every visit: WREN
/// (people::name_for), the burnt village's last voice.
pub const SURVIVOR_SEED: u32 = 41112;

/// The room a thread leg points at: leg 1 the nearest town to home, leg 2 the
/// shard den nearest that town. Deterministic per world seed.
pub fn story_pin(world: &crate::worldgen::World, step: u8) -> Option<(i32, i32)> {
    let home = super::room_props::HOME_VILLAGE;
    match step {
        1 => crate::worldgen::towns::nearest_town(world.seed, home.0, home.1),
        2 => {
            let (tx, ty) = crate::worldgen::towns::nearest_town(world.seed, home.0, home.1)?;
            world
                .shard_sites()
                .iter()
                .min_by_key(|(_, (rx, ry))| ((rx - tx) as i64).pow(2) + ((ry - ty) as i64).pow(2))
                .map(|(_, r)| *r)
        }
        _ => None,
    }
}

/// Leg 1, ready for the offer window (dialog.rs stamps the id at accept).
pub fn town_quest(world: &crate::worldgen::World) -> Option<Quest> {
    let (rx, ry) = story_pin(world, 1)?;
    let home = super::room_props::HOME_VILLAGE;
    Some(Quest {
        id: 0,
        kind: QuestKind::Story { stage: 1, rx, ry },
        done: false,
        title: "THE LAST EMBER".to_string(),
        goal: "SPEAK TO THE TOWNS KEEPER".to_string(),
        desc: "A TOWN STILL STANDS PAST THE ASHES. FIND ITS KEEPER - THEY WILL NOT SEND A HERO OUT EMPTY HANDED.".to_string(),
        reward: Reward::default(), // the keeper's tools — granted at the handoff
        giver_key: giver_key(home.0, home.1, SURVIVOR_SEED),
        giver_rx: home.0,
        giver_ry: home.1,
    })
}

/// Leg 2, issued by the keeper at the handoff (story_talks stamps the id).
pub fn den_quest(world: &crate::worldgen::World, elder_seed: u32) -> Option<Quest> {
    let (rx, ry) = story_pin(world, 2)?;
    let town = story_pin(world, 1)?;
    Some(Quest {
        id: 0,
        kind: QuestKind::Story { stage: 2, rx, ry },
        done: false,
        title: "THE FIRST SHARD".to_string(),
        goal: "CLAIM A RELIC FROM THE SHARD DEN".to_string(),
        desc: "A SHARD BEAST DENS IN THE WILD NEARBY. CUT YOUR WAY DOWN, FACE IT, AND TAKE ITS RELIC. THE WRIFT MUST BE MENDED.".to_string(),
        reward: Reward { coin: 150, xp: 40, item: None },
        giver_key: giver_key(town.0, town.1, elder_seed),
        giver_rx: town.0,
        giver_ry: town.1,
    })
}

/// The survivor's plea (and the elder's offer) follow the step — lines are
/// dressed live so the spawns stay dumb and the words stay in one place.
fn dress_lines(
    story: Res<StoryThread>,
    mut survivors: Query<&mut Villager, With<StorySurvivor>>,
    mut elders: Query<&mut Villager, (With<StoryElder>, Without<StorySurvivor>)>,
) {
    for mut v in &mut survivors {
        let line = match story.0 {
            0 => "THEY CAME AT NIGHT AND TOOK EVERYTHING. ASK OF MY TASK - THE ROAD MUST NOT END HERE.",
            1 => "SEEK THE TOWN KEEPER. THEY WILL NOT SEND YOU OUT EMPTY HANDED.",
            _ => "THE ASHES REMEMBER. MAY THE ROAD KEEP YOU.",
        };
        if v.line != line {
            v.line = line.to_string();
        }
        if v.stock_line != line {
            v.stock_line = line.to_string(); // greetings build on her plea, not on ""
        }
    }
    // The elder only wears the offer while the thread waits on them; the restore
    // to their own words happens once, at the gift (story_talks) — stomping the
    // line here forever would eat their greetings.
    for mut v in &mut elders {
        if story.0 == 1 {
            let line = "FROM THE ASHES? POOR SOUL. TAKE MY OLD AXE AND PICK - AND I WILL MARK THE SHARD BEASTS DEN.";
            if v.line != line {
                v.line = line.to_string();
            }
        }
    }
}

/// In the marked town at step 1, crown an elder: the named keeper nearest the
/// room's heart, preferring one the quest board hasn't claimed (two stacked
/// '!' glyphs read as a bug). Villagers respawn per entry, so re-crown whenever
/// none wears the mark.
fn mark_elder(
    mut commands: Commands,
    story: Res<StoryThread>,
    world: Res<super::play::GameWorld>,
    cur: Res<super::play::CurRoom>,
    villagers: Query<(Entity, &Villager), Without<StoryElder>>,
    elders: Query<(), With<StoryElder>>,
) {
    if story.0 != 1 || !elders.is_empty() {
        return;
    }
    if story_pin(&world.0, 1) != Some((cur.rx, cur.ry)) {
        return;
    }
    let mut best: Option<(Entity, bool, f32)> = None;
    for (e, v) in &villagers {
        if v.pkey.is_none() {
            continue;
        }
        let giver = super::quests::is_giver(world.0.seed, v.seed);
        let d = (v.x - 144.0).powi(2) + (v.y - 96.0).powi(2);
        if best.map(|(_, bg, bd)| (giver, d) < (bg, bd)).unwrap_or(true) {
            best = Some((e, giver, d));
        }
    }
    if let Some((e, ..)) = best {
        commands.entity(e).try_insert(StoryElder); // the room can sweep them mid-frame
    }
}

/// The log is the truth — this watcher keeps the step in step with it: accepting
/// leg 1 (dialog.rs' normal accept path) starts the thread; abandoning a leg
/// steps back (leg 1 re-offers at WREN) or lets go (leg 2, tools already given).
fn log_watch(
    mut story: ResMut<StoryThread>,
    log: Res<QuestLog>,
    mut banners: ResMut<super::banners::Banners>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
) {
    let has = |stage: u8| log.0.iter().any(|q| matches!(q.kind, QuestKind::Story { stage: s, .. } if s == stage));
    match story.0 {
        0 if has(1) => {
            story.0 = 1;
            banners.note("A TOWN STILL STANDS", "- MARKED ON YOUR MAP -");
            sfx.write(super::sfx::Sfx("songmatch"));
        }
        1 if !has(1) => story.0 = 0, // abandoned — WREN will ask again
        2 if !has(2) => story.0 = 3, // abandoned after the gift — the thread lets go
        _ => {}
    }
}

/// The handoff — talking to the crowned keeper (chat_t snapping up) resolves
/// leg 1: the first AXE + PICK, and leg 2 takes its place in the log.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn story_talks(
    mut story: ResMut<StoryThread>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut loot: ResMut<super::rewards::LootLog>,
    mut fanfare: ResMut<super::fanfare::Fanfare>,
    discovered: Res<super::codex::items_tab::Discovered>,
    mut banners: ResMut<super::banners::Banners>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut quests: ResMut<QuestLog>,
    mut counter: ResMut<super::quests::QuestCounter>,
    world: Res<super::play::GameWorld>,
    mut elders: Query<&mut Villager, With<StoryElder>>,
    mut prev: Local<u32>,
) {
    let (e_chat, e_seed) = elders.iter().next().map(|v| (v.chat_t, v.seed)).unwrap_or((0, 0));
    if story.0 == 1 && e_chat > *prev {
        let got_axe = inv.add_item("axe", 1);
        let got_pick = inv.add_item("pick", 1);
        if !got_axe && !got_pick {
            // A full pack on the first hour is a feat — but don't eat the gift.
            banners.note("YOUR PACK IS FULL", "- MAKE ROOM AND ASK AGAIN -");
        } else {
            if let Some(i) = quests.0.iter().position(|q| matches!(q.kind, QuestKind::Story { stage: 1, .. })) {
                let q = quests.0.remove(i);
                loot.add("quest", &format!("QUEST COMPLETE: {}", q.title), 1, 0x7ee08a, false, true);
            }
            story.0 = 2;
            for (got, id, label) in [(got_axe, "axe", "AXE"), (got_pick, "pick", "PICK")] {
                if got {
                    loot.add(id, label, 1, super::rewards::toast_color(id), false, false);
                }
            }
            if super::fanfare::should_play("axe", &discovered) {
                super::fanfare::begin(&mut fanfare, "axe");
            }
            // The keeper's own charge takes leg 1's place in the log.
            if let Some(mut q) = den_quest(&world.0, e_seed) {
                counter.0 += 1;
                q.id = counter.0;
                loot.add("quest", &format!("QUEST ACCEPTED: {}", q.title), 1, 0xa8e0ff, false, true);
                quests.0.push(q);
            }
            banners.note("THE KEEPERS PARTING GIFT", "- A SHARD DEN MARKED ON YOUR MAP -");
            sfx.write(super::sfx::Sfx("itemget"));
            for mut v in &mut elders {
                let stock = v.stock_line.clone();
                v.line = stock; // gift given — back to their own words
            }
        }
    }
    *prev = e_chat;
}

/// The pay-off — a relic in hand resolves leg 2 in the field: reward, banner,
/// and the thread retires for good.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn relic_watch(
    mut story: ResMut<StoryThread>,
    relics: Res<super::dungeon::Relics>,
    mut banners: ResMut<super::banners::Banners>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut quests: ResMut<QuestLog>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut progress: ResMut<super::rewards::Progress>,
    mut alloc: ResMut<super::slideout::TreeAlloc>,
    tstats: Res<super::slideout::TreeStats>,
    mut loot: ResMut<super::rewards::LootLog>,
) {
    if story.0 != 2 || relics.0.is_empty() {
        return;
    }
    story.0 = 3;
    if let Some(i) = quests.0.iter().position(|q| matches!(q.kind, QuestKind::Story { stage: 2, .. })) {
        let q = quests.0.remove(i);
        let coin = (q.reward.coin as f64 * (1.0 + tstats.coin)).round() as i64;
        inv.money += coin;
        super::rewards::gain_xp(&mut progress, &mut alloc, q.reward.xp);
        loot.add("quest", &format!("QUEST COMPLETE: {}", q.title), 1, 0x7ee08a, false, true);
        loot.add("quest", &format!("REWARD: {}C  {}XP", coin, q.reward.xp), 1, 0xfce0a8, false, true);
    }
    let sub = if relics.0.len() == 1 { "- NINE MORE SLUMBER IN THE DEEP -" } else { "- THE WRIFT STIRS -" };
    banners.note("THE FIRST SHARD SINGS", sub);
    sfx.write(super::sfx::Sfx("songmatch"));
}

pub struct StoryPlugin;

impl Plugin for StoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StoryThread>().add_systems(
            Update,
            (dress_lines, mark_elder, log_watch, story_talks, relic_watch).run_if(playing),
        );
    }
}
