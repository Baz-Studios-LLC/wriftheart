//! creator.rs — the character creator (port of js/creator.js), shown after NEW GAME.
//!
//! Name your hero (on-screen keyboard — controller and keyboard share one flow, exactly
//! the js), pick a gender (cosmetic), recolor hair/eyes/skin/outfit, choose a hairstyle
//! (live turntable preview), and see the two good + two bad traits fate dealt you
//! (re-rollable). START rolls a fresh world seed and hands loader.rs the finished hero.
//! A hero named BAZ boots up bald — applied at START, never shown in the preview.

mod tables;

use super::identity::HeroIdent;
use super::screen::Screen;
use super::title::loader::LoadSlot;
use crate::actors::hero::{build_frames, HeroFrames, Look};
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use crate::traits;
use crate::ui::Pen as UiPen;
use crate::worldgen::rng::Mulberry32;
use tables::{EYES, FEMALE, HAIRS, KB, MALE, NEUTRAL, N_FIELDS, OUTFITS, SKINS, STYLES};
use crate::{CANVAS_H, CANVAS_W};
use bevy::prelude::*;

const Z_BG: f32 = 19.3; // over the title's text band (19.0-19.2)
const Z_TEXT: f32 = 19.45;
const GOLD: u32 = 0xfce0a8;

#[derive(Component, Clone)]
pub struct CreatorUi;

/// The live hero preview sprite (kept across redraws; a spin system turns it).
#[derive(Component)]
pub struct CreatorPreview;

#[derive(Resource)]
pub struct CreatorState {
    pub slot: u32, // target save slot — the title sets this before entering
    cursor: usize,
    editing: bool,
    kb_row: usize,
    kb_col: usize,
    gender_m: bool,
    hair: usize,
    style: usize,
    eye: usize,
    skin: usize,
    outfit: usize,
    name: String,
    traits: Vec<String>,
    spin: u32,
    rng: Mulberry32,
    frames: Option<HeroFrames>,
    frame_off: [f32; 4], // per-facing head-centering x offset (sprite px)
}

impl Default for CreatorState {
    fn default() -> Self {
        Self {
            slot: 1,
            cursor: 0,
            editing: false,
            kb_row: 0,
            kb_col: 0,
            gender_m: true,
            hair: 0,
            style: 0,
            eye: 0,
            skin: 0,
            outfit: 0,
            name: "BAZ".into(),
            traits: vec![],
            spin: 0,
            rng: Mulberry32::new(1),
            frames: None,
            frame_off: [0.0; 4],
        }
    }
}

impl CreatorState {
    fn look(&self) -> Look {
        Look {
            outfit_light: OUTFITS[self.outfit].1,
            outfit_dark: OUTFITS[self.outfit].2,
            hair_light: HAIRS[self.hair].1,
            hair_dark: HAIRS[self.hair].2,
            skin: SKINS[self.skin].1,
            eye: EYES[self.eye].1,
            hair_style: STYLES[self.style].1.into(),
        }
    }
    fn ri(&mut self, n: usize) -> usize {
        (self.rng.next_f64() * n as f64) as usize % n
    }
}

pub struct CreatorPlugin;

impl Plugin for CreatorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreatorState>()
            .add_systems(OnEnter(Screen::Creator), begin)
            .add_systems(OnExit(Screen::Creator), close)
            .add_systems(
                bevy::app::FixedUpdate,
                creator_tick.before(super::play::EndTick).run_if(in_state(Screen::Creator)),
            )
            .add_systems(Update, (spin_preview, backspace).run_if(in_state(Screen::Creator)));
    }
}

/// (Re)roll everything and raise the screen (js Creator.begin(fresh)).
fn begin(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut st: ResMut<CreatorState>,
    bindings: Res<Bindings>,
    input: Res<ActionState>,
    ui: Query<Entity, With<CreatorUi>>,
) {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| d.subsec_nanos() ^ d.as_secs() as u32)
        .max(1);
    let slot = st.slot;
    *st = CreatorState { slot, rng: Mulberry32::new(seed), ..default() };
    // Roll a starter name; the gender defaults to the name's tag (N = coin flip).
    let all = MALE.len() + FEMALE.len() + NEUTRAL.len();
    let pick = st.ri(all);
    if pick < MALE.len() {
        st.name = MALE[pick].into();
        st.gender_m = true;
    } else if pick < MALE.len() + FEMALE.len() {
        st.name = FEMALE[pick - MALE.len()].into();
        st.gender_m = false;
    } else {
        st.name = NEUTRAL[pick - MALE.len() - FEMALE.len()].into();
        st.gender_m = st.rng.next_f64() < 0.5;
    }
    st.hair = st.ri(HAIRS.len());
    st.style = st.ri(STYLES.len());
    st.eye = st.ri(EYES.len());
    st.skin = st.ri(SKINS.len());
    st.outfit = st.ri(OUTFITS.len());
    st.traits = {
        let mut rng = std::mem::replace(&mut st.rng, Mulberry32::new(1));
        let t = traits::roll(&mut rng);
        st.rng = rng;
        t
    };
    rebuild_preview(&mut st, &mut images);
    // The turntable sprite — ONE entity, spun in place; redraws never touch it.
    commands.spawn((
        Sprite::default(),
        at(CANVAS_W as f32 - 94.0, 24.0, 76.0, 76.0, Z_TEXT + 0.1),
        PIXEL_LAYER,
        Visibility::Hidden, // spin_preview shows it once frames land
        CreatorPreview,
    ));
    redraw(&mut commands, &ui, &mut images, &st, &bindings, &input);
}

/// Everything the creator spawns (UI layer + the preview sprite) — the cleanup filter.
type AnyCreatorEntity = Or<(With<CreatorUi>, With<CreatorPreview>)>;

fn close(mut commands: Commands, mut input: ResMut<ActionState>, ui: Query<Entity, AnyCreatorEntity>) {
    for e in &ui {
        commands.entity(e).despawn();
    }
    for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
        input.latch(a);
    }
}

/// Bake the preview frames + the per-facing head-centering offsets (js rebuild()):
/// side sprites sit off-centre, so the turntable anchors each facing by its head.
fn rebuild_preview(st: &mut CreatorState, images: &mut Assets<Image>) {
    let frames = build_frames(&st.look(), images);
    for (i, f) in frames.frames.iter().enumerate() {
        st.frame_off[i] = head_center_offset(images.get(&f[0]));
    }
    st.frames = Some(frames);
}

/// The head rows' horizontal centre vs the sprite centre (js centerOffset).
fn head_center_offset(img: Option<&Image>) -> f32 {
    let Some(img) = img else { return 0.0 };
    let Some(data) = img.data.as_ref() else { return 0.0 };
    let (w, head_rows) = (img.width() as usize, 6usize.min(img.height() as usize));
    let (mut min, mut max) = (w as i32, -1i32);
    for x in 0..w {
        for y in 0..head_rows {
            if data[(y * w + x) * 4 + 3] > 8 {
                min = min.min(x as i32);
                max = max.max(x as i32);
                break;
            }
        }
    }
    if max < 0 { 0.0 } else { w as f32 / 2.0 - (min + max + 1) as f32 / 2.0 }
}

/// The fixed-tick input driver (js Creator.update).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn creator_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    input: Res<ActionState>,
    bindings: Res<Bindings>,
    mut st: ResMut<CreatorState>,
    mut ident: ResMut<HeroIdent>,
    ui: Query<Entity, With<CreatorUi>>,
    mut next: ResMut<NextState<Screen>>,
    mut loads: MessageWriter<LoadSlot>,
) {
    st.spin += 1;
    let confirm = input.pressed(Action::Slot1);
    let cancel = input.pressed(Action::Slot2) || input.pressed(Action::Pause);
    let mut dirty = false;

    if st.editing {
        // The on-screen keyboard — one naming flow for pad and keys alike (the js).
        if cancel {
            st.editing = false;
            dirty = true;
        } else {
            if input.pressed(Action::Up) {
                st.kb_row = (st.kb_row + KB.len() - 1) % KB.len();
                dirty = true;
            }
            if input.pressed(Action::Down) {
                st.kb_row = (st.kb_row + 1) % KB.len();
                dirty = true;
            }
            if input.pressed(Action::Left) {
                st.kb_col = (st.kb_col + KB[st.kb_row].len() - 1) % KB[st.kb_row].len();
                dirty = true;
            }
            if input.pressed(Action::Right) {
                st.kb_col = (st.kb_col + 1) % KB[st.kb_row].len();
                dirty = true;
            }
            st.kb_col = st.kb_col.min(KB[st.kb_row].len() - 1);
            if confirm {
                match KB[st.kb_row][st.kb_col] {
                    "OK" => st.editing = false,
                    "DEL" => {
                        st.name.pop();
                    }
                    k if st.name.len() < 12 => st.name.push_str(if k == "_" { " " } else { k }),
                    _ => {}
                }
                dirty = true;
            }
        }
        if dirty {
            redraw(&mut commands, &ui, &mut images, &st, &bindings, &input);
        }
        return;
    }
    if cancel {
        next.set(Screen::Title); // back out — the title is still standing behind us
        return;
    }

    if input.pressed(Action::Up) {
        st.cursor = (st.cursor + N_FIELDS - 1) % N_FIELDS;
        dirty = true;
    }
    if input.pressed(Action::Down) {
        st.cursor = (st.cursor + 1) % N_FIELDS;
        dirty = true;
    }

    // Left/right cycles the value on a pick row (js dx).
    let dx = input.pressed(Action::Right) as i32 - input.pressed(Action::Left) as i32;
    if dx != 0 {
        let step = |i: usize, n: usize| (i as i32 + dx).rem_euclid(n as i32) as usize;
        let mut changed = true;
        match st.cursor {
            1 => st.gender_m = !st.gender_m,
            2 => st.hair = step(st.hair, HAIRS.len()),
            3 => st.style = step(st.style, STYLES.len()),
            4 => st.eye = step(st.eye, EYES.len()),
            5 => st.skin = step(st.skin, SKINS.len()),
            6 => st.outfit = step(st.outfit, OUTFITS.len()),
            _ => changed = false,
        }
        if changed {
            if st.cursor >= 2 {
                rebuild_preview(&mut st, &mut images);
            }
            dirty = true;
        }
    }

    if confirm {
        match st.cursor {
            0 => {
                st.editing = true;
                st.kb_row = 0;
                st.kb_col = 0;
                dirty = true;
            }
            1 => {
                st.gender_m = !st.gender_m;
                dirty = true;
            }
            7 => {
                let mut rng = std::mem::replace(&mut st.rng, Mulberry32::new(1));
                st.traits = traits::roll(&mut rng);
                st.rng = rng;
                dirty = true;
            }
            8 => {
                // START ADVENTURE: finish the hero (the BAZ egg lands HERE, invisible in
                // the preview) and hand the loader a fresh world.
                let mut look = st.look();
                let name = st.name.trim().to_string();
                if name.eq_ignore_ascii_case("baz") {
                    look.hair_style = "bald".into();
                }
                *ident = HeroIdent {
                    name: if name.is_empty() { "HERO".into() } else { name },
                    gender: if st.gender_m { "M".into() } else { "F".into() },
                    look,
                    traits: st.traits.clone(),
                };
                let seed = ((st.rng.next_f64() * u32::MAX as f64) as u32).max(1);
                loads.write(LoadSlot { slot: st.slot, fresh: true, seed: Some(seed) });
                return;
            }
            _ => {}
        }
    }
    if dirty {
        redraw(&mut commands, &ui, &mut images, &st, &bindings, &input);
    }
}

/// Physical-keyboard convenience while naming: Backspace deletes (js keydown listener).
fn backspace(keys: Res<ButtonInput<KeyCode>>, mut st: ResMut<CreatorState>, mut bump: Local<bool>) {
    // Route through a Local so the fixed-tick redraw notices (a pop alone won't).
    if st.editing && keys.just_pressed(KeyCode::Backspace) {
        st.name.pop();
        *bump = !*bump;
        st.kb_col = st.kb_col.min(KB[st.kb_row].len() - 1); // touch st -> change detection
    }
}

/// Turn the preview: facing swaps every 45 ticks (js SPIN), one sprite updated in place.
fn spin_preview(
    st: Res<CreatorState>,
    mut q: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<CreatorPreview>>,
) {
    let Some(frames) = &st.frames else { return };
    // js SPIN order: down, right, up, left -> our Facing indices 0,2,1,3.
    let order = [0usize, 2, 1, 3];
    let f = order[(st.spin / 45) as usize % 4];
    let Ok((mut sprite, mut tf, mut vis)) = q.single_mut() else { return };
    if sprite.image != frames.frames[f][0] {
        sprite.image = frames.frames[f][0].clone();
    }
    const S: f32 = 76.0;
    sprite.custom_size = Some(Vec2::splat(S));
    let (px, py) = (CANVAS_W as f32 - S - 18.0, 24.0);
    let ox = (st.frame_off[f] * (S / 16.0)).round();
    *tf = at(px + ox, py, S, S, Z_TEXT + 0.1);
    *vis = Visibility::Inherited;
}

type Pen<'a, 'w, 's> = UiPen<'a, 'w, 's, CreatorUi>;

/// Rebuild the whole screen (js Creator.draw). The preview sprite rides separately.
fn redraw(
    commands: &mut Commands,
    old: &Query<Entity, With<CreatorUi>>,
    images: &mut Assets<Image>,
    st: &CreatorState,
    bindings: &Bindings,
    input: &ActionState,
) {
    for e in old {
        commands.entity(e).despawn();
    }
    let (w, h) = (CANVAS_W as f32, CANVAS_H as f32);
    let mut pen = Pen { commands, images, marker: CreatorUi };
    pen.fill(0.0, 0.0, w, h, 0x0a0a12, Z_BG);
    let title = "CREATE YOUR HERO";
    let tw = font::measure(title) as f32 * 2.0;
    pen.text_scaled(title, ((w - tw) / 2.0).round(), 6.0, 0xfcfcfc, Z_TEXT, 2.0);

    // The seven value rows (js row()).
    let mut y = 24.0;
    let rh = 13.0;
    let rows: [(&str, String, Option<u32>); 7] = [
        ("NAME", if st.name.is_empty() { " ".into() } else { st.name.clone() }, None),
        ("GENDER", if st.gender_m { "MALE".into() } else { "FEMALE".into() }, None),
        ("HAIR", HAIRS[st.hair].0.into(), Some(HAIRS[st.hair].1)),
        ("STYLE", STYLES[st.style].0.into(), None),
        ("EYES", EYES[st.eye].0.into(), Some(EYES[st.eye].1)),
        ("SKIN", SKINS[st.skin].0.into(), Some(SKINS[st.skin].1)),
        ("OUTFIT", OUTFITS[st.outfit].0.into(), Some(OUTFITS[st.outfit].1)),
    ];
    for (i, (label, value, swatch)) in rows.iter().enumerate() {
        draw_row(&mut pen, label, value, *swatch, y, st.cursor == i);
        y += rh;
    }

    // Hero preview backing panel (the sprite itself is CreatorPreview).
    const S: f32 = 76.0;
    let (px, py) = (w - S - 18.0, 24.0);
    pen.fill(px - 8.0, py - 6.0, S + 16.0, S + 18.0, 0x141420, Z_TEXT - 0.1);
    for (sx, sy, sw, sh) in crate::ui::border_strips(px - 8.0, py - 6.0, S + 16.0, S + 18.0, 1.0) {
        pen.fill(sx, sy, sw, sh, 0x2a2a3a, Z_TEXT - 0.05);
    }
    pen.fill_rgba(px + 12.0, py + S, S - 24.0, 5.0, Color::srgba(0.0, 0.0, 0.0, 0.55), Z_TEXT - 0.05);

    // Traits.
    pen.text("TRAITS", 18.0, 114.0, GOLD, Z_TEXT);
    for (i, key) in st.traits.iter().enumerate() {
        let Some(d) = traits::get(key) else { continue };
        let ty = 124.0 + i as f32 * 11.0;
        let col = if d.good { 0x5adc6a } else { 0xfc6868 };
        let sign = if d.good { "+ " } else { "- " };
        pen.text(&format!("{sign}{}", d.name.to_uppercase()), 18.0, ty, col, Z_TEXT);
        pen.text(d.desc, 120.0, ty, 0x9a9aa0, Z_TEXT);
    }

    draw_row(&mut pen, "", "REROLL TRAITS", None, 170.0, st.cursor == 7);
    draw_row(&mut pen, "", "START ADVENTURE", None, 184.0, st.cursor == 8);

    // On-screen keyboard overlay while naming.
    if st.editing {
        let (kw, kh) = (18.0, 15.0);
        let kbx = ((w - KB[0].len() as f32 * kw) / 2.0).round();
        let kby = 112.0;
        pen.fill_rgba(
            kbx - 6.0,
            kby - 16.0,
            KB[0].len() as f32 * kw + 12.0,
            KB.len() as f32 * kh + 24.0,
            Color::srgba(0.0, 0.0, 0.0, 0.94),
            Z_TEXT + 0.2,
        );
        let head = format!("NAME: {}", st.name);
        pen.text(&head, kbx, kby - 12.0, 0xfcfcfc, Z_TEXT + 0.3);
        if (st.spin >> 4) & 1 == 1 {
            pen.fill(kbx + font::measure(&head) as f32 + 1.0, kby - 12.0, 4.0, 5.0, GOLD, Z_TEXT + 0.3);
        }
        for (r, row) in KB.iter().enumerate() {
            for (c, k) in row.iter().enumerate() {
                let (x, y) = (kbx + c as f32 * kw, kby + r as f32 * kh);
                let on = r == st.kb_row && c == st.kb_col;
                pen.fill(x, y, kw - 2.0, kh - 2.0, if on { 0x3a3a4c } else { 0x1a1a26 }, Z_TEXT + 0.25);
                if on {
                    for (sx, sy, sw, sh) in crate::ui::border_strips(x, y, kw - 2.0, kh - 2.0, 1.0) {
                        pen.fill(sx, sy, sw, sh, GOLD, Z_TEXT + 0.28);
                    }
                }
                let label = if *k == "_" { "SP" } else { k };
                let lx = x + ((kw - 2.0 - font::measure(label) as f32) / 2.0).round();
                pen.text(label, lx, y + 4.0, if on { 0xfcfcfc } else { 0xb0b0b8 }, Z_TEXT + 0.3);
            }
        }
    }

    let conf = bindings.prompt(Action::Slot1, input.pad_present);
    let back = bindings.prompt(Action::Slot2, input.pad_present);
    let hint = if st.editing {
        format!("▲▼◀▶ MOVE   {conf} SELECT   {back} DONE")
    } else {
        format!("▲▼ SELECT   ◀▶ CHANGE   {conf} CONFIRM   {back} BACK")
    };
    pen.text_center(&hint, w / 2.0, h - 11.0, 0x606070, Z_TEXT + 0.3);
}

/// One creator row (js row()): cursor, label, optional colour swatch, `< value >`.
fn draw_row(pen: &mut Pen, label: &str, value: &str, swatch: Option<u32>, y: f32, sel: bool) {
    if sel {
        pen.text(">", 8.0, y, GOLD, Z_TEXT);
    }
    pen.text(label, 18.0, y, if sel { GOLD } else { 0x9a9aa0 }, Z_TEXT);
    let mut vx = 66.0;
    if sel && !label.is_empty() {
        pen.text("<", vx, y, GOLD, Z_TEXT);
        vx += 6.0;
    }
    if let Some(c) = swatch {
        pen.fill(vx, y - 1.0, 7.0, 7.0, c, Z_TEXT);
        for (sx, sy, sw, sh) in crate::ui::border_strips(vx - 1.0, y - 2.0, 9.0, 9.0, 1.0) {
            pen.fill(sx, sy, sw, sh, 0x000000, Z_TEXT + 0.02);
        }
        vx += 11.0;
    }
    pen.text(value, vx, y, 0xfcfcfc, Z_TEXT);
    if sel && !label.is_empty() {
        pen.text(">", vx + font::measure(value) as f32 + 3.0, y, GOLD, Z_TEXT);
    }
}
