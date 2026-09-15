use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn generate_once<F: FnOnce() -> Result<(), String>>(out: &Path, gen: F) -> Result<(), String> {
    static GEN: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = GEN
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if out.exists() {
        return Ok(());
    }
    gen()
}

pub fn tmp_sibling(out: &Path) -> PathBuf {
    let k = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let name = out
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    out.with_file_name(format!(".part-{}-{k}-{name}", std::process::id()))
}

pub fn corrupt_sibling(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!("{name}.corrupt"))
}

pub fn preserve_corrupt(path: &Path, parse_err: &dyn std::fmt::Display) {
    let corrupt = corrupt_sibling(path);
    let _ = std::fs::remove_file(&corrupt);
    if let Err(re) = std::fs::rename(path, &corrupt) {
        eprintln!(
            "{path:?} parse failed ({parse_err}) and could not be preserved at {corrupt:?}: {re}"
        );
    } else {
        eprintln!("{path:?} parse failed ({parse_err}); original preserved at {corrupt:?}");
    }
}

static FFMPEG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_ffmpeg_dir(dir: PathBuf) {
    let _ = FFMPEG_DIR.set(dir);
}

pub fn choose_dir<'a>(candidates: &'a [PathBuf], name: &str) -> Option<&'a PathBuf> {
    candidates.iter().find(|d| d.join(name).exists())
}

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

pub fn init_ffmpeg(resource_dir: Option<PathBuf>) -> String {
    let name = format!("ffmpeg{}", std::env::consts::EXE_SUFFIX);
    let candidates = ffmpeg_candidates(resource_dir.as_deref());
    let chosen = choose_dir(&candidates, &name).cloned();
    if let Some(dir) = chosen.clone() {
        set_ffmpeg_dir(dir);
    }
    format!(
        "exe={:?}\nresource_dir={:?}\ncandidates={:?}\nchosen={:?}\nresolved={:?}\n",
        std::env::current_exe(),
        resource_dir,
        candidates,
        chosen,
        resolve("ffmpeg")
    )
}

fn resolve(program: &str) -> PathBuf {
    if let Some(dir) = FFMPEG_DIR.get() {
        let exe = dir.join(format!("{program}{}", std::env::consts::EXE_SUFFIX));
        if exe.exists() {
            return exe;
        }
    }
    PathBuf::from(program)
}

pub fn ffcmd(program: &str) -> Command {
    ffcmd_prio(program, false)
}

pub fn ffcmd_bg(program: &str) -> Command {
    ffcmd_prio(program, true)
}

fn ffcmd_prio(program: &str, background: bool) -> Command {
    let mut c = Command::new(resolve(program));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows::Win32::System::Threading::{BELOW_NORMAL_PRIORITY_CLASS, CREATE_NO_WINDOW};
        let priority = if background {
            BELOW_NORMAL_PRIORITY_CLASS.0
        } else {
            0
        };
        c.creation_flags(CREATE_NO_WINDOW.0 | priority);
    }
    #[cfg(not(windows))]
    let _ = background;
    c
}

#[cfg(test)]
#[path = "proc_tests.rs"]
mod tests;
