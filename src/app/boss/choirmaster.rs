//! THE CHOIRMASTER — the Saltmaze's hierophant (the Kingsplitter questline's
//! capstone, harder than any land boss short of the finale). A floating figure
//! whose head IS a bronze bell — pale eyes burning in its dark mouth, salt-white
//! robes trimmed in gold. He serves THE FIRST BELL, and he fights like it:
//! THE TOLL (he stills, the bell swings, and a ringing shockwave rolls out as a
//! closing ring of chimes — only the rim hurts; be elsewhere), THE CHORUS (an
//! aimed fan of salt-bright bolts), and THE CONGREGATION (zealots answer his
//! call, two at his side). Crossing 66% / 33% quickens the hymn.

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 100.0;
const GOLD: u32 = 0xc09a44;
const SALT: u32 = 0xe8ecf0;

/// The js S_CHOIRMASTER 16x16, drawn 2x by the renderer (custom_size 32).
const CHOIRMASTER: [&str; 16] = [
    ".......bb.......",
    "......bBBb......",
    ".....bBBBBb.....",
    ".....BBBBBB.....",
    "....bBBBBBBb....",
    "....BBBBBBBB....",
    "...bBBBBBBBBb...",
    "...KEKKKKKKEK...",
    "..rRRRRRRRRRRr..",
    "..rRRRRTTRRRRr..",
    ".rRRRRRTTRRRRRr.",
    ".rRRRRRTTRRRRRr.",
    ".rRRRRRTTRRRRRr.",
    "rrRRRRRTTRRRRRrr",
    ".r.rRRRrrRRRr.r.",
    "....r..rr..r....",
];

const PAL: &[(char, u32)] = &[
    ('b', 0x8a6a2a), // bell bronze, shaded
    ('B', GOLD),     // bell bronze
    ('K', 0x141018), // the bell's dark mouth
    ('E', 0xfff2c8), // the pale eyes burning inside
    ('R', SALT),     // salt-white robes
    ('r', 0xb8c2c8), // robe shade
    ('T', GOLD),     // gold trim
];

#[derive(Component)]
pub struct Choirmaster {
    x: f32,
    y: f32,
    anim: u32,
    phase: u8,
    toll_cd: i32,
    chorus_cd: i32,
    summon_cd: i32,
    still: i32, // rooted frames while the bell swings
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let img = images.add(crate::gfx::bake(&CHOIRMASTER, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (cx, cy) = (136.0, 60.0);
    let mut spr = Sprite::from_image(img);
    spr.custom_size = Some(Vec2::splat(32.0)); // drawn 2x (js shared boss renderer)
    commands.spawn((
        spr,
        at(PLAY_X + cx, PLAY_Y + cy, 32.0, 32.0, 8.25), // floats over the ground rank
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE CHOIRMASTER"),
        crate::app::dungeon::DungeonBoss,
        Choirmaster { x: cx, y: cy, anim: 0, phase: 0, toll_cd: 150, chorus_cd: 80, summon_cd: 220, still: 0 },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(4), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2, kb_resist: 0.92, kb_frames: 11 },
        Knockback::default(),
        Hitbox { x: cx + 4.0, y: cy + 4.0, w: 24.0, h: 26.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    mut rng: ResMut<GameRng>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player>,
    mut bosses: Query<(&mut Choirmaster, &mut Health, &mut Hitbox, &mut Transform), Without<Player>>,
    court: Query<&crate::app::dungeon::DungeonFoe, Without<Choirmaster>>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut c, mut h, mut hb, mut tf)) = bosses.single_mut() else { return };
    c.anim += 1;
    let (ccx, ccy) = (c.x + 16.0, c.y + 16.0);
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);

    // Crossing a third quickens the hymn (enrage burst: flash + a zealot).
    let frac = h.hp as f32 / h.max.max(1) as f32;
    let want: u8 = if frac <= 0.33 { 2 } else if frac <= 0.66 { 1 } else { 0 };
    if want > c.phase {
        c.phase = want;
        h.flash = 16;
        h.invuln = h.invuln.max(18);
        spawn_burst(&mut commands, &mut rng, Vec2::new(ccx, ccy), GOLD, 12);
        sfx.write(crate::app::sfx::Sfx("bellring"));
    }
    let quick = 1.0 - c.phase as f32 * 0.22;

    // --- THE TOLL: he stills, the bell swings, a ring of chimes rolls out. ---
    c.toll_cd -= 1;
    if c.toll_cd <= 0 {
        c.toll_cd = (240.0 * quick) as i32;
        c.still = 40;
        let n = 14 + c.phase as i32 * 3;
        for q in 0..n {
            let a = q as f32 / n as f32 * std::f32::consts::TAU;
            let bolt = art.bolt(SALT, 0xfff2c8);
            commands.spawn((
                EBolt { x: ccx - 4.0, y: ccy - 4.0, vx: a.cos() * 1.4, vy: a.sin() * 1.4, life: 78 },
                Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 1.0 },
                HitOnce::default(),
                Hitbox { x: ccx - 1.0, y: ccy - 1.0, w: 7.0, h: 7.0 },
                Sprite::from_image(bolt),
                at(PLAY_X + ccx - 3.0, PLAY_Y + ccy - 3.0, 8.0, 8.0, 8.6),
                PIXEL_LAYER,
                RoomActor,
            ));
        }
        sfx.write(crate::app::sfx::Sfx("bellring"));
    }

    // --- THE CHORUS: an aimed fan of salt-bright bolts. ---
    c.chorus_cd -= 1;
    if c.chorus_cd <= 0 && c.still <= 0 {
        c.chorus_cd = (110.0 * quick) as i32;
        let base = (pcy - ccy).atan2(pcx - ccx);
        for i in -1..=1i32 {
            let a = base + i as f32 * 0.3;
            let bolt = art.bolt(GOLD, 0xfff2c8);
            commands.spawn((
                EBolt { x: ccx - 4.0, y: ccy - 4.0, vx: a.cos() * 2.4, vy: a.sin() * 2.4, life: 110 },
                Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.5 },
                HitOnce::default(),
                Hitbox { x: ccx - 1.0, y: ccy - 1.0, w: 7.0, h: 7.0 },
                Sprite::from_image(bolt),
                at(PLAY_X + ccx - 3.0, PLAY_Y + ccy - 3.0, 8.0, 8.0, 8.6),
                PIXEL_LAYER,
                RoomActor,
            ));
        }
        sfx.write(crate::app::sfx::Sfx("swing"));
    }

    // --- THE CONGREGATION: zealots answer (two at his side, replenished). ---
    c.summon_cd -= 1;
    if c.summon_cd <= 0 && c.still <= 0 {
        c.summon_cd = (260.0 * quick) as i32;
        let live = court.iter().filter(|f| f.0 == "cultist").count();
        if live < 2
            && let Some(idx) = crate::actors::mobs::def_index("cultist")
        {
            let side = if rng.0.next_f64() < 0.5 { -24.0 } else { 24.0 };
            commands.spawn((
                crate::actors::mobs::mob_bundle(idx, (c.x + side).clamp(24.0, PX_W as f32 - 40.0), c.y + 8.0),
                RoomActor,
                PIXEL_LAYER,
                crate::app::dungeon::DungeonFoe("cultist"),
            ));
            spawn_burst(&mut commands, &mut rng, Vec2::new(ccx + side, ccy), SALT, 8);
            sfx.write(crate::app::sfx::Sfx("warpGo"));
        }
    }

    // --- The float: a slow, hymn-timed drift that keeps mid-range. ---
    if c.still > 0 {
        c.still -= 1;
    } else {
        let d = (pcx - ccx).hypot(pcy - ccy).max(1.0);
        let (dirx, diry) = ((pcx - ccx) / d, (pcy - ccy) / d);
        let spd = 0.58 + c.phase as f32 * 0.08;
        let orbit = (c.anim as f32 * 0.03).sin();
        let toward = if d > 90.0 { 1.0 } else if d < 50.0 { -1.0 } else { 0.0 };
        c.x = (c.x + (dirx * toward - diry * orbit * 0.6) * spd).clamp(16.0, PX_W as f32 - 48.0);
        c.y = (c.y + (diry * toward + dirx * orbit * 0.6) * spd).clamp(20.0, PX_H as f32 - 56.0);
    }
    let bob = ((c.anim as f32) * 0.06).sin() * 2.0;
    *hb = Hitbox { x: c.x + 4.0, y: c.y + 4.0, w: 24.0, h: 26.0 };
    *tf = at(PLAY_X + c.x, PLAY_Y + c.y + bob, 32.0, 32.0, 8.25);
}

/// The fall: golden bursts + the framework's shared loot flow (boss_deaths in
/// dungeon.rs handles boss_loot; the altar's blade is saltmaze.rs business).
pub(crate) fn deaths(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    q: Query<(Entity, &Choirmaster, &Health)>,
) {
    for (e, c, h) in &q {
        if h.hp > 0 {
            continue;
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(c.x + 16.0, c.y + 16.0), GOLD, 16);
        spawn_burst(&mut commands, &mut rng, Vec2::new(c.x + 16.0, c.y + 10.0), SALT, 12);
        commands.entity(e).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids_are_rectangular() {
        for row in CHOIRMASTER {
            assert_eq!(row.len(), 16);
        }
    }
}
