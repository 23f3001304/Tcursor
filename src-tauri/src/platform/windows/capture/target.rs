use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowLongW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    IsWindowVisible, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
};
use windows_capture::monitor::Monitor;

use crate::ports::capture::{CaptureGeometry, CaptureTarget, TargetId, TargetKind};

pub(super) fn get_target_bounds(target_id: Option<&str>) -> CaptureGeometry {
    match TargetId::from_arg(target_id) {
        TargetId::Window(handle) => {
            if let Some(r) = visible_rect(HWND(handle as usize as *mut _)) {
                let w = (r.right - r.left).max(100) as u32;
                let h = (r.bottom - r.top).max(100) as u32;
                return CaptureGeometry {
                    w,
                    h,
                    origin_x: r.left,
                    origin_y: r.top,
                };
            }
        }
        TargetId::Display(index) => {
            if let Some(b) = display_bounds(index) {
                return b;
            }
        }
        TargetId::Primary => {}
    }
    primary_bounds()
}

fn primary_bounds() -> CaptureGeometry {
    let (w, h) = match Monitor::primary() {
        Ok(monitor) => (
            monitor.width().unwrap_or(1920),
            monitor.height().unwrap_or(1080),
        ),
        Err(_) => (1920, 1080),
    };
    CaptureGeometry {
        w,
        h,
        origin_x: 0,
        origin_y: 0,
    }
}

struct MonCtx {
    want: Vec<u16>,
    bounds: Option<CaptureGeometry>,
}

unsafe extern "system" fn enum_mon_cb(
    hmon: HMONITOR,
    _: HDC,
    _: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut MonCtx);
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if GetMonitorInfoW(hmon, &mut info.monitorInfo).as_bool() {
        let n = info
            .szDevice
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(info.szDevice.len());
        if info.szDevice[..n] == ctx.want[..] {
            let r = info.monitorInfo.rcMonitor;
            ctx.bounds = Some(CaptureGeometry {
                w: (r.right - r.left) as u32,
                h: (r.bottom - r.top) as u32,
                origin_x: r.left,
                origin_y: r.top,
            });
        }
    }
    BOOL(1)
}

// INVARIANT: a display is matched by GDI device name, never by enumeration index - the two
// enumerations are in different orders, and indexing here would take another monitor's origin.
fn display_bounds(index: usize) -> Option<CaptureGeometry> {
    let dev = Monitor::from_index(index)
        .ok()
        .and_then(|m| m.device_name().ok())?;
    let mut ctx = MonCtx {
        want: dev.encode_utf16().collect(),
        bounds: None,
    };
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_mon_cb),
            LPARAM(&mut ctx as *mut _ as isize),
        );
    }
    ctx.bounds
}

// INVARIANT: the pipelines size a window capture from the OUTER `GetWindowRect`, not from
// `visible_rect` - WGC frames a window by that rectangle, and the two differ by the invisible
// resize borders.
pub(super) fn capture_window_size(hwnd: HWND) -> (u32, u32) {
    let mut r = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut r) }.is_ok() {
        (
            (r.right - r.left).max(100) as u32,
            (r.bottom - r.top).max(100) as u32,
        )
    } else {
        (1920, 1080)
    }
}

fn visible_rect(hwnd: HWND) -> Option<RECT> {
    let mut r = RECT::default();
    let dwm = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut r as *mut RECT as *mut std::ffi::c_void,
            std::mem::size_of::<RECT>() as u32,
        )
    };
    if dwm.is_ok() && r.right > r.left && r.bottom > r.top {
        return Some(r);
    }
    let mut r = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut r) }.ok().map(|_| r)
}

pub(super) fn list_targets() -> Vec<CaptureTarget> {
    let mut targets = displays();
    if targets.is_empty() {
        targets.push(CaptureTarget {
            id: TargetId::Display(0),
            label: "Primary Display".into(),
            kind: TargetKind::Display,
        });
    }
    targets.extend(app_windows());
    targets
}

fn displays() -> Vec<CaptureTarget> {
    let Ok(monitors) = Monitor::enumerate() else {
        return Vec::new();
    };
    monitors
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let name = m.name().unwrap_or_else(|_| format!("Display {}", i + 1));
            let w = m.width().unwrap_or(0);
            let h = m.height().unwrap_or(0);
            let label = match (i == 0, w > 0 && h > 0) {
                (true, true) => format!("Display {}: {} ({}x{}, Primary)", i + 1, name, w, h),
                (true, false) => format!("Display {}: {} (Primary)", i + 1, name),
                (false, true) => format!("Display {}: {} ({}x{})", i + 1, name, w, h),
                (false, false) => format!("Display {}: {}", i + 1, name),
            };
            CaptureTarget {
                id: TargetId::Display(i),
                label,
                kind: TargetKind::Display,
            }
        })
        .collect()
}

unsafe extern "system" fn enum_windows_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = &mut *(lparam.0 as *mut Vec<CaptureTarget>);
    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }
    let len = GetWindowTextLengthW(hwnd);
    if len <= 0 || (GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0) != 0 {
        return BOOL(1);
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    let read = GetWindowTextW(hwnd, &mut buf);
    if read <= 0 {
        return BOOL(1);
    }
    let title = String::from_utf16_lossy(&buf[..read as usize])
        .trim()
        .to_string();
    if !title.is_empty()
        && title != "Program Manager"
        && title != "Settings"
        && title != "TCursor"
        && !title.starts_with("MSCTFIME")
    {
        list.push(CaptureTarget {
            id: TargetId::Window(hwnd.0 as usize as u64),
            label: format!("App: {}", title),
            kind: TargetKind::Window,
        });
    }
    BOOL(1)
}

fn app_windows() -> Vec<CaptureTarget> {
    let mut list: Vec<CaptureTarget> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(enum_windows_cb), LPARAM(&mut list as *mut _ as isize));
    }
    list
}

#[cfg(test)]
#[path = "target_tests.rs"]
mod tests;
