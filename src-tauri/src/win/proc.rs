use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Directory holding the bundled ffmpeg/ffprobe, set once at startup from the
/// Tauri resource dir. Unset under `cargo`/dev runs, where we fall back to PATH.
static FFMPEG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Point ffmpeg/ffprobe lookups at a bundled directory (call once at app startup).
pub fn set_ffmpeg_dir(dir: PathBuf) {
    let _ = FFMPEG_DIR.set(dir);
}

/// Resolve a tool name to the bundled exe if present, else the bare name (PATH).
fn resolve(program: &str) -> PathBuf {
    if let Some(dir) = FFMPEG_DIR.get() {
        let exe = dir.join(format!("{program}{}", std::env::consts::EXE_SUFFIX));
        if exe.exists() {
            return exe;
        }
    }
    PathBuf::from(program)
}

/// A `Command` for a console tool (ffmpeg/ffprobe) that does NOT flash a console
/// window on Windows. Prefers the bundled binary; falls back to PATH in dev.
pub fn ffcmd(program: &str) -> Command {
    let mut c = Command::new(resolve(program));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}
