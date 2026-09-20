use crate::controller::{shortcut, valid_target, Binding};
use crossbeam_channel::{bounded, Sender};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::Emitter;

struct Shared {
    allowed: AtomicBool,
    epoch: AtomicU64,
    rate: AtomicU32,
    quit: AtomicBool,
}
pub struct DesktopActions {
    shared: Arc<Shared>,
    tx: Sender<(u64, Binding)>,
}
impl DesktopActions {
    pub fn new(app: tauri::AppHandle) -> Self {
        let shared = Arc::new(Shared {
            allowed: AtomicBool::new(false),
            epoch: AtomicU64::new(0),
            rate: AtomicU32::new(0),
            quit: AtomicBool::new(false),
        });
        let state = shared.clone();
        let (tx, rx) = bounded::<(u64, Binding)>(32);
        std::thread::spawn(move || {
            let mut last = Instant::now();
            let mut remainder = 0.;
            while !state.quit.load(Ordering::Acquire) {
                let job = rx.recv_timeout(Duration::from_millis(16)).ok();
                let dt = last.elapsed().as_secs_f32().min(0.05);
                last = Instant::now();
                if !state.allowed.load(Ordering::Acquire)
                    || super::system::LOCKED.load(Ordering::Acquire)
                    || super::system::SUSPENDED.load(Ordering::Acquire)
                {
                    remainder = 0.;
                    continue;
                }
                if let Some((epoch, binding)) =
                    job.filter(|(e, _)| *e == state.epoch.load(Ordering::Acquire))
                {
                    if epoch != state.epoch.load(Ordering::Acquire) {
                        continue;
                    }
                    let result = match binding.action.as_str() {
                        "shortcut" => shortcut(&binding.value).and_then(|keys| send_keys(&keys)),
                        "open" => open_target(&app, &binding.value),
                        _ => Ok(()),
                    };
                    if let Err(error) = result {
                        let _ = app.emit("notice", error);
                    }
                }
                let rate = f32::from_bits(state.rate.load(Ordering::Relaxed));
                if rate == 0. {
                    remainder = 0.;
                    continue;
                }
                remainder += rate * dt;
                let wheel = remainder.trunc() as i32;
                if wheel != 0 {
                    remainder -= wheel as f32;
                    if let Err(error) = send_wheel(wheel) {
                        state.rate.store(0, Ordering::Release);
                        let _ = app.emit("notice", error);
                    }
                }
            }
        });
        Self { shared, tx }
    }
    pub fn allowed(&self, allowed: bool) {
        if self.shared.allowed.swap(allowed, Ordering::AcqRel) != allowed || !allowed {
            self.stop_scroll();
            if !allowed {
                self.shared.epoch.fetch_add(1, Ordering::AcqRel);
            }
        }
    }
    pub fn run(&self, binding: Binding) -> Result<(), String> {
        self.tx
            .try_send((self.shared.epoch.load(Ordering::Acquire), binding))
            .map_err(|_| "デスクトップ操作が混み合っています".into())
    }
    pub fn scroll(&self, rate: f32) {
        self.shared.rate.store(rate.to_bits(), Ordering::Release);
    }
    pub fn stop_scroll(&self) {
        self.shared.rate.store(0, Ordering::Release);
    }
}
impl Drop for DesktopActions {
    fn drop(&mut self) {
        self.shared.quit.store(true, Ordering::Release);
        self.allowed(false);
    }
}
fn open_target(app: &tauri::AppHandle, target: &str) -> Result<(), String> {
    if !valid_target(target) {
        return Err("起動先はhttp(s) URLまたは実行ファイルの絶対パスで指定してください".into());
    }
    if target.starts_with("http://") || target.starts_with("https://") {
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_url(target, None::<&str>)
            .map_err(|e| e.to_string())
    } else {
        let mut command = std::process::Command::new(target);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        command.spawn().map(|_| ()).map_err(|e| e.to_string())
    }
}
#[cfg(windows)]
fn send_keys(keys: &[u16]) -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    // Preserve keys already held by the person using the computer.
    let keys: Vec<u16> = keys
        .iter()
        .copied()
        .filter(|key| unsafe { GetAsyncKeyState(*key as i32) } >= 0)
        .collect();
    let make = |key: u16, release: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if release { KEYEVENTF_KEYUP } else { 0 }
                    | if [
                        0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x2d, 0x2e, 0x5b,
                    ]
                    .contains(&key)
                    {
                        KEYEVENTF_EXTENDEDKEY
                    } else {
                        0
                    },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let input: Vec<_> = keys
        .iter()
        .map(|k| make(*k, false))
        .chain(keys.iter().rev().map(|k| make(*k, true)))
        .collect();
    if input.is_empty() {
        return Ok(());
    }
    let sent = unsafe {
        SendInput(
            input.len() as u32,
            input.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if sent != input.len() as u32 {
        let release: Vec<_> = keys.iter().rev().map(|k| make(*k, true)).collect();
        unsafe {
            SendInput(
                release.len() as u32,
                release.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            )
        };
        return Err("Windowsがキー操作を受け付けませんでした。管理者権限のアプリは操作できない場合があります".into());
    }
    Ok(())
}
#[cfg(windows)]
fn send_wheel(delta: i32) -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: delta as u32,
                dwFlags: MOUSEEVENTF_WHEEL,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    if unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) } == 1 {
        Ok(())
    } else {
        Err("Windowsがスクロール操作を受け付けませんでした".into())
    }
}
#[cfg(not(windows))]
fn send_keys(_: &[u16]) -> Result<(), String> {
    Err("Windowsで利用できます".into())
}
#[cfg(not(windows))]
fn send_wheel(_: i32) -> Result<(), String> {
    Err("Windowsで利用できます".into())
}
