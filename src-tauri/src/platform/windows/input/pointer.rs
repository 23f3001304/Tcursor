use crate::events::collector::EventCollector;
use crate::events::model::{Button, EventKind, MouseEvent};
use crate::events::remap::Remap;
use crate::session::record::pause_totals::PauseTotals;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, MSG,
    MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_QUIT,
    WM_RBUTTONDOWN, WM_RBUTTONUP,
};

struct Sink {
    start: Instant,
    collector: EventCollector,
    ledger: Arc<PauseTotals>,
    remap: Option<Remap>,
}

// INVARIANT: private to this file. `hook_proc` is a bare `extern "system" fn` with no user
// pointer, so the callback has no other way to reach the collector.
static SINK: Mutex<Option<Sink>> = Mutex::new(None);

fn remapped(remap: Option<&Remap>, x: i32, y: i32) -> (i32, i32) {
    remap.map_or((x, y), |r| r.apply(x, y))
}

// INVARIANT: the ONE stamp site. The hook thread and the 16 ms poll both reach the collector
// through here, under the sink's lock, so push order is time order.
fn stamp_push(sink: &mut Sink, kind: EventKind, x: i32, y: i32, button: Option<Button>) {
    let raw = sink.start.elapsed().as_millis() as u64;
    let t = sink.ledger.stamp(raw);
    let (x, y) = remapped(sink.remap.as_ref(), x, y);
    sink.collector.push(t, kind, x, y, button);
}

pub(crate) fn push_polled_move(x: i32, y: i32) {
    if let Ok(mut g) = SINK.lock() {
        if let Some(sink) = g.as_mut() {
            stamp_push(sink, EventKind::Move, x, y, None);
        }
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        let mapped = match wparam.0 as u32 {
            WM_MOUSEMOVE => Some((EventKind::Move, None)),
            WM_LBUTTONDOWN => Some((EventKind::Down, Some(Button::Left))),
            WM_LBUTTONUP => Some((EventKind::Up, Some(Button::Left))),
            WM_RBUTTONDOWN => Some((EventKind::Down, Some(Button::Right))),
            WM_RBUTTONUP => Some((EventKind::Up, Some(Button::Right))),
            _ => None,
        };
        if let Some((kind, button)) = mapped {
            if let Ok(mut g) = SINK.lock() {
                if let Some(sink) = g.as_mut() {
                    stamp_push(sink, kind, info.pt.x, info.pt.y, button);
                }
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

fn run(move_min_interval_ms: u32, ledger: Arc<PauseTotals>, set_id: impl FnOnce(u32)) {
    *SINK.lock().unwrap() = Some(Sink {
        start: Instant::now(),
        collector: EventCollector::new(move_min_interval_ms),
        ledger,
        remap: None,
    });
    unsafe {
        set_id(GetCurrentThreadId());
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), None, 0);
        let hook = match hook {
            Ok(h) => h,
            Err(_) => return,
        };
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
        let _ = UnhookWindowsHookEx(hook);
    }
}

fn post_quit(thread_id: u32) {
    unsafe {
        let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
    }
}

pub struct Win32Pointer {
    thread: Option<JoinHandle<()>>,
    thread_id: u32,
}

impl Win32Pointer {
    pub fn start(move_min_interval_ms: u32, ledger: Arc<PauseTotals>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("mouse-hook".into())
            .spawn(move || {
                run(move_min_interval_ms, ledger, |id| {
                    let _ = tx.send(id);
                })
            })
            .ok();
        let thread_id = rx.recv().unwrap_or(0);
        Self { thread, thread_id }
    }

    pub fn set_remap(&self, remap: Option<Remap>) {
        if let Some(sink) = SINK.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            sink.remap = remap;
        }
    }

    pub fn stop(mut self) -> Vec<MouseEvent> {
        post_quit(self.thread_id);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        SINK.lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .map(|s| s.collector.take())
            .unwrap_or_default()
    }
}

impl Drop for Win32Pointer {
    fn drop(&mut self) {
        if self.thread.is_some() {
            post_quit(self.thread_id);
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
        }
    }
}

#[cfg(test)]
#[path = "pointer_tests.rs"]
mod tests;
