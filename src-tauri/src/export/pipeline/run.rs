use tauri::{AppHandle, Emitter};
use crate::export::settings::ExportSettings;
use crate::session::paths::ProjectPaths;

/// Run the export on a background thread, emitting progress/done/error events. `export-done`'s
/// payload is the final output file's absolute path (`<folder>/final.<ext>`, the same path
/// `audio_mux::mux` writes to - `ExportSettings` is `Copy` so reading `settings.format` after
/// handing a copy to `export()` is fine), not just the project folder, so the frontend can offer
/// "Show in folder" (`revealItemInDir`) without re-deriving the extension itself.
pub fn run_export(app: AppHandle, folder: String, settings: ExportSettings) {
    std::thread::spawn(move || {
        let paths = ProjectPaths { folder: std::path::PathBuf::from(&folder) };
        let final_path = paths.folder.join(format!("final.{}", settings.format.extension()));
        let app2 = app.clone();
        let res = crate::export::pipeline::exporter::export(&paths, settings, move |p| {
            let _ = app2.emit("export-progress", p);
            // Taskbar progress mirrors the in-app bar (Task 39) - brand flair only, never fails the export.
            crate::win::sys::brand_icon::set_export_progress(&app2, Some(p));
        });
        // Either way the export has settled - clear the taskbar bar so a finished run doesn't
        // leave a stale progress indicator sitting on the icon.
        crate::win::sys::brand_icon::set_export_progress(&app, None);
        match res {
            Ok(()) => { let _ = app.emit("export-done", final_path.to_string_lossy().into_owned()); }
            Err(e) => { let _ = app.emit("export-error", e.to_string()); }
        }
    });
}
