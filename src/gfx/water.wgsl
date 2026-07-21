// water.wgsl — the water surface, REBUILT FROM THE GROUND UP (Baz): the shader
// PAINTS the whole body — no tile sprite underneath. Pixel-quantized and
// posterized so it sits inside the game's art, not on top of it.
//
// Recipe per pixel (room space, snapped to whole px):
//   depth ramp (mask a: shore..deep) -> undulating surface height from two
//   drifting interference waves whose SAMPLE POSITION wobbles (the ripple lives
//   in the pattern itself) -> posterize into 3 banded tones -> ripple lines,
//   crest glints, and a soft shore lap on top. Storms raise every amplitude.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var mask: texture_2d<f32>;
@group(2) @binding(1) var mask_sampler: sampler;

struct WaterParams {
    time: f32,
    strength: f32,
    storm: f32,
    _p1: f32,
    shallow: vec4<f32>,
    deep: vec4<f32>,
    wave: vec4<f32>,
}
@group(2) @binding(2) var<uniform> params: WaterParams;

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let m = textureSample(mask, mask_sampler, in.uv);
    if (m.r < 0.5) {
        return vec4(0.0);
    }
    // Whole-pixel room position — the art law: no sub-pixel gradients.
    let px = floor(in.uv * vec2(304.0, 208.0));
    let t = params.time * (1.0 + params.storm * 0.7);

    // The UNDULATION: the sample position itself sways (x by row, y by column),
    // deep water swaying most — this is the ripple applied to the surface.
    let amp = (1.0 + params.storm * 1.6) * m.a;
    let sway = vec2(
        floor(sin(px.y * 0.55 + t * 2.2) * amp + 0.5),
        floor(sin(px.x * 0.31 - t * 1.6) * amp * 0.5 + 0.5),
    );
    let sp = px + sway;

    // Two slow crossing waves -> a drifting interference height field.
    let h = sin(sp.y * 0.5 + t * 1.1 + sin(sp.x * 0.19 + t * 0.6) * 2.0)
          + sin((sp.x + sp.y) * 0.16 - t * 0.7);

    // DEPTH: the mask's alpha is already bilinear-smoothed across tiles (water.rs
    // bakes it at pixel res), so deep patches arrive as rounded contours. Sample
    // at the SWAYED position so they breathe, then requantize to coarse steps so
    // it stays posterized art, not a soft gradient.
    let md = textureSampleLevel(mask, mask_sampler, (sp + vec2(0.5)) / vec2(304.0, 208.0), 0.0);
    let depth = floor(md.a * 6.0 + 0.5) / 6.0;

    // Base: depth ramp, POSTERIZED by the height into three banded tones.
    var col = mix(params.shallow.rgb, params.deep.rgb, depth * 0.8);
    if (h > 0.85) {
        col = mix(col, params.wave.rgb, 0.18); // lit band, a whisper
    } else if (h < -0.95) {
        col = col * 0.9; // trough band
    }

    // Fine ripple LINES drifting down the swayed surface.
    let rp = sin(sp.y * 1.6 - t * 1.4 + sin(sp.x * 0.5 + t * 0.5) * 1.4);
    if (rp > 0.92 - params.storm * 0.1) {
        col = mix(col, params.wave.rgb, 0.28);
    }

    // Sparse crest GLINTS (slow-twinkling, hash-seeded so they don't march).
    let cell = floor(sp / 4.0);
    let tw = hash2(cell);
    if (h > 1.55 - params.storm * 0.3 && fract(tw + t * 0.13) > 0.95) {
        col = mix(col, vec3(0.85, 0.95, 1.0), 0.5);
    }

    // The SHORE LAP: a soft bright rim right at the land edge, breathing slowly.
    if (m.a < 0.12) {
        let lap = 0.14 + 0.11 * sin(t * 1.3 + px.x * 0.22 + px.y * 0.17);
        col = mix(col, params.wave.rgb, lap);
    }

    return vec4(col * params.strength, 1.0);
}
