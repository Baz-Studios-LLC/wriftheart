//! THE MYCELIUM THRONE — boss 7 of THE TEN (BOSSES.md): the Fungal Deep's guardian.
//!
//! The boss is the ROOM. A shelf-fungus THRONE squats at the hall's head, deaf to
//! blades while its NETWORK lives: five pustule NODES seeded across the floor, each
//! creeping its SPORE CARPET outward tile by tile. The carpet bogs your boots, and
//! now and then it HATCHES a sporeling under you. Nodes erupt spore-rings when
//! approached. Cut every node and the network dies — the carpet recedes, and the
//! throne itself, bared at last, spits what's left of its spores at you.

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};

const HP: f64 = 52.0;
const SPORE: u32 = 0xc890e0;
const PAL: &[(char, u32)] = &[
    ('M', 0x9a6ab8), // cap violet
    ('m', 0x6a4888), // cap deep
    ('W', 0xe8d8f8), // spots
    ('U', 0xc8b090), // shelf tan
    ('u', 0x907858), // shelf shade
    ('E', 0x60e880), // eyes
    ('C', 0x7a5898), // carpet mottle
];

const THRONE: [&str; 22] = [
    ".......KKKKKKKKKK.........",
    ".....KKMMmMMMMmMMKK.......",
    "....KMMWMMMmMMWMMMMK......",
    "...KMmMMMMWMMMMMmMMMK.....",
    "...KMMWMMmMMMWMMMMWMK.....",
    "....KKMMMMMMMMMMMKK.......",
    "..KKUuUUuUUUUuUUuUUKK.....",
    ".KUUuUUUUuUUUUUuUUUuUK....",
    ".KUuKEEKUUuUUKEEKUuUUK....",
    ".KUUKEEKUUUUUKEEKUUuUK....",
    ".KUuUUUUuKKKKuUUUUUuUK....",
    "..KKuUUUKKKKKKUUUuKK......",
    "....KUuUUKKKKUUuUK........",
    "..KKUUuUUUuUUUUUuUKK......",
    ".KUuUUUUuUUUuUUUUUuUK.....",
    ".KUUuUUUUUUUUUuUUUUuK.....",
    "..KKUUuUKKKKKKuUUKK.......",
    "....KUUuUUUuUUUUK.........",
    "...KMmMMWMMMMmMMMK........",
    "....KKMMMMmMMMKK..........",
    "......KKKKKKKK............",
    "..........................",
];
const NODE: [&str; 12] = [
    "....KKKK....",
    "..KKMmMKK...",
    ".KMmWMMmMK..",
    ".KmMMMWMmK..",
    "KMmWMmMMmMK.",
    "KmMMMMWMMmK.",
    "KMmMWMMmMMK.",
    ".KmMMmMMmK..",
    ".KMmMMMmMK..",
    "..KKmMmKK...",
    "....KKKK....",
    "............",
];
const CARPET: [&str; 10] = [
    "..C...M.....C...",
    "M..C....C.....M.",
    "...M..C...M.C...",
    ".C....M.C....C..",
    "..M.C....C.M....",
    "C....M.C....C.M.",
    "..C....M..C.....",
    ".M..C....C...M..",
    "...C..M....C....",
    "................",
];

const NODES: [(f32, f32); 5] = [(56.0, 56.0), (232.0, 56.0), (56.0, 148.0), (232.0, 148.0), (144.0, 120.0)];

#[derive(Component)]
pub struct MyceliumThrone {
    x: f32,
    y: f32,
    anim: u32,
    nodes_left: u8,
    spit_cd: i32,
    hatch_cd: i32,
    carpet: std::collections::HashSet<(i32, i32)>,
    carpet_img: Handle<Image>,
}

#[derive(Component)]
pub struct SporeNode {
    idx: usize,
    radius: f32,
    pulse_cd: i32,
    erupt_cd: i32,
}

#[derive(Component)]
pub struct CarpetTile {
    c: i32,
    r: i32,
    fading: bool,
}

#[derive(Component)]
pub struct MyceliumSpawn;

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let throne_img = images.add(crate::gfx::bake(&THRONE, PAL));
    let node_img = images.add(crate::gfx::bake(&NODE, PAL));
    let carpet_img = images.add(crate::gfx::bake(&CARPET, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (tx, ty) = (139.0, 40.0);
    for (i, (nx, ny)) in NODES.iter().enumerate() {
        commands.spawn((
            Sprite::from_image(node_img.clone()),
            at(PLAY_X + nx, PLAY_Y + ny, 12.0, 12.0, actor_z(ny + 10.0)),
            PIXEL_LAYER,
            RoomActor,
            SporeNode { idx: i, radius: 10.0, pulse_cd: 100 + i as i32 * 37, erupt_cd: 200 + i as i32 * 41 },
            Combatant { team: Team::Enemy, hurt_team: None, damage: None, persistent: false, knock: 0.0 },
            Health { hp: 8, max: 8, defense: 0, invuln: 0, flash: 0 },
            HurtProfile { invuln: 8, flash: 6, kb_base: 0.0, kb_frames: 0 },
            Knockback::default(),
            Hitbox { x: nx + 1.0, y: ny + 1.0, w: 10.0, h: 9.0 },
        ));
    }
    commands.spawn((
        Sprite::from_image(throne_img),
        at(PLAY_X + tx, PLAY_Y + ty, 26.0, 22.0, actor_z(ty + 20.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE MYCELIUM THRONE"),
        crate::app::dungeon::DungeonBoss,
        MyceliumThrone { x: tx, y: ty, anim: 0, nodes_left: 5, spit_cd: 80, hatch_cd: 300, carpet: Default::default(), carpet_img },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 0.0, kb_frames: 0 }, // it IS the floor's will
        Knockback::default(),
        Hitbox { x: tx + 3.0, y: ty + 4.0, w: 20.0, h: 16.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut rng: ResMut<GameRng>,
    mut statuses: ResMut<crate::app::status::Statuses>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<&Player>,
    mut thrones: Query<
        (&mut MyceliumThrone, &mut Health, &mut Transform, &mut Visibility),
        (Without<SporeNode>, Without<CarpetTile>, Without<Player>),
    >,
    mut nodes: Query<(&mut SporeNode, &mut Visibility, &Health), (Without<MyceliumThrone>, Without<CarpetTile>)>,
    mut tiles: Query<(Entity, &mut CarpetTile, &mut Sprite), (Without<MyceliumThrone>, Without<SporeNode>)>,
    spawns: Query<(), With<MyceliumSpawn>>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok(p) = players.single() else { return };
    let Ok((mut th, mut h, mut tf, mut vis)) = thrones.single_mut() else { return };
    th.anim += 1;
    let network_alive = th.nodes_left > 0;
    if network_alive {
        h.invuln = h.invuln.max(2); // the network drinks every blow
    }
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let (tcx, tcy) = (th.x + 13.0, th.y + 11.0);

    // --- The nodes: creep the carpet outward; erupt when crowded. ---
    for (mut node, mut nvis, nh) in &mut nodes {
        *nvis = if nh.flash > 0 && (nh.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
        let (nx, ny) = (NODES[node.idx].0 + 6.0, NODES[node.idx].1 + 6.0);
        node.pulse_cd -= 1;
        if node.pulse_cd <= 0 && th.carpet.len() < 56 {
            node.pulse_cd = 130;
            node.radius = (node.radius + 12.0).min(70.0);
            // Claim a ring tile or two at the new reach.
            for _ in 0..2 {
                let a = rng.0.next_f64() as f32 * std::f32::consts::TAU;
                let d = node.radius * (0.5 + rng.0.next_f64() as f32 * 0.5);
                let (cx, cy) = (((nx + a.cos() * d) / 16.0) as i32, ((ny + a.sin() * d) / 16.0) as i32);
                if (1..18).contains(&cx) && (2..12).contains(&cy) && th.carpet.insert((cx, cy)) {
                    let mut s = Sprite::from_image(th.carpet_img.clone());
                    s.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
                    commands.spawn((
                        s,
                        at(PLAY_X + (cx * 16) as f32, PLAY_Y + (cy * 16) as f32 + 3.0, 16.0, 10.0, 1.4),
                        PIXEL_LAYER,
                        RoomActor,
                        CarpetTile { c: cx, r: cy, fading: false },
                    ));
                }
            }
        }
        node.erupt_cd -= 1;
        let pd = ((pcx - nx).powi(2) + (pcy - ny).powi(2)).sqrt();
        if node.erupt_cd <= 0 && pd < 70.0 {
            node.erupt_cd = 210;
            for i in 0..4 {
                let a = i as f32 / 4.0 * std::f32::consts::TAU + 0.4;
                commands.spawn((
                    EBolt { x: nx - 4.0, y: ny - 4.0, vx: a.cos() * 1.7, vy: a.sin() * 1.7, life: 90 },
                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                    HitOnce::default(),
                    Hitbox { x: nx - 1.0, y: ny - 1.0, w: 7.0, h: 7.0 },
                    Sprite::from_image(art.bolt(SPORE, 0xf0e0ff)),
                    at(PLAY_X + nx - 3.0, PLAY_Y + ny - 3.0, 8.0, 8.0, 8.6),
                    PIXEL_LAYER,
                    RoomActor,
                ));
            }
            sfx.write(crate::app::sfx::Sfx("tink"));
        }
    }

    // --- The carpet: fade in, bog boots, hatch trouble; recede once the network dies. ---
    let ptile = (((pcx) / 16.0) as i32, ((pcy) / 16.0) as i32);
    let mut on_carpet = false;
    for (e, mut ct, mut cs) in &mut tiles {
        if !network_alive {
            ct.fading = true;
        }
        let a = cs.color.alpha();
        cs.color = if ct.fading {
            Color::srgba(1.0, 1.0, 1.0, (a - 0.01).max(0.0))
        } else {
            Color::srgba(1.0, 1.0, 1.0, (a + 0.02).min(0.55))
        };
        if ct.fading && cs.color.alpha() <= 0.01 {
            th.carpet.remove(&(ct.c, ct.r));
            commands.entity(e).despawn();
            continue;
        }
        if (ct.c, ct.r) == ptile {
            on_carpet = true;
        }
    }
    if on_carpet {
        statuses.add("slow", 20);
        th.hatch_cd -= 1;
        if th.hatch_cd <= 0 && spawns.iter().count() < 3 {
            th.hatch_cd = 320;
            if let Some(idx) = crate::actors::mobs::def_index("sporeling") {
                commands.spawn((crate::actors::mobs::mob_bundle(idx, pcx - 24.0, pcy - 8.0), RoomActor, PIXEL_LAYER, MyceliumSpawn));
                spawn_burst(&mut commands, &mut rng, Vec2::new(pcx - 16.0, pcy), SPORE, 6);
                sfx.write(crate::app::sfx::Sfx("tink"));
            }
        }
    }

    // --- Bared: the throne spits back. ---
    if !network_alive {
        th.spit_cd -= 1;
        if th.spit_cd <= 0 {
            th.spit_cd = 120;
            let base = (pcy - tcy).atan2(pcx - tcx);
            for i in -1..=1i32 {
                let a = base + i as f32 * 0.3;
                commands.spawn((
                    EBolt { x: tcx - 4.0, y: tcy - 4.0, vx: a.cos() * 2.1, vy: a.sin() * 2.1, life: 120 },
                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                    HitOnce::default(),
                    Hitbox { x: tcx - 1.0, y: tcy - 1.0, w: 7.0, h: 7.0 },
                    Sprite::from_image(art.bolt(SPORE, 0xf0e0ff)),
                    at(PLAY_X + tcx - 3.0, PLAY_Y + tcy - 3.0, 8.0, 8.0, 8.6),
                    PIXEL_LAYER,
                    RoomActor,
                ));
            }
        }
    }

    // --- Sync: the throne breathes with its network. ---
    let s = 1.0 + ((th.anim as f32) * if network_alive { 0.06 } else { 0.16 }).sin() * 0.03;
    tf.scale = Vec3::new(s, s, 1.0);
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

/// Cut nodes wither (the throne flinches); the felled throne rots the whole floor.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    mut thrones: Query<(Entity, &mut MyceliumThrone, &mut Health), (Without<SporeNode>, Without<CarpetTile>)>,
    nodes: Query<(Entity, &SporeNode, &Health), Without<MyceliumThrone>>,
    tiles: Query<Entity, With<CarpetTile>>,
) {
    let Ok((te, mut th, mut thh)) = thrones.single_mut() else { return };
    for (e, node, nh) in &nodes {
        if nh.hp > 0 {
            continue;
        }
        let (nx, ny) = (NODES[node.idx].0 + 6.0, NODES[node.idx].1 + 6.0);
        spawn_burst(&mut commands, &mut rng, Vec2::new(nx, ny), SPORE, 12);
        commands.entity(e).despawn();
        th.nodes_left = th.nodes_left.saturating_sub(1);
        thh.flash = 8;
        sfx.write(crate::app::sfx::Sfx("stone"));
    }
    if thh.hp <= 0 {
        for (e, ..) in &nodes {
            commands.entity(e).despawn();
        }
        for e in &tiles {
            commands.entity(e).despawn();
        }
        let (cx, cy) = (th.x + 13.0, th.y + 11.0);
        for i in 0..3 {
            let off = i as f32 * 7.0 - 7.0;
            spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.5), SPORE, 12);
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
        check("throne", &THRONE, 26);
        check("node", &NODE, 12);
        check("carpet", &CARPET, 16);
    }
}
