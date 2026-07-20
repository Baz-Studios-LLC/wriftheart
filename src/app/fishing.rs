//! fishing.rs — the rod's cast -> bite -> tap loop (js startFishing/updateFishing/
//! resolveCatch + drawFishing). Equip the rod, face water, press its slot: the bobber
//! sails out and the WORLD KEEPS RUNNING — you stand rooted and vulnerable, and a hit
//! snaps the line. A bite dips the float and flashes "!": tap either of the first two
//! slots inside the window (tighter for rarer fish) to land it.
//!
//! What bites is items::roll_fish (water x biome x season x LIVE weather — rainfish
//! bite in the rain, voidfin only under a thunderstorm).

use super::battle::GameRng;
use super::play::{CurGrid, CurRoom, GameWorld, Player};
use super::room_render::{FrameClock, PLAY_X, PLAY_Y};
use super::screen::playing;
use crate::combat::Health;
use crate::gfx::{at, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use crate::items::Catch;
use crate::room::{COLS, PX_H, PX_W, ROWS, TILE};
use bevy::prelude::*;

/// The active cast (None = rod stowed). js `fishing`.
#[derive(Resource, Default)]
pub struct Fishing(pub Option<FishState>);

pub struct FishState {
    phase: Phase,
    t: u32,
    bx: f32, // bobber, room px
    by: f32,
    bite_at: u32,
    win: u32,
    water: &'static str,
    hooked: Option<Catch>,
    hp: i32, // a hit while rooted snaps the line
}

#[derive(PartialEq)]
enum Phase {
    Cast,
    Bite,
}

/// Everything the cast drew (bobber, line, alert, prompt) — despawned when it ends.
#[derive(Component)]
struct FishFx;

#[derive(Component)]
struct Bobber;

#[derive(Component)]
struct BiteAlert;

#[derive(Component)]
struct PromptBar;

pub struct FishingPlugin;

impl Plugin for FishingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Fishing>().add_systems(
            bevy::app::FixedUpdate,
            fish_tick.before(super::play::EndTick).run_if(playing),
        );
    }
}

/// js seasonName — the calendar's quarter (codex calendar_tab owns the index).
fn season_name(clock: i64) -> &'static str {
    ["SPRING", "SUMMER", "FALL", "WINTER"][super::codex::calendar_tab::season_index(clock) % 4]
}

/// The tile the player faces (js fishFrontTile), room coords.
fn front_tile(p: &Player) -> (i32, i32) {
    let (dx, dy) = match p.facing {
        crate::actors::hero::Facing::Up => (0, -1),
        crate::actors::hero::Facing::Down => (0, 1),
        crate::actors::hero::Facing::Left => (-1, 0),
        crate::actors::hero::Facing::Right => (1, 0),
    };
    (((p.x + 8.0) / 16.0).floor() as i32 + dx, ((p.y + 12.0) / 16.0).floor() as i32 + dy)
}

/// The read-only world context of a cast, bundled (Bevy's 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
struct CastCtx<'w> {
    clock: Res<'w, FrameClock>,
    cur: Res<'w, CurRoom>,
    world: Res<'w, GameWorld>,
    in_dungeon: Res<'w, super::dungeon::InDungeon>,
    weather: Res<'w, super::weather::WeatherState>,
}

/// The whole loop, one system: rod-slot press casts; the wait ticks in the live world;
/// the bite window resolves. Presses are CONSUMED so nothing swings when the world thaws.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn fish_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fishing: ResMut<Fishing>,
    mut input: ResMut<ActionState>,
    mut rng: ResMut<GameRng>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    ctx: CastCtx,
    grid: Res<CurGrid>,
    players: Query<(&Player, &Health)>,
    fx: Query<Entity, With<FishFx>>,
    bars: Query<Entity, With<PromptBar>>,
    mut bobbers: Query<&mut Transform, With<Bobber>>,
    mut alerts: Query<&mut Visibility, With<BiteAlert>>,
    bindings: Res<crate::input::Bindings>,
) {
    let Ok((p, health)) = players.single() else { return };
    let end = |commands: &mut Commands, fishing: &mut Fishing, fx: &Query<Entity, With<FishFx>>| {
        fishing.0 = None;
        for e in fx {
            commands.entity(e).despawn();
        }
    };

    // --- No cast in flight: does a rod-slot press start one? ---
    if fishing.0.is_none() {
        let mut cast_pressed = false;
        for (i, action) in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4].into_iter().enumerate() {
            if input.pressed(action)
                && inv.slots[i].and_then(|uid| inv.id_of(uid)) == Some("fishingrod")
                && p.cooldowns[i] == 0
            {
                input.consume(action);
                cast_pressed = true;
                break;
            }
        }
        if !cast_pressed {
            return;
        }
        if ctx.in_dungeon.0.is_some() {
            log.add("fish", "NOTHING SWIMS DOWN HERE", 1, 0x8ab0d0, false, true);
            return;
        }
        let (c, r) = front_tile(p);
        let code = grid.0.code_at(c, r);
        if code != '~' && code != 'B' {
            log.add("fish", "FACE THE WATER TO CAST", 1, 0x8ab0d0, false, true);
            return;
        }
        let (gx, gy) = (ctx.cur.rx * COLS + c, ctx.cur.ry * ROWS + r);
        let water: &'static str = if ctx.world.0.water_style(gx, gy) == "murk" { "murk" } else { "blue" };
        let bite_at = 55 + (rng.0.next_f64() * 150.0) as u32; // (js lure gear shortens this — with the trinket port)
        let (bx, by) = ((c * TILE + 8) as f32, (r * TILE + 9) as f32);
        fishing.0 = Some(FishState { phase: Phase::Cast, t: 0, bx, by, bite_at, win: 0, water, hooked: None, hp: health.hp });
        // The scene: bobber (red cap / white float), a faint line from the rod hand, and
        // the prompt bar. (The js line is a live 1px stroke; ours is a thin rotated quad.)
        let (hx, hy) = (p.x + 8.0, p.y + 2.0);
        let (mx, my) = ((hx + bx) / 2.0, (hy + by) / 2.0);
        let len = ((bx - hx).powi(2) + (by - hy).powi(2)).sqrt().max(1.0);
        let ang = (-(by - hy)).atan2(bx - hx);
        let mut line_tf = at(PLAY_X + mx - len / 2.0, PLAY_Y + my - 0.5, len, 1.0, 8.9);
        line_tf.rotation = Quat::from_rotation_z(ang);
        commands.spawn((
            Sprite::from_color(Color::srgba(0.93, 0.93, 0.93, 0.55), Vec2::new(len, 1.0)),
            line_tf,
            PIXEL_LAYER,
            FishFx,
        ));
        let be = commands
            .spawn((
                Sprite::from_color(Color::srgb_u8(0xe8, 0x38, 0x38), Vec2::new(2.0, 3.0)),
                at(PLAY_X + bx - 1.0, PLAY_Y + by - 3.0, 2.0, 3.0, 9.0),
                PIXEL_LAYER,
                FishFx,
                Bobber,
            ))
            .id();
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(0xf4, 0xf4, 0xf4), Vec2::new(2.0, 2.0)),
            Transform::from_translation(Vec3::new(0.0, -2.5, 0.01)),
            ChildOf(be),
            PIXEL_LAYER,
        ));
        // The "!" alert, hidden until the bite.
        let (img, _w) = crate::gfx::font::bake_text("!", 0xfcd000, &mut images);
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + bx - 1.0, PLAY_Y + by - 13.0, 4.0, 7.0, crate::gfx::layers::PROMPT),
            PIXEL_LAYER,
            Visibility::Hidden,
            FishFx,
            BiteAlert,
        ));
        let msg = format!("WAIT FOR A BITE - {} REEL IN", bindings.prompt(Action::Slot2, input.pad_present));
        prompt_bar(&mut commands, &mut images, &msg, 0xcfe0ec);
        return;
    }

    // --- A cast in flight. ---
    let Some(f) = &mut fishing.0 else { return };
    // The world runs live while you fish: a hit snaps the line (js).
    if health.hp < f.hp {
        end(&mut commands, &mut fishing, &fx);
        log.add("fish", "THE LINE SNAPS!", 1, 0xfc6868, false, true);
        return;
    }
    f.hp = health.hp;
    f.t += 1;
    // Bobber idle sway / bite dip (js dip math).
    let dip = if f.phase == Phase::Bite {
        ((ctx.clock.0 as f32) * 0.9).sin().max(0.0) * 3.0
    } else {
        ((ctx.clock.0 as f32) * 0.12).sin() * 1.0
    };
    if let Ok(mut tf) = bobbers.single_mut() {
        tf.translation.y = crate::gfx::at(PLAY_X + f.bx - 1.0, PLAY_Y + f.by - 3.0 + dip, 2.0, 3.0, 9.0).translation.y;
    }
    let tapped = input.pressed(Action::Slot1) || input.pressed(Action::Slot2);
    if tapped {
        input.consume(Action::Slot1);
        input.consume(Action::Slot2);
    }
    match f.phase {
        Phase::Cast => {
            if tapped {
                end(&mut commands, &mut fishing, &fx); // reel the empty line back in
                return;
            }
            if f.t >= f.bite_at {
                // A fish bites! Rarer fish = a tighter reaction window.
                let biome = ctx.world.0.biome_key_at(ctx.cur.rx, ctx.cur.ry);
                let catch = crate::items::roll_fish(biome, season_name(ctx.clock.0), ctx.weather.cur, f.water, || rng.0.next_f64());
                f.win = match &catch {
                    Catch::Fish { rarity, .. } => match rarity {
                        crate::items::Rarity::Epic | crate::items::Rarity::Legendary => 14,
                        crate::items::Rarity::Rare => 18,
                        crate::items::Rarity::Uncommon => 22,
                        crate::items::Rarity::Common => 27,
                    },
                    Catch::Junk(_) => 27,
                };
                f.hooked = Some(catch);
                f.phase = Phase::Bite;
                f.t = 0;
                for mut v in &mut alerts {
                    *v = Visibility::Inherited;
                }
                // The bar flips to the hook prompt (js: Input.prompt('slot1') + ' HOOK IT!').
                for e in &bars {
                    commands.entity(e).despawn();
                }
                let msg = format!("{} HOOK IT!", bindings.prompt(Action::Slot1, input.pad_present));
                prompt_bar(&mut commands, &mut images, &msg, 0xfcd000);
            }
        }
        Phase::Bite => {
            let resolve = tapped || f.t >= f.win;
            if !resolve {
                return;
            }
            let hooked = f.hooked.take();
            let ok = tapped;
            end(&mut commands, &mut fishing, &fx);
            match (ok, hooked) {
                (true, Some(Catch::Fish { id, name, rarity, lb })) => {
                    inv.add_item(id, 1);
                    log.add("fish", &format!("CAUGHT {}  {lb} LB", name.to_uppercase()), 1, rarity.color(), false, true);
                }
                (true, Some(Catch::Junk(id))) => {
                    inv.add_item(id, 1);
                    let name = crate::items::get(id).map(|d| d.name.to_uppercase()).unwrap_or_default();
                    log.add("fish", &format!("SNAGGED {name}"), 1, 0x9a9aa0, false, true);
                }
                _ => {
                    log.add("fish", "IT GOT AWAY", 1, 0x8ab0d0, false, true);
                }
            }
        }
    }
}

/// The bottom-centre prompt bar (js drawFishing's message strip).
fn prompt_bar(commands: &mut Commands, images: &mut Assets<Image>, msg: &str, col: u32) {
    let w = crate::gfx::font::measure(msg) as f32;
    let mx = (PLAY_X + (PX_W as f32 - w) / 2.0).round();
    let my = PLAY_Y + PX_H as f32 - 12.0;
    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.72), Vec2::new(w + 6.0, 9.0)),
        at(mx - 3.0, my - 1.0, w + 6.0, 9.0, crate::gfx::layers::PROMPT),
        PIXEL_LAYER,
        FishFx,
        PromptBar,
    ));
    crate::ui::label(commands, images, msg, mx, my, col, crate::gfx::layers::PROMPT + 0.01, (FishFx, PromptBar));
}
