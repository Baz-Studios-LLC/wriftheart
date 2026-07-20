//! Render a HEADLESS contact sheet of every seeded tree kind (3 seeds each) to
//! /tmp/trees_sheet.ppm — visual verification without a window (WRIFT_SHOT blacks
//! out under macOS occlusion). Convert: `sips -s format png ... --out ....png`.

use wriftheart::actors::props::{tree_grid, tree_pal};
use wriftheart::app::cinematic::paint::Cv;
use wriftheart::gfx::bake;

fn main() {
    const KINDS: [&str; 11] = [
        "shroom", "burnttree", "riftbulb", "voidspire", "mawtree", "giantflower", "crystalspire", "stalagmite",
        "blossom", "jungletree", "bluebloom",
    ];
    const SEEDS: [i32; 3] = [123, 1543, 3877];
    let (cell_w, cell_h) = (52, 76);
    let (w, h) = (KINDS.len() as i32 * cell_w, SEEDS.len() as i32 * cell_h);
    let mut cv = Cv::new(w, h);
    cv.rect(0, 0, w, h, 0x33502e, 1.0); // grass-ish backdrop so outlines read
    for (ki, kind) in KINDS.iter().enumerate() {
        for (si, &seed) in SEEDS.iter().enumerate() {
            let grid = tree_grid(kind, seed);
            let img = bake(&grid.iter().map(|s| s.as_str()).collect::<Vec<_>>(), &tree_pal(kind, seed));
            // Bottom-centre each sprite in its cell (small spires sit low like in play).
            let (iw, ih) = (img.width() as i32, img.height() as i32);
            let x = ki as i32 * cell_w + (cell_w - iw) / 2;
            let y = si as i32 * cell_h + (cell_h - ih) - 2;
            cv.blit(&img, x, y, false, false, 1.0, 1);
        }
    }
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    for i in 0..(w * h) as usize {
        out.extend_from_slice(&cv.px[i * 4..i * 4 + 3]);
    }
    std::fs::write("/tmp/trees_sheet.ppm", out).expect("write ppm");
    println!("wrote /tmp/trees_sheet.ppm");
}
