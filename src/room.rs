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
    /// Overworld lava tiles (baked by `bake_lava` after from_map; empty elsewhere) —
    /// non-fireproof grounded mobs refuse to step in, and standing in it burns.
    lava: Vec<bool>,
}

impl RoomGrid {
    pub fn from_map(map: &RoomMap) -> Self {
        Self { rows: map.map.iter().map(|r| r.chars().collect()).collect(), lava: Vec::new() }
    }

    /// A grid from raw row strings (dungeon rooms synthesize theirs from solidity).
    pub fn from_rows(rows: Vec<String>) -> Self {
        Self { rows: rows.iter().map(|r| r.chars().collect()).collect(), lava: Vec::new() }
    }

    /// Bake the room's lava tiles (the water.rs overlay rule: open ground whose
    /// ground variant is lava). Overworld rooms only — dungeons/interiors stay empty.
    pub fn bake_lava(&mut self, world: &crate::worldgen::World, rx: i32, ry: i32) {
        let (gx0, gy0) = (rx * COLS, ry * ROWS);
        let mut v = vec![false; (COLS * ROWS) as usize];
        let mut any = false;
        for r in 0..ROWS {
            for c in 0..COLS {
                if self.code_at(c, r) == '.' && world.ground_name(gx0 + c, gy0 + r) == "lava" {
                    v[(r * COLS + c) as usize] = true;
                    any = true;
                }
            }
        }
        self.lava = if any { v } else { Vec::new() };
    }

    /// Is the room-pixel (x, y) on a lava tile?
    pub fn lava_at(&self, x: f32, y: f32) -> bool {
        if self.lava.is_empty() {
            return false;
        }
        let col = (x / TILE as f32).floor() as i32;
        let row = (y / TILE as f32).floor() as i32;
        (0..COLS).contains(&col) && (0..ROWS).contains(&row) && self.lava[(row * COLS + col) as usize]
    }

    /// Does an axis-aligned box touch lava? (Corner sampling, the box_hits_solid twin.)
    pub fn box_hits_lava(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        !self.lava.is_empty()
            && (self.lava_at(x, y) || self.lava_at(x + w, y) || self.lava_at(x, y + h) || self.lava_at(x + w, y + h))
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
