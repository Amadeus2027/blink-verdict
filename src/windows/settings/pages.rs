//! Page layouts built only when the settings window is opened or navigated.
use super::*;

impl Settings {
    pub(super) fn root_page(&self) -> usize {
        match self.page {
            3 | 4 | 7 => 2,
            6 => 1,
            5 | 8 => 0,
            p => p,
        }
    }
    pub(super) unsafe fn build(&mut self, parent: HWND) {
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
                self.edit(parent, 303, "答案选项", &c.options.join("\r\n"), 564, 140);
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
                        Provider::LlamaCpp => 17,
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
                        "llama.cpp · 本机",
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
                self.card(36, 174, 728, 120);
                self.section(parent, "显示效果", 320);
                self.card(36, 350, 728, 248);
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
                    350,
                );
                self.rows.push((329, 412));
                self.rows.push((217, 536));
                self.button(parent, 329, "持续显示浮窗", 36, 412, 728, 62);
                self.edit(
                    parent,
                    332,
                    "浮窗背景 · 不透明度 %",
                    &format!(
                        "{:.1}",
                        c.theme.widget_background_opacity as f32 * 100.0 / 255.0
                    ),
                    474,
                    0,
                );
                self.button(parent, 217, "颜色与时长  ›", 36, 536, 728, 62);
                self.label(
                    parent,
                    "全屏只浮现文字；背景仅用于常驻浮窗。",
                    58,
                    612,
                    680,
                    26,
                );
                self.button(parent, 213, "字效与图片  ›", 36, 650, 354, 64);
                self.button(parent, 214, "位置与大小  ›", 410, 650, 354, 64);
            }
            3 => {
                self.section(parent, "答案的表达", 162);
                self.card(36, 192, 728, 62);
                self.combo(
                    parent,
                    340,
                    "文字风格",
                    &["简洁文字", "清晰纯色", "流光玻璃", "柔彩手写"],
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
                self.card(36, 192, 728, 248);
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
                        "llama.cpp",
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
                self.combo(
                    parent,
                    333,
                    "Ollama · 模型驻留",
                    &["遵循服务设置", "用后释放模型"],
                    usize::from(c.ollama_unload_after_request),
                    378,
                );
                self.label(parent,"通常保留默认值即可。专用决策模型不使用输出长度设置。\r\n模型驻留仅对 Ollama 生效，由服务决定保留多久。\r\n用后释放可减少模型驻留资源，但下次可能需要重新加载。",58,458,680,90);
            }
            7 => {
                self.section(parent, "颜色与排版", 162);
                self.card(36, 192, 728, 372);
                for (id, label, value, y) in [
                    (320, "文字字体", c.theme.font, 192),
                    (321, "文字颜色", c.theme.text_color, 254),
                    (322, "浮窗背景颜色", c.theme.background_color, 316),
                    (
                        323,
                        "整体不透明度 · %",
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
                self.button(parent, FONT, "选择字体", 590, 210, 144, 28);
                self.button(parent, TEXT_COLOR, "选色", 590, 272, 144, 28);
                self.button(parent, BACKGROUND_COLOR, "选色", 590, 334, 144, 28);
                // Leave the editable HEX value visible beside its color picker.
                for id in [320, 321, 322] {
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
                    &seconds_text(if c.theme.widget_duration_ms == 0 {
                        5000
                    } else {
                        c.theme.widget_duration_ms
                    }),
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
            let _ = EnableWindow(
                self.hwnd(333),
                index_provider(self.selection(310)) == Provider::Ollama,
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
}
