//! tree_art.rs — the EXOTIC biome tree generators (port of buildShroom/buildBurnttree/
//! buildRiftbulb/buildVoidspire/buildMawtree/buildGiantFlower/buildCrystalSpire/
//! buildStalagmite from js/entities.js). Until these landed, every one of these kinds
//! fell back to the green oak silhouette — eleven biomes of wrong trees (the Trello
//! card). Same seeded-rnd streams, same chars, same not-outlined exceptions as the js.

use super::props::{blank, outlined, px_hash};

/// The js xorshift stream every builder uses: seed*2654435761 + salt, then
/// imul-avalanche per draw (byte-parity with buildDeadtree's port).
fn stream(seed: i32, salt: i64) -> impl FnMut() -> f64 {
    let mut s = (seed as i64 * 2654435761 + salt) as u32;
    move || {
        s = ((s ^ (s >> 15)) as i32).wrapping_mul(2246822519u32 as i32) as u32;
        s ^= s >> 13;
        s as f64 / 4294967296.0
    }
}

fn put(g: &mut [Vec<char>], w: i32, h: i32, x: i32, y: i32, c: char) {
    if (0..w).contains(&x) && (0..h).contains(&y) {
        g[y as usize][x as usize] = c;
    }
}

/// Giant whimsical mushroom tree: fat pale stalk under a big spotted cap (cap colour
/// varies per tile). 48x72 (port of `buildShroom`).
pub fn build_shroom(seed: i32) -> Vec<String> {
    let (w, h, cx) = (48i32, 72i32, 24i32);
    let mut g = blank(w as usize, h as usize);
    let mut rnd = stream(seed, 7);
    let caps = [('r', 'o'), ('m', 'W'), ('u', 'l'), ('P', 'y')]; // red, pink, teal, gold
    let (cap, hi) = caps[(rnd() * caps.len() as f64).floor() as usize];
    let stalk_top = 36;
    for y in stalk_top..h {
        // Pale stalk, widening at the base.
        let t = (y - stalk_top) as f64 / (h - stalk_top) as f64;
        let hw = (4.0 + t * 3.0).round() as i32;
        for x in cx - hw..=cx + hw {
            put(&mut g, w, h, x, y, if x <= cx - hw + 1 { 'A' } else { 'W' });
        }
    }
    let (cap_top, cap_bot) = (6, stalk_top + 2); // the big dome cap
    for y in cap_top..=cap_bot {
        let t = (y - cap_top) as f64 / (cap_bot - cap_top) as f64;
        let hw = (5.0 + (t * std::f64::consts::PI * 0.55).sin() * 17.0).round() as i32;
        for x in cx - hw..=cx + hw {
            put(&mut g, w, h, x, y, if y < cap_top + 4 { hi } else { cap });
        }
    }
    for _ in 0..8 {
        // White toadstool spots.
        let sx = cx - 13 + (rnd() * 26.0).floor() as i32;
        let sy = cap_top + 4 + (rnd() * 16.0).floor() as i32;
        put(&mut g, w, h, sx, sy, 'W');
        put(&mut g, w, h, sx + 1, sy, 'W');
        put(&mut g, w, h, sx, sy + 1, 'W');
    }
    outlined(&g)
}

/// Burnt tree: charred black trunk — some snapped to a smoking stump, others bare
/// crooked limbs — with embers still glowing at the base. 48x72 (`buildBurnttree`).
pub fn build_burnttree(seed: i32) -> Vec<String> {
    let (w, h, cx) = (48i32, 72i32, 24i32);
    let mut g = blank(w as usize, h as usize);
    let mut rnd = stream(seed, 3);
    let snapped = rnd() < 0.4; // some are just charred stumps
    let top = if snapped { 42 + (rnd() * 8.0).floor() as i32 } else { 18 + (rnd() * 6.0).floor() as i32 };
    for y in top..h {
        // Charred trunk: ash-gray to black, narrow.
        let mut half = 3 + (((y - top) as f64 / (h - top) as f64) * 1.5).round() as i32;
        if y >= h - 2 {
            half += 1;
        }
        for x in cx - half..=cx + half {
            let f = (x - (cx - half)) as f64 / (2 * half) as f64;
            put(&mut g, w, h, x, y, if f < 0.3 { 'n' } else if f > 0.7 { 'K' } else { 'a' });
        }
    }
    if snapped {
        for _ in 0..5 {
            // The jagged broken top.
            let x = cx - 2 + (rnd() * 5.0).floor() as i32;
            let y = top - (rnd() * 3.0).floor() as i32;
            let c = if rnd() < 0.5 { 'K' } else { 'n' };
            put(&mut g, w, h, x, y, c);
        }
    } else {
        // Bare, crooked, charred limbs (no leaves).
        let limb = |g: &mut Vec<Vec<char>>, rnd: &mut dyn FnMut() -> f64, mut x: i32, mut y: i32, dx: i32, len: i32| {
            for _ in 0..len {
                put(g, w, h, x, y, 'K');
                put(g, w, h, x, y - 1, 'n');
                x += dx;
                if rnd() < 0.6 {
                    y -= 1;
                }
            }
        };
        let l1 = 7 + (rnd() * 3.0).floor() as i32;
        limb(&mut g, &mut rnd, cx - 2, top + 8, -1, l1);
        let l2 = 7 + (rnd() * 3.0).floor() as i32;
        limb(&mut g, &mut rnd, cx + 2, top + 5, 1, l2);
        if rnd() < 0.6 {
            limb(&mut g, &mut rnd, cx - 1, top + 16, -1, 5);
        }
        if rnd() < 0.6 {
            limb(&mut g, &mut rnd, cx + 1, top + 18, 1, 5);
        }
    }
    let n = 2 + (rnd() * 3.0).floor() as i32;
    for _ in 0..n {
        // Glowing embers at the foot.
        let x = cx - 4 + (rnd() * 9.0).floor() as i32;
        let y = h - 6 - (rnd() * 14.0).floor() as i32;
        let c = if rnd() < 0.5 { 'r' } else { 'o' };
        put(&mut g, w, h, x, y, c);
    }
    outlined(&g)
}

// --- CHAOS biome: three families of truly alien trees, seeded colour/shape varieties. ---

/// Riftbulb: a writhing dark-purple trunk crowned by a bulbous glowing orb (veined,
/// with eye-spots). 48x72 (`buildRiftbulb`).
pub fn build_riftbulb(seed: i32) -> Vec<String> {
    let (w, h, cx) = (48i32, 72i32, 24i32);
    let mut g = blank(w as usize, h as usize);
    let mut rnd = stream(seed, 11);
    const SKINS: [(char, char, char); 4] = [('x', 'm', 'W'), ('m', 'x', 'W'), ('u', 'l', 'W'), ('X', 'x', 'm')];
    let (body, hi, vein) = SKINS[(rnd() * SKINS.len() as f64).floor() as usize];
    let top = 32 + (rnd() * 4.0).floor() as i32;
    for y in top..h {
        // The twisting trunk.
        let sway = (((y - top) as f64 * 0.35 + seed as f64).sin() * 2.0).round() as i32;
        let half = 2 + ((y - top) as f64 / (h - top) as f64 * 2.0).round() as i32;
        for x in cx + sway - half..=cx + sway + half {
            put(&mut g, w, h, x, y, if x < cx + sway { 'X' } else { 'x' });
        }
    }
    let cy_b = 22i32;
    let r_base = 14 + (rnd() * 4.0).floor() as i32;
    for y in 0..top + 4 {
        for x in 0..w {
            // The bulbous orb, its rim wobbling with the seed.
            let dx = (x - cx) as f64;
            let dy = (y - cy_b) as f64 * 1.15;
            let ang = dy.atan2(dx);
            let r = r_base as f64 + 2.4 * (ang * 4.0 + seed as f64).sin() + 1.6 * (ang * 2.0 - seed as f64).sin();
            if dx.hypot(dy) > r {
                continue;
            }
            let vy = (y - cy_b) as f64 / r_base as f64;
            put(&mut g, w, h, x, y, if vy < -0.32 { hi } else if vy > 0.5 { 'X' } else { body });
        }
    }
    for _ in 0..5 {
        // Pulsing veins.
        let mut vx = cx - 8 + (rnd() * 16.0).floor() as i32;
        let vy0 = cy_b - 9 + (rnd() * 8.0).floor() as i32;
        for vy in vy0..vy0 + 6 {
            put(&mut g, w, h, vx, vy, vein);
            vx += (rnd() * 3.0).floor() as i32 - 1;
        }
    }
    for _ in 0..3 {
        // Glowing eye-spots.
        let ex = cx - 7 + (rnd() * 14.0).floor() as i32;
        let ey = cy_b - 3 + (rnd() * 8.0).floor() as i32;
        put(&mut g, w, h, ex, ey, 'W');
        put(&mut g, w, h, ex + 1, ey, hi);
        put(&mut g, w, h, ex, ey + 1, hi);
    }
    outlined(&g)
}

/// Voidspire: a craggy base from which jagged faceted crystal shards rise, each
/// spark-tipped. 48x72 (`buildVoidspire`).
pub fn build_voidspire(seed: i32) -> Vec<String> {
    let (w, h, cx) = (48i32, 72i32, 24i32);
    let mut g = blank(w as usize, h as usize);
    let mut rnd = stream(seed, 17);
    const SKINS: [(char, char, char); 3] = [('X', 'x', 'W'), ('x', 'm', 'W'), ('u', 'l', 'W')];
    let (dark, lite, spark) = SKINS[(rnd() * SKINS.len() as f64).floor() as usize];
    let base_top = 46;
    for y in base_top..h {
        // The rocky base.
        let half = 4 + ((y - base_top) as f64 / (h - base_top) as f64 * 5.0).round() as i32;
        for x in cx - half..=cx + half {
            put(&mut g, w, h, x, y, if x < cx { dark } else { 'X' });
        }
    }
    let n = 3 + (rnd() * 3.0).floor() as i32;
    for _ in 0..n {
        // One faceted shard: dark left face, lit centre ridge, spark at the tip.
        let bx = cx - 9 + (rnd() * 18.0).floor() as i32;
        let tip_y = 5 + (rnd() * 22.0).floor() as i32;
        let base_y = base_top + 2;
        let lean = (rnd() * 5.0).floor() as i32 - 2;
        let half_w = 1 + (rnd() * 2.0).floor() as i32;
        for y in tip_y..=base_y {
            let t = (base_y - y) as f64 / 1.max(base_y - tip_y) as f64;
            let cl = (bx as f64 + lean as f64 * t).round() as i32;
            let hw = ((1.0 - t) * (1 + half_w) as f64).round() as i32;
            for x in cl - hw..=cl + hw {
                put(&mut g, w, h, x, y, if x < cl { dark } else if x > cl { 'X' } else { lite });
            }
        }
        put(&mut g, w, h, bx + lean, tip_y, spark);
        put(&mut g, w, h, bx + lean, tip_y + 1, lite);
    }
    outlined(&g)
}

/// Mawtree: a fleshy maw at the base sprouting curling tentacle-branches with glowing
/// sucker-tips. 48x72 (`buildMawtree`).
pub fn build_mawtree(seed: i32) -> Vec<String> {
    let (w, h, cx) = (48i32, 72i32, 24i32);
    let mut g = blank(w as usize, h as usize);
    let mut rnd = stream(seed, 23);
    const TIPS: [char; 4] = ['m', 'x', 'W', 'u'];
    let tip = TIPS[(rnd() * TIPS.len() as f64).floor() as usize];
    let top = 40;
    for y in top..h {
        // The fleshy bulb base.
        let half = 4 + ((y - top) as f64 / (h - top) as f64 * 4.0).round() as i32;
        for x in cx - half..=cx + half {
            put(&mut g, w, h, x, y, if x < cx { 'X' } else { 'x' });
        }
    }
    for y in top + 3..top + 9 {
        // The dark maw...
        for x in cx - 4..=cx + 4 {
            put(&mut g, w, h, x, y, 'K');
        }
    }
    let mut x = cx - 4;
    while x <= cx + 4 {
        // ...and its teeth.
        put(&mut g, w, h, x, top + 3, 'W');
        put(&mut g, w, h, x, top + 8, 'W');
        x += 2;
    }
    let n = 4 + (rnd() * 3.0).floor() as i32;
    for _ in 0..n {
        // One curling tentacle, glowing at the tip.
        let mut x = cx - 8 + (rnd() * 16.0).floor() as i32;
        let mut y = top + 1;
        let dir: f64 = if rnd() < 0.5 { 1.0 } else { -1.0 };
        let curl = 0.18 + rnd() * 0.3;
        let len = 16 + (rnd() * 16.0).floor() as i32;
        for k in 0..len {
            put(&mut g, w, h, x, y, 'x');
            put(&mut g, w, h, x, y - 1, 'X');
            x += ((k as f64 * curl).sin() * dir * 1.3).round() as i32;
            y -= 1;
            if y < 2 {
                break;
            }
        }
        put(&mut g, w, h, x, y, tip);
        put(&mut g, w, h, x - 1, y, tip);
        put(&mut g, w, h, x + 1, y, tip);
        put(&mut g, w, h, x, y + 1, tip);
    }
    outlined(&g)
}

/// Giant flower (honeyglade): a tall green stalk with two leaves and a big petaled
/// bloom over a gold centre; petal hue varies per tile. 48x72, NOT outlined
/// (`buildGiantFlower` returns its raw grid).
pub fn build_giantflower(seed: i32) -> Vec<String> {
    let (w, h, cx, cy_f) = (48i32, 72i32, 24i32, 20i32);
    let mut g = blank(w as usize, h as usize);
    let r_of = |n: i32| px_hash(n, seed, seed * 3 + 7) as f64 / 1000.0;
    let ell = |g: &mut Vec<Vec<char>>, ecx: f64, ecy: f64, rx: f64, ry: f64, ch: char| {
        for y in 0.max((ecy - ry).floor() as i32)..=(h - 1).min((ecy + ry).ceil() as i32) {
            for x in 0.max((ecx - rx).floor() as i32)..=(w - 1).min((ecx + rx).ceil() as i32) {
                let dx = (x as f64 - ecx) / rx;
                let dy = (y as f64 - ecy) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    g[y as usize][x as usize] = ch;
                }
            }
        }
    };
    for y in cy_f..h {
        // The stalk (shaded on its right edge).
        for x in cx - 2..=cx + 2 {
            g[y as usize][x as usize] = if (x - (cx - 2)) as f64 / 4.0 > 0.7 { 's' } else { 'S' };
        }
    }
    // The two leaves.
    ell(&mut g, (cx - 7) as f64, 46.0, 6.0, 3.0, 'L');
    ell(&mut g, (cx - 11) as f64, 45.0, 3.0, 2.0, 'l');
    ell(&mut g, (cx + 8) as f64, 54.0, 6.0, 3.0, 'L');
    ell(&mut g, (cx + 12) as f64, 53.0, 3.0, 2.0, 'l');
    // The petal ring, each with an inner highlight.
    let n = 6 + seed.rem_euclid(3);
    for i in 0..n {
        let a = (i as f64 / n as f64) * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2 + (r_of(i) - 0.5) * 0.25;
        let pxc = cx as f64 + a.cos() * 12.0;
        let pyc = cy_f as f64 + a.sin() * 11.0;
        ell(&mut g, pxc, pyc, 7.5, 6.0, 'P');
        ell(&mut g, pxc - a.cos() * 2.0 - 1.5, pyc - a.sin() * 2.0 - 1.5, 4.0, 3.0, 'H');
    }
    // The centre disc + seed speckle.
    ell(&mut g, cx as f64, cy_f as f64, 8.5, 8.0, 'C');
    ell(&mut g, cx as f64, cy_f as f64, 5.5, 5.0, 'c');
    for i in 0..10 {
        let a = r_of(i + 40) * std::f64::consts::TAU;
        let rr = r_of(i + 60) * 4.5;
        let sy = (cy_f as f64 + a.sin() * rr).round() as i32;
        let sx = (cx as f64 + a.cos() * rr).round() as i32;
        if (0..h).contains(&sy) && (0..w).contains(&sx) {
            g[sy as usize][sx as usize] = 'o';
        }
    }
    g.iter().map(|r| r.iter().collect()).collect()
}

/// The giant flower's seeded palette: petal hue by tile (pink/coral/gold/lavender)
/// over the fixed stalk/leaf/centre greens (js GF_HUES + GF_BASE).
pub fn giantflower_pal(seed: i32) -> Vec<(char, u32)> {
    const HUES: [[u32; 3]; 4] = [
        [0xff8ec8, 0xf05aa0, 0xffd0ec], // pink
        [0xff9a6a, 0xf0603a, 0xffd6b0], // coral
        [0xffd24a, 0xf0a020, 0xfff0b0], // gold
        [0xc79cff, 0x9a6ae0, 0xeaddff], // lavender
    ];
    let [p, pl, hi] = HUES[seed.rem_euclid(4) as usize];
    vec![
        ('P', p),
        ('p', pl),
        ('H', hi),
        ('S', 0x4f9a2c),
        ('s', 0x2f7a24),
        ('L', 0x5ab84a),
        ('l', 0x8ee06a),
        ('C', 0xfcd000),
        ('c', 0xe0a020),
        ('o', 0xa06a10),
    ]
}

/// Crystal spire (prismwastes): a faceted violet cluster with a glowing cyan core,
/// seeded size. 16x26, NOT outlined (`buildCrystalSpire`).
pub fn build_crystalspire(seed: i32) -> Vec<String> {
    let (w, h) = (16i32, 26i32);
    let mut g = blank(w as usize, h as usize);
    let rnd = |n: i32| ((seed / n) % 1000) as f64 / 1000.0;
    let cx = 8i32;
    let top_y = 3 + (rnd(7) * 5.0).floor() as i32;
    let base_y = h - 3;
    let shard = |g: &mut Vec<Vec<char>>, sx: i32, s_top: i32, mul: f64| {
        for y in s_top..=base_y {
            let t = (y - s_top) as f64 / 1.max(base_y - s_top) as f64;
            let hw = 0.max(((0.5 + t * 3.0) * mul).round() as i32);
            for x in sx - hw..=sx + hw {
                if !(0..w).contains(&x) {
                    continue;
                }
                let edge = x == sx - hw || x == sx + hw;
                if g[y as usize][x as usize] == '.' || edge {
                    g[y as usize][x as usize] = if edge { 'X' } else if x < sx { 'v' } else { 'x' };
                }
            }
            if (0..w).contains(&sx) {
                g[y as usize][sx as usize] = 'c'; // the bright cyan core
            }
        }
    };
    shard(&mut g, cx, top_y, 1.0);
    shard(&mut g, cx - 3 - (rnd(11) * 2.0).floor() as i32, top_y + 6, 0.6);
    shard(&mut g, cx + 3 + (rnd(13) * 2.0).floor() as i32, top_y + 5, 0.6);
    g.iter().map(|r| r.iter().collect()).collect()
}

/// Stalagmite (blackdeep): a grey rock spike, tree-like with VARIED height per tile.
/// 16x28, NOT outlined (`buildStalagmite`).
pub fn build_stalagmite(seed: i32) -> Vec<String> {
    let (w, h) = (16i32, 28i32);
    let mut g = blank(w as usize, h as usize);
    let rnd = |n: i32| ((seed / n) % 1000) as f64 / 1000.0;
    let cx = 8i32;
    let top_y = 3 + (rnd(7) * 11.0).floor() as i32;
    let base_y = h - 3;
    for y in top_y..=base_y {
        let t = (y - top_y) as f64 / 1.max(base_y - top_y) as f64;
        let hw = 0.max((0.5 + t * 4.5).round() as i32);
        for x in cx - hw..=cx + hw {
            if !(0..w).contains(&x) {
                continue;
            }
            let edge = x == cx - hw || x == cx + hw;
            g[y as usize][x as usize] = if edge { 'K' } else if x < cx { 'a' } else { 'n' };
        }
        if cx - hw < cx - 1 {
            let rx = cx - hw + 1;
            if (0..w).contains(&rx) {
                g[y as usize][rx as usize] = 'A'; // the light ridge on the left face
            }
        }
    }
    g.iter().map(|r| r.iter().collect()).collect()
}
