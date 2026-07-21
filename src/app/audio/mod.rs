//! audio/ — the js soundscape, LANDED (audio.js port). The sfx event bus (sfx.rs)
//! has been emitting js sound keys since the architecture review; this module is
//! the consumer that finally gives them voices. Everything is baked OFFLINE at
//! startup by synth.rs (the js WebAudio math as pure DSP) into in-memory WAVs:
//! ~35 sfx recipes, the four flute notes, and the three MUSIC LOOPS (overworld
//! march / dungeon vamp / town pastoral — authored note-strings copied verbatim,
//! drums and all, rendered with wrap-around so the loop seam is silent).
//! Track choice follows the js: town music in towns (and the guildhall when it
//! ports), dungeon action underground, overworld everywhere else (title included).
//! Jingles duck the music exactly like js duckMusic. DEVIATION (flagged): track
//! switches are hard cuts (js re-seats its scheduler mid-bar too). The flute's HELD
//! ocarina voice (js noteOn/noteOff) is REAL now: seamless per-note loops + sink
//! envelopes in flute_hold_tick, driven off the flute's held state.

mod synth;
mod tracks;

use bevy::audio::{AudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, PlaybackSettings, Volume};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use synth::{noise, tone, Buf, Filter, Wave};

/// js gain staging: master 0.5, sfxGain 0.9, musicGain 0.38 — baked into the PCM.
const SFX_GAIN: f32 = 0.9 * 0.5;
const MUSIC_GAIN: f32 = 0.38 * 0.5;
/// duckMusic floor as a sink multiplier (0.05 over the baked 0.38).
const DUCK: f32 = 0.05 / 0.38;

#[derive(Resource, Default)]
pub struct SfxBank(pub HashMap<&'static str, Handle<AudioSource>>);

#[derive(Resource, Default)]
pub struct MusicBank(pub HashMap<&'static str, Handle<AudioSource>>);

/// The four held-flute loop bodies, indexed like songs::NOTES (U D L R).
#[derive(Resource, Default)]
pub struct NoteHoldBank(pub [Handle<AudioSource>; 4]);

/// One live held-flute voice (js fluteVoice): its looping sink + envelope level.
#[derive(Component)]
struct NoteHold {
    idx: usize,
    level: f32,
}

#[derive(Resource, Default)]
pub struct MusicState {
    current: Option<&'static str>,
    entity: Option<Entity>,
    duck: i32,
}

pub struct AudioPlugin;
impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SfxBank>()
            .init_resource::<MusicBank>()
            .init_resource::<NoteHoldBank>()
            .init_resource::<MusicState>()
            .add_systems(Startup, bake)
            .add_systems(Update, finish_bake)
            .add_systems(Update, (play_sfx, music_tick, flute_hold_tick));
    }
}

/// Every js sfx recipe (audio.js sfx switch), rendered to samples. The four flute
/// notes ride along as noteU/noteD/noteL/noteR (songs.js pitches).
fn render_sfx(key: &str) -> Option<Vec<f32>> {
    let mut b = Buf::secs(2.6, false);
    match key {
        "swing" => noise(&mut b, 0.0, 0.12, 0.22, Filter::Highpass, 900.0),
        "hit" => {
            noise(&mut b, 0.0, 0.10, 0.45, Filter::Lowpass, 520.0);
            tone(&mut b, 0.0, 190.0, 0.10, Wave::Square, 0.18, Some(90.0));
        }
        "enemyDie" => {
            tone(&mut b, 0.0, 320.0, 0.20, Wave::Square, 0.28, Some(80.0));
            noise(&mut b, 0.0, 0.22, 0.35, Filter::Lowpass, 700.0);
        }
        "hurt" => {
            tone(&mut b, 0.0, 340.0, 0.20, Wave::Sawtooth, 0.32, Some(110.0));
            noise(&mut b, 0.0, 0.12, 0.3, Filter::Lowpass, 800.0);
        }
        "wood" => {
            tone(&mut b, 0.0, 150.0, 0.08, Wave::Square, 0.28, Some(105.0));
            noise(&mut b, 0.0, 0.05, 0.18, Filter::Lowpass, 600.0);
        }
        "stone" => {
            tone(&mut b, 0.0, 520.0, 0.04, Wave::Square, 0.14, Some(760.0));
            noise(&mut b, 0.0, 0.06, 0.32, Filter::Bandpass, 2600.0);
        }
        "leaf" => noise(&mut b, 0.0, 0.10, 0.28, Filter::Highpass, 3200.0),
        "tink" => {
            tone(&mut b, 0.0, 1500.0, 0.05, Wave::Square, 0.12, Some(1900.0));
            tone(&mut b, 0.04, 1150.0, 0.05, Wave::Square, 0.08, None);
        }
        "coin" => {
            tone(&mut b, 0.0, 988.0, 0.06, Wave::Square, 0.22, None);
            tone(&mut b, 0.06, 1319.0, 0.10, Wave::Square, 0.22, None);
        }
        "pickup" => tone(&mut b, 0.0, 660.0, 0.05, Wave::Triangle, 0.22, Some(990.0)),
        "craft" => {
            tone(&mut b, 0.0, 300.0, 0.06, Wave::Square, 0.22, None);
            tone(&mut b, 0.06, 450.0, 0.06, Wave::Square, 0.22, None);
            tone(&mut b, 0.12, 620.0, 0.12, Wave::Square, 0.22, None);
        }
        "levelup" => {
            for (i, f) in [523.0, 659.0, 784.0, 1047.0, 1319.0].into_iter().enumerate() {
                tone(&mut b, i as f32 * 0.10, f, 0.24, Wave::Square, 0.28, None);
            }
            tone(&mut b, 0.46, 1568.0, 0.5, Wave::Triangle, 0.22, None);
        }
        "menuMove" => tone(&mut b, 0.0, 480.0, 0.03, Wave::Square, 0.10, None),
        "menuConfirm" => tone(&mut b, 0.0, 660.0, 0.05, Wave::Square, 0.18, Some(900.0)),
        "open" => tone(&mut b, 0.0, 320.0, 0.05, Wave::Square, 0.14, Some(520.0)),
        "warpCharge" => {
            tone(&mut b, 0.0, 180.0, 1.3, Wave::Triangle, 0.16, Some(760.0));
            tone(&mut b, 0.0, 270.0, 1.3, Wave::Sine, 0.10, Some(1140.0));
            noise(&mut b, 0.0, 0.4, 0.10, Filter::Highpass, 2400.0);
        }
        "warpTick" => {
            tone(&mut b, 0.0, 1200.0, 0.05, Wave::Sine, 0.10, Some(1700.0));
            tone(&mut b, 0.03, 1600.0, 0.05, Wave::Sine, 0.07, Some(2200.0));
        }
        "warpGo" => {
            noise(&mut b, 0.0, 0.22, 0.40, Filter::Highpass, 1400.0);
            tone(&mut b, 0.0, 440.0, 0.32, Wave::Sawtooth, 0.26, Some(2200.0));
            for (i, f) in [1047.0, 1319.0, 1568.0].into_iter().enumerate() {
                tone(&mut b, 0.08 + i as f32 * 0.04, f, 0.4, Wave::Triangle, 0.18, None);
            }
        }
        "warpFail" => {
            tone(&mut b, 0.0, 620.0, 0.34, Wave::Sawtooth, 0.22, Some(120.0));
            noise(&mut b, 0.0, 0.2, 0.22, Filter::Lowpass, 520.0);
        }
        "sleep" => {
            for (i, f) in [392.0, 330.0, 294.0, 262.0].into_iter().enumerate() {
                tone(&mut b, i as f32 * 0.16, f, 0.34, Wave::Triangle, 0.16, None);
            }
        }
        "wake" => {
            for (i, f) in [392.0, 523.0, 659.0].into_iter().enumerate() {
                tone(&mut b, i as f32 * 0.13, f, 0.22, Wave::Triangle, 0.15, None);
            }
            tone(&mut b, 0.4, 784.0, 0.4, Wave::Sine, 0.12, None);
        }
        "block" => {
            tone(&mut b, 0.0, 220.0, 0.07, Wave::Square, 0.28, Some(130.0));
            noise(&mut b, 0.0, 0.05, 0.24, Filter::Lowpass, 480.0);
            tone(&mut b, 0.03, 900.0, 0.05, Wave::Square, 0.10, Some(1400.0));
        }
        "thunder" => {
            noise(&mut b, 0.0, 0.16, 0.34, Filter::Highpass, 2200.0);
            noise(&mut b, 0.04, 0.6, 0.5, Filter::Lowpass, 300.0);
            tone(&mut b, 0.04, 64.0, 0.7, Wave::Sine, 0.24, Some(36.0));
            tone(&mut b, 0.12, 44.0, 0.55, Wave::Triangle, 0.14, Some(28.0));
        }
        "heartbeat" => {
            tone(&mut b, 0.0, 52.0, 0.10, Wave::Sine, 0.42, Some(38.0));
            tone(&mut b, 0.13, 40.0, 0.14, Wave::Sine, 0.30, Some(30.0));
        }
        "cast" => {
            noise(&mut b, 0.0, 0.12, 0.16, Filter::Highpass, 1700.0);
            tone(&mut b, 0.14, 280.0, 0.08, Wave::Sine, 0.14, Some(150.0));
        }
        "splash" => {
            noise(&mut b, 0.0, 0.14, 0.24, Filter::Bandpass, 820.0);
            tone(&mut b, 0.0, 220.0, 0.10, Wave::Sine, 0.12, Some(360.0));
        }
        "reel" => {
            tone(&mut b, 0.0, 523.0, 0.06, Wave::Square, 0.20, None);
            tone(&mut b, 0.07, 740.0, 0.10, Wave::Square, 0.20, None);
            tone(&mut b, 0.16, 988.0, 0.12, Wave::Triangle, 0.16, None);
        }
        "itemget" => {
            for (i, f) in [523.0, 659.0, 784.0, 1047.0].into_iter().enumerate() {
                tone(&mut b, i as f32 * 0.11, f, 0.15, Wave::Square, 0.26, None);
            }
            tone(&mut b, 0.46, 1047.0, 0.6, Wave::Triangle, 0.20, None);
            tone(&mut b, 0.46, 1319.0, 0.6, Wave::Triangle, 0.24, None);
            tone(&mut b, 0.60, 1568.0, 0.4, Wave::Square, 0.12, None);
        }
        "dig" => {
            tone(&mut b, 0.0, 110.0, 0.09, Wave::Square, 0.22, Some(70.0));
            noise(&mut b, 0.0, 0.14, 0.30, Filter::Lowpass, 460.0);
            noise(&mut b, 0.08, 0.08, 0.14, Filter::Bandpass, 1400.0);
        }
        "cluck" => {
            tone(&mut b, 0.0, 880.0, 0.05, Wave::Square, 0.12, Some(620.0));
            tone(&mut b, 0.07, 740.0, 0.04, Wave::Square, 0.09, Some(560.0));
        }
        "moo" => {
            tone(&mut b, 0.0, 196.0, 0.16, Wave::Triangle, 0.16, Some(150.0));
            tone(&mut b, 0.12, 147.0, 0.34, Wave::Triangle, 0.15, Some(110.0));
        }
        "songmatch" => {
            for (i, f) in [1047.0, 1319.0, 1568.0, 2093.0].into_iter().enumerate() {
                tone(&mut b, i as f32 * 0.05, f, 0.22, Wave::Triangle, 0.16, None);
            }
            noise(&mut b, 0.0, 0.25, 0.08, Filter::Highpass, 3400.0);
        }
        "bellring" => {
            for (i, f) in [262.0, 524.0, 629.0, 786.0].into_iter().enumerate() {
                let (w, v) = if i == 0 { (Wave::Triangle, 0.30) } else { (Wave::Sine, 0.10) };
                tone(&mut b, 0.0, f, 1.3 - i as f32 * 0.2, w, v, Some(f * 0.99));
            }
            noise(&mut b, 0.0, 0.08, 0.18, Filter::Highpass, 2600.0);
            tone(&mut b, 0.5, 262.0, 0.8, Wave::Sine, 0.12, Some(259.0));
        }
        // The flute's four notes (songs.js pitches; js note() voice).
        "noteU" => synth::flute_note(&mut b, 0.0, 440.0, 0.30),
        "noteD" => synth::flute_note(&mut b, 0.0, 261.63, 0.30),
        "noteL" => synth::flute_note(&mut b, 0.0, 293.66, 0.30),
        "noteR" => synth::flute_note(&mut b, 0.0, 392.0, 0.30),
        // The held voice's attack breath (js noteOn's chiff) — the tone itself loops.
        "noteChiff" => noise(&mut b, 0.0, 0.05, 0.05, Filter::Highpass, 3000.0),
        _ => return None,
    }
    Some(b.data)
}

/// Bake every voice the game can ask for (a few hundred ms of CPU, once).
const SFX_KEYS: [&str; 38] = [
    "swing", "hit", "enemyDie", "hurt", "wood", "stone", "leaf", "tink", "coin", "pickup", "craft", "levelup",
    "menuMove", "menuConfirm", "open", "warpCharge", "warpTick", "warpGo", "warpFail", "sleep", "wake", "block",
    "thunder", "heartbeat", "cast", "splash", "reel", "itemget", "dig", "cluck", "moo", "songmatch", "bellring",
    "noteU", "noteD", "noteL", "noteR", "noteChiff",
];

/// The finished off-thread bake: every voice pre-encoded to WAV bytes.
struct Baked {
    sfx: Vec<(&'static str, Vec<u8>)>,
    holds: [Vec<u8>; 4],
    music: Vec<(&'static str, Vec<u8>)>,
}

/// The bake in flight (removed once the banks fill).
#[derive(Resource)]
struct BakeJob(std::sync::Mutex<std::sync::mpsc::Receiver<Baked>>);

/// Kick the synthesis onto a BACKGROUND thread — it renders ~38 sfx + the full music
/// loops (seconds of DSP), which used to run inside Startup and froze the boot before
/// the first frame could present (Baz: "can we jump right into the game?"). The banks
/// fill a beat after the title appears; every consumer already tolerates a missing key
/// (play_sfx skips, music_tick retries each tick until its track exists).
fn bake(mut commands: Commands) {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let sfx = SFX_KEYS
            .iter()
            .filter_map(|k| render_sfx(k).map(|s| (*k, synth::wav_bytes(s, SFX_GAIN))))
            .collect();
        // The four HELD-voice loops (js noteOn's steady state) — perfectly seamless
        // bodies (wav_loop: trimming the tail would break the seam and warble on wrap).
        let holds = [440.0, 261.63, 293.66, 392.0].map(|f| synth::wav_loop(synth::note_hold_loop(f), SFX_GAIN));
        // wav_LOOP for music too: a loop's length is musically exact (sixteenths * beat)
        // and the render WRAPS note tails across the seam — trimming would repeat EARLY,
        // by a different amount per track (the "different speeds" bug).
        let music = tracks::render_all().into_iter().map(|(n, s)| (n, synth::wav_loop(s, MUSIC_GAIN))).collect();
        let _ = tx.send(Baked { sfx, holds, music });
    });
    commands.insert_resource(BakeJob(std::sync::Mutex::new(rx)));
}

/// Collect the finished bake into the banks (one try_recv a frame until it lands).
fn finish_bake(
    mut commands: Commands,
    job: Option<Res<BakeJob>>,
    mut sfx_bank: ResMut<SfxBank>,
    mut music_bank: ResMut<MusicBank>,
    mut hold_bank: ResMut<NoteHoldBank>,
    mut sources: ResMut<Assets<AudioSource>>,
) {
    let Some(job) = job else { return };
    let done = match job.0.lock() {
        Ok(rx) => rx.try_recv().ok(),
        Err(_) => None,
    };
    let Some(b) = done else { return };
    for (key, bytes) in b.sfx {
        sfx_bank.0.insert(key, sources.add(AudioSource { bytes: bytes.into() }));
    }
    for (i, bytes) in b.holds.into_iter().enumerate() {
        hold_bank.0[i] = sources.add(AudioSource { bytes: bytes.into() });
    }
    for (name, bytes) in b.music {
        music_bank.0.insert(name, sources.add(AudioSource { bytes: bytes.into() }));
    }
    commands.remove_resource::<BakeJob>();
}

/// The bus consumer: every Sfx key plays its baked voice (fire-and-forget entities).
fn play_sfx(
    mut commands: Commands,
    bank: Res<SfxBank>,
    mut state: ResMut<MusicState>,
    mut msgs: MessageReader<super::sfx::Sfx>,
) {
    for m in msgs.read() {
        if let Some(handle) = bank.0.get(m.0) {
            commands.spawn((AudioPlayer(handle.clone()), PlaybackSettings::DESPAWN));
        }
        // js duckMusic: the jingles push the music under themselves.
        let duck = match m.0 {
            "itemget" => 75,
            "bellring" => 84,
            "songmatch" => 60,
            _ => 0,
        };
        state.duck = state.duck.max(duck);
    }
}

/// The js noteOn/noteOff HELD flute voice, driven straight off the flute's live held
/// state (no messages to leak): a held arrow spawns its seamless looping tone — the
/// attack is a 2-frame sink ramp (the js 0.02s) plus the breath chiff — and letting
/// go (or the flute closing, catching a song, warping, anything that drops the held
/// flag or the whole Fluting) fades it out over ~0.12s and reclaims the entity.
/// Monophonic like the js: flute.rs steals all other held flags on a new press.
fn flute_hold_tick(
    mut commands: Commands,
    fluting: Option<Res<crate::app::flute::Fluting>>,
    hold_bank: Res<NoteHoldBank>,
    sfx_bank: Res<SfxBank>,
    mut voices: Query<(Entity, &mut NoteHold, Option<&mut AudioSink>)>,
) {
    let held = fluting.as_ref().and_then(|f| f.0.as_ref()).map(|f| f.held).unwrap_or([false; 4]);
    let mut live = [false; 4];
    for (e, mut v, sink) in &mut voices {
        live[v.idx] = true;
        if held[v.idx] {
            v.level = (v.level + 0.5).min(1.0); // the js 0.02s exponential attack, as frames
        } else {
            v.level -= 0.15; // the js 0.12s breath-off release
            if v.level <= 0.0 {
                commands.entity(e).despawn();
                continue;
            }
        }
        if let Some(mut sink) = sink {
            sink.set_volume(Volume::Linear(v.level));
        }
    }
    for i in 0..4 {
        if held[i] && !live[i] {
            commands.spawn((
                AudioPlayer(hold_bank.0[i].clone()),
                PlaybackSettings { volume: Volume::Linear(0.0), ..PlaybackSettings::LOOP },
                NoteHold { idx: i, level: 0.0 },
            ));
            if let Some(h) = sfx_bank.0.get("noteChiff") {
                commands.spawn((AudioPlayer(h.clone()), PlaybackSettings::DESPAWN));
            }
        }
    }
}

/// Marker on the looping music entity.
#[derive(Component)]
struct MusicLoop;

/// The track picker (js setTrack call sites + the rs variety pass — Baz: "more
/// songs in other places"): a live BOSS BAR takes the war-drums anywhere; the
/// Black Castle gets its own processional; the guildhall stays peaceful; dungeons
/// vamp; towns rest; and the open world varies — crystalline in the arctic, a
/// grinding dread in the corrupted lands, a nocturne after dark, the march
/// otherwise (title included). Hard cut on change; jingle ducking via sink volume.
#[allow(clippy::too_many_arguments)]
fn music_tick(
    mut commands: Commands,
    bank: Res<MusicBank>,
    mut state: ResMut<MusicState>,
    in_dungeon: Res<super::dungeon::InDungeon>,
    world: Res<super::play::GameWorld>,
    cur: Res<super::play::CurRoom>,
    clock: Res<super::room_render::FrameClock>,
    bosses: Query<(), With<super::boss::BossName>>,
    mut sinks: Query<&mut AudioSink, With<MusicLoop>>,
) {
    let want: &'static str = if !bosses.is_empty() {
        "boss"
    } else if let Some(run) = &in_dungeon.0 {
        if run.is_final {
            "finale"
        } else if run.dungeon.theme.key == "guildhall" {
            "town"
        } else {
            "dungeon"
        }
    } else if world.0.is_town(cur.rx, cur.ry) {
        "town"
    } else {
        match world.0.biome_key_at(cur.rx, cur.ry) {
            "arctic" => "frost",
            "graveyard" | "burnt" | "chaos" => "dread",
            _ if super::lighting::day_darkness(clock.0) > 0.72 => "night",
            _ => "overworld",
        }
    };
    if state.current != Some(want)
        && let Some(handle) = bank.0.get(want)
    {
        if let Some(e) = state.entity.take() {
            commands.entity(e).despawn();
        }
        state.entity = Some(commands.spawn((AudioPlayer(handle.clone()), PlaybackSettings::LOOP, MusicLoop)).id());
        state.current = Some(want);
    }
    if state.duck > 0 {
        state.duck -= 1;
    }
    let target = if state.duck > 0 { DUCK } else { 1.0 };
    for mut sink in &mut sinks {
        sink.set_volume(Volume::Linear(target));
    }
}
