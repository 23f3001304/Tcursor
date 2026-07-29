// TCursor Setup - a branded installer window (our look) over the silent NSIS installer.
// The NSIS `.exe` is embedded at build time and run with `/S`; this app is just the face + progress.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod install;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![install::start_install, install::launch_app])
        .run(tauri::generate_context!())
        .expect("error while running TCursor Setup");
}
