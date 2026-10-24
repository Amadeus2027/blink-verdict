mod art;
mod capture;
mod credentials;
mod display;
mod gamebar;
mod hotkey;
mod render;
mod settings;
mod settings_style;
mod worker;

use blink_verdict::{
    config::{Config, DisplayMode},
    decision::Verdict,
    provider,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{Com::*, LibraryLoader::GetModuleHandleW, Threading::*, WinRT::*},
        UI::{HiDpi::*, Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
    },
};

const TRAY_EVENT: u32 = WM_APP + 1;
const HOTKEY_ID: i32 = 1;
const TIMER_ID: usize = 1;
const WORKER_TIMER_ID: usize = 2;
const CMD_SETTINGS: usize = 100;
const CMD_FULLSCREEN: usize = 101;
const CMD_WIDGET: usize = 102;
const CMD_EDIT: usize = 103;
const CMD_PREVIEW: usize = 104;
const CMD_EXIT: usize = 105;
const CMD_STATUS: usize = 106;
const CMD_PAUSE: usize = 107;
const CMD_HIDE: usize = 108;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

#[derive(serde::Serialize, serde::Deserialize)]
enum WorkerEvent {
    Captured,
    Finished(Result<(String, bool, String), String>),
}

pub(super) struct App {
    host: HWND,
    overlay: HWND,
    settings: HWND,
    config: Config,
    presentation: Option<Config>,
    paused: bool,
    recording_preview: bool,
    config_path: PathBuf,
    status: String,
    busy: bool,
    editing: bool,
    visible: bool,
    text: String,
    renderer: Option<render::Renderer>,
    shown_at: Instant,
    expires_at: Option<Instant>,
    result_display: bool,
    worker: Option<worker::Worker>,
    capture_rect: RECT,
    display_rect: RECT,
    hotkey_active: bool,
    settings_monitor: HMONITOR,
    settings_ui: Option<settings::Settings>,
    taskbar_created: u32,
}

pub fn run() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        if worker::run_if_requested() {
            return;
        }
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let mutex = match CreateMutexW(None, false, w!("Local\\BlinkVerdict.Instance")) {
            Ok(handle) => handle,
            Err(_) => return,
        };
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(mutex);
            return;
        }
        let path = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("BlinkVerdict")
            .join("config.json");
        let recording_preview = std::env::args_os().any(|arg| arg == "--recordable-preview");
        let first_run = !path.exists();
        let (config, status) = match Config::load(&path) {
            Ok(c) => (c, "欢迎使用，先连接模型，再设置你的决策".into()),
            Err(error) => (Config::default(), error),
        };
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let classes: [(PCWSTR, WNDPROC); 3] = [
            (w!("BlinkVerdict.Host"), Some(host_proc)),
            (w!("BlinkVerdict.Overlay"), Some(overlay_proc)),
            (w!("BlinkVerdict.Settings"), Some(settings::window_proc)),
        ];
        for (name, proc) in classes {
            let class = WNDCLASSW {
                lpfnWndProc: proc,
                hInstance: instance.into(),
                lpszClassName: name,
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                ..Default::default()
            };
            RegisterClassW(&class);
        }
        let Ok(host) = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BlinkVerdict.Host"),
            w!("BlinkVerdict"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            None,
            None,
            instance,
            None,
        ) else {
            return;
        };
        let mut app = Box::new(App {
            host,
            overlay: HWND::default(),
            settings: HWND::default(),
            config,
            presentation: None,
            paused: false,
            recording_preview,
            config_path: path,
            status,
            busy: false,
            editing: false,
            visible: false,
            text: String::new(),
            renderer: None,
            shown_at: Instant::now(),
            expires_at: None,
            result_display: false,
            worker: None,
            settings_ui: None,
            capture_rect: RECT::default(),
            display_rect: RECT::default(),
            hotkey_active: false,
            settings_monitor: display::interaction_monitor(),
            taskbar_created: RegisterWindowMessageW(w!("TaskbarCreated")),
        });
        SetWindowLongPtrW(host, GWLP_USERDATA, (&mut *app as *mut App) as isize);
        app.overlay = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED,
            w!("BlinkVerdict.Overlay"),
            w!("BlinkVerdict overlay"),
            WS_POPUP,
            0,
            0,
            320,
            140,
            None,
            None,
            instance,
            None,
        )
        .unwrap_or_default();
        if app.overlay.is_invalid() {
            MessageBoxW(
                None,
                w!("无法创建覆盖窗口"),
                w!("BlinkVerdict"),
                MB_OK | MB_ICONERROR,
            );
            let _ = DestroyWindow(host);
            let _ = CloseHandle(mutex);
            CoUninitialize();
            return;
        }
        SetWindowLongPtrW(app.overlay, GWLP_USERDATA, (&mut *app as *mut App) as isize);
        // Also hide ourselves before capturing, for older Windows versions or unsupported affinity.
        if !recording_preview {
            let _ = SetWindowDisplayAffinity(app.overlay, WDA_EXCLUDEFROMCAPTURE);
        }
        match app.register_hotkey(&app.config.hotkey.clone()) {
            Ok(()) => app.hotkey_active = true,
            Err(error) => app.status = error,
        }
        app.tray(NIM_ADD);
        if !app.hotkey_active {
            let message = app.status.clone();
            settings::open(&mut app);
            settings::error(app.settings, &message);
        } else if first_run {
            settings::open(&mut app);
        } else {
            app.restore_widget();
        }
        let mut message = MSG::default();
        loop {
            let result = GetMessageW(&mut message, None, 0, 0).0;
            if result <= 0 {
                break;
            }
            if !app.settings.is_invalid() && IsDialogMessageW(app.settings, &message).as_bool() {
                continue;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        app.worker = None;
        let _ = KillTimer(app.host, WORKER_TIMER_ID);
        app.hide();
        app.tray(NIM_DELETE);
        let _ = UnregisterHotKey(host, HOTKEY_ID);
        if !app.settings.is_invalid() {
            let _ = DestroyWindow(app.settings);
        }
        let _ = DestroyWindow(app.overlay);
        let _ = DestroyWindow(host);
        let _ = CloseHandle(mutex);
        CoUninitialize();
    }
}

impl App {
    unsafe fn register_hotkey(&self, text: &str) -> Result<(), String> {
        let (modifiers, key) = hotkey::parse(text)?;
        RegisterHotKey(self.host, HOTKEY_ID, modifiers, key)
            .map_err(|_| "快捷键注册失败：可能已被其他程序占用，请在设置中修改".into())
    }

    unsafe fn tray(&self, action: NOTIFY_ICON_MESSAGE) {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.host,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: TRAY_EVENT,
            hIcon: LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
            ..Default::default()
        };
        let tooltip = wide(&format!("BlinkVerdict · {}", self.status));
        let count = tooltip.len().saturating_sub(1).min(data.szTip.len() - 1);
        data.szTip[..count].copy_from_slice(&tooltip[..count]);
        let _ = Shell_NotifyIconW(action, &data);
    }

    unsafe fn set_status(&mut self, text: String) {
        self.status = text;
        self.tray(NIM_MODIFY);
    }

    unsafe fn notify_error(&mut self, message: String) {
        self.set_status(message.clone());
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.host,
            uID: 1,
            uFlags: NIF_INFO | NIF_REALTIME,
            dwInfoFlags: NIIF_WARNING | NIIF_NOSOUND,
            ..Default::default()
        };
        for (field, value) in [
            (&mut data.szInfo[..], message.as_str()),
            (&mut data.szInfoTitle[..], "BlinkVerdict"),
        ] {
            let text = wide(value);
            let count = field
                .len()
                .saturating_sub(1)
                .min(text.len().saturating_sub(1));
            field[..count].copy_from_slice(&text[..count]);
        }
        // A native notification never activates an overlay or takes keyboard focus.
        let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
    }

    unsafe fn menu(&mut self) {
        let Ok(menu) = CreatePopupMenu() else {
            return;
        };
        for (id, title, checked) in [
            (CMD_SETTINGS, "打开设置", false),
            (
                CMD_FULLSCREEN,
                "全屏浮现",
                self.config.mode == DisplayMode::Fullscreen,
            ),
            (
                CMD_WIDGET,
                "悬浮组件",
                self.config.mode == DisplayMode::Widget,
            ),
            (
                CMD_EDIT,
                if self.editing {
                    "完成布局调整"
                } else {
                    "调整浮窗位置与大小"
                },
                self.editing,
            ),
            (CMD_PREVIEW, "预览效果", false),
            (CMD_HIDE, "收起浮窗", false),
            (
                CMD_PAUSE,
                if self.paused {
                    "继续决策"
                } else {
                    "暂停决策"
                },
                self.paused,
            ),
            (CMD_STATUS, "状态与连接信息", false),
            (CMD_EXIT, "退出", false),
        ] {
            let text = wide(title);
            if matches!(id, CMD_FULLSCREEN | CMD_EDIT | CMD_HIDE | CMD_EXIT) {
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            }
            let blocked = self.busy && id != CMD_STATUS && id != CMD_EXIT
                || !self.settings.is_invalid()
                    && matches!(id, CMD_FULLSCREEN | CMD_WIDGET | CMD_EDIT | CMD_PREVIEW);
            let flags = MF_STRING
                | if checked { MF_CHECKED } else { MF_UNCHECKED }
                | if blocked { MF_GRAYED } else { MF_ENABLED };
            let _ = AppendMenuW(menu, flags, id, PCWSTR(text.as_ptr()));
        }
        let mut point = POINT::default();
        let _ = GetCursorPos(&mut point);
        // Focus changes only after the user explicitly clicks the tray, never on a decision.
        let _ = SetForegroundWindow(self.host);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            self.host,
            None,
        )
        .0 as usize;
        let _ = PostMessageW(self.host, WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        self.command(command);
    }

    unsafe fn command(&mut self, id: usize) {
        if self.busy && id != CMD_STATUS && id != CMD_EXIT {
            return;
        }
        if !self.settings.is_invalid()
            && matches!(id, CMD_FULLSCREEN | CMD_WIDGET | CMD_EDIT | CMD_PREVIEW)
        {
            return;
        }
        match id {
            CMD_SETTINGS => settings::open(self),
            CMD_FULLSCREEN | CMD_WIDGET => {
                if self.busy {
                    self.set_status("判断中，完成后可切换模式".into());
                    return;
                }
                self.clear_gamebar();
                self.lock_widget();
                self.hide();
                self.config.mode = if id == CMD_FULLSCREEN {
                    DisplayMode::Fullscreen
                } else {
                    DisplayMode::Widget
                };
                if let Err(error) = self.config.save(&self.config_path) {
                    self.set_status(error);
                }
                self.restore_widget();
            }
            CMD_EDIT => {
                if self.busy {
                    return;
                }
                if self.editing {
                    self.lock_widget();
                    self.hide();
                    self.restore_widget();
                } else {
                    self.clear_gamebar();
                    self.hide();
                    if !self.config.gamebar_package_family.is_empty() {
                        self.set_status("Game Bar 窗口请从其面板调整位置 / 大小".into());
                        return;
                    }
                    let mut editing_config = self.config.clone();
                    editing_config.mode = DisplayMode::Widget;
                    self.editing = true;
                    let style = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED;
                    SetWindowLongPtrW(self.overlay, GWL_EXSTYLE, style.0 as isize);
                    // Sizing hit-tests require a sizing frame; WM_NCCALCSIZE keeps it borderless.
                    SetWindowLongPtrW(
                        self.overlay,
                        GWL_STYLE,
                        (WS_POPUP | WS_THICKFRAME).0 as isize,
                    );
                    let _ = SetWindowPos(
                        self.overlay,
                        None,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                    );
                    self.show(
                        "拖动到喜欢的位置".into(),
                        capture::foreground_rect(),
                        None,
                        false,
                        Some(editing_config),
                    );
                    let _ = self.config.save(&self.config_path);
                    if !self.settings.is_invalid() {
                        let _ = DestroyWindow(self.settings);
                    }
                }
            }
            CMD_PREVIEW => {
                if !self.busy {
                    self.lock_widget();
                    let text = self
                        .config
                        .options
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "YES".into());
                    if self.config.gamebar_package_family.is_empty() {
                        self.show_result(text, capture::foreground_rect());
                    } else {
                        self.start_worker(Some(text));
                    }
                }
            }
            CMD_HIDE => {
                self.clear_gamebar();
                self.lock_widget();
                self.hide();
            }
            CMD_PAUSE => {
                if self.paused {
                    match self.register_hotkey(&self.config.hotkey) {
                        Ok(()) => {
                            self.paused = false;
                            self.hotkey_active = true;
                            self.set_status("已恢复，按下快捷键即可开始".into());
                            self.restore_widget();
                        }
                        Err(error) => {
                            self.set_status(error.clone());
                            let text = wide(&error);
                            MessageBoxW(
                                self.host,
                                PCWSTR(text.as_ptr()),
                                w!("BlinkVerdict · 快捷键未启用"),
                                MB_OK | MB_ICONWARNING,
                            );
                        }
                    }
                } else {
                    let _ = UnregisterHotKey(self.host, HOTKEY_ID);
                    self.paused = true;
                    self.hotkey_active = false;
                    self.clear_gamebar();
                    self.lock_widget();
                    self.hide();
                    self.set_status("已暂停，可从托盘继续".into());
                }
            }
            CMD_STATUS => {
                let text = wide(&self.status);
                MessageBoxW(
                    self.host,
                    PCWSTR(text.as_ptr()),
                    w!("BlinkVerdict · 状态"),
                    MB_OK,
                );
            }
            CMD_EXIT => {
                self.clear_gamebar();
                PostQuitMessage(0);
            }
            _ => (),
        }
    }

    unsafe fn lock_widget(&mut self) {
        self.editing = false;
        SetWindowLongPtrW(self.overlay, GWL_STYLE, WS_POPUP.0 as isize);
        let _ = SetWindowPos(
            self.overlay,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
        SetWindowLongPtrW(
            self.overlay,
            GWL_EXSTYLE,
            (WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE
                | WS_EX_TRANSPARENT
                | WS_EX_LAYERED)
                .0 as isize,
        );
    }

    /// Only explicit tray/settings actions call this. Never block the decision hotkey UI.
    unsafe fn clear_gamebar(&self) {
        if self.config.gamebar_package_family.is_empty() {
            return;
        }
        let config = self.config.clone();
        // Explicit UI actions never join an IPC thread. Each helper has the same
        // process deadline/kill-on-close ownership as a decision.
        let _ = std::thread::Builder::new()
            .name("gamebar-hide".into())
            .spawn(move || {
                if let Ok(mut task) =
                    worker::Worker::start(config, String::new(), None, RECT::default(), true)
                {
                    loop {
                        if matches!(task.poll(), Some(WorkerEvent::Finished(_))) {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
            });
    }

    unsafe fn hide(&mut self) {
        let _ = KillTimer(self.overlay, TIMER_ID);
        let _ = ShowWindow(self.overlay, SW_HIDE);
        self.visible = false;
        self.expires_at = None;
        self.renderer = None;
        self.presentation = None;
    }

    fn display_config(&self) -> &Config {
        self.presentation.as_ref().unwrap_or(&self.config)
    }

    unsafe fn restore_widget(&mut self) {
        if self.config.mode == DisplayMode::Widget
            && self.config.theme.widget_duration_ms == 0
            && self.config.gamebar_package_family.is_empty()
            && !self.paused
            && !self.busy
            && !self.editing
            && self.settings.is_invalid()
        {
            self.show(
                "等待你的下一次按键".into(),
                capture::foreground_rect(),
                None,
                false,
                None,
            );
        }
    }

    unsafe fn show(
        &mut self,
        text: String,
        rect: RECT,
        duration: Option<Duration>,
        is_result: bool,
        presentation: Option<Config>,
    ) {
        self.hide();
        self.presentation = presentation;
        self.display_rect = rect;
        let config = self.display_config().clone();
        self.text = text;
        self.result_display = is_result;
        let (x, y, width, height) = if config.mode == DisplayMode::Fullscreen {
            (
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            )
        } else {
            let mut area = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let monitor = MonitorFromPoint(
                POINT {
                    x: config.widget_x,
                    y: config.widget_y,
                },
                MONITOR_DEFAULTTONEAREST,
            );
            let _ = GetMonitorInfoW(monitor, &mut area);
            let work = area.rcWork;
            let dpi = display::dpi(monitor);
            let width =
                display::pixels(config.widget_width, dpi).min((work.right - work.left).max(1));
            let height =
                display::pixels(config.widget_height, dpi).min((work.bottom - work.top).max(1));
            let x = config
                .widget_x
                .clamp(work.left, (work.right - width).max(work.left));
            let y = config
                .widget_y
                .clamp(work.top, (work.bottom - height).max(work.top));
            (x, y, width, height)
        };
        let _ = SetWindowPos(
            self.overlay,
            HWND_TOPMOST,
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE,
        );
        // Decode and create the surface before starting the configured result lifetime.
        match render::Renderer::new(self.overlay, &config, &self.text, self.display_rect) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(_) => {
                self.hide();
                self.set_status("图形显示初始化失败".into());
                return;
            }
        }
        self.shown_at = Instant::now();
        self.expires_at = duration.map(|d| self.shown_at + d);
        self.visible = true;
        // Present the new frame while hidden, avoiding a flash of the previous result.
        self.paint();
        if !self.visible {
            return;
        }
        let _ = ShowWindow(self.overlay, SW_SHOWNOACTIVATE);
        let _ = InvalidateRect(self.overlay, None, false);
        if let Some(delay) = blink_verdict::animation::next_tick(
            self.effect(),
            0.0,
            duration.map(|d| d.as_secs_f32()),
        ) {
            SetTimer(self.overlay, TIMER_ID, delay, None);
        }
    }

    unsafe fn show_result(&mut self, text: String, rect: RECT) {
        let ms = match self.config.mode {
            DisplayMode::Fullscreen => self.config.theme.fullscreen_duration_ms,
            DisplayMode::Widget => self.config.theme.widget_duration_ms,
        };
        self.show(
            text,
            rect,
            if ms == 0 {
                None
            } else {
                Some(Duration::from_millis(ms.into()))
            },
            true,
            None,
        );
    }

    fn effect(&self) -> blink_verdict::config::Animation {
        if self.result_display {
            self.display_config().theme.animation
        } else {
            blink_verdict::config::Animation::None
        }
    }

    unsafe fn paint(&mut self) {
        if !self.visible {
            return;
        }
        if self.renderer.is_none() {
            match render::Renderer::new(
                self.overlay,
                self.display_config(),
                &self.text,
                self.display_rect,
            ) {
                Ok(renderer) => self.renderer = Some(renderer),
                Err(_) => {
                    self.hide();
                    self.set_status("图形显示初始化失败".into());
                    return;
                }
            }
        }
        let motion = blink_verdict::animation::sample(
            self.effect(),
            self.shown_at.elapsed().as_secs_f32(),
            self.expires_at
                .map(|t| t.saturating_duration_since(Instant::now()).as_secs_f32()),
        );
        if self
            .renderer
            .as_ref()
            .unwrap()
            .paint(self.display_config(), &self.text, self.editing, motion)
            .is_err()
        {
            self.hide();
            self.set_status("图形设备不可用，本次显示已跳过".into());
        }
    }

    unsafe fn trigger(&mut self) {
        if self.recording_preview
            || self.paused
            || !self.hotkey_active
            || self.busy
            || self.editing
            || !self.settings.is_invalid()
        {
            return;
        }
        self.start_worker(None);
    }

    unsafe fn start_worker(&mut self, preview: Option<String>) {
        let rect = capture::foreground_rect();
        self.hide();
        let config = self.config.clone();
        let key = match if preview.is_some() {
            Ok(String::new())
        } else {
            credentials::read(&config)
        } {
            Ok(key) => key,
            Err(error) => {
                self.set_status(error);
                return;
            }
        };
        self.busy = true;
        self.capture_rect = rect;
        self.set_status("正在思考…".into());
        match worker::Worker::start(config, key, preview, rect, false) {
            Ok(worker) => self.worker = Some(worker),
            Err(error) => {
                self.busy = false;
                self.set_status(error);
                self.restore_widget();
                return;
            }
        }
        // Poll only during a request. Completion never depends on PostMessage delivery.
        if SetTimer(self.host, WORKER_TIMER_ID, 50, None) == 0 {
            self.worker = None;
            self.busy = false;
            self.set_status("无法启动任务计时器，请重试".into());
            self.restore_widget();
        }
    }

    unsafe fn finish(&mut self) {
        while let Some(event) = self.worker.as_mut().and_then(|w| w.poll()) {
            match event {
                WorkerEvent::Captured => {
                    self.show("正在思考…".into(), self.capture_rect, None, false, None)
                }
                WorkerEvent::Finished(result) => {
                    self.busy = false;
                    self.worker = None;
                    let _ = KillTimer(self.host, WORKER_TIMER_ID);
                    match result {
                        Ok((text, gamebar, details)) => {
                            self.set_status(format!("最近决策：{text}{details}"));
                            if !gamebar {
                                self.show_result(text, self.capture_rect);
                            }
                        }
                        Err(error) => {
                            self.hide();
                            self.notify_error(error);
                            self.restore_widget();
                        }
                    }
                }
            }
        }
    }
}

unsafe extern "system" fn host_proc(
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
    if message == app.taskbar_created {
        app.tray(NIM_ADD);
        return LRESULT(0);
    }
    match message {
        WM_HOTKEY => {
            app.trigger();
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == WORKER_TIMER_ID => {
            app.finish();
            LRESULT(0)
        }
        TRAY_EVENT => {
            app.settings_monitor = display::interaction_monitor();
            match lparam.0 as u32 {
                WM_RBUTTONUP | WM_CONTEXTMENU => app.menu(),
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => settings::open(app),
                _ => (),
            }
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            app.hide();
            app.restore_widget();
            LRESULT(0)
        }
        WM_ENDSESSION => {
            if wparam.0 != 0 {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

unsafe extern "system" fn overlay_proc(
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
        WM_RBUTTONUP | WM_NCRBUTTONUP if app.editing => {
            app.command(CMD_EDIT);
            LRESULT(0)
        }
        WM_NCCALCSIZE => LRESULT(0),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_NCHITTEST => {
            if !app.editing {
                return LRESULT(HTTRANSPARENT as isize);
            }
            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            let x = lparam.0 as u16 as i16 as i32;
            let y = ((lparam.0 >> 16) as u16) as i16 as i32;
            let edge = display::pixels(10, GetDpiForWindow(hwnd));
            let left = x < rect.left + edge;
            let right = x >= rect.right - edge;
            let top = y < rect.top + edge;
            let bottom = y >= rect.bottom - edge;
            LRESULT(match (left, right, top, bottom) {
                (true, _, true, _) => HTTOPLEFT,
                (_, true, true, _) => HTTOPRIGHT,
                (true, _, _, true) => HTBOTTOMLEFT,
                (_, true, _, true) => HTBOTTOMRIGHT,
                (true, _, _, _) => HTLEFT,
                (_, true, _, _) => HTRIGHT,
                (_, _, true, _) => HTTOP,
                (_, _, _, true) => HTBOTTOM,
                _ => HTCAPTION,
            } as isize)
        }
        WM_GETMINMAXINFO => {
            let info = &mut *(lparam.0 as *mut MINMAXINFO);
            let dpi = GetDpiForWindow(hwnd);
            info.ptMinTrackSize = POINT {
                x: display::pixels(160, dpi),
                y: display::pixels(80, dpi),
            };
            info.ptMaxTrackSize = POINT {
                x: display::pixels(1600, dpi),
                y: display::pixels(1000, dpi),
            };
            LRESULT(0)
        }
        WM_EXITSIZEMOVE if app.editing => {
            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            app.config.widget_x = rect.left;
            app.config.widget_y = rect.top;
            app.config.widget_width =
                display::logical(rect.right - rect.left, GetDpiForWindow(hwnd)).clamp(160, 1600);
            app.config.widget_height =
                display::logical(rect.bottom - rect.top, GetDpiForWindow(hwnd)).clamp(80, 1000);
            if let Err(error) = app.config.save(&app.config_path) {
                app.set_status(error);
            }
            LRESULT(0)
        }
        WM_SIZE | WM_SETTINGCHANGE => {
            app.renderer = None;
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            BeginPaint(hwnd, &mut paint);
            app.paint();
            let _ = EndPaint(hwnd, &paint);
            LRESULT(0)
        }
        WM_TIMER => {
            let now = Instant::now();
            if app.expires_at.is_some_and(|deadline| now >= deadline) {
                app.hide();
                return LRESULT(0);
            }
            let elapsed = now.duration_since(app.shown_at).as_secs_f32();
            let remaining = app
                .expires_at
                .map(|t| t.saturating_duration_since(now).as_secs_f32());
            let _ = InvalidateRect(hwnd, None, false);
            let _ = KillTimer(hwnd, TIMER_ID);
            if let Some(delay) =
                blink_verdict::animation::next_tick(app.effect(), elapsed, remaining)
            {
                SetTimer(hwnd, TIMER_ID, delay, None);
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            app.renderer = None;
            if app.visible {
                let rect = *(lparam.0 as *const RECT);
                let rect = if app.display_config().mode == DisplayMode::Fullscreen {
                    display::monitor_info(MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST))
                        .rcMonitor
                } else {
                    rect
                };
                app.display_rect = rect;
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

#[cfg(test)]
struct TestApp(Box<App>);
#[cfg(test)]
impl TestApp {
    unsafe fn new() -> Self {
        let classes: [(PCWSTR, WNDPROC); 3] = [
            (w!("BlinkVerdict.Regression.Host"), Some(host_proc)),
            (w!("BlinkVerdict.Regression.Overlay"), Some(overlay_proc)),
            (w!("BlinkVerdict.Settings"), Some(settings::window_proc)),
        ];
        for (name, proc) in classes {
            RegisterClassW(&WNDCLASSW {
                lpszClassName: name,
                lpfnWndProc: proc,
                hInstance: GetModuleHandleW(None).unwrap().into(),
                ..Default::default()
            });
        }
        let create = |name, style| {
            CreateWindowExW(
                style,
                name,
                w!("BlinkVerdict regression"),
                WS_POPUP,
                0,
                0,
                320,
                140,
                None,
                None,
                GetModuleHandleW(None).unwrap(),
                None,
            )
            .unwrap()
        };
        let host = create(w!("BlinkVerdict.Regression.Host"), WINDOW_EX_STYLE(0));
        let overlay = create(
            w!("BlinkVerdict.Regression.Overlay"),
            WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
        );
        let mut app = Box::new(App {
            host,
            overlay,
            settings: HWND::default(),
            config: Config::default(),
            presentation: None,
            paused: false,
            recording_preview: false,
            config_path: std::env::temp_dir().join(format!(
                "blink-runtime-regression-{}-{:?}.json",
                std::process::id(),
                std::thread::current().id()
            )),
            status: String::new(),
            busy: false,
            editing: false,
            visible: false,
            text: String::new(),
            renderer: None,
            shown_at: Instant::now(),
            expires_at: None,
            result_display: false,
            worker: None,
            capture_rect: display::monitor_info(MonitorFromWindow(host, MONITOR_DEFAULTTOPRIMARY))
                .rcMonitor,
            display_rect: RECT::default(),
            hotkey_active: false,
            settings_monitor: display::interaction_monitor(),
            settings_ui: None,
            taskbar_created: 0,
        });
        let pointer = (&mut *app as *mut App) as isize;
        SetWindowLongPtrW(host, GWLP_USERDATA, pointer);
        SetWindowLongPtrW(overlay, GWLP_USERDATA, pointer);
        Self(app)
    }
}
#[cfg(test)]
impl Drop for TestApp {
    fn drop(&mut self) {
        unsafe {
            self.0.worker = None;
            self.0.hide();
            let _ = KillTimer(self.0.host, WORKER_TIMER_ID);
            let _ = UnregisterHotKey(self.0.host, HOTKEY_ID);
            if !self.0.settings.is_invalid() {
                let _ = DestroyWindow(self.0.settings);
            }
            let _ = DestroyWindow(self.0.overlay);
            let _ = DestroyWindow(self.0.host);
            let _ = std::fs::remove_file(&self.0.config_path);
        }
    }
}
