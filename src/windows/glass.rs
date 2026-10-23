//! Optional system blur for the passive widget. No capture buffers or refresh timer.
//! Accent-policy values are an optional Windows capability, resolved at runtime.
use super::*;
use windows::Win32::Graphics::Dwm::DwmIsCompositionEnabled;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};

#[repr(C)]
struct Accent {
    state: i32,
    flags: u32,
    tint: u32,
    animation: u32,
}
#[repr(C)]
struct Attribute {
    attribute: i32,
    data: *mut Accent,
    size: usize,
}

pub unsafe fn set(hwnd: HWND, enabled: bool) -> bool {
    if enabled {
        let mut transparency = 1u32;
        let mut size = 4u32;
        let result = RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("EnableTransparency"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut transparency as *mut u32).cast()),
            Some(&mut size),
        );
        if result.is_ok() && transparency == 0 {
            set(hwnd, false);
            return false;
        }
        let mut contrast = HIGHCONTRASTW {
            cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
            ..Default::default()
        };
        let _ = SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cbSize,
            Some((&mut contrast as *mut HIGHCONTRASTW).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        if contrast.dwFlags.0 & HCF_HIGHCONTRASTON.0 != 0
            || !DwmIsCompositionEnabled().is_ok_and(|v| v.as_bool())
        {
            set(hwnd, false);
            return false;
        }
    }
    let Ok(module) = GetModuleHandleW(w!("user32.dll")) else {
        return false;
    };
    let Some(address) = GetProcAddress(
        module,
        windows::core::PCSTR(b"SetWindowCompositionAttribute\0".as_ptr()),
    ) else {
        return false;
    };
    let apply: unsafe extern "system" fn(HWND, *mut Attribute) -> BOOL =
        std::mem::transmute(address);
    // Blur uses DWM's shared compositor rather than periodically copying the desktop.
    let mut accent = Accent {
        state: if enabled { 3 } else { 0 },
        flags: 2,
        tint: 0,
        animation: 0,
    };
    let mut data = Attribute {
        attribute: 19,
        data: &mut accent,
        size: std::mem::size_of::<Accent>(),
    };
    apply(hwnd, &mut data).as_bool()
}
