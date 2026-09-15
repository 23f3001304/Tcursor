use tauri::Manager;

use crate::session::record::recorder::Recorder;
use crate::session::record::recorder_stop::stop_recording;

pub async fn finish_and_close(app: tauri::AppHandle) {
    let recorder = app.state::<Recorder>();
    if recorder.is_recording() {
        let _ = stop_recording(app.clone()).await;
    } else {
        let app2 = app.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            let recorder = app2.state::<Recorder>();
            while recorder.is_busy() {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        })
        .await;
    }
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.close();
    }
}
