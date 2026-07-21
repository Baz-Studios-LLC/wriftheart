//! THE ALL-EYE — boss 5 of THE TEN (BOSSES.md): the Bog's guardian. The beholder.
//!
//! A great lidded orb drifting over the mire, blind-shut and untouchable while its
//! five EYESTALKS live. Each stalk is its own menace on its own clock: the PULLER's
//! ray reels you into the orb (the tongue rig), the BOLTER fans bog-bolts, the
//! SUMMONER wakes leeches from the muck, the SLOWER's beam bogs your boots, and the
//! GAZER locks on with a thin warning line — when it fires, MOVEMENT is what it
//! punishes; stand dead still and the stare passes through you. Pluck all five and
//! THE GREAT EYE OPENS: soft at last, but now the whole orb gazes, novas, and
//! BLINKS away when wounded deep.

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 52.0; // the js bog-line pool (x HP_MUL)
const STALK_HP: i32 = 8;
const BOG: u32 = 0x6a9a4a;
const IRIS: u32 = 0xd8b030;
const PAL: &[(char, u32)] = &[
    ('G', 0x4a6a3a), // hide green
    ('g', BOG),      // hide light
    ('V', 0x8ab86a), // rim
    ('I', IRIS),     // iris gold
    ('P', 0x2a1a30), // pupil void
    ('R', 0xc05070), // veins
    ('W', 0xf0ead8), // sclera
];

const ORB_SHUT: [&str; 20] = [
    "......KKKKKKKKKK......",
    "....KKGgGgGgGgGgKK....",
    "...KGgGgGgGgGgGgGK....",
    "..KGgVVVVVVVVVVgGgK...",
    ".KGgVVVVVVVVVVVVgGK...",
    ".KgVVVVVVVVVVVVVVgK...",
    ".KGVVVKKKKKKKKVVVGK...",
    ".KgVKKKKKKKKKKKKVgK...",
    ".KGVKKKKKKKKKKKKVGK...",
    ".KgVVVKKKKKKKKVVVgK...",
    ".KGgVVVVVVVVVVVVgGK...",
    ".KGgGVVVVVVVVVVGgGK...",
    "..KGgGgVVVVVVgGgGK....",
    "...KGgGgGgGgGgGgK.....",
    "....KKGgGgGgGgKK......",
    "......KKKKKKKK........",
    "....G...G..G...G......",
    "...GgG.GgGGgG.GgG.....",
    "....G...G..G...G......",
    "......................",
];
const ORB_OPEN: [&str; 20] = [
    "......KKKKKKKKKK......",
    "....KKGgGgGgGgGgKK....",
    "...KGgWWWWWWWWWWGK....",
    "..KGgWWWWWWWWWWWWgK...",
    ".KGgWWWRWWWWRWWWWGK...",
    ".KgWWRWWWWWWWWRWWgK...",
    ".KGWWWWIIIIIIWWWWGK...",
    ".KgWWWIIIIIIIIWWWgK...",
    ".KGWWWIIPPPPIIWWWGK...",
    ".KgWWWIIPPPPIIWWWgK...",
    ".KGWWWIIIIIIIIWWWGK...",
    ".KGgWWWIIIIIIWWWgGK...",
    "..KGgWRWWWWWWRWGgK....",
    "...KGgWWWWWWWWGgK.....",
    "....KKGgGgGgGgKK......",
    "......KKKKKKKK........",
    "....G...G..G...G......",
    "...GgG.GgGGgG.GgG.....",
    "....G...G..G...G......",
    "......................",
];
const STALK: [&str; 14] = [
    "...KKKKKK...",
    "..KWWWWWWK..",
    ".KWWIIIIWWK.",
    ".KWIIPPIIWK.",
    ".KWIIPPIIWK.",
    ".KWWIIIIWWK.",
    "..KWWWWWWK..",
    "...KKgGK....",
    "....KgGK....",
    "...KGgK.....",
    "...KgGK.....",
    "....KgGgK...",
    "....KGgK....",
    ".....KK.....",
];

/// Stalk roles by ring slot.
#[derive(Clone, Copy, PartialEq)]
enum Role {
    Puller,
    Bolter,
    Summoner,
    Slower,
    Gazer,
}
const ROLES: [Role; 5] = [Role::Puller, Role::Bolter, Role::Summoner, Role::Slower, Role::Gazer];
fn role_tint(r: Role) -> Color {
    match r {
        Role::Puller => Color::srgb_u8(0xe0, 0x7a, 0xc0),
        Role::Bolter => Color::srgb_u8(0x9a, 0xd0, 0x6a),
        Role::Summoner => Color::srgb_u8(0x80, 0x60, 0x40),
        Role::Slower => Color::srgb_u8(0x6a, 0xc0, 0xd8),
        Role::Gazer => Color::srgb_u8(0xf0, 0xd0, 0x50),
    }
}

#[derive(Component)]
pub struct AllEye {
    x: f32,
    y: f32,
    anim: u32,
    open: bool,
    gaze: Option<(Entity, i32)>, // warning line + frames until the stare fires
    gaze_cd: i32,
    nova_cd: i32,
    blink_acc: i32, // damage soaked since the last blink
    last_hp: i32,
    shut_img: Handle<Image>,
    open_img: Handle<Image>,
}

#[derive(Component)]
pub struct EyeStalk {
    slot: usize,
    role: Role,
    cd: i32,
    beam: Option<(Entity, i32)>,
    x: f32,
    y: f32,
}

/// A leech the summoner woke (marker caps the muck at 2).
#[derive(Component)]
pub struct EyeSpawn;

/// Any beam quad (gaze warning or a fired ray) — despawned with its owner.
#[derive(Component)]
pub struct EyeBeam;

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let shut_img = images.add(crate::gfx::bake(&ORB_SHUT, PAL));
    let open_img = images.add(crate::gfx::bake(&ORB_OPEN, PAL));
    let stalk_img = images.add(crate::gfx::bake(&STALK, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (ox, oy) = (141.0, 78.0);
    for (slot, role) in ROLES.into_iter().enumerate() {
        let mut s = Sprite::from_image(stalk_img.clone());
        s.color = role_tint(role); // each menace wears its colour
        commands.spawn((
            s,
            at(PLAY_X + ox, PLAY_Y + oy, 12.0, 14.0, actor_z(oy + 12.0)),
            PIXEL_LAYER,
            RoomActor,
            EyeStalk { slot, role, cd: 90 + slot as i32 * 37, beam: None, x: ox, y: oy },
            Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(1), persistent: true, knock: 0.0 },
            Health { hp: STALK_HP, max: STALK_HP, defense: 0, invuln: 0, flash: 0 },
            HurtProfile { invuln: 10, flash: 8, kb_base: 0.0, kb_frames: 0 },
            Knockback::default(),
            Hitbox { x: ox, y: oy, w: 10.0, h: 8.0 },
        ));
    }
    commands.spawn((
        Sprite::from_image(shut_img.clone()),
        at(PLAY_X + ox, PLAY_Y + oy, 22.0, 20.0, actor_z(oy + 18.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE ALL-EYE"),
        crate::app::dungeon::DungeonBoss,
        AllEye {
            x: ox,
            y: oy,
            anim: 0,
            open: false,
            gaze: None,
            gaze_cd: 200,
            nova_cd: 150,
            blink_acc: 0,
            last_hp: hp,
            shut_img,
            open_img,
        },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.2 * (1.0 - 0.92), kb_frames: 11 },
        Knockback::default(),
        Hitbox { x: ox + 2.0, y: oy + 2.0, w: 18.0, h: 14.0 },
    ));
}

/// A beam quad strung between two room-px points (the tongue's transform math).
fn beam_transform(ax: f32, ay: f32, bx: f32, by: f32, z: f32) -> (Transform, f32) {
    let pa = at(PLAY_X + ax, PLAY_Y + ay, 0.0, 0.0, z).translation;
    let pb = at(PLAY_X + bx, PLAY_Y + by, 0.0, 0.0, z).translation;
    let len = (pb - pa).truncate().length().max(1.0);
    let tf = Transform::from_translation((pa + pb) / 2.0)
        .with_rotation(Quat::from_rotation_z((pb.y - pa.y).atan2(pb.x - pa.x)));
    (tf, len)
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut pulled: ResMut<crate::app::play::Pulled>,
    mut statuses: ResMut<crate::app::status::Statuses>,
    mut rng: ResMut<GameRng>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<(&Player, &Health, &Hitbox), Without<AllEye>>,
    mut orbs: Query<
        (&mut AllEye, &mut Health, &mut Hitbox, &mut Sprite, &mut Transform, &mut Visibility),
        (Without<Player>, Without<EyeStalk>, Without<EyeBeam>),
    >,
    mut stalks: Query<
        (&mut EyeStalk, &mut Health, &mut Hitbox, &mut Transform, &mut Visibility),
        (Without<AllEye>, Without<EyeBeam>, Without<Player>),
    >,
    mut beams: Query<(&mut Sprite, &mut Transform), (With<EyeBeam>, Without<AllEye>, Without<EyeStalk>)>,
    spawns: Query<(), With<EyeSpawn>>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok((p, ph, phb)) = players.single() else { return };
    let Ok((mut o, mut h, mut hb, mut spr, mut tf, mut vis)) = orbs.single_mut() else { return };
    o.anim += 1;
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let stalk_count = stalks.iter().count();
    let want_open = stalk_count == 0;
    if want_open != o.open {
        o.open = want_open;
        spr.image = if want_open { o.open_img.clone() } else { o.shut_img.clone() };
        h.flash = 8;
        sfx.write(crate::app::sfx::Sfx("warpCharge"));
    }
    if !o.open {
        h.invuln = h.invuln.max(2); // lids like bog-iron while the stalks watch
    }

    // --- The drift: a slow lemniscate over the mire (quicker bared). ---
    let rate = if o.open { 1.6 } else { 1.0 };
    o.x = 141.0 + ((o.anim as f32) * 0.011 * rate).sin() * 52.0;
    o.y = 74.0 + ((o.anim as f32) * 0.022 * rate).sin() * 16.0;
    let (ocx, ocy) = (o.x + 11.0, o.y + 9.0);

    // --- The stalks: ring the orb, each menace on its own clock. ---
    for (mut st, sth, mut sthb, mut sttf, mut stvis) in &mut stalks {
        let a = (o.anim as f32) * 0.007 + st.slot as f32 / 5.0 * std::f32::consts::TAU;
        st.x = ocx + a.cos() * 40.0 - 6.0;
        st.y = ocy + a.sin() * 26.0 - 7.0;
        *sthb = Hitbox { x: st.x + 1.0, y: st.y + 1.0, w: 10.0, h: 8.0 };
        *sttf = at(PLAY_X + st.x, PLAY_Y + st.y, 12.0, 14.0, actor_z(st.y + 12.0));
        *stvis = if sth.flash > 0 && (sth.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
        let (scx, scy) = (st.x + 6.0, st.y + 5.0);
        let pd = ((pcx - scx).powi(2) + (pcy - scy).powi(2)).sqrt().max(0.001);
        st.cd -= 1;
        // A live beam follows its endpoints, then dies.
        if let Some((be, mut left)) = st.beam {
            left -= 1;
            if left <= 0 {
                commands.entity(be).despawn();
                st.beam = None;
            } else {
                if let Ok((_, mut btf)) = beams.get_mut(be) {
                    let (t2, len) = beam_transform(scx, scy, pcx, pcy, 9.2);
                    *btf = t2.with_scale(Vec3::new(len, 1.0, 1.0));
                }
                st.beam = Some((be, left));
            }
        }
        if st.cd > 0 {
            continue;
        }
        match st.role {
            Role::Puller if pd < 110.0 && ph.invuln == 0 && pulled.0.is_none() => {
                st.cd = 190;
                pulled.0 = Some(crate::app::play::Pull { tx: o.x, ty: o.y, t: 26 });
                let mut s = Sprite::from_color(role_tint(Role::Puller), Vec2::new(1.0, 2.0));
                s.custom_size = Some(Vec2::new(1.0, 2.0));
                let (bt, len) = beam_transform(scx, scy, pcx, pcy, 9.2);
                let be = commands.spawn((s, bt.with_scale(Vec3::new(len, 1.0, 1.0)), PIXEL_LAYER, RoomActor, EyeBeam)).id();
                st.beam = Some((be, 14));
                sfx.write(crate::app::sfx::Sfx("tink"));
            }
            Role::Bolter if pd < 150.0 => {
                st.cd = 130;
                let base = (pcy - scy).atan2(pcx - scx);
                for i in -1..=1i32 {
                    let ang = base + i as f32 * 0.28;
                    commands.spawn((
                        EBolt { x: scx - 4.0, y: scy - 4.0, vx: ang.cos() * 2.2, vy: ang.sin() * 2.2, life: 120 },
                        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                        HitOnce::default(),
                        Hitbox { x: scx - 1.0, y: scy - 1.0, w: 7.0, h: 7.0 },
                        Sprite::from_image(art.bolt(BOG, 0xe8ffc0)),
                        at(PLAY_X + scx - 3.0, PLAY_Y + scy - 3.0, 8.0, 8.0, 8.6),
                        PIXEL_LAYER,
                        RoomActor,
                    ));
                }
            }
            Role::Summoner if spawns.iter().count() < 2 => {
                st.cd = 260;
                if let Some(idx) = crate::actors::mobs::def_index("leech") {
                    commands.spawn((crate::actors::mobs::mob_bundle(idx, st.x, st.y + 8.0), RoomActor, PIXEL_LAYER, EyeSpawn));
                    spawn_burst(&mut commands, &mut rng, Vec2::new(scx, scy + 8.0), BOG, 8);
                }
            }
            Role::Slower if pd < 110.0 => {
                st.cd = 170;
                statuses.add("slow", 90);
                let mut s = Sprite::from_color(role_tint(Role::Slower), Vec2::new(1.0, 2.0));
                s.custom_size = Some(Vec2::new(1.0, 2.0));
                let (bt, len) = beam_transform(scx, scy, pcx, pcy, 9.2);
                let be = commands.spawn((s, bt.with_scale(Vec3::new(len, 1.0, 1.0)), PIXEL_LAYER, RoomActor, EyeBeam)).id();
                st.beam = Some((be, 14));
            }
            Role::Gazer if pd < 150.0 && st.beam.is_none() => {
                // The lock-on: a thin warning line; the STARE fires when it snaps.
                st.cd = 220;
                let mut s = Sprite::from_color(role_tint(Role::Gazer).with_alpha(0.5), Vec2::new(1.0, 1.0));
                s.custom_size = Some(Vec2::new(1.0, 1.0));
                let (bt, len) = beam_transform(scx, scy, pcx, pcy, 9.2);
                let be = commands.spawn((s, bt.with_scale(Vec3::new(len, 1.0, 1.0)), PIXEL_LAYER, RoomActor, EyeBeam)).id();
                st.beam = Some((be, 30));
            }
            _ => {
                st.cd = 20; // conditions unmet: glance again shortly
            }
        }
        // The gazer's snap: at the beam's last frame, MOVEMENT is punished.
        if st.role == Role::Gazer
            && let Some((_, 1)) = st.beam
            && p.moving
        {
            commands.spawn((
                EBolt { x: pcx - 4.0, y: pcy - 4.0, vx: 0.0, vy: 0.0, life: 2 },
                Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                HitOnce::default(),
                Hitbox { x: phb.x, y: phb.y, w: phb.w, h: phb.h },
                Sprite::from_color(role_tint(Role::Gazer), Vec2::new(4.0, 4.0)),
                at(PLAY_X + pcx - 2.0, PLAY_Y + pcy - 2.0, 4.0, 4.0, 9.3),
                PIXEL_LAYER,
                RoomActor,
            ));
            sfx.write(crate::app::sfx::Sfx("tink"));
        }
    }

    // --- Bared: the whole orb gazes and novas, and BLINKS when wounded deep. ---
    if o.open {
        let dmg_taken = (o.last_hp - h.hp).max(0);
        o.blink_acc += dmg_taken;
        if o.blink_acc >= 6 {
            o.blink_acc = 0;
            spawn_burst(&mut commands, &mut rng, Vec2::new(ocx, ocy), 0xb060f0, 10);
            o.x = (16.0 + rng.0.next_f64() as f32 * (PX_W as f32 - 60.0)).clamp(12.0, PX_W as f32 - 34.0);
            o.y = (28.0 + rng.0.next_f64() as f32 * (PX_H as f32 - 90.0)).clamp(24.0, PX_H as f32 - 40.0);
            h.invuln = h.invuln.max(10);
            h.flash = h.flash.max(8);
        }
        o.nova_cd -= 1;
        if o.nova_cd <= 0 {
            o.nova_cd = 140;
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU + (o.anim as f32) * 0.01;
                commands.spawn((
                    EBolt { x: ocx - 4.0, y: ocy - 4.0, vx: a.cos() * 2.0, vy: a.sin() * 2.0, life: 110 },
                    Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                    HitOnce::default(),
                    Hitbox { x: ocx - 1.0, y: ocy - 1.0, w: 7.0, h: 7.0 },
                    Sprite::from_image(art.bolt(IRIS, 0xfff8d0)),
                    at(PLAY_X + ocx - 3.0, PLAY_Y + ocy - 3.0, 8.0, 8.0, 8.6),
                    PIXEL_LAYER,
                    RoomActor,
                ));
            }
        }
        o.gaze_cd -= 1;
        if o.gaze_cd <= 0 && o.gaze.is_none() {
            o.gaze_cd = 190;
            let mut s = Sprite::from_color(role_tint(Role::Gazer).with_alpha(0.5), Vec2::new(1.0, 1.0));
            s.custom_size = Some(Vec2::new(1.0, 1.0));
            let (bt, len) = beam_transform(ocx, ocy, pcx, pcy, 9.2);
            let be = commands.spawn((s, bt.with_scale(Vec3::new(len, 1.0, 1.0)), PIXEL_LAYER, RoomActor, EyeBeam)).id();
            o.gaze = Some((be, 22));
        }
        if let Some((be, mut left)) = o.gaze {
            left -= 1;
            if left <= 0 {
                if p.moving {
                    commands.spawn((
                        EBolt { x: pcx - 4.0, y: pcy - 4.0, vx: 0.0, vy: 0.0, life: 2 },
                        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                        HitOnce::default(),
                        Hitbox { x: phb.x, y: phb.y, w: phb.w, h: phb.h },
                        Sprite::from_color(role_tint(Role::Gazer), Vec2::new(4.0, 4.0)),
                        at(PLAY_X + pcx - 2.0, PLAY_Y + pcy - 2.0, 4.0, 4.0, 9.3),
                        PIXEL_LAYER,
                        RoomActor,
                    ));
                }
                commands.entity(be).despawn();
                o.gaze = None;
            } else {
                if let Ok((_, mut btf)) = beams.get_mut(be) {
                    let (t2, len) = beam_transform(ocx, ocy, pcx, pcy, 9.2);
                    *btf = t2.with_scale(Vec3::new(len, 1.0, 1.0));
                }
                o.gaze = Some((be, left));
            }
        }
    }
    o.last_hp = h.hp;

    // --- Sync. ---
    *hb = Hitbox { x: o.x + 2.0, y: o.y + 2.0, w: 18.0, h: 14.0 };
    let bob = ((o.anim as f32) * 0.09).sin() * 2.0;
    *tf = at(PLAY_X + o.x, PLAY_Y + o.y + bob, 22.0, 20.0, actor_z(o.y + 18.0));
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

/// Plucked stalks burst; the fallen orb takes its court (and any live beams) along.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    orbs: Query<(Entity, &AllEye, &Health), Without<EyeStalk>>,
    stalks: Query<(Entity, &EyeStalk, &Health), Without<AllEye>>,
    beams: Query<Entity, With<EyeBeam>>,
) {
    let Ok((oe, o, oh)) = orbs.single() else { return };
    for (e, st, sh) in &stalks {
        if sh.hp > 0 {
            continue;
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(st.x + 6.0, st.y + 5.0), BOG, 10);
        if let Some((be, _)) = st.beam {
            commands.entity(be).despawn();
        }
        commands.entity(e).despawn();
        sfx.write(crate::app::sfx::Sfx("stone"));
    }
    if oh.hp <= 0 {
        for (e, st, _) in &stalks {
            if let Some((be, _)) = st.beam {
                commands.entity(be).despawn();
            }
            commands.entity(e).despawn();
        }
        for be in &beams {
            commands.entity(be).despawn();
        }
        let (cx, cy) = (o.x + 11.0, o.y + 9.0);
        for i in 0..3 {
            let off = i as f32 * 7.0 - 7.0;
            spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.5), IRIS, 12);
        }
        let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
        crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
        crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true, None);
        crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
        stats.bump("kills", 1.0);
        stats.bump_kill("boss");
        sfx.write(crate::app::sfx::Sfx("stone"));
        commands.entity(oe).despawn();
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
        check("orb_shut", &ORB_SHUT, 22);
        check("orb_open", &ORB_OPEN, 22);
        check("stalk", &STALK, 12);
    }
}
