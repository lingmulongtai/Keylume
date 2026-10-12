const TAPS: usize = 129;
const INPUT: usize = 33;
const LATENCY: usize = 32;

/// Four-times oversampling with interpolation and anti-alias FIR filters.
/// The aligned bypass adds 32 device frames, independent of the Drive control.
pub(super) struct Drive {
    taps: [f32; TAPS],
    input: [f32; INPUT],
    high: [f32; TAPS],
    dry: [f32; LATENCY],
    input_at: usize,
    high_at: usize,
    dry_at: usize,
    active: bool,
}
impl Drive {
    pub(super) fn new() -> Self {
        let mut taps = std::array::from_fn(|i| {
            let t = i as f64 - 64.;
            let phase = std::f64::consts::TAU * i as f64 / 128.;
            let sinc = if t == 0. {
                0.225
            } else {
                (std::f64::consts::TAU * 0.1125 * t).sin() / (std::f64::consts::PI * t)
            };
            (sinc * (0.42 - 0.5 * phase.cos() + 0.08 * (2. * phase).cos())) as f32
        });
        let sum: f32 = taps.iter().sum();
        for tap in &mut taps {
            *tap /= sum;
        }
        Self {
            taps,
            input: [0.; INPUT],
            high: [0.; TAPS],
            dry: [0.; LATENCY],
            input_at: 0,
            high_at: 0,
            dry_at: 0,
            active: false,
        }
    }
    pub(super) fn reset(&mut self) {
        self.input.fill(0.);
        self.high.fill(0.);
        self.dry.fill(0.);
        self.input_at = 0;
        self.high_at = 0;
        self.dry_at = 0;
        self.active = false;
    }
    pub(super) fn process(&mut self, sample: f32, drive: f32) -> f32 {
        let dry = self.dry[self.dry_at];
        self.dry[self.dry_at] = sample;
        self.dry_at = (self.dry_at + 1) % LATENCY;
        self.input[self.input_at] = sample;
        let mut wet = 0.;
        if drive > 0.001 {
            let gain = 1. + drive * 15.;
            for phase in 0..4 {
                let mut up = 0.;
                for (k, tap) in self.taps.iter().skip(phase).step_by(4).enumerate() {
                    up += tap * self.input[(self.input_at + INPUT - k) % INPUT] * 4.;
                }
                self.high[self.high_at] = (up * gain).tanh() / gain.sqrt();
                if phase == 0 {
                    for (k, tap) in self.taps.iter().enumerate() {
                        wet += tap * self.high[(self.high_at + TAPS - k) % TAPS];
                    }
                }
                self.high_at = (self.high_at + 1) % TAPS;
            }
            self.active = true;
        } else if self.active {
            self.high.fill(0.);
            self.high_at = 0;
            self.active = false;
        }
        self.input_at = (self.input_at + 1) % INPUT;
        let blend = ((drive - 0.001) / 0.019).clamp(0., 1.);
        dry + (wet - dry) * blend
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bypass_preserves_stereo_amplitude_with_a_fixed_small_delay_and_reset_clears_history() {
        let mut left = Drive::new();
        let mut right = Drive::new();
        for i in 0..100 {
            let l = left.process(if i == 0 { 0.7 } else { 0. }, 0.);
            let r = right.process(if i == 0 { -0.2 } else { 0. }, 0.);
            assert_eq!(l, if i == LATENCY { 0.7 } else { 0. });
            assert_eq!(r, if i == LATENCY { -0.2 } else { 0. });
        }
        for _ in 0..200 {
            left.process(0.4, 1.);
        }
        left.reset();
        for _ in 0..200 {
            assert_eq!(left.process(0., 1.), 0.);
        }
    }
    fn amplitude(samples: &[f32], frequency: f64) -> f64 {
        let mut real = 0.;
        let mut imag = 0.;
        for (i, sample) in samples.iter().enumerate() {
            let phase = std::f64::consts::TAU * frequency * i as f64 / 48000.;
            real += *sample as f64 * phase.cos();
            imag += *sample as f64 * phase.sin();
        }
        real.hypot(imag) * 2. / samples.len() as f64
    }
    #[test]
    fn high_note_foldover_is_reduced_by_at_least_twenty_decibels() {
        let drive = 0.638;
        let gain: f32 = 1. + drive * 15.;
        let mut stage = Drive::new();
        let mut direct = Vec::new();
        let mut filtered = Vec::new();
        for i in 0..50000 {
            let input = (std::f64::consts::TAU * 10000. * i as f64 / 48000.).sin() as f32 * 0.3;
            let out = stage.process(input, drive);
            if i >= 2000 {
                filtered.push(out);
                direct.push((input * gain).tanh() / gain.sqrt());
            }
        }
        let old = amplitude(&direct, 18000.) / amplitude(&direct, 10000.);
        let new = amplitude(&filtered, 18000.) / amplitude(&filtered, 10000.);
        eprintln!("18kHz / 10kHz alias ratio: direct={old:.6}, filtered={new:.6}");
        assert!(new < old * 0.1, "alias ratio direct={old}, filtered={new}");
        assert!(amplitude(&filtered, 10000.) > 0.1);
    }
}
