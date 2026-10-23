//! Native controls, painting and shortcut input. No resident render loop.
use super::*;

impl Settings {
    pub(super) unsafe fn control(
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
    pub(super) unsafe fn label(
        &mut self,
        parent: HWND,
        text: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
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
    pub(super) unsafe fn card(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.cards.push((x, y, width, height));
    }
    pub(super) unsafe fn section(&mut self, parent: HWND, text: &str, y: i32) {
        self.label(parent, text, 56, y, 688, 24);
    }
    pub(super) unsafe fn row_label(&mut self, parent: HWND, id: usize, label: &str, y: i32) {
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
    pub(super) unsafe fn edit(
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
        if matches!(id, 301 | 314) {
            let hint = wide(if id == 301 {
                "按下组合键"
            } else {
                "输入或保留已有密钥"
            });
            SendMessageW(hwnd, 0x1501, WPARAM(0), LPARAM(hint.as_ptr() as isize));
        }
        SendMessageW(
            hwnd,
            EM_SETLIMITTEXT,
            WPARAM(if multiline > 0 { 8192 } else { 2048 }),
            LPARAM(0),
        );
    }
    pub(super) unsafe fn combo(
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
    pub(super) unsafe fn button(
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
    pub(super) unsafe fn rect(&self, x: i32, y: i32, width: i32, height: i32) -> RECT {
        let p = |n: i32| (n as f32 * self.scale) as i32;
        RECT {
            left: p(x),
            top: p(y),
            right: p(x + width),
            bottom: p(y + height),
        }
    }
    pub(super) unsafe fn paint_background(&self, hwnd: HWND, dc: HDC) {
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
                (48.0 * self.scale) as i32,
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
                self.rect(62, 185, 676, 58),
                self.style.title,
                visual::TEXT,
                DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            self.style.text(
                dc,
                "选择喜欢的方式，让答案自然浮现。",
                self.rect(62, 251, 676, 26),
                self.style.small,
                visual::MUTED,
                DT_CENTER | DT_SINGLELINE,
            );
        }
    }
    pub(super) unsafe fn draw_item(&self, item: &DRAWITEMSTRUCT) {
        let id = item.CtlID as usize;
        let tab = matches!(id, 210..=212);
        let selected = tab && id == PAGE_BASE + self.root_page();
        let pressed = item.itemState.0 & ODS_SELECTED.0 != 0;
        let disabled = item.itemState.0 & ODS_DISABLED.0 != 0;
        let primary = id == SAVE || id == NEXT;
        let combo = item.CtlType == ODT_COMBOBOX;
        let background = if combo || matches!(id, 329 | TEXT_COLOR | BACKGROUND_COLOR | FONT) {
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
                ((if tab { 42.0 } else { 48.0 }) * self.scale) as i32,
            );
        }
        if id == 217 {
            let mut top = item.rcItem;
            top.bottom = (top.top + top.bottom) / 2;
            let brush = CreateSolidBrush(fill);
            FillRect(item.hDC, &top, brush);
            let _ = DeleteObject(brush);
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
            let mut line = item.rcItem;
            line.top = line.bottom - 1;
            line.left += (58.0 * self.scale) as i32;
            line.right -= (22.0 * self.scale) as i32;
            let brush = CreateSolidBrush(visual::LINE);
            FillRect(item.hDC, &line, brush);
            let _ = DeleteObject(brush);
        }
        if matches!(id, TEXT_COLOR | BACKGROUND_COLOR) {
            let field = if id == TEXT_COLOR { 321 } else { 322 };
            if let Ok([r, g, b]) = blink_verdict::config::parse_color(&self.text(field)) {
                let x = item.rcItem.left + (12.0 * self.scale) as i32;
                let y = (item.rcItem.top + item.rcItem.bottom) / 2 - (6.0 * self.scale) as i32;
                let color = COLORREF(r as u32 | ((g as u32) << 8) | ((b as u32) << 16));
                visual::rounded(
                    item.hDC,
                    RECT {
                        left: x,
                        top: y,
                        right: x + (12.0 * self.scale) as i32,
                        bottom: y + (12.0 * self.scale) as i32,
                    },
                    color,
                    visual::LINE,
                    (12.0 * self.scale) as i32,
                );
            }
        }
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
        if parts.is_empty() {
            return LRESULT(0);
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
