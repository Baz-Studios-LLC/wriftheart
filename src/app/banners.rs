//! banners.rs — the centered announcement banners (port of game.js townBanner /
//! biomeBanner / interiorBanner + BIOME_INFO + getTownName).
//!
//! Landing a room-slide announces: a TOWN by its name (every town room re-announces, the
//! js rule; the home village is EMBERFALL "- IN RUINS -"), or a NEW REGION when the biome
//! key changes (towns swallow the region banner). Entering a building raises the quiet
//! title plaque. Loads/respawns/exits anchor the region silently (swap_world_room).
//!
//! Town names are procedural (prefix x suffix, ~240), unique per game, resolved at the
//! town FOOTPRINT CENTRE (all districts share one name), and saved per slot.

use super::room_render::{PLAY_X, PLAY_Y};
use super::room_props::HOME_VILLAGE;
use super::screen::playing;
use crate::gfx::{at, font, PIXEL_LAYER};
use crate::room::{PX_H, PX_W};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// js BIOME_INFO: biome key -> (region name, flavor line).
const BIOME_INFO: &[(&str, &str, &str)] = &[
    ("grassland", "GRASSLANDS", "- ROLLING GREEN PLAINS -"),
    ("forest", "DEEP WOODS", "- SHADED AND OVERGROWN -"),
    ("desert", "THE DESERT", "- SUN-SCORCHED SANDS -"),
    ("mountains", "THE HIGHLANDS", "- CRAGS AND COLD STONE -"),
    ("swamp", "THE MIRE", "- A FETID, SUNKEN BOG -"),
    ("graveyard", "CURSED RUINS", "- WHERE THE DEAD WALK -"),
    ("arctic", "THE FROZEN WASTE", "- ICE, SNOW, AND SILENCE -"),
    ("burnt", "THE SCORCHED WOOD", "- ASH WHERE A FOREST STOOD -"),
    ("mushroom", "THE FUNGAL GROVE", "- GIANT SHROOMS, AGLOW -"),
    ("chaos", "THE CHAOS WASTES", "- REALITY COMES UNDONE -"),
    ("embermaw", "THE EMBERMAW", "- WHERE THE EARTH BLEEDS FIRE -"),
    ("petalwood", "THE PETALWOOD", "- BLOSSOMS DRIFT ON SWEET AIR -"),
    ("hollowwood", "THE HOLLOW WOOD", "- THE TREES REMEMBER -"),
    ("greenmaw", "THE GREENMAW", "- JUNGLE OF VINE AND FANG -"),
    ("prismwastes", "THE PRISMWASTES", "- FIELDS OF LIVING CRYSTAL -"),
    ("blackdeep", "THE BLACKDEEP", "- A LIGHTLESS UNDER-DARK -"),
    ("honeyglade", "THE HONEYGLADE", "- A SUNLIT MEADOW OF BLOOM -"),
    ("bluebell", "THE BLUEBELL MEADOW", "- COOL AND QUIET BLOSSOMS -"),
    ("suncoast", "THE SUNKEN COAST", "- WHERE THE TIDE COMES IN -"),
    ("stormreach", "STORMREACH", "- A SKY OF ENDLESS THUNDER -"),
    ("tarmire", "THE TARMIRE", "- BLACK PITS THAT CLING -"),
    ("galewind", "THE GALEWIND STEPPE", "- NOTHING STANDS STILL HERE -"),
    ("saltwastes", "THE SALT WASTES", "- A BLINDING, BARREN WHITE -"),
    ("witherlands", "THE WITHERLANDS", "- WHERE ALL THINGS ROT -"),
];

const TOWN_PREFIX: [&str; 20] = [
    "OAK", "ASH", "STONE", "RIVER", "MILL", "FOX", "RAVEN", "THORN", "WIND", "FROST", "ELM",
    "BRIAR", "GREEN", "BLACK", "GOLD", "SILVER", "MOSS", "HAWK", "PINE", "COLD",
];
const TOWN_SUFFIX: [&str; 12] =
    ["DALE", "TON", "BURY", "FORD", "WOOD", "FIELD", "HAVEN", "BROOK", "WICK", "VALE", "HOLLOW", "GATE"];

/// Assigned town names, "cx,cy" (footprint centre) -> name; unique per game, SAVED.
#[derive(Resource, Default)]
pub struct TownNames(pub HashMap<String, String>);

impl TownNames {
    /// js getTownName: the centre's assigned name, or the first unused prefix+suffix
    /// starting from a coord-hashed index (EMBERFALL for the home village, canon).
    pub fn get(&mut self, world: &crate::worldgen::World, rx: i32, ry: i32) -> String {
        let key = crate::worldgen::towns::town_site_of(world.seed, rx, ry)
            .filter(|_| world.town_role(rx, ry).is_some())
            .map_or_else(|| format!("{rx},{ry}"), |s| format!("{},{}", s.tx, s.ty));
        if let Some(n) = self.0.get(&key) {
            return n.clone();
        }
        let name = if (rx, ry) == HOME_VILLAGE {
            "EMBERFALL".to_string()
        } else {
            let used: std::collections::HashSet<&String> = self.0.values().collect();
            let total = TOWN_PREFIX.len() * TOWN_SUFFIX.len();
            let start = (world.seed
                ^ (rx as u32).wrapping_mul(73856093)
                ^ (ry as u32).wrapping_mul(19349663)) as usize;
            (0..total)
                .map(|i| {
                    let idx = (start + i) % total;
                    format!("{}{}", TOWN_PREFIX[idx % TOWN_PREFIX.len()], TOWN_SUFFIX[idx / TOWN_PREFIX.len()])
                })
                .find(|nm| !used.contains(nm))
                .unwrap_or_else(|| format!("TOWN {}", self.0.len() + 1))
        };
        self.0.insert(key, name.clone());
        name
    }
}

/// The live banners (one of each kind at a time, js-style).
#[derive(Resource, Default)]
pub struct Banners {
    town: Option<(String, Option<&'static str>, u32)>, // (name, sub, t)
    biome: Option<(&'static str, u32)>,                // (biome key, t)
    interior: Option<(String, u32)>,                   // (building title, t)
    last_biome: &'static str,                          // region tracking (js lastBiome)
    last_town: Option<(i32, i32)>,                     // town-site tracking (multi-room towns)
    dirty: bool,
}

impl Banners {
    /// The room's town identity — the site room that owns it (multi-room towns
    /// share one site), or the home village standing as its own.
    fn town_key(world: &crate::worldgen::World, rx: i32, ry: i32) -> Option<(i32, i32)> {
        if (rx, ry) == HOME_VILLAGE {
            return Some(HOME_VILLAGE);
        }
        if !world.is_town(rx, ry) {
            return None;
        }
        world.town_site_of(rx, ry).map(|s| (s.tx, s.ty))
    }
    /// A room-slide landed (js transition-end): a town announces when you CROSS
    /// INTO it — its streets stay quiet after that (Baz: every tile of a
    /// multi-room town re-raised the name); a biome change announces the region
    /// unless a town banner owns the moment.
    pub fn room_entered(&mut self, world: &crate::worldgen::World, names: &mut TownNames, rx: i32, ry: i32) {
        let town = Self::town_key(world, rx, ry);
        if town.is_some() && town != self.last_town {
            let sub = if (rx, ry) == HOME_VILLAGE { Some("- IN RUINS -") } else { None };
            self.town = Some((names.get(world, rx, ry), sub, 0));
            self.dirty = true;
        }
        self.last_town = town;
        let bk = world.biome_key_at(rx, ry);
        if bk != self.last_biome {
            if town.is_none() && BIOME_INFO.iter().any(|(k, ..)| *k == bk) {
                self.biome = Some((bk, 0));
                self.dirty = true;
            }
            self.last_biome = bk;
        }
    }
    /// A silent arrival (load, respawn, interior exit): anchor the region, clear the air.
    pub fn anchor(&mut self, world: &crate::worldgen::World, rx: i32, ry: i32) {
        self.last_biome = world.biome_key_at(rx, ry);
        self.last_town = Self::town_key(world, rx, ry); // an interior exit must not re-announce
        self.town = None;
        self.biome = None;
        self.dirty = true;
    }
    /// An un-beaten camp announces itself on entry (the js threat banner) — the town
    /// slot carries it (big letters + a warning sub-line).
    pub fn threat(&mut self, name: &str) {
        self.town = Some((name.to_string(), Some("- CLEAR THEM OUT -"), 0));
        // Crossing a region line INTO a camp raised both at once, overlapping
        // (Baz) — the threat owns the moment; the region can introduce itself later.
        self.biome = None;
        self.dirty = true;
    }
    /// The dungeon boss's name-splash on arena entry (js drawBossName): big letters +
    /// a warning sub-line, in the same dramatic town slot as a threat banner.
    pub fn boss(&mut self, name: &str) {
        self.town = Some((name.to_string(), Some("- IT GUARDS THE SHARD -"), 0));
        self.biome = None; // the arena splash owns the moment (the threat rule)
        self.dirty = true;
    }
    /// A story beat's announcement — the same big slot, minus the threat framing.
    pub fn note(&mut self, title: &str, sub: &'static str) {
        self.town = Some((title.to_string(), Some(sub), 0));
        self.biome = None; // the beat owns the moment (the threat rule)
        self.dirty = true;
    }
    /// Stepping into a building raises its little title plaque (js interiorBanner).
    pub fn interior(&mut self, title: &str) {
        if !title.is_empty() {
            self.interior = Some((title.to_string(), 0));
            self.dirty = true;
        }
    }
}

#[derive(Component, Clone)]
struct BannerUi;

/// Which banner a fading sprite belongs to + its fade profile.
#[derive(Component)]
struct BannerFade {
    kind: u8, // 0 town, 1 biome, 2 interior
    base: f32,
}

pub struct BannersPlugin;

impl Plugin for BannersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Banners>()
            .init_resource::<TownNames>()
            .add_systems(bevy::app::FixedUpdate, banner_tick.run_if(playing))
            .add_systems(Update, banner_draw);
    }
}

/// Age the banners on the world clock (frozen with it, like the js).
fn banner_tick(mut banners: ResMut<Banners>) {
    if let Some((.., t)) = &mut banners.town {
        *t += 1;
        if *t > 150 {
            banners.town = None;
            banners.dirty = true;
        }
    }
    if let Some((_, t)) = &mut banners.biome {
        *t += 1;
        if *t > 150 {
            banners.biome = None;
            banners.dirty = true;
        }
    }
    if let Some((_, t)) = &mut banners.interior {
        *t += 1;
        if *t > 140 {
            banners.interior = None;
            banners.dirty = true;
        }
    }
}

/// js fade profile: rise over `up` frames, hold, fall over the last `down`.
fn fade(t: u32, dur: u32, up: u32, down: u32) -> f32 {
    if t < up {
        t as f32 / up as f32
    } else if t > dur - down {
        ((dur - t) as f32 / down as f32).max(0.0)
    } else {
        1.0
    }
}

/// Rebuild the banner sprites when content changes; retune alphas every frame.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn banner_draw(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut banners: ResMut<Banners>,
    old: Query<Entity, With<BannerUi>>,
    mut fades: Query<(&BannerFade, &mut Sprite)>,
) {
    let (cx, cy) = (PLAY_X + PX_W as f32 / 2.0, PLAY_Y + (PX_H as f32 / 2.0).round());
    if banners.dirty {
        banners.dirty = false;
        for e in &old {
            commands.entity(e).despawn();
        }
        let spawn_text = |images: &mut Assets<Image>, commands: &mut Commands, text: &str, y: f32, scale: f32, color: u32, kind: u8, base: f32, z: f32| {
            // A SHARP drop shadow under every banner line (Baz): the same bake in
            // near-black, one scaled pixel down-right, riding the same fade.
            let (shadow, sw) = font::bake_text(text, 0x0a0a0a, images);
            let siw = (sw + (sw & 1)) as f32;
            commands.spawn((
                Sprite {
                    image: shadow,
                    custom_size: Some(Vec2::new(siw * scale, 6.0 * scale)),
                    color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                    ..default()
                },
                at((cx - siw * scale / 2.0).round() + scale, y - 3.0 * scale + scale, siw * scale, 6.0 * scale, z - 0.01),
                PIXEL_LAYER,
                BannerUi,
                BannerFade { kind, base: base * 0.9 },
            ));
            let (img, w) = font::bake_text(text, color, images);
            let iw = (w + (w & 1)) as f32;
            // js centerText anchors mid-glyph: x centered, y is the text centre.
            commands.spawn((
                Sprite {
                    image: img,
                    custom_size: Some(Vec2::new(iw * scale, 6.0 * scale)),
                    color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                    ..default()
                },
                at((cx - iw * scale / 2.0).round(), y - 3.0 * scale, iw * scale, 6.0 * scale, z),
                PIXEL_LAYER,
                BannerUi,
                BannerFade { kind, base },
            ));
        };
        if let Some((name, sub, _)) = &banners.town {
            spawn_text(&mut images, &mut commands, name, cy - 26.0, 3.0, 0xfce0a8, 0, 1.0, 15.5);
            spawn_text(&mut images, &mut commands, sub.unwrap_or("- A QUIET VILLAGE -"), cy - 6.0, 1.0, 0xc8b890, 0, 1.0, 15.5);
        }
        if let Some((bk, _)) = &banners.biome
            && let Some((_, name, flavor)) = BIOME_INFO.iter().find(|(k, ..)| k == bk)
        {
            spawn_text(&mut images, &mut commands, name, cy - 26.0, 3.0, 0xdff0e4, 1, 1.0, 15.5);
            spawn_text(&mut images, &mut commands, flavor, cy - 6.0, 1.0, 0xa4c0ac, 1, 1.0, 15.5);
        }
        if let Some((title, _)) = &banners.interior {
            // The quiet plaque near the top: dark bar + gold rules + the title (js).
            let bw = font::measure(title) as f32 * 2.0 + 14.0;
            let bh = 18.0;
            let (bx, by) = ((cx - bw / 2.0).round(), PLAY_Y + 7.0);
            let bar = |commands: &mut Commands, y: f32, h: f32, c: u32, base: f32| {
                commands.spawn((
                    Sprite::from_color(
                        Color::srgba_u8((c >> 16) as u8, (c >> 8) as u8, c as u8, 0),
                        Vec2::new(bw, h),
                    ),
                    at(bx, y, bw, h, 15.45),
                    PIXEL_LAYER,
                    BannerUi,
                    BannerFade { kind: 2, base },
                ));
            };
            bar(&mut commands, by, bh, 0x1a1208, 0.82);
            bar(&mut commands, by, 1.0, 0x7a5418, 0.82);
            bar(&mut commands, by + bh - 1.0, 1.0, 0x7a5418, 0.82);
            spawn_text(&mut images, &mut commands, &title.clone(), by + bh / 2.0, 2.0, 0xfce0a8, 2, 1.0, 15.5);
        }
    }
    // Alpha ride: each kind reads its own timer + profile.
    for (f, mut sprite) in &mut fades {
        let a = match f.kind {
            0 => banners.town.as_ref().map_or(0.0, |(.., t)| fade(*t, 150, 18, 40)),
            1 => banners.biome.as_ref().map_or(0.0, |(_, t)| fade(*t, 150, 18, 40)),
            _ => banners.interior.as_ref().map_or(0.0, |(_, t)| fade(*t, 140, 14, 40)),
        };
        sprite.color = sprite.color.with_alpha(a * f.base);
    }
}
