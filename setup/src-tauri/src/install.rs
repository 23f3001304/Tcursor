//! Running the embedded NSIS installer and reporting on it. Every decision the window renders is
//! made here or in `progress`; the UI only paints what the commands and events hand it.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use crate::log;
use crate::progress::{self, Step, EXPECTED_BYTES, MAIN_BINARY};

/// The NSIS installer, embedded at build time (staged into OUT_DIR by build.rs).
const NSIS_PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/nsis-setup.exe"));
/// `src-tauri/installer/eula.rtf` flattened to text at build time.
const LICENSE: &str = include_str!(concat!(env!("OUT_DIR"), "/eula.txt"));
const POLL_MS: u64 = 150;

static RUNNING: AtomicBool = AtomicBool::new(false);
static CLOSE_ALLOWED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, serde::Serialize)]
struct Done {
    ok: bool,
    message: String,
}

#[derive(serde::Serialize)]
pub struct Info {
    version: String,
    destination: String,
    log_path: String,
    /// 0 when this build could not measure the payload; the UI then drops the size line.
    expected_mb: u64,
}

/// The fine print every screen shows.
#[tauri::command]
pub fn setup_info() -> Info {
    Info {
        version: env!("CARGO_PKG_VERSION").into(),
        destination: progress::destination().display().to_string(),
        log_path: log::path().display().to_string(),
        expected_mb: EXPECTED_BYTES / (1024 * 1024),
    }
}

#[tauri::command]
pub fn license_text() -> &'static str {
    LICENSE
}

/// The log text, also mirrored to disk so the named path is up to date when the user goes looking.
#[tauri::command]
pub fn copy_log() -> String {
    log::flush();
    log::text()
}

/// Close past the "Installing. Close anyway?" guard in `main`.
#[tauri::command]
pub fn allow_close(window: tauri::Window) {
    CLOSE_ALLOWED.store(true, Ordering::SeqCst);
    log::line("closed by the user while the installer was running");
    log::flush();
    let _ = window.close();
}

pub fn busy() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

pub fn close_allowed() -> bool {
    CLOSE_ALLOWED.load(Ordering::SeqCst)
}

/// Stage the embedded NSIS installer to a temp file, run it silently, and drive progress +
/// completion events (`install://tick`, `install://done`) from a worker thread. Returns once the
/// install has been kicked off - the UI reacts to the events. Retry calls this again and reuses
/// the payload already staged.
#[tauri::command]
pub fn start_install(app: AppHandle) -> Result<(), String> {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    match stage() {
        Ok(payload) => {
            std::thread::spawn(move || run(app, payload));
            Ok(())
        }
        Err(e) => {
            RUNNING.store(false, Ordering::SeqCst);
            log::line(&e);
            log::flush();
            Err(e)
        }
    }
}

fn stage() -> Result<PathBuf, String> {
    if NSIS_PAYLOAD.len() < 1024 {
        return Err("Installer payload is missing. This build was not produced by setup/build.ps1.".into());
    }
    let payload = std::env::temp_dir().join(format!("TCursor_setup_{}.exe", std::process::id()));
    if !payload.exists() {
        let mut f = std::fs::File::create(&payload).map_err(|e| format!("Could not stage the installer. {e}"))?;
        f.write_all(NSIS_PAYLOAD).map_err(|e| format!("Could not write the installer. {e}"))?;
    } // file closed here before we execute it
    Ok(payload)
}

fn run(app: AppHandle, payload: PathBuf) {
    let dest = progress::destination();
    log::line(format!("payload {} ({} bytes)", payload.display(), NSIS_PAYLOAD.len()));
    log::line(format!("destination {} (expecting {EXPECTED_BYTES} bytes)", dest.display()));
    emit(&app, progress::step(0, EXPECTED_BYTES, true, 0));

    let started = Instant::now();
    let mut child = match Command::new(&payload).arg("/S").stdout(Stdio::null()).stderr(Stdio::piped()).spawn() {
        Ok(child) => child,
        Err(e) => return finish(&app, &payload, Err(format!("Could not start the installer. {e}"))),
    };

    let mut peak = 0;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let bytes = progress::dir_bytes(&dest);
                let stderr = tail(child.stderr.take());
                log::line(format!("exit code {:?}, destination holds {bytes} bytes", status.code()));
                if !stderr.is_empty() {
                    log::line(format!("stderr tail: {stderr}"));
                }
                let outcome = match status.success() {
                    true => Ok(()),
                    false => Err(format!("The installer exited with code {}.", status.code().unwrap_or(-1))),
                };
                return finish(&app, &payload, outcome);
            }
            Ok(None) => {
                let elapsed = started.elapsed().as_millis() as u64;
                let step = progress::step(progress::dir_bytes(&dest), EXPECTED_BYTES, true, elapsed);
                // An upgrade deletes the old files before writing the new ones, so the measured
                // total can dip. The bar never walks backwards.
                peak = peak.max(step.percent);
                emit(&app, Step { percent: peak, ..step });
                std::thread::sleep(Duration::from_millis(POLL_MS));
            }
            Err(e) => return finish(&app, &payload, Err(format!("Lost track of the installer. {e}"))),
        }
    }
}

/// Last 400 characters of whatever the child wrote to stderr. NSIS `/S` is normally silent.
fn tail(stderr: Option<std::process::ChildStderr>) -> String {
    let mut text = String::new();
    if let Some(mut stderr) = stderr {
        let _ = stderr.read_to_string(&mut text);
    }
    let text = text.trim();
    match text.char_indices().nth_back(399) {
        Some((i, _)) => text[i..].to_string(),
        None => text.to_string(),
    }
}

fn finish(app: &AppHandle, payload: &PathBuf, outcome: Result<(), String>) {
    RUNNING.store(false, Ordering::SeqCst);
    match &outcome {
        // The staged payload survives a failure so Retry can run it again without rewriting 58MB.
        Err(e) => log::line(format!("failed: {e}")),
        Ok(()) => {
            let _ = std::fs::remove_file(payload);
            emit(app, progress::step(0, EXPECTED_BYTES, false, 0));
            log::line("installed");
        }
    }
    log::flush();
    let ok = outcome.is_ok();
    let _ = app.emit("install://done", Done { ok, message: outcome.err().unwrap_or_default() });
}

fn emit(app: &AppHandle, step: Step) {
    let _ = app.emit("install://tick", step);
}

/// Open the app NSIS just installed. The Done screen does this on a timer and on the button.
#[tauri::command]
pub fn launch_app() -> Result<(), String> {
    let exe = progress::destination().join(MAIN_BINARY);
    log::line(format!("launching {}", exe.display()));
    Command::new(&exe).spawn().map_err(|e| {
        let message = format!("TCursor is installed, but it would not open. {e}");
        log::line(&message);
        log::flush();
        message
    })?;
    Ok(())
}
