//! talk.rs — talking to villagers (the game.js NPC layer: meetPerson/chatWith/
//! drawNpcChat). Stand beside a named villager and press INTERACT: the first hello of
//! the day warms them (+20 points, a heart drifts up), and their speech bubble opens —
//! strangers keep their stock line, friends greet you like friends (people.rs).
//!
//! The relationship LEDGER (js `people`) lives here: pkey -> PersonRec, saved per slot.
//! The name chip over a nearby villager is the invitation to talk — gold once you're
//! confidants. Press priority is the js ladder made explicit: door/book/counter systems
//! run first and CONSUME the press; talk only sees what's left.
//!
//! An ungiven gift adds GIVE beside TALK — the press opens the chooser (dialog.rs);
//! already-gifted folk collapse to a straight chat (the js 1-option rule).
//! DEVIATIONS (flagged): QUEST/station arms join the chooser with their systems; no
//! chat sfx (audio is post-parity).

use super::gather::farm_day;
use super::play::{CurRoom, GameWorld, Player};
use super::room_render::{FrameClock, PLAY_X, PLAY_Y};
use super::screen::playing;
use crate::actors::villager::Villager;
use crate::gfx::{at, bake, font, layers, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use crate::people;
use crate::room::PX_W;
use crate::ui::label;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// One person's standing with you (js people[pkey] = {pts, lastChat, ...}).
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PersonRec {
    pub pts: i32,
    pub last_chat: i64, // day number of the last hello; -1 = never
    pub last_gift: i64, // day number of the last gift; -1 = never (one a day each)
    pub name: String,
    pub seed: u32,
    pub town: Option<String>,
    pub know_bday: bool,
    pub know_love: bool,
    pub know_hate: bool,
}

impl Default for PersonRec {
    fn default() -> Self {
        Self {
            pts: 0,
            last_chat: -1,
            last_gift: -1,
            name: String::new(),
            seed: 0,
            town: None,
            know_bday: false,
            know_love: false,
            know_hate: false,
        }
    }
}

/// The relationship ledger (js `people`; saved).
#[derive(Resource, Default)]
pub struct PeopleLedger(pub HashMap<String, PersonRec>);

/// The name-chip colour for a friendship tier (js drawNpcChat's ternary).
pub fn tier_color(hearts: i32) -> u32 {
    if hearts >= 7 {
        0xffd34d
    } else if hearts >= 3 {
        0xa8e0a8
    } else {
        0xb8c4d8
    }
}

/// The little pixel heart (js drawHeartPx's shape, as a grid).
pub const HEART_GRID: &[&str] = &["HH.HH", "HHHHH", "HHHHH", ".HHH.", "..H.."];

#[derive(Component)]
struct HeartFx {
    t: u32,
}

#[derive(Component)]
pub(crate) struct ChatUi;

/// The relationship-side resource bundle, shared with the dialog windows (gift picker).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ChatCtx<'w> {
    pub ledger: ResMut<'w, PeopleLedger>,
    pub town_names: ResMut<'w, super::banners::TownNames>,
    pub log: ResMut<'w, super::rewards::LootLog>,
    pub stats: ResMut<'w, super::stats::Stats>,
    pub saves: MessageWriter<'w, super::save::SaveRequest>,
    /// The bed/inn chooser's arms (dialog.rs): REST starts the sleep fade...
    pub sleeping: ResMut<'w, super::services::Sleeping>,
    /// ...and SET SPAWN records the death respawn point (home doorstep / inn door).
    pub house: ResMut<'w, super::home::PlayerHouse>,
    pub respawn: ResMut<'w, super::home::RespawnPoint>,
}

/// A heart drifts up — you've grown a little closer (js Entities.heartFx).
pub(crate) fn spawn_heart(commands: &mut Commands, images: &mut Assets<Image>, x: f32, y: f32) {
    commands.spawn((
        Sprite::from_image(images.add(bake(HEART_GRID, &[('H', 0xfc5878)]))),
        at(PLAY_X + x, PLAY_Y + y, 5.0, 5.0, layers::HEART_FX),
        PIXEL_LAYER,
        HeartFx { t: 0 },
    ));
}

/// Close enough to be told their birthday (js checkBirthdayLearned).
pub(crate) fn check_bday_learned(rec: &mut PersonRec, log: &mut super::rewards::LootLog) {
    if rec.know_bday || people::hearts(rec.pts) < people::BDAY_HEARTS {
        return;
    }
    rec.know_bday = true;
    let b = people::birthday_for(rec.seed);
    let season = super::codex::calendar_tab::SEASONS[b.season as usize];
    log.add("talk", &format!("{} SHARE THEIR BIRTHDAY: {season} {}", rec.name, b.day), 1, 0xfc9ab8, false, true);
}

/// js meetPerson: the record learns who they are; the day's FIRST hello warms them
/// (+20 pts, the heart, the toast). Gifting counts as meeting too.
#[allow(clippy::too_many_arguments)] // a cross-module game action's arity
pub(crate) fn meet_person(
    cx: &mut ChatCtx,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    v: &Villager,
    world: &crate::worldgen::World,
    rx: i32,
    ry: i32,
    today: i64,
) {
    let Some(pkey) = v.pkey.clone() else { return };
    let pname = v.pname.clone().unwrap_or_default();
    let town = (world.town_role(rx, ry).is_some()).then(|| cx.town_names.get(world, rx, ry));
    let rec = cx.ledger.0.entry(pkey).or_default();
    rec.name = pname.clone();
    rec.seed = v.seed;
    if rec.town.is_none() {
        rec.town = town;
    }
    if rec.last_chat != today {
        rec.last_chat = today;
        rec.pts = (rec.pts + 20).min(people::MAX_PTS);
        cx.stats.bump("hellos", 1.0);
        cx.log.add("talk", &format!("{pname} IS GLAD TO SEE YOU"), 1, 0xfc9ab8, false, true);
        check_bday_learned(rec, &mut cx.log);
        spawn_heart(commands, images, v.x + 5.0, v.y - 5.0);
        cx.saves.write(super::save::SaveRequest);
    }
}

/// js chatWith: meet them, then friends greet you like friends — and sometimes their
/// tastes slip out (1-in-3 days once you're past a heart).
#[allow(clippy::too_many_arguments)] // a cross-module game action's arity
pub(crate) fn chat_with(
    cx: &mut ChatCtx,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    v: &mut Villager,
    world: &crate::worldgen::World,
    rx: i32,
    ry: i32,
    today: i64,
) {
    meet_person(cx, commands, images, v, world, rx, ry, today);
    let Some(pkey) = v.pkey.as_deref() else { return };
    let Some(rec) = cx.ledger.0.get_mut(pkey) else { return };
    v.line = people::greeting(v.seed, rec.pts, today, &v.stock_line.clone()); // strangers mix small talk (PORT-ORIGINAL)
    if !rec.know_love && people::hearts(rec.pts) >= 1 && ((v.seed >> 4) as i64 + today) % 3 == 0 {
        rec.know_love = true;
        v.line = format!("BETWEEN US - I DO LOVE {}.", people::taste_word(people::taste_for(v.seed).love));
    }
    v.chat_t = 220;
}

pub struct TalkPlugin;

impl Plugin for TalkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PeopleLedger>().add_systems(
            bevy::app::FixedUpdate,
            (
                talk_tick
                    .after(super::prompts::prompt_tick)
                    .after(super::services::interact_tick)
                    .after(super::interior::door_enter)
                    .before(super::play::EndTick),
                heart_tick,
            )
                .run_if(playing),
        );
    }
}

/// Chat + the name chip / speech bubble over the nearest villager (js chatWith +
/// drawNpcChat). Runs AFTER every fixture system — a consumed press never reaches us.
/// With a gift still to give today, the press opens the TALK/GIVE chooser instead
/// (js openNpc; a one-option menu collapses to the action).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub(crate) fn talk_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    players: Query<&Player>,
    mut villagers: Query<(Entity, &mut Villager)>,
    mut cx: ChatCtx,
    mut dialog: ResMut<super::dialog::Dialog>,
    mut next: ResMut<NextState<super::screen::Screen>>,
    world: Res<GameWorld>,
    cur: Res<CurRoom>,
    clock: Res<FrameClock>,
    quest_log: Res<super::quests::QuestLog>,
    old: Query<Entity, With<ChatUi>>,
    mut last: Local<Option<String>>,
) {
    let Ok(p) = players.single() else { return };
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let today = farm_day(clock.0); // hellos reset at DAWN with the rest of the world (Baz)

    // The speech timer ticks on the villager (js chatT).
    for (_, mut v) in &mut villagers {
        if v.chat_t > 0 {
            v.chat_t -= 1;
        }
    }

    // Nearest named villager in arm's reach (js villagerHere, 26px centre-to-centre).
    let mut here: Option<(f32, Entity, Mut<Villager>)> = None;
    for (e, v) in &mut villagers {
        if v.pkey.is_none() {
            continue;
        }
        let d = ((v.x + 8.0) - pcx).hypot((v.y + 8.0) - pcy);
        if d < 26.0 && here.as_ref().is_none_or(|(bd, ..)| d < *bd) {
            here = Some((d, e, v));
        }
    }
    if let Some((_, entity, mut v)) = here
        && input.pressed(Action::Interact)
    {
        input.consume(Action::Interact);
        // GIVE only shows while today's gift is ungiven; QUEST while they hold a job
        // for you (offer, or an active one to review) — js npcOptions. TALK alone
        // skips the menu (js openChoice's 1-option collapse).
        let giftable = v
            .pkey
            .as_deref()
            .and_then(|k| cx.ledger.0.get(k))
            .is_none_or(|r| r.last_gift != today);
        let gkey = super::quests::giver_key(cur.rx, cur.ry, v.seed);
        let questable = quest_log.0.iter().any(|q| q.giver_key == gkey)
            || (super::quests::is_giver(world.0.seed, v.seed)
                && quest_log.0.len() < super::quests::QUEST_MAX);
        let mut opts = vec![("TALK".into(), super::dialog::ChoiceAct::Talk)];
        if giftable {
            opts.push(("GIVE".into(), super::dialog::ChoiceAct::Gift));
        }
        if questable {
            opts.push(("QUEST".into(), super::dialog::ChoiceAct::Quest));
        }
        if opts.len() > 1 {
            dialog.0 = Some(super::dialog::DialogState::choice(
                v.pname.clone().unwrap_or_default(),
                opts,
                entity,
            ));
            next.set(super::screen::Screen::Dialog);
        } else {
            chat_with(&mut cx, &mut commands, &mut images, &mut v, &world.0, cur.rx, cur.ry, today);
        }
    }

    // --- The chip + bubble over the NEAREST villager in hailing range (js drawNpcChat,
    // 40px "a touch generous so it reads through a shop counter"). ---
    let mut drawn: Option<(String, String, bool, i32, f32, f32)> = None;
    let mut best = f32::INFINITY;
    for (_, v) in villagers.iter().filter(|(_, v)| !v.line.is_empty()) {
        let d = ((v.x + 8.0) - pcx).hypot((v.y + 8.0) - pcy);
        if d < 40.0 && d < best {
            best = d;
            let h = v.pkey.as_ref().and_then(|k| cx.ledger.0.get(k)).map_or(0, |r| people::hearts(r.pts));
            drawn = Some((v.pname.clone().unwrap_or_default(), v.line.clone(), v.chat_t > 0, h, v.x.round(), v.y.round()));
        }
    }
    let key = drawn.as_ref().map(|(n, l, t, h, x, y)| format!("{n}|{l}|{t}|{h}|{x},{y}"));
    if key == *last {
        return;
    }
    *last = key;
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some((pname, line, talking, h, vx, vy)) = drawn else { return };
    let by = PLAY_Y + vy - 13.0;
    if !pname.is_empty() {
        // The name chip — the invitation to talk (gold once you're friends).
        let nw = font::measure(&pname) as f32;
        let nx = (PLAY_X + (vx + 8.0 - nw / 2.0).round()).clamp(PLAY_X + 2.0, PLAY_X + PX_W as f32 - nw - 2.0);
        let ny = if talking { by - 9.0 } else { by + 2.0 };
        commands.spawn((
            Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.75), Vec2::new(nw + 4.0, 8.0)),
            at(nx - 2.0, ny, nw + 4.0, 8.0, layers::CHAT),
            PIXEL_LAYER,
            ChatUi,
        ));
        label(&mut commands, &mut images, &pname, nx, ny + 1.0, tier_color(h), layers::CHAT_TEXT, ChatUi);
    }
    if !talking {
        return; // no press yet — hold the silence
    }
    // THE shared bubble (ui::speech_bubble — the same recipe the wilds' shouts use).
    let w = font::measure(&line) as f32 + 8.0;
    let bx = (PLAY_X + (vx + 8.0 - w / 2.0).round()).clamp(PLAY_X + 2.0, PLAY_X + PX_W as f32 - w - 2.0);
    let (bubble, _) = crate::ui::speech_bubble(&mut commands, &mut images, &line, bx, by, layers::CHAT);
    commands.entity(bubble).insert(ChatUi);
}

/// The little heart drifts up and fades (js Entities.heartFx).
fn heart_tick(mut commands: Commands, mut hearts: Query<(Entity, &mut HeartFx, &mut Transform, &mut Sprite)>) {
    for (e, mut fx, mut tf, mut sprite) in &mut hearts {
        fx.t += 1;
        tf.translation.y += 0.35;
        if fx.t > 30 {
            let a = 1.0 - (fx.t - 30) as f32 / 15.0;
            sprite.color = Color::srgba(1.0, 1.0, 1.0, a.max(0.0));
        }
        if fx.t >= 45 {
            commands.entity(e).despawn();
        }
    }
}
