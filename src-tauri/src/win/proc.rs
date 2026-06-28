use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Directory holding the bundled ffmpeg/ffprobe, set once at startup. Unset under
/// `cargo`/dev runs, where we fall back to PATH.
static FFMPEG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Point ffmpeg/ffprobe lookups at a bundled directory (call once at app startup).
pub fn set_ffmpeg_dir(dir: PathBuf) {
    let _ = FFMPEG_DIR.set(dir);
}

/// First candidate dir that actually contains `name` (pure; unit-tested).
pub fn choose_dir<'a>(cands: &'a [PathBuf], name: &str) -> Option<&'a PathBuf> {
    cands.iter().find(|d| d.join(name).exists())
}

/// Where the bundled binaries may live, most-specific first: next to the running
/// executable (the NSIS install puts them in `<install>/resources/`), then the
/// Tauri resource dir. `current_exe()` is reliable on installed builds where
/// `resource_dir()` may not resolve, so it leads.
pub fn ffmpeg_candidates(resource_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            v.push(d.join("resources"));
            v.push(d.to_path_buf());
        }
    }
    if let Some(res) = resource_dir {
        v.push(res.join("resources"));
        v.push(res.to_path_buf());
    }
    v
}

/// Locate the bundled ffmpeg and record `FFMPEG_DIR`. Returns a diagnostic string
/// (written to a log at startup) listing every path tried and the one chosen, so
/// an "ffmpeg not available" failure on any machine is explainable.
pub fn init_ffmpeg(resource_dir: Option<PathBuf>) -> String {
    let name = format!("ffmpeg{}", std::env::consts::EXE_SUFFIX);
    let cands = ffmpeg_candidates(resource_dir.as_deref());
    let chosen = choose_dir(&cands, &name).cloned();
    if let Some(dir) = chosen.clone() {
        set_ffmpeg_dir(dir);
    }
    format!(
        "exe={:?}\nresource_dir={:?}\ncandidates={:?}\nchosen={:?}\nresolved={:?}\n",
        std::env::current_exe(), resource_dir, cands, chosen, resolve("ffmpeg")
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose_dir_picks_first_with_the_binary() {
        let base = std::env::temp_dir().join(format!("tcursor-ff-{}", std::process::id()));
        let (a, b) = (base.join("a"), base.join("b"));
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(b.join("ffmpeg.exe"), b"x").unwrap();
        let cands = vec![a.clone(), b.clone()];
        assert_eq!(choose_dir(&cands, "ffmpeg.exe"), Some(&b)); // skips missing a, finds b
        assert_eq!(choose_dir(&cands, "nope.exe"), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn candidates_lead_with_exe_dir_then_resource_dir() {
        let cands = ffmpeg_candidates(Some(Path::new("C:/res")));
        // The resource-dir candidates are appended last; exe-derived ones lead.
        assert_eq!(cands[cands.len() - 2], Path::new("C:/res").join("resources"));
        assert_eq!(cands[cands.len() - 1], Path::new("C:/res").to_path_buf());
    }
}
