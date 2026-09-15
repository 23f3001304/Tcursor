#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WindowHandle(pub isize);

pub trait SystemPort: Send + Sync {
    fn primary_refresh_hz(&self) -> u32;
    fn os_prefers_dark(&self) -> bool;
    fn exclude_from_capture(&self, window: WindowHandle, exclude: bool) -> bool;
}
