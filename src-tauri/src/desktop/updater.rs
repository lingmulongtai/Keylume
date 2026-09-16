use super::{runtime::Core, update_http::fetch_releases};
use crate::updates::{select_release, UpdateHistory, UpdateState};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::{
    sync::{atomic::Ordering, Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_opener::OpenerExt;

pub struct Updater {
    state: Mutex<UpdateState>,
    request: Sender<()>,
    pending: Mutex<Pending>,
}
#[derive(Default)]
struct Pending {
    busy: bool,
    manual: bool,
}
impl Updater {
    pub fn create(core: &Core) -> (Arc<Self>, Receiver<()>) {
        let history: UpdateHistory = core
            .storage
            .lock()
            .unwrap()
            .load("update-state.json", |_| Ok(()))
            .unwrap_or_default();
        let (request, receiver) = bounded(1);
        (
            Arc::new(Self {
                state: Mutex::new(UpdateState::new(history)),
                request,
                pending: Mutex::new(Pending::default()),
            }),
            receiver,
        )
    }
    pub fn view(&self) -> UpdateState {
        self.state.lock().unwrap().clone()
    }
    pub fn request(&self, manual: bool) -> Result<(), String> {
        let mut pending = self.pending.lock().unwrap();
        pending.manual |= manual;
        if pending.busy {
            return Ok(());
        }
        pending.busy = true;
        if self.request.try_send(()).is_err() {
            *pending = Pending::default();
            return Err("更新確認を開始できませんでした".into());
        }
        Ok(())
    }
    fn finish_request(&self) -> bool {
        let mut pending = self.pending.lock().unwrap();
        let manual = pending.manual;
        *pending = Pending::default();
        manual
    }
    fn filter_channel(&self, include_previews: bool) {
        let mut state = self.state.lock().unwrap();
        if state
            .release
            .as_ref()
            .is_some_and(|r| r.prerelease && !include_previews)
        {
            state.release = None;
            if state.status == "available" {
                state.status = "idle".into();
            }
        }
    }
    pub fn settings_changed(&self, core: &Core, app: &tauri::AppHandle) {
        let include_previews = core.control.lock().unwrap().settings.include_prereleases;
        self.filter_channel(include_previews);
        self.publish(app);
    }
    fn publish(&self, app: &tauri::AppHandle) {
        let _ = app.emit("update_status", self.view());
    }
    fn save_history(&self, core: &Core) -> Result<(), String> {
        // Keep snapshot creation and persistence serialized with dismissal and worker updates.
        let state = self.state.lock().unwrap();
        core.storage
            .lock()
            .unwrap()
            .save("update-state.json", &state.history)
    }
    pub fn dismiss(&self, core: &Core, app: &tauri::AppHandle) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        state.history.dismissed_version = state.release.as_ref().map(|r| r.version.clone());
        drop(state);
        self.save_history(core)?;
        self.publish(app);
        Ok(())
    }
    pub fn open(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let release = self
            .view()
            .release
            .ok_or("新しいバージョンは見つかっていません")?;
        app.opener()
            .open_url(release.url, None::<&str>)
            .map_err(|e| e.to_string())
    }
}

pub fn spawn(
    app: tauri::AppHandle,
    core: Arc<Core>,
    updater: Arc<Updater>,
    receiver: Receiver<()>,
) {
    std::thread::spawn(move || {
        let mut next = Instant::now() + Duration::from_secs(3);
        let mut previous_preferences = None;
        while !core.quitting.load(Ordering::SeqCst) {
            let preferences = {
                let control = core.control.lock().unwrap();
                (
                    control.settings.check_for_updates,
                    control.settings.include_prereleases,
                )
            };
            if previous_preferences.is_some_and(|p| p != preferences) {
                next = Instant::now();
                updater.filter_channel(preferences.1);
                updater.publish(&app);
            }
            previous_preferences = Some(preferences);
            if preferences.0 && Instant::now() >= next {
                let _ = updater.request(false);
            }
            let Ok(()) = receiver.recv_timeout(Duration::from_secs(1)) else {
                continue;
            };
            {
                let mut state = updater.state.lock().unwrap();
                state.status = "checking".into();
                state.error = None;
            }
            updater.publish(&app);
            let result = fetch_releases()
                .and_then(|body| select_release(&body, env!("CARGO_PKG_VERSION"), preferences.1));
            next =
                Instant::now() + Duration::from_secs(if result.is_ok() { 12 * 3600 } else { 3600 });
            let current_preferences = {
                let control = core.control.lock().unwrap();
                (
                    control.settings.check_for_updates,
                    control.settings.include_prereleases,
                )
            };
            if current_preferences != preferences {
                updater.state.lock().unwrap().status = "idle".into();
                updater.filter_channel(current_preferences.1);
                let manual = updater.finish_request();
                if manual {
                    let _ = updater.request(true);
                }
                updater.publish(&app);
                continue;
            }
            let manual = updater.finish_request();
            {
                let mut state = updater.state.lock().unwrap();
                state.history.checked_at = Some(chrono::Utc::now().to_rfc3339());
                match result {
                    Ok(release) => {
                        state.status = if release.is_some() {
                            "available"
                        } else {
                            "current"
                        }
                        .into();
                        state.release = release;
                        if manual {
                            state.history.dismissed_version = None;
                        }
                    }
                    Err(error) => {
                        state.status = "error".into();
                        state.error = Some(error);
                    }
                }
            }
            if let Err(error) = updater.save_history(&core) {
                core.storage
                    .lock()
                    .unwrap()
                    .log(&format!("Update history: {error}"));
            }
            updater.publish(&app);
            let visible = app.get_webview_window("main").is_some_and(|w| {
                w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false)
            });
            if core.quitting.load(Ordering::SeqCst) {
                break;
            }
            if visible && manual {
                let state = updater.view();
                let message = state.error.unwrap_or_else(|| {
                    state
                        .release
                        .map(|r| format!("Keylume v{} が利用できます", r.version))
                        .unwrap_or_else(|| "新しいバージョンはありません。".into())
                });
                core.notify(&app, &message);
            }
            if !visible {
                let state = updater.view();
                if let Some(release) = state.release.filter(|r| {
                    state.status == "available" && (manual || state.history.should_notify(r))
                }) {
                    updater.state.lock().unwrap().history.notified_version =
                        Some(release.version.clone());
                    let _ = updater.save_history(&core);
                    let handle = app.clone();
                    let updates = updater.clone();
                    let shared_core = core.clone();
                    app.dialog().message(format!("Keylume v{} が利用できます。リリースページから Windows 版をダウンロードして更新してください。", release.version))
                        .title("Keylume のアップデート")
                        .buttons(MessageDialogButtons::OkCancelCustom("更新ページを開く".into(), "あとで".into()))
                        .show(move |accepted| {
                            if accepted {
                                if let Err(error) = handle.opener().open_url(release.url, None::<&str>) {
                                    handle.dialog().message(format!("更新ページを開けませんでした: {error}")).title("Keylume").show(|_| {});
                                }
                            } else {
                                updates.state.lock().unwrap().history.dismissed_version = Some(release.version);
                                let _ = updates.save_history(&shared_core);
                                updates.publish(&handle);
                            }
                        });
                } else if manual {
                    app.dialog()
                        .message(
                            state
                                .error
                                .unwrap_or_else(|| "新しいバージョンはありません。".into()),
                        )
                        .title("Keylume のアップデート")
                        .show(|_| {});
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_requests_are_coalesced() {
        let (tx, rx) = bounded(1);
        let updater = Updater {
            state: Mutex::new(UpdateState::new(UpdateHistory::default())),
            request: tx,
            pending: Mutex::new(Pending::default()),
        };
        updater.request(false).unwrap();
        updater.request(true).unwrap();
        assert_eq!(rx.len(), 1);
        rx.recv().unwrap();
        assert!(updater.finish_request());
        updater.request(false).unwrap();
        rx.recv().unwrap();
        assert!(!updater.finish_request());
    }
    #[test]
    fn disabling_previews_invalidates_an_existing_candidate() {
        let (tx, _) = bounded(1);
        let mut state = UpdateState::new(UpdateHistory::default());
        state.status = "available".into();
        state.release = Some(crate::updates::Release {
            version: "0.3.0".into(),
            tag: "v0.3.0".into(),
            url: "https://github.com/lingmulongtai/Keylume/releases/tag/v0.3.0".into(),
            prerelease: true,
        });
        let updater = Updater {
            state: Mutex::new(state),
            request: tx,
            pending: Mutex::new(Pending::default()),
        };
        updater.filter_channel(false);
        assert!(updater.view().release.is_none());
        assert_eq!(updater.view().status, "idle");
    }
}
