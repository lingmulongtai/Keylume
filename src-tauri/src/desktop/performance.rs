use super::runtime::Core;
use crate::performance::{PerformanceEngine, Song, StageSettings};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: usize,
    pub name: String,
    pub scale: f64,
    #[serde(flatten)]
    pub rect: Rect,
}
#[derive(Clone, Serialize)]
pub struct View {
    pub monitor: Monitor,
    pub desktop: Rect,
}
pub struct Performance {
    input_tx: crossbeam_channel::Sender<(String, Vec<u8>, i8, f64, u64)>,
    input_rx: crossbeam_channel::Receiver<(String, Vec<u8>, i8, f64, u64)>,
    overflow: AtomicBool,
    generation: [AtomicU64; 2],
    dirty: AtomicBool,
    window_generation: AtomicU64,
    windows_op: Mutex<()>,
    calibrating: AtomicBool,
    pub engine: Mutex<PerformanceEngine>,
    epoch: Instant,
    views: Mutex<HashMap<String, View>>,
}
impl Performance {
    pub fn new(settings: StageSettings) -> Arc<Self> {
        let (input_tx, input_rx) = crossbeam_channel::bounded(2048);
        Arc::new(Self {
            input_tx,
            input_rx,
            overflow: AtomicBool::new(false),
            generation: [AtomicU64::new(0), AtomicU64::new(0)],
            dirty: AtomicBool::new(false),
            window_generation: AtomicU64::new(0),
            windows_op: Mutex::new(()),
            calibrating: AtomicBool::new(false),
            engine: Mutex::new(PerformanceEngine::new(settings)),
            epoch: Instant::now(),
            views: Mutex::new(HashMap::new()),
        })
    }
    pub fn input(&self, source: &str, bytes: &[u8], octave: i8) {
        if self
            .input_tx
            .try_send((
                source.into(),
                bytes.to_vec(),
                octave,
                self.epoch.elapsed().as_secs_f64(),
                self.generation[usize::from(source == "screen")].load(Ordering::Acquire),
            ))
            .is_err()
        {
            self.overflow.store(true, Ordering::Release);
        }
    }
    pub fn release(&self, source: &str) {
        self.generation[usize::from(source == "screen")].fetch_add(1, Ordering::AcqRel);
        self.engine.lock().unwrap().release_source(source);
    }
    pub fn pause(&self) {
        for generation in &self.generation {
            generation.fetch_add(1, Ordering::AcqRel);
        }
        let mut e = self.engine.lock().unwrap();
        e.pause();
        e.release_source("keyboard");
        e.release_source("screen");
    }
}
pub fn spawn(app: tauri::AppHandle, core: Arc<Core>) {
    std::thread::spawn(move || {
        let mut next = Instant::now();
        let mut saved = Instant::now();
        while !core.quitting.load(Ordering::Acquire) {
            let snapshot = {
                let mut e = core.performance.engine.lock().unwrap();
                if core.performance.overflow.swap(false, Ordering::AcqRel) {
                    for _ in core.performance.input_rx.try_iter() {}
                    e.pause();
                    e.release_source("keyboard");
                    e.release_source("screen");
                }
                for (source, bytes, octave, at, generation) in
                    core.performance.input_rx.try_iter().take(2048)
                {
                    if generation
                        != core.performance.generation[usize::from(source == "screen")]
                            .load(Ordering::Acquire)
                    {
                        continue;
                    }
                    e.tick(at);
                    e.input(&source, &bytes, octave);
                }
                e.tick(core.performance.epoch.elapsed().as_secs_f64());
                if Instant::now() >= next {
                    next = Instant::now() + Duration::from_millis(33);
                    Some(e.snapshot())
                } else {
                    None
                }
            };
            if let Some(s) = snapshot {
                let _ = app.emit("stage_state", s);
            }
            if saved.elapsed() > Duration::from_millis(500)
                && core.performance.dirty.swap(false, Ordering::AcqRel)
            {
                let settings = core.performance.engine.lock().unwrap().settings.clone();
                if let Err(error) = core
                    .storage
                    .lock()
                    .unwrap()
                    .save("performance.json", &settings)
                {
                    let _ = app.emit("notice", format!("演奏設定の保存に失敗: {error}"));
                }
                saved = Instant::now();
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let settings = core.performance.engine.lock().unwrap().settings.clone();
        let _ = core
            .storage
            .lock()
            .unwrap()
            .save("performance.json", &settings);
    });
}
fn monitors(app: &tauri::AppHandle) -> Result<Vec<Monitor>, String> {
    Ok(app
        .available_monitors()
        .map_err(|e| e.to_string())?
        .iter()
        .enumerate()
        .map(|(id, m)| Monitor {
            id,
            name: m
                .name()
                .cloned()
                .unwrap_or_else(|| format!("Display {}", id + 1)),
            scale: m.scale_factor(),
            rect: Rect {
                x: m.position().x,
                y: m.position().y,
                width: m.size().width,
                height: m.size().height,
            },
        })
        .collect())
}
fn desktop(ms: &[Monitor]) -> Result<Rect, String> {
    let x = ms
        .iter()
        .map(|m| m.rect.x)
        .min()
        .ok_or("表示するモニターを選んでください")?;
    let y = ms.iter().map(|m| m.rect.y).min().unwrap();
    let right = ms
        .iter()
        .map(|m| m.rect.x as i64 + m.rect.width as i64)
        .max()
        .unwrap();
    let bottom = ms
        .iter()
        .map(|m| m.rect.y as i64 + m.rect.height as i64)
        .max()
        .unwrap();
    Ok(Rect {
        x,
        y,
        width: (right - x as i64) as u32,
        height: (bottom - y as i64) as u32,
    })
}
pub fn interaction(app: &tauri::AppHandle, core: &Core, editing: bool) -> Result<(), String> {
    core.performance
        .calibrating
        .store(editing, Ordering::Release);
    let through = core
        .performance
        .engine
        .lock()
        .unwrap()
        .settings
        .click_through
        && !editing;
    for (label, window) in app.webview_windows() {
        if label.starts_with("stage-") {
            window
                .set_ignore_cursor_events(through)
                .map_err(|e| e.to_string())?;
            if editing {
                let _ = window.set_focus();
            }
        }
    }
    let _ = app.emit("stage_interaction", editing);
    Ok(())
}
pub fn close_views(app: &tauri::AppHandle) {
    for (label, window) in app.webview_windows() {
        if label.starts_with("stage-") {
            let _ = window.close();
        }
    }
}
#[tauri::command]
pub async fn stage_command(
    name: String,
    args: Value,
    core: tauri::State<'_, Arc<Core>>,
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Value, String> {
    match name.as_str() {
        "interaction" => {
            if let Some(editing) = args["editing"].as_bool() {
                interaction(&app, &core, editing)?;
            }
            return Ok(json!(core.performance.calibrating.load(Ordering::Acquire)));
        }
        "monitors" => return Ok(json!(monitors(&app)?)),
        "view" => {
            return Ok(json!(core
                .performance
                .views
                .lock()
                .unwrap()
                .get(window.label())))
        }
        "open" => {
            let _guard = core.performance.windows_op.lock().unwrap();
            let generation = core
                .performance
                .window_generation
                .fetch_add(1, Ordering::AcqRel);
            let ids: Vec<usize> =
                serde_json::from_value(args["ids"].clone()).map_err(|e| e.to_string())?;
            let all = monitors(&app)?;
            let selected: Vec<_> = all.into_iter().filter(|m| ids.contains(&m.id)).collect();
            if selected.len() != ids.len() || selected.len() > 8 {
                return Err("モニターを選び直してください".into());
            }
            let bounds = desktop(&selected)?;
            let top = selected.iter().map(|m| m.rect.y).max().unwrap();
            let bottom = selected
                .iter()
                .map(|m| m.rect.y as i64 + m.rect.height as i64)
                .min()
                .unwrap() as f64;
            let kh = (bounds.height as f64 * 0.14).min(160.);
            let min = (top - bounds.y) as f64 + 40.;
            let max = bottom - bounds.y as f64 - kh - 20.;
            if min > max {
                return Err(
                    "鍵盤を連続表示するには、Windowsの画面設定でモニターの高さを揃えてください"
                        .into(),
                );
            }
            {
                let mut e = core.performance.engine.lock().unwrap();
                e.settings.line_y = e.settings.line_y.clamp(
                    (min / bounds.height as f64).clamp(0.25, 0.95),
                    (max / bounds.height as f64).clamp(0.25, 0.95),
                );
                core.performance.dirty.store(true, Ordering::Release);
            }
            for (label, w) in app.webview_windows() {
                if label.starts_with("stage-") {
                    w.close().map_err(|e| e.to_string())?;
                }
            }
            core.performance.views.lock().unwrap().clear();
            for m in selected {
                let label = format!("stage-{generation}-{}", m.id);
                core.performance.views.lock().unwrap().insert(
                    label.clone(),
                    View {
                        monitor: m.clone(),
                        desktop: bounds.clone(),
                    },
                );
                let w = WebviewWindowBuilder::new(
                    &app,
                    &label,
                    WebviewUrl::App("index.html?view=stage".into()),
                )
                .title("Keylume · 演奏画面")
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .always_on_top(true)
                .resizable(false)
                .visible(false)
                .build()
                .map_err(|e| e.to_string())?;
                w.set_position(PhysicalPosition::new(m.rect.x, m.rect.y))
                    .map_err(|e| e.to_string())?;
                w.set_size(PhysicalSize::new(m.rect.width, m.rect.height))
                    .map_err(|e| e.to_string())?;
                w.show().map_err(|e| e.to_string())?;
                if !core
                    .performance
                    .engine
                    .lock()
                    .unwrap()
                    .settings
                    .click_through
                {
                    let _ = w.set_focus();
                }
            }
            interaction(&app, &core, false)?;
            return Ok(Value::Null);
        }
        "close" => {
            let _guard = core.performance.windows_op.lock().unwrap();
            for (label, w) in app.webview_windows() {
                if label.starts_with("stage-") {
                    let _ = w.close();
                }
            }
            return Ok(Value::Null);
        }
        _ => {}
    }
    let mut e = core.performance.engine.lock().unwrap();
    e.tick(core.performance.epoch.elapsed().as_secs_f64());
    match name.as_str() {
        "state" => {}
        "song" => return Ok(json!(e.song)),
        "load" => {
            let song: Song =
                serde_json::from_value(args["song"].clone()).map_err(|e| e.to_string())?;
            e.load(song)?;
        }
        "settings" => {
            let mut next = e.settings.clone();
            let mut value = serde_json::to_value(&next).unwrap();
            let patch = args["patch"].as_object().ok_or("設定を読み込めません")?;
            for (k, v) in patch {
                value[k] = v.clone();
            }
            next = serde_json::from_value(value).map_err(|e| e.to_string())?;
            e.configure(next)?;
        }
        "play" => e.play()?,
        "pause" => e.pause(),
        "stop" => e.stop(),
        "seek" => e.seek(args["position"].as_f64().ok_or("時間が不正です")?)?,
        _ => return Err("未対応の演奏操作です".into()),
    }
    let state = e.snapshot();
    drop(e);
    if name == "settings" || name == "load" {
        core.performance.dirty.store(true, Ordering::Release);
    }
    if name == "settings" {
        interaction(
            &app,
            &core,
            core.performance.calibrating.load(Ordering::Acquire),
        )?;
    }
    let _ = app.emit("stage_state", &state);
    Ok(json!(state))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_and_mixed_dpi_monitors_share_physical_coordinates() {
        let ms = vec![
            Monitor {
                id: 0,
                name: "A".into(),
                scale: 1.5,
                rect: Rect {
                    x: -2560,
                    y: 0,
                    width: 2560,
                    height: 1440,
                },
            },
            Monitor {
                id: 1,
                name: "B".into(),
                scale: 1.,
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
            },
        ];
        let r = desktop(&ms).unwrap();
        assert_eq!((r.x, r.y, r.width, r.height), (-2560, 0, 4480, 1440));
        assert!(desktop(&[]).is_err());
    }
}
