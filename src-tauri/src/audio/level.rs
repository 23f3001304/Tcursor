use std::sync::atomic::{AtomicU32, Ordering};

/// One live audio-level reading, emitted to the frontend as `audio-level` roughly every 50ms per
/// open source. `rms` is 0..1 - the HUD's meter does its own dB mapping, so this stays the raw
/// measurement and never a display value.
#[derive(Clone, Copy, serde::Serialize)]
pub struct AudioLevel {
    /// `"mic"` or `"system"` - which capture produced it.
    pub source: &'static str,
    pub rms: f32,
}

/// RMS of one callback's worth of f32 samples, in 0..1. Interleaved channels are treated as one
/// block on purpose: the meter shows "how loud is this input", not a per-channel balance.
pub fn block_rms_f32(data: &[f32]) -> f32 {
    if data.is_empty() { return 0.0; }
    let sum: f64 = data.iter().map(|&x| { let v = x.clamp(-1.0, 1.0) as f64; v * v }).sum();
    (sum / data.len() as f64).sqrt() as f32
}

/// `block_rms_f32` for the i16 capture path, scaled by `i16::MAX` so both paths report the same
/// number for the same sound (a full-scale square wave is 1.0 either way).
pub fn block_rms_i16(data: &[i16]) -> f32 {
    if data.is_empty() { return 0.0; }
    let scale = i16::MAX as f64;
    let sum: f64 = data.iter().map(|&x| { let v = (x as f64 / scale).clamp(-1.0, 1.0); v * v }).sum();
    (sum / data.len() as f64).sqrt() as f32
}

/// A lock-free handoff from a realtime audio callback to the capture thread that owns it.
///
/// The callback may not block, allocate or emit, so it only `push`es its block's RMS here; the
/// thread's existing 50ms poll loop `take`s the loudest block seen since the last read and emits
/// that. Peak-of-block-RMS (not an average of blocks) is what a meter wants - a transient inside
/// the window still moves the needle instead of being averaged away.
///
/// The max is done with a plain `fetch_max` on the f32's bit pattern, which is exact for
/// non-negative finite floats: IEEE-754 orders them identically to their unsigned bit patterns.
#[derive(Default)]
pub struct LevelSlot {
    peak: AtomicU32,
}

impl LevelSlot {
    pub fn new() -> Self {
        Self { peak: AtomicU32::new(0) }
    }

    /// Record one block's RMS. Safe to call from an audio callback: no lock, no allocation.
    /// A negative or NaN value (impossible from the two helpers above, but cheap to rule out)
    /// is dropped rather than poisoning the bit-pattern comparison.
    pub fn push(&self, rms: f32) {
        if !(rms > 0.0) { return; }
        self.peak.fetch_max(rms.to_bits(), Ordering::Relaxed);
    }

    /// The loudest block since the last call, and reset to silence. Reading zero is meaningful:
    /// the source is open and genuinely silent, which is exactly what the meter's idle state is.
    pub fn take(&self) -> f32 {
        f32::from_bits(self.peak.swap(0, Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_and_empty_blocks_measure_zero() {
        assert_eq!(block_rms_f32(&[]), 0.0);
        assert_eq!(block_rms_i16(&[]), 0.0);
        assert_eq!(block_rms_f32(&[0.0; 64]), 0.0);
        assert_eq!(block_rms_i16(&[0; 64]), 0.0);
    }

    #[test]
    fn a_full_scale_square_wave_is_exactly_one() {
        let square: Vec<f32> = (0..64).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect();
        assert!((block_rms_f32(&square) - 1.0).abs() < 1e-6);
        let square_i: Vec<i16> = (0..64).map(|i| if i % 2 == 0 { i16::MAX } else { -i16::MAX }).collect();
        assert!((block_rms_i16(&square_i) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn a_full_scale_sine_is_one_over_root_two() {
        let n = 4096;
        let sine: Vec<f32> = (0..n)
            .map(|i| (std::f32::consts::TAU * i as f32 / n as f32).sin())
            .collect();
        assert!((block_rms_f32(&sine) - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3);
    }

    #[test]
    fn the_two_sample_formats_agree_on_the_same_sound() {
        let n = 1024;
        let f: Vec<f32> = (0..n).map(|i| 0.5 * (std::f32::consts::TAU * i as f32 / 128.0).sin()).collect();
        let i: Vec<i16> = f.iter().map(|&x| (x * i16::MAX as f32) as i16).collect();
        assert!((block_rms_f32(&f) - block_rms_i16(&i)).abs() < 1e-3);
    }

    #[test]
    fn out_of_range_samples_are_clamped_not_amplified() {
        assert!((block_rms_f32(&[4.0, -4.0, 4.0, -4.0]) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_slot_keeps_the_loudest_block_and_resets_on_take() {
        let slot = LevelSlot::new();
        assert_eq!(slot.take(), 0.0);
        slot.push(0.2);
        slot.push(0.9);
        slot.push(0.4); // a quieter block after the peak must not erase it
        assert!((slot.take() - 0.9).abs() < 1e-6);
        assert_eq!(slot.take(), 0.0, "take resets, so a silent window reads silent");
    }

    #[test]
    fn the_slot_ignores_non_positive_and_nan_pushes() {
        let slot = LevelSlot::new();
        slot.push(f32::NAN);
        slot.push(-3.0);
        assert_eq!(slot.take(), 0.0);
    }
}
