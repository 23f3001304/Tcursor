use crate::events::track::cursorlayer::CursorLayerBuilder;
use crate::events::track::cursortracker::CursorSamples;
use crate::events::track::cursortype::CursorType;
use crate::platform::windows::input::bitmap::capture;
use crate::platform::windows::input::pointer::push_polled_move;
use crate::session::record::pause_totals::PauseTotals;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorInfo, LoadCursorW, CURSORINFO, HCURSOR, IDC_APPSTARTING, IDC_ARROW, IDC_HAND,
    IDC_IBEAM, IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_WAIT,
};

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

fn track_shape(
    layer: &mut CursorLayerBuilder,
    seen: &mut HashMap<isize, Option<u32>>,
    last_id: &mut Option<u32>,
    h: HCURSOR,
    t: u32,
) {
    let id = match seen.get(&(h.0 as isize)) {
        Some(&known) => known,
        None => {
            let fresh = if layer.is_full() {
                None
            } else {
                capture(h).map(|c| layer.add(c))
            };
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

fn run(stop: Arc<AtomicBool>, ledger: Arc<PauseTotals>) -> CursorSamples {
    let base = Instant::now();
    let table = classify_table();
    let mut samples: Vec<(u32, CursorType)> = Vec::new();
    let mut last = CursorType::Arrow;
    let (mut layer, mut seen, mut last_id) = (CursorLayerBuilder::default(), HashMap::new(), None);
    while !stop.load(Relaxed) {
        let mut info = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetCursorInfo(&mut info) }.is_ok() {
            push_polled_move(info.ptScreenPos.x, info.ptScreenPos.y);
            let t = ledger.stamp(base.elapsed().as_millis() as u64);
            if let Some(&(_, ty)) = table.iter().find(|&&(h, _)| h == info.hCursor) {
                if samples.last().map(|s| s.1) != Some(ty) {
                    samples.push((t, ty));
                }
                last = ty;
            } else if samples.is_empty() {
                samples.push((t, last));
            }
            track_shape(&mut layer, &mut seen, &mut last_id, info.hCursor, t);
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    (samples, layer)
}

pub struct Win32CursorShapes {
    thread: Option<JoinHandle<CursorSamples>>,
    stop: Arc<AtomicBool>,
}

impl Win32CursorShapes {
    pub fn start(ledger: Arc<PauseTotals>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread = std::thread::Builder::new()
            .name("cursor-type".into())
            .spawn(move || run(thread_stop, ledger))
            .ok();
        Self { thread, stop }
    }

    pub fn stop(mut self) -> CursorSamples {
        self.stop.store(true, Relaxed);
        self.thread
            .take()
            .map(|t| t.join().unwrap_or_default())
            .unwrap_or_default()
    }
}

impl Drop for Win32CursorShapes {
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.stop.store(true, Relaxed);
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
        }
    }
}
