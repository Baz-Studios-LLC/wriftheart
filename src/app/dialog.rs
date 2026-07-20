//! dialog.rs — the shared ACTION CHOOSER (js openChoice/UI.listBox) + the GIFT picker
//! (js tryOpenGift/updateGiftPick/drawGiftPick), both under one `Screen::Dialog`.
//!
//! A villager with a gift still to give today offers TALK / GIVE; picking GIVE opens
//! the bag over their name. Loved gifts (+150, their taste category) teach you their
//! tastes; disliked ones (-30) cost a little; anything on their BIRTHDAY counts x4 —
//! the reaction that matters most. One gift a day each (js lastGift).
//!
//! DEVIATIONS (flagged): the js also opens the gift directly (keyboard G / pad ▼ near
//! a villager) — that shortcut joins the bindings later; keeper station options
//! (SHOP/REST in the chooser) join when their menu arms are needed.

use super::codex::calendar_tab::{day_of_season, season_index};
use super::quests::{Quest, QuestKind};
use super::gather::farm_day;
use super::room_render::FrameClock;
use super::screen::Screen;
use super::talk::{chat_with, check_bday_learned, meet_person, spawn_heart, ChatCtx};
use crate::actors::villager::Villager;
use crate::gfx::{at, bake, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::inventory::PlayerInv;
use crate::people;
use crate::ui::{border_strips, label};
use crate::{CANVAS_H, CANVAS_W};
use bevy::prelude::*;

const Z: f32 = crate::gfx::layers::WINDOW; // one popup at a time

/// What a chooser row does when picked.
#[derive(Clone, Copy, PartialEq)]
pub enum ChoiceAct {
    Talk,
    Gift,
    Quest,
}

pub enum DialogState {
    /// The centred option menu (js choiceMenu): a title + a short list.
    Choice {
        title: String,
        opts: Vec<(String, ChoiceAct)>,
        cur: usize,
        target: Entity,
    },
    /// The gift picker (js giftPick): the whole inventory, offered to `target`.
    Gift { target: Entity, cur: usize },
    /// The quest window (js questDialog): a fresh OFFER, or the giver's active quest
    /// (review / turn in / abandon-with-confirm).
    Quest { target: Entity, active_id: Option<u32>, offer: Option<super::quests::Quest>, confirm: bool },
}

impl DialogState {
    pub fn choice(title: String, opts: Vec<(String, ChoiceAct)>, target: Entity) -> Self {
        DialogState::Choice { title, opts, cur: 0, target }
    }
}

/// The open dialog (None outside `Screen::Dialog`).
#[derive(Resource, Default)]
pub struct Dialog(pub Option<DialogState>);

/// The quest window's working set (grouped under Bevy's 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub struct QuestCtx<'w, 's> {
    pub log: ResMut<'w, super::quests::QuestLog>,
    pub giver_done: ResMut<'w, super::quests::GiverDone>,
    pub counter: ResMut<'w, super::quests::QuestCounter>,
    pub cleared: Res<'w, super::encounters::ClearedEncounters>,
    pub progress: ResMut<'w, super::rewards::Progress>,
    pub alloc: ResMut<'w, super::slideout::TreeAlloc>,
    pub tstats: Res<'w, super::slideout::TreeStats>,
    pub players: Query<'w, 's, &'static super::play::Player>,
}

#[derive(Component)]
struct DialogUi;

pub struct DialogPlugin;

impl Plugin for DialogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Dialog>()
            .add_systems(
                bevy::app::FixedUpdate,
                dialog_tick.run_if(in_state(Screen::Dialog)).before(super::play::EndTick),
            )
            .add_systems(OnExit(Screen::Dialog), close_dialog);
    }
}

/// Every giftable held item — the WHOLE inventory, not just loose bag cells (js
/// giftList: an equipped tool is still a fine present).
fn gift_list(inv: &PlayerInv) -> Vec<(u32, &'static str, i32)> {
    inv.entries.iter().filter(|e| e.qty > 0).map(|e| (e.uid, e.id, e.qty)).collect()
}

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn dialog_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<ActionState>,
    bindings: Res<Bindings>,
    mut next: ResMut<NextState<Screen>>,
    mut dialog: ResMut<Dialog>,
    mut cx: ChatCtx,
    mut inv: ResMut<PlayerInv>,
    world: Res<super::play::GameWorld>,
    cur_room: Res<super::play::CurRoom>,
    clock: Res<FrameClock>,
    mut villagers: Query<&mut Villager>,
    mut qx: QuestCtx,
    old: Query<Entity, With<DialogUi>>,
    mut last: Local<Option<String>>,
    ptr: Res<crate::input::Pointer>,
) {
    for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
        state.latch(a);
    }
    let today = farm_day(clock.0); // the dawn day — gifts + hellos share the world clock
    let Some(ds) = &mut dialog.0 else {
        next.set(Screen::Play);
        return;
    };
    // B closes the window — except mid-confirm, where it only cancels the confirm.
    let in_confirm = matches!(ds, DialogState::Quest { confirm: true, .. });
    if (state.pressed(Action::Slot2) || state.pressed(Action::Pause)) && !in_confirm {
        next.set(Screen::Play);
        return;
    }
    match ds {
        DialogState::Choice { title, opts, cur, target } => {
            let n = opts.len();
            if state.pressed(Action::Up) {
                *cur = (*cur + n - 1) % n;
            }
            if state.pressed(Action::Down) {
                *cur = (*cur + 1) % n;
            }
            // Mouse: hover a row highlights it, a click picks it (Baz — NPC option windows).
            let (bx, by, bw, _) = chooser_geom(&bindings, &state, title.as_str(), opts.as_slice());
            let mut clicked = false;
            for i in 0..n {
                if ptr.over(bx + 6.0, by + 16.0 + i as f32 * 13.0 - 2.0, bw - 12.0, 12.0) {
                    if ptr.moved {
                        *cur = i;
                    }
                    if ptr.click {
                        *cur = i;
                        clicked = true;
                    }
                }
            }
            if clicked || state.pressed(Action::Slot1) || state.pressed(Action::Interact) {
                let act = opts[*cur].1;
                let target = *target;
                match act {
                    ChoiceAct::Talk => {
                        if let Ok(mut v) = villagers.get_mut(target) {
                            chat_with(&mut cx, &mut commands, &mut images, &mut v, &world.0, cur_room.rx, cur_room.ry, today);
                        }
                        next.set(Screen::Play);
                        return;
                    }
                    ChoiceAct::Quest => {
                        // js openQuestFor: doing business IS meeting them; their active
                        // quest reopens, else a fresh offer generates off (seed, done).
                        let Ok(v) = villagers.get_mut(target) else {
                            next.set(Screen::Play);
                            return;
                        };
                        meet_person(&mut cx, &mut commands, &mut images, &v, &world.0, cur_room.rx, cur_room.ry, today);
                        let key = super::quests::giver_key(cur_room.rx, cur_room.ry, v.seed);
                        let active = qx.log.0.iter().find(|q| q.giver_key == key).map(|q| q.id);
                        let offer = if active.is_none() {
                            let done = qx.giver_done.0.get(&key).copied().unwrap_or(0);
                            let gctx = super::quests::GenCtx {
                                world: &world.0,
                                cleared: &qx.cleared,
                                log: &qx.log.0,
                                rx: cur_room.rx,
                                ry: cur_room.ry,
                            };
                            let mut q = super::quests::generate(&gctx, v.seed, done);
                            q.giver_key = key;
                            Some(q)
                        } else {
                            None
                        };
                        *ds = DialogState::Quest { target, active_id: active, offer, confirm: false };
                    }
                    ChoiceAct::Gift => {
                        // js tryOpenGift: meeting them first (the record must exist), then
                        // the one-a-day gate — refused with a kindly line.
                        let Ok(mut v) = villagers.get_mut(target) else {
                            next.set(Screen::Play);
                            return;
                        };
                        meet_person(&mut cx, &mut commands, &mut images, &v, &world.0, cur_room.rx, cur_room.ry, today);
                        let gifted =
                            v.pkey.as_deref().and_then(|k| cx.ledger.0.get(k)).is_some_and(|r| r.last_gift == today);
                        if gifted {
                            v.line = "YOU ARE TOO KIND. TOMORROW, MAYBE.".to_string();
                            v.chat_t = 180;
                            next.set(Screen::Play);
                            return;
                        }
                        *ds = DialogState::Gift { target, cur: 0 };
                    }
                }
            }
        }
        DialogState::Quest { target, active_id, offer, confirm } => {
            let target = *target;
            if *confirm {
                // "ABANDON?" — confirm drops it, B backs out one layer (the UI rule).
                if state.pressed(Action::Slot1) || state.pressed(Action::Interact) {
                    if let Some(id) = *active_id
                        && let Some(i) = qx.log.0.iter().position(|q| q.id == id)
                    {
                        let q = qx.log.0.remove(i);
                        cx.log.add("quest", &format!("QUEST ABANDONED: {}", q.title), 1, 0xc87878, false, true);
                    }
                    next.set(Screen::Play);
                    return;
                }
                if state.pressed(Action::Slot2) || state.pressed(Action::Pause) {
                    *confirm = false;
                }
            } else if let Some(off) = offer {
                // A fresh offer: accept fills a log slot (js acceptQuest).
                if (state.pressed(Action::Slot1) || state.pressed(Action::Interact))
                    && qx.log.0.len() < super::quests::QUEST_MAX
                {
                    let mut q = off.clone();
                    qx.counter.0 += 1;
                    q.id = qx.counter.0;
                    cx.log.add("quest", &format!("QUEST ACCEPTED: {}", q.title), 1, 0xa8e0ff, false, true);
                    qx.log.0.push(q);
                    cx.saves.write(super::save::SaveRequest);
                    next.set(Screen::Play);
                    return;
                }
            } else if let Some(id) = *active_id {
                let ready = qx.log.0.iter().find(|q| q.id == id).is_some_and(|q| q.ready(&inv));
                if state.pressed(Action::Slot3) {
                    *confirm = true; // X: abandon (asks first)
                } else if state.pressed(Action::Slot1) || state.pressed(Action::Interact) {
                    if !ready {
                        // not finished yet (js tink)
                    } else if let Some(i) = qx.log.0.iter().position(|q| q.id == id) {
                        let q = qx.log.0.remove(i);
                        turn_in(&mut commands, &mut images, &mut cx, &mut inv, &mut qx, &mut villagers, target, q);
                        next.set(Screen::Play);
                        return;
                    }
                }
            }
        }
        DialogState::Gift { target, cur } => {
            let list = gift_list(&inv);
            if list.is_empty() {
                cx.log.add("gift", "YOUR BAG IS EMPTY", 1, 0x8a8a92, false, true);
                next.set(Screen::Play);
                return;
            }
            *cur = (*cur).min(list.len() - 1);
            if state.pressed(Action::Up) {
                *cur = (*cur + list.len() - 1) % list.len();
            }
            if state.pressed(Action::Down) {
                *cur = (*cur + 1) % list.len();
            }
            // Mouse: hover a gift row highlights it, a click gives it. Rows scroll, so map the
            // visible row back through the same scroll the draw uses.
            let (bx, by, bw, _) = gift_geom(list.len());
            let scroll = (*cur).saturating_sub(3).min(list.len().saturating_sub(GIFT_VIS));
            let mut clicked = false;
            for vi in 0..list.len().min(GIFT_VIS) {
                if ptr.over(bx + 6.0, by + 28.0 + vi as f32 * 14.0 - 2.0, bw - 12.0, 13.0) {
                    if ptr.moved {
                        *cur = scroll + vi;
                    }
                    if ptr.click {
                        *cur = scroll + vi;
                        clicked = true;
                    }
                }
            }
            if clicked || state.pressed(Action::Slot1) || state.pressed(Action::Interact) {
                let (uid, id, _) = list[*cur];
                if let Ok(mut v) = villagers.get_mut(*target) {
                    give(&mut cx, &mut commands, &mut images, &mut inv, &mut v, uid, id, clock.0);
                }
                next.set(Screen::Play);
                return;
            }
        }
    }

    // Redraw only when the picture changes.
    let key = match &dialog.0 {
        Some(DialogState::Choice { title, opts, cur, .. }) => {
            Some(format!("c|{title}|{}|{cur}", opts.len()))
        }
        Some(DialogState::Gift { cur, .. }) => Some(format!("g|{cur}|{}", gift_list(&inv).len())),
        Some(DialogState::Quest { active_id, offer, confirm, .. }) => {
            let ready = active_id
                .and_then(|id| qx.log.0.iter().find(|q| q.id == id))
                .is_some_and(|q| q.ready(&inv));
            let have = active_id.and_then(|id| qx.log.0.iter().find(|q| q.id == id)).map_or(0, |q| q.have(&inv));
            Some(format!("q|{:?}|{}|{confirm}|{ready}|{have}", active_id, offer.is_some()))
        }
        None => None,
    };
    if key == *last {
        return;
    }
    *last = key;
    for e in &old {
        commands.entity(e).despawn();
    }
    match &dialog.0 {
        Some(DialogState::Choice { title, opts, cur, .. }) => {
            draw_chooser(&mut commands, &mut images, &bindings, &state, title, opts, *cur)
        }
        Some(DialogState::Gift { target, cur }) => {
            let pname = villagers.get(*target).ok().and_then(|v| v.pname.clone()).unwrap_or_default();
            draw_gift(&mut commands, &mut images, &bindings, &state, &inv, &pname, *cur)
        }
        Some(DialogState::Quest { active_id, offer, confirm, .. }) => {
            let q = offer.as_ref().or_else(|| active_id.and_then(|id| qx.log.0.iter().find(|q| q.id == id)));
            if let Some(q) = q {
                draw_quest(&mut commands, &mut images, &bindings, &state, &inv, &qx, q, offer.is_some(), *confirm);
            }
        }
        None => {}
    }
}

/// Hand the finished job in (js turnInQuest): fetch materials leave the bag, coin
/// (Greed-scaled) + XP + any item pay out, the giver's tally bumps, and the giver
/// remembers it warmly (+1.5 hearts).
#[allow(clippy::too_many_arguments)] // the transaction's arity
fn turn_in(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    cx: &mut ChatCtx,
    inv: &mut PlayerInv,
    qx: &mut QuestCtx,
    villagers: &mut Query<&mut Villager>,
    target: Entity,
    q: Quest,
) {
    if let QuestKind::Fetch { item, need } = &q.kind {
        for _ in 0..*need {
            inv.remove_one(item);
        }
    }
    let r = &q.reward;
    if r.coin > 0 {
        inv.money += (r.coin as f64 * (1.0 + qx.tstats.coin)).round() as i64;
    }
    if r.xp > 0 {
        super::rewards::gain_xp(&mut qx.progress, &mut qx.alloc, r.xp);
    }
    let mut item_msg = String::new();
    if let Some((id, qty)) = &r.item
        && let Some(def) = crate::items::get(id)
    {
        item_msg = format!("  +{qty} {}", def.name.to_uppercase());
        if inv.can_add(def.id) {
            inv.add_item(def.id, *qty);
            inv.auto_equip(def.id);
        } else if let Ok(p) = qx.players.single() {
            // Bag full — the reward waits at your feet (js drop).
            super::gather::spawn_pickup(commands, images, def.id, *qty, p.x, p.y, true);
        }
    }
    *qx.giver_done.0.entry(q.giver_key.clone()).or_insert(0) += 1;
    // A quest done in their name is worth a season of small talk (js +150 pts).
    if let Ok(v) = villagers.get_mut(target)
        && let Some(pkey) = v.pkey.as_deref()
        && let Some(rec) = cx.ledger.0.get_mut(pkey)
    {
        rec.pts = (rec.pts + 150).min(crate::people::MAX_PTS);
        spawn_heart(commands, images, v.x + 5.0, v.y - 5.0);
        let pname = v.pname.clone().unwrap_or_default();
        cx.log.add("talk", &format!("{pname} WONT FORGET THIS"), 1, 0xfc9ab8, false, true);
        check_bday_learned(rec, &mut cx.log);
    }
    cx.stats.bump("quests", 1.0);
    cx.log.add(
        "quest",
        &format!("REWARD: {}C  {}XP{item_msg}", r.coin, r.xp),
        1,
        0xffd34d,
        false,
        true,
    );
    cx.saves.write(super::save::SaveRequest);
}

/// Word-wrap `text` to `max_w` px (js wrapText).
fn wrap_text(text: &str, max_w: i32) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let probe = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if font::measure(&probe) > max_w && !line.is_empty() {
            out.push(line);
            line = word.to_string();
        } else {
            line = probe;
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

/// The quest window (js drawQuestDialog): gold-framed title/desc/GOAL/REWARD + the
/// prompt bar; the abandon-confirm swaps in a small red box.
#[allow(clippy::too_many_arguments)] // a full-window draw's arity
fn draw_quest(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    bindings: &Bindings,
    state: &ActionState,
    inv: &PlayerInv,
    qx: &QuestCtx,
    q: &Quest,
    is_offer: bool,
    confirm: bool,
) {
    let pad_dev = state.pad_present;
    let a = bindings.prompt(Action::Slot1, pad_dev);
    let b = bindings.prompt(Action::Slot2, pad_dev);
    let x_key = bindings.prompt(Action::Slot3, pad_dev);
    let w = 220.0;
    let pad = 8.0;
    let cxm = |bx: f32| bx + w / 2.0;
    if confirm {
        let h = 40.0;
        let bx = ((CANVAS_W as f32 - w) / 2.0).round();
        let by = ((CANVAS_H as f32 - h) / 2.0).round();
        fill(commands, bx, by, w, h, Color::srgba(0.0, 0.0, 0.0, 0.92), Z);
        for (sx, sy, sw, sh) in border_strips(bx, by, w, h, 1.0) {
            fill(commands, sx, sy, sw, sh, Color::srgb_u8(0xc8, 0x78, 0x78), Z + 0.01);
        }
        let t = format!("ABANDON \"{}\"?", q.title);
        let tw = font::measure(&t) as f32;
        label(commands, images, &t, (cxm(bx) - tw / 2.0).round(), by + 10.0, 0xf0c0c0, Z + 0.04, DialogUi);
        let p = format!("{a} YES   {b} NO");
        let pw = font::measure(&p) as f32;
        label(commands, images, &p, (cxm(bx) - pw / 2.0).round(), by + h - 12.0, 0xbfb9a0, Z + 0.04, DialogUi);
        return;
    }
    let lines = wrap_text(&q.desc, (w - pad * 2.0) as i32);
    let ready = if is_offer { true } else { q.ready(inv) };
    let h = 30.0 + lines.len() as f32 * 8.0 + 34.0;
    let bx = ((CANVAS_W as f32 - w) / 2.0).round();
    let by = ((CANVAS_H as f32 - h) / 2.0).round();
    fill(commands, bx, by, w, h, Color::srgba(0.0, 0.0, 0.0, 0.92), Z);
    for (sx, sy, sw, sh) in border_strips(bx, by, w, h, 1.0) {
        fill(commands, sx, sy, sw, sh, Color::srgb_u8(0xff, 0xd3, 0x4d), Z + 0.01);
    }
    label(commands, images, &q.title, bx + pad, by + 6.0, 0xffd34d, Z + 0.04, DialogUi);
    fill(commands, bx + pad, by + 15.0, w - pad * 2.0, 1.0, Color::srgb_u8(0x3a, 0x3a, 0x44), Z + 0.02);
    let mut yy = by + 20.0;
    for ln in &lines {
        label(commands, images, ln, bx + pad, yy, 0xe6e6ee, Z + 0.04, DialogUi);
        yy += 8.0;
    }
    yy += 3.0;
    let have = if is_offer { 0 } else { q.have(inv) };
    let goal = format!("GOAL: {}", q.progress_text(have));
    label(commands, images, &goal, bx + pad, yy, if ready { 0x7ee08a } else { 0xc8c8d0 }, Z + 0.04, DialogUi);
    yy += 9.0;
    let mut rew = format!("REWARD: {}C   {}XP", q.reward.coin, q.reward.xp);
    if let Some((id, qty)) = &q.reward.item {
        let name = crate::items::get(id).map_or(id.as_str(), |d| d.name);
        rew.push_str(&format!("   +{qty} {name}"));
    }
    label(commands, images, &rew, bx + pad, yy, 0xfce0a8, Z + 0.04, DialogUi);
    let prompt = if is_offer {
        if qx.log.0.len() < super::quests::QUEST_MAX {
            format!("{a} ACCEPT   {b} DECLINE")
        } else {
            format!("LOG FULL   {b} CLOSE")
        }
    } else if ready {
        format!("{a} TURN IN   {x_key} ABANDON   {b} CLOSE")
    } else {
        format!("{x_key} ABANDON   {b} CLOSE")
    };
    let pw = font::measure(&prompt) as f32;
    label(commands, images, &prompt, (cxm(bx) - pw / 2.0).round(), by + h - 12.0, 0xbfb9a0, Z + 0.04, DialogUi);
}

/// The gift itself (js updateGiftPick's confirm arm): score it, spend it, react.
#[allow(clippy::too_many_arguments)] // the transaction's arity
fn give(
    cx: &mut ChatCtx,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    inv: &mut PlayerInv,
    v: &mut Villager,
    uid: u32,
    id: &str,
    clock: i64,
) {
    let Some(pkey) = v.pkey.as_deref() else { return };
    let pname = v.pname.clone().unwrap_or_default();
    let taste = people::taste_for(v.seed);
    let bday = people::birthday_for(v.seed);
    let is_bday = bday.season as usize == season_index(clock) && bday.day as i64 == day_of_season(clock);
    let base = people::gift_pts(crate::items::get(id).map(|d| d.kind), &taste);
    let pts = if is_bday && base > 0 { base * people::BDAY_MULT } else { base };
    inv.remove_entry(uid); // consume the exact instance chosen (equipped or bag)
    let Some(rec) = cx.ledger.0.get_mut(pkey) else { return };
    rec.pts = (rec.pts + pts).clamp(0, people::MAX_PTS);
    rec.last_gift = farm_day(clock);
    rec.know_bday = rec.know_bday || is_bday; // gifting on the day, you clearly know it
    cx.stats.bump("gifts", 1.0);
    if is_bday && pts > 0 {
        // Birthday gift — the reaction that matters most.
        if base >= 150 {
            rec.know_love = true;
        }
        v.line = "ON MY BIRTHDAY? YOU REMEMBERED!".to_string();
        for (hx, hy) in [(1.0, -5.0), (6.0, -8.0), (11.0, -5.0)] {
            spawn_heart(commands, images, v.x + hx, v.y + hy);
        }
        cx.log.add("gift", &format!("{pname} IS OVERJOYED - HAPPY BIRTHDAY!"), 1, 0xfc5878, false, true);
    } else if pts >= 150 {
        // Straight to their heart — and now you KNOW.
        rec.know_love = true;
        v.line = format!("FOR ME? {} - MY FAVORITE!", people::taste_word(taste.love));
        for (hx, hy) in [(2.0, -5.0), (9.0, -7.0)] {
            spawn_heart(commands, images, v.x + hx, v.y + hy);
        }
        cx.log.add("gift", &format!("{pname} LOVES IT"), 1, 0xfc5878, false, true);
    } else if pts < 0 {
        rec.know_hate = true;
        v.line = if is_bday { "ON MY BIRTHDAY, OF ALL DAYS..." } else { "OH. YOU REALLY SHOULD NOT HAVE." }.to_string();
        cx.log.add("gift", &format!("{pname} GRIMACES"), 1, 0x8a8a92, false, true);
    } else {
        v.line = "HOW THOUGHTFUL OF YOU. THANK YOU.".to_string();
        spawn_heart(commands, images, v.x + 5.0, v.y - 5.0);
        cx.log.add("gift", &format!("{pname} THANKS YOU"), 1, 0xfc9ab8, false, true);
    }
    check_bday_learned(rec, &mut cx.log);
    v.chat_t = 220;
    cx.saves.write(super::save::SaveRequest);
}

fn fill(commands: &mut Commands, x: f32, y: f32, w: f32, h: f32, color: Color, z: f32) {
    commands.spawn((Sprite::from_color(color, Vec2::new(w, h)), at(x, y, w, h, z), PIXEL_LAYER, DialogUi));
}

/// The chooser box rect (bx, by, bw, bh) — ONE geometry source for `draw_chooser` and the
/// mouse hit-test, so a click lands on exactly the row that's drawn. Row `i`'s hit rect is
/// `(bx + 6, by + 16 + i*13 - 2, bw - 12, 12)`.
fn chooser_geom(bindings: &Bindings, state: &ActionState, title: &str, opts: &[(String, ChoiceAct)]) -> (f32, f32, f32, f32) {
    let pad = state.pad_present;
    let hint = format!("{} SELECT - {} BACK", bindings.prompt(Action::Interact, pad), bindings.prompt(Action::Slot2, pad));
    let mut bw = (font::measure(&hint) as f32 + 16.0).max(120.0).max(font::measure(title) as f32 + 16.0);
    for (o, _) in opts {
        bw = bw.max(font::measure(o) as f32 + 24.0);
    }
    let bh = 6.0 + 10.0 + opts.len() as f32 * 13.0 + 12.0;
    let bx = ((CANVAS_W as f32 - bw) / 2.0).round();
    let by = ((CANVAS_H as f32 - bh) / 2.0).round();
    (bx, by, bw, bh)
}

/// How many gift rows show at once (js giftPick window).
const GIFT_VIS: usize = 7;

/// The gift-picker box rect (bx, by, bw, bh) — shared by `draw_gift` and the hit-test. The
/// visible row `vi` (0..GIFT_VIS) sits at `(bx + 6, by + 28 + vi*14 - 2, bw - 12, 13)`.
fn gift_geom(list_len: usize) -> (f32, f32, f32, f32) {
    let bw = 200.0;
    let bh = 54.0 + list_len.min(GIFT_VIS) as f32 * 14.0;
    let bx = ((CANVAS_W as f32 - bw) / 2.0).round();
    let by = ((CANVAS_H as f32 - bh) / 2.0).round();
    (bx, by, bw, bh)
}

/// js UI.listBox: a centred boxed list, auto-sized to its widest line.
fn draw_chooser(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    bindings: &Bindings,
    state: &ActionState,
    title: &str,
    opts: &[(String, ChoiceAct)],
    cur: usize,
) {
    let pad = state.pad_present;
    let hint = format!("{} SELECT - {} BACK", bindings.prompt(Action::Interact, pad), bindings.prompt(Action::Slot2, pad));
    let (bx, by, bw, bh) = chooser_geom(bindings, state, title, opts);
    fill(commands, bx, by, bw, bh, Color::srgba(0.016, 0.024, 0.04, 0.95), Z);
    for (sx, sy, sw, sh) in border_strips(bx, by, bw, bh, 1.0) {
        fill(commands, sx, sy, sw, sh, Color::srgb_u8(0xcf, 0xe0, 0xff), Z + 0.01);
    }
    let cxm = bx + bw / 2.0;
    let center = |c: &mut Commands, i: &mut Assets<Image>, t: &str, y: f32, col: u32, z: f32| {
        let w = font::measure(t) as f32;
        label(c, i, t, (cxm - w / 2.0).round(), y, col, z, DialogUi);
    };
    center(commands, images, title, by + 6.0, 0xfcfcfc, Z + 0.04);
    let hy = by + 16.0;
    for (i, (o, _)) in opts.iter().enumerate() {
        let y = hy + i as f32 * 13.0;
        if i == cur {
            fill(commands, bx + 6.0, y - 2.0, bw - 12.0, 12.0, Color::srgba(0.81, 0.88, 1.0, 0.14), Z + 0.02);
            for (sx, sy, sw, sh) in border_strips(bx + 6.0, y - 2.0, bw - 12.0, 12.0, 1.0) {
                fill(commands, sx, sy, sw, sh, Color::srgb_u8(0xcf, 0xe0, 0xff), Z + 0.03);
            }
        }
        center(commands, images, o, y, if i == cur { 0xfcfcfc } else { 0xb4b4bc }, Z + 0.04);
    }
    center(commands, images, &hint, by + bh - 9.0, 0x606060, Z + 0.04);
}

/// js drawGiftPick: the bag over their name, pink-framed.
#[allow(clippy::too_many_arguments)] // a full-window draw's arity
fn draw_gift(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    bindings: &Bindings,
    state: &ActionState,
    inv: &PlayerInv,
    pname: &str,
    cur: usize,
) {
    let list = gift_list(inv);
    let (bx, by, bw, bh) = gift_geom(list.len());
    fill(commands, bx, by, bw, bh, Color::srgba(0.016, 0.024, 0.04, 0.95), Z);
    for (sx, sy, sw, sh) in border_strips(bx, by, bw, bh, 1.0) {
        fill(commands, sx, sy, sw, sh, Color::srgb_u8(0xfc, 0x9a, 0xb8), Z + 0.01);
    }
    let cxm = bx + bw / 2.0;
    let center = |c: &mut Commands, i: &mut Assets<Image>, t: &str, y: f32, col: u32, z: f32| {
        let w = font::measure(t) as f32;
        label(c, i, t, (cxm - w / 2.0).round(), y, col, z, DialogUi);
    };
    center(commands, images, pname, by + 6.0, 0xfcfcfc, Z + 0.04);
    center(commands, images, "OFFER A GIFT", by + 16.0, 0x8a8a92, Z + 0.04);
    let scroll = cur.saturating_sub(3).min(list.len().saturating_sub(GIFT_VIS));
    for (i, (_, id, qty)) in list.iter().skip(scroll).take(GIFT_VIS).enumerate() {
        let y = by + 28.0 + i as f32 * 14.0;
        let on = scroll + i == cur;
        if on {
            fill(commands, bx + 6.0, y - 2.0, bw - 12.0, 13.0, Color::srgba(0.988, 0.604, 0.722, 0.12), Z + 0.02);
            for (sx, sy, sw, sh) in border_strips(bx + 6.0, y - 2.0, bw - 12.0, 13.0, 1.0) {
                fill(commands, sx, sy, sw, sh, Color::srgb_u8(0xfc, 0x9a, 0xb8), Z + 0.03);
            }
        }
        if let Some(def) = crate::items::get(id) {
            commands.spawn((
                Sprite::from_image(images.add(bake(def.icon, def.icon_pal))),
                at(bx + 10.0, y, 8.0, 8.0, Z + 0.04),
                PIXEL_LAYER,
                DialogUi,
            ));
        }
        let name = crate::items::get(id).map_or(*id, |d| d.name).to_uppercase();
        label(commands, images, &name, bx + 22.0, y + 1.0, if on { 0xfcfcfc } else { 0xb4b4bc }, Z + 0.04, DialogUi);
        let q = format!("X{qty}");
        let qw = font::measure(&q) as f32;
        label(commands, images, &q, bx + bw - 10.0 - qw, y + 1.0, 0x5a5a62, Z + 0.04, DialogUi);
    }
    if scroll > 0 {
        label(commands, images, "<", bx + bw - 10.0, by + 24.0, 0xfc9ab8, Z + 0.04, DialogUi);
    }
    if scroll + GIFT_VIS < list.len() {
        label(commands, images, ">", bx + bw - 10.0, by + bh - 18.0, 0xfc9ab8, Z + 0.04, DialogUi);
    }
    let pad = state.pad_present;
    let hint = format!("{} GIVE - {} CLOSE", bindings.prompt(Action::Slot1, pad), bindings.prompt(Action::Slot2, pad));
    center(commands, images, &hint, by + bh - 12.0, 0x8a8a92, Z + 0.04);
}

fn close_dialog(mut commands: Commands, mut dialog: ResMut<Dialog>, old: Query<Entity, With<DialogUi>>) {
    dialog.0 = None;
    for e in &old {
        commands.entity(e).despawn();
    }
}
