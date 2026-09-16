use super::{audio, runtime::Core, system};
use crate::device::hardware::{self, Ports};
use std::{
    sync::{atomic::Ordering, Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Default)]
pub struct Processes {
    pub running: Vec<String>,
    pub foreground: String,
}

#[derive(Clone, Default)]
pub struct Devices {
    pub ports: Ports,
    pub audio: Vec<String>,
}

/// Driver enumeration can take seconds. Never run it on the rendering thread.
pub struct Discovery {
    pub processes: Arc<Mutex<Option<Processes>>>,
    pub devices: Arc<Mutex<Option<Devices>>>,
}
impl Discovery {
    pub fn start(core: &Arc<Core>) -> Self {
        let processes = Arc::new(Mutex::new(None));
        let devices = Arc::new(Mutex::new(None));
        let latest = processes.clone();
        let owner = core.clone();
        std::thread::spawn(move || {
            let mut system_info = sysinfo::System::new();
            while !owner.quitting.load(Ordering::SeqCst) {
                system_info.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::All,
                    true,
                    sysinfo::ProcessRefreshKind::nothing(),
                );
                let snapshot = Processes {
                    running: system_info
                        .processes()
                        .values()
                        .map(|p| p.name().to_string_lossy().into_owned())
                        .collect(),
                    foreground: system_info
                        .process(sysinfo::Pid::from_u32(system::foreground_pid()))
                        .map(|p| p.name().to_string_lossy().into_owned())
                        .unwrap_or_default(),
                };
                *latest.lock().unwrap() = Some(snapshot);
                std::thread::sleep(Duration::from_secs(2));
            }
        });
        let latest = devices.clone();
        let owner = core.clone();
        std::thread::spawn(move || {
            let mut audio_devices = vec![];
            let mut audio_at: Option<Instant> = None;
            while !owner.quitting.load(Ordering::SeqCst) {
                let ports = hardware::ports().unwrap_or_default();
                *latest.lock().unwrap() = Some(Devices {
                    ports: ports.clone(),
                    audio: audio_devices.clone(),
                });
                if audio_at.is_none_or(|at| at.elapsed() >= Duration::from_secs(10)) {
                    audio_devices = audio::devices();
                    audio_at = Some(Instant::now());
                }
                *latest.lock().unwrap() = Some(Devices {
                    ports,
                    audio: audio_devices.clone(),
                });
                std::thread::sleep(Duration::from_secs(2));
            }
        });
        Self { processes, devices }
    }
}
