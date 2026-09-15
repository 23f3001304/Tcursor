use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowDisplayAffinity, SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
};

use crate::ports::system::{SystemPort, WindowHandle};

pub struct Win32System;

impl SystemPort for Win32System {
    fn primary_refresh_hz(&self) -> u32 {
        let mut dm = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        unsafe {
            if EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut dm).as_bool() {
                let hz = dm.dmDisplayFrequency;
                // INVARIANT: sub-24 Hz means a partial DEVMODEW, not a real display.
                if hz >= 24 {
                    return hz;
                }
            }
        }
        60
    }

    fn os_prefers_dark(&self) -> bool {
        use std::ffi::c_void;
        let subkey =
            windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
        let value = windows::core::w!("AppsUseLightTheme");
        let mut data: u32 = 0;
        let mut size: u32 = 4;
        let result = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                subkey,
                value,
                RRF_RT_REG_DWORD,
                None,
                Some(&mut data as *mut u32 as *mut c_void),
                Some(&mut size),
            )
        };
        result.is_ok() && data == 0
    }

    fn exclude_from_capture(&self, window: WindowHandle, exclude: bool) -> bool {
        let want = if exclude {
            WDA_EXCLUDEFROMCAPTURE
        } else {
            WDA_NONE
        };
        let h = HWND(window.0 as *mut _);
        let set_ok = unsafe { SetWindowDisplayAffinity(h, want).is_ok() };
        let mut have: u32 = 0;
        let read_ok = unsafe { GetWindowDisplayAffinity(h, &mut have).is_ok() };
        // INVARIANT: the answer is the affinity READ BACK, never the setter's return.
        let ok = set_ok && read_ok && have == want.0;
        if !ok {
            eprintln!("capture exclusion: wanted {:#x}, window has {have:#x} (set_ok={set_ok}, read_ok={read_ok})", want.0);
        }
        ok
    }
}
