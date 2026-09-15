use crate::audio::level::{block_rms_f32, block_rms_i16, LevelSlot};
use crate::audio::wav_writer::WavWriter;
use crate::domain::time::Clock;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

fn capture_ms(now_ms: u64, info: &cpal::InputCallbackInfo) -> u64 {
    let ts = info.timestamp();
    let lat = ts.callback.duration_since(&ts.capture).unwrap_or_default();
    now_ms.saturating_sub(lat.as_millis() as u64)
}

pub struct CpalMicHandle {
    stream: cpal::Stream,
    writer: Arc<Mutex<Option<WavWriter>>>,
}

impl CpalMicHandle {
    pub fn stop(self) -> std::io::Result<()> {
        drop(self.stream);
        if let Some(w) = self.writer.lock().unwrap_or_else(|e| e.into_inner()).take() {
            w.finalize()?;
        }
        Ok(())
    }
}

pub struct CpalMic;

impl CpalMic {
    pub fn open(
        device_name: Option<&str>,
        wav_path: &str,
        paused: Arc<AtomicBool>,
        started: Arc<AtomicU64>,
        clock: Arc<dyn Clock>,
        level: Option<Arc<LevelSlot>>,
    ) -> anyhow::Result<CpalMicHandle> {
        let host = cpal::default_host();
        let device = match device_name {
            Some(name) => host
                .input_devices()?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .ok_or_else(|| anyhow::anyhow!("input device not found: {name}"))?,
            None => host
                .default_input_device()
                .ok_or_else(|| anyhow::anyhow!("no default input device"))?,
        };
        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        let writer = Arc::new(Mutex::new(Some(WavWriter::create(
            wav_path,
            sample_rate,
            channels,
        )?)));
        let w2 = writer.clone();
        let lv = level.clone();
        let err_fn = |e| eprintln!("mic stream error: {e}");

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], info: &cpal::InputCallbackInfo| {
                    if paused.load(Ordering::SeqCst) {
                        return;
                    }
                    if started.load(Ordering::SeqCst) == 0 {
                        started.store(capture_ms(clock.now_ms(), info), Ordering::SeqCst);
                    }
                    if let Some(l) = lv.as_ref() {
                        l.push(block_rms_f32(data));
                    }
                    let s: Vec<i16> = data
                        .iter()
                        .map(|&x| (x.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                        .collect();
                    if let Some(w) = w2.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                        w.write(&s);
                    }
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], info: &cpal::InputCallbackInfo| {
                    if paused.load(Ordering::SeqCst) {
                        return;
                    }
                    if started.load(Ordering::SeqCst) == 0 {
                        started.store(capture_ms(clock.now_ms(), info), Ordering::SeqCst);
                    }
                    if let Some(l) = level.as_ref() {
                        l.push(block_rms_i16(data));
                    }
                    if let Some(w) = w2.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                        w.write(data);
                    }
                },
                err_fn,
                None,
            )?,
            other => anyhow::bail!("unsupported sample format: {other:?}"),
        };
        stream.play()?;
        Ok(CpalMicHandle { stream, writer })
    }

    pub fn default_input(wav_path: &str) -> anyhow::Result<CpalMicHandle> {
        Self::open(
            None,
            wav_path,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicU64::new(0)),
            Arc::new(crate::domain::time::SystemClock::new()),
            None,
        )
    }
}
