use cpal::traits::{DeviceTrait, HostTrait};

use crate::ports::audio::SystemAudioPort;

pub struct Win32SystemAudio;

impl SystemAudioPort for Win32SystemAudio {
    fn loopback_device(&self) -> Option<(cpal::Device, cpal::SupportedStreamConfig)> {
        let device = cpal::default_host().default_output_device()?;
        let config = device.default_output_config().ok()?;
        Some((device, config))
    }
}
