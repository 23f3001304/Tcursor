use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

// The NSIS installer, embedded at build time (staged into OUT_DIR by build.rs).
const NSIS_PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/nsis-setup.exe"));

// Faked progress: NSIS `/S` reports nothing, so we pace a bar to ~92% while it runs and finish it
// when the process actually exits. The stage labels rotate purely for feel.
const STAGES: [&str; 4] = ["Copying files", "Creating shortcuts", "Registering TCursor", "Almost there"];

#[derive(Clone, serde::Serialize)]
struct Tick {
    pct: u32,
    stage: String,
}

#[derive(Clone, serde::Serialize)]
struct Done {
    ok: bool,
    message: String,
}

/// Stage the embedded NSIS installer to a temp file, run it silently, and drive progress +
/// completion events (`install://tick`, `install://done`) from a worker thread. Returns once the
/// install has been kicked off - the UI reacts to the events.
#[tauri::command]
pub fn start_install(app: AppHandle) -> Result<(), String> {
    if NSIS_PAYLOAD.len() < 1024 {
        return Err("Installer payload is missing - this build wasn't produced by setup/build.ps1.".into());
    }
    let tmp = std::env::temp_dir().join(format!("TCursor_setup_{}.exe", std::process::id()));
    {
        let mut f = std::fs::File::create(&tmp).map_err(|e| format!("Could not stage installer: {e}"))?;
        f.write_all(NSIS_PAYLOAD).map_err(|e| format!("Could not write installer: {e}"))?;
    } // file closed here before we execute it

    std::thread::spawn(move || run(app, tmp));
    Ok(())
}

fn run(app: AppHandle, tmp: PathBuf) {
    let _ = app.emit("install://tick", Tick { pct: 3, stage: STAGES[0].into() });
    let mut child = match Command::new(&tmp).arg("/S").spawn() {
        Ok(c) => c,
        Err(e) => return finish(&app, &tmp, false, format!("Could not start installer: {e}")),
    };

    let mut pct = 3u32;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let ok = status.success();
                let msg = if ok { String::new() } else { format!("Installer exited with code {}", status.code().unwrap_or(-1)) };
                return finish(&app, &tmp, ok, msg);
            }
            Ok(None) => {
                if pct < 92 {
                    pct += 2;
                }
                let stage = STAGES[(pct as usize / 25).min(3)];
                let _ = app.emit("install://tick", Tick { pct, stage: stage.into() });
                std::thread::sleep(Duration::from_millis(220));
            }
            Err(e) => return finish(&app, &tmp, false, format!("Install error: {e}")),
        }
    }
}

fn finish(app: &AppHandle, tmp: &PathBuf, ok: bool, message: String) {
    let _ = std::fs::remove_file(tmp);
    if ok {
        let _ = app.emit("install://tick", Tick { pct: 100, stage: "Done".into() });
    }
    let _ = app.emit("install://done", Done { ok, message });
}

/// Launch the freshly-installed app (NSIS installs it to `%LOCALAPPDATA%\TCursor`), then let the
/// setup window close itself.
#[tauri::command]
pub fn launch_app() -> Result<(), String> {
    let base = std::env::var("LOCALAPPDATA").map_err(|_| "LOCALAPPDATA not set".to_string())?;
    let exe = PathBuf::from(base).join("TCursor").join("tcursor-scaffold.exe");
    Command::new(&exe)
        .spawn()
        .map_err(|e| format!("Could not launch TCursor: {e}"))?;
    Ok(())
}
