pub struct WavWriter { inner: hound::WavWriter<std::io::BufWriter<std::fs::File>> }

impl WavWriter {
    pub fn create(path: &str, sample_rate: u32, channels: u16) -> std::io::Result<Self> {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let inner = hound::WavWriter::create(path, spec)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(Self { inner })
    }
    pub fn write(&mut self, samples: &[i16]) {
        for &s in samples { let _ = self.inner.write_sample(s); }
    }
    pub fn finalize(self) -> std::io::Result<()> {
        self.inner.finalize().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn writes_readable_wav() {
        let path = std::env::temp_dir().join("m1_wav_test.wav");
        let _ = std::fs::remove_file(&path);
        let mut w = WavWriter::create(path.to_str().unwrap(), 48000, 1).unwrap();
        w.write(&[0, 100, -100, 32767, -32768]);
        w.finalize().unwrap();

        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().sample_rate, 48000);
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.into_samples::<i16>().count(), 5);
    }
}
