use image::{Rgba, RgbaImage};
use std::collections::VecDeque;

const TARGET_W: u32 = 200;
const MAX_COLS: usize = 22;

fn main() {
    let src = image::open(std::env::args().nth(1).unwrap()).unwrap().to_rgba8();
    let (w, h) = src.dimensions();

    // PIXEL MODE: a small input IS the authored pixel art (art/logo_pixel.png,
    // hand-editable) — skip background removal/downscale/quantize and emit the
    // char grid straight from its pixels.
    if w <= 400 {
        let mut pal: Vec<[i32; 3]> = Vec::new();
        for p in src.pixels() {
            if p[3] >= 128 {
                let c = [p[0] as i32, p[1] as i32, p[2] as i32];
                if !pal.contains(&c) {
                    pal.push(c);
                }
            }
        }
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars().collect();
        assert!(pal.len() <= chars.len(), "too many colours ({}) — flatten some in the editor", pal.len());
        let mut rs = String::from("//! logo.rs — the WRIFTHEART word-art wordmark (art/logo_pixel.png).\n//! GENERATED — edit the png and regenerate with tools/logoconv, not by hand.\n\n");
        rs.push_str("pub const LOGO_PAL: &[(char, u32)] = &[\n");
        for (i, c) in pal.iter().enumerate() {
            rs.push_str(&format!("    ('{}', 0x{:02x}{:02x}{:02x}),\n", chars[i], c[0], c[1], c[2]));
        }
        rs.push_str("];\n\npub const LOGO: &[&str] = &[\n");
        for y in 0..h {
            let mut row = String::new();
            for x in 0..w {
                let p = src.get_pixel(x, y);
                if p[3] < 128 {
                    row.push('.');
                } else {
                    let idx = pal.iter().position(|c| c[0] as u8 == p[0] && c[1] as u8 == p[1] && c[2] as u8 == p[2]).unwrap();
                    row.push(chars[idx]);
                }
            }
            rs.push_str(&format!("    \"{}\",\n", row));
        }
        rs.push_str("];\n");
        std::fs::write("logo.rs", rs).unwrap();
        eprintln!("pixel mode: {}x{}, {} colours -> logo.rs", w, h, pal.len());
        return;
    }

    // Background: BFS from the border, expanding across the smooth gradient
    // (neighbour-to-neighbour step small). Letter outlines are sharp -> fill stops.
    let mut bg = vec![false; (w * h) as usize];
    let mut q = VecDeque::new();
    for x in 0..w {
        for y in [0, h - 1] {
            bg[(y * w + x) as usize] = true;
            q.push_back((x, y));
        }
    }
    for y in 0..h {
        for x in [0, w - 1] {
            bg[(y * w + x) as usize] = true;
            q.push_back((x, y));
        }
    }
    let dist = |a: &Rgba<u8>, b: &Rgba<u8>| -> u32 {
        (0..3).map(|i| (a[i] as i32 - b[i] as i32).unsigned_abs()).sum()
    };
    while let Some((x, y)) = q.pop_front() {
        let c = src.get_pixel(x, y);
        for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 { continue; }
            let (nx, ny) = (nx as u32, ny as u32);
            let i = (ny * w + nx) as usize;
            if bg[i] { continue; }
            if dist(c, src.get_pixel(nx, ny)) < 24 {
                bg[i] = true;
                q.push_back((nx, ny));
            }
        }
    }

    // DEBUG: how much purple survives the background fill?
    let (mut purple_all, mut purple_kept) = (0u32, 0u32);
    for y in 0..h {
        for x in 0..w {
            let p = src.get_pixel(x, y);
            if p[0] > p[1].saturating_add(20) && p[2] > p[1].saturating_add(30) {
                purple_all += 1;
                if !bg[(y * w + x) as usize] { purple_kept += 1; }
            }
        }
    }
    eprintln!("purple px: {} total, {} kept by mask", purple_all, purple_kept);

    // NOTE: pockets of background sealed inside touching letterforms (W-R, I-F,
    // around the sword) survive the edge flood — every auto-detector tried either
    // missed them or nibbled the letters (they share the parchment's colours).
    // They are cleaned BY HAND in art/logo_pixel.png (Aseprite), which is the
    // pixel master; regenerating from the painting reintroduces them.

    // Content bbox
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0u32, 0u32);
    for y in 0..h {
        for x in 0..w {
            if !bg[(y * w + x) as usize] {
                x0 = x0.min(x); y0 = y0.min(y); x1 = x1.max(x); y1 = y1.max(y);
            }
        }
    }
    let (cw, ch) = (x1 - x0 + 1, y1 - y0 + 1);
    let th = (ch as f32 * TARGET_W as f32 / cw as f32).round() as u32;
    eprintln!("content {}x{} -> {}x{}", cw, ch, TARGET_W, th);

    // Box-downscale averaging only NON-BG pixels; alpha from coverage.
    let mut out = RgbaImage::new(TARGET_W, th);
    for oy in 0..th {
        for ox in 0..TARGET_W {
            let sx0 = x0 + (ox * cw) / TARGET_W;
            let sx1 = x0 + (((ox + 1) * cw) / TARGET_W).max(ox * cw / TARGET_W + 1);
            let sy0 = y0 + (oy * ch) / th;
            let sy1 = y0 + (((oy + 1) * ch) / th).max(oy * ch / th + 1);
            // Dominant-colour average: find the cell's most common 4-bit bucket,
            // then average only pixels near it — outlines stop muddying the fills.
            let mut px_list: Vec<[i32; 3]> = Vec::new();
            let mut tot = 0u64;
            for sy in sy0..sy1.min(y1 + 1) {
                for sx in sx0..sx1.min(x1 + 1) {
                    tot += 1;
                    if !bg[(sy * w + sx) as usize] {
                        let p = src.get_pixel(sx, sy);
                        px_list.push([p[0] as i32, p[1] as i32, p[2] as i32]);
                    }
                }
            }
            if tot > 0 && px_list.len() as u64 * 2 > tot {
                let mut buckets = std::collections::HashMap::new();
                for p in &px_list {
                    *buckets.entry((p[0] >> 4, p[1] >> 4, p[2] >> 4)).or_insert(0u32) += 1;
                }
                // Rank buckets by count WEIGHTED toward saturated/bright colour, so
                // a leaf's violet fill outranks its dark outline in a small cell.
                let (&(br, bgc, bb), _) = buckets
                    .iter()
                    .max_by_key(|((r, g, b), n)| {
                        let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
                        **n * (12 + (mx - mn) as u32 * 3 + *mx as u32)
                    })
                    .unwrap();
                let dom = [br * 16 + 8, bgc * 16 + 8, bb * 16 + 8];
                let near: Vec<_> = px_list.iter().filter(|p| (0..3).map(|i| (p[i] - dom[i]).abs()).sum::<i32>() < 90).collect();
                let n = near.len().max(1) as i32;
                let (r, g, b) = near.iter().fold((0, 0, 0), |a, p| (a.0 + p[0], a.1 + p[1], a.2 + p[2]));
                // gentle saturation lift to counter the averaging
                let (mut r, mut g, mut b) = ((r / n) as f32, (g / n) as f32, (b / n) as f32);
                let l = (r + g + b) / 3.0;
                r = (l + (r - l) * 1.2).clamp(0.0, 255.0);
                g = (l + (g - l) * 1.2).clamp(0.0, 255.0);
                b = (l + (b - l) * 1.2).clamp(0.0, 255.0);
                out.put_pixel(ox, oy, Rgba([r as u8, g as u8, b as u8, 255]));
            }
        }
    }

    let mut np = 0;
    let mut samples = Vec::new();
    for p in out.pixels() {
        if p[3] == 255 && p[0] > p[1].saturating_add(15) && p[2] > p[1].saturating_add(20) {
            np += 1;
            if samples.len() < 6 { samples.push((p[0], p[1], p[2])); }
        }
    }
    eprintln!("purple cells pre-quantize: {} samples {:?}", np, samples);

    // Popularity quantize to MAX_COLS colours (5-bit buckets), then nearest-map.
    let mut counts = std::collections::HashMap::new();
    for p in out.pixels() {
        if p[3] == 255 {
            *counts.entry(((p[0] >> 3) as u32, (p[1] >> 3) as u32, (p[2] >> 3) as u32)).or_insert(0u32) += 1;
        }
    }
    let mut pop: Vec<_> = counts.into_iter().collect();
    // Saturation-weighted popularity: small vivid regions (the violet heart and
    // leaves) must claim palette slots against the sea of golds and greys.
    pop.sort_by_key(|((r, g, b), n)| {
        let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
        std::cmp::Reverse(*n * (12 + (mx - mn) * 6 + mx))
    });
    let pop_all = pop.clone();
    let mut pal: Vec<[i32; 3]> = Vec::new();
    for ((r, g, b), _) in pop {
        let c = [(r * 8 + 4) as i32, (g * 8 + 4) as i32, (b * 8 + 4) as i32];
        // keep palette entries apart so近-dupes merge
        if pal.iter().all(|p| (0..3).map(|i| (p[i] - c[i]).abs()).sum::<i32>() > 30) {
            pal.push(c);
            if pal.len() == MAX_COLS { break; }
        }
    }
    eprintln!("palette: {:?}", pal.iter().map(|c| format!("{:02x}{:02x}{:02x}", c[0], c[1], c[2])).collect::<Vec<_>>());

    // HUE RESERVATION: a hue family holding >=0.5% of the visible art gets a
    // palette seat even when outshone (the violet heart vs an ocean of gold).
    let sector = |c: &[i32; 3]| -> usize {
        let (r, g, b) = (c[0], c[1], c[2]);
        if (r - g).abs() < 18 && (g - b).abs() < 18 && (r - b).abs() < 18 { return 6; } // grey/metal
        match () {
            _ if r >= g && g >= b => 0,
            _ if g >= r && r >= b => 1,
            _ if g >= b && b >= r => 2,
            _ if b >= g && g >= r => 3,
            _ if b >= r && r >= g => 4, // violet
            _ => 5,
        }
    };
    let mut sector_px = [0u64; 7];
    let mut total_px = 0u64;
    for p in out.pixels() {
        if p[3] == 255 {
            total_px += 1;
            sector_px[sector(&[p[0] as i32, p[1] as i32, p[2] as i32])] += 1;
        }
    }
    for sec in 0..7 {
        if sector_px[sec] * 200 >= total_px && !pal.iter().any(|c| sector(c) == sec) {
            if let Some(((r, g, b), _)) = pop_all.iter().find(|((r, g, b), _)| {
                sector(&[(*r * 8 + 4) as i32, (*g * 8 + 4) as i32, (*b * 8 + 4) as i32]) == sec
            }) {
                pal.push([(*r * 8 + 4) as i32, (*g * 8 + 4) as i32, (*b * 8 + 4) as i32]);
                eprintln!("hue seat granted: sector {}", sec);
            }
        }
    }

    for p in out.pixels_mut() {
        if p[3] == 255 {
            let best = pal.iter().min_by_key(|c| (0..3).map(|i| (c[i] - p[i as usize] as i32).abs()).sum::<i32>()).unwrap();
            *p = Rgba([best[0] as u8, best[1] as u8, best[2] as u8, 255]);
        }
    }

    out.save("logo_small.png").unwrap();
    // 4x nearest preview
    let mut prev = RgbaImage::new(TARGET_W * 4, th * 4);
    for y in 0..th * 4 { for x in 0..TARGET_W * 4 {
        let p = *out.get_pixel(x / 4, y / 4);
        prev.put_pixel(x, y, if p[3] == 0 { Rgba([40, 44, 52, 255]) } else { p });
    }}
    prev.save("logo_preview.png").unwrap();

    // Emit the game-native char grid.
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
    let mut rs = String::from("//! logo.rs — the WRIFTHEART word-art wordmark, GENERATED from WordArtLogo.png\n//! (box-downscaled to game px, popularity-quantized; regenerate via the logoconv\n//! scratch tool rather than editing by hand).\n\n");
    rs.push_str(&format!("pub const LOGO_PAL: &[(char, u32)] = &[\n"));
    for (i, c) in pal.iter().enumerate() {
        rs.push_str(&format!("    ('{}', 0x{:02x}{:02x}{:02x}),\n", chars[i], c[0], c[1], c[2]));
    }
    rs.push_str("];\n\npub const LOGO: &[&str] = &[\n");
    for y in 0..th {
        let mut row = String::new();
        for x in 0..TARGET_W {
            let p = out.get_pixel(x, y);
            if p[3] == 0 { row.push('.'); } else {
                let idx = pal.iter().position(|c| c[0] as u8 == p[0] && c[1] as u8 == p[1] && c[2] as u8 == p[2]).unwrap();
                row.push(chars[idx]);
            }
        }
        rs.push_str(&format!("    \"{}\",\n", row));
    }
    rs.push_str("];\n");
    std::fs::write("logo.rs", rs).unwrap();
    eprintln!("palette {} colours", pal.len());
}
