//! cinematic/scenes.rs — the six opening scenes, the js canvas paintings ported
//! line-for-line (game.js drawCsWholeAge / drawCsSky / drawCsScatter /
//! drawCsChoirHill / drawCsInterior / drawCsTown + csGem / csFlame / csRunner).
//! Every constant, phase frame and colour is the js's. All coords y-DOWN.

use super::paint::Cv;
use super::{CsInner, W, H};
use bevy::prelude::*;

pub const CX: f32 = W as f32 / 2.0;

/// csGem — a faceted gem (the Wriftheart + its shards): body + top light + lower shade.
pub fn gem(cv: &mut Cv, x: f32, y: f32, r: f32, col: u32, a: f32) {
    let p = |dx: f32, dy: f32| (x + dx * r, y + dy * r);
    cv.poly(&[p(0.0, -1.0), p(0.72, -0.2), p(0.45, 1.0), p(-0.45, 1.0), p(-0.72, -0.2)], col, a);
    cv.poly(&[p(0.0, -1.0), p(0.72, -0.2), p(0.0, -0.08)], 0xffffff, 0.55 * a);
    cv.poly(&[p(0.0, -0.08), p(0.45, 1.0), p(-0.45, 1.0)], 0x000000, 0.22 * a);
}

/// csFade — the black in/out wash.
pub fn fade(cv: &mut Cv, a: f32) {
    if a > 0.0 {
        cv.rect(0, 0, W, H, 0x000000, a.min(1.0));
    }
}

fn stars(cv: &mut Cv, st: &CsInner, mul: f32) {
    for &(x, y, b) in &st.stars {
        cv.rect(x.round() as i32, y.round() as i32, 1, 1, 0xdfe6ff, b * mul);
    }
}

// SCENE 0 — the Whole Age: the heart beats gently over a green and easy land.
pub fn whole_age(cv: &mut Cv, lt: f32) {
    let cy = 54.0;
    cv.grad_v(0, 0, W, H, &[(0.0, 0x1c2c50, 1.0), (0.55, 0x40607e, 1.0), (1.0, 0x7fa08a, 1.0)]); // soft dawn
    // The heart, small and serene, its pulse synced to the gentle heartbeat sfx.
    let beat = (lt * 0.157).sin().max(0.0);
    let r = 12.0 * (1.0 + beat * beat * beat * 0.1);
    cv.radial(CX, cy, 2.0, r + 30.0, 0xc88cff, 0.5);
    gem(cv, CX, cy, r, 0xb060f0, 1.0);
    // Drifting birds, up in the warm air.
    for b in 0..4 {
        let bf = b as f32;
        let bx = ((bf * 96.0 + lt * (0.3 + bf * 0.07)) % (W as f32 + 30.0)) - 15.0;
        let by = 36.0 + bf * 13.0 + (lt * 0.06 + bf * 2.0).sin() * 3.0;
        let f = (lt * 0.25 + bf).sin() * 1.5;
        cv.line(bx - 3.0, by - f, bx, by, 0x101824, 0.55, 1);
        cv.line(bx, by, bx + 3.0, by - f, 0x101824, 0.55, 1);
    }
    // Layered green hills (each a sine ridge filled to the bottom).
    let hill = |cv: &mut Cv, col: u32, base: f32, amp: f32, ph: f32| {
        cv.fill_below(H, col, 1.0, move |x| base + (x * 0.021 + ph).sin() * amp);
    };
    hill(cv, 0x2f6c3c, 126.0, 10.0, 1.2);
    // A windmill on the far hill, turning easy.
    cv.rect(84, 106, 5, 16, 0x4a3a26, 1.0);
    cv.rect(83, 104, 7, 3, 0x5a4a32, 1.0);
    for v in 0..4 {
        let a = lt * 0.02 + v as f32 * std::f32::consts::FRAC_PI_2;
        cv.line(86.5, 105.0, 86.5 + a.cos() * 10.0, 105.0 + a.sin() * 10.0, 0xd8d0b8, 1.0, 1);
    }
    hill(cv, 0x28602f, 150.0, 12.0, 3.6);
    // Sheep on the middle hill (two white dots that occasionally wander a pixel).
    let l6 = ((lt as i32) >> 6) & 1;
    let l7 = ((lt as i32) >> 7) & 1;
    cv.rect(236 + l6, 146, 3, 2, 0xe8e6da, 1.0);
    cv.rect(252, 143 + l7, 3, 2, 0xe8e6da, 1.0);
    hill(cv, 0x215426, 176.0, 9.0, 5.1);
    // A village in the near valley: cottages with warm windows, thin chimney smoke.
    for c in [[140, 186], [176, 192], [214, 188], [252, 194]] {
        cv.rect(c[0], c[1], 14, 9, 0x5a4630, 1.0); // walls
        cv.rect(c[0] - 1, c[1] - 3, 16, 4, 0x7a3a2a, 1.0); // roof
        cv.rect(c[0] + 3, c[1] + 3, 2, 3, 0xffd98a, 1.0); // lit windows
        cv.rect(c[0] + 9, c[1] + 3, 2, 3, 0xffd98a, 1.0);
        for s in 0..3 {
            let sx = c[0] + 12 + (((lt + s as f32 * 9.0) * 0.1).sin() * 1.5).round() as i32;
            let sy = c[1] - 8 - s * 4 - (((lt as i32) >> 2) % 4);
            cv.rect(sx, sy, 1, 2, 0xdcdcdc, 0.25); // smoke
        }
    }
    fade(cv, (if lt < 22.0 { 1.0 - lt / 22.0 } else { 0.0 }).max(if lt > 248.0 { (lt - 248.0) / 12.0 } else { 0.0 }));
}

// SCENE 1 — the Sundering: the heart hangs in the night, the silhouette raises the
// blade, and the WRIFTHEART shatters into TEN shards (never more).
pub fn sky(cv: &mut Cv, st: &CsInner, lt: f32) {
    let cy = H as f32 / 2.0 - 12.0;
    let shattered = lt >= 140.0;
    cv.grad_v(0, 0, W, H, &[(0.0, 0x070914, 1.0), (0.6, 0x0a1228, 1.0), (1.0, 0x05060d, 1.0)]);
    stars(cv, st, if shattered { 0.55 } else { 1.0 });
    if !shattered {
        // The heart BEATS... and at lt=110, as the blade hangs above it, it STOPS.
        let beat = (lt * 0.16).sin().max(0.0);
        let pulse = if lt < 110.0 { 1.0 + beat * beat * beat * 0.09 } else { 1.0 };
        let r = 28.0 * pulse;
        cv.radial(CX, cy, 4.0, r + 44.0, 0xbe6eff, if lt < 110.0 { 0.5 } else { 0.32 });
        gem(cv, CX, cy, r, 0xb060f0, 1.0);
        if lt > 110.0 {
            let a = ((lt - 110.0) / 20.0).min(1.0);
            cv.line(CX - 5.0, cy - r + 5.0, CX + 3.0, cy - 2.0, 0xffffff, a, 1); // a crack creeps in
            cv.line(CX + 3.0, cy - 2.0, CX - 4.0, cy + r - 5.0, 0xffffff, a, 1);
        }
        if lt > 130.0 {
            cv.rect((CX - 2.0) as i32, (cy - r) as i32, 4, (r * 2.0) as i32, 0xffffff, ((lt - 130.0) / 6.0).min(1.0)); // the seam blazes
        }
        // A low black ridge, and the lone figure crossing it toward the heart.
        let (wf, hf) = (W as f32, H as f32);
        cv.poly(&[(0.0, hf), (wf * 0.3, hf - 24.0), (wf * 0.55, hf - 32.0), (wf * 0.78, hf - 22.0), (wf, hf)], 0x04050c, 1.0);
        if lt > 20.0 {
            let fx = (CX + 2.0).max(wf - 24.0 - (lt - 20.0) * 2.2).round();
            let walking = fx > CX + 2.0;
            let fy = (hf - 36.0) + if walking { (((lt as i32) >> 3) & 1) as f32 } else { 0.0 };
            cv.rect(fx as i32 - 2, fy as i32 - 9, 5, 9, 0x05060e, 1.0); // cloaked body
            cv.rect(fx as i32 - 1, fy as i32 - 12, 3, 3, 0x05060e, 1.0); // hooded head
            if lt > 100.0 {
                // The blade rises, slowly...
                let raise = ((lt - 100.0) / 18.0).min(1.0);
                let col = if lt > 130.0 { 0xffffff } else { 0xc8d2e6 };
                cv.line(fx + 2.0, fy - 8.0, fx + 2.0 + 6.0 * raise, fy - 8.0 - 11.0 * raise, col, if lt > 130.0 { 1.0 } else { 0.8 }, 1);
            }
        }
    } else {
        // THE SHATTER: the TEN shards fly out (js cutscene.shards — one per biome).
        let f = lt - 140.0;
        for (i, &(col, ang, spd)) in st.shards.iter().enumerate() {
            let d = f * spd * 2.4;
            let (x, y) = (CX + ang.cos() * d, cy + ang.sin() * d + d * d * 0.0008);
            if !(-12.0..W as f32 + 12.0).contains(&x) || y > H as f32 + 12.0 {
                continue;
            }
            gem(cv, x, y, 5.0 + (i % 3) as f32, col, (1.0 - f / 80.0).max(0.0));
        }
        if f < 22.0 {
            cv.rect(0, 0, W, H, 0xdecaff, 1.0 - f / 22.0); // white-out
        }
        cv.radial(CX, cy, 2.0, 52.0, 0x963cd2, 0.9 * (0.7 - f / 90.0).max(0.0)); // the lingering wound
    }
    fade(cv, (if lt < 20.0 { 1.0 - lt / 20.0 } else { 0.0 }).max(if lt > 228.0 { (lt - 228.0) / 12.0 } else { 0.0 }));
}

// SCENE 2 — ten shards fall to ten far places; where each sinks, the land turns strange.
pub fn scatter(cv: &mut Cv, st: &CsInner, lt: f32) {
    cv.grad_v(0, 0, W, H, &[(0.0, 0x070914, 1.0), (0.6, 0x0a1228, 1.0), (1.0, 0x05060d, 1.0)]);
    stars(cv, st, 0.5);
    // The wound where the heart hung, still glowing faintly.
    cv.radial(CX, 30.0, 2.0, 40.0, 0x963cd2, 0.5);
    // A dark broken horizon (the js 12px-stepped hash ridge).
    cv.fill_below(H, 0x0a0d13, 1.0, |x| {
        let step = (x as i32 / 12) * 12;
        152.0 + ((step * 7919) % 13) as f32
    });
    // Ten shards, falling staggered from the wound; each landing blooms wrong-coloured.
    for (i, &(col, _, _)) in st.shards.iter().enumerate() {
        let fi = i as f32;
        let tx = 20.0 + fi * ((W as f32 - 40.0) / 9.0);
        let ty = 156.0 + ((i * 37) % 14) as f32;
        let p = ((lt - 8.0 - fi * 6.0) / 58.0).clamp(0.0, 1.0);
        if p <= 0.0 {
            continue;
        }
        if p < 1.0 {
            // In flight: an arc out from the wound, trailing.
            let (x, y) = (CX + (tx - CX) * p, 30.0 + (ty - 30.0) * (p * p));
            gem(cv, x, y, 4.0, col, 1.0);
            gem(cv, x - (tx - CX) * 0.05, y - 7.0, 2.5, col, 0.45);
        } else {
            // Landed: the land drinks it and glows where it should not.
            cv.radial(tx, ty, 1.0, 26.0, col, 0.42 + 0.1 * (lt * 0.1 + fi * 2.0).sin());
            cv.rect(tx as i32, ty as i32 - 1, 1, 2, col, 1.0); // the buried glint
        }
    }
    fade(cv, (if lt < 18.0 { 1.0 - lt / 18.0 } else { 0.0 }).max(if lt > 208.0 { (lt - 208.0) / 12.0 } else { 0.0 }));
}

// SCENE 3 — TONIGHT: grey singers ring a hilltop above a sleeping village and call the ember down.
pub fn choir(cv: &mut Cv, st: &CsInner, lt: f32) {
    let hf = H as f32;
    cv.grad_v(0, 0, W, H, &[(0.0, 0x060812, 1.0), (1.0, 0x0a0e1a, 1.0)]);
    stars(cv, st, 0.7);
    // The sky bruising open above the hill as the hymn works.
    let bruise = (lt / 170.0).min(1.0);
    cv.radial(CX, 34.0, 4.0, 14.0 + bruise * 44.0, 0x8c3cc8, 0.12 + 0.4 * bruise);
    if lt > 120.0 {
        let ca = ((lt - 120.0) / 40.0).min(1.0); // a seam opens
        cv.line(CX - 9.0, 30.0, CX - 2.0, 35.0, 0xe2c4ff, ca, 1);
        cv.line(CX - 2.0, 35.0, CX + 4.0, 31.0, 0xe2c4ff, ca, 1);
        cv.line(CX + 4.0, 31.0, CX + 10.0, 36.0, 0xe2c4ff, ca, 1);
    }
    // The sleeping village along the bottom: dark roofs, warm windows.
    cv.rect(0, H - 30, W, 30, 0x070a10, 1.0);
    for c in [[30, H - 26], [74, H - 24], [252, H - 26], [304, H - 23], [344, H - 25]] {
        cv.rect(c[0], c[1], 16, 12, 0x10141c, 1.0);
        cv.rect(c[0] - 1, c[1] - 3, 18, 4, 0x171b24, 1.0);
        if (c[0] + ((lt as i32) >> 5)) % 3 != 0 {
            cv.rect(c[0] + 4, c[1] + 4, 2, 3, 0xffcf8a, 1.0); // a few windows still lit
        }
    }
    // The hill (the js quadratic from (40,H-28) over (cx,66)), and the ring of singers.
    cv.fill_below(H - 27, 0x0b0f17, 1.0, move |x| {
        let t = ((x - 40.0) / (W as f32 - 80.0)).clamp(0.0, 1.0);
        let u = 1.0 - t;
        u * u * (hf - 28.0) + 2.0 * u * t * 66.0 + t * t * (hf - 28.0)
    });
    for k in 0..6 {
        let a = (k as f32 / 6.0) * std::f32::consts::TAU;
        let fx = (CX + a.cos() * 26.0).round() as i32;
        let fy = (104.0 + a.sin() * 8.0).round() as i32;
        cv.rect(fx - 2, fy - 8, 5, 8, 0x767b84, 1.0); // grey robe
        cv.rect(fx - 1, fy - 11, 3, 4, 0x63676f, 1.0); // hood, tilted to the sky
    }
    // The hymn: faint motes rising from the ring toward the bruise.
    for m in 0..9 {
        let my = 98.0 - ((lt * 0.8 + m as f32 * 14.0) % 62.0);
        let a = ((my - 36.0) / 62.0).max(0.0) * 0.7;
        cv.rect(CX as i32 - 20 + ((m * 47) % 40), my.round() as i32, 1, 1, 0xcfd4e0, a);
    }
    // The ember, called down: a bolt from the seam to the village, then white.
    if lt > 208.0 {
        let p = ((lt - 208.0) / 9.0).min(1.0);
        let (ex, ey) = (CX + 34.0 * p, 34.0 + (hf - 34.0 - 34.0) * p);
        cv.line(CX, 34.0, ex, ey, 0xbe78ff, 0.6, 3);
        cv.line(CX, 34.0, ex, ey, 0xfff2dc, 1.0, 1);
    }
    if lt > 218.0 {
        cv.rect(0, 0, W, H, 0xffeed6, ((lt - 218.0) / 10.0).min(1.0)); // white-out into the boom
    }
    fade(cv, if lt < 18.0 { 1.0 - lt / 18.0 } else { 0.0 });
}

// SCENE 4 — inside the hero's cottage: asleep, jolted awake by the boom, then up and OUT.
// (The js draws the real Interiors.house() room; this paints the same cottage —
// plank floor, timber walls, the bed at tile (2,2), rug, table, the door at the
// bottom — a faithful composition from the same colours.)
pub fn interior(cv: &mut Cv, hero: &Image, lt: f32) {
    cv.rect(0, 0, W, H, 0x04050a, 1.0);
    let (ox, oy, iw, ih) = (40, 4, 304, 176);
    // Plank floor + timber wall band.
    for r in 0..(ih / 16) {
        let col = if r % 2 == 0 { 0x8a6a42 } else { 0x7f6039 };
        cv.rect(ox, oy + r * 16, iw, 16, col, 1.0);
        cv.rect(ox, oy + r * 16, iw, 1, 0x6a4e30, 1.0); // board seams
    }
    for c in 0..(iw / 16) {
        cv.rect(ox + c * 16, oy, 1, ih, 0x6a4e30, 0.35);
    }
    cv.rect(ox, oy, iw, 26, 0x5a4630, 1.0); // the back wall
    cv.rect(ox, oy + 24, iw, 2, 0x3a2c1c, 1.0);
    for c in 0..(iw / 24) {
        cv.rect(ox + c * 24, oy, 1, 24, 0x4a3826, 1.0); // wall timbers
    }
    // A window on the back wall (moonlight), and the door gap at the bottom.
    cv.rect(ox + 200, oy + 4, 22, 18, 0x2a3450, 1.0);
    cv.rect(ox + 210, oy + 4, 2, 18, 0x141824, 1.0);
    cv.rect(ox + 200, oy + 12, 22, 2, 0x141824, 1.0);
    cv.rect(ox + iw / 2 - 12, oy + ih - 6, 24, 6, 0x2a1c10, 1.0); // the way out
    // A rug, a table with a stool, the hearth.
    cv.rect(ox + 120, oy + 90, 44, 26, 0xa04838, 1.0);
    cv.rect(ox + 122, oy + 92, 40, 22, 0x7a3428, 1.0);
    cv.rect(ox + 220, oy + 84, 30, 18, 0x6a4a2c, 1.0); // table
    cv.rect(ox + 222, oy + 86, 26, 3, 0x8a6a42, 1.0);
    cv.rect(ox + 256, oy + 92, 10, 8, 0x5a3e24, 1.0); // stool
    cv.rect(ox + iw - 24, oy + 40, 18, 26, 0x3a3230, 1.0); // hearth
    // The bed at tile (2,2) — frame, blanket, pillow (js bed anchor).
    let (bx, by) = (ox + 2 * 16, oy + 2 * 16);
    cv.rect(bx, by, 24, 38, 0x6a4a2c, 1.0);
    cv.rect(bx + 2, by + 2, 20, 10, 0xe8e0d0, 1.0); // pillow
    cv.rect(bx + 2, by + 12, 20, 24, 0xa03030, 1.0); // blanket
    cv.rect(bx + 2, by + 18, 20, 2, 0xc05050, 1.0); // the fold
    // Night dim over the whole room.
    cv.rect(ox, oy, iw, ih, 0x080a1a, 0.5);
    // Dust shaken loose from the rafters (after the boom).
    if lt > 50.0 && lt < 130.0 {
        for d in 0..12 {
            if (d + ((lt as i32) >> 4)) % 3 == 0 {
                continue;
            }
            let dx0 = ox + 10 + ((d * 41) % (iw - 20));
            let dy0 = oy as f32 + 6.0 + ((lt - 50.0) * (1.1 + (d % 3) as f32 * 0.5) + d as f32 * 13.0) % (ih as f32 - 14.0);
            cv.rect(dx0, dy0.round() as i32, 1, 2, 0xba9e74, 0.65);
        }
    }
    // Firelight leaking in, growing + flickering.
    let glow = ((lt - 44.0) / 130.0).clamp(0.0, 0.65) * (0.7 + 0.3 * (lt * 0.5).sin());
    if glow > 0.0 {
        cv.grad_v(ox, oy, iw, ih, &[(0.0, 0xff7828, glow * 0.3), (1.0, 0xff5a1e, glow)]);
    }
    // The hero: asleep sideways -> bolts upright -> up, and OUT the door.
    let (bed_cx, bed_cy) = (ox + 2 * 16 + 22, oy + 2 * 16 + 12);
    if lt <= 58.0 {
        cv.blit(hero, bed_cx - 8, bed_cy - 8, false, true, 1.0, 1); // asleep on the bed
    } else if lt < 116.0 {
        cv.blit(hero, bed_cx - 8, bed_cy - 11, false, false, 1.0, 1); // bolts upright
    } else {
        let wy = bed_cy as f32 - 6.0 + (20.0f32).min((lt - 116.0) * 0.5);
        cv.blit(hero, bed_cx - 8, wy.round() as i32, false, false, 1.0, 1); // heads for the door
    }
    fade(cv, (if lt < 20.0 { 1.0 - lt / 20.0 } else { 0.0 }).max(if lt > 180.0 { (lt - 180.0) / 20.0 } else { 0.0 }));
}

/// csFlame — a flickering tongue of flame anchored at (x,y) (two nested quadratic
/// tongues + the bright core, painted as per-row bezier widths).
pub fn flame(cv: &mut Cv, x: f32, y: f32, s: f32, ph: f32) {
    let h = (10.0 + ph.sin() * 3.0) * s;
    let w = (5.0 + (ph * 1.4).sin() * 2.0) * s;
    let tongue = |cv: &mut Cv, hh: f32, ww: f32, col: u32| {
        let rows = hh.round() as i32;
        for j in 0..rows {
            let u = j as f32 / rows.max(1) as f32; // 0 at the base, 1 at the tip
            let hw = 2.0 * u * (1.0 - u) * ww; // the quadratic tongue's width
            cv.rect((x - hw).round() as i32, (y - u * hh).round() as i32, (hw * 2.0).round().max(1.0) as i32, 1, col, 1.0);
        }
    };
    tongue(cv, h, w, 0xd83010);
    tongue(cv, h * 0.72, w * 0.6, 0xfc7030);
    cv.rect((x - 1.0).round() as i32, (y - h * 0.5).round() as i32, 2, (h * 0.35).round().max(1.0) as i32, 0xfcd040, 1.0);
}

// (The js csRunner — a tiny abstract panicking figure — was ported and RETIRED:
// on the dark blazing ground its rects read as blobs; the town now runs REAL
// seeded villager sprites instead. Baz: "the people aren't people".)

/// One building's blit + char wash + roof flames (the js town loop body).
pub fn burning_building(cv: &mut Cv, img: Option<&Image>, x: i32, y: i32, lt: f32) {
    if let Some(img) = img {
        cv.blit(img, x - 16, y - 32, false, false, 1.0, 1);
    }
    cv.rect(x - 16, y - 32, 48, 48, 0x1a0e08, 0.2); // char wash
    for f in 0..4 {
        flame(cv, (x - 12 + f * 8) as f32, (y - 18) as f32, 0.7 + (f & 1) as f32 * 0.45, lt * 0.7 + x as f32 + f as f32 * 5.0);
    }
}

/// Draw a pre-baked text string (st.texts) into the canvas at a global alpha/scale.
#[allow(clippy::too_many_arguments)] // a text blit is coordinates all the way down
pub fn text(cv: &mut Cv, images: &Assets<Image>, st: &CsInner, s: &str, x: i32, y: i32, a: f32, scale: i32) {
    if let Some((h, _)) = st.texts.get(s)
        && let Some(img) = images.get(h)
    {
        cv.blit(img, x, y, false, false, a, scale);
    }
}

/// Centred text (js centerText / Font.drawCentered).
#[allow(clippy::too_many_arguments)] // a text blit is coordinates all the way down
pub fn text_c(cv: &mut Cv, images: &Assets<Image>, st: &CsInner, s: &str, cx: i32, y: i32, a: f32, scale: i32) {
    let w = st.texts.get(s).map_or(0, |(_, w)| *w) * scale;
    text(cv, images, st, s, cx - w / 2, y, a, scale);
}

// SCENE 5 — EMBERFALL ABLAZE: the village on fire, villagers fleeing and screaming,
// the hero spilling out of home — running, stopping ONCE to look back at it burning.
pub fn town(cv: &mut Cv, images: &Assets<Image>, st: &CsInner, hero: Option<&Image>, lt: f32) {
    let (wf, hf) = (W as f32, H as f32);
    cv.rect(0, 0, W, H, 0x0e1c12, 1.0);
    // The grass checker.
    let mut yy = 0;
    while yy < H {
        let mut xx = ((yy / 8) & 1) * 8;
        while xx < W {
            cv.rect(xx, yy, 8, 8, 0x000000, 0.07);
            xx += 16;
        }
        yy += 8;
    }
    cv.rect(0, 126, W, 20, 0x3a2c1c, 1.0); // the dirt crossroads
    cv.rect(170, 0, 18, H, 0x2e2216, 1.0);
    // On the rise above it all, one grey-robed figure stands PERFECTLY STILL while
    // everyone runs. No caption — the miller's letter readers come back for it.
    cv.rect(346, 26, 6, 10, 0x8a8f98, 1.0);
    cv.rect(347, 22, 4, 5, 0x767b84, 1.0);
    cv.rect(348, 24, 2, 2, 0x101014, 1.0);
    // The well + torches, then every building back-to-front, each ablaze.
    if let Some(w) = images.get(&st.well) {
        cv.blit(w, 166, 96, false, false, 1.0, 1);
    }
    if let Some(t) = images.get(&st.torch[(((lt as i32) >> 3) & 1) as usize]) {
        cv.blit(t, 132, 104, false, false, 1.0, 1);
        cv.blit(t, 206, 104, false, false, 1.0, 1);
    }
    let mut order: Vec<usize> = (0..st.fronts.len()).collect();
    order.sort_by_key(|&i| st.fronts[i].2);
    for i in order {
        let (_, x, y, ref h) = st.fronts[i];
        burning_building(cv, h.as_ref().and_then(|h| images.get(h)), x, y, lt);
    }
    // Embers riding the updraft.
    for e in 0..40 {
        let ef = e as f32;
        let ex = (ef * 47.0 + lt * 1.4) % wf;
        let ey = hf - ((ef * 53.0 + lt * 2.1) % (hf + 20.0));
        cv.rect(ex.round() as i32, ey.round() as i32, 1, 1 + (e & 1), 0xfc9030, 0.5 * (ey / hf).max(0.0));
    }
    // The fleeing villagers + their screams (the js word bubbles, borders and all).
    // REAL seeded villager sprites at a frantic gait — the js's abstract runners read
    // as blobs on this dark blazing ground (Baz: "the people aren't people").
    for (i, n) in st.npcs.iter().enumerate() {
        let x = ((n.x + n.dir * lt * n.spd) % (wf + 40.0) + (wf + 40.0)) % (wf + 40.0) - 20.0;
        if let Some(set) = st.folk.get(i % st.folk.len().max(1)) {
            let facing = if n.dir > 0.0 { 2 } else { 3 }; // right / left
            let fi = (((lt + n.ph) / 4.0) as usize) % 4; // double-time panic gait
            let bob = if fi & 1 == 1 { 1 } else { 0 };
            if let Some(img) = images.get(&set[facing][fi]) {
                cv.blit(img, x.round() as i32, n.base_y.round() as i32 - 12 - bob, false, false, 1.0, 1);
            }
        }
        if n.always || (((lt as i32) >> 5) % 2) == 0 {
            let tw = st.texts.get(n.line).map_or(20, |(_, w)| *w);
            let bw = tw + 6;
            let bbx = ((x + 8.0 - bw as f32 / 2.0).round() as i32).clamp(2, W - bw - 2);
            let bby = n.base_y.round() as i32 - 16;
            cv.rect(bbx, bby, bw, 9, 0x000000, 0.78);
            for (sx, sy, sw2, sh2) in [(bbx, bby, bw, 1), (bbx, bby + 8, bw, 1), (bbx, bby, 1, 9), (bbx + bw - 1, bby, 1, 9)] {
                cv.rect(sx, sy, sw2, sh2, 0xe0a0a0, 1.0); // the bubble's border
            }
            text(cv, images, st, n.line, bbx + 3, bby + 2, 1.0, 1);
        }
    }
    // The hero spills out of home, runs, stops ONCE to look back at it burning, then is gone.
    if lt > 36.0
        && let Some(hero) = hero
    {
        let rt0 = (lt - 64.0).max(0.0);
        const HOLD_AT: f32 = 95.0;
        const HOLD: f32 = 46.0;
        let looking = (HOLD_AT..HOLD_AT + HOLD).contains(&rt0);
        let run_t = if rt0 < HOLD_AT {
            rt0
        } else if looking {
            HOLD_AT
        } else {
            rt0 - HOLD
        };
        let hx = 206.0 - run_t * 1.7;
        let hy = 178.0 - if looking { 0.0 } else { (run_t * 0.4).sin().abs() * 2.0 };
        if hx > -16.0 {
            // Mirrored = running away; unmirrored = turned back toward home.
            cv.blit(hero, hx.round() as i32, hy.round() as i32, !looking, false, 1.0, 1);
        }
    }
    cv.rect(0, 0, W, H, 0xb4280a, 0.2); // the fire wash
    cv.vignette(wf / 2.0, hf / 2.0, 40.0, 210.0, 0x280600, 0.6);
    fade(cv, (if lt < 20.0 { 1.0 - lt / 20.0 } else { 0.0 }).max(if lt > 280.0 { (lt - 280.0) / 30.0 } else { 0.0 }));
}
