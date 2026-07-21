//! THE GLACIER MAW — boss 4 of THE TEN (BOSSES.md): the Frost Cavern's guardian.
//!
//! An ice-worm that fights from UNDER the floor. Burrowed it is untouchable — only
//! a racing CRACK betrays it, hunting your feet; when the crack catches you (or
//! gives up waiting) it ERUPTS in a ring of ice. Four ICE PILLARS stand in the
//! arena: bait the eruption to burst beside one and the pillar shatters ON it —
//! stunned and soft (defense -2) for a long window. Otherwise you make do with its
//! surfaced spells: a slithering, lunge-biting worm that re-burrows all too soon.
//! Every dive spreads more hoarfrost across the floor (visual — true slide physics
//! is flagged as a follow-up).

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 56.0; // the js frostcavern pool (x HP_MUL)
const FROST: u32 = 0x9fd8f0;
const PALE: u32 = 0xdff2ff;
const PAL: &[(char, u32)] = &[('F', FROST), ('W', PALE), ('G', 0x5890b8)];

const MAW: [&str; 26] = [
    "........KKKKKKKK........",
    "......KKWWWWWWWWKK......",
    ".....KWWFFFFFFFFWWK.....",
    "....KWFFKKKKKKKKFFWK....",
    "...KWFKKWKWKWKWKKKFWK...",
    "...KWFKKKKKKKKKKKKFWK...",
    "...KWFKWKKKKKKKKWKFWK...",
    "...KWFKKKKKKKKKKKKFWK...",
    "...KWFKKWKWKWKWKKKFWK...",
    "....KWFFKKKKKKKKFFWK....",
    ".....KWWFFFFFFFFWWK.....",
    "......KKWFFFFFFWKK......",
    ".....KWFFWWFFWWFFWK.....",
    ".....KWFFFFFFFFFFWK.....",
    "......KKFFFFFFFFKK......",
    ".....KWFFWWFFWWFFWK.....",
    ".....KWFFFFFFFFFFWK.....",
    "......KKFFFFFFFFKK......",
    ".....KWFFWWFFWWFFWK.....",
    ".....KWFFFFFFFFFFWK.....",
    "......KKWFFFFFFWKK......",
    ".......KKWFFFFWKK.......",
    "........KKWFFWKK........",
    ".........KKWWKK.........",
    "..........KKKK..........",
    "........................",
];
const PILLAR: [&str; 24] = [
    "....KKKKKK....",
    "..KKWWWWWWKK..",
    ".KWWFFFFFFWWK.",
    ".KWFFWWFFGFWK.",
    ".KWFFWWFFFFWK.",
    ".KWFFFFFFGFWK.",
    ".KWFGFFFFFFWK.",
    ".KWFFFFWWFFWK.",
    ".KWFFFFWWFFWK.",
    ".KWFGFFFFFFWK.",
    ".KWFFFFFFGFWK.",
    ".KWFFWWFFFFWK.",
    ".KWFFWWFFFFWK.",
    ".KWFGFFFFFFWK.",
    ".KWFFFFFFGFWK.",
    ".KWFFFFWWFFWK.",
    ".KWFFFFWWFFWK.",
    ".KWFGFFFFFFWK.",
    ".KWFFFFFFGFWK.",
    ".KWFFFFFFFFWK.",
    ".KWWFFFFFFWWK.",
    "..KKWWWWWWKK..",
    "....KKKKKK....",
    "..............",
];
const CRACK: [&str; 6] = [
    "..W...WW....",
    ".WWW.WWWW..W",
    "WWGWWWGGWWWW",
    ".WGGWWWGGWW.",
    "..WW...WGW..",
    "...W....W...",
];
const SHEEN: [&str; 8] = [
    "W...G...",
    "..W...W.",
    ".G..W...",
    "W..G..W.",
    "..W...G.",
    "G...W...",
    ".W...W.G",
    "...G....",
];

/// The four pillar spots (sprite top-left, room px).
const PILLARS: [(f32, f32); 4] = [(56.0, 52.0), (232.0, 52.0), (56.0, 132.0), (232.0, 132.0)];

enum Phase {
    /// Under the ice: the crack races at the hero's feet.
    Burrowed { t: i32 },
    /// The rumble before the burst (at a locked spot).
    Erupting { t: i32 },
    /// Up and biting.
    Surfaced { t: i32 },
    /// Burst a pillar onto its own head: long, soft, sorry.
    Stunned { t: i32 },
}

#[derive(Component)]
pub struct GlacierMaw {
    x: f32, // active anchor: crack tip while burrowed, body top-left surfaced
    y: f32,
    phase: Phase,
    anim: u32,
    lunge: Option<(f32, f32, i32)>,
    lunge_cd: i32,
    crack_drip: i32,
    dives: u32,
    crack_img: Handle<Image>,
    sheen_img: Handle<Image>,
}

#[derive(Component)]
pub struct IcePillar {
    idx: usize,
    blocker: (f32, f32, f32, f32),
}

#[derive(Component)]
pub struct CrackDecal {
    t: i32,
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>, blockers: &mut crate::app::room_props::RoomBlockers) {
    let maw_img = images.add(crate::gfx::bake(&MAW, PAL));
    let pillar_img = images.add(crate::gfx::bake(&PILLAR, PAL));
    let crack_img = images.add(crate::gfx::bake(&CRACK, PAL));
    let sheen_img = images.add(crate::gfx::bake(&SHEEN, PAL));
    for (i, (px, py)) in PILLARS.iter().enumerate() {
        let blocker = (px + 2.0, py + 12.0, 10.0, 10.0);
        blockers.0.push(blocker);
        commands.spawn((
            Sprite::from_image(pillar_img.clone()),
            at(PLAY_X + px, PLAY_Y + py, 14.0, 24.0, actor_z(py + 22.0)),
            PIXEL_LAYER,
            RoomActor,
            IcePillar { idx: i, blocker },
        ));
    }
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (mx, my) = (140.0, 90.0);
    commands.spawn((
        Sprite::from_image(maw_img),
        at(PLAY_X + mx, PLAY_Y + my, 24.0, 26.0, actor_z(my + 24.0)),
        PIXEL_LAYER,
        RoomActor,
        Visibility::Hidden, // it starts UNDER the ice
        super::BossName("THE GLACIER MAW"),
        crate::app::dungeon::DungeonBoss,
        GlacierMaw {
            x: mx,
            y: my,
            phase: Phase::Burrowed { t: 260 },
            anim: 0,
            lunge: None,
            lunge_cd: 80,
            crack_drip: 0,
            dives: 0,
            crack_img,
            sheen_img,
        },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: None, persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 1, invuln: 30, flash: 0 }, // the js cave-line armor
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2 * (1.0 - 0.92), kb_frames: 11 },
        Knockback::default(),
        Hitbox { x: -40.0, y: -40.0, w: 1.0, h: 1.0 }, // parked while burrowed
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut rng: ResMut<GameRng>,
    mut blockers: ResMut<crate::app::room_props::RoomBlockers>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player>,
    mut maws: Query<
        (&mut GlacierMaw, &mut Health, &mut Combatant, &mut Hitbox, &mut Sprite, &mut Transform, &mut Visibility),
        (Without<Player>, Without<IcePillar>, Without<CrackDecal>),
    >,
    pillars: Query<(Entity, &IcePillar), Without<GlacierMaw>>,
    mut decals: Query<(Entity, &mut CrackDecal, &mut Sprite), (Without<GlacierMaw>, Without<IcePillar>)>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut m, mut h, mut cb, mut hb, _spr, mut tf, mut vis)) = maws.single_mut() else { return };
    m.anim += 1;
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);

    // Old crack decals melt away.
    for (e, mut d, mut ds) in &mut decals {
        d.t -= 1;
        if d.t <= 0 {
            commands.entity(e).despawn();
        } else {
            ds.color = Color::srgba(1.0, 1.0, 1.0, (d.t as f32 / 60.0).min(1.0));
        }
    }

    // Take-run: own the phase so arm bodies can touch the rest of `m` freely.
    let mut phase = std::mem::replace(&mut m.phase, Phase::Burrowed { t: 0 });
    match &mut phase {
        Phase::Burrowed { t } => {
            *t -= 1;
            h.invuln = h.invuln.max(2); // under the ice: blades find nothing
            cb.damage = None;
            *vis = Visibility::Hidden;
            *hb = Hitbox { x: -40.0, y: -40.0, w: 1.0, h: 1.0 };
            // The crack races at the hero's feet.
            let (dx, dy) = (pcx - m.x, pcy - m.y);
            let d = (dx * dx + dy * dy).sqrt().max(0.001);
            let sp = 1.7 + m.dives as f32 * 0.08;
            m.x = (m.x + dx / d * sp).clamp(12.0, PX_W as f32 - 24.0);
            m.y = (m.y + dy / d * sp).clamp(24.0, PX_H as f32 - 20.0);
            m.crack_drip -= 1;
            if m.crack_drip <= 0 {
                m.crack_drip = 5;
                let jx = (rng.0.next_f64() as f32 - 0.5) * 6.0;
                commands.spawn((
                    Sprite::from_image(m.crack_img.clone()),
                    at(PLAY_X + m.x - 6.0 + jx, PLAY_Y + m.y - 3.0, 12.0, 6.0, 1.8),
                    PIXEL_LAYER,
                    RoomActor,
                    CrackDecal { t: 70 },
                ));
                if m.anim.is_multiple_of(30) {
                    sfx.write(crate::app::sfx::Sfx("stone"));
                }
            }
            if d < 14.0 || *t <= 0 {
                phase = Phase::Erupting { t: 30 };
                sfx.write(crate::app::sfx::Sfx("stone"));
            }
        }
        Phase::Erupting { t } => {
            *t -= 1;
            h.invuln = h.invuln.max(2);
            // The rumble: cracks star out from the locked spot.
            if *t % 6 == 0 {
                let a = (*t as f32) * 1.1;
                commands.spawn((
                    Sprite::from_image(m.crack_img.clone()),
                    at(PLAY_X + m.x - 6.0 + a.cos() * 10.0, PLAY_Y + m.y - 3.0 + a.sin() * 8.0, 12.0, 6.0, 1.8),
                    PIXEL_LAYER,
                    RoomActor,
                    CrackDecal { t: 50 },
                ));
            }
            if *t <= 0 {
                // THE BURST: ice ring + hoarfrost spreads + (maybe) a pillar comes down on it.
                spawn_burst(&mut commands, &mut rng, Vec2::new(m.x, m.y), PALE, 14);
                for i in 0..8 {
                    let a = i as f32 / 8.0 * std::f32::consts::TAU;
                    commands.spawn((
                        EBolt { x: m.x - 4.0, y: m.y - 4.0, vx: a.cos() * 1.9, vy: a.sin() * 1.9, life: 90 },
                        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                        HitOnce::default(),
                        Hitbox { x: m.x - 1.0, y: m.y - 1.0, w: 7.0, h: 7.0 },
                        Sprite::from_image(art.bolt(FROST, 0xffffff)),
                        at(PLAY_X + m.x - 3.0, PLAY_Y + m.y - 3.0, 8.0, 8.0, 8.6),
                        PIXEL_LAYER,
                        RoomActor,
                    ));
                }
                m.dives += 1;
                for _ in 0..6 {
                    let sx = 16.0 + rng.0.next_f64() as f32 * (PX_W as f32 - 48.0);
                    let sy = 32.0 + rng.0.next_f64() as f32 * (PX_H as f32 - 64.0);
                    let mut s = Sprite::from_image(m.sheen_img.clone());
                    s.color = Color::srgba(1.0, 1.0, 1.0, 0.4);
                    commands.spawn((s, at(PLAY_X + sx, PLAY_Y + sy, 8.0, 8.0, 1.5), PIXEL_LAYER, RoomActor));
                }
                // Bait check: burst beside a pillar -> it shatters ON the maw.
                let mut stunned = false;
                for (pe, pil) in &pillars {
                    let (pcxp, pcyp) = (PILLARS[pil.idx].0 + 7.0, PILLARS[pil.idx].1 + 18.0);
                    if ((pcxp - m.x).powi(2) + (pcyp - m.y).powi(2)).sqrt() < 22.0 {
                        spawn_burst(&mut commands, &mut rng, Vec2::new(pcxp, pcyp - 8.0), PALE, 16);
                        blockers.0.retain(|r| *r != pil.blocker);
                        commands.entity(pe).despawn();
                        stunned = true;
                        break;
                    }
                }
                // Up it comes, centred on the burst.
                m.x = (m.x - 12.0).clamp(8.0, PX_W as f32 - 32.0);
                m.y = (m.y - 13.0).clamp(20.0, PX_H as f32 - 34.0);
                cb.damage = Some(3);
                *vis = Visibility::Inherited;
                if stunned {
                    phase = Phase::Stunned { t: 210 };
                    h.defense = -2; // the pillar cracked its armor wide open
                    h.flash = 14;
                    sfx.write(crate::app::sfx::Sfx("warpCharge"));
                } else {
                    phase = Phase::Surfaced { t: 240 };
                    h.defense = 1;
                    sfx.write(crate::app::sfx::Sfx("stone"));
                }
            }
        }
        Phase::Surfaced { t } => {
            *t -= 1;
            if let Some((vx, vy, lt)) = &mut m.lunge {
                let (vx, vy, mut lt2) = (*vx, *vy, *lt);
                lt2 -= 1;
                m.x = (m.x + vx).clamp(8.0, PX_W as f32 - 32.0);
                m.y = (m.y + vy).clamp(20.0, PX_H as f32 - 34.0);
                m.lunge = if lt2 > 0 { Some((vx, vy, lt2)) } else { None };
            } else {
                // The slither: slow drift toward the hero, then the bite.
                let (dx, dy) = (pcx - (m.x + 12.0), pcy - (m.y + 6.0));
                let d = (dx * dx + dy * dy).sqrt().max(0.001);
                m.x += dx / d * 0.5;
                m.y += dy / d * 0.5;
                m.lunge_cd -= 1;
                if m.lunge_cd <= 0 {
                    m.lunge_cd = 80;
                    let sp = 2.8;
                    m.lunge = Some((dx / d * sp, dy / d * sp, 14));
                    h.flash = 4;
                }
            }
            if *t <= 0 {
                // Back under the ice.
                phase = Phase::Burrowed { t: 260 };
                spawn_burst(&mut commands, &mut rng, Vec2::new(m.x + 12.0, m.y + 13.0), PALE, 10);
                m.x += 12.0;
                m.y += 13.0;
                sfx.write(crate::app::sfx::Sfx("stone"));
            }
        }
        Phase::Stunned { t } => {
            *t -= 1;
            // Reeling: no bites, no motion — carve it up.
            if *t <= 0 {
                phase = Phase::Surfaced { t: 160 };
                h.defense = 1;
            }
        }
    }

    m.phase = phase;

    // --- Sync (surfaced shapes only; burrowed parked the box above). ---
    if !matches!(m.phase, Phase::Burrowed { .. }) && !matches!(m.phase, Phase::Erupting { .. }) {
        *hb = Hitbox { x: m.x + 3.0, y: m.y + 4.0, w: 18.0, h: 20.0 };
        let sway = ((m.anim as f32) * 0.12).sin() * 1.5;
        let slump = if matches!(m.phase, Phase::Stunned { .. }) { 3.0 } else { 0.0 };
        *tf = at(PLAY_X + m.x + sway, PLAY_Y + m.y + slump, 24.0, 26.0, actor_z(m.y + 24.0));
        *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
    }
}

/// The fall: the cavern keeps its scars (shattered pillars stay shattered), the
/// worm keeps the js boss purse.
#[allow(clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    maws: Query<(Entity, &GlacierMaw, &Health)>,
    decals: Query<Entity, With<CrackDecal>>,
) {
    for (e, m, h) in &maws {
        if h.hp > 0 {
            continue;
        }
        for d in &decals {
            commands.entity(d).despawn();
        }
        let (cx, cy) = (m.x + 12.0, m.y + 13.0);
        for i in 0..3 {
            let off = i as f32 * 7.0 - 7.0;
            spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.5), FROST, 12);
        }
        let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
        crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
        crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true, None);
        crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
        stats.bump("kills", 1.0);
        stats.bump_kill("boss");
        sfx.write(crate::app::sfx::Sfx("stone"));
        commands.entity(e).despawn();
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
        check("maw", &MAW, 24);
        check("pillar", &PILLAR, 14);
        check("crack", &CRACK, 12);
        check("sheen", &SHEEN, 8);
    }
}
