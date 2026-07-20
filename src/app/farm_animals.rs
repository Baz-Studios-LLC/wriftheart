//! farm_animals.rs — coops, chickens, barns, and dairy cows (js game.js farm-animal
//! arc, both increments). Raise a COOP or BARN from its kit (place-at-feet — flagged
//! deviation until the blueprint placement system ports), release a CHICKEN beside
//! your coop (4 roosts a yard) or a COW by your barn (3 stalls). Hens LAY AT DAWN:
//! petted-yesterday hens always, neglected ones half the time, absences capped at 3
//! eggs so nothing floods. Pet every animal once a day (a drifting heart says so);
//! a petted cow with a MILK PAIL in the bag gives one pail of fresh milk a day.
//! Everything persists: buildings, every bird and cow, their positions and their
//! affections, in the save's own rows.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::battle::RoomActor;
use super::gather::DAY_LEN;
use super::play::{CurRoom, Player, SlideActive};
use super::room_render::{actor_z, FrameClock, PLAY_X, PLAY_Y};
use crate::gfx::{at, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use crate::room::{PX_H, PX_W};

// --- Art (js entities.js bakes; the vector barn redrawn as a grid). ---
const COOP: [&str; 14] = [
    ".....KKKKKKKKKKKKKKKK......",
    "....KrrrrrrrrrrrrrrrrK.....",
    "...KrrrrrrrrrrrrrrrrrrK....",
    "..KrrrrrrrrrrrrrrrrrrrrK...",
    ".KrrrrrrrrrrrrrrrrrrrrrrK..",
    ".KKKKKKKKKKKKKKKKKKKKKKKK..",
    ".KDDDDDDDDDDDDDDDDDDDDDDK..",
    ".KDWWDDDDKKKKKKDDDDDWWDDK..",
    ".KWeeWDDDKooooKDDDDWeeWDK..",
    ".KWeeWDDDKooooKDDDDWeeWDK..",
    ".KDWWDDDDKooooKDDDDDWWDDK..",
    ".KDDDDDDDKooooKDDDDDDDDDK..",
    ".KDdDdDdDKooooKDdDdDdDdDK..",
    ".KKKKKKKKKKKKKKKKKKKKKKKK..",
];
const COOP_PAL: &[(char, u32)] = &[('r', 0xc83828), ('D', 0x8a6a3a), ('d', 0x6a4a2a), ('W', 0xe8d8a0), ('e', 0x2a2018), ('K', 0x3a2a18), ('o', 0x141008)];
const BARN: [&str; 20] = [
    "..............KKKKKKKKKKKKKKKKKKKK..............",
    "..........KKKKgggggggggggggggggggKKKK...........",
    "......KKKKggggggggggggggggggggggggggKKKK........",
    "..KKKKggggggggggggggggggggggggggggggggggKKKK....",
    ".KggggggggggggggggggggggggggggggggggggggggggK...",
    ".KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK...",
    ".KrRRrRRrRRrRRrRRWWWWWWWWrRRrRRrRRrRRrRRrRRrK...",
    ".KrRRrRRrRRrRRrRWKKKKKKKWRrRRrRRrRRrRRrRRrRRK...",
    ".KrRRrRRrRRrRRrRWKddddDKWRrRRrRRrRRrRRrRRrRRK...",
    ".KrRRrRRrRRrRRrRWKdDDdDKWRrRRrRRrRRrRRrRRrRRK...",
    ".KWWWWWWWWWWWWWWWKdDdDDKWWWWWWWWWWWWWWWWWWWWK...",
    ".KrRRrRRrRRrRRrRWKDddDdKWRrRRrRRrRRrRRrRRrRRK...",
    ".KrRRrRRrRRrRRrRWKdDDddKWRrRRrRRrRRrRRrRRrRRK...",
    ".KrRRrRRrRRrRRrRWKKKKKKKWRrRRrRRrRRrRRrRRrRRK...",
    ".KrRRrRRrRRrRRrRRWWWWWWWWrRRrRRrRRrRRrRRrRRrK...",
    ".KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK...",
    "................................................",
    "................................................",
    "................................................",
    "................................................",
];
const BARN_PAL: &[(char, u32)] = &[('g', 0x4e3a22), ('r', 0xa83024), ('R', 0xc83828), ('W', 0xe8d8a0), ('K', 0x3a2a18), ('D', 0x5a3a20), ('d', 0x33261a)];
const HEN_A: [&str; 8] = ["...r....", "..WWWO..", ".WWWW...", "WWWWW...", "WWWWWW..", ".WWWW...", "..y.y...", "........"];
const HEN_B: [&str; 8] = ["........", "...r....", "..WWW...", ".WWWWO..", "WWWWWW..", ".WWWW...", "..y.y...", "........"];
const HEN_PAL: &[(char, u32)] = &[('W', 0xf4f0e8), ('r', 0xc83828), ('O', 0xe0902a), ('y', 0xc8a030)];
const COW_A: [&str; 10] = [
    ".....WWWWWW.hh..",
    "..WWWWWWWWWWWWh.",
    "..WWKKWWWWWWWWK.",
    "..WKKKWWWWWWWNN.",
    "..WWKKWWWWKKWNN.",
    "..WWWWWWWKKKW...",
    "..WWWWWWWWKW....",
    "...W..pp..W.....",
    "...W..pp..W.....",
    "...W......W.....",
];
const COW_B: [&str; 10] = [
    ".....WWWWWW.hh..",
    "..WWWWWWWWWWWWh.",
    "..WWKKWWWWWWWWK.",
    "..WKKKWWWWWWWNN.",
    "..WWKKWWWWKKWNN.",
    "..WWWWWWWKKKW...",
    "..WWWWWWWWKW....",
    "..W...pp...W....",
    "..W...pp...W....",
    "..W........W....",
];
const COW_PAL: &[(char, u32)] = &[('W', 0xf4f0e8), ('K', 0x2a2a30), ('N', 0xf0b8c8), ('p', 0xf0a8b8), ('h', 0x8a6a3a)];
const HEART: [&str; 5] = [".HH.HH.", "HHHHHHH", "HHHHHHH", ".HHHHH.", "..HHH.."];

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    Hen,
    Cow,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AnimalRec {
    pub kind: Kind,
    pub rx: i32,
    pub ry: i32,
    pub x: f32,
    pub y: f32,
    pub hx: f32, // home anchor (its coop/barn)
    pub hy: f32,
    pub pet_day: i64,
    pub prod_day: i64, // hen: last lay day; cow: last milk day
}

/// Every placed building + every animal (js coops/barns/animals — saved rows).
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct Livestock {
    pub coops: Vec<(i32, i32, f32, f32)>,
    pub barns: Vec<(i32, i32, f32, f32)>,
    pub animals: Vec<AnimalRec>,
}

/// A farm item used from a slot (play.rs forwards; the handler validates + consumes).
#[derive(Message)]
pub struct UseFarmItem(pub &'static str);

/// The wake tracker (room, day) — cleared to force a respawn after placements.
#[derive(Resource, Default)]
struct YardWake(Option<(i32, i32, i64)>);

#[derive(Component)]
struct FarmYard; // any spawned yard piece (buildings + animals + hearts)

#[derive(Component)]
pub(crate) struct YardAnimal {
    idx: usize, // index into Livestock.animals
    tx: f32,
    ty: f32,
    wait: i32,
    step: u32,
    flip: bool,
    hearts: i32,
    heart: Option<Entity>,
}

fn day_of(clock: i64) -> i64 {
    clock / DAY_LEN
}

/// Stand the room's yards up + run the dawn catch-up (js loadRoom + henLay).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn yard_wake(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    clock: Res<FrameClock>,
    cur: Res<CurRoom>,
    sliding: Res<SlideActive>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    mut stock: ResMut<Livestock>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut rng: ResMut<super::battle::GameRng>,
    mut stats: ResMut<super::stats::Stats>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut woke: ResMut<YardWake>,
    yard: Query<Entity, With<FarmYard>>,
) {
    if sliding.0 || in_dungeon.0.is_some() || inside.0.is_some() {
        return;
    }
    let day = day_of(clock.0);
    if woke.0 == Some((cur.rx, cur.ry, day)) {
        return;
    }
    woke.0 = Some((cur.rx, cur.ry, day));
    // Sweep-and-restand (idempotent): fresh rooms were swept by the RoomActor pass,
    // forced re-wakes and day rollovers sweep here so nothing doubles.
    for e in &yard {
        commands.entity(e).despawn();
    }
    {
        for &(rx, ry, x, y) in stock.coops.iter().filter(|c| (c.0, c.1) == (cur.rx, cur.ry)) {
            let _ = (rx, ry);
            let img = images.add(crate::gfx::bake(&COOP, COOP_PAL));
            let blk = (x + 2.0, y + 10.0, 28.0, 20.0);
            if !blockers.0.contains(&blk) {
                blockers.0.push(blk);
            }
            commands.spawn((
                Sprite::from_image(img),
                at(PLAY_X + x + 2.0, PLAY_Y + y + 2.0, 27.0, 14.0, actor_z(y + 28.0)),
                PIXEL_LAYER,
                RoomActor,
                FarmYard,
            ));
        }
        for &(rx, ry, x, y) in stock.barns.iter().filter(|b| (b.0, b.1) == (cur.rx, cur.ry)) {
            let _ = (rx, ry);
            let img = images.add(crate::gfx::bake(&BARN, BARN_PAL));
            let blk = (x + 3.0, y + 14.0, 42.0, 22.0);
            if !blockers.0.contains(&blk) {
                blockers.0.push(blk);
            }
            commands.spawn((
                Sprite::from_image(img),
                at(PLAY_X + x, PLAY_Y + y - 4.0, 48.0, 20.0, actor_z(y + 34.0)),
                PIXEL_LAYER,
                RoomActor,
                FarmYard,
            ));
        }
        let hen_img = images.add(crate::gfx::bake(&HEN_A, HEN_PAL));
        let cow_img = images.add(crate::gfx::bake(&COW_A, COW_PAL));
        for (i, a) in stock.animals.iter().enumerate() {
            if (a.rx, a.ry) != (cur.rx, cur.ry) {
                continue;
            }
            let (img, w, h) = match a.kind {
                Kind::Hen => (hen_img.clone(), 8.0, 8.0),
                Kind::Cow => (cow_img.clone(), 16.0, 10.0),
            };
            commands.spawn((
                Sprite::from_image(img),
                at(PLAY_X + a.x, PLAY_Y + a.y, w, h, actor_z(a.y + h)),
                PIXEL_LAYER,
                RoomActor,
                FarmYard,
                YardAnimal { idx: i, tx: a.x, ty: a.y, wait: 30 + (rng.0.next_f64() * 90.0) as i32, step: 0, flip: false, hearts: 0, heart: None },
            ));
        }
    }
    // --- Dawn: the hens of THIS room lay what they owe (js henLay, cap 3). ---
    let mut laid_total = 0;
    for a in stock.animals.iter_mut().filter(|a| (a.rx, a.ry) == (cur.rx, cur.ry) && a.kind == Kind::Hen) {
        let owed = (day - a.prod_day).min(3);
        if owed <= 0 {
            continue;
        }
        let sure = a.pet_day >= a.prod_day; // she was loved since the last lay
        let mut laid = 0;
        for _ in 0..owed {
            if sure || rng.0.next_f64() < 0.5 {
                laid += 1;
            }
        }
        a.prod_day = day;
        for _ in 0..laid {
            let (ex, ey) = (a.hx + 4.0 + rng.0.next_f64() as f32 * 28.0, a.hy + 30.0 + rng.0.next_f64() as f32 * 14.0);
            let e = super::gather::spawn_pickup(&mut commands, &mut images, "egg", 1, ex, ey, false);
            // Yard eggs keep until you leave the room (js life 100000).
            commands.entity(e).entry::<super::gather::Pickup>().and_modify(|mut p| p.life = 100000);
        }
        laid_total += laid;
    }
    if laid_total > 0 {
        stats.bump("eggs", laid_total as f64);
        let msg = if laid_total > 1 { format!("{laid_total} EGGS IN THE YARD") } else { "AN EGG IN THE YARD".into() };
        log.add("egg", &msg, 1, 0xf4f0e4, false, true);
        sfx.write(super::sfx::Sfx("cluck"));
    }
}

/// Kits raise buildings; baskets release animals (js releaseChicken/releaseCow).
#[allow(clippy::too_many_arguments)]
fn use_farm_item(
    mut msgs: MessageReader<UseFarmItem>,
    clock: Res<FrameClock>,
    cur: Res<CurRoom>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    grid: Res<super::play::CurGrid>,
    blockers: Res<super::room_props::RoomBlockers>,
    mut stock: ResMut<Livestock>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut rng: ResMut<super::battle::GameRng>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut woke: ResMut<YardWake>,
    players: Query<&Player>,
) {
    let Ok(p) = players.single() else { return };
    for m in msgs.read() {
        if in_dungeon.0.is_some() || inside.0.is_some() {
            sfx.write(super::sfx::Sfx("tink"));
            continue;
        }
        let (pcx, pcy) = (p.x + 8.0, p.y + 9.0);
        let day = day_of(clock.0);
        let near = |x: f32, y: f32, r: f32| ((x - pcx).powi(2) + (y - pcy).powi(2)).sqrt() < r;
        match m.0 {
            "coop" | "barn" => {
                // Raise it just above the hero's head; the footprint must be clear ground.
                let (w, h) = if m.0 == "coop" { (32.0, 30.0) } else { (48.0, 40.0) };
                let (bx, by) = ((p.x + 8.0 - w / 2.0).clamp(8.0, PX_W as f32 - w - 8.0), (p.y - h + 2.0).clamp(20.0, PX_H as f32 - h - 8.0));
                let foot = (bx + 2.0, by + 10.0, w - 4.0, h - 12.0);
                let clear = !grid.0.box_hits_solid(foot.0, foot.1, foot.2, foot.3)
                    && !blockers.0.iter().any(|r| r.0 < foot.0 + foot.2 && r.0 + r.2 > foot.0 && r.1 < foot.1 + foot.3 && r.1 + r.3 > foot.1);
                if !clear {
                    log.add("farm", "NO ROOM TO RAISE IT HERE", 1, 0xfc8868, false, true);
                    sfx.write(super::sfx::Sfx("tink"));
                    continue;
                }
                if m.0 == "coop" {
                    stock.coops.push((cur.rx, cur.ry, bx, by));
                    log.add("farm", "THE COOP STANDS READY", 1, 0xffd8a0, false, true);
                } else {
                    stock.barns.push((cur.rx, cur.ry, bx, by));
                    log.add("farm", "THE BARN STANDS READY", 1, 0xffd8a0, false, true);
                }
                inv.remove_one(m.0);
                sfx.write(super::sfx::Sfx("craft"));
                woke.0 = None; // re-wake: the new building (and later its birds) must stand up
            }
            "chicken" => {
                let Some(&(_, _, cx2, cy2)) = stock
                    .coops
                    .iter()
                    .find(|c| (c.0, c.1) == (cur.rx, cur.ry) && near(c.2 + 16.0, c.3 + 16.0, 72.0))
                else {
                    log.add("hen", "RELEASE HER BESIDE YOUR COOP", 1, 0xfc8868, false, true);
                    sfx.write(super::sfx::Sfx("tink"));
                    continue;
                };
                let here = stock.animals.iter().filter(|a| (a.rx, a.ry) == (cur.rx, cur.ry)).count();
                if here >= 4 {
                    log.add("hen", "THE COOP IS FULL - 4 ROOSTS", 1, 0xfc8868, false, true);
                    sfx.write(super::sfx::Sfx("tink"));
                    continue;
                }
                stock.animals.push(AnimalRec {
                    kind: Kind::Hen,
                    rx: cur.rx,
                    ry: cur.ry,
                    x: cx2 + 6.0 + rng.0.next_f64() as f32 * 22.0,
                    y: cy2 + 34.0,
                    hx: cx2,
                    hy: cy2,
                    pet_day: -1,
                    prod_day: day,
                });
                inv.remove_one("chicken");
                log.add("hen", "A HEN SETTLES INTO THE YARD", 1, 0xffd8a0, false, true);
                sfx.write(super::sfx::Sfx("cluck"));
                woke.0 = None;
            }
            "cow" => {
                let Some(&(_, _, bx2, by2)) = stock
                    .barns
                    .iter()
                    .find(|b| (b.0, b.1) == (cur.rx, cur.ry) && near(b.2 + 24.0, b.3 + 24.0, 84.0))
                else {
                    log.add("cow", "LEAD HER BESIDE YOUR BARN FIRST", 1, 0xfc8868, false, true);
                    sfx.write(super::sfx::Sfx("tink"));
                    continue;
                };
                let cows = stock.animals.iter().filter(|a| (a.rx, a.ry) == (cur.rx, cur.ry) && a.kind == Kind::Cow).count();
                if cows >= 3 {
                    log.add("cow", "THE BARN IS FULL - 3 STALLS", 1, 0xfc8868, false, true);
                    sfx.write(super::sfx::Sfx("tink"));
                    continue;
                }
                stock.animals.push(AnimalRec {
                    kind: Kind::Cow,
                    rx: cur.rx,
                    ry: cur.ry,
                    x: bx2 + 8.0 + rng.0.next_f64() as f32 * 30.0,
                    y: by2 + 44.0,
                    hx: bx2,
                    hy: by2,
                    pet_day: -1,
                    prod_day: -1,
                });
                inv.remove_one("cow");
                log.add("cow", "A COW SETTLES IN BY THE BARN", 1, 0xffd8a0, false, true);
                sfx.write(super::sfx::Sfx("moo"));
                woke.0 = None;
            }
            _ => {}
        }
    }
}

/// The yard lives: hens peck their rounds, cows graze theirs; hearts drift and fade.
/// Positions write back to the saved rows every step (js data.x writes).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn yard_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut stock: ResMut<Livestock>,
    mut rng: ResMut<super::battle::GameRng>,
    clock: Res<FrameClock>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut animals: Query<(&mut YardAnimal, &mut Sprite, &mut Transform)>,
    mut hearts: Query<&mut Transform, (With<FarmYard>, Without<YardAnimal>, With<HeartFx>)>,
    mut banks: Local<Option<(Handle<Image>, Handle<Image>, Handle<Image>, Handle<Image>, Handle<Image>)>>,
) {
    let (hen_a, hen_b, cow_a, cow_b, heart) = banks
        .get_or_insert_with(|| {
            (
                images.add(crate::gfx::bake(&HEN_A, HEN_PAL)),
                images.add(crate::gfx::bake(&HEN_B, HEN_PAL)),
                images.add(crate::gfx::bake(&COW_A, COW_PAL)),
                images.add(crate::gfx::bake(&COW_B, COW_PAL)),
                images.add(crate::gfx::bake(&HEART, &[('H', 0xfc6890)])),
            )
        })
        .clone();
    for (mut ya, mut spr, mut tf) in &mut animals {
        let Some(rec) = stock.animals.get_mut(ya.idx) else { continue };
        let (is_hen, w, h) = match rec.kind {
            Kind::Hen => (true, 8.0, 8.0),
            Kind::Cow => (false, 16.0, 10.0),
        };
        if ya.wait > 0 {
            ya.wait -= 1;
        } else {
            let (dx, dy) = (ya.tx - rec.x, ya.ty - rec.y);
            let m = (dx * dx + dy * dy).sqrt();
            if m < 1.5 {
                // Arrived — dally, then pick a fresh spot in the yard (js).
                ya.wait = if is_hen { 60 + (rng.0.next_f64() * 160.0) as i32 } else { 90 + (rng.0.next_f64() * 200.0) as i32 };
                let span = if is_hen { 52.0 } else { 60.0 };
                ya.tx = (rec.hx + (rng.0.next_f64() as f32 * 2.0 - 1.0) * span).clamp(8.0, PX_W as f32 - 16.0);
                ya.ty = (rec.hy + 16.0 + rng.0.next_f64() as f32 * 36.0).clamp(24.0, PX_H as f32 - 16.0);
                if is_hen && rng.0.next_f64() < 0.25 {
                    sfx.write(super::sfx::Sfx("cluck"));
                }
            } else {
                let sp = if is_hen { 0.45 } else { 0.32 };
                rec.x += dx / m * sp;
                rec.y += dy / m * sp;
                ya.flip = dx < 0.0;
                ya.step += 1;
            }
        }
        // Frame: waiting animals idle-blink on the clock; walkers on their steps (js).
        let frame = if ya.wait > 0 { (clock.0 >> 4) & 1 } else { (ya.step as i64 >> 3) & 1 };
        spr.image = match (is_hen, frame) {
            (true, 0) => hen_a.clone(),
            (true, _) => hen_b.clone(),
            (false, 0) => cow_a.clone(),
            (false, _) => cow_b.clone(),
        };
        spr.flip_x = ya.flip;
        *tf = at(PLAY_X + rec.x, PLAY_Y + rec.y, w, h, actor_z(rec.y + h));
        // The affection heart drifts up and fades.
        if ya.hearts > 0 {
            ya.hearts -= 1;
            let hy2 = rec.y - 6.0 - (40 - ya.hearts) as f32 * 0.3;
            match ya.heart {
                Some(he) => {
                    if let Ok(mut ht) = hearts.get_mut(he) {
                        *ht = at(PLAY_X + rec.x + w / 2.0 - 3.5, PLAY_Y + hy2, 7.0, 5.0, 9.4);
                    }
                    if ya.hearts == 0 {
                        commands.entity(he).despawn();
                        ya.heart = None;
                    }
                }
                None => {
                    ya.heart = Some(
                        commands
                            .spawn((
                                Sprite::from_image(heart.clone()),
                                at(PLAY_X + rec.x, PLAY_Y + hy2, 7.0, 5.0, 9.4),
                                PIXEL_LAYER,
                                RoomActor,
                                FarmYard,
                                HeartFx,
                            ))
                            .id(),
                    );
                }
            }
        }
    }
}

#[derive(Component)]
struct HeartFx;

/// PRESS beside an animal: pet her (daily), and a petted cow with a pail gives milk.
#[allow(clippy::too_many_arguments)]
pub(crate) fn pet_tick(
    mut input: ResMut<ActionState>,
    clock: Res<FrameClock>,
    mut stock: ResMut<Livestock>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut stats: ResMut<super::stats::Stats>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&Player>,
    mut animals: Query<&mut YardAnimal>,
) {
    if !input.pressed(Action::Interact) {
        return;
    }
    let Ok(p) = players.single() else { return };
    let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    let today = day_of(clock.0);
    for mut ya in &mut animals {
        let Some(rec) = stock.animals.get_mut(ya.idx) else { continue };
        let (w, h) = if rec.kind == Kind::Hen { (7.0, 6.0) } else { (12.0, 8.0) };
        let ab = (rec.x + 1.0, rec.y + 2.0, w, h);
        if !(hitbox.0 < ab.0 + ab.2 && hitbox.0 + hitbox.2 > ab.0 && hitbox.1 < ab.1 + ab.3 && hitbox.1 + hitbox.3 > ab.1) {
            continue;
        }
        input.consume(Action::Interact);
        match rec.kind {
            Kind::Hen => {
                if rec.pet_day == today {
                    sfx.write(super::sfx::Sfx("cluck"));
                    ya.hearts = 24;
                } else {
                    rec.pet_day = today;
                    ya.hearts = 40;
                    stats.bump("pets", 1.0);
                    log.add("pet", "SHE CLUCKS HAPPILY", 1, 0xfc9ab8, false, true);
                    sfx.write(super::sfx::Sfx("cluck"));
                }
            }
            Kind::Cow => {
                if rec.pet_day != today {
                    rec.pet_day = today;
                    ya.hearts = 40;
                    stats.bump("pets", 1.0);
                    log.add("pet", "SHE LOWS CONTENTEDLY", 1, 0xfc9ab8, false, true);
                    sfx.write(super::sfx::Sfx("moo"));
                } else if inv.has_item("milkpail") && rec.prod_day != today {
                    rec.prod_day = today;
                    ya.hearts = 40;
                    stats.bump("milk", 1.0);
                    inv.add_item("milk", 1);
                    log.add("milk", "A PAIL OF FRESH MILK", 1, 0xf8f4ec, false, true);
                    sfx.write(super::sfx::Sfx("pickup"));
                } else {
                    ya.hearts = 24;
                    sfx.write(super::sfx::Sfx("moo"));
                }
            }
        }
        return;
    }
}

pub struct FarmAnimalsPlugin;
impl Plugin for FarmAnimalsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Livestock>().init_resource::<YardWake>().add_message::<UseFarmItem>().add_systems(
            bevy::app::FixedUpdate,
            (
                yard_wake,
                use_farm_item.after(yard_wake),
                yard_tick.after(yard_wake),
                pet_tick.before(super::talk::talk_tick),
            )
                .before(super::play::EndTick)
                .run_if(super::screen::playing),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_is_rectangular() {
        let check = |name: &str, g: &[&str], w: usize| {
            for (i, r) in g.iter().enumerate() {
                assert_eq!(r.chars().count(), w, "{name} row {i} width");
            }
        };
        check("coop", &COOP, 27);
        check("barn", &BARN, 48);
        check("hen_a", &HEN_A, 8);
        check("hen_b", &HEN_B, 8);
        check("cow_a", &COW_A, 16);
        check("cow_b", &COW_B, 16);
        check("heart", &HEART, 7);
    }
}
