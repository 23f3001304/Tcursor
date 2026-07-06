use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::audio::wav_writer::WavWriter;

pub struct SystemAudioHandle {
    stream: cpal::Stream,
    writer: Arc<Mutex<Option<WavWriter>>>,
}

impl SystemAudioHandle {
    pub fn stop(self) -> std::io::Result<()> {
        drop(self.stream);
        if let Some(w) = self.writer.lock().unwrap().take() { w.finalize()?; }
        Ok(())
    }
}

pub struct SystemAudio;

impl SystemAudio {
    pub fn loopback(wav_path: &str, paused: Arc<AtomicBool>) -> anyhow::Result<SystemAudioHandle> {
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
                    let s: Vec<i16> = data.iter().map(|&x| (x.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).collect();
                    if let Some(w) = w2.lock().unwrap().as_mut() { w.write(&s); }
                }, err_fn, None)?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    if paused.load(Ordering::SeqCst) { return; }
                    if let Some(w) = w2.lock().unwrap().as_mut() { w.write(data); }
                }, err_fn, None)?,
            other => anyhow::bail!("unsupported sample format: {other:?}"),
        };
        stream.play()?;
        Ok(SystemAudioHandle { stream, writer })
    }
}
