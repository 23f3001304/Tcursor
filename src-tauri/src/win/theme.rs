use crate::settings::model::ThemeMode;

#[cfg(windows)]
pub fn os_prefers_dark() -> bool {
    use std::ffi::c_void;
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD,
    };
    let subkey = windows::core::w!(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"
    );
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
    // AppsUseLightTheme == 0 means dark mode is active
    result.is_ok() && data == 0
}

#[cfg(not(windows))]
pub fn os_prefers_dark() -> bool {
    false
}

pub fn resolve_dark(theme: ThemeMode) -> bool {
    match theme {
        ThemeMode::Light => false,
        ThemeMode::Dark => true,
        ThemeMode::System => os_prefers_dark(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::model::ThemeMode;

    #[test]
    fn light_is_not_dark() {
        assert!(!resolve_dark(ThemeMode::Light));
    }

    #[test]
    fn dark_is_dark() {
        assert!(resolve_dark(ThemeMode::Dark));
    }

    #[test]
    fn system_does_not_panic() {
        let _: bool = resolve_dark(ThemeMode::System);
    }
}
