//! castle_art.rs — the Black Castle facade (js/entities.js buildCastle, transcribed):
//! a dark three-tower keep, 192x144, gate at bottom-centre over the anchor tile. The
//! SEALED doors + ten shard-sockets bake in per state; the open gate's roiling rift
//! glow is approximated with a static bloom (the live shimmer joins the glow pass).

use crate::dungeon::prop_paint::Px;

const STONE: u32 = 0x403c48;
const SD: u32 = 0x2a2732;
const SL: u32 = 0x544f5e;
const TOPC: u32 = 0x615c6c;
const DARK: u32 = 0x1a1820;
const BLACK: u32 = 0x0e0c14;
const MORTAR: u32 = 0x322e3a;
const FIRE: u32 = 0xd83018;
const FIRE2: u32 = 0xfc7030;
const EYE: u32 = 0x9a50e0;
const EYE2: u32 = 0xe0b0ff;
const BAN: u32 = 0x48206a;
const BAN2: u32 = 0x7a3ab0;

pub const W: i32 = 192;
pub const H: i32 = 144;

fn block(c: &mut Px, x: i32, y: i32, w: i32, h: i32) {
    c.rect(STONE, x, y, w, h);
    c.rect(SL, x, y, 2, h);
    c.rect(SD, x + w - 2, y, 2, h);
    let mut yy = y + 8;
    while yy < y + h {
        c.rect(MORTAR, x, yy, w, 1);
        yy += 8;
    }
}

fn cren(c: &mut Px, x: i32, y: i32, w: i32) {
    c.rect(TOPC, x, y, w, 3);
    let mut i = 0;
    while i < w {
        c.rect(TOPC, x + i, y - 4, 4, 5);
        c.rect(SL, x + i, y - 4, 1, 5);
        c.rect(SD, x + i + 3, y - 4, 1, 5);
        i += 7;
    }
}

fn slit(c: &mut Px, x: i32, y: i32, h2: i32) {
    c.rect(DARK, x - 1, y - 1, 5, h2 + 2);
    c.rect(FIRE, x + 1, y, 2, h2);
    c.rect(FIRE2, x + 1, y, 2, h2 >> 1);
}

/// Bake the fortress with its gate state: `unlocked` opens the arch (the rift bloom);
/// sealed bakes the iron doors + the ten sockets, `shards` of them lit.
pub fn castle_rgba(unlocked: bool, shards: usize) -> Vec<u8> {
    let mut buf = vec![0u8; (W * H * 4) as usize];
    let mut c = Px { buf: &mut buf, w: W, h: H };
    // tier 1 - curtain walls (lowest)
    block(&mut c, 38, 66, 30, H - 66);
    cren(&mut c, 38, 66, 30);
    block(&mut c, 124, 66, 30, H - 66);
    cren(&mut c, 124, 66, 30);
    slit(&mut c, 51, 86, 12);
    slit(&mut c, 137, 86, 12);
    // tier 2 - corner towers, tattered banners streaming the same wind
    block(&mut c, 6, 40, 34, H - 40);
    cren(&mut c, 6, 40, 34);
    block(&mut c, 152, 40, 34, H - 40);
    cren(&mut c, 152, 40, 34);
    slit(&mut c, 20, 58, 12);
    slit(&mut c, 166, 58, 12);
    slit(&mut c, 20, 96, 12);
    slit(&mut c, 166, 96, 12);
    c.rect(BLACK, 22, 28, 1, 12);
    c.rect(BAN, 23, 28, 9, 5);
    c.rect(BAN2, 23, 28, 9, 1);
    c.rect(BLACK, 169, 28, 1, 12);
    c.rect(BAN, 170, 28, 9, 5);
    c.rect(BAN2, 170, 28, 9, 1);
    // tier 3 - the central keep + upper turrets
    block(&mut c, 60, 22, 72, H - 22);
    cren(&mut c, 60, 22, 72);
    c.rect(SD, 72, 22, 1, H - 22);
    c.rect(SD, 119, 22, 1, H - 22);
    block(&mut c, 60, 12, 12, 12);
    cren(&mut c, 60, 12, 12);
    block(&mut c, 120, 12, 12, 12);
    cren(&mut c, 120, 12, 12);
    // tier 4 - the spire, bearing the rift-eye
    block(&mut c, 82, 6, 28, 18);
    cren(&mut c, 82, 6, 28);
    c.rect(DARK, 86, 10, 20, 9);
    c.rect(EYE, 88, 11, 16, 6);
    c.rect(EYE2, 91, 12, 10, 4);
    c.rect(BLACK, 94, 11, 3, 6);
    // fire-lit keep windows + gate banners
    slit(&mut c, 78, 50, 16);
    slit(&mut c, 112, 50, 16);
    slit(&mut c, 95, 38, 9);
    for bx in [64, 120] {
        c.rect(BAN, bx, 70, 8, 40);
        c.rect(BAN2, bx, 70, 8, 2);
        c.rect(EYE, bx + 2, 84, 4, 5);
        c.rect(BAN, bx, 110, 3, 3);
        c.rect(BAN, bx + 5, 110, 3, 3);
    }
    // the great gate
    c.rect(SD, 76, 84, 40, H - 84);
    c.rect(SL, 76, 84, 2, H - 84);
    c.rect(BLACK, 80, 90, 32, H - 90);
    c.rect(SD, 80, 90, 4, 4);
    c.rect(SD, 108, 90, 4, 4);
    c.rect(SD, 82, 88, 28, 3);
    slit(&mut c, 72, 104, 18);
    slit(&mut c, 116, 104, 18);
    // Gate state (the js draws these live; we bake per (unlocked, shards) and rebake
    // when the count changes). Gate centre in facade space: (96, 112).
    let (gx, gy) = (96, 112);
    if unlocked {
        // The rift bloom in the open arch (static stand-in for the roiling shimmer).
        for (r, col, a) in [(26, 0x6e28c8u32, 70u8), (18, 0x9646e6, 90), (10, 0xc882ff, 130)] {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy <= r * r {
                        c.blend(col, a, gx + dx, gy + 2 + dy, 1, 1);
                    }
                }
            }
        }
    } else {
        let (dl, dr, dtop, dbot) = (gx - 15, gx + 15, gy - 20, gy + 27);
        c.rect(0x262430, dl, dtop, dr - dl, dbot - dtop);
        c.rect(0x1a1822, gx - 1, dtop, 2, dbot - dtop);
        c.rect(0x1d1b25, dl, dtop, 1, dbot - dtop);
        c.rect(0x1d1b25, dr - 1, dtop, 1, dbot - dtop);
        let mut by = dtop + 4;
        while by < dbot - 1 {
            c.rect(0x36323e, dl, by, dr - dl, 2);
            let mut rx = dl + 2;
            while rx < dr - 1 {
                c.rect(0x4c4856, rx, by, 1, 1);
                rx += 5;
            }
            by += 8;
        }
        for hx in [gx - 5, gx + 6] {
            c.rect(0x4c4856, hx - 2, gy + 4, 5, 1); // ring handles (arc -> loop)
            c.rect(0x4c4856, hx - 2, gy + 8, 5, 1);
            c.rect(0x4c4856, hx - 3, gy + 5, 1, 3);
            c.rect(0x4c4856, hx + 2, gy + 5, 1, 3);
        }
        for i in 0..10usize {
            // Ten shard-sockets on the keep face light as the Wriftheart mends.
            let sx = gx - 18 + (i as i32 % 5) * 9;
            let sy = gy - 78 + (i as i32 / 5) * 8;
            if i < shards {
                c.blend(0xc882ff, 128, sx - 1, sy - 1, 6, 6);
            }
            c.rect(if i < shards { 0xe0b0ff } else { 0x241f2e }, sx, sy, 4, 4);
        }
    }
    buf
}
