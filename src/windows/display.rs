//! Display metadata is queried on user actions, never polled while idle.
use super::*;
use windows::Win32::Devices::Display::*;

pub unsafe fn monitor_info(monitor: HMONITOR) -> MONITORINFO {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let _ = GetMonitorInfoW(monitor, &mut info);
    info
}

pub unsafe fn dpi(monitor: HMONITOR) -> u32 {
    let (mut x, mut y) = (96, 96);
    let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y);
    x.max(96)
}

pub fn pixels(value: i32, dpi: u32) -> i32 {
    (value as f64 * dpi as f64 / 96.0).round() as i32
}
pub fn logical(value: i32, dpi: u32) -> i32 {
    (value as f64 * 96.0 / dpi.max(96) as f64).round() as i32
}

pub unsafe fn interaction_monitor() -> HMONITOR {
    let foreground = GetForegroundWindow();
    let mut class = [0u16; 80];
    let count = GetClassNameW(foreground, &mut class).max(0) as usize;
    let class = String::from_utf16_lossy(&class[..count]);
    if !foreground.is_invalid()
        && !matches!(
            class.as_str(),
            "Shell_TrayWnd"
                | "Shell_SecondaryTrayWnd"
                | "NotifyIconOverflowWindow"
                | "BlinkVerdict.Host"
        )
    {
        return MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST);
    }
    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);
    MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST)
}

/// GDI has no HDR tone mapping. Refuse enabled advanced color rather than
/// silently sending altered colors. SDR-only/older drivers may lack this query.
pub unsafe fn require_sdr(rect: RECT) -> Result<(), String> {
    let mut monitor = MONITORINFOEXW::default();
    monitor.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if !GetMonitorInfoW(
        MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST),
        &mut monitor.monitorInfo,
    )
    .as_bool()
    {
        return Err("无法读取当前显示器信息，请重试".into());
    }
    for _ in 0..3 {
        let (mut paths_count, mut modes_count) = (0, 0);
        let status =
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut paths_count, &mut modes_count);
        if status == ERROR_NOT_SUPPORTED {
            return Ok(());
        }
        if status != ERROR_SUCCESS {
            return Err("无法确认显示器色彩模式，请检查显示设置".into());
        }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); paths_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); modes_count as usize];
        let status = QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut paths_count,
            paths.as_mut_ptr(),
            &mut modes_count,
            modes.as_mut_ptr(),
            None,
        );
        if status == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if status != ERROR_SUCCESS {
            return Err("无法确认显示器色彩模式，请检查显示设置".into());
        }
        let mut matched = false;
        for path in &paths[..paths_count as usize] {
            let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
            source.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                size: std::mem::size_of_val(&source) as u32,
                adapterId: path.sourceInfo.adapterId,
                id: path.sourceInfo.id,
            };
            if DisplayConfigGetDeviceInfo(&mut source.header) != 0 {
                continue;
            }
            if source.viewGdiDeviceName != monitor.szDevice {
                continue;
            }
            matched = true;
            let mut color = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO::default();
            color.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
                size: std::mem::size_of_val(&color) as u32,
                adapterId: path.targetInfo.adapterId,
                id: path.targetInfo.id,
            };
            let status = DisplayConfigGetDeviceInfo(&mut color.header);
            if status == ERROR_NOT_SUPPORTED.0 as i32 || status == ERROR_INVALID_PARAMETER.0 as i32
            {
                continue;
            }
            if status != 0 {
                return Err("无法确认显示器色彩模式，请检查显示设置".into());
            }
            if color.Anonymous.value & 2 != 0 {
                return Err("当前显示器开启了 HDR／高级色彩，现有截图无法保证颜色准确；请先关闭该显示器的 HDR／高级色彩后再试".into());
            }
        }
        return if matched {
            Ok(())
        } else {
            Err("显示器连接已变化，请重试".into())
        };
    }
    Err("显示器配置正在变化，请稍后重试".into())
}
