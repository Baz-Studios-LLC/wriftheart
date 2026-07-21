//! sidescroll.rs — THE HIDDEN SIDE-VIEW CHAMBER (js enterSideScroll): the secret
//! stairs' OTHER destination. A gravity room seen from the side — walk, fall,
//! climb LADDERS to the ledges (no jump needed: that's what makes it solvable
//! for everyone), grab the secret cache, and climb back out the exit stairs.
//! The room is exactly the play area (19x13 tiles) — no camera.
//! DEVIATION (flagged, an improvement): the js sends EVERY push-block secret
//! here; the rs port had already built hidden top-down VAULTS as its stand-in,
//! so the two now SPLIT the secrets deterministically (dungeon-seed parity) —
//! half the mazes hide a vault, half hide the chamber. Springboots hop-jumping
//! is flagged until the boots port.

use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::battle::RoomActor;
use super::play::Player;
use super::room_render::{PLAY_X, PLAY_Y};
use crate::gfx::{at, bake, PIXEL_LAYER};
use crate::input::{Action, ActionState};

/// Tiles: '#' solid, 'H' ladder, '.' air (js SIDE_LEVELS[0], markers stripped).
const LEVEL: [&str; 13] = [
    "###################",
    "#.................#",
    "#.................#",
    "#.......H..C......#",
    "#.......H#########.",
    "#.......H.........#",
    "#.......H.........#",
    "#.......H.........#",
    "#.......H.........#",
    "#.......H.........#",
    "#P..S...H.........#",
    "###################",
    "###################",
];

pub struct SideState {
    pub grid: Vec<Vec<char>>,
    pub spawn: (i32, i32),
    pub exit: (i32, i32),
    pub chest: (i32, i32),
    pub room_key: String,
    pub vy: f32,
    pub climb: bool,
}

/// The active chamber (None = top-down world as usual).
#[derive(Resource, Default)]
pub struct SideScroll(pub Option<SideState>);

/// Chambers already looted, by dungeon room key (js sideLooted; saved).
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct SideLooted(pub HashSet<String>);

#[derive(Component)]
pub struct ChamberTile;

#[derive(Component)]
pub struct ChamberChest;

const CHEST_SHUT: &[&str] = &[
    "................",
    "................",
    "................",
    "................",
    "..dddddddddddd..",
    "..dCCCCCCCCCCd..",
    "..BBBBBBBBBBBB..",
    "..BBBBBBBBBBBB..",
    "..BBBBBGGBBBBB..",
    "..BBBBBGGBBBBB..",
    "..BBBBBBBBBBBB..",
    "................",
    "................",
    "................",
    "................",
    "................",
];

/// Build the state + stand the chamber up (the caller already swept the droom).
pub fn enter_side(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    looted: &SideLooted,
    room_key: String,
) -> SideState {
    let mut grid: Vec<Vec<char>> = LEVEL.iter().map(|r| r.chars().collect()).collect();
    let (mut spawn, mut exit, mut chest) = ((1, 10), (4, 10), (11, 3));
    for (r, row) in grid.iter_mut().enumerate() {
        for (c, ch) in row.iter_mut().enumerate() {
            match *ch {
                'P' => {
                    spawn = (c as i32, r as i32);
                    *ch = '.';
                }
                'S' => {
                    exit = (c as i32, r as i32);
                    *ch = '.';
                }
                'C' => {
                    chest = (c as i32, r as i32);
                    *ch = '.';
                }
                _ => {}
            }
        }
    }
    // The chamber walls + platforms + ladders (js drawSideScroll, baked once).
    let wall = images.add(bake(
        &["TTTTTTTTTTTTTTTT", "BBBBBBBpBBBBBBBB", "BBBBBBBpBBBBBBBB", "BBBBBBBpBBBBBBBB", "BBBBBBBpBBBBBBBB", "BBBBBBBpBBBBBBBB", "BBBBBBBpBBBBBBBB", "BBBBBBBpBBBBBBBB", "BBBBBBBpBBBBBBBB", "BBBBBBBpBBBBBBBB", "BBBBBBBpBBBBBBBB", "BBBBBBBpBBBBBBBB", "BBBBBBBpBBBBBBBB", "BBBBBBBpBBBBBBBB", "bbbbbbbbbbbbbbbb", "bbbbbbbbbbbbbbbb"],
        &[('T', 0x3c3f52), ('B', 0x2a2c3a), ('p', 0x22242e), ('b', 0x1a1c26)],
    ));
    let ladder = images.add(bake(
        &["l..........l...", "lrrrrrrrrrrl...", "l..........l...", "l..........l...", "lrrrrrrrrrrl...", "l..........l...", "l..........l...", "lrrrrrrrrrrl...", "l..........l...", "l..........l...", "lrrrrrrrrrrl...", "l..........l...", "l..........l...", "lrrrrrrrrrrl...", "l..........l...", "l..........l..."],
        &[('l', 0x7c5a2c), ('r', 0xa07a3c)],
    ));
    // A dim backdrop over the whole play area (the chamber owns the screen).
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0x0a, 0x0b, 0x12), Vec2::new(crate::room::PX_W as f32, crate::room::PX_H as f32)),
        at(PLAY_X, PLAY_Y, crate::room::PX_W as f32, crate::room::PX_H as f32, 0.5),
        PIXEL_LAYER,
        RoomActor,
        ChamberTile,
    ));
    for (r, row) in grid.iter().enumerate() {
        for (c, ch) in row.iter().enumerate() {
            let (px, py) = ((c * 16) as f32, (r * 16) as f32);
            match *ch {
                '#' => {
                    commands.spawn((
                        Sprite::from_image(wall.clone()),
                        at(PLAY_X + px, PLAY_Y + py, 16.0, 16.0, 1.0),
                        PIXEL_LAYER,
                        RoomActor,
                        ChamberTile,
                    ));
                }
                'H' => {
                    commands.spawn((
                        Sprite::from_image(ladder.clone()),
                        at(PLAY_X + px + 1.0, PLAY_Y + py, 14.0, 16.0, 1.2),
                        PIXEL_LAYER,
                        RoomActor,
                        ChamberTile,
                    ));
                }
                _ => {}
            }
        }
    }
    // Exit stairs (up) — simple salt-grey steps under a dark doorway.
    let stairs = images.add(bake(
        &["..KKKKKKKKKKKK..", "..KKKKKKKKKKKK..", ".CCCCCCCCCCCCCC.", "..cccccccccccc..", "..CCCCCCCCCC....", "...cccccccc.....", "...CCCCCC.......", "....cccc........", "....CC..........", "................", "................", "................", "................", "................", "................", "................"],
        &[('K', 0x08080c), ('C', 0xc4cad4), ('c', 0x9aa0aa)],
    ));
    commands.spawn((
        Sprite::from_image(stairs),
        at(PLAY_X + (exit.0 * 16) as f32, PLAY_Y + (exit.1 * 16) as f32 - 4.0, 16.0, 16.0, 1.3),
        PIXEL_LAYER,
        RoomActor,
        ChamberTile,
    ));
    // The cache chest (dimmed once this dungeon's chamber was looted).
    let taken = looted.0.contains(&room_key);
    let chest_img = images.add(bake(
        CHEST_SHUT,
        if taken {
            &[('d', 0x3a2c14), ('C', 0x5a4a24), ('B', 0x3a2c14), ('G', 0x5a4a24)]
        } else {
            &[('d', 0x3a2410), ('C', 0xb88a2c), ('B', 0x7c5a1c), ('G', 0xfcd000)]
        },
    ));
    commands.spawn((
        Sprite::from_image(chest_img),
        at(PLAY_X + (chest.0 * 16) as f32, PLAY_Y + (chest.1 * 16) as f32, 16.0, 16.0, 1.3),
        PIXEL_LAYER,
        RoomActor,
        ChamberTile,
        ChamberChest,
    ));
    SideState { grid, spawn, exit, chest, room_key, vy: 0.0, climb: false }
}

fn tile(st: &SideState, c: i32, r: i32) -> char {
    if !(0..19).contains(&c) || !(0..13).contains(&r) {
        return '#';
    }
    st.grid[r as usize][c as usize]
}

fn solid_pt(st: &SideState, px: f32, py: f32) -> bool {
    tile(st, (px / 16.0).floor() as i32, (py / 16.0).floor() as i32) == '#'
}

fn ladder_pt(st: &SideState, px: f32, py: f32) -> bool {
    tile(st, (px / 16.0).floor() as i32, (py / 16.0).floor() as i32) == 'H'
}

/// The side-view player box: 8 wide, ~15 tall, feet at the bottom (js ssBoxHit).
fn box_hit(st: &SideState, x: f32, y: f32) -> bool {
    solid_pt(st, x + 4.0, y + 1.0)
        || solid_pt(st, x + 11.0, y + 1.0)
        || solid_pt(st, x + 4.0, y + 8.0)
        || solid_pt(st, x + 11.0, y + 8.0)
        || solid_pt(st, x + 4.0, y + 15.0)
        || solid_pt(st, x + 11.0, y + 15.0)
}

fn grounded(st: &SideState, p: &Player) -> bool {
    solid_pt(st, p.x + 4.0, p.y + 16.0) || solid_pt(st, p.x + 11.0, p.y + 16.0)
}

/// The whole side-view frame (js updateSideScroll): gravity, ladders, the cache,
/// the climb out. Runs INSTEAD of top-down movement (ModeCtx gates play.rs).
#[allow(clippy::too_many_arguments)]
pub fn side_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    mut side: ResMut<SideScroll>,
    mut looted: ResMut<SideLooted>,
    mut rng: ResMut<super::battle::GameRng>,
    tstats: Res<super::slideout::TreeStats>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut clock: Local<i64>,
    mut players: Query<&mut Player>,
    chests: Query<Entity, With<ChamberChest>>,
    mut exits: MessageWriter<ExitSide>,
) {
    let Some(st) = &mut side.0 else { return };
    let Ok(mut p) = players.single_mut() else { return };
    *clock += 1;
    const G: f32 = 0.34;
    const MAXVY: f32 = 5.0;
    const WALK: f32 = 1.3;
    const CLIMB: f32 = 1.1;
    let cx = p.x + 8.0;
    let near_ladder = ladder_pt(st, cx, p.y + 8.0) || ladder_pt(st, cx, p.y + 13.0);
    let (up, down) = (input.held(Action::Up), input.held(Action::Down));
    let (pc, pr) = (((p.x + 8.0) / 16.0).floor() as i32, ((p.y + 10.0) / 16.0).floor() as i32);
    let near_chest = (pc - st.chest.0).abs() <= 1 && (pr - st.chest.1).abs() <= 1;
    // --- ladder climbing (the core traversal) ---
    if st.climb && !near_ladder && !ladder_pt(st, cx, p.y + 15.0) {
        st.climb = false; // climbed clear off the ladder
    }
    if !st.climb && near_ladder && (up || down) {
        st.climb = true;
        p.x = (cx / 16.0).floor() * 16.0; // mount + snap to the ladder column
        st.vy = 0.0;
    }
    let mut vx = 0.0;
    if input.held(Action::Left) {
        vx = -WALK;
        p.facing = crate::actors::hero::Facing::Left;
    } else if input.held(Action::Right) {
        vx = WALK;
        p.facing = crate::actors::hero::Facing::Right;
    }
    if st.climb {
        p.x = (cx / 16.0).floor() * 16.0; // stay locked to the ladder column
        let vy = if up { -CLIMB } else if down { CLIMB } else { 0.0 };
        if vy != 0.0 && !box_hit(st, p.x, p.y + vy) {
            p.y += vy;
        }
        st.vy = 0.0;
        p.facing = crate::actors::hero::Facing::Up;
        if vy != 0.0 && *clock % 8 == 0 {
            p.anim_frame = (p.anim_frame + 1) & 3;
        }
        if vx != 0.0 {
            // Step off sideways — but ONLY onto a real ledge (never through the gap).
            let dir = if vx < 0.0 { -1.0 } else { 1.0 };
            let adj = p.x + dir * 16.0;
            let ledge = solid_pt(st, adj + 4.0, p.y + 16.0)
                || solid_pt(st, adj + 11.0, p.y + 16.0)
                || solid_pt(st, adj + 4.0, p.y + 24.0)
                || solid_pt(st, adj + 11.0, p.y + 24.0);
            if !box_hit(st, adj, p.y) && ledge {
                p.x = adj;
                st.climb = false;
            }
        }
    } else {
        st.vy = (st.vy + G).min(MAXVY); // gravity
        if grounded(st, &p) && st.vy >= 0.0 {
            st.vy = 0.0; // (springboots hop-jump joins when the boots port)
        }
        let mut ny = p.y + st.vy;
        if box_hit(st, p.x, ny) {
            let s = if st.vy > 0.0 { 1.0 } else { -1.0 };
            while box_hit(st, p.x, ny) {
                ny -= s;
            }
            st.vy = 0.0;
        }
        p.y = ny;
        if vx != 0.0 && *clock % 8 == 0 && grounded(st, &p) {
            p.anim_frame = (p.anim_frame + 1) & 3;
        }
    }
    // horizontal move with wall collision (both states — you can walk off a ladder)
    if vx != 0.0 {
        let nx = p.x + vx;
        if !box_hit(st, nx, p.y) {
            p.x = nx;
        }
    }
    p.x = p.x.clamp(0.0, 18.0 * 16.0);
    p.moving = vx != 0.0;
    // --- the cache + the way out ---
    if !looted.0.contains(&st.room_key) && near_chest && input.pressed(Action::Slot1) {
        input.consume(Action::Slot1);
        looted.0.insert(st.room_key.clone());
        let (id, qty) = crate::items::roll_loot(0.9, tstats.luck, || rng.0.next_f64());
        if !inv.add_item(id, qty) {
            super::gather::spawn_pickup(&mut commands, &mut images, id, qty, p.x + 4.0, p.y + 4.0, true, None);
        }
        let coins = 40 + (rng.0.next_f64() * 40.0) as i64;
        inv.money += coins;
        if let Some(def) = crate::items::get(id) {
            log.add("cache", &format!("{} + {} COPPER", def.name.to_uppercase(), coins), 1, 0xe8b84a, false, true);
        }
        for e in &chests {
            commands.entity(e).despawn(); // (re-entry bakes the dimmed lid)
        }
        sfx.write(super::sfx::Sfx("craft"));
        saves.write(super::save::SaveRequest);
    }
    if pc == st.exit.0 && pr == st.exit.1 && input.pressed(Action::Up) {
        input.consume(Action::Up);
        exits.write(ExitSide);
    }
}

/// "Climb out" — dungeon.rs answers (it owns the droom re-stand).
#[derive(Message)]
pub struct ExitSide;

pub struct SideScrollPlugin;

impl Plugin for SideScrollPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SideScroll>().init_resource::<SideLooted>().add_message::<ExitSide>().add_systems(
            bevy::app::FixedUpdate,
            side_tick.before(super::play::EndTick).run_if(super::screen::playing),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_is_rectangular_and_marked() {
        for row in LEVEL {
            assert_eq!(row.len(), 19);
        }
        let joined = LEVEL.join("");
        assert_eq!(joined.matches('P').count(), 1);
        assert_eq!(joined.matches('S').count(), 1);
        assert_eq!(joined.matches('C').count(), 1);
    }
}
