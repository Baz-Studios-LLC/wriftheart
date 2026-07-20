// shadow.wgsl — the cast-shadow material: the actor's own texture, flipped so its
// feet meet the quad's TOP edge (the contact line), sheared so the far end leans with
// the sun, gaussian-blurred, and flattened to black.
//
// The port's first shader. UV space: quad (0,0) = top-left of the drawn rect; the
// texture samples at v flipped (quad top reads the art's bottom rows — the feet).

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var tex: texture_2d<f32>;
@group(2) @binding(1) var tex_sampler: sampler;
@group(2) @binding(3) var mask: texture_2d<f32>;
@group(2) @binding(4) var mask_sampler: sampler;

struct ShadowParams {
    rect: vec4<f32>, // the quad in ROOM pixels (x, y, w, h) — maps fragments to the water mask
    shear: f32,      // far-end x lean, in quad widths (+ = east); feet stay planted
    blur: f32,       // gaussian radius, in texels
    opacity: f32,    // final shadow darkness
    flip_x: f32,     // reserved (match a flipped owner sprite later)
}
@group(2) @binding(2) var<uniform> params: ShadowParams;

// One bounded tap: outside [0,1] contributes nothing (no clamp-to-edge smearing).
fn tap(uv: vec2<f32>) -> f32 {
    let a = textureSample(tex, tex_sampler, clamp(uv, vec2(0.0), vec2(1.0))).a;
    let inb = step(0.0, uv.x) * step(uv.x, 1.0) * step(0.0, uv.y) * step(uv.y, 1.0);
    return a * inb;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // The quad is 1.5x wider than the art (shadows.rs CAST_MARGIN) so a sheared
    // silhouette has headroom — map quad u back onto the art's central band.
    let u = (in.uv.x - 0.5) * 1.5 + 0.5;
    // Feet (quad top, uv.y = 0) stay planted; the far end leans by `shear`.
    let src = vec2(u - params.shear * in.uv.y, 1.0 - in.uv.y);
    let ts = params.blur / vec2<f32>(textureDimensions(tex));

    // 9-tap cross gaussian (sigma ~1.4), unrolled — WGSL uniformity stays trivial.
    var a = tap(src) * 0.235;
    a += (tap(src + vec2(ts.x, 0.0)) + tap(src - vec2(ts.x, 0.0))
        + tap(src + vec2(0.0, ts.y)) + tap(src - vec2(0.0, ts.y))) * 0.118;
    a += (tap(src + vec2(ts.x * 2.0, 0.0)) + tap(src - vec2(ts.x * 2.0, 0.0))
        + tap(src + vec2(0.0, ts.y * 2.0)) + tap(src - vec2(0.0, ts.y * 2.0))) * 0.0731;

    // Shadows drown on water (Baz) — the surface shows a REFLECTION there instead,
    // so fragments landing on water tiles drop out (mask r = water).
    let room_px = params.rect.xy + in.uv * params.rect.zw;
    let m = textureSample(mask, mask_sampler, room_px / vec2(304.0, 208.0));
    let dry = 1.0 - step(0.5, m.r);

    return vec4(0.0, 0.0, 0.0, a * params.opacity * dry);
}
