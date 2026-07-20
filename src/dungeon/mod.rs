//! dungeon — procedural room-based dungeon interiors (port of js/dungeon.js).
//!
//! `generate(seed, theme, opts)` builds a STACK of floors (1F/B1/B2…), each a connected
//! grid of 19x13 rooms with doors between neighbours: a start room (the ornate way out),
//! a far treasure vault, keywork (small + ornate locks), decor, pits, and themed enemy
//! rosters. Pure data + math — the Bevy render/enter/exit side lives in app/dungeon.rs.
//!
//! PARITY: the whole build is one mulberry32 stream (worldgen::rng::Mulberry32 IS the js
//! rng). Iteration orders that consume rng mirror the js exactly: rooms iterate in
//! INSERTION order (js Map semantics — see `Floor::order`), directions in n/s/w/e order
//! (js `for (const dir in DIR_VEC)`), and BFS discovery order feeds the deepest-room
//! tie-breaks. Golden fixtures pin this against the live js (tests/dungeon_parity.rs).

pub mod decor;
mod floorgen;
pub mod prop_paint;
pub mod render;
pub mod themes;

use std::collections::{HashMap, HashSet};

pub const COLS: i32 = crate::room::COLS;
pub const ROWS: i32 = crate::room::ROWS;
pub const TILE: i32 = crate::room::TILE;
pub const MIDC: i32 = COLS >> 1; // door-gap centres (9, 6)
pub const MIDR: i32 = ROWS >> 1;

/// The four door directions, in the js DIR_VEC iteration order (n, s, w, e).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum Dir {
    N,
    S,
    W,
    E,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::N, Dir::S, Dir::W, Dir::E];
    pub fn vec(self) -> (i32, i32) {
        match self {
            Dir::N => (0, -1),
            Dir::S => (0, 1),
            Dir::W => (-1, 0),
            Dir::E => (1, 0),
        }
    }
    pub fn opp(self) -> Dir {
        match self {
            Dir::N => Dir::S,
            Dir::S => Dir::N,
            Dir::W => Dir::E,
            Dir::E => Dir::W,
        }
    }
    fn idx(self) -> usize {
        match self {
            Dir::N => 0,
            Dir::S => 1,
            Dir::W => 2,
            Dir::E => 3,
        }
    }
}

/// A room's door on one wall (js doors[dir]: absent | true | 'wide').
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Door {
    #[default]
    None,
    Open,
    /// Opens most of the wall — two rooms joined this way read as ONE chamber (guildhall).
    Wide,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RoomType {
    Start,
    Arrival,
    Normal,
    Stairs,
    Treasure,
    Boss,
}

/// One placed decor prop (js data.decor entries).
#[derive(Clone, Debug)]
pub struct Decor {
    pub kind: &'static str,
    pub c: i32,
    pub r: i32,
    pub detail: bool,
    /// Corner cobwebs remember which corner they tuck into ("tl"/"tr"/"bl"/"br").
    pub corner: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub kind: &'static str,
    pub x: i32, // room px
    pub y: i32,
}

/// One dungeon room's generated data (js rooms.get(key) record).
pub struct DRoom {
    pub rx: i32,
    pub ry: i32,
    pub rtype: RoomType,
    doors: [Door; 4],
    pub visited: bool,
    pub cleared: bool,
    pub chest: Option<(i32, i32)>, // room px
    pub key: bool,                 // a small key rests here
    pub bosskey: bool,             // the ornate key rests here
    pub decor: Vec<Decor>,
    pub pits: Vec<(i32, i32)>,
    pub secret: Option<(i32, i32)>, // the push-block tile hiding a side-room
    /// The sealed treasure room this room's push-block hides (fl.rooms key at
    /// parent+(100,100)). DEVIATION (flagged): the js secret is a side-scroll
    /// gravity room — that mini-engine ports as its own later milestone; until
    /// then the block hides a HIDDEN VAULT with the js secret-cache loot roll.
    pub vault_key: Option<(i32, i32)>,
    /// This room IS a hidden vault (sealed box: chest + the way back up).
    pub vault: bool,
    /// The parent room a vault's stairs climb back to.
    pub vault_of: Option<(i32, i32)>,
    pub enemies: Vec<Enemy>,
    pub stairs_down: Option<(i32, i32)>, // arrival room on the next floor down
    pub stairs_up: Option<(i32, i32)>,   // the room above holding the way back
    pub gwing: Option<&'static str>,     // guildhall wing tag
    pub mirror: bool,                    // the Mirror Halls' repeating haze-hall
    /// A fake chest lurks here (room px). DEVIATION from js (Baz's redesign): the js
    /// mimic replaced the TREASURE room's chest with its own visibly-different sprite —
    /// the trick spoiled itself. Ours is pixel-identical and waits in a room that holds
    /// no real chest, so a "bonus" chest is either luck or teeth.
    pub mimic: Option<(i32, i32)>,
    // Run-state (js keyTaken/bosskeyTaken/looted — the persistence layer serializes these).
    pub looted: bool,
    pub key_taken: bool,
    pub bosskey_taken: bool,
    pub boss_loot: bool, // the boss fell: its court is gone, its rewards remain
    /// Smashed furniture tiles — broken stays broken for the run (js dd.broken).
    pub broken: Vec<(i32, i32)>,
    /// The mimic was slain — the real chest it coughed up stands in its place until
    /// `looted` (plain rooms never use `looted` otherwise, so the flag is free here).
    pub mimic_slain: bool,
    /// The push-block gave way — the hidden stairs stand revealed for the run.
    pub secret_done: bool,
}

impl DRoom {
    pub(crate) fn new(rx: i32, ry: i32, rtype: RoomType) -> Self {
        DRoom {
            rx,
            ry,
            rtype,
            doors: [Door::None; 4],
            visited: false,
            cleared: false,
            chest: None,
            key: false,
            bosskey: false,
            mimic: None,
            decor: Vec::new(),
            pits: Vec::new(),
            broken: Vec::new(),
            mimic_slain: false,
            secret_done: false,
            secret: None,
            vault_key: None,
            vault: false,
            vault_of: None,
            enemies: Vec::new(),
            stairs_down: None,
            stairs_up: None,
            gwing: None,
            mirror: false,
            looted: false,
            key_taken: false,
            bosskey_taken: false,
            boss_loot: false,
        }
    }
    pub fn door(&self, d: Dir) -> Door {
        self.doors[d.idx()]
    }
    pub(crate) fn set_door(&mut self, d: Dir, k: Door) {
        self.doors[d.idx()] = k;
    }
}

/// One floor: rooms in INSERTION order (js Map) + the lock sets.
pub struct Floor {
    pub order: Vec<(i32, i32)>,
    pub rooms: HashMap<(i32, i32), DRoom>,
    pub locked: HashSet<((i32, i32), Dir)>,
    pub ornate: HashSet<((i32, i32), Dir)>,
    pub deep_key: Option<(i32, i32)>,
    /// Saltmaze floor hymnwork: "dark" | "chant" | "mirror".
    pub gimmick: Option<&'static str>,
}

impl Floor {
    pub fn room(&self, rx: i32, ry: i32) -> Option<&DRoom> {
        self.rooms.get(&(rx, ry))
    }
}

/// Options for [`generate`] (the js opts bag).
#[derive(Default)]
pub struct GenOpts {
    pub floors: Option<usize>,
    pub room_count: Option<usize>,
    pub no_locks: bool,
    pub maze: bool,
    pub guildhall: bool,
    /// A rift floor: the deepest room is a (lockless) Boss arena for the elite.
    pub rift: bool,
}

/// A generated dungeon: the floor stack + current-floor cursor (js dungeon object; its
/// `rooms`/`locked` getters become `cur()`).
pub struct Dungeon {
    pub floors: Vec<Floor>,
    pub floor: usize,
    pub theme: &'static themes::Theme,
}

impl Dungeon {
    pub fn cur(&self) -> &Floor {
        &self.floors[self.floor]
    }
    pub fn cur_mut(&mut self) -> &mut Floor {
        let f = self.floor;
        &mut self.floors[f]
    }
    /// A door's lock grade at (rx,ry): None = open/unlocked, Some(false) = small lock,
    /// Some(true) = the boss's ORNATE lock.
    pub fn lock(&self, rx: i32, ry: i32, d: Dir) -> Option<bool> {
        let f = self.cur();
        if f.locked.contains(&((rx, ry), d)) {
            Some(f.ornate.contains(&((rx, ry), d)))
        } else {
            None
        }
    }
}

pub use floorgen::generate;

/// Everything the app needs to stand one room up (js makeRoom's return, minus the
/// canvas): solidity, per-door lock grades, the entrance flag, and the baked pixels.
pub struct RoomView {
    pub solid: Vec<Vec<bool>>,
    pub locks: [Option<bool>; 4], // n,s,w,e: None=open, Some(false)=small, Some(true)=boss
    pub entrance: bool,
    pub rgba: Vec<u8>, // 304x208 RGBA
}

impl Dungeon {
    /// Build a room's view on the CURRENT floor (js Dungeon.makeRoom): destructible
    /// furniture blocks via its own LIVE entity (js solidDecorTiles skips it), and
    /// pits are open holes — walk in and you fall (app/dungeon.rs pit-fall).
    pub fn room_view(&self, drx: i32, dry: i32) -> Option<RoomView> {
        let room = self.cur().room(drx, dry)?;
        let mut locks = [None; 4];
        for d in Dir::ALL {
            if room.door(d) != Door::None {
                locks[d.idx()] = self.lock(drx, dry, d);
            }
        }
        let mut features: Vec<(i32, i32)> = Vec::new();
        for dc in &room.decor {
            let p = decor::prop(dc.kind);
            if p.solid && !p.destructible {
                for i in 0..p.w {
                    features.push((dc.c + i, dc.r));
                }
            }
        }
        let entrance = room.rtype == RoomType::Start;
        let lock_at = |d: Dir| locks[d.idx()].is_some();
        let solid = solid_grid(room, &features, lock_at, entrance);
        let key = format!("{}:{},{}", self.floor, drx, dry);
        let rgba = render::bake_room(self.theme, room, &solid, locks, entrance, &key);
        Some(RoomView { solid, locks, entrance, rgba })
    }
}

/// A RoomGrid whose chars mirror a dungeon room's solidity ('M' wall / '.' floor) — the
/// whole movement/collision stack works on it unchanged.
pub fn to_grid(solid: &[Vec<bool>]) -> crate::room::RoomGrid {
    let rows: Vec<String> = solid
        .iter()
        .map(|r| r.iter().map(|&s| if s { 'M' } else { '.' }).collect())
        .collect();
    crate::room::RoomGrid::from_rows(rows)
}

/// js solidGrid: border walls with centred 3-tile door gaps + interior feature tiles.
/// Locked doors stay SOLID (shut) until a key opens them; `entrance` opens the ornate
/// way out at the bottom of the start room.
pub fn solid_grid(room: &DRoom, features: &[(i32, i32)], locks: impl Fn(Dir) -> bool, entrance: bool) -> Vec<Vec<bool>> {
    let mut g = vec![vec![false; COLS as usize]; ROWS as usize];
    g[0].fill(true);
    g[(ROWS - 1) as usize].fill(true);
    for row in g.iter_mut() {
        row[0] = true;
        row[(COLS - 1) as usize] = true;
    }
    let open = |d: Dir| room.door(d) != Door::None && !locks(d);
    let span = |d: Dir, along_cols: bool| -> (i32, i32) {
        if room.door(d) == Door::Wide {
            (2, if along_cols { COLS - 3 } else { ROWS - 3 })
        } else if along_cols {
            (MIDC - 1, MIDC + 1)
        } else {
            (MIDR - 1, MIDR + 1)
        }
    };
    if open(Dir::N) {
        let (a, b) = span(Dir::N, true);
        for c in a..=b {
            g[0][c as usize] = false;
        }
    }
    if open(Dir::S) {
        let (a, b) = span(Dir::S, true);
        for c in a..=b {
            g[(ROWS - 1) as usize][c as usize] = false;
        }
    }
    if open(Dir::W) {
        let (a, b) = span(Dir::W, false);
        for r in a..=b {
            g[r as usize][0] = false;
        }
    }
    if open(Dir::E) {
        let (a, b) = span(Dir::E, false);
        for r in a..=b {
            g[r as usize][(COLS - 1) as usize] = false;
        }
    }
    if entrance {
        for c in (MIDC - 1)..=(MIDC + 1) {
            g[(ROWS - 1) as usize][c as usize] = false; // the ornate way out (exits to the overworld)
        }
    }
    for &(c, r) in features {
        if r > 0 && r < ROWS - 1 && c > 0 && c < COLS - 1 {
            g[r as usize][c as usize] = true;
        }
    }
    g
}
