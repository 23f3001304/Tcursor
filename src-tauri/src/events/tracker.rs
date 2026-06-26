use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Instant;
use crate::events::collector::EventCollector;
use crate::events::model::{Button, EventKind};

// Global sink: hook_proc runs on the hook thread, must reach collector without passing closures.
struct Sink { start: Instant, collector: EventCollector }
static SINK: Mutex<Option<Sink>> = Mutex::new(None);

pub struct MouseTracker {
    thread: Option<JoinHandle<()>>,
    thread_id: u32,
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
        MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
        WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP,
    };

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let mapped = match wparam.0 as u32 {
                WM_MOUSEMOVE    => Some((EventKind::Move, None)),
                WM_LBUTTONDOWN  => Some((EventKind::Down, Some(Button::Left))),
                WM_LBUTTONUP    => Some((EventKind::Up,   Some(Button::Left))),
                WM_RBUTTONDOWN  => Some((EventKind::Down, Some(Button::Right))),
                WM_RBUTTONUP    => Some((EventKind::Up,   Some(Button::Right))),
                _ => None,
            };
            if let Some((kind, button)) = mapped {
                if let Ok(mut g) = SINK.lock() {
                    if let Some(sink) = g.as_mut() {
                        let t = sink.start.elapsed().as_millis() as u32;
                        sink.collector.push(t, kind, info.pt.x, info.pt.y, button);
                    }
                }
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    pub fn run(move_min_interval_ms: u32, set_id: impl FnOnce(u32)) {
        *SINK.lock().unwrap() = Some(Sink {
            start: Instant::now(),
            collector: EventCollector::new(move_min_interval_ms),
        });
        unsafe {
            set_id(GetCurrentThreadId());
            let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), None, 0);
            let hook = match hook { Ok(h) => h, Err(_) => return };
            let mut msg = MSG::default();
            // WM_QUIT (posted by stop) makes GetMessageW return false, ending the loop.
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
            let _ = UnhookWindowsHookEx(hook);
        }
    }

    pub fn post_quit(thread_id: u32) {
        unsafe { let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0)); }
    }
}

impl MouseTracker {
    pub fn start(move_min_interval_ms: u32) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("mouse-hook".into())
            .spawn(move || {
                #[cfg(windows)]
                imp::run(move_min_interval_ms, |id| { let _ = tx.send(id); });
                #[cfg(not(windows))]
                { let _ = tx.send(0u32); }
            })
            .ok();
        let thread_id = rx.recv().unwrap_or(0);
        Self { thread, thread_id }
    }

    pub fn stop(mut self) -> Vec<crate::events::model::MouseEvent> {
        #[cfg(windows)]
        imp::post_quit(self.thread_id);
        if let Some(t) = self.thread.take() { let _ = t.join(); }
        SINK.lock().unwrap().take().map(|s| s.collector.take()).unwrap_or_default()
    }
}

impl Drop for MouseTracker {
    // Covers any drop without stop() (e.g. start_recording error path): post WM_QUIT
    // so the hook thread exits instead of leaking. No-op after stop() (thread is None).
    fn drop(&mut self) {
        if self.thread.is_some() {
            #[cfg(windows)]
            imp::post_quit(self.thread_id);
            if let Some(t) = self.thread.take() { let _ = t.join(); }
        }
    }
}
