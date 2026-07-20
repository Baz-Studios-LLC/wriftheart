//! THE MUMMY KING — the bound sovereign of the old tombs (tomb / ossuary / hollowroot). A
//! necromancer: its signature is RAISE THE DEAD — it crosses its arms and CHANNELS
//! (untouchable while the ritual holds), then skeletons claw up from grave-mounds around
//! the arena in a wave. Between raisings it flings a CURSE that saps the strength from your
//! legs (the shared Slowed rig). Cut down its risen dead and punish the channel's end; each
//! third of its health lost swells the next wave.
//!
//! One of the six consolidated bespoke bosses (boss/mod.rs). Its necromancy is its own —
//! distinct from the Unmaker's rule-theft and the Bone Colossus's self-assembly.

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Afflicts, Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 60.0; // js tomb/ossuary pool (x HP_MUL)
const CURSE: u32 = 0xb8f0a0;
const GOLD: u32 = 0xe8c84a;
const PAL: &[(char, u32)] = &[
    ('G', GOLD),       // nemes headdress
    ('K', 0x3a2e18),   // outline / wrap shadow
    ('W', 0xe8dcb0),   // bandage cream
    ('b', 0xb8a878),   // bandage seam
    ('d', 0x2a2018),   // eye sockets
    ('E', 0x8af0c0),   // curse-light in the sockets
];

const MUMMY: [&str; 20] = [
    "....GGGGGG......",
    "...GKKKKKKG.....",
    "..GKWWWWWWKG....",
    "..GKWEddEWKG....",
    "..GKWWWWWWKG....",
    "...KWWWWWWK.....",
    "...KbWWWWbK.....",
    "...KWbWWbWK.....",
    "...KWWbbWWK.....",
    "...KbWWWWbK.....",
    "...KWWbbWWK.....",
    "...KWbWWbWK.....",
    "...KWWWWWWK.....",
    "...KbWWWWbK.....",
    "...KKWWWWKK.....",
    "....KWWWWK......",
    "....KWbbWK......",
    "....KK..KK......",
    "................",
    "................",
];

/// A grave-mound telegraph that a skeleton claws out of.
const MOUND: [&str; 8] = [
    "................",
    "................",
    "....KKKKKK......",
    "...KddddddK.....",
    "..KdKddKddK.....",
    "..KddddddK......",
    "...KKKKKK.......",
    "................",
];

const RAISE_TELE: i32 = 40; // the channel: arms crossed, untouchable
const MAX_DEAD: usize = 4;

#[derive(Component)]
pub struct MummyKing {
    x: f32,
    y: f32,
    anim: u32,
    phase: u8,
    raise_cd: i32,
    curse_cd: i32,
    channel: i32, // > 0 while raising the dead (invulnerable)
    mound_img: Handle<Image>,
}

/// A skeleton the king raised — marked so the wave stays capped.
#[derive(Component)]
pub struct RaisedDead;

/// A grave-mound telegraph: after `t` frames it bursts a skeleton and vanishes.
#[derive(Component)]
pub struct GraveMound {
    x: f32,
    y: f32,
    t: i32,
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let king_img = images.add(crate::gfx::bake(&MUMMY, PAL));
    let mound_img = images.add(crate::gfx::bake(&MOUND, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (bx, by) = (128.0, 44.0);
    commands.spawn((
        Sprite::from_image(king_img),
        at(PLAY_X + bx, PLAY_Y + by, 16.0, 20.0, actor_z(by + 20.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE MUMMY KING"),
        crate::app::dungeon::DungeonBoss,
        MummyKing { x: bx, y: by, anim: 0, phase: 0, raise_cd: 120, curse_cd: 80, channel: 0, mound_img },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(3), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.0 * (1.0 - 0.8), kb_frames: 10 },
        Knockback::default(),
        Hitbox { x: bx + 2.0, y: by + 4.0, w: 12.0, h: 14.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player, Without<MummyKing>>,
    mut kings: Query<(&mut MummyKing, &mut Health, &mut Hitbox, &mut Transform, &mut Visibility), (Without<Player>, Without<GraveMound>)>,
    mut mounds: Query<(Entity, &mut GraveMound, &mut Sprite), (Without<MummyKing>, Without<Player>)>,
    dead: Query<(), With<RaisedDead>>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut b, mut h, mut hb, mut tf, mut vis)) = kings.single_mut() else { return };
    b.anim += 1;
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let (bcx, bcy) = (b.x + 8.0, b.y + 10.0);

    // Phase-up (66% / 33%): the tomb shudders and a wave answers early.
    let frac = h.hp as f32 / h.max.max(1) as f32;
    let ph = if frac > 0.66 { 0 } else if frac > 0.33 { 1 } else { 2 };
    if ph > b.phase {
        b.phase = ph;
        h.flash = 16;
        h.invuln = h.invuln.max(18);
        b.raise_cd = 24;
        sfx.write(crate::app::sfx::Sfx("stone"));
    }
    let tempo = 1.0 + b.phase as f32 * 0.22;

    // --- RAISE THE DEAD (signature): channel (untouchable), then mounds erupt skeletons. ---
    if b.channel > 0 {
        b.channel -= 1;
        h.invuln = h.invuln.max(2); // arms crossed — no blow lands mid-ritual
        if b.channel == 0 {
            let live = dead.iter().count();
            let n = (2 + b.phase as usize).min(MAX_DEAD.saturating_sub(live));
            for i in 0..n {
                let a = b.anim as f32 * 0.9 + i as f32 * 2.2;
                let (mx, my) = ((pcx + a.cos() * 34.0 - 8.0).clamp(4.0, PX_W as f32 - 20.0), (pcy + a.sin() * 34.0 - 8.0).clamp(4.0, PX_H as f32 - 20.0));
                commands.spawn((
                    Sprite::from_image(b.mound_img.clone()),
                    at(PLAY_X + mx, PLAY_Y + my, 16.0, 8.0, actor_z(my + 14.0)),
                    PIXEL_LAYER,
                    RoomActor,
                    GraveMound { x: mx, y: my, t: 24 },
                ));
            }
            sfx.write(crate::app::sfx::Sfx("stone"));
        }
    } else {
        // Shamble toward the player (slow, unhurried).
        let s = 0.42 * tempo;
        b.x = (b.x + (pcx - bcx).signum() * s).clamp(6.0, PX_W as f32 - 22.0);
        b.y = (b.y + (pcy - bcy).signum() * s * 0.8).clamp(16.0, PX_H as f32 - 28.0);
        b.raise_cd -= 1;
        if b.raise_cd <= 0 && dead.iter().count() < MAX_DEAD {
            b.raise_cd = (220.0 / tempo) as i32;
            b.channel = RAISE_TELE;
            h.flash = 4;
        }
        // --- CURSE: a bolt of grave-light that saps your legs (Slowed). ---
        b.curse_cd -= 1;
        if b.curse_cd <= 0 {
            b.curse_cd = (120.0 / tempo) as i32;
            let a = (pcy - bcy).atan2(pcx - bcx);
            let e = commands
                .spawn((
                    EBolt { x: bcx - 4.0, y: bcy, vx: a.cos() * 2.1, vy: a.sin() * 2.1, life: 130 },
                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                    HitOnce::default(),
                    Hitbox { x: bcx - 1.0, y: bcy + 3.0, w: 7.0, h: 7.0 },
                    Sprite::from_image(art.bolt(CURSE, 0xe8fff0)),
                    at(PLAY_X + bcx - 5.0, PLAY_Y + bcy + 1.0, 8.0, 8.0, 8.6),
                    PIXEL_LAYER,
                    RoomActor,
                ))
                .id();
            commands.entity(e).insert(Afflicts("slow", 120));
            sfx.write(crate::app::sfx::Sfx("tink"));
        }
    }

    // --- Grave-mounds: hump the earth, then a skeleton claws out. ---
    for (e, mut m, mut mspr) in &mut mounds {
        m.t -= 1;
        mspr.color = Color::srgba(1.0, 1.0, 1.0, (m.t as f32 / 12.0).clamp(0.3, 1.0));
        if m.t <= 0 {
            if let Some(idx) = crate::actors::mobs::def_index("skeleton") {
                commands.spawn((
                    crate::actors::mobs::mob_bundle(idx, m.x, m.y),
                    RoomActor,
                    PIXEL_LAYER,
                    crate::app::dungeon::DungeonFoe("skeleton"),
                    RaisedDead,
                ));
            }
            commands.entity(e).despawn();
        }
    }

    // --- Sync the king. ---
    *hb = Hitbox { x: b.x + 2.0, y: b.y + 4.0, w: 12.0, h: 14.0 };
    let sway = if b.channel > 0 { ((b.anim as f32) * 0.5).sin() * 1.0 } else { 0.0 };
    *tf = at(PLAY_X + b.x + sway, PLAY_Y + b.y, 16.0, 20.0, actor_z(b.y + 20.0));
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

/// The king falls: its risen dead crumble, its wrappings scatter, the arena banks the reward.
#[allow(clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    kings: Query<(Entity, &MummyKing, &Health)>,
    mounds: Query<Entity, With<GraveMound>>,
) {
    let Ok((e, b, h)) = kings.single() else { return };
    if h.hp > 0 {
        return;
    }
    for me in &mounds {
        commands.entity(me).despawn();
    }
    let (cx, cy) = (b.x + 8.0, b.y + 10.0);
    for i in 0..3 {
        let off = i as f32 * 7.0 - 7.0;
        spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.4), GOLD, 12);
    }
    let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
    crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
    crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true);
    crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
    stats.bump("kills", 1.0);
    stats.bump_kill("boss");
    sfx.write(crate::app::sfx::Sfx("stone"));
    commands.entity(e).despawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids_rectangular() {
        for (i, r) in MUMMY.iter().enumerate() {
            assert_eq!(r.chars().count(), 16, "mummy row {i}");
        }
        for (i, r) in MOUND.iter().enumerate() {
            assert_eq!(r.chars().count(), 16, "mound row {i}");
        }
    }
}
