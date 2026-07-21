//! THE ASH TITAN — boss 8 of THE TEN (BOSSES.md): the Charred Hall's guardian.
//!
//! A charcoal giant in three plates of molten armor — HEAD, CHEST, LEGS — each a
//! separate target riding the body. The core is untouchable while any plate holds,
//! and every plate you break QUICKENS it and unlocks more of its fury: three
//! plates is slow stomps and a burning wake; two adds charge dashes; one adds the
//! slam nova. Break the last and the MELTDOWN begins: soft at last, fastest of
//! all, the floor igniting behind every stride.

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 70.0;
const EMBER: u32 = 0xfc7030;
const PAL: &[(char, u32)] = &[
    ('C', 0x38302e), // char
    ('c', 0x241f1e), // char deep
    ('O', EMBER),    // molten crack
    ('o', 0xffb050), // molten bright
    ('I', 0x5a5a66), // iron plate
    ('i', 0x3c3c46), // iron shade
    ('E', 0xffd060), // eyes
];

const CORE: [&str; 30] = [
    "......KKKKKKKKKK......",
    "....KKCcCCcCCcCCKK....",
    "...KCcCOCCcCCOCcCK....",
    "...KCCcCCECCECCcCK....",
    "...KCcCCcCCcCCCcCK....",
    "....KCCOCcCCOCCK......",
    "..KKCcCCcCCcCCcCKK....",
    ".KCcCCOCCcCCCOCCcK....",
    ".KCCcCCcOCCOCcCCcK....",
    ".KCcOCCcCCcCCCOCcK....",
    ".KCCcCCOCcCCOCCCcK....",
    ".KCcCCcCCcCCcCCOcK....",
    ".KCOCCcCOCCOCcCCcK....",
    ".KCcCCOCCcCCCOCCcK....",
    "..KCcCCcCCcCCcCCK.....",
    "..KCCOCcCCOCcCCcK.....",
    "..KCcCCCcCCcCOCCK.....",
    "...KCcCCOCcCCCcK......",
    "...KCCcCCcCOCCcK......",
    "..KCcCOCcCCcCCcCK.....",
    "..KCCcCCcCOCCOCCK.....",
    "..KCcCCOCcCCcCCcK.....",
    "..KCOCcCCcCOCCcCK.....",
    "..KCcCCcOCCcCCOCK.....",
    "...KCcCCcCCOCcCK......",
    "...KCCOCcCCcCCcK......",
    "..KCcCCcCCcCCcCCK.....",
    "..KCCcCCOCCOCCcCK.....",
    "...KKKKKKKKKKKKK......",
    "......................",
];
const PLATE_HEAD: [&str; 8] = [
    "..KKKKKKKKKK..",
    ".KIIiIIIIiIIK.",
    "KIiIIOIIOIIiIK",
    "KIIiIIIIIIiIIK",
    "KIiIIiKKiIIiIK",
    ".KIIiIIIIiIIK.",
    "..KKKKKKKKKK..",
    "..............",
];
const PLATE_CHEST: [&str; 10] = [
    ".KKKKKKKKKKKKKK.",
    "KIiIIiIIIIiIIiIK",
    "KIIOIIiKKiIIOIIK",
    "KIiIIIiKKiIIIiIK",
    "KIIiIOIIIIOIiIIK",
    "KIiIIIiIIiIIIiIK",
    "KIIiKIIiiIIKiIIK",
    ".KKIIiIIIIiIIKK.",
    "..KKKKKKKKKKKK..",
    "................",
];
const PLATE_LEGS: [&str; 8] = [
    ".KKKKKKKKKKKKK..",
    "KIiIIiIKKIiIIiK.",
    "KIIOIIiKKiIIOIK.",
    "KIiIIIiKKiIIIiK.",
    "KIIiIIiKKiIIiIK.",
    ".KKKKKKKKKKKKK..",
    "................",
    "................",
];
const FIRE: [&str; 8] = [
    "....o...",
    ".O.oOo..",
    "OoOOoOO.",
    ".OOooOo.",
    "o.OOOo..",
    ".OoOo.O.",
    "..O..o..",
    "........",
];

#[derive(Clone, Copy)]
enum Slot {
    Head,
    Chest,
    Legs,
}
fn plate_offset(s: Slot) -> (f32, f32, f32, f32) {
    // (dx, dy, w, h) over the core's top-left.
    match s {
        Slot::Head => (4.0, 1.0, 14.0, 7.0),
        Slot::Chest => (3.0, 7.0, 16.0, 9.0),
        Slot::Legs => (3.0, 20.0, 16.0, 6.0),
    }
}

#[derive(Component)]
pub struct AshTitan {
    x: f32,
    y: f32,
    anim: u32,
    plates_left: u8,
    dash: Option<(f32, f32, i32)>,
    dash_cd: i32,
    nova_cd: i32,
    trail_cd: i32,
    fire_img: Handle<Image>,
}

#[derive(Component)]
pub struct TitanPlate {
    slot: Slot,
}

#[derive(Component)]
pub struct FireTrail {
    t: i32,
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let core_img = images.add(crate::gfx::bake(&CORE, PAL));
    let fire_img = images.add(crate::gfx::bake(&FIRE, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (tx, ty) = (140.0, 52.0);
    for (slot, grid) in [(Slot::Head, &PLATE_HEAD[..]), (Slot::Chest, &PLATE_CHEST[..]), (Slot::Legs, &PLATE_LEGS[..])] {
        let img = images.add(crate::gfx::bake(grid, PAL));
        let (dx, dy, w, hh) = plate_offset(slot);
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + tx + dx, PLAY_Y + ty + dy, w, hh, actor_z(ty + 28.0) + 0.1),
            PIXEL_LAYER,
            RoomActor,
            TitanPlate { slot },
            Combatant { team: Team::Enemy, hurt_team: None, damage: None, persistent: false, knock: 0.0 },
            Health { hp: 9, max: 9, defense: 0, invuln: 0, flash: 0 },
            HurtProfile { invuln: 10, flash: 8, kb_base: 0.0, kb_frames: 0 },
            Knockback::default(),
            Hitbox { x: tx + dx, y: ty + dy, w, h: hh },
        ));
    }
    commands.spawn((
        Sprite::from_image(core_img),
        at(PLAY_X + tx, PLAY_Y + ty, 22.0, 30.0, actor_z(ty + 28.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE ASH TITAN"),
        crate::app::dungeon::DungeonBoss,
        AshTitan { x: tx, y: ty, anim: 0, plates_left: 3, dash: None, dash_cd: 150, nova_cd: 180, trail_cd: 0, fire_img },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(4), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2 * (1.0 - 0.92), kb_frames: 11 },
        Knockback::default(),
        Hitbox { x: tx + 2.0, y: ty + 4.0, w: 18.0, h: 24.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    grid: Res<crate::app::play::CurGrid>,
    blockers: Res<crate::app::room_props::RoomBlockers>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player>,
    mut titans: Query<
        (&mut AshTitan, &mut Health, &mut Hitbox, &mut Transform, &mut Visibility),
        (Without<TitanPlate>, Without<FireTrail>, Without<Player>),
    >,
    mut plates: Query<
        (&TitanPlate, &mut Hitbox, &mut Transform, &mut Visibility, &Health),
        (Without<AshTitan>, Without<FireTrail>, Without<Player>),
    >,
    mut fires: Query<(Entity, &mut FireTrail, &mut Sprite), (Without<AshTitan>, Without<TitanPlate>)>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut t, mut h, mut hb, mut tf, mut vis)) = titans.single_mut() else { return };
    t.anim += 1;
    let armored = t.plates_left > 0;
    if armored {
        h.invuln = h.invuln.max(2); // the plates drink every blow
    }
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let (tcx, tcy) = (t.x + 11.0, t.y + 15.0);
    let broken = 3 - t.plates_left as i32;
    let spd = 0.35 + broken as f32 * 0.16;

    // --- Movement: stomp-chase; dashes once the second plate is gone. ---
    let bx = (2.0, 4.0, 18.0, 24.0);
    let step = |t: &mut AshTitan, dx: f32, dy: f32, grid: &crate::room::RoomGrid, blockers: &crate::app::room_props::RoomBlockers| {
        for (sx, sy) in [(dx, 0.0), (0.0, dy)] {
            if sx == 0.0 && sy == 0.0 {
                continue;
            }
            let (nx, ny) = (t.x + sx, t.y + sy);
            if !grid.box_hits_solid(nx + bx.0, ny + bx.1, bx.2, bx.3)
                && !blockers.blocks((t.x + bx.0, t.y + bx.1, bx.2, bx.3), (nx + bx.0, ny + bx.1, bx.2, bx.3))
            {
                t.x = nx.clamp(4.0, PX_W as f32 - 26.0);
                t.y = ny.clamp(18.0, PX_H as f32 - 34.0);
            }
        }
    };
    let moving;
    if let Some((vx, vy, mut dt)) = t.dash {
        dt -= 1;
        step(&mut t, vx, vy, &grid.0, &blockers);
        moving = true;
        t.dash = if dt > 0 { Some((vx, vy, dt)) } else { None };
    } else {
        let (dx, dy) = (pcx - tcx, pcy - tcy);
        step(&mut t, dx.signum() * spd, dy.signum() * spd, &grid.0, &blockers);
        moving = dx.abs() > 4.0 || dy.abs() > 4.0;
        if broken >= 1 {
            t.dash_cd -= 1;
            if t.dash_cd <= 0 {
                t.dash_cd = 170 - broken * 25;
                let d = (dx * dx + dy * dy).sqrt().max(0.001);
                t.dash = Some((dx / d * 2.6, dy / d * 2.6, 18));
                h.flash = 5;
                sfx.write(crate::app::sfx::Sfx("stone"));
            }
        }
        if broken >= 2 {
            t.nova_cd -= 1;
            if t.nova_cd <= 0 {
                t.nova_cd = 200 - broken * 20;
                for i in 0..8 {
                    let a = i as f32 / 8.0 * std::f32::consts::TAU;
                    commands.spawn((
                        EBolt { x: tcx - 4.0, y: tcy - 4.0, vx: a.cos() * 1.9, vy: a.sin() * 1.9, life: 100 },
                        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                        HitOnce::default(),
                        Hitbox { x: tcx - 1.0, y: tcy - 1.0, w: 7.0, h: 7.0 },
                        Sprite::from_image(art.bolt(EMBER, 0xffe0a0)),
                        at(PLAY_X + tcx - 3.0, PLAY_Y + tcy - 3.0, 8.0, 8.0, 8.6),
                        PIXEL_LAYER,
                        RoomActor,
                    ));
                }
                sfx.write(crate::app::sfx::Sfx("stone"));
            }
        }
    }

    // --- The burning wake: fire where it trod. ---
    t.trail_cd -= 1;
    if moving && t.trail_cd <= 0 && fires.iter().count() < 40 {
        t.trail_cd = 12 - broken * 2;
        let (fx, fy) = (t.x + 7.0, t.y + 24.0);
        commands.spawn((
            Sprite::from_image(t.fire_img.clone()),
            at(PLAY_X + fx, PLAY_Y + fy, 8.0, 8.0, 1.9),
            PIXEL_LAYER,
            RoomActor,
            FireTrail { t: 300 },
            Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(1), persistent: true, knock: 0.0 },
            Hitbox { x: fx + 1.0, y: fy + 1.0, w: 6.0, h: 6.0 },
        ));
    }
    for (e, mut fire, mut fs) in &mut fires {
        fire.t -= 1;
        let flick = 0.55 + 0.45 * ((fire.t as f32) * 0.4).sin().abs();
        fs.color = Color::srgba(1.0, 1.0, 1.0, flick * (fire.t as f32 / 120.0).min(1.0));
        if fire.t <= 0 {
            commands.entity(e).despawn();
        }
    }

    // --- Plates ride the body. ---
    for (plate, mut phb, mut ptf, mut pvis, ph) in &mut plates {
        let (dx, dy, w, hh) = plate_offset(plate.slot);
        *phb = Hitbox { x: t.x + dx, y: t.y + dy, w, h: hh };
        *ptf = at(PLAY_X + t.x + dx, PLAY_Y + t.y + dy, w, hh, actor_z(t.y + 28.0) + 0.1);
        *pvis = if ph.flash > 0 && (ph.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
    }

    // --- Sync. ---
    *hb = Hitbox { x: t.x + 2.0, y: t.y + 4.0, w: 18.0, h: 24.0 };
    let stomp = ((t.anim as f32) * 0.11).sin() * 1.2;
    *tf = at(PLAY_X + t.x, PLAY_Y + t.y + stomp, 22.0, 30.0, actor_z(t.y + 28.0));
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

/// Broken plates bare more fury; the felled titan gutters out.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    mut titans: Query<(Entity, &mut AshTitan, &mut Health), (Without<TitanPlate>, Without<FireTrail>)>,
    plates: Query<(Entity, &TitanPlate, &Hitbox, &Health), Without<AshTitan>>,
    fires: Query<Entity, With<FireTrail>>,
) {
    let Ok((te, mut t, mut th)) = titans.single_mut() else { return };
    for (e, _, phb, ph) in &plates {
        if ph.hp > 0 {
            continue;
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(phb.x + phb.w / 2.0, phb.y + phb.h / 2.0), EMBER, 12);
        commands.entity(e).despawn();
        t.plates_left = t.plates_left.saturating_sub(1);
        th.flash = 10;
        sfx.write(crate::app::sfx::Sfx("stone"));
    }
    if th.hp <= 0 {
        for (e, ..) in &plates {
            commands.entity(e).despawn();
        }
        for e in &fires {
            commands.entity(e).despawn();
        }
        let (cx, cy) = (t.x + 11.0, t.y + 15.0);
        for i in 0..3 {
            let off = i as f32 * 8.0 - 8.0;
            spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.5), EMBER, 14);
        }
        let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
        crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
        crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true, None);
        crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
        stats.bump("kills", 1.0);
        stats.bump_kill("boss");
        sfx.write(crate::app::sfx::Sfx("stone"));
        commands.entity(te).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids_are_rectangular() {
        let check = |name: &str, g: &[&str], w: usize| {
            for (i, r) in g.iter().enumerate() {
                assert_eq!(r.chars().count(), w, "{name} row {i} width");
            }
        };
        check("core", &CORE, 22);
        check("head", &PLATE_HEAD, 14);
        check("chest", &PLATE_CHEST, 16);
        check("legs", &PLATE_LEGS, 16);
        check("fire", &FIRE, 8);
    }
}
