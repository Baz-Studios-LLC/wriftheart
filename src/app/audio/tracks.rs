//! audio/tracks.rs — the three authored music loops (audio.js), note-strings copied
//! VERBATIM and rendered offline into seamless loop buffers (the Buf wraps, so a
//! tail ringing past the end folds onto the top of the loop — no seam).

use super::synth::{hat, kick, music_tone, snare, Buf, Wave, SR};

/// js NOTE table (name -> Hz).
fn hz(name: &str) -> f32 {
    match name {
        "C2" => 65.0, "D2" => 73.0, "E2" => 82.0, "F2" => 87.0, "G2" => 98.0, "A2" => 110.0, "B2" => 123.0,
        "C3" => 131.0, "D3" => 147.0, "E3" => 165.0, "F3" => 175.0, "G3" => 196.0, "A3" => 220.0, "B3" => 247.0,
        "C4" => 262.0, "D4" => 294.0, "E4" => 330.0, "F4" => 349.0, "G4" => 392.0, "A4" => 440.0, "B4" => 494.0,
        "C5" => 523.0, "D5" => 587.0, "E5" => 659.0, "F5" => 698.0, "G5" => 784.0, "A5" => 880.0, "B5" => 988.0,
        "C6" => 1047.0, "D6" => 1175.0, "E6" => 1319.0,
        _ => 0.0,
    }
}

/// js parseN: "name:dur" tokens, dur in sixteenths, '-' = rest.
fn parse(s: &str) -> Vec<(f32, i32)> {
    s.split_whitespace()
        .map(|tok| {
            let (name, d) = tok.split_once(':').unwrap_or((tok, "1"));
            (if name == "-" { 0.0 } else { hz(name) }, d.parse().unwrap_or(1))
        })
        .collect()
}

struct Vox {
    notes: &'static str,
    wave: Wave,
    vol: f32,
    swell: bool,
    sub: bool,
}

// --- DARK EPIC FANTASY overworld march (A minor, 128 sixteenths per voice). ---
const LEAD_STR: &str = "A4:6 C5:2 E5:8 D5:2 C5:2 A4:4 F4:8 E5:6 G5:2 C5:8 D5:8 B4:8 \
    A4:4 E5:4 A5:8 G5:4 F5:4 C5:8 D5:4 A4:4 D5:8 B4:8 E5:8";
const PAD_STR: &str = "C5:16 A4:16 E4:16 B4:16 C5:16 A4:16 F4:16 G4:16";
const BASS_STR: &str = "A2:16 F2:16 C2:16 G2:16 A2:16 F2:16 D2:16 E2:16";
const ARP_STR: &str = "A3:2 C4:2 E4:2 A4:2 C5:2 E4:2 A4:2 C4:2 F3:2 A3:2 C4:2 F4:2 A4:2 C4:2 F4:2 A3:2 \
    C4:2 E4:2 G4:2 C5:2 G4:2 E4:2 C4:2 G3:2 G3:2 B3:2 D4:2 G4:2 B4:2 D4:2 G4:2 B3:2 \
    A3:2 C4:2 E4:2 A4:2 C5:2 E4:2 A4:2 C4:2 F3:2 A3:2 C4:2 F4:2 A4:2 C4:2 F4:2 A3:2 \
    D4:2 F4:2 A4:2 D5:2 A4:2 F4:2 D4:2 A3:2 E4:2 G4:2 B4:2 E5:2 B4:2 G4:2 E4:2 B3:2";
// --- FAST DUNGEON ACTION (A-minor vamp, 128 sixteenths). ---
const LEAD_D: &str = "A4:2 E5:2 A5:2 G5:2 E5:2 C5:2 E5:2 A4:2 C5:2 D5:2 E5:4 C5:2 A4:4 -:2 \
    D5:2 F5:2 A5:2 F5:2 D5:2 A4:2 D5:2 F5:2 A5:2 G5:2 F5:2 E5:2 D5:4 A4:4 \
    F5:2 A5:2 C6:2 A5:2 F5:2 A5:2 C6:2 A5:2 C6:2 B5:2 A5:2 G5:2 F5:4 C5:4 \
    E5:2 G5:2 B5:2 G5:2 E5:2 B4:2 E5:2 G5:2 B5:1 A5:1 G5:1 F5:1 E5:1 D5:1 C5:1 B4:1 E5:4 -:4";
const BASS_D: &str = "A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 E3:2 E2:2 \
    D2:2 D3:2 D2:2 D3:2 D2:2 D3:2 D2:2 D3:2 D2:2 D3:2 D2:2 D3:2 D2:2 D3:2 A2:2 A3:2 \
    F2:2 F3:2 F2:2 F3:2 F2:2 F3:2 F2:2 F3:2 F2:2 F3:2 F2:2 F3:2 C3:2 C4:2 C3:2 C4:2 \
    E2:2 E3:2 E2:2 E3:2 E2:2 E3:2 E2:2 E3:2 E2:2 E3:2 E2:2 E3:2 E2:2 B2:2 E2:2 B2:2";
// --- SOFT PEACEFUL TOWN (C major, 64 sixteenths). ---
const LEAD_T: &str = "G4:4 C5:4 E5:4 G5:4 E5:6 D5:2 C5:8 A4:4 C5:4 E5:4 D5:4 C5:8 -:8";
const PAD_T: &str = "E5:16 D5:16 C5:16 C5:16";
const ARP_T: &str = "C4:2 E4:2 G4:2 C5:2 G4:2 E4:2 G4:2 E4:2 G3:2 B3:2 D4:2 G4:2 D4:2 B3:2 D4:2 B3:2 \
    A3:2 C4:2 E4:2 A4:2 E4:2 C4:2 E4:2 C4:2 F3:2 A3:2 C4:2 F4:2 C4:2 A3:2 C4:2 A3:2";
const BASS_T: &str = "C3:16 G2:16 A2:16 F2:16";

// --- NEW LOOPS (rs originals, Baz: "more songs in other places to vary soundtrack").
// Same sequencer, same rules: every voice totals its track's sixteenth count. ---
// BOSS — a relentless A-minor assault; sawtooth lead over a pumping bass + war kit.
const LEAD_B: &str = "A4:2 A4:2 C5:2 A4:2 E5:2 D5:2 C5:2 B4:2 A4:2 C5:2 E5:2 A5:2 G5:2 E5:2 D5:2 C5:2 \
    F5:2 E5:2 D5:2 C5:2 D5:2 C5:2 B4:2 A4:2 E5:4 B4:2 C5:2 E5:2 D5:2 C5:2 B4:2 \
    A4:2 A4:2 C5:2 A4:2 E5:2 D5:2 C5:2 B4:2 A4:2 C5:2 E5:2 A5:2 G5:2 A5:2 B5:2 C6:2 \
    A5:2 G5:2 F5:2 E5:2 F5:2 E5:2 D5:2 C5:2 B4:2 C5:2 D5:2 E5:2 A4:4 -:4";
const BASS_B: &str = "A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 \
    D2:2 D3:2 D2:2 D3:2 D2:2 D3:2 D2:2 D3:2 E2:2 E3:2 E2:2 E3:2 E2:2 E3:2 E2:2 E3:2 \
    A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 A2:2 A3:2 \
    F2:2 F3:2 F2:2 F3:2 F2:2 F3:2 F2:2 F3:2 E2:2 E3:2 E2:2 E3:2 E2:2 E3:2 E2:2 E3:2";
// NIGHT — the overworld gone quiet: long airy phrases, no drums, moonlit.
const LEAD_N: &str = "E5:8 C5:4 A4:4 B4:6 C5:2 D5:8 C5:8 A4:4 E4:4 B4:12 -:4";
const PAD_N: &str = "A4:16 F4:16 C5:16 E4:16";
const BASS_N: &str = "A2:16 F2:16 C3:16 E2:16";
// FROST — crystalline bells over deep cold roots (the arctic's overworld).
const LEAD_F: &str = "A5:4 F5:4 D5:8 -:4 E5:4 G5:8 A5:4 C6:4 A5:8 G5:8 E5:4 -:4";
const SHIM_F: &str = "D6:16 C6:16 D6:16 A5:16";
const BASS_F: &str = "D3:16 C3:16 D3:16 A2:16";
// DREAD — the corrupted lands: two slow drones grinding, a voice that barely dares.
const DRONE_A: &str = "A2:16 A2:16 G2:16 E2:16";
const DRONE_B: &str = "E3:16 D3:16 C3:16 B2:16";
const LEAD_X: &str = "-:8 A4:4 -:4 -:4 B4:2 A4:2 -:8 -:8 F4:4 -:4 E4:2 F4:2 E4:4 -:8";
// FINALE — the Black Castle: an E-minor processional, wide and doomed.
const LEAD_C: &str = "E5:4 G5:4 B5:4 G5:4 C6:4 B5:4 A5:4 G5:4 A5:4 E5:4 F5:4 E5:4 B4:8 D5:4 B4:4 \
    E5:4 G5:4 B5:4 E6:4 C6:4 A5:4 G5:4 E5:4 D5:4 F5:4 A5:4 D5:4 B4:4 E5:4 B4:4 E5:4";
const ARP_C: &str = "E3:2 G3:2 B3:2 E4:2 G4:2 B3:2 E4:2 G3:2 C3:2 E3:2 G3:2 C4:2 E4:2 G3:2 C4:2 E3:2 \
    A3:2 C4:2 E4:2 A4:2 E4:2 C4:2 A3:2 E3:2 B3:2 D4:2 F4:2 B4:2 F4:2 D4:2 B3:2 F3:2 \
    E3:2 G3:2 B3:2 E4:2 G4:2 B3:2 E4:2 G3:2 C3:2 E3:2 G3:2 C4:2 E4:2 G3:2 C4:2 E3:2 \
    D3:2 F3:2 A3:2 D4:2 A3:2 F3:2 D3:2 A2:2 B3:2 D4:2 F4:2 B4:2 F4:2 D4:2 B3:2 F3:2";
const BASS_C: &str = "E2:8 E2:8 C2:8 C2:8 A2:8 A2:8 B2:8 B2:8 E2:8 E2:8 C2:8 C2:8 D2:8 D2:8 B2:8 B2:8";

fn render(beat: f32, sixteenths: i32, drums: &str, voices: &[Vox]) -> Vec<f32> {
    let mut buf = Buf::secs(sixteenths as f32 * beat, true);
    for v in voices {
        let mut t = 0i32;
        for (freq, d) in parse(v.notes) {
            if freq > 0.0 {
                music_tone(&mut buf, t as f32 * beat, freq, d as f32 * beat, v.wave, v.vol, v.swell);
                if v.sub {
                    music_tone(&mut buf, t as f32 * beat, freq / 2.0, d as f32 * beat, Wave::Sine, v.vol * 0.7, false);
                }
            }
            t += d;
        }
    }
    let pattern: Vec<&str> = drums.split_whitespace().collect();
    if !pattern.is_empty() {
        for i in 0..sixteenths {
            match pattern[i as usize % pattern.len()] {
                "K" => kick(&mut buf, i as f32 * beat),
                "S" => snare(&mut buf, i as f32 * beat),
                "H" => hat(&mut buf, i as f32 * beat),
                _ => {}
            }
        }
    }
    let _ = SR;
    buf.data
}

pub fn render_all() -> Vec<(&'static str, Vec<f32>)> {
    vec![
        (
            "boss",
            render(
                0.085,
                128,
                "K H S H K H S H K H S H K S S H",
                &[
                    Vox { notes: LEAD_B, wave: Wave::Sawtooth, vol: 0.12, swell: false, sub: false },
                    Vox { notes: BASS_B, wave: Wave::Triangle, vol: 0.19, swell: false, sub: true },
                ],
            ),
        ),
        (
            "night",
            render(
                0.2,
                64,
                "- - - - - - - - - - - - - - - -",
                &[
                    Vox { notes: LEAD_N, wave: Wave::Triangle, vol: 0.13, swell: true, sub: false },
                    Vox { notes: PAD_N, wave: Wave::Triangle, vol: 0.05, swell: true, sub: false },
                    Vox { notes: BASS_N, wave: Wave::Sine, vol: 0.1, swell: false, sub: true },
                ],
            ),
        ),
        (
            "frost",
            render(
                0.18,
                64,
                "- - - - - - - - - - - - - - - -",
                &[
                    Vox { notes: LEAD_F, wave: Wave::Triangle, vol: 0.13, swell: true, sub: false },
                    Vox { notes: SHIM_F, wave: Wave::Sine, vol: 0.05, swell: true, sub: false },
                    Vox { notes: BASS_F, wave: Wave::Triangle, vol: 0.09, swell: false, sub: true },
                ],
            ),
        ),
        (
            "dread",
            render(
                0.17,
                64,
                "K - - - - - - - - - - - - - - -",
                &[
                    Vox { notes: DRONE_A, wave: Wave::Sawtooth, vol: 0.06, swell: true, sub: true },
                    Vox { notes: DRONE_B, wave: Wave::Sawtooth, vol: 0.04, swell: true, sub: false },
                    Vox { notes: LEAD_X, wave: Wave::Triangle, vol: 0.1, swell: true, sub: false },
                ],
            ),
        ),
        (
            "finale",
            render(
                0.11,
                128,
                "K - - H S - - H K - K H S - H H",
                &[
                    Vox { notes: LEAD_C, wave: Wave::Square, vol: 0.13, swell: false, sub: false },
                    Vox { notes: ARP_C, wave: Wave::Triangle, vol: 0.07, swell: false, sub: false },
                    Vox { notes: BASS_C, wave: Wave::Triangle, vol: 0.19, swell: false, sub: true },
                ],
            ),
        ),
        (
            "overworld",
            render(
                0.15,
                128,
                "K - - - - - - - - - - - - - - -",
                &[
                    Vox { notes: LEAD_STR, wave: Wave::Triangle, vol: 0.22, swell: true, sub: false },
                    Vox { notes: ARP_STR, wave: Wave::Triangle, vol: 0.09, swell: false, sub: false },
                    Vox { notes: PAD_STR, wave: Wave::Triangle, vol: 0.08, swell: true, sub: false },
                    Vox { notes: BASS_STR, wave: Wave::Triangle, vol: 0.18, swell: false, sub: true },
                ],
            ),
        ),
        (
            "dungeon",
            render(
                0.09,
                128,
                "K H H H S H H H K H K H S H H H",
                &[
                    Vox { notes: LEAD_D, wave: Wave::Square, vol: 0.15, swell: false, sub: false },
                    Vox { notes: BASS_D, wave: Wave::Triangle, vol: 0.18, swell: false, sub: false },
                ],
            ),
        ),
        (
            "town",
            render(
                0.16,
                64,
                "- - - - - - - - - - - - - - - -",
                &[
                    Vox { notes: LEAD_T, wave: Wave::Triangle, vol: 0.18, swell: true, sub: false },
                    Vox { notes: ARP_T, wave: Wave::Triangle, vol: 0.07, swell: false, sub: false },
                    Vox { notes: PAD_T, wave: Wave::Triangle, vol: 0.06, swell: true, sub: false },
                    Vox { notes: BASS_T, wave: Wave::Triangle, vol: 0.13, swell: false, sub: true },
                ],
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voices_stay_in_sync() {
        // Every voice of a track must total the track's sixteenth count (the js comment's
        // promise) — a drifted string would phase the whole loop.
        let total = |s: &str| parse(s).iter().map(|(_, d)| d).sum::<i32>();
        for s in [LEAD_STR, ARP_STR] {
            assert_eq!(total(s), 128, "overworld voice length");
        }
        for s in [PAD_STR, BASS_STR] {
            assert_eq!(total(s), 128, "overworld pad/bass length");
        }
        for s in [LEAD_D, BASS_D] {
            assert_eq!(total(s), 128, "dungeon voice length");
        }
        for s in [LEAD_T, ARP_T, PAD_T, BASS_T] {
            assert_eq!(total(s), 64, "town voice length");
        }
        for s in [LEAD_B, BASS_B, LEAD_C, ARP_C, BASS_C] {
            assert_eq!(total(s), 128, "boss/finale voice length");
        }
        for s in [LEAD_N, PAD_N, BASS_N, LEAD_F, SHIM_F, BASS_F, DRONE_A, DRONE_B, LEAD_X] {
            assert_eq!(total(s), 64, "night/frost/dread voice length");
        }
        // Every named note resolves to a pitch.
        for s in [LEAD_STR, PAD_STR, BASS_STR, ARP_STR, LEAD_D, BASS_D, LEAD_T, PAD_T, ARP_T, BASS_T,
            LEAD_B, BASS_B, LEAD_N, PAD_N, BASS_N, LEAD_F, SHIM_F, BASS_F, DRONE_A, DRONE_B, LEAD_X, LEAD_C, ARP_C, BASS_C] {
            for tok in s.split_whitespace() {
                let name = tok.split(':').next().unwrap();
                assert!(name == "-" || hz(name) > 0.0, "unknown note {name}");
            }
        }
    }
}
