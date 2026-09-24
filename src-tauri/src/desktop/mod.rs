mod audio;
mod controller_actions;
mod desktop_actions;
mod discovery;
mod performance;
mod piano;
mod runtime;
mod smoke;
mod sound_library;
mod system;
mod update_http;
mod updater;
use crate::{
    device::hardware::MidiPacket,
    model::*,
    storage::{import_preset, Storage},
};
use runtime::{Action, Core, StateView};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc},
};
use tauri::{
    menu::{Menu, MenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder,
};
use updater::Updater;

fn show(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    } else {
        let _ = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
            .title("Keylume")
            .inner_size(1280., 800.)
            .min_inner_size(1024., 680.)
            .build();
    }
}
#[tauri::command]
fn get_state(core: tauri::State<'_, Arc<Core>>) -> Result<StateView, String> {
    core.view()
}
#[tauri::command]
fn get_input_state(core: tauri::State<'_, Arc<Core>>) -> crate::device::input::InputState {
    core.input.lock().unwrap().clone()
}
#[tauri::command]
fn get_piano_state(core: tauri::State<'_, Arc<Core>>) -> piano::PianoStatus {
    core.piano.view()
}
#[tauri::command]
fn groove_command(
    name: String,
    args: Value,
    core: tauri::State<'_, Arc<Core>>,
) -> Result<Value, String> {
    use crate::groove::{LoopCommand, LoopConfig};
    let bus = &core.piano.bus;
    match name.as_str() {
        "state" => {}
        "drum" => {
            let pad = args["pad"]
                .as_u64()
                .filter(|p| *p < 16)
                .ok_or("パッドが範囲外です")?;
            bus.drum(pad as u8, 100);
        }
        "record" => {
            let config: LoopConfig = serde_json::from_value(args).map_err(|e| e.to_string())?;
            config.validate()?;
            bus.loop_command(LoopCommand::Record(config))?;
        }
        "configure" => {
            let config: LoopConfig = serde_json::from_value(args).map_err(|e| e.to_string())?;
            config.validate()?;
            bus.loop_command(LoopCommand::Configure(config))?;
        }
        "recordToggle" => bus.loop_command(LoopCommand::RecordToggle)?,
        "metronome" => bus.loop_command(LoopCommand::Metronome)?,
        "play" => bus.loop_command(LoopCommand::Play)?,
        "overdub" => bus.loop_command(LoopCommand::Overdub)?,
        "stop" => bus.loop_command(LoopCommand::Stop)?,
        "clear" => bus.loop_command(LoopCommand::Clear)?,
        "undo" => bus.loop_command(LoopCommand::Undo)?,
        _ => return Err("未対応のルーパー操作です".into()),
    }
    Ok(json!(bus.loop_status()))
}
#[tauri::command]
fn get_update_state(updater: tauri::State<'_, Arc<Updater>>) -> crate::updates::UpdateState {
    updater.view()
}
#[tauri::command]
fn check_updates(updater: tauri::State<'_, Arc<Updater>>) -> Result<(), String> {
    updater.request(true)
}
#[tauri::command]
fn dismiss_update(
    updater: tauri::State<'_, Arc<Updater>>,
    core: tauri::State<'_, Arc<Core>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    updater.dismiss(&core, &app)
}
#[tauri::command]
fn open_update(
    updater: tauri::State<'_, Arc<Updater>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    updater.open(&app)
}
#[tauri::command]
fn command(
    name: String,
    args: Value,
    core: tauri::State<'_, Arc<Core>>,
    updater: tauri::State<'_, Arc<Updater>>,
    app: tauri::AppHandle,
) -> Result<Value, String> {
    let value = dispatch(&core, &name, args)?;
    if name == "save_settings" || name == "patch_settings" {
        updater.settings_changed(&core, &app);
    }
    Ok(value)
}
fn dispatch(core: &Core, name: &str, args: Value) -> Result<Value, String> {
    let mut c = core.control.lock().map_err(|e| e.to_string())?;
    let text = |key: &str| {
        args.get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("Missing {key}"))
    };
    let action = |a: Action| {
        core.action
            .try_send(a)
            .map_err(|_| "操作が混み合っています。もう一度お試しください".to_string())
    };
    match name {
        "controller_learn" => {
            action(Action::ControllerLearn(
                args["enabled"].as_bool().unwrap_or(false),
            ))?;
            return Ok(Value::Null);
        }
        "list_presets" => return Ok(json!(c.presets)),
        "list_profiles" => return Ok(json!(c.profiles)),
        "get_layout" => return Ok(json!(c.layout)),
        "apply_preset" => {
            let id = text("id")?;
            let p = c
                .presets
                .iter()
                .find(|p| p.id == id)
                .ok_or("プリセットがありません")?
                .clone();
            c.settings.active_preset = p.id.clone();
            core.storage
                .lock()
                .unwrap()
                .save("settings.json", &c.settings)?;
            c.draft = p;
        }
        "update_preset" => {
            let p: Preset =
                serde_json::from_value(args["preset"].clone()).map_err(|e| e.to_string())?;
            p.validate()?;
            c.draft = p;
        }
        "update_layer" => {
            let layer: Layer =
                serde_json::from_value(args["layer"].clone()).map_err(|e| e.to_string())?;
            let mut p = c.draft.clone();
            let l = p
                .layers
                .iter_mut()
                .find(|l| l.id == layer.id)
                .ok_or("レイヤーがありません")?;
            *l = layer;
            p.validate()?;
            c.draft = p;
        }
        "save_preset" => {
            let mut p: Preset =
                serde_json::from_value(args["preset"].clone()).map_err(|e| e.to_string())?;
            p.builtin = false;
            p.validate()?;
            if c.presets.iter().any(|x| x.id == p.id && x.builtin) {
                return Err("同梱プリセットは複製して保存してください".into());
            }
            core.storage
                .lock()
                .unwrap()
                .save(&format!("presets/{}.json", p.id), &p)?;
            if let Some(old) = c.presets.iter_mut().find(|x| x.id == p.id) {
                *old = p.clone();
            } else {
                c.presets.push(p.clone());
            }
            c.settings.active_preset = p.id.clone();
            core.storage
                .lock()
                .unwrap()
                .save("settings.json", &c.settings)?;
            c.draft = p;
        }
        "delete_preset" => {
            let id = text("id")?;
            let p = c
                .presets
                .iter()
                .find(|p| p.id == id)
                .ok_or("プリセットがありません")?;
            if p.builtin {
                return Err("同梱プリセットは削除できません".into());
            }
            if c.profiles.iter().any(|p| p.preset_id == id) {
                return Err("先にこのプリセットを参照するプロファイルを変更してください".into());
            }
            core.storage
                .lock()
                .unwrap()
                .remove(&format!("presets/{id}.json"))?;
            c.presets.retain(|p| p.id != id);
            if c.draft.id == id {
                c.draft = c.presets[0].clone();
                c.settings.active_preset = c.draft.id.clone();
                core.storage
                    .lock()
                    .unwrap()
                    .save("settings.json", &c.settings)?;
            }
        }
        "import_preset" => {
            let mut p = import_preset(text("json")?)?;
            if c.presets.iter().any(|x| x.id == p.id) {
                p.id = format!("import-{}", chrono::Utc::now().timestamp_millis());
            }
            core.storage
                .lock()
                .unwrap()
                .save(&format!("presets/{}.json", p.id), &p)?;
            c.presets.push(p);
        }
        "export_preset" => {
            let id = args.get("id").and_then(Value::as_str);
            let p = id
                .and_then(|id| c.presets.iter().find(|p| p.id == id))
                .unwrap_or(&c.draft);
            return serde_json::to_value(p).map_err(|e| e.to_string());
        }
        "set_master_brightness" => {
            let v = args["value"].as_f64().ok_or("輝度が不正です")? as f32;
            if !(0.0..=1.0).contains(&v) {
                return Err("輝度は 0–100% です".into());
            }
            let mut s = c.settings.clone();
            s.master_brightness = v;
            core.storage.lock().unwrap().save("settings.json", &s)?;
            c.settings = s;
        }
        "set_paused" => c.paused = args["paused"].as_bool().ok_or("paused is required")?,
        "set_coexist_mode" => {
            let mode: CoexistMode =
                serde_json::from_value(args["mode"].clone()).map_err(|e| e.to_string())?;
            let mut s = c.settings.clone();
            s.coexist_mode = mode;
            core.storage.lock().unwrap().save("settings.json", &s)?;
            c.settings = s;
        }
        "save_settings" => {
            let s: Settings =
                serde_json::from_value(args["settings"].clone()).map_err(|e| e.to_string())?;
            s.validate()?;
            core.storage.lock().unwrap().save("settings.json", &s)?;
            c.settings = s;
        }
        "patch_settings" => {
            let next = c.settings.patched(args["patch"].clone())?;
            core.storage.lock().unwrap().save("settings.json", &next)?;
            c.settings = next;
        }
        "save_layout" => {
            let l: DeviceLayout =
                serde_json::from_value(args["layout"].clone()).map_err(|e| e.to_string())?;
            l.validate()?;
            core.storage
                .lock()
                .unwrap()
                .save("layouts/layout.json", &l)?;
            c.layout = l;
        }
        "save_profile" => {
            let p: Profile =
                serde_json::from_value(args["profile"].clone()).map_err(|e| e.to_string())?;
            p.validate()?;
            if !c.presets.iter().any(|x| x.id == p.preset_id) {
                return Err("プリセットがありません".into());
            }
            core.storage
                .lock()
                .unwrap()
                .save(&format!("profiles/{}.json", p.id), &p)?;
            if let Some(old) = c.profiles.iter_mut().find(|x| x.id == p.id) {
                *old = p;
            } else {
                c.profiles.push(p);
            }
        }
        "delete_profile" => {
            let id = text("id")?;
            if !safe_id(id) {
                return Err("Invalid ID".into());
            }
            core.storage
                .lock()
                .unwrap()
                .remove(&format!("profiles/{id}.json"))?;
            c.profiles.retain(|p| p.id != id);
        }
        "run_led_probe" => {
            if c.paused || !["preview", "connected"].contains(&c.status.connection.as_str()) {
                return Err("デバイスを接続してライティングを再開してください".into());
            }
            let variant = args["variant"].as_u64().unwrap_or(0);
            if variant > 2 {
                return Err("Invalid probe variant".into());
            }
            action(Action::Probe(text("id")?.to_string(), variant as u8))?;
        }
        "stop_led_probe" => action(Action::StopProbe)?,
        "answer_led_probe" => {
            let mut layout = c.layout.clone();
            let led = layout
                .leds
                .iter_mut()
                .find(|l| Some(l.id.as_str()) == args["id"].as_str())
                .ok_or("LED がありません")?;
            let kind = text("kind")?;
            if !["rgb", "mono", "none"].contains(&kind) {
                return Err("Invalid LED type".into());
            }
            led.kind = kind.into();
            led.verified = !c.settings.mock;
            if let Some(status) = args["monoStatus"].as_u64() {
                if status > 255 {
                    return Err("Invalid status".into());
                }
                led.address.mono_status = Some(status as u8);
            }
            layout.validate()?;
            core.storage
                .lock()
                .unwrap()
                .save("layouts/layout.json", &layout)?;
            c.layout = layout;
            action(Action::StopProbe)?;
        }
        "set_device_feature" => {
            let cc = args["cc"].as_u64().ok_or("Missing cc")?;
            let value = args["value"].as_u64().ok_or("Missing value")?;
            if ![0x6f, 0x70, 0x71, 0x72].contains(&cc) || value > 127 {
                return Err("この本体設定は変更できません".into());
            }
            if !["connected", "preview"].contains(&c.status.connection.as_str()) {
                return Err("デバイスを接続してください".into());
            }
            action(Action::Feature(cc as u8, value as u8))?;
        }
        "reconnect" => action(Action::Reconnect)?,
        "tap_tempo" => action(Action::Tap)?,
        "piano_input" => {
            let bytes: Vec<u8> =
                serde_json::from_value(args["bytes"].clone()).map_err(|e| e.to_string())?;
            if bytes.len() != 3
                || ![0x90, 0x80].contains(&bytes[0])
                || bytes[1] > 127
                || bytes[2] > 127
            {
                return Err("鍵盤入力が不正です".into());
            }
            core.piano.bus.midi(true, &bytes);
            if core.piano.bus.is_performing() {
                core.performance
                    .input("screen", &bytes, core.piano.bus.octave());
            }
            action(Action::Input(MidiPacket {
                source: "screen".into(),
                bytes,
                received_at: std::time::Instant::now(),
            }))
            .inspect_err(|_| {
                core.piano.bus.panic();
            })?;
        }
        "piano_panic" => {
            core.piano.bus.panic();
            action(Action::Panic)?;
        }
        "piano_screen_release" => {
            // Screen note-on/off and release share the same audio queue; UI snapshots are independent.
            core.piano.bus.midi(true, &[0, 0, 0]);
            core.performance.release("screen");
            action(Action::Input(MidiPacket {
                source: "screen".into(),
                bytes: vec![0xb0, 123, 0],
                received_at: std::time::Instant::now(),
            }))
            .inspect_err(|_| {
                core.piano.bus.panic();
            })?;
        }
        "simulate_input" => {
            if !c.settings.mock {
                return Err("シミュレーションはプレビュー専用です".into());
            }
            let bytes: Vec<u8> =
                serde_json::from_value(args["bytes"].clone()).map_err(|e| e.to_string())?;
            if bytes.len() > 128 {
                return Err("Message too long".into());
            }
            action(Action::Input(MidiPacket {
                source: args["source"].as_str().unwrap_or("daw").into(),
                bytes,
                received_at: std::time::Instant::now(),
            }))?;
        }
        "mock_daw" | "mock_disconnect" => {
            if !cfg!(debug_assertions) || !c.settings.mock {
                return Err("開発ビルドのプレビュー専用です".into());
            }
            let v = args["value"].as_bool().unwrap_or(false);
            action(if name == "mock_daw" {
                Action::MockDaw(v)
            } else {
                Action::MockDisconnect(v)
            })?;
        }
        _ => return Err(format!("Unknown command: {name}")),
    }
    if !matches!(
        name,
        "simulate_input"
            | "piano_input"
            | "piano_panic"
            | "piano_screen_release"
            | "tap_tempo"
            | "run_led_probe"
            | "stop_led_probe"
            | "mock_daw"
            | "mock_disconnect"
            | "set_device_feature"
            | "reconnect"
    ) {
        c.revision += 1;
    }
    Ok(Value::Null)
}
#[tauri::command]
fn save_export(path: String, contents: String) -> Result<(), String> {
    if contents.len() > 4_000_000 {
        return Err("ファイルが大きすぎます".into());
    }
    let path = PathBuf::from(path);
    if path.extension().and_then(|s| s.to_str()) != Some("json") {
        return Err("JSON ファイルとして保存してください".into());
    }
    std::fs::write(path, contents).map_err(|e| e.to_string())
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--tray"])
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            performance::stage_command,
            sound_library::library_command,
            groove_command,
            get_state,
            get_input_state,
            get_piano_state,
            command,
            save_export,
            get_update_state,
            check_updates,
            dismiss_update,
            open_update
        ])
        .setup(|app| {
            let smoke_report = std::env::args()
                .skip_while(|a| a != "--self-test")
                .nth(1)
                .map(PathBuf::from);
            let root = smoke_report
                .as_ref()
                .map(|p| p.with_extension("data"))
                .unwrap_or_else(|| {
                    std::env::var_os("APPDATA")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| {
                            app.path()
                                .app_config_dir()
                                .expect("configuration directory")
                        })
                        .join("Keylume")
                });
            let storage = Storage::new(root).map_err(std::io::Error::other)?;
            let (core, rx) = Core::create(storage);
            if smoke_report.is_some() {
                let mut control = core.control.lock().unwrap();
                control.settings = Settings::default();
                control.settings.daw_processes = vec!["keylume-self-test-daw.exe".into()];
                control.draft = builtin_presets().remove(0);
            }
            app.manage(core.clone());
            let (updater, update_rx) = Updater::create(&core);
            app.manage(updater.clone());
            let open = MenuItem::with_id(app, "open", "開く", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "一時停止 / 再開", true, None::<&str>)?;
            let recent = Submenu::new(app, "プリセット", true)?;
            for p in core.control.lock().unwrap().presets.iter().take(8) {
                recent.append(&MenuItem::with_id(
                    app,
                    format!("preset:{}", p.id),
                    &p.name,
                    true,
                    None::<&str>,
                )?)?;
            }
            let bright = Submenu::new(app, "輝度", true)?;
            for level in [100, 75, 50, 25, 0] {
                bright.append(&MenuItem::with_id(
                    app,
                    format!("brightness:{level}"),
                    format!("{level}%"),
                    true,
                    None::<&str>,
                )?)?;
            }
            let modes = Submenu::new(app, "共存モード", true)?;
            for (id, label) in [
                ("lightingFirst", "ライティング優先"),
                ("handoff", "DAW に譲る"),
            ] {
                modes.append(&MenuItem::with_id(
                    app,
                    format!("mode:{id}"),
                    label,
                    true,
                    None::<&str>,
                )?)?;
            }
            let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
            let stage_edit = MenuItem::with_id(
                app,
                "stage_edit",
                "演奏表示の位置合わせ / 操作を解除",
                true,
                None::<&str>,
            )?;
            let stage_lock = MenuItem::with_id(
                app,
                "stage_lock",
                "演奏表示の位置合わせを終了",
                true,
                None::<&str>,
            )?;
            let stage_hide =
                MenuItem::with_id(app, "stage_hide", "演奏表示を閉じる", true, None::<&str>)?;
            let updates =
                MenuItem::with_id(app, "updates", "アップデートを確認", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &open,
                    &recent,
                    &pause,
                    &bright,
                    &modes,
                    &stage_edit,
                    &stage_lock,
                    &stage_hide,
                    &updates,
                    &quit,
                ],
            )?;
            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or("Missing application icon")?;
            TrayIconBuilder::with_id("keylume")
                .icon(icon)
                .tooltip("Keylume · Light your sound")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show(tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| {
                    let core = app.state::<Arc<Core>>();
                    let id = event.id().as_ref();
                    let result = if id == "open" {
                        show(app);
                        Ok(Value::Null)
                    } else if id == "updates" {
                        app.state::<Arc<Updater>>()
                            .request(true)
                            .map(|_| Value::Null)
                    } else if id == "stage_edit" || id == "stage_lock" {
                        performance::interaction(app, &core, id == "stage_edit")
                            .map(|_| Value::Null)
                    } else if id == "stage_hide" {
                        performance::close_views(app);
                        Ok(Value::Null)
                    } else if id == "quit" {
                        core.quitting.store(true, Ordering::SeqCst);
                        Ok(Value::Null)
                    } else if id == "pause" {
                        let paused = core.control.lock().unwrap().paused;
                        dispatch(&core, "set_paused", json!({"paused":!paused}))
                    } else if let Some(id) = id.strip_prefix("preset:") {
                        dispatch(&core, "apply_preset", json!({"id":id}))
                    } else if let Some(b) = id.strip_prefix("brightness:") {
                        dispatch(
                            &core,
                            "set_master_brightness",
                            json!({"value":b.parse::<f32>().unwrap_or(100.)/100.}),
                        )
                    } else if let Some(mode) = id.strip_prefix("mode:") {
                        dispatch(&core, "set_coexist_mode", json!({"mode":mode}))
                    } else {
                        Ok(Value::Null)
                    };
                    if let Err(e) = result {
                        core.notify(app, &e);
                    }
                })
                .build(app)?;
            if std::env::args().any(|arg| arg == "--tray") {
                if let Some(w) = app.get_webview_window("main") {
                    w.close()?;
                }
            }
            system::listen();
            runtime::spawn(app.handle().clone(), core.clone(), rx);
            performance::spawn(app.handle().clone(), core.clone());
            if let Some(report) = smoke_report {
                smoke::start(core, report);
            } else {
                updater::spawn(app.handle().clone(), core, updater, update_rx);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main"
                && matches!(
                    event,
                    tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
                )
            {
                if let Some(core) = window.try_state::<Arc<Core>>() {
                    let _ = dispatch(&core, "piano_screen_release", Value::Null);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Unable to start Keylume")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                let core = app.state::<Arc<Core>>();
                if !core.terminated.load(Ordering::SeqCst) {
                    api.prevent_exit();
                }
            }
        });
}
