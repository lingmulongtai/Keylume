use super::{
    dispatch,
    runtime::{Action, Core},
};
use serde_json::json;
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};
/// Explicit local self-test. Uses isolated storage and MockDevice; never opens hardware.
pub fn start(core: Arc<Core>, report: PathBuf) {
    std::thread::spawn(move || {
        let mut checks = vec![];
        let wait = |expected: &str, seconds: u64| {
            let until = Instant::now() + Duration::from_secs(seconds);
            while Instant::now() < until {
                let status = core.control.lock().unwrap().status.clone();
                if status.connection == expected && (expected != "preview" || status.messages > 0) {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            false
        };
        let run = || -> Result<(), String> {
            let settings = crate::model::Settings {
                daw_processes: vec!["keylume-self-test-daw.exe".into()],
                setup_complete: true,
                ..Default::default()
            };
            dispatch(&core, "save_settings", json!({"settings":settings}))?;
            if !wait("preview", 10) {
                return Err("MockDevice did not connect".into());
            }
            std::thread::sleep(Duration::from_millis(700));
            checks.push("mock_connection_and_render");
            if core.control.lock().unwrap().status.messages == 0 {
                return Err("No frames were rendered".into());
            }
            dispatch(&core, "set_paused", json!({"paused":true}))?;
            if !wait("paused", 5) {
                return Err("Pause/release failed".into());
            }
            checks.push("pause_release");
            dispatch(&core, "set_paused", json!({"paused":false}))?;
            if !wait("preview", 5) {
                return Err("Resume failed".into());
            }
            checks.push("resume_reinitialize");
            dispatch(&core, "set_coexist_mode", json!({"mode":"handoff"}))?;
            core.action
                .send(Action::MockDaw(true))
                .map_err(|e| e.to_string())?;
            if !wait("handoff", 5) {
                return Err("DAW handoff failed".into());
            }
            checks.push("daw_handoff");
            core.action
                .send(Action::MockDaw(false))
                .map_err(|e| e.to_string())?;
            if !wait("preview", 8) {
                return Err("DAW exit recovery failed".into());
            }
            checks.push("daw_exit_recovery");
            core.action
                .send(Action::MockDisconnect(true))
                .map_err(|e| e.to_string())?;
            if !wait("disconnected", 5) {
                return Err("Disconnect failed".into());
            }
            core.action
                .send(Action::MockDisconnect(false))
                .map_err(|e| e.to_string())?;
            if !wait("preview", 5) {
                return Err("Reconnect failed".into());
            }
            checks.push("hotplug_reinitialize");
            let mut p = crate::model::builtin_presets().remove(0);
            p.id = "self-test-preset".into();
            p.name = "Self test".into();
            p.builtin = false;
            dispatch(&core, "save_preset", json!({"preset":p}))?;
            if !core
                .storage
                .lock()
                .unwrap()
                .root
                .join("presets/self-test-preset.json")
                .exists()
            {
                return Err("Persistence failed".into());
            }
            checks.push("native_preset_persistence");
            Ok(())
        };
        let mut run = run;
        let result = run();
        let status = core.view().ok().map(|v| v.status);
        let value = json!({"passed":result.is_ok(),"checks":checks,"error":result.err(),"status":status,"hardwareTested":false});
        if let Some(parent) = report.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&report, serde_json::to_vec_pretty(&value).unwrap());
        core.quitting.store(true, Ordering::SeqCst);
    });
}
