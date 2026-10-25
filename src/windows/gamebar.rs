//! Opt-in AppService IPC. No screenshot, prompt, model name or credential crosses this bridge.
use blink_verdict::config::{Config, DisplayMode};
use std::time::{Duration, Instant};
use windows::{
    core::{Result, RuntimeType, HSTRING},
    ApplicationModel::AppService::{
        AppServiceConnection, AppServiceConnectionStatus, AppServiceResponseStatus,
    },
    Foundation::{
        AsyncStatus, Collections::ValueSet, IAsyncOperation, IPropertyValue, PropertyValue,
    },
    Win32::Foundation::RECT,
};

pub(super) fn wait<T: RuntimeType + 'static>(
    operation: IAsyncOperation<T>,
    timeout: Duration,
) -> Result<T> {
    let start = Instant::now();
    while operation.Status()? == AsyncStatus::Started {
        if start.elapsed() > timeout {
            let _ = operation.Cancel();
            return Err(windows::core::Error::from(windows::core::HRESULT(
                0x800705B4u32 as i32,
            )));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    operation.GetResults()
}

pub struct Bridge(AppServiceConnection, i64);

impl Bridge {
    pub fn connect(family: &str, operation: i64) -> std::result::Result<Self, String> {
        (|| -> Result<Self> {
            let connection = AppServiceConnection::new()?;
            connection.SetAppServiceName(&HSTRING::from("BlinkVerdictBridge"))?;
            connection.SetPackageFamilyName(&HSTRING::from(family))?;
            if wait(connection.OpenAsync()?, Duration::from_secs(3))?
                != AppServiceConnectionStatus::Success
            {
                return Err(windows::core::Error::from(windows::core::HRESULT(
                    0x80004005u32 as i32,
                )));
            }
            // Probe without a payload: older components must not apply an
            // unsequenced hide/show before the host discovers the mismatch.
            let probe = ValueSet::new()?;
            probe.Insert(&HSTRING::from("protocol"), &PropertyValue::CreateInt32(2)?)?;
            let response = wait(connection.SendMessageAsync(&probe)?, Duration::from_secs(3))?;
            use windows::core::Interface;
            let version: IPropertyValue = response.Message()?.Lookup(&HSTRING::from("protocol"))?.cast()?;
            if response.Status()? != AppServiceResponseStatus::Success || version.GetInt32()? != 2 {
                return Err(windows::core::Error::from(windows::core::HRESULT(0x80004005u32 as i32)));
            }
            Ok(Self(connection, operation))
        })()
        .map_err(|_| {
            "无法连接匹配版本的 Game Bar 组件；请同时更新主程序与组件，并检查包家族名、固定状态及点击穿透".into()
        })
    }

    pub fn send(
        &self,
        config: &Config,
        action: &str,
        text: &str,
        rect: RECT,
    ) -> std::result::Result<(), String> {
        use base64::Engine;
        let custom_art = if action == "show" {
            if let Some(path) = config.option_images.get(text) {
                use std::io::Read;
                let mut bytes = Vec::new();
                std::fs::File::open(path)
                    .map_err(|_| "自定义字效文件不可用".to_string())?
                    .take(32769)
                    .read_to_end(&mut bytes)
                    .map_err(|_| "无法读取自定义字效".to_string())?;
                if bytes.len() > 32768 {
                    return Err(
                        "Game Bar 自定义 PNG 须不超过 32 KB；可缩小图片或改用原生通道".into(),
                    );
                }
                base64::engine::general_purpose::STANDARD.encode(bytes)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let offset = super::art::atlas_offset(config, text);
        (|| -> Result<bool> {
            let payload = serde_json::json!({
                "protocol": 2, "operation": self.1.to_string(),
                "action": action, "text": text,
                "fullscreen": config.mode == DisplayMode::Fullscreen,
                "durationMs": if config.mode == DisplayMode::Fullscreen { config.theme.fullscreen_duration_ms } else { config.theme.widget_duration_ms },
                "font": config.theme.font, "textColor": config.theme.text_color,
                "backgroundColor": config.theme.background_color, "opacity": config.theme.opacity,
                "widgetBackgroundOpacity": config.theme.widget_background_opacity,
                "fontSize": if config.mode == DisplayMode::Fullscreen { config.theme.fullscreen_font_size } else { config.theme.widget_font_size },
                "animation": config.theme.animation, "hotkey": config.hotkey,
                "artStyle": config.theme.art_style, "customArt": custom_art,
                "artOffsetX": offset[0], "artOffsetY": offset[1],
                "width": if config.mode == DisplayMode::Fullscreen { rect.right - rect.left } else { config.widget_width },
                "height": if config.mode == DisplayMode::Fullscreen { rect.bottom - rect.top } else { config.widget_height },
            });
            let message = ValueSet::new()?;
            message.Insert(&HSTRING::from("payload"), &PropertyValue::CreateString(&HSTRING::from(payload.to_string()))?)?;
            let response = wait(self.0.SendMessageAsync(&message)?, Duration::from_secs(3))?;
            if response.Status()? != AppServiceResponseStatus::Success { return Ok(false); }
            use windows::core::Interface;
            let value: IPropertyValue = response.Message()?.Lookup(&HSTRING::from("accepted"))?.cast()?;
            value.GetBoolean()
        })().map_err(|_| "Game Bar 组件通信失败或超时".to_string()).and_then(|accepted|
            if accepted { Ok(()) } else { Err("Game Bar 未就绪：须手动打开并固定组件、启用点击穿透；也可能不支持请求的窗口尺寸".into()) })
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}
