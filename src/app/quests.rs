//! quests.rs — town NPCs hand out jobs (port of js/quests.js + the game.js quest state):
//! ~20% of town folk are QUEST GIVERS ('!' overhead; '-' in progress, '?' ready). Four
//! kinds: CLEAR a nearby encounter camp, BOUNTY a named elite lairing in a marked room,
//! SLAY a quota of a common creature anywhere, FETCH a quota of materials. A 3-slot log;
//! coin + XP (sometimes an item) on turn-in, +1.5 hearts with the giver.
//!
//! The pure logic (is_giver / generate / rewards) is js-verbatim; the window lives in
//! dialog.rs (DialogState::Quest). DEVIATION (flagged): clear quests only target
//! HOSTILE camps (the js could point at friendly wanderer camps, which auto-cleared on
//! arrival — a bug we don't keep). NOT YET: sidebar QUESTS list + codex map pins.

use super::battle::RoomActor;
use super::encounters::{self, ClearedEncounters};
use super::play::{CurRoom, GameWorld, SlideActive};
use super::screen::playing;
use crate::actors::mobs::{self, mob_bundle};
use crate::actors::villager::Villager;
use crate::combat::Health;
use crate::gfx::{font, PIXEL_LAYER};
use crate::worldgen::rng::Mulberry32;
use crate::worldgen::World;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const QUEST_MAX: usize = 3;
const GIVER_PCT: u32 = 20; // was the js 45 — half the town wore a '!' (Baz)
const MIN_RANGE: i32 = 3;
const ENC_RANGE: i32 = 12;
const SALT: u32 = 0x5177e3a1;

/// js Quests.hash — FNV over (a lo/hi, b lo, c), seeded by world seed ^ SALT.
fn qhash(world_seed: u32, a: i32, b: i32, c: i32) -> u32 {
    let mut h = 2166136261u32 ^ world_seed ^ SALT;
    h = (h ^ ((a & 0xffff) as u32)).wrapping_mul(16777619);
    h = (h ^ (((a >> 16) & 0xffff) as u32)).wrapping_mul(16777619);
    h = (h ^ ((b & 0xffff) as u32)).wrapping_mul(16777619);
    h = (h ^ (c as u32)).wrapping_mul(16777619);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    h
}

/// Is this town NPC (stable seed) a quest giver? Deterministic per world seed.
pub fn is_giver(world_seed: u32, seed: u32) -> bool {
    if seed == crate::app::story::SURVIVOR_SEED {
        return false; // the survivor's '!' is the story thread's, never the board's
    }
    qhash(world_seed, seed as i32, 0, 7) % 100 < GIVER_PCT
}

/// js makeRngFor — a mulberry stream seeded by (giver seed, quests-done count).
fn rng_for(world_seed: u32, seed: u32, done: i32) -> Mulberry32 {
    Mulberry32::new(seed ^ (done as u32).wrapping_mul(0x9e3779b1) ^ world_seed)
}

const DIRS: [&str; 8] = ["east", "southeast", "south", "southwest", "west", "northwest", "north", "northeast"];
fn dir_word(dx: i32, dy: i32) -> &'static str {
    if dx == 0 && dy == 0 {
        return "nearby";
    }
    let i = ((dy as f64).atan2(dx as f64) / (std::f64::consts::PI / 4.0)).round() as i32;
    DIRS[i.rem_euclid(8) as usize]
}

const SLAY_KINDS: [&str; 10] =
    ["goblin", "wolf", "spider", "skeleton", "zombie", "bat", "boar", "scorpion", "bandit", "ghoul"];
const BOUNTY_KINDS: [&str; 9] =
    ["bear", "golem", "lurker", "revenant", "charbrute", "icetroll", "myconid", "scorpion", "ghoul"];
const BOUNTY_NAMES: [&str; 10] = [
    "Gravemaw", "Bloodfang", "Old Scar", "Ironhide", "Direclaw", "Gloomtooth", "Ashmaw", "Venomspine", "Rendgut",
    "Skullcrack",
];
const MATERIALS: [&str; 3] = ["wood", "stone", "fiber"];

/// js Enemies.BESTIARY display names for the quest-facing kinds.
pub fn kind_name(kind: &str) -> &'static str {
    match kind {
        "goblin" => "GOBLIN",
        "wolf" => "DIRE WOLF",
        "spider" => "SPITTING SPIDER",
        "skeleton" => "SKELETON",
        "zombie" => "ZOMBIE",
        "bat" => "CAVE BAT",
        "boar" => "TUSK BOAR",
        "scorpion" => "DUSTBACK SCORPION",
        "bandit" => "BANDIT",
        "ghoul" => "GHOUL",
        "bear" => "FOREST BEAR",
        "golem" => "STONE GOLEM",
        "lurker" => "BOG LURKER",
        "revenant" => "REVENANT",
        "charbrute" => "CHAR BRUTE",
        "icetroll" => "ICE TROLL",
        "myconid" => "MYCONID",
        _ => "BEAST",
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum QuestKind {
    /// Wipe out the encounter camp at (rx, ry).
    Clear { rx: i32, ry: i32, enc_name: String },
    /// Slay the named elite lairing at (rx, ry).
    Bounty { rx: i32, ry: i32, kind: String, name: String },
    /// Kill `need` of a common creature, anywhere.
    Slay { kind: String, need: i32, have: i32 },
    /// Bring `need` of a material (checked live against the bag).
    Fetch { item: String, need: i32 },
    /// The first-hour STORY thread's legs (story.rs): stage 1 reach the town's
    /// keeper, stage 2 claim a relic from the marked den. Resolves in the field
    /// (never `ready`, no hand-in) and never counts against the 3-slot log.
    Story { stage: u8, rx: i32, ry: i32 },
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Reward {
    pub coin: i32,
    pub xp: i32,
    pub item: Option<(String, i32)>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Quest {
    pub id: u32,
    pub kind: QuestKind,
    pub done: bool, // clear/bounty objective flags (slay counts, fetch reads the bag)
    pub title: String,
    pub goal: String,
    pub desc: String,
    pub reward: Reward,
    pub giver_key: String,
    pub giver_rx: i32,
    pub giver_ry: i32,
}

impl Quest {
    /// The room this quest pins on the map, if any (js q.marker).
    pub fn marker(&self) -> Option<(i32, i32)> {
        match &self.kind {
            QuestKind::Clear { rx, ry, .. } | QuestKind::Bounty { rx, ry, .. } | QuestKind::Story { rx, ry, .. } => {
                Some((*rx, *ry))
            }
            _ => None,
        }
    }
    /// Story-thread legs are slot-exempt and never hand in (story.rs).
    pub fn is_story(&self) -> bool {
        matches!(self.kind, QuestKind::Story { .. })
    }
    /// js questSig — the "same target" signature offers dedup against.
    pub fn sig(&self) -> String {
        match &self.kind {
            QuestKind::Bounty { kind, .. } => format!("bounty:{kind}"),
            QuestKind::Slay { kind, .. } => format!("slay:{kind}"),
            QuestKind::Fetch { item, .. } => format!("fetch:{item}"),
            QuestKind::Clear { rx, ry, .. } => format!("clear:{rx},{ry}"),
            QuestKind::Story { stage, .. } => format!("story:{stage}"),
        }
    }
    fn type_key(&self) -> &'static str {
        match self.kind {
            QuestKind::Clear { .. } => "clear",
            QuestKind::Bounty { .. } => "bounty",
            QuestKind::Slay { .. } => "slay",
            QuestKind::Fetch { .. } => "fetch",
            QuestKind::Story { .. } => "story",
        }
    }
    /// js questReady — finished and waiting on the hand-in.
    pub fn ready(&self, inv: &crate::inventory::PlayerInv) -> bool {
        match &self.kind {
            QuestKind::Fetch { item, need } => inv.count(item) >= *need,
            QuestKind::Slay { need, have, .. } => have >= need,
            _ => self.done,
        }
    }
    /// js questHave — the live progress count.
    pub fn have(&self, inv: &crate::inventory::PlayerInv) -> i32 {
        match &self.kind {
            QuestKind::Fetch { item, .. } => inv.count(item),
            QuestKind::Slay { have, .. } => *have,
            _ => 0,
        }
    }
    /// js Quests.progressText — the short GOAL line.
    pub fn progress_text(&self, have: i32) -> String {
        match &self.kind {
            QuestKind::Slay { kind, need, .. } => format!("{} {}/{}", kind_name(kind), have.min(*need), need),
            QuestKind::Fetch { item, need } => {
                let name = crate::items::get(item).map_or(item.as_str(), |d| d.name).to_uppercase();
                format!("{} {}/{}", name, have.min(*need), need)
            }
            _ => self.goal.clone(),
        }
    }
}

/// The active log (js quests, max 3 — saved).
#[derive(Resource, Default)]
pub struct QuestLog(pub Vec<Quest>);

/// Per-giver completed count — varies their next offer (js questGiverDone, saved).
#[derive(Resource, Default)]
pub struct GiverDone(pub HashMap<String, i32>);

/// Monotonic quest-id source (js questCounter, saved).
#[derive(Resource, Default)]
pub struct QuestCounter(pub u32);

/// The named elite a bounty spawns — excluded from the room cache so it reappears at
/// full strength each visit until slain (js: no spawnKind).
#[derive(Component)]
pub struct BountyTag(pub u32);

/// A slain foe, reported by the death systems for slay/bounty credit.
#[derive(Message)]
pub struct KillCredit {
    pub kind: &'static str,
    pub bounty: Option<u32>,
}

/// js rollReward: coin/XP scaled by type + distance + tier; ~14% gear, ~28% materials.
fn roll_reward(qtype: &str, dist: i32, tier: i32, rng: &mut Mulberry32) -> Reward {
    let base: f64 = match qtype {
        "clear" => 60.0,
        "bounty" => 70.0,
        "slay" => 35.0,
        _ => 30.0,
    };
    let coin = (base + dist as f64 * 4.0 + tier as f64 * 8.0 + rng.next_f64() * 20.0).round() as i32;
    let xp = (base * 0.8 + dist as f64 * 3.0 + tier as f64 * 7.0 + rng.next_f64() * 12.0).round() as i32;
    let roll = rng.next_f64();
    let item = if roll < 0.14 {
        let (id, qty) = crate::items::roll_loot((tier as f64 * 0.3).min(2.0), 0.0, || rng.next_f64());
        Some((id.to_string(), qty))
    } else if roll < 0.42 {
        let mat = MATERIALS[(rng.next_f64() * MATERIALS.len() as f64) as usize];
        Some((mat.to_string(), 2 + (rng.next_f64() * 3.0) as i32))
    } else {
        None
    };
    Reward { coin, xp, item }
}

/// js findRoom — spiral out over ring perimeters [min_rad..=max_rad].
fn find_room(cx: i32, cy: i32, min_rad: i32, max_rad: i32, ok: impl Fn(i32, i32) -> bool) -> Option<(i32, i32)> {
    for rad in min_rad..=max_rad {
        for dy in -rad..=rad {
            for dx in -rad..=rad {
                if dx.abs().max(dy.abs()) != rad {
                    continue;
                }
                if ok(cx + dx, cy + dy) {
                    return Some((cx + dx, cy + dy));
                }
            }
        }
    }
    None
}

/// The generation context (js questGenCtx): giver room + the active log to dedup against.
pub struct GenCtx<'a> {
    pub world: &'a World,
    pub cleared: &'a ClearedEncounters,
    pub log: &'a [Quest],
    pub rx: i32,
    pub ry: i32,
}

/// js buildType — one concrete quest of `qtype`, or None if it can't be made here.
fn build_type(qtype: &str, ctx: &GenCtx, rng: &mut Mulberry32) -> Option<Quest> {
    let tier = World::threat_tier(ctx.rx, ctx.ry);
    let taken: Vec<String> = ctx.log.iter().map(|q| q.sig()).collect();
    let pick_avoid = |arr: &[&'static str], prefix: &str, rng: &mut Mulberry32| -> &'static str {
        for _ in 0..10 {
            let k = arr[(rng.next_f64() * arr.len() as f64) as usize];
            if !taken.iter().any(|t| t == &format!("{prefix}{k}")) {
                return k;
            }
        }
        arr[(rng.next_f64() * arr.len() as f64) as usize]
    };
    let base = Quest {
        id: 0,
        kind: QuestKind::Fetch { item: String::new(), need: 0 },
        done: false,
        title: String::new(),
        goal: String::new(),
        desc: String::new(),
        reward: Reward::default(),
        giver_key: String::new(),
        giver_rx: ctx.rx,
        giver_ry: ctx.ry,
    };
    match qtype {
        "clear" => {
            let min_r = MIN_RANGE + (rng.next_f64() * 7.0) as i32; // vary 3..9 so rewards differ
            let spot = find_room(ctx.rx, ctx.ry, min_r, ENC_RANGE, |x, y| {
                if ctx.cleared.0.contains(&(x, y)) || taken.contains(&format!("clear:{x},{y}")) || ctx.world.is_town(x, y)
                {
                    return false;
                }
                // Hostile camps only (DEVIATION: the js also matched friendly ones).
                encounters::for_room(ctx.world, x, y).is_some_and(|(d, _)| !d.friendly)
            })?;
            let (def, _) = encounters::for_room(ctx.world, spot.0, spot.1)?;
            let place = def.name.to_lowercase();
            let dir = dir_word(spot.0 - ctx.rx, spot.1 - ctx.ry);
            let dist = (spot.0 - ctx.rx).abs().max((spot.1 - ctx.ry).abs());
            Some(Quest {
                kind: QuestKind::Clear { rx: spot.0, ry: spot.1, enc_name: def.name.to_string() },
                title: format!("TROUBLE TO THE {}", dir.to_uppercase()),
                goal: format!("Wipe out the {place}"),
                desc: format!("A {place} has been spotted to the {dir}. Clear them out for me."),
                reward: roll_reward("clear", dist, tier, rng),
                ..base
            })
        }
        "bounty" => {
            let min_r = MIN_RANGE + (rng.next_f64() * 7.0) as i32;
            let spot = find_room(ctx.rx, ctx.ry, min_r, ENC_RANGE, |x, y| !ctx.world.is_town(x, y))?;
            let kind = pick_avoid(&BOUNTY_KINDS, "bounty:", rng);
            let name = BOUNTY_NAMES[(rng.next_f64() * BOUNTY_NAMES.len() as f64) as usize];
            let dir = dir_word(spot.0 - ctx.rx, spot.1 - ctx.ry);
            let dist = (spot.0 - ctx.rx).abs().max((spot.1 - ctx.ry).abs());
            Some(Quest {
                kind: QuestKind::Bounty { rx: spot.0, ry: spot.1, kind: kind.to_string(), name: name.to_string() },
                title: format!("BOUNTY: {}", name.to_uppercase()),
                goal: format!("Slay {name}"),
                desc: format!("{name}, a monstrous {}, lairs to the {dir}. Put it down.", kind_name(kind).to_lowercase()),
                reward: roll_reward("bounty", dist, tier, rng),
                ..base
            })
        }
        "slay" => {
            let kind = pick_avoid(&SLAY_KINDS, "slay:", rng);
            let need = 5 + (tier as f64 * 1.5) as i32 + (rng.next_f64() * 4.0) as i32;
            Some(Quest {
                kind: QuestKind::Slay { kind: kind.to_string(), need, have: 0 },
                title: format!("CULL THE {}", kind_name(kind)),
                goal: format!("Slay {need} {}s", kind_name(kind).to_lowercase()),
                desc: format!("{}s have been a menace. Slay {need} of them, wherever you find them.", kind_name(kind)),
                reward: roll_reward("slay", 1 + tier, tier, rng),
                ..base
            })
        }
        _ => {
            // fetch (always buildable)
            let item = pick_avoid(&MATERIALS, "fetch:", rng);
            let name = crate::items::get(item).map_or(item, |d| d.name).to_uppercase();
            let need = 4 + (tier as f64 * 1.5) as i32 + (rng.next_f64() * 4.0) as i32;
            Some(Quest {
                kind: QuestKind::Fetch { item: item.to_string(), need },
                title: format!("GATHER {name}"),
                goal: format!("Bring {need} {}", name.to_lowercase()),
                desc: format!("I need {need} {}. Gather it from the wilds and bring it back.", name.to_lowercase()),
                reward: roll_reward("fetch", 1 + tier, tier, rng),
                ..base
            })
        }
    }
}

/// js Quests.generate — seeded type shuffle, fresh-types-first, first buildable wins.
pub fn generate(ctx: &GenCtx, giver_seed: u32, done: i32) -> Quest {
    let mut rng = rng_for(ctx.world.seed, giver_seed, done);
    let mut order = ["clear", "bounty", "slay", "fetch"];
    for i in (1..order.len()).rev() {
        let j = (rng.next_f64() * (i + 1) as f64) as usize;
        order.swap(i, j);
    }
    let active: Vec<&'static str> = ctx.log.iter().map(|q| q.type_key()).collect();
    order.sort_by_key(|t| active.contains(t)); // stable — fresh types first, shuffle kept within groups
    for t in order {
        if let Some(q) = build_type(t, ctx, &mut rng) {
            return q;
        }
    }
    build_type("fetch", ctx, &mut rng).expect("fetch always builds")
}

/// A giver's live glyph row: (glyph, glyph entity, plate entity, texture draw width, ink width).
type GlyphRow = (char, Entity, Entity, f32, f32);

/// The glyph over a giver, decided live (js giverGlyph): '!' new offer (log has room),
/// '-' in progress, GOLD '?' ready to hand in — the WoW read (Baz). ('-' stands in for
/// the js '·' — no font glyph.)
pub fn giver_glyph(
    world_seed: u32,
    v: &Villager,
    key: &str,
    log: &QuestLog,
    inv: &crate::inventory::PlayerInv,
) -> Option<(char, u32)> {
    if let Some(q) = log.0.iter().find(|q| q.giver_key == key) {
        return Some(if q.ready(inv) { ('?', 0xffd34d) } else { ('-', 0xb4b4bc) });
    }
    if is_giver(world_seed, v.seed) && log.0.iter().filter(|q| !q.is_story()).count() < QUEST_MAX {
        return Some(('!', 0xffd34d));
    }
    None
}

/// The stable per-giver key (js v.giverKey = "rx,ry,seed").
pub fn giver_key(rx: i32, ry: i32, seed: u32) -> String {
    format!("{rx},{ry},{seed}")
}

// --- Live systems -----------------------------------------------------------------

/// The horizontal offset of a frame's INK centre from its canvas centre — side
/// facings carry the body off-centre, and a glyph centred on the canvas hung
/// beside the head (Baz). Cached per frame image by the caller.
fn ink_center_off(img: Option<&Image>) -> f32 {
    let Some(img) = img else { return 0.0 };
    let w = img.size().x as usize;
    let data = img.data.as_deref().unwrap_or(&[]);
    let (mut min_x, mut max_x) = (usize::MAX, 0usize);
    for (i, px) in data.chunks_exact(4).enumerate() {
        if px[3] > 0 {
            let x = i % w;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
        }
    }
    if min_x == usize::MAX {
        return 0.0;
    }
    (min_x + max_x + 1) as f32 / 2.0 - w as f32 / 2.0
}

/// A SCALE-2 glyph on a dark plate (js drawQuestMarkers) — the WoW-style overhead
/// read; the scale-1 plateless glyph read as a stray speck. Shared with the story
/// thread's mark. Returns (glyph entity, plate entity, texture width, ink width) —
/// the texture pads odd bakes with a blank RIGHT column, so centring uses INK.
pub fn spawn_glyph_pair(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    ch: char,
    col: u32,
) -> (Entity, Entity, f32, f32) {
    let (img, w) = font::bake_text(&ch.to_string(), col, images);
    let iw2 = ((w + (w & 1)) * 2) as f32;
    let ink2 = (w * 2) as f32; // (Baz: '!' sat off-centre when centred by texture)
    let plate = commands
        .spawn((
            Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.5), Vec2::new(ink2 + 2.0, 12.0)),
            crate::gfx::at(0.0, -40.0, ink2 + 2.0, 12.0, crate::gfx::layers::PROMPT + 0.14),
            PIXEL_LAYER,
            GlyphSprite,
        ))
        .id();
    let mut spr = Sprite::from_image(img);
    spr.custom_size = Some(Vec2::new(iw2, 12.0)); // 2x the 6px bake, integer-crisp
    let ge = commands
        .spawn((
            spr,
            crate::gfx::at(0.0, -40.0, iw2, 12.0, crate::gfx::layers::PROMPT + 0.16),
            PIXEL_LAYER,
            GlyphSprite,
        ))
        .id();
    (ge, plate, iw2, ink2)
}

/// The floating '!' / '-' / '?' over quest givers, tracking them as they wander.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn giver_glyph_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    world: Res<GameWorld>,
    cur: Res<CurRoom>,
    sliding: Res<SlideActive>,
    log: Res<QuestLog>,
    inv: Res<crate::inventory::PlayerInv>,
    clock: Res<super::room_render::FrameClock>,
    story: Res<crate::app::story::StoryThread>,
    villagers: Query<(
        Entity,
        &Villager,
        &Sprite,
        bevy::ecs::query::Has<crate::app::story::StorySurvivor>,
        bevy::ecs::query::Has<crate::app::story::StoryElder>,
    )>,
    mut sprites: Query<(&mut Transform, &mut Visibility), (With<GlyphSprite>, Without<Villager>)>,
    // (char, glyph entity, plate entity, glyph width at 2x)
    mut live: Local<HashMap<Entity, GlyphRow>>,
    // Ink-centre offset per frame image (facing frames differ) — scanned once each.
    mut centers: Local<HashMap<bevy::asset::AssetId<Image>, f32>>,
) {
    // Mid-slide the villagers ride their room root but these are FREE sprites — they'd
    // hang at settled coords over nothing (the torch-light bug's cousin). Hide till land.
    if sliding.0 {
        for (_, mut vis) in &mut sprites {
            *vis = Visibility::Hidden;
        }
        return;
    }
    let mut seen: Vec<Entity> = Vec::new();
    for (ve, v, vsprite, is_survivor, is_elder) in &villagers {
        if v.pkey.is_none() {
            continue;
        }
        let key = giver_key(cur.rx, cur.ry, v.seed);
        // The story thread's mark outranks the board: the survivor's offer at
        // step 0, the crowned elder's handoff at step 1 (story.rs).
        let story_want = ((is_survivor && story.0 == 0) || (is_elder && story.0 == 1)).then_some(('!', 0xffd34d));
        let want = story_want.or_else(|| giver_glyph(world.0.seed, v, &key, &log, &inv));
        let have = live.get(&ve).map(|(c, g, p, w, i)| (*c, *g, *p, *w, *i));
        match (want, have) {
            (Some((ch, col)), have) if have.map(|(c, ..)| c) != Some(ch) => {
                if let Some((_, og, op, ..)) = have {
                    commands.entity(og).despawn();
                    commands.entity(op).despawn();
                }
                let (ge, plate, iw2, ink2) = spawn_glyph_pair(&mut commands, &mut images, ch, col);
                live.insert(ve, (ch, ge, plate, iw2, ink2));
            }
            (None, Some((_, og, op, ..))) => {
                commands.entity(og).despawn();
                commands.entity(op).despawn();
                live.remove(&ve);
            }
            _ => {}
        }
        if let Some((_, ge, plate, iw2, ink2)) = live.get(&ve).copied() {
            // Centred over their head, bobbing as they wander (js: sin(t/14 + x), -11 up).
            // The glyph centres by its INK — and over the FRAME's ink too (side
            // facings carry the body off-centre in the canvas).
            let body = *centers.entry(vsprite.image.id()).or_insert_with(|| ink_center_off(images.get(&vsprite.image)));
            let bob = (clock.0 as f32 / 14.0 + v.x).sin().round() - 11.0;
            let gx = (super::room_render::PLAY_X + v.x + 8.0 + body - ink2 / 2.0).round();
            let gy = (super::room_render::PLAY_Y + v.y.round() + bob).round();
            if let Ok((mut tf, mut vis)) = sprites.get_mut(ge) {
                *tf = crate::gfx::at(gx, gy, iw2, 12.0, crate::gfx::layers::PROMPT + 0.16);
                *vis = Visibility::Inherited;
            }
            if let Ok((mut tf, mut vis)) = sprites.get_mut(plate) {
                *tf = crate::gfx::at(gx - 1.0, gy - 1.0, ink2 + 2.0, 12.0, crate::gfx::layers::PROMPT + 0.14);
                *vis = Visibility::Inherited;
            }
        }
        seen.push(ve);
    }
    // Villagers that left (room change, despawn) take their glyph + plate along.
    live.retain(|ve, (_, ge, plate, ..)| {
        if seen.contains(ve) {
            true
        } else {
            commands.entity(*ge).despawn();
            commands.entity(*plate).despawn();
            false
        }
    });
}

/// Slay/bounty credit from the death systems (js onEnemyKilled).
fn quest_credit_tick(
    mut kills: MessageReader<KillCredit>,
    mut log: ResMut<QuestLog>,
    mut toasts: ResMut<super::rewards::LootLog>,
    mut sfx: MessageWriter<super::sfx::Sfx>,
) {
    for k in kills.read() {
        for q in &mut log.0 {
            let complete = match &mut q.kind {
                QuestKind::Bounty { .. } if !q.done && k.bounty == Some(q.id) => {
                    q.done = true;
                    true
                }
                QuestKind::Slay { kind, need, have } => {
                    // Slingers count as goblins (js e.type is 'goblin' for both).
                    let kk = if k.kind == "slinger" { "goblin" } else { k.kind };
                    if kk == kind && *have < *need {
                        *have += 1;
                        *have >= *need
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if complete {
                toasts.add("quest", &format!("QUEST READY: {}", q.goal.to_uppercase()), 1, 0xffd34d, false, true);
                sfx.write(super::sfx::Sfx("craft"));
            }
        }
    }
}

/// While a bounty is active, its named elite lairs in the marked room — respawning at
/// full strength each visit until slain (js loadRoomEntities' bounty block).
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn bounty_spawn_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut human_art: ResMut<crate::actors::goblin::HumanArt>,
    cur: Res<CurRoom>,
    sliding: Res<SlideActive>,
    inside: Res<super::interior::Inside>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    grid: Res<super::play::CurGrid>,
    blockers: Res<super::room_props::RoomBlockers>,
    log: Res<QuestLog>,
    mut last: Local<Option<(i32, i32)>>,
) {
    if sliding.0 || inside.0.is_some() || in_dungeon.0.is_some() {
        return;
    }
    let room = (cur.rx, cur.ry);
    if *last == Some(room) {
        return;
    }
    *last = Some(room);
    for q in &log.0 {
        let QuestKind::Bounty { rx, ry, kind, .. } = &q.kind else { continue };
        if q.done || (*rx, *ry) != room {
            continue;
        }
        // Bandits + goblins have no MobDef — they live on the goblin chassis. Without
        // this arm the `else { continue }` below left their bounty lair EMPTY and the
        // quest could never finish.
        let human = matches!(kind.as_str(), "bandit" | "goblin");
        let idx = mobs::def_index(kind);
        if idx.is_none() && !human {
            continue;
        }
        let (cx, cy) = ((crate::room::PX_W / 2 - 8) as f32, (crate::room::PX_H / 2 - 8) as f32);
        // js findClearSpot: spiral out from the room centre for ground the beast's own
        // hitbox can actually stand on — no lairing in the lake (Baz's swimming bear).
        let (hox, hoy, hw, hh) = idx.map_or((3.0, 4.0, 10.0, 10.0), |i| mobs::MOB_DEFS[i].hb);
        let clear = |x: f32, y: f32| {
            let b = (x + hox, y + hoy, hw, hh);
            !grid.0.box_hits_solid(b.0, b.1, b.2, b.3)
                && !blockers.0.iter().any(|r| b.0 < r.0 + r.2 && b.0 + b.2 > r.0 && b.1 < r.1 + r.3 && b.1 + b.3 > r.1)
        };
        let mut spot = (cx, cy);
        'search: for rad in 0..=9i32 {
            for dy in -rad..=rad {
                for dx in -rad..=rad {
                    if dx.abs().max(dy.abs()) != rad {
                        continue; // ring perimeter only, nearest ground wins
                    }
                    let (x, y) = (cx + (dx * 16) as f32, cy + (dy * 16) as f32);
                    if clear(x, y) {
                        spot = (x, y);
                        break 'search;
                    }
                }
            }
        }
        let mut e = if let Some(idx) = idx {
            commands.spawn((mob_bundle(idx, spot.0, spot.1), RoomActor, PIXEL_LAYER, BountyTag(q.id)))
        } else {
            // The goblin-chassis elite; a bandit target wears its person-in-costume skin.
            let mut e = commands.spawn((
                crate::actors::goblin::goblin_bundle(crate::actors::goblin::GoblinKind::Melee, spot.0, spot.1),
                RoomActor,
                PIXEL_LAYER,
                BountyTag(q.id),
            ));
            e.insert(Sprite::default());
            if *kind == "bandit" {
                let seed = q.id.wrapping_mul(2654435761) ^ 0xba9d;
                let frames = human_art.frames("bandit", seed, &mut images);
                e.insert(crate::actors::goblin::HumanSkin { kind: "bandit", seed, frames });
            }
            e
        };
        e.entry::<Health>().and_modify(move |mut h| {
            h.hp *= 4; // the elite bump (js makeElite; affix auras join later)
            h.max *= 4;
        });
    }
}

#[derive(Component)]
pub struct GlyphSprite;

/// The bounty's NAME floats over its elite (js eliteName), tracking it as it prowls.
#[allow(clippy::too_many_arguments)] // ECS system params are wide by nature
fn bounty_name_tick(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    log: Res<QuestLog>,
    sliding: Res<SlideActive>,
    bounties: Query<(Entity, &BountyTag, &crate::actors::mobs::Mob)>,
    gob_bounties: Query<(Entity, &BountyTag, &crate::actors::goblin::Goblin), Without<crate::actors::mobs::Mob>>,
    mut sprites: Query<(&mut Transform, &mut Visibility), With<GlyphSprite>>,
    mut live: Local<HashMap<Entity, (Entity, f32)>>,
) {
    if sliding.0 {
        for (_, mut vis) in &mut sprites {
            *vis = Visibility::Hidden;
        }
        return;
    }
    let mut seen: Vec<Entity> = Vec::new();
    // Both chassis carry names: real MobDef elites AND goblin-frame ones (bandit/goblin).
    let all: Vec<(Entity, u32, f32, f32)> = bounties
        .iter()
        .map(|(e, t, m)| (e, t.0, m.x, m.y))
        .chain(gob_bounties.iter().map(|(e, t, g)| (e, t.0, g.x, g.y)))
        .collect();
    for (me, tag, mx, my) in all {
        if !live.contains_key(&me) {
            let name = log
                .0
                .iter()
                .find_map(|q| match &q.kind {
                    QuestKind::Bounty { name, .. } if q.id == tag => Some(name.to_uppercase()),
                    _ => None,
                })
                .unwrap_or_else(|| "BOUNTY".into());
            let (img, w) = font::bake_text(&name, 0xffd34d, &mut images);
            let iw = (w + (w & 1)) as f32;
            let ge = commands
                .spawn((
                    Sprite::from_image(img),
                    crate::gfx::at(0.0, -40.0, iw, 6.0, crate::gfx::layers::PROMPT),
                    PIXEL_LAYER,
                    GlyphSprite,
                ))
                .id();
            live.insert(me, (ge, iw));
        }
        if let Some((ge, iw)) = live.get(&me)
            && let Ok((mut tf, mut vis)) = sprites.get_mut(*ge)
        {
            let x = super::room_render::PLAY_X + mx + 8.0 - iw / 2.0;
            let y = super::room_render::PLAY_Y + my - 10.0;
            *tf = crate::gfx::at(x.round(), y.round(), *iw, 6.0, crate::gfx::layers::PROMPT);
            *vis = Visibility::Inherited;
        }
        seen.push(me);
    }
    live.retain(|me, (ge, _)| {
        if seen.contains(me) {
            true
        } else {
            commands.entity(*ge).despawn();
            false
        }
    });
}

pub struct QuestsPlugin;

impl Plugin for QuestsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestLog>()
            .init_resource::<GiverDone>()
            .init_resource::<QuestCounter>()
            .add_message::<KillCredit>()
            .add_systems(
                bevy::app::FixedUpdate,
                (quest_credit_tick, bounty_spawn_tick).run_if(playing),
            )
            .add_systems(Update, (giver_glyph_tick, bounty_name_tick).run_if(playing));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic_and_dedups() {
        let world = World::new(1337);
        let cleared = ClearedEncounters::default();
        let ctx = GenCtx { world: &world, cleared: &cleared, log: &[], rx: -4, ry: -10 };
        let a = generate(&ctx, 12345, 0);
        let b = generate(&ctx, 12345, 0);
        assert_eq!(a.title, b.title, "same giver + done count -> same offer");
        assert_eq!(a.sig(), b.sig());
        let c = generate(&ctx, 12345, 1);
        // A different completed count reseeds the stream (usually a different job).
        assert!(c.reward.coin > 0 && a.reward.coin > 0);
        // With the first quest active, the next offer avoids its exact signature.
        let log = vec![a.clone()];
        let ctx2 = GenCtx { world: &world, cleared: &cleared, log: &log, rx: -4, ry: -10 };
        let d = generate(&ctx2, 999, 0);
        assert_ne!(d.sig(), a.sig(), "no two quests at the same target");
    }

    #[test]
    fn giver_split_matches_pct() {
        // ~45% of seeds are givers; sanity band over 1000 samples.
        let n = (0..1000u32).filter(|s| is_giver(1337, s.wrapping_mul(2654435761))).count();
        assert!((350..550).contains(&n), "giver rate off: {n}/1000");
    }

    #[test]
    fn slay_and_fetch_track_progress() {
        let mut q = Quest {
            id: 1,
            kind: QuestKind::Slay { kind: "wolf".into(), need: 3, have: 0 },
            done: false,
            title: String::new(),
            goal: "Slay 3 dire wolfs".into(),
            desc: String::new(),
            reward: Reward::default(),
            giver_key: String::new(),
            giver_rx: 0,
            giver_ry: 0,
        };
        let inv = crate::inventory::PlayerInv::default();
        assert!(!q.ready(&inv));
        if let QuestKind::Slay { have, .. } = &mut q.kind {
            *have = 3;
        }
        assert!(q.ready(&inv));
        assert_eq!(q.progress_text(3), "DIRE WOLF 3/3");
    }
}
