use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::{fs, path::Path};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum ArtStyle {
    #[default]
    Plain,
    // Keep serialized keys stable for saved themes and the Game Bar bridge.
    Ink,       // Clear sans
    Cyberpunk, // Soft rounded
    Graffiti,  // Modern serif
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Animation {
    None,
    #[default]
    Fade,
    Flash,
    Pop,
    Slide,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum DisplayMode {
    #[default]
    Fullscreen,
    Widget,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Provider {
    #[default]
    Jev,
    SystemOne,
    Decider,
    Clef,
    OpenAiDecisions,
    OpenAi,
    OpenAiResponses,
    Anthropic,
    Gemini,
    Ollama,
}

impl Provider {
    pub fn is_decision(self) -> bool {
        matches!(
            self,
            Self::Jev | Self::SystemOne | Self::Decider | Self::Clef | Self::OpenAiDecisions
        )
    }

    pub fn defaults(self) -> (&'static str, &'static str) {
        match self {
            Self::Jev => ("https://api.typesafe.ai/v1", "jev-latest"),
            Self::SystemOne => ("http://127.0.0.1:8000/v1", "local-decision-model"),
            Self::Decider => ("https://api.perplexity.ai/v1", "pplx-decider-latest"),
            Self::Clef => (
                "https://api.cloudflare.com/client/v4/accounts/YOUR_ACCOUNT_ID/ai/run",
                "clef",
            ),
            Self::OpenAiDecisions => ("https://api.openai.com/v1", "gpt-6-luna"),
            Self::OpenAi => ("https://api.openai.com/v1", "gpt-4o-mini"),
            Self::OpenAiResponses => ("https://api.openai.com/v1", "gpt-4o-mini"),
            Self::Anthropic => ("https://api.anthropic.com/v1", "claude-sonnet-4-20250514"),
            Self::Gemini => (
                "https://generativelanguage.googleapis.com/v1beta",
                "gemini-2.5-flash",
            ),
            Self::Ollama => ("http://127.0.0.1:11434", "gemma3:4b"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub art_style: ArtStyle,
    pub font: String,
    pub text_color: String,
    pub background_color: String,
    pub opacity: u8,
    /// Alpha of the widget pane only; fullscreen never draws a background.
    pub widget_background_opacity: u8,
    pub fullscreen_font_size: u32,
    pub widget_font_size: u32,
    pub fullscreen_duration_ms: u32,
    /// Zero keeps the widget visible until the next trigger or mode switch.
    pub widget_duration_ms: u32,
    pub animation: Animation,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            art_style: ArtStyle::Plain,
            font: "Microsoft YaHei UI".into(),
            text_color: "1D1D1F".into(),
            background_color: "F4F4F7".into(),
            opacity: 244,
            widget_background_opacity: 180,
            fullscreen_font_size: 150,
            widget_font_size: 32,
            fullscreen_duration_ms: 1500,
            widget_duration_ms: 0,
            animation: Animation::Fade,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub mode: DisplayMode,
    pub hotkey: String,
    pub prompt: String,
    pub options: Vec<String>,
    pub option_images: BTreeMap<String, String>,
    pub provider: Provider,
    pub endpoint: String,
    pub model: String,
    pub vision: bool,
    pub timeout_seconds: u32,
    pub output_tokens: u32,
    /// Empty uses the native overlay. Otherwise route display to the optional Game Bar package.
    pub gamebar_package_family: String,
    pub theme: Theme,
    pub widget_x: i32,
    pub widget_y: i32,
    pub widget_width: i32,
    pub widget_height: i32,
}

impl Default for Config {
    fn default() -> Self {
        let (endpoint, model) = Provider::Jev.defaults();
        Self {
            mode: DisplayMode::Fullscreen,
            hotkey: "Ctrl+Alt+Space".into(),
            prompt: "根据当前屏幕和可选答案判断最合适的选择。若无法明确判断用户的目标，或证据不足，返回无法判断。".into(),
            options: vec!["YES".into(), "NO".into()],
            option_images: BTreeMap::new(),
            provider: Provider::Jev,
            endpoint: endpoint.into(),
            model: model.into(),
            vision: true,
            timeout_seconds: 15,
            output_tokens: 1024,
            gamebar_package_family: String::new(),
            theme: Theme::default(),
            widget_x: 48,
            widget_y: 48,
            widget_width: 320,
            widget_height: 140,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.prompt.trim().is_empty() || self.model.trim().is_empty() {
            return Err("判断指令和模型名称不能为空".into());
        }
        if !(2..=20).contains(&self.options.len())
            || self
                .options
                .iter()
                .any(|v| v.trim().is_empty() || v.chars().count() > 80)
        {
            return Err("请提供 2–20 个非空答案，每个不超过 80 字".into());
        }
        for (i, value) in self.options.iter().enumerate() {
            if self.options[..i].iter().any(|v| v.trim() == value.trim()) {
                return Err("答案选项不能重复".into());
            }
        }
        crate::provider::validate_endpoint(&self.endpoint)?;
        if !(1..=120).contains(&self.timeout_seconds)
            || !(64..=8192).contains(&self.output_tokens)
            || !(300..=60000).contains(&self.theme.fullscreen_duration_ms)
            || self.theme.widget_duration_ms > 600000
            || !(24..=400).contains(&self.theme.fullscreen_font_size)
            || !(12..=120).contains(&self.theme.widget_font_size)
            || self.theme.font.trim().is_empty()
            || self.theme.opacity < 30
            || !(160..=1600).contains(&self.widget_width)
            || !(80..=1000).contains(&self.widget_height)
        {
            return Err("时长、字号、透明度或窗口尺寸超出允许范围".into());
        }
        parse_color(&self.theme.text_color)?;
        parse_color(&self.theme.background_color)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        match fs::read(path) {
            Ok(bytes) => {
                let config: Self = serde_json::from_slice(&bytes)
                    .map_err(|_| "配置文件格式错误；原文件未修改".to_string())?;
                config.validate()?;
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("无法读取配置：{e}")),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let temporary = path.with_extension("json.tmp");
        let data = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&temporary, data).map_err(|e| e.to_string())?;
        fs::rename(temporary, path).map_err(|e| format!("无法保存配置：{e}"))
    }
}

pub fn parse_color(value: &str) -> Result<[u8; 3], String> {
    let s = value.trim().trim_start_matches('#');
    if s.len() != 6 || !s.is_ascii() {
        return Err("颜色使用六位十六进制，例如 D95C54".into());
    }
    let n = u32::from_str_radix(s, 16).map_err(|_| "颜色格式错误".to_string())?;
    Ok([(n >> 16) as u8, (n >> 8) as u8, n as u8])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_roundtrip_and_reject_invalid_choices() {
        let mut c = Config::default();
        assert!(c.validate().is_ok());
        assert_eq!(c.theme.widget_duration_ms, 0);
        c.options = vec!["进攻".into(), "撤退".into(), "等待".into()];
        c.theme.widget_background_opacity = 0;
        c.widget_x = -1920;
        c.widget_width = 600;
        let copy: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(copy.options, c.options);
        assert_eq!(copy.theme.widget_background_opacity, 0);
        assert_eq!((copy.widget_x, copy.widget_width), (-1920, 600));
        let legacy: Config = serde_json::from_value(serde_json::json!({"provider":"OpenAi","endpoint":"https://api.openai.com/v1","model":"gpt-4o-mini"})).unwrap();
        assert_eq!(legacy.provider, Provider::OpenAi);
        let legacy_theme: Theme = serde_json::from_str(r#"{"opacity":200}"#).unwrap();
        assert_eq!(legacy_theme.widget_background_opacity, 180);
        assert_eq!(legacy_theme.opacity, 200);
        assert!(legacy.validate().is_ok());
        assert!(copy.validate().is_ok());
        c.options.push("等待".into());
        assert!(c.validate().is_err());
        c.options = vec!["".into(), "NO".into()];
        assert!(c.validate().is_err());
    }
}
