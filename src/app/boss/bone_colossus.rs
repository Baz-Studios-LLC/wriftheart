//! THE BONE COLOSSUS — boss 1 of THE TEN (BOSSES.md): the crypt's guardian.
//!
//! A giant assembled skeleton, twice the hero's height. Its signature: at each
//! third of its health it COLLAPSES into a bone pile and the SKULL flies free —
//! small, quick, evasive, spitting bone-bolts, and soft (defense -2) while loose.
//! Wound it hard before the timer runs out, because the bones reassemble around
//! wherever the skull fled to — minus an arm each rebuild, and faster. With both
//! arms it hurls rib volleys; one-armed it throws harder; armless it fights with
//! lunging bites and full-ring bone novas. Kill it in either shape.

use bevy::prelude::*;

use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::battle::projectiles::EBolt;
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 42.0; // the js crypt boss's authored pool (x HP_MUL like every foe)
const BONE: u32 = 0xe8e4d0;
const ACCENT: u32 = 0x8fd4ff; // socket-glow blue — bolt cores match the eyes
const PAL: &[(char, u32)] = &[('B', BONE), ('b', 0xb0a890), ('E', ACCENT), ('D', 0x241c28)];

// --- The art: skull + ribcage core + one arm, composed per rebuild cycle so a
// lost arm is really GONE from the sprite (not a palette trick). ---
const SKULL: [&str; 15] = [
    "......KKKKKKKK......",
    "....KKBBBBBBBBKK....",
    "...KBBBBBBBBBBBBK...",
    "..KBBBBBBBBBBBBBBK..",
    ".KBBBBBBBBBBBBBBBBK.",
    ".KBBKKKKBBBBKKKKBBK.",
    ".KBBKEEKBBBBKEEKBBK.",
    ".KBBKKKKBBBBKKKKBBK.",
    ".KBBBBBBBKKBBBBBBBK.",
    "..KBBBBBBBBBBBBBBK..",
    "..KBKBKBKBBKBKBKBK..",
    "..KBKBKBKBBKBKBKBK..",
    "...KBBBBBBBBBBBBK...",
    "....KKKKKKKKKKKK....",
    "....................",
];
const CORE: [&str; 26] = [
    ".......KKKKKKKKKKKK.......",
    "....KKKBbBBBBBBBBbBKKK....",
    "..KKBBBBBBBBBBBBBBBBBBKK..",
    ".KBBBbBBBBBBBBBBBBBBbBBBK.",
    ".KBBK.KKDDDDDDDDDDKK.KBBK.",
    ".KBBK.KBBBBBBBBBBBBK.KBBK.",
    ".KBBK.KKDDDDDDDDDDKK.KBBK.",
    ".KKK..KBBBBBBBBBBBBK..KKK.",
    "......KKDDDDDDDDDDKK......",
    "......KBBBBBBBBBBBBK......",
    ".......KDDDDDDDDDDK.......",
    ".......KBBBBBBBBBBK.......",
    "........KDDDDDDDDK........",
    "........KBbBBBBbBK........",
    ".........KKBBBBKK.........",
    "..........KBBBBK..........",
    ".......KKKBBBBBBKKK.......",
    "......KBbBBBBBBBBbBK......",
    "......KBBK.KKKK.KBBK......",
    "......KBBK......KBBK......",
    "......KBBK......KBBK......",
    "......KBBK......KBBK......",
    ".....KBBBK......KBBBK.....",
    ".....KBbBK......KBbBK.....",
    "....KBBBBK......KBBBBK....",
    "....KKKKK........KKKKK....",
];
const ARM: [&str; 24] = [
    "KBBBK....",
    "KBbBBK...",
    "KBBBBK...",
    ".KBBBK...",
    ".KBbBK...",
    ".KBBK....",
    "..KBBK...",
    "..KBbBK..",
    "..KBBBK..",
    "..KBbBK..",
    "..KBBK...",
    ".KBBBK...",
    ".KBbBK...",
    ".KBBBK...",
    ".KBBK....",
    "KBBBBK...",
    "KBbBBBK..",
    "KBBBBBBK.",
    "KBbBKBBK.",
    "KBBKKBBK.",
    ".KK.KBKK.",
    ".K...KK..",
    ".........",
    ".........",
];
const PILE: [&str; 8] = [
    "..........KKKK............",
    ".....KKK.KBBbBK..KKK......",
    "....KBbBKKBBBBBKKBbBK.....",
    "...KBBbBBBBbBBBBbBBBBbBK..",
    "..KBbBBBBBBbBBBBBBBBbBBK..",
    ".KBBBBbBBBBBBbBBBBBBBbBK..",
    "KBbBBBBBBBBbBBBBBBBBBBbBK.",
    "KKKKKKKKKKKKKKKKKKKKKKKKKK",
];

const BODY_W: f32 = 44.0;
const BODY_H: f32 = 41.0;
const REASSEMBLE_T: i32 = 420; // seven seconds of skull-chase per collapse

/// Compose the full body for a rebuild cycle: skull banded over the core, arms
/// flanking (cycle 0 = both, 1 = right only, 2 = none). Widths are guaranteed by
/// construction — every row is arm(9) + core(26) + arm(9) or padded skull(20).
fn build_body(cycle: u8) -> Vec<String> {
    let blank = ".........";
    let mirror = |s: &str| s.chars().rev().collect::<String>();
    let mut rows = Vec::with_capacity(41);
    for s in SKULL.iter() {
        rows.push(format!("............{s}............"));
    }
    for (cr, core_row) in CORE.iter().enumerate() {
        let ar = cr as i32 - 2; // arms hang from the shoulder line (core row 2)
        let (l, r) = if (0..24).contains(&ar) {
            let a = ARM[ar as usize];
            (
                if cycle == 0 { mirror(a) } else { blank.to_string() },
                if cycle <= 1 { a.to_string() } else { blank.to_string() },
            )
        } else {
            (blank.to_string(), blank.to_string())
        };
        rows.push(format!("{l}{core_row}{r}"));
    }
    rows
}

enum Form {
    Assembled,
    Collapsed { t: i32 },
}

#[derive(Clone, Copy)]
enum Atk {
    Volley(u8),
    Nova(u8),
    Lunge,
}
const CYCLE0: [Atk; 3] = [Atk::Volley(3), Atk::Lunge, Atk::Nova(16)];
const CYCLE1: [Atk; 4] = [Atk::Volley(5), Atk::Lunge, Atk::Nova(16), Atk::Volley(5)];
const CYCLE2: [Atk; 3] = [Atk::Lunge, Atk::Nova(24), Atk::Lunge];

#[derive(Component)]
pub struct BoneColossus {
    pub x: f32,
    pub y: f32,
    form: Form,
    cycle: u8,   // rebuilds survived: 0 both arms, 1 one arm, 2 armless
    crossed: u8, // health thirds crossed — becomes `cycle` on the next rebuild
    cd: i32,
    atk_idx: usize,
    windup: i32, // nova telegraph: the frame shudders, then the ring
    pending_nova: Option<u8>,
    dash: Option<(f32, f32, i32)>,
    spit_cd: i32,
    orbit_flip: f32,
    anim: u32,
    pile: Option<Entity>,
    bodies: [Handle<Image>; 3],
    skull: Handle<Image>,
    pile_img: Handle<Image>,
}

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>, x: f32, y: f32) {
    let bake = |g: &[String], images: &mut Assets<Image>| {
        let refs: Vec<&str> = g.iter().map(|s| s.as_str()).collect();
        images.add(crate::gfx::bake(&refs, PAL))
    };
    let bodies = [
        bake(&build_body(0), images),
        bake(&build_body(1), images),
        bake(&build_body(2), images),
    ];
    let skull = images.add(crate::gfx::bake(&SKULL, PAL));
    let pile_img = images.add(crate::gfx::bake(&PILE, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    commands.spawn((
        Sprite::from_image(bodies[0].clone()),
        at(PLAY_X + x, PLAY_Y + y, BODY_W, BODY_H, actor_z(y + BODY_H - 2.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE BONE COLOSSUS"),
        crate::app::dungeon::DungeonBoss,
        BoneColossus {
            x,
            y,
            form: Form::Assembled,
            cycle: 0,
            crossed: 0,
            cd: 70, // a beat of dread before the first attack (js grace)
            atk_idx: 0,
            windup: 0,
            pending_nova: None,
            dash: None,
            spit_cd: 50,
            orbit_flip: 1.0,
            anim: 0,
            pile: None,
            bodies,
            skull,
            pile_img,
        },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(3), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        // js boss knockResist 0.92: it barely flinches.
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2 * (1.0 - 0.92), kb_frames: 11 },
        Knockback::default(),
        Hitbox { x: x + 4.0, y: y + 16.0, w: 36.0, h: 22.0 },
    ));
}

/// One bone bolt (the shared EBolt kit, bone-white with a socket-glow core).
fn bolt(commands: &mut Commands, art: &crate::actors::mobs::MobArtBank, x: f32, y: f32, ang: f32, sp: f32) {
    commands.spawn((
        EBolt { x, y, vx: ang.cos() * sp, vy: ang.sin() * sp, life: 130 },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
        HitOnce::default(),
        Hitbox { x: x + 3.0, y: y + 3.0, w: 7.0, h: 7.0 },
        Sprite::from_image(art.bolt(BONE, 0xffffff)),
        at(PLAY_X + x + 1.0, PLAY_Y + y + 1.0, 8.0, 8.0, 8.6),
        PIXEL_LAYER,
        RoomActor,
    ));
}

fn step(b: &mut BoneColossus, grid: &crate::room::RoomGrid, blockers: &crate::app::room_props::RoomBlockers, bx: (f32, f32, f32, f32), dx: f32, dy: f32) -> bool {
    let mut moved = false;
    for (sx, sy) in [(dx, 0.0), (0.0, dy)] {
        if sx == 0.0 && sy == 0.0 {
            continue;
        }
        let (nx, ny) = (b.x + sx, b.y + sy);
        let nb = (nx + bx.0, ny + bx.1, bx.2, bx.3);
        if !grid.box_hits_solid(nb.0, nb.1, nb.2, nb.3)
            && !blockers.blocks((b.x + bx.0, b.y + bx.1, bx.2, bx.3), nb)
        {
            b.x = nx;
            b.y = ny;
            moved = true;
        }
    }
    moved
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    grid: Res<crate::app::play::CurGrid>,
    blockers: Res<crate::app::room_props::RoomBlockers>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut rng: ResMut<GameRng>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player>,
    mut q: Query<
        (&mut BoneColossus, &mut Health, &mut Combatant, &mut Knockback, &mut Hitbox, &mut Sprite, &mut Transform),
        Without<Player>,
    >,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    for (mut b, mut h, mut cb, kb, mut hb, mut spr, mut tf) in &mut q {
        b.anim += 1;
        let frac = h.hp as f32 / h.max.max(1) as f32;
        let want: u8 = if frac <= 1.0 / 3.0 { 2 } else if frac <= 2.0 / 3.0 { 1 } else { 0 };
        match b.form {
            Form::Assembled => {
                // --- THE COLLAPSE: a third falls — the bones clatter down, the skull flies. ---
                if want > b.crossed && h.hp > 0 {
                    b.crossed = want;
                    b.form = Form::Collapsed { t: REASSEMBLE_T };
                    let (px, py) = (b.x + 9.0, b.y + BODY_H - 10.0);
                    b.pile = Some(
                        commands
                            .spawn((
                                Sprite::from_image(b.pile_img.clone()),
                                at(PLAY_X + px, PLAY_Y + py, 26.0, 8.0, actor_z(py + 8.0)),
                                PIXEL_LAYER,
                                RoomActor,
                            ))
                            .id(),
                    );
                    spawn_burst(&mut commands, &mut rng, Vec2::new(b.x + 22.0, b.y + 22.0), BONE, 14);
                    // The component's (x, y) tracks the ACTIVE shape: now it's the skull.
                    b.x += 12.0;
                    b.y += 2.0;
                    spr.image = b.skull.clone();
                    cb.damage = Some(2);
                    h.defense = -2; // loose, the skull is SOFT — every blow bites deeper
                    h.invuln = h.invuln.max(18);
                    h.flash = 10;
                    b.dash = None;
                    b.windup = 0;
                    b.pending_nova = None;
                    b.spit_cd = 40;
                    sfx.write(crate::app::sfx::Sfx("stone"));
                }
                if matches!(b.form, Form::Collapsed { .. }) {
                    // fall through to the shared sync below on the next tick
                } else {
                    // --- Assembled brain: dash > windup > next attack > stalk. ---
                    let (pdx, pdy) = (p.x + 8.0 - (b.x + 22.0), p.y + 8.0 - (b.y + 18.0));
                    let pd = (pdx * pdx + pdy * pdy).sqrt().max(0.001);
                    let bx = (4.0, 16.0, 36.0, 22.0);
                    if let Some((vx, vy, mut t)) = b.dash {
                        t -= 1;
                        let moved = step(&mut b, &grid.0, &blockers, bx, vx, vy);
                        b.dash = if moved && t > 0 { Some((vx, vy, t)) } else { None };
                    } else if b.windup > 0 {
                        b.windup -= 1;
                        if b.windup == 0
                            && let Some(n) = b.pending_nova.take()
                        {
                            // THE STOMP NOVA: a full ring of bone bursting outward.
                            let (cx, cy) = (b.x + 18.0, b.y + 18.0);
                            for i in 0..n {
                                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                                bolt(&mut commands, &art, cx, cy, a, 1.8);
                            }
                            spawn_burst(&mut commands, &mut rng, Vec2::new(cx + 4.0, cy + 4.0), BONE, 8);
                            sfx.write(crate::app::sfx::Sfx("stone"));
                        }
                    } else {
                        b.cd -= 1;
                        if b.cd <= 0 {
                            let list: &[Atk] = match b.cycle {
                                0 => &CYCLE0,
                                1 => &CYCLE1,
                                _ => &CYCLE2,
                            };
                            let atk = list[b.atk_idx % list.len()];
                            b.atk_idx += 1;
                            let scale = 1.0 - b.cycle as f32 * 0.15;
                            match atk {
                                Atk::Volley(n) => {
                                    // Rib volley: a fanned spread hurled at the hero.
                                    let base = pdy.atan2(pdx);
                                    let (cx, cy) = (b.x + 18.0, b.y + 14.0);
                                    for i in 0..n {
                                        let a = base + 0.5 * (i as f32 - (n - 1) as f32 / 2.0) / ((n - 1) as f32 / 2.0).max(1.0);
                                        bolt(&mut commands, &art, cx, cy, a, 2.2);
                                    }
                                    b.cd = (110.0 * scale) as i32;
                                }
                                Atk::Nova(n) => {
                                    b.windup = 22;
                                    b.pending_nova = Some(n);
                                    b.cd = (150.0 * scale) as i32;
                                }
                                Atk::Lunge => {
                                    // The grasp (armless: the BITE) — a heavy dash at the hero.
                                    let sp = 2.4 + b.cycle as f32 * 0.5;
                                    b.dash = Some((pdx / pd * sp, pdy / pd * sp, 20));
                                    b.cd = (130.0 * scale) as i32;
                                }
                            }
                        } else {
                            // The stalk: slow, axis-locked, inevitable.
                            let sp = 0.42 + b.cycle as f32 * 0.15;
                            step(&mut b, &grid.0, &blockers, bx, pdx.signum() * sp, pdy.signum() * sp);
                        }
                    }
                }
            }
            Form::Collapsed { .. } => {
                // --- The skull, loose: keep away, spit, run the clock. ---
                if let Form::Collapsed { t } = &mut b.form {
                    *t -= 1;
                }
                let (pdx, pdy) = (p.x + 8.0 - (b.x + 10.0), p.y + 8.0 - (b.y + 7.0));
                let pd = (pdx * pdx + pdy * pdy).sqrt().max(0.001);
                let bx = (2.0, 1.0, 16.0, 13.0);
                if b.anim.is_multiple_of(120) {
                    b.orbit_flip = -b.orbit_flip;
                }
                let (ux, uy) = (pdx / pd, pdy / pd);
                let (mut vx, mut vy) = (-uy * b.orbit_flip, ux * b.orbit_flip); // circle the hero
                if pd < 60.0 {
                    vx -= ux * 1.4;
                    vy -= uy * 1.4;
                } else if pd > 100.0 {
                    vx += ux * 0.9;
                    vy += uy * 0.9;
                }
                if !step(&mut b, &grid.0, &blockers, bx, vx, vy) {
                    b.orbit_flip = -b.orbit_flip; // cornered: circle the other way
                }
                b.spit_cd -= 1;
                if b.spit_cd <= 0 {
                    b.spit_cd = 64 - b.crossed as i32 * 8;
                    let base = pdy.atan2(pdx);
                    for i in -1..=1i32 {
                        bolt(&mut commands, &art, b.x + 6.0, b.y + 4.0, base + i as f32 * 0.26, 2.5);
                    }
                }
                let done = matches!(b.form, Form::Collapsed { t } if t <= 0);
                if done && h.hp > 0 {
                    // --- REASSEMBLY: the bones answer the skull's call, one arm poorer. ---
                    b.cycle = b.crossed;
                    b.form = Form::Assembled;
                    b.x = (b.x - 12.0).clamp(4.0, PX_W as f32 - BODY_W - 4.0);
                    b.y = (b.y - 2.0).clamp(4.0, PX_H as f32 - BODY_H - 4.0);
                    spr.image = b.bodies[b.cycle.min(2) as usize].clone();
                    cb.damage = Some(3);
                    h.defense = 0;
                    h.invuln = h.invuln.max(24);
                    h.flash = 12;
                    b.cd = 50;
                    if let Some(pile) = b.pile.take() {
                        commands.entity(pile).despawn();
                    }
                    spawn_burst(&mut commands, &mut rng, Vec2::new(b.x + 22.0, b.y + 22.0), BONE, 14);
                    sfx.write(crate::app::sfx::Sfx("stone"));
                }
            }
        }
        // --- Shared sync: hitbox + transform follow the active shape. ---
        let assembled = matches!(b.form, Form::Assembled);
        if assembled {
            *hb = Hitbox { x: b.x + 4.0, y: b.y + 16.0, w: 36.0, h: 22.0 };
            let bob = (b.anim as f32 * 0.08).sin() * 1.5;
            let jit = if b.windup > 0 { ((b.anim as f32) * 0.9).sin() * 1.2 } else { 0.0 };
            *tf = at(PLAY_X + b.x + jit, PLAY_Y + b.y + bob, BODY_W, BODY_H, actor_z(b.y + BODY_H - 2.0));
        } else {
            *hb = Hitbox { x: b.x + 2.0, y: b.y + 1.0, w: 16.0, h: 13.0 };
            let hover = (b.anim as f32 * 0.15).sin() * 2.5;
            *tf = at(PLAY_X + b.x, PLAY_Y + b.y + hover, 20.0, 15.0, actor_z(b.y + 16.0));
        }
        // Hit flash: skip-draw on alternating frames (the shared js rule).
        spr.color = if h.flash > 0 && (h.flash & 1) == 1 {
            Color::srgba(1.0, 1.0, 1.0, 0.0)
        } else {
            Color::WHITE
        };
        let _ = kb; // resolve_combat writes it; the 0.92 resist keeps it near-nil
    }
}

/// The fall of the colossus: a triple burst of bone, the js boss purse (30-69 coin +
/// a guaranteed potion + 45 xp), and the empty DungeonBoss query lets navigate()
/// unseal the arena and stand up the rune + shard + gilded chest.
#[allow(clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    q: Query<(Entity, &BoneColossus, &Health)>,
) {
    for (e, b, h) in &q {
        if h.hp > 0 {
            continue;
        }
        let (cx, cy) = (b.x + 16.0, b.y + 14.0);
        for i in 0..3 {
            let off = i as f32 * 7.0 - 7.0;
            spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.5), BONE, 12);
        }
        let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
        crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
        crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true);
        crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
        stats.bump("kills", 1.0);
        stats.bump_kill("boss");
        if let Some(pile) = b.pile {
            commands.entity(pile).despawn();
        }
        sfx.write(crate::app::sfx::Sfx("stone"));
        commands.entity(e).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids_are_rectangular() {
        for r in SKULL {
            assert_eq!(r.chars().count(), 20, "skull row width");
        }
        for r in CORE {
            assert_eq!(r.chars().count(), 26, "core row width");
        }
        for r in ARM {
            assert_eq!(r.chars().count(), 9, "arm row width");
        }
        for r in PILE {
            assert_eq!(r.chars().count(), 26, "pile row width");
        }
        for cycle in 0..3u8 {
            let body = build_body(cycle);
            assert_eq!(body.len(), 41, "body height");
            for row in &body {
                assert_eq!(row.chars().count(), 44, "body row width (cycle {cycle})");
            }
        }
    }
}
