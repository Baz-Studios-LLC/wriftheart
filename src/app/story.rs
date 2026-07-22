//! story.rs — THE FIRST-HOUR THREAD: one gentle breadcrumb trail from the ashes
//! of Emberfall to the first shard, then it bows out forever.
//!
//! Step 0: a lone SURVIVOR stands in the burnt village — talk to her and the
//! nearest living town lands on your map. Step 1: that town's keeper wears the
//! story mark — talking gifts the first AXE + PICK and pins the nearest shard
//! den. Step 2: claim any relic and the thread retires (step 3, saved for good).
//! Nothing is gated behind it: skip every beat and the world plays as before.

use bevy::prelude::*;

use super::screen::playing;
use crate::actors::villager::Villager;

/// Where the hero stands on the thread (saved): 0 fresh, 1 town marked,
/// 2 tools gifted + den marked, 3 retired.
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

/// The room the thread points at right now — the map's blue '!' (and the pin
/// that keeps it in frame). Steps 0 and 3 point nowhere.
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

/// The survivor's plea (and the elder's offer) follow the step — lines are
/// dressed live so the spawns stay dumb and the words stay in one place.
fn dress_lines(
    story: Res<StoryThread>,
    mut survivors: Query<&mut Villager, With<StorySurvivor>>,
    mut elders: Query<&mut Villager, (With<StoryElder>, Without<StorySurvivor>)>,
) {
    for mut v in &mut survivors {
        let line = match story.0 {
            0 => "THEY CAME AT NIGHT AND TOOK EVERYTHING. A TOWN STILL STANDS - LET ME MARK YOUR MAP.",
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
        commands.entity(e).insert(StoryElder);
    }
}

/// The beats themselves — a talk (chat_t snapping up) advances the thread.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn story_talks(
    mut story: ResMut<StoryThread>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut fanfare: ResMut<super::fanfare::Fanfare>,
    discovered: Res<super::codex::items_tab::Discovered>,
    mut banners: ResMut<super::banners::Banners>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    survivors: Query<&Villager, With<StorySurvivor>>,
    mut elders: Query<&mut Villager, (With<StoryElder>, Without<StorySurvivor>)>,
    mut prev: Local<(u32, u32)>,
) {
    let s_chat = survivors.iter().next().map(|v| v.chat_t).unwrap_or(0);
    let e_chat = elders.iter().next().map(|v| v.chat_t).unwrap_or(0);
    if story.0 == 0 && s_chat > prev.0 {
        story.0 = 1;
        banners.note("A TOWN STILL STANDS", "- MARKED ON YOUR MAP -");
        sfx.write(super::sfx::Sfx("songmatch"));
    }
    if story.0 == 1 && e_chat > prev.1 {
        let got_axe = inv.add_item("axe", 1);
        let got_pick = inv.add_item("pick", 1);
        if !got_axe && !got_pick {
            // A full pack on the first hour is a feat — but don't eat the gift.
            banners.note("YOUR PACK IS FULL", "- MAKE ROOM AND ASK AGAIN -");
        } else {
            story.0 = 2;
            for (got, id, label) in [(got_axe, "axe", "AXE"), (got_pick, "pick", "PICK")] {
                if got {
                    log.add(id, label, 1, super::rewards::toast_color(id), false, false);
                }
            }
            if super::fanfare::should_play("axe", &discovered) {
                super::fanfare::begin(&mut fanfare, "axe");
            }
            banners.note("THE KEEPERS PARTING GIFT", "- A SHARD DEN MARKED ON YOUR MAP -");
            sfx.write(super::sfx::Sfx("itemget"));
            for mut v in &mut elders {
                let stock = v.stock_line.clone();
                v.line = stock; // gift given — back to their own words
            }
        }
    }
    *prev = (s_chat, e_chat);
}

/// The pay-off — the first relic in hand ends the thread. If the hero found a
/// shard on their own before the elder's gift, it retires silently instead.
fn relic_watch(
    mut story: ResMut<StoryThread>,
    relics: Res<super::dungeon::Relics>,
    mut banners: ResMut<super::banners::Banners>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
) {
    if story.0 >= 3 || relics.0.is_empty() {
        return;
    }
    let engaged = story.0 == 2;
    story.0 = 3;
    if engaged {
        banners.note("THE FIRST SHARD SINGS", "- NINE MORE SLUMBER IN THE DEEP -");
        sfx.write(super::sfx::Sfx("songmatch"));
    }
}

/// The gold '!' over whoever the thread waits on — the survivor at step 0, the
/// elder at step 1. Rides the quest glyph gear (plate + bake + slide-hide).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn story_glyph_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    story: Res<StoryThread>,
    sliding: Res<super::play::SlideActive>,
    clock: Res<super::room_render::FrameClock>,
    survivors: Query<&Villager, With<StorySurvivor>>,
    elders: Query<&Villager, With<StoryElder>>,
    mut sprites: Query<(&mut Transform, &mut Visibility), With<super::quests::GlyphSprite>>,
    mut live: Local<Option<(Entity, Entity, f32, f32)>>,
) {
    if sliding.0 {
        return; // quests' tick hides every GlyphSprite mid-slide, ours included
    }
    let target = match story.0 {
        0 => survivors.iter().next(),
        1 => elders.iter().next(),
        _ => None,
    };
    match (target.is_some(), *live) {
        (true, None) => {
            *live = Some(super::quests::spawn_glyph_pair(&mut commands, &mut images, '!', 0xffd34d));
        }
        (false, Some((ge, pe, ..))) => {
            commands.entity(ge).despawn();
            commands.entity(pe).despawn();
            *live = None;
        }
        _ => {}
    }
    if let (Some(v), Some((ge, pe, iw2, ink2))) = (target, *live) {
        // Centred by INK over their head, bobbing — the quest glyph's exact ride.
        let bob = (clock.0 as f32 / 14.0 + v.x).sin().round() - 11.0;
        let gx = (super::room_render::PLAY_X + v.x + 8.0 - ink2 / 2.0).round();
        let gy = (super::room_render::PLAY_Y + v.y.round() + bob).round();
        if let Ok((mut tf, mut vis)) = sprites.get_mut(ge) {
            *tf = crate::gfx::at(gx, gy, iw2, 12.0, crate::gfx::layers::PROMPT + 0.02);
            *vis = Visibility::Inherited;
        }
        if let Ok((mut tf, mut vis)) = sprites.get_mut(pe) {
            *tf = crate::gfx::at(gx - 2.0, gy - 1.0, ink2 + 4.0, 12.0, crate::gfx::layers::PROMPT + 0.005);
            *vis = Visibility::Inherited;
        }
    }
}

pub struct StoryPlugin;

impl Plugin for StoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StoryThread>().add_systems(
            Update,
            (dress_lines, mark_elder, story_talks, relic_watch, story_glyph_tick).run_if(playing),
        );
    }
}
