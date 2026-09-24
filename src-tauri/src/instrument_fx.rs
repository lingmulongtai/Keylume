use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

/// Normalized controls, ordered to match the eight factory encoder bindings.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct InstrumentFx {
    pub reverb: f32,
    pub delay: f32,
    pub cutoff: f32,
    pub resonance: f32,
    pub chorus: f32,
    pub drive: f32,
    pub width: f32,
    pub tremolo: f32,
}
impl Default for InstrumentFx {
    fn default() -> Self {
        Self {
            reverb: 0.,
            delay: 0.,
            cutoff: 1.,
            resonance: 0.,
            chorus: 0.,
            drive: 0.,
            width: 0.5,
            tremolo: 0.,
        }
    }
}
impl InstrumentFx {
    pub fn values(self) -> [f32; 8] {
        [
            self.reverb,
            self.delay,
            self.cutoff,
            self.resonance,
            self.chorus,
            self.drive,
            self.width,
            self.tremolo,
        ]
    }
    pub fn from_values(v: [f32; 8]) -> Self {
        Self {
            reverb: v[0],
            delay: v[1],
            cutoff: v[2],
            resonance: v[3],
            chorus: v[4],
            drive: v[5],
            width: v[6],
            tremolo: v[7],
        }
    }
    pub fn validate(self) -> Result<(), String> {
        if self.values().iter().all(|v| (0.0..=1.).contains(v)) {
            Ok(())
        } else {
            Err("音作りの値は0〜100%です".into())
        }
    }
}
struct Delay {
    data: Vec<f32>,
    index: usize,
}
impl Delay {
    fn new(size: usize) -> Self {
        Self {
            data: vec![0.; size.max(2)],
            index: 0,
        }
    }
    fn read(&self, delay: f32) -> f32 {
        let p = (self.index as f32 - delay).rem_euclid(self.data.len() as f32);
        // A tiny negative remainder can round to len in f32. Wrap the integer
        // index too, so interpolation across the buffer seam stays in bounds.
        let i = p.floor() as usize % self.data.len();
        let f = p - p.floor();
        self.data[i] * (1. - f) + self.data[(i + 1) % self.data.len()] * f
    }
    fn tick(&mut self, input: f32, feedback: f32) -> f32 {
        let output = self.data[self.index];
        self.write(input + output * feedback);
        output
    }
    fn write(&mut self, input: f32) {
        self.data[self.index] = input;
        self.index = (self.index + 1) % self.data.len();
    }
    fn reset(&mut self) {
        self.data.fill(0.);
        self.index = 0;
    }
}
pub struct Effects {
    rate: f32,
    current: [f32; 8],
    filters: [[f32; 2]; 2],
    echoes: [Delay; 2],
    reverbs: [Delay; 8],
    chorus: [Delay; 2],
    phase: f32,
}
impl Effects {
    pub fn new(rate: u32, settings: InstrumentFx) -> Self {
        let rate = rate as f32;
        Self {
            rate,
            current: settings.values(),
            filters: [[0.; 2]; 2],
            echoes: [
                Delay::new((rate * 0.31) as usize),
                Delay::new((rate * 0.43) as usize),
            ],
            reverbs: std::array::from_fn(|i| {
                Delay::new(
                    (rate
                        * [
                            0.0297, 0.0371, 0.0411, 0.0437, 0.0309, 0.0383, 0.0427, 0.0451,
                        ][i]) as usize,
                )
            }),
            chorus: std::array::from_fn(|_| Delay::new((rate * 0.04) as usize)),
            phase: 0.,
        }
    }
    pub fn reset(&mut self) {
        self.filters = [[0.; 2]; 2];
        for d in self
            .echoes
            .iter_mut()
            .chain(self.reverbs.iter_mut())
            .chain(self.chorus.iter_mut())
        {
            d.reset();
        }
    }
    /// No allocation or lock on the audio thread; parameters slew to avoid zipper noise.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], settings: InstrumentFx) {
        let target = settings.values();
        let smooth = 1. - (-1. / (self.rate * 0.012)).exp();
        for (left, right) in left.iter_mut().zip(right) {
            for (v, target) in self.current.iter_mut().zip(target) {
                *v += (target - *v) * smooth;
            }
            let [reverb, delay, cutoff, resonance, chorus, drive, width, tremolo] = self.current;
            let mut pair = [*left, *right];
            let frequency = (60. * 300f32.powf(cutoff)).min(self.rate * 0.42);
            let g = (std::f32::consts::PI * frequency / self.rate).tan();
            let k = 2. - resonance * 1.85;
            let a = 1. / (1. + g * (g + k));
            for (channel, sample) in pair.iter_mut().enumerate() {
                let [s1, s2] = self.filters[channel];
                let v1 = a * (s1 + g * (*sample - s2));
                let v2 = s2 + g * v1;
                self.filters[channel] = [2. * v1 - s1, 2. * v2 - s2];
                if cutoff < 0.999 || resonance > 0.001 {
                    *sample = v2;
                }
                if drive > 0.001 {
                    let gain = 1. + drive * 15.;
                    *sample = (*sample * gain).tanh() / gain.sqrt();
                }
                let modulated = self.chorus[channel].read(
                    self.rate
                        * (0.018 + 0.005 * (self.phase * TAU * 0.31 + channel as f32 * 1.5).sin()),
                );
                self.chorus[channel].write(*sample);
                *sample += modulated * chorus * 0.45;
                let echo = self.echoes[channel].tick(*sample * delay, 0.4);
                let mut room = 0.;
                for i in 0..4 {
                    room += self.reverbs[channel * 4 + i]
                        .tick(*sample * reverb * 0.14, 0.74 - i as f32 * 0.013);
                }
                *sample += echo * 0.65 + room;
            }
            let mid = (pair[0] + pair[1]) * 0.5;
            let side = (pair[0] - pair[1]) * width;
            let trem = 1. - tremolo * (0.5 + 0.5 * (self.phase * TAU * 4.2).sin());
            *left = (mid + side) * trem;
            *right = (mid - side) * trem;
            self.phase = (self.phase + 1. / self.rate) % 100.;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_delay_wrap_cannot_index_past_the_buffer() {
        let mut line = Delay::new(1920);
        line.index = 818;
        line.data[0] = 0.5;
        line.data[1919] = -0.5;
        // The next representable delay crosses zero by less than half an ULP
        // at the far end of this ring, so rem_euclid rounds to exactly 1920.
        let delay = f32::from_bits(818f32.to_bits() + 1);
        assert_eq!((818. - delay).rem_euclid(1920.), 1920.);
        assert!((line.read(delay) - 0.5).abs() < 0.0001);
        assert_eq!(line.read(818.25), 0.25);
        assert_eq!(line.read(819.), -0.5);
        assert_eq!(line.read(818.), 0.5);
    }
    #[test]
    fn rapid_knob_changes_stay_finite_over_long_playback() {
        let mut seed = 42u32;
        for rate in [22050, 44100, 48000, 96000] {
            let mut fx = Effects::new(rate, InstrumentFx::default());
            let mut settings = InstrumentFx::default();
            let mut left = [0.; 128];
            let mut right = [0.; 128];
            for block in 0..(rate * 120 / 128) {
                if block % 8 == 0 {
                    settings = InstrumentFx::from_values(std::array::from_fn(|_| {
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        (seed % 128) as f32 / 127.
                    }));
                }
                for i in 0..128 {
                    let phase = (block as f32 * 128. + i as f32) * TAU * 220. / rate as f32;
                    left[i] = phase.sin() * 0.3;
                    right[i] = (phase * 1.013).sin() * 0.3;
                }
                fx.process(&mut left, &mut right, settings);
                assert!(
                    left.iter()
                        .chain(&right)
                        .all(|x| x.is_finite() && x.abs() < 20.),
                    "rate={rate}, block={block}, settings={settings:?}"
                );
            }
        }
    }
    #[test]
    fn dry_signal_stays_unchanged_and_wet_impulses_leave_a_tail() {
        let dry = InstrumentFx::default();
        let mut fx = Effects::new(48000, dry);
        let mut l = vec![0.25; 480];
        let mut r = vec![-0.1; 480];
        fx.process(&mut l, &mut r, dry);
        assert!(l.iter().all(|n| (*n - 0.25).abs() < 0.000001));
        let wet = InstrumentFx {
            reverb: 1.,
            delay: 1.,
            ..dry
        };
        let mut fx = Effects::new(48000, wet);
        let mut l = vec![0.; 48000];
        let mut r = l.clone();
        l[0] = 1.;
        r[0] = 1.;
        fx.process(&mut l, &mut r, wet);
        assert!(l[1000..].iter().map(|x| x * x).sum::<f32>() > 0.1);
        fx.reset();
        l.fill(0.);
        r.fill(0.);
        fx.process(&mut l, &mut r, wet);
        assert!(l.iter().all(|x| *x == 0.));
    }
    #[test]
    fn controls_are_bounded_and_maximum_settings_stay_finite() {
        assert!(InstrumentFx::from_values([f32::NAN; 8]).validate().is_err());
        for rate in [22050, 44100, 48000, 96000] {
            let settings = InstrumentFx::from_values([1.; 8]);
            let mut fx = Effects::new(rate, settings);
            let mut l = vec![0.4; rate as usize];
            let mut r = l.clone();
            fx.process(&mut l, &mut r, settings);
            assert!(l.iter().chain(&r).all(|x| x.is_finite() && x.abs() < 10.));
        }
    }
}
