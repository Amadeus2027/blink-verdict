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

/// Coordinate settings persistence with the credential stored for the new endpoint.
/// No credential access is needed when the user has not edited the key.
pub fn save_config(
    config: &Config,
    key: Option<&str>,
    path: &std::path::Path,
) -> Result<(), String> {
    let Some(key) = key else {
        return config.save(path);
    };
    let previous = read(config)?;
    write(config, key)?;
    if let Err(error) = config.save(path) {
        if let Err(rollback) = write(config, &previous) {
            return Err(format!(
                "{error}\n旧密钥恢复失败：{rollback}。请在 Windows 凭据管理器中核对该服务的密钥。"
            ));
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::windows::fs::OpenOptionsExt,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    #[ignore = "targeted Windows regression: real credential rollback against a locked config"]
    fn settings_regression_failed_save_restores_credentials() {
        struct Fixture {
            config: Config,
            alternate: Config,
            directory: PathBuf,
        }
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = write(&self.config, "");
                let _ = write(&self.alternate, "");
                let _ = fs::remove_dir_all(&self.directory);
            }
        }
        let id = format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let config = Config {
            endpoint: format!("https://settings-regression.invalid/{id}"),
            ..Config::default()
        };
        let alternate = Config {
            endpoint: format!("https://settings-regression.invalid/{id}/alternate"),
            ..config.clone()
        };
        let fixture = Fixture {
            config,
            alternate,
            directory: std::env::temp_dir().join(format!("blink-settings-{id}")),
        };
        let path = fixture.directory.join("config.json");
        fixture.config.save(&path).unwrap();
        let original = fs::read(&path).unwrap();
        assert_eq!(read(&fixture.config).unwrap(), "");
        write(&fixture.config, "old-test-key").unwrap();
        write(&fixture.alternate, "alternate-old-test-key").unwrap();
        let changed = Config {
            prompt: "Changed task".into(),
            ..fixture.config.clone()
        };
        // Deny deletion/replacement while allowing the old config to be read.
        let locked = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        for pending in [Some("replacement-test-key"), Some(""), None] {
            assert!(save_config(&changed, pending, &path).is_err());
            assert_eq!(read(&fixture.config).unwrap(), "old-test-key");
            assert_eq!(fs::read(&path).unwrap(), original);
        }
        assert!(save_config(&fixture.alternate, Some("new-alternate-key"), &path).is_err());
        assert_eq!(read(&fixture.alternate).unwrap(), "alternate-old-test-key");
        assert_eq!(read(&fixture.config).unwrap(), "old-test-key");
        write(&fixture.config, "").unwrap();
        assert!(save_config(&changed, Some("new-test-key"), &path).is_err());
        assert_eq!(read(&fixture.config).unwrap(), "");
        assert_eq!(fs::read(&path).unwrap(), original);
        drop(locked);
        save_config(&changed, Some("replacement-test-key"), &path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), changed);
        assert_eq!(read(&fixture.config).unwrap(), "replacement-test-key");
        save_config(&changed, Some(""), &path).unwrap();
        assert_eq!(read(&fixture.config).unwrap(), "");
    }
}
