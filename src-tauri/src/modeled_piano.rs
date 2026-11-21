//! Sample-free modal piano. A strike excites the modes of a stiff string at the
//! hammer position. Contact bandwidth, string losses and unison tuning vary
//! across the keyboard. Bridge motion feeds a shared soundboard and open strings.
use std::f32::consts::{PI, TAU};

const POLYPHONY: usize = 48;
const MODES: usize = 32;
const BLOCK: usize = 64;

#[derive(Clone, Copy)]
struct Character {
    felt: f32,
    decay: f32,
    resonance: f32,
}

#[derive(Clone, Copy)]
struct Channel {
    sustain: bool,
    sostenuto: bool,
    soft: bool,
    bend: f32,
    volume: f32,
    expression: f32,
    pan: f32,
}
impl Default for Channel {
    fn default() -> Self {
        Self {
            sustain: false,
            sostenuto: false,
            soft: false,
            bend: 1.,
            volume: 1.,
            expression: 1.,
            pan: 0.,
        }
    }
}

fn hz(note: u8) -> f32 {
    440. * 2f32.powf((note as f32 - 69.) / 12.)
}

// Amplitude multiplier for a 60 dB decay over the specified number of seconds.
fn loss(seconds: f32, rate: f32) -> f32 {
    (-std::f32::consts::LN_10 * 3. / (seconds * rate)).exp()
}

#[derive(Clone, Copy, Default)]
struct Mode {
    x: f32,
    y: f32,
    real: f32,
    imag: f32,
    hz: f32,
    seconds: f32,
}
impl Mode {
    fn tune(&mut self, rate: f32, bend: f32, decay_scale: f32) {
        let radius = loss(self.seconds * decay_scale, rate);
        let (sin, cos) = (TAU * self.hz * bend / rate).sin_cos();
        self.real = cos * radius;
        self.imag = sin * radius;
    }
}

struct Voice {
    channel: usize,
    key: u8,
    note: u8,
    // Bit 0 is hardware, bit 1 is the onscreen key.
    owners: u8,
    latched: bool,
    modes: [Mode; MODES],
    string_modes: usize,
    count: usize,
    longest_t60: f32,
    level: f32,
    natural_loss: f32,
    damper_loss: f32,
    damper: f32,
    attack: f32,
    attack_loss: f32,
    pan: f32,
}
impl Voice {
    fn strike(
        channel: usize,
        key: u8,
        note: u8,
        velocity: u8,
        rate: f32,
        felt: f32,
        soft: bool,
    ) -> Self {
        let fundamental = hz(note);
        let touch = velocity as f32 / 127. * if soft { 0.75 } else { 1. };
        let impact = 0.16 * touch.powf(1.45);
        let hardness = (0.16 + 0.72 * felt + 0.63 * touch).clamp(0.1, 1.4);
        let register = ((note as f32 - 60.) / 27.).clamp(0., 1.);
        let treble = register * register * (3. - 2. * register);
        let bandwidth = fundamental * (1.8 - treble) + 450. + 2400. * hardness * hardness;
        let slope = 1.42 + treble * (2.7 - 0.8 * hardness);
        let strike_position = 0.115 + 0.02 * ((note as f32 - 21.) / 87.).clamp(0., 1.);
        let stiffness = 0.00009 * 2f32.powf((note as f32 - 55.).abs() / 24.);
        let lifetime = (9.0 * (220. / fundamental).powf(0.32)).clamp(2., 19.);
        let mut v = Self {
            channel,
            key,
            note,
            owners: 0,
            latched: false,
            modes: [Mode::default(); MODES],
            string_modes: 0,
            count: 0,
            longest_t60: 0.,
            level: impact,
            natural_loss: 1.,
            damper_loss: loss(
                (0.14 * (220. / fundamental).powf(0.3)).clamp(0.075, 0.34),
                rate,
            ),
            damper: 1.,
            attack: 1.,
            attack_loss: loss(0.008 / hardness, rate),
            pan: ((note as f32 - 62.) / 100.).clamp(-0.42, 0.42),
        };
        // Bass strings have two slightly split transverse polarizations. Higher
        // notes add unison strings with different tuning and bridge loss.
        let spread = if note < 40 {
            0.24
        } else {
            1.25 - 0.45 * treble
        };
        // The differently coupled unisons give a prompt decay and a quieter
        // aftersound. Shorter strings lose energy more quickly on both slopes.
        let top_octaves = ((note as f32 - 84.) / 24.).clamp(0., 1.);
        let treble_decay = (1_046.5 / fundamental).powf(0.38 + 0.24 * top_octaves);
        let strings = [
            (18, 1., 0.64, 0.76, 4.4),
            (7, 2f32.powf(spread / 1200.), 0.26, 1.3, 1.85),
            (3, 2f32.powf(-0.8 * spread / 1200.), 0.10, 1.05, 1.45),
        ];
        for (partials, detune, share, decay, treble_t60) in strings {
            let bass_t60 = lifetime * decay;
            let string_t60 = bass_t60 + treble * (treble_t60 * treble_decay - bass_t60);
            v.longest_t60 = v.longest_t60.max(string_t60);
            for harmonic in 1..=partials {
                let n = harmonic as f32;
                let frequency =
                    fundamental * detune * n * ((1. + stiffness * n * n) / (1. + stiffness)).sqrt();
                // Leave headroom for the two-semitone bend range.
                if frequency >= rate * 0.44 {
                    break;
                }
                let position = (PI * n * strike_position).sin() / (PI * strike_position).sin();
                let contact = (-0.5 * (frequency / bandwidth).powi(2)).exp();
                let amplitude = impact * share * position * contact / n.powf(slope);
                if amplitude.abs() < 0.0000005 {
                    continue;
                }
                v.modes[v.count] = Mode {
                    x: amplitude,
                    hz: frequency,
                    seconds: string_t60 / (1. + 0.24 * (n - 1.).powf(1.23)),
                    ..Default::default()
                };
                v.count += 1;
            }
        }
        v.string_modes = v.count;
        // The initial hammer impulse also excites broad, unpitched soundboard
        // modes. Their frequencies belong to the instrument, independent of
        // pitch bend, and their brief settling time is independent of string loss.
        if treble > 0. && v.string_modes > 0 {
            let coupling = treble * (fundamental / 1_600.).powf(0.75).min(2.5);
            let knock = impact * coupling * (0.9 + 0.38 * hardness);
            let bridge_position = ((note as f32 - 60.) / 48.).clamp(0., 1.);
            for (hz, seconds, share) in [
                (245., 0.097, 0.55 + 0.15 * bridge_position),
                (487., 0.075, -0.82 + 0.3 * bridge_position),
                (917., 0.055, 0.65 - 0.3 * bridge_position),
                (1_681., 0.037, -0.35 - 0.15 * bridge_position),
            ] {
                let mode = &mut v.modes[v.count];
                mode.x = knock * share;
                mode.hz = hz;
                mode.seconds = seconds;
                mode.tune(rate, 1., 1.);
                v.count += 1;
            }
        }
        v
    }
    fn tune(&mut self, rate: f32, bend: f32, decay: f32) {
        let scale = 0.4 + 1.2 * decay;
        self.natural_loss = loss(self.longest_t60 * scale, rate);
        for mode in &mut self.modes[..self.string_modes] {
            mode.tune(rate, bend, scale);
        }
    }
    fn render(&mut self, channel: Channel, left: &mut [f32], right: &mut [f32]) {
        // Advance four independent partials together. The final group is padded
        // with silent modes at construction so each recurrence can use SIMD.
        let mut signal = [[0.; 4]; BLOCK];
        for group in self.modes[..self.count.next_multiple_of(4)].chunks_exact_mut(4) {
            let mut x: [f32; 4] = std::array::from_fn(|i| group[i].x);
            let mut y: [f32; 4] = std::array::from_fn(|i| group[i].y);
            let real: [f32; 4] = std::array::from_fn(|i| group[i].real);
            let imag: [f32; 4] = std::array::from_fn(|i| group[i].imag);
            for lane in 0..4 {
                if x[lane].abs() + y[lane].abs() < 1e-10 {
                    x[lane] = 0.;
                    y[lane] = 0.;
                }
            }
            for frame in &mut signal[..left.len()] {
                for i in 0..4 {
                    let next_x = real[i] * x[i] - imag[i] * y[i];
                    y[i] = imag[i] * x[i] + real[i] * y[i];
                    x[i] = next_x;
                    frame[i] += y[i];
                }
            }
            for (i, mode) in group.iter_mut().enumerate() {
                mode.x = x[i];
                mode.y = y[i];
            }
        }
        if self.attack < 1e-10 {
            self.attack = 0.;
        }
        let pan = (self.pan + channel.pan).clamp(-1., 1.);
        let gain = channel.volume * channel.expression;
        let left_gain = gain * (0.5 * (1. - pan)).sqrt();
        let right_gain = gain * (0.5 * (1. + pan)).sqrt();
        let released = self.owners == 0 && !channel.sustain && !self.latched;
        // As on an acoustic piano the highest treble strings have no dampers.
        let damping = if released && self.note < 90 {
            self.damper_loss
        } else {
            1.
        };
        let level_loss = self.natural_loss * damping;
        for ((l, r), sample) in left.iter_mut().zip(right).zip(signal) {
            self.attack *= self.attack_loss;
            self.damper *= damping;
            self.level *= level_loss;
            let out = sample.iter().sum::<f32>() * self.damper * (1. - self.attack);
            *l += out * left_gain;
            *r += out * right_gain;
        }
    }
}

// A driven pair of quadrature states has bounded gain at its plate/string mode.
#[derive(Clone, Copy)]
struct Resonator {
    x: f32,
    y: f32,
    real: f32,
    imag: f32,
    drive: f32,
}
impl Resonator {
    fn new(frequency: f32, seconds: f32, rate: f32) -> Self {
        let radius = loss(seconds, rate);
        let (sin, cos) = (TAU * frequency / rate).sin_cos();
        Self {
            x: 0.,
            y: 0.,
            real: radius * cos,
            imag: radius * sin,
            drive: 2. * (1. - radius),
        }
    }
    fn next(&mut self, drive: f32) -> f32 {
        let x = self.real * self.x - self.imag * self.y + drive * self.drive;
        self.y = self.imag * self.x + self.real * self.y;
        self.x = x;
        x
    }
    fn clear(&mut self) {
        self.x = 0.;
        self.y = 0.;
    }
}

pub struct ModeledPiano {
    rate: f32,
    octave: i8,
    character: Character,
    channels: [Channel; 16],
    voices: Vec<Voice>,
    board: [Resonator; 4],
    strings: [Resonator; 36],
    string_damper: f32,
    ring_frames: usize,
}
impl ModeledPiano {
    pub fn new(sample_rate: i32) -> Self {
        let rate = sample_rate.max(8000) as f32;
        Self {
            rate,
            octave: 0,
            character: Character {
                felt: 0.5,
                decay: 0.5,
                resonance: 0.5,
            },
            channels: [Channel::default(); 16],
            voices: Vec::with_capacity(POLYPHONY),
            board: [(97., 0.075), (203., 0.065), (411., 0.04), (830., 0.027)]
                .map(|(freq, time)| Resonator::new(freq, time, rate)),
            strings: std::array::from_fn(|i| Resonator::new(hz(i as u8 + 48), 1.9, rate)),
            string_damper: loss(0.09, rate),
            ring_frames: 0,
        }
    }
    pub fn set_character(&mut self, felt: f32, decay: f32, resonance: f32) {
        let normalized = |v: f32| if v.is_finite() { v.clamp(0., 1.) } else { 0.5 };
        let decay = normalized(decay);
        if decay != self.character.decay {
            for v in &mut self.voices {
                v.tune(self.rate, self.channels[v.channel].bend, decay);
            }
        }
        self.character = Character {
            felt: normalized(felt),
            decay,
            resonance: normalized(resonance),
        };
    }
    fn clear_body(&mut self) {
        for mode in self.board.iter_mut().chain(&mut self.strings) {
            mode.clear();
        }
        self.ring_frames = 0;
    }
    pub fn panic(&mut self) {
        self.voices.clear();
        self.channels.fill(Channel::default());
        self.clear_body();
    }
    pub fn set_octave(&mut self, octave: i8) {
        if self.octave != octave {
            self.panic();
            self.octave = octave;
        }
    }
    pub fn midi(&mut self, screen: bool, bytes: [u8; 3]) {
        let [status, key, value] = bytes;
        let owner = if screen { 2 } else { 1 };
        if screen && bytes == [0, 0, 0] {
            for v in &mut self.voices {
                v.owners &= !owner;
            }
            return;
        }
        if key > 127 || value > 127 {
            return;
        }
        let ch = (status & 15) as usize;
        let channel = &mut self.channels[ch];
        match status & 0xf0 {
            0x90 if value != 0 => {
                let note = key as i16 + 12 * self.octave as i16;
                if !(0..=127).contains(&note) {
                    return;
                }
                let mut voice = Voice::strike(
                    ch,
                    key,
                    note as u8,
                    value,
                    self.rate,
                    self.character.felt,
                    channel.soft,
                );
                voice.owners = owner;
                for old in self
                    .voices
                    .iter_mut()
                    .filter(|v| v.channel == ch && v.key == key)
                {
                    voice.owners |= old.owners;
                    voice.latched |= old.latched;
                    old.owners = 0;
                }
                voice.tune(self.rate, channel.bend, self.character.decay);
                if self.voices.len() == POLYPHONY {
                    let quietest = self
                        .voices
                        .iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| a.level.total_cmp(&b.level))
                        .unwrap()
                        .0;
                    self.voices.swap_remove(quietest);
                }
                self.voices.push(voice);
            }
            0x80 | 0x90 => {
                for v in self
                    .voices
                    .iter_mut()
                    .filter(|v| v.channel == ch && v.key == key)
                {
                    v.owners &= !owner;
                }
            }
            0xe0 => {
                let pitch = ((value as i32) << 7) + key as i32 - 8192;
                channel.bend = 2f32.powf(pitch as f32 / (8192. * 6.));
                for v in self.voices.iter_mut().filter(|v| v.channel == ch) {
                    v.tune(self.rate, channel.bend, self.character.decay);
                }
            }
            0xb0 => match key {
                7 => channel.volume = value as f32 / 127.,
                10 => channel.pan = ((value as f32 - 64.) / 63.).clamp(-1., 1.),
                11 => channel.expression = value as f32 / 127.,
                64 => channel.sustain = value >= 64,
                66 => {
                    let down = value >= 64;
                    if down != channel.sostenuto {
                        for v in self.voices.iter_mut().filter(|v| v.channel == ch) {
                            v.latched = down && v.owners != 0;
                        }
                        channel.sostenuto = down;
                    }
                }
                67 => channel.soft = value >= 64,
                120 => {
                    self.voices.retain(|v| v.channel != ch);
                    if self.voices.is_empty() {
                        self.clear_body();
                    }
                }
                121 => {
                    let volume = channel.volume;
                    *channel = Channel {
                        volume,
                        ..Default::default()
                    };
                    for v in self.voices.iter_mut().filter(|v| v.channel == ch) {
                        v.latched = false;
                        v.tune(self.rate, 1., self.character.decay);
                    }
                }
                123 => {
                    for v in self.voices.iter_mut().filter(|v| v.channel == ch) {
                        v.owners = 0;
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        left.fill(0.);
        right.fill(0.);
        let pedal = self.channels.iter().any(|ch| ch.sustain);
        let mut open = [pedal; 36];
        for v in &self.voices {
            if (48..84).contains(&v.note) && (v.owners != 0 || v.latched) {
                open[(v.note - 48) as usize] = true;
            }
        }
        for (l, r) in left.chunks_mut(BLOCK).zip(right.chunks_mut(BLOCK)) {
            let frames = l.len().min(r.len());
            let (l, r) = (&mut l[..frames], &mut r[..frames]);
            if self.voices.is_empty() {
                if self.ring_frames == 0 {
                    break;
                }
                self.ring_frames = self.ring_frames.saturating_sub(frames);
            } else {
                self.ring_frames = (self.rate * 2.) as usize;
                for v in &mut self.voices {
                    v.render(self.channels[v.channel], l, r);
                }
                self.voices.retain(|v| v.level > 0.000001 && v.count > 0);
            }
            for (left, right) in l.iter_mut().zip(r.iter_mut()) {
                let bridge = (*left + *right) * 0.5;
                let [a, b, c, d] = &mut self.board;
                let low = a.next(bridge) + b.next(bridge);
                let mid = c.next(bridge) + d.next(bridge);
                *left += low * 0.22 + mid * 0.16;
                *right += low * 0.16 + mid * 0.22;
            }
            if self.character.resonance > 0. {
                let mut ringing = [0.; BLOCK];
                for (mode, open) in self.strings.iter_mut().zip(open) {
                    if open {
                        for ((l, r), out) in l.iter().zip(r.iter()).zip(&mut ringing) {
                            *out += mode.next((l + r) * 0.5);
                        }
                    } else if mode.x.abs() + mode.y.abs() > 1e-9 {
                        for out in &mut ringing[..frames] {
                            mode.x *= self.string_damper;
                            mode.y *= self.string_damper;
                            *out += mode.next(0.);
                        }
                    } else {
                        mode.clear();
                    }
                }
                for ((l, r), tail) in l.iter_mut().zip(r).zip(ringing) {
                    let wet = tail * self.character.resonance * 0.32;
                    *l += wet;
                    *r += wet;
                }
            } else {
                for mode in &mut self.strings {
                    mode.clear();
                }
            }
            if self.ring_frames == 0 {
                self.clear_body();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: i32 = 44100;
    fn audio(p: &mut ModeledPiano, frames: usize) -> Vec<f32> {
        let mut l = vec![0.; frames];
        let mut r = l.clone();
        p.render(&mut l, &mut r);
        assert!(l.iter().chain(&r).all(|s| s.is_finite()));
        l
    }
    fn energy(s: &[f32]) -> f32 {
        s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32
    }
    fn brightness(s: &[f32]) -> f32 {
        let differences: Vec<_> = s.windows(2).map(|w| w[1] - w[0]).collect();
        energy(&differences) / energy(s)
    }
    // A Hann-windowed projection isolates one spectral component of the output.
    fn amplitude(samples: &[f32], rate: i32, frequency: f32) -> f32 {
        let mut x = 0.;
        let mut y = 0.;
        for (frame, sample) in samples.iter().enumerate() {
            let angle = TAU * frame as f32;
            let window = 0.5 - 0.5 * (angle / (samples.len() - 1) as f32).cos();
            let (sin, cos) = (angle * frequency / rate as f32).sin_cos();
            x += sample * window * cos;
            y += sample * window * sin;
        }
        x.hypot(y) / samples.len() as f32
    }
    #[test]
    fn treble_has_a_brief_body_impulse_and_a_pitched_aftersound() {
        for rate in [22050, 48000, 96000] {
            for note in [84, 96, 108] {
                let mut p = ModeledPiano::new(rate);
                p.midi(false, [0x90, note, 85]);
                let signal = audio(&mut p, rate as usize);
                let band = |start_ms, end_ms, frequency| {
                    let frame = |ms: usize| ms * rate as usize / 1000;
                    amplitude(&signal[frame(start_ms)..frame(end_ms)], rate, frequency)
                };
                let pitch = hz(note);
                let fundamental = band(2, 32, pitch);
                let knock = band(2, 32, 245.);
                assert!(knock > 0.08 * fundamental && knock < fundamental);
                assert!(band(155, 185, 245.) < 0.01 * knock);
                // Keep the pitch audible after the impact, with the octave
                // partial well below the fundamental even at the attack.
                assert!(band(2, 32, pitch * 2.) < 0.22 * fundamental);
                let tail = band(500, 540, pitch) / fundamental;
                assert!((0.05..0.45).contains(&tail));
            }
        }
    }
    #[test]
    fn a_harder_treble_strike_increases_octave_content() {
        let octave_ratio = |velocity| {
            let mut p = ModeledPiano::new(RATE);
            p.midi(false, [0x90, 96, velocity]);
            let samples = audio(&mut p, 2048);
            let onset = &samples[128..1536];
            amplitude(onset, RATE, 2. * hz(96)) / amplitude(onset, RATE, hz(96))
        };
        assert!(octave_ratio(112) > 1.4 * octave_ratio(48));
    }
    #[test]
    fn hammer_changes_level_and_harmonics() {
        let strike = |v, felt| {
            let mut p = ModeledPiano::new(RATE);
            p.set_character(felt, 0.5, 0.5);
            p.midi(false, [0x90, 60, v]);
            audio(&mut p, 4096)
        };
        let quiet = strike(35, 0.5);
        let loud = strike(110, 0.5);
        assert!(energy(&loud) > 4. * energy(&quiet));
        assert!(brightness(&loud) > brightness(&quiet));
        assert!(brightness(&strike(80, 1.)) > brightness(&strike(80, 0.)) * 1.1);
    }
    #[test]
    fn sostenuto_captures_held_notes_and_damper_respects_both_sources() {
        let mut p = ModeledPiano::new(RATE);
        p.midi(false, [0x90, 60, 100]);
        p.midi(true, [0x90, 60, 100]);
        p.midi(true, [0, 0, 0]);
        assert_eq!(p.voices.last().unwrap().owners, 1);
        p.midi(false, [0xb0, 66, 127]);
        p.midi(false, [0x80, 60, 0]);
        p.midi(false, [0x90, 65, 100]);
        p.midi(false, [0x90, 65, 0]);
        audio(&mut p, RATE as usize);
        assert!(p.voices.iter().all(|v| v.note == 60));
        assert!(energy(&audio(&mut p, 512)) > 1e-6);
        p.midi(false, [0xb0, 66, 0]);
        audio(&mut p, RATE as usize);
        assert!(energy(&audio(&mut p, 512)) < 1e-10);
        p.midi(false, [0x91, 70, 110]);
        p.midi(false, [0xb1, 64, 127]);
        p.midi(false, [0x81, 70, 0]);
        audio(&mut p, RATE as usize);
        assert!(energy(&audio(&mut p, 1024)) > 1e-6);
        p.midi(false, [0xb1, 64, 0]);
        audio(&mut p, RATE as usize);
        assert!(energy(&audio(&mut p, 1024)) < 1e-10);
    }
    #[test]
    fn decay_changes_the_tail_and_open_strings_respond() {
        let tone = |decay, resonance| {
            let mut p = ModeledPiano::new(RATE);
            p.set_character(0.5, decay, resonance);
            p.midi(false, [0xb0, 64, 127]);
            p.midi(false, [0x90, 57, 100]);
            audio(&mut p, RATE as usize);
            audio(&mut p, 2048)
        };
        let short = tone(0., 0.);
        let long = tone(1., 0.);
        assert!(energy(&long) > 3. * energy(&short));
        let wet = tone(1., 1.);
        let difference: Vec<_> = wet.iter().zip(&long).map(|(a, b)| a - b).collect();
        assert!(energy(&difference) > energy(&long) * 0.015);
    }
    #[test]
    fn bounded_polyphony_and_resets_clear_the_body_and_strings() {
        for rate in [22050, 48000, 96000] {
            let mut p = ModeledPiano::new(rate);
            p.midi(false, [0xb0, 64, 127]);
            for note in 0..128 {
                p.midi(false, [0x90, note, 127]);
            }
            assert_eq!(p.voices.len(), POLYPHONY);
            assert!(energy(&audio(&mut p, 2048)) > 0.);
            p.set_octave(1);
            p.midi(false, [0x90, 127, 127]);
            assert_eq!(energy(&audio(&mut p, 128)), 0.);
            p.midi(false, [0x90, 60, 127]);
            assert_eq!(p.voices[0].note, 72);
            p.panic();
            assert_eq!(energy(&audio(&mut p, 128)), 0.);
        }
    }
}
