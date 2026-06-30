/// Toggle whether a window is excluded from screen capture / screenshots.
/// `exclude = true` sets WDA_EXCLUDEFROMCAPTURE (the HUD, hidden from recordings);
/// `exclude = false` restores WDA_NONE (the editor, which opts back into capture).
#[cfg(windows)]
pub fn set_capture_exclusion(hwnd: isize, exclude: bool) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    };
    let affinity = if exclude { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE };
    unsafe { SetWindowDisplayAffinity(HWND(hwnd as *mut _), affinity).is_ok() }
}

#[cfg(not(windows))]
pub fn set_capture_exclusion(_hwnd: isize, _exclude: bool) -> bool {
    false
}
