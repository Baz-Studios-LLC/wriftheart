// weather_fx.wgsl — the sky's moods as ONE full-screen pass (PORT-ORIGINAL: the js
// pushed 90-150 canvas particles; we synthesize precipitation procedurally, in three
// PARALLAX depths the js never had). Two crossfading layers (the incoming weather and
// the outgoing one) render the same way at their own visibilities.
//
// Pixel discipline: everything quantizes to the 304x208 grid — drops are 1px columns,
// flakes are quantized squares, fog posterizes into alpha steps. WriftHeart, not a
// weather demo.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct WeatherParams {
    // (kind, visibility, heavy, unused) per layer. Kinds: 0 none / 1 rain / 2 snow /
    // 3 dust / 4 fog. (Wind's leaves are ECS sprites, not a shader pass.)
    layer_a: vec4<f32>,
    layer_b: vec4<f32>,
    time: f32,
    wind: f32,  // -1..1 — the INSTANT wind (static shear only)
    flash: f32, // lightning whiteout 0..1 (pre-scaled for REDUCE FLASHING)
    windx: f32, // accumulated wind travel — ALL displacement comes from this
}
@group(2) @binding(0) var<uniform> params: WeatherParams;

fn hash21(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2(123.34, 345.45));
    q += dot(q, q + 34.345);
    return fract(q.x * q.y);
}

// 2-octave value noise (the fog's rolling banks).
fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2(1.0, 0.0));
    let c = hash21(i + vec2(0.0, 1.0));
    let d = hash21(i + vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// One parallax sheet of rain: 1px streaks in hashed columns (depth sets scale/speed/alpha).
// `cut` = the dry-column threshold: only columns hashing ABOVE it carry drops. The js drew
// ~90-150 drops TOTAL; three dense sheets were painting ~800+ (Baz: "a crazy amount of
// rain lol") — the deeper sheets cover more field per screen pixel, so they cut harder.
fn rain_sheet(px: vec2<f32>, t: f32, wind: f32, scale: f32, speed: f32, cut: f32) -> f32 {
    let slant = wind * 4.0 + 2.2; // the js drop drift, as a field shear
    let sp = vec2((px.x - px.y * slant / 10.0) * scale, px.y * scale);
    let colw = 4.0;
    let col = floor(sp.x / colw);
    let h = hash21(vec2(col, 7.0));
    let wet = step(cut, h);                         // most columns stay dry
    let gap = 70.0 + h * 80.0;                      // the fall between drops (js: ~90-150 on screen)
    let y = sp.y - t * speed + h * 977.0;           // minus: the pattern advances DOWN screen
    let ly = y - floor(y / gap) * gap;              // position within this column's cycle
    let len = 6.0 + h * 6.0;
    let on_x = step(fract(sp.x / colw), 1.0 / colw); // 1px of the 4px column
    return step(ly, len) * on_x * wet;
}

// One sheet of snow: quantized flakes drifting down. The fall is slow; the sway is a
// BOUNDED per-flake sine (the old version multiplied the sway by total elapsed time,
// which sent everything racing sideways — Baz caught it).
fn snow_sheet(px: vec2<f32>, t: f32, wind: f32, windx: f32, scale: f32, speed: f32, heavy: f32) -> f32 {
    let cell = 14.0 / scale;
    let wx = windx * (72.0 + heavy * 220.0); // js windAmt*1.2 (5 heavy) per frame
    let sp = vec2(px.x - wx, px.y - t * speed);
    let id = floor(sp / cell);
    let h = hash21(id);
    if (h < 0.82) { // most cells hold no flake
        return 0.0;
    }
    let fp = fract(sp / cell) * cell;
    // The flake's home in its cell + a gentle sway at its own tempo — floaty, not fast.
    let sway = sin(t * (0.8 + h * 0.7) + h * 6.28318) * 1.8;
    let home = vec2(clamp(1.0 + h * (cell - 3.0) + sway, 0.5, cell - 1.5), 1.0 + fract(h * 13.7) * (cell - 3.0));
    let d = abs(fp - floor(home));
    let size = 1.0 + step(0.5, fract(h * 29.3)); // 1px or 2px flakes
    return step(max(d.x, d.y), size * 0.5 + 0.5);
}

// Dust: SPARSE horizontal streaks racing with the wind (the haze carries the storm's
// weight — a dense streak field read as a wall of static; Baz: "a little much lol").
fn dust_sheet(px: vec2<f32>, t: f32, windx: f32, scale: f32) -> f32 {
    let sp = vec2(px.x - (t * 170.0 + windx * 90.0) * scale, px.y + sin((px.x + px.y) * 0.05) * 2.0);
    let roww = 4.0;
    let row = floor(sp.y * scale / roww);
    let h = hash21(vec2(row, 3.0));
    let windswept = step(0.35, h); // a third of the rows carry nothing
    let gap = 55.0 + h * 70.0;
    let x = sp.x * scale + h * 733.0;
    let lx = x - floor(x / gap) * gap;
    let len = 3.0 + h * 5.0; // GRIT, not dashes — small and fast
    let on_y = step(fract(sp.y * scale / roww), 1.0 / roww);
    return step(lx, len) * on_y * windswept;
}

// One layer's colour + alpha at this pixel.
fn layer(px: vec2<f32>, kind: f32, v: f32, heavy: f32, t: f32, wind: f32, windx: f32) -> vec4<f32> {
    if (v <= 0.003 || kind < 0.5) {
        return vec4(0.0);
    }
    if (kind < 1.5) { // RAIN — three depths: near bright + fast, far faint + slow
        // Heavy storms darken the sky, which makes the bright near sheet POP and read
        // as "too much" (Trello: overbearing in really bad storms). So when heavy, ease
        // the boldest sheet back and drop the base below light-rain — the darker backdrop
        // carries the weight, the streaks stay atmospheric instead of a whiteout wall.
        let near = select(1.0, 0.68, heavy > 0.5);
        var a = rain_sheet(px, t, wind, 1.0, 230.0, 0.5) * near;
        a = max(a, rain_sheet(px + vec2(53.0, 0.0), t, wind, 1.5, 165.0, 0.75) * 0.45);
        a = max(a, rain_sheet(px + vec2(11.0, 0.0), t, wind, 2.1, 115.0, 0.8) * 0.2);
        let base = select(0.42, 0.40, heavy > 0.5);
        return vec4(0.737, 0.831, 0.910, a * base * v); // #bcd4e8
    }
    if (kind < 2.5) { // SNOW — two depths of drifting flakes
        var a = snow_sheet(px, t, wind, windx, 1.0, select(18.0, 34.0, heavy > 0.5), heavy);
        a = max(a, snow_sheet(px + vec2(37.0, 11.0), t, wind, windx, 1.7, select(11.0, 20.0, heavy > 0.5), heavy) * 0.6);
        return vec4(0.933, 0.949, 0.984, a * v); // #eef2fb
    }
    if (kind < 3.5) { // DUST — streaks + the haze the js flooded over everything
        let s = dust_sheet(px, t, windx, 1.0) * 0.30 + dust_sheet(px + vec2(0.0, 1.7), t, windx, 1.6) * 0.16;
        let haze = 0.12;
        let a = min(1.0, s + haze) * v;
        return vec4(0.784, 0.659, 0.416, a); // #c8a86a
    }
    // FOG — rolling noise banks (the js drew five gradient blobs). SMOOTH, not banded
    // (Baz: the posterized steps read as chunky blobs): three octaves and a long,
    // gentle density ramp. The ramp FLOOR sits above the noise field's average
    // (~0.92), so open air stays genuinely clear and only the thick banks roll
    // through — the old 0.45 floor fogged every pixel (Baz: "pretty thick fog lol").
    let n = vnoise(px * 0.014 + vec2(t * 0.10 + windx * 0.05, t * 0.013))
        + 0.55 * vnoise(px * 0.035 - vec2(t * 0.06, t * 0.008))
        + 0.28 * vnoise(px * 0.08 + vec2(t * 0.03, -t * 0.02));
    let dens = smoothstep(0.95, 1.7, n);
    return vec4(0.769, 0.792, 0.824, dens * 0.36 * v); // #c4caD2
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let px = floor(in.uv * vec2(304.0, 208.0)); // the pixel grid IS the canvas
    let t = params.time;
    let a = layer(px, params.layer_a.x, params.layer_a.y, params.layer_a.z, t, params.wind, params.windx);
    let b = layer(px, params.layer_b.x, params.layer_b.y, params.layer_b.z, t, params.wind, params.windx);
    // The stronger layer wins each pixel (matches the js kind-map crossfade).
    var c = select(b, a, a.a >= b.a);
    // Lightning: the world blinks white (pre-scaled for the REDUCE FLASHING setting).
    let fl = params.flash * 0.5;
    c = vec4(mix(c.rgb, vec3(0.918, 0.941, 1.0), fl), max(c.a, fl));
    return c;
}
