use blink_verdict::provider::Observation;
use windows::{
    core::Result as WinResult,
    Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::Streams::DataWriter,
    Win32::{
        Foundation::*,
        Graphics::{Dwm::DwmFlush, Gdi::*},
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
                right: GetSystemMetricsWidth(),
                bottom: GetSystemMetricsHeight(),
            }
        }
    }
}

#[allow(non_snake_case)]
unsafe fn GetSystemMetricsWidth() -> i32 {
    windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
        windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
    )
}
#[allow(non_snake_case)]
unsafe fn GetSystemMetricsHeight() -> i32 {
    windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
        windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN,
    )
}

/// All GDI handles are released before encoding/OCR or network activity begins.
pub fn observe(rect: RECT, vision: bool) -> Result<Observation, String> {
    let (width, height, pixels) = unsafe { pixels(rect)? };
    if pixels
        .chunks_exact(4)
        .all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0)
    {
        return Err("屏幕捕获为黑屏；此游戏或受保护画面暂不支持，请尝试无边框模式".into());
    }
    if !vision {
        return ocr(width, height, &pixels)
            .map(Observation::Text)
            .map_err(|_| "本机 OCR 不可用，请安装 Windows OCR 语言包或改用视觉模型".into());
    }
    blink_verdict::imaging::encode_screen_png(width, height, &pixels)
        .map(Observation::Image)
        .map_err(|_| "Windows PNG 编码失败".into())
}

unsafe fn pixels(rect: RECT) -> Result<(u32, u32, Vec<u8>), String> {
    let source_width = rect.right - rect.left;
    let source_height = rect.bottom - rect.top;
    if source_width <= 0 || source_height <= 0 {
        return Err("显示器尺寸无效".into());
    }
    let ratio = (1600.0 / source_width.max(source_height) as f64).min(1.0);
    let width = ((source_width as f64 * ratio) as i32).max(1);
    let height = ((source_height as f64 * ratio) as i32).max(1);
    let _ = DwmFlush();
    let screen = GetDC(None);
    if screen.is_invalid() {
        return Err("无法访问屏幕".into());
    }
    let dc = CreateCompatibleDC(screen);
    if dc.is_invalid() {
        ReleaseDC(None, screen);
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
    let mut bits = std::ptr::null_mut();
    let bitmap = match CreateDIBSection(screen, &info, DIB_RGB_COLORS, &mut bits, None, 0) {
        Ok(bitmap) => bitmap,
        Err(_) => {
            let _ = DeleteDC(dc);
            ReleaseDC(None, screen);
            return Err("无法分配截图缓冲区".into());
        }
    };
    let old = SelectObject(dc, bitmap);
    SetStretchBltMode(dc, HALFTONE);
    let result = StretchBlt(
        dc,
        0,
        0,
        width,
        height,
        screen,
        rect.left,
        rect.top,
        source_width,
        source_height,
        SRCCOPY | CAPTUREBLT,
    );
    let captured = if result.as_bool() && !bits.is_null() {
        Ok((
            width as u32,
            height as u32,
            std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec(),
        ))
    } else {
        Err("屏幕捕获失败，请尝试无边框模式".into())
    };
    SelectObject(dc, old);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(dc);
    ReleaseDC(None, screen);
    captured
}

fn ocr(width: u32, height: u32, pixels: &[u8]) -> WinResult<String> {
    let writer = DataWriter::new()?;
    writer.WriteBytes(pixels)?;
    let buffer = writer.DetachBuffer()?;
    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        width as i32,
        height as i32,
        BitmapAlphaMode::Ignore,
    )?;
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
    let result = engine.RecognizeAsync(&bitmap)?.get()?;
    Ok(result.Text()?.to_string())
}
