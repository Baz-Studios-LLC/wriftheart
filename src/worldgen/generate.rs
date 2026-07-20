//! generate.rs — build one room's tile map (port of `generate()` in js/world.js).
//!
//! Every stage below runs in the SAME ORDER as the JS: water fill, edge walls + doors, corner
//! opening, road paving, hub Dijkstra, door connects, border repair, bridge-stub pruning, door
//! footing, town streets, castle/shard paths, stray-plank cleanup, road restyle, causeways.
//! Order is not stylistic — later stages read tiles earlier stages wrote, so any reorder
//! changes maps. The golden parity test (tests/worldmap_parity.rs) holds this to the JS.

// Lint policy: this file mirrors js/world.js statement-for-statement so it can be
// audited by side-by-side diff. Stylistic reshaping (collapsed ifs, range-contains)
// would break that mapping — allowed here, and ONLY here.
#![allow(clippy::collapsible_if, clippy::manual_range_contains, clippy::needless_range_loop, clippy::int_plus_one, clippy::ptr_arg, clippy::too_many_arguments, clippy::type_complexity)]

use super::doors::{in_span, span_center, Span};
use super::world::{World, COLS, ROWS};
use std::collections::HashSet;

const MID_C: i32 = COLS >> 1; // 9
const MID_R: i32 = ROWS >> 1; // 6

/// A generated room: 13 rows of 19 tile chars, plus the protected-tile set (tiles that
/// entity placement must never cover — carved paths, roads, door footings).
pub struct RoomMap {
    pub map: Vec<String>,
    pub prot: HashSet<usize>,
}

impl World {
    /// Port of `generate(rx, ry)`.
    pub fn generate(&self, rx: i32, ry: i32) -> RoomMap {
        let gx0 = rx * COLS;
        let gy0 = ry * ROWS;
        let wall = self.biome_at(rx, ry).wall;

        let village_room = rx == 1 && ry == 0;
        let castle_room = World::is_castle(rx, ry);
        let town = self.is_town(rx, ry) || village_room;

        let idx = |r: i32, c: i32| (r * COLS + c) as usize;
        let mut grid = vec![vec!['.'; COLS as usize]; ROWS as usize];

        // Water fill. The bottom row & right column sample the NEIGHBOUR room's first
        // row/col so both sides of a seam agree on water-vs-land.
        for r in 0..ROWS {
            for c in 0..COLS {
                let sgx = gx0 + if c == COLS - 1 { COLS } else { c };
                let sgy = gy0 + if r == ROWS - 1 { ROWS } else { r };
                grid[r as usize][c as usize] = if self.tile_water(sgx, sgy) { '~' } else { '.' };
            }
        }

        let super::edges::EdgeSpans { left, right, top, bottom } = self.resolve_edges(rx, ry);

        // Stand the edge walls, leaving the door gaps open.
        for r in 0..ROWS {
            if !in_span(left.as_deref(), r) {
                grid[r as usize][0] = wall;
            }
            if !in_span(right.as_deref(), r) {
                grid[r as usize][(COLS - 1) as usize] = wall;
            }
        }
        for c in 0..COLS {
            if !in_span(top.as_deref(), c) {
                grid[0][c as usize] = wall;
            }
            if !in_span(bottom.as_deref(), c) {
                grid[(ROWS - 1) as usize][c as usize] = wall;
            }
        }
        // Open a corner when BOTH adjoining edges have a gap reaching it.
        {
            let mut open_corner = |r: i32, c: i32, open: bool| {
                if !open {
                    return;
                }
                let sgx = gx0 + if c == COLS - 1 { COLS } else { c };
                let sgy = gy0 + if r == ROWS - 1 { ROWS } else { r };
                grid[r as usize][c as usize] = if self.tile_water(sgx, sgy) { '~' } else { '.' };
            };
            open_corner(0, 0, in_span(top.as_deref(), 1) && in_span(left.as_deref(), 1));
            open_corner(0, COLS - 1, in_span(top.as_deref(), COLS - 2) && in_span(right.as_deref(), 1));
            open_corner(ROWS - 1, 0, in_span(bottom.as_deref(), 1) && in_span(left.as_deref(), ROWS - 2));
            open_corner(
                ROWS - 1,
                COLS - 1,
                in_span(bottom.as_deref(), COLS - 2) && in_span(right.as_deref(), ROWS - 2),
            );
        }

        let mut prot: HashSet<usize> = HashSet::new();
        let is_walk = |ch: char| ch == '.' || ch == 'B';

        // Pave the road/gate corridors as protected ground (restyled to `=` at the very end).
        // (road_edges is pure — recomputed here rather than threaded out of resolve_edges.)
        let r_edges = self.road_edges(rx, ry); // [N, S, E, W]
        let g_edges = self.gate_edges(rx, ry);
        let p_edges = [
            r_edges[0] || g_edges[0], // N
            r_edges[1] || g_edges[1], // S
            r_edges[2] || g_edges[2], // E
            r_edges[3] || g_edges[3], // W
        ];
        let mut road_strip: Vec<usize> = Vec::new();
        if p_edges.iter().any(|&e| e) {
            let mut pave = |c: i32, r: i32, grid: &mut Vec<Vec<char>>, prot: &mut HashSet<usize>| {
                if c < 0 || c >= COLS || r < 0 || r >= ROWS || grid[r as usize][c as usize] == wall {
                    return;
                }
                road_strip.push(idx(r, c));
                grid[r as usize][c as usize] =
                    if grid[r as usize][c as usize] == '~' { 'B' } else { '.' };
                prot.insert(idx(r, c));
            };
            pave(MID_C, MID_R, &mut grid, &mut prot); // centre junction
            if p_edges[3] {
                for c in 0..MID_C {
                    pave(c, MID_R, &mut grid, &mut prot);
                }
            }
            if p_edges[2] {
                for c in MID_C..COLS {
                    pave(c, MID_R, &mut grid, &mut prot);
                }
            }
            if p_edges[0] {
                for r in 0..MID_R {
                    pave(MID_C, r, &mut grid, &mut prot);
                }
            }
            if p_edges[1] {
                for r in MID_R..ROWS {
                    pave(MID_C, r, &mut grid, &mut prot);
                }
            }
        }

        // Hub: the land tile nearest the room centre (or the centre itself, grounded).
        let mut hub_r = MID_R;
        let mut hub_c = MID_C;
        if grid[hub_r as usize][hub_c as usize] != '.' {
            let mut bd = i32::MAX;
            for r in 1..ROWS - 1 {
                for c in 1..COLS - 1 {
                    if grid[r as usize][c as usize] == '.' {
                        let d = (r - MID_R).abs() + (c - MID_C).abs();
                        if d < bd {
                            bd = d;
                            hub_r = r;
                            hub_c = c;
                        }
                    }
                }
            }
        }
        let must_link = self.shard_dungeon_at(rx, ry).is_some();
        if grid[hub_r as usize][hub_c as usize] == '.' || must_link {
            let (r, c) = (hub_r, hub_c);
            grid[r as usize][c as usize] =
                if grid[r as usize][c as usize] == '~' { 'B' } else { '.' };
            prot.insert(idx(r, c));
        }

        // Dijkstra from the hub: land/bridge cost 1, water 40, walls impassable — exactly the
        // JS's O(n^2) scan-min version, so tie-breaks (and therefore `prev` chains) match.
        const WATER_COST: i64 = 40;
        let nt = (ROWS * COLS) as usize;
        let mut dist = vec![i64::MAX; nt];
        let mut prev = vec![-1i32; nt];
        let mut settled = vec![false; nt];
        let enter = |grid: &Vec<Vec<char>>, r: i32, c: i32| -> Option<i64> {
            match grid[r as usize][c as usize] {
                '.' | 'B' => Some(1),
                '~' => Some(WATER_COST),
                _ => None,
            }
        };
        dist[idx(hub_r, hub_c)] = 0;
        for _ in 0..nt {
            let mut u: i64 = -1;
            let mut bv = i64::MAX;
            for i in 0..nt {
                if !settled[i] && dist[i] < bv {
                    bv = dist[i];
                    u = i as i64;
                }
            }
            if u < 0 {
                break;
            }
            let u = u as usize;
            settled[u] = true;
            let ur = u as i32 / COLS;
            let uc = u as i32 % COLS;
            for (nr, nc) in [(ur + 1, uc), (ur - 1, uc), (ur, uc + 1), (ur, uc - 1)] {
                if nr < 0 || nr >= ROWS || nc < 0 || nc >= COLS {
                    continue;
                }
                let Some(ec) = enter(&grid, nr, nc) else { continue };
                let ni = idx(nr, nc);
                let nd = dist[u] + ec;
                if nd < dist[ni] {
                    dist[ni] = nd;
                    prev[ni] = u as i32;
                }
            }
        }

        // Carve the cheapest path from a door back to the hub, bridging only short crossings
        // (longest open-water run <= MAX_BRIDGE) unless forced — port of `connect`.
        const MAX_BRIDGE: i32 = 3;
        let connect = |r: i32, c: i32, force: bool, grid: &mut Vec<Vec<char>>, prot: &mut HashSet<usize>| {
            let mut i = idx(r, c) as i32;
            if dist[i as usize] == i64::MAX {
                return;
            }
            if !force {
                if grid[r as usize][c as usize] == '~' {
                    return; // water door: the seam stays water on both sides
                }
                let mut run = 0;
                let mut worst = 0;
                let mut j = i;
                while j != -1 {
                    let jr = (j / COLS) as usize;
                    let jc = (j % COLS) as usize;
                    run = if grid[jr][jc] == '~' { run + 1 } else { 0 };
                    if run > worst {
                        worst = run;
                    }
                    j = prev[j as usize];
                }
                if worst > MAX_BRIDGE {
                    return;
                }
            }
            while i != -1 {
                let rr = (i / COLS) as usize;
                let cc = (i % COLS) as usize;
                grid[rr][cc] = if grid[rr][cc] == '~' { 'B' } else { '.' };
                prot.insert(i as usize);
                i = prev[i as usize];
            }
        };
        let door_force = |nx: i32, ny: i32| must_link || self.shard_dungeon_at(nx, ny).is_some();
        // One table drives all four edges (JS order: left, right, top, bottom). Each row:
        // the edge's spans, the neighbour room it crosses to, and how a span maps to (r, c).
        type SpanPos = fn(&Span) -> (i32, i32);
        let connect_edges: [(&Option<Vec<Span>>, (i32, i32), SpanPos); 4] = [
            (&left, (rx - 1, ry), |s| (span_center(s), 0)),
            (&right, (rx + 1, ry), |s| (span_center(s), COLS - 1)),
            (&top, (rx, ry - 1), |s| (0, span_center(s))),
            (&bottom, (rx, ry + 1), |s| (ROWS - 1, span_center(s))),
        ];
        for (spans, (nx, ny), pos) in connect_edges {
            if let Some(spans) = spans {
                for s in spans {
                    let (r, c) = pos(s);
                    connect(r, c, door_force(nx, ny), &mut grid, &mut prot);
                }
            }
        }

        // Repair: flood from the hub, then link any walkable border tile still stranded.
        let mut reach: HashSet<usize> = HashSet::new();
        let mut st = vec![(hub_r, hub_c)];
        while let Some((r, c)) = st.pop() {
            if r < 0 || r >= ROWS || c < 0 || c >= COLS {
                continue;
            }
            let k = idx(r, c);
            if reach.contains(&k) || !is_walk(grid[r as usize][c as usize]) {
                continue;
            }
            reach.insert(k);
            st.push((r + 1, c));
            st.push((r - 1, c));
            st.push((r, c + 1));
            st.push((r, c - 1));
        }
        {
            let repair = |r: i32, c: i32, grid: &mut Vec<Vec<char>>, prot: &mut HashSet<usize>| {
                if is_walk(grid[r as usize][c as usize]) && !reach.contains(&idx(r, c)) {
                    connect(r, c, must_link, grid, prot);
                }
            };
            for r in 0..ROWS {
                repair(r, 0, &mut grid, &mut prot);
                repair(r, COLS - 1, &mut grid, &mut prot);
            }
            for c in 0..COLS {
                repair(0, c, &mut grid, &mut prot);
                repair(ROWS - 1, c, &mut grid, &mut prot);
            }
        }

        // Prune dead-end bridge spurs (iterate so multi-tile stubs unravel from the tip).
        let mut pruned = true;
        while pruned {
            pruned = false;
            for r in 1..ROWS - 1 {
                for c in 1..COLS - 1 {
                    if grid[r as usize][c as usize] != 'B' {
                        continue;
                    }
                    let mut n = 0;
                    if is_walk(grid[(r - 1) as usize][c as usize]) {
                        n += 1;
                    }
                    if is_walk(grid[(r + 1) as usize][c as usize]) {
                        n += 1;
                    }
                    if is_walk(grid[r as usize][(c - 1) as usize]) {
                        n += 1;
                    }
                    if is_walk(grid[r as usize][(c + 1) as usize]) {
                        n += 1;
                    }
                    if n <= 1 {
                        grid[r as usize][c as usize] = '~'; // stub -> back to water
                        prot.remove(&idx(r, c));
                        pruned = true;
                    }
                }
            }
        }

        // Door FOOTING: behind every land border opening, turn the OPEN_DEPTH water tiles
        // directly inside into land (never the border tile itself) and protect the strip.
        const OPEN_DEPTH: i32 = 2;
        {
            let footing = |br: i32, bc: i32, dr: i32, dc: i32, grid: &mut Vec<Vec<char>>, prot: &mut HashSet<usize>| {
                if br < 0 || br >= ROWS || bc < 0 || bc >= COLS {
                    return;
                }
                let b = grid[br as usize][bc as usize];
                if b == wall || b == '~' {
                    return; // walled/water border: not a standable crossing
                }
                for d in 0..OPEN_DEPTH {
                    let r = br + dr * d;
                    let c = bc + dc * d;
                    if r < 0 || r >= ROWS || c < 0 || c >= COLS || grid[r as usize][c as usize] == wall {
                        break;
                    }
                    if d > 0 && grid[r as usize][c as usize] == '~' {
                        grid[r as usize][c as usize] = '.';
                    }
                    if grid[r as usize][c as usize] == '.' || grid[r as usize][c as usize] == 'B' {
                        prot.insert(idx(r, c));
                    }
                }
            };
            // Table-driven like the connects — but note the JS runs footings in a DIFFERENT
            // edge order (top, bottom, left, right), and order is observable (footings mutate
            // the grid). Each row: spans, then span-tile -> (border r, border c, inward dr/dc).
            type FootPos = fn(i32) -> (i32, i32, i32, i32);
            let footing_edges: [(&Option<Vec<Span>>, FootPos); 4] = [
                (&top, |t| (0, t, 1, 0)),
                (&bottom, |t| (ROWS - 1, t, -1, 0)),
                (&left, |t| (t, 0, 0, 1)),
                (&right, |t| (t, COLS - 1, 0, -1)),
            ];
            for (spans, pos) in footing_edges {
                if let Some(spans) = spans {
                    for s in spans {
                        for i in 0..s.width {
                            let (br, bc, dr, dc) = pos(1 + s.start + i);
                            footing(br, bc, dr, dc, &mut grid, &mut prot);
                        }
                    }
                }
            }
        }

        // Town streets, shaped by the district (the home village keeps the classic cross).
        if town {
            let role = self.town_role(rx, ry); // None for the village -> 'market' behaviour
            let is_market_or_quarter = matches!(
                role,
                None | Some(super::towns::TownRole::Market) | Some(super::towns::TownRole::Quarter)
            );
            let is_homes = matches!(role, Some(super::towns::TownRole::Homes));
            // cross(): main street + cross street
            for c in 1..COLS - 1 {
                if grid[MID_R as usize][c as usize] == '.' {
                    grid[MID_R as usize][c as usize] = '_';
                }
            }
            for r in 1..ROWS - 1 {
                if grid[r as usize][MID_C as usize] == '.' {
                    grid[r as usize][MID_C as usize] = '_';
                }
            }
            if is_market_or_quarter {
                // connectors(): shopping streets
                for bc in [3, 7, 11, 15] {
                    for r in 3..=9 {
                        if grid[r as usize][bc as usize] == '.' {
                            grid[r as usize][bc as usize] = '_';
                        }
                    }
                }
            } else if is_homes {
                // lanes(): residential lanes along the house rows
                for lr in [3, 9] {
                    for c in 2..COLS - 2 {
                        if grid[lr as usize][c as usize] == '.' {
                            grid[lr as usize][c as usize] = '_';
                        }
                    }
                }
            }
            // Streets run THROUGH the gates.
            if grid[MID_R as usize][0] == '.' {
                grid[MID_R as usize][0] = '_';
            }
            if grid[MID_R as usize][(COLS - 1) as usize] == '.' {
                grid[MID_R as usize][(COLS - 1) as usize] = '_';
            }
            if grid[0][MID_C as usize] == '.' {
                grid[0][MID_C as usize] = '_';
            }
            if grid[(ROWS - 1) as usize][MID_C as usize] == '.' {
                grid[(ROWS - 1) as usize][MID_C as usize] = '_';
            }
        }
        if castle_room {
            // A worn flagstone path runs from the bottom edge up to the gate.
            for r in 7..ROWS {
                for c in [MID_C - 1, MID_C] {
                    if grid[r as usize][c as usize] == '.' {
                        grid[r as usize][c as usize] = 'p';
                    }
                }
            }
        }
        if self.shard_dungeon_at(rx, ry).is_some() {
            // A worn processional way runs from the bottom edge up to the monument's mouth.
            for r in 5..ROWS {
                for c in [MID_C - 1, MID_C, MID_C + 1] {
                    if grid[r as usize][c as usize] == '.' {
                        grid[r as usize][c as usize] = 'p';
                    }
                }
            }
        }

        // A bridge tile only makes sense spanning water — strays on dry land become ground.
        for r in 0..ROWS {
            for c in 0..COLS {
                if grid[r as usize][c as usize] != 'B' {
                    continue;
                }
                let water_at = |rr: i32, cc: i32| {
                    rr >= 0 && rr < ROWS && cc >= 0 && cc < COLS && grid[rr as usize][cc as usize] == '~'
                };
                if !(water_at(r - 1, c) || water_at(r + 1, c) || water_at(r, c - 1) || water_at(r, c + 1)) {
                    grid[r as usize][c as usize] = '.';
                }
            }
        }
        // Dress the paved road as the dirt-road tile `=` (real crossings keep their deck).
        for k in &road_strip {
            let r = *k / COLS as usize;
            let c = *k % COLS as usize;
            if grid[r][c] == '.' {
                grid[r][c] = '=';
            }
        }
        // A road/street with open water straddling it is a CAUSEWAY — deck it as a bridge.
        for r in 0..ROWS {
            for c in 0..COLS {
                let ch = grid[r as usize][c as usize];
                if ch != '=' && ch != '_' {
                    continue;
                }
                let water_at = |rr: i32, cc: i32| {
                    rr >= 0 && rr < ROWS && cc >= 0 && cc < COLS && grid[rr as usize][cc as usize] == '~'
                };
                let lr = water_at(r, c - 1) && water_at(r, c + 1);
                let ud = water_at(r - 1, c) && water_at(r + 1, c);
                if lr || ud {
                    grid[r as usize][c as usize] = 'B';
                }
            }
        }

        RoomMap { map: grid.into_iter().map(|row| row.into_iter().collect()).collect(), prot }
    }
}
