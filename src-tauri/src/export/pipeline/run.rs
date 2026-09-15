use crate::export::settings::ExportSettings;
use crate::platform::Platform;
use crate::session::paths::ProjectPaths;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        return (*s).to_string();
    }
    if let Some(s) = p.downcast_ref::<String>() {
        return s.clone();
    }
    "unknown panic".to_string()
}

pub fn run_export(app: AppHandle, folder: String, settings: ExportSettings) {
    use tauri::Manager;
    let platform = app.state::<Arc<Platform>>().inner().clone();
    std::thread::spawn(move || {
        let paths = ProjectPaths {
            folder: std::path::PathBuf::from(&folder),
        };
        let final_path = paths
            .folder
            .join(format!("final.{}", settings.format.extension()));
        let app2 = app.clone();
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::export::pipeline::exporter::export(
                &paths,
                settings,
                platform.system.as_ref(),
                move |p| {
                    let _ = app2.emit("export-progress", p);
                    crate::shell::brand_icon::set_export_progress(&app2, Some(p));
                },
            )
        }))
        .unwrap_or_else(|p| Err(anyhow::anyhow!("export panicked: {}", panic_text(p))));
        crate::shell::brand_icon::set_export_progress(&app, None);
        match res {
            Ok(warnings) => {
                for w in warnings {
                    let _ = app.emit("export-warning", w);
                }
                let _ = app.emit("export-done", final_path.to_string_lossy().into_owned());
            }
            Err(e) => {
                let _ = app.emit("export-error", e.to_string());
            }
        }
    });
}
