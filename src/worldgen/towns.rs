//! towns.rs — town placement, footprints, and district roles (port of js/world.js towns).
//!
//! Pure functions of (seed, room) — no shard/rift state — so they live outside `World`.
//! Towns are 3x3 FOOTPRINTS around a site: the site hash rolls hamlet/town/city, which sides
//! join, and each side's flavour. Every room resolves its own role purely, so a town looks the
//! same from every approach (seam-safe).

use super::rng::hash;

pub const TOWN_CELL: i32 = 8;
const TOWN_BAND_LO: i32 = 3;
const TOWN_BAND_W: u32 = 3;
const TOWN_MIN_GAP: i32 = 8;
const SALT_TOWN: u32 = 0x77ab;

/// A town site: its market-square room + the site hash that decides everything about it.
#[derive(Clone, Copy, Debug)]
pub struct TownSite {
    pub tx: i32,
    pub ty: i32,
    pub h: u32,
}

/// A district's role within a town footprint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownRole {
    Market,
    Homes,
    Green,
    Farmrow,
    Quarter,
    Yards,
    Hall,
}

const TOWN_DIRS: [char; 4] = ['n', 'e', 's', 'w'];
const TOWN_SIDE_ROLES: [TownRole; 4] =
    [TownRole::Homes, TownRole::Green, TownRole::Farmrow, TownRole::Quarter];

/// The town site for a room's 8x8 block, or None — port of `townSiteOf`.
pub fn town_site_of(seed: u32, rx: i32, ry: i32) -> Option<TownSite> {
    let cx = rx.div_euclid(TOWN_CELL);
    let cy = ry.div_euclid(TOWN_CELL);
    if cx == 0 && cy == 0 {
        return None; // the spawn block has no procedural town
    }
    let h = hash(seed, cx, cy, SALT_TOWN);
    let tx = cx * TOWN_CELL + TOWN_BAND_LO + (h % TOWN_BAND_W) as i32;
    let ty = cy * TOWN_CELL + TOWN_BAND_LO + ((h >> 8) % TOWN_BAND_W) as i32;
    if (tx - 1).abs().max(ty.abs()) < TOWN_MIN_GAP {
        return None; // keep clear of the home village at (1,0)
    }
    Some(TownSite { tx, ty, h })
}

/// The nearest town's centre room to (rx,ry) — scans surrounding blocks (js
/// nearestTown; a subtle in-world compass: startled birds fly toward it).
pub fn nearest_town(seed: u32, rx: i32, ry: i32) -> Option<(i32, i32)> {
    let cx = rx.div_euclid(TOWN_CELL);
    let cy = ry.div_euclid(TOWN_CELL);
    let mut best: Option<(i32, i32)> = None;
    let mut bd = i64::MAX;
    for dcy in -2..=2 {
        for dcx in -2..=2 {
            let Some(s) = town_site_of(seed, (cx + dcx) * TOWN_CELL + 3, (cy + dcy) * TOWN_CELL + 3) else { continue };
            if !is_town(seed, s.tx, s.ty) {
                continue;
            }
            let dd = ((s.tx - rx) as i64).pow(2) + ((s.ty - ry) as i64).pow(2);
            if dd < bd {
                bd = dd;
                best = Some((s.tx, s.ty));
            }
        }
    }
    best
}

/// This room's role in its town footprint, or None — port of `townRole`.
pub fn town_role(seed: u32, rx: i32, ry: i32) -> Option<TownRole> {
    let s = town_site_of(seed, rx, ry)?;
    let dx = rx - s.tx;
    let dy = ry - s.ty;
    if dx.abs() > 1 || dy.abs() > 1 {
        return None;
    }
    if dx == 0 && dy == 0 {
        return Some(TownRole::Market);
    }
    let sz = (s.h >> 16) % 10; // 0-2 hamlet, 3-8 town, 9 city
    if sz <= 2 {
        return None;
    }
    let city = sz == 9;
    if dx.abs() == 1 && dy.abs() == 1 {
        // Corners are a city luxury — and ONE of them keeps the old GUILDHALL.
        if !city {
            return None;
        }
        let hall_corner = (s.h >> 28) % 4;
        let ci = (if dx == 1 { 1 } else { 0 }) + (if dy == 1 { 2 } else { 0 });
        return Some(if ci == hall_corner { TownRole::Hall } else { TownRole::Yards });
    }
    let dir = if dy == -1 { 'n' } else if dx == 1 { 'e' } else if dy == 1 { 's' } else { 'w' };
    if !city {
        // A town keeps 2-4 sides, picked around the compass.
        let n = 2 + ((s.h >> 20) % 3);
        let rot = (s.h >> 24) % 4;
        let mut hit = false;
        for i in 0..n {
            if TOWN_DIRS[((rot + i) % 4) as usize] == dir {
                hit = true;
            }
        }
        if !hit {
            return None;
        }
    }
    let off = (s.h >> 26) % 4; // rotate the flavors so towns differ
    let di = TOWN_DIRS.iter().position(|&d| d == dir).unwrap() as u32;
    Some(TOWN_SIDE_ROLES[((di + off) % 4) as usize])
}

pub fn is_town(seed: u32, rx: i32, ry: i32) -> bool {
    town_role(seed, rx, ry).is_some()
}

/// A deterministic 4-connected room path A->B (horizontal leg, then vertical) — `roadPath`.
pub fn road_path(ax: i32, ay: i32, bx: i32, by: i32) -> Vec<(i32, i32)> {
    let mut p = vec![(ax, ay)];
    let (mut x, mut y) = (ax, ay);
    while x != bx {
        x += (bx - x).signum();
        p.push((x, y));
    }
    while y != by {
        y += (by - y).signum();
        p.push((x, y));
    }
    p
}
