//! room_props.rs — turn a room's [`RoomEntity`] layout into sprites + collision:
//! big seeded trees/cacti (y-sorted into the actor depth band), bushes/boulders/flowers/
//! clutter on the fixed under-layer, swaying tall grass, and the [`RoomBlockers`] list the
//! movement code collides with.
//!
//! Deltas (each arrives with its system): interactive props (shop/tradewagon/chest/
//! songstone/crackedrock/saltmaze door), set-piece structures (castle/dungeon/rift, fences,
//! braziers, wisps, guards), swamp reeds/lilypads, biome grass tints, gathering/chopping.

use super::farm::{spawn_farm_layer, spawn_wildcrops, FarmTiles, GroundVeg};
use super::gather::{farm_day, GatherNode, GatherState, TreeGrowth, TREE_GROW_DAYS};
use super::room_render::{actor_z, child, PLAY_X, PLAY_Y};
use crate::actors::props::{pick_variant, prop_anchor, PropArt};
use crate::combat::{Blood, Combatant, GatherTool, Health, Hitbox, HurtProfile, Team, Tool};
use crate::gfx::at;
use crate::room::{RoomGrid, TILE};
use crate::worldgen::{RoomEntity, World};
use bevy::prelude::*;

/// Solid prop hitboxes in room px — rebuilt with each room. Movement may leave a box it
/// already overlaps (a room-entry landing inside a bush isn't a trap) but never enter one.
#[derive(Resource, Default)]
pub struct RoomBlockers(pub Vec<(f32, f32, f32, f32)>);

impl RoomBlockers {
    /// Would the move from `from` to `to` (boxes in room px) enter a blocker?
    pub fn blocks(&self, from: (f32, f32, f32, f32), to: (f32, f32, f32, f32)) -> bool {
        crate::room::blockers_block(&self.0, from, to)
    }
}

/// Tall grass sways on the shared clock (port of the grass draw's phase math).
#[derive(Component)]
pub struct GrassSway {
    pub x: i32,
    pub y: i32,
}

/// The big-prop kinds drawn with the seeded tree pipeline (everything in the js PROPS
/// table that spawns in the wild; unported exotics fall back to the oak silhouette).
pub(crate) fn is_big_prop(kind: &str) -> bool {
    matches!(
        kind,
        "oak" | "pine" | "cactus" | "deadtree" | "blossom" | "riftbulb" | "voidspire" | "mawtree"
            | "shroom" | "burnttree" | "jungletree" | "giantflower" | "bluebloom"
            | "crystalspire" | "stalagmite"
    )
}

/// The combat side of a gatherable node: health + the tool that bites + chip colour.
#[allow(clippy::too_many_arguments)] // (identity, tool, hp, boxes, chips, tier gate) IS the node's arity
fn node_bundle(
    kind: &'static str,
    c: i32,
    r: i32,
    tool: Tool,
    hp: i32,
    hitbox: Hitbox,
    blocker: Option<(f32, f32, f32, f32)>,
    chips: u32,
    tree: bool,
    tier: i32,
    req_tier: i32,
) -> impl Bundle {
    (
        GatherNode { kind, c, r, blocker, tree, tier },
        GatherTool(tool, req_tier),
        Combatant { team: Team::Object, hurt_team: None, damage: None, persistent: false, knock: 0.0 },
        Health { hp, max: hp, defense: 0, invuln: 0, flash: 0 },
        HurtProfile { invuln: 0, flash: 0, kb_base: 0.0, kb_frames: 0 },
        Blood(chips),
        hitbox,
    )
}

/// Spawn a room's props as children of `root`; returns the solid hitboxes for
/// [`RoomBlockers`]. Gathered nodes stay gone for the day; felled trees come back at their
/// growth stage. Mobs in the list are the battle module's business, not ours.
#[allow(clippy::too_many_arguments)] // room composition needs the room's whole context
pub fn spawn_room_props(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    art: &mut PropArt,
    world: &World,
    grid: &RoomGrid,
    ents: &[RoomEntity],
    root: Entity,
    gather: &GatherState,
    growth: &mut TreeGrowth,
    farm: &FarmTiles,
    cleared: &super::encounters::ClearedEncounters,
    caves: &super::caves::CrackCaves,
    songs_opened: &super::caves::OpenedSongstones,
    clock: i64,
    room: (i32, i32),
) -> Vec<(f32, f32, f32, f32)> {
    let today = farm_day(clock); // the dawn day — must match the harvest stamp (gather.rs)
    let fday = farm_day(clock);
    let ztier = World::zone_tier(room.0, room.1); // js harvestTier: richer mats + tougher gates deeper
    // The HARVEST GATE (can this axe/pick cut it) keys off the BIOME, not the zone tier — a
    // biome region straddles tier rings, so a tier-gated tree was cuttable in one room and
    // "needs a stronger axe" in the identical next room (Baz). Biome tier is uniform per region.
    let btier = world.biome_at(room.0, room.1).tier;
    let mut blockers = Vec::new();
    // The burnt village is hand-built INSTEAD of its natural contents (js game.js:1036
    // spawns from an empty list there) — no trees, no bushes, only the ruin.
    if room == HOME_VILLAGE {
        spawn_ruined_village(commands, images, art, root, &mut blockers, gather);
        return blockers;
    }
    for e in ents {
        let (x, y) = (e.x, e.y);
        let (c, r) = (x.div_euclid(TILE), y.div_euclid(TILE));
        let fx = x as f32;
        let fy = y as f32;
        let gatherable = is_big_prop(e.kind) || matches!(e.kind, "bush" | "boulder" | "grass");
        if gatherable && gather.taken(room, c, r, today) {
            continue; // gathered earlier today — regrows on the next day's first entry
        }
        // Nothing natural regrows through hoed soil (js loadRoomEntities' VEG skip).
        let natural = gatherable || matches!(e.kind, "flower" | "clutter" | "mushroom" | "reed" | "lilypad");
        if natural && farm.tile(room, c, r).is_some() {
            continue;
        }
        match e.kind {
            k if is_big_prop(k) => {
                let (ox, oy, hx, hy, hw, hh) = prop_anchor(k);
                // A day-stamped (felled) tree comes back through its growth stages.
                let stage = growth.0.get(&room).and_then(|m| m.get(&(c, r))).map(|cut| fday - cut);
                if let Some(stage) = stage {
                    if stage < TREE_GROW_DAYS {
                        let img = art.stage(k, stage.max(0) as u8, images);
                        let tf = at(PLAY_X + (fx - 16.0), PLAY_Y + (fy - 56.0), 48.0, 72.0, actor_z(fy + TILE as f32));
                        let pe = child(commands, root, Sprite::from_image(img), tf);
                        // Stumps/saplings cast too — same feet anchor as the grown
                        // tree, silhouette from the live stage art.
                        commands.entity(pe).insert(super::shadows::CastsShadow {
                            left: fx + 1.0,
                            top: fy + 12.0,
                            w: 14,
                            a: 0.9,
                        });
                        if stage >= 2 {
                            // Only the YOUNG tree blocks; stump/sapling walk over.
                            blockers.push(((x + hx) as f32, (y + hy) as f32, hw as f32, hh as f32));
                        }
                        continue; // stages aren't harvestable (no wood until full-grown)
                    }
                    growth.0.get_mut(&room).map(|m| m.remove(&(c, r))); // full-grown again
                }
                let img = art.tree(k, x, y, images);
                let size = images.get(&img).map(|i| i.size().as_vec2()).unwrap_or(Vec2::new(48.0, 72.0));
                let tf = at(
                    PLAY_X + (x + ox) as f32,
                    PLAY_Y + (y + oy) as f32,
                    size.x,
                    size.y,
                    actor_z(fy + TILE as f32), // depth-sorted by the foot tile, like every actor
                );
                let hb = Hitbox { x: (x + hx) as f32, y: (y + hy) as f32, w: hw as f32, h: hh as f32 };
                let blocker = ((x + hx) as f32, (y + hy) as f32, hw as f32, hh as f32);
                let (tool, hp, tree) = match k {
                    "cactus" => (Tool::Sword, 6, false),
                    "crystalspire" | "stalagmite" => (Tool::Pick, 10, false),
                    _ => (Tool::Axe, 12, true),
                };
                let chips = match k {
                    "crystalspire" => 0x9fe8ff,
                    "stalagmite" => 0xc0c0cc,
                    "cactus" => 0x3a8a2a,
                    _ => 0x8a5a2a,
                };
                let pe = child(commands, root, Sprite::from_image(img), tf);
                // Trees + crystal/stalagmite gate on the BIOME tier (uniform per region); a
                // cactus doesn't. The DROP tier stays `ztier` so deeper still pays better.
                let req = if k == "cactus" { 0 } else { btier.clamp(1, 6) };
                commands.entity(pe).insert(node_bundle(k, c, r, tool, hp, hb, Some(blocker), chips, tree, ztier, req));
                blockers.push(blocker);
            }
            "bush" => {
                let img = art.bushes[pick_variant(x, y, 0x11, art.bushes.len())].clone();
                let pe = child(commands, root, Sprite::from_image(img), at(PLAY_X + fx, PLAY_Y + fy, 16.0, 16.0, 3.4));
                let blocker = (fx + 2.0, fy + 2.0, 12.0, 12.0);
                let hb = Hitbox { x: fx + 2.0, y: fy + 2.0, w: 12.0, h: 12.0 };
                commands.entity(pe).insert(node_bundle("bush", c, r, Tool::Sword, 2, hb, Some(blocker), 0x3a8a2a, false, ztier, 0));
                blockers.push(blocker);
            }
            "boulder" => {
                // Ore node: the rock advertises its metal by zone tier — the vein art climbs
                // copper -> mithril (the baked 0..=4 sets), and a deeper vein needs a stronger
                // pick (req_tier), which is exactly the metal it yields.
                let rq = (ztier.max(1) - 1).clamp(0, 4) as usize;
                let pool = &art.boulders[rq];
                let img = pool[pick_variant(x, y, 0x22 + 1 + rq as i32, pool.len())].clone();
                let pe = child(commands, root, Sprite::from_image(img), at(PLAY_X + fx, PLAY_Y + fy, 16.0, 16.0, 3.4));
                let blocker = (fx + 2.0, fy + 3.0, 12.0, 11.0);
                let hb = Hitbox { x: fx + 2.0, y: fy + 3.0, w: 12.0, h: 11.0 };
                // Gate on biome tier (uniform per region); drop tier stays ztier (deeper = better).
                commands.entity(pe).insert(node_bundle("boulder", c, r, Tool::Pick, 3, hb, Some(blocker), 0xa8a8a8, false, ztier, btier.max(1)));
                blockers.push(blocker);
            }
            "grass" => {
                // Spawn ON the live sway phase — frame 0 would freeze the new room's
                // field out of step with the old room still sliding past.
                let img = art.grass[grass_phase(x, y, clock) % art.grass.len()].clone();
                let ge = child(commands, root, Sprite::from_image(img), at(PLAY_X + fx, PLAY_Y + fy, 16.0, 16.0, 3.2));
                let hb = Hitbox { x: fx + 2.0, y: fy + 3.0, w: 12.0, h: 12.0 };
                commands.entity(ge).insert((GrassSway { x, y }, node_bundle("grass", c, r, Tool::Sword, 1, hb, None, 0x3a8a2a, false, ztier, 0)));
            }
            "flower" => {
                let img = art.flowers[pick_variant(x, y, 0x44, art.flowers.len())].clone();
                let fe = child(commands, root, Sprite::from_image(img), at(PLAY_X + fx, PLAY_Y + fy, 16.0, 16.0, 3.0));
                commands.entity(fe).insert(GroundVeg { c, r }); // a hoe clears it off fresh soil
            }
            // --- Town dressing (townEntities): storefronts, the well, braziers, folk. ---
            "town" => {
                // js townBuilding: 48x48 front anchored (x-16, y-32), depth-sorted at y+16;
                // the whole mass blocks (the doorway opens with the interiors port).
                if let Some(img) = art.fronts.get(e.sub.as_str()) {
                    let tf = at(PLAY_X + fx - 16.0, PLAY_Y + fy - 32.0, 48.0, 48.0, actor_z(fy + 16.0));
                    child(commands, root, Sprite::from_image(img.clone()), tf);
                    blockers.push((fx - 12.0, fy - 28.0, 40.0, 42.0));
                }
            }
            // A lone roadside SHOP (worldgen "rare timber storefront"): it had no overworld
            // sprite, so all you saw was its lantern glow + the F ENTER prompt (Baz: "what am
            // I entering lol"). Render the store front + block it, like a town storefront.
            "shop" => {
                if let Some(img) = art.fronts.get("store") {
                    let tf = at(PLAY_X + fx - 16.0, PLAY_Y + fy - 32.0, 48.0, 48.0, actor_z(fy + 16.0));
                    child(commands, root, Sprite::from_image(img.clone()), tf);
                    blockers.push((fx - 12.0, fy - 28.0, 40.0, 42.0));
                }
            }
            "crackedrock" => {
                // A fissured wall section (js): sits ON the border wall tile (the wall
                // already blocks, so no blocker). Bomb it or pick it open; once broken
                // its cave door is recorded forever — the door spawns instead (caves.rs).
                if caves.opened(room, c, r) {
                    super::caves::spawn_cave_door(commands, images, root, caves, room, c, r);
                    continue;
                }
                let img = images.add(crate::gfx::bake(super::caves::CRACK_ART, &[('A', 0x8a8a92)]));
                let pe = child(commands, root, Sprite::from_image(img), at(PLAY_X + fx, PLAY_Y + fy, 16.0, 16.0, 3.35));
                let hb = Hitbox { x: fx + 1.0, y: fy + 1.0, w: 14.0, h: 14.0 };
                commands.entity(pe).insert(node_bundle("crackedrock", c, r, Tool::Pick, 4, hb, None, 0xa8a8a8, false, ztier, 0));
            }
            "saltmaze" => {
                // The Choir sanctum's half-buried door (js): no map pin — the books
                // are the map. Press at the mouth to descend (dungeon.rs).
                let img = images.add(super::saltmaze::salt_door_image());
                let pe = child(commands, root, Sprite::from_image(img), at(PLAY_X + fx - 16.0, PLAY_Y + fy - 24.0, 48.0, 40.0, actor_z(fy + 16.0)));
                commands.entity(pe).insert(super::saltmaze::SaltDoor { x: fx, y: fy });
                blockers.push((fx - 14.0, fy - 20.0, 44.0, 34.0));
            }
            "songstone" => {
                // The singing stone (js): sung open once -> its door forever after.
                if songs_opened.0.contains(&format!("{},{}", room.0, room.1)) {
                    super::caves::spawn_song_door(commands, images, fx, fy, e.sub.clone());
                    continue;
                }
                let img = images.add(crate::gfx::bake(super::caves::SONGSTONE_ART, super::caves::SONGSTONE_PAL));
                let pe = child(commands, root, Sprite::from_image(img), at(PLAY_X + fx, PLAY_Y + fy - 8.0, 16.0, 24.0, actor_z(fy + 16.0)));
                commands.entity(pe).insert(super::caves::Songstone { x: fx, y: fy, dest: e.sub.clone() });
                blockers.push((fx + 2.0, fy + 6.0, 12.0, 10.0));
            }
            "guildhall" => {} // the boarded-up GUILDHALL — its own port (restoration arc)
            "stallspot" => {} // open ground until the Tillers' produce stall returns
            "well" => {
                let size = images.get(&art.well).map(|i| i.size().as_vec2()).unwrap_or(Vec2::new(20.0, 25.0));
                let tf = at(PLAY_X + fx - 2.0, PLAY_Y + fy - 8.0, size.x, size.y, actor_z(fy + 16.0));
                child(commands, root, Sprite::from_image(art.well.clone()), tf);
                blockers.push((fx, fy + 4.0, 16.0, 12.0));
            }
            "torch" => {
                let size = images.get(&art.torch[0]).map(|i| i.size().as_vec2()).unwrap_or(Vec2::new(12.0, 16.0));
                let tf = at(PLAY_X + fx + 2.0, PLAY_Y + fy, size.x, size.y, actor_z(fy + 14.0));
                let te = child(commands, root, Sprite::from_image(art.torch[0].clone()), tf);
                commands.entity(te).insert(TorchAnim([art.torch[0].clone(), art.torch[1].clone()]));
                blockers.push((fx + 5.0, fy + 10.0, 6.0, 6.0));
            }
            "npc" => {
                // A villager: the sprite bank keys off their identity seed; sync_villagers
                // dresses the sprite each frame (no image needed at spawn). A NAMED person
                // (js pkey "rx,ry:seed") the relationship ledger can track.
                let ve = child(commands, root, Sprite::default(), at(PLAY_X + fx, PLAY_Y + fy, 16.0, 16.0, actor_z(fy + 16.0)));
                let mut v = crate::actors::villager::Villager::new(fx, fy, e.seed, e.sub.clone());
                v.identify(format!("{},{}:{}", room.0, room.1, e.seed), crate::people::name_for(e.seed).to_string());
                v.stagger();
                commands.entity(ve).insert(v);
            }
            "castle" => {
                // THE BLACK CASTLE (js Entities.castle): the facade sprite spawns bare;
                // dress_castle (app/dungeon.rs) bakes the gate state (sockets / doors /
                // rift bloom) from the live shard count and keeps it current.
                let me = child(commands, root, Sprite::default(), at(PLAY_X + fx - 96.0, PLAY_Y + fy - 112.0, 192.0, 144.0, actor_z(fy + TILE as f32)));
                commands.entity(me).insert(super::dungeon::CastleGate { x: fx, y: fy, baked: None });
                // Solid mass: towers + walls; the gate is handled by dress_castle (top
                // half only when open, whole arch when sealed).
                blockers.push((fx - 96.0, fy - 112.0, 80.0, 130.0));
                blockers.push((fx + 16.0, fy - 112.0, 80.0, 130.0));
            }
            "dungeon" => {
                // The land's SHARD MONUMENT (js dungeonEntrance): a 64x56 archetype
                // silhouette in the land's palette, mouth south, anchored so the mouth
                // floor sits on the entity tile. Eyes glow in the shard colour (the
                // breathing pulse + press-to-enter live in app/dungeon.rs).
                let ea = crate::actors::entrance_art::entrance(e.sub.as_str());
                let img = images.add(crate::gfx::bake(ea.grid, ea.pal));
                let tf = at(PLAY_X + fx - 24.0, PLAY_Y + fy - 40.0, 64.0, 56.0, actor_z(fy + TILE as f32));
                let me = child(commands, root, Sprite::from_image(img), tf);
                commands.entity(me).insert(super::dungeon::DungeonEntrance {
                    biome: e.sub.clone(),
                    x: fx,
                    y: fy,
                });
                super::dungeon::spawn_eyes(commands, me, ea);
                blockers.push((fx - 20.0, fy - 36.0, 56.0, 48.0)); // the monument's solid mass
            }
            "rift" => {
                // THE RIFT SPIRE (js riftSpire, painted layer for layer — riftspire.rs);
                // press-to-enter + the endless descent live in app/dungeon.rs.
                super::riftspire::spawn(commands, images, root, fx, fy);
                blockers.push((fx + 8.0 - 34.0, fy - 14.0, 68.0, 28.0)); // the js hitbox: the tower's ground mass
            }
            "clutter" => {
                if let Some(img) = art.clutter.get(e.sub.as_str()) {
                    let ce = child(commands, root, Sprite::from_image(img.clone()), at(PLAY_X + fx, PLAY_Y + fy, 16.0, 16.0, 3.0));
                    commands.entity(ce).insert(GroundVeg { c, r });
                }
            }
            "mushroom" => {
                // Swamp glow-mushroom: the toadstool clutter art stands in until its own port.
                if let Some(img) = art.clutter.get("toadstool") {
                    let ce = child(commands, root, Sprite::from_image(img.clone()), at(PLAY_X + fx, PLAY_Y + fy, 16.0, 16.0, 3.0));
                    commands.entity(ce).insert(GroundVeg { c, r });
                }
            }
            _ => {} // mobs -> battle.rs; interactive/set-piece props -> their own ports
        }
    }
    // An un-beaten encounter dresses its scene over the natural room (decor rebuilds
    // identically every visit; the foes are spawn_room_mobs' business).
    if !cleared.0.contains(&room)
        && let Some((def, _)) = super::encounters::for_room(world, room.0, room.1)
    {
        let scene = super::encounters::build(def, world, room.0, room.1);
        super::encounters::spawn_decor(commands, images, art, root, &scene, &mut blockers);
        super::encounters::spawn_wanderers(commands, root, &scene);
    }
    // The farm layer rides the root with the props: tilled beds + growing crops, then
    // the day's seasonal wild forage (js Farm.draw under the entities + spawnWildCrops).
    spawn_farm_layer(commands, images, root, farm, room, today);
    spawn_wildcrops(commands, images, root, world, grid, farm, gather, room, clock);
    blockers
}

const SWAY_TICKS: i64 = 64; // js SWAY_TICKS — frames per sway step

/// A tuft's sway frame: the shared clock plus a per-tile offset, so a field ripples like
/// wind (the js grass draw's phase formula, verbatim).
fn grass_phase(x: i32, y: i32, clock: i64) -> usize {
    ((clock + ((x + y) >> 2) as i64) / SWAY_TICKS) as usize
}

/// The hero's home village — one room EAST of spawn (js isHomeVillage: startRX+1).
/// Burned in the opening; a permanent ruin, never a living town.
pub const HOME_VILLAGE: (i32, i32) = (1, 0);

/// The fixed ruin scene over the village's normal terrain (js buildRuinedVillage):
/// scorched earth, five gutted shells, the dead in the streets, smouldering wreckage.
/// (The lore-book fragments + EMBERFALL banner arrive with their own systems.)
fn spawn_ruined_village(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    art: &mut PropArt,
    root: Entity,
    blockers: &mut Vec<(f32, f32, f32, f32)>,
    gather: &GatherState,
) {
    // Scorched earth: soft radial burn decals, UNDER everything that stands.
    let scorch = images.add(scorch_image());
    for (sx, sy, r) in [(80, 62, 62.0), (232, 70, 64.0), (70, 152, 60.0), (240, 150, 64.0), (150, 108, 88.0), (150, 184, 54.0)] {
        let (sx, sy) = (sx as f32, sy as f32);
        commands.entity(root).with_children(|p| {
            p.spawn((
                Sprite { image: scorch.clone(), custom_size: Some(Vec2::new(r * 2.0, r * 2.0)), ..default() },
                at(PLAY_X + sx - r, PLAY_Y + sy - r, r * 2.0, r * 2.0, 2.5),
                crate::gfx::PIXEL_LAYER,
            ));
        });
    }
    // The gutted shells (corners + the hero's own home); only the rubble mound blocks.
    for (x, y, grid) in crate::actors::buildings_art::RUINS {
        let img = images.add(crate::gfx::bake(grid, &[]));
        let size = images.get(&img).map(|i| i.size().as_vec2()).unwrap_or(Vec2::new(40.0, 26.0));
        let (fx, fy) = (*x as f32, *y as f32);
        let tf = at(PLAY_X + fx - 20.0, PLAY_Y + fy - 8.0, size.x, size.y, actor_z(fy + 16.0));
        child(commands, root, Sprite::from_image(img), tf);
        blockers.push((fx - 14.0, fy + 2.0, 28.0, 12.0));
    }
    // The dead, sprawled in the streets (walkable ground decals).
    let skel = images.add(crate::gfx::bake(
        crate::actors::buildings_art::SKELETON.0,
        crate::actors::buildings_art::SKELETON.1,
    ));
    for (cx, cy) in [(96, 92), (188, 104), (120, 138), (214, 146), (78, 118), (176, 80), (206, 96), (228, 132)] {
        let tf = at(PLAY_X + cx as f32, PLAY_Y + cy as f32, 16.0, 16.0, 2.8);
        child(commands, root, Sprite::from_image(skel.clone()), tf);
    }
    // A handful of loose stones in the rubble — free pickings for a fresh hero. They
    // wait until taken (no despawn), then stay gone for good (the permanent record).
    for (sx, sy) in [(90.0, 74.0), (208.0, 64.0), (66.0, 126.0), (232.0, 98.0), (126.0, 160.0)] {
        let (c, r) = ((sx as i32).div_euclid(TILE), (sy as i32).div_euclid(TILE));
        if !gather.placed_taken(HOME_VILLAGE, c, r) {
            super::gather::spawn_placed_item(commands, images, "stone", sx, sy, Some(root));
        }
    }

    // THE SURVIVOR — the first-hour thread's opening voice (story.rs). She stands
    // in the ashes at every step; her words and her '!' follow StoryThread.
    let ve = child(
        commands,
        root,
        Sprite::default(), // sync_villagers dresses her from the sprite bank
        at(PLAY_X + 150.0, PLAY_Y + 92.0, 16.0, 16.0, actor_z(92.0 + 16.0)),
    );
    let seed = super::story::SURVIVOR_SEED;
    let mut v = crate::actors::villager::Villager::new(150.0, 92.0, seed, String::new());
    // A REAL person (WREN): the pkey makes her talkable (talk_tick skips key-less
    // villagers) and joins her to the relationship ledger like anyone else.
    v.identify(format!("{},{}:{}", HOME_VILLAGE.0, HOME_VILLAGE.1, seed), crate::people::name_for(seed).to_string());
    v.hold_post(); // she keeps her vigil — no wandering through the dead
    commands.entity(ve).insert((v, super::story::StorySurvivor));

    // The Emberfall fragments lie where they fell (js authored spots; read ones stay
    // gone): the survivor's plea by the hero's ruin, the slate by the inn, the miller's
    // letter by the bakery, and the Lantern Order's roll at the centre.
    for (id, bx, by) in [
        ("lastnight", 158.0, 196.0),
        ("emberslate", 66.0, 82.0),
        ("embermiller", 232.0, 178.0),
        ("emberroll", 146.0, 122.0),
    ] {
        if !gather.tomes.contains(id) {
            super::gather::spawn_book(commands, images, id, bx, by, Some(root));
        }
    }

    // Smouldering wreckage.
    for (kind, wx, wy) in [
        ("charredlog", 60, 100), ("ashpile", 110, 84), ("charcoal", 170, 140), ("embers", 142, 150),
        ("bones", 132, 110), ("charredlog", 206, 128), ("ashpile", 86, 138), ("embers", 244, 108),
        ("bones", 104, 160), ("charcoal", 196, 86), ("ashpile", 124, 64),
    ] {
        if let Some(img) = art.clutter.get(kind) {
            child(commands, root, Sprite::from_image(img.clone()), at(PLAY_X + wx as f32, PLAY_Y + wy as f32, 16.0, 16.0, 3.0));
        }
    }
}

/// The js scorch decal: a radial burn (near-black core fading out), baked once and
/// stretched per radius. Alphas sit above the js stops (linear-blend compensation).
fn scorch_image() -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let s = 96u32;
    let c = s as f32 / 2.0;
    let mut buf = vec![0u8; (s * s * 4) as usize];
    for y in 0..s {
        for x in 0..s {
            let d = ((x as f32 + 0.5 - c).hypot(y as f32 + 0.5 - c)) / c;
            if d > 1.0 {
                continue;
            }
            // js stops: 0 -> a .85 (10,8,6); .7 -> a .5 (20,14,10); 1 -> 0. Alphas ride
            // WELL above the js numbers: Bevy blends in linear space, where a dark wash
            // reads much thinner than the canvas (side-by-side calibrated).
            let (rgb, a) = if d < 0.7 {
                let t = d / 0.7;
                ((10.0 + 10.0 * t, 8.0 + 6.0 * t, 6.0 + 4.0 * t), 0.96 - 0.22 * t)
            } else {
                ((20.0, 14.0, 10.0), 0.74 * (1.0 - (d - 0.7) / 0.3))
            };
            let i = ((y * s + x) * 4) as usize;
            buf[i] = rgb.0 as u8;
            buf[i + 1] = rgb.1 as u8;
            buf[i + 2] = rgb.2 as u8;
            buf[i + 3] = (a * 255.0) as u8;
        }
    }
    Image::new(
        Extent3d { width: s, height: s, depth_or_array_layers: 1 },
        TextureDimension::D2,
        buf,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// The brazier's two-frame flicker (js: TORCH_FRAMES[(clock >> 3) & 1]).
#[derive(Component)]
pub struct TorchAnim(pub [Handle<Image>; 2]);

pub fn animate_torches(clock: Res<super::room_render::FrameClock>, mut q: Query<(&TorchAnim, &mut Sprite)>) {
    let f = ((clock.0 >> 3) & 1) as usize;
    for (t, mut sprite) in &mut q {
        if sprite.image != t.0[f] {
            sprite.image = t.0[f].clone();
        }
    }
}

/// Sway the tall grass on the shared clock (port of the grass entity's draw phase).
/// Each tuft crosses its OWN phase boundary — a whole-field early-out would batch the
/// ripple into one snap per window (and leave a fresh room's field out of step until
/// the next one). Only a real frame change touches the sprite.
pub fn sway_grass(
    clock: Res<super::room_render::FrameClock>,
    art: Res<PropArt>,
    mut q: Query<(&GrassSway, &mut Sprite)>,
) {
    for (g, mut sprite) in &mut q {
        let want = &art.grass[grass_phase(g.x, g.y, clock.0) % art.grass.len()];
        if sprite.image != *want {
            sprite.image = want.clone();
        }
    }
}

