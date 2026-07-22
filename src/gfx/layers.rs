//! layers.rs — THE Z-LADDER, named. Every band the game draws in, top to bottom of
//! this file = back to front on screen. New sprites take a constant from here (or a
//! documented offset from one); raw z literals are legacy and migrate as files are
//! touched. The in-room ACTOR band (1..~12) is dynamic — see `room_render::actor_z` —
//! so only its ceiling is named here.
//!
//! Ladder (from PORT.md, now code): tiles 1 / dressing 2 / props 3.x / actors 4-8 /
//! FX 8.5-12 / overlays below.

/// The living-water surface overlay: over the water tiles (1), under bridge decks (1.5).
pub const WATER_OVERLAY: f32 = 1.3;
/// Actor reflections in water: over the surface overlay, still under the decks.
pub const REFLECTION: f32 = 1.4;
/// Ground shadows under EVERY actor (the blob band): over tiles/dressing/clutter,
/// under the whole depth-sorted actor range (4..8).
pub const SHADOW: f32 = 3.95;
/// Room FX above the actors: particles, bursts (battle/fx.rs spawns at 12.0).
pub const FX: f32 = 12.0;
/// The firefly glow drifting over the meadow.
pub const FIREFLY: f32 = 12.4;
// --- UI IS NEVER SUBDUED (Baz): every band below carries interface, not world —
// name chips, prompts, quest marks, hearts — so they ALL live above the darkness
// (LIGHTING 13.0) and the weather pass (13.2). Nothing UI may sit below those two.
/// The heart that drifts up when a villager warms to you.
pub const HEART_FX: f32 = 13.24;
/// The villager name chip + speech bubble (bubble text at +0.05). ABOVE the
/// prompt band (13.36-13.42) and the quest ! glyphs (13.50-13.52): live speech
/// owns its airspace — the town garble was the TALK plate and a giver's ! both
/// carving through a lower, see-through bubble (Baz).
pub const CHAT: f32 = 13.56;
pub const CHAT_TEXT: f32 = 13.62;
/// The interact prompts: the floating "F ENTER" bubble and the bottom-centre bar
/// (border +0.02, text +0.05).
pub const PROMPT: f32 = 13.36;
pub const PROMPT_TEXT: f32 = 13.42;
/// The death scene's washes + corpse (grey 12.85 / dark 12.9 / pool 12.95 / corpse 12.97).
pub const DEATH_SCENE: f32 = 12.85;
/// The day/night darkness overlay — covers the play field + canopy strip.
pub const LIGHTING: f32 = 13.0;
/// The flute overlay band (rose, staff, sparks, halo, banner): over the darkness AND
/// the weather pass (13.2) — the tune owns the screen while you play — under the
/// loot feed. Internally spans -0.05..+0.03 around this base.
pub const FLUTE_UI: f32 = 13.3;
/// The loot-toast feed, over the dark (js draws it after lighting).
pub const LOOT_FEED: f32 = 13.5;
/// Town / region / interior announcement banners.
pub const BANNERS: f32 = 15.5;
/// Centred windows over the frozen world: the shop, the chooser, the gift picker.
/// One popup at a time — they share the band (chrome +0.01..+0.04).
pub const WINDOW: f32 = 16.5;
/// The slide-out panel band (16.x internally; below every HUD element).
pub const SLIDEOUT: f32 = 16.0;
/// The sidebar HUD band (17.2 .. 18.7 internally).
pub const HUD: f32 = 17.2;
/// The YOU DIED text block.
pub const DEATH_TEXT: f32 = 18.8;
/// The sleep fade (shade, Z Z Z at +0.01) — over the HUD, under the menus.
pub const SLEEP: f32 = 18.85;
/// The codex band (18.9 .. 19.8 internally).
pub const CODEX: f32 = 18.9;
/// Title UI 18.9-19.2, creator 19.3-19.6 (own screens; they never stack with codex).
pub const TITLE_UI: f32 = 18.9;
/// The pause menu band (19.85 .. 20.2 internally) — the topmost MENU.
pub const PAUSE: f32 = 19.85;
/// The level-up flourish rides over EVERYTHING (Baz: it never hides behind a menu).
pub const FLOURISH: f32 = 20.5;
