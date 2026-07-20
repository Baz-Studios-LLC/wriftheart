//! digging.rs — TREASURE MAPS & THE SHOVEL (js readTreasureMap + doDig + digMound):
//! read a weathered chart under open sky and an X lands on a random room 4-10 out
//! (farther rings roll richer threat tiers, max 5 held); the codex world map pins
//! it, the room grows a subtle mound of disturbed earth at the spot (nudged clear
//! of decor, and the nudge is SAVED so pin, mound, and dig agree forever), and a
//! shovel-dig within a tile of the X unearths tiered coin + loot — with a 15%
//! chance the trail continues in another chart. Ordinary ground sometimes coughs
//! up scraps; water swallows every scoop.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::battle::{spawn_burst, GameRng, RoomActor};
use super::gather::{spawn_coin, spawn_pickup};
use super::play::{CurGrid, CurRoom, GameWorld, Player};
use super::room_render::{PLAY_X, PLAY_Y};
use crate::gfx::{at, bake, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use crate::room::{COLS, ROWS};

/// One undug X (js treasureMaps rows) — saved via SaveExtras.
#[derive(Clone, Serialize, Deserialize)]
pub struct TMap {
    pub rx: i32,
    pub ry: i32,
    pub c: i32,
    pub r: i32,
    pub tier: i32,
}

#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct TreasureMaps(pub Vec<TMap>);

const MAX_TMAPS: usize = 5;

/// play.rs routes a map's slot-press here — reading validates (and only then
/// consumes, the js use() veto rule).
#[derive(Message)]
pub struct ReadMap(pub &'static str);

/// A heap of fresh-turned earth over the X (js digMound; glint anim flagged).
#[derive(Component)]
pub struct DigMound;

const MOUND_ART: &[&str] = &[
    "................",
    "................",
    "................",
    "................",
    "................",
    "......cccc......",
    "....ccbbbbcc....",
    "...cbbBBBBbbc...",
    "...dbbbbbbbbd...",
    "..dddddddddddd..",
    "....d..d..d.....",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const MOUND_PAL: &[(char, u32)] = &[
    ('d', 0x2e2012), // dark rim + scattered clods
    ('b', 0x5a3e22), // earthy body
    ('B', 0x6a4a2a), // turned heart
    ('c', 0x7a5a36), // sunlit crown
];

/// Read a chart (js readTreasureMap): overworld, under the cap, a clear dry tile
/// in a plain room 4-10 out — or the chart stays unread (and unconsumed).
#[allow(clippy::too_many_arguments)]
pub fn read_map(
    mut reads: MessageReader<ReadMap>,
    mut rng: ResMut<GameRng>,
    cur: Res<CurRoom>,
    world: Res<GameWorld>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    mut maps: ResMut<TreasureMaps>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
) {
    for ReadMap(id) in reads.read() {
        if in_dungeon.0.is_some() || inside.0.is_some() {
            log.add("map", "READ IT UNDER OPEN SKY", 1, 0xd8c8a0, false, true);
            sfx.write(super::sfx::Sfx("tink"));
            continue;
        }
        if maps.0.len() >= MAX_TMAPS {
            log.add("map", "YOU HOLD MYSTERIES ENOUGH - GO DIG", 1, 0xd8c8a0, false, true);
            sfx.write(super::sfx::Sfx("tink"));
            continue;
        }
        let mut placed = false;
        for _ in 0..24 {
            let ang = rng.0.next_f64() * std::f64::consts::TAU;
            let dist = 4.0 + rng.0.next_f64() * 6.0; // 4-10 rooms out — farther rings, richer tiers
            let tx = cur.rx + (ang.cos() * dist).round() as i32;
            let ty = cur.ry + (ang.sin() * dist).round() as i32;
            if (tx == cur.rx && ty == cur.ry) || (tx == 0 && ty == 0) {
                continue;
            }
            if world.0.is_town(tx, ty)
                || crate::worldgen::World::is_castle(tx, ty)
                || world.0.shard_dungeon_at(tx, ty).is_some()
                || world.0.saltmaze_at(tx, ty)
                || maps.0.iter().any(|m| m.rx == tx && m.ry == ty)
            {
                continue;
            }
            // The room's tiles are deterministic, so the X stays honest (js makeRoom).
            let grid = crate::room::RoomGrid::from_map(&world.0.generate(tx, ty));
            let mut spot = None;
            for _ in 0..40 {
                let cc = 2 + (rng.0.next_f64() * (COLS - 4) as f64) as i32;
                let rr = 2 + (rng.0.next_f64() * (ROWS - 4) as f64) as i32;
                if grid.solid_at((cc * 16 + 8) as f32, (rr * 16 + 8) as f32) {
                    continue;
                }
                if world.0.is_water(tx * COLS + cc, ty * ROWS + rr) {
                    continue; // never under water (or a plank bridge)
                }
                spot = Some((cc, rr));
                break;
            }
            let Some((c, r)) = spot else { continue }; // a drowned room — chart another
            maps.0.push(TMap { rx: tx, ry: ty, c, r, tier: crate::worldgen::World::threat_tier(tx, ty) });
            inv.remove_one(id); // consumed only once the X lands (js return-false veto)
            log.add("map", "THE CHART MARKS AN X - SEE YOUR WORLD MAP", 1, 0xe8b84a, false, true);
            sfx.write(super::sfx::Sfx("itemget"));
            saves.write(super::save::SaveRequest);
            placed = true;
            break;
        }
        if !placed {
            log.add("map", "THE CHART IS TOO SMUDGED TO READ HERE", 1, 0xd8c8a0, false, true);
            sfx.write(super::sfx::Sfx("tink"));
        }
    }
}

/// Stand the mound up when its room arrives (the hall_wake idiom) — nudging the X
/// to clear ground if decor sits on it, and SAVING the nudge (js henLay tail).
#[allow(clippy::too_many_arguments)]
pub fn mound_wake(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cur: Res<CurRoom>,
    sliding: Res<super::play::SlideActive>,
    grid: Res<CurGrid>,
    world: Res<GameWorld>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    farm: Res<super::farm::FarmTiles>,
    blockers: Res<super::room_props::RoomBlockers>,
    mut maps: ResMut<TreasureMaps>,
    mut woke: Local<Option<(i32, i32)>>,
    live: Query<Entity, With<DigMound>>,
) {
    if sliding.0 || in_dungeon.0.is_some() || inside.0.is_some() {
        *woke = None;
        return;
    }
    if *woke == Some((cur.rx, cur.ry)) && !maps.is_changed() {
        return;
    }
    *woke = Some((cur.rx, cur.ry));
    for e in &live {
        commands.entity(e).despawn();
    }
    let room = (cur.rx, cur.ry);
    let Some(tm) = maps.0.iter_mut().find(|m| m.rx == room.0 && m.ry == room.1) else { return };
    let free = |c: i32, r: i32| {
        (1..COLS - 1).contains(&c)
            && (1..ROWS - 1).contains(&r)
            && !grid.0.solid_at((c * 16 + 8) as f32, (r * 16 + 8) as f32)
            && !blockers.0.iter().any(|b| ((c * 16) as f32) < b.0 + b.2 && ((c * 16 + 16) as f32) > b.0 && ((r * 16) as f32) < b.1 + b.3 && ((r * 16 + 16) as f32) > b.1)
            && !world.0.is_water(room.0 * COLS + c, room.1 * ROWS + r)
            && farm.tile(room, c, r).is_none()
    };
    if !free(tm.c, tm.r) {
        'nudge: for rad in 1..=2 {
            for dc in -rad..=rad {
                for dr in -rad..=rad {
                    if free(tm.c + dc, tm.r + dr) {
                        tm.c += dc;
                        tm.r += dr;
                        break 'nudge;
                    }
                }
            }
        }
    }
    let (x, y) = ((tm.c * 16) as f32, (tm.r * 16) as f32);
    let img = images.add(bake(MOUND_ART, MOUND_PAL));
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + x, PLAY_Y + y, 16.0, 16.0, 3.15),
        PIXEL_LAYER,
        RoomActor,
        DigMound,
    ));
}

/// The shovel (js doDig): dig the faced tile — the X pays out, plain dirt mostly
/// holds dirt. Rides the farm.rs slot-press idiom.
#[allow(clippy::too_many_arguments)]
pub fn dig_tool(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    mut rng: ResMut<GameRng>,
    cur: Res<CurRoom>,
    world: Res<GameWorld>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    grid: Res<CurGrid>,
    farm: Res<super::farm::FarmTiles>,
    mut ctx: DigCtx,
    mut players: Query<&mut Player>,
    mounds: Query<Entity, With<DigMound>>,
) {
    let Ok(mut p) = players.single_mut() else { return };
    if in_dungeon.0.is_some() || inside.0.is_some() {
        return;
    }
    let (fc, fr) = front_tile(&p);
    for (i, action) in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4].into_iter().enumerate() {
        if !input.pressed(action) || p.cooldowns[i] > 0 {
            continue;
        }
        let Some(def) = ctx.inv.slots[i].and_then(|uid| ctx.inv.def_of(uid)) else { continue };
        if def.id != "shovel" {
            continue;
        }
        input.consume(action);
        p.cooldowns[i] = def.cooldown;
        p.lock_timer = p.lock_timer.max(8); // js player.lock(8)
        let (gx, gy) = (cur.rx * COLS + fc, cur.ry * ROWS + fr);
        if world.0.is_water(gx, gy) {
            ctx.log.add("dig", "THE WATER SWALLOWS EVERY SCOOP", 1, 0x8ab0d0, false, true);
            ctx.sfx.write(super::sfx::Sfx("splash"));
            continue;
        }
        if grid.0.solid_at((fc * 16 + 8) as f32, (fr * 16 + 8) as f32) || farm.tile((cur.rx, cur.ry), fc, fr).is_some() {
            ctx.sfx.write(super::sfx::Sfx("tink")); // walls resist; never wreck tilled soil
            continue;
        }
        ctx.sfx.write(super::sfx::Sfx("dig"));
        ctx.stats.bump("digs", 1.0);
        spawn_burst(&mut commands, &mut rng, Vec2::new((fc * 16 + 8) as f32, (fr * 16 + 8) as f32), 0x6a4a2a, 8);
        let hit = ctx
            .maps
            .0
            .iter()
            .position(|m| m.rx == cur.rx && m.ry == cur.ry && (m.c - fc).abs() <= 1 && (m.r - fr).abs() <= 1);
        if let Some(idx) = hit {
            // X marks the spot (a tile of forgiveness either way).
            let tm = ctx.maps.0.swap_remove(idx);
            for e in &mounds {
                commands.entity(e).despawn();
            }
            let (bx, by) = ((fc * 16) as f32, (fr * 16) as f32);
            spawn_coin(&mut commands, &mut images, 20 + tm.tier * 25 + (rng.0.next_f64() * 20.0) as i32, bx + 2.0, by);
            for k in 0..2 {
                // Maps pay by tier, but purple stays a deep-map prize (js boost curve).
                let (id, qty) = crate::items::roll_loot(0.25 + tm.tier as f64 * 0.3, 0.0, || rng.0.next_f64());
                spawn_pickup(&mut commands, &mut images, id, qty, bx + 4.0 + k as f32 * 7.0, by + 4.0, true);
            }
            if rng.0.next_f64() < 0.15 {
                spawn_pickup(&mut commands, &mut images, "treasuremap", 1, bx + 8.0, by - 4.0, true); // the trail continues...
            }
            ctx.log.add("dig", "TREASURE UNEARTHED", 1, 0xe8b84a, false, true);
            ctx.sfx.write(super::sfx::Sfx("itemget"));
            ctx.saves.write(super::save::SaveRequest);
        } else if rng.0.next_f64() < 0.08 {
            // Ordinary ground sometimes coughs up scraps.
            let roll = rng.0.next_f64();
            if roll < 0.5 {
                spawn_coin(&mut commands, &mut images, 1 + (rng.0.next_f64() * 5.0) as i32, (fc * 16 + 4) as f32, (fr * 16 + 4) as f32);
            } else {
                let id = if roll < 0.75 { "stone" } else { "fiber" };
                spawn_pickup(&mut commands, &mut images, id, 1, (fc * 16 + 4) as f32, (fr * 16 + 4) as f32, true);
            }
        }
    }
}

/// dig_tool's write-side bundle (the system sits at the 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub struct DigCtx<'w> {
    pub inv: ResMut<'w, crate::inventory::PlayerInv>,
    pub maps: ResMut<'w, TreasureMaps>,
    pub stats: ResMut<'w, super::stats::Stats>,
    pub log: ResMut<'w, super::rewards::LootLog>,
    pub saves: MessageWriter<'w, super::save::SaveRequest>,
    pub sfx: MessageWriter<'w, super::sfx::Sfx>,
}

/// The tile the hero faces (the farm.rs/fishing.rs helper, same math).
fn front_tile(p: &Player) -> (i32, i32) {
    let (dx, dy) = match p.facing {
        crate::actors::hero::Facing::Up => (0, -1),
        crate::actors::hero::Facing::Down => (0, 1),
        crate::actors::hero::Facing::Left => (-1, 0),
        crate::actors::hero::Facing::Right => (1, 0),
    };
    (((p.x + 8.0) / 16.0) as i32 + dx, ((p.y + 12.0) / 16.0) as i32 + dy)
}

pub struct DiggingPlugin;

impl Plugin for DiggingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TreasureMaps>().add_message::<ReadMap>().add_systems(
            bevy::app::FixedUpdate,
            (read_map, mound_wake, dig_tool.after(mound_wake))
                .before(super::play::EndTick)
                .run_if(super::screen::playing),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mound_art_is_rectangular() {
        for row in MOUND_ART {
            assert_eq!(row.len(), 16);
        }
    }
}
