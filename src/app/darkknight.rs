//! darkknight.rs — THE GATE GUARDIANS (js darkknight): two towering knights in
//! void-touched plate stand the Black Castle's gate, and they do not move fast
//! because they do not need to. Slow, relentless stalk; a shouldered GREATSWORD
//! that leaves the shoulder only to sweep (the ogre's club-swing pattern, in
//! steel); a dread aura; and a truck's worth of damage if the arc lands. Beat
//! both and the gate stands unguarded forever (js castleGuardsCleared, saved).

use bevy::prelude::*;

use super::battle::{spawn_burst, GameRng, RoomActor};
use super::play::Player;
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, bake, flip_h, PIXEL_LAYER};

const SIZE: f32 = 1.3; // js sizeMul — towering
const SWING_LIFE: i32 = 18;

#[derive(Component)]
pub struct DarkKnight {
    pub x: f32,
    pub y: f32,
    pub left: bool,
    pub swing_t: i32,
    pub cd: i32,
    pub anim: u32,
}

/// Both fell — the gate stands unguarded forever (saved via SaveExtras).
#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct CastleGuards(pub bool);

#[derive(Component)]
pub struct GreatswordSwing {
    pub owner: Entity,
    pub fx: f32,
    pub fy: f32,
    pub life: i32,
}

const KNIGHT: [&str; 16] = [
    "......KnnK......",
    "......KAAK......",
    ".....KnAAnK.....",
    "....KnxAAxnK....",
    "....KnAAAAnK....",
    "...KAaKnnKaAK...",
    "..KAaanKKnaaAK..",
    "..KnaKKKKKKanK..",
    "....KnXXXXnK....",
    "....KnXxxXnK....",
    "....KAnnnnAK....",
    "....KnnnnnnK....",
    ".....KnAAnK.....",
    ".....KnKKnK.....",
    "....KKn..nKK....",
    "................",
];

const KNIGHT_PAL: &[(char, u32)] = &[
    ('n', 0x54505c), // dark plate
    ('A', 0x8a8a96), // lit plate
    ('a', 0x6a6a76), // plate shade
    ('x', 0xb070ff), // the void glow in the visor slits
    ('X', 0x2a2732), // tabard black
];

const GREATSWORD: [&str; 18] = [
    ".WA.",
    ".WA.",
    ".WA.",
    ".WA.",
    ".WA.",
    ".WA.",
    ".WA.",
    ".WA.",
    ".WA.",
    ".WA.",
    ".WA.",
    ".WA.",
    "WAAW",
    "aaaa",
    ".nn.",
    ".nn.",
    ".xn.",
    "....",
];

const SWORD_PAL: &[(char, u32)] = &[
    ('W', 0xeef0f6),
    ('A', 0xbcbcc4),
    ('a', 0xd8d8e0),
    ('n', 0x54505c),
    ('x', 0xb070ff),
];

#[derive(Resource)]
pub struct KnightArt {
    pub body: [Handle<Image>; 2], // right / left
    pub sword: Handle<Image>,
}

impl KnightArt {
    pub fn build(images: &mut Assets<Image>) -> Self {
        let left: Vec<String> = flip_h(&KNIGHT);
        let refs: Vec<&str> = left.iter().map(|s| s.as_str()).collect();
        KnightArt {
            body: [images.add(bake(&KNIGHT, KNIGHT_PAL)), images.add(bake(&refs, KNIGHT_PAL))],
            sword: images.add(bake(&GREATSWORD, SWORD_PAL)),
        }
    }
}

/// Stand a guardian up (battle's 'guard' rows call this).
pub fn spawn_knight(commands: &mut Commands, images: &mut Assets<Image>, x: f32, y: f32) {
    // The dread aura rides the champions' ring machinery, tinted void.
    let e = commands
        .spawn((
            Sprite {
                custom_size: Some(Vec2::splat(16.0 * SIZE)),
                ..Sprite::default()
            },
            at(PLAY_X + x, PLAY_Y + y - (16.0 * (SIZE - 1.0)), 16.0 * SIZE, 16.0 * SIZE, actor_z(y + 16.0)),
            PIXEL_LAYER,
            RoomActor,
            DarkKnight { x, y, left: false, swing_t: 0, cd: 30, anim: 0 },
            Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(7), persistent: true, knock: 0.0 },
            Health { hp: (44.0 * crate::actors::mobs::HP_MUL).round() as i32, max: (44.0 * crate::actors::mobs::HP_MUL).round() as i32, defense: 2, invuln: 0, flash: 0 },
            HurtProfile { invuln: 10, flash: 8, kb_base: 2.2 * (1.0 - 0.9), kb_frames: 11 },
            Knockback::default(),
            Hitbox { x: x + 1.0, y: y + 1.0, w: 14.0, h: 15.0 },
        ))
        .id();
    let img = super::champions::aura_image(images, 0xb070ff);
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + x, PLAY_Y + y + 12.0, 20.0, 10.0, 3.9),
        PIXEL_LAYER,
        RoomActor,
        super::champions::AuraRing { owner: e, t: 0.0, scale: 1.5 * SIZE },
    ));
}

/// Stand the guardians up when the castle-gate room arrives (the yard_wake idiom;
/// battle's spawn skips 'guard' rows — persistence lives here).
#[allow(clippy::too_many_arguments)]
pub fn guard_wake(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cur: Res<super::play::CurRoom>,
    sliding: Res<super::play::SlideActive>,
    world: Res<super::play::GameWorld>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    inside: Res<super::interior::Inside>,
    guards: Res<CastleGuards>,
    mut woke: Local<Option<(i32, i32)>>,
    live: Query<(), With<DarkKnight>>,
) {
    if sliding.0 || in_dungeon.0.is_some() || inside.0.is_some() {
        *woke = None;
        return;
    }
    if *woke == Some((cur.rx, cur.ry)) {
        return;
    }
    *woke = Some((cur.rx, cur.ry));
    if guards.0 || live.iter().next().is_some() {
        return; // beaten forever / already standing
    }
    for e in world.0.room_entities(cur.rx, cur.ry) {
        if e.kind == "guard" {
            spawn_knight(&mut commands, &mut images, e.x as f32, e.y as f32);
        }
    }
}

/// One standing knight (the tick re-poses body + blade + box each frame).
type KnightRow<'w> = (Entity, &'w mut DarkKnight, &'w mut Transform, &'w mut Hitbox, &'w mut Sprite, &'w Health);

/// The stalk + the sweep (js darkknight ai): slow, relentless; rooted mid-swing.
#[allow(clippy::too_many_arguments)]
pub fn knight_tick(
    mut commands: Commands,
    art: Res<KnightArt>,
    grid: Res<super::play::CurGrid>,
    blockers: Res<super::room_props::RoomBlockers>,
    players: Query<&Player>,
    mut knights: Query<KnightRow, Without<Player>>,
) {
    let Ok(p) = players.single() else { return };
    for (ent, mut k, mut tf, mut hb, mut spr, h) in &mut knights {
        if h.hp <= 0 {
            continue;
        }
        if k.cd > 0 {
            k.cd -= 1;
        }
        k.anim += 1;
        let (dx, dy) = (p.x - k.x, p.y - k.y);
        let d = dx.hypot(dy).max(0.001);
        k.left = dx < 0.0;
        if k.swing_t > 0 {
            k.swing_t -= 1; // rooted, mid-swing
        } else if k.cd <= 0 && d < 38.0 {
            // Heavy SWORD SWING in melee.
            let (fx, fy) = if dx.abs() > dy.abs() {
                (dx.signum(), 0.0)
            } else {
                (0.0, dy.signum())
            };
            commands.spawn((
                Sprite::from_image(art.sword.clone()),
                at(PLAY_X + k.x, PLAY_Y + k.y, 4.0, 18.0, 8.7),
                PIXEL_LAYER,
                RoomActor,
                GreatswordSwing { owner: ent, fx, fy, life: SWING_LIFE },
                Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: None, persistent: false, knock: 2.0 },
                HitOnce::default(),
                Hitbox { x: -999.0, y: -999.0, w: 18.0, h: 18.0 },
            ));
            k.swing_t = SWING_LIFE;
            k.cd = 60;
        } else {
            // The slow, relentless stalk.
            let s = 0.32;
            let step = |k: &mut DarkKnight, sx: f32, sy: f32| {
                let (nx, ny) = (k.x + sx, k.y + sy);
                let feet = (nx + 1.0, ny + 1.0, 14.0, 15.0);
                if !grid.0.box_hits_solid(feet.0, feet.1, feet.2, feet.3)
                    && !blockers.blocks((k.x + 1.0, k.y + 1.0, 14.0, 15.0), feet)
                {
                    k.x = nx;
                    k.y = ny;
                }
            };
            step(&mut k, dx / d * s, 0.0);
            step(&mut k, 0.0, dy / d * s);
        }
        spr.image = art.body[k.left as usize].clone();
        // The shouldered blade is baked into the pose while idle; mid-swing the
        // GreatswordSwing entity draws the arc instead (drawn always here — the
        // held-blade overlay is folded into the flash of the swing).
        let grow = 16.0 * (SIZE - 1.0);
        *tf = at(PLAY_X + k.x - grow / 2.0, PLAY_Y + k.y - grow, 16.0 * SIZE, 16.0 * SIZE, actor_z(k.y + 16.0));
        *hb = Hitbox { x: k.x + 1.0, y: k.y + 1.0, w: 14.0, h: 15.0 };
    }
}

/// The arc (js greatswordSwing): wound back, swept through; the hitbox goes live
/// for the back half, reaching well in front.
pub fn swing_tick(
    mut commands: Commands,
    knights: Query<&DarkKnight>,
    mut swings: Query<(Entity, &mut GreatswordSwing, &mut Transform, &mut Hitbox, &mut Combatant)>,
) {
    for (e, mut s, mut tf, mut hb, mut cb) in &mut swings {
        s.life -= 1;
        if s.life <= 0 {
            commands.entity(e).despawn();
            continue;
        }
        let Ok(k) = knights.get(s.owner) else {
            commands.entity(e).despawn();
            continue;
        };
        let p = 1.0 - s.life as f32 / SWING_LIFE as f32;
        if (s.life as f32) < SWING_LIFE as f32 * 0.6 {
            let (cx, cy) = (k.x + 8.0 + s.fx * 14.0, k.y + 10.0 + s.fy * 14.0);
            cb.damage = Some(7);
            *hb = Hitbox { x: cx - 9.0, y: cy - 9.0, w: 18.0, h: 18.0 };
        } else {
            cb.damage = None;
            *hb = Hitbox { x: -999.0, y: -999.0, w: 18.0, h: 18.0 };
        }
        let base = s.fy.atan2(s.fx) + std::f32::consts::FRAC_PI_2;
        let ang = base + (-1.3 + 2.4 * p);
        let (cx, cy) = (k.x + 8.0, k.y + 10.0);
        *tf = at(PLAY_X + cx - 2.0, PLAY_Y + cy - 16.0, 4.0, 18.0, 8.7);
        tf.rotation = Quat::from_rotation_z(-ang);
    }
}

/// Both guardians down -> the gate stands unguarded forever (js).
#[allow(clippy::too_many_arguments)]
pub fn knight_deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut guards: ResMut<CastleGuards>,
    mut log: ResMut<super::rewards::LootLog>,
    mut saves: MessageWriter<super::save::SaveRequest>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    q: Query<(Entity, &DarkKnight, &Health)>,
) {
    let mut fell = false;
    for (e, k, h) in &q {
        if h.hp > 0 {
            continue;
        }
        fell = true;
        spawn_burst(&mut commands, &mut rng, Vec2::new(k.x + 8.0, k.y + 8.0), 0xb070ff, 14);
        let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
        super::gather::spawn_coin(&mut commands, &mut images, coins, k.x + 4.0, k.y + 8.0);
        commands.entity(e).despawn();
    }
    if fell && q.iter().filter(|(_, _, h)| h.hp > 0).count() == 0 && !guards.0 {
        guards.0 = true;
        log.add("gate", "THE GATE STANDS UNGUARDED", 1, 0xb070ff, false, true);
        sfx.write(super::sfx::Sfx("levelup"));
        saves.write(super::save::SaveRequest);
    }
}

pub struct DarkKnightPlugin;

impl Plugin for DarkKnightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CastleGuards>()
            .add_systems(Startup, |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
                commands.insert_resource(KnightArt::build(&mut images));
            })
            .add_systems(
                bevy::app::FixedUpdate,
                (guard_wake, knight_tick.after(guard_wake), swing_tick.after(knight_tick), knight_deaths.after(crate::combat::resolve_combat))
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
        for row in KNIGHT {
            assert_eq!(row.len(), 16);
        }
        for row in GREATSWORD {
            assert_eq!(row.len(), 4);
        }
    }
}
