use crate::capture::frame::Frame;
use crate::capture::frame_source::FrameSource;
use crate::domain::time::Clock;
use crate::ports::capture::TargetId;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::frame::Frame as WgcFrame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings, TryIntoCaptureItemWithType,
};
use windows_capture::window::Window;

use super::super::target::capture_window_size;

type Flags = (SyncSender<Frame>, Arc<dyn Clock>, Arc<AtomicU64>);

pub struct WgcFrameSource {
    rx: Receiver<Frame>,
    dims: (u32, u32),
    halt: Arc<AtomicBool>,
    stopper: Option<Box<dyn FnOnce() + Send>>,
    drops: Arc<AtomicU64>,
}

fn try_send_or_drop(tx: &SyncSender<Frame>, frame: Frame, drops: &AtomicU64) {
    if tx.try_send(frame).is_err() {
        drops.fetch_add(1, Ordering::Relaxed);
    }
}

struct Handler {
    tx: SyncSender<Frame>,
    clock: Arc<dyn Clock>,
    drops: Arc<AtomicU64>,
}

impl GraphicsCaptureApiHandler for Handler {
    type Flags = Flags;
    type Error = anyhow::Error;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            tx: ctx.flags.0,
            clock: ctx.flags.1,
            drops: ctx.flags.2,
        })
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
        try_send_or_drop(
            &self.tx,
            Frame {
                width: w,
                height: h,
                bgra,
                ts,
            },
            &self.drops,
        );
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn launch<T: TryIntoCaptureItemWithType + Send + 'static>(
    item: T,
    cursor: CursorCaptureSettings,
    interval: MinimumUpdateIntervalSettings,
    flags: Flags,
) -> anyhow::Result<(Arc<AtomicBool>, Box<dyn FnOnce() + Send>)> {
    let control = Handler::start_free_threaded(Settings::new(
        item,
        cursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        interval,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        flags,
    ))?;
    let halt = control.halt_handle();
    let stopper: Box<dyn FnOnce() + Send> = Box::new(move || {
        let _ = control.stop();
    });
    Ok((halt, stopper))
}

impl WgcFrameSource {
    pub fn for_primary_display(
        clock: Arc<dyn Clock>,
        fps: u32,
        with_cursor: bool,
    ) -> anyhow::Result<Self> {
        Self::for_target(clock, fps, with_cursor, None)
    }

    pub fn for_target(
        clock: Arc<dyn Clock>,
        fps: u32,
        with_cursor: bool,
        target_id: Option<&str>,
    ) -> anyhow::Result<Self> {
        let (tx, rx) = sync_channel(8);
        let drops = Arc::new(AtomicU64::new(0));
        let flags: Flags = (tx, clock, drops.clone());
        let cursor = if with_cursor {
            CursorCaptureSettings::WithCursor
        } else {
            CursorCaptureSettings::WithoutCursor
        };
        let interval = MinimumUpdateIntervalSettings::Custom(std::time::Duration::from_micros(
            1_000_000 / fps.max(1) as u64,
        ));

        let (dims, halt, stopper) = match TargetId::from_arg(target_id) {
            TargetId::Window(handle) => {
                let hwnd = windows::Win32::Foundation::HWND(handle as usize as *mut _);
                let dims = capture_window_size(hwnd);
                let win = Window::from_raw_hwnd(hwnd.0 as *mut _);
                let (halt, stopper) = launch(win, cursor, interval, flags)?;
                (dims, halt, stopper)
            }
            TargetId::Display(index) => match Monitor::from_index(index) {
                Ok(monitor) => {
                    let dims = (
                        monitor.width().unwrap_or(1920),
                        monitor.height().unwrap_or(1080),
                    );
                    let (halt, stopper) = launch(monitor, cursor, interval, flags)?;
                    (dims, halt, stopper)
                }
                Err(_) => primary(cursor, interval, flags)?,
            },
            TargetId::Primary => primary(cursor, interval, flags)?,
        };

        Ok(Self {
            rx,
            dims,
            halt,
            stopper: Some(stopper),
            drops,
        })
    }

    pub fn halt_handle(&self) -> Arc<AtomicBool> {
        self.halt.clone()
    }

    pub fn take_stopper(&mut self) -> Option<Box<dyn FnOnce() + Send>> {
        self.stopper.take()
    }
}

fn primary(
    cursor: CursorCaptureSettings,
    interval: MinimumUpdateIntervalSettings,
    flags: Flags,
) -> anyhow::Result<((u32, u32), Arc<AtomicBool>, Box<dyn FnOnce() + Send>)> {
    let monitor = Monitor::primary()?;
    let dims = (monitor.width()?, monitor.height()?);
    let (halt, stopper) = launch(monitor, cursor, interval, flags)?;
    Ok((dims, halt, stopper))
}

impl FrameSource for WgcFrameSource {
    fn dimensions(&self) -> (u32, u32) {
        self.dims
    }

    fn next_frame(&mut self) -> Option<Frame> {
        self.rx.recv().ok()
    }

    fn drain_latest(&mut self) -> Option<Frame> {
        let mut last = None;
        while let Ok(f) = self.rx.try_recv() {
            last = Some(f);
        }
        last
    }
}

impl Drop for WgcFrameSource {
    fn drop(&mut self) {
        let n = self.drops.load(Ordering::Relaxed);
        if n > 0 {
            eprintln!("capture: dropped {n} frames (encoder behind)");
        }
    }
}

#[cfg(test)]
#[path = "wgc_source_tests.rs"]
mod tests;
