//! wrift.rs — the WRIFT star-dust baker: dim dust, brighter motes, and the rare
//! teal / violet shard glint, deterministic per seed and seamless at 96px. Shared
//! by the codex map's parallax backdrop and the skill constellation (Baz: the
//! same parallax constellation everywhere the void shows).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// The pattern period — scroll offsets wrap at this many pixels.
pub const WRIFT_T: u32 = 96;

/// A `w` x `h` sheet of the pattern for `seed`: the 96px tile stamped at every
/// repeat, so any crop of the sheet scrolls seamlessly modulo `WRIFT_T`.
pub fn wrift_sheet(seed: u32, w: u32, h: u32) -> Image {
    let t = WRIFT_T;
    let mut rng = crate::worldgen::rng::Mulberry32::new(seed);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let mut put = |x: u32, y: u32, c: u32| {
        let mut yy = y % t;
        while yy < h {
            let mut xx = x % t;
            while xx < w {
                let i = ((yy * w + xx) * 4) as usize;
                buf[i] = (c >> 16) as u8;
                buf[i + 1] = (c >> 8) as u8;
                buf[i + 2] = c as u8;
                buf[i + 3] = 255;
                xx += t;
            }
            yy += t;
        }
    };
    for _ in 0..26 {
        // Faint dust.
        let (x, y) = ((rng.next_f64() * t as f64) as u32, (rng.next_f64() * t as f64) as u32);
        put(x, y, 0x1e1e2a);
    }
    for _ in 0..9 {
        // Brighter motes.
        let (x, y) = ((rng.next_f64() * t as f64) as u32, (rng.next_f64() * t as f64) as u32);
        put(x, y, 0x33314a);
    }
    for _ in 0..3 {
        // Shard glints: a 2x2 fleck of the rift's teal or violet.
        let (x, y) = ((rng.next_f64() * t as f64) as u32, (rng.next_f64() * t as f64) as u32);
        let c = if rng.next_f64() < 0.5 { 0x2e4a52 } else { 0x443257 };
        put(x, y, c);
        put(x + 1, y, c);
        put(x, y + 1, c);
        put(x + 1, y + 1, c);
    }
    Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        buf,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// One seamless 96x96 tile — the map's tiled-sprite layers use this.
pub fn wrift_tile(seed: u32) -> Image {
    wrift_sheet(seed, WRIFT_T, WRIFT_T)
}
