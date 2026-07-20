// water.wgsl — the living water surface (PORT-ORIGINAL): moving highlight bands and a
// deep-water tint over the baked tile animation. Drawn as one room-covering quad,
// clipped to the WATER MASK (r = water, a = shore distance).
//
// Restraint is the design: low alphas, slow drift — WriftHeart, not a tech demo.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var mask: texture_2d<f32>;
@group(2) @binding(1) var mask_sampler: sampler;

struct WaterParams {
    time: f32,    // seconds
    strength: f32,// master dial for the whole effect
    storm: f32,   // 0..1 — rain agitates the surface (weather tie-in)
    _p1: f32,
}
@group(2) @binding(2) var<uniform> params: WaterParams;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let m = textureSample(mask, mask_sampler, in.uv);
    if (m.r < 0.5) {
        return vec4(0.0);
    }
    // Room-pixel position (the quad covers the 304x208 room exactly).
    let px = in.uv * vec2(304.0, 208.0);
    let t = params.time * (1.0 + params.storm * 0.8); // a storm hurries every wave

    // Two slow crossing waves make a drifting interference pattern.
    let w = sin(px.y * 0.35 + t * 1.1 + sin(px.x * 0.13 + t * 0.6) * 2.0)
          + sin((px.x + px.y) * 0.11 - t * 0.7);
    // Sparse glints on the crests; a faint trough shade opposite.
    let glint = smoothstep(1.55 - params.storm * 0.35, 1.9, w) * (0.14 + params.storm * 0.10);
    let shade = smoothstep(-1.9, -1.55, -w) * 0.05;
    // The RIPPLE (Baz): fine undulating lines drifting down the surface — thin light
    // crests with a whisper of shade beneath, wavelength a few pixels.
    let rp = sin(px.y * 1.6 - t * 1.4 + sin(px.x * 0.35 + t * 0.5) * 1.4);
    let ripple_lit = smoothstep(0.86 - params.storm * 0.12, 0.99, rp) * (0.07 + params.storm * 0.06);
    let ripple_dim = smoothstep(0.86, 0.99, -rp) * 0.04;
    // Deeper water sits darker (mask alpha = shore distance).
    let deep = m.a * 0.16;

    let light = vec3(0.75, 0.92, 1.0) * (glint + ripple_lit);
    let dark = (shade + deep + ripple_dim) * params.strength;
    // Composite: glints + ripple crests lighten, depth/troughs darken.
    return vec4(light * params.strength, (glint + ripple_lit + dark));
}
