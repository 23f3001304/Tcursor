//! Where a capture target actually sits on the desktop: the size and the origin of a monitor or
//! an application window, in the same absolute coordinates the mouse hook reports.
//!
//! Split from `video_sink.rs` (which picks and starts the pipeline) because the origin is now
//! asked for twice - once when a take starts, to fill the take's one `ScreenInfo`, and again on
//! every mid-take display switch, to build the `events::remap::Remap` that puts later mouse
//! samples where the pixels are.

/// `(w, h, origin_x, origin_y)` for `target_id` (`window:0x…`, `display:N`, or `None` for the
/// primary display). Falls back to the primary display, and finally to 1920x1080 at the origin,
/// so a caller always gets a usable rectangle.
pub fn get_target_bounds(target_id: Option<&str>) -> (u32, u32, i32, i32) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{HWND, RECT};
        use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW};

        if let Some(tid) = target_id {
            if let Some(hex) = tid.strip_prefix("window:0x") {
                if let Ok(hwnd_val) = usize::from_str_radix(hex, 16) {
                    let hwnd = HWND(hwnd_val as *mut _);
                    if let Some(r) = visible_rect(hwnd) {
                        let w = (r.right - r.left).max(100) as u32;
                        let h = (r.bottom - r.top).max(100) as u32;
                        return (w, h, r.left, r.top);
                    }
                }
            } else if let Some(idx_str) = tid.strip_prefix("display:") {
                // Resolve the origin for the monitor windows_capture ACTUALLY captures
                // (Monitor::from_index, the same order list_displays + GpuRecorder use),
                // correlating to its Win32 rect by GDI device name. Indexing EnumDisplayMonitors
                // directly used a DIFFERENT order, so display:N could take another monitor's origin
                // and shift every cursor point by the delta - the "cursor drawn in the wrong place"
                // regression. Matching device names keeps the video and the origin on one screen.
                if let Some(dev) = idx_str.parse::<usize>().ok()
                    .and_then(|i| windows_capture::monitor::Monitor::from_index(i).ok())
                    .and_then(|m| m.device_name().ok())
                {
                    struct MonCtx { want: Vec<u16>, bounds: Option<(u32, u32, i32, i32)> }
                    unsafe extern "system" fn enum_mon_cb(hmon: HMONITOR, _: HDC, _: *mut RECT, lparam: windows::Win32::Foundation::LPARAM) -> windows::Win32::Foundation::BOOL {
                        let ctx = &mut *(lparam.0 as *mut MonCtx);
                        let mut info = MONITORINFOEXW::default();
                        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
                        if GetMonitorInfoW(hmon, &mut info.monitorInfo).as_bool() {
                            let n = info.szDevice.iter().position(|&c| c == 0).unwrap_or(info.szDevice.len());
                            if info.szDevice[..n] == ctx.want[..] {
                                let r = info.monitorInfo.rcMonitor;
                                ctx.bounds = Some(((r.right - r.left) as u32, (r.bottom - r.top) as u32, r.left, r.top));
                            }
                        }
                        windows::Win32::Foundation::BOOL(1)
                    }
                    let mut ctx = MonCtx { want: dev.encode_utf16().collect(), bounds: None };
                    unsafe { let _ = EnumDisplayMonitors(HDC::default(), None, Some(enum_mon_cb), windows::Win32::Foundation::LPARAM(&mut ctx as *mut _ as isize)); }
                    if let Some(b) = ctx.bounds { return b; }
                }
            }
        }
    }
    if let Ok(mon) = windows_capture::monitor::Monitor::primary() {
        let w = mon.width().unwrap_or(1920);
        let h = mon.height().unwrap_or(1080);
        return (w, h, 0, 0);
    }
    (1920, 1080, 0, 0)
}

/// The rectangle of a window's PIXELS, which is what WGC captures: `DWMWA_EXTENDED_FRAME_BOUNDS`,
/// the visible frame without the invisible resize borders. `GetWindowRect` includes those borders
/// (a maximized window reads 1944x1104 at -12,-12 while its capture is 1920x1080 at 0,0), and a
/// take that stored the outer rectangle as its `ScreenInfo` drew every cursor and click 12 px
/// down and to the right of where it happened. Falls back to `GetWindowRect` if DWM declines.
#[cfg(windows)]
fn visible_rect(hwnd: windows::Win32::Foundation::HWND) -> Option<windows::Win32::Foundation::RECT> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    let mut r = RECT::default();
    let dwm = unsafe { DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS,
        &mut r as *mut RECT as *mut std::ffi::c_void, std::mem::size_of::<RECT>() as u32) };
    if dwm.is_ok() && r.right > r.left && r.bottom > r.top { return Some(r); }
    let mut r = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut r) }.ok().map(|_| r)
}
