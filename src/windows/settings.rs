mod controls;
mod pages;

use super::settings_style::{self as visual, Style};
use super::*;
use blink_verdict::config::{Animation, ArtStyle, Provider};
use windows::Win32::UI::Controls::Dialogs::*;
use windows::Win32::UI::Controls::*;

const SAVE: usize = 200;
const PREVIEW: usize = 201;
const CANCEL: usize = 202;
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
const FONT: usize = 225;

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
                let opacity = self
                    .text(332)
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "请填写 0–100 之间的浮窗背景不透明度".to_string())?;
                if !opacity.is_finite() || !(0.0..=100.0).contains(&opacity) {
                    return Err("浮窗背景不透明度应为 0–100%".into());
                }
                c.theme.widget_background_opacity = (opacity * 255.0 / 100.0).round() as u8;
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
                let alpha = (opacity * 255.0 / 100.0).round();
                // Legacy alpha 30 is displayed as 11.8%; accepting that rounded
                // value preserves valid saved themes when this page is opened.
                if !opacity.is_finite()
                    || !(0.0..=100.0).contains(&opacity)
                    || !(30.0..=255.0).contains(&alpha)
                {
                    return Err("整体不透明度应为约 12–100%".into());
                }
                c.theme.opacity = alpha as u8;
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

unsafe fn save(app: &mut App) -> bool {
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
    app.config = config;
    app.hotkey_active = !app.paused;
    app.set_status(
        if app.paused {
            "已保存 · 快捷键已暂停"
        } else {
            "已准备好，按下快捷键即可开始"
        }
        .into(),
    );
    let _ = DestroyWindow(app.settings);
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
                save(app);
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
                FONT => {
                    let ui = app.settings_ui.as_mut().unwrap();
                    let face = wide(&ui.text(320));
                    let mut font = LOGFONTW::default();
                    for (slot, ch) in font.lfFaceName.iter_mut().zip(face.iter().take(31)) {
                        *slot = *ch;
                    }
                    let mut chooser = CHOOSEFONTW {
                        lStructSize: std::mem::size_of::<CHOOSEFONTW>() as u32,
                        hwndOwner: hwnd,
                        lpLogFont: &mut font,
                        Flags: CF_SCREENFONTS
                            | CF_INITTOLOGFONTSTRUCT
                            | CF_NOSIZESEL
                            | CF_NOSTYLESEL,
                        ..Default::default()
                    };
                    if ChooseFontW(&mut chooser).as_bool() {
                        let _ = SetWindowTextW(ui.hwnd(320), PCWSTR(font.lfFaceName.as_ptr()));
                    }
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
                    save(app);
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
                    if save(app) {
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
