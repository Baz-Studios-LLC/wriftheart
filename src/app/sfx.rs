//! sfx.rs — the sound EVENT BUS (the architecture decision from the 2026-07-16 review).
//! The js calls `Sound.sfx('coin')` inline at ~200 sites; porting audio that way would
//! re-touch every system. Instead: gameplay systems emit [`Sfx`] messages by js sound KEY,
//! and the audio port (`app/audio` — AudioPlugin/GameAudioPlugin) is the ONE consumer that
//! bakes + plays the voices. Keys are the js names verbatim ('coin', 'tink', 'swing', …).

use bevy::prelude::*;

/// One sound request, by js sound key. Fire-and-forget from any system:
/// `sfx.write(Sfx("coin"))`.
#[derive(Message)]
pub struct Sfx(pub &'static str);

pub struct SfxPlugin;

impl Plugin for SfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Sfx>().add_systems(Last, drain);
    }
}

/// Safety net: drain any Sfx a reader missed so the bus never piles up (app/audio is the
/// real consumer, with its own reader cursor — this just keeps the queue from growing if
/// audio is ever disabled/absent).
fn drain(mut msgs: MessageReader<Sfx>) {
    for _ in msgs.read() {}
}
