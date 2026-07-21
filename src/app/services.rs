//! services.rs — interior interactables (js updateInterior's zone loop + doInteract):
//! stand at a counter/altar/bed and press INTERACT. Ported services: SHOP (opens the
//! buy/sell window), REST (the inn: pay by depth, sleep to morning), HEAL (the chapel
//! blessing), BED (sleep at home). Plus the js doSleep fade — the world freezes under
//! it (screen::playing checks Sleeping).
//!
//! DEVIATIONS (flagged): storage/wandtable/bard/lorevendor counters stay silent until
//! their systems port (js prompts them); a bed just sleeps — the sleep-and-set-spawn
//! chooser joins with the respawn-point port; guild/festival discounts join with their
//! systems (the KEEPER's own hearts discount is live — shop::keeper_discount).

use super::gather::{Pickup, PickupKind, DAY_LEN};
use super::interior::{DoorCooldown, Inside};
use super::play::{CurRoom, Player};
use super::room_render::FrameClock;
use super::screen::{playing, Screen};
use super::shop::{stock_up, BoughtShop, ShopState};
use crate::combat::Health;
use crate::gfx::{at, layers, PIXEL_LAYER};
use crate::input::{Action, ActionState, Bindings};
use bevy::prelude::*;

const SLEEP_FADE: u32 = 38;
const SLEEP_HOLD: u32 = 36;

/// The in-flight sleep fade (js `sleeping`). Some -> the world holds its breath.
#[derive(Resource, Default)]
pub struct Sleeping(pub Option<SleepFx>);

pub struct SleepFx {
    t: u32,
    applied: bool,
    /// Heal target as a fraction of max HP (bed = 1.0 full; sleeping bag = 0.5 rough).
    heal_frac: f32,
}

/// A sleeping bag / bedroll use (js sleepingbag use()) — an OVERWORLD-only rough rest.
/// The handler vetoes (dungeon/interior, or foes about) + consumes only on success.
#[derive(Message)]
pub struct SleepRequest;

#[derive(Component, Clone)]
pub(crate) struct PromptBar;

#[derive(Component)]
struct SleepShade;

#[derive(Component)]
struct SleepZzz;

/// Any sleep-overlay entity (the shade or the Z Z Z) — the teardown sweep.
type AnySleepEntity = Or<(With<SleepShade>, With<SleepZzz>)>;

pub struct ServicesPlugin;

impl Plugin for ServicesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sleeping>().add_message::<SleepRequest>().add_systems(
            bevy::app::FixedUpdate,
            (
                // After prompts: a grabbed tome's press must not also work the counter.
                interact_tick
                    .after(super::prompts::prompt_tick)
                    .before(super::play::EndTick)
                    .run_if(playing),
                sleeping_bag.before(super::play::EndTick).run_if(playing),
                sleep_tick.run_if(in_state(Screen::Play)),
            ),
        );
    }
}

fn overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

/// The services' resource bundle (Bevy caps systems at 16 params — see play.rs RoomCtx).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ServiceCtx<'w> {
    shop: ResMut<'w, ShopState>,
    bought: Res<'w, BoughtShop>,
    people: Res<'w, super::talk::PeopleLedger>,
    next: ResMut<'w, NextState<Screen>>,
    sleeping: ResMut<'w, Sleeping>,
    log: ResMut<'w, super::rewards::LootLog>,
    inv: ResMut<'w, crate::inventory::PlayerInv>,
    saves: MessageWriter<'w, super::save::SaveRequest>,
    learned: ResMut<'w, super::flute::LearnedSongs>,
    tomes: Res<'w, super::gather::GatherState>,
}

/// The interior zone loop: the FIRST fixture under the player prompts + serves. A
/// takeable tome at your feet outranks the counter it sits on (js skipAction).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub(crate) fn interact_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    bindings: Res<Bindings>,
    inside: Res<Inside>,
    cooldown: Res<DoorCooldown>,
    cur: Res<CurRoom>,
    clock: Res<FrameClock>,
    perks: Res<super::guildhall::CityPerks>,
    mut sc: ServiceCtx,
    mut dialog: ResMut<super::dialog::Dialog>,
    mut players: Query<(&Player, &mut Health)>,
    books: Query<&Pickup>,
    old: Query<Entity, With<PromptBar>>,
    mut last: Local<Option<(String, i32, i32)>>,
) {
    let mut want: Option<(&str, &str)> = None; // (kind, label)
    if let (Some(state), Ok((p, mut health))) = (&inside.0, players.single_mut()) {
        let hitbox = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
        let book_near = books
            .iter()
            .any(|pk| matches!(pk.kind, PickupKind::Book(_)) && overlap(hitbox, (pk.x - 8.0, pk.y - 8.0, 32.0, 32.0)));
        if !book_near {
            for (kind, zlabel, zx, zy, zw, zh) in state.def.interact {
                if !overlap(hitbox, (*zx as f32, *zy as f32, *zw as f32, *zh as f32)) {
                    continue;
                }
                // Only ported services prompt (js also runs storage/bard/… — they join later).
                let served = match *kind {
                    "shop" => state.shop_key.is_some(),
                    "rest" | "heal" | "bed" | "bard" | "storage" => true,
                    _ => false,
                };
                if !served {
                    continue;
                }
                want = Some((kind, if *kind == "bed" { "REST" } else { zlabel }));
                if cooldown.0 == 0 && input.pressed(Action::Interact) {
                    input.consume(Action::Interact); // the counter eats the press
                    match *kind {
                        "shop" => {
                            stock_up(&mut sc.shop, state, &sc.bought, &sc.people, cur.rx, cur.ry, super::gather::farm_day(clock.0));
                            sc.next.set(Screen::Shop);
                            want = None; // the window replaces the bar (it returns on close)
                        }
                        "rest" => {
                            // The inn's chooser: REST, or REST AND SET SPAWN (Baz) — the
                            // cost + doorstep are precomputed so the pick just pays.
                            let cost = inn_cost(state, &perks, &sc.people, cur.rx, cur.ry);
                            let back = state.return_pos;
                            let mk = |spawn| super::dialog::ChoiceAct::InnRest {
                                cost,
                                spawn,
                                room: (cur.rx, cur.ry),
                                x: back.0,
                                y: back.1,
                            };
                            dialog.0 = Some(super::dialog::DialogState::choice(
                                "THE INN".into(),
                                vec![("REST".into(), mk(false)), ("REST AND SET SPAWN".into(), mk(true))],
                                Entity::PLACEHOLDER,
                            ));
                            sc.next.set(Screen::Dialog);
                            want = None; // the window replaces the bar
                        }
                        "bard" => bard_talk(&mut sc),
                        "heal" => church_heal(&mut health, &mut sc),
                        "bed" if state.def.kind == "house" => {
                            // YOUR bed offers the chooser: REST or SET SPAWN (Baz).
                            dialog.0 = Some(super::dialog::DialogState::choice(
                                "THE BED".into(),
                                vec![
                                    ("REST".into(), super::dialog::ChoiceAct::Rest),
                                    ("SET SPAWN".into(), super::dialog::ChoiceAct::SetSpawn),
                                ],
                                Entity::PLACEHOLDER,
                            ));
                            sc.next.set(Screen::Dialog);
                            want = None; // the window replaces the bar
                        }
                        "bed" => start_sleep(&mut sc.sleeping, 1.0),
                        "storage" => {
                            sc.next.set(Screen::Storage); // the two-pane chest (storage.rs resets on open)
                            want = None; // the window replaces the bar (it returns on close)
                        }
                        _ => {}
                    }
                }
                break;
            }
        }
    }

    // The shared by-the-character bubble (prompts.rs — Baz: one prompt language),
    // re-anchored as the hero moves along the counter.
    let msg = want.map(|(_, l)| format!("{} {}", bindings.prompt(Action::Interact, input.pad_present), l));
    let (px, py) = players.single().map(|(p, _)| (p.x as i32, p.y as i32)).unwrap_or((0, 0));
    let key = msg.clone().map(|m| (m, px, py));
    if key == *last {
        return;
    }
    *last = key;
    for e in &old {
        commands.entity(e).despawn();
    }
    let Some(text) = msg else { return };
    super::prompts::spawn_bubble(&mut commands, &mut images, &text, px as f32 + 8.0, py as f32 - 10.0, PromptBar);
}

/// The bard on the tavern stage (js bardTalk): hands you the flute + the Song of
/// Returning, sells a spare for coin, and afterwards drops hints about where the
/// other songs are written down.
fn bard_talk(sc: &mut ServiceCtx) {
    if !sc.learned.0.contains("returning") {
        if !sc.inv.has_item("flute") {
            sc.inv.add_item("flute", 1);
            sc.inv.auto_equip("flute");
            sc.log.add("talk", "THE BARD GIVES YOU A WINDWOOD FLUTE", 1, 0xcfe0ff, false, true);
        }
        // Add directly (not learn_song) so a full bag or a self-crafted flute can't
        // stop the teaching; the catch-up pass re-teaches any songbooks already read.
        sc.learned.0.insert("returning");
        for id in sc.tomes.tomes.iter() {
            if let Some(b) = crate::lore_books::get(id)
                && let Some(song) = b.teaches
            {
                super::flute::learn_song(&mut sc.learned, &sc.inv, &mut sc.log, song, true);
            }
        }
        sc.log.add("talk", "THE BARD TEACHES YOU THE SONG OF RETURNING", 1, 0xd8b8ff, false, true);
        sc.saves.write(super::save::SaveRequest);
        return;
    }
    if !sc.inv.has_item("flute") {
        const PRICE: i64 = 30;
        if sc.inv.money < PRICE {
            sc.log.add("talk", "A SPARE FLUTE COSTS 30 COIN", 1, 0xfc8868, false, true);
            return;
        }
        sc.inv.money -= PRICE;
        sc.inv.add_item("flute", 1);
        sc.log.add("talk", "THE BARD SELLS YOU A SPARE FLUTE", 1, 0xcfe0ff, false, true);
        sc.saves.write(super::save::SaveRequest);
        return;
    }
    // Hints at where the other songs are written down (js HINTS, cycled).
    const HINTS: [&str; 7] = [
        "A FARMER TAUGHT ME A RAIN SONG ONCE - I WROTE IT DOWN IN SOME TAVERN OR OTHER",
        "THE DAWN PRIESTS HAVE A HYMN THAT HURRIES THE SUN - THEIR CHAPELS KEEP IT",
        "OLD CAMP TALES SPEAK OF STONES THAT SING - AND THE NOTES THAT OPEN THEM",
        "THE CHOIR HAD A HYMN THAT RANG LIKE A BELL - BURIED IN SOME DARK PLACE",
        "THE OLD KINGSGUARD SANG A WARD BEFORE BATTLE - LOOK FOR IT IN A CASTLE",
        "THERE IS A CRADLE-SONG THAT LULLS ANYTHING TO SLEEP - SET DOWN IN SOME TOWN BOOK",
        "THE WILDFOLK SING THE FIELDS AWAKE - THEIR CAMPS REMEMBER THE TUNE",
    ];
    let n = sc.learned.0.len() % HINTS.len(); // steady, seedless rotation
    sc.log.add("talk", HINTS[n], 1, 0xcfe0ff, false, true);
}

/// js innRest's price: base 40, +50% per zone tier, keeper-discounted; the Provisioners'
/// restored city rests you FREE (their perk).
fn inn_cost(
    inside: &super::interior::InsideState,
    perks: &super::guildhall::CityPerks,
    people: &super::talk::PeopleLedger,
    rx: i32,
    ry: i32,
) -> i64 {
    if perks.free_inn {
        return 0;
    }
    const INN_COST: f64 = 40.0;
    let tier = crate::worldgen::world::World::zone_tier(rx, ry);
    let kd = super::shop::keeper_discount(inside, people);
    (INN_COST * (1.0 + tier as f64 * 0.5) * kd).ceil() as i64
}

/// js churchHeal: the free blessing — full health, or a gentle nudge if already hale.
fn church_heal(health: &mut Health, sc: &mut ServiceCtx) {
    if health.hp >= health.max {
        sc.log.add("church", "YOU ARE HALE", 1, 0xcfe0ff, false, true);
        return;
    }
    health.hp = health.max;
    sc.log.add("church", "BLESSED - HEALED", 1, 0xa8e0ff, false, true);
    sc.saves.write(super::save::SaveRequest);
}

pub(crate) fn start_sleep(sleeping: &mut Sleeping, heal_frac: f32) {
    if sleeping.0.is_none() {
        sleeping.0 = Some(SleepFx { t: 0, applied: false, heal_frac });
    }
}

/// A sleeping-bag use (js sleepingbag use()): rest ONLY in the open world, only with no
/// foes about — a rough sleep (heal to half). Vetoes + consumes the bag only on success.
#[allow(clippy::too_many_arguments)]
fn sleeping_bag(
    mut reqs: MessageReader<SleepRequest>,
    mut sleeping: ResMut<Sleeping>,
    inside: Res<Inside>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    foes: Query<&crate::combat::Combatant>,
) {
    for SleepRequest in reqs.read() {
        if in_dungeon.0.is_some() || inside.0.is_some() {
            log.add("sleep", "CANT REST HERE", 1, 0xfc8868, false, true);
            sfx.write(super::sfx::Sfx("tink"));
            continue;
        }
        if foes.iter().any(|c| c.team == crate::combat::Team::Enemy && c.persistent) {
            log.add("sleep", "TOO DANGEROUS - CLEAR THE AREA FIRST", 1, 0xfc8868, false, true);
            sfx.write(super::sfx::Sfx("tink"));
            continue;
        }
        inv.remove_one("sleepingbag");
        start_sleep(&mut sleeping, 0.5);
        sfx.write(super::sfx::Sfx("sleep"));
    }
}

/// The js updateSleep/drawSleep pair: fade to black, rest + jump to next morning at
/// full dark, fade back in. Runs while `playing` is false — sleep IS the freeze.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn sleep_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut sleeping: ResMut<Sleeping>,
    mut clock: ResMut<FrameClock>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut players: Query<&mut Health, With<Player>>,
    mut shades: Query<&mut Sprite, (With<SleepShade>, Without<SleepZzz>)>,
    mut zzz: Query<&mut Sprite, (With<SleepZzz>, Without<SleepShade>)>,
    shade_ents: Query<Entity, AnySleepEntity>,
) {
    let Some(fx) = &mut sleeping.0 else {
        for e in &shade_ents {
            commands.entity(e).despawn();
        }
        return;
    };
    if fx.t == 0 {
        // Stand the overlay up: the full-canvas shade + the Z Z Z (alpha-driven below).
        commands.spawn((
            Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::new(crate::CANVAS_W as f32, crate::CANVAS_H as f32)),
            at(0.0, 0.0, crate::CANVAS_W as f32, crate::CANVAS_H as f32, layers::SLEEP),
            PIXEL_LAYER,
            SleepShade,
        ));
        let (img, w) = crate::gfx::font::bake_text("Z Z Z", 0x9ab0e0, images.as_mut());
        let (sw, sh) = ((w * 2) as f32, 12.0);
        let mut sprite = Sprite::from_image(img);
        sprite.custom_size = Some(Vec2::new(sw, sh));
        sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
        commands.spawn((
            sprite,
            at((crate::CANVAS_W as f32 - sw) / 2.0, (crate::CANVAS_H as f32 - sh) / 2.0, sw, sh, layers::SLEEP + 0.01),
            PIXEL_LAYER,
            SleepZzz,
        ));
    }
    fx.t += 1;
    if !fx.applied && fx.t >= SLEEP_FADE {
        // Full black: rest + jump to the next morning (js updateSleep's apply). A bed heals
        // full; a sleeping bag only brings you UP TO half (heal_frac).
        fx.applied = true;
        if let Ok(mut health) = players.single_mut() {
            let target = (health.max as f32 * fx.heal_frac).round() as i32;
            health.hp = health.hp.max(target).min(health.max);
        }
        // js: frameCount = ceil((frameCount + 1) / DAY_LEN) * DAY_LEN — the next morning.
        clock.0 = (clock.0 + DAY_LEN).div_euclid(DAY_LEN) * DAY_LEN;
        saves.write(super::save::SaveRequest);
    }
    let a = if fx.t < SLEEP_FADE {
        fx.t as f32 / SLEEP_FADE as f32
    } else if fx.t < SLEEP_FADE + SLEEP_HOLD {
        1.0
    } else {
        (1.0 - (fx.t - SLEEP_FADE - SLEEP_HOLD) as f32 / SLEEP_FADE as f32).max(0.0)
    };
    for mut s in &mut shades {
        s.color = Color::srgba(0.0, 0.0, 0.0, a);
    }
    for mut s in &mut zzz {
        s.color = Color::srgba(1.0, 1.0, 1.0, ((a - 0.55) / 0.45).clamp(0.0, 1.0));
    }
    if fx.t >= SLEEP_FADE * 2 + SLEEP_HOLD {
        sleeping.0 = None;
        for e in &shade_ents {
            commands.entity(e).despawn();
        }
    }
}
