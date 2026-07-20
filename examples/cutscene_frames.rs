//! Render opening-cutscene frames HEADLESSLY to PPM files — the WRIFT_SHOT window
//! capture blacks out under macOS occlusion whenever another app holds focus, but
//! the cutscene is a pure CPU canvas, so the painters can run without a window.
//!
//!   cargo run --release --example cutscene_frames [t t t ...]
//!
//! Writes /tmp/cs_<t>.ppm for each requested cutscene frame (defaults below).
//! Convert with `sips -s format png /tmp/cs_150.ppm --out /tmp/cs_150.png`.

use bevy::asset::{Assets, RenderAssetUsages};
use bevy::image::Image;
use bevy::platform::collections::HashMap;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use wriftheart::app::cinematic::{paint::Cv, scenes, CsInner, Npc, H, W};

fn main() {
    let mut images: Assets<Image> = Assets::default();
    let prop_art = wriftheart::actors::props::PropArt::build(&mut images);

    // Deterministic 70-star field + the TEN shards + the six fleeing villagers.
    let mut seed = 0x5eedu32;
    let mut rnd = move || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed >> 8) as f32 / 16777216.0
    };
    let stars: Vec<(f32, f32, f32)> =
        (0..70).map(|_| (rnd() * W as f32, rnd() * (H as f32 * 0.72), 0.25 + rnd() * 0.7)).collect();
    // TEN shards, like js worldShards() — never the whole relic catalog.
    let shards: Vec<(u32, f32, f32)> = wriftheart::relics_data::LIST
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, r)| (r.col, (i as f32 / 10.0) * std::f32::consts::TAU + 0.3, 1.4 + (i % 4) as f32 * 0.4))
        .collect();
    let looks = [0x3cdc5au32, 0x4a9cff, 0xc060fc, 0xfc7460, 0xe0c040, 0x50c0a0];
    let lines = ["THE OLD DOOM COMES AGAIN!", "RUN!", "FIRE!", "FLEE!", "THE SKY TORE OPEN!", "HELP US!"];
    let npcs: Vec<Npc> = (0..6)
        .map(|i| Npc {
            x: 24.0 + i as f32 * 60.0,
            base_y: 104.0 + (i % 3) as f32 * 30.0,
            spd: 0.8 + rnd() * 0.9,
            col: looks[i % 6],
            line: lines[i % 6],
            dir: if i % 2 == 1 { 1.0 } else { -1.0 },
            always: i < 2,
            ph: i as f32 * 4.0,
        })
        .collect();
    let mut texts = HashMap::default();
    for s in lines {
        let (h, w) = wriftheart::gfx::font::bake_text(s, 0xffd0d0, &mut images);
        texts.insert(s.to_string(), (h, w));
    }
    for (s, col) in [("Z Z Z", 0x8a9ab0u32), ("!", 0xffd0c0)] {
        let (h, w) = wriftheart::gfx::font::bake_text(s, col, &mut images);
        texts.insert(s.to_string(), (h, w));
    }
    // A stand-in hero sprite (blue tunic block — the real one needs the whole creator).
    let mut hero_img = Image::new_fill(
        Extent3d { width: 16, height: 16, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    for y in 2..16u32 {
        for x in 4..12u32 {
            let c: [u8; 4] = if y < 7 { [240, 192, 144, 255] } else { [60, 90, 200, 255] };
            if let Ok(px) = hero_img.pixel_bytes_mut(bevy::math::UVec3::new(x, y, 0)) {
                px.copy_from_slice(&c);
            }
        }
    }
    let hero = images.add(hero_img);
    let fronts = [("inn", 52, 68), ("store", 150, 58), ("blacksmith", 252, 68), ("tavern", 96, 150), ("bakery", 330, 150), ("cottage", 206, 176)]
        .into_iter()
        .map(|(k, x, y)| (k, x, y, prop_art.fronts.get(k).cloned()))
        .collect();
    let dummy = images.add(Image::new_fill(
        Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let folk = (0..6u32)
        .map(|i| wriftheart::actors::hero::build_frames(&wriftheart::actors::hero::random_look(i * 7919 + 3), &mut images).frames)
        .collect();
    let st = CsInner {
        canvas: dummy,
        sprite: bevy::ecs::entity::Entity::PLACEHOLDER,
        stars,
        shards,
        npcs,
        texts,
        hero: hero.clone(),
        folk,
        well: prop_art.well.clone(),
        torch: prop_art.torch.clone(),
        fronts,
        skip_hint: String::new(),
    };

    let args: Vec<u32> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    let frames = if args.is_empty() { vec![150, 380, 425, 600, 900, 1030, 1290] } else { args };
    for t in frames {
        let mut cv = Cv::new(W, H);
        let tf = t as f32;
        let hero_ref = images.get(&hero);
        match t {
            0..=259 => scenes::whole_age(&mut cv, tf),
            260..=499 => scenes::sky(&mut cv, &st, tf - 260.0),
            500..=719 => scenes::scatter(&mut cv, &st, tf - 500.0),
            720..=959 => scenes::choir(&mut cv, &st, tf - 720.0),
            960..=1159 => {
                if let Some(h) = hero_ref {
                    scenes::interior(&mut cv, h, tf - 960.0);
                }
                let lt = tf - 960.0;
                let (bx, by) = (40 + 2 * 16 + 22, 4 + 2 * 16 + 12);
                if lt < 54.0 {
                    scenes::text(&mut cv, &images, &st, "Z Z Z", bx + 14, by - 12, 0.8, 1);
                } else if lt < 100.0 {
                    scenes::text(&mut cv, &images, &st, "!", bx + 13, by - 18, 1.0, 2);
                }
            }
            _ => scenes::town(&mut cv, &images, &st, hero_ref, tf - 1160.0),
        }
        // PPM out (P6): trivial to write, `sips` turns it into a PNG.
        let mut out = format!("P6\n{} {}\n255\n", W, H).into_bytes();
        for i in 0..(W * H) as usize {
            out.extend_from_slice(&cv.px[i * 4..i * 4 + 3]);
        }
        let path = format!("/tmp/cs_{t}.ppm");
        std::fs::write(&path, out).expect("write ppm");
        println!("wrote {path}");
        // CS_ZOOM="x,y,w,h,scale": also write a nearest-neighbour zoomed crop.
        if let Ok(z) = std::env::var("CS_ZOOM") {
            let v: Vec<i32> = z.split(',').filter_map(|s| s.parse().ok()).collect();
            if let [zx, zy, zw, zh, sc] = v[..] {
                let mut out = format!("P6\n{} {}\n255\n", zw * sc, zh * sc).into_bytes();
                for oy in 0..zh * sc {
                    for ox in 0..zw * sc {
                        let (sx, sy) = ((zx + ox / sc).clamp(0, W - 1), (zy + oy / sc).clamp(0, H - 1));
                        let i = ((sy * W + sx) * 4) as usize;
                        out.extend_from_slice(&cv.px[i..i + 3]);
                    }
                }
                let path = format!("/tmp/cs_{t}_zoom.ppm");
                std::fs::write(&path, out).expect("write zoom ppm");
                println!("wrote {path}");
            }
        }
    }
}
