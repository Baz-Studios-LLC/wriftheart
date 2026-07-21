//! gather.rs — the gathering loop: resource nodes take tool hits (resolve_combat gates the
//! tool + emits tinks), felled nodes spray chips, drop ITEM PICKUPS that magnet into the
//! player's bag, and are remembered per room — gathered nodes respawn NEXT DAY, felled
//! trees day-stamp into [`TreeGrowth`] and grow back stump -> sapling -> young -> full over
//! [`TREE_GROW_DAYS`] (js: roomState/treeGrowth in game.js).
//!
//! The in-game day is wall-clock play time: DAY_LEN = 36000 frames (~10 min); tree stamps
//! use `farm_day` (dawn-shifted, so a day rolls at first light like the crops).

use super::battle::{not_sliding, spawn_burst, GameRng, RoomActor};
use super::play::{CurRoom, Player};
use super::slideout::TreeStats;
use super::room_props::RoomBlockers;
use super::room_render::{actor_z, FrameClock, PLAY_X, PLAY_Y};
use crate::combat::{Health, HitLanded, Tinked};
use crate::gfx::{at, PIXEL_LAYER};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

pub const DAY_LEN: i64 = 36000; // frames per full day/night cycle (~10 min)
pub const TREE_GROW_DAYS: i64 = 3;

/// 0-based absolute day (rolls at noon) — port of `dayNumber`.
pub fn day_number(clock: i64) -> i64 {
    clock.div_euclid(DAY_LEN)
}
/// The farming day (rolls at DAWN, so one night = one growth stage) — port of `farmDay`.
pub fn farm_day(clock: i64) -> i64 {
    (clock + DAY_LEN / 4).div_euclid(DAY_LEN)
}

/// A live resource node: what it is, its tile, and the blocker rect it owns (removed on
/// harvest so the stump doesn't keep blocking).
#[derive(Component)]
pub struct GatherNode {
    pub kind: &'static str,
    pub c: i32,
    pub r: i32,
    pub blocker: Option<(f32, f32, f32, f32)>,
    pub tree: bool,
    /// The area's zone tier (js harvestTier) — deeper nodes drop the richer ore/timber.
    pub tier: i32,
}

/// The ore a rock node yields as a bonus at zone `t` (js ORE_LADDER; T6 -> voidsteel).
pub fn ore_at_tier(t: i32) -> &'static str {
    ["copper", "copper", "iron", "silver", "gold", "mithril", "voidsteel"][t.clamp(0, 6) as usize]
}

/// The wood a biome's TREES yield (Baz: petalwood trees drop PETALWOOD, not oak) —
/// kin biomes share a timber, strangers differ; everywhere plain stays plain wood.
pub fn wood_for_biome(biome: &str) -> &'static str {
    match biome {
        "petalwood" | "bluebell" | "honeyglade" => "petalwood",
        "hollowwood" | "gloammoor" | "starhollow" => "gloomwood",
        "burnt" | "embermaw" | "emberscar" => "charwood",
        "swamp" | "tarmire" | "witherlands" | "blackdeep" => "mirewood",
        "arctic" => "frostpine",
        _ => "wood",
    }
}

/// The timber a tree yields as a bonus at zone `t` (js WOOD_LADDER; only tier 3+ upgrade).
pub fn wood_at_tier(t: i32) -> Option<&'static str> {
    [None, None, None, Some("hardwood"), Some("ironbark"), Some("ironbark"), Some("voidwood")][t.clamp(0, 6) as usize]
}

/// Hit feedback: the node wobbles for 8 frames (js shakeX = sin(hit * 1.7) * 2).
#[derive(Component)]
pub struct Shake {
    pub t: u32,
    pub base_x: f32,
}

/// Per-room gather memory: gathered nodes stay gone for the rest of the DAY they were taken
/// (js roomState nodes + roomExpired), then the room regrows on next entry.
/// (day taken, taken tiles) — one record per disturbed room.
pub type RoomRecord = (i64, HashSet<(i32, i32)>);

#[derive(Resource, Default)]
pub struct GatherState {
    pub rooms: HashMap<(i32, i32), RoomRecord>,
    /// Hand-placed items taken FOREVER: room -> tiles (saved; never regrows).
    pub placed: HashMap<(i32, i32), HashSet<(i32, i32)>>,
    /// Lore tomes collected (saved) — read tomes never respawn anywhere.
    pub tomes: HashSet<&'static str>,
}

impl GatherState {
    pub fn taken(&self, room: (i32, i32), c: i32, r: i32, today: i64) -> bool {
        self.rooms.get(&room).is_some_and(|(day, set)| *day == today && set.contains(&(c, r)))
    }
    /// Has a hand-placed item at this tile been picked up (EVER)? Placed items never
    /// come back — they wait until taken, then stay gone (saved).
    pub fn placed_taken(&self, room: (i32, i32), c: i32, r: i32) -> bool {
        self.placed.get(&room).is_some_and(|set| set.contains(&(c, r)))
    }
}

/// Felled trees: "room -> tile -> cut farm-day" (js treeGrowth). Stage = days since cut.
#[derive(Resource, Default)]
pub struct TreeGrowth(pub HashMap<(i32, i32), HashMap<(i32, i32), i64>>);

/// What a ground pickup yields on touch: an item into the bag, or copper into the purse.
#[derive(Clone, Copy, PartialEq)]
pub enum PickupKind {
    /// A lore tome (id into lore_books::BOOKS) — collects into the tome ledger.
    Book(&'static str),
    Item { id: &'static str, qty: i32 },
    Coin(i32),
}

/// A loose drop in the room (port of `itemPickup`/`coin`): pops up on spawn, settles on
/// the ground with a soft glow, bobs, and — when spawned with `magnet` — homes to a player
/// who can take it (coins always fit; a full bag leaves items resting). Player-dropped
/// items spawn WITHOUT magnet so you don't instantly re-vacuum what you just dropped.
#[derive(Component)]
pub struct Pickup {
    pub kind: PickupKind,
    pub x: f32,
    pub y: f32,
    pub life: u32,
    pub magnet: bool,
    pub t: u32,  // spawn-pop timer (js pickup(): a 9-frame upward arc before it settles)
    pub vy: f32, // pop velocity
    /// A PLACED pickup's home tile: collecting stamps the room's daily gather record
    /// (it returns tomorrow, like every gatherable), and it never blinks out on a timer.
    pub tile: Option<(i32, i32)>,
}

/// The soft ground-glow under a [`Pickup`] — positioned (and reaped) by pickups_tick.
#[derive(Component)]
pub struct PickupGlow(pub Entity);

const GLOW_R: f32 = 10.0; // js collectLights 'item': glowR 10
// Depth-sort at the item's visual FOOT like every actor (js sorts pickups by y; its
// glow lives in the LIGHT pass). The old +0.5 boost ≈ 26px of depth — a drop beside a
// trunk drew its glow OVER the tree (Baz's bug). Icon rides a hair over its own glow.
const ICON_Z_OFF: f32 = 0.02;
const GLOW_Z_OFF: f32 = 0.01;

/// Bake the radial item glow (js Lighting.render step 2: an additive gradient from
/// rgba(150,190,255, gi 0.11) at the centre to transparent at glowR — the "glowing items"
/// bloom, which runs day and night). Bevy sprites alpha-blend instead of adding, and JS
/// alphas read STRONGER under linear blending — halve the intensity (PORT.md gotcha).
fn glow_image(images: &mut Assets<Image>, rgb: [u8; 3], gi: f32) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let s = (GLOW_R * 2.0) as u32; // 20 — even (the centre-anchored sprite rule)
    let mut img = Image::new_fill(
        Extent3d { width: s, height: s, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    for y in 0..s {
        for x in 0..s {
            let d = (x as f32 + 0.5 - GLOW_R).hypot(y as f32 + 0.5 - GLOW_R);
            let a = gi * 0.5 * (1.0 - d / GLOW_R).max(0.0);
            if a > 0.0
                && let Ok(px) = img.pixel_bytes_mut(UVec3::new(x, y, 0))
            {
                px.copy_from_slice(&[rgb[0], rgb[1], rgb[2], (a * 255.0).round() as u8]);
            }
        }
    }
    images.add(img)
}

/// Spawn an item drop at (x, y) — its icon, bobbing over a soft blue glow (js
/// Entities.itemPickup + collectLights type 'item').
pub fn spawn_pickup(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    id: &'static str,
    qty: i32,
    x: f32,
    y: f32,
    magnet: bool,
) -> Entity {
    let icon = crate::items::get(id).map(|d| (d.icon, d.icon_pal));
    let sprite = match icon {
        Some((grid, pal)) => Sprite::from_image(images.add(crate::gfx::bake(grid, pal))),
        // No def (shouldn't happen) — the old 4px material dot as a fallback.
        None => Sprite::from_color(
            Color::srgb_u8((material_color(id) >> 16) as u8, (material_color(id) >> 8) as u8, material_color(id) as u8),
            Vec2::splat(4.0),
        ),
    };
    let s = if icon.is_some() { 8.0 } else { 4.0 };
    spawn_drop(commands, images, PickupKind::Item { id, qty }, sprite, s, 1200, [150, 190, 255], 0.11, x, y, magnet)
}

/// A lore tome on the ground (js Entities.bookPickup): the category-coloured spine over
/// its own glow, a GENEROUS grab zone (books sit on furniture — the take must reach the
/// floor beside it), bobbing gently, waiting forever. Collects into the tome ledger.
pub fn spawn_book(commands: &mut Commands, images: &mut Assets<Image>, id: &'static str, x: f32, y: f32) {
    let Some(b) = crate::lore_books::get(id) else { return };
    let icon = Sprite::from_image(images.add(crate::gfx::bake(
        crate::lore_books::BOOK_GRID,
        &[('C', b.col), ('W', 0xf4ecd0)],
    )));
    let rgb = [(b.col >> 16) as u8, (b.col >> 8) as u8, b.col as u8];
    spawn_drop(commands, images, PickupKind::Book(id), icon, 8.0, u32::MAX, rgb, 0.14, x, y, false);
}

/// A hand-PLACED ground item (the Emberfall stones): sits on its tile with no spawn pop
/// or expiry; collecting stamps the daily gather record so it regrows tomorrow.
pub fn spawn_placed_item(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    id: &'static str,
    x: f32,
    y: f32,
) {
    let pe = spawn_pickup(commands, images, id, 1, x, y, false);
    commands.entity(pe).entry::<Pickup>().and_modify(move |mut p| {
        p.tile = Some(((x as i32).div_euclid(crate::room::TILE), (y as i32).div_euclid(crate::room::TILE)));
        p.t = 9; // already settled — no pop arc
        p.vy = 0.0;
    });
}

/// Spawn a copper drop (js Entities.coin): the square coin over a gold glow, always
/// magnetised — money always fits.
pub fn spawn_coin(commands: &mut Commands, images: &mut Assets<Image>, value: i32, x: f32, y: f32) -> Entity {
    let sprite = Sprite::from_image(images.add(crate::gfx::bake(crate::actors::items_art::COIN_ICON, &[])));
    spawn_drop(commands, images, PickupKind::Coin(value), sprite, 8.0, 600, [255, 210, 90], 0.10, x, y, true)
}

#[allow(clippy::too_many_arguments)] // the two public spawns above are the real API
fn spawn_drop(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    kind: PickupKind,
    sprite: Sprite,
    s: f32,
    life: u32,
    glow_rgb: [u8; 3],
    glow_gi: f32,
    x: f32,
    y: f32,
    magnet: bool,
) -> Entity {
    let pe = commands
        .spawn((
            Pickup { kind, x, y, life, magnet, t: 0, vy: -1.4, tile: None },
            sprite,
            at(PLAY_X + x + 2.0 - s / 2.0, PLAY_Y + y + 2.0 - s / 2.0, s, s, actor_z(y + 6.0) + ICON_Z_OFF),
            PIXEL_LAYER,
            RoomActor,
        ))
        .id();
    commands.spawn((
        PickupGlow(pe),
        Sprite::from_image(glow_image(images, glow_rgb, glow_gi)),
        at(PLAY_X + x + 2.0 - GLOW_R, PLAY_Y + y + 2.0 - GLOW_R, GLOW_R * 2.0, GLOW_R * 2.0, actor_z(y + 6.0) + GLOW_Z_OFF),
        PIXEL_LAYER,
        RoomActor,
    ));
    pe
}

pub struct GatherPlugin;

impl Plugin for GatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GatherState>()
            .init_resource::<TreeGrowth>()
            .add_systems(
                FixedUpdate,
                (node_hits, node_deaths, pickups_tick, leaves_tick)
                    .chain()
                    .run_if(not_sliding)
                    .after(crate::combat::resolve_combat),
            )
            .add_systems(Update, apply_shake);
    }
}

/// Foliage colours per leafy tree kind (js LEAF_COLS): blossoms shed pink petals,
/// blueblooms blue. Dead/burnt/crystal trees aren't here — nothing to shed.
fn leaf_cols(kind: &str) -> Option<[u32; 2]> {
    match kind {
        "oak" => Some([0x74d07d, 0x2f8a3c]),
        "pine" => Some([0x4f9e58, 0x2f7a38]),
        "blossom" => Some([0xffd0ec, 0xf070b0]),
        "jungletree" => Some([0x7fe88a, 0x1ca838]),
        "bluebloom" => Some([0xa9d4ff, 0x5a8fe0]),
        "giantflower" => Some([0xffd24a, 0xff6aa8]),
        _ => None,
    }
}

/// One leaf shaken loose from a struck canopy (js Entities.leafFall): unlike the
/// chip burst, these flutter DOWN gently — slow fall plus side-to-side sway —
/// from the tree's lower crown, drifting past the trunk.
#[derive(Component)]
struct Leaf {
    x: f32,
    y: f32,
    vy: f32,
    ph: f32,
    amp: f32,
    life: i32,
}

/// Shake 3-5 leaves from a canopy (js leafFall(x+8, y+offY+22, 13, cols)) — fired
/// on ANY hit, tink and too-weak-tool included: even a glance shivers the crown.
fn shed_leaves(commands: &mut Commands, rng: &mut GameRng, kind: &'static str, c: i32, r: i32) {
    let Some(cols) = leaf_cols(kind) else { return };
    let (x, y) = ((c * 16) as f32, (r * 16) as f32);
    let (_, oy, ..) = crate::actors::props::prop_anchor(kind);
    let (cx, top) = (x + 8.0, y + oy as f32 + 22.0);
    let n = 3 + (rng.0.next_f64() * 3.0) as i32;
    for _ in 0..n {
        let col = cols[(rng.0.next_f64() * 2.0) as usize % 2];
        let leaf = Leaf {
            x: cx + (rng.0.next_f64() as f32 * 2.0 - 1.0) * 13.0,
            y: top + rng.0.next_f64() as f32 * 12.0,
            vy: 0.16 + rng.0.next_f64() as f32 * 0.14, // a drifting descent (half the js rate)
            ph: rng.0.next_f64() as f32 * std::f32::consts::TAU,
            amp: 0.3 + rng.0.next_f64() as f32 * 0.15, // the pendulum glide width
            life: 70 + (rng.0.next_f64() * 40.0) as i32,
        };
        let tf = at(PLAY_X + leaf.x, PLAY_Y + leaf.y, 2.0, 1.0, 12.0);
        commands.spawn((
            Sprite::from_color(
                Color::srgb_u8((col >> 16) as u8, (col >> 8) as u8, col as u8),
                Vec2::new(2.0, 1.0),
            ),
            leaf,
            tf,
            PIXEL_LAYER,
            super::battle::RoomActor,
        ));
    }
}

/// Flutter the shed leaves down like FALLING LEAVES (a feel pass past the js
/// original — Baz: slower, more leaf-like): each rides a slow pendulum, gliding
/// wide to one side, stalling, then flipping back — the descent is slowest
/// mid-glide and quickest as it turns, and the leaf shows edge-on (1px) through
/// the turn, broadside (2px) through the glide.
fn leaves_tick(mut commands: Commands, mut q: Query<(Entity, &mut Leaf, &mut Sprite, &mut Transform)>) {
    for (e, mut leaf, mut sprite, mut tf) in &mut q {
        leaf.life -= 1;
        if leaf.life <= 0 {
            commands.entity(e).despawn();
            continue;
        }
        let glide = (leaf.life as f32 / 16.0 + leaf.ph).cos();
        leaf.x += glide * leaf.amp;
        leaf.y += leaf.vy * (1.35 - 0.75 * glide.abs());
        let w = if leaf.life <= 10 || glide.abs() < 0.35 { 1.0 } else { 2.0 };
        sprite.custom_size = Some(Vec2::new(w, 1.0));
        *tf = at(PLAY_X + leaf.x.round(), PLAY_Y + leaf.y.round(), w, 1.0, 12.0);
    }
}

/// Chip colour per node kind (js `blood` on the object entities).
fn chip_color(kind: &str) -> u32 {
    match kind {
        "boulder" | "stalagmite" => 0xa8a8a8,
        "crystalspire" => 0x9fe8ff,
        "bush" | "grass" | "cactus" => 0x3a8a2a,
        _ => 0x8a5a2a, // trees: wood chips
    }
}

/// What a felled node drops (the STAPLE half of the js deathEffect tables; tier ladders,
/// herbs-by-luck and silk join with the items/skills ports).
fn drops_for(kind: &str, tier: i32, biome: &str, rng: &mut GameRng) -> Vec<&'static str> {
    let mut out = Vec::new();
    match kind {
        "bush" => {
            out.push("fiber");
            if rng.0.next_f64() < 0.3 {
                out.push("herb");
            }
        }
        "boulder" | "stalagmite" | "crystalspire" => {
            let n = 2 + (rng.0.next_f64() * 2.0) as usize;
            out.extend(std::iter::repeat_n("stone", n));
            // ~35% a chunk of the zone's ore (copper -> voidsteel deeper) (js ORE_LADDER).
            if rng.0.next_f64() < 0.35 {
                out.push(ore_at_tier(tier));
            }
        }
        "cactus" => {
            out.push("fiber");
            out.push("fiber");
        }
        "grass" => {
            if rng.0.next_f64() < 0.05 {
                out.push("fiber");
            }
            // js brush: a stray arrow hides in the tall grass now and then.
            if rng.0.next_f64() < 0.05 {
                out.push("arrow");
            }
        }
        _ => {
            // Trees: 3-5 of the BIOME's wood, + ~30% the zone's better timber deeper
            // in (js WOOD_LADDER).
            let n = 3 + (rng.0.next_f64() * 3.0) as usize;
            out.extend(std::iter::repeat_n(wood_for_biome(biome), n));
            if let Some(w) = wood_at_tier(tier)
                && rng.0.next_f64() < 0.3
            {
                out.push(w);
            }
        }
    }
    out
}

/// Fallback pickup-dot colour for an id with no item def (pickups normally draw icons).
pub fn material_color(id: &str) -> u32 {
    match id {
        "wood" => 0x8a5a2a,
        "petalwood" => 0xd8a0c0,
        "gloomwood" => 0x4a5468,
        "charwood" => 0x3a3230,
        "mirewood" => 0x5a6a3a,
        "frostpine" => 0x9ac0d0,
        "stone" => 0xa8a8a8,
        "fiber" => 0x3a8a2a,
        "herb" => 0x2fbf4f,
        "copper" => 0xc87838,
        _ => 0xfcfcfc,
    }
}

/// Hits on nodes: start the wobble; tinks: a little grey spark (wrong tool bounced off).
fn node_hits(
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    mut hits: MessageReader<HitLanded>,
    mut tinks: MessageReader<Tinked>,
    mut log: ResMut<super::rewards::LootLog>,
    nodes: Query<(&GatherNode, &Transform)>,
) {
    for hit in hits.read() {
        if let Ok((node, tf)) = nodes.get(hit.target) {
            commands.entity(hit.target).insert(Shake { t: 8, base_x: tf.translation.x });
            shed_leaves(&mut commands, &mut rng, node.kind, node.c, node.r);
        }
    }
    for tink in tinks.read() {
        spawn_burst(&mut commands, &mut rng, tink.at, 0xdcdce0, 3);
        // Even a tink shakes a few leaves loose (js: resolveCombat's wrong-tool arm).
        if let Ok((node, _)) = nodes.get(tink.target) {
            shed_leaves(&mut commands, &mut rng, node.kind, node.c, node.r);
        }
        // A too-weak pick/axe on a high-tier vein/tree says so (js resistTool toast).
        if let Some(note) = tink.note {
            log.add("resist", note, 1, 0xfc8868, false, true);
        }
    }
}

/// Wobble a struck node around its resting x (js: draw at x + sin(hit * 1.7) * 2).
fn apply_shake(mut commands: Commands, mut q: Query<(Entity, &mut Shake, &mut Transform)>) {
    for (e, mut s, mut tf) in &mut q {
        if s.t == 0 {
            tf.translation.x = s.base_x;
            commands.entity(e).remove::<Shake>();
            continue;
        }
        tf.translation.x = s.base_x + ((s.t as f32) * 1.7).sin().round() * 2.0;
        s.t -= 1;
    }
}

/// A broken cracked wall's context (node_deaths sits at the 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub struct CrackBreak<'w> {
    pub caves: ResMut<'w, super::caves::CrackCaves>,
    pub world: Res<'w, super::play::GameWorld>,
    pub log: ResMut<'w, super::rewards::LootLog>,
    pub saves: MessageWriter<'w, super::save::SaveRequest>,
    pub sfx: MessageWriter<'w, super::sfx::Sfx>,
}

/// Fallen nodes: chips + drops + persistence, then gone (a felled tree reappears as a
/// stump on the next room entry via its TreeGrowth stamp).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn node_deaths(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut rng: ResMut<GameRng>,
    clock: Res<FrameClock>,
    tstats: Res<TreeStats>,
    cur: Res<CurRoom>,
    mut blockers: ResMut<RoomBlockers>,
    mut gathered: ResMut<GatherState>,
    mut growth: ResMut<TreeGrowth>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut art: ResMut<crate::actors::props::PropArt>,
    mut crack: CrackBreak,
    q: Query<(Entity, &GatherNode, &Health)>,
) {
    for (e, node, h) in &q {
        if h.hp > 0 {
            continue;
        }
        let (x, y) = ((node.c * 16) as f32, (node.r * 16) as f32);
        if node.kind == "crackedrock" {
            // The wall gives: a cave door is carved here FOREVER (js crackCaves) —
            // no drops, no daily regrow record; the door record supersedes the crack.
            spawn_burst(&mut commands, &mut rng, Vec2::new(x + 8.0, y + 8.0), 0xa8a8a8, 8);
            let dest = super::caves::crack_broken(
                &mut commands, &mut images, &mut crack.caves, crack.world.0.seed, (cur.rx, cur.ry), node.c, node.r,
            );
            let line = match dest {
                "shop" => "A LANTERN GLOWS IN THE DARK BEYOND",
                "mini" => "SOMETHING GROWLS IN THE HOLLOW BEYOND",
                _ => "COLD AIR SPILLS FROM THE DEPTHS",
            };
            crack.log.add("cave", line, 1, 0xc8b088, false, true);
            crack.sfx.write(super::sfx::Sfx("open"));
            crack.saves.write(super::save::SaveRequest);
            commands.entity(e).despawn();
            continue;
        }
        spawn_burst(&mut commands, &mut rng, Vec2::new(x + 8.0, y + 8.0), chip_color(node.kind), 8);
        let mut drops = drops_for(node.kind, node.tier, crack.world.0.biome_key_at(cur.rx, cur.ry), &mut rng);
        // HARVEST YIELD: each drop has a tree-granted chance of a bonus copy (js gatherBonus).
        if tstats.gather > 0.0 {
            let bonus: Vec<&'static str> =
                drops.iter().filter(|_| rng.0.next_f64() < tstats.gather).copied().collect();
            drops.extend(bonus);
        }
        for id in drops {
            let (dx, dy) = (2.0 + rng.0.next_f64() as f32 * 10.0, 2.0 + rng.0.next_f64() as f32 * 10.0);
            spawn_pickup(&mut commands, &mut images, id, 1, x + dx, y + dy, true);
        }
        if let Some(b) = node.blocker {
            blockers.0.retain(|r| *r != b);
        }
        // The ledger (js bump sites: trees felled / stones broken / grass mown).
        match node.kind {
            "boulder" | "stalagmite" | "crystalspire" => stats.bump("stones", 1.0),
            "grass" => stats.bump("grass", 1.0),
            _ if node.tree => stats.bump("trees", 1.0),
            _ => {}
        }
        let room = (cur.rx, cur.ry);
        if node.tree {
            growth.0.entry(room).or_default().insert((node.c, node.r), farm_day(clock.0));
            // The stump appears the moment the tree falls (same sprite + anchor the room
            // rebuild uses; it re-materialises from the growth stamp on the next entry).
            let img = art.stage(node.kind, 0, &mut images);
            commands.spawn((
                Sprite::from_image(img),
                at(PLAY_X + x - 16.0, PLAY_Y + y - 56.0, 48.0, 72.0, actor_z(y + 16.0)),
                PIXEL_LAYER,
                RoomActor,
            ));
        } else {
            let today = farm_day(clock.0); // gatherables regrow at DAWN with the rest of the world
            let rec = gathered.rooms.entry(room).or_insert_with(|| (today, HashSet::default()));
            if rec.0 != today {
                *rec = (today, HashSet::default()); // yesterday's record expired
            }
            rec.1.insert((node.c, node.r));
        }
        commands.entity(e).despawn();
    }
}

/// Pickups pop up on spawn, bob, magnet to the player (only while they can be taken —
/// js: a drop rests on the ground instead of clinging to a full-bagged player; coins
/// always fit), blink when about to despawn, and bank on touch — coins into the purse
/// (scaled by the tree's GOLD stat), items into the bag with the auto-equip courtesy;
/// both toast in the loot feed. Their ground glows tag along and are reaped with them.
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // ECS system params are wide by nature
fn pickups_tick(
    mut commands: Commands,
    clock: Res<FrameClock>,
    tstats: Res<TreeStats>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut log: ResMut<crate::app::rewards::LootLog>,
    mut stats: ResMut<crate::app::stats::Stats>,
    mut gathered: ResMut<GatherState>,
    cur: Res<super::play::CurRoom>,
    discovered: Res<super::codex::items_tab::Discovered>,
    mut fanfare: ResMut<super::fanfare::Fanfare>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    players: Query<&Player>,
    mut q: Query<(Entity, &mut Pickup, &mut Transform, &Sprite, &mut Visibility)>,
    mut glows: Query<(Entity, &PickupGlow, &mut Transform), Without<Pickup>>,
) {
    let cur_room = (cur.rx, cur.ry);
    let Ok(p) = players.single() else { return };
    for (e, mut pk, mut tf, sprite, mut vis) in &mut q {
        if pk.tile.is_none() && !matches!(pk.kind, PickupKind::Book(_)) {
            pk.life = pk.life.saturating_sub(1); // placed items + tomes sit until taken
        }
        let (dx, dy) = (p.x + 8.0 - pk.x, p.y + 8.0 - pk.y);
        let dist = dx.hypot(dy);
        let takeable = match pk.kind {
            PickupKind::Coin(_) => true,
            PickupKind::Book(_) => false, // books are PRESS-to-take (prompts.rs)
            PickupKind::Item { id, .. } => inv.can_add(id),
        };
        if dist < 10.0 && takeable {
            // js collectPickup: the toast + coin-stat scaling + the auto-equip courtesy.
            match pk.kind {
                PickupKind::Coin(v) => {
                    let add = ((v as f64 * (1.0 + tstats.coin)) + 0.5).floor() as i64;
                    inv.money += add;
                    stats.bump("coins", add as f64); // js bump('coins') — the ledger
                    log.add("coin", "COPPER", add as i32, 0xfcd000, true, false);
                    sfx.write(super::sfx::Sfx("coin"));
                }
                PickupKind::Book(id) => {
                    gathered.tomes.insert(id);
                    let title = crate::lore_books::get(id).map_or(id, |b| b.title);
                    log.add("tome", &format!("NEW TOME: {title}"), 1, 0xd8b8ff, false, true);
                }
                PickupKind::Item { id, qty } => {
                    inv.add_item(id, qty);
                    let slotted = inv.auto_equip(id); // an EMPTY matching slot claims it
                    let name = crate::items::get(id).map_or(id, |d| d.name);
                    let label = format!("{}{}", name.to_uppercase(), if slotted { " (EQUIPPED)" } else { "" });
                    log.add(id, &label, qty, crate::app::rewards::toast_color(id), false, false);
                    // The "got it!" cutscene — keys every time, all else on first discovery
                    // (Discovered is stamped by a separate watcher, so it still holds the
                    // PRE-pickup set here).
                    if super::fanfare::should_play(id, &discovered) {
                        super::fanfare::begin(&mut fanfare, id); // fanfare fires "itemget" itself
                    } else {
                        sfx.write(super::sfx::Sfx("pickup")); // repeat pickups just chirp
                    }
                }
            }
            // A placed item is once-ever: record its tile permanently (saved).
            if let Some((c, r)) = pk.tile {
                gathered.placed.entry(cur_room).or_default().insert((c, r));
            }
            commands.entity(e).despawn();
            continue;
        }
        if pk.t < 9 {
            // The spawn pop: a little upward arc before it settles (js pickup()).
            pk.y += pk.vy;
            pk.vy += 0.22;
            pk.t += 1;
        } else if pk.magnet && takeable {
            // js magnetToPlayer: R = 40 + the tree's magnet stat; the pull eases up to 2.6
            // as it closes.
            let r = 40.0 + tstats.magnet as f32;
            if dist > 0.5 && dist < r {
                let pull = ((r - dist) / 12.0 + 0.5).min(2.6);
                pk.x += dx / dist * pull;
                pk.y += dy / dist * pull;
            }
        }
        if pk.life == 0 {
            commands.entity(e).despawn();
            continue;
        }
        // Blink before despawning (js: skip the sprite draw — the glow stays lit).
        *vis = if pk.life < 90 && (pk.life >> 3) % 2 == 0 { Visibility::Hidden } else { Visibility::Inherited };
        let bob = (((clock.0 as f32) + pk.x) / 16.0).sin().round();
        let s = sprite.custom_size.map_or(8.0, |v| v.x);
        *tf = at(
            PLAY_X + pk.x + 2.0 - s / 2.0,
            PLAY_Y + pk.y + 2.0 - s / 2.0 - bob,
            s,
            s,
            actor_z(pk.y + 6.0) + ICON_Z_OFF,
        );
    }
    // Glows shadow their pickup (no bob — the js light sits at the entity, not the icon).
    for (ge, glow, mut gtf) in &mut glows {
        if let Ok((_, pk, ..)) = q.get(glow.0) {
            *gtf = at(
                PLAY_X + pk.x + 2.0 - GLOW_R,
                PLAY_Y + pk.y + 2.0 - GLOW_R,
                GLOW_R * 2.0,
                GLOW_R * 2.0,
                actor_z(pk.y + 6.0) + GLOW_Z_OFF,
            );
        } else {
            commands.entity(ge).despawn();
        }
    }
}
