use tauri::{AppHandle, Emitter};
use crate::export::settings::ExportSettings;
use crate::session::paths::ProjectPaths;

/// The message inside a caught panic payload (`&str` or `String`; anything else is opaque).
fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() { return (*s).to_string(); }
    if let Some(s) = p.downcast_ref::<String>() { return s.clone(); }
    "unknown panic".to_string()
}

/// Run the export on a background thread, emitting progress/warning/done/error events. `export-done`'s
/// payload is the final output file's absolute path (`<folder>/final.<ext>`, the same path
/// `audio_mux::mux` writes to - `ExportSettings` is `Copy` so reading `settings.format` after
/// handing a copy to `export()` is fine), not just the project folder, so the frontend can offer
/// "Show in folder" (`revealItemInDir`) without re-deriving the extension itself.
///
/// The export body runs under `catch_unwind`: a panic in it (the compositor's readback `.expect`,
/// a `resize_crop` assert, a GPU device fault) used to unwind straight out of this closure, so
/// NEITHER event fired and the taskbar progress indicator stayed pinned at whatever percent it
/// had reached, forever. A panic is now just another `export-error`, and the settle path below
/// (which clears the taskbar bar) always runs.
pub fn run_export(app: AppHandle, folder: String, settings: ExportSettings) {
    std::thread::spawn(move || {
        let paths = ProjectPaths { folder: std::path::PathBuf::from(&folder) };
        let final_path = paths.folder.join(format!("final.{}", settings.format.extension()));
        let app2 = app.clone();
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::export::pipeline::exporter::export(&paths, settings, move |p| {
                let _ = app2.emit("export-progress", p);
                // Taskbar progress mirrors the in-app bar (Task 39) - brand flair only, never fails the export.
                crate::win::sys::brand_icon::set_export_progress(&app2, Some(p));
            })
        }))
        .unwrap_or_else(|p| Err(anyhow::anyhow!("export panicked: {}", panic_text(p))));
        // Either way the export has settled - clear the taskbar bar so a finished run doesn't
        // leave a stale progress indicator sitting on the icon.
        crate::win::sys::brand_icon::set_export_progress(&app, None);
        match res {
            Ok(warnings) => {
                // Non-fatal: the file exists but is missing something the user asked for (see
                // `export`'s return). Emitted BEFORE `export-done` so a listener that reacts to
                // done can already see them.
                for w in warnings { let _ = app.emit("export-warning", w); }
                let _ = app.emit("export-done", final_path.to_string_lossy().into_owned());
            }
            Err(e) => { let _ = app.emit("export-error", e.to_string()); }
        }
    });
}
