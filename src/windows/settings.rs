use super::settings_style::{self as visual, Style};
use super::*;
use blink_verdict::config::{Animation, ArtStyle, Provider};
use windows::Win32::UI::Controls::Dialogs::*;
use windows::Win32::UI::Controls::*;

const SAVE: usize = 200;
const PREVIEW: usize = 201;
const CANCEL: usize = 202;
const APPLY: usize = 203;
const CLEAR_KEY: usize = 204;
const PICK_IMAGE: usize = 205;
const REMOVE_IMAGE: usize = 206;
const RESET_WIDGET: usize = 207;
const EDIT_WIDGET: usize = 208;
const NEXT: usize = 209;
const PAGE_BASE: usize = 210;
const BACK: usize = 220;
const TEXT_COLOR: usize = 221;
const BACKGROUND_COLOR: usize = 222;
const CLOSE: usize = 223;

#[derive(Clone)]
enum Field {
    Text(String),
    Choice(usize),
    Check(bool),
}

pub(super) struct Settings {
    draft: Config,
    page: usize,
    onboarding: bool,
    controls: Vec<(usize, HWND)>,
    cards: Vec<(i32, i32, i32, i32)>,
    rows: Vec<(usize, i32)>,
    style: Style,
    scale: f32,
    pending_key: Option<String>,
    fields: std::collections::BTreeMap<usize, Field>,
    visited: std::collections::BTreeSet<usize>,
    building: bool,
    choice_labels: std::collections::BTreeMap<usize, Vec<String>>,
}

pub(super) unsafe fn open(app: &mut App) {
    if app.busy {
        app.set_status("正在判断，稍后即可打开设置".into());
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
        Some((&mut work as *mut RECT).cast()),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    );
    let scale = dpi
        .min((work.bottom - work.top - 44).max(480) as f32 / 860.0)
        .min((work.right - work.left - 40).max(520) as f32 / 800.0);
    let onboarding = !app.config_path.exists();
    // Result fonts stay independent of the interface. Capture-only fonts are never shipped.
    let face = if app.recording_preview {
        app.config.theme.font.as_str()
    } else {
        "Microsoft YaHei UI"
    };
    let style = Style::new(scale, face);
    let mut fields = std::collections::BTreeMap::new();
    fields.insert(329, Field::Check(app.config.theme.widget_duration_ms == 0));
    app.settings_ui = Some(Settings {
        draft: app.config.clone(),
        page: if onboarding { 1 } else { 0 },
        onboarding,
        controls: Vec::new(),
        cards: Vec::new(),
        rows: Vec::new(),
        style,
        scale,
        pending_key: None,
        fields,
        visited: Default::default(),
        building: false,
        choice_labels: Default::default(),
    });
    let hwnd = match CreateWindowExW(
        WS_EX_APPWINDOW | WS_EX_CONTROLPARENT,
        w!("BlinkVerdict.Settings"),
        w!("BlinkVerdict"),
        WS_POPUP | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        (800.0 * scale) as i32,
        (860.0 * scale) as i32,
        None,
        None,
        GetModuleHandleW(None).unwrap_or_default(),
        None,
    ) {
        Ok(h) => h,
        Err(_) => {
            app.settings_ui = None;
            app.set_status("设置暂时无法打开，请重试".into());
            return;
        }
    };
    app.settings = hwnd;
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, app as *mut App as isize);
    SendMessageW(
        hwnd,
        WM_CHANGEUISTATE,
        WPARAM(UIS_SET as usize | ((UISF_HIDEFOCUS as usize) << 16)),
        LPARAM(0),
    );
    app.settings_ui.as_mut().unwrap().style.attach(hwnd);
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
        let button = class.as_wide() == w!("BUTTON").as_wide();
        let style = if button {
            WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32)
        } else {
            style
        };
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
        SendMessageW(
            hwnd,
            WM_SETFONT,
            WPARAM(if id == 0 {
                self.style.small.0
            } else {
                self.style.font.0
            } as usize),
            LPARAM(1),
        );
        if id >= 300 {
            let _ = SetWindowTheme(hwnd, w!(""), w!(""));
        }
        if class.as_wide() == w!("COMBOBOX").as_wide() || id == 301 {
            let _ = SetWindowSubclass(
                hwnd,
                Some(control_proc),
                id,
                GetWindowLongPtrW(parent, GWLP_USERDATA) as usize,
            );
        }
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
    unsafe fn card(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.cards.push((x, y, width, height));
    }
    unsafe fn section(&mut self, parent: HWND, text: &str, y: i32) {
        self.label(parent, text, 56, y, 688, 24);
    }
    unsafe fn row_label(&mut self, parent: HWND, id: usize, label: &str, y: i32) {
        self.rows.push((id, y));
        self.control(
            parent,
            11,
            w!("STATIC"),
            label,
            WINDOW_STYLE(0x0200), // SS_CENTERIMAGE
            94,
            y + 16,
            242,
            28,
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
        if multiline > 0 {
            self.card(36, y, 728, multiline);
            self.control(
                parent,
                11,
                w!("STATIC"),
                label,
                WINDOW_STYLE(0),
                58,
                y + 16,
                680,
                26,
            );
        } else {
            self.row_label(parent, id, label, y);
        }
        let (x, top, width, height) = if multiline > 0 {
            (58, y + 48, 678, multiline - 60)
        } else {
            (350, y + 19, 372, 26)
        };
        let style = WS_TABSTOP
            | if multiline > 0 {
                WINDOW_STYLE((ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN) as u32)
            } else {
                WINDOW_STYLE((ES_AUTOHSCROLL | ES_RIGHT) as u32)
            };
        let hwnd = self.control(parent, id, w!("EDIT"), value, style, x, top, width, height);
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
        self.row_label(parent, id, label, y);
        self.choice_labels
            .insert(id, values.iter().map(|s| s.to_string()).collect());
        let hwnd = self.control(
            parent,
            id,
            w!("COMBOBOX"),
            "",
            WS_TABSTOP
                | WS_VSCROLL
                | WINDOW_STYLE((CBS_DROPDOWNLIST | CBS_OWNERDRAWFIXED | CBS_HASSTRINGS) as u32),
            322,
            y + 13,
            412,
            280,
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
        let selection = match self.fields.get(&id) {
            Some(Field::Choice(i)) => *i,
            _ => selection,
        };
        self.fields.insert(id, Field::Choice(selection));
        SendMessageW(hwnd, CB_SETCURSEL, WPARAM(selection), LPARAM(0));
    }
    unsafe fn button(
        &mut self,
        parent: HWND,
        id: usize,
        text: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        self.control(
            parent,
            id,
            w!("BUTTON"),
            text,
            WS_TABSTOP,
            x,
            y,
            width,
            height,
        );
    }
    fn root_page(&self) -> usize {
        match self.page {
            3 | 4 | 7 => 2,
            6 => 1,
            5 | 8 => 0,
            p => p,
        }
    }
    unsafe fn build(&mut self, parent: HWND) {
        self.building = true;
        self.visited.insert(self.page);
        for (_, hwnd) in self.controls.drain(..) {
            let _ = DestroyWindow(hwnd);
        }
        self.cards.clear();
        self.rows.clear();
        for (page, label, x) in [(0, "决策", 216), (2, "外观", 340), (1, "模型", 464)] {
            self.button(parent, PAGE_BASE + page, label, x, 94, 120, 42);
        }
        if matches!(self.page, 3..=8) {
            self.button(parent, BACK, "‹", 36, 94, 44, 42);
        }
        if self.page != 5 {
            self.button(parent, 215, "…", 680, 28, 44, 42);
        }
        self.button(parent, CLOSE, "×", 738, 28, 40, 42);
        let mut c = self.draft.clone();
        if let Some(Field::Text(options)) = self.fields.get(&303) {
            c.options = options
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        match self.page {
            0 => {
                self.section(parent, "日常使用", 162);
                self.card(36, 192, 728, 124);
                self.combo(
                    parent,
                    300,
                    "显示方式",
                    &["全屏浮现", "悬浮组件"],
                    if c.mode == DisplayMode::Fullscreen {
                        0
                    } else {
                        1
                    },
                    192,
                );
                self.edit(parent, 301, "决策快捷键", &c.hotkey, 254, 0);
                self.label(
                    parent,
                    "点击快捷键后，直接按下你想使用的组合键。",
                    58,
                    330,
                    680,
                    24,
                );
                self.section(parent, "你的决策", 378);
                self.edit(parent, 302, "想判断什么", &c.prompt, 408, 140);
                self.edit(parent, 303, "答案选项", &c.options.join("\r\n"), 532, 106);
                self.label(
                    parent,
                    "每行一个答案。证据不足时会显示“无法判断”。",
                    58,
                    716,
                    680,
                    28,
                );
            }
            1 => {
                self.section(
                    parent,
                    if self.onboarding {
                        "先连接一个模型"
                    } else {
                        "模型服务"
                    },
                    162,
                );
                self.card(36, 192, 728, 62);
                let preset = if c.endpoint == c.provider.defaults().0 {
                    match c.provider {
                        Provider::Jev => 1,
                        Provider::Decider => 2,
                        Provider::Clef => 3,
                        Provider::OpenAiDecisions => 4,
                        Provider::OpenAi => 5,
                        Provider::Anthropic => 11,
                        Provider::Gemini => 12,
                        Provider::Ollama => 13,
                        _ => 0,
                    }
                } else {
                    0
                };
                self.combo(
                    parent,
                    311,
                    "使用服务",
                    &[
                        "自定义连接",
                        "TypeSafe Jev",
                        "Perplexity Decider",
                        "Cloudflare Clef",
                        "OpenAI Decisions",
                        "OpenAI",
                        "DeepSeek",
                        "通义千问",
                        "SiliconFlow",
                        "OpenRouter",
                        "LM Studio · 本机",
                        "Anthropic",
                        "Google Gemini",
                        "Ollama · 本机",
                        "System One · 本机",
                        "Decider · 本机",
                        "OpenAI Decisions · 本机",
                    ],
                    preset,
                    192,
                );
                self.section(parent, "连接信息", 280);
                self.card(36, 310, 728, 186);
                self.edit(parent, 313, "模型名称", &c.model, 310, 0);
                self.edit(parent, 312, "服务地址", &c.endpoint, 372, 0);
                let pending = self.pending_key.clone().unwrap_or_default();
                self.edit(parent, 314, "API 密钥", &pending, 434, 0);
                SendMessageW(
                    self.hwnd(314),
                    EM_SETPASSWORDCHAR,
                    WPARAM('●' as usize),
                    LPARAM(0),
                );
                self.control(
                    parent,
                    318,
                    w!("STATIC"),
                    if self.pending_key.as_deref() == Some("") {
                        "移除密钥将在完成设置后生效。"
                    } else {
                        "密钥保存在此设备。留空可继续使用已保存的密钥。"
                    },
                    WINDOW_STYLE(0),
                    58,
                    510,
                    680,
                    28,
                );
                self.button(parent, CLEAR_KEY, "移除密钥", 594, 542, 146, 30);
                self.card(36, 590, 728, 62);
                self.combo(
                    parent,
                    315,
                    "读取屏幕",
                    &["屏幕画面", "屏幕文字 · OCR"],
                    if c.vision { 0 } else { 1 },
                    590,
                );
                self.button(parent, 216, "连接选项  ›", 36, 664, 728, 42);
            }
            2 => {
                self.card(36, 174, 728, 152);
                self.section(parent, "显示效果", 352);
                self.card(36, 382, 728, 186);
                self.combo(
                    parent,
                    328,
                    "出现方式",
                    &["直接出现", "轻柔浮现", "闪现", "缩放进入", "向上浮入"],
                    match c.theme.animation {
                        Animation::None => 0,
                        Animation::Fade => 1,
                        Animation::Flash => 2,
                        Animation::Pop => 3,
                        Animation::Slide => 4,
                    },
                    382,
                );
                self.button(parent, 329, "持续显示浮窗", 36, 444, 728, 62);
                self.button(parent, 217, "颜色与时长  ›", 36, 506, 728, 62);
                self.label(
                    parent,
                    "预览会短暂显示当前外观，不会触发决策。",
                    58,
                    582,
                    680,
                    26,
                );
                self.button(parent, 213, "字效与图片  ›", 36, 626, 354, 64);
                self.button(parent, 214, "位置与大小  ›", 410, 626, 354, 64);
            }
            3 => {
                self.section(parent, "答案的表达", 162);
                self.card(36, 192, 728, 62);
                self.combo(
                    parent,
                    340,
                    "文字风格",
                    &["简洁文字", "血墨书法", "赛博故障", "霓虹涂鸦"],
                    match c.theme.art_style {
                        ArtStyle::Plain => 0,
                        ArtStyle::Ink => 1,
                        ArtStyle::Cyberpunk => 2,
                        ArtStyle::Graffiti => 3,
                    },
                    192,
                );
                self.label(
                    parent,
                    "内置字效适用于 YES / NO。其他答案也可以使用自己的图片。",
                    58,
                    268,
                    680,
                    48,
                );
                self.section(parent, "为答案添加图片", 342);
                self.card(36, 372, 728, 124);
                let labels: Vec<&str> = c.options.iter().map(String::as_str).collect();
                self.combo(parent, 341, "选择答案", &labels, 0, 372);
                let path = c
                    .options
                    .first()
                    .and_then(|s| c.option_images.get(s))
                    .map(String::as_str)
                    .unwrap_or("");
                self.edit(parent, 342, "图片文件", path, 434, 0);
                SendMessageW(self.hwnd(342), EM_SETREADONLY, WPARAM(1), LPARAM(0));
                self.button(parent, PICK_IMAGE, "选择图片…", 36, 520, 354, 52);
                self.button(parent, REMOVE_IMAGE, "恢复文字", 410, 520, 354, 52);
                self.label(
                    parent,
                    "选择带透明背景的 PNG。图片会保留在原位置，\r\n与文字使用相同的出现动画。",
                    58,
                    594,
                    680,
                    64,
                );
            }
            4 => {
                self.section(parent, "放在顺手的位置", 162);
                self.button(parent, EDIT_WIDGET, "在屏幕上拖动调整  ›", 36, 192, 728, 76);
                self.label(
                    parent,
                    "拖动移动，边缘缩放。右键单击即可完成。",
                    58,
                    282,
                    680,
                    28,
                );
                self.section(parent, "精确调整 · 像素", 336);
                self.card(36, 366, 728, 248);
                for (id, label, value, y) in [
                    (350, "水平位置", c.widget_x, 366),
                    (351, "垂直位置", c.widget_y, 428),
                    (352, "宽度", c.widget_width, 490),
                    (353, "高度", c.widget_height, 552),
                ] {
                    self.edit(parent, id, label, &value.to_string(), y, 0);
                }
                self.button(parent, RESET_WIDGET, "恢复默认布局", 36, 632, 728, 48);
            }
            5 => {
                self.section(parent, "BlinkVerdict · 一瞬决断", 174);
                self.card(36, 204, 728, 160);
                self.control(
                    parent,
                    11,
                    w!("STATIC"),
                    "按一下，让答案浮现。",
                    WINDOW_STYLE(0),
                    60,
                    230,
                    680,
                    32,
                );
                self.label(
                    parent,
                    "为眼前的选择，留一点思考空间。\r\n版本 0.1.0 · 预览版",
                    60,
                    284,
                    680,
                    64,
                );
                self.section(parent, "关于你的隐私", 394);
                self.card(36, 424, 728, 148);
                self.label(parent,"仅在你按下快捷键时读取屏幕。\r\n截图与结果不保存到本机；云端模型会收到画面或文字。\r\nAPI 密钥保存在 Windows 凭据管理器中。",60,448,680,100);
                self.button(parent, 218, "游戏中的显示  ›", 36, 594, 728, 62);
            }
            6 => {
                self.section(parent, "连接选项", 162);
                self.card(36, 192, 728, 186);
                self.combo(
                    parent,
                    310,
                    "连接协议",
                    &[
                        "TypeSafe Jev",
                        "Perplexity Decider",
                        "Cloudflare Clef",
                        "OpenAI Decisions",
                        "System One / vLLM",
                        "OpenAI 兼容",
                        "OpenAI Responses",
                        "Anthropic Messages",
                        "Google Gemini",
                        "Ollama",
                    ],
                    provider_index(c.provider),
                    192,
                );
                self.edit(
                    parent,
                    316,
                    "等待时间 · 秒",
                    &c.timeout_seconds.to_string(),
                    254,
                    0,
                );
                self.edit(
                    parent,
                    317,
                    "最大输出 · tokens",
                    &c.output_tokens.to_string(),
                    316,
                    0,
                );
                self.label(parent,"通常保留默认值即可。专用决策模型不使用输出长度设置。\r\n本地服务需要先在此设备上运行。",58,396,680,64);
            }
            7 => {
                self.section(parent, "颜色与排版", 162);
                self.card(36, 192, 728, 372);
                for (id, label, value, y) in [
                    (320, "文字字体", c.theme.font, 192),
                    (321, "文字颜色", c.theme.text_color, 254),
                    (322, "背景颜色", c.theme.background_color, 316),
                    (
                        323,
                        "不透明度 · %",
                        format!("{:.1}", c.theme.opacity as f32 * 100.0 / 255.0),
                        378,
                    ),
                    (
                        324,
                        "全屏字号",
                        c.theme.fullscreen_font_size.to_string(),
                        440,
                    ),
                    (325, "浮窗字号", c.theme.widget_font_size.to_string(), 502),
                ] {
                    self.edit(parent, id, label, &value, y, 0);
                }
                self.button(parent, TEXT_COLOR, "选色", 590, 272, 144, 28);
                self.button(parent, BACKGROUND_COLOR, "选色", 590, 334, 144, 28);
                // Leave the editable HEX value visible beside its color picker.
                for id in [321, 322] {
                    let h = self.hwnd(id);
                    let mut r = RECT::default();
                    let _ = GetWindowRect(h, &mut r);
                    let _ = SetWindowPos(
                        h,
                        None,
                        0,
                        0,
                        (180.0 * self.scale) as i32,
                        r.bottom - r.top,
                        SWP_NOMOVE | SWP_NOZORDER,
                    );
                }
                self.section(parent, "停留时间 · 秒", 584);
                self.card(36, 614, 728, 124);
                self.edit(
                    parent,
                    326,
                    "全屏浮现",
                    &seconds_text(c.theme.fullscreen_duration_ms),
                    614,
                    0,
                );
                self.edit(
                    parent,
                    327,
                    "浮窗收起",
                    &seconds_text(c.theme.widget_duration_ms.max(5000)),
                    676,
                    0,
                );
            }
            _ => {
                self.section(parent, "游戏中的显示", 162);
                self.card(36, 192, 728, 124);
                self.edit(
                    parent,
                    330,
                    "Game Bar 组件",
                    &c.gamebar_package_family,
                    192,
                    0,
                );
                self.label(parent, "留空使用桌面浮层。", 58, 268, 680, 28);
                self.label(parent,"独占全屏游戏可尝试单独安装 Game Bar 组件。\r\n安装后需手动固定组件，并启用点击穿透。\r\n不同游戏的显示效果需要分别确认。",58,346,680,100);
            }
        }
        let footer = 760;
        self.button(parent, PREVIEW, "预览效果", 36, footer, 170, 44);
        self.button(parent, CANCEL, "取消", 508, footer, 106, 44);
        if self.onboarding && self.page == 1 {
            self.button(parent, NEXT, "下一步  ›", 632, footer, 132, 44);
        } else {
            self.button(
                parent,
                SAVE,
                if self.onboarding {
                    "完成设置"
                } else {
                    "完成"
                },
                632,
                footer,
                132,
                44,
            );
        }
        for (id, hwnd) in &self.controls {
            if let Some(field) = self.fields.get(id) {
                match field {
                    Field::Text(text) => {
                        let text = wide(text);
                        let _ = SetWindowTextW(*hwnd, PCWSTR(text.as_ptr()));
                    }
                    Field::Choice(index) => {
                        SendMessageW(*hwnd, CB_SETCURSEL, WPARAM(*index), LPARAM(0));
                    }
                    Field::Check(_) => (),
                }
            }
        }
        if self.page == 6 {
            let _ = EnableWindow(
                self.hwnd(317),
                !index_provider(self.selection(310)).is_decision(),
            );
        }
        if self.page == 7 {
            let _ = EnableWindow(self.hwnd(327), !self.checked(329));
        }
        self.building = false;
        let focus = self.hwnd(PAGE_BASE + self.root_page());
        let _ = SetFocus(focus);
        let _ = InvalidateRect(parent, None, true);
    }
    unsafe fn rect(&self, x: i32, y: i32, width: i32, height: i32) -> RECT {
        let p = |n: i32| (n as f32 * self.scale) as i32;
        RECT {
            left: p(x),
            top: p(y),
            right: p(x + width),
            bottom: p(y + height),
        }
    }
    unsafe fn paint_background(&self, hwnd: HWND, dc: HDC) {
        let mut all = RECT::default();
        let _ = GetClientRect(hwnd, &mut all);
        FillRect(dc, &all, self.style.background);
        let title = if self.onboarding {
            "欢迎使用 BlinkVerdict"
        } else {
            "BlinkVerdict"
        };
        self.style.text(
            dc,
            title,
            self.rect(100, 24, 600, 36),
            self.style.title,
            visual::TEXT,
            DT_CENTER | DT_SINGLELINE | DT_VCENTER,
        );
        for (x, y, w, h) in &self.cards {
            visual::rounded(
                dc,
                self.rect(*x, *y, *w, *h),
                visual::FIELD,
                visual::FIELD,
                (32.0 * self.scale) as i32,
            );
        }
        for (id, y) in &self.rows {
            visual::icon(
                dc,
                (58.0 * self.scale) as i32,
                ((*y + 20) as f32 * self.scale) as i32,
                self.scale,
                *id,
            );
            if self.rows.iter().any(|(_, other)| *other == *y + 62) {
                let brush = CreateSolidBrush(visual::LINE);
                FillRect(dc, &self.rect(94, *y + 61, 646, 1), brush);
                let _ = DeleteObject(brush);
            }
        }
        if self.page == 2 {
            let text = if let Some(Field::Text(s)) = self.fields.get(&303) {
                s.lines().find(|s| !s.trim().is_empty()).unwrap_or("YES")
            } else {
                self.draft
                    .options
                    .first()
                    .map(String::as_str)
                    .unwrap_or("YES")
            };
            self.style.text(
                dc,
                text,
                self.rect(62, 200, 676, 66),
                self.style.title,
                visual::TEXT,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            self.style.text(
                dc,
                "选择喜欢的方式，让答案自然浮现。",
                self.rect(62, 280, 676, 26),
                self.style.small,
                visual::MUTED,
                DT_CENTER | DT_SINGLELINE,
            );
        }
    }
    unsafe fn draw_item(&self, item: &DRAWITEMSTRUCT) {
        let id = item.CtlID as usize;
        let tab = matches!(id, 210..=212);
        let selected = tab && id == PAGE_BASE + self.root_page();
        let pressed = item.itemState.0 & ODS_SELECTED.0 != 0;
        let disabled = item.itemState.0 & ODS_DISABLED.0 != 0;
        let primary = id == SAVE || id == NEXT;
        let combo = item.CtlType == ODT_COMBOBOX;
        let background = if combo || id == 329 {
            visual::FIELD
        } else {
            visual::BACKGROUND
        };
        let clear = CreateSolidBrush(background);
        FillRect(item.hDC, &item.rcItem, clear);
        let _ = DeleteObject(clear);
        let fill = if primary {
            visual::TEXT
        } else if selected {
            visual::FIELD
        } else if pressed {
            COLORREF(0x00E9E6E3)
        } else if matches!(
            id,
            BACK | CLOSE | 215 | PREVIEW | 213 | 214 | 217 | 218 | EDIT_WIDGET | RESET_WIDGET
        ) {
            visual::FIELD
        } else {
            background
        };
        if primary || selected || pressed || (!tab && !combo && id != CANCEL && id != CLEAR_KEY) {
            visual::rounded(
                item.hDC,
                item.rcItem,
                fill,
                fill,
                ((if tab { 34.0 } else { 28.0 }) * self.scale) as i32,
            );
        }
        if id == 217 {
            let mut top = item.rcItem;
            top.bottom = (top.top + top.bottom) / 2;
            FillRect(item.hDC, &top, self.style.field);
        }
        let label = if combo {
            self.choice_labels
                .get(&id)
                .and_then(|v| v.get(item.itemID as usize))
                .cloned()
                .unwrap_or_default()
        } else {
            let len = GetWindowTextLengthW(item.hwndItem).max(0) as usize;
            let mut value = vec![0u16; len + 1];
            GetWindowTextW(item.hwndItem, &mut value);
            String::from_utf16_lossy(&value[..len])
        };
        let mut rect = item.rcItem;
        let row = id == 329 || matches!(id, 217 | 218 | EDIT_WIDGET);
        if row {
            visual::icon(
                item.hDC,
                rect.left + (22.0 * self.scale) as i32,
                rect.top + (20.0 * self.scale) as i32,
                self.scale,
                id,
            );
            rect.left += (58.0 * self.scale) as i32;
            rect.right -= (80.0 * self.scale) as i32;
        }
        if combo {
            rect.left += (12.0 * self.scale) as i32;
        }
        self.style.text(
            item.hDC,
            &label,
            rect,
            if matches!(id, BACK | CLOSE) {
                self.style.title
            } else if primary {
                self.style.strong
            } else {
                self.style.font
            },
            if primary {
                visual::FIELD
            } else if disabled || id == CLEAR_KEY || (tab && !selected) {
                visual::MUTED
            } else {
                visual::TEXT
            },
            DT_SINGLELINE
                | DT_VCENTER
                | DT_END_ELLIPSIS
                | if row || combo { DT_LEFT } else { DT_CENTER },
        );
        if id == 329 {
            let r = item.rcItem;
            let x = r.right - (80.0 * self.scale) as i32;
            let y = r.top + (15.0 * self.scale) as i32;
            let checked = self.checked(329);
            let color = if checked {
                COLORREF(0x0064C735)
            } else {
                COLORREF(0x00DEDAD7)
            };
            visual::rounded(
                item.hDC,
                RECT {
                    left: x,
                    top: y,
                    right: x + (56.0 * self.scale) as i32,
                    bottom: y + (32.0 * self.scale) as i32,
                },
                color,
                color,
                (32.0 * self.scale) as i32,
            );
            let x = x + ((if checked { 27.0 } else { 3.0 }) * self.scale) as i32;
            visual::rounded(
                item.hDC,
                RECT {
                    left: x,
                    top: y + (3.0 * self.scale) as i32,
                    right: x + (26.0 * self.scale) as i32,
                    bottom: y + (29.0 * self.scale) as i32,
                },
                visual::FIELD,
                visual::FIELD,
                (26.0 * self.scale) as i32,
            );
        }
        if item.itemState.0 & ODS_FOCUS.0 != 0 && item.itemState.0 & ODS_NOFOCUSRECT.0 == 0 {
            let mut r = item.rcItem;
            let _ = InflateRect(&mut r, -4, -4);
            let _ = DrawFocusRect(item.hDC, &r);
        }
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
        if hwnd.is_invalid() {
            return match self.fields.get(&id) {
                Some(Field::Text(text)) => text.clone(),
                _ => String::new(),
            };
        }
        let count = GetWindowTextLengthW(hwnd).max(0) as usize;
        let mut buffer = vec![0u16; count + 1];
        let len = GetWindowTextW(hwnd, &mut buffer).max(0) as usize;
        String::from_utf16_lossy(&buffer[..len])
    }
    unsafe fn checked(&self, id: usize) -> bool {
        matches!(self.fields.get(&id), Some(Field::Check(true)))
    }
    unsafe fn cache(&mut self) {
        for (id, _) in &self.controls {
            if *id < 300 {
                continue;
            }
            let value = if matches!(*id, 300 | 310 | 311 | 315 | 328 | 340 | 341) {
                Field::Choice(self.selection(*id))
            } else if *id == 329 {
                Field::Check(self.checked(*id))
            } else if *id == 318 || *id == 342 {
                continue;
            } else {
                Field::Text(self.text(*id))
            };
            self.fields.insert(*id, value);
        }
    }
    unsafe fn selection(&self, id: usize) -> usize {
        if self.hwnd(id).is_invalid() {
            return match self.fields.get(&id) {
                Some(Field::Choice(index)) => *index,
                _ => 0,
            };
        }
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

    unsafe fn configuration(&mut self, style_only: bool) -> Result<Config, (usize, String)> {
        self.cache();
        let mut c = self.draft.clone();
        let pages: Vec<_> = self.visited.iter().copied().collect();
        for page in pages {
            if style_only && !matches!(page, 2 | 3 | 4 | 7) {
                continue;
            }
            self.read_page(page, &mut c)
                .and_then(|_| c.validate())
                .map_err(|message| (page, message))?;
        }
        if style_only && self.visited.contains(&0) {
            c.mode = if self.selection(300) == 0 {
                DisplayMode::Fullscreen
            } else {
                DisplayMode::Widget
            };
        }
        Ok(c)
    }

    unsafe fn read_page(&mut self, page: usize, c: &mut Config) -> Result<(), String> {
        match page {
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
                c.endpoint = self.text(312).trim().into();
                c.model = self.text(313).trim().into();
                let key = self.text(314);
                self.pending_key = if !key.trim().is_empty() {
                    Some(key.trim().into())
                } else if self.pending_key.as_deref() == Some("") {
                    Some(String::new()) // Only the explicit delete action means deletion.
                } else {
                    None
                };
                c.vision = self.selection(315) == 0;
            }
            2 => {
                if self.checked(329) {
                    c.theme.widget_duration_ms = 0;
                } else if c.theme.widget_duration_ms == 0 {
                    c.theme.widget_duration_ms = 5000;
                }
                c.theme.animation = match self.selection(328) {
                    0 => Animation::None,
                    2 => Animation::Flash,
                    3 => Animation::Pop,
                    4 => Animation::Slide,
                    _ => Animation::Fade,
                };
            }
            6 => {
                c.provider = index_provider(self.selection(310));
                c.timeout_seconds = self.number(316)?;
                c.output_tokens = self.number(317)?;
            }
            7 => {
                c.theme.font = self.text(320).trim().into();
                c.theme.text_color = self.text(321).trim().into();
                c.theme.background_color = self.text(322).trim().into();
                let opacity = self
                    .text(323)
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "请填写 12–100 之间的不透明度".to_string())?;
                if !opacity.is_finite() || !(12.0..=100.0).contains(&opacity) {
                    return Err("不透明度应为 12–100%".into());
                }
                c.theme.opacity = (opacity * 255.0 / 100.0).round() as u8;
                c.theme.fullscreen_font_size = self.number(324)?;
                c.theme.widget_font_size = self.number(325)?;
                let seconds = |id| -> Result<u32, String> {
                    let n = self
                        .text(id)
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| "请以秒填写停留时间，例如 1.5".to_string())?;
                    if !n.is_finite() || !(0.0..=600.0).contains(&n) {
                        return Err("停留时间应为 0–600 秒".into());
                    }
                    Ok((n * 1000.0).round() as u32)
                };
                c.theme.fullscreen_duration_ms = seconds(326)?;
                if !self.checked(329) {
                    c.theme.widget_duration_ms = seconds(327)?;
                    if c.theme.widget_duration_ms == 0 {
                        return Err("请开启持续显示，或填写大于 0 的浮窗停留时间".into());
                    }
                }
            }
            3 => {
                c.theme.art_style = match self.selection(340) {
                    1 => ArtStyle::Ink,
                    2 => ArtStyle::Cyberpunk,
                    3 => ArtStyle::Graffiti,
                    _ => ArtStyle::Plain,
                };
            }
            4 => {
                let coordinate = |id| {
                    self.text(id)
                        .trim()
                        .parse::<i32>()
                        .map_err(|_| "位置和尺寸请填写有效整数".to_string())
                };
                c.widget_x = coordinate(350)?;
                c.widget_y = coordinate(351)?;
                c.widget_width = coordinate(352)?;
                c.widget_height = coordinate(353)?;
            }
            _ => {
                c.gamebar_package_family = self.text(330).trim().into();
            }
        }
        Ok(())
    }

    unsafe fn preset(&mut self, parent: HWND) {
        let selected = self.selection(311);
        let Some(index) = selected.checked_sub(1) else {
            return;
        };
        self.cache();
        let (provider, endpoint, model, vision) = match index {
            5 => (
                Provider::OpenAi,
                "https://api.deepseek.com/v1",
                "deepseek-chat",
                false,
            ),
            6 => (
                Provider::OpenAi,
                "https://dashscope.aliyuncs.com/compatible-mode/v1",
                "qwen-vl-plus",
                true,
            ),
            7 => (
                Provider::OpenAi,
                "https://api.siliconflow.cn/v1",
                "Qwen/Qwen2.5-VL-72B-Instruct",
                true,
            ),
            8 => (
                Provider::OpenAi,
                "https://openrouter.ai/api/v1",
                "openai/gpt-4o-mini",
                true,
            ),
            9 => (
                Provider::OpenAi,
                "http://127.0.0.1:1234/v1",
                "local-model",
                true,
            ),
            10 => {
                let (e, m) = Provider::Anthropic.defaults();
                (Provider::Anthropic, e, m, true)
            }
            11 => {
                let (e, m) = Provider::Gemini.defaults();
                (Provider::Gemini, e, m, true)
            }
            12 => {
                let (e, m) = Provider::Ollama.defaults();
                (Provider::Ollama, e, m, true)
            }
            13 => (
                Provider::SystemOne,
                "http://127.0.0.1:8000/v1",
                "local-decision-model",
                false,
            ),
            14 => (
                Provider::Decider,
                "http://127.0.0.1:8000/v1",
                "local-decision-model",
                true,
            ),
            15 => (
                Provider::OpenAiDecisions,
                "http://127.0.0.1:8000/v1",
                "local-decision-model",
                false,
            ),
            0..=3 => {
                let p = index_provider(index);
                let (e, m) = p.defaults();
                (p, e, m, true)
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
        self.fields
            .insert(310, Field::Choice(provider_index(provider)));
        self.fields.insert(312, Field::Text(endpoint.into()));
        self.fields.insert(313, Field::Text(model.into()));
        self.fields.insert(314, Field::Text(String::new()));
        self.fields
            .insert(315, Field::Choice(if vision { 0 } else { 1 }));
        self.build(parent);
        let _ = SetFocus(self.hwnd(if endpoint.starts_with("http://127.0.0.1") {
            313
        } else {
            314
        }));
    }

    unsafe fn image(&mut self, parent: HWND, remove: bool) {
        let options = if let Some(Field::Text(text)) = self.fields.get(&303) {
            text.lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            self.draft.options.clone()
        };
        let Some(label) = options.get(self.selection(341)).cloned() else {
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

const PROVIDERS: [Provider; 10] = [
    Provider::Jev,
    Provider::Decider,
    Provider::Clef,
    Provider::OpenAiDecisions,
    Provider::SystemOne,
    Provider::OpenAi,
    Provider::OpenAiResponses,
    Provider::Anthropic,
    Provider::Gemini,
    Provider::Ollama,
];

fn provider_index(provider: Provider) -> usize {
    PROVIDERS.iter().position(|p| *p == provider).unwrap_or(0)
}
fn index_provider(index: usize) -> Provider {
    PROVIDERS.get(index).copied().unwrap_or(Provider::Jev)
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

unsafe fn form_error(app: &mut App, page: usize, message: &str) {
    let ui = app.settings_ui.as_mut().unwrap();
    ui.page = page;
    ui.build(app.settings);
    error(app.settings, message);
}

unsafe fn save(app: &mut App, close: bool) -> bool {
    let ui = app.settings_ui.as_mut().unwrap();
    let config = match ui.configuration(false) {
        Ok(config) => config,
        Err((page, message)) => {
            form_error(app, page, &message);
            return false;
        }
    };
    let key = ui.pending_key.clone();
    if !app.paused && (config.hotkey != app.config.hotkey || !app.hotkey_active) {
        let _ = UnregisterHotKey(app.host, HOTKEY_ID);
        if let Err(message) = app.register_hotkey(&config.hotkey) {
            app.hotkey_active = app.register_hotkey(&app.config.hotkey).is_ok();
            form_error(app, 0, &message);
            return false;
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
        app.hotkey_active = !app.paused && app.register_hotkey(&app.config.hotkey).is_ok();
        error(app.settings, &message);
        return false;
    }
    app.hide();
    app.config = config.clone();
    app.hotkey_active = !app.paused;
    app.set_status(
        if app.paused {
            "已保存 · 快捷键已暂停"
        } else {
            "已准备好，按下快捷键即可开始"
        }
        .into(),
    );
    let ui = app.settings_ui.as_mut().unwrap();
    ui.draft = config;
    ui.pending_key = None;
    ui.fields.insert(314, Field::Text(String::new()));
    if close {
        let _ = DestroyWindow(app.settings);
    } else {
        ui.build(app.settings);
    }
    true
}

unsafe fn preview(app: &mut App) {
    let ui = app.settings_ui.as_mut().unwrap();
    let config = match ui.configuration(true) {
        Ok(config) => config,
        Err((page, message)) => {
            form_error(app, page, &message);
            return;
        }
    };
    let text = ui
        .fields
        .get(&303)
        .and_then(|value| {
            if let Field::Text(s) = value {
                s.lines()
                    .find(|s| !s.trim().is_empty())
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| config.options[0].clone());
    // A short native preview never needs model credentials, capture, or a Game Bar connection.
    app.show(
        text,
        capture::foreground_rect(),
        Some(Duration::from_secs(4)),
        true,
        Some(config),
    );
}

unsafe fn request_close(app: &mut App) {
    let ui = app.settings_ui.as_mut().unwrap();
    let changed = match ui.configuration(false) {
        Ok(config) => config != app.config || ui.pending_key.is_some(),
        Err(_) => true,
    };
    if changed {
        match MessageBoxW(
            app.settings,
            w!("要保存这次更改吗？\n选择“是”保存并关闭，“否”放弃更改，或“取消”继续设置。"),
            w!("BlinkVerdict · 未保存的修改"),
            MB_YESNOCANCEL | MB_ICONQUESTION,
        ) {
            IDYES => {
                save(app, true);
                return;
            }
            IDNO => (),
            _ => return,
        }
    }
    let _ = DestroyWindow(app.settings);
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
        WM_NCCALCSIZE => LRESULT(0),
        WM_NCHITTEST => {
            let mut point = POINT {
                x: (lparam.0 as u32 as u16 as i16) as i32,
                y: ((lparam.0 as u32 >> 16) as u16 as i16) as i32,
            };
            let _ = ScreenToClient(hwnd, &mut point);
            let ui = app.settings_ui.as_ref().unwrap();
            LRESULT(
                if point.y < (82.0 * ui.scale) as i32 && point.x < (670.0 * ui.scale) as i32 {
                    HTCAPTION as isize
                } else {
                    HTCLIENT as isize
                },
            )
        }
        WM_COMMAND => {
            let id = wparam.0 & 0xffff;
            // Owner-drawn buttons also notify focus changes. Only a click is an action.
            if app.settings_ui.as_ref().is_some_and(|ui| ui.building)
                || (id < 300 && wparam.0 >> 16 != BN_CLICKED as usize)
            {
                return LRESULT(0);
            }
            if wparam.0 >> 16 == CBN_SELCHANGE as usize
                && app
                    .settings_ui
                    .as_ref()
                    .unwrap()
                    .choice_labels
                    .contains_key(&id)
            {
                let ui = app.settings_ui.as_mut().unwrap();
                let index = ui.selection(id);
                ui.fields.insert(id, Field::Choice(index));
                let _ = InvalidateRect(ui.hwnd(id), None, false);
            }
            match id {
                NEXT => {
                    if let Err((page, message)) =
                        app.settings_ui.as_mut().unwrap().configuration(false)
                    {
                        form_error(app, page, &message);
                        return LRESULT(0);
                    }
                    let ui = app.settings_ui.as_mut().unwrap();
                    ui.cache();
                    ui.page = 0;
                    ui.build(hwnd);
                }
                BACK => {
                    let ui = app.settings_ui.as_mut().unwrap();
                    ui.cache();
                    ui.page = ui.root_page();
                    ui.build(hwnd);
                }
                TEXT_COLOR | BACKGROUND_COLOR => {
                    let ui = app.settings_ui.as_mut().unwrap();
                    let field = if id == TEXT_COLOR { 321 } else { 322 };
                    let [r, g, b] = blink_verdict::config::parse_color(&ui.text(field))
                        .unwrap_or([100, 100, 100]);
                    let mut custom = [COLORREF(0); 16];
                    let mut picker = CHOOSECOLORW {
                        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
                        hwndOwner: hwnd,
                        rgbResult: COLORREF(r as u32 | ((g as u32) << 8) | ((b as u32) << 16)),
                        lpCustColors: custom.as_mut_ptr(),
                        Flags: CC_FULLOPEN | CC_RGBINIT,
                        ..Default::default()
                    };
                    if ChooseColorW(&mut picker).as_bool() {
                        let c = picker.rgbResult.0;
                        let value = wide(&format!(
                            "{:02X}{:02X}{:02X}",
                            c & 255,
                            (c >> 8) & 255,
                            (c >> 16) & 255
                        ));
                        let _ = SetWindowTextW(ui.hwnd(field), PCWSTR(value.as_ptr()));
                    }
                }
                1 if app
                    .settings_ui
                    .as_ref()
                    .is_some_and(|ui| ui.onboarding && ui.page == 1) =>
                {
                    SendMessageW(hwnd, WM_COMMAND, WPARAM(NEXT), LPARAM(0));
                }
                SAVE | 1 => {
                    save(app, true);
                }
                APPLY => {
                    save(app, false);
                }
                PREVIEW => preview(app),
                CANCEL => {
                    let _ = DestroyWindow(hwnd);
                }
                CLOSE => request_close(app),
                2 => {
                    if app.presentation.is_some() {
                        app.hide();
                    } else {
                        request_close(app);
                    }
                }
                EDIT_WIDGET => {
                    if save(app, true) {
                        app.command(CMD_EDIT);
                    }
                }
                311 if wparam.0 >> 16 == CBN_SELCHANGE as usize => {
                    app.settings_ui.as_mut().unwrap().preset(hwnd)
                }
                PICK_IMAGE | REMOVE_IMAGE => app
                    .settings_ui
                    .as_mut()
                    .unwrap()
                    .image(hwnd, id == REMOVE_IMAGE),
                310 | 312
                    if (id == 310 && wparam.0 >> 16 == CBN_SELCHANGE as usize)
                        || (id == 312 && wparam.0 >> 16 == EN_CHANGE as usize) =>
                {
                    let ui = app.settings_ui.as_mut().unwrap();
                    if !ui.building {
                        ui.pending_key = None;
                        ui.fields.insert(314, Field::Text(String::new()));
                        let _ = SetWindowTextW(ui.hwnd(314), w!(""));
                        let _ = SetWindowTextW(
                            ui.hwnd(318),
                            w!("密钥保存在此设备。连接变更后，请输入对应服务的密钥。"),
                        );
                        SendMessageW(ui.hwnd(311), CB_SETCURSEL, WPARAM(0), LPARAM(0));
                        ui.fields.insert(311, Field::Choice(0));
                        let _ = InvalidateRect(ui.hwnd(311), None, false);
                        let _ = EnableWindow(
                            ui.hwnd(317),
                            !index_provider(ui.selection(310)).is_decision(),
                        );
                    }
                }
                329 => {
                    let ui = app.settings_ui.as_mut().unwrap();
                    ui.fields.insert(329, Field::Check(!ui.checked(329)));
                    let _ = InvalidateRect(ui.hwnd(329), None, false);
                }
                341 if wparam.0 >> 16 == CBN_SELCHANGE as usize => {
                    let ui = app.settings_ui.as_ref().unwrap();
                    let options: Vec<_> = if let Some(Field::Text(text)) = ui.fields.get(&303) {
                        text.lines()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    } else {
                        ui.draft.options.clone()
                    };
                    let label = options.get(ui.selection(341));
                    let path = label
                        .and_then(|v| ui.draft.option_images.get(v))
                        .map(String::as_str)
                        .unwrap_or("");
                    let value = wide(path);
                    let _ = SetWindowTextW(ui.hwnd(342), PCWSTR(value.as_ptr()));
                }
                RESET_WIDGET => {
                    let ui = app.settings_ui.as_mut().unwrap();
                    ui.cache();
                    let defaults = Config::default();
                    ui.draft.widget_x = defaults.widget_x;
                    ui.draft.widget_y = defaults.widget_y;
                    ui.draft.widget_width = defaults.widget_width;
                    ui.draft.widget_height = defaults.widget_height;
                    for (id, value) in [
                        (350, defaults.widget_x),
                        (351, defaults.widget_y),
                        (352, defaults.widget_width),
                        (353, defaults.widget_height),
                    ] {
                        ui.fields.insert(id, Field::Text(value.to_string()));
                    }
                    ui.build(hwnd);
                }
                CLEAR_KEY => {
                    let ui = app.settings_ui.as_mut().unwrap();
                    ui.pending_key = Some(String::new());
                    let _ = SetWindowTextW(ui.hwnd(314), w!(""));
                    ui.fields.insert(314, Field::Text(String::new()));
                    let _ = SetWindowTextW(ui.hwnd(318), w!("密钥将在完成设置后移除。"));
                }
                value if (PAGE_BASE..PAGE_BASE + 9).contains(&value) => {
                    let ui = app.settings_ui.as_mut().unwrap();
                    ui.cache();
                    ui.page = value - PAGE_BASE;
                    ui.build(hwnd);
                }
                _ => (),
            }
            LRESULT(0)
        }
        WM_MEASUREITEM => {
            let item = &mut *(lparam.0 as *mut MEASUREITEMSTRUCT);
            if let Some(ui) = &app.settings_ui {
                item.itemHeight = (28.0 * ui.scale) as u32;
            }
            LRESULT(1)
        }
        WM_DRAWITEM => {
            let item = &*(lparam.0 as *const DRAWITEMSTRUCT);
            if let Some(ui) = &app.settings_ui {
                ui.draw_item(item);
            }
            LRESULT(1)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN => {
            if let Some(ui) = &app.settings_ui {
                let dc = HDC(wparam.0 as *mut _);
                SetTextColor(
                    dc,
                    if message == WM_CTLCOLORSTATIC {
                        if GetDlgCtrlID(HWND(lparam.0 as *mut _)) == 11 {
                            visual::TEXT
                        } else {
                            visual::MUTED
                        }
                    } else {
                        visual::TEXT
                    },
                );
                SetBkColor(
                    dc,
                    if matches!(message, WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX)
                        || GetDlgCtrlID(HWND(lparam.0 as *mut _)) == 11
                    {
                        visual::FIELD
                    } else {
                        visual::BACKGROUND
                    },
                );
                return LRESULT(if matches!(message, WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX)
                    || GetDlgCtrlID(HWND(lparam.0 as *mut _)) == 11
                {
                    ui.style.field.0
                } else {
                    ui.style.background.0
                } as isize);
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            // Paint in a client-origin DC; erase notifications may carry a window-origin DC.
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut paint);
            if let Some(ui) = &app.settings_ui {
                ui.paint_background(hwnd, dc);
            }
            let _ = EndPaint(hwnd, &paint);
            LRESULT(0)
        }
        WM_CLOSE => {
            request_close(app);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            app.settings = HWND::default();
            app.settings_ui = None;
            app.hide();
            app.restore_widget();
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

fn seconds_text(ms: u32) -> String {
    let s = format!("{:.3}", ms as f64 / 1000.0);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

unsafe extern "system" fn control_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    if message == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(control_proc), id);
        return DefSubclassProc(hwnd, message, wparam, lparam);
    }
    if id == 301 && matches!(message, WM_CHAR | WM_SYSCHAR) {
        return LRESULT(0);
    }
    if id == 301 && matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
        let key = wparam.0 as u16;
        let name = match key {
            0x41..=0x5A | 0x30..=0x39 => (key as u8 as char).to_string(),
            0x70..=0x87 => format!("F{}", key - 0x70 + 1),
            0x20 => "Space".into(),
            _ => return DefSubclassProc(hwnd, message, wparam, lparam),
        };
        let mut parts = Vec::new();
        for (vk, label) in [(0x11, "Ctrl"), (0x12, "Alt"), (0x10, "Shift")] {
            if GetKeyState(vk) < 0 {
                parts.push(label.to_string());
            }
        }
        if GetKeyState(0x5B) < 0 || GetKeyState(0x5C) < 0 {
            parts.push("Win".into());
        }
        parts.push(name);
        let value = wide(&parts.join("+"));
        let _ = SetWindowTextW(hwnd, PCWSTR(value.as_ptr()));
        return LRESULT(0);
    }
    if id != 301 && message == WM_PAINT && data != 0 {
        let app = &*(data as *const App);
        if let Some(ui) = &app.settings_ui {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut paint);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            FillRect(dc, &rect, ui.style.field);
            let index = SendMessageW(hwnd, CB_GETCURSEL, WPARAM(0), LPARAM(0))
                .0
                .max(0) as usize;
            let value = ui
                .choice_labels
                .get(&id)
                .and_then(|v| v.get(index))
                .map(String::as_str)
                .unwrap_or("");
            let mut text = rect;
            text.right -= (24.0 * ui.scale) as i32;
            text.left += (8.0 * ui.scale) as i32;
            ui.style.text(
                dc,
                value,
                text,
                ui.style.font,
                visual::MUTED,
                DT_SINGLELINE | DT_RIGHT | DT_VCENTER | DT_END_ELLIPSIS,
            );
            let x = rect.right - (14.0 * ui.scale) as i32;
            let y = (rect.bottom - rect.top) / 2;
            let pen = CreatePen(PS_SOLID, (1.5 * ui.scale).max(1.0) as i32, visual::MUTED);
            let old = SelectObject(dc, pen);
            let _ = Polyline(
                dc,
                &[
                    POINT {
                        x: x - (4.0 * ui.scale) as i32,
                        y: y - (2.0 * ui.scale) as i32,
                    },
                    POINT {
                        x,
                        y: y + (2.0 * ui.scale) as i32,
                    },
                    POINT {
                        x: x + (4.0 * ui.scale) as i32,
                        y: y - (2.0 * ui.scale) as i32,
                    },
                ],
            );
            SelectObject(dc, old);
            let _ = DeleteObject(pen);
            let _ = EndPaint(hwnd, &paint);
            return LRESULT(0);
        }
    }
    DefSubclassProc(hwnd, message, wparam, lparam)
}
