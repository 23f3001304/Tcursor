#[cfg(test)]
pub mod mock;
#[cfg(windows)]
pub mod windows;

use crate::ports::audio::SystemAudioPort;
use crate::ports::capture::CapturePort;
use crate::ports::input::InputPort;
use crate::ports::system::SystemPort;

pub struct Platform {
    pub capture: Box<dyn CapturePort>,
    pub input: Box<dyn InputPort>,
    pub system: Box<dyn SystemPort>,
    pub audio: Box<dyn SystemAudioPort>,
}

pub fn current() -> Platform {
    #[cfg(windows)]
    {
        Platform {
            capture: Box::new(windows::Win32Capture),
            input: Box::new(windows::Win32Input),
            system: Box::new(windows::Win32System),
            audio: Box::new(windows::Win32SystemAudio),
        }
    }
    #[cfg(not(windows))]
    {
        unimplemented!("no platform adapter for this target yet")
    }
}
