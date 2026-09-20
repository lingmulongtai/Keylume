use crate::model::DeviceLayout;
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

pub const KIT_COUNT: usize = 6;
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PadSound {
    pub kind: u8,
    pub tune: f32,
    pub decay: f32,
    pub level: f32,
    pub pan: f32,
}
impl Default for PadSound {
    fn default() -> Self {
        Self {
            kind: 8,
            tune: 0.,
            decay: 1.,
            level: 1.,
            pan: 0.,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DrumSettings {
    pub kit: u8,
    pub banks: [[PadSound; 16]; KIT_COUNT],
}
impl Default for DrumSettings {
    fn default() -> Self {
        Self {
            kit: 0,
            banks: std::array::from_fn(|kit| {
                std::array::from_fn(|pad| {
                    let (tune, decay) = [
                        (0., 1.),
                        (-3., 1.7),
                        (1., 0.8),
                        (-5., 0.65),
                        (12., 2.2),
                        (5., 0.7),
                    ][kit];
                    PadSound {
                        kind: pad as u8,
                        tune,
                        decay,
                        ..Default::default()
                    }
                })
            }),
        }
    }
}
impl DrumSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.kit as usize >= KIT_COUNT
            || self.banks.iter().flatten().any(|p| {
                p.kind > 15
                    || !(-24.0..=24.).contains(&p.tune)
                    || !(0.15..=4.).contains(&p.decay)
                    || !(0.0..=1.).contains(&p.level)
                    || !(-1.0..=1.).contains(&p.pan)
            })
        {
            Err("ドラムの設定値が範囲外です".into())
        } else {
            Ok(())
        }
    }
}
/// Layout order is top row left-to-right, then bottom row. Only DAW note channels are pads.
pub fn pad_hit(source: &str, b: &[u8], layout: &DeviceLayout) -> Option<(u8, u8)> {
    if source != "daw"
        || b.len() != 3
        || b[0] & 0xf0 != 0x90
        || ![0, 9].contains(&(b[0] & 15))
        || b[2] == 0
        || b[2] > 127
    {
        return None;
    }
    layout
        .leds
        .iter()
        .filter(|l| l.group == "pads")
        .position(|l| {
            if b[0] & 15 == 9 {
                l.address.drum_note == Some(b[1])
            } else {
                l.address.daw_note == Some(b[1])
            }
        })
        .filter(|i| *i < 16)
        .map(|i| (i as u8, b[2]))
}
#[derive(Clone, Copy, Default)]
struct Voice {
    kind: u8,
    age: f32,
    phase: f32,
    level: f32,
    active: bool,
    tuning: f32,
    decay: f32,
    pan: f32,
    kit: u8,
}
pub struct DrumSynth {
    voices: [Voice; 32],
    cursor: usize,
    noise: u32,
    rate: f32,
}
impl DrumSynth {
    pub fn new(rate: u32) -> Self {
        Self {
            voices: [Voice::default(); 32],
            cursor: 0,
            noise: 0xabc123,
            rate: rate as f32,
        }
    }
    pub fn panic(&mut self) {
        self.voices.fill(Voice::default());
    }
    pub fn hit(&mut self, kind: u8, velocity: u8) {
        self.hit_sound(
            PadSound {
                kind,
                ..Default::default()
            },
            0,
            velocity,
        );
    }
    pub fn hit_sound(&mut self, sound: PadSound, kit: u8, velocity: u8) {
        let kind = sound.kind;
        if kind > 16 || velocity == 0 {
            return;
        }
        if kind == 10 {
            for v in &mut self.voices {
                if v.kind == 11 {
                    v.active = false;
                }
            }
        }
        self.voices[self.cursor] = Voice {
            kind,
            level: velocity as f32 / 127. * sound.level,
            tuning: 2f32.powf(sound.tune / 12.),
            decay: sound.decay,
            pan: sound.pan,
            kit,
            active: true,
            ..Default::default()
        };
        self.cursor = (self.cursor + 1) % 32;
    }
    pub fn stereo(&mut self) -> (f32, f32) {
        let (mut left, mut right) = (0., 0.);
        for v in &mut self.voices {
            if !v.active {
                continue;
            }
            let t = v.age;
            self.noise ^= self.noise << 13;
            self.noise ^= self.noise >> 17;
            self.noise ^= self.noise << 5;
            let noise = self.noise as f32 / u32::MAX as f32 * 2. - 1.;
            let (freq, decay, tone, nz) = match v.kind {
                0 => (155., 9., 0.85, 0.15),
                1 => (220., 12., 0.85, 0.15),
                2 => (330., 14., 0.85, 0.15),
                3 => (175., 24., 0.3, 0.7),
                4 => (790., 8., 0.5, 0.5),
                5 => (1200., 10., 0.6, 0.4),
                6 => (370., 28., 0.8, 0.2),
                7 => (520., 32., 0.8, 0.2),
                8 => (48. + 110. * (-t * 50.).exp(), 8., 1., 0.),
                9 => (170., 17., 0.3, 0.7),
                10 => (6700., 65., 0.15, 0.85),
                11 => (6300., 9., 0.2, 0.8),
                12 => (185., 20., 0.15, 0.85),
                13 => (4300., 28., 0.1, 0.9),
                14 => (5100., 3.3, 0.25, 0.75),
                15 => (610., 15., 0.75, 0.25),
                _ => (1500., 100., 1., 0.),
            };
            let env = (-t * decay / v.decay).exp();
            if env < 0.001 {
                v.active = false;
                continue;
            }
            v.phase = (v.phase + (freq * v.tuning).min(self.rate * 0.45) / self.rate) % 1.;
            let mut wave = (v.phase * TAU).sin() * tone + noise * nz;
            if v.kind == 12 {
                wave *= if t < 0.035 {
                    (t * 350.).sin().abs()
                } else {
                    0.6
                };
            }
            if v.kind == 15 {
                wave = ((v.phase * TAU).sin() + ((v.phase * 1.48) % 1. * TAU).sin()) * 0.5;
            }
            wave = match v.kit {
                1 => (v.phase * TAU).sin() * tone + noise * nz * 0.7,
                2 => (wave * 2.2).tanh() * 0.75,
                3 => (wave * 12.).round() / 12. * 0.85,
                4 => {
                    ((v.phase * TAU).sin()
                        + (v.phase * TAU * 2.76).sin() * 0.65
                        + (v.phase * TAU * 4.1).sin() * 0.25)
                        * 0.5
                        + noise * nz * 0.15
                }
                5 => wave * (1. + (t * 95.).sin() * 0.25),
                _ => wave,
            };
            let sample = wave * env * v.level * 0.34;
            left += sample * (1. - v.pan.max(0.));
            right += sample * (1. + v.pan.min(0.));
            v.age += 1. / self.rate;
        }
        (left, right)
    }
    #[cfg(test)]
    fn sample(&mut self) -> f32 {
        let (l, r) = self.stereo();
        (l + r) * 0.5
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kit_timbres_are_distinct_and_pad_pan_is_independent() {
        let settings = DrumSettings::default();
        assert!(settings.validate().is_ok());
        let mut signatures = Vec::new();
        for kit in 0..KIT_COUNT {
            let mut synth = DrumSynth::new(48000);
            let mut sound = settings.banks[kit][8];
            sound.pan = -1.;
            synth.hit_sound(sound, kit as u8, 100);
            let mut signature = 0.;
            for _ in 0..4800 {
                let (l, r) = synth.stereo();
                assert!(l.is_finite());
                assert_eq!(r, 0.);
                signature += l * l;
            }
            assert!(signature > 1.);
            assert!(!signatures
                .iter()
                .any(|v: &f32| (v - signature).abs() < 0.01));
            signatures.push(signature);
        }
        let mut invalid = settings;
        invalid.banks[1][0].decay = f32::NAN;
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn kit_is_finite_audible_and_decays() {
        for pad in 0..16 {
            let mut d = DrumSynth::new(48000);
            d.hit(pad, 100);
            let mut energy = 0.;
            for _ in 0..4800 {
                let v = d.sample();
                assert!(v.is_finite());
                energy += v * v;
            }
            assert!(energy > 1., "pad {pad}");
            for _ in 0..192000 {
                d.sample();
            }
            assert!(d.sample().abs() < 0.001);
            d.panic();
            assert_eq!(d.sample(), 0.);
        }
    }
    #[test]
    fn only_matching_pad_note_ons_trigger() {
        let l = DeviceLayout::default();
        assert_eq!(pad_hit("daw", &[0x90, 112, 100], &l), Some((8, 100)));
        assert_eq!(pad_hit("daw", &[0x99, 36, 80], &l), Some((8, 80)));
        for (source, b) in [
            ("keyboard", [0x90, 112, 100]),
            ("daw", [0x91, 112, 100]),
            ("daw", [0x90, 112, 0]),
            ("daw", [0xa0, 112, 100]),
        ] {
            assert!(pad_hit(source, &b, &l).is_none());
        }
    }
}
