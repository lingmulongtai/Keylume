#[cfg(windows)]
pub fn fetch_releases() -> Result<Vec<u8>, String> {
    use std::{
        ffi::c_void,
        ptr::{null, null_mut},
        time::{Duration, Instant},
    };
    use windows_sys::Win32::Networking::WinHttp::*;
    struct Handle(*mut c_void);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                WinHttpCloseHandle(self.0);
            }
        }
    }
    fn handle(value: *mut c_void) -> Result<Handle, String> {
        if value.is_null() {
            Err(format!(
                "更新サーバーへ接続できません: {}",
                std::io::Error::last_os_error()
            ))
        } else {
            Ok(Handle(value))
        }
    }
    fn checked(value: i32) -> Result<(), String> {
        if value == 0 {
            Err(format!(
                "更新の通信に失敗しました: {}",
                std::io::Error::last_os_error()
            ))
        } else {
            Ok(())
        }
    }
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    // All handles remain on this worker thread. WinHTTP retains normal certificate validation.
    unsafe {
        let agent = wide(concat!("Keylume/", env!("CARGO_PKG_VERSION")));
        let session = handle(WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            null(),
            null(),
            0,
        ))?;
        checked(WinHttpSetTimeouts(session.0, 5000, 5000, 5000, 5000))?;
        let protocols = WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_2 | WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_3;
        checked(WinHttpSetOption(
            session.0,
            WINHTTP_OPTION_SECURE_PROTOCOLS,
            &protocols as *const _ as *const c_void,
            4,
        ))?;
        let host = wide("api.github.com");
        let connection = handle(WinHttpConnect(
            session.0,
            host.as_ptr(),
            INTERNET_DEFAULT_HTTPS_PORT,
            0,
        ))?;
        let get = wide("GET");
        let path = wide("/repos/lingmulongtai/Keylume/releases?per_page=100");
        let request = handle(WinHttpOpenRequest(
            connection.0,
            get.as_ptr(),
            path.as_ptr(),
            null(),
            null(),
            null(),
            WINHTTP_FLAG_SECURE,
        ))?;
        let disabled =
            WINHTTP_DISABLE_AUTHENTICATION | WINHTTP_DISABLE_COOKIES | WINHTTP_DISABLE_REDIRECTS;
        checked(WinHttpSetOption(
            request.0,
            WINHTTP_OPTION_DISABLE_FEATURE,
            &disabled as *const _ as *const c_void,
            4,
        ))?;
        let header_limit: u32 = 64 * 1024;
        checked(WinHttpSetOption(
            request.0,
            WINHTTP_OPTION_MAX_RESPONSE_HEADER_SIZE,
            &header_limit as *const _ as *const c_void,
            4,
        ))?;
        let headers =
            wide("Accept: application/vnd.github+json\r\nX-GitHub-Api-Version: 2022-11-28\r\n");
        let started = Instant::now();
        checked(WinHttpSendRequest(
            request.0,
            headers.as_ptr(),
            (headers.len() - 1) as u32,
            null(),
            0,
            0,
            0,
        ))?;
        checked(WinHttpReceiveResponse(request.0, null_mut()))?;
        let mut status: u32 = 0;
        let mut size = 4;
        checked(WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            null(),
            &mut status as *mut _ as *mut c_void,
            &mut size,
            null_mut(),
        ))?;
        if status != 200 {
            return Err(format!(
                "更新を確認できませんでした（HTTP {status}）。時間をおいて再試行してください"
            ));
        }
        let mut body = Vec::new();
        let mut buffer = [0u8; 8192];
        loop {
            if started.elapsed() > Duration::from_secs(30) {
                return Err("更新の通信がタイムアウトしました".into());
            }
            let mut read = 0;
            checked(WinHttpReadData(
                request.0,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut read,
            ))?;
            if read == 0 {
                break;
            }
            if body.len() + read as usize > 2 * 1024 * 1024 {
                return Err("更新情報が大きすぎます".into());
            }
            body.extend_from_slice(&buffer[..read as usize]);
        }
        Ok(body)
    }
}

#[cfg(not(windows))]
pub fn fetch_releases() -> Result<Vec<u8>, String> {
    Err("更新確認は Windows 版で利用できます".into())
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    #[ignore = "Explicit live GitHub network verification"]
    fn github_release_endpoint_responds_over_verified_https() {
        let body = super::fetch_releases().unwrap();
        let release = crate::updates::select_release(&body, "0.0.0", true)
            .unwrap()
            .expect("published Windows release");
        assert!(release
            .url
            .starts_with("https://github.com/lingmulongtai/Keylume/releases/tag/v"));
    }
}
