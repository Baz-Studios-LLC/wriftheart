//! dungeon.rs — the playable dungeons (dungeons arc, steps 2-3): the shard monuments'
//! breathing eyes + PRESS-TO-ENTER at the mouth, the themed interior (src/dungeon bake),
//! room-to-room walking, stairs between floors, and the ornate way back out.
//!
//! Port of js enterDungeon/exitDungeon + the dungeon-mode edge walk. DEVIATIONS
//! (flagged, temporary): room changes are instant swaps (the js slides — landing next);
//! enemies/chests/keys don't spawn yet (the combat pass); no dungeon-dark ambient yet
//! (the lighting pass); destructibles baked solid; pits solid (pit-falls port later).

use super::battle::RoomActor;
use super::interior::DoorCooldown;
use super::play::{CurRoom, GameWorld, Player};
use super::room_render::{child, RoomRoot, PLAY_X, PLAY_Y};
use super::screen::playing;
use crate::combat::Health;
use crate::dungeon::{self, Dir, Door, Dungeon, GenOpts, RoomType};
use crate::gfx::{at, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use crate::room::{PX_H, PX_W};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};


/// The RIFT SPIRE (js riftSpire, redrawn as a grid): a black tiered tower, seams
/// aglow, a maw at its base. Endless descent waits inside.
pub(crate) const RIFT_SPIRE_ART: [&str; 52] = [
    "..............bbbbbbbbbbbbbbbb..............",
    "..............bBBBBBBBBBBBBBBe..............",
    "..............bBBBBBBBBBBBBBBe..............",
    "..............bBBBBBBBBBBBBBBe..............",
    "..............bBBBBBBBBBBBBBBe..............",
    "..............bBBBBBBBBBBBBBBe..............",
    "..............bBBBBBBBBBBBBBBe..............",
    "..............bBBBBBBBBBBBBBBe..............",
    "..........bbVbbbbVbbbbVbbbbVbbbbbb..........",
    "..........bBBBBBBBBBBBBBBBBBBBBBBe..........",
    "..........bBBBBBBBBBBBBBBBBBBBBBBe..........",
    "..........bBBBBBBBBBBBBBBBBBBBBBBe..........",
    "..........bBBBBBBVBBBBBBBBBBBBBBBe..........",
    "..........bBBBBBBBBBBBBBBBBBBBBBBe..........",
    "..........bBBBBBBVBBBBBBBBBBBBBBBe..........",
    "..........bBBBBBBBBBBBBBBBBBBBBBBe..........",
    "..........bBBBBBBVBBBBBBBBBBBBBBBe..........",
    "..........bBBBBBBBBBBBBBBBBBBBBBBe..........",
    "..........bBBBBBBVBBBBBBBBBBBBBBBe..........",
    "..........bBBBBBBBBBBBBBBBBBBBBBBe..........",
    "......bbVbbbbVbbbVVbbbbVbbbbVbbbbVbbbb......",
    "......bBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBe......",
    "......bBBBBBBBBBBVBBBBBBBBBBBBBBBBBBBe......",
    "......bBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBe......",
    "......bBBBBBBBBBBVBBBBBBBBBBVBBBBBBBBe......",
    "......bBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBe......",
    "......bBBBBBBBBBBVBBBBBBBBBBVBBBBBBBBe......",
    "......bBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBe......",
    "......bBBBBBBBBBBVBBBBBBBBBBVBBBBBBBBe......",
    "......bBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBe......",
    "......bBBBBBBBBBBBBBBBBBBBBBVBBBBBBBBe......",
    "......bBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBe......",
    "......bBBBBBBBBBBBBBBBBBBBBBVBBBBBBBBe......",
    "......bBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBe......",
    ".bbVbbbbVbbbbVbbbbVbbbbVbbbbVbbbbVbbbbVbbbb.",
    ".bBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBe.",
    ".bBBBBBBBBVBBBBBBBBBBBBBBBBBVBBBBBBBBBBBBBe.",
    ".bBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBe.",
    ".bBBBBBBBBVBBBBBBBBBBBBBBBBBVBBBBBBVBBBBBBe.",
    ".bBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBe.",
    ".bBBBBBBBBVBBBBBBBBBBBBBBBBBVBBBBBBVBBBBBBe.",
    ".bBBBBBBBBBBBBBBVVVVVVVVVVVVBBBBBBBBBBBBBBe.",
    ".bBBBBBBBBVBBBBBVDDDDDDDDDDVVBBBBBBVBBBBBBe.",
    ".bBBBBBBBBBBBBBBVDDDDDDDDDDVBBBBBBBBBBBBBBe.",
    ".bBBBBBBBBVBBBBBVDDDDDDDDDDVBBBBBBBVBBBBBBe.",
    ".bBBBBBBBBBBBBBBVDDDDDDDDDDVBBBBBBBBBBBBBBe.",
    ".bBBBBBBBBVBBBBBVDDDDDDDDDDVBBBBBBBVBBBBBBe.",
    ".bBBBBBBBBBBBBBBVDDDDDDDDDDVBBBBBBBBBBBBBBe.",
    ".bBBBBBBBBBBBBBBVDDDDDDDDDDVBBBBBBBVBBBBBBe.",
    ".bBBBBBBBBBBBBBBVDDDDDDDDDDVBBBBBBBBBBBBBBe.",
    ".bBBBBBBBBBBBBBBVDDDDDDDDDDVBBBBBBBBBBBBBBe.",
    "............................................",
];
pub(crate) const RIFT_SPIRE_PAL: &[(char, u32)] = &[('B', 0x1a1426), ('b', 0x2e2440), ('e', 0x0c0a12), ('V', 0xa06eff), ('D', 0x080610)];

/// js THEME_BY_BIOME — every land's dungeon wears its land's interior.
const THEME_BY_BIOME: &[(&str, &str)] = &[
    ("grassland", "crypt"),
    ("forest", "ruins"),
    ("desert", "tomb"),
    ("mountains", "cave"),
    ("swamp", "bog"),
    ("graveyard", "ossuary"),
    ("arctic", "frostcavern"),
    ("burnt", "charhall"),
    ("mushroom", "fungal"),
    ("chaos", "riftvault"),
    ("petalwood", "petalhall"),
    ("honeyglade", "hivehollow"),
    ("bluebell", "bellbarrow"),
    ("greenmaw", "vinewarren"),
    ("prismwastes", "crystalcave"),
    ("suncoast", "searuin"),
    ("stormreach", "stormspire"),
    ("tarmire", "tarpit"),
    ("galewind", "windbarrow"),
    ("saltwastes", "saltmine"),
    ("hollowwood", "hollowroot"),
    ("witherlands", "blightvault"),
    ("embermaw", "lavatube"),
    ("blackdeep", "darkdepths"),
    ("wriftscar", "wriftvault"),
    ("emberscar", "wriftvault"),
    ("gloammoor", "wriftvault"),
    ("starhollow", "wriftvault"),
];

/// js FLOORS_BY_TIER — shard dungeons deepen with their land's zone tier.
const FLOORS_BY_TIER: [usize; 7] = [2, 2, 3, 3, 4, 5, 6];

/// On the monument sprite: which land's dungeon this mouth opens into.
#[derive(Component)]
pub struct DungeonEntrance {
    pub biome: String,
    pub x: f32, // entity tile, room px (the mouth floor)
    pub y: f32,
}

/// The run in progress (None = the overworld). js `dungeon` + returnPos.
#[derive(Resource, Default)]
pub struct InDungeon(pub Option<DungeonRun>);

/// Dungeon keys are a per-dungeon COUNT shown in the HUD, NOT bag items (Baz). Zeroed the
/// moment you leave a dungeon (`clear_keys_outside`), so keys can't exist in the overworld;
/// persists across floors (only reset outside). `small` opens small locks, `ornate` the boss
/// door.
#[derive(Resource, Default)]
pub struct DungeonKeys {
    pub small: u32,
    pub ornate: u32,
}

/// The in-view key counter (bottom-right of the play area).
#[derive(Component)]
struct KeyHud;

/// Keys don't exist in the overworld (Baz): zero the count the moment you're out of a dungeon.
/// Inside, they persist across floors (the run stays live, so this never fires there).
fn clear_keys_outside(in_dungeon: Res<InDungeon>, mut keys: ResMut<DungeonKeys>) {
    if in_dungeon.0.is_none() && (keys.small != 0 || keys.ornate != 0) {
        keys.small = 0;
        keys.ornate = 0;
    }
}

/// The in-view key counter — a small key icon + `xN` per kind at the bottom-right of the play
/// area (Baz), shown only inside a dungeon; rebuilt only when a count changes.
fn key_hud(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    in_dungeon: Res<InDungeon>,
    keys: Res<DungeonKeys>,
    old: Query<Entity, With<KeyHud>>,
    mut last: Local<(u32, u32)>,
) {
    let cur = if in_dungeon.0.is_some() { (keys.small, keys.ornate) } else { (0, 0) };
    if *last == cur {
        return;
    }
    *last = cur;
    for e in &old {
        commands.entity(e).despawn();
    }
    if cur == (0, 0) {
        return;
    }
    use crate::room::PX_H;
    use super::room_render::{PLAY_X, PLAY_Y};
    let z = crate::gfx::layers::PROMPT_TEXT;
    let key_img = images.add(crate::gfx::bake(crate::actors::items_art::KEY_ICON, &[]));
    let okey_img = images.add(crate::gfx::bake(crate::actors::items_art::OKEY_ICON, &[('m', 0xc878ff)]));
    let mut y = PLAY_Y + PX_H as f32 - 11.0; // bottom row, stacking upward
    for (n, img) in [(cur.0, key_img), (cur.1, okey_img)] {
        if n == 0 {
            continue;
        }
        let txt = format!("x{n}");
        // Bottom-LEFT (Baz): the notification feed owns the bottom-right corner.
        let icx = PLAY_X + 3.0;
        commands.spawn((
            Sprite::from_image(img),
            crate::gfx::at(icx, y, 8.0, 8.0, z),
            crate::gfx::PIXEL_LAYER,
            KeyHud,
        ));
        crate::ui::label(&mut commands, &mut images, &txt, icx + 9.0, y + 1.0, 0xfce0a8, z, KeyHud);
        y -= 10.0;
    }
}

/// Banked dungeon progress per entrance (js dungeonState, in-memory like RoomCache —
/// the save-file layer is a flagged follow-up; a slot load clears it). Dungeons
/// REGENERATE deterministically, so the ledger stores only what play changed.
#[derive(Resource, Default)]
pub struct DungeonLedger(pub bevy::platform::collections::HashMap<String, DgSave>);

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DgSave {
    rooms: Vec<(usize, i32, i32, RoomSave)>, // (floor, rx, ry, state) — JSON needs list keys
    /// Remaining locks per floor (an OPENED door stays open — simpler than the js
    /// opened-list replay: we store what's still shut).
    locked: Vec<std::collections::HashSet<((i32, i32), crate::dungeon::Dir)>>,
    ornate: Vec<std::collections::HashSet<((i32, i32), crate::dungeon::Dir)>>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct RoomSave {
    cleared: bool,
    looted: bool,
    key_taken: bool,
    bosskey_taken: bool,
    boss_loot: bool,
    roster: Vec<(String, i32, i32)>, // kinds re-intern via themes::intern_kind on apply
    #[serde(default)]
    broken: Vec<(i32, i32)>, // smashed furniture stays smashed for the run
    #[serde(default)]
    mimic_slain: bool, // teeth only bite once — the coughed-up chest waits under `looted`
    #[serde(default)]
    secret_done: bool, // a shoved block stays shoved — the hidden stairs stand revealed
}

/// Bank the whole run (js serializeDungeon) — bank_room kept per-room rosters live,
/// so this is a straight copy of flags + survivors + the surviving locks.
fn serialize_run(run: &DungeonRun, ledger: &mut DungeonLedger) {
    let mut save = DgSave { rooms: Vec::new(), locked: Vec::new(), ornate: Vec::new() };
    for (f, fl) in run.dungeon.floors.iter().enumerate() {
        save.locked.push(fl.locked.clone());
        save.ornate.push(fl.ornate.clone());
        for (&(x, y), room) in &fl.rooms {
            save.rooms.push((
                f,
                x,
                y,
                RoomSave {
                    cleared: room.cleared,
                    looted: room.looted,
                    key_taken: room.key_taken,
                    bosskey_taken: room.bosskey_taken,
                    boss_loot: room.boss_loot,
                    roster: room.enemies.iter().map(|e| (e.kind.to_string(), e.x, e.y)).collect(),
                    broken: room.broken.clone(),
                    mimic_slain: room.mimic_slain,
                    secret_done: room.secret_done,
                },
            ));
        }
    }
    ledger.0.insert(run.entrance_key.clone(), save);
}

/// Overlay banked progress onto a fresh (deterministic) generation (js applyDungeonState).
fn apply_ledger(d: &mut Dungeon, key: &str, ledger: &DungeonLedger) {
    let Some(save) = ledger.0.get(key) else { return };
    for (f, fl) in d.floors.iter_mut().enumerate() {
        if let (Some(l), Some(o)) = (save.locked.get(f), save.ornate.get(f)) {
            fl.locked = l.clone();
            fl.ornate = o.clone();
        }
        for (&(x, y), room) in fl.rooms.iter_mut() {
            let Some((.., rs)) = save.rooms.iter().find(|&&(sf, sx, sy, _)| sf == f && sx == x && sy == y) else { continue };
            room.cleared = rs.cleared;
            room.looted = rs.looted;
            room.key_taken = rs.key_taken;
            room.bosskey_taken = rs.bosskey_taken;
            room.boss_loot = rs.boss_loot;
            room.broken = rs.broken.clone();
            room.mimic_slain = rs.mimic_slain;
            room.secret_done = rs.secret_done;
            room.enemies = rs
                .roster
                .iter()
                .filter_map(|(kind, ex, ey)| {
                    crate::dungeon::themes::intern_kind(kind).map(|k| crate::dungeon::Enemy { kind: k, x: *ex, y: *ey })
                })
                .collect();
        }
    }
}

/// The active dungeon room's darkness-hole lights (torches + lit decor), room px.
/// Rebuilt by spawn_droom; ignored outside dungeon mode.
#[derive(Resource, Default)]
pub struct DungeonLights(pub Vec<(i32, i32, i32)>);

/// A dungeon chest: walk onto it to spring it (js chest overlap). `hold` = fixed
/// contents (a key); None = a treasure roll. Marks its room flag when opened.
#[derive(Component)]
pub struct Chest {
    pub hold: Option<&'static str>,
    pub gilded: bool,
    pub open: bool,
    pub x: f32,
    pub y: f32,
}

/// Not every chest is a chest (js enemies.js mimic, REDESIGNED per Baz: the js one wore
/// its own sprite at the treasure room's chest spot — obvious twice over). Shut, this is
/// furniture: the real chest's exact bake, unhittable, harmless, in a room that holds no
/// real chest. Reach for the lid and it springs — teeth frames, 3-damage lunges.
#[derive(Component)]
pub struct MimicChest {
    pub x: f32,
    pub y: f32,
    /// The chest's rightful spot (js homeX/homeY) — where the REAL one lands on the kill.
    pub home: (i32, i32),
    pub sprung: bool,
    anim: u32,
    hop_t: i32,
    run_t: i32,
    cvx: f32,
    cvy: f32,
    /// Baked frames: [open maw, bite] — swapped by handle, no per-frame rebakes.
    frames: [Handle<Image>; 2],
    frame: u8, // 0 = shut (chest), 1 = open, 2 = bite
    /// The lash in flight (mouth hangs open for its whole arc — js frog st==2).
    tongue: Option<Tongue>,
    tongue_cd: i32,
}

/// One tongue lash (js frogTongue verbatim: 8f extend / 3f hold / 9f retract, direction
/// locked at launch, tip grabs and reels). `line`/`tip` are its two fx sprites.
pub struct Tongue {
    ux: f32,
    uy: f32,
    max_len: f32,
    len: f32,
    t: i32,
    grabbed: bool,
    line: Entity,
    tip: Entity,
}
const TONGUE_EXT: i32 = 8;
const TONGUE_HOLD: i32 = 3;
const TONGUE_RET: i32 = 9;

/// Marker on the two tongue fx sprites (keeps their query disjoint from the mimic's).
#[derive(Component)]
pub struct TongueFx;

/// The secret PUSH-BLOCK (js pushBlock): a slate block squatting where no block
/// belongs. Shove it from an adjacent tile for 48 grinding frames and it gives way —
/// hidden stairs to the vault stand where it stood.
#[derive(Component)]
pub struct PushBlock {
    pub c: i32,
    pub r: i32,
    blocker: (f32, f32, f32, f32),
}

/// The revealed way down (and, inside the vault, the way back up).
#[derive(Component)]
pub struct SecretStairs;

/// On the vault's chest: the js secret-cache roll (20-49 coin + a boost-0.9 drop —
/// stronger than a regular chest, shy of a boss purse).
#[derive(Component)]
pub struct VaultChest;

/// js pushBlock's exact slate look, as a bake grid ('S' base / 'H' highlight / 'D' dark).
const PUSH_BLOCK_ART: &[&str] = &[
    "SSSSSSSSSSSSSSSS",
    "SHHHHHHHHHHHHHHS",
    "SHHHHHHHHHHHHHDS",
    "SSHSSSSSSSSSSDSS",
    "SSHSSSSSSSSSSDSS",
    "SSHSSSSSSSSSSDSS",
    "SSHSSSSSSSSSSDSS",
    "SSHSSSSSSSSSSDSS",
    "SSHSSDDDDDDDSDSS",
    "SSHSSSSSSSSSSDSS",
    "SSHSSSSSSSSSSDSS",
    "SSHSSSSSSSSSSDSS",
    "SSHSSSSSSSSSSDSS",
    "SDDDDDDDDDDDDDDS",
    "SDDDDDDDDDDDDDDS",
    "SSSSSSSSSSSSSSSS",
];
const STAIRS_ART: &[&str] = &[
    "KKKKKKKKKKKKKKKK",
    "KHHHHHHHHHHHHHHK",
    "KHSSSSSSSSSSSSHK",
    "KHSSSSSSSSSSSSHK",
    "KSSDDDDDDDDDDSSK",
    "KSSDDDDDDDDDDSSK",
    "KSDDKKKKKKKKDDSK",
    "KSDDKKKKKKKKDDSK",
    "KSDKKKKKKKKKKDSK",
    "KSDKKKKKKKKKKDSK",
    "KDKKKKKKKKKKKKDK",
    "KDKKKKKKKKKKKKDK",
    "KDKKKKKKKKKKKKDK",
    "KDKKKKKKKKKKKKDK",
    "KKKKKKKKKKKKKKKK",
    "KKKKKKKKKKKKKKKK",
];
const SLATE: &[(char, u32)] = &[('S', 0x3a3c48), ('H', 0x4c4f60), ('D', 0x24252d)];

/// Wake a room's secret furniture: the unshoved block (solid), or the revealed
/// stairs; inside a vault, the way back up + the cache chest.
pub(crate) fn spawn_room_secret(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    room: &crate::dungeon::DRoom,
    blockers: &mut super::room_props::RoomBlockers,
) {
    let stairs_pad = |commands: &mut Commands, images: &mut Assets<Image>, c: i32, r: i32| {
        let img = images.add(crate::gfx::bake(STAIRS_ART, SLATE));
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + (c * 16) as f32, PLAY_Y + (r * 16) as f32, 16.0, 16.0, 1.6),
            PIXEL_LAYER,
            RoomActor,
            SecretStairs,
        ));
    };
    if let Some((c, r)) = room.secret {
        if room.secret_done {
            stairs_pad(commands, images, c, r);
        } else {
            let img = images.add(crate::gfx::bake(PUSH_BLOCK_ART, SLATE));
            let (px, py) = ((c * 16) as f32, (r * 16) as f32);
            let blocker = (px + 1.0, py + 1.0, 14.0, 14.0);
            blockers.0.push(blocker);
            commands.spawn((
                Sprite::from_image(img),
                at(PLAY_X + px, PLAY_Y + py, 16.0, 16.0, super::room_render::actor_z(py + 15.0)),
                PIXEL_LAYER,
                RoomActor,
                PushBlock { c, r, blocker },
            ));
        }
    }
    if room.vault {
        stairs_pad(commands, images, 4, 3);
        if let Some((cx, cy)) = room.chest
            && !room.looted
        {
            let img = images.add(crate::gfx::bake(crate::actors::items_art::CHEST_ICON, &[]));
            commands.spawn((
                Sprite::from_image(img),
                at(PLAY_X + cx as f32, PLAY_Y + cy as f32 + 2.0, 16.0, 12.0, super::room_render::actor_z(cy as f32 + 14.0)),
                PIXEL_LAYER,
                RoomActor,
                Chest { hold: None, gilded: false, open: false, x: cx as f32, y: cy as f32 },
                VaultChest,
            ));
        }
    }
}

/// The shove (js game.js pushT): stand on the tile beside the block, hold INTO it —
/// 48 frames of stone grinding on stone, then it gives way one tile and the hidden
/// stairs stand revealed. The target tile must be clear (never onto a pit).
#[allow(clippy::too_many_arguments)]
fn push_block_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut in_dungeon: ResMut<InDungeon>,
    state: Res<ActionState>,
    grid: Res<super::play::CurGrid>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut push_t: Local<i32>,
    players: Query<&Player>,
    mut blocks: Query<(Entity, &mut PushBlock, &mut Transform)>,
) {
    let Some(run) = &mut in_dungeon.0 else { return };
    let Ok(p) = players.single() else { return };
    let Ok((be, mut b, mut tf)) = blocks.single_mut() else {
        *push_t = 0;
        return;
    };
    let (pc, pr) = (((p.x + 8.0) / 16.0).floor() as i32, ((p.y + 8.0) / 16.0).floor() as i32);
    let (dc, dr) = if pr == b.r && pc == b.c - 1 && state.held(Action::Right) {
        (1, 0)
    } else if pr == b.r && pc == b.c + 1 && state.held(Action::Left) {
        (-1, 0)
    } else if pc == b.c && pr == b.r - 1 && state.held(Action::Down) {
        (0, 1)
    } else if pc == b.c && pr == b.r + 1 && state.held(Action::Up) {
        (0, -1)
    } else {
        *push_t = 0;
        return;
    };
    let (tc, tr) = (b.c + dc, b.r + dr);
    let room = run.dungeon.cur().room(run.drx, run.dry);
    let on_pit = room.is_some_and(|rm| rm.pits.contains(&(tc, tr)));
    let dest = ((tc * 16) as f32 + 2.0, (tr * 16) as f32 + 2.0, 12.0, 12.0);
    let clear = !on_pit
        && !grid.0.box_hits_solid(dest.0, dest.1, dest.2, dest.3)
        && !blockers.0.iter().any(|r| *r != b.blocker && overlap(*r, dest));
    if !clear {
        *push_t = 0;
        return;
    }
    *push_t += 1;
    if *push_t == 1 || *push_t % 12 == 0 {
        sfx.write(super::sfx::Sfx("stone")); // stone grinding on stone (js)
    }
    if *push_t < 48 {
        return;
    }
    // It gives way: the block slides one tile aside, the stairs stand where it stood.
    *push_t = 0;
    let (oc, or) = (b.c, b.r);
    blockers.0.retain(|r| *r != b.blocker);
    b.c = tc;
    b.r = tr;
    b.blocker = ((tc * 16) as f32 + 1.0, (tr * 16) as f32 + 1.0, 14.0, 14.0);
    blockers.0.push(b.blocker);
    *tf = at(PLAY_X + (tc * 16) as f32, PLAY_Y + (tr * 16) as f32, 16.0, 16.0, super::room_render::actor_z((tr * 16) as f32 + 15.0));
    let _ = be;
    if let Some(rm) = run.dungeon.cur_mut().rooms.get_mut(&(run.drx, run.dry)) {
        rm.secret_done = true;
    }
    let img = images.add(crate::gfx::bake(STAIRS_ART, SLATE));
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + (oc * 16) as f32, PLAY_Y + (or * 16) as f32, 16.0, 16.0, 1.6),
        PIXEL_LAYER,
        RoomActor,
        SecretStairs,
    ));
    sfx.write(super::sfx::Sfx("warpCharge")); // the reveal chime (js)
}

pub struct DungeonRun {
    pub dungeon: Dungeon,
    pub drx: i32,
    pub dry: i32,
    /// The overworld doorstep to land back on (rx, ry, px, py).
    pub return_pos: (i32, i32, f32, f32),
    /// The ledger key ("rx,ry" — js entranceKey): progress survives re-entry.
    pub entrance_key: String,
    /// The land whose shard this dungeon guards (None: bonus caves, no shard).
    pub biome: Option<String>,
    /// The Black Castle finale — its boss is THE WRIFTHEART (js isFinal).
    pub is_final: bool,
    /// Doors we slammed shut for the boss (js arenaLock) — reopened when it falls.
    pub arena: Option<Vec<Dir>>,
    /// A MINI cave's elite stand-in boss kind (js dungeon.miniKind; None elsewhere).
    pub mini: Option<&'static str>,
    /// Rift depth (0 = not a rift). Scales foes + loot; rifts REGENERATE per visit
    /// (never banked to the ledger) and descend forever through RiftGates.
    pub rift: i32,
    /// The spire's seed root, carried down every floor (js dungeon.riftBase).
    pub rift_base: u32,
}

/// The shards claimed so far (js `relics` Set — the WIN CONDITION counter). In-memory
/// like the dungeon ledger; the save-file layer lands together with it (FLAGGED).
#[derive(Resource, Default)]
pub struct Relics(pub std::collections::HashSet<String>);

/// The boss holding this dungeon's shard (STAND-IN, flagged: an elite-scaled heavy of
/// the theme roster until the js themed boss AIs port).
#[derive(Component)]
pub struct DungeonBoss;

/// The fallen boss's shard, glowing in its land's colour — touch to claim.
#[derive(Component)]
pub struct RelicShard {
    pub biome: String,
    pub x: f32,
    pub y: f32,
}

/// The way home after the kill — a bare anchor; rune_tick draws the js warpRune
/// live and turns a PRESS on it into the ride out.
#[derive(Component)]
pub struct WarpRune {
    pub x: f32,
    pub y: f32,
}

/// "The rune was pressed" — rune_tick writes, navigate rides home.
#[derive(Message)]
pub struct RuneActivate;

/// Marker on the rune's live sprites (rebuilt each tick, immediate-mode).
#[derive(Component)]
struct RuneFxUi;

/// Marker on the rune's ACTIVATE bubble.
#[derive(Component, Clone)]
struct RunePromptUi;

/// The js warpRune, pixel for pixel: a pulsing violet radial glow, the r6 ring,
/// the cross glyph, four motes orbiting — SEALED grey and near-still while the
/// boss's shard sits unclaimed (Baz: it wakes when you take it). Standing on a
/// woken rune offers ACTIVATE by the character; the press rides home.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn rune_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    clock: Res<super::room_render::FrameClock>,
    mut input: ResMut<ActionState>,
    in_dungeon: Res<InDungeon>,
    bindings: Res<crate::input::Bindings>,
    mut goes: MessageWriter<RuneActivate>,
    players: Query<&Player>,
    runes: Query<&WarpRune>,
    shards: Query<(), With<RelicShard>>,
    old_fx: Query<Entity, With<RuneFxUi>>,
    old_prompt: Query<Entity, With<RunePromptUi>>,
    mut tex: Local<Option<(Handle<Image>, Handle<Image>)>>,
) {
    for e in old_fx.iter().chain(old_prompt.iter()) {
        commands.entity(e).despawn();
    }
    if in_dungeon.0.is_none() || runes.is_empty() {
        return;
    }
    let tint = |c: u32, a: f32| {
        Color::srgba((c >> 16 & 255) as f32 / 255.0, (c >> 8 & 255) as f32 / 255.0, (c & 255) as f32 / 255.0, a)
    };
    let (glow, ring) = tex.get_or_insert_with(|| (crate::gfx::radial_glow_tex(&mut images, 48), rune_ring_tex(&mut images))).clone();
    let dim = !shards.is_empty(); // the unclaimed shard outranks the ride home
    let t = clock.0 as f32;
    let pulse = ((0.55 + 0.45 * (t * 0.11).sin()) * if dim { 0.3 } else { 1.0 }).clamp(0.0, 1.0);
    let gr = 10.0 + (t * 0.11).sin() * 1.5 + 6.0; // js R + the gradient's 6px skirt
    let glow_col = if dim { 0x6e7882 } else { 0xa064f5 };
    let ring_col = if dim { 0x8a94a0 } else { 0xc8a0ff };
    let glyph_col = if dim { 0xaab4c0 } else { 0xede0ff };
    for r in &runes {
        let (cx, cy) = (PLAY_X + r.x + 8.0, PLAY_Y + r.y + 8.0);
        let mut g = Sprite::from_image(glow.clone());
        g.custom_size = Some(Vec2::splat(gr * 2.0));
        g.color = tint(glow_col, 0.55 * pulse);
        commands.spawn((g, at(cx - gr, cy - gr, gr * 2.0, gr * 2.0, 2.9), PIXEL_LAYER, RuneFxUi));
        let mut rs = Sprite::from_image(ring.clone());
        rs.color = tint(ring_col, pulse);
        commands.spawn((rs, at(cx - 6.5, cy - 6.5, 13.0, 13.0, 3.0), PIXEL_LAYER, RuneFxUi));
        for (gx, gy, gw, gh) in [(cx - 1.0, cy - 5.0, 2.0, 10.0), (cx - 5.0, cy - 1.0, 10.0, 2.0)] {
            commands.spawn((
                Sprite::from_color(tint(glyph_col, pulse), Vec2::new(gw, gh)),
                at(gx, gy, gw, gh, 3.02),
                PIXEL_LAYER,
                RuneFxUi,
            ));
        }
        if !dim {
            for i in 0..4 {
                let a = t * 0.05 + i as f32 * std::f32::consts::FRAC_PI_2;
                commands.spawn((
                    Sprite::from_color(tint(glyph_col, pulse), Vec2::splat(2.0)),
                    at((cx + a.cos() * 7.0).round() - 1.0, (cy + a.sin() * 7.0).round() - 1.0, 2.0, 2.0, 3.05),
                    PIXEL_LAYER,
                    RuneFxUi,
                ));
            }
        }
    }
    // The prompt + the press — a WOKEN rune only (the grey one just sleeps).
    let Ok(p) = players.single() else { return };
    let hb = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    let on_rune = runes
        .iter()
        .any(|r| hb.0 < r.x + 16.0 && hb.0 + hb.2 > r.x && hb.1 < r.y + 16.0 && hb.1 + hb.3 > r.y);
    if on_rune && !dim {
        let text = format!("{} ACTIVATE", bindings.prompt(Action::Interact, input.pad_present));
        super::prompts::spawn_bubble(&mut commands, &mut images, &text, p.x + 8.0, p.y - 10.0, RunePromptUi);
        if input.pressed(Action::Interact) {
            input.consume(Action::Interact);
            goes.write(RuneActivate);
        }
    }
}


/// The js ctx.arc(r6) stroke as a crisp pixel circle, baked once (13x13, white).
fn rune_ring_tex(images: &mut Assets<Image>) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const S: usize = 13;
    let mut data = vec![0u8; S * S * 4];
    for y in 0..S {
        for x in 0..S {
            let (dx, dy) = (x as f32 - 6.0, y as f32 - 6.0);
            if ((dx * dx + dy * dy).sqrt() - 6.0).abs() < 0.55 {
                let i = (y * S + x) * 4;
                data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    images.add(Image::new(
        Extent3d { width: S as u32, height: S as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ))
}

/// The Black Castle's facade — dress_castle bakes gate state from the shard count.
#[derive(Component)]
pub struct CastleGate {
    pub x: f32, // anchor tile (gate bottom-centre), room px
    pub y: f32,
    pub baked: Option<(bool, usize)>, // the (unlocked, shards) state currently baked
}

/// The YOU WIN overlay (js victory): Some(frames since the Wriftheart mended).
#[derive(Resource, Default)]
pub struct Victory {
    /// The YOU-WIN overlay's clock (None = closed).
    pub t: Option<u32>,
    /// js gameWon — the Wriftheart has been mended (saved).
    pub won: bool,
    /// The credits phase after the win screen (0 = off).
    pub credits: u32,
}

/// One 2x2 eye overlay (child of the monument sprite; colour pulses).
#[derive(Component)]
struct EntranceEye;

pub struct DungeonPlugin;

impl Plugin for DungeonPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RuneActivate>().add_systems(Update, (dress_castle, victory_tick, pit_anim, descent_overlay)).init_resource::<InDungeon>().init_resource::<DungeonLights>().init_resource::<DungeonLedger>().init_resource::<Relics>().init_resource::<Victory>().init_resource::<PitFalling>().init_resource::<Descending>().init_resource::<DungeonKeys>().add_systems(Update, (breathe, clear_keys_outside, key_hud)).add_systems(
            bevy::app::FixedUpdate,
            (
                enter_dungeon.after(super::interior::door_enter),
                rune_tick.before(navigate),
                navigate,
                mark_visited.after(navigate),
                chest_touch,
                push_block_tick,
                mimic_tick.before(crate::combat::resolve_combat),
                dprop_deaths.after(crate::combat::resolve_combat),
                mimic_deaths.after(crate::combat::resolve_combat),
            )
                .before(super::play::EndTick)
                .run_if(playing),
        );
    }
}

/// Stamp the CURRENT dungeon room VISITED every settled tick (the floor map draws
/// only visited rooms — js set the flag on entry; one idempotent watcher beats
/// chasing every entry path: edge walks, stairs, pit falls, warp runes).
fn mark_visited(mut in_dungeon: ResMut<InDungeon>) {
    if let Some(run) = &mut in_dungeon.0 {
        let (x, y) = (run.drx, run.dry);
        if let Some(r) = run.dungeon.cur_mut().rooms.get_mut(&(x, y))
            && !r.visited
        {
            r.visited = true;
        }
    }
}

/// Attach the pulsing eye quads to a just-spawned monument (called inline from
/// room_props while the parent entity is certainly alive; children ride room slides
/// with the sprite for free).
pub(crate) fn spawn_eyes(commands: &mut Commands, monument: Entity, ea: &crate::actors::entrance_art::EntranceArt) {
    let [r, g, b] = [(ea.glow >> 16) as u8, (ea.glow >> 8) as u8, ea.glow as u8];
    for &(ex, ey) in ea.eyes {
        // Pixel (ex,ey) in the 64x56 grid -> offset from the sprite's centre (2x2 quad).
        let rel = Vec3::new(ex as f32 + 1.0 - 32.0, 28.0 - ey as f32 - 1.0, 0.05);
        commands.spawn((
            Sprite::from_color(Color::srgba_u8(r, g, b, 0), Vec2::new(2.0, 2.0)),
            Transform::from_translation(rel),
            ChildOf(monument),
            PIXEL_LAYER,
            EntranceEye,
        ));
    }
}

/// The shard colour breathes in the stone (js: p = 0.5 + 0.5·sin(clock/24)).
fn breathe(clock: Res<super::room_render::FrameClock>, mut eyes: Query<&mut Sprite, With<EntranceEye>>) {
    let p = 0.5 + 0.5 * ((clock.0 as f32) / 24.0).sin();
    let a = 0.25 + 0.45 * p;
    for mut s in &mut eyes {
        s.color = s.color.with_alpha(a);
    }
}

fn overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

/// On a spawned dungeon foe: its ROSTER kind (survivors bank back under this name —
/// the placeholder goblin must not forget it was a wraith).
#[derive(Component)]
pub struct DungeonFoe(pub &'static str);

/// Wake an uncleared room's roster (js loadDungeonRoom's enemy spawn).
/// A live smashable furniture piece (js makeDungeonProp): any weapon thunks it,
/// enough blows smash it — debris, sometimes coin, rarely real gear; broken stays
/// broken for the run. Cobwebs hang in the air (never solid, one swing).
#[derive(Component)]
pub struct DProp {
    pub kind: &'static str,
    pub c: i32,
    pub r: i32,
    pub blocker: Option<(f32, f32, f32, f32)>,
    pub debris: u32,
}

/// The pit-fall in flight (js pitFalling): the hero tips + shrinks into the hole,
/// then it costs TWO hearts and spits him back out the dungeon door.
pub struct PitFall {
    pub t: i32,
    pub px: f32,
    pub py: f32,
    pub cx: f32,
    pub cy: f32,
}
#[derive(Resource, Default)]
pub struct PitFalling(pub Option<PitFall>);
pub const PIT_FALL: i32 = 46;

/// A stair walk between floors in flight (js `descending`): the hero keeps stepping
/// INTO the steps as the screen fades to black, the floor swaps at full black, then
/// you fade back in stepping off the far stairs — a ~3s walk, not an instant warp.
/// Control is locked (play::tick early-returns on it, like a pit fall).
pub struct DescendFx {
    pub t: u32,
    /// +1 descends (walk DOWN into the steps), -1 climbs (walk UP).
    pub dir: i32,
    /// The room to land in on the new floor (its arrival / the stairs-down host).
    target: (i32, i32),
    applied: bool,
}
#[derive(Resource, Default)]
pub struct Descending(pub Option<DescendFx>);
/// 55 + 70 + 55 = 180 frames ≈ 3s at 60fps (js STAIR_FADE / STAIR_HOLD).
const STAIR_FADE: u32 = 55;
const STAIR_HOLD: u32 = 70;

impl DescendFx {
    /// A descent staged for the visual harness (WRIFT_SHOT=descent): parked in the
    /// fade-out at ~0.82 wash so the room shows through under a clearly-lit word.
    /// Already applied, so it never swaps floors.
    pub fn staged(dir: i32) -> Self {
        Self { t: STAIR_FADE + STAIR_HOLD + 10, dir, target: (0, 0), applied: true }
    }
}

/// Wake a room's smashable furniture (skipping the already-broken) — live entities
/// with their own blockers, painted by the shared prop painter.
pub(crate) fn spawn_room_dprops(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    room: &crate::dungeon::DRoom,
    theme: &'static crate::dungeon::themes::Theme,
    blockers: &mut super::room_props::RoomBlockers,
) {
    use crate::dungeon::decor;
    for d in &room.decor {
        let meta = decor::prop(d.kind);
        if !meta.destructible || room.broken.contains(&(d.c, d.r)) {
            continue;
        }
        let Some((debris, hp, _flammable)) = decor::smash_stats(d.kind) else { continue };
        let (px, py, w) = ((d.c * 16) as f32, (d.r * 16) as f32, meta.w * 16);
        // Paint the prop into its own canvas (rows -16.. above its tile, like the bake).
        let (cw, ch) = (w, 32);
        let mut buf = vec![0u8; (cw * ch * 4) as usize];
        {
            let mut c = crate::dungeon::prop_paint::Px { buf: &mut buf, w: cw, h: ch };
            let mut pr = crate::worldgen::rng::Mulberry32::new(
                (d.c.wrapping_mul(131).wrapping_add(d.r.wrapping_mul(17)) as u32)
                    .wrapping_add((d.kind.as_bytes()[0] as u32).wrapping_mul(2654435761)),
            );
            crate::dungeon::prop_paint::paint_prop(&mut c, d.kind, 0, 16, theme, &mut pr, d.corner);
        }
        use bevy::asset::RenderAssetUsages;
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        let img = images.add(bevy::image::Image::new(
            Extent3d { width: cw as u32, height: ch as u32, depth_or_array_layers: 1 },
            TextureDimension::D2,
            buf,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        ));
        let blocker = meta.solid.then_some((px + 1.0, py + 2.0, w as f32 - 2.0, 13.0));
        if let Some(b) = blocker {
            blockers.0.push(b);
        }
        let mut e = commands.spawn((
            DProp { kind: d.kind, c: d.c, r: d.r, blocker, debris },
            crate::combat::Combatant { team: crate::combat::Team::Object, hurt_team: None, damage: None, persistent: false, knock: 0.0 },
            crate::combat::Health { hp, max: hp, defense: 0, invuln: 0, flash: 0 },
            crate::combat::HurtProfile { invuln: 0, flash: 6, kb_base: 0.0, kb_frames: 0 },
            crate::combat::Blood(debris),
            crate::combat::Hitbox { x: px + 1.0, y: py + 2.0, w: w as f32 - 2.0, h: 13.0 },
            Sprite::from_image(img),
            crate::gfx::at(super::room_render::PLAY_X + px, super::room_render::PLAY_Y + py - 16.0, cw as f32, ch as f32, super::room_render::actor_z(py + 16.0)),
            crate::gfx::PIXEL_LAYER,
            RoomActor,
        ));
        if d.kind == "crystal" {
            // It wears the ore-node sprite, so it MINES like one: only a pick works
            // (a sword tinks off), any pick tier (Baz: "are they minable?" — now yes).
            e.insert(crate::combat::GatherTool(crate::combat::Tool::Pick, 0));
        }
    }
}

/// Smashed furniture: debris burst, half the time a little coin, rarely real gear;
/// the tile is recorded broken for the run (js makeDungeonProp deathEffect/onDeath).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub(crate) fn dprop_deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<super::battle::GameRng>,
    mut in_dungeon: ResMut<InDungeon>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut dlights: ResMut<DungeonLights>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    q: Query<(Entity, &DProp, &crate::combat::Health)>,
) {
    let Some(run) = &mut in_dungeon.0 else { return };
    for (e, prop, h) in &q {
        if h.hp > 0 {
            continue;
        }
        let (px, py) = ((prop.c * 16) as f32, (prop.r * 16) as f32);
        super::battle::spawn_burst(&mut commands, &mut rng, bevy::math::Vec2::new(px + 8.0, py + 8.0), prop.debris, 8);
        if prop.kind == "cobweb" {
            // A cut web always yields its thread (Baz) — no coin/gear, just 1 string.
            super::gather::spawn_pickup(&mut commands, &mut images, "string", 1, px + 4.0, py + 2.0, true);
        } else if prop.kind == "crystal" {
            // A mined crystal yields a gemstone, and its glow dies with it (the decor light
            // was collected at room spawn — pull this tile's entry so darkness closes in).
            super::gather::spawn_pickup(&mut commands, &mut images, "gem", 1, px + 4.0, py + 2.0, true);
            dlights.0.retain(|&(lx, ly, _)| !(lx == prop.c * 16 + 8 && ly == prop.r * 16 + 8));
        } else {
            if rng.0.next_f64() < 0.5 {
                let coins = 2 + (rng.0.next_f64() * 6.0) as i32;
                super::gather::spawn_coin(&mut commands, &mut images, coins, px + 4.0, py + 4.0);
            }
            if rng.0.next_f64() < 0.02 {
                // Smashed furniture rarely hides real gear.
                let (id, qty) = crate::items::roll_loot(0.2, 0.0, || rng.0.next_f64());
                super::gather::spawn_pickup(&mut commands, &mut images, id, qty, px + 4.0, py + 2.0, true);
            }
        }
        if let Some(b) = prop.blocker {
            blockers.0.retain(|r| *r != b);
        }
        if let Some(room) = run.dungeon.cur_mut().rooms.get_mut(&(run.drx, run.dry)) {
            room.broken.push((prop.c, prop.r));
        }
        sfx.write(super::sfx::Sfx("stone"));
        commands.entity(e).despawn();
    }
}

/// The tumble (js renderPlayfield's pitFalling draw): tip 45 degrees and shrink
/// toward the pit's centre, sinking in over PIT_FALL frames.
pub(crate) fn pit_anim(
    pits: Res<PitFalling>,
    mut players: Query<&mut Transform, With<Player>>,
) {
    let Some(f) = &pits.0 else { return };
    let Ok(mut tf) = players.single_mut() else { return };
    let k = (f.t as f32 / PIT_FALL as f32).min(1.0);
    let x = f.px + (f.cx - 8.0 - f.px) * k;
    let y = f.py + (f.cy - 8.0 - f.py) * k;
    *tf = crate::gfx::at(super::room_render::PLAY_X + x, super::room_render::PLAY_Y + y, 16.0, 16.0, 10.0);
    tf.scale = Vec3::splat((1.0 - k * 0.9).max(0.1));
    tf.rotation = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_4 * k);
}

#[derive(Component)]
struct DescendShade;
#[derive(Component)]
struct DescendText;
/// Any descent-fade overlay entity (the wash or the word) — the teardown sweep.
type AnyDescendEntity = Or<(With<DescendShade>, With<DescendText>)>;

/// The stair-walk fade (js drawDescent): a full-canvas black wash ramps up over
/// STAIR_FADE, holds at black through STAIR_HOLD, then ramps back down — the word
/// DESCENDING / CLIMBING surfacing while it is darkest. Over the HUD, under the
/// menus (layers::SLEEP), exactly like the sleep fade. The navigate tick owns the
/// timer + the floor swap; this is purely the wash.
fn descent_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    descend: Res<Descending>,
    mut shades: Query<&mut Sprite, (With<DescendShade>, Without<DescendText>)>,
    mut texts: Query<&mut Sprite, (With<DescendText>, Without<DescendShade>)>,
    ents: Query<Entity, AnyDescendEntity>,
) {
    let Some(fx) = &descend.0 else {
        for e in &ents {
            commands.entity(e).despawn();
        }
        return;
    };
    if ents.is_empty() {
        // Stand the wash + the word up once (alpha driven below).
        commands.spawn((
            Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::new(crate::CANVAS_W as f32, crate::CANVAS_H as f32)),
            at(0.0, 0.0, crate::CANVAS_W as f32, crate::CANVAS_H as f32, crate::gfx::layers::SLEEP),
            PIXEL_LAYER,
            DescendShade,
        ));
        let label = if fx.dir == 1 { "DESCENDING" } else { "CLIMBING" };
        let (img, w) = crate::gfx::font::bake_text(label, 0x9ab0e0, images.as_mut());
        // Even-padded width -> exact 2x scale, crisp glyphs (the whole-pixel law).
        let iw = (w + (w & 1)) as f32;
        let (sw, sh) = (iw * 2.0, 12.0);
        let mut sprite = Sprite::from_image(img);
        sprite.custom_size = Some(Vec2::new(sw, sh));
        sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
        commands.spawn((
            sprite,
            at(
                ((crate::CANVAS_W as f32 - sw) / 2.0).floor(),
                ((crate::CANVAS_H as f32 - sh) / 2.0).floor(),
                sw,
                sh,
                crate::gfx::layers::SLEEP + 0.01,
            ),
            PIXEL_LAYER,
            DescendText,
        ));
    }
    let t = fx.t;
    let a = if t < STAIR_FADE {
        t as f32 / STAIR_FADE as f32
    } else if t < STAIR_FADE + STAIR_HOLD {
        1.0
    } else {
        (1.0 - (t - STAIR_FADE - STAIR_HOLD) as f32 / STAIR_FADE as f32).max(0.0)
    };
    for mut s in &mut shades {
        s.color = Color::srgba(0.0, 0.0, 0.0, a);
    }
    for mut s in &mut texts {
        // js: the word shows only while a > 0.5, fading in as it darkens.
        s.color = Color::srgba(1.0, 1.0, 1.0, ((a - 0.5) / 0.5).clamp(0.0, 1.0));
    }
}

pub(crate) fn spawn_room_foes(commands: &mut Commands, room: &crate::dungeon::DRoom, rift: i32) {
    if room.cleared {
        return;
    }
    for e in &room.enemies {
        if let Some(idx) = crate::actors::mobs::def_index(e.kind) {
            let ent = commands
                .spawn((
                    crate::actors::mobs::mob_bundle(idx, e.x as f32, e.y as f32),
                    RoomActor,
                    PIXEL_LAYER,
                    DungeonFoe(e.kind),
                ))
                .id();
            if rift > 0 {
                rift_scale(commands, ent, rift);
            }
        } else {
            // Unported roster kinds wear the goblin placeholder (the overworld's rule —
            // they join real as their defs port).
            let mut ec = commands.spawn((
                crate::actors::goblin::goblin_bundle(crate::actors::goblin::GoblinKind::Melee, e.x as f32, e.y as f32),
                RoomActor,
                PIXEL_LAYER,
                DungeonFoe(e.kind),
            ));
            ec.insert(Sprite::default());
        }
    }
}

/// Write the room's SURVIVORS back into the run before leaving it (js serializeDungeon's
/// roster, kept in-run): kills stay killed, an emptied room reads cleared.
fn bank_room(
    run: &mut DungeonRun,
    mobs: &Query<(&crate::actors::mobs::Mob, &Health, &DungeonFoe), Without<Player>>,
    goblins: &Query<(&crate::actors::goblin::Goblin, &Health, &DungeonFoe), Without<Player>>,
) {
    let key = (run.drx, run.dry);
    let Some(room) = run.dungeon.cur_mut().rooms.get_mut(&key) else { return };
    let mut left: Vec<crate::dungeon::Enemy> = Vec::new();
    for (m, h, f) in mobs {
        if h.hp > 0 {
            left.push(crate::dungeon::Enemy { kind: f.0, x: m.x as i32, y: m.y as i32 });
        }
    }
    for (g, h, f) in goblins {
        if h.hp > 0 {
            left.push(crate::dungeon::Enemy { kind: f.0, x: g.x as i32, y: g.y as i32 });
        }
    }
    room.cleared = left.is_empty();
    room.enemies = left;
}

/// Stand the run's current room up through the swap context: bake -> one image
/// sprite under a fresh root; grid + blockers swap to the room's solidity.
pub(crate) fn spawn_droom(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    d: &Dungeon,
    drx: i32,
    dry: i32,
    delta: Vec2, // a sliding arrival spawns one screen over; ZERO for in-place stand-ups
    swap: &mut super::title::loader::SwapCtx,
) {
    let Some(view) = d.room_view(drx, dry) else { return };
    let img = images.add(Image::new(
        Extent3d { width: PX_W as u32, height: PX_H as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        view.rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let new_root = commands
        .spawn((Transform::from_translation(delta.extend(0.0)), Visibility::default(), RoomRoot))
        .id();
    child(commands, new_root, Sprite::from_image(img), at(PLAY_X, PLAY_Y, PX_W as f32, PX_H as f32, 1.0));
    swap.grid.0 = dungeon::to_grid(&view.solid);
    swap.blockers.0 = vec![];
    swap.active.0 = new_root;
    // The room's darkness holes: wall torches (skipped where a wide door opened their
    // wall) + lit decor (js torchLights + decorLights).
    swap.dungeon_lights.0.clear();
    // The sconce FLAMES ride as live two-frame sprites over the baked stems (the
    // town-brazier TorchAnim clock; Baz: dungeon torches must flicker). Frame 0 is
    // the old baked flame pixel-for-pixel; frame 1 sways the lick.
    const FLAME_PAL: &[(char, u32)] = &[('Y', 0xffd24a), ('F', 0xff7a1e), ('W', 0xfff0b0)];
    let flames = [
        images.add(crate::gfx::bake(&[".YY.", ".YY.", "FYYF", "FWFF", "FWFF", "FFFF"], FLAME_PAL)),
        images.add(crate::gfx::bake(&["..YY", ".YY.", "FYYF", "FFWF", "FFFF", "FFFF"], FLAME_PAL)),
    ];
    for &(fx, fy, wc, wr) in &crate::dungeon::render::TORCH_SPOTS {
        if view.solid[wr as usize][wc as usize] {
            swap.dungeon_lights.0.push((fx + 1, fy - 10, 28));
            let te = child(
                commands,
                new_root,
                Sprite::from_image(flames[0].clone()),
                at(PLAY_X + (fx - 1) as f32, PLAY_Y + (fy - 14) as f32, 4.0, 6.0, 1.05),
            );
            commands.entity(te).insert(super::room_props::TorchAnim([flames[0].clone(), flames[1].clone()]));
        }
    }
    if let Some(room) = d.cur().room(drx, dry) {
        for dc in &room.decor {
            if room.broken.contains(&(dc.c, dc.r)) {
                continue; // a mined crystal's glow died with it — stay dark on re-entry
            }
            if let Some(r) = crate::dungeon::decor::light_radius(dc.kind) {
                let w = crate::dungeon::decor::prop(dc.kind).w;
                swap.dungeon_lights.0.push((dc.c * 16 + w * 8, dc.r * 16 + 8, r));
            }
        }
    }
}

/// One chest on a tile (closed; chest_touch springs it).
pub(crate) fn spawn_chest(commands: &mut Commands, images: &mut Assets<Image>, x: i32, y: i32, hold: Option<&'static str>, gilded: bool) {
    let pal: &[(char, u32)] = if gilded { &[('D', 0x4a3a5e), ('P', 0xffd34d)] } else { &[] };
    let img = images.add(crate::gfx::bake(crate::actors::items_art::CHEST_ICON, pal));
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + x as f32, PLAY_Y + y as f32 + 2.0, 16.0, 12.0, super::room_render::actor_z(y as f32 + 14.0)),
        PIXEL_LAYER,
        RoomActor,
        Chest { hold, gilded, open: false, x: x as f32, y: y as f32 },
    ));
}

/// Stand a room's LIVE contents up (chests; foes go via spawn_room_foes): the treasure
/// chest, the small key's chest at room centre, the gilded ornate-key chest.
pub(crate) fn spawn_room_chests(commands: &mut Commands, images: &mut Assets<Image>, room: &crate::dungeon::DRoom) {
    let spawn_chest = |x: i32, y: i32, hold: Option<&'static str>, gilded: bool, commands: &mut Commands, images: &mut Assets<Image>| {
        let pal: &[(char, u32)] = if gilded { &[('D', 0x4a3a5e), ('P', 0xffd34d)] } else { &[] };
        let img = images.add(crate::gfx::bake(crate::actors::items_art::CHEST_ICON, pal));
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + x as f32, PLAY_Y + y as f32 + 2.0, 16.0, 12.0, super::room_render::actor_z(y as f32 + 14.0)),
            PIXEL_LAYER,
            RoomActor,
            Chest { hold, gilded, open: false, x: x as f32, y: y as f32 },
        ));
    };
    if room.rtype == crate::dungeon::RoomType::Treasure
        && let Some((cx, cy)) = room.chest
        && !room.looted
        && (!room.bosskey || room.bosskey_taken) // the gilded chest takes the spot alone
    {
        spawn_chest(cx, cy, None, false, commands, images);
    }
    if room.key && !room.key_taken {
        spawn_chest(9 * 16, 6 * 16, Some("key"), false, commands, images); // a small key, waiting in a chest
    }
    if room.bosskey && !room.bosskey_taken {
        // The ORNATE key's gilded chest stands ALONE (Baz: no twin beside it) — it
        // takes the treasure spot; the plain chest returns once the key is claimed.
        let (okx, oky) = room.chest.unwrap_or(if room.key { (11 * 16, 6 * 16) } else { (9 * 16, 6 * 16) });
        spawn_chest(okx, oky, Some("ornatekey"), true, commands, images);
    }
    // THE TRICK: a plain room's "bonus" chest may be teeth. Slain is slain (the ledger
    // remembers), and the real chest it coughed up waits at its spot until looted.
    if let Some((mx, my)) = room.mimic {
        if !room.mimic_slain {
            spawn_mimic(commands, images, mx, my, false);
        } else if !room.looted {
            spawn_chest(mx, my, None, false, commands, images);
        }
    }
}

/// The fake chest, shut: the SAME bake, anchor, and z as spawn_chest — nothing on
/// screen can tell it apart, which is the whole point. (pub(crate) + the `sprung`
/// flag for the shot harness — live rooms always plant it shut.)
pub(crate) fn spawn_mimic(commands: &mut Commands, images: &mut Assets<Image>, x: i32, y: i32, sprung: bool) {
    const MAW: &[(char, u32)] = &[('R', 0x8c1616), ('T', 0xe87a96), ('t', 0xc0506e)];
    let shut = images.add(crate::gfx::bake(crate::actors::items_art::CHEST_ICON, &[]));
    let frames = [
        images.add(crate::gfx::bake(crate::actors::items_art::MIMIC_OPEN_ICON, MAW)),
        images.add(crate::gfx::bake(crate::actors::items_art::MIMIC_BITE_ICON, MAW)),
    ];
    let hp = (12.0 * crate::actors::mobs::HP_MUL).round() as i32; // js mimic health 12
    commands.spawn((
        Sprite::from_image(shut),
        at(PLAY_X + x as f32, PLAY_Y + y as f32 + 2.0, 16.0, 12.0, super::room_render::actor_z(y as f32 + 14.0)),
        PIXEL_LAYER,
        RoomActor,
        MimicChest { x: x as f32, y: y as f32, home: (x, y), sprung, anim: 0, hop_t: 0, run_t: 0, cvx: 0.0, cvy: 0.0, frames, frame: 0, tongue: None, tongue_cd: if sprung { 1 } else { 40 } },
        crate::combat::Combatant { team: crate::combat::Team::Enemy, hurt_team: Some(crate::combat::Team::Player), damage: if sprung { Some(3) } else { None }, persistent: true, knock: 0.0 },
        crate::combat::Health { hp, max: hp, defense: 0, invuln: 2, flash: 0 },
        // js mimic: knockResist 0.5, the standard 11-frame flinch.
        crate::combat::HurtProfile { invuln: 10, flash: 8, kb_base: 2.2 * 0.5, kb_frames: 11 },
        crate::combat::Knockback::default(),
        crate::combat::Hitbox { x: x as f32 + 1.0, y: y as f32 + 2.0, w: 14.0, h: 12.0 },
    ));
}

/// The js GEM icon (relics.js): the shard sprite in its land's colour.
pub(crate) fn shard_image(images: &mut Assets<Image>, col: u32) -> Handle<Image> {
    const GEM: &[&str] = &["..xWWx..", ".xXXXXx.", "xXXXXXXx", "xXXWXXXx", "xXXXXXXx", ".xXXXXx.", "..xXXx..", "...XX..."];
    let lighten = |c: u32, f: f32| -> u32 {
        let ch = |i: u32| ((((c >> i) & 255) as f32 * f).min(255.0)) as u32;
        (ch(16) << 16) | (ch(8) << 8) | ch(0)
    };
    images.add(crate::gfx::bake(GEM, &[('x', lighten(col, 1.5)), ('X', col), ('W', 0xffffff)]))
}

/// Boss-room stand-ups (js loadDungeonRoom's boss branch): a living boss seals its
/// arena via navigate; a fallen one leaves the rune + shard + reward chest.
#[allow(clippy::too_many_arguments)] // a boss room is its whole context
pub(crate) fn spawn_room_boss(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    blockers: &mut super::room_props::RoomBlockers,
    rift: i32,
    mini: Option<&'static str>,
    blade_taken: bool,
    room: &crate::dungeon::DRoom,
    theme_key: &str,
    biome: Option<&str>,
    is_final: bool,
    relics: &Relics,
) {
    if room.rtype != crate::dungeon::RoomType::Boss {
        return;
    }
    // The First Bell's altar stands at the head of the sanctum through the whole
    // fight (js dungeon.saltmaze); the blade rests once the Choirmaster falls.
    if theme_key == "saltmaze" {
        super::saltmaze::spawn_sword_altar(commands, images, blockers, room.boss_loot, blade_taken);
    }
    if room.boss_loot {
        // The boss already fell: the way home, the unclaimed shard, the unopened chest.
        // (The sanctum instead: NO chest — the blade — and the way home stays sealed
        // until the Kingsplitter is taken up, js rune.sealed.)
        if theme_key != "saltmaze" || blade_taken {
            // A bare anchor — rune_tick draws the js warpRune live (pulse, ring,
            // glyph, orbiting motes; grey while the shard sits unclaimed).
            let (rx2, ry2) = if theme_key == "saltmaze" { (5.0 * 16.0, 2.0 * 16.0) } else { (9.0 * 16.0, 2.0 * 16.0) };
            commands.spawn((WarpRune { x: rx2, y: ry2 }, RoomActor));
        }
        if !room.looted && theme_key != "saltmaze" {
            spawn_chest(commands, images, 9 * 16, 7 * 16, None, true); // the reward, unclaimed
        }
        if let Some(b) = biome
            && !relics.0.contains(b)
            && let Some(r) = crate::relics_data::by_biome(b)
        {
            let img = shard_image(images, r.col);
            commands.spawn((
                Sprite::from_image(img),
                at(PLAY_X + 9.0 * 16.0 + 4.0, PLAY_Y + 6.0 * 16.0 + 4.0, 8.0, 8.0, super::room_render::actor_z(6.0 * 16.0 + 12.0)),
                PIXEL_LAYER,
                RoomActor,
                RelicShard { biome: b.to_string(), x: 9.0 * 16.0, y: 6.0 * 16.0 },
            ));
        }
        return;
    }
    if room.cleared {
        return;
    }
    // A MINI cave's stand-in (js dungeon.mini): the rolled heavyweight at x3, no
    // full boss — its fall still runs the boss_loot flow (chest + warp rune home).
    if let Some(kind) = mini {
        let (x, y) = (8.0 * 16.0, 4.0 * 16.0);
        if kind == "ogre" {
            // The roster's brute, real at last (ogre.rs) — x3 like every mini stand-in.
            let e = super::ogre::spawn_ogre(commands, x, y);
            commands.entity(e).insert((DungeonFoe("ogre"), DungeonBoss));
            commands.entity(e).entry::<Health>().and_modify(|mut h| {
                h.hp *= 3;
                h.max *= 3;
            });
            return;
        }
        if let Some(idx) = crate::actors::mobs::def_index(kind) {
            let mut ec = commands.spawn((crate::actors::mobs::mob_bundle(idx, x, y), RoomActor, PIXEL_LAYER, DungeonFoe(kind), DungeonBoss));
            ec.entry::<Health>().and_modify(|mut h| {
                h.hp *= 3;
                h.max *= 3;
            });
        } else {
            let mut ec = commands.spawn((
                crate::actors::goblin::goblin_bundle(crate::actors::goblin::GoblinKind::Melee, x, y),
                RoomActor,
                PIXEL_LAYER,
                DungeonFoe(kind),
                DungeonBoss,
            ));
            ec.insert(Sprite::default());
            ec.entry::<Health>().and_modify(|mut h| {
                h.hp *= 3;
                h.max *= 3;
            });
        }
        return;
    }
    // A rift floor's champion: the theme heavyweight, ELITE-scaled by depth (js
    // makeElite + riftScale — the affix system is flagged for later).
    if rift > 0 {
        let kind = crate::dungeon::themes::pool(theme_key)[4];
        let (x, y) = (8.0 * 16.0, 4.0 * 16.0);
        let ent = if let Some(idx) = crate::actors::mobs::def_index(kind) {
            let mut ec = commands.spawn((crate::actors::mobs::mob_bundle(idx, x, y), RoomActor, PIXEL_LAYER, DungeonFoe(kind), DungeonBoss));
            ec.entry::<Health>().and_modify(|mut h| {
                h.hp *= 3;
                h.max *= 3;
            });
            ec.id()
        } else {
            let mut ec = commands.spawn((
                crate::actors::goblin::goblin_bundle(crate::actors::goblin::GoblinKind::Melee, x, y),
                RoomActor,
                PIXEL_LAYER,
                DungeonFoe(kind),
                DungeonBoss,
            ));
            ec.insert(Sprite::default());
            ec.entry::<Health>().and_modify(|mut h| {
                h.hp *= 3;
                h.max *= 3;
            });
            ec.id()
        };
        rift_scale(commands, ent, rift);
        return;
    }
    // THE FINALE: the real THE WRIFTHEART (boss/wriftheart.rs) — no more stand-in.
    if is_final {
        super::boss::wriftheart::spawn(commands, images);
        return;
    }
    // THE TEN (BOSSES.md): themes with an AUTHORED boss spawn the real thing; the
    // rest keep the elite stand-in until their turn in the roster comes.
    if super::boss::spawn_authored(commands, images, blockers, theme_key) {
        return;
    }
    // STAND-IN BOSS (flagged): the theme roster's heavyweight, elite-scaled — the
    // authored bosses replace these one at a time (BOSSES.md).
    let kind = crate::dungeon::themes::pool(theme_key)[4];
    let scale = 6;
    let (x, y) = (8.0 * 16.0, 4.0 * 16.0);
    if let Some(idx) = crate::actors::mobs::def_index(kind) {
        let mut ec = commands.spawn((crate::actors::mobs::mob_bundle(idx, x, y), RoomActor, PIXEL_LAYER, DungeonFoe(kind), DungeonBoss));
        ec.entry::<Health>().and_modify(move |mut h| {
            h.hp *= scale;
            h.max *= scale;
        });
    } else {
        let mut ec = commands.spawn((
            crate::actors::goblin::goblin_bundle(crate::actors::goblin::GoblinKind::Melee, x, y),
            RoomActor,
            PIXEL_LAYER,
            DungeonFoe(kind),
            DungeonBoss,
        ));
        ec.insert(Sprite::default());
        ec.entry::<Health>().and_modify(move |mut h| {
            h.hp *= scale;
            h.max *= scale;
        });
    }
}

/// Walk onto a closed chest to spring it (js overlap flow): fixed contents go straight
/// to the bag; a treasure roll spills coins + a drop at the lid.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn chest_touch(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut in_dungeon: ResMut<InDungeon>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut keys: ResMut<DungeonKeys>,
    mut log: ResMut<super::rewards::LootLog>,
    discovered: Res<super::codex::items_tab::Discovered>,
    mut fanfare: ResMut<super::fanfare::Fanfare>,
    players: Query<&Player>,
    mut chests: Query<(Entity, &mut Chest, &mut Sprite, Option<&VaultChest>)>,
) {
    let Some(run) = &mut in_dungeon.0 else { return };
    let Ok(p) = players.single() else { return };
    let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    for (_e, mut chest, mut spr, vault) in &mut chests {
        if chest.open || !overlap(hitbox, (chest.x, chest.y, 16.0, 16.0)) {
            continue;
        }
        chest.open = true;
        let pal: &[(char, u32)] = if chest.gilded { &[('D', 0x4a3a5e), ('P', 0xffd34d)] } else { &[] };
        spr.image = images.add(crate::gfx::bake(crate::actors::items_art::CHEST_OPEN_ICON, pal));
        let Some(room) = run.dungeon.cur_mut().rooms.get_mut(&(run.drx, run.dry)) else { continue };
        match chest.hold {
            Some(id) => {
                // The hold contents home to you on open (js: they never drop loose), so the
                // "got it!" fanfare — which pickups_tick fires — is triggered here too (Baz:
                // "no fanfare when I got the key from the chest").
                let def = crate::items::get(id);
                log.add(id, &def.map(|d| d.name.to_uppercase()).unwrap_or_default(), 1, super::rewards::toast_color(id), false, false);
                if super::fanfare::should_play(id, &discovered) {
                    super::fanfare::begin(&mut fanfare, id);
                }
                // Keys are a per-dungeon COUNT (HUD), never bag items (Baz); anything else homes
                // to the bag as before.
                match id {
                    "key" => {
                        keys.small += 1;
                        room.key_taken = true;
                    }
                    "ornatekey" => {
                        keys.ornate += 1;
                        room.bosskey_taken = true;
                    }
                    _ => {
                        inv.add_item(id, 1);
                    }
                }
            }
            None => {
                // The js chest loot table: a gilded chest is the BOSS purse (30-69
                // coin + a boost-1.6 roll — rare the norm); a regular chest gives a
                // modest purse, a boost-0.25 roll, and half the time a little material.
                room.looted = true;
                let h = ((run.drx * 40503) ^ (run.dry * 88339) ^ (chest.x as i32 * 7) ^ (chest.y as i32 * 13)) as u32;
                let mut rng = crate::worldgen::rng::Mulberry32::new(h ^ 0xc4e57);
                if vault.is_some() {
                    // The hidden vault's cache (js secret cache: rollLoot boost 0.9 —
                    // a strong reward, shy of a boss purse).
                    let coins = 20 + (rng.next_f64() * 30.0) as i32;
                    super::gather::spawn_coin(&mut commands, &mut images, coins, chest.x + 4.0, chest.y + 2.0);
                    let (id, qty) = crate::items::roll_loot(0.9, 0.0, || rng.next_f64());
                    super::gather::spawn_pickup(&mut commands, &mut images, id, qty, chest.x + 4.0, chest.y, true);
                } else if chest.gilded {
                    let coins = 30 + (rng.next_f64() * 40.0) as i32;
                    super::gather::spawn_coin(&mut commands, &mut images, coins, chest.x + 4.0, chest.y + 2.0);
                    let (id, qty) = crate::items::roll_loot(1.6, 0.0, || rng.next_f64());
                    super::gather::spawn_pickup(&mut commands, &mut images, id, qty, chest.x + 4.0, chest.y, true);
                } else {
                    let coins = 10 + (rng.next_f64() * 24.0) as i32;
                    super::gather::spawn_coin(&mut commands, &mut images, coins, chest.x + 4.0, chest.y + 2.0);
                    let (id, qty) = crate::items::roll_loot(0.25, 0.0, || rng.next_f64());
                    super::gather::spawn_pickup(&mut commands, &mut images, id, qty, chest.x + 4.0, chest.y, true);
                    if rng.next_f64() < 0.5 {
                        let mat = if rng.next_f64() < 0.5 { "wood" } else { "stone" };
                        let q = 1 + (rng.next_f64() * 2.0) as i32;
                        super::gather::spawn_pickup(&mut commands, &mut images, mat, q, chest.x + 8.0, chest.y + 2.0, true);
                    }
                }
            }
        }
    }
}

/// The mimic's whole little brain (js enemies.js mimic ai + frogTongue). Shut:
/// furniture — `invuln` topped up so blades thunk off, no contact damage,
/// indistinguishable from the real thing. Reach for the lid (dist < 30, the js
/// trigger) and it springs: teeth frames, 3 contact damage, 1.9px lunges re-aimed
/// every 26 frames — and at mid-range it LASHES A TONGUE (the frog's deferred rig,
/// Baz's ask) that grabs and reels the hero into the maw.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn mimic_tick(
    mut commands: Commands,
    in_dungeon: Res<InDungeon>,
    grid: Res<super::play::CurGrid>,
    blockers: Res<super::room_props::RoomBlockers>,
    mut pulled: ResMut<super::play::Pulled>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<(&Player, &Health, &crate::combat::Hitbox)>,
    mut mimics: Query<
        (&mut MimicChest, &mut crate::combat::Combatant, &mut Health, &mut crate::combat::Knockback, &mut crate::combat::Hitbox, &mut Sprite, &mut Transform, &mut Visibility),
        Without<Player>,
    >,
    mut fx: Query<(&mut Sprite, &mut Transform), (With<TongueFx>, Without<MimicChest>)>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok((p, ph, phb)) = players.single() else { return };
    for (mut m, mut cb, mut h, mut kb, mut hb, mut spr, mut tf, mut vis) in &mut mimics {
        if !m.sprung {
            h.invuln = h.invuln.max(2); // just a chest... until you reach for it (js e.invuln = 2)
            let (dx, dy) = (p.x - m.x, p.y - m.y);
            if dx * dx + dy * dy < 30.0 * 30.0 {
                m.sprung = true;
                h.flash = 8;
                cb.damage = Some(3);
                sfx.write(super::sfx::Sfx("enemyDie")); // the js spring screech
            }
            continue;
        }
        m.anim += 1;
        let (pdx, pdy) = (p.x - m.x, p.y - m.y);
        let pdist = (pdx * pdx + pdy * pdy).sqrt().max(0.001);
        if let Some(mut tg) = m.tongue.take() {
            // THE LASH in flight (js frogTongue: 8f out, 3f held, 9f back; the mimic
            // stands rooted, mouth agape, the whole arc — the frog's st==2 hold).
            tg.t += 1;
            tg.len = if tg.t <= TONGUE_EXT {
                tg.t as f32 / TONGUE_EXT as f32 * tg.max_len
            } else if tg.t <= TONGUE_EXT + TONGUE_HOLD {
                tg.max_len
            } else {
                (tg.max_len * (1.0 - (tg.t - TONGUE_EXT - TONGUE_HOLD) as f32 / TONGUE_RET as f32)).max(0.0)
            };
            let (ax, ay) = (m.x + 8.0, m.y + 6.0); // the maw (js frog mouth anchor)
            let (tipx, tipy) = (ax + tg.ux * tg.len, ay + tg.uy * tg.len);
            if tg.grabbed {
                if let Some(g) = &mut pulled.0 {
                    g.tx = m.x; // the reel tracks the maw through knock-shoves (js)
                    g.ty = m.y;
                }
            } else if tg.t <= TONGUE_EXT + TONGUE_HOLD
                && ph.invuln == 0
                && pulled.0.is_none()
                && overlap((tipx - 4.0, tipy - 4.0, 8.0, 8.0), (phb.x, phb.y, phb.w, phb.h))
            {
                // The tip snags (js: player.grab(frog.x, frog.y, 28)) — reeled to the maw.
                tg.grabbed = true;
                pulled.0 = Some(super::play::Pull { tx: m.x, ty: m.y, t: 28 });
                sfx.write(super::sfx::Sfx("tink"));
            }
            // Redraw the lash: a 2px line maw->tip + a 4px tip blob (js stroke + fillRect).
            let pa = at(PLAY_X + ax, PLAY_Y + ay, 0.0, 0.0, 9.0).translation;
            let pb = at(PLAY_X + tipx, PLAY_Y + tipy, 0.0, 0.0, 9.0).translation;
            if let Ok((mut ls, mut lt)) = fx.get_mut(tg.line) {
                ls.custom_size = Some(Vec2::new((pb - pa).truncate().length().max(1.0), 2.0));
                *lt = Transform::from_translation((pa + pb) / 2.0)
                    .with_rotation(Quat::from_rotation_z((pb.y - pa.y).atan2(pb.x - pa.x)));
            }
            if let Ok((_, mut tt)) = fx.get_mut(tg.tip) {
                *tt = at(PLAY_X + tipx - 2.0, PLAY_Y + tipy - 2.0, 4.0, 4.0, 9.05);
            }
            if tg.t > TONGUE_EXT + TONGUE_HOLD + TONGUE_RET {
                commands.entity(tg.line).despawn();
                commands.entity(tg.tip).despawn();
                m.tongue_cd = 75; // js frog cooldown after a flick
            } else {
                m.tongue = Some(tg);
            }
        } else {
            m.tongue_cd -= 1;
            // Hop cycle (js): every 26 frames lock a lunge at the hero, ride it 10 frames.
            m.hop_t -= 1;
            if m.hop_t <= 0 {
                m.hop_t = 26;
                m.cvx = pdx / pdist * 1.9;
                m.cvy = pdy / pdist * 1.9;
                m.run_t = 10;
            }
            // Mid-range — too far to chomp, close enough to reach: lash at where the
            // hero STANDS (direction locked at launch, js frogTongue).
            if m.tongue_cd <= 0 && (34.0..72.0).contains(&pdist) {
                let line = commands
                    .spawn((
                        Sprite::from_color(Color::srgb_u8(0xc0, 0x50, 0x6e), Vec2::new(1.0, 2.0)),
                        at(PLAY_X + m.x + 8.0, PLAY_Y + m.y + 6.0, 0.0, 0.0, 9.0),
                        PIXEL_LAYER,
                        RoomActor,
                        TongueFx,
                    ))
                    .id();
                let tip = commands
                    .spawn((
                        Sprite::from_color(Color::srgb_u8(0xe8, 0x7a, 0x96), Vec2::new(4.0, 4.0)),
                        at(PLAY_X + m.x + 6.0, PLAY_Y + m.y + 4.0, 4.0, 4.0, 9.05),
                        PIXEL_LAYER,
                        RoomActor,
                        TongueFx,
                    ))
                    .id();
                m.tongue = Some(Tongue { ux: pdx / pdist, uy: pdy / pdist, max_len: pdist.min(62.0), len: 0.0, t: 0, grabbed: false, line, tip });
            }
        }
        let mut steps: [(f32, f32); 4] = [(0.0, 0.0); 4];
        let mut n = 0;
        if m.tongue.is_none() && m.run_t > 0 {
            // (Rooted while the tongue is out — only the knock-shove moves it then.)
            m.run_t -= 1;
            steps[0] = (m.cvx, 0.0);
            steps[1] = (0.0, m.cvy);
            n = 2;
        }
        if kb.timer > 0 {
            kb.timer -= 1;
            steps[n] = (kb.kx, 0.0);
            steps[n + 1] = (0.0, kb.ky);
            n += 2;
        }
        for &(sx, sy) in &steps[..n] {
            let (nx, ny) = (m.x + sx, m.y + sy);
            if !grid.0.box_hits_solid(nx + 1.0, ny + 2.0, 14.0, 12.0)
                && !blockers.blocks((m.x + 1.0, m.y + 2.0, 14.0, 12.0), (nx + 1.0, ny + 2.0, 14.0, 12.0))
            {
                m.x = nx;
                m.y = ny;
            }
        }
        // Chomp frames (js: anim>>3 & 1 alternates OPEN/BITE), swapped by baked handle —
        // mouth pinned OPEN for the whole lash.
        let want = if m.tongue.is_some() { 1 } else { 1 + ((m.anim >> 3) & 1) as u8 };
        if want != m.frame {
            m.frame = want;
            spr.image = m.frames[(want - 1) as usize].clone();
        }
        hb.x = m.x + 1.0;
        hb.y = m.y + 2.0;
        *tf = at(PLAY_X + m.x, PLAY_Y + m.y + 2.0, 16.0, 12.0, super::room_render::actor_z(m.y + 14.0));
        // Hit flash: skip-draw on alternating frames (js: if (e.flash & 1) return).
        *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
    }
}

/// The pounce that got greedy (js mimic drops): burst + purse + XP, and the REAL chest
/// it was guarding stands up at its home spot. Slain is forever for this dungeon.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn mimic_deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<super::battle::GameRng>,
    mut in_dungeon: ResMut<InDungeon>,
    mut progress: ResMut<super::rewards::Progress>,
    mut alloc: ResMut<super::slideout::TreeAlloc>,
    mut stats: ResMut<super::stats::Stats>,
    mut pulled: ResMut<super::play::Pulled>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    q: Query<(Entity, &MimicChest, &Health)>,
) {
    let Some(run) = &mut in_dungeon.0 else { return };
    for (e, m, h) in &q {
        if h.hp > 0 {
            continue;
        }
        if let Some(tg) = &m.tongue {
            // Died mid-lash: the tongue goes with it, and a snagged hero goes free.
            commands.entity(tg.line).despawn();
            commands.entity(tg.tip).despawn();
            if tg.grabbed {
                pulled.0 = None;
            }
        }
        super::battle::spawn_burst(&mut commands, &mut rng, Vec2::new(m.x + 8.0, m.y + 8.0), 0x7c4c1c, 10);
        let coins = 10 + (rng.0.next_f64() * 10.0) as i32; // js coin: 10 + rand*10
        super::gather::spawn_coin(&mut commands, &mut images, coins, m.x + 4.0, m.y + 4.0);
        super::rewards::gain_xp(&mut progress, &mut alloc, 25); // js xp 25
        stats.bump("kills", 1.0);
        stats.bump_kill("mimic");
        spawn_chest(&mut commands, &mut images, m.home.0, m.home.1, None, false); // it coughs up the REAL one
        if let Some(room) = run.dungeon.cur_mut().rooms.get_mut(&(run.drx, run.dry)) {
            room.mimic_slain = true;
        }
        sfx.write(super::sfx::Sfx("enemyDie"));
        commands.entity(e).despawn();
    }
}

/// One rift floor (js: riftvault, 1 floor, min(9, 4+depth) rooms, no key-hunts —
/// speed is the rift's rhythm). Depth folds into the seed so every floor differs.
fn rift_floor(base: u32, depth: i32) -> Dungeon {
    let seed = base ^ (depth as u32).wrapping_mul(0x9e37_79b9);
    dungeon::generate(seed, "riftvault", &GenOpts {
        floors: Some(1),
        room_count: Some((4 + depth).min(9) as usize),
        no_locks: true,
        rift: true,
        ..Default::default()
    })
}

/// The way DOWN after a rift champion falls (js riftGate): touch to descend.
#[derive(Component)]
pub struct RiftGate {
    pub depth: i32,
    pub x: f32,
    pub y: f32,
}

const RIFT_GATE_ART: [&str; 12] = [
    "....V..VV.....V.",
    "..VV.VV..VV..V..",
    ".V..DDDDDD..V...",
    "V.DDDDDDDDDDV...",
    ".VDDDWWDDDDD.V..",
    "V.DDDWWDDDDDDV..",
    ".VDDDDDDDWWDD.V.",
    "V..DDDDDDWWDDV..",
    ".V..DDDDDDDD.V..",
    "..VV.DDDDD.VV...",
    "....VV..VVV.....",
    "......VV........",
];
const RIFT_GATE_PAL: &[(char, u32)] = &[('V', 0xc850ff), ('D', 0x12081e), ('W', 0xe0b8ff)];

/// js riftScale: depth is its own difficulty tier — hp x(2+0.25d), damage +1+d/4 (cap 7).
fn rift_scale(commands: &mut Commands, e: Entity, depth: i32) {
    let mul = 2.0 + 0.25 * depth as f64;
    let add = (1 + depth / 4).min(6);
    commands.entity(e).entry::<Health>().and_modify(move |mut h| {
        h.hp = ((h.hp as f64) * mul).round().max(2.0) as i32;
        h.max = h.hp;
    });
    commands.entity(e).entry::<crate::combat::Combatant>().and_modify(move |mut c| {
        if let Some(d) = c.damage {
            c.damage = Some((d + add).min(7));
        }
    });
}

/// The floor's banner line ("THE VINE WARREN  1F" / "B2").
fn floor_banner_rift(banners: &mut super::banners::Banners, rift: i32) {
    banners.interior(&format!("THE RIFT - FLOOR {rift}"));
}

fn floor_banner(banners: &mut super::banners::Banners, d: &Dungeon) {
    let name = d.theme.name;
    if d.floors.len() > 1 {
        let fl = if d.floor == 0 { "1F".to_string() } else { format!("B{}", d.floor) };
        banners.interior(&format!("{name}  {fl}"));
    } else {
        banners.interior(name);
    }
}

/// Climbing out of the hidden chamber (sfx + no banner — you know where you are).
fn sfx_open(_banners: &mut super::banners::Banners) {}

/// The entry announcement (js interiorBanner): the hall's name, plus the floor
/// tally when there's more than one.
fn entry_banner(banners: &mut super::banners::Banners, d: &crate::dungeon::Dungeon) {
    if d.floors.len() > 1 {
        banners.interior(&format!("{}  1F", d.theme.name));
    } else {
        banners.interior(d.theme.name);
    }
}

/// Press INTERACT at the monument's mouth -> inside (js enterDungeon).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn enter_dungeon(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut hall: ResMut<super::guildhall::CurrentHall>,
    mut input: ResMut<ActionState>,
    mut cooldown: ResMut<DoorCooldown>,
    cur: Res<CurRoom>,
    mut swap: super::title::loader::SwapCtx,
    actors: Query<Entity, With<RoomActor>>,
    mut players: Query<(&mut Player, &mut Health, &mut crate::combat::Knockback)>,
    ledger: Res<DungeonLedger>,
    relics: Res<Relics>,
    mut log: ResMut<super::rewards::LootLog>,
    doors: Query<&super::caves::CaveDoor>,
    salt_doors: Query<&super::saltmaze::SaltDoor>,
) {
    if swap.in_dungeon.0.is_some() || cooldown.0 > 0 || !input.pressed(Action::Interact) {
        return;
    }
    let Ok((mut p, mut health, mut kb)) = players.single_mut() else { return };
    let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    // --- The Saltmaze's half-buried arch (js saltmaze press). ---
    for d in &salt_doors {
        let zone = (d.x - 4.0, d.y + 8.0, 24.0, 18.0);
        if !overlap(hitbox, zone) {
            continue;
        }
        input.consume(Action::Interact);
        // ONE maze per world, stable across visits (js seed ^ 0x5a17b311).
        let seed = swap.world.0.seed ^ 0x5a17_b311;
        let mut dgn = dungeon::generate(seed, "saltmaze", &GenOpts { floors: Some(5), maze: true, ..Default::default() });
        let entrance_key = format!("{},{}", cur.rx, cur.ry);
        apply_ledger(&mut dgn, &entrance_key, &ledger);
        commands.entity(swap.active.0).despawn();
        for a in &actors {
            commands.entity(a).despawn();
        }
        spawn_droom(&mut commands, &mut images, &dgn, 0, 0, Vec2::ZERO, &mut swap);
        entry_banner(&mut swap.banners, &dgn);
        swap.in_dungeon.0 = Some(DungeonRun {
            dungeon: dgn,
            drx: 0,
            dry: 0,
            return_pos: (cur.rx, cur.ry, p.x.round(), p.y.round()),
            entrance_key,
            biome: None,
            is_final: false,
            arena: None,
            mini: None,
            rift: 0,
            rift_base: 0,
        });
        p.x = 9.0 * 16.0;
        p.y = 10.0 * 16.0;
        p.facing = crate::actors::hero::Facing::Up;
        health.invuln = 30;
        kb.timer = 0;
        cooldown.0 = 45;
        return;
    }
    // --- Cave doors (js cavedoor/songdoor press): a crack you opened swallows you. ---
    for d in &doors {
        let (zx, zy, zw, zh) = super::caves::door_zone(d);
        if !overlap(hitbox, (zx, zy, zw, zh)) {
            continue;
        }
        input.consume(Action::Interact);
        if d.dest == "shop" {
            // The hidden shop is an INTERIOR, not a dungeon (js enterInterior('caveshop')).
            continue; // interior.rs claims this press (cave_shop_enter)
        }
        // js: seed distinct per crack — base ^ 0xca5e3d ^ imul(x+1, 2654435761) ^ imul(y+1, 40503).
        let base = (swap.world.0.seed as i32 ^ cur.rx.wrapping_mul(73856093) ^ cur.ry.wrapping_mul(19349663)) as u32;
        let seed = base
            ^ 0xca5e3d
            ^ ((d.x as i32).wrapping_add(1) as u32).wrapping_mul(2654435761)
            ^ ((d.y as i32).wrapping_add(1) as u32).wrapping_mul(40503);
        let (theme, opts, mini) = if d.dest == "mini" {
            // A SMALL cave: 5-6 rooms, an elite mini-boss, no full boss.
            const MK: [&str; 5] = ["ogre", "golem", "revenant", "charbrute", "icetroll"];
            (
                "cave",
                GenOpts { floors: Some(1), room_count: Some(5 + (seed & 1) as usize), ..Default::default() },
                Some(MK[seed as usize % MK.len()]),
            )
        } else {
            // A distinct underground-biome cavern, a full boss (no shard — bonus loot).
            const POOL: [&str; 5] = ["crystalcave", "fungal", "lavatube", "darkdepths", "frostcavern"];
            (POOL[seed as usize % POOL.len()], GenOpts { floors: Some(1 + (seed as usize % 2)), ..Default::default() }, None)
        };
        let mut dgn = dungeon::generate(seed, theme, &opts);
        let entrance_key = format!("{},{}:c{},{}", cur.rx, cur.ry, d.x as i32, d.y as i32);
        apply_ledger(&mut dgn, &entrance_key, &ledger);
        commands.entity(swap.active.0).despawn();
        for a in &actors {
            commands.entity(a).despawn();
        }
        spawn_droom(&mut commands, &mut images, &dgn, 0, 0, Vec2::ZERO, &mut swap);
        entry_banner(&mut swap.banners, &dgn);
        swap.in_dungeon.0 = Some(DungeonRun {
            dungeon: dgn,
            drx: 0,
            dry: 0,
            return_pos: (cur.rx, cur.ry, p.x.round(), p.y.round()),
            entrance_key,
            biome: None,
            is_final: false,
            arena: None,
            mini,
            rift: 0,
            rift_base: 0,
        });
        p.x = 9.0 * 16.0;
        p.y = 10.0 * 16.0;
        p.facing = crate::actors::hero::Facing::Up;
        health.invuln = 30;
        kb.timer = 0;
        cooldown.0 = 45;
        return;
    }
    let ents = swap.world.0.room_entities(cur.rx, cur.ry);
    for e in &ents {
        if e.kind == "castle" {
            // THE BLACK CASTLE GATE (js): sealed until the Wriftheart is whole; open,
            // it swallows you into the four-floor finale.
            let door = ((e.x - 16) as f32, (e.y + 5) as f32, 32.0, 33.0);
            if !overlap(hitbox, door) {
                continue;
            }
            input.consume(Action::Interact);
            let goal = swap.world.0.shard_biomes().len();
            if relics.0.len() < goal {
                log.add("seal", &format!("THE GATE IS SEALED - {} OF {goal} SHARDS", relics.0.len()), 1, 0x9a50e0, false, true);
                cooldown.0 = 45;
                return;
            }
            let seed = (swap.world.0.seed as i32 ^ cur.rx.wrapping_mul(73856093) ^ cur.ry.wrapping_mul(19349663)) as u32;
            let mut d = dungeon::generate(seed, "castle", &GenOpts { floors: Some(4), ..Default::default() });
            let entrance_key = format!("{},{}", cur.rx, cur.ry);
            apply_ledger(&mut d, &entrance_key, &ledger);
            commands.entity(swap.active.0).despawn();
            for a in &actors {
                commands.entity(a).despawn();
            }
            spawn_droom(&mut commands, &mut images, &d, 0, 0, Vec2::ZERO, &mut swap);
            floor_banner(&mut swap.banners, &d);
            swap.in_dungeon.0 = Some(DungeonRun {
                dungeon: d,
                drx: 0,
                dry: 0,
                return_pos: (cur.rx, cur.ry, p.x.round(), p.y.round()),
                entrance_key,
                biome: None, // the final boss guards no shard
                is_final: true,
                arena: None,
                mini: None,
            rift: 0,
                rift_base: 0,
            });
            p.x = 9.0 * 16.0;
            p.y = 10.0 * 16.0;
            p.facing = crate::actors::hero::Facing::Up;
            health.invuln = 30;
            kb.timer = 0;
            cooldown.0 = 45;
            return;
        }
        if e.kind == "guildhall" {
            // THE CITY'S GREAT HALL (js): one fixed, peaceful, lit floor — wings branch off.
            let door = ((e.x + 46) as f32, (e.y + 16) as f32, 20.0, 14.0); // js Entities.guildhall door
            if !overlap(hitbox, door) {
                continue;
            }
            input.consume(Action::Interact);
            let seed = (swap.world.0.seed as i32 ^ cur.rx.wrapping_mul(73856093) ^ cur.ry.wrapping_mul(19349663)) as u32;
            let mut d = dungeon::generate(seed, "guildhall", &GenOpts { guildhall: true, ..Default::default() });
            let entrance_key = format!("gh:{},{}", cur.rx, cur.ry);
            apply_ledger(&mut d, &entrance_key, &ledger);
            hall.0 = super::guildhall::city_key(&swap.world.0, cur.rx, cur.ry);
            commands.entity(swap.active.0).despawn();
            for a in &actors {
                commands.entity(a).despawn();
            }
            spawn_droom(&mut commands, &mut images, &d, 0, 0, Vec2::ZERO, &mut swap);
            swap.banners.interior("THE GUILDHALL");
            swap.in_dungeon.0 = Some(DungeonRun {
                dungeon: d,
                drx: 0,
                dry: 0,
                return_pos: (cur.rx, cur.ry, p.x.round(), p.y.round()),
                entrance_key,
                biome: None,
                is_final: false,
                arena: None,
                mini: None,
            rift: 0,
                rift_base: 0,
            });
            p.x = 9.0 * 16.0;
            p.y = 10.0 * 16.0;
            p.facing = crate::actors::hero::Facing::Up;
            health.invuln = 30;
            kb.timer = 0;
            cooldown.0 = 45;
            return;
        }
        if e.kind == "rift" {
            // THE RIFT SPIRE (js): press at the maw -> floor 1 of the endless descent.
            let door = ((e.x - 10) as f32, (e.y + 14) as f32, 36.0, 18.0);
            if !overlap(hitbox, door) {
                continue;
            }
            input.consume(Action::Interact);
            let base = (swap.world.0.seed as i32 ^ cur.rx.wrapping_mul(73856093) ^ cur.ry.wrapping_mul(19349663)) as u32;
            let d = rift_floor(base, 1);
            commands.entity(swap.active.0).despawn();
            for a in &actors {
                commands.entity(a).despawn();
            }
            spawn_droom(&mut commands, &mut images, &d, 0, 0, Vec2::ZERO, &mut swap);
            swap.banners.interior("THE RIFT - FLOOR 1");
            swap.in_dungeon.0 = Some(DungeonRun {
                dungeon: d,
                drx: 0,
                dry: 0,
                return_pos: (cur.rx, cur.ry, p.x.round(), p.y.round()),
                entrance_key: String::new(), // rifts regenerate per visit — never banked
                biome: None,
                is_final: false,
                arena: None,
                mini: None,
                rift: 1,
                rift_base: base,
            });
            p.x = 9.0 * 16.0;
            p.y = 10.0 * 16.0;
            p.facing = crate::actors::hero::Facing::Up;
            health.invuln = 30;
            kb.timer = 0;
            cooldown.0 = 45;
            return;
        }
        if e.kind != "dungeon" {
            continue;
        }
        let door = ((e.x - 4) as f32, (e.y + 8) as f32, 24.0, 18.0); // just in front of the mouth
        if !overlap(hitbox, door) {
            continue;
        }
        input.consume(Action::Interact);
        // js: seed = (World.getSeed() ^ (rx * 73856093) ^ (ry * 19349663)) >>> 0
        let seed =
            (swap.world.0.seed as i32 ^ cur.rx.wrapping_mul(73856093) ^ cur.ry.wrapping_mul(19349663)) as u32;
        let theme = THEME_BY_BIOME.iter().find(|(b, _)| *b == e.sub).map(|(_, t)| *t).unwrap_or("cave");
        let tier = crate::worldgen::World::zone_tier(cur.rx, cur.ry).clamp(0, 6) as usize;
        let mut d = dungeon::generate(seed, theme, &GenOpts { floors: Some(FLOORS_BY_TIER[tier]), ..Default::default() });
        // Dungeons are PERMANENT: overlay banked progress onto the fresh generation.
        let entrance_key = format!("{},{}", cur.rx, cur.ry);
        apply_ledger(&mut d, &entrance_key, &ledger);
        // The swap: the overworld leaves, the start room stands up.
        commands.entity(swap.active.0).despawn();
        for a in &actors {
            commands.entity(a).despawn();
        }
        spawn_droom(&mut commands, &mut images, &d, 0, 0, Vec2::ZERO, &mut swap);
        floor_banner(&mut swap.banners, &d);
        swap.in_dungeon.0 = Some(DungeonRun {
            dungeon: d,
            drx: 0,
            dry: 0,
            return_pos: (cur.rx, cur.ry, p.x.round(), p.y.round()),
            entrance_key,
            biome: Some(e.sub.clone()),
            is_final: false,
            arena: None,
            mini: None,
            rift: 0,
            rift_base: 0,
        });
        p.x = 9.0 * 16.0; // just inside the ornate bottom doorway, facing into the dungeon
        p.y = 10.0 * 16.0;
        p.facing = crate::actors::hero::Facing::Up;
        health.invuln = 30;
        kb.timer = 0;
        cooldown.0 = 45;
        return;
    }
}

/// In-dungeon movement consequences: room-to-room SLIDES through door gaps, the
/// stairs between floors, and the ornate exit (js dungeon-mode edge walk + overlaps).
/// The run is TAKEN out of the resource for the tick (borrow-free), and put back
/// unless we left — swap_world_room clears the resource on every outdoor stand-up.
#[allow(clippy::too_many_arguments, clippy::type_complexity)] // ECS params are wide; the hunt tuple bundles three tiny queries under the 16-cap
fn navigate(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut cooldown: ResMut<DoorCooldown>,
    state: Res<ActionState>,
    mut ctx: super::save::SaveCtx,
    mut swap: super::title::loader::SwapCtx,
    caves: Res<super::caves::CrackCaves>,
    songs_opened: Res<super::caves::OpenedSongstones>,
    actors: Query<Entity, With<RoomActor>>,
    mut players: Query<(&mut Player, &mut Health), Without<DungeonFoe>>,
    mobs: Query<(&crate::actors::mobs::Mob, &Health, &DungeonFoe), Without<Player>>,
    goblins: Query<(&crate::actors::goblin::Goblin, &Health, &DungeonFoe), Without<Player>>,
    mut log: ResMut<super::rewards::LootLog>,
    mut victory: ResMut<Victory>,
    mut pits: ResMut<PitFalling>,
    hunt: (
        Query<Option<&super::boss::BossName>, With<DungeonBoss>>,
        Query<(Entity, &RelicShard)>,
        MessageReader<RuneActivate>,
        Query<&RiftGate>,
        ResMut<super::saltmaze::MirrorStep>,
        ResMut<super::sidescroll::SideScroll>,
        Res<super::sidescroll::SideLooted>,
        MessageReader<super::sidescroll::ExitSide>,
        ResMut<Descending>,
        MessageWriter<super::sfx::Sfx>,
        ResMut<DungeonKeys>,
        Res<super::home::PlayerHouse>,
    ),
) {
    if swap.in_dungeon.0.is_none() {
        return;
    }
    if swap.sliding.0 {
        return; // a slide in flight owns the world
    }
    let Some(mut run) = swap.in_dungeon.0.take() else { return };
    let Ok((mut p, mut health)) = players.single_mut() else {
        swap.in_dungeon.0 = Some(run);
        return;
    };
    let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    // Capture the room's facts up front (the arena/stairs branches need &mut run).
    let (bosses, shards, mut rune_go, gates, mut mirror_step, mut side, side_looted, mut side_exits, mut descend, mut sfx, mut keys, house) = hunt;
    // --- A stair descent in flight (js updateDescent): fade out while the hero keeps
    //     stepping into the steps, swap the floor at full black, fade back in. Control
    //     is locked (play::tick early-returns on Descending, like a pit fall). ---
    if let Some(fx) = &mut descend.0 {
        fx.t += 1;
        p.facing = if fx.dir == 1 { crate::actors::hero::Facing::Down } else { crate::actors::hero::Facing::Up };
        if fx.t % 8 == 0 {
            p.anim_frame = (p.anim_frame + 1) & 3; // alternate the walk frames -> legs keep moving
        }
        if fx.t % 14 == 0 {
            sfx.write(super::sfx::Sfx("dig")); // footfalls on the steps
        }
        if fx.t < STAIR_FADE {
            p.y += if fx.dir == 1 { 0.36 } else { -0.36 }; // walk DOWN (or UP) into the steps as it fades
        }
        if !fx.applied && fx.t >= STAIR_FADE {
            // Full black -> swap floors. (The stairs block below only STARTS the walk.)
            fx.applied = true;
            let (delta, (trx, try_)) = (fx.dir, fx.target);
            bank_room(&mut run, &mobs, &goblins); // this floor remembers its dead
            super::battle::despawn_room_actors(&mut commands, &actors);
            run.dungeon.floor = (run.dungeon.floor as i32 + delta) as usize;
            run.drx = trx;
            run.dry = try_;
            commands.entity(swap.active.0).despawn();
            spawn_droom(&mut commands, &mut images, &run.dungeon, trx, try_, Vec2::ZERO, &mut swap);
            floor_banner(&mut swap.banners, &run.dungeon);
            if let Some(droom) = run.dungeon.cur().room(trx, try_) {
                spawn_room_foes(&mut commands, droom, run.rift); // maze floors post a stairs GUARD
                spawn_room_dprops(&mut commands, &mut images, droom, run.dungeon.theme, &mut swap.blockers);
                spawn_room_chests(&mut commands, &mut images, droom);
                spawn_room_secret(&mut commands, &mut images, droom, &mut swap.blockers);
                spawn_room_boss(&mut commands, &mut images, &mut swap.blockers, run.rift, run.mini, ctx.inv.has_item("kingsplitter"), droom, run.dungeon.theme.key, run.biome.as_deref(), run.is_final, &ctx.social.relics);
            }
            // Land beside the stairs, not on them (or you bounce straight back).
            p.x = 6.0 * 16.0;
            p.y = 3.0 * 16.0;
        }
        if fx.t >= STAIR_FADE * 2 + STAIR_HOLD {
            descend.0 = None;
            cooldown.0 = 45; // js floorCooldown — don't bounce onto the arrival stairs
        }
        swap.in_dungeon.0 = Some(run);
        return;
    }
    // --- The hidden side-view chamber owns the frame while it's up. ---
    if side.0.is_some() {
        if side_exits.read().next().is_some() {
            // Climb out: the secret room stands back up around the pad.
            super::battle::despawn_room_actors(&mut commands, &actors);
            commands.entity(swap.active.0).despawn();
            let (drx, dry) = (run.drx, run.dry);
            spawn_droom(&mut commands, &mut images, &run.dungeon, drx, dry, Vec2::ZERO, &mut swap);
            if let Some(droom) = run.dungeon.cur().room(drx, dry) {
                spawn_room_foes(&mut commands, droom, run.rift);
                spawn_room_dprops(&mut commands, &mut images, droom, run.dungeon.theme, &mut swap.blockers);
                spawn_room_chests(&mut commands, &mut images, droom);
                spawn_room_secret(&mut commands, &mut images, droom, &mut swap.blockers);
            }
            p.x = 4.0 * 16.0;
            p.y = 70.0;
            p.facing = crate::actors::hero::Facing::Down;
            health.invuln = 20;
            cooldown.0 = 30;
            side.0 = None;
            sfx_open(&mut swap.banners);
        }
        swap.in_dungeon.0 = Some(run);
        return;
    }
    let Some((rtype, doorv, st_down, st_up, room_pits, secret_hop)) = run.dungeon.cur().room(run.drx, run.dry).map(|room| {
        // secret_hop: where the (4,3) pad leads — down into the vault, or back up out of it.
        let hop = if room.secret_done { room.vault_key.map(|k| (k, true)) } else { room.vault_of.map(|k| (k, false)) };
        (room.rtype, [room.door(Dir::N), room.door(Dir::S), room.door(Dir::W), room.door(Dir::E)], room.stairs_down, room.stairs_up, room.pits.clone(), hop)
    }) else {
        swap.in_dungeon.0 = Some(run);
        return;
    };
    // --- A pit-fall in flight: control locked while the hero tumbles; at the end it
    // costs TWO hearts and (if he lives) spits him back out the dungeon door (js). ---
    if let Some(fall) = &mut pits.0 {
        fall.t += 1;
        if fall.t >= PIT_FALL {
            pits.0 = None;
            health.hp = (health.hp - 2).max(0);
            health.invuln = 40;
            if health.hp > 0 {
                let (orx, ory, opx, opy) = run.return_pos;
                if run.rift == 0 {
                    serialize_run(&run, &mut ctx.social.dungeon_ledger);
                }
                super::title::loader::swap_world_room(&mut commands, &mut images, &mut swap, &mut ctx, &caves, &songs_opened, &actors, orx, ory, house.0.as_ref().map(|h| h.room));
                p.x = opx;
                p.y = opy + 4.0;
                p.facing = crate::actors::hero::Facing::Down;
                cooldown.0 = 50;
                return; // (run stays taken — dropped, like every exit)
            }
            // Dead in the hole: check_death takes the screen; the run drops with the world swap there.
        }
        swap.in_dungeon.0 = Some(run);
        return;
    }
    // Step onto an open pit (js onPit at the foot point) -> the plunge begins.
    if !room_pits.is_empty() && cooldown.0 == 0 {
        let (fc, fr) = (((p.x + 8.0) / 16.0).floor() as i32, ((p.y + 11.0) / 16.0).floor() as i32);
        if room_pits.contains(&(fc, fr)) {
            pits.0 = Some(PitFall { t: 0, px: p.x, py: p.y, cx: (fc * 16 + 8) as f32, cy: (fr * 16 + 8) as f32 });
            log.add("pit", "YOU TUMBLE INTO THE DARK", 1, 0xb8a0d8, false, true);
            swap.in_dungeon.0 = Some(run);
            return;
        }
    }
    let door_at = |dd: Dir| match dd {
        Dir::N => doorv[0],
        Dir::S => doorv[1],
        Dir::W => doorv[2],
        Dir::E => doorv[3],
    };

    // --- THE BOSS ARENA (js sealBossArena / disarmBossArena). ---
    if rtype == RoomType::Boss && !swap.sliding.0 {
        let boss_alive = !bosses.is_empty();
        let key = (run.drx, run.dry);
        if run.arena.is_none() && boss_alive {
            // The doors slam shut behind you.
            let doors: Vec<Dir> = Dir::ALL.iter().copied().filter(|&dd| door_at(dd) != Door::None).collect();
            let fl = run.dungeon.cur_mut();
            let mut dirs = Vec::new();
            for dd in doors {
                if fl.locked.insert((key, dd)) {
                    dirs.push(dd);
                }
            }
            run.arena = Some(dirs);
            // THE GUARDIAN ANNOUNCES ITSELF (js boss name-splash on arena entry): the boss's
            // own name for THE TEN, a generic for the elite stand-ins that don't carry one.
            let boss_name = bosses.iter().flatten().next().map_or("THE GUARDIAN", |n| n.0);
            swap.banners.boss(boss_name); // the arena's own banner slot (SwapCtx) — one Banners per system
            commands.entity(swap.active.0).despawn();
            spawn_droom(&mut commands, &mut images, &run.dungeon, key.0, key.1, Vec2::ZERO, &mut swap);
        } else if run.arena.is_some() && !boss_alive {
            // The guardian falls: the doors reopen, the rewards appear.
            let dirs = run.arena.take().unwrap();
            {
                let fl = run.dungeon.cur_mut();
                for &dd in &dirs {
                    fl.locked.remove(&(key, dd));
                }
                if let Some(r) = fl.rooms.get_mut(&key) {
                    r.cleared = true;
                    r.boss_loot = run.rift == 0;
                }
            }
            commands.entity(swap.active.0).despawn();
            spawn_droom(&mut commands, &mut images, &run.dungeon, key.0, key.1, Vec2::ZERO, &mut swap);
            if run.rift > 0 {
                // THE RIFT CHAMPION FALLS (js): the purse chest, the way home, the way
                // DEEPER, and a free depth-tiered roll — THE endgame fountain.
                let depth = run.rift;
                spawn_chest(&mut commands, &mut images, 8 * 16, 7 * 16, None, true);
                commands.spawn((WarpRune { x: 9.0 * 16.0, y: 2.0 * 16.0 }, RoomActor)); // rune_tick draws it live
                let gate = images.add(crate::gfx::bake(&RIFT_GATE_ART, RIFT_GATE_PAL));
                commands.spawn((
                    Sprite::from_image(gate),
                    at(PLAY_X + 12.0 * 16.0, PLAY_Y + 6.0 * 16.0 + 2.0, 16.0, 12.0, 2.4),
                    PIXEL_LAYER,
                    RoomActor,
                    RiftGate { depth: depth + 1, x: 12.0 * 16.0, y: 6.0 * 16.0 },
                ));
                let mut roll_rng = crate::worldgen::rng::Mulberry32::new(run.rift_base ^ (depth as u32).wrapping_mul(77))
;
                let (id, qty) = crate::items::roll_loot(0.3 + depth as f64 * 0.3, 0.0, || roll_rng.next_f64());
                super::gather::spawn_pickup(&mut commands, &mut images, id, qty, 6.0 * 16.0, 7.0 * 16.0 + 4.0, true);
                let best = ctx.stats.0.get("riftbest").copied().unwrap_or(0.0);
                if (depth as f64) > best {
                    ctx.stats.0.insert("riftbest".into(), depth as f64);
                    log.add("rift", &format!("RIFT RECORD - FLOOR {depth}"), 1, 0xc8a0ff, false, true);
                }
                log.add("bossdown", "THE RIFT CHAMPION FALLS", 1, 0xc850ff, false, true);
            } else if let Some(droom) = run.dungeon.cur().room(key.0, key.1) {
                spawn_room_boss(&mut commands, &mut images, &mut swap.blockers, run.rift, run.mini, ctx.inv.has_item("kingsplitter"), droom, run.dungeon.theme.key, run.biome.as_deref(), run.is_final, &ctx.social.relics);
            }
            if run.is_final {
                log.add("finale", "THE FAR SIDE OF THE WOUND FALLS SILENT", 1, 0xc882ff, false, true);
                victory.t = Some(0);
                victory.won = true; // js gameWon — saved with the run's bank
            } else {
                log.add("bossdown", "THE GUARDIAN FALLS", 1, 0xffd34d, false, true);
            }
        }
    }
    // --- The shard: touch to claim the land's piece of the Wriftheart. ---
    for (e, shard) in &shards {
        if !overlap(hitbox, (shard.x, shard.y, 16.0, 16.0)) {
            continue;
        }
        commands.entity(e).despawn();
        ctx.social.relics.0.insert(shard.biome.clone());
        if let Some(r) = crate::relics_data::by_biome(&shard.biome) {
            log.add("shard", &format!("THE {} IS YOURS", r.name.to_uppercase()), 1, r.col, false, true);
        }
        let have = ctx.social.relics.0.len();
        let goal = swap.world.0.shard_biomes().len();
        log.add("shardcount", &format!("{have} OF {goal} SHARDS"), 1, 0xe0b8ff, false, true);
    }
    // --- The rift gate: touch to fall DEEPER (regenerated floor, same way home). ---
    if cooldown.0 == 0
        && let Some(g) = gates.iter().find(|g| overlap(hitbox, (g.x, g.y, 16.0, 14.0)))
    {
        let depth = g.depth;
        super::battle::despawn_room_actors(&mut commands, &actors);
        run.dungeon = rift_floor(run.rift_base, depth);
        run.rift = depth;
        run.drx = 0;
        run.dry = 0;
        run.arena = None;
        commands.entity(swap.active.0).despawn();
        spawn_droom(&mut commands, &mut images, &run.dungeon, 0, 0, Vec2::ZERO, &mut swap);
        floor_banner_rift(&mut swap.banners, depth);
        p.x = 9.0 * 16.0;
        p.y = 10.0 * 16.0;
        p.facing = crate::actors::hero::Facing::Up;
        health.invuln = 30;
        cooldown.0 = 45;
        swap.in_dungeon.0 = Some(run);
        return;
    }

    // --- The warp rune: the ride home after the kill — ACTIVATED by a press now
    //     (rune_tick owns the look, the prompt, and the press). ---
    let mut go_home = rune_go.read().next().is_some();

    // --- The ornate way out: the dungeonExit pad in the start room's south gap. ---
    go_home |= rtype == RoomType::Start && overlap(hitbox, (9.0 * 16.0, 12.0 * 16.0, 16.0, 16.0));
    if go_home && cooldown.0 == 0 {
        let (rx, ry, px, py) = run.return_pos;
        if run.rift == 0 {
            serialize_run(&run, &mut ctx.social.dungeon_ledger); // bank this run's progress before leaving (js)
        }
        // swap_world_room despawns the dungeon root (it IS the active root), stands the
        // overworld back up, and clears InDungeon (the run stays taken = dropped).
        super::title::loader::swap_world_room(&mut commands, &mut images, &mut swap, &mut ctx, &caves, &songs_opened, &actors, rx, ry, house.0.as_ref().map(|h| h.room));
        p.x = px;
        p.y = py + 4.0; // a step off the mouth (js placeAfterExit lands on the doorstep)
        p.facing = crate::actors::hero::Facing::Down;
        health.invuln = 30;
        cooldown.0 = 50; // js dungeonCooldown
        return;
    }

    // --- Stairs (the fixed 4,3 tile): descend / climb between floors. ---
    if cooldown.0 == 0 && overlap(hitbox, (4.0 * 16.0, 3.0 * 16.0, 16.0, 16.0)) {
        let (target, delta) = if st_down.is_some() {
            (st_down, 1i32)
        } else if st_up.is_some() {
            (st_up, -1)
        } else {
            (None, 0)
        };
        if let Some((trx, try_)) = target {
            // Not an instant warp — START the walk-down (js startDescent). The tick
            // block up top runs the ~3s fade and swaps the floor at full black; control
            // is locked meanwhile (play::tick early-returns on Descending).
            descend.0 = Some(DescendFx { t: 0, dir: delta, target: (trx, try_), applied: false });
            p.hop = None;
            p.hop_z = 0.0;
            p.grapple = None;
            p.blocking = false; // cancel any leap / reel / guard
            // Snap onto the steps: a tile ABOVE the 4,3 stair (down) or BELOW it (up),
            // then the tick walks the hero INTO them.
            p.x = 4.0 * 16.0;
            p.y = 3.0 * 16.0 + if delta == 1 { -16.0 } else { 16.0 };
            p.facing = if delta == 1 { crate::actors::hero::Facing::Down } else { crate::actors::hero::Facing::Up };
            health.invuln = STAIR_FADE * 2 + STAIR_HOLD + 40; // no hits through the transition + landing grace
            sfx.write(super::sfx::Sfx("open")); // step onto the stairs
            swap.in_dungeon.0 = Some(run);
            return;
        }
    }

    // --- The SECRET stairs (the (4,3) pad): down into the hidden vault, or back up.
    // Same-floor hop — the regular stairs machinery changes floors, this never does. ---
    if cooldown.0 == 0
        && let Some(((trx, try_), descending)) = secret_hop
        && overlap(hitbox, (4.0 * 16.0, 3.0 * 16.0, 16.0, 16.0))
    {
        // DEVIATION (flagged): half the mazes hide the top-down VAULT (the rs
        // stand-in that stayed), half the js SIDE-VIEW CHAMBER — split on the
        // dungeon seed so each maze's secret is forever its own.
        if descending && run.entrance_key.bytes().map(|b| b as u32).sum::<u32>() & 1 == 1 {
            bank_room(&mut run, &mobs, &goblins);
            super::battle::despawn_room_actors(&mut commands, &actors);
            commands.entity(swap.active.0).despawn();
            let key = format!("{}:{}", run.entrance_key, run.dungeon.floor);
            let st = super::sidescroll::enter_side(&mut commands, &mut images, &side_looted, key);
            p.x = (st.spawn.0 * 16) as f32;
            p.y = (st.spawn.1 * 16) as f32;
            p.facing = crate::actors::hero::Facing::Right;
            health.invuln = 20;
            side.0 = Some(st);
            swap.banners.interior("A HIDDEN PASSAGE");
            cooldown.0 = 30;
            swap.in_dungeon.0 = Some(run);
            return;
        }
        bank_room(&mut run, &mobs, &goblins);
        super::battle::despawn_room_actors(&mut commands, &actors);
        run.drx = trx;
        run.dry = try_;
        commands.entity(swap.active.0).despawn();
        spawn_droom(&mut commands, &mut images, &run.dungeon, trx, try_, Vec2::ZERO, &mut swap);
        if descending {
            swap.banners.interior("A HIDDEN VAULT");
        }
        if let Some(droom) = run.dungeon.cur().room(trx, try_) {
            spawn_room_foes(&mut commands, droom, run.rift);
            spawn_room_dprops(&mut commands, &mut images, droom, run.dungeon.theme, &mut swap.blockers);
            spawn_room_chests(&mut commands, &mut images, droom);
            spawn_room_secret(&mut commands, &mut images, droom, &mut swap.blockers);
        }
        if descending {
            p.x = 9.0 * 16.0;
            p.y = 8.0 * 16.0;
            p.facing = crate::actors::hero::Facing::Up;
        } else {
            // Back beside the shoved block's old spot, a step south of the pad.
            p.x = 4.0 * 16.0;
            p.y = 70.0;
            p.facing = crate::actors::hero::Facing::Down;
        }
        health.invuln = 20;
        cooldown.0 = 45;
        swap.in_dungeon.0 = Some(run);
        return;
    }

    // --- Push a LOCKED door (js tryLockedDoor). A locked door stays a SOLID WALL, so
    //     the tight edge-walk zone below can NEVER be reached against it (the feet box
    //     stops ~22px short; REACH is 12) — Baz: "I have the key but the door won't
    //     open." So locked doors get their OWN looser 26px push-zone (js `near`), same
    //     as the js. The right key turns it (both faces, forever); without one, a
    //     LOCKED toast + tink. ---
    {
        let (lcx, lcy) = (p.x + 8.0, p.y + 8.0);
        let near = |d: Dir| match d {
            Dir::E => lcx >= PX_W as f32 - 26.0 && lcy > 68.0 && lcy < 140.0,
            Dir::W => lcx <= 26.0 && lcy > 68.0 && lcy < 140.0,
            Dir::N => lcy <= 26.0 && lcx > 116.0 && lcx < 188.0,
            Dir::S => lcy >= PX_H as f32 - 26.0 && lcx > 116.0 && lcx < 188.0,
        };
        for d in Dir::ALL {
            let held = match d {
                Dir::N => state.held(Action::Up),
                Dir::S => state.held(Action::Down),
                Dir::W => state.held(Action::Left),
                Dir::E => state.held(Action::Right),
            };
            if !held || !near(d) || door_at(d) == Door::None {
                continue;
            }
            let Some(grand) = run.dungeon.lock(run.drx, run.dry, d) else { continue };
            if cooldown.0 == 0 {
                // Keys are the per-dungeon COUNT now, not bag items (Baz).
                let have = if grand { keys.ornate > 0 } else { keys.small > 0 };
                if have {
                    if grand {
                        keys.ornate -= 1;
                    } else {
                        keys.small -= 1;
                    }
                    // Open THIS door only (both faces), forever (js dungeon.opened).
                    let (dx2, dy2) = d.vec();
                    let both = [((run.drx, run.dry), d), ((run.drx + dx2, run.dry + dy2), d.opp())];
                    let fl = run.dungeon.cur_mut();
                    for k in both {
                        fl.locked.remove(&k);
                        fl.ornate.remove(&k);
                    }
                    log.add("unlock", if grand { "THE ORNATE DOOR SWINGS WIDE" } else { "UNLOCKED!" }, 1, if grand { 0xffd34d } else { 0xa8e0ff }, false, true);
                    sfx.write(super::sfx::Sfx("craft"));
                    // Re-bake this room in place: the door art clears, the gap opens.
                    commands.entity(swap.active.0).despawn();
                    super::battle::despawn_room_actors(&mut commands, &actors);
                    let (drx, dry) = (run.drx, run.dry);
                    spawn_droom(&mut commands, &mut images, &run.dungeon, drx, dry, Vec2::ZERO, &mut swap);
                    if let Some(droom) = run.dungeon.cur().room(drx, dry) {
                        spawn_room_foes(&mut commands, droom, run.rift);
                        spawn_room_dprops(&mut commands, &mut images, droom, run.dungeon.theme, &mut swap.blockers);
                        spawn_room_chests(&mut commands, &mut images, droom);
                        spawn_room_secret(&mut commands, &mut images, droom, &mut swap.blockers);
                        spawn_room_boss(&mut commands, &mut images, &mut swap.blockers, run.rift, run.mini, ctx.inv.has_item("kingsplitter"), droom, run.dungeon.theme.key, run.biome.as_deref(), run.is_final, &ctx.social.relics);
                    }
                } else {
                    // js: the "LOCKED" toast + tink when you shove it empty-handed.
                    log.add("lock", if grand { "LOCKED - NEEDS THE ORNATE KEY" } else { "LOCKED - NEEDS A KEY" }, 1, 0xd0d0d0, false, true);
                    sfx.write(super::sfx::Sfx("tink"));
                }
                cooldown.0 = 45; // js lockCooldown (with or without the key)
            }
            swap.in_dungeon.0 = Some(run);
            return;
        }
    }

    // --- Edge walk through a door gap -> SLIDE to the neighbouring room. ---
    let (cx, cy) = (p.x + 8.0, p.y + 8.0);
    // js dungeon edge-walk = PX_W - 12 (game.js 2420). The port had 2, which the room
    // boundary makes UNREACHABLE: OOB tiles are solid, so the feet box clamps the hero's
    // centre to ~PX_W-6 — cx never hits PX_W-2, and NO dungeon door could be walked
    // through (Baz: "there is a path right i cant walk it"). 12 matches the overworld
    // EDGE_REACH, which works with the identical clamp.
    const REACH: f32 = 12.0;
    let dir: Option<(Dir, i32, i32)> = if state.held(Action::Right) && cx >= PX_W as f32 - REACH {
        Some((Dir::E, 1, 0))
    } else if state.held(Action::Left) && cx <= REACH {
        Some((Dir::W, -1, 0))
    } else if state.held(Action::Down) && cy >= PX_H as f32 - REACH {
        Some((Dir::S, 0, 1))
    } else if state.held(Action::Up) && cy <= REACH {
        Some((Dir::N, 0, -1))
    } else {
        None
    };
    let Some((d, ddx, ddy)) = dir else {
        swap.in_dungeon.0 = Some(run);
        return;
    };
    // THE MIRROR HALLS (js): in the haze hall every exit walks you back in — unless
    // your feet sing the Maze Song (LEFT AND LEFT AND ROUND ABOUT, AND DOWN THE BELLS
    // DARK THROAT IS OUT). West at rest is honest. This MUST be judged BEFORE the
    // missing-room veto below: two of the hall's four "open" doors lead nowhere by
    // design (js genMirrorFloor — every way LOOKS open), and the js Lost Woods counted
    // a step toward them and walked you back in. Vetoing first made the hymn's N and E
    // steps silently impossible — the song could never resolve and the hall played as
    // a trap (Trello: "some dungeon openings don't work" / "enter a room and can't
    // get out").
    let mut mirror_repeat = false;
    if run.dungeon.cur().gimmick == Some("mirror")
        && run.dungeon.cur().room(run.drx, run.dry).is_some_and(|r| r.mirror)
    {
        let mirror = &mut mirror_step;
        if d == super::saltmaze::SONG[mirror.0] {
            mirror.0 += 1;
            if mirror.0 >= super::saltmaze::SONG.len() {
                mirror.0 = 0; // sung true — the south way is real; fall through
                log.add("maze", "THE HYMN RESOLVES - THE WAY OPENS", 1, 0xffd865, false, true);
            } else {
                mirror_repeat = true; // a true step, but the song is unfinished
            }
        } else if !(d == Dir::W && mirror.0 == 0) {
            mirror.0 = 0; // wrong turn: the hymn turns you around
            mirror_repeat = true;
        }
    }
    if !mirror_repeat && (door_at(d) == Door::None || run.dungeon.cur().room(run.drx + ddx, run.dry + ddy).is_none()) {
        swap.in_dungeon.0 = Some(run); // no door that way / nothing beyond
        return;
    }
    if !mirror_repeat && run.dungeon.lock(run.drx, run.dry, d).is_some() {
        // A locked door is a solid wall — the unlock is handled by the looser push-zone
        // above (the edge-walk can't reach it). This guard just never slides THROUGH one.
        swap.in_dungeon.0 = Some(run);
        return;
    }
    bank_room(&mut run, &mobs, &goblins);
    super::battle::despawn_room_actors(&mut commands, &actors);
    let (nrx, nry) = if mirror_repeat { (run.drx, run.dry) } else { (run.drx + ddx, run.dry + ddy) };
    run.drx = nrx;
    run.dry = nry;
    let delta = Vec2::new((ddx * PX_W) as f32, (-ddy * PX_H) as f32);
    let old_root = swap.active.0;
    spawn_droom(&mut commands, &mut images, &run.dungeon, nrx, nry, delta, &mut swap);
    // Cross to the opposite edge, same lane (door gaps are centred on both sides);
    // tick's slide branch scrolls the roots + lerps the player + spawns foes on landing.
    let start = Vec2::new(p.x, p.y);
    // Normally you land a hair inside the far door (js ex=2). A BOSS room lands you a
    // TILE in instead (Baz: "it spawned me on the door") — you enter, the arena slams
    // the doors behind you, and at 2px you'd be standing IN the doorway that just became
    // a wall (embedded / on the threshold). A tile of inset clears the slam.
    let is_boss = run.dungeon.cur().room(nrx, nry).is_some_and(|r| r.rtype == RoomType::Boss);
    let inset = if is_boss { 18.0 } else { 2.0 };
    let end = match d {
        Dir::E => Vec2::new(inset, p.y),
        Dir::W => Vec2::new((PX_W - 16) as f32 - inset, p.y),
        Dir::S => Vec2::new(p.x, inset),
        Dir::N => Vec2::new(p.x, (PX_H - 16) as f32 - inset),
    };
    super::play::start_slide(&mut swap.slide, &mut swap.sliding, old_root, swap.active.0, start, end, delta, ddx != 0);
    swap.in_dungeon.0 = Some(run);
}


/// Bake (and re-bake) the castle's gate state from the live shard count — the sockets
/// light as shards land, and the tenth swings the doors into the rift bloom.
fn dress_castle(
    mut images: ResMut<Assets<Image>>,
    relics: Res<Relics>,
    world: Res<GameWorld>,
    mut blockers: ResMut<super::room_props::RoomBlockers>,
    mut gates: Query<(&mut CastleGate, &mut Sprite)>,
) {
    for (mut gate, mut spr) in &mut gates {
        let goal = world.0.shard_biomes().len();
        let unlocked = relics.0.len() >= goal;
        let state = (unlocked, relics.0.len().min(goal));
        if gate.baked == Some(state) {
            continue;
        }
        let first = gate.baked.is_none();
        gate.baked = Some(state);
        let rgba = crate::actors::castle_art::castle_rgba(unlocked, state.1);
        spr.image = images.add(Image::new(
            bevy::render::render_resource::Extent3d {
                width: crate::actors::castle_art::W as u32,
                height: crate::actors::castle_art::H as u32,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            rgba,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        ));
        // The gate's solid mass: whole arch sealed, top half only when open (walk the
        // bottom half into the rift). Swap the rect on state changes.
        let sealed = (gate.x - 16.0, gate.y - 112.0, 32.0, 144.0);
        let open = (gate.x - 16.0, gate.y - 112.0, 32.0, 117.0);
        if !first {
            let stale = if unlocked { sealed } else { open };
            blockers.0.retain(|b| *b != stale);
        }
        blockers.0.push(if unlocked { open } else { sealed });
    }
}

/// The YOU WIN celebration (js victory overlay): fade up, hold, dismiss on INTERACT.
fn victory_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut victory: ResMut<Victory>,
    mut input: ResMut<ActionState>,
    overlay: Query<Entity, With<VictoryOverlay>>,
) {
    let Some(t) = &mut victory.t else { return };
    *t += 1;
    if *t == 1 {
        commands.spawn((
            Sprite::from_color(Color::srgba(0.04, 0.015, 0.08, 0.88), Vec2::new(crate::CANVAS_W as f32, crate::CANVAS_H as f32)),
            at(0.0, 0.0, crate::CANVAS_W as f32, crate::CANVAS_H as f32, 20.6),
            PIXEL_LAYER,
            VictoryOverlay,
        ));
        let cx = crate::CANVAS_W as f32 / 2.0;
        for (text, dy, col) in [("THE WRIFTHEART IS MENDED", -24.0, 0xc8a8f0u32), ("YOU WIN", -4.0, 0xffd34d), ("YOU MAY KEEP PLAYING", 26.0, 0xcfcfcf)] {
            let w = crate::gfx::font::measure(text) as f32;
            crate::ui::label(&mut commands, &mut images, text, cx - w / 2.0, crate::CANVAS_H as f32 / 2.0 + dy, col, 20.7, VictoryOverlay);
        }
    }
    if *t > 30 && input.pressed(Action::Interact) {
        input.consume(Action::Interact);
        for e in &overlay {
            commands.entity(e).despawn();
        }
        if victory.credits == 0 {
            // The mend deserves its names: a small credits card, one press to close.
            victory.credits = 1;
            commands.spawn((
                Sprite::from_color(Color::srgba(0.02, 0.01, 0.05, 0.94), Vec2::new(crate::CANVAS_W as f32, crate::CANVAS_H as f32)),
                at(0.0, 0.0, crate::CANVAS_W as f32, crate::CANVAS_H as f32, 20.6),
                PIXEL_LAYER,
                VictoryOverlay,
            ));
            let cx = crate::CANVAS_W as f32 / 2.0;
            for (text, dy, col) in [
                ("WRIFTHEART", -44.0, 0xc060ffu32),
                ("A WORLD BROKEN AND MENDED", -28.0, 0x9a8ab0),
                ("MADE BY BAZ", -4.0, 0xffd34d),
                ("EVERY SPRITE  EVERY SONG  EVERY SECRET", 8.0, 0xcfcfcf),
                ("AND YOU  WHO WALKED IT WHOLE", 32.0, 0xc8a8f0),
                ("THANK YOU FOR PLAYING", 48.0, 0xffd34d),
            ] {
                let w = crate::gfx::font::measure(text) as f32;
                crate::ui::label(&mut commands, &mut images, text, (cx - w / 2.0).floor(), crate::CANVAS_H as f32 / 2.0 + dy, col, 20.7, VictoryOverlay);
            }
        } else {
            victory.credits = 0;
            victory.t = None;
        }
    }
}

#[derive(Component)]
struct VictoryOverlay;
