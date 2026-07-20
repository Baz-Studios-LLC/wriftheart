//! room.rs — one screen of the world: the tile grid + "is this pixel solid?" (port of js/room.js).

use crate::tiles;
use crate::worldgen::RoomMap;

pub const COLS: i32 = 19;
pub const ROWS: i32 = 13;
pub const TILE: i32 = 16;
pub const PX_W: i32 = COLS * TILE; // 304
pub const PX_H: i32 = ROWS * TILE; // 208

/// A room's tile grid with the collision queries every mover needs.
pub struct RoomGrid {
    rows: Vec<Vec<char>>,
}

impl RoomGrid {
    pub fn from_map(map: &RoomMap) -> Self {
        Self { rows: map.map.iter().map(|r| r.chars().collect()).collect() }
    }

    /// A grid from raw row strings (dungeon rooms synthesize theirs from solidity).
    pub fn from_rows(rows: Vec<String>) -> Self {
        Self { rows: rows.iter().map(|r| r.chars().collect()).collect() }
    }

    /// Tile code at (col, row); out of bounds reads as a tree wall, exactly like the JS.
    pub fn code_at(&self, col: i32, row: i32) -> char {
        if !(0..ROWS).contains(&row) || !(0..COLS).contains(&col) {
            return 'T';
        }
        self.rows[row as usize][col as usize]
    }

    /// Is the room-pixel (x, y) inside a solid tile?
    pub fn solid_at(&self, x: f32, y: f32) -> bool {
        let col = (x / TILE as f32).floor() as i32;
        let row = (y / TILE as f32).floor() as i32;
        tiles::is_solid(self.code_at(col, row))
    }

    /// Does an axis-aligned box collide with any solid tile? (Corner sampling, like the JS —
    /// valid because every box in play is smaller than a tile.)
    pub fn box_hits_solid(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.solid_at(x, y)
            || self.solid_at(x + w - 1.0, y)
            || self.solid_at(x, y + h - 1.0)
            || self.solid_at(x + w - 1.0, y + h - 1.0)
    }
}

/// Enter-only static-blocker test (solid props): a mover may keep overlapping a box it is
/// already inside (so landing on one never traps you) but may not move INTO one.
pub fn blockers_block(
    blockers: &[(f32, f32, f32, f32)],
    from: (f32, f32, f32, f32),
    to: (f32, f32, f32, f32),
) -> bool {
    let hit = |b: &(f32, f32, f32, f32), r: (f32, f32, f32, f32)| {
        r.0 < b.0 + b.2 && r.0 + r.2 > b.0 && r.1 < b.1 + b.3 && r.1 + r.3 > b.1
    };
    blockers.iter().any(|b| hit(b, to) && !hit(b, from))
}
