//! farm.rs — Stardew-style farming (port of js/farm.js + the game.js farm hooks): the
//! hoe tills any SOFT EARTH, seeds plant in season, the watering can waters daily (12
//! pours, refill at open water or a town well), each dawn watered crops grow a stage —
//! unwatered ones dry out and wither, out-of-season ones die — and a walk-up Interact
//! press harvests ripe produce. Rain waters every tilled tile in the room for free.
//!
//! Tiles live in [`FarmTiles`] (saved); soil + crop sprites are rebuilt from data on
//! every change ([`FarmDirty`]) and spawn with the room (room_props) so they ride
//! slides. Wilderness soil untended past DECAY days reverts at dawn; the js HOME-room
//! permanence waits on the housing port (DEVIATION, flagged in PORT.md).
//!
//! Seasonal WILD CROPS forage-spawn in ~30% of ordinary rooms per day (seeded hash,
//! js spawnWildCrops) — press to pick; the daily gather stamp keeps them picked.

use super::battle::{spawn_burst, GameRng};
use super::dungeon::InDungeon;
use super::fishing::Fishing;
use super::gather::{farm_day, spawn_pickup, GatherNode, GatherState};
use super::interior::Inside;
use super::play::{ActiveRoot, CurGrid, CurRoom, GameWorld, Player, SlideActive};
use super::room_props::HOME_VILLAGE;
use super::room_render::{child, FrameClock, PLAY_X, PLAY_Y};
use super::screen::playing;
use crate::actors::hero::Facing;
use crate::gfx::{at, PIXEL_LAYER};
use crate::input::{Action, ActionState};
use crate::items::{crop, CropDef};
use crate::room::{RoomGrid, COLS, ROWS};
use crate::worldgen::World;
use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Wilderness tilled soil reverts this many days after it was last tended (js DECAY).
pub const DECAY: i64 = 3;
/// The watering can's tank (js wateringcan.capacity).
pub const CAN_CAP: i32 = 12;

/// A planted crop on one tile (js t.crop): id into [`crate::items::CROPS`], watered
/// growth stage, and consecutive dry days (3 -> it withers).
#[derive(Clone)]
pub struct Crop {
    pub id: &'static str,
    pub stage: i32,
    pub dry: i32,
}

/// One tilled tile (js tile state {tilled, home, tendedDay, wateredDay, crop} — tilled
/// is implicit: a tile only exists while hoed).
#[derive(Clone)]
pub struct FarmTile {
    pub home: bool,
    pub tended: i64,
    pub watered: i64,
    pub crop: Option<Crop>,
}

/// Every hoed tile in the world: room -> tile -> state (js `tiles`, saved).
#[derive(Resource, Default)]
pub struct FarmTiles(pub HashMap<(i32, i32), HashMap<(i32, i32), FarmTile>>);

fn mature(c: &Crop) -> bool {
    crop(c.id).is_some_and(|d| c.stage >= d.stages)
}

impl FarmTiles {
    pub fn tile(&self, room: (i32, i32), c: i32, r: i32) -> Option<&FarmTile> {
        self.0.get(&room).and_then(|m| m.get(&(c, r)))
    }
    pub fn till(&mut self, room: (i32, i32), c: i32, r: i32, home: bool, day: i64) {
        self.0.entry(room).or_default().insert((c, r), FarmTile { home, tended: day, watered: -1, crop: None });
    }
    /// Water one tile; false if there's no soil here (js Farm.water).
    pub fn water(&mut self, room: (i32, i32), c: i32, r: i32, day: i64) -> bool {
        let Some(t) = self.0.get_mut(&room).and_then(|m| m.get_mut(&(c, r))) else { return false };
        t.watered = day;
        t.tended = day;
        true
    }
    /// Rain wets EVERY tilled tile in a room at once (js Farm.waterRoom).
    pub fn water_room(&mut self, room: (i32, i32), day: i64) {
        if let Some(m) = self.0.get_mut(&room) {
            for t in m.values_mut() {
                t.watered = day;
                t.tended = day;
            }
        }
    }
    /// Pick the soil back up (js Farm.clear). True if there was tilled soil here.
    pub fn clear(&mut self, room: (i32, i32), c: i32, r: i32) -> bool {
        self.0.get_mut(&room).is_some_and(|m| m.remove(&(c, r)).is_some())
    }
    /// Plant on bare tilled soil (js Farm.plant).
    pub fn plant(&mut self, room: (i32, i32), c: i32, r: i32, id: &'static str, day: i64) -> bool {
        let Some(t) = self.0.get_mut(&room).and_then(|m| m.get_mut(&(c, r))) else { return false };
        if t.crop.is_some() {
            return false;
        }
        t.crop = Some(Crop { id, stage: 0, dry: 0 });
        t.tended = day;
        true
    }
    /// The crop id of a RIPE planted crop here, else None (js Farm.readyAt).
    pub fn ready_at(&self, room: (i32, i32), c: i32, r: i32) -> Option<&'static str> {
        self.tile(room, c, r).and_then(|t| t.crop.as_ref()).filter(|cr| mature(cr)).map(|cr| cr.id)
    }
    /// Take a ripe crop's produce; the soil stays tilled (js Farm.harvest).
    pub fn harvest(&mut self, room: (i32, i32), c: i32, r: i32) -> Option<&'static str> {
        let t = self.0.get_mut(&room).and_then(|m| m.get_mut(&(c, r)))?;
        let id = t.crop.as_ref().filter(|cr| mature(cr)).map(|cr| cr.id)?;
        t.crop = None;
        Some(id)
    }
    /// Dawn of `new_day` (js Farm.dawnTick): out-of-season crops die; crops watered the
    /// prior day grow a stage; unwatered ones dry out and (3 days) wither.
    pub fn dawn_tick(&mut self, new_day: i64, season: &str) {
        for m in self.0.values_mut() {
            for t in m.values_mut() {
                let Some(cr) = &mut t.crop else { continue };
                let def = crop(cr.id);
                if def.is_some_and(|d| !d.seasons.contains(&season)) {
                    t.crop = None; // the season turned — it dies
                    continue;
                }
                if mature(cr) {
                    continue; // ripe crops just wait to be picked
                }
                if t.watered == new_day - 1 {
                    cr.stage += 1;
                    cr.dry = 0;
                } else {
                    cr.dry += 1;
                    if cr.dry >= 3 {
                        t.crop = None; // 3 dry days -> it withers
                    }
                }
            }
        }
    }
    /// GREENSONG: bring every unripe crop in one room straight to maturity; returns
    /// how many leapt to fruit (js Farm.ripenRoom).
    pub fn ripen_room(&mut self, room: (i32, i32)) -> i32 {
        let Some(m) = self.0.get_mut(&room) else { return 0 };
        let mut n = 0;
        for t in m.values_mut() {
            let Some(cr) = &mut t.crop else { continue };
            let Some(d) = crop(cr.id) else { continue };
            if cr.stage < d.stages {
                cr.stage = d.stages;
                cr.dry = 0;
                n += 1;
            }
        }
        n
    }
    /// Wilderness soil untended past DECAY days reverts; HOME soil is permanent
    /// (js Farm.prune).
    pub fn prune(&mut self, day: i64) {
        for m in self.0.values_mut() {
            m.retain(|_, t| t.home || (day - t.tended) <= DECAY);
        }
        self.0.retain(|_, m| !m.is_empty());
    }
}

/// The watering can's remaining pours (js stores it on the can's inventory entry; one
/// resource serves — you only ever need one can. DEVIATION, flagged).
#[derive(Resource)]
pub struct CanWater(pub i32);
impl Default for CanWater {
    fn default() -> Self {
        CanWater(CAN_CAP)
    }
}

/// Set after any tile change; sync_farm_sprites rebuilds the room's farm layer.
#[derive(Resource, Default)]
pub struct FarmDirty(pub bool);

/// The last farm day the dawn pass ran for (js lastFarmDay).
#[derive(Resource)]
pub struct LastFarmDay(pub i64);
impl Default for LastFarmDay {
    fn default() -> Self {
        LastFarmDay(i64::MIN)
    }
}

/// A forageable wild crop (js Entities.wildcrop) — press to pick its produce.
#[derive(Component)]
pub struct WildCrop {
    pub crop: &'static str,
    pub c: i32,
    pub r: i32,
}

/// Marker on soil/crop sprites so the sync pass can rebuild them in place.
#[derive(Component)]
pub struct FarmSprite;

/// The pulsing farm reticle's two layers (fill wash + corner brackets).
#[derive(Component)]
struct ReticlePart;

/// The hoe's veg-strip query row (entity + whichever marker the scenery carries).
type VegQuery = (Entity, Option<&'static GatherNode>, Option<&'static GroundVeg>);
type VegFilter = Or<(With<GatherNode>, With<GroundVeg>)>;

/// SOFT EARTH — every biome's growing ground takes the blade (js TILLABLE).
const TILLABLE: &[&str] = &[
    "grass", "dirt", "meadow", "bluemeadow", "jungle", "spore", "mud", "deadgrass", "rotleaf", "gravedirt", "steppe", "bog",
];

/// js seasonName for the farm passes.
fn season_name(clock: i64) -> &'static str {
    super::codex::calendar_tab::SEASONS[super::codex::calendar_tab::season_index(clock) % 4]
}

/// js Farm.tillable: inside the border ring, unhoed, not solid, on soft earth.
fn tillable(world: &World, grid: &RoomGrid, farm: &FarmTiles, room: (i32, i32), c: i32, r: i32) -> bool {
    if !(1..COLS - 1).contains(&c) || !(1..ROWS - 1).contains(&r) {
        return false;
    }
    if farm.tile(room, c, r).is_some() {
        return false;
    }
    if grid.box_hits_solid((c * 16 + 8) as f32, (r * 16 + 10) as f32, 1.0, 1.0) {
        return false;
    }
    TILLABLE.contains(&world.ground_name(room.0 * COLS + c, room.1 * ROWS + r))
}

/// The tile the player faces, in room coords (js farmFrontTile).
fn front_tile(p: &Player) -> (i32, i32) {
    let (dx, dy) = match p.facing {
        Facing::Up => (0, -1),
        Facing::Down => (0, 1),
        Facing::Left => (-1, 0),
        Facing::Right => (1, 0),
    };
    ((((p.x + 8.0) / 16.0).floor() as i32) + dx, (((p.y + 12.0) / 16.0).floor() as i32) + dy)
}

/// What the hoe would do at (c,r): till bare earth, clear empty soil, or nothing
/// (js hoeActionAt — towns and the burnt village refuse the blade).
#[derive(PartialEq, Clone, Copy)]
enum HoeAct {
    Till,
    Clear,
    None,
}
fn hoe_action(world: &World, grid: &RoomGrid, farm: &FarmTiles, room: (i32, i32), c: i32, r: i32) -> HoeAct {
    if world.is_town(room.0, room.1) || room == HOME_VILLAGE {
        return HoeAct::None;
    }
    if let Some(t) = farm.tile(room, c, r) {
        return if t.crop.is_some() { HoeAct::None } else { HoeAct::Clear };
    }
    if tillable(world, grid, farm, room, c, r) { HoeAct::Till } else { HoeAct::None }
}

// --- The pixel art: soil beds, growing crops, wild forage, the reticle. Baked fresh
// per room build — a handful of tiny images, same budget as the room bake itself. ---

struct Px {
    w: u32,
    h: u32,
    buf: Vec<u8>,
}
impl Px {
    fn new(w: u32, h: u32) -> Px {
        Px { w, h, buf: vec![0; (w * h * 4) as usize] }
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, rgb: u32) {
        for yy in y.max(0)..(y + h).min(self.h as i32) {
            for xx in x.max(0)..(x + w).min(self.w as i32) {
                let i = ((yy as u32 * self.w + xx as u32) * 4) as usize;
                self.buf[i] = (rgb >> 16) as u8;
                self.buf[i + 1] = (rgb >> 8) as u8;
                self.buf[i + 2] = rgb as u8;
                self.buf[i + 3] = 255;
            }
        }
    }
    fn into_image(self, images: &mut Assets<Image>) -> Handle<Image> {
        use bevy::asset::RenderAssetUsages;
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        images.add(Image::new(
            Extent3d { width: self.w, height: self.h, depth_or_array_layers: 1 },
            TextureDimension::D2,
            self.buf,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        ))
    }
}

/// 40% white over a colour (the js 'hi' highlight pixel).
fn lighten(rgb: u32) -> u32 {
    let mix = |c: u32| c + (255 - c) * 2 / 5;
    (mix(rgb >> 16 & 255) << 16) | (mix(rgb >> 8 & 255) << 8) | mix(rgb & 255)
}

/// A ripe crop's fruit, centred at (cx,cy), in a SHAPE distinct per crop (js drawFruit).
fn draw_fruit(px: &mut Px, cx: i32, cy: i32, def: &CropDef) {
    let col = def.color;
    let hi = lighten(col);
    let grn = 0x3cba4a;
    let mut r = |x: i32, y: i32, w: i32, h: i32, c: u32| px.rect(cx + x, cy + y, w, h, c);
    match def.shape {
        "oval" => {
            // potato — squat oval
            r(-3, -1, 7, 3, col); r(-2, -2, 5, 1, col); r(-2, 2, 5, 1, col); r(-2, -1, 1, 1, hi);
        }
        "cone" => {
            // carrot — wide top tapering to a point, leafy crown
            r(-1, -5, 1, 1, grn); r(1, -5, 1, 1, grn);
            r(-3, -3, 6, 1, col); r(-2, -2, 4, 1, col); r(-2, -1, 4, 1, col); r(-1, 0, 2, 1, col);
            r(0, 1, 1, 1, col); r(-2, -3, 1, 1, hi);
        }
        "grain" => {
            // wheat — tall thin grain head
            r(0, -5, 2, 9, col); r(-2, -4, 1, 1, col); r(2, -3, 1, 1, col); r(-2, -2, 1, 1, col);
            r(2, -1, 1, 1, col); r(-2, 0, 1, 1, col);
        }
        "long" => {
            // pepper — narrow, slightly bent, green stem
            r(0, -5, 1, 1, grn); r(-2, -4, 4, 2, col); r(-1, -2, 4, 2, col); r(0, 0, 3, 2, col); r(-2, -4, 1, 1, hi);
        }
        "big" => {
            // pumpkin — wide, ribbed
            r(-4, -2, 9, 5, col); r(-3, -3, 7, 1, col); r(-3, 3, 7, 1, col);
            r(-2, -2, 1, 5, 0xb0601a); r(1, -2, 1, 5, 0xb0601a); r(0, -4, 1, 1, grn); r(-3, -2, 1, 1, hi);
        }
        "cluster" => {
            // cranberry — a little bunch of berries
            r(-3, -1, 2, 2, col); r(0, -2, 2, 2, col); r(-1, 1, 2, 2, col); r(2, 0, 2, 2, col); r(-3, -1, 1, 1, hi);
        }
        _ => {
            // 'round' — turnip, tomato
            r(-3, -2, 6, 5, col); r(-2, -3, 4, 1, col); r(-2, 3, 4, 1, col); r(-2, -2, 1, 1, hi);
        }
    }
}

/// One 16x16 soil bed, moist or dry (js Farm.draw's fillRects, verbatim colours).
fn soil_image(images: &mut Assets<Image>, wet: bool) -> Handle<Image> {
    let mut px = Px::new(16, 16);
    let (base, furrow, top) = if wet { (0x4a3016, 0x3a2410, 0x5a3a1e) } else { (0x6b4a2a, 0x5a3c20, 0x7c5430) };
    px.rect(1, 1, 14, 14, base);
    px.rect(1, 4, 14, 1, furrow);
    px.rect(1, 9, 14, 1, furrow);
    px.rect(1, 13, 14, 1, furrow);
    px.rect(1, 1, 14, 1, top);
    px.into_image(images)
}

/// The growing plant: stem + leaves + (ripe) fruit, in a 16x20 canvas whose top sits
/// 4px ABOVE the tile (tall fruit pokes over the bed, js baseY - h - 1).
fn crop_image(images: &mut Assets<Image>, def: &CropDef, stage: i32) -> Handle<Image> {
    let mut px = Px::new(16, 20);
    let ripe = stage >= def.stages;
    let prog = (stage as f32 / def.stages as f32).min(1.0);
    let h = 2 + (prog * 9.0).round() as i32;
    let base_y = 18; // tile y+14, in canvas rows
    px.rect(7, base_y - h, 2, h, 0x2a8a3a); // stem
    if prog > 0.25 {
        px.rect(5, base_y - h + 2, 2, 2, 0x3cba4a); // leaves
        px.rect(9, base_y - h + 4, 2, 2, 0x3cba4a);
    }
    if ripe {
        draw_fruit(&mut px, 8, base_y - h - 1, def);
    }
    px.into_image(images)
}

/// A wild forage plant: stems + leaves + the crop's fruit (js Entities.wildcrop draw).
fn wild_image(images: &mut Assets<Image>, def: &CropDef) -> Handle<Image> {
    let mut px = Px::new(16, 16);
    px.rect(5, 8, 2, 7, 0x2a8a3a); // stems
    px.rect(9, 8, 2, 6, 0x2a8a3a);
    px.rect(2, 9, 3, 2, 0x3cba4a); // leaves
    px.rect(11, 10, 3, 2, 0x3cba4a);
    px.rect(6, 6, 4, 2, 0x3cba4a);
    draw_fruit(&mut px, 8, 5, def);
    px.into_image(images)
}

/// Spawn the room's soil + crop sprites as children of `root` (room entry builds them
/// with the room; in-room changes rebuild via [`FarmDirty`]). Soil sits under the
/// cosmetic statics (flowers 3.0); plants over the beds, still under every actor.
pub fn spawn_farm_layer(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    root: Entity,
    farm: &FarmTiles,
    room: (i32, i32),
    today: i64,
) {
    let Some(tiles) = farm.0.get(&room) else { return };
    let (dry, wet) = (soil_image(images, false), soil_image(images, true));
    for (&(c, r), t) in tiles {
        let (x, y) = ((c * 16) as f32, (r * 16) as f32);
        let img = if t.watered == today { wet.clone() } else { dry.clone() };
        let e = child(commands, root, Sprite::from_image(img), at(PLAY_X + x, PLAY_Y + y, 16.0, 16.0, 2.85));
        commands.entity(e).insert(FarmSprite);
        if let Some(cr) = &t.crop
            && let Some(def) = crop(cr.id)
        {
            let img = crop_image(images, def, cr.stage);
            let e = child(commands, root, Sprite::from_image(img), at(PLAY_X + x, PLAY_Y + y - 4.0, 16.0, 20.0, 2.9));
            commands.entity(e).insert(FarmSprite);
        }
    }
}

/// Seasonal forageable wild crops scattered in ordinary rooms (js spawnWildCrops):
/// seeded per (room, day), ~30% of rooms, 1-2 plants on bare grass. Picked ones stay
/// gone for the day via the gather stamp.
#[allow(clippy::too_many_arguments)] // room composition needs the room's whole context
pub fn spawn_wildcrops(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    root: Entity,
    world: &World,
    grid: &RoomGrid,
    farm: &FarmTiles,
    gather: &GatherState,
    room: (i32, i32),
    clock: i64,
) {
    let (rx, ry) = room;
    if world.is_town(rx, ry) || room == HOME_VILLAGE || World::is_castle(rx, ry) {
        return;
    }
    let season = season_name(clock);
    let in_season: Vec<&CropDef> = crate::items::CROPS.iter().filter(|d| d.seasons.contains(&season)).collect();
    if in_season.is_empty() {
        return; // winter: the wild lies fallow
    }
    let day = super::gather::day_number(clock);
    let mut h = (rx.wrapping_mul(73856093) ^ ry.wrapping_mul(19349663) ^ (day as i32).wrapping_mul(83492791) ^ world.seed as i32)
        as u32;
    let mut rnd = move || {
        h = (h ^ (h >> 15)).wrapping_mul(2246822519);
        h ^= h >> 13;
        h as f64 / 4294967296.0
    };
    if rnd() > 0.3 {
        return; // ~30% of wild rooms grow forage that day
    }
    let n = if rnd() < 0.35 { 2 } else { 1 };
    let today = farm_day(clock);
    for _ in 0..n {
        let c = 2 + (rnd() * (COLS - 4) as f64) as i32;
        let r = 2 + (rnd() * (ROWS - 4) as f64) as i32;
        let def = in_season[(rnd() * in_season.len() as f64) as usize];
        // Not on solids, non-grass ground, or hoed soil (js), nor picked today.
        if grid.box_hits_solid((c * 16 + 8) as f32, (r * 16 + 8) as f32, 1.0, 1.0)
            || world.ground_name(rx * COLS + c, ry * ROWS + r) != "grass"
            || farm.tile(room, c, r).is_some()
            || gather.taken(room, c, r, today)
        {
            continue;
        }
        let img = wild_image(images, def);
        let e = child(
            commands,
            root,
            Sprite::from_image(img),
            at(PLAY_X + (c * 16) as f32, PLAY_Y + (r * 16) as f32, 16.0, 16.0, 3.1),
        );
        commands.entity(e).insert(WildCrop { crop: def.id, c, r });
    }
}

/// The world/room/mode context the farm passes read (grouped under the 16-param cap).
#[derive(SystemParam)]
pub struct FarmCtx<'w> {
    world: Res<'w, GameWorld>,
    cur: Res<'w, CurRoom>,
    grid: Res<'w, CurGrid>,
    clock: Res<'w, FrameClock>,
    inside: Res<'w, Inside>,
    in_dungeon: Res<'w, InDungeon>,
    fishing: Res<'w, Fishing>,
    sliding: Res<'w, SlideActive>,
    stations: Res<'w, super::cooking::PlacedStations>,
}

impl FarmCtx<'_> {
    /// Out in the overworld proper, feet on the ground, hands free.
    fn overworld(&self) -> bool {
        self.inside.0.is_none() && self.in_dungeon.0.is_none() && self.fishing.0.is_none() && !self.sliding.0
    }
}

/// Dawn rolls the crops (grow / dry / wither / season-cull, then soil decay); rain
/// waters the room's tilled tiles for free every 16 frames (js weather-effects block).
fn farm_sim_tick(
    ctx: FarmCtx,
    weather: Res<super::weather::WeatherState>,
    mut farm: ResMut<FarmTiles>,
    mut last: ResMut<LastFarmDay>,
    mut dirty: ResMut<FarmDirty>,
) {
    let fd = farm_day(ctx.clock.0);
    if last.0 == i64::MIN {
        last.0 = fd; // first tick after a boot/load: today is already accounted for
    } else if fd != last.0 {
        last.0 = fd;
        farm.dawn_tick(fd, season_name(ctx.clock.0));
        farm.prune(fd);
        dirty.0 = true;
    }
    let room = (ctx.cur.rx, ctx.cur.ry);
    if ctx.inside.0.is_none()
        && ctx.in_dungeon.0.is_none()
        && ctx.clock.0 & 15 == 0
        && crate::weather::get(weather.cur).kind == crate::weather::Kind::Rain
        && farm.0.get(&room).is_some_and(|m| !m.is_empty())
    {
        farm.water_room(room, fd);
        dirty.0 = true;
    }
}

/// The farm tools' slot presses: the hoe tills/clears, the can waters/refills, seeds
/// plant (js farmTill / farmWater / farmPlant, threaded through the item use() env).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn farm_tool_tick(
    mut commands: Commands,
    mut input: ResMut<ActionState>,
    mut rng: ResMut<GameRng>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut can: ResMut<CanWater>,
    mut farm: ResMut<FarmTiles>,
    mut dirty: ResMut<FarmDirty>,
    mut log: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    ctx: FarmCtx,
    mut players: Query<&mut Player>,
    veg: Query<VegQuery, VegFilter>,
) {
    let Ok(mut p) = players.single_mut() else { return };
    if !ctx.overworld() {
        return;
    }
    let room = (ctx.cur.rx, ctx.cur.ry);
    let fd = farm_day(ctx.clock.0);
    let (fc, fr) = front_tile(&p);
    for (i, action) in [Action::Slot1, Action::Slot2, Action::Slot3, Action::Slot4].into_iter().enumerate() {
        if !input.pressed(action) || p.cooldowns[i] > 0 {
            continue;
        }
        let Some(def) = inv.slots[i].and_then(|uid| inv.def_of(uid)) else { continue };
        match def.id {
            "hoe" => {
                input.consume(action);
                p.cooldowns[i] = def.cooldown;
                match hoe_action(&ctx.world.0, &ctx.grid.0, &farm, room, fc, fr) {
                    HoeAct::Till => {
                        farm.till(room, fc, fr, false, fd); // home plots land with housing
                        // Strip grass + cosmetic scenery off the fresh soil (js GROUND_VEG).
                        for (e, node, gv) in &veg {
                            let tile = node
                                .filter(|n| n.kind == "grass")
                                .map(|n| (n.c, n.r))
                                .or(gv.map(|g| (g.c, g.r)));
                            if tile == Some((fc, fr)) {
                                commands.entity(e).despawn();
                            }
                        }
                        sfx.write(super::sfx::Sfx("stone"));
                        dirty.0 = true;
                    }
                    HoeAct::Clear => {
                        farm.clear(room, fc, fr);
                        sfx.write(super::sfx::Sfx("stone"));
                        dirty.0 = true;
                    }
                    HoeAct::None => {
                        sfx.write(super::sfx::Sfx("tink"));
                    }
                }
            }
            "wateringcan" => {
                input.consume(action);
                p.cooldowns[i] = def.cooldown;
                let has_soil = farm.tile(room, fc, fr).is_some();
                let facing_water = matches!(ctx.grid.0.code_at(fc, fr), '~' | 'B');
                let near_well = ctx.world.0.room_entities(room.0, room.1).iter().any(|e| {
                    e.kind == "well" && ((e.x + 8) as f32 - (p.x + 8.0)).hypot((e.y + 8) as f32 - (p.y + 9.0)) < 40.0
                }) || ctx.stations.0.iter().any(|s| {
                    // A well you BUILT (placed like a station) refills the can too.
                    s.kind == "well" && s.room == (room.0, room.1) && (s.x + 8.0 - (p.x + 8.0)).hypot(s.y + 8.0 - (p.y + 9.0)) < 40.0
                });
                let splash = Vec2::new((fc * 16 + 8) as f32, (fr * 16 + 8) as f32);
                if !has_soil && (facing_water || near_well) {
                    // REFILL: press with no soil in front, at open water or beside a well.
                    spawn_burst(&mut commands, &mut rng, splash, 0x8ecbe0, 8);
                    if can.0 < CAN_CAP {
                        can.0 = CAN_CAP;
                        log.add("farm", "WATERING CAN FILLED", 1, 0x8ecbe0, false, true);
                    }
                    sfx.write(super::sfx::Sfx("splash"));
                } else if !has_soil {
                    sfx.write(super::sfx::Sfx("tink")); // nothing to water, no water to draw
                } else if can.0 <= 0 {
                    log.add("farm", "THE CAN IS DRY - FILL IT AT WATER OR A WELL", 1, 0xfc8868, false, true);
                    sfx.write(super::sfx::Sfx("tink"));
                } else {
                    p.lock_timer = 6; // a short pause for the watering gesture
                    spawn_burst(&mut commands, &mut rng, splash, 0x8ecbe0, 8);
                    if farm.water(room, fc, fr, fd) {
                        can.0 -= 1;
                        sfx.write(super::sfx::Sfx("open"));
                        dirty.0 = true;
                    }
                }
            }
            _ => {
                let Some(crop_id) = def.seed else { continue };
                input.consume(action);
                let season = season_name(ctx.clock.0);
                let cd = crop(crop_id);
                if cd.is_some_and(|d| !d.seasons.contains(&season)) {
                    log.add("season", &format!("WONT GROW IN {season}"), 1, 0xfc8868, false, true);
                    sfx.write(super::sfx::Sfx("tink"));
                } else if farm.plant(room, fc, fr, cd.map_or(crop_id, |d| d.id), fd) {
                    inv.remove_one(def.id);
                    sfx.write(super::sfx::Sfx("craft"));
                    dirty.0 = true;
                } else {
                    sfx.write(super::sfx::Sfx("tink"));
                }
            }
        }
    }
}

/// The action-button harvest: a WILD crop you're touching, or a ripe PLANTED crop at
/// your feet (js: either suppresses the swing; press picks instead). Runs before
/// talk_tick so the press never falls through to a villager menu.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn farm_harvest_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut input: ResMut<ActionState>,
    mut rng: ResMut<GameRng>,
    mut inv: ResMut<crate::inventory::PlayerInv>,
    mut farm: ResMut<FarmTiles>,
    mut gather: ResMut<GatherState>,
    mut dirty: ResMut<FarmDirty>,
    mut log: ResMut<super::rewards::LootLog>,
    mut stats: ResMut<super::stats::Stats>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
    ctx: FarmCtx,
    players: Query<&Player>,
    wild: Query<(Entity, &WildCrop)>,
) {
    let Ok(p) = players.single() else { return };
    if !ctx.overworld() || !input.pressed(Action::Interact) {
        return;
    }
    let room = (ctx.cur.rx, ctx.cur.ry);
    // A wild crop underfoot (js nearForage: hitbox overlap with the plant's 10x11 zone).
    let hb = (p.x + 3.0, p.y + 2.0, 10.0, 13.0);
    let wild_here = wild.iter().find(|(_, w)| {
        let z = ((w.c * 16 + 3) as f32, (w.r * 16 + 3) as f32, 10.0, 11.0);
        hb.0 < z.0 + z.2 && hb.0 + hb.2 > z.0 && hb.1 < z.1 + z.3 && hb.1 + hb.3 > z.1
    });
    if let Some((we, w)) = wild_here {
        input.consume(Action::Interact);
        // Wild: produce drops at your feet + a leaf burst (js deathEffect; no seeds —
        // buy those from a shop).
        spawn_pickup(&mut commands, &mut images, w.crop, 1, (w.c * 16 + 4) as f32, (w.r * 16 + 2) as f32, true);
        spawn_burst(&mut commands, &mut rng, Vec2::new((w.c * 16 + 8) as f32, (w.r * 16 + 8) as f32), 0x3cba4a, 6);
        stats.bump("crops", 1.0);
        // Picked -> gone for the rest of the day (the js tileTaken window).
        let today = farm_day(ctx.clock.0);
        let rec = gather.rooms.entry(room).or_insert_with(|| (today, Default::default()));
        if rec.0 != today {
            *rec = (today, Default::default());
        }
        rec.1.insert((w.c, w.r));
        sfx.write(super::sfx::Sfx("pickup"));
        commands.entity(we).despawn();
        return;
    }
    // A ripe planted crop at your feet: straight to the bag (js ripeHere branch).
    let (pc, pr) = (((p.x + 8.0) / 16.0).floor() as i32, (((p.y + 12.0) / 16.0).floor()) as i32);
    if let Some(id) = farm.ready_at(room, pc, pr)
        && inv.can_add(id)
    {
        input.consume(Action::Interact);
        farm.harvest(room, pc, pr);
        inv.add_item(id, 1);
        stats.bump("crops", 1.0);
        let name = crate::items::get(id).map_or(id, |d| d.name);
        log.add(id, &name.to_uppercase(), 1, super::rewards::toast_color(id), false, false);
        sfx.write(super::sfx::Sfx("pickup"));
        dirty.0 = true;
    }
}

/// Rebuild the current room's farm sprites whenever the tile data changed. Room ENTRY
/// builds them with the room (spawn_room_props); this pass covers in-room edits.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn sync_farm_sprites(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut dirty: ResMut<FarmDirty>,
    farm: Res<FarmTiles>,
    active: Res<ActiveRoot>,
    ctx: FarmCtx,
    old: Query<Entity, With<FarmSprite>>,
    roots: Query<(), With<super::room_render::RoomRoot>>,
) {
    if !dirty.0 || ctx.sliding.0 {
        return; // mid-slide edits (rain) wait for the room to settle
    }
    dirty.0 = false;
    for e in &old {
        commands.entity(e).despawn();
    }
    // A same-tick room swap can have retired the root already — the swap's own
    // spawn_room_props rebuilt the farm layer with the new room.
    if ctx.inside.0.is_some() || ctx.in_dungeon.0.is_some() || roots.get(active.0).is_err() {
        return;
    }
    spawn_farm_layer(&mut commands, &mut images, active.0, &farm, (ctx.cur.rx, ctx.cur.ry), farm_day(ctx.clock.0));
}

/// The pulsing corner-bracket reticle on the farm target tile while the hoe or can is
/// slotted (js tileReticle): green/amber/red for the hoe, cyan/teal/red for the can.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn farm_reticle(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    inv: Res<crate::inventory::PlayerInv>,
    farm: Res<FarmTiles>,
    ctx: FarmCtx,
    players: Query<&Player>,
    mut parts: Local<Option<(Entity, Entity)>>,
    mut sprites: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<ReticlePart>>,
) {
    let (fill_e, edge_e) = *parts.get_or_insert_with(|| {
        let mut fill = Px::new(14, 14);
        fill.rect(0, 0, 14, 14, 0xffffff);
        // Four 4px corner brackets (js tileReticle's TL/TR/BL/BR arms).
        let mut edge = Px::new(14, 14);
        for (x0, y0) in [(0, 0), (10, 0), (0, 13), (10, 13)] {
            edge.rect(x0, y0, 4, 1, 0xffffff);
        }
        for (x0, y0) in [(0, 0), (13, 0), (0, 10), (13, 10)] {
            edge.rect(x0, y0, 1, 4, 0xffffff);
        }
        let mk = |img: Px, z: f32, images: &mut Assets<Image>, commands: &mut Commands| {
            commands
                .spawn((
                    Sprite::from_image(img.into_image(images)),
                    at(0.0, 0.0, 14.0, 14.0, z),
                    PIXEL_LAYER,
                    Visibility::Hidden,
                    ReticlePart,
                ))
                .id()
        };
        (mk(fill, 3.9, &mut images, &mut commands), mk(edge, 3.91, &mut images, &mut commands))
    });
    let hide = |sprites: &mut Query<(&mut Sprite, &mut Transform, &mut Visibility), With<ReticlePart>>| {
        for e in [fill_e, edge_e] {
            if let Ok((.., mut v)) = sprites.get_mut(e) {
                *v = Visibility::Hidden;
            }
        }
    };
    let Ok(p) = players.single() else { return };
    let hoe = inv.slots.iter().flatten().any(|&uid| inv.id_of(uid) == Some("hoe"));
    let wcan = inv.slots.iter().flatten().any(|&uid| inv.id_of(uid) == Some("wateringcan"));
    let (fc, fr) = front_tile(p);
    if !ctx.overworld() || (!hoe && !wcan) || !(0..COLS).contains(&fc) || !(0..ROWS).contains(&fr) {
        hide(&mut sprites);
        return;
    }
    let room = (ctx.cur.rx, ctx.cur.ry);
    // The hoe's read outranks the can's when both ride the belt (js branch order).
    let (fill_c, edge_c) = if hoe {
        match hoe_action(&ctx.world.0, &ctx.grid.0, &farm, room, fc, fr) {
            HoeAct::Till => (0xa8ff84, 0xe8ffdc),
            HoeAct::Clear => (0xffd27a, 0xfff0c8),
            HoeAct::None => (0xff8a78, 0xffd4cc),
        }
    } else {
        match farm.tile(room, fc, fr) {
            Some(t) if t.watered == farm_day(ctx.clock.0) => (0x7ac8b8, 0xd4f0e8), // already wet
            Some(_) => (0x7fd8ff, 0xdcf4ff),                                       // will water
            None => (0xff8a78, 0xffd4cc),                                          // no soil
        }
    };
    let pulse = 0.5 + 0.5 * (ctx.clock.0 as f32 * 0.18).sin();
    for (e, color, alpha) in [(fill_e, fill_c, 0.16 + 0.16 * pulse), (edge_e, edge_c, 0.55 + 0.4 * pulse)] {
        if let Ok((mut s, mut tf, mut v)) = sprites.get_mut(e) {
            s.color = Color::srgba((color >> 16 & 255) as f32 / 255.0, (color >> 8 & 255) as f32 / 255.0, (color & 255) as f32 / 255.0, alpha);
            *tf = at(PLAY_X + (fc * 16 + 1) as f32, PLAY_Y + (fr * 16 + 1) as f32, 14.0, 14.0, tf.translation.z);
            *v = Visibility::Inherited;
        }
    }
}

/// Cosmetic ground scenery a hoe clears off a tile (flowers, reeds, clutter — the js
/// GROUND_VEG list; grass carries a GatherNode instead). room_props tags these.
#[derive(Component)]
pub struct GroundVeg {
    pub c: i32,
    pub r: i32,
}

pub struct FarmPlugin;

impl Plugin for FarmPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FarmTiles>()
            .init_resource::<CanWater>()
            .init_resource::<FarmDirty>()
            .init_resource::<LastFarmDay>()
            .add_systems(
                bevy::app::FixedUpdate,
                (
                    farm_sim_tick,
                    // After the door/prompt consumers (their priorities win), before
                    // talk so a harvest press never opens the villager menu (js
                    // onObject) — and sprites rebuild the same tick anything changed.
                    (farm_harvest_tick, farm_tool_tick)
                        .after(super::prompts::prompt_tick)
                        .after(super::services::interact_tick)
                        .after(super::interior::door_enter),
                    sync_farm_sprites,
                )
                    .chain()
                    .before(super::play::EndTick)
                    .before(super::talk::talk_tick)
                    .run_if(playing),
            )
            .add_systems(Update, farm_reticle.run_if(playing));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOM: (i32, i32) = (5, -3);

    #[test]
    fn crops_grow_when_watered_and_wither_dry() {
        let mut f = FarmTiles::default();
        f.till(ROOM, 4, 4, false, 10);
        assert!(f.plant(ROOM, 4, 4, "turnip", 10));
        assert!(!f.plant(ROOM, 4, 4, "turnip", 10), "no double planting");
        // Watered each day: stage climbs to maturity (turnip: 3 stages).
        for day in 10..13 {
            assert!(f.water(ROOM, 4, 4, day));
            f.dawn_tick(day + 1, "SPRING");
        }
        assert_eq!(f.ready_at(ROOM, 4, 4), Some("turnip"));
        // Ripe crops wait; harvest empties the tile but keeps the soil.
        f.dawn_tick(14, "SPRING");
        assert_eq!(f.harvest(ROOM, 4, 4), Some("turnip"));
        assert!(f.tile(ROOM, 4, 4).is_some());
        // Unwatered: 3 dry dawns wither a fresh planting.
        assert!(f.plant(ROOM, 4, 4, "turnip", 20));
        for day in 21..24 {
            f.dawn_tick(day, "SPRING");
        }
        assert!(f.tile(ROOM, 4, 4).unwrap().crop.is_none(), "3 dry days -> withered");
    }

    #[test]
    fn season_cull_and_soil_decay() {
        let mut f = FarmTiles::default();
        f.till(ROOM, 3, 3, false, 0);
        assert!(f.plant(ROOM, 3, 3, "turnip", 0));
        f.dawn_tick(1, "SUMMER"); // turnip is spring-only — the season turned
        assert!(f.tile(ROOM, 3, 3).unwrap().crop.is_none());
        // Untended wilderness soil reverts after DECAY days; tending resets the clock.
        f.prune(DECAY); // day 3: exactly DECAY days -> stays
        assert!(f.tile(ROOM, 3, 3).is_some());
        f.prune(DECAY + 1); // day 4: past it -> gone
        assert!(f.tile(ROOM, 3, 3).is_none());
    }

    #[test]
    fn rain_waters_only_the_room() {
        let mut f = FarmTiles::default();
        f.till(ROOM, 2, 2, false, 5);
        f.till((0, 0), 2, 2, false, 5);
        f.water_room(ROOM, 6);
        assert_eq!(f.tile(ROOM, 2, 2).unwrap().watered, 6);
        assert_eq!(f.tile((0, 0), 2, 2).unwrap().watered, -1);
    }
}
