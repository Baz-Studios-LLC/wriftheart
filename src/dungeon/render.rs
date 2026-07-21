//! render.rs — bake one dungeon room to a 304x208 RGBA buffer (js/dungeon.js render()):
//! themed walls + floors (brick / raw cave rock / guildhall timber), cave lips + still
//! pools, locked-door art (small padlock / the boss's gilded horned door), pits, the
//! decor props (prop_paint), wall torches, and the ornate entrance out.
//!
//! DEVIATION (flagged, temporary): destructible furniture is baked in and kept solid —
//! the js spawns it as smashable live entities; those arrive with the dungeon combat
//! pass. Stairs are also painted here (js spawns stairs entities; ours are baked +
//! rect-checked until the entity port).

use super::decor::prop;
use super::prop_paint::{paint_prop, Px};
use super::themes::{Style, Theme};
use super::{DRoom, COLS, MIDC, MIDR, ROWS, TILE};
use crate::room::{PX_H, PX_W};
use crate::worldgen::rng::Mulberry32;

/// js hashK — FNV-1a over the room cache key ("floor:rx,ry"), seeds the rock/pool rng.
fn hash_k(s: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in s.bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    h
}

fn tile_rng(rk: u32, col: i32, row: i32) -> Mulberry32 {
    Mulberry32::new(rk ^ (col + 1).wrapping_mul(0x9e3779b1u32 as i32) as u32 ^ (row + 1).wrapping_mul(40503) as u32)
}

/// Wall-sconce torch spots (js TORCH_SPOTS) — (fx, fy, wall col, wall row).
pub const TORCH_SPOTS: [(i32, i32, i32, i32); 8] = [
    (4 * TILE + 7, TILE + 10, 4, 0),
    (14 * TILE + 7, TILE + 10, 14, 0),
    (4 * TILE + 7, (ROWS - 1) * TILE - 1, 4, ROWS - 1),
    (14 * TILE + 7, (ROWS - 1) * TILE - 1, 14, ROWS - 1),
    (TILE + 3, 3 * TILE + 13, 0, 3),
    (TILE + 3, 9 * TILE + 13, 0, 9),
    ((COLS - 1) * TILE - 5, 3 * TILE + 13, COLS - 1, 3),
    ((COLS - 1) * TILE - 5, 9 * TILE + 13, COLS - 1, 9),
];

fn draw_torch(c: &mut Px, fx: i32, fy: i32) {
    // Stem + bracket only — the FLAME rides as a live two-frame sprite over the
    // bake (app/dungeon.rs spawn_droom; Baz: dungeon torches must flicker).
    c.rect(0x5a3a1c, fx, fy - 6, 2, 6);
    c.rect(0x2a1a0e, fx - 1, fy - 8, 4, 2);
}

/// A banded locked door over a (shut) gap; the boss grade is gilded + horned.
fn lock_door(c: &mut Px, x: i32, y: i32, horiz: bool, boss: bool) {
    let (w, h) = if horiz { (3 * TILE, TILE) } else { (TILE, 3 * TILE) };
    c.rect(if boss { 0x2e2438 } else { 0x5a3a1c }, x, y, w, h);
    let seam = if boss { 0x1a1424 } else { 0x3a2410 };
    if horiz {
        c.rect(seam, x + TILE, y, 1, h);
        c.rect(seam, x + 2 * TILE, y, 1, h);
    } else {
        c.rect(seam, x, y + TILE, w, 1);
        c.rect(seam, x, y + 2 * TILE, w, 1);
    }
    let lx = x + w / 2;
    let ly = y + h / 2;
    if boss {
        if horiz {
            c.rect(0xcaa84a, x, y, w, 1);
            c.rect(0xcaa84a, x, y + h - 1, w, 1);
        } else {
            c.rect(0xcaa84a, x, y, 1, h);
            c.rect(0xcaa84a, x + w - 1, y, 1, h);
        }
        c.rect(0xcaa84a, lx - 6, ly - 5, 2, 3);
        c.rect(0xcaa84a, lx + 4, ly - 5, 2, 3);
        c.rect(0xcaa84a, lx - 4, ly - 2, 8, 7);
        c.rect(0x7a44c8, lx - 1, ly, 2, 3);
        c.rect(0x8a6e28, lx - 4, ly + 4, 8, 1);
    } else {
        c.rect(0xfcd000, lx - 3, ly - 1, 6, 5);
        c.rect(0x7a5a00, lx - 1, ly + 1, 2, 2);
        // The js strokes a shackle arc; two posts + a lid read the same at this size.
        c.rect(0xcaa000, lx - 2, ly - 3, 1, 2);
        c.rect(0xcaa000, lx + 1, ly - 3, 1, 2);
        c.rect(0xcaa000, lx - 1, ly - 4, 2, 1);
    }
}

/// The grand decorated doorway over the start room's south gap (js drawEntrance).
fn draw_entrance(c: &mut Px, th: &Theme) {
    let gx = (MIDC - 1) * TILE;
    let gw = 3 * TILE;
    let sy = (ROWS - 1) * TILE;
    c.rect(0x13121a, gx, sy, gw, TILE);
    c.rect(0x08070d, gx + 3, sy + 3, gw - 6, TILE - 3);
    c.rect(th.grout, gx, sy, gw, 1);
    for px in [(MIDC - 2) * TILE, (MIDC + 2) * TILE] {
        c.rect(th.wall_top, px + 2, sy - 6, TILE - 4, TILE + 6);
        c.rect(th.grout, px + 2, sy - 2, TILE - 4, 1);
        c.rect(th.grout, px + 2, sy + 6, TILE - 4, 1);
        c.rect(0xcaa84a, px + 5, sy - 5, TILE - 10, 2);
    }
    let ay = sy - TILE;
    c.rect(th.wall_top, gx - TILE + 4, ay + 4, gw + 2 * TILE - 8, 2);
    c.rect(th.wall, gx - TILE + 4, ay + 6, gw + 2 * TILE - 8, 4);
    c.rect(0xcaa84a, gx + gw / 2 - 3, ay + 2, 6, 9);
    c.rect(0x8a6e28, gx + gw / 2 - 1, ay + 5, 2, 5);
    c.blend(0xffbe6e, 31, gx, sy - 4, gw, 6); // warm glow from the doorway
}

/// Simple stairs (PORT-ORIGINAL stand-in until the stairs entities port): stone steps
/// descending into dark (down) or rising toward light (up), on the fixed (4,3) tile.
fn draw_stairs(c: &mut Px, th: &Theme, x: i32, y: i32, down: bool) {
    c.rect(th.grout, x, y, TILE, TILE);
    for i in 0..4 {
        let shade = if down { [0x9a9a9a, 0x6e6e6e, 0x4a4a4a, 0x262626][i] } else { [0x262626, 0x4a4a4a, 0x6e6e6e, 0x9a9a9a][i] };
        c.rect(shade, x + 1, y + 1 + i as i32 * 4, TILE - 2, 4);
    }
    c.rect(th.wall_top, x, y, TILE, 1);
}

/// Bake the room. `locks[dir]`: None = open, Some(false) = small lock, Some(true) = boss.
/// `cache_key` seeds the per-room rock (js render's hashK(k)).
#[allow(clippy::too_many_arguments)] // a room bake is its whole context
pub fn bake_room(
    th: &Theme,
    room: &DRoom,
    solid: &[Vec<bool>],
    locks: [Option<bool>; 4], // n, s, w, e
    entrance: bool,
    cache_key: &str,
) -> Vec<u8> {
    let mut buf = vec![0u8; (PX_W * PX_H * 4) as usize];
    let mut c = Px { buf: &mut buf, w: PX_W, h: PX_H };
    let rk = hash_k(cache_key);
    c.rect(th.floor, 0, 0, PX_W, PX_H);
    for row in 0..ROWS {
        for col in 0..COLS {
            let (px, py) = (col * TILE, row * TILE);
            if solid[row as usize][col as usize] {
                match th.style {
                    Style::Hall => {
                        c.rect(th.wall, px, py, TILE, TILE);
                        c.rect(th.wall_top, px, py, TILE, 3);
                        c.rect(th.grout, px, py + TILE - 1, TILE, 1);
                        c.rect(th.grout, px + 5, py + 3, 1, TILE - 4);
                        c.rect(th.grout, px + 10, py + 3, 1, TILE - 4);
                        c.rect(0x7a5f42, px, py + 11, TILE, 1);
                    }
                    Style::Cave => {
                        let mut tr = tile_rng(rk, col, row);
                        let mut ri = |n: f64| (tr.next_f64() * n) as i32;
                        c.rect(th.wall, px, py, TILE, TILE);
                        for _ in 0..3 {
                            let (bx, by) = (ri(11.0), ri(11.0));
                            c.rect(th.wall_top, px + bx, py + by, 2 + ri(4.0), 2 + ri(3.0));
                        }
                        for _ in 0..2 {
                            let (bx, by) = (ri(12.0), ri(13.0));
                            c.rect(th.grout, px + bx, py + by, 2 + ri(4.0), 1);
                        }
                        let (bx, by) = (ri(14.0), ri(6.0));
                        c.rect(th.grout, px + bx, py + by, 1, 4 + ri(8.0));
                    }
                    Style::Brick => {
                        c.rect(th.wall, px, py, TILE, TILE);
                        c.rect(th.wall_top, px, py, TILE, 3);
                        c.rect(th.grout, px, py + TILE - 1, TILE, 1);
                        c.rect(th.grout, px, py + 8, TILE, 1);
                        c.rect(th.grout, px + if row & 1 == 1 { 8 } else { 0 }, py, 1, 8);
                        c.rect(th.grout, px + if row & 1 == 1 { 0 } else { 8 }, py + 8, 1, 8);
                    }
                }
            } else {
                match th.style {
                    Style::Hall => {
                        c.rect(if row & 1 == 1 { th.floor } else { th.floor_alt }, px, py, TILE, TILE);
                        c.rect(0x6f5a32, px, py + TILE - 1, TILE, 1);
                        c.rect(0x82693c, px + if row & 1 == 1 { 8 } else { 0 }, py, 1, TILE);
                    }
                    Style::Cave => {
                        let mut tr = tile_rng(rk, col, row);
                        c.rect(th.floor, px, py, TILE, TILE);
                        if tr.next_f64() < 0.6 {
                            let (bx, by) = ((tr.next_f64() * 8.0) as i32, (tr.next_f64() * 8.0) as i32);
                            let (bw, bh) = (4 + (tr.next_f64() * 9.0) as i32, 3 + (tr.next_f64() * 7.0) as i32);
                            c.rect(th.floor_alt, px + bx, py + by, bw, bh);
                        }
                        for _ in 0..2 {
                            if tr.next_f64() < 0.6 {
                                let col2 = if tr.next_f64() < 0.6 { th.grout } else { th.wall_top };
                                let (bx, by) = ((tr.next_f64() * 14.0) as i32, (tr.next_f64() * 14.0) as i32);
                                c.rect(col2, px + bx, py + by, 1, 1);
                            }
                        }
                    }
                    Style::Brick => {
                        c.rect(if (col + row) & 1 == 1 { th.floor_alt } else { th.floor }, px, py, TILE, TILE);
                        c.rect(th.grout, px, py, TILE, 1);
                        c.rect(th.grout, px, py, 1, TILE);
                    }
                }
            }
        }
    }
    if th.style == Style::Cave {
        // Rough lips wherever rock meets open floor — a cave mouth was never cut straight.
        for r2 in 0..ROWS {
            for col in 0..COLS {
                if !solid[r2 as usize][col as usize] {
                    continue;
                }
                let (px, py) = (col * TILE, r2 * TILE);
                let mut tr = Mulberry32::new(rk ^ (col + 3).wrapping_mul(83492791) as u32 ^ (r2 + 7).wrapping_mul(19349663) as u32);
                let mut ri = |n: f64| (tr.next_f64() * n) as i32;
                let open = |rr: i32, cc: i32| (0..ROWS).contains(&rr) && (0..COLS).contains(&cc) && !solid[rr as usize][cc as usize];
                if open(r2 + 1, col) {
                    for _ in 0..3 {
                        let (bx, bw, bh) = (1 + ri(12.0), 2 + ri(2.0), 1 + ri(2.0));
                        c.rect(th.wall, px + bx, py + TILE, bw, bh);
                    }
                }
                if open(r2 - 1, col) {
                    for _ in 0..3 {
                        let (bx, bw) = (1 + ri(12.0), 2 + ri(2.0));
                        let hh = 1 + ri(2.0);
                        c.rect(th.wall, px + bx, py - hh, bw, hh);
                    }
                }
                if open(r2, col + 1) {
                    for _ in 0..3 {
                        let (by, bw, bh) = (1 + ri(12.0), 1 + ri(2.0), 2 + ri(2.0));
                        c.rect(th.wall, px + TILE, py + by, bw, bh);
                    }
                }
                if open(r2, col - 1) {
                    for _ in 0..3 {
                        let by = 1 + ri(12.0);
                        let ww = 1 + ri(2.0);
                        c.rect(th.wall, px - ww, py + by, ww, 2 + ri(2.0));
                    }
                }
            }
        }
        // Still pools toward the corners, clear of the door lanes (walkable shallows).
        let mut pr = Mulberry32::new(rk ^ 0x9e3779b9);
        let pool = th.pool.unwrap_or([0x1c3448, 0x2c5068]);
        let n_p = if pr.next_f64() < 0.6 { 1 + i32::from(pr.next_f64() < 0.3) } else { 0 };
        for _ in 0..n_p {
            let pc = if pr.next_f64() < 0.5 { 3 + (pr.next_f64() * 3.0) as i32 } else { 12 + (pr.next_f64() * 3.0) as i32 };
            let prow = if pr.next_f64() < 0.5 { 2 + (pr.next_f64() * 2.0) as i32 } else { 8 + (pr.next_f64() * 2.0) as i32 };
            let s = |rr: i32, cc: i32| solid.get(rr as usize).is_some_and(|row| row.get(cc as usize).copied().unwrap_or(true));
            if s(prow, pc) || s(prow, pc + 1) || s(prow + 1, pc) || s(prow + 1, pc + 1) {
                continue;
            }
            let (wx, wy) = (pc * TILE + 2, prow * TILE + 3);
            let ww = TILE + 10 + (pr.next_f64() * 8.0) as i32;
            let wh = TILE + 2 + (pr.next_f64() * 6.0) as i32;
            c.rect(th.grout, wx - 1, wy - 1, ww + 2, wh + 2);
            c.rect(pool[0], wx, wy, ww, wh);
            c.rect(pool[1], wx + 3, wy + 2, ww - 9, 1);
            c.rect(pool[1], wx + 5, wy + wh - 4, ww - 12, 1);
            c.rect(pool[1], wx + ww / 2 - 2, wy + wh / 2, 4, 1);
        }
    }
    // Locked doors over their (shut) gaps.
    if let Some(boss) = locks[0] {
        lock_door(&mut c, (MIDC - 1) * TILE, 0, true, boss);
    }
    if let Some(boss) = locks[1] {
        lock_door(&mut c, (MIDC - 1) * TILE, PX_H - TILE, true, boss);
    }
    if let Some(boss) = locks[2] {
        lock_door(&mut c, 0, (MIDR - 1) * TILE, false, boss);
    }
    if let Some(boss) = locks[3] {
        lock_door(&mut c, PX_W - TILE, (MIDR - 1) * TILE, false, boss);
    }
    // PITS: holes in the floor; adjacent pits merge into one void (shared edges open).
    let is_pit = |c2: i32, r2: i32| room.pits.contains(&(c2, r2));
    for &(pc, prr) in &room.pits {
        let (px, py) = (pc * TILE, prr * TILE);
        let (up, dn, lf, rt) = (is_pit(pc, prr - 1), is_pit(pc, prr + 1), is_pit(pc - 1, prr), is_pit(pc + 1, prr));
        c.rect(th.grout, px, py, TILE, TILE);
        let vx = px + if lf { 0 } else { 1 };
        let vy = py + if up { 0 } else { 2 };
        let vw = TILE - i32::from(!lf) - i32::from(!rt);
        let vh = TILE - if up { 0 } else { 2 } - i32::from(!dn);
        c.rect(0x050507, vx, vy, vw, vh);
        if !up {
            c.rect(th.wall_top, px + if lf { 0 } else { 1 }, py + 1, TILE - i32::from(!lf) - i32::from(!rt), 1);
        }
        if !dn {
            c.blend(0x000000, 128, px + 2, py + TILE - 3, TILE - 4, 2);
        }
    }
    // Decor — destructible furniture is a LIVE smashable entity now, not baked.
    for d in &room.decor {
        if prop(d.kind).destructible {
            continue;
        }
        let mut pr = Mulberry32::new(
            (d.c.wrapping_mul(131).wrapping_add(d.r.wrapping_mul(17)) as u32)
                .wrapping_add((d.kind.as_bytes()[0] as u32).wrapping_mul(2654435761)),
        );
        paint_prop(&mut c, d.kind, d.c * TILE, d.r * TILE, th, &mut pr, d.corner);
    }
    for &(fx, fy, wc, wr) in &TORCH_SPOTS {
        if solid[wr as usize][wc as usize] {
            draw_torch(&mut c, fx, fy);
        }
    }
    if entrance {
        draw_entrance(&mut c, th);
    }
    if room.stairs_down.is_some() {
        draw_stairs(&mut c, th, 4 * TILE, 3 * TILE, true);
    }
    if room.stairs_up.is_some() {
        draw_stairs(&mut c, th, 4 * TILE, 3 * TILE, false);
    }
    buf
}
