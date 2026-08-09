use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use crate::capture::frame::Frame;
use crate::capture::frame_source::FrameSource;
use crate::domain::time::Clock;

/// Bridges windows-capture's callback delivery into our pull-based FrameSource
/// via an internal mpsc channel. Each captured BGRA frame is stripped of any
/// row-padding before being sent, so the consumer receives tight rows.
pub struct WgcFrameSource {
    rx: Receiver<Frame>,
    dims: (u32, u32),
    halt: Arc<AtomicBool>,
    // Calling stop() posts WM_QUIT to the WGC thread, which unblocks GetMessageW
    // → WGC thread exits → tx drops → rx.recv() returns Err → next_frame returns None.
    stopper: Option<Box<dyn FnOnce() + Send>>,
    drops: Arc<AtomicU64>, // frames dropped (queue full, encoder behind); logged once in Drop
}

// Non-blocking hand-off: drops (and counts) `frame` instead of stalling the WGC callback
// when the bounded queue is full. A free function so it's unit-testable without WGC.
fn try_send_or_drop(tx: &SyncSender<Frame>, frame: Frame, drops: &AtomicU64) {
    if tx.try_send(frame).is_err() { drops.fetch_add(1, Ordering::Relaxed); }
}

impl WgcFrameSource {
    pub fn for_primary_display(clock: Arc<dyn Clock>, fps: u32, with_cursor: bool) -> anyhow::Result<Self> {
        Self::for_target(clock, fps, with_cursor, None)
    }

    pub fn for_target(clock: Arc<dyn Clock>, fps: u32, with_cursor: bool, target_id: Option<&str>) -> anyhow::Result<Self> {
        use windows_capture::{
            capture::{Context, GraphicsCaptureApiHandler},
            frame::Frame as WgcFrame,
            graphics_capture_api::InternalCaptureControl,
            monitor::Monitor,
            window::Window,
            settings::{
                ColorFormat, CursorCaptureSettings, DirtyRegionSettings,
                DrawBorderSettings, MinimumUpdateIntervalSettings,
                SecondaryWindowSettings, Settings,
            },
        };

        struct Handler {
            tx: SyncSender<Frame>,
            clock: Arc<dyn Clock>,
            drops: Arc<AtomicU64>,
        }

        impl GraphicsCaptureApiHandler for Handler {
            type Flags = (SyncSender<Frame>, Arc<dyn Clock>, Arc<AtomicU64>);
            type Error = anyhow::Error;

            fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
                Ok(Self { tx: ctx.flags.0, clock: ctx.flags.1, drops: ctx.flags.2 })
            }

            fn on_frame_arrived(
                &mut self,
                frame: &mut WgcFrame,
                _ctl: InternalCaptureControl,
            ) -> Result<(), Self::Error> {
                let w = frame.width();
                let h = frame.height();
                let mut buf = frame.buffer()?;
                let bgra = if buf.has_padding() {
                    let row_pitch = buf.row_pitch() as usize;
                    let row_bytes = w as usize * 4;
                    let raw = buf.as_raw_buffer();
                    let mut tight = Vec::with_capacity(row_bytes * h as usize);
                    for row in 0..h as usize {
                        let start = row * row_pitch;
                        tight.extend_from_slice(&raw[start..start + row_bytes]);
                    }
                    tight
                } else {
                    buf.as_raw_buffer().to_vec()
                };
                let ts = crate::domain::time::Timestamp(self.clock.now_ms());
                try_send_or_drop(&self.tx, Frame { width: w, height: h, bgra, ts }, &self.drops);
                Ok(())
            }

            fn on_closed(&mut self) -> Result<(), Self::Error> { Ok(()) }
        }

        let (tx, rx) = sync_channel(8);
        let drops = Arc::new(AtomicU64::new(0));
        let cursor_setting = if with_cursor { CursorCaptureSettings::WithCursor } else { CursorCaptureSettings::WithoutCursor };
        let interval_setting = MinimumUpdateIntervalSettings::Custom(std::time::Duration::from_micros(
            1_000_000 / fps.max(1) as u64,
        ));

        if let Some(tid) = target_id {
            if let Some(hex) = tid.strip_prefix("window:0x") {
                if let Ok(hwnd_val) = usize::from_str_radix(hex, 16) {
                    let hwnd = windows::Win32::Foundation::HWND(hwnd_val as *mut _);
                    let win = Window::from_raw_hwnd(hwnd.0 as *mut _);
                    let mut r = windows::Win32::Foundation::RECT::default();
                    let (w, h) = if unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut r) }.is_ok() {
                        ((r.right - r.left).max(100) as u32, (r.bottom - r.top).max(100) as u32)
                    } else {
                        (1920, 1080)
                    };
                    let settings = Settings::new(
                        win,
                        cursor_setting,
                        DrawBorderSettings::WithoutBorder,
                        SecondaryWindowSettings::Default,
                        interval_setting,
                        DirtyRegionSettings::Default,
                        ColorFormat::Bgra8,
                        (tx, clock, drops.clone()),
                    );
                    let control = Handler::start_free_threaded(settings)?;
                    let halt = control.halt_handle();
                    let stopper: Box<dyn FnOnce() + Send> = Box::new(move || { let _ = control.stop(); });
                    return Ok(Self { rx, dims: (w, h), halt, stopper: Some(stopper), drops });
                }
            } else if let Some(idx_str) = tid.strip_prefix("display:") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if let Ok(mon) = Monitor::from_index(idx) {
                        let w = mon.width().unwrap_or(1920);
                        let h = mon.height().unwrap_or(1080);
                        let settings = Settings::new(
                            mon,
                            cursor_setting,
                            DrawBorderSettings::WithoutBorder,
                            SecondaryWindowSettings::Default,
                            interval_setting,
                            DirtyRegionSettings::Default,
                            ColorFormat::Bgra8,
                            (tx, clock, drops.clone()),
                        );
                        let control = Handler::start_free_threaded(settings)?;
                        let halt = control.halt_handle();
                        let stopper: Box<dyn FnOnce() + Send> = Box::new(move || { let _ = control.stop(); });
                        return Ok(Self { rx, dims: (w, h), halt, stopper: Some(stopper), drops });
                    }
                }
            }
        }

        let monitor = Monitor::primary()?;
        let (w, h) = (monitor.width()?, monitor.height()?);
        let settings = Settings::new(
            monitor,
            cursor_setting,
            DrawBorderSettings::WithoutBorder,
            SecondaryWindowSettings::Default,
            interval_setting,
            DirtyRegionSettings::Default,
            ColorFormat::Bgra8,
            (tx, clock, drops.clone()),
        );
        let control = Handler::start_free_threaded(settings)?;
        let halt = control.halt_handle();
        let stopper: Box<dyn FnOnce() + Send> = Box::new(move || { let _ = control.stop(); });
        Ok(Self { rx, dims: (w, h), halt, stopper: Some(stopper), drops })
    }

    /// Returns the WGC halt handle. Store before moving source into the video thread;
    /// calling stop_wgc() posts WM_QUIT which unblocks the capture thread and drops tx.
    pub fn halt_handle(&self) -> Arc<AtomicBool> {
        self.halt.clone()
    }

    /// Extracts the stop callable. Call from the main thread in stop_recording AFTER
    /// setting halt_handle to true; this posts WM_QUIT so the WGC thread exits promptly.
    pub fn take_stopper(&mut self) -> Option<Box<dyn FnOnce() + Send>> {
        self.stopper.take()
    }
}

impl FrameSource for WgcFrameSource {
    fn dimensions(&self) -> (u32, u32) { self.dims }

    fn next_frame(&mut self) -> Option<Frame> {
        self.rx.recv().ok()
    }

    fn drain_latest(&mut self) -> Option<Frame> {
        let mut last = None;
        while let Ok(f) = self.rx.try_recv() { last = Some(f); }
        last
    }
}

impl Drop for WgcFrameSource {
    fn drop(&mut self) {
        let n = self.drops.load(Ordering::Relaxed);
        if n > 0 { eprintln!("capture: dropped {n} frames (encoder behind)"); }
    }
}

#[cfg(test)]
#[path = "windows_capture_tests.rs"]
mod tests;
