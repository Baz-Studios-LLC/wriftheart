//! traversal.rs — the GRAPPLE HOOK's flying claw (js grappleHook). The wand-family
//! and the boots set player-motion state directly in play.rs; the hook is the one
//! traversal gadget that needs a PROJECTILE: it flies forward at 5.5, and biting a
//! wall tile OR a resource prop (tree / bush / boulder — js "you to a tree/boulder";
//! bushes are Baz's addition) lodges — setting p.grapple so play.rs reels the hero to
//! the anchor. Snagging a FOE yanks the foe to the hero instead (js "reeling a foe to
//! you"); champions and bosses are too heavy. A miss past 150px just fizzles. The taut
//! rope draws hand-to-claw the whole ride — flight, reel and yank alike — built from
//! endpoint translations like the mimic's tongue (at() treats x,y as top-left, so
//! passing a midpoint displaced the old rope by half its length: the Trello bug).

use bevy::prelude::*;

use super::battle::RoomActor;
use super::gather::GatherNode;
use super::play::{CurGrid, Grapple, Player};
use super::room_render::{PLAY_X, PLAY_Y};
use crate::actors::mobs::Mob;
use crate::combat::Hitbox;
use crate::gfx::{at, bake, PIXEL_LAYER};

/// "Fire the hook" — play.rs gated it on cooldown + no active grapple/hop.
#[derive(Message)]
pub struct FireHook {
    pub dx: f32,
    pub dy: f32,
    pub sx: f32,
    pub sy: f32,
}

/// The claw (js grappleHook): flies straight, then HOLDS — lodged in a wall/prop
/// while the reel runs, or riding a snagged foe while the yank drags it home.
#[derive(Component)]
pub struct Hook {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    sx: f32,
    sy: f32,
    /// Bit a wall or prop: the claw holds until the hero's reel ends.
    lodged: bool,
    /// Snagged a foe: the claw rides it until the yank ends.
    mob: Option<Entity>,
}

/// A foe being YANKED to the hero (js: the hook "reeling a foe to you") — dragged
/// ~4.5px a frame until it lands in melee range or the timer dies. The drag is a
/// straight line (walls don't interrupt it — the js hookshot's cheerful physics).
#[derive(Component)]
pub struct Yanked {
    pub t: i32,
}

/// The rope segment drawn from the hero's hand to the claw (rebuilt each tick).
#[derive(Component)]
struct Rope;

#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn hook_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fires: MessageReader<FireHook>,
    grid: Res<CurGrid>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    mut players: Query<&mut Player>,
    mut hooks: Query<(Entity, &mut Hook, &mut Transform, &mut Sprite), Without<Rope>>,
    ropes: Query<Entity, With<Rope>>,
    nodes: Query<&GatherNode>,
    foes: Query<(Entity, &Mob, &Hitbox), Without<Yanked>>,
    mut yanked: Query<(Entity, &mut Mob, &mut Yanked)>,
) {
    let Ok(mut p) = players.single_mut() else { return };
    for f in fires.read() {
        let img = images.add(bake(&["cccc", "cKKc", "cKKc", "cccc"], &[('c', 0xcfcfcf), ('K', 0x8a8a8a)]));
        let mut spr = Sprite::from_image(img);
        spr.custom_size = Some(Vec2::splat(4.0));
        commands.spawn((
            spr,
            at(PLAY_X + f.sx - 2.0, PLAY_Y + f.sy - 2.0, 4.0, 4.0, 8.7),
            PIXEL_LAYER,
            RoomActor,
            Hook { x: f.sx, y: f.sy, dx: f.dx, dy: f.dy, sx: f.sx, sy: f.sy, lodged: false, mob: None },
        ));
    }
    // Old rope segments are rebuilt fresh below (cheap; one per live hook).
    for e in &ropes {
        commands.entity(e).despawn();
    }
    // --- The yank in progress: drag each snagged foe toward the hero. ---
    let (pcx, pcy) = (p.x + 8.0, p.y + 9.0);
    let mut riding: Vec<(Entity, f32, f32)> = Vec::new();
    for (e, mut m, mut y) in &mut yanked {
        y.t -= 1;
        let (mcx, mcy) = (m.x + 8.0, m.y + 8.0);
        let dist = (pcx - mcx).hypot(pcy - mcy);
        if dist < 18.0 || y.t <= 0 {
            commands.entity(e).remove::<Yanked>(); // delivered to melee range
            continue;
        }
        m.x += (pcx - mcx) / dist * 4.5;
        m.y += (pcy - mcy) / dist * 4.5;
        m.aggro = true; // nothing sleeps through being reeled in
        riding.push((e, m.x + 8.0, m.y + 8.0));
    }
    const SP: f32 = 5.5;
    for (e, mut h, mut tf, _spr) in &mut hooks {
        if let Some(me) = h.mob {
            // Riding a snagged foe: the claw tracks it; the yank's end frees the claw.
            match riding.iter().find(|(re, ..)| *re == me) {
                Some(&(_, mx, my)) => {
                    h.x = mx;
                    h.y = my;
                }
                None => {
                    commands.entity(e).despawn();
                    continue;
                }
            }
        } else if h.lodged {
            // Holding a wall/prop: gone the moment the hero's reel resolves.
            if p.grapple.is_none() {
                commands.entity(e).despawn();
                continue;
            }
        } else {
            h.x += h.dx * SP;
            h.y += h.dy * SP;
            let hook_box = (h.x - 2.0, h.y - 2.0, 4.0, 4.0);
            // A FOE in the rope's path: snag it — the yank reels IT to the hero.
            // Champions (size_mul > 1) are too heavy; bosses aren't Mobs at all.
            let snag = foes.iter().find(|(_, m, hb)| {
                m.size_mul <= 1.0 && !m.downed && overlap(hook_box, (hb.x, hb.y, hb.w, hb.h))
            });
            if let Some((fe, ..)) = snag {
                commands.entity(fe).insert(Yanked { t: 30 });
                h.mob = Some(fe);
                sfx.write(super::sfx::Sfx("reel"));
            } else if nodes.iter().any(|n| overlap(hook_box, ((n.c * 16) as f32, (n.r * 16) as f32, 16.0, 16.0))) {
                // A tree / bush / boulder: lodge and reel the hero to it (js tree/boulder).
                p.grapple = Some(Grapple { tx: h.x - h.dx * 12.0 - 8.0, ty: h.y - h.dy * 12.0 - 8.0, t: 36 });
                h.lodged = true;
                sfx.write(super::sfx::Sfx("tink"));
            } else if grid.0.box_hits_solid(h.x - 2.0, h.y - 2.0, 4.0, 4.0) {
                // Bit a wall tile — lodge and reel the hero to the anchor (js p.grapple).
                p.grapple = Some(Grapple { tx: h.x - h.dx * 12.0 - 8.0, ty: h.y - h.dy * 12.0 - 8.0, t: 36 });
                h.lodged = true;
                sfx.write(super::sfx::Sfx("tink"));
            } else if (h.x - h.sx).hypot(h.y - h.sy) > 150.0 {
                // Missed — fizzle past its reach.
                commands.entity(e).despawn();
                continue;
            }
        }
        *tf = at(PLAY_X + h.x - 2.0, PLAY_Y + h.y - 2.0, 4.0, 4.0, 8.7);
        // The taut rope, hero's CURRENT hand -> claw, endpoint-built like the mimic's
        // tongue (never hand a midpoint to at() — it expects a top-left).
        let pa = at(PLAY_X + p.x + 8.0, PLAY_Y + p.y + 9.0, 0.0, 0.0, 8.65).translation;
        let pb = at(PLAY_X + h.x, PLAY_Y + h.y, 0.0, 0.0, 8.65).translation;
        let len = (pb - pa).truncate().length().max(1.0);
        let mut rope = Sprite::from_color(Color::srgb_u8(0x8a, 0x7a, 0x5a), Vec2::new(len, 1.0));
        rope.custom_size = Some(Vec2::new(len, 1.0));
        let rtf = Transform::from_translation((pa + pb) / 2.0)
            .with_rotation(Quat::from_rotation_z((pb.y - pa.y).atan2(pb.x - pa.x)));
        commands.spawn((rope, rtf, PIXEL_LAYER, RoomActor, Rope));
    }
}

fn overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

pub struct TraversalPlugin;

impl Plugin for TraversalPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FireHook>().add_systems(
            bevy::app::FixedUpdate,
            hook_tick.after(super::play::tick).before(super::play::EndTick).run_if(super::screen::playing),
        );
    }
}
