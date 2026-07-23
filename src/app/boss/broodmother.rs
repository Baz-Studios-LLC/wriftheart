//! THE BROODMOTHER — the nest-tyrant of the old ruins and barrows (ruins / bellbarrow). She
//! turns the arena into her web: her signature is the SNARE + SWARM — she lays sticky WEB
//! PATCHES that bog your boots, then pours streams of SPIDERLINGS out to overwhelm you while
//! you're stuck. A SPINNERET SHOT gums you at range (Slowed) if you try to keep your
//! distance. Thin the swarm, tear free of the web, and punish her between broods. Each third
//! of her health lost quickens the spinning.
//!
//! One of the six consolidated bespoke bosses (boss/mod.rs).

use bevy::prelude::*;

use crate::app::battle::projectiles::EBolt;
use crate::app::battle::{spawn_burst, GameRng, RoomActor};
use crate::app::play::Player;
use crate::app::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::combat::{Afflicts, Combatant, Health, Hitbox, HitOnce, HurtProfile, Knockback, Team};
use crate::gfx::{at, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};

const HP: f64 = 58.0; // js ruins/bellbarrow pool (x HP_MUL)
const CHITIN: u32 = 0x5a3a4a;
const MARK: u32 = 0xd83848;
const WEB: u32 = 0xd8d4e0;
const PAL: &[(char, u32)] = &[
    ('C', CHITIN),     // chitin
    ('c', 0x3a2432),   // chitin shade
    ('K', 0x201018),   // outline
    ('M', MARK),       // hourglass mark
    ('L', 0x7a5a68),   // legs
    ('E', 0xff6070),   // eyes
    ('W', WEB),        // web silk
];

const MOTHER: [&str; 20] = [
    "L..L......L..L..",
    ".LL.L....L.LL...",
    "..LL.LKKL.LL....",
    "...LLKCCKLL.....",
    "....KCEEC K.....",
    "...KCCCCCCK.....",
    "..KCcCCCCcCK....",
    "..KCCCCCCCCK....",
    "..KCCMMMMCCK....",
    "..KCMMWWMMCK....",
    "..KCCMMMMCCK....",
    "..KCcCCCCcCK....",
    "...KCCCCCCK.....",
    "....KCCCCK......",
    "...LLKCCKLL.....",
    "..LL.LKKL.LL....",
    ".LL.L....L.LL...",
    "L..L......L..L..",
    "................",
    "................",
];

const WEBART: [&str; 10] = [
    "WW..W..W..W..WW.",
    ".W.WWWWWWWW.W...",
    "..WW.WWWW.WW....",
    ".WWWWWWWWWWWW...",
    "W..WWWWWWWW..W..",
    ".WWWWWWWWWWWW...",
    "..WW.WWWW.WW....",
    ".W.WWWWWWWW.W...",
    "WW..W..W..W..WW.",
    "................",
];

const MAX_SPIDERS: usize = 5;

#[derive(Component)]
pub struct Broodmother {
    x: f32,
    y: f32,
    anim: u32,
    phase: u8,
    web_cd: i32,
    swarm_cd: i32,
    shot_cd: i32,
    web_img: Handle<Image>,
}

/// A sticky web patch — bogs boots that cross it (no damage), fades slowly.
#[derive(Component)]
pub struct WebPatch {
    x: f32,
    y: f32,
    t: i32,
}

/// A spiderling the mother birthed — marked so the swarm stays capped.
#[derive(Component)]
pub struct Spiderling;

pub(crate) fn spawn(commands: &mut Commands, images: &mut Assets<Image>) {
    let img = images.add(crate::gfx::bake(&MOTHER, PAL));
    let web_img = images.add(crate::gfx::bake(&WEBART, PAL));
    let hp = (HP * crate::actors::mobs::HP_MUL).round() as i32;
    let (bx, by) = (128.0, 44.0);
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X + bx, PLAY_Y + by, 16.0, 20.0, actor_z(by + 18.0)),
        PIXEL_LAYER,
        RoomActor,
        super::BossName("THE BROODMOTHER"),
        crate::app::dungeon::DungeonBoss,
        Broodmother { x: bx, y: by, anim: 0, phase: 0, web_cd: 90, swarm_cd: 130, shot_cd: 70, web_img },
        Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(3), persistent: true, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 30, flash: 0 },
        HurtProfile { invuln: 10, flash: 8, kb_base: 2.0, kb_resist: 0.55, kb_frames: 10 },
        Knockback::default(),
        Hitbox { x: bx + 2.0, y: by + 4.0, w: 12.0, h: 12.0 },
    ));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn tick(
    mut commands: Commands,
    in_dungeon: Res<crate::app::dungeon::InDungeon>,
    art: Res<crate::actors::mobs::MobArtBank>,
    mut statuses: ResMut<crate::app::status::Statuses>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    players: Query<(&Player, &Hitbox), Without<Broodmother>>,
    mut mothers: Query<(&mut Broodmother, &mut Health, &mut Hitbox, &mut Transform, &mut Visibility), (Without<Player>, Without<WebPatch>)>,
    mut webs: Query<(Entity, &mut WebPatch, &mut Sprite), (Without<Broodmother>, Without<Player>)>,
    spiders: Query<(), With<Spiderling>>,
) {
    if in_dungeon.0.is_none() {
        return;
    }
    let Ok((p, phb)) = players.single() else { return };
    let Ok((mut b, mut h, mut hb, mut tf, mut vis)) = mothers.single_mut() else { return };
    b.anim += 1;
    let (pcx, pcy) = (p.x + 8.0, p.y + 8.0);
    let (bcx, bcy) = (b.x + 8.0, b.y + 10.0);

    let frac = h.hp as f32 / h.max.max(1) as f32;
    let ph = if frac > 0.66 { 0 } else if frac > 0.33 { 1 } else { 2 };
    if ph > b.phase {
        b.phase = ph;
        h.flash = 16;
        h.invuln = h.invuln.max(18);
        b.swarm_cd = 24;
        sfx.write(crate::app::sfx::Sfx("tink"));
    }
    let tempo = 1.0 + b.phase as f32 * 0.22;

    // --- Skitter: keep a little distance, circling on the web she's spun. ---
    let dx = bcx - pcx;
    let dy = bcy - pcy;
    let d = (dx * dx + dy * dy).sqrt().max(0.001);
    let want = if d < 56.0 { 0.8 } else if d > 100.0 { -0.6 } else { 0.0 };
    b.x = (b.x + dx / d * want).clamp(8.0, PX_W as f32 - 24.0);
    b.y = (b.y + dy / d * want * 0.7).clamp(16.0, PX_H as f32 - 30.0);

    // --- SNARE (signature part 1): lay a sticky web patch on your ground. ---
    b.web_cd -= 1;
    if b.web_cd <= 0 {
        b.web_cd = (140.0 / tempo) as i32;
        let n = 1 + b.phase as i32;
        for i in 0..n {
            let a = b.anim as f32 * 0.8 + i as f32 * 2.4;
            let (wx, wy) = ((pcx + a.cos() * 18.0 * i as f32 - 8.0).clamp(2.0, PX_W as f32 - 18.0), (pcy + a.sin() * 18.0 * i as f32 - 5.0).clamp(2.0, PX_H as f32 - 12.0));
            commands.spawn((
                Sprite::from_image(b.web_img.clone()),
                at(PLAY_X + wx, PLAY_Y + wy, 16.0, 10.0, 1.6),
                PIXEL_LAYER,
                RoomActor,
                WebPatch { x: wx, y: wy, t: 420 },
            ));
        }
    }

    // --- SWARM (signature part 2): pour spiderlings out (capped). ---
    b.swarm_cd -= 1;
    if b.swarm_cd <= 0 {
        b.swarm_cd = (180.0 / tempo) as i32;
        let live = spiders.iter().count();
        let n = (2 + b.phase as usize).min(MAX_SPIDERS.saturating_sub(live));
        for i in 0..n {
            if let Some(idx) = crate::actors::mobs::def_index("spider") {
                let off = (i as f32 - n as f32 / 2.0) * 10.0;
                commands.spawn((
                    crate::actors::mobs::mob_bundle(idx, bcx + off - 7.0, bcy),
                    RoomActor,
                    PIXEL_LAYER,
                    crate::app::dungeon::DungeonFoe("spider"),
                    Spiderling,
                ));
            }
        }
        sfx.write(crate::app::sfx::Sfx("tink"));
    }

    // --- SPINNERET SHOT: gum you at range (Slowed). ---
    b.shot_cd -= 1;
    if b.shot_cd <= 0 {
        b.shot_cd = (120.0 / tempo) as i32;
        let a = (pcy - bcy).atan2(pcx - bcx);
        let e = commands
            .spawn((
                EBolt { x: bcx - 4.0, y: bcy, vx: a.cos() * 2.2, vy: a.sin() * 2.2, life: 120 },
                Combatant { team: Team::Enemy, hurt_team: Some(Team::Player), damage: Some(2), persistent: false, knock: 0.0 },
                HitOnce::default(),
                Hitbox { x: bcx - 1.0, y: bcy + 3.0, w: 7.0, h: 7.0 },
                Sprite::from_image(art.bolt(WEB, 0xffffff)),
                at(PLAY_X + bcx - 5.0, PLAY_Y + bcy + 1.0, 8.0, 8.0, 8.6),
                PIXEL_LAYER,
                RoomActor,
            ))
            .id();
        commands.entity(e).insert(Afflicts("slow", 140));
    }

    // --- Web patches: bog the boots that cross them. ---
    for (e, mut w, mut wspr) in &mut webs {
        w.t -= 1;
        if w.t <= 0 {
            commands.entity(e).despawn();
            continue;
        }
        wspr.color = Color::srgba(1.0, 1.0, 1.0, (w.t as f32 / 90.0).min(0.7));
        if overlap((phb.x, phb.y, phb.w, phb.h), (w.x + 1.0, w.y + 1.0, 14.0, 8.0)) {
            statuses.add("slow", 18);
        }
    }

    // --- Sync. ---
    *hb = Hitbox { x: b.x + 2.0, y: b.y + 4.0, w: 12.0, h: 12.0 };
    let bob = ((b.anim as f32) * 0.18).sin() * 1.0;
    *tf = at(PLAY_X + b.x, PLAY_Y + b.y + bob, 16.0, 20.0, actor_z(b.y + 18.0));
    *vis = if h.flash > 0 && (h.flash & 1) == 1 { Visibility::Hidden } else { Visibility::Inherited };
}

fn overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

/// The mother falls; her web tears away; the arena banks the reward.
#[allow(clippy::too_many_arguments)]
pub(crate) fn deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    mut progress: ResMut<crate::app::rewards::Progress>,
    mut alloc: ResMut<crate::app::slideout::TreeAlloc>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut sfx: MessageWriter<crate::app::sfx::Sfx>,
    mothers: Query<(Entity, &Broodmother, &Health)>,
    webs: Query<Entity, With<WebPatch>>,
) {
    let Ok((e, b, h)) = mothers.single() else { return };
    if h.hp > 0 {
        return;
    }
    for we in &webs {
        commands.entity(we).despawn();
    }
    let (cx, cy) = (b.x + 8.0, b.y + 10.0);
    for i in 0..3 {
        let off = i as f32 * 7.0 - 7.0;
        spawn_burst(&mut commands, &mut rng, Vec2::new(cx + off, cy + off * 0.4), MARK, 12);
    }
    let coins = 30 + (rng.0.next_f64() * 40.0) as i32;
    crate::app::gather::spawn_coin(&mut commands, &mut images, coins, cx, cy);
    crate::app::gather::spawn_pickup(&mut commands, &mut images, "potion", 1, cx + 6.0, cy + 4.0, true, None);
    crate::app::rewards::gain_xp(&mut progress, &mut alloc, 45);
    stats.bump("kills", 1.0);
    stats.bump_kill("boss");
    sfx.write(crate::app::sfx::Sfx("tink"));
    commands.entity(e).despawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids_rectangular() {
        for (i, r) in MOTHER.iter().enumerate() {
            assert_eq!(r.chars().count(), 16, "mother row {i}");
        }
        for (i, r) in WEBART.iter().enumerate() {
            assert_eq!(r.chars().count(), 16, "web row {i}");
        }
    }
}
