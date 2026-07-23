//! ogre.rs — THE OGRE (js enemies.js): the mini-cave roster's brute, finally home.
//! A hulking body over a small hitbox, the knotted CLUB a LIVE overlay at his fist
//! (shouldered while he prowls, wound higher on the windup, hidden mid-swing while
//! the clubSwing entity sweeps it through the front). Five states, js-verbatim:
//! prowl (0) -> charge windup, trembling (1) -> the pounding charge (2) -> the
//! point-blank slam windup (3) -> rooted mid-swing (4). The swing's hitbox goes
//! live only for the downstroke; impact dust kicks where it lands.
//! DEVIATION (flagged): the js ground-crack stroke under the impact is a dust
//! burst here; the Dark Knight shares this swing pattern when he ports.

use bevy::prelude::*;

use super::battle::{spawn_burst, GameRng, RoomActor};
use super::play::Player;
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, bake, flip_h, PIXEL_LAYER};

const SLAM_WIND: i32 = 12; // js OGRE_SLAM_WIND
const SLAM_SWING: i32 = 22; // js OGRE_SLAM_SWING (= the clubSwing lifetime)

#[derive(Component)]
pub struct Ogre {
    pub x: f32,
    pub y: f32,
    pub facing: usize, // 0 down / 1 up / 2 right / 3 left
    pub st: i32,
    pub t: i32,
    pub cd: i32,
    pub cd2: i32,
    pub cvx: f32,
    pub cvy: f32,
    pub anim: i32,
}

/// The held club overlay (hidden mid-swing).
#[derive(Component)]
pub struct OgreClub(pub Entity);

/// The sweeping club (js clubSwing): the hitbox lands on the downstroke.
#[derive(Component)]
pub struct ClubSwing {
    pub owner: Entity,
    pub fx: f32,
    pub fy: f32,
    pub life: i32,
}

const CLUB: &[&str] = &[
    "..dDDd..",
    ".dDDDDd.",
    ".dDnDDd.",
    ".dDDDDd.",
    ".dDDDd..",
    "..dDDd..",
    "..dDd...",
    "..dDd...",
    "..dDd...",
    "...dd...",
    "........",
    "........",
];

const OGRE_A: &[&str] = &[
    "........................",
    "........................",
    "......KKKKK.............",
    ".....KqqqqqK............",
    "....KQQqqqQQK...........",
    "....KqoqqqoqK...........",
    "....KqoqQqoqK...........",
    "....KWqqqqqWK...........",
    "....KqQQQQQqK...........",
    "...KKqqqqqqqKK...KdDdK..",
    "..KqqqqqqqqqqqqqqqqDqK..",
    ".KqqqqqqqqqqqqqKKqqqqK..",
    ".KqQqqqqqqqqqqqqK.KKK...",
    ".KqQqqqqqqqqqqqqK.......",
    ".KqqqQQQQQQQqqqqK.......",
    ".KqqqQQQQQQQqqqqqK......",
    "..KqqqQQQQQqqqqqK.......",
    "..KYYYYYYYYYYYYK........",
    "..KYdYYdYYdYYdYK........",
    "...KqqqqK.KqqqqK........",
    "...KqQqqK.KqqQqK........",
    "...KqqqqK.KqqqqK........",
    "...KqqqqK.KnnnnK........",
    "...KnnnnK...............",
];

const OGRE_B: &[&str] = &[
    "........................",
    "........................",
    "......KKKKK.............",
    ".....KqqqqqK............",
    "....KQQqqqQQK...........",
    "....KqoqqqoqK...........",
    "....KqoqQqoqK...........",
    "....KWqqqqqWK...........",
    "....KqQQQQQqK...........",
    "...KKqqqqqqqKK...KdDdK..",
    "..KqqqqqqqqqqqqqqqqDqK..",
    ".KqqqqqqqqqqqqqKKqqqqK..",
    ".KqQqqqqqqqqqqqqK.KKK...",
    ".KqQqqqqqqqqqqqqK.......",
    ".KqqqQQQQQQQqqqqK.......",
    ".KqqqQQQQQQQqqqqqK......",
    "..KqqqQQQQQqqqqqK.......",
    "..KYYYYYYYYYYYYK........",
    "..KYdYYdYYdYYdYK........",
    "...KqqqqK.KqqqqK........",
    "...KqQqqK.KqqQqK........",
    "...KqqqqK.KqqqqK........",
    "...KnnnnK.KqqqqK........",
    "..........KnnnnK........",
];

const OGRE_UA: &[&str] = &[
    "........................",
    "........................",
    "......KKKKK.............",
    ".....KqqqqqK............",
    "....KQQqqqQQK...........",
    "....KqQqqqQqK...........",
    "....KqqqqqqqK...........",
    "....KqqqqqqqK...........",
    "....KqqQQQqqK...........",
    "...KKqqqqqqqKK...KdDdK..",
    "..KqqqqqqqqqqqqqqqqDqK..",
    ".KqqqqqqqqqqqqqKKqqqqK..",
    ".KqQqqqqqqqqqqqqK.KKK...",
    ".KqQqqqqqqqqqqqqK.......",
    ".KqqqQQQQQQQqqqqK.......",
    ".KqqqQQQQQQQqqqqqK......",
    "..KqqqQQQQQqqqqqK.......",
    "..KYYYYYYYYYYYYK........",
    "..KYYdYYYYdYYYYK........",
    "...KqqqqK.KqqqqK........",
    "...KqQqqK.KqqQqK........",
    "...KqqqqK.KqqqqK........",
    "...KqqqqK.KnnnnK........",
    "...KnnnnK...............",
];

const OGRE_SA: &[&str] = &[
    "................KKKKK.......",
    "...............KqqqqqK......",
    "..............KQQqqqqQK.....",
    "..............KqqqoqqqK.....",
    "..............KqqqoqQWK.....",
    "..............KqqqqqqWK.....",
    "..............KqQQQQQqK.....",
    ".............KKqqqqqqKK.....",
    ".........KKqqqqqqqqqqqK.....",
    ".......KqDqKqqqqqqqqqqqK....",
    ".......KqqqKqQqqqqqqqqqK....",
    "........KKqqqqqqqqqqqqqK....",
    "..........KqqqqqQQqqqqqK....",
    "..........KqqqQQQQQqqqK.....",
    "..........KqqQQQQQQQqqK.....",
    "..........KqQQQQQQQQqqK.....",
    "..........KYYYYYYYYYYYK.....",
    "..........KYdYYdYYdYYdK.....",
    "...........KqqqqK.KqqqqK....",
    "...........KqQqqK.KqqQqK....",
    "...........KqqqqK.KqqqqK....",
    "...........KqqqqK.KnnnnK....",
    "...........KnnnnK...........",
    "............................",
];

const OGRE_SB: &[&str] = &[
    "................KKKKK.......",
    "...............KqqqqqK......",
    "..............KQQqqqqQK.....",
    "..............KqqqoqqqK.....",
    "..............KqqqoqQWK.....",
    "..............KqqqqqqWK.....",
    "..............KqQQQQQqK.....",
    ".............KKqqqqqqKK.....",
    ".........KKqqqqqqqqqqqK.....",
    ".......KqDqKqqqqqqqqqqqK....",
    ".......KqqqKqQqqqqqqqqqK....",
    "........KKqqqqqqqqqqqqqK....",
    "..........KqqqqqQQqqqqqK....",
    "..........KqqqQQQQQqqqK.....",
    "..........KqqQQQQQQQqqK.....",
    "..........KqQQQQQQQQqqK.....",
    "..........KYYYYYYYYYYYK.....",
    "..........KYdYYdYYdYYdK.....",
    "...........KqqqqK.KqqqqK....",
    "...........KqQqqK.KqqQqK....",
    "...........KqqqqK.KqqqqK....",
    "...........KnnnnK.KqqqqK....",
    "..................KnnnnK....",
    "............................",
];

/// The baked facings: [down A/B, up A/B (flipped), right A/B, left A/B].
#[derive(Resource)]
pub struct OgreArt {
    pub body: [[Handle<Image>; 2]; 4],
    pub club: Handle<Image>,
}

impl OgreArt {
    pub fn build(images: &mut Assets<Image>) -> Self {
        let up_a: Vec<String> = flip_h(OGRE_UA);
        let up_b_grid: Vec<&str> = OGRE_UA[..22]
            .iter()
            .copied()
            .chain(["...KnnnnK.KqqqqK........", "..........KnnnnK........"])
            .collect();
        let up_b: Vec<String> = flip_h(&up_b_grid);
        let left_a: Vec<String> = flip_h(OGRE_SA);
        let left_b: Vec<String> = flip_h(OGRE_SB);
        let bake_owned = |images: &mut Assets<Image>, g: &[String]| {
            let refs: Vec<&str> = g.iter().map(|s| s.as_str()).collect();
            images.add(bake(&refs, &[]))
        };
        OgreArt {
            body: [
                [images.add(bake(OGRE_A, &[])), images.add(bake(OGRE_B, &[]))],
                [bake_owned(images, &up_a), bake_owned(images, &up_b)],
                [images.add(bake(OGRE_SA, &[])), images.add(bake(OGRE_SB, &[]))],
                [bake_owned(images, &left_a), bake_owned(images, &left_b)],
            ],
            club: images.add(bake(CLUB, &[])),
        }
    }
}

/// Stand one up (mini-cave boss + encounters) — art lands on the first tick
/// (ogre_tick re-images every frame; club_ensure hangs the club).
pub fn spawn_ogre(commands: &mut Commands, x: f32, y: f32) -> Entity {
    commands
        .spawn((
            Sprite::default(),
            at(PLAY_X + x, PLAY_Y + y - 8.0, 24.0, 24.0, actor_z(y + 15.0)),
            PIXEL_LAYER,
            RoomActor,
            Ogre { x, y, facing: 0, st: 0, t: 0, cd: 0, cd2: 0, cvx: 0.0, cvy: 0.0, anim: 0 },
            Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(4), persistent: true, knock: 0.0 },
            Health { hp: 24, max: 24, defense: 1, invuln: 0, flash: 0 },
            HurtProfile { invuln: 10, flash: 8, kb_base: 2.2, kb_resist: 0.85, kb_frames: 11 },
            Knockback::default(),
            Hitbox { x: x + 1.0, y: y + 2.0, w: 14.0, h: 13.0 },
        ))
        .id()
}

/// Every ogre gets his club overlay (spawn sites don't carry the art bank).
fn club_ensure(
    mut commands: Commands,
    art: Res<OgreArt>,
    ogres: Query<(Entity, &Ogre)>,
    clubs: Query<&OgreClub>,
) {
    let held: Vec<Entity> = clubs.iter().map(|c| c.0).collect();
    for (e, o) in &ogres {
        if !held.contains(&e) {
            commands.spawn((
                Sprite::from_image(art.club.clone()),
                at(PLAY_X + o.x, PLAY_Y + o.y, 8.0, 12.0, actor_z(o.y + 15.0) + 0.006),
                PIXEL_LAYER,
                RoomActor,
                OgreClub(e),
            ));
        }
    }
}

const FACE_VEC: [(f32, f32); 4] = [(0.0, 1.0), (0.0, -1.0), (1.0, 0.0), (-1.0, 0.0)];

fn face_from(dx: f32, dy: f32) -> usize {
    if dx.abs() > dy.abs() {
        if dx > 0.0 { 2 } else { 3 }
    } else if dy < 0.0 {
        1
    } else {
        0
    }
}

/// One live ogre row (the tick rebuilds art, body, and hitbox each frame).
type OgreRow<'w> = (Entity, &'w mut Ogre, &'w mut Transform, &'w mut Hitbox, &'w mut Sprite, &'w Health);

/// The five states, js-verbatim; movement slides on the room grid + blockers.
#[allow(clippy::too_many_arguments)]
fn ogre_tick(
    mut commands: Commands,
    grid: Res<super::play::CurGrid>,
    blockers: Res<super::room_props::RoomBlockers>,
    players: Query<&Player>,
    mut ogres: Query<OgreRow, Without<Player>>,
    art: Res<OgreArt>,
) {
    let Ok(p) = players.single() else { return };
    for (ent, mut o, mut tf, mut hb, mut spr, h) in &mut ogres {
        if h.hp <= 0 {
            continue; // deaths sweeps
        }
        if o.cd > 0 {
            o.cd -= 1;
        }
        if o.cd2 > 0 {
            o.cd2 -= 1;
        }
        let (dx, dy) = (p.x - o.x, p.y - o.y);
        let d = dx.hypot(dy).max(0.001);
        let step = |o: &mut Ogre, sx: f32, sy: f32, grid: &crate::room::RoomGrid, blk: &super::room_props::RoomBlockers| -> bool {
            let (nx, ny) = (o.x + sx, o.y + sy);
            let feet = (nx + 1.0, ny + 2.0, 14.0, 13.0);
            if grid.box_hits_solid(feet.0, feet.1, feet.2, feet.3)
                || blk.blocks((o.x + 1.0, o.y + 2.0, 14.0, 13.0), feet)
            {
                return false;
            }
            o.x = nx;
            o.y = ny;
            true
        };
        match o.st {
            4 => {
                // Mid-swing (rooted) — the clubSwing entity is doing the talking.
                o.t -= 1;
                if o.t <= 0 {
                    o.st = 0;
                }
            }
            3 => {
                // Slam wind-up: trembles with intent, then the sweep goes live.
                o.t -= 1;
                if o.t <= 0 {
                    o.st = 4;
                    o.t = SLAM_SWING;
                    let f = FACE_VEC[o.facing];
                    commands.spawn((
                        Sprite::from_image(art.club.clone()),
                        at(PLAY_X + o.x, PLAY_Y + o.y, 8.0, 12.0, 8.7),
                        PIXEL_LAYER,
                        RoomActor,
                        ClubSwing { owner: ent, fx: f.0, fy: f.1, life: SLAM_SWING },
                        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: None, persistent: false, knock: 1.5 },
                        HitOnce::default(),
                        Hitbox { x: -999.0, y: -999.0, w: 18.0, h: 16.0 },
                    ));
                }
            }
            2 => {
                // The pounding charge: straight down the committed line until it hits.
                let (cvx, cvy) = (o.cvx, o.cvy);
                let moved = step(&mut o, cvx, cvy, &grid.0, &blockers);
                o.t -= 1;
                if !moved || o.t <= 0 {
                    o.st = 0;
                }
                o.anim += 2; // pounding strides
            }
            1 => {
                // Charge wind-up, trembling; then commit at 3.4.
                o.facing = face_from(dx, dy);
                o.t -= 1;
                if o.t <= 0 {
                    o.cvx = dx / d * 3.4;
                    o.cvy = dy / d * 3.4;
                    o.st = 2;
                    o.t = 26;
                }
            }
            _ => {
                if d < 36.0 && o.cd2 <= 0 {
                    // Point blank: SMASH.
                    o.st = 3;
                    o.t = SLAM_WIND;
                    o.cd2 = 110;
                    o.facing = face_from(dx, dy);
                } else if d < 130.0 && o.cd <= 0 {
                    o.st = 1;
                    o.t = 22;
                    o.cd = 150;
                    o.facing = face_from(dx, dy);
                } else if d < 200.0 {
                    let s = 0.4;
                    step(&mut o, dx.signum() * s, 0.0, &grid.0, &blockers);
                    step(&mut o, 0.0, dy.signum() * s, &grid.0, &blockers);
                    o.facing = face_from(dx, dy);
                    o.anim += 1;
                }
            }
        }
        // Body frame: charge pounds fast, everything else strides slow; wind-ups tremble.
        let fi = if o.st == 2 { (o.t / 3) as usize % 2 } else { (o.anim / 12) as usize % 2 };
        let jx = if o.st == 1 || o.st == 3 {
            if (o.t & 2) != 0 { 1.0 } else { -1.0 }
        } else {
            0.0
        };
        spr.image = art.body[o.facing][fi].clone();
        // Anchors per facing (js ax offsets; profiles are 28 wide).
        let (ax, w) = match o.facing {
            2 => (-9.0, 28.0),
            3 => (-2.0, 28.0),
            1 => (-4.0, 24.0),
            _ => (0.0, 24.0),
        };
        *tf = at(PLAY_X + o.x + ax + jx, PLAY_Y + o.y - 8.0, w, 24.0, actor_z(o.y + 15.0));
        *hb = Hitbox { x: o.x + 1.0, y: o.y + 2.0, w: 14.0, h: 13.0 };
    }
}

/// The held club rides the fist — shouldered at rest, higher through the windup,
/// hidden mid-swing (js drawClub).
fn club_tick(
    mut commands: Commands,
    ogres: Query<(&Ogre, &Health)>,
    mut clubs: Query<(Entity, &OgreClub, &mut Transform, &mut Visibility)>,
) {
    for (e, club, mut tf, mut vis) in &mut clubs {
        let Ok((o, h)) = ogres.get(club.0) else {
            commands.entity(e).despawn();
            continue;
        };
        if h.hp <= 0 {
            commands.entity(e).despawn();
            continue;
        }
        if o.st == 4 {
            *vis = Visibility::Hidden; // the swing entity draws the club
            continue;
        }
        *vis = Visibility::Inherited;
        let (cxo, dir) = match o.facing {
            2 => (0.0, -1.0),
            3 => (16.0, 1.0),
            1 => (0.0, 1.0),
            _ => (19.0, 1.0),
        };
        let ang = if o.st == 3 { dir * (0.3 + 0.5 * (1.0 - o.t as f32 / SLAM_WIND as f32)) } else { dir * 0.3 };
        let jx = if o.st == 1 || o.st == 3 {
            if (o.t & 2) != 0 { 1.0 } else { -1.0 }
        } else {
            0.0
        };
        // Pivot at the fist (grip 4px in, 11 down from the club top).
        let z = actor_z(o.y + 15.0) + if o.facing == 1 { -0.006 } else { 0.006 };
        *tf = at(PLAY_X + o.x + cxo + jx - 4.0, PLAY_Y + o.y + 2.0 - 11.0, 8.0, 12.0, z);
        tf.rotation = Quat::from_rotation_z(-ang);
    }
}

/// The sweep (js clubSwing): winds back then arcs through the front; the hitbox
/// lands for the downstroke, with impact dust where it bites.
fn swing_tick(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    ogres: Query<&Ogre>,
    mut swings: Query<(Entity, &mut ClubSwing, &mut Transform, &mut Hitbox, &mut Combatant)>,
) {
    for (e, mut s, mut tf, mut hb, mut cb) in &mut swings {
        s.life -= 1;
        if s.life <= 0 {
            commands.entity(e).despawn();
            continue;
        }
        let Ok(o) = ogres.get(s.owner) else {
            commands.entity(e).despawn();
            continue;
        };
        let p = 1.0 - s.life as f32 / SLAM_SWING as f32;
        let (ix, iy) = (o.x + 8.0 + s.fx * 15.0, o.y + 10.0 + s.fy * 15.0);
        if (s.life as f32) < SLAM_SWING as f32 * 0.55 {
            cb.damage = Some(3);
            *hb = Hitbox { x: ix - 9.0, y: iy - 8.0, w: 18.0, h: 16.0 };
        } else {
            cb.damage = None;
            *hb = Hitbox { x: -999.0, y: -999.0, w: 18.0, h: 16.0 };
        }
        if (0.55..0.62).contains(&p) {
            spawn_burst(&mut commands, &mut rng, Vec2::new(ix, iy + 2.0), 0xc9bfae, 5); // impact dust
        }
        let base = s.fy.atan2(s.fx) + std::f32::consts::FRAC_PI_2;
        let ang = base + (-1.4 + 2.5 * p);
        let (cx, cy) = (o.x + 8.0, o.y + 10.0);
        *tf = at(PLAY_X + cx - 4.0, PLAY_Y + cy - 16.0, 8.0, 12.0, 8.7);
        tf.rotation = Quat::from_rotation_z(-ang);
    }
}

/// A fallen ogre coughs up gear (js drops: rollLoot boost threat+1).
fn ogre_deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    q: Query<(Entity, &Ogre, &Health)>,
) {
    for (e, o, h) in &q {
        if h.hp > 0 {
            continue;
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(o.x + 8.0, o.y + 8.0), 0xa85820, 12);
        let (id, qty) = crate::items::roll_loot(1.0, 0.0, || rng.0.next_f64());
        super::gather::spawn_pickup(&mut commands, &mut images, id, qty, o.x + 4.0, o.y + 4.0, true, None);
        commands.entity(e).despawn();
    }
}

pub struct OgrePlugin;

impl Plugin for OgrePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
            commands.insert_resource(OgreArt::build(&mut images));
        })
        .add_systems(
            bevy::app::FixedUpdate,
            (club_ensure, ogre_tick, club_tick.after(ogre_tick), swing_tick.after(ogre_tick), ogre_deaths.after(crate::combat::resolve_combat))
                .before(super::play::EndTick)
                .run_if(super::screen::playing),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids_are_rectangular() {
        for g in [OGRE_A, OGRE_B, OGRE_UA] {
            for row in g {
                assert_eq!(row.len(), 24);
            }
        }
        for g in [OGRE_SA, OGRE_SB] {
            for row in g {
                assert_eq!(row.len(), 28);
            }
        }
        for row in CLUB {
            assert_eq!(row.len(), 8);
        }
    }
}
