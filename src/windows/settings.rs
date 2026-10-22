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
const PAGE_BASE: usize = 210;

#[derive(Clone)]
enum Field {
    Text(String),
    Choice(usize),
    Check(bool),
}

pub(super) struct Settings {
    draft: Config,
    page: usize,
    controls: Vec<(usize, HWND)>,
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
    let style = Style::new(scale, &app.config.theme.font);
    app.settings_ui = Some(Settings {
        draft: app.config.clone(),
        page: 0,
        controls: Vec::new(),
        style,
        scale,
        pending_key: None,
        fields: Default::default(),
        visited: Default::default(),
        building: false,
        choice_labels: Default::default(),
    });
    let instance = GetModuleHandleW(None).unwrap_or_default();
    let hwnd = match CreateWindowExW(
        WS_EX_APPWINDOW | WS_EX_CONTROLPARENT,
        w!("BlinkVerdict.Settings"),
        w!("BlinkVerdict · 一瞬决断"),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        (960.0 * scale) as i32,
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
    SendMessageW(
        hwnd,
        WM_CHANGEUISTATE,
        WPARAM(UIS_SET as usize | ((UISF_HIDEFOCUS as usize) << 16)),
        LPARAM(0),
    );
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, app as *mut App as isize);
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
        let sidebar = (PAGE_BASE..PAGE_BASE + 6).contains(&id);
        let x = if sidebar { x } else { x + 200 };
        let button = class.as_wide() == w!("BUTTON").as_wide();
        let style = if button && id != 329 {
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
            WPARAM(self.style.font.0 as usize),
            LPARAM(1),
        );
        if id >= 300 {
            let _ = SetWindowTheme(hwnd, w!(""), w!(""));
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
        let style = WS_TABSTOP
            | if multiline > 0 {
                WS_VSCROLL | WINDOW_STYLE((ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN) as u32)
            } else {
                WINDOW_STYLE(ES_AUTOHSCROLL as u32)
            };
        let hwnd = self.control(
            parent,
            id,
            w!("EDIT"),
            value,
            style,
            210,
            y + 5,
            490,
            height - 9,
        );
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
        self.choice_labels.insert(
            id,
            values.iter().map(|value| (*value).to_string()).collect(),
        );
        let hwnd = self.control(
            parent,
            id,
            w!("COMBOBOX"),
            "",
            WS_TABSTOP
                | WS_VSCROLL
                | WINDOW_STYLE((CBS_DROPDOWNLIST | CBS_OWNERDRAWFIXED | CBS_HASSTRINGS) as u32),
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
        self.building = true;
        self.visited.insert(self.page);
        for (_, hwnd) in self.controls.drain(..) {
            let _ = DestroyWindow(hwnd);
        }

        for (index, title) in ["决策", "模型", "外观", "字效", "浮窗", "关于 / 状态"]
            .iter()
            .enumerate()
        {
            self.control(
                parent,
                PAGE_BASE + index,
                w!("BUTTON"),
                title,
                WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
                16,
                116 + index as i32 * 52,
                168,
                42,
            );
        }
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
                        "专用决策 · TypeSafe Jev",
                        "专用决策 · Perplexity Decider",
                        "专用决策 · Cloudflare Clef",
                        "专用决策 · OpenAI Decisions",
                        "本地决策 · System One / vLLM",
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
                        "当前配置 / 自定义",
                        "TypeSafe Jev（云端）",
                        "Perplexity Decider（云端）",
                        "Cloudflare Clef（填写 Account ID）",
                        "OpenAI Decisions（云端）",
                        "OpenAI",
                        "DeepSeek（文字）",
                        "通义千问 Qwen",
                        "SiliconFlow",
                        "OpenRouter",
                        "LM Studio（本机）",
                        "Anthropic",
                        "Google Gemini",
                        "Ollama（本机）",
                        "本地决策 · System One",
                        "本地决策 · Decider",
                        "本地决策 · OpenAI Decisions（文字）",
                    ],
                    0,
                    156,
                );
                self.label(
                    parent,
                    "选择预设自动填入；先填地址，再输入该服务的密钥。",
                    200,
                    198,
                    510,
                    32,
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
                self.control(
                    parent,
                    318,
                    w!("STATIC"),
                    if self.pending_key.as_deref() == Some("") {
                        "待删除密钥；应用或确定后生效，取消可放弃。"
                    } else {
                        "留空保留当前地址的密钥；切换协议或地址会清空未保存密钥。"
                    },
                    WINDOW_STYLE(0),
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
                    "兼容接口输出 tokens",
                    &c.output_tokens.to_string(),
                    522,
                    0,
                );
                let _ = EnableWindow(self.hwnd(317), !c.provider.is_decision());
                self.label(
                    parent,
                    "本地填 http://127.0.0.1:端口/版本路径，需运行匹配协议的服务，密钥可留空。\r\nJev 截图为实验能力；专用决策接口不使用输出 tokens。",
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
                        "结果后隐藏（毫秒）",
                        c.theme.widget_duration_ms.max(5000).to_string(),
                        406,
                    ),
                ] {
                    self.edit(parent, id, label, &value, y, 0);
                }
                let keep = self.control(
                    parent,
                    329,
                    w!("BUTTON"),
                    "悬浮常驻 · 不自动隐藏",
                    WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                    200,
                    450,
                    510,
                    28,
                );
                SendMessageW(
                    keep,
                    BM_SETCHECK,
                    WPARAM(if c.theme.widget_duration_ms == 0 {
                        1
                    } else {
                        0
                    }),
                    LPARAM(0),
                );
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
                    492,
                );
                self.label(parent, "勾选常驻后，悬浮窗口持续等待快捷键；不在后台截图或请求模型。\r\n预览仅展示当前外观 2 秒，不保存、不截图、不调用模型。", 28, 542, 680, 50);
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
                self.label(parent, "为 YES / NO 选择内置风格，或为自己的答案添加透明 PNG。\r\n图片与文字共用当前动画；预览不需要调用模型。", 28, 174, 680, 82);
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
            4 => {
                for (id, label, value, y) in [
                    (350, "横坐标 X（像素）", c.widget_x, 120),
                    (351, "纵坐标 Y（像素）", c.widget_y, 168),
                    (352, "宽度 160–1600", c.widget_width, 216),
                    (353, "高度 80–1000", c.widget_height, 264),
                ] {
                    self.edit(parent, id, label, &value.to_string(), y, 0);
                }
                self.control(
                    parent,
                    RESET_WIDGET,
                    w!("BUTTON"),
                    "恢复默认位置 / 尺寸",
                    WS_TABSTOP,
                    460,
                    324,
                    250,
                    32,
                );
                self.control(
                    parent,
                    EDIT_WIDGET,
                    w!("BUTTON"),
                    "应用并拖动调整",
                    WS_TABSTOP,
                    200,
                    324,
                    230,
                    32,
                );
                self.label(parent, "支持负坐标（左侧 / 上方显示器），显示时自动限制在可见工作区。\r\n也可从托盘进入“编辑悬浮窗口”：拖动内部移动，拖动边缘或角落缩放。\r\n右键浮窗或点击托盘锁定即可结束；拖放自动保存，结束后恢复点击穿透。\r\n日常显示和调整时均不主动激活窗口。Game Bar 的位置 / 大小由其面板调整。", 28, 390, 680, 150);
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
        for (id, title, x, width) in [
            (PREVIEW, "预览 · 不保存", 28, 190),
            (APPLY, "应用", 266, 132),
            (SAVE, "确定", 422, 132),
            (CANCEL, "取消", 578, 132),
        ] {
            self.control(
                parent,
                id,
                w!("BUTTON"),
                title,
                WS_TABSTOP
                    | if id == SAVE {
                        WINDOW_STYLE(BS_DEFPUSHBUTTON as u32)
                    } else {
                        WINDOW_STYLE(0)
                    },
                x,
                610,
                width,
                34,
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
                    Field::Check(checked) => {
                        SendMessageW(*hwnd, BM_SETCHECK, WPARAM(usize::from(*checked)), LPARAM(0));
                    }
                }
            }
        }
        if self.page == 1 {
            let _ = EnableWindow(
                self.hwnd(317),
                !index_provider(self.selection(310)).is_decision(),
            );
        }
        if self.page == 2 {
            let _ = EnableWindow(self.hwnd(327), !self.checked(329));
        }
        self.building = false;
        let selected = self.hwnd(PAGE_BASE + self.page);
        let _ = SetFocus(selected);
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
        if self.style.glass {
            FillRect(
                dc,
                &self.rect(0, 0, 200, 670),
                HBRUSH(GetStockObject(BLACK_BRUSH).0),
            );
        } else {
            visual::rounded(
                dc,
                self.rect(8, 8, 184, 644),
                visual::SIDEBAR,
                visual::SIDEBAR,
                24,
            );
        }
        self.style.text(
            dc,
            "BlinkVerdict",
            self.rect(28, 30, 160, 34),
            self.style.strong,
            visual::TEXT,
            DT_SINGLELINE | DT_VCENTER,
        );
        self.style.text(
            dc,
            "一瞬决断",
            self.rect(28, 65, 160, 24),
            self.style.small,
            visual::MUTED,
            DT_SINGLELINE,
        );
        self.style.text(
            dc,
            "按下快捷键\n让决策浮现",
            self.rect(28, 564, 155, 70),
            self.style.small,
            visual::MUTED,
            DT_WORDBREAK,
        );
        let (title, subtitle) = [
            (
                "让判断，简单一点",
                "设置目标、选项和快捷键，然后继续眼前的事。",
            ),
            ("连接你的模型", "云端 API 或本地服务，选择适合你的方式。"),
            (
                "只留下需要的表达",
                "字体、节奏与动画，两种显示方式共享一套风格。",
            ),
            ("让答案更有个性", "原生文字，或为每个答案选择一张透明字效。"),
            ("放在顺手的位置", "精确输入尺寸，或直接拖动调整。"),
            ("安静地在后台", "只在你需要时，给出一次决策。"),
        ][self.page];
        self.style.text(
            dc,
            title,
            self.rect(228, 28, 660, 40),
            self.style.title,
            visual::TEXT,
            DT_SINGLELINE | DT_VCENTER,
        );
        self.style.text(
            dc,
            subtitle,
            self.rect(228, 76, 680, 26),
            self.style.small,
            visual::MUTED,
            DT_SINGLELINE,
        );
        // Rounded input backplates keep real native edit controls, keyboard navigation and IME.
        for (id, hwnd) in &self.controls {
            if matches!(*id, 301..=303 | 312..=314 | 316..=317 | 320..=327 | 330 | 342 | 350..=353)
            {
                let mut r = RECT::default();
                let _ = GetWindowRect(*hwnd, &mut r);
                let mut origin = POINT {
                    x: r.left,
                    y: r.top,
                };
                let _ = ScreenToClient(GetParent(*hwnd).unwrap_or_default(), &mut origin);
                r = RECT {
                    left: origin.x - (10.0 * self.scale) as i32,
                    top: origin.y - (5.0 * self.scale) as i32,
                    right: origin.x + r.right - r.left + (10.0 * self.scale) as i32,
                    bottom: origin.y + r.bottom - r.top + (4.0 * self.scale) as i32,
                };
                visual::rounded(
                    dc,
                    r,
                    visual::FIELD,
                    COLORREF(0x0044372D),
                    (16.0 * self.scale) as i32,
                );
            }
        }
    }
    unsafe fn draw_item(&self, item: &DRAWITEMSTRUCT) {
        let id = item.CtlID as usize;
        let nav = (PAGE_BASE..PAGE_BASE + 6).contains(&id);
        let selected = nav && id == PAGE_BASE + self.page;
        let pressed = item.itemState.0 & ODS_SELECTED.0 != 0;
        let disabled = item.itemState.0 & ODS_DISABLED.0 != 0;
        let primary = id == SAVE;
        let fill = if primary {
            COLORREF(0x00E28D40)
        } else if selected || pressed {
            COLORREF(0x004C3828)
        } else {
            visual::FIELD
        };
        let backdrop = if nav { if self.style.glass { COLORREF(0) } else { visual::SIDEBAR } } else { visual::BACKGROUND };
        let clear = CreateSolidBrush(backdrop);
        FillRect(item.hDC, &item.rcItem, clear);
        let _ = DeleteObject(clear);
        if nav && !selected && !pressed {
            // The unselected sidebar row shares the material; only its text is drawn.
        } else {
                visual::SIDEBAR
            };
            visual::rounded(
                item.hDC,
                item.rcItem,
                background,
                background,
                (20.0 * self.scale) as i32,
            );
        } else {
            visual::rounded(
                item.hDC,
                item.rcItem,
                fill,
                if selected || primary {
                    visual::ACCENT
                } else {
                    COLORREF(0x0044372D)
                },
                (20.0 * self.scale) as i32,
            );
        }
        let mut rect = item.rcItem;
        if nav {
            rect.left += (18.0 * self.scale) as i32;
        }
        let label = if item.CtlType == ODT_COMBOBOX {
            // Never query the combo synchronously while Windows is painting that combo.
            self.choice_labels
                .get(&id)
                .and_then(|values| values.get(item.itemID as usize))
                .cloned()
                .unwrap_or_default()
        } else {
            let len = GetWindowTextLengthW(item.hwndItem).max(0) as usize;
            let mut text = vec![0u16; len + 1];
            GetWindowTextW(item.hwndItem, &mut text);
            String::from_utf16_lossy(&text[..len])
        };
        if item.CtlType == ODT_COMBOBOX {
            rect.left += (12.0 * self.scale) as i32;
        }
        self.style.text(
            item.hDC,
            &label,
            rect,
            if nav || primary {
                self.style.strong
            } else {
                self.style.font
            },
            if disabled {
                visual::MUTED
            } else {
                visual::TEXT
            },
            DT_SINGLELINE
                | DT_VCENTER
                | DT_END_ELLIPSIS
                | if nav || item.CtlType == ODT_COMBOBOX {
                    DT_LEFT
                } else {
                    DT_CENTER
                },
        );
        if item.itemState.0 & ODS_FOCUS.0 != 0 && item.itemState.0 & ODS_NOFOCUSRECT.0 == 0 {
            let mut focus = item.rcItem;
            let _ = InflateRect(&mut focus, -4, -4);
            let _ = DrawFocusRect(item.hDC, &focus);
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
        let hwnd = self.hwnd(id);
        if hwnd.is_invalid() {
            return matches!(self.fields.get(&id), Some(Field::Check(true)));
        }
        SendMessageW(hwnd, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == 1
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
            if style_only && !matches!(page, 2 | 3 | 4) {
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
                c.provider = index_provider(self.selection(310));
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
                c.theme.widget_duration_ms = if self.checked(329) {
                    0
                } else {
                    let duration = self.number(327)?;
                    if duration == 0 {
                        return Err("请选择“悬浮常驻”，或填写大于 0 的隐藏时长".into());
                    }
                    duration
                };
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
            "设置已应用；快捷键仍暂停"
        } else {
            "设置已应用，等待快捷键"
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
        Some(Duration::from_secs(2)),
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
            w!("是否保存本次修改？\n“是”保存并关闭，“否”放弃修改，“取消”继续编辑。"),
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
        WM_COMMAND => {
            let id = wparam.0 & 0xffff;
            // Owner-drawn buttons also notify focus changes. Only a click is an action.
            if app.settings_ui.as_ref().is_some_and(|ui| ui.building)
                || (id < 300 && wparam.0 >> 16 != BN_CLICKED as usize)
            {
                return LRESULT(0);
            }
            match id {
                SAVE | 1 => {
                    save(app, true);
                }
                APPLY => {
                    save(app, false);
                }
                PREVIEW => preview(app),
                CANCEL => request_close(app),
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
                            w!("留空保留当前地址的密钥；切换协议或地址会清空未保存密钥。"),
                        );
                        SendMessageW(ui.hwnd(311), CB_SETCURSEL, WPARAM(0), LPARAM(0));
                        let _ = EnableWindow(
                            ui.hwnd(317),
                            !index_provider(ui.selection(310)).is_decision(),
                        );
                    }
                }
                329 => {
                    let ui = app.settings_ui.as_ref().unwrap();
                    let _ = EnableWindow(ui.hwnd(327), !ui.checked(329));
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
                    let _ = SetWindowTextW(
                        ui.hwnd(318),
                        w!("待删除密钥；应用或确定后生效，取消可放弃。"),
                    );
                }
                value if (PAGE_BASE..PAGE_BASE + 6).contains(&value) => {
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
                        visual::MUTED
                    } else {
                        visual::TEXT
                    },
                );
                SetBkColor(
                    dc,
                    if matches!(message, WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX) {
                        visual::FIELD
                    } else {
                        visual::BACKGROUND
                    },
                );
                return LRESULT(if matches!(message, WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX) {
                    ui.style.field.0
                } else {
                    ui.style.background.0
                } as isize);
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => {
            if let Some(ui) = &app.settings_ui {
                ui.paint_background(hwnd, HDC(wparam.0 as *mut _));
            }
            LRESULT(1)
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
