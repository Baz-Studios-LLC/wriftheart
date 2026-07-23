//! THE WRIFTHEART — the finale. The broken heart of the Whole Age itself, vast
//! and wrong, hanging in the Black Castle's deepest hall with the FRACTURE
//! running down its face like a lightning scar that never healed. It does not
//! walk. The room fights for it: THE HEARTBEAT (a slow, arena-wide pulse ring —
//! only the rim hurts, and it comes on the drum of the opening cinematic's dying
//! beats), THE SHARD STORM (radial bursts of rift glass), THE WOUND (void tears
//! that open at the hall's edges and STAY, biting anything that steps in them),
//! and THE CALL (voidlings pour from the crack, three at a time). Crossing 66% /
//! 33% widens the fracture — faster drums, denser glass, and in the last third
//! it HURLS ITSELF across the hall in shuddering lunges. The Kingsplitter bites
//! deepest here (wriftbane: every marked hit lands twice).

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, HitLanded, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 130.0;
const VOID: u32 = 0xc060ff;
const DEEP: u32 = 0x7a28a8;

/// The heart: 30x28, drawn 2x (60x56) — rift crystal around a black fracture.
const HEART: [&str; 28] = [
    "..........vvvvvv..............",
    "......vvvvVVVVVVvvvv..........",
    "....vvVVVVVVkVVVVVVVvv........",
    "...vVVVVVVVVkkVVVVVVVv........",
    "..vVVVVWVVVVVkVVVVVVVVv.......",
    ".vVVVWWVVVVVVkVVVVVVVVVv......",
    ".vVVVWVVVVVVkkkVVVVVVVVv......",
    "vVVVVVVVVVVVkVVVVVVVVVVVv.....",
    "vVVVVVVVVVVkkVVVVVVWVVVVv.....",
    "vVVVVVVVVVVkVVVVVVWWVVVVv.....",
    "vVVVVVVVVVkkkVVVVVVWVVVVv.....",
    "vVVVVVVVVVVkVVVVVVVVVVVVv.....",
    ".vVVVVVVVVkkVVVVVVVVVVVv......",
    ".vVVVVVVVVkVVVVVVVVVVVVv......",
    ".vVVVVVVVkkkVVVVVVVVVVVv......",
    "..vVVVVVVVkVVVVVVVVVVVv.......",
    "..vVVVVVVkkVVVVVVVVVVVv.......",
    "...vVVVVVkVVVVVVVVVVVv........",
    "...vVVVVkkkVVVVVVVVVv.........",
    "....vVVVVkVVVVVVVVVv..........",
    ".....vVVkkVVVVVVVVv...........",
    "......vVVkVVVVVVVv............",
    ".......vVkkVVVVVv.............",
    "........vVkVVVVv..............",
    ".........vkkVVv...............",
    "..........vkVv................",
    "...........vv.................",
    "..............................",
];

const PAL: &[(char, u32)] = &[
    ('V', DEEP),     // rift crystal
    ('v', 0x48186a), // crystal rim, darker
    ('W', 0xe0b8ff), // glints
    ('k', 0x0a0410), // THE FRACTURE
];

const TEAR: [&str; 12] = [
    "..K...KK...K....",
    ".KVK.KVVK.KVK...",
    "KVPVKVPPVKVPVK..",
    "KVPPVPPPPVPPVK..",
    ".KVPPPPPPPPVK...",
    "KVPPPPPPPPPPVK..",
    ".KVPPPPPPPPVK...",
    "KVPPVPPPPVPPVK..",
    "KVPVKVPPVKVPVK..",
    ".KVK.KVVK.KVK...",
    "..K...KK...K....",
    "................",
];

/// The cinematic borrows the face (whole or broken, its palette's choice).
pub fn heart_grid() -> &'static [&'static str] {
    &HEART
}

#[derive(Component)]
pub struct TheWriftheart {
    x: f32,
    y: f32,
    anim: u32,
    phase: u8,
    beat_cd: i32,
    storm_cd: i32,
    tear_cd: i32,
    call_cd: i32,
    lunge: Option<(f32, f32, i32)>, // (vx, vy, frames) — the last third's shudder
    lunge_cd: i32,
}

#[derive(Component)]
pub struct WoundTear;

/// A Kingsplitter hit (swing or beam) — it bites the heart twice (js wriftbane).
#[derive(Component)]
pub struct Wriftbane;

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let img = images.add(crate::gfx::bake(&HEART, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (hx, hy) = (122.0, 44.0);
    let mut spr = Sprite::from_image(img);
    spr.custom_size = Some(Vec2::new(60.0, 56.0)); // drawn 2x — it FILLS the hall's head
    commands.spawn((
        spr,
        at(PLAY_X + hx, PLAY_Y + hy, 60.0, 56.0, 8.25),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE WRIFTHEART"),
        crate::app::dungeon::DungeonBoss,
        TheWriftheart { x: hx, y: hy, anim: 0, phase: 0, beat_cd: 170, storm_cd: 90, tear_cd: 260, call_cd: 200, lunge: None, lunge_cd: 120 },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(4), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2, kb_resist: 0.92, kb_frames: 11 },
        Knockback::default(),
        Hitbox { x: hx + 8.0, y: hy + 6.0, w: 44.0, h: 44.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player>,
    mut hearts: Query<(&mut TheWriftheart, &mut Health, &mut Hitbox, &mut Transform), Without<Player>>,
    court: Query<&crate::app::dungeon::DungeonFoe, Without<TheWriftheart>>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut w, mut h, mut hb, mut tf)) = hearts.single_mut() else { return };
    w.anim += 1;
    let (wcx, wcy) = (w.x + 30.0, w.y + 28.0);
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);

    // The fracture widens at each third: faster drums, denser glass, the frenzy.
    let frac = h.hp as f32 / h.max.max(1) as f32;
    let want: u8 = if frac <= 0.33 { 2 } else if frac <= 0.66 { 1 } else { 0 };
    if want > w.phase {
        w.phase = want;
        h.flash = 18;
        h.invuln = h.invuln.max(20);
        spawn_burst(&mut commands, &mut rng, Vec2::new(wcx, wcy), VOID, 16);
        sfx.write(crate::app::sfx::Sfx("thunder"));
    }
    let quick = 1.0 - w.phase as f32 * 0.22;

    // --- THE HEARTBEAT: the dying drum — an arena pulse ring, only the rim hurts. ---
    w.beat_cd -= 1;
    if w.beat_cd <= 0 {
        w.beat_cd = (200.0 * quick) as i32;
        let n = 18 + w.phase as i32 * 4;
        let gap = (rng.0.next_f64() * n as f64) as i32; // one silent chime — the way through
        for q in 0..n {
            if q == gap || q == (gap + 1) % n {
                continue;
            }
            let a = q as f32 / n as f32 * std::f32::consts::TAU;
            let bolt = art.bolt(VOID, 0xe0b8ff);
            commands.spawn((
                EBolt { x: wcx - 4.0, y: wcy - 4.0, vx: a.cos() * 1.25, vy: a.sin() * 1.25, life: 130 },
                Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 1.2 },
                HitOnce::default(),
                Hitbox { x: wcx - 1.0, y: wcy - 1.0, w: 7.0, h: 7.0 },
                Sprite::from_image(bolt),
                at(PLAY_X + wcx - 3.0, PLAY_Y + wcy - 3.0, 8.0, 8.0, 8.6),
                PIXEL_LAYER,
                RoomActor,
            ));
        }
        sfx.write(crate::app::sfx::Sfx("heartbeat"));
    }

    // --- THE SHARD STORM: aimed bursts of rift glass. ---
    w.storm_cd -= 1;
    if w.storm_cd <= 0 {
        w.storm_cd = (120.0 * quick) as i32;
        let base = (pcy - wcy).atan2(pcx - wcx);
        for i in -2..=2i32 {
            let a = base + i as f32 * 0.22;
            let bolt = art.bolt(0xe0b8ff, 0xffffff);
            commands.spawn((
                EBolt { x: wcx - 4.0, y: wcy - 4.0, vx: a.cos() * 2.6, vy: a.sin() * 2.6, life: 100 },
                Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.6 },
                HitOnce::default(),
                Hitbox { x: wcx - 1.0, y: wcy - 1.0, w: 7.0, h: 7.0 },
                Sprite::from_image(bolt),
                at(PLAY_X + wcx - 3.0, PLAY_Y + wcy - 3.0, 8.0, 8.0, 8.6),
                PIXEL_LAYER,
                RoomActor,
            ));
        }
        sfx.write(crate::app::sfx::Sfx("swing"));
    }

    // --- THE WOUND: void tears open at the hall's edges and STAY. ---
    w.tear_cd -= 1;
    if w.tear_cd <= 0 {
        w.tear_cd = (300.0 * quick) as i32;
        let img = images.add(crate::gfx::bake(&TEAR, &[('V', VOID), ('P', 0x140a20), ('K', 0x0a0410)]));
        let (tx, ty) = (
            24.0 + rng.0.next_f64() as f32 * (PX_W as f32 - 64.0),
            32.0 + rng.0.next_f64() as f32 * (PX_H as f32 - 80.0),
        );
        commands.spawn((
            Sprite::from_image(img),
            at(PLAY_X + tx, PLAY_Y + ty, 16.0, 12.0, 3.4),
            PIXEL_LAYER,
            RoomActor,
            WoundTear,
            Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(1), persistent: true, knock: 0.4 },
            Hitbox { x: tx + 2.0, y: ty + 2.0, w: 12.0, h: 8.0 },
        ));
        sfx.write(crate::app::sfx::Sfx("warpCharge"));
    }

    // --- THE CALL: voidlings pour from the crack (three at a time). ---
    w.call_cd -= 1;
    if w.call_cd <= 0 {
        w.call_cd = (260.0 * quick) as i32;
        let live = court.iter().filter(|f| f.0 == "voidling").count();
        if live < 3
            && let Some(idx) = crate::actors::mobs::def_index("voidling")
        {
            commands.spawn((
                crate::actors::mobs::mob_bundle(idx, wcx - 8.0, wcy + 20.0),
                RoomActor,
                PIXEL_LAYER,
                crate::app::dungeon::DungeonFoe("voidling"),
            ));
            spawn_burst(&mut commands, &mut rng, Vec2::new(wcx, wcy + 16.0), VOID, 8);
        }
    }

    // --- The last third: it HURLS ITSELF across the hall in shuddering lunges. ---
    if w.phase >= 2 {
        if let Some((vx, vy, mut t)) = w.lunge {
            w.x = (w.x + vx).clamp(8.0, PX_W as f32 - 68.0);
            w.y = (w.y + vy).clamp(20.0, PX_H as f32 - 72.0);
            t -= 1;
            w.lunge = if t <= 0 { None } else { Some((vx, vy, t)) };
        } else {
            w.lunge_cd -= 1;
            if w.lunge_cd <= 0 {
                w.lunge_cd = 130;
                let d = (pcx - wcx).hypot(pcy - wcy).max(1.0);
                w.lunge = Some(((pcx - wcx) / d * 2.6, (pcy - wcy) / d * 2.6, 24));
                sfx.write(crate::app::sfx::Sfx("warpGo"));
            }
        }
    }

    // The hang: a slow, wrong bob — deeper and quicker as the fracture widens.
    let bob = ((w.anim as f32) * (0.03 + w.phase as f32 * 0.012)).sin() * (2.0 + w.phase as f32);
    *hb = Hitbox { x: w.x + 8.0, y: w.y + 6.0, w: 44.0, h: 44.0 };
    *tf = at(PLAY_X + w.x, PLAY_Y + w.y + bob, 60.0, 56.0, 8.25);
}

/// Wriftbane: a marked hit (the Kingsplitter's swing or beam) bites twice.
pub(crate) fn wriftbane_hits(
    mut hits: MessageReader<HitLanded>,
    marked: Query<&Wriftbane>,
    mut hearts: Query<&mut Health, With<TheWriftheart>>,
) {
    for hit in hits.read() {
        if marked.get(hit.attacker).is_err() {
            continue;
        }
        if let Ok(mut h) = hearts.get_mut(hit.target)
            && h.hp > 0
        {
            h.hp = (h.hp - hit.dealt).max(0); // the blade that broke it, finishing the work
        }
    }
}

/// The fall: the fracture gives — bursts up the whole face, then the shared
/// boss_loot flow (navigate's is_final arm raises the victory).
pub(crate) fn deaths(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    tears: Query<Entity, With<WoundTear>>,
    q: Query<(Entity, &TheWriftheart, &Health)>,
) {
    for (e, w, h) in &q {
        if h.hp > 0 {
            continue;
        }
        for i in 0..6 {
            let a = i as f32 * 1.05;
            spawn_burst(
                &mut commands,
                &mut rng,
                Vec2::new(w.x + 30.0 + a.cos() * 14.0, w.y + 28.0 + a.sin() * 14.0),
                if i % 2 == 0 { VOID } else { 0xe0b8ff },
                12,
            );
        }
        for t in &tears {
            commands.entity(t).despawn(); // the wound closes with it
        }
        commands.entity(e).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids_are_rectangular() {
        for row in HEART {
            assert_eq!(row.len(), 30);
        }
        for row in TEAR {
            assert_eq!(row.len(), 16);
        }
    }
}
