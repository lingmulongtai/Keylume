use crate::piano::{self, PianoSettings, PianoSynth};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{bounded, Receiver, Sender};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, AtomicI8, AtomicU32, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PianoStatus {
    pub state: String,
    pub device: String,
    pub sample_rate: u32,
    pub buffer_frames: u32,
    pub error: String,
    pub peak: f32,
    pub muted: bool,
}
#[derive(Clone, Copy)]
struct Event {
    generation: u64,
    screen: bool,
    bytes: [u8; 3],
}
struct Shared {
    generation: AtomicU64,
    enabled: AtomicBool,
    blocked: AtomicBool,
    volume: AtomicU32,
    octave: AtomicI8,
    peak: AtomicU32,
    frames: AtomicU32,
}
#[derive(Clone)]
pub struct PianoBus {
    tx: Sender<Event>,
    shared: Arc<Shared>,
}
impl PianoBus {
    pub fn midi(&self, screen: bool, bytes: &[u8]) {
        if bytes.len() != 3
            || !self.shared.enabled.load(Ordering::Acquire)
            || self.shared.blocked.load(Ordering::Acquire)
        {
            return;
        }
        if self
            .tx
            .try_send(Event {
                generation: self.shared.generation.load(Ordering::Acquire),
                screen,
                bytes: [bytes[0], bytes[1], bytes[2]],
            })
            .is_err()
        {
            self.panic();
        }
    }
    pub fn panic(&self) {
        self.shared.generation.fetch_add(1, Ordering::AcqRel);
    }
    pub fn block(&self, blocked: bool) {
        if self.shared.blocked.swap(blocked, Ordering::AcqRel) != blocked {
            self.panic();
        }
    }
}
pub struct Piano {
    pub bus: PianoBus,
    config: Arc<Mutex<PianoSettings>>,
    status: Arc<Mutex<PianoStatus>>,
    stop: Arc<AtomicBool>,
    wake: Sender<()>,
}
impl Piano {
    pub fn new() -> Self {
        let (tx, rx) = bounded(1024);
        let (wake, wakeup) = bounded(1);
        let shared = Arc::new(Shared {
            generation: AtomicU64::new(0),
            enabled: AtomicBool::new(false),
            blocked: AtomicBool::new(false),
            volume: AtomicU32::new(0.5f32.to_bits()),
            octave: AtomicI8::new(0),
            peak: AtomicU32::new(0),
            frames: AtomicU32::new(0),
        });
        let bus = PianoBus { tx, shared };
        let config = Arc::new(Mutex::new(PianoSettings::default()));
        let status = Arc::new(Mutex::new(PianoStatus {
            state: "off".into(),
            ..Default::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (thread_bus, thread_config, thread_status, thread_stop) =
            (bus.clone(), config.clone(), status.clone(), stop.clone());
        std::thread::spawn(move || {
            let mut stream = None;
            let mut font = None;
            let mut current = PianoSettings::default();
            let mut retry = Instant::now();
            let (error_tx, error_rx) = bounded::<String>(1);
            while !thread_stop.load(Ordering::Acquire) {
                let next = thread_config.lock().unwrap().clone();
                let restart = next.enabled != current.enabled
                    || next.output_device != current.output_device
                    || next.buffer_frames != current.buffer_frames;
                if restart {
                    thread_bus.shared.enabled.store(false, Ordering::Release);
                    thread_bus.panic();
                    stream = None;
                    retry = Instant::now();
                    for _ in error_rx.try_iter() {}
                }
                current = next;
                if let Ok(error) = error_rx.try_recv() {
                    thread_bus.shared.enabled.store(false, Ordering::Release);
                    thread_bus.panic();
                    stream = None;
                    *thread_status.lock().unwrap() = PianoStatus {
                        state: "error".into(),
                        error,
                        ..Default::default()
                    };
                    retry = Instant::now() + Duration::from_secs(3);
                }
                if !current.enabled {
                    *thread_status.lock().unwrap() = PianoStatus {
                        state: "off".into(),
                        ..Default::default()
                    };
                } else if stream.is_none() && Instant::now() >= retry {
                    thread_status.lock().unwrap().state = "loading".into();
                    let result = (|| {
                        if font.is_none() {
                            font = Some(piano::sound_font()?);
                        }
                        start(
                            font.as_ref().unwrap(),
                            &current,
                            rx.clone(),
                            thread_bus.shared.clone(),
                            error_tx.clone(),
                        )
                    })();
                    match result {
                        Ok((output, status)) => {
                            for _ in rx.try_iter() {}
                            thread_bus.panic();
                            stream = Some(output);
                            *thread_status.lock().unwrap() = status;
                            thread_bus.shared.enabled.store(true, Ordering::Release);
                        }
                        Err(error) => {
                            *thread_status.lock().unwrap() = PianoStatus {
                                state: "error".into(),
                                error,
                                ..Default::default()
                            };
                            retry = Instant::now() + Duration::from_secs(5);
                        }
                    }
                }
                let _ = wakeup.recv_timeout(Duration::from_millis(100));
            }
            thread_bus.shared.enabled.store(false, Ordering::Release);
            thread_bus.panic();
            drop(stream);
        });
        Self {
            bus,
            config,
            status,
            stop,
            wake,
        }
    }
    pub fn configure(&self, settings: &PianoSettings) {
        let mut config = self.config.lock().unwrap();
        if *config == *settings {
            return;
        }
        self.bus
            .shared
            .volume
            .store(settings.volume.to_bits(), Ordering::Release);
        self.bus
            .shared
            .octave
            .store(settings.octave, Ordering::Release);
        if settings.octave != config.octave
            || settings.enabled != config.enabled
            || settings.output_device != config.output_device
            || settings.buffer_frames != config.buffer_frames
        {
            self.bus.panic();
            if !settings.enabled {
                self.bus.shared.enabled.store(false, Ordering::Release);
            }
        }
        *config = settings.clone();
        let _ = self.wake.try_send(());
    }
    pub fn view(&self) -> PianoStatus {
        let mut status = self.status.lock().unwrap().clone();
        status.peak = f32::from_bits(self.bus.shared.peak.load(Ordering::Relaxed));
        status.buffer_frames = self.bus.shared.frames.load(Ordering::Relaxed);
        status.muted = self.bus.shared.blocked.load(Ordering::Acquire);
        status
    }
    pub fn stop(&self) {
        self.bus.block(true);
        self.stop.store(true, Ordering::Release);
        let _ = self.wake.try_send(());
    }
}

impl Drop for Piano {
    fn drop(&mut self) {
        self.stop();
    }
}

fn start(
    font: &Arc<rustysynth::SoundFont>,
    settings: &PianoSettings,
    rx: Receiver<Event>,
    shared: Arc<Shared>,
    errors: Sender<String>,
) -> Result<(cpal::Stream, PianoStatus), String> {
    let host = cpal::default_host();
    let device = if settings.output_device.is_empty() {
        host.default_output_device()
    } else {
        host.output_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.name().is_ok_and(|n| n == settings.output_device))
    }
    .ok_or("ピアノの出力先が見つかりません。出力デバイスを選び直してください")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let mut config: cpal::StreamConfig = supported.clone().into();
    if config.channels == 0 {
        return Err("音声出力チャンネルがありません".into());
    }
    if let cpal::SupportedBufferSize::Range { min, max } = supported.buffer_size() {
        config.buffer_size = cpal::BufferSize::Fixed(settings.buffer_frames.clamp(*min, *max));
    }
    let build = |config: &cpal::StreamConfig| match supported.sample_format() {
        cpal::SampleFormat::F32 => build::<f32>(
            &device,
            config,
            font,
            rx.clone(),
            shared.clone(),
            errors.clone(),
        ),
        cpal::SampleFormat::I16 => build::<i16>(
            &device,
            config,
            font,
            rx.clone(),
            shared.clone(),
            errors.clone(),
        ),
        cpal::SampleFormat::U16 => build::<u16>(
            &device,
            config,
            font,
            rx.clone(),
            shared.clone(),
            errors.clone(),
        ),
        format => Err(format!("未対応の音声形式です: {format:?}")),
    };
    let stream = match build(&config) {
        Ok(stream) => stream,
        Err(_) if config.buffer_size != cpal::BufferSize::Default => {
            config.buffer_size = cpal::BufferSize::Default;
            build(&config)?
        }
        Err(error) => return Err(error),
    };
    stream.play().map_err(|e| e.to_string())?;
    Ok((
        stream,
        PianoStatus {
            state: "ready".into(),
            device: device.name().unwrap_or_default(),
            sample_rate: config.sample_rate.0,
            ..Default::default()
        },
    ))
}
fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    font: &Arc<rustysynth::SoundFont>,
    rx: Receiver<Event>,
    shared: Arc<Shared>,
    errors: Sender<String>,
) -> Result<cpal::Stream, String> {
    let mut synth = PianoSynth::new(font, config.sample_rate.0 as i32)?;
    let mut generation = shared.generation.load(Ordering::Acquire);
    let mut left = [0.; 256];
    let mut right = [0.; 256];
    let channels = config.channels as usize;
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let next = shared.generation.load(Ordering::Acquire);
                if generation != next {
                    synth.panic();
                    generation = next;
                }
                synth.set_octave(shared.octave.load(Ordering::Acquire));
                let active = shared.enabled.load(Ordering::Acquire)
                    && !shared.blocked.load(Ordering::Acquire);
                for event in rx.try_iter().take(1024) {
                    if active && event.generation == generation {
                        synth.midi(event.screen, event.bytes);
                    }
                }
                shared
                    .frames
                    .store((data.len() / channels) as u32, Ordering::Relaxed);
                let volume = f32::from_bits(shared.volume.load(Ordering::Relaxed));
                let mut peak = 0f32;
                for chunk in data.chunks_mut(256 * channels) {
                    let frames = chunk.len() / channels;
                    if active {
                        synth.render(&mut left[..frames], &mut right[..frames]);
                    }
                    for (i, frame) in chunk.chunks_mut(channels).enumerate() {
                        for (channel, sample) in frame.iter_mut().enumerate() {
                            let value = if !active {
                                0.
                            } else if channels == 1 {
                                (left[i] + right[i]) * 0.5
                            } else if channel == 0 {
                                left[i]
                            } else if channel == 1 {
                                right[i]
                            } else {
                                0.
                            };
                            let value = (value * volume).tanh();
                            peak = peak.max(value.abs());
                            *sample = T::from_sample(value);
                        }
                    }
                }
                shared.peak.store(peak.to_bits(), Ordering::Relaxed);
            },
            move |e| {
                let _ = errors.try_send(e.to_string());
            },
            None,
        )
        .map_err(|e| e.to_string())
}
