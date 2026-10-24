use blink_verdict::provider::Observation;
use windows::{
    core::Result as WinResult,
    Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::Streams::DataWriter,
    Win32::{
        Foundation::*,
        Graphics::{Dwm::DwmFlush, Gdi::*},
        UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN},
    },
};

pub fn foreground_rect() -> RECT {
    unsafe {
        let monitor = MonitorFromWindow(
            windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow(),
            MONITOR_DEFAULTTOPRIMARY,
        );
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            info.rcMonitor
        } else {
            RECT {
                left: 0,
                top: 0,
                right: GetSystemMetrics(SM_CXSCREEN),
                bottom: GetSystemMetrics(SM_CYSCREEN),
            }
        }
    }
}

/// Encode directly from a borrowed DIB. Release capture handles before OCR recognition/network.
pub fn observe(rect: RECT, vision: bool) -> Result<Observation, String> {
    unsafe {
        super::display::require_sdr(rect)?;
    }
    let frame = unsafe { Frame::capture(rect)? };
    if frame
        .pixels()
        .chunks_exact(4)
        .all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0)
    {
        return Err("屏幕捕获为黑屏；此游戏或受保护画面暂不支持，请尝试无边框模式".into());
    }
    if vision {
        return blink_verdict::imaging::encode_screen_png(
            frame.width,
            frame.height,
            frame.pixels(),
        )
        .map(Observation::Image)
        .map_err(|_| "Windows PNG 编码失败".into());
    }
    let bitmap = ocr_bitmap(frame.width, frame.height, frame.pixels())
        .map_err(|_| "本机 OCR 图像准备失败".to_string())?;
    drop(frame);
    ocr(&bitmap).map(Observation::Text).map_err(|e| {
        if e.code() == windows::core::HRESULT::from_win32(ERROR_TIMEOUT.0) {
            "本机 OCR 超时，可以重试或改用视觉模型".into()
        } else {
            "本机 OCR 不可用，请安装 Windows OCR 语言包或改用视觉模型".into()
        }
    })
}

struct Frame {
    screen: HDC,
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *mut std::ffi::c_void,
    width: u32,
    height: u32,
}
impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            if !self.bitmap.is_invalid() {
                if !self.old.is_invalid() {
                    SelectObject(self.dc, self.old);
                }
                let _ = DeleteObject(self.bitmap);
            }
            if !self.dc.is_invalid() {
                let _ = DeleteDC(self.dc);
            }
            if !self.screen.is_invalid() {
                ReleaseDC(None, self.screen);
            }
        }
    }
}
impl Frame {
    fn pixels(&self) -> &[u8] {
        // Construction succeeds only after a full-sized, non-null DIB is selected and captured.
        unsafe {
            std::slice::from_raw_parts(self.bits.cast(), (self.width * self.height * 4) as usize)
        }
    }
    unsafe fn capture(rect: RECT) -> Result<Self, String> {
        let source_width = rect.right - rect.left;
        let source_height = rect.bottom - rect.top;
        if source_width <= 0 || source_height <= 0 {
            return Err("显示器尺寸无效".into());
        }
        let ratio = (1600.0 / source_width.max(source_height) as f64).min(1.0);
        let width = ((source_width as f64 * ratio) as i32).max(1);
        let height = ((source_height as f64 * ratio) as i32).max(1);
        let _ = DwmFlush();
        let mut frame = Self {
            screen: GetDC(None),
            dc: HDC::default(),
            bitmap: HBITMAP::default(),
            old: HGDIOBJ::default(),
            bits: std::ptr::null_mut(),
            width: width as u32,
            height: height as u32,
        };
        if frame.screen.is_invalid() {
            return Err("无法访问屏幕".into());
        }
        frame.dc = CreateCompatibleDC(frame.screen);
        if frame.dc.is_invalid() {
            return Err("无法创建截图缓冲区".into());
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        frame.bitmap = CreateDIBSection(
            frame.screen,
            &info,
            DIB_RGB_COLORS,
            &mut frame.bits,
            None,
            0,
        )
        .map_err(|_| "无法分配截图缓冲区".to_string())?;
        frame.old = SelectObject(frame.dc, frame.bitmap);
        if frame.old.is_invalid() {
            return Err("无法选择截图缓冲区".into());
        }
        SetStretchBltMode(frame.dc, HALFTONE);
        let result = StretchBlt(
            frame.dc,
            0,
            0,
            width,
            height,
            frame.screen,
            rect.left,
            rect.top,
            source_width,
            source_height,
            SRCCOPY | CAPTUREBLT,
        );
        if !result.as_bool() || frame.bits.is_null() {
            return Err("屏幕捕获失败，请尝试无边框模式".into());
        }
        Ok(frame)
    }
}

fn ocr_bitmap(width: u32, height: u32, pixels: &[u8]) -> WinResult<SoftwareBitmap> {
    let writer = DataWriter::new()?;
    writer.WriteBytes(pixels)?;
    let buffer = writer.DetachBuffer()?;
    SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        width as i32,
        height as i32,
        BitmapAlphaMode::Ignore,
    )
}

fn ocr(bitmap: &SoftwareBitmap) -> WinResult<String> {
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
    let result = super::gamebar::wait(
        engine.RecognizeAsync(bitmap)?,
        std::time::Duration::from_secs(10),
    )?;
    Ok(result.Text()?.to_string())
}
