use crate::ports::system::WindowHandle;

pub fn handle(window: &tauri::WebviewWindow) -> Option<WindowHandle> {
    #[cfg(windows)]
    {
        window.hwnd().ok().map(|h| WindowHandle(h.0 as isize))
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        None
    }
}
