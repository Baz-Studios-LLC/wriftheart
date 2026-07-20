//! prop_paint.rs — the decor prop painters (js/dungeon.js PROP draw closures), each a
//! fill-rect display list over the room buffer. `x,y` = the prop's tile top-left in room
//! px; tall props draw UP out of their tile; `R` is the js placement rng (mulberry32).
//!
//! Transcribed verbatim; the only licence taken: canvas arc strokes (magiccircle) plot
//! parametric pixels instead of anti-aliased arcs — at 16px it reads identically.

use super::themes::Theme;
use crate::worldgen::rng::Mulberry32;

/// A bare-bones canvas over an RGBA room buffer.
pub struct Px<'a> {
    pub buf: &'a mut [u8],
    pub w: i32,
    pub h: i32,
}

impl Px<'_> {
    pub fn rect(&mut self, col: u32, x: i32, y: i32, w: i32, h: i32) {
        self.blend(col, 255, x, y, w, h);
    }
    /// Alpha fill (the js rgba() details) — a in 0..=255.
    pub fn blend(&mut self, col: u32, a: u8, x: i32, y: i32, w: i32, h: i32) {
        let (cr, cg, cb) = ((col >> 16) as u8, (col >> 8) as u8, col as u8);
        for yy in y.max(0)..(y + h).min(self.h) {
            for xx in x.max(0)..(x + w).min(self.w) {
                let i = ((yy * self.w + xx) * 4) as usize;
                if a == 255 {
                    self.buf[i..i + 4].copy_from_slice(&[cr, cg, cb, 255]);
                } else {
                    let k = a as u32;
                    for (c, s) in [(0, cr), (1, cg), (2, cb)] {
                        self.buf[i + c] = ((self.buf[i + c] as u32 * (255 - k) + s as u32 * k) / 255) as u8;
                    }
                    self.buf[i + 3] = 255;
                }
            }
        }
    }
}

#[allow(clippy::approx_constant)] // the js hardcodes 6.28, and parity is the point
const JS_TAU: f64 = 6.28;

const BOOK_COLS: [u32; 7] = [0x9a3030, 0x2a6a8a, 0x3a7a3a, 0xb08020, 0x6a3a8a, 0x7c4c1c, 0x8a8a4a];

/// Paint one prop (js drawProp / render's decor pass). `corner` = a corner cobweb's tuck.
#[allow(clippy::too_many_lines)] // one match arm per js PROP entry — a table, not logic
fn ri(r: &mut Mulberry32, n: f64) -> i32 {
    (r.next_f64() * n) as i32
}

pub fn paint_prop(c: &mut Px, kind: &str, x: i32, y: i32, th: &Theme, r: &mut Mulberry32, corner: Option<&str>) {
    match kind {
        "bookshelf" => {
            c.rect(0x3a2614, x, y - 12, 32, 28);
            c.rect(0x5a3a1c, x, y - 12, 32, 2);
            c.rect(0x241408, x, y + 14, 32, 2);
            for s in 0..4 {
                let sy = -9 + s * 6;
                c.rect(0x241408, x + 1, y + sy + 5, 30, 1);
                let mut bx = 2;
                while bx < 30 {
                    let bw = 2 + ri(r, 2.0);
                    let bh = 4 + ri(r, 2.0);
                    let col = BOOK_COLS[ri(r, BOOK_COLS.len() as f64) as usize];
                    c.rect(col, x + bx, y + sy + 5 - bh, bw, bh);
                    bx += bw + 1;
                }
            }
        }
        "table" => {
            c.rect(0x6b4a2a, x, y + 4, 32, 7);
            c.rect(0x86603a, x, y + 4, 32, 2);
            c.rect(0x4a3018, x + 3, y + 11, 3, 5);
            c.rect(0x4a3018, x + 26, y + 11, 3, 5);
            if r.next_f64() < 0.6 {
                c.rect(0xe8d8a0, x + 22, y - 1, 3, 6);
                c.rect(0xfcae40, x + 23, y - 3, 1, 3);
            }
            if r.next_f64() < 0.5 {
                c.rect(BOOK_COLS[ri(r, BOOK_COLS.len() as f64) as usize], x + 6, y + 1, 6, 4);
            } else {
                c.rect(0xcaa84a, x + 8, y, 4, 5);
                c.rect(0xe0c060, x + 8, y, 4, 1);
            }
        }
        "barrel" => {
            c.rect(0x7c4c1c, x + 2, y - 2, 12, 18);
            c.rect(0x9a6028, x + 2, y - 2, 12, 2);
            c.rect(0x4a3018, x + 2, y + 14, 12, 2);
            c.rect(0x3f2a14, x + 2, y + 2, 12, 2);
            c.rect(0x3f2a14, x + 2, y + 8, 12, 2);
            c.rect(0x5a3a1c, x + 5, y - 2, 1, 18);
            c.rect(0x5a3a1c, x + 9, y - 2, 1, 18);
        }
        "crate" => {
            c.rect(0x8a5a2c, x + 2, y, 12, 15);
            c.rect(0xa5703a, x + 2, y, 12, 2);
            c.rect(0x5a3a1c, x + 2, y + 13, 12, 2);
            c.rect(0xa5703a, x + 2, y, 2, 15);
            c.rect(0x5a3a1c, x + 12, y, 2, 15);
            c.rect(0x6b4a2a, x + 7, y, 2, 15);
            c.rect(0x6b4a2a, x + 2, y + 7, 12, 2);
        }
        "sarcophagus" => {
            c.rect(0x5c5448, x + 2, y, 28, 15);
            c.rect(0x6e6557, x + 2, y, 28, 3);
            c.rect(0x3c362d, x + 2, y + 13, 28, 2);
            c.rect(0x7a7064, x + 5, y + 3, 22, 9);
            c.rect(0x3c362d, x + 13, y + 3, 6, 5);
            c.rect(0x9a968a, x + 13, y + 3, 6, 1);
            c.rect(0x2a2620, x + 14, y + 4, 1, 2);
            c.rect(0x2a2620, x + 17, y + 4, 1, 2);
        }
        "statue" => {
            c.rect(0x8a8680, x + 3, y + 12, 10, 4);
            c.rect(0x6e6a62, x + 3, y + 14, 10, 2);
            c.rect(0xb6b2a6, x + 5, y - 8, 6, 20);
            c.rect(0xc6c2b6, x + 5, y - 10, 6, 4);
            c.rect(0xa6a296, x + 4, y - 5, 2, 8);
            c.rect(0xa6a296, x + 11, y - 5, 2, 8);
            if r.next_f64() < 0.4 {
                c.rect(0x3c362d, x + 6, y - 9, 4, 1);
            }
        }
        "urn" => {
            let base = if r.next_f64() < 0.5 { 0x9a6e3a } else { 0x7a5a8a };
            c.rect(base, x + 4, y, 8, 16);
            c.rect(0x000000, x + 5, y, 6, 2);
            c.rect(0xcaa84a, x + 4, y + 6, 8, 1);
            c.rect(0xcaa84a, x + 4, y + 10, 8, 1);
            c.rect(0x3a2a18, x + 4, y + 14, 8, 2);
            c.rect(0xbcbcb0, x + 4, y, 2, 16);
        }
        "brokenpillar" => {
            let top = -4 - ri(r, 8.0);
            c.rect(th.wall, x + 4, y + top, 8, 16 - top);
            c.rect(th.wall_top, x + 4, y + top, 8, 2);
            c.rect(th.grout, x + 4, y + 8, 8, 1);
            c.rect(0x000000, x + 4, y + top, 1, 16 - top);
        }
        "block" => {
            c.rect(th.grout, x, y, 16, 16);
            c.rect(th.wall, x + 1, y + 1, 14, 14);
            c.rect(th.wall_top, x + 1, y + 1, 14, 2);
            c.rect(th.grout, x + 1, y + 13, 14, 2);
            c.rect(th.wall_top, x + 2, y + 3, 1, 10);
            c.rect(th.grout, x + 13, y + 3, 1, 10);
            c.rect(th.grout, x + 3, y + 8, 10, 1);
        }
        "stalagmite" => {
            let tip = -6 - ri(r, 6.0);
            for yy in tip..16 {
                let t = (yy - tip) as f32 / (16 - tip) as f32;
                let hw = ((1.0 + t * 5.0).round() as i32).max(1);
                c.rect(th.wall, x + 8 - hw, y + yy, hw * 2, 1);
                c.rect(th.wall_top, x + 8 - hw, y + yy, 1, 1);
            }
        }
        "crystal" => {
            let cols: [u32; 3] = if r.next_f64() < 0.5 { [0x7028a8, 0xb060f0, 0xe0c0ff] } else { [0x1a6a8a, 0x3aa8d0, 0xbff0ff] };
            for (cx, top) in [(8, -2), (5, 4), (11, 5)] {
                for yy in top..14 {
                    let t = (yy - top) as f32 / (14 - top) as f32;
                    let hw = (((1.0 - t) * 3.0 + 1.0).round() as i32).max(1);
                    c.rect(cols[0], x + cx - hw, y + yy, hw * 2, 1);
                    c.rect(cols[1], x + cx, y + yy, 1, 1);
                }
                c.rect(cols[2], x + cx, y + top, 1, 2);
            }
        }
        "armorstand" => {
            c.rect(0x9a9aa5, x + 5, y - 10, 6, 5);
            c.rect(0x2a2a34, x + 6, y - 8, 4, 2);
            c.rect(0x8a8a95, x + 4, y - 4, 8, 9);
            c.rect(0xb0b0bc, x + 4, y - 4, 8, 2);
            c.rect(0x7a7a85, x + 3, y - 3, 2, 7);
            c.rect(0x7a7a85, x + 11, y - 3, 2, 7);
            c.rect(0x5a3a1c, x + 7, y + 5, 2, 11);
        }
        "weaponrack" => {
            c.rect(0x4a3320, x, y - 10, 32, 2);
            c.rect(0x4a3320, x, y + 12, 32, 3);
            let cols = [0xbcbcbc, 0xcfcfcf, 0x9a9aa5];
            let mut i = 3;
            while i < 29 {
                c.rect(cols[((i / 6) % 3) as usize], x + i, y - 8, 2, 20);
                c.rect(0x6b4a2a, x + i - 1, y + 8, 4, 3);
                i += 6;
            }
        }
        "altar" => {
            c.rect(0x9a968c, x + 2, y + 2, 28, 13);
            c.rect(0xb6b2a6, x + 2, y + 2, 28, 3);
            c.rect(0x6a675e, x + 2, y + 13, 28, 2);
            c.rect(0xcaa84a, x + 13, y - 3, 6, 6);
            c.rect(0xe0d0ff, x + 14, y - 2, 4, 4);
            c.rect(0xffffff, x + 15, y - 1, 2, 2);
        }
        "throne" => {
            c.rect(0x3a3a4a, x + 6, y - 12, 20, 28);
            c.rect(0x4a4a5e, x + 6, y - 12, 20, 3);
            c.rect(0xcaa84a, x + 6, y - 12, 20, 1);
            c.rect(0x7a2a3a, x + 9, y - 4, 14, 14);
            c.rect(0xcaa84a, x + 8, y - 13, 2, 5);
            c.rect(0xcaa84a, x + 22, y - 13, 2, 5);
        }
        "brazier" => {
            c.rect(0x3a3a3a, x + 6, y + 6, 4, 9);
            c.rect(0x2a2a2a, x + 4, y + 14, 8, 2);
            c.rect(0x5a5a5a, x + 3, y + 3, 10, 4);
            c.rect(0x1a1410, x + 4, y + 3, 8, 2);
            c.rect(0xfc7460, x + 4, y, 3, 4);
            c.rect(0xfcae40, x + 7, y - 2, 3, 5);
            c.rect(0xfce0a8, x + 8, y - 1, 2, 2);
            c.rect(0xfc7460, x + 10, y + 1, 2, 3);
        }
        "candelabra" => {
            c.rect(0xcaa84a, x + 7, y + 1, 2, 13);
            c.rect(0xcaa84a, x + 3, y + 5, 10, 2);
            c.rect(0xe8d8a0, x + 2, y + 1, 2, 5);
            c.rect(0xe8d8a0, x + 7, y - 1, 2, 5);
            c.rect(0xe8d8a0, x + 12, y + 1, 2, 5);
            c.rect(0xfcae40, x + 2, y - 1, 2, 2);
            c.rect(0xfcae40, x + 7, y - 3, 2, 2);
            c.rect(0xfcae40, x + 12, y - 1, 2, 2);
        }
        "bonepile" => {
            c.rect(0xd8d4c4, x + 4, y + 7, 8, 7);
            c.rect(0xd8d4c4, x + 4, y + 5, 5, 5);
            c.rect(0x1a1a1a, x + 5, y + 6, 1, 1);
            c.rect(0x1a1a1a, x + 7, y + 6, 1, 1);
            c.rect(0x9a968a, x + 3, y + 13, 10, 1);
            c.rect(0xd8d4c4, x + 11, y + 9, 4, 1);
            c.rect(0xd8d4c4, x + 2, y + 11, 4, 1);
        }
        "fireplace" => {
            c.rect(0x5a5650, x, y - 15, 32, 31);
            c.rect(0x6e6a62, x, y - 15, 32, 3);
            c.rect(0xcaa84a, x, y - 15, 32, 1);
            c.rect(0x1a1410, x + 5, y - 5, 22, 21);
            c.rect(0x3a1e10, x + 7, y - 1, 18, 17);
            c.rect(0xfc6020, x + 9, y + 6, 14, 9);
            c.rect(0xfcae40, x + 12, y + 3, 8, 12);
            c.rect(0xfce0a8, x + 14, y + 6, 4, 8);
        }
        "banner" => {
            let cols = [0x7a2a3a, 0x2a3a7a, 0x3a6a3a, 0x6a4a8a];
            let col = cols[ri(r, cols.len() as f64) as usize];
            c.rect(0xcaa84a, x + 3, y - 14, 10, 2);
            c.rect(col, x + 4, y - 12, 8, 22);
            c.rect(0xcaa84a, x + 6, y - 6, 4, 4);
            c.rect(col, x + 4, y + 10, 3, 3);
            c.rect(col, x + 9, y + 10, 3, 3);
        }
        "painting" => {
            c.rect(0xcaa84a, x + 2, y - 11, 12, 13);
            c.rect(0x3a2a18, x + 3, y - 10, 10, 11);
            c.rect(0x3a5a7a, x + 4, y - 9, 8, 9);
            c.rect(0x2a8a3a, x + 4, y - 3, 8, 3);
            c.rect(0xcaa84a, x + 9, y - 8, 2, 2);
        }
        "chains" => {
            let len = 8 + ri(r, 6.0);
            for i in 0..len {
                c.rect(if i & 1 == 1 { 0x5a5a62 } else { 0x3a3a42 }, x + 5 + (i & 1), y - 13 + i, 2, 1);
            }
            if r.next_f64() < 0.5 {
                c.rect(0x3a3a42, x + 4, y - 13 + len, 6, 2);
            }
        }
        "vines" => {
            let mut vx = 5 + ri(r, 6.0);
            for yy in -13..12 {
                c.rect(0x2a5a24, x + vx, y + yy, 2, 1);
                if r.next_f64() < 0.3 {
                    let off = if r.next_f64() < 0.5 { -1 } else { 2 };
                    c.rect(0x3a7a2e, x + vx + off, y + yy, 1, 1);
                }
                vx = (vx + ri(r, 3.0) - 1).clamp(2, 13);
            }
        }
        "cobweb" => {
            let a = 128; // rgba(228,228,234,0.5)
            const WEB: u32 = 0xe4e4ea;
            if let Some(cn) = corner {
                let rgt = cn.contains('r');
                let bot = cn.contains('b');
                let (ax, ay) = (if rgt { x + 15 } else { x }, if bot { y + 15 } else { y });
                let (dx, dy) = (if rgt { -1 } else { 1 }, if bot { -1 } else { 1 });
                for i in 0..6 {
                    let len = 13 - i * 2;
                    if len <= 0 {
                        continue;
                    }
                    c.blend(WEB, a, if rgt { ax - len + 1 } else { ax }, ay + i * 2 * dy, len, 1);
                    c.blend(WEB, a, ax + i * 2 * dx, if bot { ay - len + 1 } else { ay }, 1, len);
                }
            } else {
                let (cx, cy) = (x + 8, y + 8);
                c.blend(WEB, a, cx - 7, cy, 15, 1);
                c.blend(WEB, a, cx, cy - 7, 1, 15);
                let mut s = -6;
                while s <= 6 {
                    c.blend(WEB, a, cx + s, cy + s, 1, 1);
                    c.blend(WEB, a, cx + s, cy - s, 1, 1);
                    s += 2;
                }
                for rr in [4, 7] {
                    for o in 0..=rr {
                        c.blend(WEB, a, cx - rr + o, cy - o, 1, 1);
                        c.blend(WEB, a, cx + rr - o, cy - o, 1, 1);
                        c.blend(WEB, a, cx - rr + o, cy + o, 1, 1);
                        c.blend(WEB, a, cx + rr - o, cy + o, 1, 1);
                    }
                }
            }
        }
        "crack" => {
            let mut cx = 2 + ri(r, 12.0);
            let mut cy = 2;
            for _ in 0..12 {
                c.rect(th.grout, x + cx, y + cy, 1, 1);
                cx += ri(r, 3.0) - 1;
                if r.next_f64() < 0.7 {
                    cy += 1;
                }
                if cy > 14 {
                    break;
                }
            }
        }
        "moss" => {
            for _ in 0..10 {
                let col = if r.next_f64() < 0.5 { 0x2a5a24 } else { 0x3a7a2e };
                let (mx, my) = (ri(r, 14.0), ri(r, 14.0));
                c.rect(col, x + mx, y + my, 1 + ri(r, 2.0), 1 + ri(r, 2.0));
            }
        }
        "bones" => {
            c.rect(0xd8d4c4, x + 3, y + 6, 5, 4);
            c.rect(0x1a1a1a, x + 4, y + 7, 1, 1);
            c.rect(0x1a1a1a, x + 6, y + 7, 1, 1);
            c.rect(0xd8d4c4, x + 9, y + 9, 5, 1);
            c.rect(0x9a968a, x + 8, y + 6, 1, 4);
        }
        "rubble" => {
            for _ in 0..6 {
                let col = if r.next_f64() < 0.5 { th.wall } else { th.wall_top };
                let (rx2, ry2) = (ri(r, 13.0), ri(r, 13.0));
                c.rect(col, x + rx2, y + ry2, 2 + ri(r, 2.0), 2);
            }
        }
        "mushroompatch" => {
            let cap = if r.next_f64() < 0.4 {
                0xc83838
            } else if r.next_f64() < 0.6 {
                0xa050d0
            } else {
                0x40c0a8
            };
            for i in 0..3 {
                let mx = 3 + i * 4 + ri(r, 2.0);
                let my = 6 + ri(r, 4.0);
                c.rect(0xe0e0d0, x + mx, y + my, 2, 4);
                c.rect(cap, x + mx - 1, y + my - 2, 4, 2);
            }
        }
        "bloodstain" => {
            for _ in 0..8 {
                let a = r.next_f64() * JS_TAU; // js-verbatim 6.28
                let d = r.next_f64() * 6.0;
                let (bx, by) = ((a.cos() * d).round() as i32, (a.sin() * d).round() as i32);
                c.blend(0x501010, 128, x + 8 + bx, y + 8 + by, 2 + ri(r, 2.0), 2);
            }
        }
        "puddle" => {
            c.blend(0x28465a, 128, x + 2, y + 6, 12, 6);
            c.blend(0x28465a, 128, x + 4, y + 4, 8, 2);
            c.blend(0x78a0b4, 102, x + 4, y + 6, 4, 1);
        }
        "rug" => {
            let cs = [[0x5a2a3a, 0x9a5a6a], [0x2a3a5a, 0x4a5a8a], [0x3a2a5a, 0x6a4a9a]][ri(r, 3.0) as usize];
            c.rect(cs[0], x, y, 48, 32);
            c.rect(cs[1], x, y, 48, 2);
            c.rect(cs[1], x, y + 30, 48, 2);
            c.rect(cs[1], x, y, 2, 32);
            c.rect(cs[1], x + 46, y, 2, 32);
            c.rect(cs[1], x + 20, y + 12, 8, 8);
            c.rect(cs[0], x + 22, y + 14, 4, 4);
        }
        "magiccircle" => {
            let (cx, cy) = (x + 24, y + 16);
            for rr in [14.0f32, 9.0] {
                let mut t = 0.0f32;
                while t < 6.3 {
                    c.blend(0x965af0, 153, cx + (t.cos() * rr).round() as i32, cy + (t.sin() * rr).round() as i32, 1, 1);
                    t += 0.05;
                }
            }
            for i in 0..6 {
                let a = i as f32 / 6.0 * JS_TAU as f32;
                c.blend(0xb478ff, 128, cx + (a.cos() * 11.0).round() as i32 - 1, cy + (a.sin() * 11.0).round() as i32 - 1, 2, 2);
            }
        }
        _ => {}
    }
}
