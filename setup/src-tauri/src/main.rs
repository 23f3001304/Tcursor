// TCursor Setup - a branded installer window (our look) over the silent NSIS installer.
// The NSIS `.exe` is embedded at build time and run with `/S`; this app is the face, the progress
// measurement and the log.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod install;
mod log;
mod progress;

use tauri::{Emitter, WindowEvent};

fn main() {
    log::line("setup window opened");
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            install::start_install,
            install::launch_app,
            install::setup_info,
            install::license_text,
            install::copy_log,
            install::allow_close,
        ])
        // Esc, the window's own close button and Alt+F4 all arrive here. Mid-install they are
        // answered by a line in the window, never by an OS dialog; the UI replies with
        // `allow_close`, which sets the flag this guard reads.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if install::busy() && !install::close_allowed() {
                    api.prevent_close();
                    let _ = window.emit("setup://close-request", ());
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running TCursor Setup");
}
