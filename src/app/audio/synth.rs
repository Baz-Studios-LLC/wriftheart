//! audio/synth.rs — the js WebAudio synth (audio.js), rendered OFFLINE. The js built
//! every sound live from oscillator + filtered-noise nodes; here the same two
//! primitives run as pure-Rust DSP into PCM buffers baked once at startup. Same
//! recipes, same envelopes, same numbers — the exponential ramps are WebAudio's
//! geometric interpolation, the filters are RBJ biquads standing in for
//! BiquadFilterNode, the noise is a deterministic LCG.

pub const SR: u32 = 44100;
const EPS: f32 = 0.0001;

#[derive(Clone, Copy, PartialEq)]
pub enum Wave {
    Square,
    Sawtooth,
    Triangle,
    Sine,
}

#[derive(Clone, Copy)]
pub enum Filter {
    Lowpass,
    Highpass,
    Bandpass,
}

/// A mix target; music loops render with wrap-around so note tails cross the seam.
pub struct Buf {
    pub data: Vec<f32>,
    pub wrap: bool,
}

impl Buf {
    pub fn secs(seconds: f32, wrap: bool) -> Self {
        Buf { data: vec![0.0; (seconds * SR as f32) as usize], wrap }
    }
    #[inline]
    fn add(&mut self, i: usize, v: f32) {
        let n = self.data.len();
        if n == 0 {
            return;
        }
        if self.wrap {
            self.data[i % n] += v;
        } else if i < n {
            self.data[i] += v;
        }
    }
}

#[inline]
fn osc(wave: Wave, phase: f32) -> f32 {
    let p = phase - phase.floor();
    match wave {
        Wave::Sine => (p * std::f32::consts::TAU).sin(),
        Wave::Square => {
            if p < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Wave::Sawtooth => 2.0 * p - 1.0,
        Wave::Triangle => {
            if p < 0.5 {
                4.0 * p - 1.0
            } else {
                3.0 - 4.0 * p
            }
        }
    }
}

/// WebAudio exponentialRamp: geometric interpolation v0 -> v1 over [0,1].
#[inline]
fn exp_ramp(v0: f32, v1: f32, k: f32) -> f32 {
    v0 * (v1 / v0).powf(k.clamp(0.0, 1.0))
}

/// js tone(): fast exp attack (6ms), exp decay to the end, optional exp pitch glide.
pub fn tone(buf: &mut Buf, t0: f32, freq: f32, dur: f32, wave: Wave, vol: f32, slide_to: Option<f32>) {
    let start = (t0 * SR as f32) as usize;
    let n = (dur * SR as f32) as usize;
    let attack = 0.006_f32.min(dur * 0.5);
    let slide = slide_to.map(|s| s.max(20.0));
    let mut phase = 0.0_f32;
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let f = match slide {
            Some(s) => exp_ramp(freq, s, t / dur),
            None => freq,
        };
        phase += f / SR as f32;
        let env = if t < attack {
            exp_ramp(EPS, vol, t / attack)
        } else {
            exp_ramp(vol, EPS, (t - attack) / (dur - attack).max(0.001))
        };
        buf.add(start + i, osc(wave, phase) * env);
    }
}

/// js musicTone(): the melodic voice — optional orchestral swell on long notes.
pub fn music_tone(buf: &mut Buf, t0: f32, freq: f32, dur: f32, wave: Wave, vol: f32, swell: bool) {
    let start = (t0 * SR as f32) as usize;
    let n = (dur * SR as f32) as usize;
    let mut phase = 0.0_f32;
    for i in 0..n {
        let t = i as f32 / SR as f32;
        phase += freq / SR as f32;
        let env = if swell && dur > 0.5 {
            if t < 0.05 {
                exp_ramp(EPS, vol * 0.4, t / 0.05)
            } else if t < dur * 0.72 {
                // linear crescendo to full voice
                let k = (t - 0.05) / (dur * 0.72 - 0.05).max(0.001);
                vol * 0.4 + (vol - vol * 0.4) * k
            } else if t < dur * 0.84 {
                vol
            } else {
                exp_ramp(vol, EPS, (t - dur * 0.84) / (dur * 0.16).max(0.001))
            }
        } else if t < 0.02 {
            exp_ramp(EPS, vol, t / 0.02)
        } else if t < dur * 0.6 {
            vol
        } else {
            exp_ramp(vol, EPS, (t - dur * 0.6) / (dur * 0.4).max(0.001))
        };
        buf.add(start + i, osc(wave, phase) * env);
    }
}

/// RBJ biquad standing in for the js BiquadFilterNode.
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    fn new(kind: Filter, freq: f32) -> Self {
        let q = match kind {
            Filter::Bandpass => 1.0,
            _ => std::f32::consts::FRAC_1_SQRT_2,
        };
        let w0 = std::f32::consts::TAU * (freq / SR as f32).clamp(0.0005, 0.49);
        let (sw, cw) = (w0.sin(), w0.cos());
        let alpha = sw / (2.0 * q);
        let (b0, b1, b2) = match kind {
            Filter::Lowpass => ((1.0 - cw) / 2.0, 1.0 - cw, (1.0 - cw) / 2.0),
            Filter::Highpass => ((1.0 + cw) / 2.0, -(1.0 + cw), (1.0 + cw) / 2.0),
            Filter::Bandpass => (alpha, 0.0, -alpha),
        };
        let a0 = 1.0 + alpha;
        Biquad {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: -2.0 * cw / a0,
            a2: (1.0 - alpha) / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// js noise(): a filtered white-noise burst with an exp decay. Deterministic LCG so
/// bakes are reproducible run to run.
pub fn noise(buf: &mut Buf, t0: f32, dur: f32, vol: f32, kind: Filter, freq: f32) {
    let start = (t0 * SR as f32) as usize;
    let n = (dur * SR as f32) as usize;
    let mut filt = Biquad::new(kind, freq);
    let mut seed: u32 = 0x1234_5678 ^ ((t0 * 1000.0) as u32).wrapping_mul(2654435761);
    for i in 0..n {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let white = (seed >> 8) as f32 / 8388608.0 - 1.0;
        let t = i as f32 / SR as f32;
        let env = exp_ramp(vol, EPS, t / dur.max(0.001));
        buf.add(start + i, filt.run(white) * env);
    }
}

/// js kick(): a deep timpani boom (sine 120 -> 34).
pub fn kick(buf: &mut Buf, t0: f32) {
    let start = (t0 * SR as f32) as usize;
    let n = (0.4 * SR as f32) as usize;
    let mut phase = 0.0_f32;
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let f = if t < 0.22 { exp_ramp(120.0, 34.0, t / 0.22) } else { 34.0 };
        phase += f / SR as f32;
        let env = if t < 0.01 { exp_ramp(EPS, 0.30, t / 0.01) } else { exp_ramp(0.30, EPS, (t - 0.01) / 0.39) };
        buf.add(start + i, osc(Wave::Sine, phase) * env);
    }
}

/// js snare(): highpassed crack + a quick pitched thump.
pub fn snare(buf: &mut Buf, t0: f32) {
    noise(buf, t0, 0.13, 0.32, Filter::Highpass, 1500.0);
    let start = (t0 * SR as f32) as usize;
    let n = (0.06 * SR as f32) as usize;
    let mut phase = 0.0_f32;
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let f = if t < 0.05 { exp_ramp(340.0, 180.0, t / 0.05) } else { 180.0 };
        phase += f / SR as f32;
        let env = exp_ramp(0.14, EPS, t / 0.06);
        buf.add(start + i, osc(Wave::Triangle, phase) * env);
    }
}

/// js hat(): the tiniest highpass tick.
pub fn hat(buf: &mut Buf, t0: f32) {
    noise(buf, t0, 0.04, 0.10, Filter::Highpass, 7000.0);
}

/// js note(): one flute note — breathy triangle + octave shimmer + attack chiff.
pub fn flute_note(buf: &mut Buf, t0: f32, freq: f32, dur: f32) {
    tone(buf, t0, freq, dur, Wave::Triangle, 0.30, Some(freq * 1.005));
    tone(buf, t0, freq * 2.0, dur * 0.7, Wave::Sine, 0.09, None);
    noise(buf, t0, 0.05, 0.05, Filter::Highpass, 3000.0);
}

/// The HELD flute voice's steady state (js noteOn minus its live envelope): one
/// second of triangle-at-the-note + the octave sine shimmer, with the vibrato.
/// SEAMLESS by construction: pitch snaps to whole cycles/sec, the 5Hz vibrato (js
/// 5.4) fits the second exactly, and its integral over whole cycles is zero — so
/// phase, pitch and vibrato all land back at their start at the loop point. The
/// attack (sink-volume ramp + the breath chiff) and the noteOff release live in
/// mod.rs's flute_hold_tick.
pub fn note_hold_loop(freq: f32) -> Vec<f32> {
    let f = freq.round(); // whole cycles per second -> silent seam (<= 2.5 cents off)
    let depth = f * 0.007; // js vibrato depth
    let n = SR as usize; // exactly 1.0s
    let mut out = vec![0.0f32; n];
    let (mut p1, mut p2) = (0.0_f32, 0.0_f32);
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / SR as f32;
        let vib = depth * (t * 5.0 * std::f32::consts::TAU).sin();
        p1 += (f + vib) / SR as f32;
        p2 += (f + vib) * 2.0 / SR as f32;
        *s = osc(Wave::Triangle, p1) * 0.30 + osc(Wave::Sine, p2) * 0.09;
    }
    out
}

/// Trim trailing near-silence and wrap the samples as a 16-bit mono WAV (rodio-decodable).
/// A one-shot SFX voice: trims trailing near-silence (a shorter buffer, and the DESPAWN
/// player never needs the dead tail).
pub fn wav_bytes(mut samples: Vec<f32>, gain: f32) -> Vec<u8> {
    while samples.len() > 64 && samples[samples.len() - 1].abs() * gain < 0.0005 {
        samples.pop();
    }
    wav_encode(samples, gain)
}

/// A LOOP body (music tracks, held-flute voices): the length is EXACT and seam-critical
/// (note tails wrap across it), so NEVER trim — a shorter buffer repeats early and, since
/// each track ends on a different amount of quiet, at a different wrong tempo.
pub fn wav_loop(samples: Vec<f32>, gain: f32) -> Vec<u8> {
    wav_encode(samples, gain)
}

fn wav_encode(samples: Vec<f32>, gain: f32) -> Vec<u8> {
    let n = samples.len() as u32;
    let mut out = Vec::with_capacity(44 + n as usize * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + n * 2).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SR.to_le_bytes());
    out.extend_from_slice(&(SR * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(n * 2).to_le_bytes());
    for s in samples {
        let v = (s * gain).clamp(-1.0, 1.0);
        out.extend_from_slice(&((v * 32767.0) as i16).to_le_bytes());
    }
    out
}
