//! edges.rs — resolve one room's four edge openings (door spans + every override).
//!
//! Extracted from `generate()` so it stays reusable on its own: room transitions, the codex
//! map, and entity placement all eventually need "where are this room's openings?" without
//! building the whole tile map. Order of overrides (forced seams -> town edges -> road gate
//! unions) is observable and mirrors the JS exactly.

// Lint policy: this file mirrors js/world.js statement-for-statement so it can be
// audited by side-by-side diff. Stylistic reshaping (collapsed ifs, range-contains)
// would break that mapping — allowed here, and ONLY here.
#![allow(clippy::collapsible_if, clippy::manual_range_contains, clippy::needless_range_loop, clippy::int_plus_one, clippy::ptr_arg, clippy::too_many_arguments, clippy::type_complexity)]

use super::doors::{door_span, Span};
use super::rng::hash;
use super::world::{World, COLS, ROWS, SALT_H, SALT_V};

const MID_C: i32 = COLS >> 1; // 9
const MID_R: i32 = ROWS >> 1; // 6

/// A room's four resolved edges; `None` = a solid wall.
pub struct EdgeSpans {
    pub left: Option<Vec<Span>>,
    pub right: Option<Vec<Span>>,
    pub top: Option<Vec<Span>>,
    pub bottom: Option<Vec<Span>>,
}

impl World {
    /// The fully-resolved openings for room (rx, ry) — the JS door-span section of `generate()`.
    pub fn resolve_edges(&self, rx: i32, ry: i32) -> EdgeSpans {
        // Door spans per edge, with forced-open seams (spawn<->village, all castle edges),
        // town open/gate edges, and road gates unioned in.
        let open_v = vec![Span { start: 0, width: ROWS - 2 }];
        let open_h = vec![Span { start: 0, width: COLS - 2 }];
        let home_seam =
            |ax: i32, ay: i32, bx: i32, by: i32| ax == 0 && ay == 0 && bx == 1 && by == 0;
        let force_v = |ax: i32, ay: i32, bx: i32, by: i32| {
            home_seam(ax, ay, bx, by) || World::is_castle(ax, ay) || World::is_castle(bx, by)
        };
        let force_h =
            |ax: i32, ay: i32, bx: i32, by: i32| World::is_castle(ax, ay) || World::is_castle(bx, by);

        let mut left = if force_v(rx - 1, ry, rx, ry) {
            Some(open_v.clone())
        } else {
            door_span(hash(self.seed, rx - 1, ry, SALT_V), ROWS - 2)
        };
        let mut right = if force_v(rx, ry, rx + 1, ry) {
            Some(open_v.clone())
        } else {
            door_span(hash(self.seed, rx, ry, SALT_V), ROWS - 2)
        };
        let mut top = if force_h(rx, ry - 1, rx, ry) {
            Some(open_h.clone())
        } else {
            door_span(hash(self.seed, rx, ry - 1, SALT_H), COLS - 2)
        };
        let mut bottom = if force_h(rx, ry, rx, ry + 1) {
            Some(open_h.clone())
        } else {
            door_span(hash(self.seed, rx, ry, SALT_H), COLS - 2)
        };

        let town_gate_v = vec![Span { start: MID_R - 2, width: 3 }];
        let town_gate_h = vec![Span { start: MID_C - 2, width: 3 }];
        // 'open' if both sides are town, 'gate' if exactly one — port of `townEdge`.
        let town_edge = |ax: i32, ay: i32, bx: i32, by: i32| -> Option<bool> {
            let ta = self.is_town(ax, ay);
            let tb = self.is_town(bx, by);
            if ta && tb {
                Some(true) // open
            } else if ta || tb {
                Some(false) // gate
            } else {
                None
            }
        };
        let te_l = town_edge(rx - 1, ry, rx, ry);
        let te_r = town_edge(rx, ry, rx + 1, ry);
        let te_t = town_edge(rx, ry - 1, rx, ry);
        let te_b = town_edge(rx, ry, rx, ry + 1);
        if let Some(open) = te_l {
            if !force_v(rx - 1, ry, rx, ry) {
                left = Some(if open { open_v.clone() } else { town_gate_v.clone() });
            }
        }
        if let Some(open) = te_r {
            if !force_v(rx, ry, rx + 1, ry) {
                right = Some(if open { open_v.clone() } else { town_gate_v.clone() });
            }
        }
        if let Some(open) = te_t {
            if !force_h(rx, ry - 1, rx, ry) {
                top = Some(if open { open_h.clone() } else { town_gate_h.clone() });
            }
        }
        if let Some(open) = te_b {
            if !force_h(rx, ry, rx, ry + 1) {
                bottom = Some(if open { open_h.clone() } else { town_gate_h.clone() });
            }
        }
        // Road edges force a centred gate, unioned with the natural door (never narrowing it).
        let r_edges = self.road_edges(rx, ry); // [N, S, E, W]
        let union = |spans: &mut Option<Vec<Span>>, gate: &[Span]| {
            let mut v = spans.take().unwrap_or_default();
            v.extend_from_slice(gate);
            *spans = Some(v);
        };
        if r_edges[3] && te_l.is_none() && !force_v(rx - 1, ry, rx, ry) {
            union(&mut left, &town_gate_v);
        }
        if r_edges[2] && te_r.is_none() && !force_v(rx, ry, rx + 1, ry) {
            union(&mut right, &town_gate_v);
        }
        if r_edges[0] && te_t.is_none() && !force_h(rx, ry - 1, rx, ry) {
            union(&mut top, &town_gate_h);
        }
        if r_edges[1] && te_b.is_none() && !force_h(rx, ry, rx, ry + 1) {
            union(&mut bottom, &town_gate_h);
        }

        EdgeSpans { left, right, top, bottom }
    }
}
