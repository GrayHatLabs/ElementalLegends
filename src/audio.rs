// A tiny NES-flavoured synth: square / triangle / saw / sine / noise voices with
// exponential pitch and volume sweeps, plus a step sequencer for the music.
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::AudioSubsystem;
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq)]
pub enum Wave {
    Square,
    Tri,
    Saw,
    Sine,
    Noise,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sfx {
    Shoot,
    Spell,
    Hit,
    Weak,
    Boom,
    BigBoom,
    Pickup,
    Heal,
    Eat,
    Hurt,
    Gate,
    Select,
    Deny,
    Chest,
    Fanfare,
    Zap,
    Quake,
    Freeze,
    Coin,
}
pub const SFX_COUNT: usize = 19;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Song {
    Title,
    Field,
    Lair,
}

struct Voice {
    wave: Wave,
    freq: f32,
    fmul: f32,
    gain: f32,
    gmul: f32,
    cut: f32,
    cmul: f32,
    left: i32,
    delay: i32,
    phase: f32,
    lp: f32,
    music: bool,
}

struct SongData {
    bpm: f32,
    vol: f32,
    lead: Vec<u8>,
    bass: Vec<u8>,
    drum: &'static [u8],
}

pub struct Synth {
    sr: f32,
    voices: Vec<Voice>,
    songs: Vec<SongData>,
    song: Option<Song>,
    step: usize,
    step_left: f32,
    pub muted: bool,
    noise: u32,
}

fn rep(a: &[u8], n: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for _ in 0..n {
        v.extend_from_slice(a);
    }
    v
}
fn mtof(m: u8) -> f32 {
    440.0 * 2f32.powf((m as f32 - 69.0) / 12.0)
}

impl Synth {
    pub fn new(sr: f32) -> Self {
        let title = SongData {
            bpm: 100.0,
            vol: 0.06,
            lead: vec![
                69, 0, 72, 0, 76, 0, 81, 0, 79, 0, 76, 0, 74, 0, 0, 0, 65, 0, 69, 0, 72, 0, 77, 0, 76, 0, 0, 0, 68, 0, 71, 0,
            ],
            bass: [rep(&[45, 0, 57, 0], 2), rep(&[43, 0, 55, 0], 2), rep(&[41, 0, 53, 0], 2), rep(&[40, 0, 52, 0], 2)]
                .concat(),
            drum: &[],
        };
        let field = SongData {
            bpm: 128.0,
            vol: 0.055,
            lead: vec![
                69, 72, 76, 81, 79, 76, 72, 74, 76, 0, 76, 74, 72, 71, 69, 0, 65, 69, 72, 77, 76, 72, 69, 71, 72, 0, 71, 0,
                68, 0, 64, 0, 69, 0, 72, 0, 76, 0, 74, 72, 71, 0, 67, 0, 71, 74, 79, 0, 77, 0, 76, 0, 74, 72, 74, 76, 76,
                0, 0, 0, 64, 68, 71, 76,
            ],
            bass: [rep(&[45, 57], 4), rep(&[43, 55], 4), rep(&[41, 53], 4), rep(&[40, 52], 4)].concat(),
            drum: b"k.h.s.h.",
        };
        let lair = SongData {
            bpm: 168.0,
            vol: 0.055,
            lead: vec![76, 77, 76, 75, 76, 0, 71, 0, 72, 71, 70, 71, 0, 0, 64, 0],
            bass: vec![40, 40, 52, 40, 40, 52, 40, 41, 40, 40, 52, 40, 41, 53, 41, 52],
            drum: b"khsh",
        };
        Synth {
            sr,
            voices: Vec::new(),
            songs: vec![title, field, lair],
            song: None,
            step: 0,
            step_left: 0.0,
            muted: false,
            noise: 0x1234_5678,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push(&mut self, wave: Wave, f0: f32, f1: f32, dur: f32, vol: f32, delay: f32, cutoff: f32, music: bool) {
        let n = (dur * self.sr).max(1.0);
        let (f0, f1) = (f0.max(20.0), f1.max(20.0));
        let cut = cutoff.max(100.0);
        let vol = vol.max(0.001);
        let v = Voice {
            wave,
            freq: f0,
            fmul: (f1 / f0).powf(1.0 / n),
            gain: vol,
            gmul: (0.0008 / vol).powf(1.0 / n),
            cut,
            cmul: (80.0 / cut).powf(1.0 / n),
            left: n as i32,
            delay: (delay * self.sr) as i32,
            phase: 0.0,
            lp: 0.0,
            music,
        };
        if self.voices.len() >= 64 {
            self.voices.remove(0);
        }
        self.voices.push(v);
    }
    fn tone(&mut self, w: Wave, f0: f32, f1: f32, dur: f32, vol: f32, delay: f32) {
        self.push(w, f0, f1, dur, vol, delay, 1000.0, false);
    }
    fn noise(&mut self, dur: f32, vol: f32, cut: f32, delay: f32) {
        self.push(Wave::Noise, 100.0, 100.0, dur, vol, delay, cut, false);
    }
    fn arp(&mut self, freqs: &[f32], step: f32, dur: f32) {
        for (i, &f) in freqs.iter().enumerate() {
            self.tone(Wave::Square, f, f, dur, 0.05, i as f32 * step);
        }
    }

    pub fn sfx(&mut self, s: Sfx) {
        if self.muted {
            return;
        }
        use Wave::*;
        match s {
            Sfx::Shoot => self.tone(Square, 900.0, 450.0, 0.06, 0.035, 0.0),
            Sfx::Spell => {
                self.tone(Saw, 300.0, 1400.0, 0.25, 0.06, 0.0);
                self.tone(Tri, 150.0, 700.0, 0.25, 0.15, 0.0);
            }
            Sfx::Hit => self.tone(Square, 320.0, 120.0, 0.05, 0.06, 0.0),
            Sfx::Weak => {
                self.tone(Square, 660.0, 1320.0, 0.08, 0.06, 0.0);
                self.tone(Square, 990.0, 1980.0, 0.06, 0.04, 0.04);
            }
            Sfx::Boom => {
                self.noise(0.35, 0.3, 2400.0, 0.0);
                self.tone(Tri, 160.0, 40.0, 0.3, 0.2, 0.0);
            }
            Sfx::BigBoom => {
                self.noise(1.1, 0.5, 2000.0, 0.0);
                self.tone(Tri, 120.0, 25.0, 1.0, 0.4, 0.0);
            }
            Sfx::Pickup | Sfx::Coin => {
                self.tone(Square, 988.0, 988.0, 0.05, 0.05, 0.0);
                self.tone(Square, 1319.0, 1319.0, 0.08, 0.05, 0.05);
            }
            Sfx::Heal => self.arp(&[523.0, 659.0, 784.0, 1046.0], 0.05, 0.06),
            Sfx::Eat => {
                for i in 0..3 {
                    self.noise(0.05, 0.12, 1500.0, i as f32 * 0.1);
                }
                self.tone(Tri, 220.0, 160.0, 0.3, 0.12, 0.0);
            }
            Sfx::Hurt => {
                self.tone(Saw, 420.0, 60.0, 0.25, 0.1, 0.0);
                self.noise(0.15, 0.15, 1400.0, 0.0);
            }
            Sfx::Gate => {
                self.tone(Square, 150.0, 1800.0, 0.9, 0.06, 0.0);
                self.tone(Tri, 80.0, 900.0, 0.9, 0.2, 0.0);
            }
            Sfx::Select => self.tone(Square, 660.0, 660.0, 0.06, 0.05, 0.0),
            Sfx::Deny => self.tone(Square, 140.0, 110.0, 0.14, 0.06, 0.0),
            Sfx::Chest => self.arp(&[392.0, 523.0, 659.0, 784.0], 0.08, 0.1),
            Sfx::Fanfare => {
                for (i, m) in [72u8, 76, 79, 84, 0, 79, 84].iter().enumerate() {
                    if *m > 0 {
                        let f = mtof(*m);
                        self.tone(Square, f, f, if i == 6 { 0.6 } else { 0.12 }, 0.06, i as f32 * 0.12);
                    }
                }
            }
            Sfx::Zap => {
                self.noise(0.25, 0.3, 8000.0, 0.0);
                self.tone(Saw, 1800.0, 200.0, 0.25, 0.06, 0.0);
            }
            Sfx::Quake => {
                self.noise(0.8, 0.5, 600.0, 0.0);
                self.tone(Sine, 60.0, 30.0, 0.8, 0.4, 0.0);
            }
            Sfx::Freeze => {
                self.tone(Tri, 2000.0, 3000.0, 0.4, 0.08, 0.0);
                self.noise(0.4, 0.12, 9000.0, 0.0);
            }
        }
    }

    pub fn play_song(&mut self, s: Option<Song>) {
        if self.song == s {
            return;
        }
        self.song = s;
        self.step = 0;
        self.step_left = 0.05 * self.sr;
        self.voices.retain(|v| !v.music);
    }

    fn song_step(&mut self, s: Song) {
        let (bpm, vol, lead, bass, drum) = {
            let d = &self.songs[s as usize];
            let i = self.step;
            (
                d.bpm,
                d.vol,
                d.lead[i % d.lead.len()],
                d.bass[i % d.bass.len()],
                if d.drum.is_empty() { b'.' } else { d.drum[i % d.drum.len()] },
            )
        };
        let dur = 60.0 / bpm / 2.0;
        if lead > 0 {
            let f = mtof(lead);
            self.push(Wave::Square, f, f, dur * 0.9, vol, 0.0, 1000.0, true);
        }
        if bass > 0 {
            let f = mtof(bass);
            self.push(Wave::Tri, f, f, dur * 0.95, 0.22, 0.0, 1000.0, true);
        }
        match drum {
            b'k' => self.push(Wave::Sine, 150.0, 40.0, 0.12, 0.35, 0.0, 1000.0, true),
            b'h' => self.push(Wave::Noise, 100.0, 100.0, 0.03, 0.05, 0.0, 9000.0, true),
            b's' => self.push(Wave::Noise, 100.0, 100.0, 0.12, 0.12, 0.0, 4000.0, true),
            _ => {}
        }
        self.step += 1;
    }
}

impl AudioCallback for Synth {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        let sr = self.sr;
        for o in out.iter_mut() {
            if let Some(s) = self.song {
                if !self.muted {
                    self.step_left -= 1.0;
                    if self.step_left <= 0.0 {
                        self.song_step(s);
                        self.step_left += 60.0 / self.songs[s as usize].bpm / 2.0 * sr;
                    }
                }
            }
            let (mut m, mut fx) = (0.0f32, 0.0f32);
            let mut nz = self.noise;
            for v in self.voices.iter_mut() {
                if v.delay > 0 {
                    v.delay -= 1;
                    continue;
                }
                if v.left <= 0 {
                    continue;
                }
                let smp = match v.wave {
                    Wave::Square => {
                        if v.phase < 0.5 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    Wave::Tri => 4.0 * (v.phase - 0.5).abs() - 1.0,
                    Wave::Saw => 2.0 * v.phase - 1.0,
                    Wave::Sine => (v.phase * TAU).sin(),
                    Wave::Noise => {
                        nz ^= nz << 13;
                        nz ^= nz >> 17;
                        nz ^= nz << 5;
                        let r = (nz as f32 / u32::MAX as f32) * 2.0 - 1.0;
                        let a = (v.cut * TAU / sr).min(1.0);
                        v.lp += (r - v.lp) * a;
                        v.cut *= v.cmul;
                        v.lp * 1.5
                    }
                };
                v.phase += v.freq / sr;
                if v.phase >= 1.0 {
                    v.phase -= 1.0;
                }
                v.freq *= v.fmul;
                let val = smp * v.gain;
                v.gain *= v.gmul;
                v.left -= 1;
                if v.music {
                    m += val;
                } else {
                    fx += val;
                }
            }
            self.noise = nz;
            *o = if self.muted { 0.0 } else { ((m * 0.7 + fx * 0.8) * 0.5).clamp(-1.0, 1.0) };
        }
        self.voices.retain(|v| v.left > 0 || v.delay > 0);
    }
}

pub fn open(a: &AudioSubsystem) -> Option<AudioDevice<Synth>> {
    let spec = AudioSpecDesired { freq: Some(44100), channels: Some(1), samples: Some(1024) };
    let dev = a.open_playback(None, &spec, |s| Synth::new(s.freq as f32)).ok()?;
    dev.resume();
    Some(dev)
}
