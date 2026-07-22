//! uniques.rs — the UNIQUE TRINKET mechanics (js batch 1: "items you FEEL"): swing
//! procs (Ember Fang scorch / Winter Shard chill — chances scale with Luck), kill
//! procs (Midas Tooth coin bursts, Soul Locket mends), hurt procs (Bramble Band
//! thorns, Grudge Purse coin spills, Saint's Glass shattering forever, Warden's
//! Knuckle mercy frames), the Owl Talisman's night hunt, the Wispstone's orbiting
//! grave-wisp (singes foes, swats bolts from the air), and the Windwood Boomerang's
//! out-and-back throw (chilling both ways). Stat-row trinkets need no code here —
//! the gear-stats pipeline (items::gear_stat via skills_tab::recompute) carries
//! them. DEVIATION (flagged): scorch burns the foe but does not yet ignite grass
//! (the fire-spread system ports with the wands).

use bevy::prelude::*;

use super::battle::{spawn_burst, GameRng, RoomActor};
use super::gather::spawn_coin;
use super::play::Player;
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::actors::mobs::Mob;
use crate::combat::{Combatant, Health, HitLanded, Hitbox, Team};
use crate::gfx::{at, bake, PIXEL_LAYER};

/// A swing that rolled its chill (frames) — landed hits slow the foe.
#[derive(Component)]
pub struct ChillHit(pub i32);

/// A swing that rolled its scorch — landed hits set the foe alight.
#[derive(Component)]
pub struct ScorchHit(pub i32);

/// The frost beam's bite (frames) — a landed hit freezes the foe SOLID: no
/// thinking, no stepping, no contact bite, an ice-blue cast with mist curling off.
#[derive(Component)]
pub struct FreezeHit(pub i32);

/// The venom spray's bite (frames) — a landed splash ENVENOMS the foe: a purple
/// cast, poison motes, and a slow DoT that never lands the killing blow.
#[derive(Component)]
pub struct PoisonHit(pub i32);

/// Generated-weapon / gear extras a swing carries: bonus knockback shove + a lifesteal
/// chance (js atk.knock / atk.leech). Read on the swing's landed hits.
#[derive(Component)]
pub struct SwingBonus {
    pub knock: f32,
    pub leech: f64,
}

/// Chilled/burning foes (js e.slowT / fire): the mob-side timers, ticked here.
#[derive(Component, Default)]
pub struct MobAfflictions {
    pub chill: i32,
    pub burn: i32,
    pub(crate) burn_clock: i32,
    /// Frozen solid (the frost beam) — battle/ai.rs stops the body while it runs.
    pub freeze: i32,
    /// Envenomed (the venom spray / its puddles) — a purple cast + a slow DoT.
    pub poison: i32,
    pub(crate) poison_clock: i32,
    /// Reeling from a SHIELD BASH — no thinking, no bite, until it clears.
    pub stagger: i32,
    /// Re-stagger immunity — a foe can't be bash-locked forever.
    pub(crate) stagger_guard: i32,
}

/// On an attack entity: its landed hit STAGGERS the foe for this many frames
/// (the shield bash; guards prevent chain-stunning).
#[derive(Component)]
pub struct StaggerHit(pub i32);

/// Enemy bolts the wisp can swat (a type alias keeps the query readable).
type SwattableBolts<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Hitbox),
    (
        Or<(
            With<super::battle::projectiles::EBolt>,
            With<crate::actors::mobs::WebBolt>,
            With<super::battle::projectiles::EnemyArrow>,
        )>,
        Without<Wisp>,
    ),
>;

/// The grave-wisp (js orbital): circles the hero, singes what it touches.
#[derive(Component)]
pub struct Wisp {
    pub ang: f32,
}

/// The boomerang in flight (js boomerangFx): out 24 ticks, then home to the hand.
#[derive(Component)]
pub struct Boomerang {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub t: i32,
}

#[derive(Message)]
pub struct ThrowBoomerang;

/// Worn-gear helpers.
fn worn(inv: &crate::inventory::PlayerInv, id: &str) -> bool {
    inv.gear.iter().flatten().any(|&uid| inv.id_of(uid) == Some(id))
}

/// Swing procs roll at spawn (js makeSwing): chance = stat x (1 + luck).
pub fn roll_swing_procs(
    commands: &mut Commands,
    swing: Entity,
    inv: &crate::inventory::PlayerInv,
    luck: f64,
    rng: &mut impl FnMut() -> f64,
) {
    let chill = crate::items::gear_stat(inv, "chill");
    if chill > 0.0 && rng() < chill * (1.0 + luck) {
        commands.entity(swing).insert(ChillHit(140));
    }
    let scorch = crate::items::gear_stat(inv, "scorch");
    if scorch > 0.0 && rng() < scorch * (1.0 + luck) {
        commands.entity(swing).insert(ScorchHit(150));
    }
}

/// A generated weapon's landed hit: a lifesteal roll mends the hero, and the bonus
/// knockback rides through (the shove itself is applied in resolve_combat via the
/// swing's Combatant.knock; here we handle the LEECH heal — js atk.leech).
fn swing_bonus_hits(
    mut hits: MessageReader<HitLanded>,
    mut rng: ResMut<GameRng>,
    bonuses: Query<&SwingBonus>,
    mobs: Query<(), With<Mob>>,
    mut players: Query<&mut Health, With<Player>>,
) {
    for hit in hits.read() {
        let Ok(b) = bonuses.get(hit.attacker) else { continue };
        // Leech only off a living foe, and only while wounded (js owner.health < max).
        if b.leech > 0.0
            && mobs.get(hit.target).is_ok()
            && rng.0.next_f64() < b.leech
            && let Ok(mut h) = players.single_mut()
            && h.hp < h.max
        {
            h.hp += 1;
        }
    }
}

/// Landed proc hits mark the foe; the timers live on the mob (js e.slowT / burning).
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // the Or-filter + one query per proc kind
fn proc_hits(
    mut commands: Commands,
    mut hits: MessageReader<HitLanded>,
    chills: Query<&ChillHit>,
    scorches: Query<&ScorchHit>,
    freezes: Query<&FreezeHit>,
    poisons: Query<&PoisonHit>,
    staggers: Query<&StaggerHit>,
    mut mobs: Query<(Entity, Option<&mut MobAfflictions>), Or<(With<Mob>, With<crate::actors::goblin::Goblin>)>>,
) {
    for hit in hits.read() {
        let (chill, scorch, freeze, poison) = (
            chills.get(hit.attacker).ok(),
            scorches.get(hit.attacker).ok(),
            freezes.get(hit.attacker).ok(),
            poisons.get(hit.attacker).ok(),
        );
        let stagger = staggers.get(hit.attacker).ok();
        if chill.is_none() && scorch.is_none() && freeze.is_none() && poison.is_none() && stagger.is_none() {
            continue;
        }
        let Ok((me, aff)) = mobs.get_mut(hit.target) else { continue };
        let (c, b, f, v) = (
            chill.map_or(0, |c| c.0),
            scorch.map_or(0, |s| s.0),
            freeze.map_or(0, |f| f.0),
            poison.map_or(0, |p| p.0),
        );
        // The bash's stagger respects the re-stagger guard (no chain-stunning).
        let st = stagger.map_or(0, |s| s.0);
        if let Some(mut a) = aff {
            a.chill = a.chill.max(c);
            a.burn = a.burn.max(b);
            a.freeze = a.freeze.max(f);
            a.poison = a.poison.max(v);
            if st > 0 && a.stagger_guard == 0 {
                a.stagger = st;
                a.stagger_guard = st + 90;
            }
        } else {
            commands.entity(me).insert(MobAfflictions {
                chill: c,
                burn: b,
                freeze: f,
                poison: v,
                stagger: st,
                stagger_guard: if st > 0 { st + 90 } else { 0 },
                ..Default::default()
            });
        }
    }
}

/// Tick the mob-side timers: chill halves movement (mob_step reads the component),
/// burn bites for 1 every 30 with a flash — and, like the player's DoTs, never
/// lands the killing blow on its own.
#[allow(clippy::type_complexity)] // the Or-filter (mobs AND goblinkind) is the point
fn affliction_tick(
    mut mobs: Query<(&mut MobAfflictions, &mut Health), Or<(With<Mob>, With<crate::actors::goblin::Goblin>)>>,
) {
    for (mut a, mut h) in &mut mobs {
        if a.chill > 0 {
            a.chill -= 1;
        }
        if a.stagger > 0 {
            a.stagger -= 1;
        }
        if a.stagger_guard > 0 {
            a.stagger_guard -= 1;
        }
        if a.freeze > 0 {
            a.freeze -= 1;
        }
        if a.burn > 0 {
            a.burn -= 1;
            a.burn_clock += 1;
            if a.burn_clock >= 30 {
                a.burn_clock = 0;
                if h.hp > 1 {
                    h.hp -= 1;
                    h.flash = h.flash.max(4);
                }
            }
        }
        // Venom: slower than fire (1 every 45), and like every DoT it never lands
        // the killing blow on its own.
        if a.poison > 0 {
            a.poison -= 1;
            a.poison_clock += 1;
            if a.poison_clock >= 45 {
                a.poison_clock = 0;
                if h.hp > 1 {
                    h.hp -= 1;
                    h.flash = h.flash.max(4);
                }
            }
        }
    }
}


/// Hurt procs (js onPlayerHurt): thorns bite back, the purse spills a coin, the
/// glass shatters forever, the knuckle stretches the mercy frames.
#[allow(clippy::too_many_arguments)]
fn hurt_procs(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut hits: MessageReader<HitLanded>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut players: Query<(Entity, &Player, &mut Health)>,
    mut foes: Query<(&Combatant, &Hitbox, &mut Health), Without<Player>>,
) {
    let Ok((pe, p, mut ph)) = players.single_mut() else { return };
    for hit in hits.read() {
        if hit.target != pe {
            continue;
        }
        // Warden's Knuckle: longer grace after a blow (js iframes stat).
        let iframes = crate::items::gear_stat(&inv, "iframes");
        if iframes > 0.0 {
            ph.invuln = ((ph.invuln as f64) * (1.0 + iframes)).round() as u32;
        }
        // Bramble Band: whatever strikes you gets bitten back (js thorns).
        let thorns = crate::items::gear_stat(&inv, "thorns") as i32;
        if thorns > 0
            && let Ok((_, _, mut fh)) = foes.get_mut(hit.attacker)
            && fh.hp > 0
        {
            fh.hp -= thorns;
            fh.flash = fh.flash.max(4);
        }
        // Grudge Purse: spills a coin when you are struck — snatch it back.
        if worn(&inv, "grudgepurse") {
            let ang = rng.0.next_f64() * std::f64::consts::TAU;
            spawn_coin(&mut commands, &mut images, 1, p.x + 4.0 + ang.cos() as f32 * 14.0, p.y + 8.0 + ang.sin() as f32 * 10.0);
        }
        // Saint's Glass: shatters forever at the first blow.
        if let Some(g) = inv.gear.iter().position(|g| g.is_some_and(|uid| inv.id_of(uid) == Some("saintsglass"))) {
            let uid = inv.gear[g].unwrap();
            inv.gear[g] = None;
            inv.remove_entry(uid);
            spawn_burst(&mut commands, &mut rng, Vec2::new(p.x + 8.0, p.y + 4.0), 0xeef8ff, 10);
            log.add("gear", "THE SAINTS GLASS SHATTERS", 1, 0xeef8ff, false, true);
            sfx.write(super::sfx::Sfx("tink"));
        }
    }
}

/// The Owl Talisman hunts at night: HUNTERS HOUR while the moon is up (js nightowl).
fn owl_tick(
    inv: Res<crate::inventory::PlayerInv>,
    clock: Res<super::room_render::FrameClock>,
    mut statuses: ResMut<super::status::Statuses>,
) {
    if worn(&inv, "owltalisman") && super::lighting::day_darkness(clock.0) > 0.5 {
        statuses.add("hunterhour", 90); // refreshed while worn; fades soon after dawn
    }
}

const WISP_ART: &[&str] = &[
    "..ss....",
    ".sWWs...",
    ".sWws.s.",
    "..ss..s.",
    "....ss..",
    "........",
    "........",
    "........",
];

/// The grave-wisp: one while the stone is worn — it circles, singes, and swats
/// enemy bolts from the air (js orbital tick).
#[allow(clippy::too_many_arguments)]
fn wisp_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    inv: Res<crate::inventory::PlayerInv>,
    players: Query<&Player>,
    mut wisps: Query<(Entity, &mut Wisp, &mut Transform, &mut Hitbox), Without<Player>>,
    bolts: SwattableBolts,
    mut rng: ResMut<GameRng>,
) {
    let Ok(p) = players.single() else { return };
    let want = worn(&inv, "wispstone");
    let mut have = false;
    for (e, mut w, mut tf, mut hb) in &mut wisps {
        if !want {
            commands.entity(e).despawn();
            continue;
        }
        have = true;
        w.ang += 0.045;
        let (x, y) = (p.x + 4.0 + w.ang.cos() * 26.0, p.y + 4.0 + w.ang.sin() * 18.0);
        *hb = Hitbox { x, y, w: 8.0, h: 8.0 };
        *tf = at(PLAY_X + x, PLAY_Y + y, 8.0, 8.0, actor_z(y + 8.0) + 0.01);
        // Swat arrows from the air: any enemy bolt it clips fizzles.
        for (be, bb) in &bolts {
            if hb.overlaps(bb) {
                commands.entity(be).despawn();
                spawn_burst(&mut commands, &mut rng, Vec2::new(bb.x + 3.0, bb.y + 3.0), 0x5ad0c8, 4);
            }
        }
    }
    if want && !have {
        let img = images.add(bake(WISP_ART, &[('s', 0x5ad0c8), ('W', 0xc8fff8), ('w', 0xffffff)]));
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + p.x, PLAY_Y + p.y, 8.0, 8.0, 8.3),
            PIXEL_LAYER,
            RoomActor,
            Wisp { ang: 0.0 },
            Combatant { team: Team::Player, hurt_team: Some(Team::Enemy), damage: Some(1), persistent: true, knock: 0.5 },
            Hitbox { x: p.x, y: p.y, w: 8.0, h: 8.0 },
        ));
    }
}

/// Throw + flight (js boomerangFx): out at 3.4 for 24 ticks, then home at 3.8;
/// chills both ways (ChillHit rides the whole flight).
fn boomerang_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut throws: MessageReader<ThrowBoomerang>,
    players: Query<&Player>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut flying: Query<(Entity, &mut Boomerang, &mut Transform, &mut Hitbox)>,
) {
    let Ok(p) = players.single() else { return };
    for _ in throws.read() {
        let (dx, dy) = match p.facing {
            crate::actors::hero::Facing::Up => (0.0, -1.0),
            crate::actors::hero::Facing::Down => (0.0, 1.0),
            crate::actors::hero::Facing::Left => (-1.0, 0.0),
            crate::actors::hero::Facing::Right => (1.0, 0.0),
        };
        let img = images.add(bake(
            &[".DD.....", "DddD....", "D.DdD...", "...DdD..", "....DdD.", ".....DdD", "......DD", "........"],
            &[('D', 0x8a6a3a), ('d', 0xc8a060)],
        ));
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + p.x + 4.0, PLAY_Y + p.y + 4.0, 10.0, 10.0, 8.6),
            PIXEL_LAYER,
            RoomActor,
            Boomerang { x: p.x + 4.0, y: p.y + 4.0, vx: dx * 3.4, vy: dy * 3.4, t: 0 },
            ChillHit(90),
            Combatant { team: Team::Player, hurt_team: Some(Team::Enemy), damage: Some(1), persistent: true, knock: 0.5 },
            crate::combat::HitOnce::default(),
            Hitbox { x: p.x + 4.0, y: p.y + 4.0, w: 10.0, h: 10.0 },
        ));
        sfx.write(super::sfx::Sfx("swing"));
    }
    for (e, mut b, mut tf, mut hb) in &mut flying {
        b.t += 1;
        if b.t < 24 {
            b.x += b.vx;
            b.y += b.vy;
        } else {
            let (dx, dy) = (p.x + 4.0 - b.x, p.y + 4.0 - b.y);
            let m = dx.hypot(dy).max(1.0);
            b.x += dx / m * 3.8;
            b.y += dy / m * 3.8;
            if m < 9.0 {
                commands.entity(e).despawn();
                sfx.write(super::sfx::Sfx("pickup"));
                continue;
            }
        }
        *hb = Hitbox { x: b.x, y: b.y, w: 10.0, h: 10.0 };
        *tf = at(PLAY_X + b.x, PLAY_Y + b.y, 10.0, 10.0, 8.6);
        tf.rotation = Quat::from_rotation_z(-(b.t as f32) * 0.45); // the spin
    }
}

pub struct UniquesPlugin;

impl Plugin for UniquesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ThrowBoomerang>().add_systems(
            bevy::app::FixedUpdate,
            (
                proc_hits.after(crate::combat::resolve_combat),
                swing_bonus_hits.after(crate::combat::resolve_combat),
                affliction_tick,
                hurt_procs.after(crate::combat::resolve_combat),
                owl_tick,
                wisp_tick.before(crate::combat::resolve_combat),
                boomerang_tick.before(crate::combat::resolve_combat),
            )
                .before(super::play::EndTick)
                .run_if(super::screen::playing),
        );
    }
}
