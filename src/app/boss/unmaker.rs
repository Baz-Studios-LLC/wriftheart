//! THE UNMAKER — boss 9 of THE TEN (BOSSES.md): the Rift Vault's guardian.
//!
//! It does not fight you. It unmakes the RULES you fight with. Its HEX mirrors
//! your hands — left is right, up is down — for long, lurching spells. It blinks
//! across the vault instead of walking. At each third of its health it splits
//! into a court of FALSE SELVES (one blow bursts them; hurting the real one
//! scatters them all) — and the real one has a tell, if you watch: its eyes
//! GLINT. All the while VOID TEARS open at the vault's edges and stay, biting
//! anything that steps in them.

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 84.0;
const VOID: u32 = 0xb060f0;
const PAL: &[(char, u32)] = &[
    ('X', 0x6a3aa0), // robe violet
    ('x', 0x48287a), // robe deep
    ('V', VOID),     // rift glow
    ('W', 0xf0e8ff), // eye glint
    ('P', 0x140a20), // void black
];

const UNMAKER: [&str; 24] = [
    ".....KKKKKKKK.....",
    "...KKXxXXxXXXKK...",
    "..KXxXPPPPPPXxXK..",
    "..KXPPPPPPPPPPXK..",
    ".KXxPPWPPPPWPPxK..",
    ".KXPPPWPPPPWPPPK..",
    ".KXxPPPPPPPPPPxK..",
    "..KXPPPPPPPPPPXK..",
    "..KXxXPPPPPPXxXK..",
    ".KXxXXxVVVVxXXxXK.",
    ".KXXxXVvvvvVXxXXK.",
    ".KXxXXxVVVVxXXxXK.",
    ".KXXxXXxXXxXXxXXK.",
    ".KXxXXXxXXxXXXxXK.",
    "..KXXxXXVVXXxXXK..",
    "..KXxXXxVVxXXxXK..",
    "..KXXxXXxXXxXXXK..",
    "...KXxXXxXXxXXK...",
    "...KXXxXXXxXXXK...",
    "....KXxXXXxXXK....",
    "....KXXxXXXxXK....",
    ".....KKXXXKK......",
    ".......KKK........",
    "..................",
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

#[derive(Component)]
pub struct Unmaker {
    x: f32,
    y: f32,
    anim: u32,
    crossed: u8,
    hex_cd: i32,
    blink_cd: i32,
    bolt_cd: i32,
    tear_cd: i32,
    glint: i32,
    img: Handle<Image>,
    tear_img: Handle<Image>,
}

#[derive(Component)]
pub struct FalseSelf {
    x: f32,
    y: f32,
    drift: f32,
}

#[derive(Component)]
pub struct VoidTear;

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let img = images.add(crate::gfx::bake(&UNMAKER, PAL));
    let tear_img = images.add(crate::gfx::bake(&TEAR, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (ux, uy) = (143.0, 70.0);
    commands.spawn((
        Sprite::from_image(img.clone()),
        at(PLAY_X + ux, PLAY_Y + uy, 18.0, 24.0, actor_z(uy + 22.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE UNMAKER"),
        crate::app::dungeon::DungeonBoss,
        Unmaker { x: ux, y: uy, anim: 0, crossed: 0, hex_cd: 300, blink_cd: 140, bolt_cd: 90, tear_cd: 240, glint: 0, img, tear_img },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(4), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2 * (1.0 - 0.92), kb_frames: 11 },
        Knockback::default(),
        Hitbox { x: ux + 2.0, y: uy + 3.0, w: 14.0, h: 18.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut rng: ResMut<GameRng>,
    mut hexed: ResMut<crate::app::play::Hexed>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player>,
    mut unmakers: Query<
        (&mut Unmaker, &mut Health, &mut Hitbox, &mut Sprite, &mut Transform, &mut Visibility),
        (Without<FalseSelf>, Without<Player>),
    >,
    mut clones: Query<(&mut FalseSelf, &mut Transform, &Health), (Without<Unmaker>, Without<Player>)>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut u, mut h, mut hb, mut spr, mut tf, mut vis)) = unmakers.single_mut() else { return };
    u.anim += 1;
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let (ucx, ucy) = (u.x + 9.0, u.y + 12.0);

    // --- The court of false selves at each third. ---
    let frac = h.hp as f32 / h.max.max(1) as f32;
    let want: u8 = if frac <= 1.0 / 3.0 { 2 } else if frac <= 2.0 / 3.0 { 1 } else { 0 };
    if want > u.crossed {
        u.crossed = want;
        for i in 0..2 {
            let a = i as f32 * 2.6 + u.anim as f32 * 0.1;
            let (cx, cy) = ((ucx + a.cos() * 40.0).clamp(16.0, PX_W as f32 - 34.0), (ucy + a.sin() * 30.0).clamp(24.0, PX_H as f32 - 40.0));
            commands.spawn((
                Sprite::from_image(u.img.clone()),
                at(PLAY_X + cx, PLAY_Y + cy, 18.0, 24.0, actor_z(cy + 22.0)),
                PIXEL_LAYER,
                RoomActor,
                FalseSelf { x: cx, y: cy, drift: 0.7 + i as f32 * 0.4 },
                Combatant { team: Team::Enemy, hurt_team: None, damage: None, persistent: false, knock: 0.0 },
                Health { hp: 1, max: 1, defense: 0, invuln: 20, flash: 0 },
                HurtProfile { invuln: 2, flash: 4, kb_base: 0.0, kb_frames: 0 },
                Knockback::default(),
                Hitbox { x: cx + 2.0, y: cy + 3.0, w: 14.0, h: 18.0 },
            ));
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(ucx, ucy), VOID, 12);
        h.flash = 10;
        sfx.write(crate::app::sfx::Sfx("warpCharge"));
    }

    // --- The hex: your hands betray you. ---
    u.hex_cd -= 1;
    if u.hex_cd <= 0 {
        u.hex_cd = 520;
        hexed.0 = 240;
        spawn_burst(&mut commands, &mut rng, Vec2::new(pcx, pcy - 6.0), VOID, 10);
        sfx.write(crate::app::sfx::Sfx("warpCharge"));
    }

    // --- Blinks instead of steps; bolt pairs between. ---
    u.blink_cd -= 1;
    if u.blink_cd <= 0 {
        u.blink_cd = 150 - u.crossed as i32 * 25;
        spawn_burst(&mut commands, &mut rng, Vec2::new(ucx, ucy), VOID, 8);
        let a = rng.0.next_f64() as f32 * std::f32::consts::TAU;
        let d = 46.0 + rng.0.next_f64() as f32 * 30.0;
        u.x = (pcx + a.cos() * d - 9.0).clamp(12.0, PX_W as f32 - 30.0);
        u.y = (pcy + a.sin() * d - 12.0).clamp(22.0, PX_H as f32 - 40.0);
        h.invuln = h.invuln.max(8);
        h.flash = h.flash.max(6);
        sfx.write(crate::app::sfx::Sfx("tink"));
    }
    u.bolt_cd -= 1;
    if u.bolt_cd <= 0 {
        u.bolt_cd = 110 - u.crossed as i32 * 15;
        let base = (pcy - ucy).atan2(pcx - ucx);
        for off in [-0.18, 0.18] {
            let a = base + off;
            commands.spawn((
                EBolt { x: ucx - 4.0, y: ucy - 4.0, vx: a.cos() * 2.4, vy: a.sin() * 2.4, life: 120 },
                Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                HitOnce::default(),
                Hitbox { x: ucx - 1.0, y: ucy - 1.0, w: 7.0, h: 7.0 },
                Sprite::from_image(art.bolt(VOID, 0xf0e0ff)),
                at(PLAY_X + ucx - 3.0, PLAY_Y + ucy - 3.0, 8.0, 8.0, 8.6),
                PIXEL_LAYER,
                RoomActor,
            ));
        }
    }

    // --- Void tears: the vault frays at its edges, and stays frayed. ---
    u.tear_cd -= 1;
    if u.tear_cd <= 0 {
        u.tear_cd = 460;
        const EDGES: [(f32, f32); 4] = [(40.0, 40.0), (232.0, 40.0), (40.0, 156.0), (232.0, 156.0)];
        let (tx, ty) = EDGES[(rng.0.next_f64() * 4.0) as usize % 4];
        commands.spawn((
            Sprite::from_image(u.tear_img.clone()),
            at(PLAY_X + tx, PLAY_Y + ty, 16.0, 12.0, 1.9),
            PIXEL_LAYER,
            RoomActor,
            VoidTear,
            Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: true, knock: 0.0 },
            Hitbox { x: tx + 2.0, y: ty + 2.0, w: 12.0, h: 8.0 },
        ));
        sfx.write(crate::app::sfx::Sfx("stone"));
    }

    // --- The false court mirrors the hero's orbit. ---
    for (mut c, mut ctf, ch) in &mut clones {
        let a = (u.anim as f32) * 0.02 * c.drift;
        c.x = (pcx + a.cos() * 52.0 - 9.0).clamp(12.0, PX_W as f32 - 30.0);
        c.y = (pcy + a.sin() * 36.0 - 12.0).clamp(22.0, PX_H as f32 - 40.0);
        *ctf = at(PLAY_X + c.x, PLAY_Y + c.y, 18.0, 24.0, actor_z(c.y + 22.0));
        let _ = ch;
    }

    // --- The tell: the real one's eyes GLINT. ---
    u.glint -= 1;
    if u.glint <= -70 {
        u.glint = 14;
    }
    spr.color = if u.glint > 0 {
        Color::srgb(1.25, 1.2, 1.4)
    } else {
        Color::WHITE
    };

    // --- Sync. ---
    *hb = Hitbox { x: u.x + 2.0, y: u.y + 3.0, w: 14.0, h: 18.0 };
    let hover = ((u.anim as f32) * 0.07).sin() * 2.0;
    *tf = at(PLAY_X + u.x, PLAY_Y + u.y + hover, 18.0, 24.0, actor_z(u.y + 22.0));
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

/// Burst clones are just smoke; a wounded real one scatters the whole court; the
/// unmade Unmaker leaves its tears to close on their own (room teardown).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    mut hexed: ResMut<crate::app::play::Hexed>,
    unmakers: Query<(Entity, &Unmaker, &Health), Without<FalseSelf>>,
    clones: Query<(Entity, &FalseSelf, &Health), Without<Unmaker>>,
    tears: Query<Entity, With<VoidTear>>,
) {
    let Ok((ue, u, uh)) = unmakers.single() else { return };
    let real_was_hit = uh.flash > 6; // fresh wound this tick-ish
    for (e, c, ch) in &clones {
        if ch.hp > 0 && !real_was_hit {
            continue;
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(c.x + 9.0, c.y + 12.0), VOID, 8);
        commands.entity(e).despawn();
    }
    if uh.hp <= 0 {
        for e in &tears {
            commands.entity(e).despawn();
        }
        hexed.0 = 0; // the rules are yours again
        let (cx, cy) = (u.x + 9.0, u.y + 12.0);
        for i in 0..3 {
            let off = i as f32 * 8.0 - 8.0;
            spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.5), VOID, 14);
        }
        let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
        crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
        crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true, None);
        crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
        stats.bump("kills", 1.0);
        stats.bump_kill("boss");
        sfx.write(crate::app::sfx::Sfx("stone"));
        commands.entity(ue).despawn();
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
        check("unmaker", &UNMAKER, 18);
        check("tear", &TEAR, 16);
    }
}
