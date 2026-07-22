//! shield.rs — THE WOODEN SHIELD + BLOCKING (js items.js 'shield' + player.blocks()):
//! equip the shield to an ability slot and HOLD that button to raise it. While it's
//! up you walk at half speed, you can't swing, and EVERY incoming enemy projectile
//! ricochets off — arrow, bolt, or web, from any side (the js code never checks the
//! facing, only the desc claims "the front"; the CODE is what ports). Each block
//! wears the shield one notch (dur 12); the last block still lands, then it
//! SHATTERS — wooden splinters, the wood crack, "YOUR SHIELD SHATTERS". Durability
//! rides the inventory entry and the save.
//! The raised shield draws on the hero: a narrow EDGE sliver at the leading arm for
//! the side facings, the full face low for down, mostly hidden behind the body for
//! up (js drawShield placements).
//! DEVIATION (flagged): the Bubble Ring's one-shot auto-block joins with that
//! trinket's port; the slot durability bar joins the HUD polish pass.

use bevy::prelude::*;

use super::battle::projectiles::{EBolt, EnemyArrow};
use super::battle::{spawn_burst, GameRng, RoomActor};
use super::play::Player;
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::actors::hero::Facing;
use crate::actors::mobs::WebBolt;
use crate::combat::Hitbox;
use crate::gfx::{at, bake, PIXEL_LAYER};

/// The js SHIELD_BMP (10x12): wooden face, steel boss ring, gold dome.
const SHIELD_FACE: [&str; 12] = [
    "..KKKKKK..",
    ".KDDDDDDK.",
    "KDdDDDDdDK",
    "KDDDaaDDDK",
    "KDDaAAaDDK",
    "KDdaAPadDK",
    "KDDaAAaDDK",
    "KDDDaaDDDK",
    "KDdDDDDdDK",
    ".KDDDDDDK.",
    "..KDDDDK..",
    "...KKKK...",
];

/// The js SHIELD_EDGE_R (4x12): the shield seen edge-on at the leading arm.
const SHIELD_EDGE_R: [&str; 12] = [
    ".KK.", "KDAK", "KDAK", "KDaK", "KDAK", "KDPK", "KDPK", "KDAK", "KDaK", "KDAK", "KDAK", ".KK.",
];

/// js Assets.flipH(SHIELD_EDGE_R).
const SHIELD_EDGE_L: [&str; 12] = [
    ".KK.", "KADK", "KADK", "KaDK", "KADK", "KPDK", "KPDK", "KADK", "KaDK", "KADK", "KADK", ".KK.",
];

/// Enemy shots a raised shield turns away (arrow, bolt, web).
type IncomingShots<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Hitbox),
    (Or<(With<EnemyArrow>, With<EBolt>, With<WebBolt>)>, Without<Player>),
>;

/// A raised shield deflects the shot before combat can land it (js resolveCombat's
/// projectile-block arm): the shot dies in a spark, the shield wears one notch, and
/// on the last notch it shatters — that shot still blocked, its last stand.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
pub fn shield_block(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut players: Query<(&mut Player, &Hitbox)>,
    shots: IncomingShots,
) {
    let Ok((mut p, phb)) = players.single_mut() else { return };
    if !p.blocking {
        return;
    }
    for (e, hb) in &shots {
        if !hb.overlaps(phb) {
            continue;
        }
        commands.entity(e).despawn();
        spawn_burst(&mut commands, &mut rng, Vec2::new(hb.x + hb.w / 2.0, hb.y + hb.h / 2.0), 0xd8e8ff, 5);
        if wear_shield(&mut commands, &mut rng, &mut inv, &mut log, &mut sfx, &mut p) {
            return; // it shattered — nothing left to block with this tick
        }
    }
}

/// One notch of wear on the raised shield (js blocks()): dur seeds from the def on
/// first use; the last notch SHATTERS it. Returns true on the shatter. Shared by the
/// projectile deflect above and the melee clang below.
fn wear_shield(
    commands: &mut Commands,
    rng: &mut GameRng,
    inv: &mut crate::inventory::PlayerInv,
    log: &mut super::rewards::LootLog,
    sfx: &mut MessageWriter<super::sfx::Sfx>,
    p: &mut Player,
) -> bool {
    let Some(uid) = p.block_uid else {
        sfx.write(super::sfx::Sfx("block"));
        return false;
    };
    let max = inv.id_of(uid).and_then(crate::items::get).map_or(0, |d| d.dur);
    let Some(entry) = inv.entries.iter_mut().find(|en| en.uid == uid) else { return false };
    if max > 0 {
        let d = entry.dur.get_or_insert(max);
        *d -= 1;
        if *d <= 0 {
            inv.remove_entry(uid);
            log.add("shield", "YOUR SHIELD SHATTERS", 1, 0xc8a060, false, true);
            spawn_burst(commands, rng, Vec2::new(p.x + 8.0, p.y + 9.0), 0xa06a2a, 10);
            sfx.write(super::sfx::Sfx("wood"));
            p.blocking = false;
            p.block_uid = None;
            return true;
        }
    }
    sfx.write(super::sfx::Sfx("block"));
    false
}

/// A frontal melee hit turned by the raised shield (resolve_combat's clang): the
/// spark at the contact point + the same notch of wear the shots pay.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn shield_clang(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut clangs: MessageReader<crate::combat::ShieldClang>,
    mut players: Query<&mut Player>,
) {
    let Ok(mut p) = players.single_mut() else { return };
    for c in clangs.read() {
        spawn_burst(&mut commands, &mut rng, c.at, 0xd8e8ff, 5);
        if wear_shield(&mut commands, &mut rng, &mut inv, &mut log, &mut sfx, &mut p) {
            return;
        }
    }
}

#[derive(Component)]
struct ShieldFx;

/// The raised shield on the hero (js drawShield): edge slivers for the side facings,
/// the face low for down (in front), tucked behind the body for up.
fn shield_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    players: Query<&Player>,
    mut fx: Query<(Entity, &mut Transform), With<ShieldFx>>,
    mut shown: Local<Option<Facing>>,
) {
    let Ok(p) = players.single() else { return };
    let want = p.blocking.then_some(p.facing);
    if want != *shown {
        *shown = want;
        for (e, _) in &fx {
            commands.entity(e).despawn();
        }
        if let Some(f) = want {
            let (grid, w, h): (&[&str], f32, f32) = match f {
                Facing::Right => (&SHIELD_EDGE_R, 4.0, 12.0),
                Facing::Left => (&SHIELD_EDGE_L, 4.0, 12.0),
                _ => (&SHIELD_FACE, 10.0, 12.0),
            };
            let img = images.add(bake(grid, &[]));
            commands.spawn((Sprite::from_image(img), at(0.0, 0.0, w, h, 0.0), PIXEL_LAYER, RoomActor, ShieldFx));
        }
    }
    let Some(f) = want else { return };
    // The bash's PUNCH (Baz): the shield jabs 2px out and settles back.
    let punch = match p.bash_t {
        5..=6 => 2.0,
        2..=4 => 1.0,
        _ => 0.0,
    };
    let (px_, py_) = f.offset();
    for (_, mut tf) in &mut fx {
        let ((ox, oy, w, h), dz) = match f {
            Facing::Right => ((10.0, 3.0, 4.0, 12.0), 0.01),
            Facing::Left => ((2.0, 3.0, 4.0, 12.0), 0.01),
            Facing::Down => ((3.0, 6.0, 10.0, 12.0), 0.01),
            Facing::Up => ((3.0, -3.0, 10.0, 12.0), -0.01), // mostly occluded by the body
        };
        *tf = at(PLAY_X + p.x + ox + px_ * punch, PLAY_Y + p.y + oy + py_ * punch, w, h, actor_z(p.y + 16.0) + dz);
    }
}

/// The Bubble Ring's charge (js p.bubble / bubbleTimer): recharges over BUBBLE_RECHARGE
/// frames while the ring is worn, pops to deflect ONE incoming shot, then recharges.
#[derive(Resource, Default)]
pub struct Bubble {
    pub charged: bool,
    timer: i32,
}

const BUBBLE_RECHARGE: i32 = 200; // js frames to recharge the deflect

/// Recharge the bubble while a bubble-flagged item is worn; drop it if unequipped.
fn bubble_recharge(mut bubble: ResMut<Bubble>, inv: Res<crate::inventory::PlayerInv>) {
    if inv.has_gear_flag("bubble") {
        if !bubble.charged {
            bubble.timer += 1;
            if bubble.timer >= BUBBLE_RECHARGE {
                bubble.charged = true;
                bubble.timer = 0;
            }
        }
    } else {
        bubble.charged = false;
        bubble.timer = 0;
    }
}

/// A charged bubble pops to deflect the next projectile that reaches the player
/// (js blocks()'s bubble arm) — independent of a raised shield, and after it, so a
/// shield-blocked shot never also drains the bubble.
fn bubble_deflect(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    mut bubble: ResMut<Bubble>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&Hitbox, With<Player>>,
    shots: IncomingShots,
) {
    if !bubble.charged {
        return;
    }
    let Ok(phb) = players.single() else { return };
    for (e, hb) in &shots {
        if !hb.overlaps(phb) {
            continue;
        }
        commands.entity(e).despawn();
        spawn_burst(&mut commands, &mut rng, Vec2::new(hb.x + hb.w / 2.0, hb.y + hb.h / 2.0), 0x78d2ff, 6);
        sfx.write(super::sfx::Sfx("block"));
        bubble.charged = false; // popped — back to recharging
        bubble.timer = 0;
        return;
    }
}

#[derive(Component)]
struct BubbleFx;

/// The faint cyan deflector sphere around the hero while the bubble is charged
/// (js drawHero's bubble ring).
fn bubble_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    bubble: Res<Bubble>,
    players: Query<&Player>,
    mut fx: Query<(Entity, &mut Transform), With<BubbleFx>>,
    mut shown: Local<bool>,
) {
    let Ok(p) = players.single() else { return };
    if bubble.charged != *shown {
        *shown = bubble.charged;
        for (e, _) in &fx {
            commands.entity(e).despawn();
        }
        if bubble.charged {
            // A 22x22 ring: cyan rim, hollow centre.
            let mut grid: Vec<String> = Vec::new();
            for y in 0..22i32 {
                let mut row = String::new();
                for x in 0..22i32 {
                    let (dx, dy) = (x as f32 - 10.5, y as f32 - 10.5);
                    let d = (dx * dx + dy * dy).sqrt();
                    row.push(if (9.0..=10.6).contains(&d) { 'w' } else { '.' });
                }
                grid.push(row);
            }
            let refs: Vec<&str> = grid.iter().map(|s| s.as_str()).collect();
            let img = images.add(crate::gfx::bake(&refs, &[('w', 0x9ce0ff)]));
            let mut spr = Sprite::from_image(img);
            spr.color = spr.color.with_alpha(0.7);
            commands.spawn((spr, at(0.0, 0.0, 22.0, 22.0, 8.2), PIXEL_LAYER, RoomActor, BubbleFx));
        }
    }
    for (_, mut tf) in &mut fx {
        *tf = at(PLAY_X + p.x + 8.0 - 11.0, PLAY_Y + p.y + 8.0 - 11.0, 22.0, 22.0, actor_z(p.y + 16.0) - 0.1);
    }
}

pub struct ShieldPlugin;

impl Plugin for ShieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy::app::FixedUpdate,
            (
                shield_clang.after(crate::combat::resolve_combat).before(super::play::EndTick),
                shield_block
                    .after(super::battle::projectiles::enemy_shots_tick)
                    .after(super::battle::projectiles::mob_projectiles_tick)
                    .before(crate::combat::resolve_combat)
                    .before(super::play::EndTick),
            )
                .run_if(super::battle::not_sliding),
        )
        .add_systems(
            bevy::app::FixedUpdate,
            (
                bubble_recharge,
                bubble_deflect
                    .after(shield_block)
                    .after(super::battle::projectiles::enemy_shots_tick)
                    .before(crate::combat::resolve_combat)
                    .run_if(super::battle::not_sliding),
            ),
        )
        .init_resource::<Bubble>()
        .add_systems(
            bevy::app::FixedUpdate,
            (shield_overlay, bubble_overlay).after(super::play::tick).before(super::play::EndTick).run_if(super::screen::playing),
        );
    }
}
