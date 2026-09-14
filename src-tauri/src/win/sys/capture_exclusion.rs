/// Toggle whether a window is excluded from screen capture / screenshots.
/// `exclude = true` sets WDA_EXCLUDEFROMCAPTURE (the HUD, hidden from recordings);
/// `exclude = false` restores WDA_NONE (the editor, which opts back into capture).
///
/// Returns whether the window now HAS the requested affinity, read back rather than trusted:
/// the owner saw the take pill in a display capture (2026-09-14) even though the flag was set at
/// startup, so callers re-apply it after every HUD window morph and this reports the truth each
/// time. A mismatch is logged once per call so a dev console shows exactly when Windows dropped it.
#[cfg(windows)]
pub fn set_capture_exclusion(hwnd: isize, exclude: bool) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowDisplayAffinity, SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    };
    let want = if exclude { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE };
    let h = HWND(hwnd as *mut _);
    let set_ok = unsafe { SetWindowDisplayAffinity(h, want).is_ok() };
    let mut have: u32 = 0; // the raw affinity bits; `want.0` is the same encoding
    let read_ok = unsafe { GetWindowDisplayAffinity(h, &mut have).is_ok() };
    let ok = set_ok && read_ok && have == want.0;
    if !ok { eprintln!("capture exclusion: wanted {:#x}, window has {have:#x} (set_ok={set_ok}, read_ok={read_ok})", want.0); }
    ok
}

#[cfg(not(windows))]
pub fn set_capture_exclusion(_hwnd: isize, _exclude: bool) -> bool {
    false
}
