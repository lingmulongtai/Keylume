use crate::model::DeviceLayout;
use std::f32::consts::TAU;
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
            level: velocity as f32 / 127.,
            active: true,
            ..Default::default()
        };
        self.cursor = (self.cursor + 1) % 32;
    }
    pub fn sample(&mut self) -> f32 {
        let mut sum = 0.;
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
            let env = (-t * decay).exp();
            if env < 0.001 {
                v.active = false;
                continue;
            }
            v.phase = (v.phase + freq / self.rate) % 1.;
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
            sum += wave * env * v.level * 0.34;
            v.age += 1. / self.rate;
        }
        sum
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
