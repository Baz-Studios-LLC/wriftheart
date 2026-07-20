//! rng.rs — the deterministic core of world generation.
//!
//! The JS overworld is a PURE FUNCTION of (world seed, room x, room y): the same seed always
//! grows the same world. That property rides entirely on the exact bit-arithmetic of these
//! three functions, ported char-for-char from `js/world.js`. If any of them drifts, EVERY
//! existing world changes — so `tests/worldgen_parity.rs` pins them to golden values captured
//! from the live JS, and CI-style `cargo test` catches any regression.
//!
//! JS -> Rust arithmetic contract:
//! * `Math.imul(a, b)` == `a.wrapping_mul(b)` on u32 (low 32 bits; signedness is irrelevant)
//! * `x >>> n` == `x >> n` on u32 (logical shift)
//! * `x >> n` (on int) == `x >> n` on i32 (arithmetic / sign-extending — the trap for negative
//!   room coordinates)
//! * `x & mask`, `| 0`, `>>> 0` == plain u32/i32 bit ops
//! * `/ 4294967296` == `/ 4294967296.0` in f64 (2^32 and any u32 are exact in f64, so the single
//!   division is bit-identical to JS)

/// FNV-flavoured spatial hash — `hash(x, y, salt)` in js/world.js, with the world seed threaded
/// in explicitly (the JS closes over a module-level `WORLD_SEED`).
///
/// `x`/`y` are signed room coordinates (rooms exist at negative positions); the `>> 16`
/// sign-extends exactly as JS's does.
pub fn hash(seed: u32, x: i32, y: i32, salt: u32) -> u32 {
    let mut h = 2166136261u32 ^ seed ^ salt;
    h = (h ^ ((x & 0xffff) as u32)).wrapping_mul(16777619);
    h = (h ^ (((x >> 16) & 0xffff) as u32)).wrapping_mul(16777619);
    h = (h ^ ((y & 0xffff) as u32)).wrapping_mul(16777619);
    h = (h ^ (((y >> 16) & 0xffff) as u32)).wrapping_mul(16777619);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    h
}

/// A mulberry32 PRNG — `makeRng(seed)` in js/world.js. Stateful (the JS returns a closure over
/// `a`); call [`Mulberry32::next_f64`] for each successive value in `[0, 1)`.
pub struct Mulberry32 {
    a: u32,
}

impl Mulberry32 {
    /// `makeRng(seed)` — `a = seed >>> 0`.
    pub fn new(seed: u32) -> Self {
        Self { a: seed }
    }

    /// One draw in `[0, 1)`, advancing the stream — the closure body in `makeRng`.
    pub fn next_f64(&mut self) -> f64 {
        self.a = self.a.wrapping_add(0x6d2b79f5); // (a + 0x6d2b79f5) | 0
        let mut t = (self.a ^ (self.a >> 15)).wrapping_mul(1 | self.a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        ((t ^ (t >> 14)) as f64) / 4294967296.0
    }
}

/// A lattice sample in `[0, 1)` — `lattice(ix, iy, salt)` in js/world.js.
pub fn lattice(seed: u32, ix: i32, iy: i32, salt: u32) -> f64 {
    hash(seed, ix, iy, salt) as f64 / 4294967296.0
}

/// Smoothstep — `smooth(t)` in js/world.js. Kept private; only `value_noise` needs it.
fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// Bilinear value noise — `valueNoise(x, y, salt)` in js/world.js. The operation ORDER matches
/// the JS exactly so the f64 result is bit-identical (no fused multiply-add is introduced).
pub fn value_noise(seed: u32, x: f64, y: f64, salt: u32) -> f64 {
    let x0 = x.floor();
    let y0 = y.floor();
    let sx = smooth(x - x0);
    let sy = smooth(y - y0);
    let (ix, iy) = (x0 as i32, y0 as i32);
    let n00 = lattice(seed, ix, iy, salt);
    let n10 = lattice(seed, ix + 1, iy, salt);
    let n01 = lattice(seed, ix, iy + 1, salt);
    let n11 = lattice(seed, ix + 1, iy + 1, salt);
    let a = n00 + (n10 - n00) * sx;
    let b = n01 + (n11 - n01) * sx;
    a + (b - a) * sy
}
