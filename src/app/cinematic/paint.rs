//! cinematic/paint.rs — a tiny SOFTWARE CANVAS, so the opening can be the js's
//! full canvas paintings instead of a sprite collage. The js drew each cutscene
//! frame immediate-mode with gradients, beziers and polygons; here the same ops
//! run on a CPU RGBA buffer that uploads to one screen-sized Image per frame
//! (the flyover bake's idiom, animated). Everything is y-DOWN in js canvas
//! coordinates — no at()/bevy-y anywhere in the painters.

use bevy::prelude::*;

/// The frame buffer (CANVAS_W x CANVAS_H, RGBA8, y-down).
pub struct Cv {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u8>,
}

fn rgb(col: u32) -> [f32; 3] {
    [((col >> 16) & 255) as f32, ((col >> 8) & 255) as f32, (col & 255) as f32]
}

impl Cv {
    pub fn new(w: i32, h: i32) -> Self {
        Cv { w, h, px: vec![0; (w * h * 4) as usize] }
    }

    #[inline]
    fn blend(&mut self, x: i32, y: i32, c: [f32; 3], a: f32) {
        if !(0..self.w).contains(&x) || !(0..self.h).contains(&y) || a <= 0.0 {
            return;
        }
        let a = a.min(1.0);
        let i = ((y * self.w + x) * 4) as usize;
        for (k, ch) in c.iter().enumerate() {
            self.px[i + k] = (ch * a + self.px[i + k] as f32 * (1.0 - a)) as u8;
        }
        self.px[i + 3] = 255;
    }

    /// ctx.fillRect with a solid or translucent colour.
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, col: u32, a: f32) {
        let c = rgb(col);
        for yy in y..y + h {
            for xx in x..x + w {
                self.blend(xx, yy, c, a);
            }
        }
    }

    /// A vertical linear gradient over a rect (createLinearGradient(0,y0,0,y1)):
    /// stops are (position 0..1, colour, alpha).
    pub fn grad_v(&mut self, x: i32, y: i32, w: i32, h: i32, stops: &[(f32, u32, f32)]) {
        for yy in y..y + h {
            let t = (yy - y) as f32 / (h - 1).max(1) as f32;
            let (mut c, mut a) = (rgb(stops[0].1), stops[0].2);
            for win in stops.windows(2) {
                let (p0, c0, a0) = win[0];
                let (p1, c1, a1) = win[1];
                if t >= p0 {
                    let k = ((t - p0) / (p1 - p0).max(0.0001)).clamp(0.0, 1.0);
                    let (r0, r1) = (rgb(c0), rgb(c1));
                    c = [r0[0] + (r1[0] - r0[0]) * k, r0[1] + (r1[1] - r0[1]) * k, r0[2] + (r1[2] - r0[2]) * k];
                    a = a0 + (a1 - a0) * k;
                }
            }
            for xx in x..x + w {
                self.blend(xx, yy, c, a);
            }
        }
    }

    /// A radial GLOW (createRadialGradient): alpha `a0` at r0 fading to 0 at r1.
    pub fn radial(&mut self, cx: f32, cy: f32, r0: f32, r1: f32, col: u32, a0: f32) {
        let c = rgb(col);
        let (x0, x1) = ((cx - r1).floor() as i32, (cx + r1).ceil() as i32);
        let (y0, y1) = ((cy - r1).floor() as i32, (cy + r1).ceil() as i32);
        for yy in y0..=y1 {
            for xx in x0..=x1 {
                let d = ((xx as f32 - cx).powi(2) + (yy as f32 - cy).powi(2)).sqrt();
                if d >= r1 {
                    continue;
                }
                let k = ((d - r0) / (r1 - r0).max(0.0001)).clamp(0.0, 1.0);
                self.blend(xx, yy, c, a0 * (1.0 - k));
            }
        }
    }

    /// The vignette: alpha 0 inside r0 ramping to `a_edge` at r1 and beyond.
    pub fn vignette(&mut self, cx: f32, cy: f32, r0: f32, r1: f32, col: u32, a_edge: f32) {
        let c = rgb(col);
        for yy in 0..self.h {
            for xx in 0..self.w {
                let d = ((xx as f32 - cx).powi(2) + (yy as f32 - cy).powi(2)).sqrt();
                let k = ((d - r0) / (r1 - r0).max(0.0001)).clamp(0.0, 1.0);
                self.blend(xx, yy, c, a_edge * k);
            }
        }
    }

    /// A filled CONVEX polygon (the gem facets, the ridge) — scanline over edges.
    pub fn poly(&mut self, pts: &[(f32, f32)], col: u32, a: f32) {
        let c = rgb(col);
        let (mut ymin, mut ymax) = (f32::MAX, f32::MIN);
        for &(_, y) in pts {
            ymin = ymin.min(y);
            ymax = ymax.max(y);
        }
        for yy in ymin.floor() as i32..=ymax.ceil() as i32 {
            let fy = yy as f32 + 0.5;
            let (mut xa, mut xb) = (f32::MAX, f32::MIN);
            for i in 0..pts.len() {
                let (x0, y0) = pts[i];
                let (x1, y1) = pts[(i + 1) % pts.len()];
                if (y0 <= fy) != (y1 <= fy) {
                    let x = x0 + (fy - y0) / (y1 - y0) * (x1 - x0);
                    xa = xa.min(x);
                    xb = xb.max(x);
                }
            }
            if xa <= xb {
                for xx in xa.round() as i32..=xb.round() as i32 {
                    self.blend(xx, yy, c, a);
                }
            }
        }
    }

    /// ctx.lineTo strokes, width 1 or 3 (the bolt) — sampled along the segment.
    #[allow(clippy::too_many_arguments)] // a stroke is coordinates all the way down
    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, col: u32, a: f32, wpx: i32) {
        let c = rgb(col);
        let n = ((x1 - x0).abs().max((y1 - y0).abs()).ceil() as i32).max(1);
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let (x, y) = ((x0 + (x1 - x0) * t).round() as i32, (y0 + (y1 - y0) * t).round() as i32);
            let r = wpx / 2;
            for dy in -r..=r {
                for dx in -r..=r {
                    self.blend(x + dx, y + dy, c, a);
                }
            }
        }
    }

    /// Fill every column from `ridge(x)` down to `bottom` (the js hill paths).
    pub fn fill_below(&mut self, bottom: i32, col: u32, a: f32, ridge: impl Fn(f32) -> f32) {
        let c = rgb(col);
        for xx in 0..self.w {
            let top = ridge(xx as f32).round() as i32;
            for yy in top.max(0)..bottom.min(self.h) {
                self.blend(xx, yy, c, a);
            }
        }
    }

    /// drawImage: blit a baked asset image (optionally mirrored / rotated 90° —
    /// the sleeping hero), with a global alpha and a whole-pixel scale (the js
    /// Font scale-2 '!').
    #[allow(clippy::too_many_arguments)] // a blit is coordinates all the way down
    pub fn blit(&mut self, img: &Image, dx: i32, dy: i32, flip_x: bool, rot90: bool, alpha: f32, scale: i32) {
        let Some(data) = img.data.as_ref() else { return };
        let (sw, sh) = (img.width() as i32, img.height() as i32);
        let (ow, oh) = if rot90 { (sh, sw) } else { (sw, sh) };
        for oy in 0..oh * scale {
            for ox in 0..ow * scale {
                let (bx, by) = (ox / scale, oy / scale);
                // Source pixel for this output pixel (rot90: right edge becomes top).
                let (mut sx, sy) = if rot90 { (by, sw - 1 - bx) } else { (bx, by) };
                if flip_x {
                    sx = sw - 1 - sx;
                }
                let si = ((sy * sw + sx) * 4) as usize;
                let a = data[si + 3] as f32 / 255.0 * alpha;
                if a <= 0.003 {
                    continue;
                }
                let c = [data[si] as f32, data[si + 1] as f32, data[si + 2] as f32];
                self.blend(dx + ox, dy + oy, c, a);
            }
        }
    }
}
