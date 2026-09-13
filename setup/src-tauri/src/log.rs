//! The small text log behind the error screen's "Copy log" button. Lines are stamped with
//! seconds since the window opened and mirrored to `%TEMP%\TCursorSetup.log`, so a failed
//! install leaves something behind even after the window is gone.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn buffer() -> &'static Mutex<Vec<String>> {
    static BUFFER: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    BUFFER.get_or_init(|| {
        let epoch = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        Mutex::new(vec![format!(
            "TCursor Setup {}, started at unix time {epoch}",
            env!("CARGO_PKG_VERSION")
        )])
    })
}

fn started() -> Instant {
    static STARTED: OnceLock<Instant> = OnceLock::new();
    *STARTED.get_or_init(Instant::now)
}

/// `%TEMP%\TCursorSetup.log`, the path the error screen names.
pub fn path() -> PathBuf {
    std::env::temp_dir().join("TCursorSetup.log")
}

pub fn line(message: impl AsRef<str>) {
    let at = started().elapsed().as_secs_f64();
    if let Ok(mut buffer) = buffer().lock() {
        buffer.push(format!("[{at:8.3}s] {}", message.as_ref()));
    }
}

pub fn text() -> String {
    buffer().lock().map(|b| b.join("\r\n")).unwrap_or_default()
}

/// Write what we have so far to `path()`. Called at every end state, so the file is complete
/// whether the install succeeded, failed, or the user closed mid-run.
pub fn flush() {
    let _ = std::fs::write(path(), text());
}
