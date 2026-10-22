use super::*;
use blink_verdict::config::{Animation, ArtStyle, Provider};
use windows::Win32::UI::Controls::Dialogs::*;
use windows::Win32::UI::Controls::*;

const SAVE: usize = 200;
const PREVIEW: usize = 201;
const CANCEL: usize = 202;
const APPLY_PRESET: usize = 203;
const CLEAR_KEY: usize = 204;
const PICK_IMAGE: usize = 205;
const REMOVE_IMAGE: usize = 206;
const PAGE_BASE: usize = 210;

pub(super) struct Settings {
    draft: Config,
    page: usize,
    controls: Vec<(usize, HWND)>,
    font: HFONT,
    brush: HBRUSH,
    scale: f32,
    pending_key: Option<String>,
}

impl Drop for Settings {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.font);
            let _ = DeleteObject(self.brush);
        }
    }
}

pub(super) unsafe fn open(app: &mut App) {
    if app.busy {
        app.set_status("判断中，完成后可打开设置".into());
        return;
    }
    if !app.settings.is_invalid() {
        let _ = ShowWindow(app.settings, SW_RESTORE);
        let _ = SetForegroundWindow(app.settings);
        return;
    }
    app.clear_gamebar();
    app.lock_widget();
    app.hide();
    let dpi = GetDpiForSystem() as f32 / 96.0;
    let mut work = RECT::default();
    let _ = SystemParametersInfoW(
        SPI_GETWORKAREA,
        0,
        Some(&mut work as *mut RECT as *mut _),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    );
    let scale = dpi.min((work.bottom - work.top - 50).max(480) as f32 / 710.0);
    let face = wide(&app.config.theme.font);
    let font = CreateFontW(
        -(16.0 * scale) as i32,
        0,
        0,
        0,
        FW_NORMAL.0 as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0 as u32,
        CLIP_DEFAULT_PRECIS.0 as u32,
        CLEARTYPE_QUALITY.0 as u32,
        DEFAULT_PITCH.0 as u32,
        PCWSTR(face.as_ptr()),
    );
    let background = blink_verdict::config::parse_color(&app.config.theme.background_color)
        .unwrap_or([17, 19, 24]);
    let brush = CreateSolidBrush(COLORREF(
        background[0] as u32 | ((background[1] as u32) << 8) | ((background[2] as u32) << 16),
    ));
    app.settings_ui = Some(Settings {
        draft: app.config.clone(),
        page: 0,
        controls: Vec::new(),
        font,
        brush,
        scale,
        pending_key: None,
    });
    let instance = GetModuleHandleW(None).unwrap_or_default();
    let hwnd = match CreateWindowExW(
        WS_EX_APPWINDOW | WS_EX_CONTROLPARENT,
        w!("BlinkVerdict.Settings"),
        w!("BlinkVerdict · 一瞬决断"),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        (760.0 * scale) as i32,
        (710.0 * scale) as i32,
        None,
        None,
        instance,
        None,
    ) {
        Ok(hwnd) => hwnd,
        Err(_) => {
            app.settings_ui = None;
            app.set_status("无法创建设置窗口".into());
            return;
        }
    };
    app.settings = hwnd;
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, app as *mut App as isize);
    app.settings_ui.as_mut().unwrap().build(hwnd);
    let _ = ShowWindow(hwnd, SW_SHOW);
    let _ = SetForegroundWindow(hwnd);
}

impl Settings {
    unsafe fn control(
        &mut self,
        parent: HWND,
        id: usize,
        class: PCWSTR,
        text: &str,
        style: WINDOW_STYLE,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> HWND {
        let content = wide(text);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            PCWSTR(content.as_ptr()),
            WS_CHILD | WS_VISIBLE | style,
            (x as f32 * self.scale) as i32,
            (y as f32 * self.scale) as i32,
            (width as f32 * self.scale) as i32,
            (height as f32 * self.scale) as i32,
            parent,
            HMENU(id as *mut _),
            GetModuleHandleW(None).unwrap_or_default(),
            None,
        )
        .unwrap_or_default();
        SendMessageW(hwnd, WM_SETFONT, WPARAM(self.font.0 as usize), LPARAM(1));
        self.controls.push((id, hwnd));
        hwnd
    }

    unsafe fn label(&mut self, parent: HWND, text: &str, x: i32, y: i32, width: i32, height: i32) {
        self.control(
            parent,
            0,
            w!("STATIC"),
            text,
            WINDOW_STYLE(0),
            x,
            y,
            width,
            height,
        );
    }
    unsafe fn edit(
        &mut self,
        parent: HWND,
        id: usize,
        label: &str,
        value: &str,
        y: i32,
        multiline: i32,
    ) {
        self.label(parent, label, 28, y, 170, 25);
        let height = if multiline > 0 { multiline } else { 28 };
        let style = WS_BORDER
            | WS_TABSTOP
            | if multiline > 0 {
                WS_VSCROLL | WINDOW_STYLE((ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN) as u32)
            } else {
                WINDOW_STYLE(ES_AUTOHSCROLL as u32)
            };
        let hwnd = self.control(parent, id, w!("EDIT"), value, style, 200, y, 510, height);
        SendMessageW(
            hwnd,
            EM_SETLIMITTEXT,
            WPARAM(if multiline > 0 { 8192 } else { 2048 }),
            LPARAM(0),
        );
    }
    unsafe fn combo(
        &mut self,
        parent: HWND,
        id: usize,
        label: &str,
        values: &[&str],
        selection: usize,
        y: i32,
    ) {
        self.label(parent, label, 28, y, 170, 25);
        let hwnd = self.control(
            parent,
            id,
            w!("COMBOBOX"),
            "",
            WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
            200,
            y,
            510,
            230,
        );
        for value in values {
            let value = wide(value);
            SendMessageW(
                hwnd,
                CB_ADDSTRING,
                WPARAM(0),
                LPARAM(value.as_ptr() as isize),
            );
        }
        SendMessageW(hwnd, CB_SETCURSEL, WPARAM(selection), LPARAM(0));
    }

    unsafe fn build(&mut self, parent: HWND) {
        for (_, hwnd) in self.controls.drain(..) {
            let _ = DestroyWindow(hwnd);
        }
        self.label(
            parent,
            "B L I N K V E R D I C T   /   一 瞬 决 断",
            28,
            18,
            680,
            28,
        );
        for (index, title) in ["决策", "模型", "外观", "字效", "关于 / 状态"]
            .iter()
            .enumerate()
        {
            self.control(
                parent,
                PAGE_BASE + index,
                w!("BUTTON"),
                title,
                WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
                28 + index as i32 * 138,
                56,
                126,
                32,
            );
        }
        let c = self.draft.clone();
        match self.page {
            0 => {
                self.combo(
                    parent,
                    300,
                    "显示模式",
                    &["全屏浮现", "悬浮窗口"],
                    if c.mode == DisplayMode::Fullscreen {
                        0
                    } else {
                        1
                    },
                    114,
                );
                self.edit(parent, 301, "全局快捷键", &c.hotkey, 160, 0);
                self.label(
                    parent,
                    "例：Ctrl+Alt+Space、Ctrl+Shift+F8；判断中重复按键不排队。",
                    200,
                    194,
                    510,
                    40,
                );
                self.edit(parent, 302, "判断指令", &c.prompt, 242, 105);
                self.edit(
                    parent,
                    303,
                    "答案选项（每行一个）",
                    &c.options.join("\r\n"),
                    370,
                    132,
                );
                self.label(parent, "说明目标才能准确判断，例如：根据当前局面选择进攻、撤退或等待。\r\n证据不足时显示“无法判断”；选项不是判断任务本身。", 28, 524, 680, 58);
            }
            1 => {
                self.combo(
                    parent,
                    310,
                    "接口协议",
                    &[
                        "OpenAI 兼容 · Chat Completions",
                        "OpenAI · Responses",
                        "Anthropic · Messages",
                        "Google · Gemini",
                        "Ollama · 本地服务",
                    ],
                    provider_index(c.provider),
                    112,
                );
                self.combo(
                    parent,
                    311,
                    "服务预设",
                    &[
                        "OpenAI",
                        "DeepSeek（文字）",
                        "通义千问 Qwen",
                        "SiliconFlow",
                        "OpenRouter",
                        "LM Studio（本机）",
                        "Anthropic",
                        "Google Gemini",
                        "Ollama（本机）",
                    ],
                    0,
                    156,
                );
                self.control(
                    parent,
                    APPLY_PRESET,
                    w!("BUTTON"),
                    "填入选中预设",
                    WS_TABSTOP,
                    510,
                    196,
                    200,
                    30,
                );
                self.edit(parent, 312, "API 基础地址", &c.endpoint, 242, 0);
                self.edit(parent, 313, "模型名称", &c.model, 282, 0);
                let pending = self.pending_key.clone().unwrap_or_default();
                self.edit(parent, 314, "新 API 密钥", &pending, 322, 0);
                SendMessageW(
                    self.hwnd(314),
                    EM_SETPASSWORDCHAR,
                    WPARAM('●' as usize),
                    LPARAM(0),
                );
                self.label(
                    parent,
                    "留空保留此协议 / 地址已有密钥；密钥存入 Windows 凭据管理器。",
                    200,
                    356,
                    510,
                    38,
                );
                self.control(
                    parent,
                    CLEAR_KEY,
                    w!("BUTTON"),
                    "删除此地址的密钥",
                    WS_TABSTOP,
                    510,
                    394,
                    200,
                    28,
                );
                self.combo(
                    parent,
                    315,
                    "屏幕输入",
                    &["截图 → 视觉模型", "本机 OCR → 文字模型"],
                    if c.vision { 0 } else { 1 },
                    436,
                );
                self.edit(
                    parent,
                    316,
                    "请求超时（秒）",
                    &c.timeout_seconds.to_string(),
                    482,
                    0,
                );
                self.edit(
                    parent,
                    317,
                    "最大输出 tokens",
                    &c.output_tokens.to_string(),
                    522,
                    0,
                );
                self.label(
                    parent,
                    "云端服务接收截图或 OCR 文字；本地需自行运行模型。请确认模型支持所选输入。",
                    28,
                    566,
                    680,
                    38,
                );
            }
            2 => {
                for (id, label, value, y) in [
                    (320, "字体", c.theme.font, 112),
                    (321, "文字颜色（HEX）", c.theme.text_color, 154),
                    (322, "背景颜色（HEX）", c.theme.background_color, 196),
                    (323, "整体不透明度 30–255", c.theme.opacity.to_string(), 238),
                    (
                        324,
                        "全屏字号 24–400",
                        c.theme.fullscreen_font_size.to_string(),
                        280,
                    ),
                    (
                        325,
                        "悬浮字号 12–120",
                        c.theme.widget_font_size.to_string(),
                        322,
                    ),
                    (
                        326,
                        "全屏时长（毫秒）",
                        c.theme.fullscreen_duration_ms.to_string(),
                        364,
                    ),
                    (
                        327,
                        "悬浮保留（毫秒）",
                        c.theme.widget_duration_ms.to_string(),
                        406,
                    ),
                ] {
                    self.edit(parent, id, label, &value, y, 0);
                }
                self.combo(
                    parent,
                    328,
                    "显示动画",
                    &[
                        "无动画",
                        "渐入渐出",
                        "闪现（单次强调）",
                        "缩放弹入",
                        "上浮进入",
                    ],
                    match c.theme.animation {
                        Animation::None => 0,
                        Animation::Fade => 1,
                        Animation::Flash => 2,
                        Animation::Pop => 3,
                        Animation::Slide => 4,
                    },
                    452,
                );
                self.label(parent, "悬浮保留设为 0 表示保持可见。位置和大小从托盘“编辑悬浮窗口”调整。\r\n两种模式共用字体与配色；字号和时长独立。预览会先保存设置。", 28, 514, 680, 64);
            }
            3 => {
                self.combo(
                    parent,
                    340,
                    "默认 YES / NO 字效",
                    &[
                        "原生字体",
                        "血墨书法 · 只狼灵感",
                        "酸黄故障 · 2077 灵感",
                        "霓虹涂鸦 · 迈阿密灵感",
                    ],
                    match c.theme.art_style {
                        ArtStyle::Plain => 0,
                        ArtStyle::Ink => 1,
                        ArtStyle::Cyberpunk => 2,
                        ArtStyle::Graffiti => 3,
                    },
                    118,
                );
                self.label(parent, "三组透明字效由图像生成工具制作，适用于 YES / NO（忽略大小写）。\r\n这不是通用字体文件。其他答案继续使用原生字体，或配置自定义 PNG。\r\n字效与文字共用动画；两种显示模式共用风格。", 28, 174, 680, 82);
                let labels: Vec<&str> = c.options.iter().map(String::as_str).collect();
                self.combo(parent, 341, "自定义字效对应答案", &labels, 0, 290);
                let path = c
                    .options
                    .first()
                    .and_then(|v| c.option_images.get(v))
                    .map(String::as_str)
                    .unwrap_or("");
                self.edit(parent, 342, "PNG 文件", path, 342, 0);
                SendMessageW(self.hwnd(342), EM_SETREADONLY, WPARAM(1), LPARAM(0));
                self.control(
                    parent,
                    PICK_IMAGE,
                    w!("BUTTON"),
                    "选择 PNG…",
                    WS_TABSTOP,
                    408,
                    394,
                    144,
                    32,
                );
                self.control(
                    parent,
                    REMOVE_IMAGE,
                    w!("BUTTON"),
                    "移除此答案字效",
                    WS_TABSTOP,
                    566,
                    394,
                    144,
                    32,
                );
                self.label(parent, "PNG 映射优先于内置风格；文件无效时回退到原生文字。\r\n原生通道上限：8 MB / 4096×4096；Game Bar 自定义图上限：32 KB。\r\n文件保留在原位置，移动文件后请重新选择；这里不会调用在线图像生成。", 28, 460, 680, 96);
            }
            _ => {
                self.label(parent, "轻量后台 AI 决策助手 · v0.1.0 开发预览\r\n\r\n按键 → 当前屏幕 → 模型判断 → 短暂显示\r\n默认不保存截图或判断历史，不进行后台屏幕监控。\r\n结果仅供参考；模型与服务由用户自行配置。\r\n\r\n独占全屏采用可选 Game Bar 组件，尚未实机验证。\r\n不采用进程注入或图形 Hook。\r\n项目：https://github.com/Amadeus2027/blink-verdict", 28, 118, 680, 240);
                self.edit(
                    parent,
                    330,
                    "Game Bar 包家族名",
                    &c.gamebar_package_family,
                    380,
                    0,
                );
                self.label(parent, "留空使用原生覆盖；填写后仅使用 Game Bar，不自动回退或打开面板。\r\n需单独安装、手动打开并固定组件、启用点击穿透。\r\n该组件不接收截图或密钥；安装步骤见 docs/gamebar.md。", 28, 428, 680, 80);
            }
        }
        self.control(
            parent,
            SAVE,
            w!("BUTTON"),
            "保存",
            WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
            270,
            610,
            132,
            34,
        );
        self.control(
            parent,
            PREVIEW,
            w!("BUTTON"),
            "保存并预览",
            WS_TABSTOP,
            420,
            610,
            140,
            34,
        );
        self.control(
            parent,
            CANCEL,
            w!("BUTTON"),
            "取消",
            WS_TABSTOP,
            578,
            610,
            132,
            34,
        );
        let _ = InvalidateRect(parent, None, true);
    }

    unsafe fn hwnd(&self, id: usize) -> HWND {
        self.controls
            .iter()
            .find(|v| v.0 == id)
            .map(|v| v.1)
            .unwrap_or_default()
    }
    unsafe fn text(&self, id: usize) -> String {
        let hwnd = self.hwnd(id);
        let count = GetWindowTextLengthW(hwnd).max(0) as usize;
        let mut buffer = vec![0u16; count + 1];
        let len = GetWindowTextW(hwnd, &mut buffer).max(0) as usize;
        String::from_utf16_lossy(&buffer[..len])
    }
    unsafe fn selection(&self, id: usize) -> usize {
        SendMessageW(self.hwnd(id), CB_GETCURSEL, WPARAM(0), LPARAM(0))
            .0
            .max(0) as usize
    }
    unsafe fn number(&self, id: usize) -> Result<u32, String> {
        self.text(id)
            .trim()
            .parse()
            .map_err(|_| "请在数值字段填写非负整数".into())
    }

    unsafe fn collect(&mut self) -> Result<(), String> {
        let mut c = self.draft.clone();
        match self.page {
            0 => {
                c.mode = if self.selection(300) == 0 {
                    DisplayMode::Fullscreen
                } else {
                    DisplayMode::Widget
                };
                c.hotkey = self.text(301).trim().into();
                hotkey::parse(&c.hotkey)?;
                c.prompt = self.text(302).trim().into();
                c.options = self
                    .text(303)
                    .lines()
                    .map(|v| v.trim().to_string())
                    .collect();
            }
            1 => {
                c.provider = index_provider(self.selection(310));
                c.endpoint = self.text(312).trim().into();
                c.model = self.text(313).trim().into();
                let key = self.text(314);
                if !key.is_empty() {
                    self.pending_key = Some(key.trim().into());
                }
                c.vision = self.selection(315) == 0;
                c.timeout_seconds = self.number(316)?;
                c.output_tokens = self.number(317)?;
            }
            2 => {
                c.theme.font = self.text(320).trim().into();
                c.theme.text_color = self.text(321).trim().into();
                c.theme.background_color = self.text(322).trim().into();
                c.theme.opacity = u8::try_from(self.number(323)?)
                    .map_err(|_| "透明度范围为 30–255".to_string())?;
                c.theme.fullscreen_font_size = self.number(324)?;
                c.theme.widget_font_size = self.number(325)?;
                c.theme.fullscreen_duration_ms = self.number(326)?;
                c.theme.widget_duration_ms = self.number(327)?;
                c.theme.animation = match self.selection(328) {
                    0 => Animation::None,
                    2 => Animation::Flash,
                    3 => Animation::Pop,
                    4 => Animation::Slide,
                    _ => Animation::Fade,
                };
            }
            3 => {
                c.theme.art_style = match self.selection(340) {
                    1 => ArtStyle::Ink,
                    2 => ArtStyle::Cyberpunk,
                    3 => ArtStyle::Graffiti,
                    _ => ArtStyle::Plain,
                };
            }
            _ => {
                c.gamebar_package_family = self.text(330).trim().into();
            }
        }
        c.validate()?;
        self.draft = c;
        Ok(())
    }

    unsafe fn preset(&mut self, parent: HWND) {
        let index = self.selection(311);
        let (provider, endpoint, model, vision) = match index {
            1 => (
                Provider::OpenAi,
                "https://api.deepseek.com/v1",
                "deepseek-chat",
                false,
            ),
            2 => (
                Provider::OpenAi,
                "https://dashscope.aliyuncs.com/compatible-mode/v1",
                "qwen-vl-plus",
                true,
            ),
            3 => (
                Provider::OpenAi,
                "https://api.siliconflow.cn/v1",
                "Qwen/Qwen2.5-VL-72B-Instruct",
                true,
            ),
            4 => (
                Provider::OpenAi,
                "https://openrouter.ai/api/v1",
                "openai/gpt-4o-mini",
                true,
            ),
            5 => (
                Provider::OpenAi,
                "http://127.0.0.1:1234/v1",
                "local-model",
                true,
            ),
            6 => {
                let (e, m) = Provider::Anthropic.defaults();
                (Provider::Anthropic, e, m, true)
            }
            7 => {
                let (e, m) = Provider::Gemini.defaults();
                (Provider::Gemini, e, m, true)
            }
            8 => {
                let (e, m) = Provider::Ollama.defaults();
                (Provider::Ollama, e, m, true)
            }
            _ => {
                let (e, m) = Provider::OpenAi.defaults();
                (Provider::OpenAi, e, m, true)
            }
        };
        self.draft.provider = provider;
        self.draft.endpoint = endpoint.into();
        self.draft.model = model.into();
        self.draft.vision = vision;
        // A newly selected endpoint never inherits a typed key intended for another service.
        self.pending_key = None;
        self.build(parent);
    }

    unsafe fn image(&mut self, parent: HWND, remove: bool) {
        let Some(label) = self.draft.options.get(self.selection(341)).cloned() else {
            return;
        };
        if remove {
            self.draft.option_images.remove(&label);
        } else {
            let mut file = vec![0u16; 4096];
            let filter = wide("PNG 图片\0*.png\0");
            let mut dialog = OPENFILENAMEW {
                lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
                hwndOwner: parent,
                lpstrFilter: PCWSTR(filter.as_ptr()),
                lpstrFile: windows::core::PWSTR(file.as_mut_ptr()),
                nMaxFile: file.len() as u32,
                Flags: OFN_FILEMUSTEXIST
                    | OFN_PATHMUSTEXIST
                    | OFN_NOCHANGEDIR
                    | OFN_DONTADDTORECENT,
                ..Default::default()
            };
            if !GetOpenFileNameW(&mut dialog).as_bool() {
                return;
            }
            let len = file.iter().position(|c| *c == 0).unwrap_or(file.len());
            self.draft
                .option_images
                .insert(label.clone(), String::from_utf16_lossy(&file[..len]));
        }
        let value = wide(
            self.draft
                .option_images
                .get(&label)
                .map(String::as_str)
                .unwrap_or(""),
        );
        let _ = SetWindowTextW(self.hwnd(342), PCWSTR(value.as_ptr()));
    }
}

fn provider_index(provider: Provider) -> usize {
    match provider {
        Provider::OpenAi => 0,
        Provider::OpenAiResponses => 1,
        Provider::Anthropic => 2,
        Provider::Gemini => 3,
        Provider::Ollama => 4,
    }
}
fn index_provider(index: usize) -> Provider {
    match index {
        1 => Provider::OpenAiResponses,
        2 => Provider::Anthropic,
        3 => Provider::Gemini,
        4 => Provider::Ollama,
        _ => Provider::OpenAi,
    }
}

unsafe fn error(parent: HWND, message: &str) {
    let text = wide(message);
    MessageBoxW(
        parent,
        PCWSTR(text.as_ptr()),
        w!("BlinkVerdict · 设置"),
        MB_OK | MB_ICONWARNING,
    );
}

unsafe fn save(app: &mut App, preview: bool) {
    let ui = app.settings_ui.as_mut().unwrap();
    if let Err(message) = ui.collect() {
        error(app.settings, &message);
        return;
    }
    let config = ui.draft.clone();
    let key = ui.pending_key.clone();
    if config.hotkey != app.config.hotkey || !app.hotkey_active {
        let _ = UnregisterHotKey(app.host, HOTKEY_ID);
        if let Err(message) = app.register_hotkey(&config.hotkey) {
            app.hotkey_active = app.register_hotkey(&app.config.hotkey).is_ok();
            error(app.settings, &message);
            return;
        }
    }
    let result = (|| {
        if let Some(key) = key {
            credentials::write(&config, &key)?;
        }
        config.save(&app.config_path)
    })();
    if let Err(message) = result {
        let _ = UnregisterHotKey(app.host, HOTKEY_ID);
        app.hotkey_active = app.register_hotkey(&app.config.hotkey).is_ok();
        error(app.settings, &message);
        return;
    }
    app.config = config;
    app.hotkey_active = true;
    app.set_status("设置已保存，等待快捷键".into());
    let _ = DestroyWindow(app.settings);
    if preview {
        app.command(CMD_PREVIEW);
    }
}

pub(super) unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
    if pointer.is_null() {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    let app = &mut *pointer;
    match message {
        WM_COMMAND => {
            let id = wparam.0 & 0xffff;
            match id {
                SAVE => save(app, false),
                PREVIEW => save(app, true),
                CANCEL | 2 => {
                    let _ = DestroyWindow(hwnd);
                }
                APPLY_PRESET => app.settings_ui.as_mut().unwrap().preset(hwnd),
                PICK_IMAGE | REMOVE_IMAGE => app
                    .settings_ui
                    .as_mut()
                    .unwrap()
                    .image(hwnd, id == REMOVE_IMAGE),
                341 if wparam.0 >> 16 == CBN_SELCHANGE as usize => {
                    let ui = app.settings_ui.as_ref().unwrap();
                    let label = ui.draft.options.get(ui.selection(341));
                    let path = label
                        .and_then(|v| ui.draft.option_images.get(v))
                        .map(String::as_str)
                        .unwrap_or("");
                    let value = wide(path);
                    let _ = SetWindowTextW(ui.hwnd(342), PCWSTR(value.as_ptr()));
                }
                CLEAR_KEY => {
                    let ui = app.settings_ui.as_mut().unwrap();
                    ui.pending_key = Some(String::new());
                    let _ = SetWindowTextW(ui.hwnd(314), w!(""));
                    error(hwnd, "保存后删除当前地址的密钥；取消则不删除。");
                }
                value if (PAGE_BASE..PAGE_BASE + 5).contains(&value) => {
                    let ui = app.settings_ui.as_mut().unwrap();
                    match ui.collect() {
                        Ok(()) => {
                            ui.page = value - PAGE_BASE;
                            ui.build(hwnd);
                        }
                        Err(message) => error(hwnd, &message),
                    }
                }
                _ => (),
            }
            LRESULT(0)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            if let Some(ui) = &app.settings_ui {
                let dc = HDC(wparam.0 as *mut _);
                SetTextColor(dc, COLORREF(0x00E6E3E2));
                let [r, g, b] =
                    blink_verdict::config::parse_color(&app.config.theme.background_color)
                        .unwrap_or([17, 19, 24]);
                SetBkColor(
                    dc,
                    COLORREF(r as u32 | ((g as u32) << 8) | ((b as u32) << 16)),
                );
                return LRESULT(ui.brush.0 as isize);
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => {
            if let Some(ui) = &app.settings_ui {
                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);
                FillRect(HDC(wparam.0 as *mut _), &rect, ui.brush);
            }
            LRESULT(1)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            app.settings = HWND::default();
            app.settings_ui = None;
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}
