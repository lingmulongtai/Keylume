use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
pub static RESUME_GENERATION: AtomicU64 = AtomicU64::new(0);
pub static SUSPENDED: AtomicBool = AtomicBool::new(false);
pub static LOCKED: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
pub fn midi_service() -> String {
    unsafe {
        use windows_sys::Win32::System::Services::*;
        let manager = OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_CONNECT);
        if manager.is_null() {
            return "unknown".into();
        }
        let name: Vec<u16> = "MidiSrv\0".encode_utf16().collect();
        let service = OpenServiceW(manager, name.as_ptr(), SERVICE_QUERY_STATUS);
        if service.is_null() {
            CloseServiceHandle(manager);
            return "unavailable".into();
        }
        let mut status: SERVICE_STATUS = std::mem::zeroed();
        let ok = QueryServiceStatus(service, &mut status) != 0;
        CloseServiceHandle(service);
        CloseServiceHandle(manager);
        if ok && status.dwCurrentState == SERVICE_RUNNING {
            "running".into()
        } else {
            "stopped".into()
        }
    }
}
#[cfg(not(windows))]
pub fn midi_service() -> String {
    "notWindows".into()
}
#[cfg(windows)]
pub fn idle_minutes() -> u32 {
    unsafe {
        use windows_sys::Win32::{
            System::SystemInformation::GetTickCount,
            UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
        };
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut info) != 0 {
            GetTickCount().wrapping_sub(info.dwTime) / 60000
        } else {
            0
        }
    }
}
#[cfg(not(windows))]
pub fn idle_minutes() -> u32 {
    0
}
#[cfg(windows)]
pub fn foreground_pid() -> u32 {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        let mut pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &mut pid);
        pid
    }
}
#[cfg(not(windows))]
pub fn foreground_pid() -> u32 {
    0
}
#[cfg(windows)]
pub fn listen() {
    std::thread::spawn(|| unsafe {
        use windows_sys::Win32::{
            Foundation::*,
            System::{LibraryLoader::GetModuleHandleW, RemoteDesktop::*},
            UI::WindowsAndMessaging::*,
        };
        unsafe extern "system" fn wnd(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
            match msg {
                WM_POWERBROADCAST => {
                    if w == 4 {
                        SUSPENDED.store(true, Ordering::SeqCst);
                    } else if w == 18 || w == 7 {
                        SUSPENDED.store(false, Ordering::SeqCst);
                        RESUME_GENERATION.fetch_add(1, Ordering::SeqCst);
                    }
                    1
                }
                WM_WTSSESSION_CHANGE => {
                    if w == WTS_SESSION_LOCK as usize {
                        LOCKED.store(true, Ordering::SeqCst);
                    } else if w == WTS_SESSION_UNLOCK as usize {
                        LOCKED.store(false, Ordering::SeqCst);
                    }
                    0
                }
                _ => DefWindowProcW(hwnd, msg, w, l),
            }
        }
        let class: Vec<u16> = "KeylumePowerMonitor\0".encode_utf16().collect();
        let instance = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);
        // A hidden top-level window receives broadcast power events even with no WebView.
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            class.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if !hwnd.is_null() {
            WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
}
#[cfg(not(windows))]
pub fn listen() {}
