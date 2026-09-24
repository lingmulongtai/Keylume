use crate::instrument_fx::{Effects, InstrumentFx};
use crate::piano::{PianoSettings, PianoSynth};
use crate::sound_library::SoundId;
use crate::{
    drums::{DrumSettings, DrumSynth, PadSound},
    groove::{LoopCommand, LoopStatus, Looper, SoundEvent},
};
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
    sound: Option<SoundEvent>,
    command: Option<LoopCommand>,
}
struct Shared {
    generation: AtomicU64,
    looper: Arc<Mutex<Looper>>,
    enabled: AtomicBool,
    blocked: AtomicBool,
    performing: AtomicBool,
    volume: AtomicU32,
    octave: AtomicI8,
    peak: AtomicU32,
    frames: AtomicU32,
    drums: AtomicBool,
    drum_volume: AtomicU32,
    drum_kit: AtomicU32,
    drum_pads: [[AtomicU32; 5]; 16],
    effects: [AtomicU32; 8],
    patch: AtomicU32,
    loop_mode: AtomicU32,
    loop_beat: AtomicU64,
    loop_count: AtomicU32,
    loop_bars: AtomicU32,
    loop_bpm: AtomicU64,
    loop_full: AtomicBool,
    loop_metronome: AtomicBool,
    loop_undo: AtomicBool,
}
impl Shared {
    fn hit_drum(&self, synth: &mut DrumSynth, pad: u8, velocity: u8) {
        if pad == 16 {
            synth.hit(16, velocity);
            return;
        }
        let Some(params) = self.drum_pads.get(pad as usize) else {
            return;
        };
        let p = params
            .each_ref()
            .map(|v| f32::from_bits(v.load(Ordering::Relaxed)));
        synth.hit_sound(
            PadSound {
                kind: p[0] as u8,
                tune: p[1],
                decay: p[2],
                level: p[3],
                pan: p[4],
            },
            self.drum_kit.load(Ordering::Relaxed) as u8,
            velocity,
        );
    }
}
#[derive(Clone)]
pub struct PianoBus {
    tx: Sender<Event>,
    shared: Arc<Shared>,
}
impl PianoBus {
    pub fn is_performing(&self) -> bool {
        self.shared.performing.load(Ordering::Acquire)
    }
    pub fn set_performing(&self, performing: bool) {
        if self.shared.performing.swap(performing, Ordering::AcqRel) != performing {
            self.panic();
        }
    }
    pub fn octave(&self) -> i8 {
        self.shared.octave.load(Ordering::Acquire)
    }
    pub fn midi(&self, screen: bool, bytes: &[u8]) {
        if bytes.len() != 3
            || !self.shared.enabled.load(Ordering::Acquire)
            || self.shared.blocked.load(Ordering::Acquire)
            || !self.is_performing()
        {
            return;
        }
        if self
            .tx
            .try_send(Event {
                generation: self.shared.generation.load(Ordering::Acquire),
                sound: Some(SoundEvent::Piano(screen, [bytes[0], bytes[1], bytes[2]])),
                command: None,
            })
            .is_err()
        {
            self.panic();
        }
    }
    pub fn drum(&self, pad: u8, velocity: u8) {
        if pad >= 16
            || velocity == 0
            || velocity > 127
            || !self.shared.drums.load(Ordering::Acquire)
            || !self.shared.enabled.load(Ordering::Acquire)
            || self.shared.blocked.load(Ordering::Acquire)
            || !self.is_performing()
        {
            return;
        }
        if self
            .tx
            .try_send(Event {
                generation: self.shared.generation.load(Ordering::Acquire),
                sound: Some(SoundEvent::Drum(pad, velocity)),
                command: None,
            })
            .is_err()
        {
            self.panic();
        }
    }
    pub fn loop_command(&self, command: LoopCommand) -> Result<(), String> {
        if !self.shared.enabled.load(Ordering::Acquire)
            || self.shared.blocked.load(Ordering::Acquire)
            || !self.is_performing()
        {
            return Err("ピアノをオンにして音声出力が準備できてから操作してください".into());
        }
        self.tx
            .try_send(Event {
                generation: self.shared.generation.load(Ordering::Acquire),
                sound: None,
                command: Some(command),
            })
            .map_err(|_| "演奏操作が混み合っています".into())
    }
    pub fn loop_status(&self) -> LoopStatus {
        LoopStatus {
            mode: ["stopped", "countIn", "recording", "playing", "overdub"]
                [self.shared.loop_mode.load(Ordering::Acquire).min(4) as usize]
                .into(),
            beat: f64::from_bits(self.shared.loop_beat.load(Ordering::Relaxed)),
            count: self.shared.loop_count.load(Ordering::Relaxed) as usize,
            beats: self.shared.loop_bars.load(Ordering::Relaxed) as f64 * 4.,
            bpm: f64::from_bits(self.shared.loop_bpm.load(Ordering::Relaxed)),
            metronome: self.shared.loop_metronome.load(Ordering::Relaxed),
            full: self.shared.loop_full.load(Ordering::Relaxed),
            can_undo: self.shared.loop_undo.load(Ordering::Relaxed),
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
    pub fn new(library: Arc<super::sound_library::Library>) -> Self {
        let (tx, rx) = bounded(1024);
        let (wake, wakeup) = bounded(1);
        let shared = Arc::new(Shared {
            generation: AtomicU64::new(0),
            looper: Arc::new(Mutex::new(Looper::default())),
            enabled: AtomicBool::new(false),
            blocked: AtomicBool::new(false),
            performing: AtomicBool::new(true),
            volume: AtomicU32::new(0.5f32.to_bits()),
            octave: AtomicI8::new(0),
            peak: AtomicU32::new(0),
            frames: AtomicU32::new(0),
            drums: AtomicBool::new(true),
            drum_volume: AtomicU32::new(0.7f32.to_bits()),
            drum_kit: AtomicU32::new(0),
            drum_pads: DrumSettings::default().banks[0].map(|p| {
                [p.kind as f32, p.tune, p.decay, p.level, p.pan]
                    .map(|v| AtomicU32::new(v.to_bits()))
            }),
            effects: InstrumentFx::default()
                .values()
                .map(|v| AtomicU32::new(v.to_bits())),
            loop_mode: AtomicU32::new(0),
            patch: AtomicU32::new(0),
            loop_beat: AtomicU64::new(0),
            loop_count: AtomicU32::new(0),
            loop_bars: AtomicU32::new(2),
            loop_bpm: AtomicU64::new(100f64.to_bits()),
            loop_full: AtomicBool::new(false),
            loop_metronome: AtomicBool::new(false),
            loop_undo: AtomicBool::new(false),
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
            let mut endpoint: Option<cpal::Device> = None;
            let mut checked = Instant::now();
            let (error_tx, error_rx) = bounded::<String>(1);
            while !thread_stop.load(Ordering::Acquire) {
                let next = thread_config.lock().unwrap().clone();
                let default_changed = if next.enabled
                    && next.output_device.is_empty()
                    && checked.elapsed() >= Duration::from_secs(1)
                {
                    checked = Instant::now();
                    let candidate = cpal::default_host().default_output_device();
                    !same_endpoint(endpoint.as_ref(), candidate.as_ref())
                } else {
                    false
                };
                let bank_key = |id: &str| SoundId::parse(id).map(|s| s.key).unwrap_or_default();
                let manual_restart = bank_key(&next.sound) != bank_key(&current.sound)
                    || next.enabled != current.enabled
                    || next.output_device != current.output_device
                    || next.buffer_frames != current.buffer_frames;
                if manual_restart || default_changed {
                    thread_bus.shared.enabled.store(false, Ordering::Release);
                    stream = None;
                    thread_bus.panic();
                    endpoint = None;
                    if manual_restart {
                        thread_bus
                            .shared
                            .looper
                            .lock()
                            .unwrap()
                            .command(LoopCommand::Stop);
                    }
                    if !next.enabled {
                        thread_bus.shared.looper.lock().unwrap().reset();
                        thread_bus.shared.loop_undo.store(false, Ordering::Release);
                        thread_bus.shared.loop_mode.store(0, Ordering::Release);
                        thread_bus.shared.loop_count.store(0, Ordering::Release);
                    }
                    retry = Instant::now();
                    for _ in error_rx.try_iter() {}
                }
                current = next;
                if let Ok(error) = error_rx.try_recv() {
                    thread_bus.shared.enabled.store(false, Ordering::Release);
                    stream = None;
                    thread_bus.panic();
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
                        let key = bank_key(&current.sound);
                        if font.as_ref().is_none_or(|(id, _)| id != &key) {
                            font = Some((key, library.load(&current.sound)?));
                        }
                        start(
                            &font.as_ref().unwrap().1,
                            &current,
                            rx.clone(),
                            thread_bus.shared.clone(),
                            error_tx.clone(),
                        )
                    })();
                    match result {
                        Ok((output, status, device)) => {
                            for _ in rx.try_iter() {}
                            endpoint = Some(device);
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
        for (atomic, value) in self
            .bus
            .shared
            .effects
            .iter()
            .zip(settings.effects.values())
        {
            atomic.store(value.to_bits(), Ordering::Release);
        }
        self.bus
            .shared
            .drums
            .store(settings.drums, Ordering::Release);
        self.bus
            .shared
            .drum_kit
            .store(settings.drum_kit.kit as u32, Ordering::Release);
        for (atomic, p) in self
            .bus
            .shared
            .drum_pads
            .iter()
            .zip(&settings.drum_kit.banks[settings.drum_kit.kit as usize])
        {
            for (a, v) in atomic
                .iter()
                .zip([p.kind as f32, p.tune, p.decay, p.level, p.pan])
            {
                a.store(v.to_bits(), Ordering::Release);
            }
        }
        self.bus
            .shared
            .drum_volume
            .store(settings.drum_volume.to_bits(), Ordering::Release);
        self.bus
            .shared
            .volume
            .store(settings.volume.to_bits(), Ordering::Release);
        self.bus
            .shared
            .octave
            .store(settings.octave, Ordering::Release);
        if let Ok(id) = SoundId::parse(&settings.sound) {
            self.bus.shared.patch.store(
                ((id.bank as u32) << 8) | id.program as u32,
                Ordering::Release,
            );
        }
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
    pub fn set_volume(&self, volume: f32) {
        self.config.lock().unwrap().volume = volume;
        self.bus
            .shared
            .volume
            .store(volume.to_bits(), Ordering::Release);
    }
    pub fn volume(&self) -> f32 {
        f32::from_bits(self.bus.shared.volume.load(Ordering::Acquire))
    }
    pub fn view(&self) -> PianoStatus {
        let mut status = self.status.lock().unwrap().clone();
        status.peak = f32::from_bits(self.bus.shared.peak.load(Ordering::Relaxed));
        status.buffer_frames = self.bus.shared.frames.load(Ordering::Relaxed);
        status.muted = self.bus.shared.blocked.load(Ordering::Acquire) || !self.bus.is_performing();
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
) -> Result<(cpal::Stream, PianoStatus, cpal::Device), String> {
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
        device,
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
    let mut loop_synth = PianoSynth::new(font, config.sample_rate.0 as i32)?;
    let mut drums = DrumSynth::new(config.sample_rate.0);
    let mut loop_drums = DrumSynth::new(config.sample_rate.0);
    let mut effects = Effects::new(
        config.sample_rate.0,
        InstrumentFx::from_values(std::array::from_fn(|i| {
            f32::from_bits(shared.effects[i].load(Ordering::Acquire))
        })),
    );
    let loop_session = shared.looper.clone();
    let mut loop_events = Vec::with_capacity(8193);
    let mut loop_left = [0.; 256];
    let mut loop_right = [0.; 256];
    let rate = config.sample_rate.0 as f64;
    let mut generation = shared.generation.load(Ordering::Acquire);
    let mut left = [0.; 256];
    let mut right = [0.; 256];
    let channels = config.channels as usize;
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let Ok(mut looper) = loop_session.try_lock() else {
                    data.fill(T::from_sample(0.));
                    return;
                };
                let next = shared.generation.load(Ordering::Acquire);
                if generation != next {
                    synth.panic();
                    loop_synth.panic();
                    drums.panic();
                    loop_drums.panic();
                    effects.reset();
                    looper.command(LoopCommand::Stop);
                    generation = next;
                }
                synth.set_octave(shared.octave.load(Ordering::Acquire));
                let patch = shared.patch.load(Ordering::Acquire);
                synth.set_patch((patch >> 8) as u16, patch as u8);
                loop_synth.set_patch((patch >> 8) as u16, patch as u8);
                let active = shared.enabled.load(Ordering::Acquire)
                    && !shared.blocked.load(Ordering::Acquire)
                    && shared.performing.load(Ordering::Acquire);
                for event in rx.try_iter().take(1024) {
                    if active && event.generation == generation {
                        if let Some(command) = event.command {
                            looper.command(command);
                            if !matches!(
                                command,
                                LoopCommand::Overdub
                                    | LoopCommand::Configure(_)
                                    | LoopCommand::Tempo(_)
                                    | LoopCommand::Metronome
                            ) {
                                loop_synth.panic();
                                loop_drums.panic();
                            }
                        }
                        if let Some(sound) = event.sound {
                            looper.capture(sound, shared.octave.load(Ordering::Acquire));
                            match sound {
                                SoundEvent::Piano(screen, bytes) => synth.midi(screen, bytes),
                                SoundEvent::Drum(pad, velocity) => {
                                    shared.hit_drum(&mut drums, pad, velocity)
                                }
                                SoundEvent::Release => {}
                            }
                        }
                    }
                }
                shared
                    .frames
                    .store((data.len() / channels) as u32, Ordering::Relaxed);
                let volume = f32::from_bits(shared.volume.load(Ordering::Relaxed));
                let drum_volume = f32::from_bits(shared.drum_volume.load(Ordering::Relaxed));
                let effect_settings = InstrumentFx::from_values(std::array::from_fn(|i| {
                    f32::from_bits(shared.effects[i].load(Ordering::Relaxed))
                }));
                let mut peak = 0f32;
                for chunk in data.chunks_mut(64 * channels) {
                    let frames = chunk.len() / channels;
                    if active {
                        looper.advance(frames as f64 / rate, &mut loop_events);
                        for event in &loop_events {
                            match *event {
                                SoundEvent::Piano(screen, bytes) => loop_synth.midi(screen, bytes),
                                SoundEvent::Drum(pad, velocity) => {
                                    shared.hit_drum(&mut loop_drums, pad, velocity)
                                }
                                SoundEvent::Release => loop_synth.panic(),
                            }
                        }
                        synth.render(&mut left[..frames], &mut right[..frames]);
                        loop_synth.render(&mut loop_left[..frames], &mut loop_right[..frames]);
                        for i in 0..frames {
                            left[i] += loop_left[i];
                            right[i] += loop_right[i];
                        }
                        effects.process(&mut left[..frames], &mut right[..frames], effect_settings);
                        for i in 0..frames {
                            let (dl, dr) = drums.stereo();
                            let (ll, lr) = loop_drums.stereo();
                            left[i] = left[i] * volume + (dl + ll) * drum_volume;
                            right[i] = right[i] * volume + (dr + lr) * drum_volume;
                        }
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
                            let value = value.tanh();
                            peak = peak.max(value.abs());
                            *sample = T::from_sample(value);
                        }
                    }
                }
                shared
                    .loop_mode
                    .store(looper.mode as u32, Ordering::Release);
                shared
                    .loop_beat
                    .store(looper.beat.to_bits(), Ordering::Relaxed);
                shared
                    .loop_count
                    .store(looper.count() as u32, Ordering::Relaxed);
                shared
                    .loop_bars
                    .store(looper.config.bars as u32, Ordering::Relaxed);
                shared
                    .loop_bpm
                    .store(looper.config.bpm.to_bits(), Ordering::Relaxed);
                shared
                    .loop_metronome
                    .store(looper.config.metronome, Ordering::Relaxed);
                shared.loop_full.store(looper.full, Ordering::Relaxed);
                shared.loop_undo.store(looper.can_undo(), Ordering::Relaxed);
                shared.peak.store(peak.to_bits(), Ordering::Relaxed);
            },
            move |e| {
                let _ = errors.try_send(e.to_string());
            },
            None,
        )
        .map_err(|e| e.to_string())
}

// CPAL WASAPI compares IMMDevice endpoint IDs, including devices with identical display names.
fn same_endpoint(a: Option<&cpal::Device>, b: Option<&cpal::Device>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            #[cfg(windows)]
            {
                use cpal::platform::DeviceInner::Wasapi;
                let (Wasapi(a), Wasapi(b)) = (a.as_inner(), b.as_inner());
                a == b
            }
            #[cfg(not(windows))]
            {
                a.name().ok() == b.name().ok()
            }
        }
        _ => false,
    }
}
