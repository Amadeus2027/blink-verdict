//! Small, event-painted light controls. No web runtime or idle redraw loop.
use super::*;
use windows::Win32::Graphics::Dwm::*;

pub const BACKGROUND: COLORREF = COLORREF(0x00F7F5F4);
pub const FIELD: COLORREF = COLORREF(0x00FFFFFF);
pub const TEXT: COLORREF = COLORREF(0x00201E1C);
pub const MUTED: COLORREF = COLORREF(0x008D8884);
pub const LINE: COLORREF = COLORREF(0x00EDE9E6);

pub struct Style {
    pub font: HFONT,
    pub title: HFONT,
    pub strong: HFONT,
    pub small: HFONT,
    pub background: HBRUSH,
    pub field: HBRUSH,
}
impl Drop for Style {
    fn drop(&mut self) {
        unsafe {
            for font in [self.font, self.title, self.strong, self.small] {
                let _ = DeleteObject(font);
            }
            let _ = DeleteObject(self.background);
            let _ = DeleteObject(self.field);
        }
    }
}
impl Style {
    pub unsafe fn new(scale: f32, face: &str) -> Self {
        let face = wide(face);
        let font = |size: f32, weight| {
            CreateFontW(
                -(size * scale) as i32,
                0,
                0,
                0,
                weight,
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                CLEARTYPE_QUALITY.0 as u32,
                DEFAULT_PITCH.0 as u32,
                PCWSTR(face.as_ptr()),
            )
        };
        Self {
            font: font(17.0, 400),
            title: font(23.0, 600),
            strong: font(17.0, 600),
            small: font(13.0, 400),
            background: CreateSolidBrush(BACKGROUND),
            field: CreateSolidBrush(FIELD),
        }
    }
    pub unsafe fn attach(&mut self, hwnd: HWND) {
        let dark = BOOL(0);
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark as *const BOOL).cast(),
            4,
        );
        let corner = DWMWCP_ROUND.0;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&corner as *const i32).cast(),
            4,
        );
        let border = BACKGROUND.0;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, (&border as *const u32).cast(), 4);
    }
    pub unsafe fn text(
        &self,
        dc: HDC,
        value: &str,
        mut rect: RECT,
        font: HFONT,
        color: COLORREF,
        flags: DRAW_TEXT_FORMAT,
    ) {
        if value.is_empty() || rect.right <= rect.left || rect.bottom <= rect.top {
            return;
        }
        let old = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, color);
        let mut value = wide(value);
        let length = value.len() - 1;
        DrawTextW(dc, &mut value[..length], &mut rect, flags);
        SelectObject(dc, old);
    }
}
pub unsafe fn rounded(dc: HDC, rect: RECT, color: COLORREF, border: COLORREF, radius: i32) {
    let brush = CreateSolidBrush(color);
    let pen = CreatePen(PS_SOLID, 1, border);
    let ob = SelectObject(dc, brush);
    let op = SelectObject(dc, pen);
    let _ = RoundRect(
        dc,
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        radius,
        radius,
    );
    SelectObject(dc, op);
    SelectObject(dc, ob);
    let _ = DeleteObject(pen);
    let _ = DeleteObject(brush);
}

/// Geometric outline icons share the same stroke and occupy the same 22 px box.
pub unsafe fn icon(dc: HDC, x: i32, y: i32, scale: f32, kind: usize) {
    let pen = CreatePen(PS_SOLID, (1.6 * scale).round().max(1.0) as i32, TEXT);
    let old = SelectObject(dc, pen);
    let brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
    let p = |a: i32, b: i32| POINT {
        x: x + (a as f32 * scale) as i32,
        y: y + (b as f32 * scale) as i32,
    };
    let path = |points: &[(i32, i32)]| {
        let v: Vec<_> = points.iter().map(|(a, b)| p(*a, *b)).collect();
        let _ = Polyline(dc, &v);
    };
    match kind {
        300 | 350..=353 => {
            let a = p(2, 3);
            let b = p(20, 18);
            let _ = RoundRect(dc, a.x, a.y, b.x, b.y, 4, 4);
            path(&[(7, 21), (15, 21)]);
        }
        301 => {
            let a = p(1, 5);
            let b = p(21, 18);
            let _ = RoundRect(dc, a.x, a.y, b.x, b.y, 4, 4);
            path(&[(5, 10), (17, 10)]);
            path(&[(7, 14), (15, 14)]);
        }
        311 | 310 | 313 => {
            let a = p(3, 2);
            let b = p(19, 20);
            let _ = RoundRect(dc, a.x, a.y, b.x, b.y, 5, 5);
            path(&[(7, 8), (15, 8)]);
            path(&[(7, 14), (15, 14)]);
        }
        312 | 315 => {
            let a = p(2, 2);
            let b = p(20, 20);
            let _ = Ellipse(dc, a.x, a.y, b.x, b.y);
            path(&[(2, 11), (20, 11)]);
            path(&[(11, 2), (7, 11), (11, 20), (15, 11), (11, 2)]);
        }
        314 => {
            let a = p(5, 2);
            let b = p(17, 14);
            let _ = Ellipse(dc, a.x, a.y, b.x, b.y);
            path(&[(11, 14), (11, 22), (15, 22)]);
        }
        328 => {
            path(&[
                (11, 1),
                (13, 8),
                (21, 11),
                (13, 14),
                (11, 21),
                (9, 14),
                (1, 11),
                (9, 8),
                (11, 1),
            ]);
        }
        329 | 326 | 327 => {
            let a = p(2, 2);
            let b = p(20, 20);
            let _ = Ellipse(dc, a.x, a.y, b.x, b.y);
            path(&[(11, 5), (11, 11), (16, 14)]);
        }
        _ => {
            path(&[(4, 17), (17, 4), (20, 7), (7, 20), (3, 21), (4, 17)]);
        }
    }
    SelectObject(dc, brush);
    SelectObject(dc, old);
    let _ = DeleteObject(pen);
}
