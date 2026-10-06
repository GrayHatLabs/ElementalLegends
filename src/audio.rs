// SNES-flavoured synth, generated entirely in code.
//
// * Music: a step sequencer playing multi-track songs written as note text, through
//   instrument patches (flute, strings, harp, brass, organ, bells, basses, drums) with
//   ADSR envelopes, vibrato, detune, low-pass filters and stereo panning, all fed into
//   a stereo echo (the classic SNES "echo buffer" sound).
// * Sound effects: pitch/volume-sweep voices (dry, with a little echo send).
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
    Ignite,
    Shatter,
    Roar,
    Unlock,
    Rumble,
    Push,
    Click,
}
pub const SFX_COUNT: usize = 26;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Song {
    Title,
    /// Greenwood and the default overworld theme.
    Field,
    Lair,
    /// Monolith clearing and the village shop.
    Village,
    Crypt,
    Swamp,
    Volcano,
    Dungeon,
    /// Lair interiors, one theme each (lairs 1-6).
    Shrine,
    Catacomb,
    Castle,
    Fortress,
    Sanctuary,
    Tower,
}
const SONG_COUNT: usize = 14;

// ---------------------------------------------------------------- instruments
#[derive(Clone, Copy, PartialEq)]
enum Osc {
    Pulse(f32),
    Tri,
    Saw,
    /// Three detuned saws spread across the stereo field.
    SuperSaw,
    Sine,
    /// Sine with 2nd and 3rd harmonics.
    Organ,
    /// Sine plus a fast-decaying inharmonic partial.
    Bell,
    Noise,
}

#[derive(Clone, Copy)]
struct Patch {
    osc: Osc,
    a: f32,
    d: f32,
    s: f32,
    r: f32,
    gain: f32,
    pan: f32,
    /// Low-pass cutoff (Hz); `cut_env` multiplies it at note start and decays back.
    cut: f32,
    cut_env: f32,
    cut_decay: f32,
    vib_rate: f32,
    vib_depth: f32,
    vib_delay: f32,
    /// Exponential pitch drop over the note (drums), as a ratio reached after 0.1 s.
    drop: f32,
    /// Extra second oscillator an octave down (bass body).
    sub: f32,
}

const fn patch(osc: Osc) -> Patch {
    Patch {
        osc, a: 0.01, d: 0.1, s: 0.8, r: 0.1, gain: 0.2, pan: 0.0, cut: 8000.0, cut_env: 1.0, cut_decay: 0.2,
        vib_rate: 0.0, vib_depth: 0.0, vib_delay: 0.0, drop: 1.0, sub: 0.0,
    }
}

const P_FLUTE: usize = 0;
const P_STRINGS: usize = 1;
const P_HARP: usize = 2;
const P_BRASS: usize = 3;
const P_BASS: usize = 4;
const P_PBASS: usize = 5;
const P_ORGAN: usize = 6;
const P_BELL: usize = 7;
const P_LEAD: usize = 8;
const P_KICK: usize = 9;
const P_SNARE: usize = 10;
const P_HAT: usize = 11;
const P_TOM: usize = 12;

fn patches() -> Vec<Patch> {
    let mut v = vec![patch(Osc::Sine); 13];
    v[P_FLUTE] = Patch {
        osc: Osc::Tri, a: 0.05, d: 0.2, s: 0.75, r: 0.18, gain: 0.16, pan: 0.15, cut: 3200.0, vib_rate: 5.2,
        vib_depth: 0.006, vib_delay: 0.18, ..patch(Osc::Tri)
    };
    v[P_STRINGS] = Patch { osc: Osc::SuperSaw, a: 0.28, d: 0.3, s: 0.85, r: 0.5, gain: 0.07, cut: 1900.0, ..patch(Osc::SuperSaw) };
    v[P_HARP] = Patch {
        osc: Osc::Saw, a: 0.002, d: 0.45, s: 0.0, r: 0.3, gain: 0.11, pan: -0.35, cut: 900.0, cut_env: 4.5,
        cut_decay: 0.12, ..patch(Osc::Saw)
    };
    v[P_BRASS] = Patch {
        osc: Osc::Saw, a: 0.03, d: 0.25, s: 0.75, r: 0.1, gain: 0.12, pan: 0.2, cut: 1300.0, cut_env: 2.4,
        cut_decay: 0.18, vib_rate: 5.5, vib_depth: 0.004, vib_delay: 0.25, ..patch(Osc::Saw)
    };
    v[P_BASS] = Patch { osc: Osc::Tri, a: 0.004, d: 0.3, s: 0.8, r: 0.05, gain: 0.3, cut: 2000.0, sub: 0.5, ..patch(Osc::Tri) };
    v[P_PBASS] = Patch {
        osc: Osc::Pulse(0.25), a: 0.003, d: 0.12, s: 0.6, r: 0.04, gain: 0.13, cut: 1100.0, cut_env: 2.0,
        cut_decay: 0.08, sub: 0.35, ..patch(Osc::Pulse(0.25))
    };
    v[P_ORGAN] = Patch { osc: Osc::Organ, a: 0.06, d: 0.2, s: 0.9, r: 0.25, gain: 0.1, pan: -0.15, cut: 2600.0, ..patch(Osc::Organ) };
    v[P_BELL] = Patch { osc: Osc::Bell, a: 0.002, d: 1.3, s: 0.0, r: 0.6, gain: 0.14, pan: 0.3, ..patch(Osc::Bell) };
    v[P_LEAD] = Patch {
        osc: Osc::Pulse(0.5), a: 0.01, d: 0.2, s: 0.7, r: 0.1, gain: 0.08, pan: 0.1, cut: 3500.0, vib_rate: 5.8,
        vib_depth: 0.007, vib_delay: 0.2, ..patch(Osc::Pulse(0.5))
    };
    v[P_KICK] = Patch { osc: Osc::Sine, a: 0.001, d: 0.16, s: 0.0, r: 0.05, gain: 0.55, drop: 0.3, ..patch(Osc::Sine) };
    v[P_SNARE] = Patch { osc: Osc::Noise, a: 0.001, d: 0.14, s: 0.0, r: 0.05, gain: 0.2, cut: 5200.0, pan: 0.05, ..patch(Osc::Noise) };
    v[P_HAT] = Patch { osc: Osc::Noise, a: 0.001, d: 0.035, s: 0.0, r: 0.02, gain: 0.07, cut: 11000.0, pan: -0.25, ..patch(Osc::Noise) };
    v[P_TOM] = Patch { osc: Osc::Sine, a: 0.001, d: 0.22, s: 0.0, r: 0.05, gain: 0.35, drop: 0.55, pan: 0.2, ..patch(Osc::Sine) };
    v
}

// ---------------------------------------------------------------- songs (note text)
//
// Each track: patch, volume, then steps separated by spaces. A step is a note ("C4",
// "F#3", "Bb2"), a chord ("D3+F3+A3"), "." (silence) or "-" (hold the previous note).
// Drum tracks use k (kick), s (snare), h (hat), t (tom). "|" marks bars (ignored).
struct SongText {
    bpm: f32,
    /// Steps per beat (4 = sixteenth notes).
    div: f32,
    tracks: &'static [(usize, f32, &'static str)],
}

const TITLE: SongText = SongText {
    bpm: 78.0,
    div: 4.0,
    tracks: &[
        (P_STRINGS, 1.0, "D3+F3+A3 - - - - - - - - - - - - - - - | Bb2+D3+F3 - - - - - - - - - - - - - - - | C3+E3+G3 - - - - - - - - - - - - - - - | A2+C#3+E3 - - - - - - - - - - - - - - -"),
        (P_HARP, 1.0, "D4 F4 A4 D5 A4 F4 D4 F4 A4 D5 F5 D5 A4 F4 D4 A3 | Bb3 D4 F4 Bb4 F4 D4 Bb3 D4 F4 Bb4 D5 Bb4 F4 D4 Bb3 F3 | C4 E4 G4 C5 G4 E4 C4 E4 G4 C5 E5 C5 G4 E4 C4 G3 | A3 C#4 E4 A4 E4 C#4 A3 C#4 E4 A4 C#5 A4 E4 C#4 A3 E3"),
        (P_FLUTE, 1.0, "A4 - - - - - - - D5 - - - E5 - F5 - | F5 - - - E5 - D5 - C5 - - - D5 - - - | E5 - - - - - G5 - F5 - E5 - C5 - - - | C#5 - - - - - - - E5 - - - - - - -"),
        (P_BASS, 0.8, "D2 - - - - - - - D2 - - - A2 - - - | Bb1 - - - - - - - Bb1 - - - F2 - - - | C2 - - - - - - - C2 - - - G2 - - - | A1 - - - - - - - A1 - - - E2 - - -"),
    ],
};

const VILLAGE: SongText = SongText {
    bpm: 96.0,
    div: 4.0,
    tracks: &[
        (P_FLUTE, 1.0, "E5 - - - G5 - - - C6 - - - B5 - A5 - | A5 - - - - - G5 - E5 - - - D5 - - - | F5 - - - A5 - - - C6 - - - A5 - - - | G5 - - - - - - - D5 - E5 - F5 - G5 -"),
        (P_HARP, 0.9, "C4 E4 G4 C5 G4 E4 C4 G3 C4 E4 G4 C5 E5 C5 G4 E4 | A3 C4 E4 A4 E4 C4 A3 E3 A3 C4 E4 A4 C5 A4 E4 C4 | F3 A3 C4 F4 C4 A3 F3 C3 F3 A3 C4 F4 A4 F4 C4 A3 | G3 B3 D4 G4 D4 B3 G3 D3 G3 B3 D4 G4 B4 G4 D4 B3"),
        (P_STRINGS, 0.7, "C4+E4+G4 - - - - - - - - - - - - - - - | A3+C4+E4 - - - - - - - - - - - - - - - | F3+A3+C4 - - - - - - - - - - - - - - - | G3+B3+D4 - - - - - - - - - - - - - - -"),
        (P_BASS, 0.7, "C2 - - - - - - - G2 - - - - - - - | A1 - - - - - - - E2 - - - - - - - | F1 - - - - - - - C2 - - - - - - - | G1 - - - - - - - D2 - - - - - - -"),
        (P_HAT, 0.6, ". . h . . . h . . . h . . . h h | . . h . . . h . . . h . . . h . | . . h . . . h . . . h . . . h h | . . h . . . h . . . h . h . h ."),
    ],
};

const FIELD: SongText = SongText {
    bpm: 132.0,
    div: 4.0,
    tracks: &[
        (P_BRASS, 1.0, "A4 - - - C5 - E5 - A5 - - - G5 - E5 - | F5 - - - E5 - D5 - C5 - - - D5 - E5 - | E5 - - - G5 - - - C6 - - - B5 - G5 - | B5 - - - - - A5 - G5 - - - D5 - - - | A4 - - - C5 - E5 - A5 - - - G5 - E5 - | F5 - - - A5 - C6 - B5 - - - A5 - G5 - | A5 - - - G5 - E5 - C5 - - - D5 - E5 - | E5 - - - - - - - - - - - . . . ."),
        (P_STRINGS, 0.9, "A3+C4+E4 - - - - - - - - - - - - - - - | F3+A3+C4 - - - - - - - - - - - - - - - | C4+E4+G4 - - - - - - - - - - - - - - - | G3+B3+D4 - - - - - - - - - - - - - - - | A3+C4+E4 - - - - - - - - - - - - - - - | F3+A3+C4 - - - - - - - - - - - - - - - | C4+E4+G4 - - - - - - - - - - - - - - - | E3+G#3+B3 - - - - - - - - - - - - - - -"),
        (P_PBASS, 1.0, "A2 - A2 - A3 - A2 - A2 - A2 - E3 - A2 - | F2 - F2 - F3 - F2 - F2 - F2 - C3 - F2 - | C3 - C3 - C4 - C3 - C3 - C3 - G3 - C3 - | G2 - G2 - G3 - G2 - G2 - G2 - D3 - G2 - | A2 - A2 - A3 - A2 - A2 - A2 - E3 - A2 - | F2 - F2 - F3 - F2 - F2 - F2 - C3 - F2 - | C3 - C3 - C4 - C3 - C3 - C3 - G3 - C3 - | E2 - E2 - E3 - E2 - E2 - E2 - B2 - E2 -"),
        (P_KICK, 1.0, "k . . . . . . k k . . . . . . . | k . . . . . . k k . . . . . k ."),
        (P_SNARE, 1.0, ". . . . s . . . . . . . s . . . | . . . . s . . . . . . . s . s s"),
        (P_HAT, 1.0, ". . h . . . h . . . h . . . h h"),
    ],
};

const CRYPT: SongText = SongText {
    bpm: 70.0,
    div: 4.0,
    tracks: &[
        (P_ORGAN, 1.0, "E3+G3+B3 - - - - - - - - - - - - - - - | F3+A3+C4 - - - - - - - - - - - - - - - | E3+G3+B3 - - - - - - - - - - - - - - - | D#3+F#3+B3 - - - - - - - - - - - - - - -"),
        (P_BELL, 1.0, "E5 - - - . . . . G5 - - - . . F5 - | E5 - - - - - - - . . . . B4 - - - | C5 - - - . . B4 - A4 - - - G4 - - - | F#4 - - - - - - - B4 - - - - - - -"),
        (P_BASS, 0.8, "E2 - - - - - - - - - - - - - - - | F2 - - - - - - - - - - - - - - - | E2 - - - - - - - - - - - - - - - | B1 - - - - - - - - - - - - - - -"),
        (P_TOM, 0.8, "t . . . . . . . . . . . . . . . | t . . . . . . . . . . . t . . ."),
    ],
};

const SWAMP: SongText = SongText {
    bpm: 104.0,
    div: 4.0,
    tracks: &[
        (P_PBASS, 1.2, "D2 . D2 F2 . D2 C2 . D2 . D2 F2 . G2 A2 . | G2 . G2 Bb2 . G2 F2 . G2 . G2 Bb2 . C3 D3 . | D2 . D2 F2 . D2 C2 . D2 . D2 F2 . G2 A2 . | A1 . A1 C2 . A1 G1 . A1 . C2 . E2 . G2 ."),
        (P_BELL, 0.9, "A4 . C5 . D5 - - . F5 . E5 . D5 . C5 . | B4 . D5 . E5 - - . G5 . F5 . E5 . D5 . | A4 . C5 . D5 - - . F5 . A5 . G5 . F5 . | E5 - - - C#5 - - - E5 - - - . . . ."),
        (P_STRINGS, 0.6, "D3+F3+A3+C4 - - - - - - - - - - - - - - - | G3+B3+D4+F4 - - - - - - - - - - - - - - - | D3+F3+A3+C4 - - - - - - - - - - - - - - - | A2+C#3+E3+G3 - - - - - - - - - - - - - - -"),
        (P_KICK, 1.0, "k . . . . . . . k . k . . . . ."),
        (P_SNARE, 0.8, ". . . . . . s . . . . . . . s ."),
        (P_HAT, 0.9, ". . h . . h . . . . h . . h . h"),
    ],
};

const VOLCANO: SongText = SongText {
    bpm: 150.0,
    div: 4.0,
    tracks: &[
        (P_BRASS, 1.0, "C5 - - - Eb5 - G5 - C6 - - - Bb5 - G5 - | Ab5 - - - G5 - F5 - Eb5 - - - F5 - G5 - | Bb5 - - - Ab5 - G5 - F5 - - - G5 - Ab5 - | G5 - - - - - - - D5 - F5 - B5 - - -"),
        (P_PBASS, 1.0, "C2 C3 C2 C3 C2 C3 C2 C3 C2 C3 C2 C3 C2 C3 G2 G3 | Ab1 Ab2 Ab1 Ab2 Ab1 Ab2 Ab1 Ab2 Ab1 Ab2 Ab1 Ab2 Ab1 Ab2 Eb2 Eb3 | Bb1 Bb2 Bb1 Bb2 Bb1 Bb2 Bb1 Bb2 Bb1 Bb2 Bb1 Bb2 Bb1 Bb2 F2 F3 | G1 G2 G1 G2 G1 G2 G1 G2 G1 G2 G1 G2 B1 B2 D2 D3"),
        (P_STRINGS, 0.8, "C4+Eb4+G4 - - - - - - - - - - - - - - - | Ab3+C4+Eb4 - - - - - - - - - - - - - - - | Bb3+D4+F4 - - - - - - - - - - - - - - - | G3+B3+D4 - - - - - - - - - - - - - - -"),
        (P_KICK, 1.0, "k . . . k . . . k . . . k . k ."),
        (P_SNARE, 1.0, ". . s . . . s . . . s . . . s s"),
        (P_HAT, 1.0, "h . h h h . h h h . h h h . h h"),
    ],
};

const DUNGEON: SongText = SongText {
    bpm: 108.0,
    div: 4.0,
    tracks: &[
        (P_HARP, 0.9, "A3 C4 E4 A4 E4 C4 A3 C4 E4 A4 C5 A4 E4 C4 A3 E3 | D3 F3 A3 D4 A3 F3 D3 F3 A3 D4 F4 D4 A3 F3 D3 A2 | E3 G#3 B3 E4 B3 G#3 E3 G#3 B3 E4 G#4 E4 B3 G#3 E3 B2 | A3 C4 E4 A4 E4 C4 A3 C4 E4 A4 C5 A4 E4 C4 A3 E3"),
        (P_FLUTE, 0.9, "E5 - - - - - - - C5 - - - D5 - E5 - | F5 - - - - - E5 - D5 - - - A4 - - - | G#4 - - - - - B4 - D5 - - - C5 - B4 - | A4 - - - - - - - - - - - . . . ."),
        (P_STRINGS, 0.6, "A2+E3+A3 - - - - - - - - - - - - - - - | D3+F3+A3 - - - - - - - - - - - - - - - | E3+G#3+B3 - - - - - - - - - - - - - - - | A2+E3+A3 - - - - - - - - - - - - - - -"),
        (P_BASS, 0.7, "A1 - - - - - - - E2 - - - - - - - | D2 - - - - - - - A1 - - - - - - - | E2 - - - - - - - B1 - - - - - - - | A1 - - - - - - - E2 - - - - - - -"),
        (P_KICK, 0.7, "k . . . . . . . . . . . . . . . | k . . . . . . . k . . . . . . ."),
        (P_HAT, 0.6, ". . . . h . . . . . . . h . . ."),
    ],
};

// ---------------------------------------------------------------- lair interiors
const SHRINE: SongText = SongText {
    bpm: 100.0,
    div: 4.0,
    tracks: &[
        (P_FLUTE, 0.9, "A4 - - - D5 - C5 - A4 - G4 - F4 - G4 - | E4 - - - G4 - A4 - C5 - - - B4 - G4 - | D4 - - - B4 - - - A4 - G4 - F#4 - G4 - | A4 - - - - - - - . . . . D5 - C5 -"),
        (P_HARP, 0.8, "D3 F3 A3 D4 A3 F3 D3 F3 A3 D4 F4 D4 A3 F3 D3 F3 | C3 E3 G3 C4 G3 E3 C3 E3 G3 C4 E4 C4 G3 E3 C3 E3 | G2 B2 D3 G3 D3 B2 G2 B2 D3 G3 B3 G3 D3 B2 G2 B2 | D3 F3 A3 D4 A3 F3 D3 F3 A3 D4 F4 D4 A3 F3 D3 F3"),
        (P_STRINGS, 0.45, "D3+F3+A3 - - - - - - - - - - - - - - - | C3+E3+G3 - - - - - - - - - - - - - - - | G2+B2+D3 - - - - - - - - - - - - - - - | D3+F3+A3 - - - - - - - - - - - - - - -"),
        (P_BASS, 0.7, "D2 - - - - - A2 - D2 - - - - - A2 - | C2 - - - - - G2 - C2 - - - - - G2 - | G1 - - - - - D2 - G1 - - - - - D2 - | D2 - - - - - A2 - D2 - - - - - A2 -"),
        (P_HAT, 0.4, ". . h . . . h . . . h . . . h ."),
    ],
};

const CATACOMB: SongText = SongText {
    bpm: 80.0,
    div: 4.0,
    tracks: &[
        (P_ORGAN, 0.55, "C3+D#3+G3 - - - - - - - - - - - - - - - | G#2+C3+D#3 - - - - - - - - - - - - - - - | F2+G#2+C3 - - - - - - - - - - - - - - - | G2+B2+D3 - - - - - - - - - - - - - - -"),
        (P_BELL, 0.75, "G4 - - - - - - - D#4 - - - - - - - | C5 - - - - - - - G#4 - - - G4 - - - | F4 - - - - - G#4 - C5 - - - - - - - | B4 - - - D5 - - - G4 - - - - - - -"),
        (P_BASS, 0.75, "C2 - - - - - - - C2 - - - G2 - - - | G#1 - - - - - - - G#1 - - - D#2 - - - | F1 - - - - - - - F1 - - - C2 - - - | G1 - - - - - - - G1 - - - D2 - - -"),
        (P_TOM, 0.5, "t . . . . . . . . . . . t . . ."),
    ],
};

const CASTLE: SongText = SongText {
    bpm: 112.0,
    div: 4.0,
    tracks: &[
        (P_BRASS, 0.8, "E4 - - B3 E4 - G4 - F#4 - E4 - D#4 - B3 - | C4 - - G3 C4 - E4 - D4 - C4 - B3 - G3 - | A3 - C4 - E4 - A4 - G4 - F#4 - E4 - C4 - | B3 - - - D#4 - F#4 - B4 - - - - - . ."),
        (P_STRINGS, 0.5, "E3 . G3 . B3 . G3 . E3 . G3 . B3 . G3 . | C3 . E3 . G3 . E3 . C3 . E3 . G3 . E3 . | A2 . C3 . E3 . C3 . A2 . C3 . E3 . C3 . | B2 . D#3 . F#3 . D#3 . B2 . D#3 . F#3 . D#3 ."),
        (P_BASS, 0.75, "E2 - . E2 B2 - . B2 E2 - . E2 E3 - B2 - | C2 - . C2 G2 - . G2 C2 - . C2 C3 - G2 - | A1 - . A1 E2 - . E2 A1 - . A1 A2 - E2 - | B1 - . B1 F#2 - . F#2 B1 - . B1 B2 - F#2 -"),
        (P_SNARE, 0.6, "s . s s s . s . s . s s s s s ."),
        (P_KICK, 0.7, "k . . . k . . . k . . . k . . ."),
    ],
};

const FORTRESS: SongText = SongText {
    bpm: 138.0,
    div: 4.0,
    tracks: &[
        (P_PBASS, 1.0, "A1 A1 A2 A1 A1 A2 A1 E2 A1 A1 A2 A1 E2 A2 E2 A1 | A#1 A#1 A#2 A#1 A#1 A#2 A#1 F2 A#1 A#1 A#2 A#1 F2 A#2 F2 A#1 | G1 G1 G2 G1 G1 G2 G1 D2 G1 G1 G2 G1 D2 G2 D2 G1 | A1 A1 A2 A1 A1 A2 A1 E2 A1 A1 A2 A1 E2 A2 E2 A1"),
        (P_LEAD, 0.75, "A4 - - - A#4 - A4 - E5 - - - D5 - C5 - | D5 - - - F5 - E5 - D5 - A#4 - A4 - - - | G4 - - - A#4 - D5 - G5 - F5 - D5 - A#4 - | C#5 - - - E5 - - - A5 - - - - - . ."),
        (P_BRASS, 0.45, "A2+C3+E3 - - - - - - - - - - - - - - - | A#2+D3+F3 - - - - - - - - - - - - - - - | G2+A#2+D3 - - - - - - - - - - - - - - - | A2+C#3+E3 - - - - - - - - - - - - - - -"),
        (P_KICK, 0.9, "k . . k . . k . k . . k . . k ."),
        (P_SNARE, 0.7, ". . . . s . . . . . . . s . s s"),
        (P_HAT, 0.5, "h h h h h h h h h h h h h h h h"),
    ],
};

const SANCTUARY: SongText = SongText {
    bpm: 88.0,
    div: 4.0,
    tracks: &[
        (P_BELL, 0.98, "C6 - - - A5 - - - B5 - - - G5 - - - | D6 - - - B5 - - - A5 - G5 - E5 - - - | G5 - - - E5 - C6 - B5 - - - G5 - - - | A5 - - - - - - - F5 - - - - - - -"),
        (P_HARP, 0.84, "F4 A4 C5 A5 C6 A5 C5 A4 F4 A4 C5 A5 C6 A5 C5 A4 | G4 B4 D5 B5 D6 B5 D5 B4 G4 B4 D5 B5 D6 B5 D5 B4 | E4 G4 C5 G5 C6 G5 C5 G4 E4 G4 C5 G5 C6 G5 C5 G4 | F4 A4 C5 A5 C6 A5 C5 A4 F4 A4 C5 A5 C6 A5 C5 A4"),
        (P_STRINGS, 0.7, "F3+A3+C4 - - - - - - - - - - - - - - - | G3+B3+D4 - - - - - - - - - - - - - - - | E3+G3+C4 - - - - - - - - - - - - - - - | F3+A3+C4 - - - - - - - - - - - - - - -"),
        (P_BASS, 0.77, "F2 - - - - - - - C3 - - - - - - - | G2 - - - - - - - D3 - - - - - - - | C2 - - - - - - - G2 - - - - - - - | F2 - - - - - - - C3 - - - - - - -"),
    ],
};

const TOWER: SongText = SongText {
    bpm: 120.0,
    div: 4.0,
    tracks: &[
        (P_ORGAN, 0.62, "B2+D3+F#3 - - - - - - - - - - - - - - - | G2+B2+D3 - - - - - - - - - - - - - - - | C3+E3+G3 - - - - - - - - - - - - - - - | F#2+A#2+C#3 - - - - - - - - - - - - - - -"),
        (P_LEAD, 0.88, "B4 - - - C5 - B4 - A#4 - B4 - F#4 - - - | G4 - - - A#4 - B4 - D5 - C#5 - B4 - - - | C5 - - - E5 - G5 - F#5 - E5 - C5 - - - | C#5 - - - A#4 - - - F#4 - G4 - A#4 - C#5 -"),
        (P_PBASS, 1.12, "B1 . B1 . B2 . B1 . B1 . B1 . F#2 . B2 . | G1 . G1 . G2 . G1 . G1 . G1 . D2 . G2 . | C2 . C2 . C3 . C2 . C2 . C2 . G2 . C3 . | F#1 . F#1 . F#2 . F#1 . F#1 . F#1 . C#2 . F#2 ."),
        (P_TOM, 0.69, "t . . . . . t . . . t . . . . ."),
        (P_KICK, 1.0, "k . . . k . . . k . . . k . k ."),
    ],
};

const LAIR: SongText = SongText {
    bpm: 168.0,
    div: 4.0,
    tracks: &[
        (P_LEAD, 1.2, "E5 F5 E5 D#5 E5 - B4 - C5 B4 A#4 B4 - - E4 - | E5 F5 E5 D#5 E5 - G5 - F#5 E5 D#5 E5 - - B4 - | C5 D5 C5 B4 C5 - G4 - A4 G4 F#4 G4 - - C5 - | B4 C5 B4 A#4 B4 - F#5 - D#5 - B4 - F#4 - - -"),
        (P_PBASS, 1.1, "E2 E2 E3 E2 E2 E3 E2 F2 E2 E2 E3 E2 F2 F3 F2 E2 | E2 E2 E3 E2 E2 E3 E2 F2 E2 E2 E3 E2 G2 G3 F#2 E2 | C2 C2 C3 C2 C2 C3 C2 D2 C2 C2 C3 C2 D2 D3 C2 B1 | B1 B1 B2 B1 B1 B2 B1 C2 B1 B1 B2 B1 F#2 F#2 B1 B1"),
        (P_STRINGS, 0.8, "E3+G3+B3 - - - . . . . E3+G3+B3 - - - . . . . | E3+G3+B3 - - - . . . . E3+G3+C4 - - - . . . . | C3+E3+G3 - - - . . . . C3+E3+A3 - - - . . . . | B2+D#3+F#3 - - - - - - - B2+D#3+F#3 - - - - - - -"),
        (P_KICK, 1.0, "k . . . k . . . k . . . k . k k"),
        (P_SNARE, 1.0, ". . s . . . s . . . s . . . s s"),
        (P_HAT, 1.0, "h h h h h h h h h h h h h h h h"),
    ],
};

/// One note (or chord) in a track: start step, length in steps, MIDI notes.
struct Ev {
    start: usize,
    len: usize,
    notes: Vec<u8>,
    drum: usize,
}

struct Track {
    patch: usize,
    vol: f32,
    events: Vec<Ev>,
}

struct SongData {
    step_s: f32,
    len: usize,
    tracks: Vec<Track>,
}

fn note_midi(t: &str) -> Option<u8> {
    let b = t.as_bytes();
    let base = match b.first()? {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => return None,
    };
    let (acc, rest) = match b.get(1) {
        Some(b'#') => (1, &t[2..]),
        Some(b'b') => (-1, &t[2..]),
        _ => (0, &t[1..]),
    };
    let oct: i32 = rest.parse().ok()?;
    Some(((oct + 1) * 12 + base + acc) as u8)
}

fn parse(s: &SongText) -> SongData {
    let mut tracks = vec![];
    let mut len = 0;
    for &(patch, vol, text) in s.tracks {
        let steps: Vec<&str> = text.split_whitespace().filter(|t| *t != "|").collect();
        let mut events: Vec<Ev> = vec![];
        for (i, &tok) in steps.iter().enumerate() {
            match tok {
                "." => {}
                "-" => {
                    if let Some(e) = events.last_mut() {
                        if e.start + e.len == i {
                            e.len += 1;
                        }
                    }
                }
                "k" | "s" | "h" | "t" => {
                    let drum = match tok {
                        "k" => P_KICK,
                        "s" => P_SNARE,
                        "h" => P_HAT,
                        _ => P_TOM,
                    };
                    events.push(Ev { start: i, len: 1, notes: vec![if drum == P_TOM { 45 } else { 36 }], drum });
                }
                _ => {
                    let notes: Vec<u8> = tok.split('+').filter_map(note_midi).collect();
                    if !notes.is_empty() {
                        events.push(Ev { start: i, len: 1, notes, drum: usize::MAX });
                    }
                }
            }
        }
        len = len.max(steps.len());
        tracks.push(Track { patch, vol, events });
    }
    SongData { step_s: 60.0 / s.bpm / s.div, len: len.max(1), tracks }
}

// ---------------------------------------------------------------- voices
/// A music note through a patch, with ADSR envelope.
struct MVoice {
    p: Patch,
    freq: f32,
    phase: [f32; 3],
    sub_phase: f32,
    t: f32,
    gate: f32,
    env: f32,
    rel_from: f32,
    released: bool,
    lp: [f32; 2],
    vel: f32,
    done: bool,
}

/// A sound-effect voice: exponential pitch and volume sweeps.
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
}

pub struct Synth {
    sr: f32,
    voices: Vec<Voice>,
    mvoices: Vec<MVoice>,
    patches: Vec<Patch>,
    songs: Vec<SongData>,
    song: Option<Song>,
    step: usize,
    step_left: f32,
    pub muted: bool,
    noise: u32,
    /// Stereo echo buffers and write position.
    echo: [Vec<f32>; 2],
    echo_pos: usize,
    echo_lp: [f32; 2],
    /// Fades music in after a song change (0..1).
    fade: f32,
}

fn mtof(m: u8) -> f32 {
    440.0 * 2f32.powf((m as f32 - 69.0) / 12.0)
}

impl Synth {
    pub fn new(sr: f32) -> Self {
        let order = [TITLE, FIELD, LAIR, VILLAGE, CRYPT, SWAMP, VOLCANO, DUNGEON, SHRINE, CATACOMB, CASTLE, FORTRESS, SANCTUARY, TOWER];
        let songs: Vec<SongData> = order.iter().map(parse).collect();
        debug_assert_eq!(songs.len(), SONG_COUNT);
        let echo_len = |s: f32| vec![0.0; (sr * s) as usize + 1];
        Synth {
            sr,
            voices: Vec::new(),
            mvoices: Vec::new(),
            patches: patches(),
            songs,
            song: None,
            step: 0,
            step_left: 0.0,
            muted: false,
            noise: 0x1234_5678,
            echo: [echo_len(0.24), echo_len(0.31)],
            echo_pos: 0,
            echo_lp: [0.0; 2],
            fade: 1.0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn push(&mut self, wave: Wave, f0: f32, f1: f32, dur: f32, vol: f32, delay: f32, cutoff: f32) {
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
        };
        if self.voices.len() >= 48 {
            self.voices.remove(0);
        }
        self.voices.push(v);
    }
    fn tone(&mut self, w: Wave, f0: f32, f1: f32, dur: f32, vol: f32, delay: f32) {
        self.push(w, f0, f1, dur, vol, delay, 1000.0);
    }
    fn noise(&mut self, dur: f32, vol: f32, cut: f32, delay: f32) {
        self.push(Wave::Noise, 100.0, 100.0, dur, vol, delay, cut);
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
            Sfx::Ignite => {
                self.noise(0.3, 0.18, 3000.0, 0.0);
                self.tone(Saw, 200.0, 500.0, 0.2, 0.04, 0.0);
            }
            Sfx::Shatter => {
                self.noise(0.25, 0.25, 9000.0, 0.0);
                for i in 0..4 {
                    let f = 2400.0 + i as f32 * 500.0;
                    self.tone(Square, f, f * 0.7, 0.05, 0.03, i as f32 * 0.03);
                }
            }
            Sfx::Roar => {
                self.noise(1.0, 0.4, 900.0, 0.0);
                self.tone(Saw, 110.0, 55.0, 1.0, 0.12, 0.0);
                self.tone(Square, 90.0, 60.0, 0.9, 0.05, 0.05);
            }
            Sfx::Unlock => {
                self.tone(Square, 1200.0, 1200.0, 0.04, 0.05, 0.0);
                self.tone(Square, 800.0, 800.0, 0.06, 0.05, 0.06);
                self.arp(&[523.0, 784.0, 1046.0], 0.08, 0.1);
            }
            Sfx::Rumble => {
                self.noise(1.2, 0.35, 400.0, 0.0);
                self.tone(Sine, 50.0, 35.0, 1.2, 0.3, 0.0);
            }
            Sfx::Push => self.noise(0.12, 0.15, 700.0, 0.0),
            Sfx::Click => {
                self.tone(Square, 1500.0, 900.0, 0.03, 0.05, 0.0);
                self.tone(Square, 600.0, 600.0, 0.05, 0.04, 0.05);
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
        // Let playing notes ring out briefly instead of cutting them off.
        for v in self.mvoices.iter_mut() {
            v.released = true;
            v.rel_from = v.env;
            v.t = v.gate;
        }
        self.fade = 0.0;
    }

    fn note_on(&mut self, patch: usize, midi: u8, gate: f32, vel: f32) {
        let p = self.patches[patch];
        if self.mvoices.len() >= 40 {
            self.mvoices.remove(0);
        }
        self.mvoices.push(MVoice {
            p, freq: mtof(midi), phase: [0.0, 0.33, 0.66], sub_phase: 0.0, t: 0.0, gate, env: 0.0, rel_from: 0.0,
            released: false, lp: [0.0; 2], vel, done: false,
        });
    }

    fn song_step(&mut self, s: Song) {
        let (step_s, len) = {
            let d = &self.songs[s as usize];
            (d.step_s, d.len)
        };
        let i = self.step % len;
        let mut on: Vec<(usize, u8, f32, f32)> = vec![];
        for tr in &self.songs[s as usize].tracks {
            // Tracks shorter than the song loop on their own length.
            let tl = tr.events.last().map_or(1, |e| e.start + e.len).max(1);
            let tl = if tl <= len && len % tl == 0 { tl } else { len };
            let j = i % tl;
            for e in tr.events.iter().filter(|e| e.start == j) {
                let patch = if e.drum != usize::MAX { e.drum } else { tr.patch };
                let gate = (e.len as f32 * step_s - 0.012).max(0.02);
                for &n in &e.notes {
                    on.push((patch, n, gate, tr.vol));
                }
            }
        }
        for (p, n, g, v) in on {
            self.note_on(p, n, g, v);
        }
        self.step += 1;
    }

    /// One stereo sample of all music voices.
    fn music_sample(&mut self, nz: &mut u32) -> (f32, f32) {
        let sr = self.sr;
        let (mut l, mut r) = (0.0f32, 0.0f32);
        for v in self.mvoices.iter_mut() {
            let p = v.p;
            let dt = 1.0 / sr;
            // ADSR.
            if !v.released && v.t >= v.gate {
                v.released = true;
                v.rel_from = v.env;
            }
            v.env = if v.released {
                let k = ((v.t - v.gate) / p.r.max(0.005)).min(1.0);
                v.rel_from * (1.0 - k) * (1.0 - k)
            } else if v.t < p.a {
                v.t / p.a.max(0.001)
            } else {
                let k = ((v.t - p.a) / p.d.max(0.001)).min(1.0);
                1.0 + (p.s - 1.0) * k
            };
            if v.released && v.t > v.gate + p.r {
                v.done = true;
                continue;
            }
            let vib = if p.vib_depth > 0.0 && v.t > p.vib_delay {
                1.0 + (TAU * p.vib_rate * v.t).sin() * p.vib_depth
            } else {
                1.0
            };
            let drop = if p.drop < 1.0 { p.drop.powf((v.t / 0.1).min(3.0)) } else { 1.0 };
            let f = v.freq * vib * drop;
            let mut sl;
            let mut sr_ = 0.0;
            let mut stereo = false;
            match p.osc {
                Osc::Pulse(duty) => sl = if v.phase[0] < duty { 1.0 } else { -1.0 },
                Osc::Tri => sl = 4.0 * (v.phase[0] - 0.5).abs() - 1.0,
                Osc::Saw => sl = 2.0 * v.phase[0] - 1.0,
                Osc::Sine => sl = (v.phase[0] * TAU).sin(),
                Osc::Organ => {
                    let ph = v.phase[0] * TAU;
                    sl = ph.sin() * 0.6 + (ph * 2.0).sin() * 0.3 + (ph * 3.0).sin() * 0.15;
                }
                Osc::Bell => {
                    let ph = v.phase[0] * TAU;
                    sl = ph.sin() * 0.7 + (ph * 2.76).sin() * 0.4 * (-v.t * 9.0).exp();
                }
                Osc::SuperSaw => {
                    let s0 = 2.0 * v.phase[0] - 1.0;
                    let s1 = 2.0 * v.phase[1] - 1.0;
                    let s2 = 2.0 * v.phase[2] - 1.0;
                    sl = s0 * 0.6 + s1 * 0.6;
                    sr_ = s0 * 0.6 + s2 * 0.6;
                    stereo = true;
                }
                Osc::Noise => {
                    *nz ^= *nz << 13;
                    *nz ^= *nz >> 17;
                    *nz ^= *nz << 5;
                    sl = (*nz as f32 / u32::MAX as f32) * 2.0 - 1.0;
                }
            }
            let inc = f / sr;
            v.phase[0] = (v.phase[0] + inc).fract();
            if p.osc == Osc::SuperSaw {
                v.phase[1] = (v.phase[1] + inc * 1.0045).fract();
                v.phase[2] = (v.phase[2] + inc * 0.9955).fract();
            }
            if p.sub > 0.0 {
                v.sub_phase = (v.sub_phase + inc * 0.5).fract();
                let sub = 4.0 * (v.sub_phase - 0.5).abs() - 1.0;
                sl += sub * p.sub;
                sr_ += sub * p.sub;
            }
            if !stereo {
                sr_ = sl;
            }
            // Low-pass filter with an optional opening envelope.
            let cut = p.cut * (1.0 + (p.cut_env - 1.0) * (-v.t / p.cut_decay.max(0.001)).exp());
            let a = (cut * TAU / sr).min(1.0);
            v.lp[0] += (sl - v.lp[0]) * a;
            v.lp[1] += (sr_ - v.lp[1]) * a;
            let g = v.env * p.gain * v.vel;
            let (pl, pr) = ((1.0 - p.pan).min(1.0), (1.0 + p.pan).min(1.0));
            l += v.lp[0] * g * pl;
            r += v.lp[1] * g * pr;
            v.t += dt;
        }
        (l, r)
    }
}

impl AudioCallback for Synth {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        let sr = self.sr;
        let mut nz = self.noise;
        for frame in out.chunks_mut(2) {
            if let Some(s) = self.song {
                if !self.muted {
                    self.step_left -= 1.0;
                    if self.step_left <= 0.0 {
                        self.song_step(s);
                        self.step_left += self.songs[s as usize].step_s * sr;
                    }
                }
            }
            self.fade = (self.fade + 1.0 / (sr * 0.6)).min(1.0);
            let (ml, mr) = self.music_sample(&mut nz);
            let (ml, mr) = (ml * self.fade, mr * self.fade);
            let mut fx = 0.0f32;
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
                fx += smp * v.gain;
                v.gain *= v.gmul;
                v.left -= 1;
            }
            // Stereo echo with a darkening feedback path (SNES-style).
            let pos = self.echo_pos;
            let mut wet = [0.0f32; 2];
            for ch in 0..2 {
                let buf = &mut self.echo[ch];
                let i = pos % buf.len();
                let delayed = buf[i];
                self.echo_lp[ch] += (delayed - self.echo_lp[ch]) * 0.35;
                let send = (if ch == 0 { ml } else { mr }) * 0.55 + fx * 0.2;
                buf[i] = send + self.echo_lp[ch] * 0.38;
                wet[ch] = delayed;
            }
            self.echo_pos = pos + 1;
            let mix = |dry_m: f32, w: f32| ((dry_m * 0.9 + fx * 0.8 + w * 0.32) * 0.55).clamp(-1.0, 1.0);
            let (l, r) = if self.muted { (0.0, 0.0) } else { (mix(ml, wet[0]), mix(mr, wet[1])) };
            frame[0] = l;
            if frame.len() > 1 {
                frame[1] = r;
            }
        }
        self.noise = nz;
        self.voices.retain(|v| v.left > 0 || v.delay > 0);
        self.mvoices.retain(|v| !v.done);
    }
}

/// Render every song to a 16-bit stereo WAV (<dir>/<song>.wav, secs long) so the
/// music can be reviewed without running the game: elementallegends --render-music <dir>.
pub fn render_music(dir: &str, secs: f32) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let sr = 44100u32;
    let songs = [
        Song::Title, Song::Village, Song::Field, Song::Crypt, Song::Swamp, Song::Volcano, Song::Dungeon, Song::Lair, Song::Shrine,
        Song::Catacomb, Song::Castle, Song::Fortress, Song::Sanctuary, Song::Tower,
    ];
    for song in songs {
        let mut s = Synth::new(sr as f32);
        s.play_song(Some(song));
        let frames = (secs * sr as f32) as usize;
        let mut buf = vec![0.0f32; frames * 2];
        for chunk in buf.chunks_mut(2048) {
            s.callback(chunk);
        }
        let mut wav = Vec::with_capacity(44 + buf.len() * 2);
        let data_len = (buf.len() * 2) as u32;
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&sr.to_le_bytes());
        wav.extend_from_slice(&(sr * 4).to_le_bytes());
        wav.extend_from_slice(&4u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        for v in &buf {
            wav.extend_from_slice(&((v.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
        }
        let name = format!("{song:?}").to_lowercase();
        std::fs::write(format!("{dir}/{name}.wav"), wav)?;
        println!("wrote {dir}/{name}.wav");
    }
    Ok(())
}

pub fn open(a: &AudioSubsystem) -> Option<AudioDevice<Synth>> {
    let spec = AudioSpecDesired { freq: Some(44100), channels: Some(2), samples: Some(1024) };
    let dev = a.open_playback(None, &spec, |s| Synth::new(s.freq as f32)).ok()?;
    dev.resume();
    Some(dev)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_parse() {
        assert_eq!(note_midi("A4"), Some(69));
        assert_eq!(note_midi("C4"), Some(60));
        assert_eq!(note_midi("F#3"), Some(54));
        assert_eq!(note_midi("Bb2"), Some(46));
        assert_eq!(note_midi("x"), None);
    }

    #[test]
    fn every_song_parses_and_renders_sound() {
        let mut s = Synth::new(22050.0);
        for song in [
            Song::Title, Song::Field, Song::Lair, Song::Village, Song::Crypt, Song::Swamp, Song::Volcano, Song::Dungeon, Song::Shrine,
            Song::Catacomb, Song::Castle, Song::Fortress, Song::Sanctuary, Song::Tower,
        ] {
            let d = &s.songs[song as usize];
            assert!(d.len >= 16, "{song:?} has a full bar");
            assert!(d.tracks.iter().all(|t| !t.events.is_empty()), "{song:?} tracks have notes");
            s.play_song(Some(song));
            let mut buf = vec![0.0f32; 22050 * 2];
            s.callback(&mut buf);
            let peak = buf.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.02 && peak <= 1.0, "{song:?} is audible and not clipping hard (peak {peak})");
            assert!(buf.iter().all(|v| v.is_finite()));
        }
    }
}
