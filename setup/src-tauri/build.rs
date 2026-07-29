use std::{env, fs, path::PathBuf};

// Stage the NSIS installer (built by the main app, copied here by setup/build.ps1) into OUT_DIR
// so `install.rs` can `include_bytes!` it. To keep `cargo build`/`cargo check` working during dev
// without a 58MB payload, fall back to an empty placeholder and warn - the runtime detects the
// tiny payload and refuses to install, and build.ps1 always stages the real one for a shipping build.
fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest = out.join("nsis-setup.exe");
    let staged = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("assets/nsis-setup.exe");

    if staged.exists() && fs::metadata(&staged).map(|m| m.len() > 1024).unwrap_or(false) {
        fs::copy(&staged, &dest).expect("copy staged NSIS payload into OUT_DIR");
    } else if !dest.exists() {
        fs::write(&dest, b"").expect("write placeholder NSIS payload");
        println!("cargo:warning=No NSIS payload at assets/nsis-setup.exe; embedding empty placeholder. Run setup/build.ps1 to produce a real installer.");
    }

    println!("cargo:rerun-if-changed=assets/nsis-setup.exe");
    tauri_build::build();
}
