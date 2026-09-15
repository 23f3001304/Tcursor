pub trait SystemAudioPort: Send + Sync {
    fn loopback_device(&self) -> Option<(cpal::Device, cpal::SupportedStreamConfig)>;
}
