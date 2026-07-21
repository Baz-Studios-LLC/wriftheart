//! ledger.rs — BANKED dungeon progress (split from the dungeon monolith, task
//! #46): the per-entrance DgSave rows, banking a run (serialize_run) and
//! overlaying it back onto a fresh deterministic generation (apply_ledger).

use super::*;

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
pub(super) struct RoomSave {
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
pub(crate) fn serialize_run(run: &DungeonRun, ledger: &mut DungeonLedger) {
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
pub(crate) fn apply_ledger(d: &mut Dungeon, key: &str, ledger: &DungeonLedger) {
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
