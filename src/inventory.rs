//! inventory.rs — the player's unified item model (port of the bag section of js/player.js).
//!
//! EVERY owned item is an [`InvEntry`] with a per-instance uid; the bag cells, the four
//! ability slots and the six gear slots all store UIDS, never item-ids — so two copies of
//! the same item are tracked separately and can't shadow each other, and an equipped item
//! can never also linger in the bag. Items MOVE between references (bag/slots/gear); a
//! slot is a slot.

use crate::items::{self, ItemDef};
use bevy::prelude::*;

pub const BAG_COLS: usize = 8;
pub const BAG_MAX_ROWS: usize = 5; // unlocked one row at a time via tiered Satchels
pub const START_ROWS: usize = 1; // a fresh hero starts with a single 8-slot row

/// Gear-slot order — indexes into [`PlayerInv::gear`] (the js p.gear object's keys).
pub const GEAR_KEYS: [&str; 6] = ["head", "body", "feet", "trinket1", "trinket2", "trinket3"];

/// One owned item instance (js inventory record {uid, id, qty}).
pub struct InvEntry {
    pub uid: u32,
    pub id: &'static str,
    pub qty: i32,
    /// Shield wear (js e.dur): seeded from the def on the first block; None = fresh.
    pub dur: Option<i32>,
}

#[derive(Resource)]
pub struct PlayerInv {
    pub entries: Vec<InvEntry>,     // js p.inventory
    pub bag: Vec<Option<u32>>,      // js p.bagOrder — grows by rows, never shrinks
    pub bag_rows: usize,            // js p.bagRows
    pub slots: [Option<u32>; 4],    // js p.slots — the four ability slots
    pub gear: [Option<u32>; 6],     // js p.gear — GEAR_KEYS order
    pub money: i64,                 // js p.money (stays 0 until the economy ports)
    pub(crate) next_uid: u32,
}

impl Default for PlayerInv {
    fn default() -> Self {
        let mut inv = PlayerInv {
            entries: Vec::new(),
            bag: vec![None; START_ROWS * BAG_COLS],
            bag_rows: START_ROWS,
            slots: [None; 4],
            gear: [None; 6],
            money: 0,
            next_uid: 0,
        };
        // Starting loadout (js + Baz): sword in the FIRST slot, shield in the SECOND,
        // nothing else — axes and picks are earned, and the pocket watch went back to
        // the shops (the old bootstrap deviations, retired now that shields block).
        for (i, id) in ["sword", "shield"].into_iter().enumerate() {
            let uid = inv.new_entry(id, 1);
            inv.slots[i] = Some(uid);
        }
        inv
    }
}

impl PlayerInv {
    /// Mint a new inventory record (js nextUid + push), returning its uid. The record is
    /// UNREFERENCED — the caller parks it in a bag cell or slot.
    fn new_entry(&mut self, id: &'static str, qty: i32) -> u32 {
        self.next_uid += 1;
        let uid = self.next_uid;
        self.entries.push(InvEntry { uid, id, qty, dur: None });
        uid
    }

    pub fn bag_cap(&self) -> usize {
        self.bag_rows * BAG_COLS
    }

    /// Sew on another row of bag space. False if already at the max (js expandBag).
    pub fn expand_bag(&mut self) -> bool {
        if self.bag_rows >= BAG_MAX_ROWS {
            return false;
        }
        self.bag_rows += 1;
        while self.bag.len() < self.bag_cap() {
            self.bag.push(None); // the bag only ever grows
        }
        true
    }

    /// The inventory record behind a reference (js p.entry).
    pub fn entry(&self, uid: u32) -> Option<&InvEntry> {
        self.entries.iter().find(|e| e.uid == uid)
    }
    pub fn def_of(&self, uid: u32) -> Option<&'static ItemDef> {
        self.entry(uid).and_then(|e| items::get(e.id))
    }
    pub fn id_of(&self, uid: u32) -> Option<&'static str> {
        self.entry(uid).map(|e| e.id)
    }

    /// Drop ALL references to an instance (bag cell + ability slot + gear) WITHOUT deleting
    /// its record — used when moving an item between slots. Returns whether a GEAR slot
    /// emptied (the caller re-skins the hero / restats, the js refreshSprite hook).
    pub fn detach(&mut self, uid: u32) -> bool {
        for c in &mut self.bag {
            if *c == Some(uid) {
                *c = None;
            }
        }
        for s in &mut self.slots {
            if *s == Some(uid) {
                *s = None;
            }
        }
        let mut was_gear = false;
        for g in &mut self.gear {
            if *g == Some(uid) {
                *g = None;
                was_gear = true;
            }
        }
        was_gear
    }

    /// Park an (already-owned) instance in the first free bag cell (js toBag).
    pub fn to_bag(&mut self, uid: u32) {
        self.detach(uid);
        if let Some(c) = self.bag.iter_mut().find(|c| c.is_none()) {
            *c = Some(uid);
        }
    }

    /// Filled bag cells (equipped items live in slots/gear, not the bag).
    pub fn bag_used(&self) -> usize {
        self.bag.iter().flatten().count()
    }

    /// Tidy the bag: compact to the front, grouped (weapons, shields, armor, trinkets,
    /// consumables, tools, seeds, materials, the rest), best rarity first, then by name.
    pub fn sort_bag(&mut self) {
        // js groupOf — group NUMBERS preserved (seeds sit between tools and materials).
        let group_of = |d: Option<&ItemDef>| -> i32 {
            let Some(d) = d else { return 9 };
            if d.weapon {
                0
            } else if d.kind == "SHIELD" {
                1 // js: shields shelve right after weapons
            } else if matches!(d.slot, Some("head") | Some("body") | Some("feet")) {
                2
            } else if d.slot == Some("trinket") {
                3
            } else if d.kind == "SEED" {
                6 // seeds checked before consumable — they carry both flags
            } else if d.consumable {
                4
            } else if d.material {
                7
            } else {
                8 // (js group 5 = tool-flagged utility items; none ported yet)
            }
        };
        let mut uids: Vec<u32> = self.bag.iter().flatten().copied().collect();
        uids.sort_by(|a, b| {
            let (da, db) = (self.def_of(*a), self.def_of(*b));
            let g = group_of(da).cmp(&group_of(db));
            if g != std::cmp::Ordering::Equal {
                return g;
            }
            let tier = |d: Option<&ItemDef>| d.map_or(0, |d| d.rarity.tier());
            let r = tier(db).cmp(&tier(da)); // rarity DESC
            if r != std::cmp::Ordering::Equal {
                return r;
            }
            da.map_or("", |d| d.name).cmp(db.map_or("", |d| d.name))
        });
        let cap = self.bag_cap();
        self.bag = uids.into_iter().map(Some).collect();
        self.bag.resize(cap, None);
    }

    pub fn has_item(&self, id: &str) -> bool {
        self.entries.iter().any(|e| e.id == id)
    }

    /// Does ANY owned copy (bag, slot, or worn) carry this flag? The widget system's
    /// unlock check — OWNING the watch lists CLOCK in the arranger; WEARING it
    /// lights the widget (has_gear_flag below).
    pub fn owns_flagged(&self, flag: &str) -> bool {
        self.entries.iter().any(|e| items::get(e.id).is_some_and(|d| d.flags.contains(&flag)))
    }

    /// js p.hasGearFlag: does any WORN gear piece carry this power flag (clock/light/…)?
    /// Ability-slot and bag copies don't count — the item works only while worn.
    pub fn has_gear_flag(&self, flag: &str) -> bool {
        self.gear.iter().flatten().any(|u| self.def_of(*u).is_some_and(|d| d.flags.contains(&flag)))
    }

    /// Would add_item succeed? Stacks merge onto an existing stack; else need a free cell.
    pub fn can_add(&self, id: &str) -> bool {
        let def = items::get(id);
        if def.is_some_and(|d| d.unique) && self.has_item(id) {
            return false; // one-of-a-kind — already own it
        }
        if def.is_some_and(|d| d.stackable) && self.has_item(id) {
            return true; // merges, wherever the stack sits
        }
        self.bag_used() < self.bag_cap()
    }

    /// Add an item to the BAG (the player equips it manually). False (left in the world)
    /// when the bag is full — or when it's a unique you already own.
    pub fn add_item(&mut self, id: &'static str, qty: i32) -> bool {
        let def = items::get(id);
        if def.is_some_and(|d| d.unique) && self.has_item(id) {
            return false;
        }
        if def.is_some_and(|d| d.stackable)
            && let Some(e) = self.entries.iter_mut().find(|e| e.id == id)
        {
            e.qty += qty; // merges into the existing stack, wherever it sits
            return true;
        }
        let Some(cell) = self.bag.iter().position(|c| c.is_none()) else {
            return false; // bag full
        };
        let uid = self.new_entry(id, qty);
        self.bag[cell] = Some(uid);
        true
    }

    /// Delete a record and every reference to it (the shared tail of the js removers).
    fn delete_entry(&mut self, uid: u32) {
        self.entries.retain(|e| e.uid != uid);
        self.detach(uid);
    }

    /// Remove ONE of an item-id, whichever instance is found first (js removeOne).
    /// Returns whether a unit was actually removed (home-craft falls back to the chest).
    pub fn remove_one(&mut self, id: &str) -> bool {
        let Some(e) = self.entries.iter_mut().find(|e| e.id == id) else { return false };
        e.qty -= 1;
        let uid = e.uid;
        if e.qty <= 0 {
            self.delete_entry(uid); // gone -> drop every reference
        }
        true
    }

    /// Remove ONE from a SPECIFIC instance — the right copy is consumed even when several
    /// share an item-id (js removeEntry).
    pub fn remove_entry(&mut self, uid: u32) {
        let Some(e) = self.entries.iter_mut().find(|e| e.uid == uid) else { return };
        e.qty -= 1;
        if e.qty <= 0 {
            self.delete_entry(uid);
        }
    }

    /// Remove a SPECIFIC instance's whole stack; returns how many were in it (js
    /// removeEntryAll — drop/trash-stack).
    pub fn remove_entry_all(&mut self, uid: u32) -> i32 {
        let Some(e) = self.entries.iter().find(|e| e.uid == uid) else { return 0 };
        let qty = e.qty;
        self.delete_entry(uid);
        qty
    }

    /// Remove an item's WHOLE stack and return how many were removed (js removeStack —
    /// storage transfers).
    pub fn remove_stack(&mut self, id: &str) -> i32 {
        let Some(e) = self.entries.iter().find(|e| e.id == id) else { return 0 };
        let uid = e.uid;
        let qty = e.qty;
        self.delete_entry(uid);
        qty
    }

    /// Total count of an item-id across every stack (js itemCount).
    /// Bag total across every FISH-kind stack (the "@FISH" recipe wildcard).
    pub fn count_fish(&self) -> i32 {
        self.entries.iter().filter(|e| crate::items::get(e.id).is_some_and(|d| d.kind == "FISH")).map(|e| e.qty).sum()
    }

    /// Consume one fish for an "@FISH" line — the CHEAPEST in the bag goes first (js rule).
    pub fn remove_cheapest_fish(&mut self) {
        let target = self
            .entries
            .iter()
            .filter(|e| crate::items::get(e.id).is_some_and(|d| d.kind == "FISH"))
            .min_by_key(|e| crate::items::sell_price_of(e.id))
            .map(|e| e.id);
        if let Some(id) = target {
            self.remove_one(id);
        }
    }

    pub fn count(&self, id: &str) -> i32 {
        self.entries.iter().filter(|e| e.id == id).map(|e| e.qty).sum()
    }

    /// js autoEquipPickup: a picked-up item fills an EMPTY matching slot automatically —
    /// armor/trinkets to their gear slot, weapons/tools/consumables to a free ability
    /// slot. It NEVER swaps out something you chose, and a stack already on the bar just
    /// grows. Returns true if it equipped (the pickup toast says so).
    pub fn auto_equip(&mut self, id: &str) -> bool {
        let referenced = |inv: &Self, uid: u32| {
            inv.slots.contains(&Some(uid)) || inv.gear.contains(&Some(uid))
        };
        if let Some(gs) = items::gear_slot(id) {
            let slot = if gs == "trinket" {
                (3..6).find(|g| self.gear[*g].is_none())
            } else {
                GEAR_KEYS.iter().position(|k| *k == gs).filter(|g| self.gear[*g].is_none())
            };
            let Some(g) = slot else { return false };
            let Some(uid) = self.entries.iter().rev().find(|e| e.id == id && !referenced(self, e.uid)).map(|e| e.uid) else {
                return false;
            };
            // (js equipGear; unlike the js we detach the bag reference too — the uid
            // model forbids an equipped item lingering in a bag cell.)
            self.detach(uid);
            self.gear[g] = Some(uid);
            return true;
        }
        if items::equippable(id) {
            let Some(free) = self.slots.iter().position(|u| u.is_none()) else { return false };
            if self.slots.iter().flatten().any(|u| self.id_of(*u) == Some(id)) {
                return false; // already on the bar (stacks merge into it)
            }
            let Some(uid) = self.entries.iter().rev().find(|e| e.id == id && !self.slots.contains(&Some(e.uid))).map(|e| e.uid) else {
                return false;
            };
            self.detach(uid);
            self.slots[free] = Some(uid);
            return true;
        }
        false
    }

    /// Would `auto_equip` find an EMPTY home for this id right now — its gear slot
    /// (trinkets: any of the three) or, for bar equippables, a free ability slot?
    /// Mirrors auto_equip's own rules exactly.
    pub fn slot_room(&self, id: &str) -> bool {
        if let Some(gs) = items::gear_slot(id) {
            return if gs == "trinket" {
                (3..6).any(|g| self.gear[g].is_none())
            } else {
                GEAR_KEYS.iter().position(|k| *k == gs).is_some_and(|g| self.gear[g].is_none())
            };
        }
        items::equippable(id)
            && self.slots.iter().any(|u| u.is_none())
            && !self.slots.iter().flatten().any(|u| self.id_of(*u) == Some(id))
    }

    /// Bank a ground drop: add_item + the auto-equip courtesy — and when the BAG is
    /// full but an empty slot would wear it (Baz: boots drop, feet slot empty, bag
    /// full — put them ON), the entry is born detached and auto_equip claims it in
    /// the same breath. slot_room is checked FIRST, so a floating unreferenced entry
    /// can never be created. Some(equipped) if taken, None if there is truly no room.
    /// NOT for shops/quests — those want add_item's plain bag semantics.
    pub fn take_drop(&mut self, id: &'static str, qty: i32) -> Option<bool> {
        if self.add_item(id, qty) {
            return Some(self.auto_equip(id));
        }
        if items::get(id).is_some_and(|d| d.unique) && self.has_item(id) {
            return None; // one-of-a-kind — already own it
        }
        if self.slot_room(id) {
            self.new_entry(id, qty);
            return Some(self.auto_equip(id)); // slot_room guarantees the claim
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_loadout() {
        let inv = PlayerInv::default();
        assert_eq!(inv.entries.len(), 4);
        assert_eq!(inv.id_of(inv.slots[0].unwrap()), Some("sword"));
        assert_eq!(inv.id_of(inv.slots[1].unwrap()), Some("axe"));
        assert_eq!(inv.id_of(inv.slots[2].unwrap()), Some("pick"));
        assert_eq!(inv.id_of(inv.gear[3].unwrap()), Some("pocketwatch")); // trinket 1
        assert!(inv.has_gear_flag("clock"));
        assert_eq!(inv.bag_used(), 0); // equipped items are NOT in the bag
    }

    #[test]
    fn stacking_and_capacity() {
        let mut inv = PlayerInv::default();
        assert!(inv.add_item("wood", 3));
        assert!(inv.add_item("wood", 2)); // merges
        assert_eq!(inv.bag_used(), 1);
        assert_eq!(inv.count("wood"), 5);
        // Fill the remaining 7 cells with distinct one-off stacks.
        for id in ["stone", "fiber", "herb", "copper", "potion"] {
            assert!(inv.add_item(id, 1));
        }
        assert!(inv.add_item("sword", 1)); // a second sword: new instance, own cell
        assert!(inv.add_item("axe", 1));
        assert_eq!(inv.bag_used(), 8);
        assert!(!inv.add_item("pick", 1)); // bag full — non-stackable bounces
        assert!(inv.can_add("wood")); // ...but a stackable still merges
        assert!(inv.add_item("wood", 1));
        assert_eq!(inv.count("wood"), 6);
    }

    #[test]
    fn remove_entry_clears_references() {
        let mut inv = PlayerInv::default();
        inv.add_item("potion", 2);
        let uid = inv.bag[0].unwrap();
        inv.remove_entry(uid);
        assert_eq!(inv.count("potion"), 1);
        assert_eq!(inv.bag[0], Some(uid)); // stack survives in place
        inv.remove_entry(uid);
        assert_eq!(inv.count("potion"), 0);
        assert_eq!(inv.bag[0], None); // last one -> the cell empties
    }

    #[test]
    fn sort_groups_and_rarity() {
        let mut inv = PlayerInv::default();
        inv.add_item("wood", 5); // material (7)
        inv.add_item("potion", 1); // consumable (4)
        inv.add_item("sword", 1); // weapon (0)
        inv.sort_bag();
        let ids: Vec<_> = inv.bag.iter().flatten().map(|u| inv.id_of(*u).unwrap()).collect();
        assert_eq!(ids, ["sword", "potion", "wood"]);
    }
}
