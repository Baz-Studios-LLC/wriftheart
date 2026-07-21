//! app/guildhall.rs — the city guildhall's GAME layer (js game.js increments 1-3):
//! enter the boarded hall (a lit, peaceful guildhall "dungeon" — gen has carried
//! its floors and wing tags all along), find each wing's DONATION ALTAR under its
//! crest banner, and fill the bundle line by line straight from your bag. A filled
//! wing brings its guild home: a toast, a one-time reward, and a city-wide perk.
//! All five home = THE GUILDHALL STANDS WHOLE (the guild seal). Progress is
//! per-city (keyed by the town centre) and rides the save.
//! WIRED PERKS: the Anglers (fish sell x1.5 in their city's market) and the
//! Provisioners (the inn rests you free). FLAGGED: tillers stall / smiths stock /
//! scholars discount perks, the hall steward + desk, wing-room dressing.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::battle::RoomActor;
use super::play::{CurRoom, Player};
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::gfx::{at, PIXEL_LAYER};
use crate::guildhall::{req_matches, wing_progress, WINGS};
use crate::input::{Action, ActionState};
use crate::ui::label;

/// One city's restoration (js guildhalls[key]).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct GuildState {
    pub donated: HashMap<String, Vec<i32>>,
    pub done: Vec<String>,
}

/// Every city's hall, keyed "tx,ty" (the town centre) — saved.
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct GuildLedger(pub HashMap<String, GuildState>);

/// Which city's hall you're inside (js currentHallKey; transient).
#[derive(Resource, Default)]
pub struct CurrentHall(pub Option<String>);

/// The open donation window (js guildDonate; transient).
#[derive(Resource, Default)]
pub struct DonateState(pub Option<(usize, usize)>); // (wing index, cursor)

/// The current city's live perks, refreshed on room change — shops/inns read this.
#[derive(Resource, Default)]
pub struct CityPerks {
    pub fish_mult: f32,
    pub free_inn: bool,
}

/// A wing's donation altar under its crest banner.
#[derive(Component)]
pub struct GuildAltar {
    pub wing: usize,
    pub x: f32,
    pub y: f32,
}

const ALTAR: [&str; 18] = [
    ".KKKKKKKKKK.",
    ".KCCCCCCCCK.",
    ".KCcCCCCcCK.",
    ".KCCWWWWCCK.",
    ".KCcWCCWcCK.",
    ".KCCWWWWCCK.",
    ".KCcCCCCcCK.",
    ".KCCCCCCCCK.",
    "..KCCCCCCK..",
    "...KCCCCK...",
    "....KCCK....",
    ".....KK.....",
    "..KKKKKKKK..",
    ".KDDDDDDDDK.",
    ".KDdDDDDdDK.",
    ".KDDDDDDDDK.",
    "KKKKKKKKKKKK",
    "............",
];

pub fn city_key(world: &crate::worldgen::World, rx: i32, ry: i32) -> Option<String> {
    crate::worldgen::towns::town_site_of(world.seed, rx, ry).map(|s| format!("{},{}", s.tx, s.ty))
}

/// Refresh the current city's perks whenever the room changes.
fn perks_tick(
    cur: Res<CurRoom>,
    world: Res<super::play::GameWorld>,
    ledger: Res<GuildLedger>,
    mut perks: ResMut<CityPerks>,
) {
    if !cur.is_changed() && !ledger.is_changed() {
        return;
    }
    let done = city_key(&world.0, cur.rx, cur.ry)
        .and_then(|k| ledger.0.get(&k).map(|g| g.done.clone()))
        .unwrap_or_default();
    perks.fish_mult = if done.iter().any(|d| d == "anglers") { 1.5 } else { 1.0 };
    perks.free_inn = done.iter().any(|d| d == "provisioners");
}

/// Wing altars stand up with the room (called from spawn_room_chests' wake path).
pub(crate) fn spawn_room_altar(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    blockers: &mut super::room_props::RoomBlockers,
    room: &crate::dungeon::DRoom,
    ledger: &GuildLedger,
    hall: &CurrentHall,
) {
    let Some(gw) = room.gwing else { return };
    let Some(widx) = WINGS.iter().position(|w| w.id == gw) else { return };
    let w = &WINGS[widx];
    let restored = hall
        .0
        .as_ref()
        .and_then(|k| ledger.0.get(k))
        .map(|g| g.done.iter().any(|d| d == w.id))
        .unwrap_or(false);
    let pal: &[(char, u32)] = &[
        ('C', if restored { w.crest } else { 0x4a4a52 }),
        ('c', if restored { 0xffffff } else { 0x6a6a72 }),
        ('D', 0x8a6a3a),
        ('d', 0x6a4a2a),
    ];
    let img = images.add(crate::gfx::bake(&ALTAR, pal));
    let (x, y) = (8.0 * 16.0 + 8.0, 2.0 * 16.0);
    let blk = (x - 1.0, y + 2.0, 14.0, 14.0);
    if !blockers.0.contains(&blk) {
        blockers.0.push(blk);
    }
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + x, PLAY_Y + y, 12.0, 18.0, actor_z(y + 16.0)),
        PIXEL_LAYER,
        RoomActor,
        GuildAltar { wing: widx, x, y },
    ));
}

/// Stand each wing room's altar up when the hall's room changes (self-contained
/// watcher — no wake-site churn); clears CurrentHall once you're back outside.
#[allow(clippy::too_many_arguments)]
fn altar_wake(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    ledger: Res<GuildLedger>,
    mut hall: ResMut<CurrentHall>,
    mut donate: ResMut<DonateState>,
    mut woke: Local<Option<(i32, i32, usize)>>,
    altars: Query<Entity, With<GuildAltar>>,
) {
    let Some(run) = &in_dungeon.0 else {
        if hall.0.is_some() {
            hall.0 = None;
            donate.0 = None;
        }
        *woke = None;
        return;
    };
    if run.dungeon.theme.key != "guildhall" {
        return;
    }
    let key = (run.drx, run.dry, run.dungeon.floor);
    if *woke == Some(key) {
        return;
    }
    *woke = Some(key);
    donate.0 = None;
    for e in &altars {
        commands.entity(e).despawn();
    }
    if let Some(room) = run.dungeon.cur().room(run.drx, run.dry) {
        spawn_room_altar(&mut commands, &mut images, &mut blockers, room, &ledger, &hall);
    }
}

/// PRESS at an altar -> the wing's checklist opens.
fn altar_interact(
    mut input: ResMut<ActionState>,
    mut donate: ResMut<DonateState>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&Player>,
    altars: Query<&GuildAltar>,
) {
    if donate.0.is_some() || !input.pressed(Action::Interact) {
        return;
    }
    let Ok(p) = players.single() else { return };
    let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    for a in &altars {
        let ab = (a.x - 6.0, a.y + 4.0, 24.0, 22.0);
        if hitbox.0 < ab.0 + ab.2 && hitbox.0 + hitbox.2 > ab.0 && hitbox.1 < ab.1 + ab.3 && hitbox.1 + hitbox.3 > ab.1 {
            input.consume(Action::Interact);
            donate.0 = Some((a.wing, 0));
            sfx.write(super::sfx::Sfx("open"));
            return;
        }
    }
}

#[derive(Component)]
struct GuildUi;

/// The checklist window (js updateGuildDonate + drawGuildDonate): up/down picks a
/// line, PRESS donates one matching bag item, filled wings bring the guild home.
#[allow(clippy::too_many_arguments)]
fn donate_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    mut donate: ResMut<DonateState>,
    mut ledger: ResMut<GuildLedger>,
    hall: Res<CurrentHall>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut alloc: ResMut<super::slideout::TreeAlloc>,
    mut rng: ResMut<super::battle::GameRng>,
    mut log: ResMut<super::rewards::LootLog>,
    mut banners: ResMut<super::banners::Banners>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut altars: Query<(&GuildAltar, &mut Sprite)>,
    old_ui: Query<Entity, With<GuildUi>>,
    // Tuple-nested (the flat list sits at Bevy's 16-param cap): the live bindings
    // for the GIVE/CLOSE prompts, the player query, and the mouse pointer.
    extras: (Res<crate::input::Bindings>, Query<&Player>, Res<crate::input::Pointer>),
) {
    let Some((widx, mut cur)) = donate.0 else {
        for e in &old_ui {
            commands.entity(e).despawn();
        }
        return;
    };
    let w = &WINGS[widx];
    let key = hall.0.clone().unwrap_or_else(|| "lost".into());
    let gh = ledger.0.entry(key).or_default();
    let counts = gh.donated.entry(w.id.to_string()).or_insert_with(|| vec![0; w.reqs.len()]);
    let done = gh.done.iter().any(|d| d == w.id);
    let mut dirty = donate.is_changed();
    if input.pressed(Action::Slot2) || input.pressed(Action::Pause) {
        input.consume(Action::Slot2);
        input.consume(Action::Pause);
        donate.0 = None;
        sfx.write(super::sfx::Sfx("open"));
        return;
    }
    // The checklist OWNS the buttons while open (the menus rule): nothing leaks to
    // the ability slots (Baz: B closed the menu on paper but played the flute in
    // practice — flute_tick is ordered after this and finds the press spent).
    for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
        input.consume(a);
    }
    if input.pressed(Action::Up) {
        cur = (cur + w.reqs.len() - 1) % w.reqs.len();
        sfx.write(super::sfx::Sfx("menuMove"));
        dirty = true;
    }
    if input.pressed(Action::Down) {
        cur = (cur + 1) % w.reqs.len();
        sfx.write(super::sfx::Sfx("menuMove"));
        dirty = true;
    }
    // Mouse: hover a requirement highlights it, a click donates to it. Rows mirror the draw
    // (bx+6, by+30+i*16-2, bw-12, 14).
    let mut req_click = false;
    {
        use super::room_render::{PLAY_X, PLAY_Y};
        use crate::room::{PX_H, PX_W};
        let bw = 250.0;
        let bh = 64.0 + w.reqs.len() as f32 * 16.0;
        let bx = PLAY_X + (PX_W as f32 - bw) / 2.0;
        let by = PLAY_Y + (PX_H as f32 - bh) / 2.0;
        for i in 0..w.reqs.len() {
            if extras.2.over(bx + 6.0, by + 30.0 + i as f32 * 16.0 - 2.0, bw - 12.0, 14.0) {
                if extras.2.moved && cur != i {
                    cur = i;
                    sfx.write(super::sfx::Sfx("menuMove"));
                    dirty = true;
                }
                if extras.2.click {
                    cur = i;
                    req_click = true;
                }
            }
        }
    }
    if donate.0 != Some((widx, cur)) {
        donate.0 = Some((widx, cur));
    }
    if (input.pressed(Action::Interact) || input.pressed(Action::MenuConfirm) || req_click) && !done {
        input.consume(Action::Interact);
        input.consume(Action::MenuConfirm);
        let req = &w.reqs[cur];
        if counts[cur] >= req.n {
            sfx.write(super::sfx::Sfx("tink"));
        } else if let Some(id) = inv
            .bag
            .iter()
            .flatten()
            .filter_map(|uid| inv.entry(*uid))
            .map(|e| e.id)
            .find(|id| req_matches(req.matches, id))
        {
            inv.remove_one(id);
            counts[cur] += 1;
            sfx.write(super::sfx::Sfx("craft"));
            dirty = true;
            let (_, _, whole) = wing_progress(w, counts);
            if whole {
                // THE WING IS WHOLE — the guild comes home.
                gh.done.push(w.id.to_string());
                log.add("gh", &format!("{} RETURN TO THE CITY", w.name), 1, w.crest, false, true);
                sfx.write(super::sfx::Sfx("itemget"));
                donate.0 = None;
                for (a, mut s) in &mut altars {
                    if a.wing == widx {
                        s.color = Color::WHITE; // rebake shortcut: the banner lights on re-entry
                    }
                }
                grant_loot(w.id, &mut commands, &mut images, &mut inv, &mut alloc, &mut rng, &mut log, &mut sfx, extras.1.single().ok());
                if gh.done.len() >= WINGS.len() {
                    // THE CAPSTONE: every guild home.
                    if let Some(def) = crate::items::get("guildseal") {
                        inv.add_item(def.id, 1);
                    }
                    banners.interior("THE GUILDHALL STANDS WHOLE");
                    sfx.write(super::sfx::Sfx("levelup"));
                }
            }
            saves.write(super::save::SaveRequest);
        } else {
            log.add("gh", "NOTHING IN YOUR BAG FITS THAT LINE", 1, 0xfc8868, false, true);
            sfx.write(super::sfx::Sfx("tink"));
        }
    }
    if !dirty {
        return;
    }
    // --- Redraw (the shop-window idiom: quads + labels under a GuildUi sweep). ---
    for e in &old_ui {
        commands.entity(e).despawn();
    }
    let done_now = ledger
        .0
        .get(hall.0.as_deref().unwrap_or("lost"))
        .map(|g| g.done.iter().any(|d| d == w.id))
        .unwrap_or(false);
    let counts2 = ledger
        .0
        .get(hall.0.as_deref().unwrap_or("lost"))
        .and_then(|g| g.donated.get(w.id))
        .cloned()
        .unwrap_or_else(|| vec![0; w.reqs.len()]);
    let (bw, bh) = (250.0, 64.0 + w.reqs.len() as f32 * 16.0);
    let bx = PLAY_X + (crate::room::PX_W as f32 - bw) / 2.0;
    let by = PLAY_Y + (crate::room::PX_H as f32 - bh) / 2.0;
    const Z: f32 = crate::gfx::layers::WINDOW;
    let quad = |commands: &mut Commands, c: Color, x: f32, y: f32, qw: f32, qh: f32, z: f32| {
        commands.spawn((Sprite::from_color(c, Vec2::new(qw, qh)), at(x, y, qw, qh, z), PIXEL_LAYER, GuildUi));
    };
    quad(&mut commands, Color::srgba(0.016, 0.024, 0.04, 0.93), bx, by, bw, bh, Z);
    let [cr, cg, cb] = [(w.crest >> 16) as u8, (w.crest >> 8) as u8, w.crest as u8];
    quad(&mut commands, Color::srgb_u8(cr, cg, cb), bx, by, bw, 1.0, Z + 0.01);
    quad(&mut commands, Color::srgb_u8(cr, cg, cb), bx, by + bh - 1.0, bw, 1.0, Z + 0.01);
    let title_w = crate::gfx::font::measure(w.name) as f32;
    label(&mut commands, &mut images, w.name, (bx + (bw - title_w) / 2.0).floor(), by + 6.0, w.crest, Z + 0.02, GuildUi);
    let sub = if done_now { "THE WING IS RESTORED" } else { w.desc };
    let sub_w = crate::gfx::font::measure(sub) as f32;
    label(&mut commands, &mut images, sub, (bx + (bw - sub_w) / 2.0).floor(), by + 16.0, if done_now { 0x7ee08a } else { 0x8a8a92 }, Z + 0.02, GuildUi);
    for (i, req) in w.reqs.iter().enumerate() {
        let y = by + 30.0 + i as f32 * 16.0;
        let full = counts2.get(i).copied().unwrap_or(0) >= req.n;
        let on = i == cur && !done_now;
        if on {
            quad(&mut commands, Color::srgba(0.98, 0.88, 0.66, 0.12), bx + 6.0, y - 2.0, bw - 12.0, 14.0, Z + 0.015);
        }
        let col = if full { 0x7ee08a } else if on { 0xfcfcfc } else { 0xb4b4bc };
        label(&mut commands, &mut images, req.label, bx + 12.0, y + 1.0, col, Z + 0.02, GuildUi);
        let has = !full
            && inv
                .bag
                .iter()
                .flatten()
                .filter_map(|uid| inv.entry(*uid))
                .any(|e| req_matches(req.matches, e.id));
        // LIVE bindings (Baz caught a hardcoded "E GIVE" while interact = F): GIVE is
        // Action::Interact, CLOSE is Slot2 — rebind in CONTROLS and these follow.
        let give_key = extras.0.prompt(Action::Interact, input.pad_present);
        let tag = format!(
            "{}/{}{}",
            counts2.get(i).copied().unwrap_or(0),
            req.n,
            if !full && has { format!("  {give_key} GIVE") } else { String::new() }
        );
        let tw = crate::gfx::font::measure(&tag) as f32;
        let tcol = if full { 0x7ee08a } else if has { 0xffd34d } else { 0x5a5a62 };
        label(&mut commands, &mut images, &tag, bx + bw - 12.0 - tw, y + 1.0, tcol, Z + 0.02, GuildUi);
    }
    let hint = format!(
        "{} GIVE   {} CLOSE",
        extras.0.prompt(Action::Interact, input.pad_present),
        extras.0.prompt(Action::Slot2, input.pad_present)
    );
    let hw = crate::gfx::font::measure(&hint) as f32;
    label(&mut commands, &mut images, &hint, (bx + (bw - hw) / 2.0).floor(), by + bh - 14.0, 0x6c6c74, Z + 0.02, GuildUi);
}

/// The guild's thank-you (js grantGuildLoot; smiths use the loot roll until the
/// procedural forge ports, provisioners feed you potions until cooking lands).
#[allow(clippy::too_many_arguments)]
fn grant_loot(
    id: &str,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    inv: &mut crate::inventory::PlayerInv,
    alloc: &mut super::slideout::TreeAlloc,
    rng: &mut super::battle::GameRng,
    log: &mut super::rewards::LootLog,
    sfx: &mut MessageWriter<super::sfx::Sfx>,
    player: Option<&Player>,
) {
    let (px, py) = player.map(|p| (p.x, p.y)).unwrap_or((144.0, 100.0));
    let drop = |iid: &'static str, qty: i32, commands: &mut Commands, images: &mut Assets<Image>, inv: &mut crate::inventory::PlayerInv| {
        if !inv.add_item(iid, qty) {
            super::gather::spawn_pickup(commands, images, iid, qty, px + 4.0, py + 18.0, false, None);
        }
    };
    match id {
        "tillers" => {
            drop("pumpkinseed", 2, commands, images, inv);
            drop("cranberryseed", 2, commands, images, inv);
            drop("tomatoseed", 2, commands, images, inv);
            log.add("gh", "A PACKET OF RARE SEEDS", 1, 0x7ee08a, false, true);
        }
        "anglers" => {
            drop("luckyhook", 1, commands, images, inv);
            log.add("gh", "THE ANGLERS LUCKY HOOK", 1, 0x7090d8, false, true);
        }
        "smiths" => {
            let (iid, qty) = crate::items::roll_loot(1.6, 0.0, || rng.0.next_f64());
            drop(iid, qty, commands, images, inv);
            log.add("gh", "A MASTERWORK PRIZE", 1, 0xe0903a, false, true);
        }
        "scholars" => {
            alloc.points += 1;
            log.add("gh", "A LESSON LEARNED - +1 SKILL POINT", 1, 0xc878ff, false, true);
            sfx.write(super::sfx::Sfx("levelup"));
        }
        "provisioners" => {
            drop("potion", 2, commands, images, inv);
            drop("greaterpotion", 1, commands, images, inv);
            log.add("gh", "A FEAST FOR THE ROAD", 1, 0xffd34d, false, true);
        }
        _ => {}
    }
}

pub struct GuildhallPlugin;
impl Plugin for GuildhallPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuildLedger>()
            .init_resource::<CurrentHall>()
            .init_resource::<DonateState>()
            .init_resource::<CityPerks>()
            .add_systems(
                bevy::app::FixedUpdate,
                (perks_tick, super::hall_exterior::hall_wake, altar_wake, altar_interact.before(super::talk::talk_tick).after(altar_wake), donate_tick.after(altar_interact).before(super::flute::flute_tick))
                    .before(super::play::EndTick)
                    .run_if(super::screen::playing),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn altar_is_rectangular() {
        for r in ALTAR {
            assert_eq!(r.chars().count(), 12, "altar row width");
        }
    }
}
