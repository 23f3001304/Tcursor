use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;
use crate::events::collector::EventCollector;
use crate::events::model::{Button, EventKind};
use crate::events::remap::Remap;
use crate::session::record::pause_totals::PauseTotals;

// Global sink: hook_proc runs on the hook thread, must reach collector without passing closures.
// `remap` is `None` for the whole of an ordinary take and `Some` only while `switch_display` has
// moved the capture to another monitor (`events/remap.rs`).
struct Sink { start: Instant, collector: EventCollector, ledger: Arc<PauseTotals>, remap: Option<Remap> }
static SINK: Mutex<Option<Sink>> = Mutex::new(None);

/// The point one hook sample contributes: the raw desktop coordinate, or where it lands on the
/// take's own display once a mid-take display switch has installed a `Remap`. A named function so
/// the mapping can be tested at the stamp site without a live Windows hook.
fn remapped(remap: Option<&Remap>, x: i32, y: i32) -> (i32, i32) {
    remap.map_or((x, y), |r| r.apply(x, y))
}

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
                        let (x, y) = remapped(sink.remap.as_ref(), info.pt.x, info.pt.y);
                        sink.collector.push(t, kind, x, y, button);
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
            remap: None,
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

    /// Install (or clear) the display remap every later sample is mapped through. Called by
    /// `switch_display` under the recorder lock; `None` restores raw desktop coordinates, which
    /// is what a switch BACK to the take's own display wants. A no-op before `start`'s thread has
    /// filled the sink, and poison-tolerant like every other lock on the recording path: losing
    /// the mapping must not turn a source switch into a panicked command.
    pub fn set_remap(&self, remap: Option<Remap>) {
        if let Some(sink) = SINK.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            sink.remap = remap;
        }
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

    // The other half of the same stamp site, for a take that switched to the monitor to the right
    // of its own: the raw hook coordinate is the second monitor's, and what reaches the collector
    // is where that point sits on the take's own display - which is what `export::coordmap`
    // subtracts the take's single `ScreenInfo` origin from.
    #[test]
    fn a_switched_display_is_remapped_before_reaching_the_collector() {
        let remap = Remap::for_switch((1920, 0), (1280, 800), (0, 0), (1920, 1080));

        let mut collector = EventCollector::new(0);
        let (x, y) = remapped(Some(&remap), 1920 + 640, 400);
        collector.push(0, EventKind::Move, x, y, None);
        let (x, y) = remapped(None, 1920 + 640, 400); // the remap cleared: raw coordinates again
        collector.push(20, EventKind::Down, x, y, Some(Button::Left));

        let events = collector.take();
        assert_eq!((events[0].x, events[0].y), (960, 540), "not fitted into the canvas");
        assert_eq!((events[1].x, events[1].y), (2560, 400), "cleared remap still mapped");
    }
}
