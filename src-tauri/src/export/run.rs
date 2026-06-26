use tauri::{AppHandle, Emitter};
use crate::session::paths::ProjectPaths;

/// Run the export on a background thread, emitting progress/done/error events.
pub fn run_export(app: AppHandle, folder: String) {
    let fps = crate::win::display::primary_refresh_hz().min(60);
    std::thread::spawn(move || {
        let paths = ProjectPaths { folder: std::path::PathBuf::from(&folder) };
        let app2 = app.clone();
        let res = crate::export::exporter::export(&paths, fps, move |p| {
            let _ = app2.emit("export-progress", p);
        });
        match res {
            Ok(()) => { let _ = app.emit("export-done", folder); }
            Err(e) => { let _ = app.emit("export-error", e.to_string()); }
        }
    });
}
