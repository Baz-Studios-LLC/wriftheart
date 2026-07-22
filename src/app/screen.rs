//! screen.rs — WHICH full-screen mode the game is in, as a real state machine.
//!
//! The JS tracked this with a pile of booleans (`mapOpen`, `paused`, `ui.mode` strings…) that
//! every draw/update path re-checked; adding a screen meant auditing all of them. Here it's
//! one Bevy `States` enum: gameplay runs `in_state(Screen::Play)`, each screen owns its
//! open/close, and a new screen is a new variant. (Improve-don't-copy: see PORT.md.)

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Screen {
    /// The start menu (js gameState 'title') — the game BOOTS here; the world spawns
    /// beneath it, frozen, and the title's opaque flyover covers it.
    #[default]
    Title,
    /// The settings-only pause panel opened from the title's OPTIONS row.
    TitleOptions,
    /// The character creator (js gameState 'creator') — NEW GAME passes through here.
    Creator,
    /// The YOU DIED sequence (js `death`) — world frozen under the fade.
    Dead,
    Play,
    Pause,
    Codex,
    /// The DEV PANEL (rebuilt from scratch — the js overlay was a garbled strip).
    Dev,
    SlideOut,
    /// The vendor buy/sell window (js `shopOpen`) — opened at a shop counter.
    Shop,
    /// The home storage chest (js `storageOpen`) — a two-pane bag<->chest bank.
    Storage,
    /// A small centred dialog (js `choiceMenu`/`giftPick`) — the action chooser and
    /// the gift picker share it; the world freezes underneath.
    Dialog,
}

/// Run condition: the world simulates (player tick, battle chain). Any non-Play screen
/// freezes it — the JS behaviour for both the pause menu and the codex. Sleep freezes
/// it too (js: `if (sleeping) { updateSleep(); return; }`).
pub fn playing(
    state: Res<State<Screen>>,
    sleeping: Option<Res<super::services::Sleeping>>,
    fanfare: Option<Res<super::fanfare::Fanfare>>,
    shard_rite: Option<Res<super::shard_fanfare::ShardFanfare>>,
) -> bool {
    *state.get() == Screen::Play
        && sleeping.is_none_or(|s| s.0.is_none())
        && fanfare.is_none_or(|f| f.0.is_none()) // the item-get cutscene freezes the world too (js)
        && shard_rite.is_none_or(|f| f.0.is_none()) // ...and so does the shard rite
}
