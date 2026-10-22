use blink_verdict::config::Config;
use windows::{
    core::PWSTR,
    Win32::{Foundation::ERROR_NOT_FOUND, Security::Credentials::*},
};

fn target(config: &Config) -> Vec<u16> {
    format!(
        "BlinkVerdict/{:?}/{}",
        config.provider,
        config.endpoint.trim().trim_end_matches('/')
    )
    .encode_utf16()
    .chain(Some(0))
    .collect()
}

pub fn read(config: &Config) -> Result<String, String> {
    unsafe {
        let name = target(config);
        let mut credential = std::ptr::null_mut();
        match CredReadW(
            windows::core::PCWSTR(name.as_ptr()),
            CRED_TYPE_GENERIC,
            0,
            &mut credential,
        ) {
            Ok(()) => {
                let value = &*credential;
                let text = if value.CredentialBlobSize == 0 {
                    Ok(String::new())
                } else {
                    String::from_utf8(
                        std::slice::from_raw_parts(
                            value.CredentialBlob,
                            value.CredentialBlobSize as usize,
                        )
                        .to_vec(),
                    )
                    .map_err(|_| "密钥编码无效".to_string())
                };
                CredFree(credential as *const _);
                text
            }
            Err(e) if e.code() == windows::core::HRESULT::from_win32(ERROR_NOT_FOUND.0) => {
                Ok(String::new())
            }
            Err(_) => Err("无法读取 Windows 凭据管理器".into()),
        }
    }
}

/// None leaves the existing key untouched; an empty string explicitly removes it.
pub fn write(config: &Config, key: &str) -> Result<(), String> {
    unsafe {
        let mut name = target(config);
        if key.is_empty() {
            return match CredDeleteW(windows::core::PCWSTR(name.as_ptr()), CRED_TYPE_GENERIC, 0) {
                Ok(()) => Ok(()),
                Err(e) if e.code() == windows::core::HRESULT::from_win32(ERROR_NOT_FOUND.0) => {
                    Ok(())
                }
                Err(_) => Err("无法删除保存的密钥".into()),
            };
        }
        let mut user: Vec<u16> = "API key\0".encode_utf16().collect();
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(name.as_mut_ptr()),
            CredentialBlobSize: key.len() as u32,
            CredentialBlob: key.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: PWSTR(user.as_mut_ptr()),
            ..Default::default()
        };
        CredWriteW(&credential, 0)
            .map_err(|_| "无法保存密钥到 Windows 凭据管理器（密钥长度上限为 2560 字节）".into())
    }
}
