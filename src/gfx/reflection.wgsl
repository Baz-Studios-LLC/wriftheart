// reflection.wgsl — actors mirrored in water (PORT-ORIGINAL): the owner's live sprite
// flipped below the feet, tinted toward the water, rippled by a slow sine, and CLIPPED
// to the water mask so it only exists on water and cuts cleanly at the bank.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var tex: texture_2d<f32>;
@group(2) @binding(1) var tex_sampler: sampler;
@group(2) @binding(2) var mask: texture_2d<f32>;
@group(2) @binding(3) var mask_sampler: sampler;

struct ReflectionParams {
    rect: vec4<f32>, // the quad in ROOM pixels (x, y, w, h) — maps fragments to the mask
    time: f32,       // seconds
    opacity: f32,
    ripple: f32,     // x wobble amplitude, in texels
    _pad: f32,
}
@group(2) @binding(4) var<uniform> params: ReflectionParams;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Only on water: fragment -> room pixel -> mask texel.
    let room_px = params.rect.xy + in.uv * params.rect.zw;
    let m = textureSample(mask, mask_sampler, room_px / vec2(304.0, 208.0));
    if (m.r < 0.5) {
        return vec4(0.0);
    }
    // Mirror: quad top (uv.y = 0) reads the art's bottom rows — the feet touch first.
    // The ripple wobbles x, phase running down the reflection.
    let ts = 1.0 / vec2<f32>(textureDimensions(tex));
    let wob = sin(room_px.y * 0.55 + params.time * 2.2) * params.ripple * ts.x;
    let src = vec2(in.uv.x + wob, 1.0 - in.uv.y);
    let inb = step(0.0, src.x) * step(src.x, 1.0);
    let c = textureSample(tex, tex_sampler, clamp(src, vec2(0.0), vec2(1.0)));
    // Pull the colours toward the water; fade with distance from the feet.
    let tinted = mix(c.rgb, vec3(0.30, 0.52, 0.72), 0.5);
    let fade = 1.0 - in.uv.y * 0.55;
    // STRAIGHT alpha — the blend multiplies by a; premultiplying too double-darkens.
    let a = c.a * inb * params.opacity * fade;
    return vec4(tinted, a);
}
