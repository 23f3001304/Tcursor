use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::audio::wav_writer::WavWriter;
use crate::domain::time::Clock;

pub struct SystemAudioHandle {
    stream: cpal::Stream,
    writer: Arc<Mutex<Option<WavWriter>>>,
}

impl SystemAudioHandle {
    pub fn stop(self) -> std::io::Result<()> {
        drop(self.stream);
        // Poison-tolerant for the same reason as `CpalMicHandle::stop`: a panic on this thread
        // is swallowed by the discarded join, leaving system.wav unfinalized and unreadable.
        if let Some(w) = self.writer.lock().unwrap_or_else(|e| e.into_inner()).take() { w.finalize()?; }
        Ok(())
    }
}

pub struct SystemAudio;

impl SystemAudio {
    pub fn loopback(wav_path: &str, paused: Arc<AtomicBool>, started: Arc<AtomicU64>, clock: Arc<dyn Clock>) -> anyhow::Result<SystemAudioHandle> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("no default output device"))?;
        let config = device.default_output_config()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let writer = Arc::new(Mutex::new(Some(WavWriter::create(wav_path, sample_rate, channels)?)));
        let w2 = writer.clone();
        let err_fn = |e| eprintln!("system-audio stream error: {e}");
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    if paused.load(Ordering::SeqCst) { return; }
                    if !data.is_empty() && started.load(Ordering::SeqCst) == 0 {
                        started.store(clock.now_ms(), Ordering::SeqCst);
                    }
                    let s: Vec<i16> = data.iter().map(|&x| (x.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).collect();
                    if let Some(w) = w2.lock().unwrap().as_mut() { w.write(&s); }
                }, err_fn, None)?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    if paused.load(Ordering::SeqCst) { return; }
                    if !data.is_empty() && started.load(Ordering::SeqCst) == 0 {
                        started.store(clock.now_ms(), Ordering::SeqCst);
                    }
                    if let Some(w) = w2.lock().unwrap().as_mut() { w.write(data); }
                }, err_fn, None)?,
            other => anyhow::bail!("unsupported sample format: {other:?}"),
        };
        stream.play()?;
        Ok(SystemAudioHandle { stream, writer })
    }
}
