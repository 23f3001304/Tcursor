pub trait AudioSource: Send {
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16;
}

pub struct FakeAudioSource { pub sample_rate: u32, pub channels: u16 }
impl AudioSource for FakeAudioSource {
    fn sample_rate(&self) -> u32 { self.sample_rate }
    fn channels(&self) -> u16 { self.channels }
}
