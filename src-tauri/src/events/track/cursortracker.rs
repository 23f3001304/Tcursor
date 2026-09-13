use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;
use crate::events::track::cursorlayer::CursorLayerBuilder;
use crate::events::track::cursortype::CursorType;
use crate::session::record::pause_totals::PauseTotals;

/// What one take's cursor polling produced: the `(t_ms, type)` shape log Enhanced replays from,
/// and the captured OS-cursor layer System composites (see `cursorlayer`).
pub type CursorSamples = (Vec<(u32, CursorType)>, CursorLayerBuilder);

/// Polls the global cursor (~60 Hz) and logs `(t_ms, type)` on every change, while capturing each
/// distinct cursor's real bitmap into a `CursorLayerBuilder`. The display is always captured
/// WITHOUT the OS cursor, so both the shape and the pixels have to be recorded live here.
pub struct CursorTypeTracker {
    thread: Option<JoinHandle<CursorSamples>>,
    stop: Arc<AtomicBool>,
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::Ordering::Relaxed;
    use std::time::{Duration, Instant};
    use crate::events::track::cursorcapture::capture;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCursorInfo, LoadCursorW, CURSORINFO, HCURSOR, IDC_APPSTARTING, IDC_ARROW, IDC_HAND,
        IDC_IBEAM, IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_WAIT,
    };

    /// Build the (standard handle -> type) classification table once at thread start.
    fn classify_table() -> Vec<(HCURSOR, CursorType)> {
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

    /// Capture `h`'s bitmap the first time this handle is seen, and append `(t, id)` whenever the
    /// shown cursor changes. `seen` also remembers FAILURES (as `None`) so an uncapturable cursor
    /// is not retried 60 times a second; such a poll leaves the previous id in force, exactly as
    /// an unrecognized handle leaves the previous type in force above.
    fn track_shape(layer: &mut CursorLayerBuilder, seen: &mut HashMap<isize, Option<u32>>,
                   last_id: &mut Option<u32>, h: HCURSOR, t: u32) {
        let id = match seen.get(&(h.0 as isize)) {
            Some(&known) => known,
            None => {
                let fresh = if layer.is_full() { None } else { capture(h).map(|c| layer.add(c)) };
                seen.insert(h.0 as isize, fresh);
                fresh
            }
        };
        if let Some(id) = id {
            if *last_id != Some(id) {
                layer.mark(t, id);
                *last_id = Some(id);
            }
        }
    }

    pub fn run(stop: Arc<AtomicBool>, ledger: Arc<PauseTotals>) -> CursorSamples {
        let base = Instant::now();
        let table = classify_table();
        let mut samples: Vec<(u32, CursorType)> = Vec::new();
        let mut last = CursorType::Arrow;
        let (mut layer, mut seen, mut last_id) = (CursorLayerBuilder::default(), HashMap::new(), None);
        while !stop.load(Relaxed) {
            let mut info = CURSORINFO { cbSize: std::mem::size_of::<CURSORINFO>() as u32, ..Default::default() };
            if unsafe { GetCursorInfo(&mut info) }.is_ok() {
                // One stamp per poll, shared by both logs so a shape change and the bitmap
                // change that goes with it can never land on different milliseconds.
                let t = ledger.stamp(base.elapsed().as_millis() as u64);
                // Match the live handle against the standard set. No match = custom
                // app cursor; keep the last known type rather than guessing.
                if let Some(&(_, ty)) = table.iter().find(|&&(h, _)| h == info.hCursor) {
                    if samples.last().map(|s| s.1) != Some(ty) {
                        samples.push((t, ty));
                    }
                    last = ty;
                } else if samples.is_empty() {
                    // First sample is custom: seed with the last (Arrow) so type_at has a base.
                    samples.push((t, last));
                }
                // The bitmap layer has no such fallback - a custom app cursor is exactly the case
                // the type track cannot describe, and the one the captured layer gets right.
                track_shape(&mut layer, &mut seen, &mut last_id, info.hCursor, t);
            }
            std::thread::sleep(Duration::from_millis(16));
        }
        (samples, layer)
    }
}

impl CursorTypeTracker {
    pub fn start(ledger: Arc<PauseTotals>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread = std::thread::Builder::new()
            .name("cursor-type".into())
            .spawn(move || {
                #[cfg(windows)]
                { imp::run(thread_stop, ledger) }
                #[cfg(not(windows))]
                { let _ = (thread_stop, ledger); <CursorSamples as Default>::default() }
            })
            .ok();
        Self { thread, stop }
    }

    pub fn stop(mut self) -> CursorSamples {
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
