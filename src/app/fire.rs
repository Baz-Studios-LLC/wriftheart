//! fire.rs — REAL FIRE (js burningFx + the game.js wildfire loop): firebolts set
//! the world alight. A flammable gather node (grass, bush, tree) that a fire bolt
//! touches IGNITES — grass lets the bolt streak on over it (js passFire), a bush
//! or tree stops it in a burst. A burning thing wears clinging flames, lights the
//! dark, and burns down on the js clocks (grass 40, bush 70, tree 220); when the
//! fire wins, the node dies through the NORMAL gather death (drops + the regrowth
//! ledger), same as a chop. WILDFIRE: every 16 ticks, each burning thing (props
//! AND burning foes — the Ember Fang's mark counts) rolls 20% to ignite each
//! flammable neighbour within ~a tile, and sets touching foes burning. Rain stops
//! the spread AND douses standing flames (the target survives); nothing spreads
//! indoors or underground.
//! DEVIATION (flagged): a burning bush stays solid until it burns away (the js
//! drops its blocker the moment it lights).

use bevy::prelude::*;

use super::battle::{GameRng, RoomActor};
use super::gather::GatherNode;
use super::room_render::{PLAY_X, PLAY_Y};
use super::uniques::MobAfflictions;
use super::wands::SpellBolt;
use crate::actors::mobs::Mob;
use crate::combat::{Health, Hitbox};
use crate::gfx::{at, PIXEL_LAYER};

/// Clinging flames on a node (js burningFx): burns down, then the node falls.
#[derive(Component)]
pub struct Burning {
    pub t: i32,
    /// Lit by LIGHTNING (weather.rs): the rain that rides the storm can't douse
    /// it — the strike's fire burns down through the downpour (spread still stops).
    pub stormborn: bool,
}

/// One flame tongue of the overlay (jittered by flame_flicker), or its soft glow
/// halo, or the hot spark riding near the crown.
#[derive(Component)]
struct FlameFx {
    host: Entity,
    lane: i32,
    kind: FlameKind,
}

/// The js draws its fire with 'lighter' (ADDITIVE) compositing — sprites can't, so
/// the look is rebuilt in layers: each tongue wears a wider half-alpha halo behind
/// it, and the #fce0a8 hot spark flecks the top (js burningFx.draw, all three parts).
enum FlameKind {
    Tongue,
    Glow,
    Spark,
}

/// Positions of everything burning this tick — lighting.rs reads it (a blaze
/// lights the area, js collectLights' `.burning` arm).
#[derive(Resource, Default)]
pub struct BurningLights(pub Vec<(i32, i32)>);

/// What catches (js flammable flags): grass and bushes, and the trees — never
/// stone, crystal, cactus, cracked walls, or songstones.
pub(crate) fn flammable(kind: &str) -> bool {
    matches!(kind, "grass" | "bush") || !matches!(kind, "boulder" | "stalagmite" | "crystalspire" | "cactus" | "crackedrock" | "songstone")
}

/// The js burn clocks: grass flashes over, a tree takes a while to come down.
fn burn_frames(kind: &str) -> i32 {
    match kind {
        "grass" => 40,
        "bush" => 70,
        _ => 220,
    }
}

pub fn ignite(commands: &mut Commands, host: Entity, kind: &str, stormborn: bool) {
    commands.entity(host).insert(Burning { t: burn_frames(kind), stormborn });
    for lane in 0..4 {
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(0xfc, if lane & 1 == 1 { 0xae } else { 0x60 }, if lane & 1 == 1 { 0x40 } else { 0x20 }), Vec2::new(2.0, 5.0)),
            at(0.0, 0.0, 2.0, 5.0, 9.1),
            PIXEL_LAYER,
            RoomActor,
            FlameFx { host, lane, kind: FlameKind::Tongue },
        ));
        commands.spawn((
            Sprite::from_color(Color::srgba(0.99, 0.55, 0.16, 0.35), Vec2::new(4.0, 7.0)),
            at(0.0, 0.0, 4.0, 7.0, 9.05),
            PIXEL_LAYER,
            RoomActor,
            FlameFx { host, lane, kind: FlameKind::Glow },
        ));
    }
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(0xfc, 0xe0, 0xa8), Vec2::new(2.0, 3.0)),
        at(0.0, 0.0, 2.0, 3.0, 9.15),
        PIXEL_LAYER,
        RoomActor,
        FlameFx { host, lane: 0, kind: FlameKind::Spark },
    ));
}

/// Fire bolts touching a flammable node set it alight; grass lets the bolt fly on,
/// anything bigger stops it in a burst (js resolveCombat's fire arm + passFire).
fn fire_ignition(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    bolts: Query<(Entity, &SpellBolt, &Hitbox)>,
    nodes: Query<(Entity, &GatherNode, &Hitbox), Without<Burning>>,
) {
    for (be, b, bhb) in &bolts {
        if !b.fire {
            continue;
        }
        for (ne, node, nhb) in &nodes {
            if !flammable(node.kind) || !bhb.overlaps(nhb) {
                continue;
            }
            ignite(&mut commands, ne, node.kind, false);
            if node.kind != "grass" {
                // a bush or tree stops the bolt in a little explosion
                super::battle::spawn_burst(&mut commands, &mut rng, Vec2::new(bhb.x + 4.0, bhb.y + 4.0), 0xfc7430, 8);
                commands.entity(be).despawn();
                break;
            }
        }
    }
}

/// Burn-down + douse + the flame overlay + the light list.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn fire_tick(
    mut commands: Commands,
    clock: Res<super::room_render::FrameClock>,
    weather: Res<super::weather::WeatherState>,
    slide: Res<super::play::SlideState>,
    mut lights: ResMut<BurningLights>,
    mut burning: Query<(Entity, &mut Burning, Option<&mut Health>, &Hitbox)>,
    mut flames: Query<(Entity, &FlameFx, &mut Transform)>,
    hosts: Query<&Hitbox, With<Burning>>,
) {
    let raining = crate::weather::get(weather.cur).kind == crate::weather::Kind::Rain;
    // Mid-slide, burning things ride the OUTGOING root but their hitboxes stay
    // room-local — shift flames AND glow by the live offset so the blaze slides out
    // with its bush instead of hanging in the air (Baz: "the glow from fire doesn't
    // transition right").
    let (sx, sy) = slide.outgoing_offset().unwrap_or((0.0, 0.0));
    lights.0.clear();
    for (e, mut b, health, hb) in &mut burning {
        if raining && !b.stormborn {
            // rain puts it out — the target SURVIVES (js doused); a lightning-lit
            // burn shrugs the rain off
            commands.entity(e).remove::<Burning>();
            continue;
        }
        b.t -= 1;
        lights.0.push(((hb.x + hb.w / 2.0 + sx) as i32, (hb.y + hb.h / 2.0 + sy) as i32));
        if b.t <= 0 {
            commands.entity(e).remove::<Burning>();
            if let Some(mut h) = health {
                h.hp = 0; // the normal node death runs: drops, despawn, the regrowth ledger
            }
        }
    }
    // Flames ride their host's hitbox, flickering (js burningFx.draw's dance: tongues
    // bottom out 5px above the base at x+3+i*3, the spark flecks near the crown).
    for (fe, fx, mut tf) in &mut flames {
        let Ok(hb) = hosts.get(fx.host) else {
            commands.entity(fe).despawn();
            continue;
        };
        let t = clock.0 as f32;
        let sway = ((t + fx.lane as f32 * 7.0) * 0.3).sin() * 1.5;
        let h = 5.0 + ((clock.0 + fx.lane as i64 * 5) % 4) as f32;
        let (bx, by) = (PLAY_X + sx + hb.x, PLAY_Y + sy + hb.y);
        let base = by + hb.h - 5.0; // js: tongue bottoms at target y+11 of the 16px tile
        *tf = match fx.kind {
            FlameKind::Tongue => at(bx + 3.0 + fx.lane as f32 * 3.0 + sway, base - h, 2.0, h, 9.1),
            FlameKind::Glow => at(bx + 2.0 + fx.lane as f32 * 3.0 + sway, base - h - 1.0, 4.0, h + 2.0, 9.05),
            FlameKind::Spark => at(bx + 6.0, by + hb.h - 14.0, 2.0, 3.0, 9.15),
        };
    }
}

/// The wildfire creep (js game.js): every 16 ticks, each burning thing — prop or
/// marked foe — rolls to ignite flammable neighbours and sets touching foes alight.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn wildfire(
    mut commands: Commands,
    clock: Res<super::room_render::FrameClock>,
    weather: Res<super::weather::WeatherState>,
    inside: Res<super::interior::Inside>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    mut rng: ResMut<GameRng>,
    burning_props: Query<&Hitbox, (With<Burning>, With<GatherNode>)>,
    nodes: Query<(Entity, &GatherNode, &Hitbox), Without<Burning>>,
    mut foes: Query<(Entity, &Hitbox, Option<&mut MobAfflictions>), With<Mob>>,
) {
    if clock.0 % 16 != 0 || inside.0.is_some() || in_dungeon.0.is_some() {
        return;
    }
    if crate::weather::get(weather.cur).kind == crate::weather::Kind::Rain {
        return; // rain stops fire spreading
    }
    let mut sources: Vec<(f32, f32)> = burning_props.iter().map(|hb| (hb.x + hb.w / 2.0, hb.y + hb.h / 2.0)).collect();
    // Burning FOES spread too (the Ember Fang's mark counts) — a read-only pass
    // over the same query the ignition pass borrows mutably below.
    sources.extend(
        foes.iter()
            .filter(|(_, _, a)| a.as_ref().is_some_and(|a| a.burn > 0))
            .map(|(_, hb, _)| (hb.x + hb.w / 2.0, hb.y + hb.h / 2.0)),
    );
    if sources.is_empty() {
        return;
    }
    for &(ex, ey) in &sources {
        for (ne, node, nhb) in &nodes {
            let (dx, dy) = (nhb.x + nhb.w / 2.0 - ex, nhb.y + nhb.h / 2.0 - ey);
            if dx * dx + dy * dy > 19.0 * 19.0 || !flammable(node.kind) {
                continue;
            }
            if rng.0.next_f64() < 0.2 {
                ignite(&mut commands, ne, node.kind, false);
            }
        }
        for (fe, fhb, aff) in &mut foes {
            let (dx, dy) = (fhb.x + fhb.w / 2.0 - ex, fhb.y + fhb.h / 2.0 - ey);
            if dx * dx + dy * dy > 19.0 * 19.0 {
                continue;
            }
            match aff {
                Some(mut a) => {
                    if a.burn <= 0 {
                        a.burn = 110;
                    }
                }
                None => {
                    commands.entity(fe).insert(MobAfflictions { burn: 110, ..Default::default() });
                }
            }
        }
    }
}

pub struct FirePlugin;

impl Plugin for FirePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BurningLights>().add_systems(
            bevy::app::FixedUpdate,
            (fire_ignition.after(crate::combat::resolve_combat), fire_tick, wildfire)
                .before(super::play::EndTick)
                .run_if(super::screen::playing),
        );
    }
}
