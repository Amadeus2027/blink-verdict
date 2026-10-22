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

fn wait<T: RuntimeType + 'static>(operation: IAsyncOperation<T>) -> Result<T> {
    let start = Instant::now();
    while operation.Status()? == AsyncStatus::Started {
        if start.elapsed() > Duration::from_secs(3) {
            let _ = operation.Cancel();
            return Err(windows::core::Error::from(windows::core::HRESULT(
                0x800705B4u32 as i32,
            )));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    operation.GetResults()
}

pub struct Bridge(AppServiceConnection);

impl Bridge {
    pub fn connect(family: &str) -> std::result::Result<Self, String> {
        (|| -> Result<Self> {
            let connection = AppServiceConnection::new()?;
            connection.SetAppServiceName(&HSTRING::from("BlinkVerdictBridge"))?;
            connection.SetPackageFamilyName(&HSTRING::from(family))?;
            if wait(connection.OpenAsync()?)? != AppServiceConnectionStatus::Success {
                return Err(windows::core::Error::from(windows::core::HRESULT(
                    0x80004005u32 as i32,
                )));
            }
            Ok(Self(connection))
        })()
        .map_err(|_| {
            "无法连接 Game Bar 组件；请检查包家族名并手动打开、固定组件及启用点击穿透".into()
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
                if std::fs::metadata(path)
                    .map_err(|_| "自定义字效文件不可用".to_string())?
                    .len()
                    > 32768
                {
                    return Err(
                        "Game Bar 自定义 PNG 须不超过 32 KB；可缩小图片或改用原生通道".into(),
                    );
                }
                base64::engine::general_purpose::STANDARD
                    .encode(std::fs::read(path).map_err(|_| "无法读取自定义字效".to_string())?)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        (|| -> Result<bool> {
            let payload = serde_json::json!({
                "action": action, "text": text,
                "fullscreen": config.mode == DisplayMode::Fullscreen,
                "durationMs": if config.mode == DisplayMode::Fullscreen { config.theme.fullscreen_duration_ms } else { config.theme.widget_duration_ms },
                "font": config.theme.font, "textColor": config.theme.text_color,
                "backgroundColor": config.theme.background_color, "opacity": config.theme.opacity,
                "fontSize": if config.mode == DisplayMode::Fullscreen { config.theme.fullscreen_font_size } else { config.theme.widget_font_size },
                "animation": config.theme.animation, "hotkey": config.hotkey,
                "artStyle": config.theme.art_style, "customArt": custom_art,
                "width": if config.mode == DisplayMode::Fullscreen { rect.right - rect.left } else { config.widget_width },
                "height": if config.mode == DisplayMode::Fullscreen { rect.bottom - rect.top } else { config.widget_height },
            });
            let message = ValueSet::new()?;
            message.Insert(&HSTRING::from("payload"), &PropertyValue::CreateString(&HSTRING::from(payload.to_string()))?)?;
            let response = wait(self.0.SendMessageAsync(&message)?)?;
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
