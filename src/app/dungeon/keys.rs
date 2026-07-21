//! keys.rs — the dungeon KEY RING (split from the dungeon monolith, task #46):
//! the small/ornate counters, the outside-reset, and the in-view key HUD.

use super::*;

/// Dungeon keys are a per-dungeon COUNT shown in the HUD, NOT bag items (Baz). Zeroed the
/// moment you leave a dungeon (`clear_keys_outside`), so keys can't exist in the overworld;
/// persists across floors (only reset outside). `small` opens small locks, `ornate` the boss
/// door.
#[derive(Resource, Default)]
pub struct DungeonKeys {
    pub small: u32,
    pub ornate: u32,
}

/// The in-view key counter (bottom-right of the play area).
#[derive(Component)]
pub(super) struct KeyHud;

/// Keys don't exist in the overworld (Baz): zero the count the moment you're out of a dungeon.
/// Inside, they persist across floors (the run stays live, so this never fires there).
pub(super) fn clear_keys_outside(in_dungeon: Res<InDungeon>, mut keys: ResMut<DungeonKeys>) {
    if in_dungeon.0.is_none() && (keys.small != 0 || keys.ornate != 0) {
        keys.small = 0;
        keys.ornate = 0;
    }
}

/// The in-view key counter — a small key icon + `xN` per kind at the bottom-right of the play
/// area (Baz), shown only inside a dungeon; rebuilt only when a count changes.
pub(super) fn key_hud(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    in_dungeon: Res<InDungeon>,
    keys: Res<DungeonKeys>,
    old: Query<Entity, With<KeyHud>>,
    mut last: Local<(u32, u32)>,
) {
    let cur = if in_dungeon.0.is_some() { (keys.small, keys.ornate) } else { (0, 0) };
    if *last == cur {
        return;
    }
    *last = cur;
    for e in &old {
        commands.entity(e).despawn();
    }
    if cur == (0, 0) {
        return;
    }
    use crate::room::PX_H;
    use crate::app::room_render::{PLAY_X, PLAY_Y};
    let z = crate::gfx::layers::PROMPT_TEXT;
    let key_img = images.add(crate::gfx::bake(crate::actors::items_art::KEY_ICON, &[]));
    let okey_img = images.add(crate::gfx::bake(crate::actors::items_art::OKEY_ICON, &[('m', 0xc878ff)]));
    let mut y = PLAY_Y + PX_H as f32 - 11.0; // bottom row, stacking upward
    for (n, img) in [(cur.0, key_img), (cur.1, okey_img)] {
        if n == 0 {
            continue;
        }
        let txt = format!("x{n}");
        // Bottom-LEFT (Baz): the notification feed owns the bottom-right corner.
        let icx = PLAY_X + 3.0;
        commands.spawn((
            Sprite::from_image(img),
            crate::gfx::at(icx, y, 8.0, 8.0, z),
            crate::gfx::PIXEL_LAYER,
            KeyHud,
        ));
        crate::ui::label(&mut commands, &mut images, &txt, icx + 9.0, y + 1.0, 0xfce0a8, z, KeyHud);
        y -= 10.0;
    }
}
