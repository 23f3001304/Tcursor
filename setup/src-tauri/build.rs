use std::{env, fs, path::Path, path::PathBuf};

// Stage the NSIS installer (built by the main app, copied here by setup/build.ps1) into OUT_DIR
// so `install.rs` can `include_bytes!` it. To keep `cargo build`/`cargo check` working during dev
// without a 58MB payload, fall back to an empty placeholder and warn - the runtime detects the
// tiny payload and refuses to install, and build.ps1 always stages the real one for a shipping build.
//
// Two more things are baked in here so the window can measure real work instead of pacing a fake
// bar: how many bytes the installer will write (`expected_bytes.rs`) and the license as plain text
// (`eula.txt`, converted from the RTF the NSIS page shows).
fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let dest = out.join("nsis-setup.exe");
    let staged = manifest.join("assets/nsis-setup.exe");

    if staged.exists() && fs::metadata(&staged).map(|m| m.len() > 1024).unwrap_or(false) {
        fs::copy(&staged, &dest).expect("copy staged NSIS payload into OUT_DIR");
    } else if !dest.exists() {
        fs::write(&dest, b"").expect("write placeholder NSIS payload");
        println!("cargo:warning=No NSIS payload at assets/nsis-setup.exe; embedding empty placeholder. Run setup/build.ps1 to produce a real installer.");
    }

    let bytes = expected_bytes(&manifest);
    if bytes == 0 {
        println!("cargo:warning=Install size unknown; the progress bar will use its timed fallback. Build through setup/build.ps1, or set TCURSOR_INSTALL_BYTES.");
    }
    fs::write(out.join("expected_bytes.rs"), format!("pub const EXPECTED_BYTES: u64 = {bytes};\n"))
        .expect("write expected_bytes.rs");
    fs::write(out.join("eula.txt"), license_text(&manifest)).expect("write eula.txt");

    println!("cargo:rerun-if-changed=assets/nsis-setup.exe");
    println!("cargo:rerun-if-env-changed=TCURSOR_INSTALL_BYTES");
    tauri_build::build();
}

/// Bytes the NSIS installer will write into `$INSTDIR`, or 0 when this build cannot know.
///
/// The payload we embed is a copy; its own source tree is the main app's bundle staging folder,
/// `<repo>/src-tauri/target/release/nsis/x64/`, which we can reach because CARGO_MANIFEST_DIR is
/// `<repo>/setup/src-tauri`. That folder holds the generated `installer.nsi`, and the bundler
/// writes its own sum of every file the installer places, in KB, as `!define ESTIMATEDSIZE`.
/// That is the uncompressed install size, from the same tool that packs the payload.
/// `TCURSOR_INSTALL_BYTES` is the fallback for a tree where the staging folder was pruned.
fn expected_bytes(manifest: &Path) -> u64 {
    if let Some(repo) = manifest.parent().and_then(Path::parent) {
        let nsi = repo.join("src-tauri/target/release/nsis/x64/installer.nsi");
        println!("cargo:rerun-if-changed={}", nsi.display());
        if let Some(kb) = fs::read_to_string(&nsi).ok().as_deref().and_then(estimated_size_kb) {
            return kb * 1024;
        }
    }
    env::var("TCURSOR_INSTALL_BYTES").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(0)
}

fn estimated_size_kb(nsi: &str) -> Option<u64> {
    let line = nsi.lines().find(|l| l.trim_start().starts_with("!define ESTIMATEDSIZE"))?;
    line.split('"').nth(1)?.trim().parse().ok()
}

fn license_text(manifest: &Path) -> String {
    let rtf = manifest
        .parent()
        .and_then(Path::parent)
        .map(|repo| repo.join("src-tauri/installer/eula.rtf"))
        .and_then(|p| {
            println!("cargo:rerun-if-changed={}", p.display());
            fs::read_to_string(p).ok()
        });
    match rtf {
        Some(rtf) => rtf_to_text(&rtf),
        None => "The license could not be read at build time. See eula.rtf in the TCursor repository.".into(),
    }
}

/// Flatten the small, hand-written subset of RTF that `eula.rtf` uses: one ignorable destination
/// (`\fonttbl`), `\par` breaks, `\endash`, formatting control words, and brace groups.
fn rtf_to_text(rtf: &str) -> String {
    let mut out = String::new();
    let mut chars = rtf.chars().peekable();
    let mut skip_to_depth: Option<i32> = None;
    let mut depth = 0;
    while let Some(c) = chars.next() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if skip_to_depth == Some(depth) {
                    skip_to_depth = None;
                }
            }
            '\\' => {
                let escaped = matches!(chars.peek(), Some('\\') | Some('{') | Some('}'));
                if escaped {
                    let c = chars.next().unwrap();
                    if skip_to_depth.is_none() {
                        out.push(c);
                    }
                    continue;
                }
                let mut word = String::new();
                while chars.peek().is_some_and(|c| c.is_ascii_alphanumeric() || *c == '-') {
                    word.push(chars.next().unwrap());
                }
                if chars.peek() == Some(&' ') {
                    chars.next();
                }
                if word == "fonttbl" || word == "colortbl" || word == "stylesheet" || word == "info" {
                    skip_to_depth = Some(depth - 1);
                } else if skip_to_depth.is_none() {
                    match word.as_str() {
                        "par" | "line" => out.push('\n'),
                        "tab" => out.push('\t'),
                        "endash" | "emdash" => out.push('-'),
                        _ => {}
                    }
                }
            }
            '\r' | '\n' => {}
            _ if skip_to_depth.is_none() => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}
