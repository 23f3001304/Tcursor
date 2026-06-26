use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::audio::wav_writer::WavWriter;

pub struct CpalMicHandle {
    stream: cpal::Stream,
    writer: Arc<Mutex<Option<WavWriter>>>,
}

impl CpalMicHandle {
    pub fn stop(self) -> std::io::Result<()> {
        drop(self.stream);
        if let Some(w) = self.writer.lock().unwrap().take() { w.finalize()?; }
        Ok(())
    }
}

pub struct CpalMic;

impl CpalMic {
    /// Open an input device by name (or the default if None). While `paused` is set,
    /// incoming samples are dropped so paused time is excluded from the WAV.
    pub fn open(
        device_name: Option<&str>,
        wav_path: &str,
        paused: Arc<AtomicBool>,
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

        let writer = Arc::new(Mutex::new(Some(
            WavWriter::create(wav_path, sample_rate, channels)?,
        )));
        let w2 = writer.clone();
        let err_fn = |e| eprintln!("mic stream error: {e}");

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    if paused.load(Ordering::SeqCst) { return; }
                    let s: Vec<i16> = data.iter()
                        .map(|&x| (x.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).collect();
                    if let Some(w) = w2.lock().unwrap().as_mut() { w.write(&s); }
                },
                err_fn, None)?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    if paused.load(Ordering::SeqCst) { return; }
                    if let Some(w) = w2.lock().unwrap().as_mut() { w.write(data); }
                },
                err_fn, None)?,
            other => anyhow::bail!("unsupported sample format: {other:?}"),
        };
        stream.play()?;
        Ok(CpalMicHandle { stream, writer })
    }

    pub fn default_input(wav_path: &str) -> anyhow::Result<CpalMicHandle> {
        Self::open(None, wav_path, Arc::new(AtomicBool::new(false)))
    }
}
