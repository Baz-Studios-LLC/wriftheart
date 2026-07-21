//! rune.rs — the WARP RUNE (split from the dungeon monolith, task #46): the js
//! warpRune drawn live (pulse/ring/glyph/motes, GREY while the shard waits), the
//! ACTIVATE prompt, and the press that rides home (RuneActivate -> navigate).

use super::*;

/// The way home after the kill — a bare anchor; rune_tick draws the js warpRune
/// live and turns a PRESS on it into the ride out.
#[derive(Component)]
pub struct WarpRune {
    pub x: f32,
    pub y: f32,
}

/// "The rune was pressed" — rune_tick writes, navigate rides home.
#[derive(Message)]
pub struct RuneActivate;

/// Marker on the rune's live sprites (rebuilt each tick, immediate-mode).
#[derive(Component)]
pub(super) struct RuneFxUi;

/// Marker on the rune's ACTIVATE bubble.
#[derive(Component, Clone)]
pub(super) struct RunePromptUi;

/// The js warpRune, pixel for pixel: a pulsing violet radial glow, the r6 ring,
/// the cross glyph, four motes orbiting — SEALED grey and near-still while the
/// boss's shard sits unclaimed (Baz: it wakes when you take it). Standing on a
/// woken rune offers ACTIVATE by the character; the press rides home.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub(super) fn rune_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    clock: Res<crate::app::room_render::FrameClock>,
    mut input: ResMut<ActionState>,
    in_dungeon: Res<InDungeon>,
    bindings: Res<crate::input::Bindings>,
    mut goes: MessageWriter<RuneActivate>,
    players: Query<&Player>,
    runes: Query<&WarpRune>,
    shards: Query<(), With<RelicShard>>,
    old_fx: Query<Entity, With<RuneFxUi>>,
    old_prompt: Query<Entity, With<RunePromptUi>>,
    mut tex: Local<Option<(Handle<Image>, Handle<Image>)>>,
) {
    for e in old_fx.iter().chain(old_prompt.iter()) {
        commands.entity(e).despawn();
    }
    if in_dungeon.0.is_none() || runes.is_empty() {
        return;
    }
    let tint = |c: u32, a: f32| {
        Color::srgba((c >> 16 & 255) as f32 / 255.0, (c >> 8 & 255) as f32 / 255.0, (c & 255) as f32 / 255.0, a)
    };
    let (glow, ring) = tex.get_or_insert_with(|| (crate::gfx::radial_glow_tex(&mut images, 48), rune_ring_tex(&mut images))).clone();
    let dim = !shards.is_empty(); // the unclaimed shard outranks the ride home
    let t = clock.0 as f32;
    let pulse = ((0.55 + 0.45 * (t * 0.11).sin()) * if dim { 0.3 } else { 1.0 }).clamp(0.0, 1.0);
    let gr = 10.0 + (t * 0.11).sin() * 1.5 + 6.0; // js R + the gradient's 6px skirt
    let glow_col = if dim { 0x6e7882 } else { 0xa064f5 };
    let ring_col = if dim { 0x8a94a0 } else { 0xc8a0ff };
    let glyph_col = if dim { 0xaab4c0 } else { 0xede0ff };
    for r in &runes {
        let (cx, cy) = (PLAY_X + r.x + 8.0, PLAY_Y + r.y + 8.0);
        let mut g = Sprite::from_image(glow.clone());
        g.custom_size = Some(Vec2::splat(gr * 2.0));
        g.color = tint(glow_col, 0.55 * pulse);
        commands.spawn((g, at(cx - gr, cy - gr, gr * 2.0, gr * 2.0, 2.9), PIXEL_LAYER, RuneFxUi));
        let mut rs = Sprite::from_image(ring.clone());
        rs.color = tint(ring_col, pulse);
        commands.spawn((rs, at(cx - 6.5, cy - 6.5, 13.0, 13.0, 3.0), PIXEL_LAYER, RuneFxUi));
        for (gx, gy, gw, gh) in [(cx - 1.0, cy - 5.0, 2.0, 10.0), (cx - 5.0, cy - 1.0, 10.0, 2.0)] {
            commands.spawn((
                Sprite::from_color(tint(glyph_col, pulse), Vec2::new(gw, gh)),
                at(gx, gy, gw, gh, 3.02),
                PIXEL_LAYER,
                RuneFxUi,
            ));
        }
        if !dim {
            for i in 0..4 {
                let a = t * 0.05 + i as f32 * std::f32::consts::FRAC_PI_2;
                commands.spawn((
                    Sprite::from_color(tint(glyph_col, pulse), Vec2::splat(2.0)),
                    at((cx + a.cos() * 7.0).round() - 1.0, (cy + a.sin() * 7.0).round() - 1.0, 2.0, 2.0, 3.05),
                    PIXEL_LAYER,
                    RuneFxUi,
                ));
            }
        }
    }
    // The prompt + the press — a WOKEN rune only (the grey one just sleeps).
    let Ok(p) = players.single() else { return };
    let hb = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    let on_rune = runes
        .iter()
        .any(|r| hb.0 < r.x + 16.0 && hb.0 + hb.2 > r.x && hb.1 < r.y + 16.0 && hb.1 + hb.3 > r.y);
    if on_rune && !dim {
        let text = format!("{} ACTIVATE", bindings.prompt(Action::Interact, input.pad_present));
        crate::app::prompts::spawn_bubble(&mut commands, &mut images, &text, p.x + 8.0, p.y - 10.0, RunePromptUi);
        if input.pressed(Action::Interact) {
            input.consume(Action::Interact);
            goes.write(RuneActivate);
        }
    }
}


/// The js ctx.arc(r6) stroke as a crisp pixel circle, baked once (13x13, white).
pub(super) fn rune_ring_tex(images: &mut Assets<Image>) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const S: usize = 13;
    let mut data = vec![0u8; S * S * 4];
    for y in 0..S {
        for x in 0..S {
            let (dx, dy) = (x as f32 - 6.0, y as f32 - 6.0);
            if ((dx * dx + dy * dy).sqrt() - 6.0).abs() < 0.55 {
                let i = (y * S + x) * 4;
                data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    images.add(Image::new(
        Extent3d { width: S as u32, height: S as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ))
}
