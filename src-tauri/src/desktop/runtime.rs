use super::{audio::AudioCapture, piano::Piano, system};
use crate::{
    device::{
        constants::*,
        hardware::{self, HardwareTransport, MidiPacket, Ports},
        input::InputState,
        protocol,
        transport::{differences, LedTransport, MockTransport},
    },
    engine::{Color, Engine, Hit},
    model::*,
    profiles,
    routing::{self, Destination},
    storage::Storage,
};
use chrono::Timelike;
use crossbeam_channel::{bounded, Receiver, Sender};
use midir::MidiOutputConnection;
use serde::Serialize;
use std::{
    collections::{HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub connection: String,
    pub device_name: String,
    pub service: String,
    pub daw_active: Vec<String>,
    pub active_profile: Option<String>,
    pub active_preset: String,
    pub effective_mode: CoexistMode,
    pub pads_port: bool,
    pub controls_port: bool,
    pub audio: String,
    pub keyboard: String,
    pub inquiry: String,
    pub pad_mode: u8,
    pub messages: u64,
    pub bytes: u64,
    pub bpm: f32,
    pub warnings: Vec<String>,
    pub monitor: Vec<String>,
    pub ports: Ports,
    pub audio_devices: Vec<String>,
    pub probe: Option<String>,
    pub hardware_verified: bool,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            connection: "starting".into(),
            device_name: "MockDevice · Launchkey MK4 61".into(),
            service: system::midi_service(),
            daw_active: vec![],
            active_profile: None,
            active_preset: "aurora".into(),
            effective_mode: CoexistMode::LightingFirst,
            pads_port: false,
            controls_port: false,
            audio: "stopped".into(),
            keyboard: "未接続".into(),
            inquiry: String::new(),
            pad_mode: 2,
            messages: 0,
            bytes: 0,
            bpm: 120.,
            warnings: vec![],
            monitor: vec![],
            ports: Ports::default(),
            audio_devices: vec![],
            probe: None,
            hardware_verified: false,
        }
    }
}
#[derive(Clone)]
pub struct Control {
    pub settings: Settings,
    pub presets: Vec<Preset>,
    pub profiles: Vec<Profile>,
    pub layout: DeviceLayout,
    pub draft: Preset,
    pub paused: bool,
    pub revision: u64,
    pub status: Status,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateView {
    pub settings: Settings,
    pub presets: Vec<Preset>,
    pub profiles: Vec<Profile>,
    pub layout: DeviceLayout,
    pub preset: Preset,
    pub paused: bool,
    pub status: Status,
    pub storage_path: String,
    pub debug: bool,
}
pub struct Core {
    pub control: Mutex<Control>,
    pub storage: Mutex<Storage>,
    pub action: Sender<Action>,
    pub input: Mutex<InputState>,
    pub piano: Piano,
    pub quitting: AtomicBool,
    pub terminated: AtomicBool,
}
pub enum Action {
    Input(MidiPacket),
    Probe(String, u8),
    StopProbe,
    Feature(u8, u8),
    Reconnect,
    MockDaw(bool),
    MockDisconnect(bool),
    Tap,
    Panic,
}
impl Core {
    pub fn create(mut storage: Storage) -> (Arc<Self>, Receiver<Action>) {
        let settings = storage.settings();
        let presets = storage.presets();
        let profiles = storage.profiles();
        let layout = storage.layout();
        let draft = presets
            .iter()
            .find(|p| p.id == settings.active_preset)
            .unwrap_or(&presets[0])
            .clone();
        let status = Status {
            warnings: storage.notices.clone(),
            ..Status::default()
        };
        storage.prune_logs();
        storage.log("Keylume starting");
        let (tx, rx) = bounded(256);
        (
            Arc::new(Self {
                control: Mutex::new(Control {
                    settings,
                    presets,
                    profiles,
                    layout,
                    draft,
                    paused: false,
                    revision: 1,
                    status,
                }),
                storage: Mutex::new(storage),
                action: tx,
                input: Mutex::new(InputState::default()),
                piano: Piano::new(),
                quitting: AtomicBool::new(false),
                terminated: AtomicBool::new(false),
            }),
            rx,
        )
    }
    pub fn view(&self) -> Result<StateView, String> {
        let c = self.control.lock().map_err(|e| e.to_string())?;
        Ok(StateView {
            settings: c.settings.clone(),
            presets: c.presets.clone(),
            profiles: c.profiles.clone(),
            layout: c.layout.clone(),
            preset: c.draft.clone(),
            paused: c.paused,
            status: c.status.clone(),
            storage_path: self
                .storage
                .lock()
                .map_err(|e| e.to_string())?
                .root
                .to_string_lossy()
                .into_owned(),
            debug: cfg!(debug_assertions),
        })
    }
    pub fn notify(&self, app: &AppHandle, text: &str) {
        if let Ok(s) = self.storage.lock() {
            s.log(text);
        }
        let _ = app.emit("notice", text);
    }
}
struct ForwardPorts {
    pads: Option<MidiOutputConnection>,
    controls: Option<MidiOutputConnection>,
    held: HashSet<(u8, u8)>,
}
impl ForwardPorts {
    fn new() -> Self {
        Self {
            pads: None,
            controls: None,
            held: HashSet::new(),
        }
    }
    fn release(&mut self) {
        if let Some(p) = &mut self.pads {
            for (channel, note) in self.held.drain() {
                let _ = p.send(&[0x80 | channel, note, 0]);
            }
        }
        self.held.clear();
        if let Some(c) = &mut self.controls {
            for ch in 0..16 {
                let _ = c.send(&[0xb0 | ch, 64, 0]);
            }
        }
    }
    fn close(&mut self) {
        self.release();
        self.pads = None;
        self.controls = None;
    }
    fn send(&mut self, d: Destination, b: Vec<u8>) {
        let port = match d {
            Destination::Pads => &mut self.pads,
            Destination::Controls => &mut self.controls,
        };
        if let Some(p) = port {
            if p.send(&b).is_err() {
                self.release();
                if d == Destination::Pads {
                    self.pads = None;
                } else {
                    self.controls = None;
                }
                return;
            }
            if d == Destination::Pads {
                let key = (b[0] & 15, b[1]);
                if b[0] & 0xf0 == 0x90 && b[2] > 0 {
                    self.held.insert(key);
                } else if b[0] & 0xf0 == 0x80 || (b[0] & 0xf0 == 0x90 && b[2] == 0) {
                    self.held.remove(&key);
                }
            }
        }
    }
}
struct Probe {
    led: LedDef,
    variant: u8,
    started: f32,
}
pub fn spawn(app: AppHandle, core: Arc<Core>, actions: Receiver<Action>) {
    std::thread::spawn(move || worker(app, core, actions));
}
fn worker(app: AppHandle, core: Arc<Core>, actions: Receiver<Action>) {
    let start = Instant::now();
    let (tx, rx) = bounded::<MidiPacket>(1024);
    let mut desired = core.control.lock().unwrap().clone();
    let mut status = desired.status.clone();
    let mut engine = Engine::default();
    let mut transport: Option<Box<dyn LedTransport>> = None;
    let mut forward = ForwardPorts::new();
    let mut keyboard: Option<hardware::KeyboardInput> = None;
    let mut keyboard_retry = 0.;
    core.piano.configure(&desired.settings.piano);
    let mut audio: Option<AudioCapture> = None;
    let mut audio_retry = 0f32;
    let mut last_colors: Vec<Color> = vec![];
    let mut last_frame: Vec<[u8; 3]> = vec![];
    let mut active_preset = desired.draft.clone();
    let mut active_mode = desired.settings.coexist_mode.clone();
    let mut last_tx = -1f32;
    let mut last_preview = -1f32;
    let mut applied_revision = 0;
    let mut last_full = -5f32;
    let mut last_scan = -3f32;
    let mut last_status = -1f32;
    let mut last_query = 0f32;
    let mut last_response = 0f32;
    let mut pending_query = false;
    let mut repair_after = 0f32;
    let mut next_connect = 0f32;
    let mut attempts = 0;
    let mut connected_at = 0f32;
    let mut last_daw = false;
    let mut resume_at = 0f32;
    let mut was_suspended = false;
    let mut resume_generation = system::RESUME_GENERATION.load(Ordering::SeqCst);
    let mut mock_daw = false;
    let mut mock_disconnected = false;
    let mut monitor = VecDeque::new();
    let mut probe: Option<Probe> = None;
    let mut feature: std::collections::HashMap<u8, (u8, f32)> = Default::default();
    let mut bitmap_pending: Option<f32> = None;
    let mut display_disabled = false;
    let mut last_display = -10f32;
    let mut previous_widget = String::new();
    let mut oled_active = false;
    let mut temporary_pending = true;
    let mut tap: Option<f32> = None;
    let discovery = super::discovery::Discovery::start(&core);
    let mut discovery_ready = false;
    let mut running = vec![];
    let mut foreground = String::new();
    let mut errors = 0u32;
    loop {
        let tick = Instant::now();
        let now = start.elapsed().as_secs_f32();
        engine.time = now;
        let changed = {
            let c = core.control.lock().unwrap();
            if c.revision != desired.revision {
                Some(c.clone())
            } else {
                None
            }
        };
        if let Some(next) = changed {
            let a = &desired.settings;
            let b = &next.settings;
            let layout_changed = serde_json::to_string(&desired.layout).ok()
                != serde_json::to_string(&next.layout).ok();
            let reconnect = a.mock != b.mock
                || a.keyboard_reactive != b.keyboard_reactive
                || a.daw_drum != b.daw_drum
                || layout_changed;
            if a.pads_port != b.pads_port
                || a.controls_port != b.controls_port
                || a.pad_notes != b.pad_notes
                || a.pad_channel != b.pad_channel
                || a.forwarding != b.forwarding
                || a.cc_map != b.cc_map
                || a.control_channel != b.control_channel
                || a.shortcut_ccs != b.shortcut_ccs
            {
                forward.close();
                last_scan = -3.;
            }
            if a.audio_device != b.audio_device {
                audio = None;
                audio_retry = 0.;
            }
            if reconnect {
                release(
                    &mut transport,
                    &desired.layout,
                    &last_frame,
                    a,
                    &mut status,
                    &mut monitor,
                    false,
                );
                forward.close();
                next_connect = now;
                attempts = 0;
            }
            core.piano.configure(&next.settings.piano);
            if a.mock != b.mock {
                keyboard = None;
                core.piano.bus.panic();
                keyboard_retry = now;
            }
            desired = next;
            last_full = -5.;
            display_disabled = false;
            last_display = -10.;
            previous_widget.clear();
            temporary_pending = true;
        }
        for action in actions.try_iter().take(128) {
            match action {
                Action::Input(packet) => {
                    if packet.source == "keyboard" {
                        core.piano.bus.midi(false, &packet.bytes);
                    }
                    if tx.try_send(packet).is_err() {
                        hardware::INPUT_OVERFLOW.store(true, Ordering::SeqCst);
                    }
                }
                Action::Panic => {
                    core.piano.bus.panic();
                    engine.held.clear();
                    *core.input.lock().unwrap() = InputState::default();
                    for _ in rx.try_iter() {}
                }

                Action::Reconnect => {
                    keyboard = None;
                    keyboard_retry = now;
                    core.piano.bus.panic();
                    release(
                        &mut transport,
                        &desired.layout,
                        &last_frame,
                        &desired.settings,
                        &mut status,
                        &mut monitor,
                        false,
                    );
                    forward.close();
                    next_connect = now;
                    attempts = 0;
                }
                Action::Probe(id, variant) => {
                    if let Some(led) = desired.layout.leds.iter().find(|l| l.id == id) {
                        probe = Some(Probe {
                            led: led.clone(),
                            variant,
                            started: now,
                        });
                        status.probe = Some(id);
                    }
                }
                Action::StopProbe => {
                    probe = None;
                    status.probe = None;
                    last_full = -5.;
                }
                Action::Feature(cc, value) => {
                    feature.insert(cc, (value, now + 1.));
                }
                Action::MockDaw(v) => {
                    mock_daw = v;
                    last_scan = -3.;
                }
                Action::MockDisconnect(v) => {
                    mock_disconnected = v;
                    if v {
                        transport = None;
                        forward.close();
                    } else {
                        next_connect = now;
                    }
                }
                Action::Tap => {
                    if let Some(t) = tap {
                        let d = now - t;
                        if (0.2..2.).contains(&d) {
                            engine.bpm = 60. / d;
                        }
                    }
                    tap = Some(now);
                }
            }
        }
        let settings = &desired.settings;
        if now - last_scan >= 2. {
            last_scan = now;
            let processes = discovery.processes.lock().unwrap().clone();
            let devices = discovery.devices.lock().unwrap().clone();
            discovery_ready = processes.is_some() && devices.is_some();
            if let Some(processes) = processes {
                running = processes.running;
                foreground = processes.foreground;
            }
            status.daw_active = running
                .iter()
                .filter(|r| {
                    settings
                        .daw_processes
                        .iter()
                        .any(|p| profiles::wildcard(p, r))
                })
                .cloned()
                .collect();
            status.daw_active.sort();
            status.daw_active.dedup();
            if settings.mock && mock_daw {
                status.daw_active.push("Mock DAW".into());
            }
            if let Some(devices) = devices {
                status.ports = devices.ports;
                status.audio_devices = devices.audio;
            }
            if !status.ports.outputs.contains(&settings.pads_port) && forward.pads.is_some() {
                forward.close();
            }
            if !status.ports.outputs.contains(&settings.controls_port) && forward.controls.is_some()
            {
                forward.close();
            }
            if !settings.mock
                && transport.is_some()
                && !status.ports.outputs.contains(&status.device_name)
            {
                release(
                    &mut transport,
                    &desired.layout,
                    &last_frame,
                    settings,
                    &mut status,
                    &mut monitor,
                    false,
                );
                forward.close();
                next_connect = now + 2.;
                core.notify(&app, "Launchkey が切断されました");
            }
            // Profiles are evaluated before connection, so handoff never seizes an already-busy DAW port.
            if settings.forwarding
                && active_mode == CoexistMode::LightingFirst
                && !settings.mock
                && transport.is_some()
            {
                if forward.pads.is_none() {
                    forward.pads = hardware::open_forward(&settings.pads_port).ok();
                }
                if forward.controls.is_none() {
                    forward.controls = hardware::open_forward(&settings.controls_port).ok();
                }
            }
        }
        let local = chrono::Local::now();
        let minute = local.hour() * 60 + local.minute();
        let idle = system::idle_minutes();
        let selected = if settings.manual_lock {
            None
        } else {
            profiles::select(&desired.profiles, &running, &foreground, minute, idle)
        };
        status.active_profile = selected.map(|p| p.name.clone());
        let next_preset = selected
            .and_then(|p| desired.presets.iter().find(|x| x.id == p.preset_id))
            .unwrap_or(&desired.draft);
        let next_mode = selected
            .map(|p| p.coexist_mode.clone())
            .unwrap_or(settings.coexist_mode.clone());
        if active_preset.id != next_preset.id || applied_revision != desired.revision {
            if serde_json::to_string(&active_preset).ok() != serde_json::to_string(next_preset).ok()
            {
                engine.transition(last_colors.clone());
                active_preset = next_preset.clone();
                last_full = -5.;
                last_display = -10.;
                bitmap_pending = None;
                display_disabled = false;
                previous_widget.clear();
                temporary_pending = true;
            }
            applied_revision = desired.revision;
        }
        if active_mode != next_mode {
            forward.close();
        }
        active_mode = next_mode;
        status.effective_mode = active_mode.clone();
        status.active_preset = active_preset.id.clone();
        let daw = !status.daw_active.is_empty();
        if last_daw && !daw && active_mode == CoexistMode::Handoff {
            resume_at = now + 3.;
        }
        last_daw = daw;
        let suspended = system::SUSPENDED.load(Ordering::SeqCst);
        let generation = system::RESUME_GENERATION.load(Ordering::SeqCst);
        if (was_suspended && !suspended) || generation != resume_generation {
            resume_at = now + 2.;
            resume_generation = generation;
        }
        was_suspended = suspended;
        let stopping = core.quitting.load(Ordering::SeqCst);
        let handoff = active_mode == CoexistMode::Handoff && (daw || now < resume_at);
        let inactive = desired.paused
            || suspended
            || handoff
            || stopping
            || mock_disconnected
            || now < resume_at;
        core.piano.bus.block(
            suspended
                || handoff
                || stopping
                || mock_disconnected
                || now < resume_at
                || (settings.piano.mute_with_daw && daw),
        );
        let keyboard_needed = !settings.mock
            && !suspended
            && !handoff
            && !stopping
            && now >= resume_at
            && (!desired.paused || settings.piano.enabled);
        if !keyboard_needed
            || keyboard
                .as_ref()
                .is_some_and(|k| !status.ports.inputs.contains(&k.name))
        {
            if keyboard.take().is_some() {
                core.piano.bus.panic();
                *core.input.lock().unwrap() = InputState::default();
            }
            status.keyboard = if handoff {
                "DAW に譲っています"
            } else {
                "未接続"
            }
            .into();
        }
        if keyboard_needed && keyboard.is_none() && now >= keyboard_retry && discovery_ready {
            let bus = core.piano.bus.clone();
            match hardware::open_keyboard(tx.clone(), move |b| bus.midi(false, b)) {
                Ok(input) => {
                    status.keyboard = input.name.clone();
                    keyboard = Some(input);
                }
                Err(error) => {
                    status.keyboard = error;
                    keyboard_retry = now + 3.;
                }
            }
        }
        if settings.mock {
            status.keyboard = "プレビュー入力".into();
        }
        if inactive {
            if transport.is_some() {
                release(
                    &mut transport,
                    &desired.layout,
                    &last_frame,
                    settings,
                    &mut status,
                    &mut monitor,
                    settings.fade_on_release && !suspended,
                );
                forward.close();
                engine.held.clear();
                probe = None;
                status.probe = None;
                core.notify(
                    &app,
                    if handoff {
                        "ライティングを DAW に譲りました"
                    } else {
                        "ライティングを停止しました"
                    },
                );
            }
            status.connection = if handoff {
                "handoff"
            } else if desired.paused {
                "paused"
            } else {
                "disconnected"
            }
            .into();
            if stopping {
                core.piano.stop();
                drop(keyboard.take());
                core.terminated.store(true, Ordering::SeqCst);
                app.exit(0);
                break;
            }
        } else if transport.is_none()
            && now >= next_connect
            && attempts < 12
            && (settings.mock || discovery_ready)
        {
            for _ in rx.try_iter() {}
            *core.input.lock().unwrap() = InputState::default();
            engine.held.clear();
            let result: Result<Box<dyn LedTransport>, String> = if settings.mock {
                status.device_name = "MockDevice · Launchkey MK4 61".into();
                Ok(Box::new(MockTransport::connected()))
            } else {
                HardwareTransport::connect(tx.clone()).map(|h| {
                    status.device_name = h.name.clone();
                    Box::new(h) as Box<dyn LedTransport>
                })
            };
            match result {
                Ok(mut t) => {
                    let init = [
                        INQUIRY.to_vec(),
                        DAW_ON.to_vec(),
                        vec![0xb6, PAD_MODE, 2],
                        vec![0xb7, 0x1e, 0],
                        vec![0xb7, 0x1f, 0],
                        vec![0xb6, DAW_DRUM, if settings.daw_drum { 1 } else { 0 }],
                    ];
                    let mut success = true;
                    for message in init {
                        if send(
                            &mut *t,
                            &message,
                            &mut status,
                            &mut monitor,
                            settings.midi_log,
                        )
                        .is_err()
                        {
                            success = false;
                            break;
                        }
                    }
                    if success {
                        *core.input.lock().unwrap() = InputState::default();
                        engine.held.clear();
                        transport = Some(t);
                        last_full = -5.;
                        last_frame.clear();
                        status.pad_mode = 2;
                        attempts = 0;
                        errors = 0;
                        connected_at = now;
                        status.inquiry = if settings.mock {
                            "Mock inquiry · manufacturer 00 20 29".into()
                        } else {
                            String::new()
                        };
                        status.connection = if settings.mock {
                            "preview"
                        } else {
                            "connected"
                        }
                        .into();
                        last_query = now;
                        last_response = now;
                        pending_query = false;
                        display_disabled = false;
                        bitmap_pending = None;
                        previous_widget.clear();
                        temporary_pending = true;
                        last_display = -10.;
                        core.notify(&app, "Launchkey のライティングを開始しました");
                    } else {
                        let _ = t.send_raw(&[0xb6, DAW_DRUM, 0]);
                        let _ = t.send_raw(&DAW_OFF);
                        next_connect = now + 5.;
                        attempts += 1;
                    }
                }
                Err(e) => {
                    status.connection = if e.contains("未対応") {
                        "unsupported"
                    } else {
                        "disconnected"
                    }
                    .into();
                    warn(&mut status, &e);
                    next_connect = now
                        + if e.contains("開けません") {
                            5.
                        } else {
                            2.
                        };
                    if e.contains("開けません") {
                        attempts += 1;
                    } else {
                        attempts = 0;
                    }
                }
            }
        }
        if hardware::INPUT_OVERFLOW.swap(false, Ordering::SeqCst) {
            // Discard the uncertain event batch and release every note we actually sent.
            for _ in rx.try_iter() {}
            forward.release();
            engine.held.clear();
            core.piano.bus.panic();
            *core.input.lock().unwrap() = InputState::default();
            warn(&mut status, "入力が集中したため、転送ノートを解放しました");
        }
        if (inactive && keyboard.is_none())
            || (!settings.mock && transport.is_none() && keyboard.is_none())
        {
            *core.input.lock().unwrap() = InputState::default();
            engine.held.clear();
        }
        for packet in rx.try_iter().take(512) {
            let b = &packet.bytes;
            if !inactive
                || ((packet.source == "keyboard" || packet.source == "screen")
                    && !suspended
                    && !handoff
                    && !stopping
                    && !mock_disconnected)
            {
                core.input
                    .lock()
                    .unwrap()
                    .receive(&packet.source, b, &desired.layout);
            }
            if settings.midi_log {
                log_midi(&mut monitor, "←", b);
            }
            if protocol::is_bitmap_ack(b) {
                bitmap_pending = None;
            }
            if protocol::is_novation_inquiry(b) {
                status.inquiry = hex_bytes(b);
            }
            if b.len() >= 3 && b[0] == 0xb6 && b[1] == PAD_MODE {
                status.pad_mode = b[2];
                last_response = now;
                pending_query = false;
                last_full = -5.;
            }
            if b == &DAW_OFF && active_mode == CoexistMode::LightingFirst {
                repair_after = now + 0.5;
            }
            if b.len() == 1 && b[0] == 0xf8 {
                engine.clock(
                    packet
                        .received_at
                        .saturating_duration_since(start)
                        .as_secs_f64(),
                );
            }
            if !inactive && b.len() == 3 {
                let kind = b[0] & 0xf0;
                let pressed = kind == 0x90 && b[2] > 0;
                let released = kind == 0x80 || (kind == 0x90 && b[2] == 0);
                if pressed || released {
                    let led = if packet.source == "daw" {
                        desired.layout.leds.iter().find(|l| {
                            l.group == "pads"
                                && if b[0] & 15 == 9 {
                                    l.address.drum_note == Some(b[1])
                                } else {
                                    l.address.daw_note == Some(b[1])
                                }
                        })
                    } else {
                        None
                    };
                    if (packet.source == "keyboard" || packet.source == "screen")
                        && settings.keyboard_reactive
                    {
                        if pressed {
                            engine.held.insert(b[1]);
                        } else {
                            engine.held.remove(&b[1]);
                        }
                    }
                    if pressed && (packet.source == "daw" || settings.keyboard_reactive) {
                        engine.hit(Hit {
                            x: led
                                .map(|l| (l.pos.x + l.size.w / 2.) / desired.layout.canvas.w)
                                .unwrap_or_else(|| desired.layout.key_position(b[1])),
                            y: led
                                .map(|l| (l.pos.y + l.size.h / 2.) / desired.layout.canvas.h)
                                .unwrap_or_else(|| desired.layout.keybed_y()),
                            at: now,
                            velocity: b[2] as f32 / 127.,
                            led: led.map(|l| l.id.clone()),
                            note: b[1],
                        });
                    }
                    if app.get_webview_window("main").is_some() {
                        let _=app.emit("input_event",serde_json::json!({"note":b[1],"ledId":led.map(|l|&l.id),"pressed":pressed}));
                    }
                }
                if packet.source == "daw" && active_mode == CoexistMode::LightingFirst {
                    if let Some((destination, bytes)) = routing::route(b, &desired.layout, settings)
                    {
                        forward.send(destination, bytes);
                    }
                }
            }
        }
        if settings.keyboard_reactive && !inactive {
            engine.held = core
                .input
                .lock()
                .unwrap()
                .held
                .iter()
                .filter_map(|id| id.strip_prefix("key.").and_then(|n| n.parse().ok()))
                .collect();
        } else {
            engine.held.clear();
        }
        let audio_needed = (active_preset
            .layers
            .iter()
            .any(|l| l.enabled && l.effect.starts_with("audio_"))
            || active_preset
                .display
                .as_ref()
                .is_some_and(|d| d.enabled && d.widget == "miniSpectrum"))
            && !inactive;
        if audio.as_ref().is_some_and(|a| a.errors.try_recv().is_ok()) {
            audio = None;
            engine.bands = [0.; 8];
            engine.peaks = [0.; 8];
            status.audio = "音声デバイスが切断されました。再試行中".into();
            audio_retry = now + 2.;
        }
        if audio_needed && audio.is_none() && now >= audio_retry {
            match AudioCapture::start(&settings.audio_device) {
                Ok(a) => {
                    audio = Some(a);
                    status.audio = "capturing".into();
                }
                Err(e) => {
                    status.audio = format!("音声を取得できません: {e}");
                    audio_retry = now + 5.;
                }
            }
        }
        if !audio_needed {
            audio = None;
            engine.bands = [0.; 8];
            engine.peaks = [0.; 8];
            status.audio = "stopped".into();
        }
        if let Some(a) = &audio {
            for bands in a.levels.try_iter() {
                engine.bands = bands;
            }
            for i in 0..8 {
                engine.peaks[i] = (engine.peaks[i] * 0.985).max(engine.bands[i]);
            }
        }
        let mut brightness = settings.master_brightness;
        if system::LOCKED.load(Ordering::SeqCst)
            || (settings.idle_minutes > 0 && idle >= settings.idle_minutes)
        {
            brightness *= settings.idle_brightness;
        }
        if settings.night_enabled
            && profiles::in_time_range(minute, &settings.night_start, &settings.night_end)
        {
            brightness *= settings.night_brightness;
        }
        let (colors, mut frame) = engine.render(
            &active_preset,
            &desired.layout,
            if inactive { 0. } else { brightness },
        );
        last_colors = colors;
        if let Some(p) = &probe {
            frame.fill([0; 3]);
            if let Some(i) = desired.layout.leds.iter().position(|l| l.id == p.led.id) {
                frame[i] = if p.variant == 0 {
                    let mut c = [0; 3];
                    c[((now - p.started) as usize) % 3] = 90;
                    c
                } else {
                    [90; 3]
                };
            }
        }
        if let Some(t) = &mut transport {
            let full = now - last_full >= 3.
                && settings.auto_repair
                && active_mode == CoexistMode::LightingFirst
                || last_full < 0.;
            if now - last_tx >= 1. / settings.fps as f32 {
                last_tx = now;
                let hardware_layer = active_preset
                    .layers
                    .iter()
                    .filter(|l| l.enabled)
                    .collect::<Vec<_>>();
                let power_save = hardware_layer.len() == 1
                    && hardware_layer[0].effect == "hardware_fx"
                    && probe.is_none()
                    && brightness >= 0.999
                    && active_preset.post.brightness >= 0.999
                    && active_preset.post.saturation == 1.
                    && active_preset.post.temperature_k == 6500.
                    && active_preset.post.gamma == 2.2
                    && hardware_layer[0].opacity == 1.;
                if active_preset.id == "vegas" {
                    warn(
                        &mut status,
                        "本体デモの値は未検証です。デバイス画面で明示的に操作してください",
                    );
                }
                for (i, color) in differences(&last_frame, &frame, full) {
                    let led = &desired.layout.leds[i];
                    if led.kind == "none" && probe.is_none() {
                        continue;
                    }
                    if led.group == "pads" && ![2, 15].contains(&status.pad_mode) && probe.is_none()
                    {
                        continue;
                    }
                    let msg = if let Some(p) = probe.as_ref().filter(|p| p.led.id == led.id) {
                        let mut candidate = p.led.clone();
                        if p.variant > 0 {
                            candidate.address.mono_status =
                                Some(if p.variant == 1 { 0xb3 } else { 0x93 });
                            protocol::mono(&candidate, 90)
                        } else {
                            protocol::rgb(&candidate, color, status.pad_mode == 15)
                        }
                    } else if power_save && led.kind == "rgb" {
                        if !full {
                            continue;
                        }
                        let layer = hardware_layer[0];
                        protocol::palette(
                            led,
                            if brightness == 0. || !layer.zone.contains(led) {
                                0
                            } else {
                                layer.number("palette", 76.).clamp(0., 127.) as u8
                            },
                            match layer.text("mode", "pulse") {
                                "flash" => 1,
                                "pulse" => 2,
                                _ => 0,
                            },
                            status.pad_mode == 15,
                        )
                    } else if led.kind == "mono" {
                        protocol::mono(led, color[0])
                    } else {
                        protocol::rgb(led, color, status.pad_mode == 15)
                    };
                    if let Ok(msg) = msg {
                        if send(&mut **t, &msg, &mut status, &mut monitor, settings.midi_log)
                            .is_err()
                        {
                            errors += 1;
                        } else {
                            errors = 0;
                        }
                    }
                }
                if full {
                    last_full = now;
                }
                last_frame = frame.clone();
                if t.flush().is_err() {
                    errors += 3;
                }
            }
            if active_mode == CoexistMode::LightingFirst && settings.auto_repair {
                if now - last_query >= 10. {
                    let _ = send(
                        &mut **t,
                        &[0xb7, PAD_MODE, 0],
                        &mut status,
                        &mut monitor,
                        settings.midi_log,
                    );
                    last_query = now;
                    pending_query = true;
                    if settings.mock {
                        last_response = now;
                        pending_query = false;
                    }
                }
                if pending_query && now - last_query > 2. && last_response < last_query {
                    warn(
                        &mut status,
                        "モード問い合わせに応答がありません（実機確認が必要です）",
                    );
                    pending_query = false;
                    if [2, 15].contains(&status.pad_mode) && now > repair_after {
                        let _ = send(
                            &mut **t,
                            &DAW_ON,
                            &mut status,
                            &mut monitor,
                            settings.midi_log,
                        );
                        last_full = -5.;
                        repair_after = now + 30.;
                    }
                }
            }
            if now - connected_at > 2. && status.inquiry.is_empty() {
                status.inquiry = "応答なし（ポート名で 61 鍵を確認）".into();
            }
            let due: Vec<_> = feature
                .iter()
                .filter(|(_, (_, at))| *at <= now)
                .map(|(&cc, &(v, _))| (cc, v))
                .collect();
            for (cc, value) in due {
                let _ = send(
                    &mut **t,
                    &[0xb6, cc, value],
                    &mut status,
                    &mut monitor,
                    settings.midi_log,
                );
                feature.remove(&cc);
            }
            let display_enabled = active_preset
                .display
                .as_ref()
                .is_some_and(|d| d.enabled && d.widget != "off");
            if oled_active && !display_enabled {
                if let Ok(msg) = protocol::sysex(&[4, 0x20, 0]) {
                    let _ = send(&mut **t, &msg, &mut status, &mut monitor, settings.midi_log);
                }
                bitmap_pending = None;
            }
            oled_active = display_enabled;
            if display_enabled && temporary_pending {
                if active_preset
                    .display
                    .as_ref()
                    .is_some_and(|d| d.show_on_preset_change)
                {
                    for msg in
                        protocol::display_text(&format!("Keylume {}", active_preset.id), 0x21)
                            .unwrap_or_default()
                    {
                        let _ = send(&mut **t, &msg, &mut status, &mut monitor, settings.midi_log);
                    }
                }
                temporary_pending = false;
            }
            if let Some(display) = active_preset
                .display
                .as_ref()
                .filter(|d| d.enabled && d.widget != "off")
            {
                if !display_disabled && probe.is_none() {
                    if bitmap_pending.is_some_and(|at| now - at > 2.) {
                        display_disabled = true;
                        warn(
                            &mut status,
                            "OLED の受理応答がありません。再接続まで画像送信を停止します",
                        );
                    }
                    let interval = if display.widget == "miniSpectrum" {
                        0.1
                    } else {
                        1.
                    };
                    if now - last_display >= interval && bitmap_pending.is_none() {
                        last_display = now;
                        let mut messages = vec![];
                        if display.widget == "image" || display.widget == "miniSpectrum" {
                            let bits = if display.widget == "image" {
                                display.image_bits.clone().unwrap_or(vec![0; 8192])
                            } else {
                                let mut bits = vec![0; 8192];
                                for y in 0..64 {
                                    for x in 0..128 {
                                        if y >= 64 - (engine.bands[x / 16] * 64.) as usize {
                                            bits[y * 128 + x] = 1;
                                        }
                                    }
                                }
                                bits
                            };
                            if let Ok(msg) = protocol::bitmap(&bits, 0x20) {
                                messages.push(msg);
                                bitmap_pending = Some(now);
                                if settings.mock {
                                    bitmap_pending = None;
                                }
                            }
                        } else {
                            let text = if display.widget == "clock" {
                                local.format("%H:%M:%S").to_string()
                            } else {
                                active_preset.id.clone()
                            };
                            if previous_widget != text {
                                messages = protocol::display_text(&text, 0x20).unwrap_or_default();
                                previous_widget = text;
                            }
                        }
                        for msg in messages {
                            let _ =
                                send(&mut **t, &msg, &mut status, &mut monitor, settings.midi_log);
                        }
                    }
                }
            }
        }
        if errors >= 3 {
            release(
                &mut transport,
                &desired.layout,
                &last_frame,
                settings,
                &mut status,
                &mut monitor,
                false,
            );
            forward.close();
            next_connect = now + 2.;
            errors = 0;
            warn(
                &mut status,
                "送信エラーが続いたため、接続を再試行しています",
            );
        }
        if now - last_status >= 0.5 {
            last_status = now;
            status.pads_port = forward.pads.is_some();
            status.controls_port = forward.controls.is_some();
            status.bpm = engine.bpm;
            status.monitor = monitor.iter().cloned().collect();
            core.control.lock().unwrap().status = status.clone();
            let _ = app.emit("device_status", &status);
            let _ = app.emit("piano_state", &core.piano.view());
        }
        if app
            .get_webview_window("main")
            .is_some_and(|w| w.is_visible().unwrap_or(false))
            && now - last_preview >= 1. / 30.
        {
            last_preview = now;
            let _ = app.emit("frame_preview", &frame);
            let _ = app.emit("input_state", &*core.input.lock().unwrap());
        }
        let remaining = Duration::from_secs_f64(1. / 60.).saturating_sub(tick.elapsed());
        if !remaining.is_zero() {
            std::thread::sleep(remaining);
        }
    }
}
fn warn(status: &mut Status, message: &str) {
    if !status.warnings.iter().any(|s| s == message) {
        if status.warnings.len() >= 8 {
            status.warnings.remove(0);
        }
        status.warnings.push(message.into());
    }
}
fn hex_bytes(b: &[u8]) -> String {
    b.iter()
        .take(128)
        .map(|v| format!("{v:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
fn log_midi(log: &mut VecDeque<String>, direction: &str, b: &[u8]) {
    if log.len() >= 100 {
        log.pop_front();
    }
    log.push_back(format!(
        "{} {} {}",
        chrono::Local::now().format("%H:%M:%S%.3f"),
        direction,
        hex_bytes(b)
    ));
}
fn send(
    t: &mut dyn LedTransport,
    b: &[u8],
    status: &mut Status,
    monitor: &mut VecDeque<String>,
    logging: bool,
) -> Result<(), String> {
    t.send_raw(b)?;
    status.messages += 1;
    status.bytes += b.len() as u64;
    if logging {
        log_midi(monitor, "→", b);
    }
    Ok(())
}
fn release(
    transport: &mut Option<Box<dyn LedTransport>>,
    layout: &DeviceLayout,
    frame: &[[u8; 3]],
    settings: &Settings,
    status: &mut Status,
    monitor: &mut VecDeque<String>,
    fade: bool,
) {
    if let Some(t) = transport {
        if fade {
            for step in (0..6).rev() {
                for (i, led) in layout.leds.iter().enumerate() {
                    let c = frame
                        .get(i)
                        .copied()
                        .unwrap_or([0; 3])
                        .map(|v| (v as u16 * step as u16 / 6) as u8);
                    let msg = if led.kind == "mono" {
                        protocol::mono(led, c[0])
                    } else {
                        protocol::rgb(led, c, status.pad_mode == 15)
                    };
                    if led.kind != "none" {
                        if let Ok(msg) = msg {
                            let _ = send(&mut **t, &msg, status, monitor, settings.midi_log);
                        }
                    }
                }
                let _ = t.flush();
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        for led in &layout.leds {
            let msg = if led.kind == "mono" {
                protocol::mono(led, 0)
            } else {
                protocol::palette(led, 0, 0, status.pad_mode == 15)
            };
            if led.kind != "none" {
                if let Ok(msg) = msg {
                    let _ = send(&mut **t, &msg, status, monitor, settings.midi_log);
                }
            }
        }
        for b in [[0xb6, DAW_DRUM, 0], DAW_OFF] {
            let _ = send(&mut **t, &b, status, monitor, settings.midi_log);
        }
    }
    *transport = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Record(Arc<Mutex<Vec<Vec<u8>>>>);
    impl LedTransport for Record {
        fn send_raw(&mut self, b: &[u8]) -> Result<(), String> {
            self.0.lock().unwrap().push(b.to_vec());
            Ok(())
        }
    }
    #[test]
    fn release_fades_full_brightness_without_overflow_and_exits_daw_mode() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut t: Option<Box<dyn LedTransport>> = Some(Box::new(Record(log.clone())));
        let layout = DeviceLayout::default();
        let frame = vec![[127; 3]; layout.leds.len()];
        release(
            &mut t,
            &layout,
            &frame,
            &Settings::default(),
            &mut Status::default(),
            &mut VecDeque::new(),
            true,
        );
        assert!(t.is_none());
        let log = log.lock().unwrap();
        assert_eq!(log.last().unwrap(), &DAW_OFF);
        assert_eq!(log[log.len() - 2], vec![0xb6, DAW_DRUM, 0]);
        assert!(log
            .iter()
            .filter(|b| b.len() == 13)
            .all(|b| b[9..12].iter().all(|&v| v <= 127)));
    }
}
