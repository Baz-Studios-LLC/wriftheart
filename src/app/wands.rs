//! wands.rs — MAGIC (js items.js wands + spell bolts): ONE wand, five runes (venom
//! is PAST-JS — Baz's fourth socketable: a droplet SPRAY that envenoms and leaves
//! poison puddles). The
//! wand casts its socketed rune's spell from an ability slot, spending mana — a
//! tapped-out cast fizzles with a red bar flash and the dry click. USING a rune
//! sockets it (the old rune pops back into the bag; plain Arcane is the default).
//! The spells, js verbatim: ARCANE BOLT (2 mana, the workhorse), FIREBOLT (3 —
//! REAL fire: ignites foes and the world, app/fire.rs spreads it), the FROST BEAM
//! (3 — DEVIATION, Baz: an instant ray, not a bolt — freezes the first foe SOLID
//! for 2s, ice-blue with mist), SPARK BOLT (4 — fast and PIERCING, a whole line).
//! Damage scales with the spell stat; bolts carry the crit fields; frost rides
//! the ChillHit machinery and fire the ScorchHit burn, so mob afflictions land
//! through the same proc pipeline swings use.
//! DEVIATION (flagged): the js tints the wand's SLOT ICON to the socketed gem;
//! the rs slot icon stays arcane-purple until per-state icons port. The js also
//! stops player projectiles on solid PROPS (blockShotsOnProps) — unported for
//! arrows and bolts alike; both sail over bushes (walls still stop them).

use bevy::prelude::*;

use super::battle::RoomActor;
use super::flute::Mana;
use super::play::{CurGrid, Player};
use super::room_render::{PLAY_X, PLAY_Y};
use super::uniques::{ChillHit, ScorchHit};
use crate::combat::{Combatant, CritChance, HitLanded, HitOnce, Hitbox, Team};
use crate::gfx::{at, bake, PIXEL_LAYER};
use crate::input::{Action, ActionState};

/// The socketed rune's element (js player.wandRune) — saved.
#[derive(Resource)]
pub struct WandRune(pub &'static str);
impl Default for WandRune {
    fn default() -> Self {
        WandRune("arcane")
    }
}

/// One spell's numbers (js SPELLS).
pub struct Spell {
    pub el: &'static str,
    pub mana: i32,
    pub dmg: f64,
    pub speed: f32,
    pub life: i32,
    pub color: u32,
    pub core: u32,
    pub fire: bool,
    pub slow: i32,
    pub pierce: bool,
}

pub static SPELLS: [Spell; 5] = [
    Spell { el: "arcane", mana: 2, dmg: 2.0, speed: 4.6, life: 64, color: 0xc8a0ff, core: 0x8050e0, fire: false, slow: 0, pierce: false },
    Spell { el: "fire", mana: 3, dmg: 2.0, speed: 4.2, life: 64, color: 0xfcae40, core: 0xfc5018, fire: true, slow: 0, pierce: false },
    Spell { el: "frost", mana: 3, dmg: 2.0, speed: 4.0, life: 64, color: 0xbff0ff, core: 0x3aa8ff, fire: false, slow: 150, pierce: false },
    Spell { el: "storm", mana: 4, dmg: 2.0, speed: 7.0, life: 42, color: 0xfff2a0, core: 0xfce64a, fire: false, slow: 0, pierce: true },
    Spell { el: "venom", mana: 3, dmg: 1.0, speed: 2.6, life: 24, color: 0xb060f0, core: 0x7028a8, fire: false, slow: 0, pierce: false },
];

pub fn spell_for(el: &str) -> &'static Spell {
    SPELLS.iter().find(|s| s.el == el).unwrap_or(&SPELLS[0])
}

/// The element a rune item carries (js RUNE_DEFS) and the rune an element ejects.
pub fn rune_element(id: &str) -> Option<&'static str> {
    match id {
        "firerune" => Some("fire"),
        "frostrune" => Some("frost"),
        "stormrune" => Some("storm"),
        "venomrune" => Some("venom"),
        _ => None,
    }
}
pub fn element_rune(el: &str) -> Option<&'static str> {
    match el {
        "fire" => Some("firerune"),
        "frost" => Some("frostrune"),
        "storm" => Some("stormrune"),
        "venom" => Some("venomrune"),
        _ => None, // arcane is the bare wand — nothing to eject
    }
}

/// The magic slot presses, routed from play.rs.
#[derive(Message)]
pub enum WandMsg {
    Cast,
    Socket(&'static str), // a rune id was used
    Potion { id: &'static str, amt: i32 }, // manapotion 8; manaelixir full (i32::MAX)
}

/// A spell bolt in flight (js spellBolt): straight, glowing, trailing motes.
#[derive(Component)]
pub struct SpellBolt {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: i32,
    pub color: u32,
    pub fire: bool,
    pub pierce: bool,
}

/// One trail mote (js bolt.trail): a lightening fleck that fades over 9 frames.
#[derive(Component)]
struct TrailMote(i32);

/// The frost beam's 2-tick freezing bite, seated at the tip (no sprite — combat
/// reads the Combatant + Hitbox; FreezeHit rides it into the proc pipeline).
#[derive(Component)]
struct FrostLance {
    t: i32,
}

/// One bar of the frost ray (halo + core), fading over 12 frames.
#[derive(Component)]
struct FrostBeamFx {
    t: i32,
    a0: f32,
}

/// One droplet of the VENOM SPRAY (Baz's fourth rune): a short-lived purple mote
/// that poisons what it splashes; marked droplets leave a PUDDLE where they die.
#[derive(Component)]
struct VenomMote {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: i32,
    puddle: bool,
}

/// A poison puddle on the ground — foes standing in it are re-envenomed while it
/// lasts, and it bubbles as it slowly sinks away.
#[derive(Component)]
pub struct PoisonPuddle {
    pub x: f32,
    pub y: f32,
    pub life: i32,
}

/// One rising poison bubble off a puddle (cosmetic).
#[derive(Component)]
struct PuddleBubble(i32);

const PUDDLE_LIFE: i32 = 360; // ~6s on the ground
const POISON_FRAMES: i32 = 300; // ~5s of venom per splash (uniques.rs ticks it)

/// The 8-way aim (shared shape with archery's — small enough to keep local).
fn aim_vec(state: &ActionState, p: &Player) -> (f32, f32) {
    let dx = (state.held(Action::Right) as i32 - state.held(Action::Left) as i32) as f32;
    let dy = (state.held(Action::Down) as i32 - state.held(Action::Up) as i32) as f32;
    if dx == 0.0 && dy == 0.0 {
        return match p.facing {
            crate::actors::hero::Facing::Up => (0.0, -1.0),
            crate::actors::hero::Facing::Down => (0.0, 1.0),
            crate::actors::hero::Facing::Left => (-1.0, 0.0),
            crate::actors::hero::Facing::Right => (1.0, 0.0),
        };
    }
    let m = dx.hypot(dy);
    (dx / m, dy / m)
}

fn bolt_image(images: &mut Assets<Image>, sp: &Spell) -> Handle<Image> {
    images.add(bake(
        &[".cccccc.", "cccccccc", "ccKKKKcc", "ccKWWKcc", "ccKWWKcc", "ccKKKKcc", "cccccccc", ".cccccc."],
        &[('c', sp.color), ('K', sp.core), ('W', 0xffffff)],
    ))
}

/// Cast / socket / drink — one reader for the three magic routes.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
#[allow(clippy::type_complexity)] // the frost beam's Or-filter (mobs AND goblinkind) is the point
fn wand_msgs(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut msgs: MessageReader<WandMsg>,
    mut rune: ResMut<WandRune>,
    mut mana: ResMut<Mana>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    state: Res<ActionState>,
    tstats: Res<super::slideout::TreeStats>,
    mut rng: ResMut<super::battle::GameRng>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    grid: Res<CurGrid>,
    blockers: Res<super::room_props::RoomBlockers>,
    mobs: Query<&Hitbox, Or<(With<crate::actors::mobs::Mob>, With<crate::actors::goblin::Goblin>)>>,
    players: Query<&Player>,
) {
    let Ok(p) = players.single() else { return };
    for m in msgs.read() {
        match m {
            WandMsg::Cast => {
                let sp = spell_for(rune.0);
                if !mana.spend(sp.mana) {
                    sfx.write(super::sfx::Sfx("tink")); // tapped out — the bar flashes red
                    continue;
                }
                let (dx, dy) = aim_vec(&state, p);
                // js: dmg x (1 + spell stat); the gear rows the catalog banked now bite.
                let dmg = ((sp.dmg * (1.0 + crate::items::gear_stat(&inv, "spell"))) + 0.5).floor().max(1.0) as i32;
                if sp.el == "frost" {
                    // FROST BEAM (Baz: make it unique — a ray, not a ball): an instant
                    // lance from the hero's center along the aim. It sails OVER water
                    // (the bolt rule), stops at walls AND solid props (bushes, trees,
                    // rocks — the blocker list), and the first foe it touches is
                    // FROZEN SOLID for 5 seconds.
                    let (cx, cy) = (p.x + 8.0, p.y + 9.0);
                    let mut len: f32 = 8.0;
                    while len < 140.0 {
                        let (qx, qy) = (cx + dx * (len + 4.0), cy + dy * (len + 4.0));
                        if qx < 2.0 || qy < 2.0 || qx > crate::room::PX_W as f32 - 2.0 || qy > crate::room::PX_H as f32 - 2.0 {
                            break;
                        }
                        let over_water = grid.0.code_at((qx / 16.0).floor() as i32, (qy / 16.0).floor() as i32) == '~';
                        if !over_water && grid.0.box_hits_solid(qx - 1.0, qy - 1.0, 2.0, 2.0) {
                            break;
                        }
                        if blockers.0.iter().any(|b| qx > b.0 && qx < b.0 + b.2 && qy > b.1 && qy < b.1 + b.3) {
                            break;
                        }
                        len += 4.0;
                    }
                    // The nearest foe along the line caps the ray at its body.
                    let mut best: Option<(f32, Hitbox)> = None;
                    for mhb in &mobs {
                        let (mx, my) = (mhb.x + mhb.w / 2.0, mhb.y + mhb.h / 2.0);
                        let proj = (mx - cx) * dx + (my - cy) * dy;
                        if proj < 0.0 || proj > len {
                            continue;
                        }
                        let (qx, qy) = (cx + dx * proj, cy + dy * proj);
                        if (mx - qx).hypot(my - qy) < mhb.w.max(mhb.h) / 2.0 + 4.0
                            && best.as_ref().is_none_or(|(b, _)| proj < *b)
                        {
                            best = Some((proj, *mhb));
                        }
                    }
                    let tip = best.as_ref().map_or(len, |(p, _)| *p);
                    if let Some((_, mhb)) = best {
                        // The freezing bite, seated ON the foe's own hitbox so the ray
                        // can't miss what it visibly touched — knock 0: a statue stays put.
                        commands.spawn((
                            FrostLance { t: 3 },
                            crate::combat::Combatant { team: Team::Player, hurt_team: Some(Team::Enemy), damage: Some(dmg), persistent: true, knock: 0.0 },
                            CritChance { chance: tstats.crit, mult: 2.0 + tstats.critmult },
                            HitOnce::default(),
                            super::uniques::FreezeHit(300),
                            Hitbox { x: mhb.x - 1.0, y: mhb.y - 1.0, w: mhb.w + 2.0, h: mhb.h + 2.0 },
                            RoomActor,
                        ));
                    }
                    // The ray: a soft ice halo under a white-cold core, fading fast.
                    for (wid, col, a0) in [(5.0, Color::srgba(0.48, 0.78, 1.0, 0.45), 0.45), (2.0, Color::srgba(0.92, 0.99, 1.0, 0.9), 0.9)] {
                        let blen = tip.max(8.0);
                        let spr = Sprite::from_color(col, Vec2::new(blen, wid));
                        let mut tf = at(
                            PLAY_X + cx + (dx * blen - blen) / 2.0,
                            PLAY_Y + cy + (dy * blen - wid) / 2.0,
                            blen,
                            wid,
                            8.9,
                        );
                        tf.rotation = Quat::from_rotation_z(-dy.atan2(dx));
                        commands.spawn((spr, tf, PIXEL_LAYER, RoomActor, FrostBeamFx { t: 0, a0 }));
                    }
                    super::battle::spawn_burst(&mut commands, &mut rng, Vec2::new(cx + dx * tip, cy + dy * tip), 0xbff0ff, 6);
                    sfx.write(super::sfx::Sfx("swing"));
                    continue;
                }
                if sp.el == "venom" {
                    // VENOM SPRAY (Baz's fourth rune): a fan of purple droplets — each
                    // splash envenoms (PoisonHit rides the proc pipeline), and every
                    // third droplet leaves a POISON PUDDLE where it dies (venom_tick).
                    let (cx, cy) = (p.x + 8.0, p.y + 9.0);
                    for i in 0..9 {
                        let spread = (rng.0.next_f64() as f32 - 0.5) * 0.9;
                        let (ca, sa) = (spread.cos(), spread.sin());
                        let (ux, uy) = (dx * ca - dy * sa, dx * sa + dy * ca);
                        let spd = sp.speed * (0.75 + rng.0.next_f64() as f32 * 0.55);
                        let life = sp.life + (rng.0.next_f64() * 10.0) as i32;
                        let col: u32 = if i & 1 == 1 { 0xb060f0 } else { 0x8a48c8 };
                        commands.spawn((
                            Sprite::from_color(Color::srgb_u8((col >> 16) as u8, (col >> 8) as u8, col as u8), Vec2::splat(3.0)),
                            at(PLAY_X + cx - 1.5, PLAY_Y + cy - 1.5, 3.0, 3.0, 8.6),
                            PIXEL_LAYER,
                            RoomActor,
                            VenomMote { x: cx, y: cy, vx: ux * spd, vy: uy * spd, life, puddle: i % 3 == 0 },
                            crate::combat::Combatant { team: Team::Player, hurt_team: Some(Team::Enemy), damage: Some(dmg), persistent: true, knock: 0.0 },
                            CritChance { chance: tstats.crit, mult: 2.0 + tstats.critmult },
                            HitOnce::default(),
                            super::uniques::PoisonHit(POISON_FRAMES),
                            Hitbox { x: cx - 2.0, y: cy - 2.0, w: 4.0, h: 4.0 },
                        ));
                    }
                    super::battle::spawn_burst(&mut commands, &mut rng, Vec2::new(cx + dx * 8.0, cy + dy * 8.0), 0xb060f0, 4);
                    sfx.write(super::sfx::Sfx("swing"));
                    continue;
                }
                let (x, y) = (p.x + dx * 8.0, p.y + dy * 8.0);
                let bolt = commands
                    .spawn((
                        Sprite::from_image(bolt_image(&mut images, sp)),
                        at(PLAY_X + x + 4.0, PLAY_Y + y + 4.0, 8.0, 8.0, 8.6),
                        PIXEL_LAYER,
                        RoomActor,
                        SpellBolt { x, y, vx: dx * sp.speed, vy: dy * sp.speed, life: sp.life, color: sp.color, fire: sp.fire, pierce: sp.pierce },
                        Combatant { team: Team::Player, hurt_team: Some(Team::Enemy), damage: Some(dmg), persistent: true, knock: 1.0 },
                        CritChance { chance: tstats.crit, mult: 2.0 + tstats.critmult },
                        HitOnce::default(),
                        Hitbox { x: x + 4.0, y: y + 4.0, w: 8.0, h: 8.0 },
                    ))
                    .id();
                // Elements ride the swing-proc pipeline: frost chills, fire burns.
                if sp.slow > 0 {
                    commands.entity(bolt).insert(ChillHit(sp.slow));
                }
                if sp.fire {
                    commands.entity(bolt).insert(ScorchHit(96));
                }
                super::battle::spawn_burst(&mut commands, &mut rng, Vec2::new(x + 8.0 + dx * 8.0, y + 9.0 + dy * 8.0), sp.color, 3); // muzzle
                sfx.write(super::sfx::Sfx("swing"));
            }
            WandMsg::Socket(id) => {
                let Some(el) = rune_element(id) else { continue };
                if rune.0 == el {
                    sfx.write(super::sfx::Sfx("tink")); // already imbued — don't waste it
                    continue;
                }
                inv.remove_one(id);
                if let Some(old) = element_rune(rune.0) {
                    inv.add_item(old, 1); // the old rune pops back into the bag
                }
                rune.0 = el;
                sfx.write(super::sfx::Sfx("craft"));
            }
            WandMsg::Potion { id, amt } => {
                if mana.cur >= mana.max {
                    sfx.write(super::sfx::Sfx("tink")); // wasted at full — the js vetoes
                    continue;
                }
                mana.cur = mana.cur.saturating_add(*amt).min(mana.max);
                inv.remove_one(id);
                sfx.write(super::sfx::Sfx("pickup"));
            }
        }
    }
}

/// Flight + trail + retirement (js spellbolt.update): bolts sail over water, die on
/// walls/bounds/timeout in a colour burst; non-piercers also die on the first bite.
fn bolt_tick(
    mut commands: Commands,
    grid: Res<CurGrid>,
    mut rng: ResMut<super::battle::GameRng>,
    mut hits: MessageReader<HitLanded>,
    mut bolts: Query<(Entity, &mut SpellBolt, &mut Transform, &mut Hitbox)>,
    mut motes: Query<(Entity, &mut TrailMote, &mut Sprite), Without<SpellBolt>>,
) {
    let mut spent: Vec<Entity> = Vec::new();
    for hit in hits.read() {
        if bolts.get(hit.attacker).is_ok_and(|(_, b, _, _)| !b.pierce) {
            spent.push(hit.attacker);
        }
    }
    for (e, mut b, mut tf, mut hb) in &mut bolts {
        let done = spent.contains(&e);
        b.x += b.vx;
        b.y += b.vy;
        b.life -= 1;
        let (tc, tr) = (((b.x + 8.0) / 16.0).floor() as i32, ((b.y + 8.0) / 16.0).floor() as i32);
        let over_water = grid.0.code_at(tc, tr) == '~';
        let dead = done
            || (!over_water && grid.0.box_hits_solid(b.x + 4.0, b.y + 4.0, 8.0, 8.0))
            || b.x < -16.0
            || b.x > crate::room::PX_W as f32
            || b.y < -16.0
            || b.y > crate::room::PX_H as f32
            || b.life <= 0;
        if dead {
            let n = if b.fire { 8 } else { 5 };
            super::battle::spawn_burst(&mut commands, &mut rng, Vec2::new(b.x + 8.0, b.y + 8.0), b.color, n);
            commands.entity(e).despawn();
            continue;
        }
        // The trail: one fading mote per tick at the bolt's heels.
        commands.spawn((
            Sprite::from_color(
                Color::srgba(
                    ((b.color >> 16) & 0xff) as f32 / 255.0,
                    ((b.color >> 8) & 0xff) as f32 / 255.0,
                    (b.color & 0xff) as f32 / 255.0,
                    0.7,
                ),
                Vec2::splat(2.0),
            ),
            at(PLAY_X + b.x + 7.0, PLAY_Y + b.y + 7.0, 2.0, 2.0, 8.55),
            PIXEL_LAYER,
            RoomActor,
            TrailMote(9),
        ));
        *hb = Hitbox { x: b.x + 4.0, y: b.y + 4.0, w: 8.0, h: 8.0 };
        *tf = at(PLAY_X + b.x + 4.0, PLAY_Y + b.y + 4.0, 8.0, 8.0, 8.6);
    }
    for (e, mut t, mut spr) in &mut motes {
        t.0 -= 1;
        if t.0 <= 0 {
            commands.entity(e).despawn();
            continue;
        }
        spr.color = spr.color.with_alpha(t.0 as f32 / 9.0 * 0.7);
    }
}

/// The puddle splat, hand-drawn (rim / body / gleam), baked once.
const PUDDLE_ART: [&str; 7] = [
    "....pppp....",
    "..pPPPPPPp..",
    ".pPPdPPPPPp.",
    "pPPPPPPdPPPp",
    ".pPPPPPPPPp.",
    "..ppPPPPpp..",
    "....pp.pp...",
];
const PUDDLE_PAL: &[(char, u32)] = &[('p', 0x5a2a80), ('P', 0x8a48c8), ('d', 0xb060f0)];

/// The foes a puddle can envenom (beasts AND goblinkind — the freeze rule). The
/// Without wall keeps this query's Hitbox READ disjoint from the motes' WRITE —
/// Bevy's checker is archetype-conservative and panics the app at boot otherwise (B0001).
type PuddleFoes<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Hitbox, Option<&'static mut crate::app::uniques::MobAfflictions>),
    (Or<(With<crate::actors::mobs::Mob>, With<crate::actors::goblin::Goblin>)>, Without<VenomMote>),
>;

/// Venom in flight and on the ground: droplets fly, slow, and die (marked ones
/// splat into puddles); puddles bubble, re-envenom whoever stands in them, and
/// sink away; bubbles rise and pop. All of it rides the outgoing slide offset
/// (the fire-glow rule).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
#[allow(clippy::type_complexity)] // three look-alike sprite queries need Without walls
fn venom_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    grid: Res<CurGrid>,
    slide: Res<super::play::SlideState>,
    clock: Res<super::room_render::FrameClock>,
    mut rng: ResMut<super::battle::GameRng>,
    mut motes: Query<(Entity, &mut VenomMote, &mut Transform, &mut Hitbox)>,
    mut puddles: Query<(Entity, &mut PoisonPuddle, &mut Transform, &mut Sprite), (Without<VenomMote>, Without<PuddleBubble>)>,
    mut bubbles: Query<(Entity, &mut PuddleBubble, &mut Transform, &mut Sprite), (Without<VenomMote>, Without<PoisonPuddle>)>,
    mut foes: PuddleFoes,
    mut puddle_img: Local<Option<Handle<Image>>>,
) {
    let (sx, sy) = slide.outgoing_offset().unwrap_or((0.0, 0.0));
    for (e, mut m, mut tf, mut hb) in &mut motes {
        m.vx *= 0.96;
        m.vy *= 0.96;
        m.x += m.vx;
        m.y += m.vy;
        m.life -= 1;
        let over_water = grid.0.code_at(((m.x) / 16.0).floor() as i32, ((m.y) / 16.0).floor() as i32) == '~';
        let dead = m.life <= 0
            || m.x < 2.0
            || m.y < 2.0
            || m.x > crate::room::PX_W as f32 - 2.0
            || m.y > crate::room::PX_H as f32 - 2.0
            || (!over_water && grid.0.box_hits_solid(m.x - 1.0, m.y - 1.0, 2.0, 2.0));
        if dead {
            if m.puddle && !over_water {
                let img = puddle_img.get_or_insert_with(|| images.add(crate::gfx::bake(&PUDDLE_ART, PUDDLE_PAL))).clone();
                let mut spr = Sprite::from_image(img);
                spr.color = Color::srgba(1.0, 1.0, 1.0, 0.85);
                commands.spawn((
                    spr,
                    at(PLAY_X + m.x - 6.0, PLAY_Y + m.y - 3.0, 12.0, 7.0, 3.2), // under the actors
                    PIXEL_LAYER,
                    RoomActor,
                    PoisonPuddle { x: m.x, y: m.y, life: PUDDLE_LIFE },
                ));
            }
            super::battle::spawn_burst(&mut commands, &mut rng, Vec2::new(m.x, m.y), 0x8a48c8, 3);
            commands.entity(e).despawn();
            continue;
        }
        *hb = Hitbox { x: m.x - 2.0, y: m.y - 2.0, w: 4.0, h: 4.0 };
        *tf = at(PLAY_X + sx + m.x - 1.5, PLAY_Y + sy + m.y - 1.5, 3.0, 3.0, 8.6);
    }
    for (e, mut pd, mut tf, mut spr) in &mut puddles {
        pd.life -= 1;
        if pd.life <= 0 {
            commands.entity(e).despawn();
            continue;
        }
        // The last stretch sinks away.
        if pd.life < 60 {
            spr.color = spr.color.with_alpha(pd.life as f32 / 60.0 * 0.85);
        }
        *tf = at(PLAY_X + sx + pd.x - 6.0, PLAY_Y + sy + pd.y - 3.0, 12.0, 7.0, 3.2);
        // Stand in it, wear it: re-envenom overlapping foes (every 12 ticks).
        if clock.0 % 12 == 0 {
            let (px, py, pw, ph) = (pd.x - 7.0, pd.y - 4.0, 14.0, 8.0);
            for (fe, fhb, aff) in &mut foes {
                if fhb.x < px + pw && fhb.x + fhb.w > px && fhb.y < py + ph && fhb.y + fhb.h > py {
                    match aff {
                        Some(mut a) => {
                            if a.poison < POISON_FRAMES {
                                a.poison = POISON_FRAMES;
                            }
                        }
                        None => {
                            commands
                                .entity(fe)
                                .insert(crate::app::uniques::MobAfflictions { poison: POISON_FRAMES, ..Default::default() });
                        }
                    }
                }
            }
        }
        // A lazy bubble now and then.
        if (clock.0 + e.to_bits() as i64) % 26 == 0 {
            let bx = pd.x - 4.0 + rng.0.next_f64() as f32 * 8.0;
            commands.spawn((
                Sprite::from_color(Color::srgba(0.78, 0.56, 0.94, 0.7), Vec2::splat(2.0)),
                at(PLAY_X + sx + bx, PLAY_Y + sy + pd.y - 4.0, 2.0, 2.0, 3.3),
                PIXEL_LAYER,
                RoomActor,
                PuddleBubble(18),
            ));
        }
    }
    for (e, mut b, mut tf, mut spr) in &mut bubbles {
        b.0 -= 1;
        if b.0 <= 0 {
            commands.entity(e).despawn();
            continue;
        }
        tf.translation.y += 0.3; // rises
        spr.color = spr.color.with_alpha(b.0 as f32 / 18.0 * 0.7);
    }
}

/// The frost beam's afterlife: the bite lives 2 ticks, the ray fades over 12.
fn frost_tick(
    mut commands: Commands,
    mut lances: Query<(Entity, &mut FrostLance)>,
    mut beams: Query<(Entity, &mut FrostBeamFx, &mut Sprite)>,
) {
    for (e, mut l) in &mut lances {
        l.t -= 1;
        if l.t <= 0 {
            commands.entity(e).despawn();
        }
    }
    for (e, mut b, mut spr) in &mut beams {
        b.t += 1;
        if b.t >= 12 {
            commands.entity(e).despawn();
            continue;
        }
        spr.color = spr.color.with_alpha((1.0 - b.t as f32 / 12.0) * b.a0);
    }
}

pub struct WandsPlugin;

impl Plugin for WandsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WandRune>().add_message::<WandMsg>().add_systems(
            bevy::app::FixedUpdate,
            (
                wand_msgs.after(super::play::tick).before(crate::combat::resolve_combat),
                bolt_tick.after(crate::combat::resolve_combat),
                frost_tick.after(crate::combat::resolve_combat),
                venom_tick.after(crate::combat::resolve_combat),
            )
                .before(super::play::EndTick)
                .run_if(super::screen::playing),
        );
    }
}
