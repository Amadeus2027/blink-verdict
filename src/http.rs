//! Synchronous native transport, used only inside the short-lived decision worker.
use crate::endpoint::Endpoint;
use std::{
    ffi::c_void,
    time::{Duration, Instant},
};
use windows::{
    core::{w, PCWSTR},
    Win32::Networking::WinHttp::*,
};

struct Handle(*mut c_void);
impl Handle {
    fn new(raw: *mut c_void) -> Result<Self, String> {
        if raw.is_null() {
            Err(last_error())
        } else {
            Ok(Self(raw))
        }
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = WinHttpCloseHandle(self.0);
        }
    }
}
fn last_error() -> String {
    native_error(windows::core::Error::from_win32())
}
fn native_error(error: windows::core::Error) -> String {
    if error.code() == windows::core::HRESULT::from_win32(ERROR_WINHTTP_TIMEOUT) {
        "模型请求超时".into()
    } else {
        "模型连接失败，请检查网络、地址与证书".into()
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
unsafe fn timeouts(handle: &Handle, deadline: Instant) -> Result<(), String> {
    let millis = deadline
        .saturating_duration_since(Instant::now())
        .as_millis()
        .min(i32::MAX as u128) as i32;
    if millis == 0 {
        return Err("模型请求超时".into());
    }
    WinHttpSetTimeouts(handle.0, millis.min(8000), millis.min(8000), millis, millis)
        .map_err(native_error)
}

pub(crate) fn post(url: &str, headers: &str, body: &[u8], seconds: u32) -> Result<Vec<u8>, String> {
    let endpoint = Endpoint::parse(url)?;
    let length = u32::try_from(body.len()).map_err(|_| "请求过大".to_string())?;
    let deadline = Instant::now() + Duration::from_secs(seconds.into());
    unsafe {
        // Loopback never enters Windows' proxy machinery. No resident session/pool/thread remains.
        let session = Handle::new(WinHttpOpen(
            w!("BlinkVerdict/0.1"),
            if endpoint.loopback {
                WINHTTP_ACCESS_TYPE_NO_PROXY
            } else {
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY
            },
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        ))?;
        timeouts(&session, deadline)?;
        let host = wide(endpoint.host);
        let connection = Handle::new(WinHttpConnect(
            session.0,
            PCWSTR(host.as_ptr()),
            endpoint.port,
            0,
        ))?;
        let path = wide(endpoint.path);
        let request = Handle::new(WinHttpOpenRequest(
            connection.0,
            w!("POST"),
            PCWSTR(path.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            if endpoint.secure {
                WINHTTP_FLAG_SECURE
            } else {
                WINHTTP_OPEN_REQUEST_FLAGS(0)
            },
        ))?;
        let disabled =
            WINHTTP_DISABLE_REDIRECTS | WINHTTP_DISABLE_COOKIES | WINHTTP_DISABLE_AUTHENTICATION;
        WinHttpSetOption(
            Some(request.0),
            WINHTTP_OPTION_DISABLE_FEATURE,
            Some(&disabled.to_ne_bytes()),
        )
        .map_err(native_error)?;
        let headers = headers.encode_utf16().collect::<Vec<_>>();
        timeouts(&request, deadline)?;
        WinHttpSendRequest(
            request.0,
            Some(&headers),
            Some(body.as_ptr().cast()),
            length,
            length,
            0,
        )
        .map_err(native_error)?;
        timeouts(&request, deadline)?;
        WinHttpReceiveResponse(request.0, std::ptr::null_mut()).map_err(native_error)?;
        let mut status = 0u32;
        let mut size = 4u32;
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some((&mut status as *mut u32).cast()),
            &mut size,
            std::ptr::null_mut(),
        )
        .map_err(native_error)?;
        if !(200..300).contains(&status) {
            return Err(format!(
                "模型服务返回 HTTP {status}；请检查地址、密钥、模型名称与额度"
            ));
        }
        let mut bytes = Vec::new();
        loop {
            timeouts(&request, deadline)?;
            let mut chunk = [0u8; 8192];
            let mut count = 0;
            WinHttpReadData(
                request.0,
                chunk.as_mut_ptr().cast(),
                chunk.len() as u32,
                &mut count,
            )
            .map_err(native_error)?;
            if count == 0 {
                break;
            }
            if bytes.len() + count as usize > 1_048_576 {
                return Err("模型响应过大".into());
            }
            bytes.extend_from_slice(&chunk[..count as usize]);
        }
        Ok(bytes)
    }
}
