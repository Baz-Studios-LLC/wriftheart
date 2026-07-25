//! capital_town.rs — the capital's AUTHORED DRESSING (Baz: bespoke everything,
//! one room at a time). A wake system stands each room's props up from a table,
//! exactly the guild-wing idiom: wall pieces flat, floor pieces blocked + sorted.
//! First pass: THE SOUTH GATE — twin crenellated towers in the kingdom's livery,
//! the arch the traveler walks UNDER (canopy z), lampposts on the Royal Way.

use bevy::prelude::*;

use super::battle::RoomActor;
use super::play::{CurRoom, GameWorld, SlideState};
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::gfx::{at, PIXEL_LAYER};

const CAPITAL_PAL: &[(char, u32)] = &[
    ('K', 0x000000),
    ('A', 0x8a8a92), // stone
    ('a', 0xb0b4be), // stone lite
    ('d', 0x6a7078), // mortar
    ('s', 0x545a64), // base course
    ('b', 0x2a4a8a), // the kingdom's blue
    ('y', 0xe8c050), // the kingdom's gold
];

const CP_TOWER: [&str; 44] = [
    "KKKKKKKKKKKKKKKKKKKK",
    "KAAA..AA..AA..AA..AK",
    "KAAA..AA..AA..AA..AK",
    "KAAA..AA..AA..AA..AK",
    "KKKKKKKKKKKKKKKKKKKK",
    "KAaaaaaaaaaaaaaaaaAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAdAAAAdAAAAdAAAAdAK",
    "KAAAAAAAAKKAAAAAAAAK",
    "KAAAAAAAAKKAAAAAAAAK",
    "KAAAAAAAAKKAAAAAAAAK",
    "KAAAAAAAAKKAAAAAAAAK",
    "KAAAAAAAAKKAAAAAAAAK",
    "KAdAAAAdAKKAdAAAAdAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAKKKKKKKKAAAAAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAdAAAbbbbbbbbAAAdAK",
    "KAAAAAbbyyyybbAAAAAK",
    "KAAAAAbbyyyybbAAAAAK",
    "KAAAAAbbbyybbbAAAAAK",
    "KAAAAAbbbyybbbAAAAAK",
    "KAAAAAbbbyybbbAAAAAK",
    "KAdAAAbbbbbbbbAAAdAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAAAAAbbbbbbbbAAAAAK",
    "KAAAAAKKKKKKKKAAAAAK",
    "KAdAAAbdbAbAbAAAAdAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAdAAAAdAAAAdAAAAdAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KAAAAAAAAAAAAAAAAAAK",
    "KKKKKKKKKKKKKKKKKKKK",
    "KssssssssssssssssssK",
    "KssssssssssssssssssK",
    "KssssssssssssssssssK",
];
const CP_ARCH_W: [&str; 18] = [
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "AaaaaaaaaaaaaaaaaaaaaaaaaaaaaA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAdAAAAdAAAAdAAAAdAAAAdAAAAdAA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAyyyy",
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAA..",
    "AAAAAAAAAAAAAAAAAAAA..........",
    "AAAAAAAAAA....................",
    "AAAAAAAAAA....................",
    "..............................",
    "..............................",
    "..............................",
    "..............................",
    "..............................",
    "..............................",
    "..............................",
];
const CP_ARCH_E: [&str; 18] = [
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "AaaaaaaaaaaaaaaaaaaaaaaaaaaaaA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAdAAAAdAAAAdAAAAdAAAAdAAAAdAA",
    "yyyyAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "KKKKKKKKKKKKKKKKKKKKKKKKKKKKKK",
    "..AAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "..........AAAAAAAAAAAAAAAAAAAA",
    "....................AAAAAAAAAA",
    "....................AAAAAAAAAA",
    "..............................",
    "..............................",
    "..............................",
    "..............................",
    "..............................",
    "..............................",
    "..............................",
];
const CP_LAMP: [&str; 22] = [
    "..KKKK..",
    ".KKKKKK.",
    ".KyyyyK.",
    ".KyyyyK.",
    ".KyyyyK.",
    ".KKKKKK.",
    "...KK...",
    "..KKKK..",
    "..KKKK..",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    "...KK...",
    ".KKKKKK.",
    ".KssssK.",
    ".KKKKKK.",
];

#[derive(Component)]
pub struct CapitalProp;

/// (art, x, y, canopy, blocker) per room — indexed by the room's (kx, ky).
type Dress = (&'static [&'static str], f32, f32, bool, Option<(f32, f32, f32, f32)>);

fn dressing(kx: i32, ky: i32) -> &'static [Dress] {
    match (kx, ky) {
        // THE SOUTH GATE (2,4): twin towers flanking the centred mouth, the arch
        // halves meeting over the Royal Way, lamps greeting the traveler inside.
        (2, 4) => &[
            (&CP_TOWER, 102.0, 164.0, false, None), // stand ON the rampart (already solid)
            (&CP_TOWER, 182.0, 164.0, false, None),
            (&CP_ARCH_W, 122.0, 158.0, true, None),
            (&CP_ARCH_E, 152.0, 158.0, true, None),
            (&CP_LAMP, 112.0, 138.0, false, Some((114.0, 156.0, 4.0, 4.0))),
            (&CP_LAMP, 184.0, 138.0, false, Some((186.0, 156.0, 4.0, 4.0))),
        ],
        _ => &[],
    }
}

/// Stand the room's authored props up (hall_wake idiom: slide-sparing sweep).
#[allow(clippy::too_many_arguments)]
pub fn capital_wake(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cur: Res<CurRoom>,
    slide: Res<SlideState>,
    world: Res<GameWorld>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut woke: Local<Option<(i32, i32)>>,
    props: Query<Entity, With<CapitalProp>>,
    parents: Query<&ChildOf>,
) {
    if in_dungeon.0.is_some() || inside.0.is_some() {
        *woke = None;
        return;
    }
    if *woke == Some((cur.rx, cur.ry)) {
        return;
    }
    *woke = Some((cur.rx, cur.ry));
    let outgoing = slide.outgoing_root();
    for e in &props {
        if outgoing.is_some() && parents.get(e).ok().map(|p| p.parent()) == outgoing {
            continue;
        }
        commands.entity(e).despawn();
    }
    let Some((kx, ky)) = world.0.capital_room(cur.rx, cur.ry) else { return };
    for (grid, x, y, canopy, blk) in dressing(kx, ky) {
        let img = images.add(crate::gfx::bake(grid, CAPITAL_PAL));
        let (w, h) = (grid[0].len() as f32, grid.len() as f32);
        if let Some(b) = blk {
            if !blockers.0.contains(b) {
                blockers.0.push(*b);
            }
        }
        // Canopy pieces (the arch) draw ABOVE the hero — you walk under them.
        let z = if *canopy { 8.5 } else { actor_z(y + h) };
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + *x, PLAY_Y + *y, w, h, z),
            PIXEL_LAYER,
            RoomActor,
            CapitalProp,
        ));
    }
}

pub struct CapitalTownPlugin;
impl Plugin for CapitalTownPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy::app::FixedUpdate,
            capital_wake.before(super::play::EndTick).run_if(super::screen::playing),
        );
    }
}
