use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;
use crate::events::cursortype::CursorType;

/// Polls the global cursor shape (~60 Hz) and logs `(t_ms, type)` on every change.
/// Enhanced capture hides the OS cursor, so the shape must be recorded live here.
pub struct CursorTypeTracker {
    thread: Option<JoinHandle<Vec<(u32, CursorType)>>>,
    stop: Arc<AtomicBool>,
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::sync::atomic::Ordering::Relaxed;
    use std::time::{Duration, Instant};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCursorInfo, LoadCursorW, CURSORINFO, IDC_APPSTARTING, IDC_ARROW, IDC_HAND, IDC_IBEAM,
        IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_WAIT,
    };

    /// Build the (standard handle -> type) classification table once at thread start.
    fn classify_table() -> Vec<(windows::Win32::UI::WindowsAndMessaging::HCURSOR, CursorType)> {
        let pairs = [
            (IDC_ARROW, CursorType::Arrow),
            (IDC_IBEAM, CursorType::IBeam),
            (IDC_HAND, CursorType::Hand),
            (IDC_SIZENS, CursorType::ResizeNs),
            (IDC_SIZEWE, CursorType::ResizeEw),
            (IDC_SIZENWSE, CursorType::ResizeNwse),
            (IDC_SIZENESW, CursorType::ResizeNesw),
            (IDC_SIZEALL, CursorType::Move),
            (IDC_WAIT, CursorType::Busy),
            (IDC_APPSTARTING, CursorType::Busy),
        ];
        let mut table = Vec::with_capacity(pairs.len());
        for (idc, ty) in pairs {
            if let Ok(h) = unsafe { LoadCursorW(None, idc) } {
                table.push((h, ty));
            }
        }
        table
    }

    pub fn run(stop: Arc<AtomicBool>) -> Vec<(u32, CursorType)> {
        let base = Instant::now();
        let table = classify_table();
        let mut samples: Vec<(u32, CursorType)> = Vec::new();
        let mut last = CursorType::Arrow;
        while !stop.load(Relaxed) {
            let mut info = CURSORINFO { cbSize: std::mem::size_of::<CURSORINFO>() as u32, ..Default::default() };
            if unsafe { GetCursorInfo(&mut info) }.is_ok() {
                // Match the live handle against the standard set. No match = custom
                // app cursor; keep the last known type rather than guessing.
                if let Some(&(_, ty)) = table.iter().find(|&&(h, _)| h == info.hCursor) {
                    if samples.last().map(|s| s.1) != Some(ty) {
                        samples.push((base.elapsed().as_millis() as u32, ty));
                    }
                    last = ty;
                } else if samples.is_empty() {
                    // First sample is custom: seed with the last (Arrow) so type_at has a base.
                    samples.push((base.elapsed().as_millis() as u32, last));
                }
            }
            std::thread::sleep(Duration::from_millis(16));
        }
        samples
    }
}

impl CursorTypeTracker {
    pub fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread = std::thread::Builder::new()
            .name("cursor-type".into())
            .spawn(move || {
                #[cfg(windows)]
                { imp::run(thread_stop) }
                #[cfg(not(windows))]
                { let _ = thread_stop; Vec::new() }
            })
            .ok();
        Self { thread, stop }
    }

    pub fn stop(mut self) -> Vec<(u32, CursorType)> {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        self.thread.take().map(|t| t.join().unwrap_or_default()).unwrap_or_default()
    }
}

impl Drop for CursorTypeTracker {
    // Covers any drop without stop() (e.g. start_recording error path): signal + join
    // so the polling thread exits instead of leaking. No-op after stop() (thread is None).
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
            if let Some(t) = self.thread.take() { let _ = t.join(); }
        }
    }
}
