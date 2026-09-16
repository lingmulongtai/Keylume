use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{bounded, Receiver, Sender};
use rustfft::{num_complex::Complex, FftPlanner};
pub struct AudioCapture {
    _stream: cpal::Stream,
    pub levels: Receiver<[f32; 8]>,
    pub errors: Receiver<String>,
}
pub fn devices() -> Vec<String> {
    cpal::default_host()
        .output_devices()
        .map(|d| d.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}
impl AudioCapture {
    pub fn start(name: &str) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = if name.is_empty() {
            host.default_output_device()
        } else {
            host.output_devices()
                .ok()
                .and_then(|mut ds| ds.find(|d| d.name().is_ok_and(|n| n == name)))
        }
        .ok_or("音声出力デバイスがありません")?;
        let format = device.default_output_config().map_err(|e| e.to_string())?;
        let config: cpal::StreamConfig = format.clone().into();
        let (tx, rx) = bounded(2);
        let (error_tx, errors) = bounded(1);
        let err = move |e: cpal::StreamError| {
            let _ = error_tx.try_send(e.to_string());
        };
        // CPAL WASAPI uses AUDCLNT_STREAMFLAGS_LOOPBACK for input on a render endpoint.
        let stream = match format.sample_format() {
            cpal::SampleFormat::F32 => {
                let mut a = Analyzer::new(config.channels as usize, config.sample_rate.0, tx);
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| a.push(data.iter().copied()),
                    err,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let mut a = Analyzer::new(config.channels as usize, config.sample_rate.0, tx);
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| a.push(data.iter().map(|v| *v as f32 / 32768.)),
                    err,
                    None,
                )
            }
            _ => return Err("未対応の音声サンプル形式です".into()),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            _stream: stream,
            levels: rx,
            errors,
        })
    }
}
struct Analyzer {
    channels: usize,
    rate: f32,
    samples: Vec<f32>,
    channel: usize,
    sum: f32,
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    buffer: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    smooth: [f32; 8],
    gain: f32,
    tx: Sender<[f32; 8]>,
}
impl Analyzer {
    fn new(channels: usize, rate: u32, tx: Sender<[f32; 8]>) -> Self {
        let fft = FftPlanner::new().plan_fft_forward(2048);
        let scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
        Self {
            channels,
            rate: rate as f32,
            samples: Vec::with_capacity(2048),
            channel: 0,
            sum: 0.,
            fft,
            buffer: vec![Complex::default(); 2048],
            scratch,
            smooth: [0.; 8],
            gain: 1.,
            tx,
        }
    }
    fn push(&mut self, data: impl Iterator<Item = f32>) {
        for v in data {
            self.sum += v;
            self.channel += 1;
            if self.channel < self.channels {
                continue;
            }
            self.samples.push(self.sum / self.channels as f32);
            self.sum = 0.;
            self.channel = 0;
            if self.samples.len() != 2048 {
                continue;
            }
            for (i, v) in self.samples.iter().enumerate() {
                self.buffer[i] = Complex::new(
                    v * (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / 2047.).cos()),
                    0.,
                );
            }
            self.fft
                .process_with_scratch(&mut self.buffer, &mut self.scratch);
            let mut bands = [0f32; 8];
            for (i, band) in bands.iter_mut().enumerate() {
                let lo = 40. * (16000f32 / 40.).powf(i as f32 / 8.);
                let hi = 40. * (16000f32 / 40.).powf((i + 1) as f32 / 8.);
                let end = self.buffer.len() / 2;
                let a = ((lo * 2048. / self.rate) as usize).min(end);
                let b = ((hi * 2048. / self.rate) as usize).max(a + 1).min(end);
                *band = self.buffer[a..b]
                    .iter()
                    .map(|c| c.norm_sqr())
                    .sum::<f32>()
                    .sqrt()
                    / 100.;
            }
            let peak = bands.iter().copied().fold(0f32, f32::max);
            self.gain = self.gain * 0.98 + (0.65 / peak.max(0.02)).clamp(0.5, 8.) * 0.02;
            for (i, b) in bands.iter().enumerate() {
                self.smooth[i] = self.smooth[i] * 0.65 + (b * self.gain).clamp(0., 1.) * 0.35;
            }
            let _ = self.tx.try_send(self.smooth);
            self.samples.clear();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn low_sample_rates_keep_bands_above_nyquist_silent() {
        for rate in [4000, 8000, 16000, 44100, 48000] {
            let (tx, rx) = bounded(2);
            let mut analyzer = Analyzer::new(1, rate, tx);
            analyzer.push((0..2048).map(|i| (i as f32 * 0.17).sin()));
            let bands = rx.try_recv().unwrap();
            assert!(bands
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
            for (i, band) in bands.iter().enumerate() {
                let lo = 40. * (16000f32 / 40.).powf(i as f32 / 8.);
                if lo >= rate as f32 / 2. {
                    assert_eq!(*band, 0.);
                }
            }
        }
    }
}
