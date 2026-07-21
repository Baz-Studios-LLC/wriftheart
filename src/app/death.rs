//! death.rs — YOU DIED (js startDeath / updateDeath / drawDeath + deathlines.js).
//!
//! Falling: half your coin and all progress toward the next level are lost (the level
//! stays), and everything in the BAG explodes out around the corpse (equipped slots and
//! gear stay yours). The world drains dark while a blood pool grows under the fallen
//! hero, YOU DIED fades in over a random epitaph and the itemized toll, then
//! CONTINUE / TITLE SCREEN. Respawn is the start room, full HP, saved immediately.
//!
//! DEVIATION (flagged): the js drains the play area via a canvas 'saturation' composite;
//! sprites have no such pass, so a grey wash + the dark fade stand in.

use super::gather::spawn_pickup;
use super::battle::RoomActor;
use super::play::{HeroArt, Player};
use super::room_render::{FrameClock, PLAY_X, PLAY_Y};
use super::save::{write_save, SaveCtx};
use super::screen::{playing, Screen};
use super::title::loader::{swap_world_room, SwapCtx};
use crate::combat::{Health, Hitbox};
use crate::deathlines;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use crate::room::{PX_H, PX_W};
use crate::ui::label;
use crate::worldgen::rng::Mulberry32;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const Z_GREY: f32 = 12.85; // over the play field's actors + FX
const Z_DARK: f32 = 12.9;
const Z_POOL: f32 = 12.95;
const Z_CORPSE: f32 = 12.97;
const Z_TEXT: f32 = 18.8; // over the HUD, under the pause band

/// The active death sequence (js `death`). Inserted by check_death, removed on exit.
#[derive(Resource)]
pub struct DeathState {
    t: u32,
    choice: usize,
    line: &'static str,
    /// Who did it (display name) — None (a nameless end) skips the line.
    killer: Option<&'static str>,
    coin_lost: i64,
    xp_lost: i32,
    items_dropped: i32,
}

/// The previous epitaph's index — DeathLines.random() never repeats itself back-to-back.
#[derive(Resource, Default)]
struct LastLine(Option<usize>);

#[derive(Component, Clone)]
struct DeathUi;

/// Animated pieces, updated from `DeathState.t` each tick (js redraws; we retune sprites).
#[derive(Component)]
enum DeathFx {
    Grey,   // saturation-drain stand-in over the play area
    Dark,   // full-canvas fade
    Pool,   // the growing blood ellipse
    Title,  // YOU DIED (fades in from t=38)
    Line,   // the epitaph (fades from t=46)
}

/// The two menu rows (rebuilt when the choice flips).
#[derive(Component)]
struct DeathChoice;

/// Where the hero last FELL — (overworld room, farm day). Session-only and
/// same-day-only, exactly like the corpse bag it points at: room snapshots don't
/// cross a load, and the next day's room reset sweeps the bag — the map's
/// gravestone marker dies with it (Baz). Dungeon deaths don't mark (runs regen).
#[derive(Resource, Default)]
pub struct LastDeath(pub Option<((i32, i32), i64)>);

/// Any slot load (or new game) drops the marker — the bag's snapshot didn't
/// cross either.
fn clear_death_mark(mut loads: MessageReader<super::title::loader::LoadSlot>, mut ld: ResMut<LastDeath>) {
    if loads.read().next().is_some() {
        ld.0 = None;
    }
}

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LastLine>()
            .init_resource::<LastDeath>()
            .add_systems(
                bevy::app::FixedUpdate,
                (
                    clear_death_mark,
                    check_death.run_if(playing),
                    death_tick.before(super::play::EndTick).run_if(in_state(Screen::Dead)),
                ),
            )
            .add_systems(OnExit(Screen::Dead), close_death);
    }
}

/// The hero hit zero: apply the toll, scatter the bag, raise the overlay (js startDeath).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn check_death(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut progress: ResMut<super::rewards::Progress>,
    mut last: ResMut<LastLine>,
    clock: Res<FrameClock>,
    hero_art: Res<HeroArt>,
    mut players: Query<(&Player, &Health, &mut Visibility)>,
    existing: Option<Res<DeathState>>,
    cur: Res<super::play::CurRoom>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    mut last_death: ResMut<LastDeath>,
    last_attacker: Res<crate::combat::LastAttacker>,
    mut next: ResMut<NextState<Screen>>,
) {
    let Ok((p, h, mut vis)) = players.single_mut() else { return };
    // The Screen::Dead transition lags next.set by a frame — without this guard a second
    // pass would re-run the toll on the already-emptied bag and zero the numbers.
    if h.hp > 0 || existing.is_some() {
        return;
    }
    // Pin the fall on the map (overworld only — a dungeon run regenerates, so
    // there's no bag to walk back to).
    if in_dungeon.0.is_none() {
        last_death.0 = Some(((cur.rx, cur.ry), super::gather::farm_day(clock.0)));
    }
    let mut rng = Mulberry32::new(clock.0 as u32 ^ 0x9e3779b1);

    // The toll: half the purse, all progress toward the next level (the level stays).
    let coin_lost = inv.money / 2;
    inv.money -= coin_lost;
    let xp_lost = progress.xp;
    progress.xp = 0;

    // The bag explodes out and is lost — equipped ability slots + worn gear stay.
    let equipped: Vec<u32> = inv.slots.iter().chain(inv.gear.iter()).flatten().copied().collect();
    let bag_uids: Vec<u32> = inv.bag.iter().flatten().copied().collect();
    let mut items_dropped = 0;
    for uid in &bag_uids {
        let Some(e) = inv.entries.iter().find(|e| e.uid == *uid) else { continue };
        items_dropped += e.qty;
        let a = rng.next_f64() * std::f64::consts::TAU;
        let dist = 8.0 + rng.next_f64() * 22.0;
        let (dx, dy) = (p.x + (a.cos() * dist) as f32, p.y + (a.sin() * dist) as f32);
        let drop = spawn_pickup(
            &mut commands,
            &mut images,
            e.id,
            e.qty,
            dx.clamp(4.0, (PX_W - 20) as f32),
            dy.clamp(4.0, (PX_H - 20) as f32),
            false,
            None,
        );
        // CORPSE RUN (Baz, 2026-07-16 — deviates from the js 20s expiry): the scattered
        // bag never blinks out. It lies where you fell — walk back and reclaim it. The
        // room cache carries it across re-entry; DAWN's world refresh is the deadline.
        commands.entity(drop).entry::<super::gather::Pickup>().and_modify(|mut pk| pk.life = u32::MAX);
    }
    inv.entries.retain(|e| equipped.contains(&e.uid));
    let cap = inv.bag.len();
    inv.bag = vec![None; cap];
    info!("death: {coin_lost} coin, {xp_lost} xp, {items_dropped} items scattered from {} bag stacks", bag_uids.len());

    // A fresh epitaph (never the same twice running).
    let mut idx = (rng.next_f64() * deathlines::LINES.len() as f64) as usize % deathlines::LINES.len();
    if last.0 == Some(idx) {
        idx = (idx + 1) % deathlines::LINES.len();
    }
    last.0 = Some(idx);

    // --- The scene: grey wash, dark fade, blood pool, the fallen hero on his side. ---
    *vis = Visibility::Hidden; // the corpse sprite stands in for the live body
    let (w, h_c) = (crate::CANVAS_W as f32, crate::CANVAS_H as f32);
    commands.spawn((
        Sprite::from_color(Color::srgba(0.5, 0.5, 0.5, 0.0), Vec2::new(PX_W as f32, PX_H as f32)),
        at(PLAY_X, PLAY_Y, PX_W as f32, PX_H as f32, Z_GREY),
        PIXEL_LAYER,
        DeathUi,
        DeathFx::Grey,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::new(w, h_c)),
        at(0.0, 0.0, w, h_c, Z_DARK),
        PIXEL_LAYER,
        DeathUi,
        DeathFx::Dark,
    ));
    let pool = images.add(pool_image());
    commands.spawn((
        Sprite { image: pool, custom_size: Some(Vec2::new(10.0, 5.0)), ..default() },
        at(PLAY_X + p.x + 8.0 - 5.0, PLAY_Y + p.y + 13.0 - 2.5, 10.0, 5.0, Z_POOL),
        PIXEL_LAYER,
        DeathUi,
        DeathFx::Pool,
    ));
    let mut corpse_tf = at(PLAY_X + p.x, PLAY_Y + p.y + 3.0, 16.0, 16.0, Z_CORPSE);
    corpse_tf.rotation = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2); // lying down
    commands.spawn((Sprite::from_image(hero_art.0.frames[0][0].clone()), corpse_tf, PIXEL_LAYER, DeathUi));

    commands.insert_resource(DeathState {
        t: 0,
        choice: 0,
        line: deathlines::LINES[idx],
        killer: last_attacker.0,
        coin_lost,
        xp_lost,
        items_dropped,
    });
    next.set(Screen::Dead);
}

/// Drive the sequence: fades, the toll reveal at t=52, then the CONTINUE/TITLE choice
/// (js updateDeath + the time-keyed parts of drawDeath).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn death_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    input: Res<ActionState>,
    mut death: ResMut<DeathState>,
    mut ctx: SaveCtx,
    extras: super::save::SaveExtras,
    mut swap: SwapCtx,
    actors: Query<Entity, With<RoomActor>>,
    mut players: Query<(&mut Player, &mut Health, &mut Hitbox, &mut crate::combat::Knockback)>,
    mut fx: Query<(&DeathFx, &mut Sprite, &mut Transform)>,
    choices: Query<Entity, With<DeathChoice>>,
    mut next: ResMut<NextState<Screen>>,
    ptr: Res<crate::input::Pointer>,
) {
    death.t += 1;
    let t = death.t;
    let p_fade = (t as f32 / 46.0).min(1.0);

    // --- Animate the scene from the shared timer. ---
    for (kind, mut sprite, mut tf) in &mut fx {
        match kind {
            DeathFx::Grey => sprite.color = Color::srgba(0.5, 0.5, 0.5, 0.4 * p_fade),
            DeathFx::Dark => sprite.color = Color::srgba(0.0, 0.0, 0.0, 0.6 * p_fade),
            DeathFx::Pool => {
                let r = 5.0 + p_fade * 16.0;
                let base = tf.translation; // keep the centre, grow the size
                sprite.custom_size = Some(Vec2::new(r * 2.0, r));
                tf.translation = base;
            }
            DeathFx::Title => sprite.color = Color::srgba(1.0, 1.0, 1.0, ((t as f32 - 38.0) / 16.0).clamp(0.0, 1.0)),
            DeathFx::Line => sprite.color = Color::srgba(1.0, 1.0, 1.0, ((t as f32 - 46.0) / 14.0).clamp(0.0, 1.0)),
        }
    }

    // --- One-shot spawns on the js schedule. ---
    let (w, _) = (crate::CANVAS_W as f32, crate::CANVAS_H as f32);
    let cx = w / 2.0;
    if t == 39 {
        // YOU DIED, scale 3, blood red, fading in (js centerText scale 3 at y 52).
        let (img, tw) = font::bake_text("YOU DIED", 0xb41818, &mut images);
        let iw = (tw + (tw & 1)) as f32;
        commands.spawn((
            Sprite {
                image: img,
                custom_size: Some(Vec2::new(iw * 3.0, 18.0)),
                color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                ..default()
            },
            at((cx - iw * 3.0 / 2.0).round(), 52.0, iw * 3.0, 18.0, Z_TEXT),
            PIXEL_LAYER,
            DeathUi,
            DeathFx::Title,
        ));
    }
    if t == 47 && !death.line.is_empty() {
        let (img, tw) = font::bake_text(death.line, 0xe8dcc0, &mut images);
        let iw = (tw + (tw & 1)) as f32;
        commands.spawn((
            Sprite { image: img, color: Color::srgba(1.0, 1.0, 1.0, 0.0), ..default() },
            at((cx - iw / 2.0).round(), 80.0, iw, 6.0, Z_TEXT),
            PIXEL_LAYER,
            DeathUi,
            DeathFx::Line,
        ));
        // The culprit, when the blow had a name (Baz: "YOU WERE KILLED BY A [MOB]").
        if let Some(name) = death.killer {
            let phrase = if name.starts_with("THE ") {
                format!("YOU WERE KILLED BY {name}")
            } else if name.starts_with(['A', 'E', 'I', 'O', 'U']) {
                format!("YOU WERE KILLED BY AN {name}")
            } else {
                format!("YOU WERE KILLED BY A {name}")
            };
            let (img, tw) = font::bake_text(&phrase, 0xb8a8a0, &mut images);
            let iw = (tw + (tw & 1)) as f32;
            commands.spawn((
                Sprite { image: img, color: Color::srgba(1.0, 1.0, 1.0, 0.0), ..default() },
                at((cx - iw / 2.0).round(), 70.0, iw, 6.0, Z_TEXT),
                PIXEL_LAYER,
                DeathUi,
                DeathFx::Line,
            ));
        }
    }
    if t == 52 {
        // The itemized toll + the menu.
        for (i, line) in [
            format!("XP LOST: {}", death.xp_lost),
            format!("COIN LOST: {}", death.coin_lost),
            format!("ITEMS DROPPED: {}", death.items_dropped),
        ]
        .iter()
        .enumerate()
        {
            let lx = (cx - font::measure(line) as f32 / 2.0).round();
            label(&mut commands, &mut images, line, lx, 98.0 + i as f32 * 10.0, 0xc87878, Z_TEXT, DeathUi);
        }
        spawn_choices(&mut commands, &mut images, death.choice, cx);
    }

    if t < 52 {
        return; // let the fall + fade play before menu input (js)
    }
    if input.pressed(Action::Up) || input.pressed(Action::Down) {
        death.choice ^= 1;
        for e in &choices {
            commands.entity(e).despawn();
        }
        spawn_choices(&mut commands, &mut images, death.choice, cx);
    }
    // Mouse: hover an option highlights it, a click confirms it (rows centred on cx, y=138+i*18).
    let mut opt_click = false;
    for i in 0..2 {
        if ptr.over(cx - 55.0, 138.0 + i as f32 * 18.0 - 3.0, 110.0, 14.0) {
            if ptr.moved && death.choice != i {
                death.choice = i;
                for e in &choices {
                    commands.entity(e).despawn();
                }
                spawn_choices(&mut commands, &mut images, death.choice, cx);
            }
            if ptr.click {
                death.choice = i;
                opt_click = true;
            }
        }
    }
    if input.pressed(Action::Slot1) || opt_click {
        // Revive at the start room, full HP, save the (now-emptied) bag — then title if
        // that was the pick (js respawn() runs first either way).
        let to_title = death.choice == 1;
        let Ok((mut p, mut h, mut hb, mut kb)) = players.single_mut() else { return };
        h.hp = h.max;
        h.invuln = 0;
        h.flash = 0;
        kb.timer = 0; // the killing blow's shove dies with you (js respawn: knockTimer = 0)
        // SET SPAWN (bed or inn): wake at the chosen point; else the start room (js).
        let set = extras.respawn.0.as_ref().map(|r| (r.room, r.x, r.y));
        let (room, sx, sy) = set.unwrap_or(((0, 0), (PX_W / 2 - 8) as f32, (PX_H / 2 - 8) as f32));
        p.x = sx;
        p.y = sy;
        p.facing = crate::actors::hero::Facing::Down;
        p.cooldowns = [0; 4];
        p.lock_timer = 0;
        *hb = Hitbox { x: p.x + 3.0, y: p.y + 2.0, w: 10.0, h: 13.0 };
        swap_world_room(&mut commands, &mut images, &mut swap, &mut ctx, &extras.caves, &extras.songs, &actors, room.0, room.1, extras.house.0.as_ref().map(|h| h.room));
        write_save(&ctx, &extras, &p, &h, swap.world.0.seed);
        commands.remove_resource::<DeathState>();
        next.set(if to_title { Screen::Title } else { Screen::Play });
    }
}

/// The CONTINUE / TITLE SCREEN rows (js opts loop — `> CONTINUE <` when hot).
fn spawn_choices(commands: &mut Commands, images: &mut Assets<Image>, choice: usize, cx: f32) {
    for (i, opt) in ["CONTINUE", "TITLE SCREEN"].iter().enumerate() {
        let on = i == choice;
        let text = if on { format!("> {opt} <") } else { opt.to_string() };
        let lx = (cx - font::measure(&text) as f32 / 2.0).round();
        label(
            commands,
            images,
            &text,
            lx,
            138.0 + i as f32 * 18.0,
            if on { 0xfce0a8 } else { 0x7a7a7a },
            Z_TEXT,
            (DeathUi, DeathChoice),
        );
    }
}

/// Any way out (respawned to play OR to the title): clear the scene, show the hero again.
fn close_death(
    mut commands: Commands,
    mut input: ResMut<ActionState>,
    ui: Query<Entity, With<DeathUi>>,
    mut players: Query<&mut Visibility, With<Player>>,
) {
    for e in &ui {
        commands.entity(e).despawn();
    }
    if let Ok(mut vis) = players.single_mut() {
        *vis = Visibility::Inherited;
    }
    for a in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4] {
        input.latch(a);
    }
}

/// The two-tone blood ellipse (js drawDeath's pool), baked once and scaled with the fade.
fn pool_image() -> Image {
    let (w, h) = (64u32, 32u32);
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = ((x as f32 + 0.5 - cx) / cx, (y as f32 + 0.5 - cy) / cy);
            let d = dx * dx + dy * dy;
            let i = ((y * w + x) * 4) as usize;
            if d <= 0.62 * 0.62 {
                buf[i..i + 4].copy_from_slice(&[0xa8, 0x14, 0x14, 214]); // bright core (0.82)
            } else if d <= 1.0 {
                buf[i..i + 4].copy_from_slice(&[0x6a, 0x00, 0x00, 209]); // dark rim (0.8)
            }
        }
    }
    Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        buf,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
