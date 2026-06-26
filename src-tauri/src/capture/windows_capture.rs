use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, atomic::AtomicBool};
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
}

impl WgcFrameSource {
    pub fn for_primary_display(clock: Arc<dyn Clock>, fps: u32) -> anyhow::Result<Self> {
        use windows_capture::{
            capture::{Context, GraphicsCaptureApiHandler},
            frame::Frame as WgcFrame,
            graphics_capture_api::InternalCaptureControl,
            monitor::Monitor,
            settings::{
                ColorFormat, CursorCaptureSettings, DirtyRegionSettings,
                DrawBorderSettings, MinimumUpdateIntervalSettings,
                SecondaryWindowSettings, Settings,
            },
        };

        struct Handler {
            tx: Sender<Frame>,
            clock: Arc<dyn Clock>,
        }

        impl GraphicsCaptureApiHandler for Handler {
            type Flags = (Sender<Frame>, Arc<dyn Clock>);
            type Error = anyhow::Error;

            fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
                Ok(Self { tx: ctx.flags.0, clock: ctx.flags.1 })
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
                let _ = self.tx.send(Frame { width: w, height: h, bgra, ts });
                Ok(())
            }

            fn on_closed(&mut self) -> Result<(), Self::Error> { Ok(()) }
        }

        let monitor = Monitor::primary()?;
        let (w, h) = (monitor.width()?, monitor.height()?);
        let (tx, rx) = channel();
        let settings = Settings::new(
            monitor,
            CursorCaptureSettings::WithCursor,
            DrawBorderSettings::WithoutBorder,
            SecondaryWindowSettings::Default,
            // Match the encoder framerate (caller passes the display refresh, capped).
            // Default fires at the monitor refresh and would mismatch the labeled fps,
            // which made recordings play at the wrong speed.
            MinimumUpdateIntervalSettings::Custom(std::time::Duration::from_micros(
                1_000_000 / fps.max(1) as u64,
            )),
            DirtyRegionSettings::Default,
            ColorFormat::Bgra8,
            (tx, clock),
        );
        let control = Handler::start_free_threaded(settings)?;
        let halt = control.halt_handle();
        let stopper: Box<dyn FnOnce() + Send> = Box::new(move || { let _ = control.stop(); });
        Ok(Self { rx, dims: (w, h), halt, stopper: Some(stopper) })
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
}
