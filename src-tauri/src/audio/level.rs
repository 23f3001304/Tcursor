use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Copy, serde::Serialize)]
pub struct AudioLevel {
    pub source: &'static str,
    pub rms: f32,
}

pub fn block_rms_f32(data: &[f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let sum: f64 = data
        .iter()
        .map(|&x| {
            let v = x.clamp(-1.0, 1.0) as f64;
            v * v
        })
        .sum();
    (sum / data.len() as f64).sqrt() as f32
}

pub fn block_rms_i16(data: &[i16]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let scale = i16::MAX as f64;
    let sum: f64 = data
        .iter()
        .map(|&x| {
            let v = (x as f64 / scale).clamp(-1.0, 1.0);
            v * v
        })
        .sum();
    (sum / data.len() as f64).sqrt() as f32
}

#[derive(Default)]
pub struct LevelSlot {
    peak: AtomicU32,
}

impl LevelSlot {
    pub fn new() -> Self {
        Self {
            peak: AtomicU32::new(0),
        }
    }

    pub fn push(&self, rms: f32) {
        if !(rms > 0.0) {
            return;
        }
        self.peak.fetch_max(rms.to_bits(), Ordering::Relaxed);
    }

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
        let square: Vec<f32> = (0..64)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        assert!((block_rms_f32(&square) - 1.0).abs() < 1e-6);
        let square_i: Vec<i16> = (0..64)
            .map(|i| if i % 2 == 0 { i16::MAX } else { -i16::MAX })
            .collect();
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
        let f: Vec<f32> = (0..n)
            .map(|i| 0.5 * (std::f32::consts::TAU * i as f32 / 128.0).sin())
            .collect();
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
        slot.push(0.4);
        assert!((slot.take() - 0.9).abs() < 1e-6);
        assert_eq!(
            slot.take(),
            0.0,
            "take resets, so a silent window reads silent"
        );
    }

    #[test]
    fn the_slot_ignores_non_positive_and_nan_pushes() {
        let slot = LevelSlot::new();
        slot.push(f32::NAN);
        slot.push(-3.0);
        assert_eq!(slot.take(), 0.0);
    }
}
