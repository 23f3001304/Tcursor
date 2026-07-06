/// Refresh rate (Hz) of the primary display, used as the capture/encode framerate.
/// Falls back to 60 if it can't be read.
#[cfg(windows)]
pub fn primary_refresh_hz() -> u32 {
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
    let mut dm = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    unsafe {
        if EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut dm).as_bool() {
            let hz = dm.dmDisplayFrequency;
            if hz >= 24 {
                return hz;
            }
        }
    }
    60
}

#[cfg(not(windows))]
pub fn primary_refresh_hz() -> u32 {
    60
}
