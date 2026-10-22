//! Event-painted native controls; system acrylic where supported, opaque fallback elsewhere.
use super::*;
use windows::Win32::{Graphics::Dwm::*, UI::Controls::*};

pub const BACKGROUND: COLORREF = COLORREF(0x00201913);
pub const SIDEBAR: COLORREF = COLORREF(0x001C1510);
pub const FIELD: COLORREF = COLORREF(0x00312620);
pub const TEXT: COLORREF = COLORREF(0x00F7F5F2);
pub const MUTED: COLORREF = COLORREF(0x00B9AB9B);
pub const ACCENT: COLORREF = COLORREF(0x00FF9B53);

pub struct Style {
    pub font: HFONT,
    pub title: HFONT,
    pub strong: HFONT,
    pub small: HFONT,
    pub background: HBRUSH,
    pub field: HBRUSH,
    pub glass: bool,
    glass_width: i32,
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
            font: font(14.0, 400),
            title: font(28.0, 600),
            strong: font(15.0, 600),
            small: font(12.0, 400),
            background: CreateSolidBrush(BACKGROUND),
            field: CreateSolidBrush(FIELD),
            glass: false,
            glass_width: (200.0 * scale) as i32,
        }
    }
    pub unsafe fn attach(&mut self, hwnd: HWND) {
        let dark = BOOL(1);
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
        let backdrop = DWMSBT_TRANSIENTWINDOW.0;
        if DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            (&backdrop as *const i32).cast(),
            4,
        )
        .is_ok()
        {
            self.glass = DwmExtendFrameIntoClientArea(
                hwnd,
                &MARGINS {
                    cxLeftWidth: self.glass_width,
                    cxRightWidth: 0,
                    cyTopHeight: 0,
                    cyBottomHeight: 0,
                },
            )
            .is_ok();
        }
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
        // DrawText may inspect its pointer even at count zero; an empty Vec has no backing buffer.
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
    let old_brush = SelectObject(dc, brush);
    let old_pen = SelectObject(dc, pen);
    let _ = RoundRect(
        dc,
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        radius,
        radius,
    );
    SelectObject(dc, old_pen);
    SelectObject(dc, old_brush);
    let _ = DeleteObject(pen);
    let _ = DeleteObject(brush);
}
