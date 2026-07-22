//! champions.rs — CHAMPIONS & ELITES (js makeChampion/makeElite + AFFIXES): the
//! same creature, promoted. Worldgen has rolled `champ`/`elite` flags onto mob
//! rows all along (odds climb with distance — parity-pinned); battle's spawn now
//! applies the real js promotion instead of a bare hp multiplier. A CHAMPION is
//! tougher (hp x2.5, +1 damage, xp x3), carries ONE random affix, and glows with
//! an aura tinted to it; an ELITE is a super-rare roaming miniboss — TWICE the
//! size, faster, hp x4, +2 damage, xp x6, TWO affixes, and double loot. Affixes:
//! Venomous / Chilling (on-hit statuses), Swift (speed), Vampiric (its hits mend
//! it), Toughened (hp/armor/knock), Volatile (explodes on death).
//! DEVIATION (flagged): pack-affix projection (the leader lending its affixes to
//! the room's lesser mobs) and the elite's floating name tag come later; goblin
//! champions take the stat bump + aura but not Swift (their walker owns speed).

use bevy::prelude::*;

use super::battle::RoomActor;
use super::play::Player;
use super::room_render::{actor_z, PLAY_X, PLAY_Y};
use crate::actors::mobs::Mob;
use crate::combat::{Afflicts, Health, HitLanded, HurtProfile};
use crate::gfx::{at, PIXEL_LAYER};

pub struct AffixDef {
    pub key: &'static str,
    pub name: &'static str,
    pub color: u32,
}

/// The six (js AFFIXES) — application lives in `apply_affix`.
pub static AFFIXES: [AffixDef; 6] = [
    AffixDef { key: "venomous", name: "VENOMOUS", color: 0x5acb3a },
    AffixDef { key: "chilling", name: "CHILLING", color: 0x7fd8ff },
    AffixDef { key: "swift", name: "SWIFT", color: 0xfce64a },
    AffixDef { key: "vampiric", name: "VAMPIRIC", color: 0xd83048 },
    AffixDef { key: "toughened", name: "TOUGHENED", color: 0xb8b8c0 },
    AffixDef { key: "volatile", name: "VOLATILE", color: 0xfc7030 },
];

/// A promoted leader: rank + its rolled affix keys (the aura tints to the first).
#[derive(Component)]
pub struct Promoted {
    pub elite: bool,
    pub affixes: Vec<&'static str>,
    pub color: u32,
}

/// Its hits mend it (Vampiric).
#[derive(Component)]
pub struct Lifesteal;

/// Explodes on death regardless of its def (the Volatile affix).
#[derive(Component)]
pub struct AffixVolatile;

/// The pulsing ground ring under a leader (js drawChampAura).
#[derive(Component)]
pub struct AuraRing {
    pub owner: Entity,
    pub t: f32,
    pub scale: f32,
}

/// A soft additive ellipse ring, tinted at spawn (20x10, supersampled edge).
pub fn aura_image(images: &mut Assets<Image>, color: u32) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let (w, h) = (20u32, 10u32);
    let mut img = Image::new_fill(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let (r, g, b) = ((color >> 16) as u8, (color >> 8) as u8, color as u8);
    for y in 0..h {
        for x in 0..w {
            // Ellipse ring: distance from the unit-ellipse edge, soft both sides.
            let nx = (x as f32 + 0.5 - w as f32 / 2.0) / (w as f32 / 2.0 - 0.5);
            let ny = (y as f32 + 0.5 - h as f32 / 2.0) / (h as f32 / 2.0 - 0.5);
            let d = (nx * nx + ny * ny).sqrt();
            let a = (1.0 - (d - 0.85).abs() * 6.0).clamp(0.0, 1.0);
            if a > 0.0
                && let Ok(px) = img.pixel_bytes_mut(UVec3::new(x, y, 0))
            {
                px.copy_from_slice(&[r, g, b, (a * 255.0) as u8]);
            }
        }
    }
    images.add(img)
}

/// Promote a freshly-spawned foe (js makeChampion/makeElite). Works on mobs and
/// goblins alike — stat work rides Health/HurtProfile/Afflicts components; the
/// mob-only tweaks (speed/size) apply when a Mob is present.
#[allow(clippy::too_many_arguments)]
pub fn promote(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    ent: Entity,
    elite: bool,
    rng: &mut impl FnMut() -> f64,
) {
    let n = if elite { 2 } else { 1 };
    let mut keys: Vec<&'static str> = Vec::new();
    while keys.len() < n {
        let a = &AFFIXES[(rng() * AFFIXES.len() as f64) as usize % AFFIXES.len()];
        if !keys.contains(&a.key) {
            keys.push(a.key);
        }
    }
    let color = AFFIXES.iter().find(|a| a.key == keys[0]).map(|a| a.color).unwrap_or(0xfc6030);
    let toughened = keys.contains(&"toughened");
    let (hp_mul, dmg_add) = if elite { (4.0, 2) } else { (2.5, 1) };
    commands.entity(ent).entry::<Health>().and_modify(move |mut h| {
        let base = h.hp;
        h.hp = ((base as f64 * hp_mul).round() as i32).max(if elite { 4 } else { 2 });
        if toughened {
            h.hp = (h.hp as f64 * 1.6).round() as i32;
            h.defense += 1;
        }
        h.max = h.hp;
    });
    commands.entity(ent).entry::<crate::combat::Combatant>().and_modify(move |mut c| {
        if let Some(d) = &mut c.damage {
            *d += dmg_add;
        }
    });
    // Knock resistance rides the HurtProfile's shove base (js knockResist).
    let kr = if elite { 0.6 } else { 0.4 } + if toughened { 0.2 } else { 0.0 };
    commands.entity(ent).entry::<HurtProfile>().and_modify(move |mut p| {
        p.kb_base *= (1.0 - kr as f32).max(0.0);
    });
    for key in &keys {
        match *key {
            "venomous" => {
                commands.entity(ent).insert(Afflicts("poison", 130));
            }
            "chilling" => {
                commands.entity(ent).insert(Afflicts("slow", 100));
            }
            "vampiric" => {
                commands.entity(ent).insert(Lifesteal);
            }
            "volatile" => {
                commands.entity(ent).insert(AffixVolatile);
            }
            _ => {} // swift/toughened land below / above
        }
    }
    let swift = keys.contains(&"swift");
    commands.entity(ent).entry::<Mob>().and_modify(move |mut m| {
        if swift {
            m.speed_mul *= 1.4;
        }
        if elite {
            m.speed_mul *= 1.4;
            m.size_mul = (m.size_mul * 2.0).min(3.2); // scaled kinds keep their bulk edge
        }
    });
    commands.entity(ent).insert(Promoted { elite, affixes: keys, color });
    // The aura ring stands under the leader.
    let img = aura_image(images, color);
    commands.spawn((
        Sprite::from_image(img),
        at(PLAY_X, PLAY_Y, 20.0, 10.0, 3.9),
        PIXEL_LAYER,
        RoomActor,
        AuraRing { owner: ent, t: 0.0, scale: if elite { 2.0 } else { 1.0 } },
    ));
}

/// The aura follows its leader and pulses (js drawChampAura); it dies with them.
fn aura_tick(
    mut commands: Commands,
    mut rings: Query<(Entity, &mut AuraRing, &mut Transform, &mut Sprite)>,
    owners: Query<(&Hitbox2, &Health)>,
) {
    for (e, mut ring, mut tf, mut spr) in &mut rings {
        let Ok((hb, h)) = owners.get(ring.owner) else {
            commands.entity(e).despawn();
            continue;
        };
        if h.hp <= 0 {
            commands.entity(e).despawn();
            continue;
        }
        ring.t += 1.0;
        let pulse = 0.35 + 0.2 * (ring.t * 0.15).sin();
        spr.color = Color::srgba(1.0, 1.0, 1.0, pulse);
        let (cx, cy) = (hb.x + hb.w / 2.0, hb.y + hb.h);
        let (w, hgt) = (20.0 * ring.scale, 10.0 * ring.scale);
        spr.custom_size = Some(Vec2::new(w, hgt));
        *tf = at(PLAY_X + cx - w / 2.0, PLAY_Y + cy - hgt / 2.0, w, hgt, actor_z(cy) - 0.02);
    }
}

/// (aura_tick's owner lookup — Hitbox lives in combat; a thin alias keeps the
/// query from clashing with the ring's own components.)
type Hitbox2 = crate::combat::Hitbox;

/// Vampiric: a leader's landed hit on the hero mends it (js lifesteal).
fn lifesteal_tick(
    mut hits: MessageReader<HitLanded>,
    players: Query<Entity, With<Player>>,
    mut leaders: Query<&mut Health, With<Lifesteal>>,
) {
    let Ok(pe) = players.single() else { return };
    for hit in hits.read() {
        if hit.target != pe {
            continue;
        }
        if let Ok(mut h) = leaders.get_mut(hit.attacker)
            && h.hp > 0
        {
            h.hp = (h.hp + 1).min(h.max);
        }
    }
}

/// A floating name tag over an elite (js e.eliteName): the affix names + the mob's
/// bestiary name. Spawned once per elite, then trailed above it.
#[derive(Component)]
struct EliteTag {
    owner: Entity,
    w: f32,
}

/// Build "AFFIXNAME AFFIXNAME BASENAME" (js affixName + BESTIARY name).
fn elite_name(affixes: &[&'static str], kind: &str) -> String {
    let mut parts: Vec<&str> =
        affixes.iter().filter_map(|k| AFFIXES.iter().find(|a| a.key == *k).map(|a| a.name)).collect();
    parts.push(crate::actors::mobs::bestiary_name(kind));
    parts.join(" ")
}

/// Stand a name tag over each fresh elite; trail every live tag above its mob.
fn elite_tags(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    elites: Query<(Entity, &Promoted, &Mob), Without<EliteTag>>,
    mut tags: Query<(Entity, &EliteTag, &mut Transform)>,
    mobs: Query<(&Mob, &Health)>,
) {
    for (e, promo, mob) in &elites {
        if !promo.elite {
            continue;
        }
        let kind = crate::actors::mobs::MOB_DEFS[mob.def].kind;
        let name = elite_name(&promo.affixes, kind);
        // Even-pad to match bake_text's image width — an odd width centres the sprite on
        // a half-pixel and the integer upscale shears the glyphs (the WINDVALE garble,
        // here on every elite's name tag).
        let m = crate::gfx::font::measure(&name);
        let w = (m + (m & 1)) as f32;
        let tag = crate::ui::label(&mut commands, &mut images, &name, 0.0, 0.0, 0xffd0d0, 12.5, RoomActor);
        commands.entity(tag).insert(EliteTag { owner: e, w });
    }
    for (te, tag, mut tf) in &mut tags {
        let Ok((mob, h)) = mobs.get(tag.owner) else {
            commands.entity(te).despawn(); // its elite fell
            continue;
        };
        if h.hp <= 0 {
            commands.entity(te).despawn();
            continue;
        }
        // Centred over the (elite-scaled) body, a little above the head.
        *tf = crate::gfx::at(
            (PLAY_X + mob.x + 8.0 - tag.w / 2.0).floor(),
            (PLAY_Y + mob.y - 10.0).floor(),
            tag.w,
            6.0,
            12.5,
        );
    }
}

/// Which mob a pack projection touched, so it reverts cleanly when the leader falls
/// (js setPackAffixes' _affixBase snapshot). Behavioural affixes only — no max-HP.
#[derive(Component)]
struct PackProjected {
    base_speed: f32,
    afflicted: bool,
    lifesteal: bool,
    volatile: bool,
    toughened: bool,
}

/// The room's lesser mobs borrow their leader's affixes WHILE IT LIVES (js
/// setPackAffixes): the union of every live leader's projectable affixes lands on the
/// non-leaders; the moment the last leader dies, every projection reverts.
/// DEVIATION (flagged): the js also tints a faint pack-aura under each borrower; that
/// visual joins a later polish pass. Projects the packApply values.
#[allow(clippy::type_complexity)]
fn pack_project(
    mut commands: Commands,
    leaders: Query<(&Promoted, &Health)>,
    mut members: Query<(Entity, &mut Mob, &mut Health, Option<&PackProjected>), Without<Promoted>>,
) {
    // The union of live leaders' affixes (js borrows the leader's list).
    let mut keys: Vec<&'static str> = Vec::new();
    for (p, h) in &leaders {
        if h.hp > 0 {
            for k in &p.affixes {
                if !keys.contains(k) {
                    keys.push(k);
                }
            }
        }
    }
    let active = !keys.is_empty();
    for (e, mut m, mut h, proj) in &mut members {
        match (active, proj) {
            (true, None) => {
                // Project (packApply values): snapshot the base, then lend the affixes.
                let base_speed = m.speed_mul;
                let (mut afflicted, mut lifesteal, mut volatile, mut toughened) = (false, false, false, false);
                for k in &keys {
                    match *k {
                        "venomous" => {
                            commands.entity(e).insert(Afflicts("poison", 130));
                            afflicted = true;
                        }
                        "chilling" => {
                            commands.entity(e).insert(Afflicts("slow", 100));
                            afflicted = true;
                        }
                        "vampiric" => {
                            commands.entity(e).insert(Lifesteal);
                            lifesteal = true;
                        }
                        "volatile" => {
                            commands.entity(e).insert(AffixVolatile);
                            volatile = true;
                        }
                        "swift" => m.speed_mul *= 1.3,
                        "toughened" => {
                            h.defense += 1;
                            toughened = true;
                        }
                        _ => {}
                    }
                }
                commands.entity(e).insert(PackProjected { base_speed, afflicted, lifesteal, volatile, toughened });
            }
            (false, Some(p)) => {
                // The last leader fell — revert exactly what was lent.
                m.speed_mul = p.base_speed;
                if p.afflicted {
                    commands.entity(e).remove::<Afflicts>();
                }
                if p.lifesteal {
                    commands.entity(e).remove::<Lifesteal>();
                }
                if p.volatile {
                    commands.entity(e).remove::<AffixVolatile>();
                }
                if p.toughened {
                    h.defense = (h.defense - 1).max(0);
                }
                commands.entity(e).remove::<PackProjected>();
            }
            _ => {}
        }
    }
}

pub struct ChampionsPlugin;

impl Plugin for ChampionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy::app::FixedUpdate,
            (aura_tick, elite_tags, pack_project, lifesteal_tick.after(crate::combat::resolve_combat))
                .before(super::play::EndTick)
                .run_if(super::screen::playing),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affix_keys_unique() {
        let mut keys: Vec<_> = AFFIXES.iter().map(|a| a.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 6);
    }
}
