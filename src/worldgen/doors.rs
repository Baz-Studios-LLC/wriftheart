//! doors.rs — edge openings between rooms (port of doorSpan/inSpan/spanCenter, js/world.js).
//!
//! Both sides of an edge hash the same (room, salt) pair, so neighbours always agree on where
//! the gaps are. The bit-slicing of `hv` below must stay exactly as the JS has it.

/// One opening in an edge wall: `start` is the offset into the lane (the edge minus its two
/// corners); tiles `[1 + start, 1 + start + width)` are open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: i32,
    pub width: i32,
}

/// The openings for one edge, or `None` for a solid wall — port of `doorSpan(hv, lane)`.
pub fn door_span(hv: u32, lane: i32) -> Option<Vec<Span>> {
    if hv % 100 < 10 {
        return None; // ~10% of edges are a solid wall
    }
    let t = (hv >> 7) % 100;
    let mut width: i32 = if t < 10 {
        1 + ((hv >> 3) % 2) as i32 // 10%: a narrow pinch (1-2)
    } else if t < 30 {
        3 + ((hv >> 5) % 3) as i32 // 20%: a doorway (3-5)
    } else if t < 55 {
        6 + ((hv >> 9) % (lane - 8).max(1) as u32) as i32 // 25%: a wide gap
    } else {
        lane // 45%: the whole edge is open (no wall)
    };
    // JS: `width = clamp(lane, 1, width)` — note the odd arg order: it clamps LANE into
    // [1, width], i.e. width = min(lane, width) for any lane >= 1. Keep it verbatim.
    width = lane.clamp(1, width.max(1));
    let start = if width >= lane { 0 } else { ((hv >> 13) % (lane - width + 1) as u32) as i32 };
    let mut spans = vec![Span { start, width }];
    // For tighter openings, sometimes punch a second gap so the edge branches into two.
    if width <= 5 && ((hv >> 17) % 100) < 38 {
        let w2 = 1 + ((hv >> 19) % 2) as i32; // a small secondary gap (1-2)
        let after = start + width + 2;
        let before = start - 2 - w2;
        let mut s2 = -1;
        if after + w2 <= lane && ((hv >> 23) & 1) == 1 {
            s2 = after;
        } else if before >= 0 {
            s2 = before;
        } else if after + w2 <= lane {
            s2 = after;
        }
        if s2 >= 0 {
            spans.push(Span { start: s2, width: w2 });
        }
    }
    Some(spans)
}

/// Is edge index `i` (a full-edge row/col, corners included) inside any opening?
#[allow(clippy::int_plus_one)] // `1 + s.start` mirrors the JS lane offset — keep the shape
pub fn in_span(spans: Option<&[Span]>, i: i32) -> bool {
    if let Some(spans) = spans {
        for s in spans {
            if i >= 1 + s.start && i < 1 + s.start + s.width {
                return true;
            }
        }
    }
    false
}

/// The centre tile index of an opening (the connectivity carve target).
pub fn span_center(s: &Span) -> i32 {
    1 + s.start + (s.width >> 1)
}
