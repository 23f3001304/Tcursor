//! Pure decision point for a mid-record dimension change on the GPU-native path
//! (`gpu_frames::Cap`). The encoder is configured for one fixed `(w, h)` at recorder start
//! (`gpu_record::GpuRecorder::start`); after a window maximize/restore/snap, or a display
//! resolution/rotation change, or a dock/undock, WGC keeps delivering frames at the new size -
//! it does NOT end the capture on its own (finding H1). `Cap::on_frame_arrived` needs live WGC
//! and a real `VideoEncoder` to run at all, so this decision is split out here where it can be
//! tested without either - the same reason `gpu_frames::record_if_encoded` exists as its own
//! function.
pub struct DimGuard {
    cfg: (u32, u32),
    fired: bool,
}

impl DimGuard {
    pub fn new(cfg: (u32, u32)) -> Self {
        Self { cfg, fired: false }
    }

    /// True the FIRST time `frame` differs from the configured size; false on every call after
    /// that - a repeat of the same mismatch, a different mismatch, or even a frame back at the
    /// original size - since the caller has already begun ending the take. The crate's own halt
    /// flag (set by the `InternalCaptureControl::stop()` the caller issues on a `true`) already
    /// guarantees no later frame reaches `on_frame_arrived` at all; this latch is the defensive
    /// half of that guarantee, provable without WGC or the vendored crate's internals.
    pub fn mismatched(&mut self, frame: (u32, u32)) -> bool {
        if self.fired || frame == self.cfg {
            return false;
        }
        self.fired = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_dimensions_never_fire() {
        let mut g = DimGuard::new((1920, 1080));
        assert!(!g.mismatched((1920, 1080)));
        assert!(!g.mismatched((1920, 1080)));
    }

    #[test]
    fn the_first_mismatch_fires_exactly_once() {
        let mut g = DimGuard::new((1920, 1080));
        assert!(g.mismatched((2560, 1440)), "a maximize/resize must be caught the moment it arrives");
        assert!(!g.mismatched((2560, 1440)), "a second frame at the same new size must not re-fire");
        assert!(!g.mismatched((1920, 1080)), "even a frame back at the original size must not re-fire once ended");
    }
}
