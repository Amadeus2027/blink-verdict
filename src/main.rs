#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod windows;

fn main() {
    #[cfg(windows)]
    windows::run();
    #[cfg(not(windows))]
    eprintln!("BlinkVerdict is a Windows 10/11 desktop application.");
}
