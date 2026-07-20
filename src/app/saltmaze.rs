//! saltmaze.rs — THE KINGSPLITTER QUESTLINE (js: the Choir sanctum). A half-buried
//! salt arch stands in the Saltwastes (one per world, no map pin — the lore books
//! are the map); inside, FIVE maze floors run the Choir's hymnwork: floor 2 DARK
//! HYMNS (lighting.rs already drowns the torchlight), floor 3 THE CHANT (linger
//! and the hymn rises — at full voice a zealot answers), floor 4 THE MIRROR HALLS
//! (every exit walks you back in unless your feet sing the Maze Song: LEFT AND
//! LEFT AND ROUND ABOUT, AND DOWN THE BELLS DARK THROAT IS OUT), and at the
//! bottom THE CHOIRMASTER — past him, the First Bell's altar holds THE
//! KINGSPLITTER: a legendary blade that sings a beam of light at full health.
//! The way home stays sealed until the blade is taken up.

use bevy::prelude::*;

use super::battle::{GameRng, RoomActor};
use super::play::Player;
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Hitbox, HitOnce, Team};
use crate::gfx::{at, PIXEL_LAYER};

/// The half-buried arch in the Saltwastes (press at its mouth).
#[derive(Component)]
pub struct SaltDoor {
    pub x: f32,
    pub y: f32,
}

/// The First Bell's altar (js swordAltar): wait -> rest (the boss fell) -> taken.
#[derive(Component)]
pub struct SwordAltar {
    pub x: f32,
    pub y: f32,
    pub taken: bool,
}

/// The Maze Song's progress (js mirrorStep; transient).
#[derive(Resource, Default)]
pub struct MirrorStep(pub usize);

/// The Chant's rising hymn (js chantT; transient).
#[derive(Resource, Default)]
pub struct ChantClock(pub i32);

pub const CHANT_FRAMES: i32 = 900; // js: ~15s of lingering per zealot

/// The weathered salt arch, generated like the js buildSaltDoor (48x40: speckled
/// salt-stone, the carved bell + clapper, the dark mouth, drifted heaps).
pub fn salt_door_image() -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let (w, h) = (48u32, 40u32);
    let mut img = Image::new_fill(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let put = |img: &mut Image, x: i32, y: i32, hex: u32| {
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
            return;
        }
        if let Ok(px) = img.pixel_bytes_mut(UVec3::new(x as u32, y as u32, 0)) {
            px.copy_from_slice(&[(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255]);
        }
    };
    let (cx, salt_b, salt_dim, bell, mouth) = (24i32, 0xdfe4e6u32, 0xb0bac0u32, 0x8a7444u32, 0x0a0806u32);
    for y in 8..39 {
        let t = if y < 16 { (16 - y) as f32 / 8.0 } else { 0.0 };
        let hw = (18.0 - t * 5.0).round() as i32;
        for x in (cx - hw)..=(cx + hw) {
            put(&mut img, x, y, if (x * 5 + y * 3) % 11 == 0 { salt_dim } else { salt_b });
        }
    }
    // The carved bell (disc), lip, and clapper.
    for dy in -4..=4i32 {
        for dx in -5..=5i32 {
            if (dx * dx) as f32 / 25.0 + (dy * dy) as f32 / 16.0 <= 1.0 {
                put(&mut img, cx + dx, 13 + dy, bell);
            }
        }
    }
    for x in (cx - 6)..=(cx + 6) {
        put(&mut img, x, 17, bell);
    }
    put(&mut img, cx, 18, bell);
    put(&mut img, cx, 19, bell);
    // The dark mouth, widening down.
    for y in 22..=38i32 {
        let t = (y - 22) as f32 / 16.0;
        let hw = (2.0 + t * 4.0).round() as i32;
        for x in (cx - hw)..=(cx + hw) {
            put(&mut img, x, y, mouth);
        }
    }
    // Drifted salt heaps at the foot.
    for (sx, sy) in [(-14i32, 36i32), (15, 37), (-19, 38)] {
        for dy in -2..=2i32 {
            for dx in -3..=3i32 {
                if (dx * dx) as f32 / 9.0 + (dy * dy) as f32 / 4.0 <= 1.0 {
                    put(&mut img, cx + sx + dx, sy + dy, salt_dim);
                }
            }
        }
    }
    img
}

/// The Maze Song (js SONG): read walking in, facing east — LEFT AND LEFT AND
/// ROUND ABOUT, AND DOWN THE BELLS DARK THROAT IS OUT.
pub const SONG: [crate::dungeon::Dir; 4] =
    [crate::dungeon::Dir::N, crate::dungeon::Dir::W, crate::dungeon::Dir::E, crate::dungeon::Dir::S];

/// The Chant floor (js updateWarp tail): linger and the hymn rises — at full
/// voice it calls a zealot to your side.
#[allow(clippy::too_many_arguments)]
fn chant_tick(
    mut commands: Commands,
    mut clock: ResMut<ChantClock>,
    mut mirror: ResMut<MirrorStep>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    mut rng: ResMut<GameRng>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&Player>,
) {
    // The Maze Song only holds while you walk the halls (js mirrorStep reset).
    if in_dungeon.0.as_ref().is_none_or(|run| run.dungeon.cur().gimmick != Some("mirror")) {
        mirror.0 = 0;
    }
    let chanting = in_dungeon
        .0
        .as_ref()
        .is_some_and(|run| run.dungeon.theme.key == "saltmaze" && run.dungeon.cur().gimmick == Some("chant"));
    if !chanting {
        clock.0 = 0;
        return;
    }
    let Ok(p) = players.single() else { return };
    clock.0 += 1;
    if clock.0 == (CHANT_FRAMES as f64 * 0.75) as i32 {
        log.add("chant", "THE HYMN RISES - MOVE ON", 1, 0xe8dfa8, false, true);
    }
    if clock.0 >= CHANT_FRAMES {
        clock.0 = 0;
        let zx = (p.x + if rng.0.next_f64() < 0.5 { -56.0 } else { 56.0 }).clamp(32.0, crate::room::PX_W as f32 - 48.0);
        let zy = (p.y + if rng.0.next_f64() < 0.5 { -40.0 } else { 40.0 }).clamp(32.0, crate::room::PX_H as f32 - 48.0);
        if let Some(idx) = crate::actors::mobs::def_index("cultist") {
            commands.spawn((
                crate::actors::mobs::mob_bundle(idx, zx, zy),
                RoomActor,
                PIXEL_LAYER,
                super::dungeon::DungeonFoe("cultist"),
            ));
        }
        log.add("chant", "A ZEALOT ANSWERS THE HYMN", 1, 0xfc7460, false, true);
        sfx.write(super::sfx::Sfx("warpGo"));
    }
}

// ===================== THE KINGSPLITTER =====================

/// The singing beam (js swordBeam): a slim flying blade of light that pierces a
/// whole line — fired only while the hero is hale.
#[derive(Component)]
pub struct SwordBeamFx {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: i32,
}

#[derive(Message)]
pub struct FireBeam;

const BEAM_ART: &[&str] = &["..WW..", ".WBBW.", "WBBBBW", "WBBBBW", ".WBBW.", "..WW.."];

#[allow(clippy::too_many_arguments)]
fn beam_tick(
    mut commands: Commands,
    mut fires: MessageReader<FireBeam>,
    mut images: ResMut<Assets<Image>>,
    grid: Res<super::play::CurGrid>,
    players: Query<&Player>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    tstats: Res<super::slideout::TreeStats>,
    mut beams: Query<(Entity, &mut SwordBeamFx, &mut Transform, &mut Hitbox)>,
) {
    let Ok(p) = players.single() else { return };
    for _ in fires.read() {
        let (dx, dy) = match p.facing {
            crate::actors::hero::Facing::Up => (0.0, -1.0),
            crate::actors::hero::Facing::Down => (0.0, 1.0),
            crate::actors::hero::Facing::Left => (-1.0, 0.0),
            crate::actors::hero::Facing::Right => (1.0, 0.0),
        };
        let dmg = ((3.0 * (1.0 + tstats.melee)) + 0.5).floor().max(1.0) as i32;
        let (x, y) = (p.x + dx * 8.0, p.y + 1.0 + dy * 8.0);
        let img = images.add(crate::gfx::bake(BEAM_ART, &[('W', 0xffffff), ('B', 0xe8f4ff)]));
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + x + 5.0, PLAY_Y + y + 5.0, 6.0, 6.0, 8.65),
            PIXEL_LAYER,
            RoomActor,
            SwordBeamFx { x, y, vx: dx * 4.2, vy: dy * 4.2, life: 46 },
            // Pierces a whole line: HitOnce keeps it to one bite per foe, and it
            // never dies on impact. (js wriftbane doubling joins with the finale.)
            Combatant { team: Team::Player, hurt_team: Some(Team::Enemy), damage: Some(dmg), persistent: false, knock: 1.0 },
            HitOnce::default(),
            Hitbox { x: x + 4.0, y: y + 4.0, w: 8.0, h: 8.0 },
            super::boss::wriftheart::Wriftbane, // the blade that broke it, singing
        ));
        sfx.write(super::sfx::Sfx("swing"));
    }
    for (e, mut b, mut tf, mut hb) in &mut beams {
        b.x += b.vx;
        b.y += b.vy;
        b.life -= 1;
        *hb = Hitbox { x: b.x + 4.0, y: b.y + 4.0, w: 8.0, h: 8.0 };
        *tf = at(PLAY_X + b.x + 5.0, PLAY_Y + b.y + 5.0, 6.0, 6.0, 8.65);
        if grid.0.box_hits_solid(b.x + 4.0, b.y + 4.0, 8.0, 8.0)
            || b.x < -16.0
            || b.x > crate::room::PX_W as f32
            || b.y < -16.0
            || b.y > crate::room::PX_H as f32
            || b.life <= 0
        {
            commands.entity(e).despawn();
        }
    }
}

/// The altar art (js swordAltar draw, rasterised): the salt-stone pedestal with
/// the bell mark; the blade point-down in the stone once it rests.
const ALTAR_BASE: &[&str] = &[
    "DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD",
    "LLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLL",
    "SSLSSSSSSSSSSSSSSSSSSSSSSSSSLLSS",
    "SSLSSSSSSTTTTTTSSSSSSSSSSSSSLLSS",
    "SSLSSSSSSTTTTTTSSSSSSSSSSSSSLLSS",
    "SSLSSSSSSSSTTSSSSSSSSSSSSSSSLLSS",
    "SSSSSSSSSSSSSSSSSSSSSSSSSSSSSSSS",
    "SSSSSSSSSSSSSSSSSSSSSSSSSSSSSSSS",
    "ssssssssssssssssssssssssssssssss",
    "ssssssssssssssssssssssssssssssss",
];

const BLADE: &[&str] = &[
    ".gGGg.",
    "..GG..",
    "gGGGGg",
    "..WB..",
    "..WB..",
    "..WB..",
    "..WB..",
    "..WB..",
    "..WB..",
    "..WB..",
];

/// Stand the altar up in the sanctum's boss room (dungeon.rs calls on room wake).
pub fn spawn_sword_altar(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    blockers: &mut super::room_props::RoomBlockers,
    rested: bool,
    taken: bool,
) {
    let (x, y) = (9.0 * 16.0 - 8.0, 2.0 * 16.0);
    let img = images.add(crate::gfx::bake(
        ALTAR_BASE,
        &[('D', 0xdfe8ec), ('L', 0xb8c2c6), ('S', 0xb8c2c6), ('s', 0x7e8a90), ('T', 0x8a7444)],
    ));
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + x, PLAY_Y + y - 2.0, 32.0, 10.0, actor_z(y + 12.0)),
        PIXEL_LAYER,
        RoomActor,
        SwordAltar { x, y, taken },
    ));
    if rested && !taken {
        let blade = images.add(crate::gfx::bake(BLADE, &[('g', 0x8a6a2a), ('G', 0xc09a44), ('W', 0xffffff), ('B', 0xe8f0fa)]));
        commands.spawn((
            Sprite::from_image(blade),
            at(PLAY_X + x + 13.0, PLAY_Y + y - 14.0, 6.0, 10.0, actor_z(y + 12.0) + 0.01),
            PIXEL_LAYER,
            RoomActor,
            BladeSprite,
        ));
    }
    let blk = (x, y - 2.0, 32.0, 14.0);
    if !blockers.0.contains(&blk) {
        blockers.0.push(blk);
    }
}

#[derive(Component)]
pub struct BladeSprite;

/// PRESS before the rested blade -> THE FIRST BELL IS YOURS (js doInteract).
#[allow(clippy::too_many_arguments)]
fn altar_interact(
    mut commands: Commands,
    mut input: ResMut<crate::input::ActionState>,
    mut in_dungeon: ResMut<super::dungeon::InDungeon>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut banners: ResMut<super::banners::Banners>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&Player>,
    mut altars: Query<&mut SwordAltar>,
    blades: Query<Entity, With<BladeSprite>>,
) {
    if !input.pressed(crate::input::Action::Interact) {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Some(run) = &mut in_dungeon.0 else { return };
    let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    for mut a in &mut altars {
        if a.taken {
            continue;
        }
        let boss_down = run.dungeon.cur().room(run.drx, run.dry).is_some_and(|r| r.boss_loot);
        let zone = (a.x + 2.0, a.y + 12.0, 28.0, 14.0);
        if hitbox.0 < zone.0 + zone.2 && hitbox.0 + hitbox.2 > zone.0 && hitbox.1 < zone.1 + zone.3 && hitbox.1 + hitbox.3 > zone.1 {
            input.consume(crate::input::Action::Interact);
            if !boss_down {
                log.add("altar", "THE BELL WAITS FOR SILENCE", 1, 0xd8e0e8, false, true);
                sfx.write(super::sfx::Sfx("tink"));
                return;
            }
            if inv.add_item("kingsplitter", 1) {
                inv.auto_equip("kingsplitter");
                a.taken = true;
                for e in &blades {
                    commands.entity(e).despawn();
                }
                banners.interior("THE FIRST BELL IS YOURS");
                log.add("kingsplitter", "THE KINGSPLITTER", 1, 0xffd865, false, true);
                sfx.write(super::sfx::Sfx("levelup"));
                saves.write(super::save::SaveRequest);
            } else {
                log.add("altar", "YOUR HANDS ARE TOO FULL FOR THE BELL", 1, 0xfc8868, false, true);
                sfx.write(super::sfx::Sfx("tink"));
            }
            return;
        }
    }
}

pub struct SaltmazePlugin;

impl Plugin for SaltmazePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MirrorStep>().init_resource::<ChantClock>().add_message::<FireBeam>().add_systems(
            bevy::app::FixedUpdate,
            (chant_tick, beam_tick.before(crate::combat::resolve_combat), altar_interact.before(super::talk::talk_tick))
                .before(super::play::EndTick)
                .run_if(super::screen::playing),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_shapes() {
        for row in ALTAR_BASE {
            assert_eq!(row.len(), 32);
        }
        for row in BLADE {
            assert_eq!(row.len(), 6);
        }
        let img = salt_door_image();
        assert_eq!(img.size(), bevy::math::UVec2::new(48, 40));
    }
}
