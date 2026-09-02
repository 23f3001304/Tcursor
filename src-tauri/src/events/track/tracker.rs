use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;
use crate::events::collector::EventCollector;
use crate::events::model::{Button, EventKind};
use crate::session::record::pause_totals::PauseTotals;

// Global sink: hook_proc runs on the hook thread, must reach collector without passing closures.
struct Sink { start: Instant, collector: EventCollector, ledger: Arc<PauseTotals> }
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
                        let raw = sink.start.elapsed().as_millis() as u64;
                        let t = sink.ledger.stamp(raw);
                        sink.collector.push(t, kind, info.pt.x, info.pt.y, button);
                    }
                }
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    pub fn run(move_min_interval_ms: u32, ledger: Arc<PauseTotals>, set_id: impl FnOnce(u32)) {
        *SINK.lock().unwrap() = Some(Sink {
            start: Instant::now(),
            collector: EventCollector::new(move_min_interval_ms),
            ledger,
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
    pub fn start(move_min_interval_ms: u32, ledger: Arc<PauseTotals>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("mouse-hook".into())
            .spawn(move || {
                #[cfg(windows)]
                imp::run(move_min_interval_ms, ledger, |id| { let _ = tx.send(id); });
                #[cfg(not(windows))]
                { let _ = ledger; let _ = tx.send(0u32); }
            })
            .ok();
        let thread_id = rx.recv().unwrap_or(0);
        Self { thread, thread_id }
    }

    pub fn stop(mut self) -> Vec<crate::events::model::MouseEvent> {
        #[cfg(windows)]
        imp::post_quit(self.thread_id);
        if let Some(t) = self.thread.take() { let _ = t.join(); }
        // Poison-tolerant like every other lock in the recording path: `hook_proc` skips a
        // poisoned SINK silently, so a raw unwrap here would turn "stopped collecting events"
        // into a panicked `stop_recording` command and lose the whole take's inputs.
        SINK.lock().unwrap_or_else(|e| e.into_inner()).take().map(|s| s.collector.take()).unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::record::pause_totals::PauseTotals;

    // Mirrors what hook_proc does at its stamp site (raw elapsed ms -> ledger.stamp ->
    // collector.push) without needing an actual Windows hook: this is the "collector that
    // receives (t, kind, x, y)" seam called out in the task brief. Paused 1000..3000; an
    // event whose raw elapsed reading is 3100 must collapse to 3100 - 2000 = 1100.
    #[test]
    fn paused_span_is_subtracted_before_reaching_the_collector() {
        let ledger = PauseTotals::new();
        ledger.pause(1000);
        ledger.resume(3000);

        let mut collector = EventCollector::new(0);
        let t = ledger.stamp(3100);
        collector.push(t, EventKind::Move, 5, 5, None);

        let events = collector.take();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].t, 1100);
    }
}
