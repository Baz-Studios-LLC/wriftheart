//! THE WARREN HYDRA — boss 2 of THE TEN (BOSSES.md): the Vine Warren's guardian.
//!
//! A rooted HEART BULB in the arena's middle, sheathed in bark while any of its
//! vine-serpent HEADS live. Heads erupt from burrows, sway, and lunge at whoever
//! comes close (seeds spat at cowards). Sever a head and its STUMP glows for a
//! few seconds: strike it and that burrow is cauterized FOREVER (the heart flinches
//! for 3); let it tick out and TWO heads regrow. The heart only peels open while
//! zero heads stand — so the last severed head is always a choice: the stump, or
//! the wound. Cauterize all five burrows and the heart lies open for the finish.

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};

const HP: f64 = 46.0; // the js vinewarren pool (x HP_MUL)
const HEAD_HP: i32 = 7; // flat — the heads are the fight's rhythm, not its wall
const STUMP_T: i32 = 210; // ~3.5s to cauterize before the regrowth
const CAUTERIZE_WOUND: i32 = 3; // a burned burrow stings the heart itself
const VINE: u32 = 0x51a04a;
const SAP: u32 = 0xffe080;
const PAL: &[(char, u32)] = &[
    ('G', 0x2e6b34), // deep vine
    ('g', VINE),     // mid vine
    ('V', 0x8fd47a), // pale vine
    ('E', 0xffd040), // serpent eyes
    ('R', 0xe04858), // heart flesh
    ('r', 0x8c1626), // heart deep
    ('S', SAP),      // stump sap
    ('D', 0x4a3a20), // bark
    ('d', 0x7a5c30), // bark light
];

// --- Art ---
const HEART_CLOSED: [&str; 20] = [
    "........KKKKKK........",
    "......KKDdDdDDKK......",
    ".....KDdDDDDDDdDK.....",
    "....KDdDDdDDdDDDdK....",
    "...KDDdDDDDDDDdDDDK...",
    "..KDdDDGGGGGGGGDDdK...",
    "..KDDGGgggggggGGDDK...",
    ".KDdDGgggggggggGDdDK..",
    ".KDDGggGGgggGGggGDDK..",
    ".KDdGggGGgggGGggGdDK..",
    ".KDDGgggggggggggGDDK..",
    ".KDdDGgggggggggGDdDK..",
    "..KDDGGgggggggGGDDK...",
    "..KDdDDGGGGGGGGDdDK...",
    "...KDDdDDDDDDDdDDK....",
    "....KDdDDdDDdDDdK.....",
    ".....KDdDDDDDDdK......",
    "......KKDdDdDKK.......",
    "........KKKKK.........",
    "......................",
];
const HEART_OPEN: [&str; 20] = [
    "..KK....KKKKKK....KK..",
    ".KDdK.KKGgggGKK.KdDK..",
    ".KDdKKGgggggggKKdDK...",
    "..KDGggggggggggGDK....",
    "..KGggRRrrrrRRggGK....",
    ".KGgRRrrrrrrrrRRgGK...",
    ".KGgRrrRRRRRRrrRgGK...",
    ".KGgRrRRWWWWRRrRgGK...",
    ".KGgRrRWWWWWWRrRgGK...",
    ".KGgRrRWWWWWWRrRgGK...",
    ".KGgRrRRWWWWRRrRgGK...",
    ".KGgRrrRRRRRRrrRgGK...",
    ".KGgRRrrrrrrrrRRgGK...",
    "..KGggRRrrrrRRggGK....",
    "..KDGggggggggggGDK....",
    "...KDdGgggggggGdDK....",
    "....KDdGGggGGGdDK.....",
    ".....KKDdDDdDKK.......",
    ".......KKKKK..........",
    "......................",
];
const HEAD_SHUT: [&str; 20] = [
    ".....KKKK.....",
    "...KKgggKK....",
    "..KgggggggK...",
    ".KggVVVggggK..",
    ".KgVEKEVggVK..",
    ".KggVVVggggK..",
    "..KgggggggK...",
    "..KKgggggKK...",
    "....KgggK.....",
    "....KgggK.....",
    "...KggGggK....",
    "...KgGgGgK....",
    "....KgggK.....",
    "....KGgGK.....",
    "...KgGgGgK....",
    "...KggGggK....",
    "....KgGgK.....",
    "....KgggK.....",
    ".....KgK......",
    ".....KgK......",
];
const HEAD_BITE: [&str; 20] = [
    ".KKK.....KKK..",
    "KgggK...KgggK.",
    "KgVgWK.KWgVgK.",
    "KgggWK.KWgggK.",
    ".KgggK.KgggK..",
    ".KgRRKKKRRgK..",
    ".KgRrRRRrRgK..",
    "..KgRRRRRgK...",
    "...KgggggK....",
    "....KgggK.....",
    "...KggGggK....",
    "...KgGgGgK....",
    "....KgggK.....",
    "....KGgGK.....",
    "...KgGgGgK....",
    "...KggGggK....",
    "....KgGgK.....",
    "....KgggK.....",
    ".....KgK......",
    ".....KgK......",
];
const STUMP: [&str; 10] = [
    "..KKKKKK..",
    ".KVSSSSVK.",
    "KVSWWWWSVK",
    "KgSWWWWSgK",
    "KgVSSSSVgK",
    ".KgggggGK.",
    ".KgGgGggK.",
    "..KgggGK..",
    "..KggGgK..",
    "...KKKK...",
];
const BURROW: [&str; 7] = [
    "..KKKKKKKKK...",
    ".KDddddddDK...",
    "KDdKKKKKKdDK..",
    "KdKKKKKKKKdK..",
    "KDdKKKKKKdDK..",
    ".KDddddddDK...",
    "..KKKKKKKKK...",
];
const SEG: [&str; 6] = [".KKKK.", "KgGggK", "KGgVgK", "KggGgK", "KgGggK", ".KKKK."];

/// The five burrow mouths, ringing the heart (room px, burrow-sprite top-left).
const BURROWS: [(f32, f32); 5] = [(56.0, 64.0), (216.0, 64.0), (48.0, 140.0), (224.0, 140.0), (138.0, 166.0)];

#[derive(Clone, Copy, PartialEq)]
enum BurrowState {
    Empty,
    Head,
    Stump,
    Dead, // cauterized — nothing grows here again
}

#[derive(Component)]
pub struct WarrenHydra {
    burrows: [BurrowState; 5],
    burrow_spr: [Option<Entity>; 5],
    open: bool,
    anim: u32,
    closed_img: Handle<Image>,
    open_img: Handle<Image>,
    head_shut: Handle<Image>,
    head_bite: Handle<Image>,
    stump_img: Handle<Image>,
    burrow_img: Handle<Image>,
    seg_img: Handle<Image>,
}

enum HeadPhase {
    Rise,
    Sway,
    Windup,
    Strike,
    Hold,
    Retract,
}

#[derive(Component)]
pub struct HydraHead {
    burrow: usize,
    x: f32,
    y: f32,
    phase: HeadPhase,
    t: i32,
    anim: u32,
    seed: f32,
    strike_cd: i32,
    spit_cd: i32,
    /// Strike geometry: from the sway perch toward the locked target.
    from: (f32, f32),
    target: (f32, f32),
    segs: [Entity; 3],
    biting: bool,
}

#[derive(Component)]
pub struct HydraStump {
    burrow: usize,
    t: i32,
}

#[derive(Component)]
pub struct HydraSeg;

fn burrow_center(i: usize) -> (f32, f32) {
    (BURROWS[i].0 + 7.0, BURROWS[i].1 + 3.0)
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let heart_closed = images.add(crate::gfx::bake(&HEART_CLOSED, PAL));
    let heart_open = images.add(crate::gfx::bake(&HEART_OPEN, PAL));
    let head_shut = images.add(crate::gfx::bake(&HEAD_SHUT, PAL));
    let head_bite = images.add(crate::gfx::bake(&HEAD_BITE, PAL));
    let stump_img = images.add(crate::gfx::bake(&STUMP, PAL));
    let burrow_img = images.add(crate::gfx::bake(&BURROW, PAL));
    let seg_img = images.add(crate::gfx::bake(&SEG, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (hx, hy) = (141.0, 86.0); // the bulb, rooted at the arena's middle
    let mut hydra = WarrenHydra {
        burrows: [BurrowState::Empty; 5],
        burrow_spr: [None; 5],
        open: false,
        anim: 0,
        closed_img: heart_closed.clone(),
        open_img: heart_open,
        head_shut,
        head_bite,
        stump_img,
        burrow_img,
        seg_img,
    };
    for i in [0usize, 1, 4] {
        grow_head(commands, &mut hydra, i, false);
    }
    commands.spawn((
        Sprite::from_image(heart_closed),
        at(PLAY_X + hx, PLAY_Y + hy, 22.0, 20.0, actor_z(hy + 18.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE WARREN HYDRA"),
        crate::app::dungeon::DungeonBoss,
        hydra,
        // The bulb bites nobody — its heads do the biting.
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: None, persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 8, flash: 8, kb_base: 0.0, kb_resist: 0.0, kb_frames: 0 }, // rooted: unshovable
        Knockback::default(),
        Hitbox { x: hx + 2.0, y: hy + 2.0, w: 18.0, h: 16.0 },
    ));
}

/// Stand a serpent up out of burrow `i` (Rise phase: it grows in, briefly safe).
fn grow_head(commands: &mut Commands, hydra: &mut WarrenHydra, i: usize, burst_in: bool) {
    let (cx, cy) = burrow_center(i);
    if hydra.burrow_spr[i].is_none() {
        hydra.burrow_spr[i] = Some(
            commands
                .spawn((
                    Sprite::from_image(hydra.burrow_img.clone()),
                    at(PLAY_X + BURROWS[i].0, PLAY_Y + BURROWS[i].1, 14.0, 7.0, 2.5),
                    PIXEL_LAYER,
                    RoomActor,
                ))
                .id(),
        );
    }
    let segs = std::array::from_fn(|_| {
        commands
            .spawn((
                Sprite::from_image(hydra.seg_img.clone()),
                at(PLAY_X + cx - 3.0, PLAY_Y + cy - 3.0, 6.0, 6.0, actor_z(cy) - 0.1),
                PIXEL_LAYER,
                RoomActor,
                HydraSeg,
            ))
            .id()
    });
    let (hx, hy) = (cx - 7.0, cy - 26.0);
    commands.spawn((
        Sprite::from_image(hydra.head_shut.clone()),
        at(PLAY_X + hx, PLAY_Y + hy, 14.0, 20.0, actor_z(cy + 4.0)),
        PIXEL_LAYER,
        RoomActor,
        HydraHead {
            burrow: i,
            x: hx,
            y: hy,
            phase: HeadPhase::Rise,
            t: 20,
            anim: 0,
            seed: i as f32 * 1.7,
            strike_cd: 90 + i as i32 * 17,
            spit_cd: 120 + i as i32 * 23,
            from: (hx, hy),
            target: (hx, hy),
            segs,
            biting: false,
        },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: true, knock: 0.0 },
        Health { hp: HEAD_HP, max: HEAD_HP, defense: 0, invuln: if burst_in { 20 } else { 30 }, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 0.0, kb_resist: 0.0, kb_frames: 0 }, // rooted in its vine
        Knockback::default(),
        Hitbox { x: hx + 1.0, y: hy + 1.0, w: 12.0, h: 12.0 },
    ));
    hydra.burrows[i] = BurrowState::Head;
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut rng: ResMut<GameRng>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player>,
    mut hearts: Query<
        (&mut WarrenHydra, &mut Health, &mut Sprite, &mut Transform),
        (Without<HydraHead>, Without<HydraStump>, Without<HydraSeg>),
    >,
    mut heads: Query<
        (&mut HydraHead, &mut Health, &mut Hitbox, &mut Sprite, &mut Transform, &mut Visibility),
        (Without<WarrenHydra>, Without<HydraStump>, Without<HydraSeg>, Without<Player>),
    >,
    mut stumps: Query<
        (Entity, &mut HydraStump, &mut Sprite),
        (Without<WarrenHydra>, Without<HydraHead>, Without<HydraSeg>),
    >,
    mut segs: Query<&mut Transform, (With<HydraSeg>, Without<WarrenHydra>, Without<HydraHead>, Without<Player>)>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut hy, mut hh, mut hspr, mut htf)) = hearts.single_mut() else { return };
    hy.anim += 1;

    // --- The heads: rise, sway, rear, LUNGE; seeds for the timid. ---
    let mut alive = 0usize;
    for (mut hd, mut hp, mut hb, mut spr, mut tf, mut vis) in &mut heads {
        alive += 1;
        hd.anim += 1;
        let (bcx, bcy) = burrow_center(hd.burrow);
        let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
        let pd = ((pcx - bcx).powi(2) + (pcy - bcy).powi(2)).sqrt().max(0.001);
        let perch = (
            bcx - 7.0 + ((hd.anim as f32) * 0.045 + hd.seed).sin() * 10.0,
            bcy - 26.0 + ((hd.anim as f32) * 0.08 + hd.seed).sin() * 3.0,
        );
        let mut bite = false;
        match hd.phase {
            HeadPhase::Rise => {
                hp.invuln = hp.invuln.max(4);
                hd.t -= 1;
                let k = 1.0 - (hd.t.max(0) as f32 / 20.0);
                hd.x = bcx - 7.0;
                hd.y = bcy - 3.0 - 23.0 * k;
                if hd.t <= 0 {
                    hd.phase = HeadPhase::Sway;
                }
            }
            HeadPhase::Sway => {
                hd.x = perch.0;
                hd.y = perch.1;
                hd.strike_cd -= 1;
                hd.spit_cd -= 1;
                if hd.strike_cd <= 0 && pd < 78.0 {
                    // Rear back, jaw open — the lunge is telegraphed.
                    hd.phase = HeadPhase::Windup;
                    hd.t = 16;
                    hd.from = (hd.x, hd.y);
                } else if hd.spit_cd <= 0 && pd > 85.0 {
                    // A seed for whoever thinks range is safety.
                    hd.spit_cd = 160;
                    let ang = (pcy - (hd.y + 6.0)).atan2(pcx - (hd.x + 7.0));
                    commands.spawn((
                        EBolt { x: hd.x + 3.0, y: hd.y + 2.0, vx: ang.cos() * 2.0, vy: ang.sin() * 2.0, life: 150 },
                        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(1), persistent: false, knock: 0.0 },
                        HitOnce::default(),
                        Hitbox { x: hd.x + 6.0, y: hd.y + 5.0, w: 7.0, h: 7.0 },
                        Sprite::from_image(art.bolt(VINE, 0xd8ffb0)),
                        at(PLAY_X + hd.x + 4.0, PLAY_Y + hd.y + 3.0, 8.0, 8.0, 8.6),
                        PIXEL_LAYER,
                        RoomActor,
                    ));
                }
            }
            HeadPhase::Windup => {
                hd.t -= 1;
                bite = hd.t < 8;
                // Pull away from the hero — the coil before the spring.
                let (ux, uy) = ((pcx - bcx) / pd, (pcy - bcy) / pd);
                hd.x = hd.from.0 - ux * 5.0;
                hd.y = hd.from.1 - uy * 5.0;
                if hd.t <= 0 {
                    // Lock the lunge at where the hero STANDS (reach capped at the vine's length).
                    let reach = pd.min(48.0);
                    hd.target = (bcx + (pcx - bcx) / pd * reach - 7.0, bcy + (pcy - bcy) / pd * reach - 10.0);
                    hd.from = (hd.x, hd.y);
                    hd.phase = HeadPhase::Strike;
                    hd.t = 9;
                }
            }
            HeadPhase::Strike => {
                bite = true;
                hd.t -= 1;
                let k = 1.0 - (hd.t.max(0) as f32 / 9.0);
                hd.x = hd.from.0 + (hd.target.0 - hd.from.0) * k;
                hd.y = hd.from.1 + (hd.target.1 - hd.from.1) * k;
                if hd.t <= 0 {
                    hd.phase = HeadPhase::Hold;
                    hd.t = 5;
                }
            }
            HeadPhase::Hold => {
                bite = true;
                hd.t -= 1;
                if hd.t <= 0 {
                    hd.phase = HeadPhase::Retract;
                    hd.t = 14;
                    hd.from = (hd.x, hd.y);
                }
            }
            HeadPhase::Retract => {
                hd.t -= 1;
                let k = 1.0 - (hd.t.max(0) as f32 / 14.0);
                hd.x = hd.from.0 + (perch.0 - hd.from.0) * k;
                hd.y = hd.from.1 + (perch.1 - hd.from.1) * k;
                if hd.t <= 0 {
                    hd.phase = HeadPhase::Sway;
                    hd.strike_cd = 100 + hd.burrow as i32 * 17;
                }
            }
        }
        if bite != hd.biting {
            hd.biting = bite;
            spr.image = if bite { hy.head_bite.clone() } else { hy.head_shut.clone() };
        }
        // The neck: three vine balls strung burrow-mouth to head.
        let (hcx, hcy) = (hd.x + 7.0, hd.y + 10.0);
        for (si, se) in hd.segs.iter().enumerate() {
            if let Ok(mut st) = segs.get_mut(*se) {
                let k = (si as f32 + 1.0) / 4.0;
                let sx = bcx + (hcx - bcx) * k;
                let sy = bcy + (hcy - bcy) * k;
                *st = at(PLAY_X + sx - 3.0, PLAY_Y + sy - 3.0, 6.0, 6.0, actor_z(bcy + 3.0) - 0.1);
            }
        }
        *hb = Hitbox { x: hd.x + 1.0, y: hd.y + 1.0, w: 12.0, h: 12.0 };
        *tf = at(PLAY_X + hd.x, PLAY_Y + hd.y, 14.0, 20.0, actor_z(bcy + 4.0));
        *vis = if hp.flash > 0 && (hp.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
    }

    // --- The stumps: glowing timers. Expiry = TWO grow back (this burrow + a fresh one). ---
    for (se, mut st, mut sspr) in &mut stumps {
        st.t -= 1;
        let pulse = 0.6 + 0.4 * ((st.t as f32) * 0.25).sin();
        sspr.color = Color::srgba(1.0, 1.0, 1.0, pulse);
        if st.t <= 0 {
            let b = st.burrow;
            commands.entity(se).despawn();
            grow_head(&mut commands, &mut hy, b, true);
            if let Some(fresh) = (0..5).find(|&i| hy.burrows[i] == BurrowState::Empty) {
                grow_head(&mut commands, &mut hy, fresh, true);
            }
            spawn_burst(&mut commands, &mut rng, Vec2::new(burrow_center(b).0, burrow_center(b).1), VINE, 10);
            sfx.write(crate::app::sfx::Sfx("tink"));
        }
    }

    // --- The heart: barked shut while any head stands; open flesh when none do. ---
    let want_open = alive == 0;
    if want_open != hy.open {
        hy.open = want_open;
        hspr.image = if want_open { hy.open_img.clone() } else { hy.closed_img.clone() };
        hh.flash = 6;
    }
    if !hy.open {
        hh.invuln = hh.invuln.max(2); // blades thunk off the bark
    }
    // The bulb breathes — deeper and quicker with its flesh bared.
    let (rate, depth) = if hy.open { (0.22, 0.06) } else { (0.09, 0.025) };
    let s = 1.0 + ((hy.anim as f32) * rate).sin() * depth;
    htf.scale = Vec3::new(s, s, 1.0);
    hspr.color = if hh.flash > 0 && (hh.flash & 1) == 1 { Color::srgba(1.0, 1.0, 1.0, 0.3) } else { Color::WHITE };
}

/// Severed heads leave stumps; struck stumps cauterize (and sting the heart); the
/// felled heart takes the whole warren down with it.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    mut hearts: Query<(Entity, &mut WarrenHydra, &mut Health), (Without<HydraHead>, Without<HydraStump>)>,
    heads: Query<(Entity, &HydraHead, &Health), (Without<WarrenHydra>, Without<HydraStump>)>,
    stumps: Query<(Entity, &HydraStump, &Health), (Without<WarrenHydra>, Without<HydraHead>)>,
) {
    let Ok((he, mut hy, mut hh)) = hearts.single_mut() else { return };
    // --- Severed heads -> glowing stumps. ---
    for (e, hd, hp) in &heads {
        if hp.hp > 0 {
            continue;
        }
        let (bcx, bcy) = burrow_center(hd.burrow);
        spawn_burst(&mut commands, &mut rng, Vec2::new(hd.x + 7.0, hd.y + 10.0), VINE, 10);
        for s in hd.segs {
            commands.entity(s).despawn();
        }
        commands.entity(e).despawn();
        hy.burrows[hd.burrow] = BurrowState::Stump;
        commands.spawn((
            Sprite::from_image(hy.stump_img.clone()),
            at(PLAY_X + bcx - 5.0, PLAY_Y + bcy - 7.0, 10.0, 10.0, actor_z(bcy + 3.0)),
            PIXEL_LAYER,
            RoomActor,
            HydraStump { burrow: hd.burrow, t: STUMP_T },
            Combatant { team: Team::Enemy, hurt_team: None, damage: None, persistent: false, knock: 0.0 },
            Health { hp: 1, max: 1, defense: 0, invuln: 12, flash: 0 }, // any blow cauterizes
            HurtProfile { invuln: 4, flash: 4, kb_base: 0.0, kb_resist: 0.0, kb_frames: 0 },
            Knockback::default(),
            Hitbox { x: bcx - 5.0, y: bcy - 7.0, w: 10.0, h: 10.0 },
        ));
        sfx.write(crate::app::sfx::Sfx("stone"));
    }
    // --- Struck stumps -> cauterized burrows + a wound straight to the heart. ---
    for (e, st, hp) in &stumps {
        if hp.hp > 0 {
            continue;
        }
        let (bcx, bcy) = burrow_center(st.burrow);
        spawn_burst(&mut commands, &mut rng, Vec2::new(bcx, bcy), SAP, 12);
        commands.entity(e).despawn();
        hy.burrows[st.burrow] = BurrowState::Dead;
        if let Some(bs) = hy.burrow_spr[st.burrow].take() {
            commands.entity(bs).despawn();
        }
        hh.hp = (hh.hp - CAUTERIZE_WOUND).max(1); // the sting can't finish it — the open heart must
        hh.flash = 10;
        sfx.write(crate::app::sfx::Sfx("tink"));
    }
    // --- The heart falls: the whole warren withers. ---
    if hh.hp <= 0 {
        for (e, hd, _) in &heads {
            for s in hd.segs {
                commands.entity(s).despawn();
            }
            commands.entity(e).despawn();
        }
        for (e, ..) in &stumps {
            commands.entity(e).despawn();
        }
        for bs in hy.burrow_spr.iter_mut().filter_map(|b| b.take()) {
            commands.entity(bs).despawn();
        }
        let (cx, cy) = (152.0, 96.0);
        for i in 0..3 {
            let off = i as f32 * 8.0 - 8.0;
            spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.4), 0xe04858, 12);
        }
        let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
        crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
        crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true, None);
        crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
        stats.bump("kills", 1.0);
        stats.bump_kill("boss");
        sfx.write(crate::app::sfx::Sfx("stone"));
        commands.entity(he).despawn();
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
        check("heart_closed", &HEART_CLOSED, 22);
        check("heart_open", &HEART_OPEN, 22);
        check("head_shut", &HEAD_SHUT, 14);
        check("head_bite", &HEAD_BITE, 14);
        check("stump", &STUMP, 10);
        check("burrow", &BURROW, 14);
        check("seg", &SEG, 6);
    }
}
