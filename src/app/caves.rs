//! caves.rs — CRACKED WALLS & SECRET CAVES (js world.js placement + game.js arc):
//! worldgen has seeded a fissured wall section in ~7% of rooms all along; bomb it
//! or pick it open and a CAVE DOOR is carved there forever (CrackCaves rides the
//! save). What's behind it is a deterministic hash roll (js verbatim): 22% a
//! HIDDEN SHOP, 38% a MINI CAVE (5-6 rooms, an elite mini-boss), 40% a full
//! UNDERGROUND CAVERN (crystal/fungal/lava/dark/frost — bonus dungeons, no shard).
//! The BOMB lives here too: drop, back away, a wide neutral blast (Team::Hazard)
//! that hurts foes AND you, and shatters cracked walls without a pickaxe.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::battle::RoomActor;
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Hitbox, HitOnce, Team};
use crate::gfx::{at, bake, PIXEL_LAYER};

/// The fissure overlay (js CRACKROCK_BMP): a jagged black crack + light edge,
/// transparent elsewhere — it rides on top of the wall tile's art.
pub const CRACK_ART: &[&str] = &[
    "................",
    ".......K........",
    ".......K........",
    "......KA........",
    "......K.........",
    ".....KA.........",
    ".....K..........",
    "....KKA.........",
    ".....K..........",
    ".....KA.........",
    "......K.........",
    "......KA........",
    ".......K........",
    ".......K........",
    "................",
    "................",
];

/// Every cave door ever opened: "rx,ry" -> [(c, r, dest)] (js crackCaves; saved).
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct CrackCaves(pub HashMap<String, Vec<(i32, i32, String)>>);

impl CrackCaves {
    pub fn opened(&self, room: (i32, i32), c: i32, r: i32) -> bool {
        self.0
            .get(&format!("{},{}", room.0, room.1))
            .is_some_and(|v| v.iter().any(|(oc, or, _)| *oc == c && *or == r))
    }
}

/// Songstones sung open, by room key "rx,ry" (js openedSongstones; saved).
#[derive(Resource, Default, Clone, Serialize, Deserialize)]
pub struct OpenedSongstones(pub bevy::platform::collections::HashSet<String>);

/// A carved standing stone only the SONG OF OPENING unseals (js songstone).
#[derive(Component)]
pub struct Songstone {
    pub x: f32,
    pub y: f32,
    pub dest: String,
}

/// The flute's opening verse rang out — caves.rs answers (js song.id 'opening').
#[derive(Message)]
pub struct OpeningSung {
    pub mana: i32,
}

/// js SONGSTONE_BMP: a 2-tile carved monolith, mossy at the foot (16x24, drawn 8 up).
pub const SONGSTONE_ART: &[&str] = &[
    ".....KKKKK......",
    "....KSSSSSK.....",
    "...KSWSSSSsK....",
    "...KSSSSSSsK....",
    "..KSSKKKSSSsK...",
    "..KSSK.KSSSsK...",
    "..KSWSKSSSSsK...",
    "..KSSSSSSSSsK...",
    "..KSSKKKKSSsK...",
    "..KSSSSSSSssK...",
    "..KSWSSSSSSsK...",
    "..KSSSSKKSSsK...",
    "..KSSSSSSSssK...",
    "..KMSSSSSSSsK...",
    "..KSSWSSSSMsK...",
    "..KSSSSSSSssK...",
    "..KMSSSSSSssK...",
    "..KSSSSSMSSsK...",
    ".KSSWSSSSSSssK..",
    ".KMSSSSSSSSssK..",
    ".KSSSSSSSMSssK..",
    "KMMSSSSSSSSsssK.",
    "KSSSSMSSSSSMssK.",
    ".KKKKKKKKKKKKK..",
];

pub const SONGSTONE_PAL: &[(char, u32)] = &[
    ('S', 0x7a828e),
    ('s', 0x5a626e),
    ('W', 0xc8ccd4),
    ('M', 0x5a7a4a),
    ('K', 0x20242a),
];

/// The stone split open (js songDoor): two leaning halves around a dark stair-mouth.
/// 24x24, anchored 4 left + 9 up of the tile origin; a last note hangs in the dark.
const SONGDOOR_ART: &[&str] = &[
    ".ddddd......ddddd.......",
    ".dllsd......dslld.......",
    ".dllsd.nn...dslld.......",
    ".dllsd......dslld.......",
    ".dllsd......dslld.......",
    ".dllsdkkkkkkdslld.......",
    ".dllsdkkkkkkdslld.......",
    ".dllsdkkkkkkdslld.......",
    ".dllsdkkkkkkdslld.......",
    ".dllsdkkKKkkdslld.......",
    ".dllsdkkKKkkdslld.......",
    ".dllsdkkKKkkdslld.......",
    ".dllsdkkKKkkdslld.......",
    ".dllsdkkKKkkdslld.......",
    ".dllsdkkKKkkdslld.......",
    ".dllsdkkkkkkdslld.......",
    ".dllsdkkkkkkdslld.......",
    ".dllsdkkkkkkdslld.......",
    ".dllsd......dslld.......",
    ".dllsd......dslld.......",
    ".dllsd......dslld.......",
    ".dllsd......dslld.......",
    ".ddddd......ddddd.......",
    "........................",
];

const SONGDOOR_PAL: &[(char, u32)] = &[
    ('d', 0x20242a), // the halves' dark rim
    ('l', 0x7a828e), // lit stone face
    ('s', 0x5a626e), // shaded edge
    ('k', 0x0a0806), // the dark throat between them
    ('K', 0x000000), // black depths
    ('n', 0xb8a0e0), // a last note hanging in the dark (glow frozen mid-pulse)
];

/// Stand the split-open stone up as a walkable door (room build + the open moment).
pub fn spawn_song_door(commands: &mut Commands, images: &mut Assets<Image>, x: f32, y: f32, dest: String) {
    let img = images.add(bake(SONGDOOR_ART, SONGDOOR_PAL));
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + x - 4.0, PLAY_Y + y - 9.0, 24.0, 24.0, 3.36),
        PIXEL_LAYER,
        RoomActor,
        CaveDoor { x, y, dest }, // the same door logic: press to descend / enter the shop
    ));
}

/// The Song of Opening lands here (flute.rs writes; the stone is caves' business):
/// any live stone in the room splits open — no stone, and the notes fade unanswered.
#[allow(clippy::too_many_arguments)]
pub fn opening_sung(
    mut sung: MessageReader<OpeningSung>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cur: Res<super::play::CurRoom>,
    mut opened: ResMut<OpenedSongstones>,
    mut mana: ResMut<super::flute::Mana>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut log: ResMut<super::rewards::LootLog>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    stones: Query<(Entity, &Songstone)>,
) {
    for OpeningSung { mana: cost } in sung.read() {
        let Some((e, st)) = stones.iter().next() else {
            log.add("song", "THE NOTES FADE - NOTHING ANSWERS", 1, 0xb8a0d8, false, true);
            sfx.write(super::sfx::Sfx("warpFail"));
            continue;
        };
        mana.cur -= *cost; // the song only costs when something answers (js)
        opened.0.insert(format!("{},{}", cur.rx, cur.ry));
        let blk = (st.x + 2.0, st.y + 6.0, 12.0, 10.0);
        blockers.0.retain(|b| *b != blk); // the stone no longer stands in the way
        spawn_song_door(&mut commands, &mut images, st.x, st.y, st.dest.clone());
        commands.entity(e).despawn();
        log.add("song", "THE STONE SINGS BACK - AND OPENS", 1, 0xc8a060, false, true);
        sfx.write(super::sfx::Sfx("stone"));
        sfx.write(super::sfx::Sfx("itemget"));
        saves.write(super::save::SaveRequest);
    }
}

/// The dark mouth a broken crack reveals (js caveDoor): press at it to descend.
#[derive(Component)]
pub struct CaveDoor {
    pub x: f32,
    pub y: f32,
    pub dest: String, // "shop" | "mini" | "biome"
}

/// Nested arches carved into the wall tile — the js draw, rasterised to a grid.
const DOOR_ART: &[&str] = &[
    "................",
    "................",
    "....mmmmmmm.....",
    "...mmdddddmm....",
    "..mmdddkdddmm...",
    "..mddkkkkkddm...",
    "..mddkkkkkddm...",
    "..mddkkkkkddm...",
    "..mddkkkkkddm...",
    "..mddkkkkkddm...",
    "..mddkkkkkddm...",
    "..mddkkkkkddm...",
    "..mddkkkkkddm...",
    "..mddkkkkkddm...",
    "rrmddkkkkkddmrr.",
    "rr.............r",
];

const DOOR_PAL: &[(char, u32)] = &[
    ('m', 0x1a1410), // shadowed rim
    ('d', 0x0a0806), // the dark throat
    ('k', 0x000000), // black depths
    ('r', 0x3a322a), // spilled rubble
];

/// Stand a recorded cave door up (room build path — the arm in room_props).
pub fn spawn_cave_door(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    root: Entity,
    caves: &CrackCaves,
    room: (i32, i32),
    c: i32,
    r: i32,
) {
    let Some(dest) = caves
        .0
        .get(&format!("{},{}", room.0, room.1))
        .and_then(|v| v.iter().find(|(oc, or, _)| *oc == c && *or == r))
        .map(|(_, _, d)| d.clone())
    else {
        return;
    };
    let (x, y) = ((c * 16) as f32, (r * 16) as f32);
    let img = images.add(bake(DOOR_ART, DOOR_PAL));
    let e = commands
        .spawn((
            Sprite::from_image(img),
            at(PLAY_X + x, PLAY_Y + y, 16.0, 16.0, 3.36),
            PIXEL_LAYER,
            CaveDoor { x, y, dest },
        ))
        .id();
    commands.entity(root).add_child(e);
}

/// What a fresh break hides (js game.js: the deterministic per-crack hash roll —
/// re-breaking the same wall on another save always finds the same thing).
pub fn roll_dest(seed: u32, rx: i32, ry: i32, c: i32, r: i32) -> &'static str {
    let mut h = seed
        ^ (rx.wrapping_add(1) as u32).wrapping_mul(73856093)
        ^ (ry.wrapping_add(1) as u32).wrapping_mul(19349663)
        ^ (c.wrapping_add(1) as u32).wrapping_mul(83492791)
        ^ (r.wrapping_add(1) as u32).wrapping_mul(2654435761);
    h = (h ^ (h >> 15)).wrapping_mul(2246822519);
    let roll = (h % 1000) as f64 / 1000.0;
    if roll < 0.22 {
        "shop"
    } else if roll < 0.60 {
        "mini"
    } else {
        "biome"
    }
}

// ===================== THE BOMB =====================

/// play.rs slot-use drops one at the hero's feet.
#[derive(Message)]
pub struct DropBomb(pub f32, pub f32);

#[derive(Component)]
pub struct Bomb {
    pub x: f32,
    pub y: f32,
    pub fuse: i32,
}

/// The lit blast: one resolve_combat pass wide (HitOnce keeps it to one bite each).
#[derive(Component)]
pub struct Blast {
    pub life: i32,
}

const BOMB_ART: &[&str] = &[
    "................",
    ".......ff.......",
    "........c.......",
    "........c.......",
    "....KKKKKKKK....",
    "...KKKKKKKKKK...",
    "...KKKKKKKKKK...",
    "...KKKKKKKKKK...",
    "...KKKKKKKKKK...",
    "....KKKKKKKK....",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
];

fn bomb_art(blink: bool) -> (&'static [&'static str], &'static [(char, u32)]) {
    if blink {
        (BOMB_ART, &[('K', 0xfc7460), ('c', 0xcaa000), ('f', 0xfc7460)])
    } else {
        (BOMB_ART, &[('K', 0x000000), ('c', 0xcaa000), ('f', 0xfc7460)])
    }
}

fn drop_bombs(
    mut drops: MessageReader<DropBomb>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
) {
    for DropBomb(x, y) in drops.read() {
        let (art, pal) = bomb_art(false);
        let img = images.add(bake(art, pal));
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + x, PLAY_Y + y, 16.0, 16.0, actor_z(y + 13.0)),
            PIXEL_LAYER,
            RoomActor,
            Bomb { x: *x, y: *y, fuse: 75 },
        ));
        sfx.write(super::sfx::Sfx("tink"));
    }
}

fn bomb_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut bombs: Query<(Entity, &mut Bomb, &mut Sprite)>,
) {
    for (e, mut b, mut spr) in &mut bombs {
        b.fuse -= 1;
        if b.fuse <= 0 {
            commands.entity(e).despawn();
            let (cx, cy) = (b.x + 8.0, b.y + 8.0);
            // The blast (js explosion): 44x44, damage 4, team hazard — friend and foe alike.
            commands.spawn((
                Combatant { team: Team::Hazard, hurt_team: None, damage: Some(4), persistent: false, knock: 2.0 },
                Hitbox { x: cx - 22.0, y: cy - 22.0, w: 44.0, h: 44.0 },
                HitOnce::default(),
                RoomActor,
                Blast { life: 14 },
            ));
            // The flash: a hot core ring, gone with the blast.
            let ring = images.add(bake(
                &["..ffff..", ".fFFFFf.", "fFWWWWFf", "fFWWWWFf", "fFWWWWFf", "fFWWWWFf", ".fFFFFf.", "..ffff.."],
                &[('f', 0xfc7430), ('F', 0xfcae40), ('W', 0xfce0a8)],
            ));
            let fx = commands
                .spawn((
                    Sprite::from_image(ring),
                    at(PLAY_X + cx - 20.0, PLAY_Y + cy - 20.0, 40.0, 40.0, 12.0),
                    PIXEL_LAYER,
                    RoomActor,
                    BlastFlash { life: 14 },
                ))
                .id();
            let _ = fx;
            sfx.write(super::sfx::Sfx("enemyDie"));
            continue;
        }
        // The fuse burns down: blink red under 30 (js (fuse >> 2) % 2).
        if b.fuse < 30 {
            let (art, pal) = bomb_art((b.fuse >> 2) % 2 == 0);
            spr.image = images.add(bake(art, pal));
        }
    }
}

#[derive(Component)]
pub struct BlastFlash {
    pub life: i32,
}

fn blast_tick(
    mut commands: Commands,
    mut blasts: Query<(Entity, &mut Blast)>,
    mut flashes: Query<(Entity, &mut BlastFlash, &mut Sprite), Without<Blast>>,
) {
    for (e, mut b) in &mut blasts {
        b.life -= 1;
        if b.life <= 0 {
            commands.entity(e).despawn();
        }
    }
    for (e, mut f, mut spr) in &mut flashes {
        f.life -= 1;
        spr.color = Color::srgba(1.0, 1.0, 1.0, (f.life as f32 / 14.0).max(0.0));
        if f.life <= 0 {
            commands.entity(e).despawn();
        }
    }
}

/// A volatile creature's death blast (js blast): R 18, one bite, enemy-side only —
/// unlike the bomb it cannot chain into other foes.
pub fn spawn_death_blast(commands: &mut Commands, images: &mut Assets<Image>, cx: f32, cy: f32, dmg: i32) {
    commands.spawn((
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(dmg), persistent: false, knock: 1.5 },
        Hitbox { x: cx - 18.0, y: cy - 18.0, w: 36.0, h: 36.0 },
        HitOnce::default(),
        RoomActor,
        Blast { life: 14 },
    ));
    let ring = images.add(bake(
        &["..ffff..", ".fFFFFf.", "fFWWWWFf", "fFWWWWFf", "fFWWWWFf", "fFWWWWFf", ".fFFFFf.", "..ffff.."],
        &[('f', 0xfc7430), ('F', 0xfcae40), ('W', 0xfce0a8)],
    ));
    commands.spawn((
        Sprite::from_image(ring),
        at(PLAY_X + cx - 16.0, PLAY_Y + cy - 16.0, 32.0, 32.0, 12.0),
        PIXEL_LAYER,
        RoomActor,
        BlastFlash { life: 14 },
    ));
}

/// A broken crack becomes a cave door on the spot (gather.rs node_deaths hands the
/// fallen crack here): roll what it hides, record it forever, carve the mouth.
#[allow(clippy::too_many_arguments)]
pub fn crack_broken(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    caves: &mut CrackCaves,
    seed: u32,
    room: (i32, i32),
    c: i32,
    r: i32,
) -> &'static str {
    let dest = roll_dest(seed, room.0, room.1, c, r);
    let key = format!("{},{}", room.0, room.1);
    let list = caves.0.entry(key).or_default();
    if !list.iter().any(|(oc, or, _)| *oc == c && *or == r) {
        list.push((c, r, dest.to_string()));
    }
    // Standalone spawn (no root handy mid-death): RoomActor sweeps it on room change,
    // and the room rebuild re-stands it from the record.
    let (x, y) = ((c * 16) as f32, (r * 16) as f32);
    let img = images.add(bake(DOOR_ART, DOOR_PAL));
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + x, PLAY_Y + y, 16.0, 16.0, 3.36),
        PIXEL_LAYER,
        RoomActor,
        CaveDoor { x, y, dest: dest.to_string() },
    ));
    dest
}

/// The generous walk-up zone (js door: x-5, y-5, 26x26 — triggered from the floor
/// beside the wall).
pub fn door_zone(d: &CaveDoor) -> (f32, f32, f32, f32) {
    (d.x - 5.0, d.y - 5.0, 26.0, 26.0)
}

pub struct CavesPlugin;

impl Plugin for CavesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CrackCaves>()
            .init_resource::<OpenedSongstones>()
            .add_message::<DropBomb>()
            .add_message::<OpeningSung>()
            .add_systems(
                bevy::app::FixedUpdate,
                (drop_bombs, bomb_tick.before(crate::combat::resolve_combat), blast_tick.after(crate::combat::resolve_combat), opening_sung)
                    .before(super::play::EndTick)
                    .run_if(super::screen::playing),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dest roll is deterministic and js-shaped (all three outcomes reachable).
    #[test]
    fn dest_roll_is_stable() {
        let mut seen = std::collections::HashSet::new();
        for i in 0..200 {
            let d = roll_dest(1337, i, -i, 3, 0);
            assert_eq!(d, roll_dest(1337, i, -i, 3, 0));
            seen.insert(d);
        }
        assert_eq!(seen.len(), 3, "shop/mini/biome all rollable");
    }

    #[test]
    fn art_is_rectangular() {
        for row in CRACK_ART {
            assert_eq!(row.len(), 16);
        }
        for row in DOOR_ART {
            assert_eq!(row.len(), 16);
        }
        for row in BOMB_ART {
            assert_eq!(row.len(), 16);
        }
    }
}
