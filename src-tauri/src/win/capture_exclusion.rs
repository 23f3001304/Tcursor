#[cfg(windows)]
pub fn exclude_from_capture(hwnd: isize) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE,
    };
    unsafe { SetWindowDisplayAffinity(HWND(hwnd as *mut _), WDA_EXCLUDEFROMCAPTURE).is_ok() }
}

#[cfg(not(windows))]
pub fn exclude_from_capture(_hwnd: isize) -> bool {
    false
}
