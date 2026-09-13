//! Measuring the install instead of miming it: where NSIS writes, how much it will write, and
//! the one pure mapping from what we observe to what the window shows.

use std::path::{Path, PathBuf};

// `pub const EXPECTED_BYTES: u64` - the uncompressed install size, taken at build time from the
// bundler's own `ESTIMATEDSIZE`. 0 means this build could not know it (see build.rs).
include!(concat!(env!("OUT_DIR"), "/expected_bytes.rs"));

/// The binary NSIS installs, from `MAINBINARYNAME` in the generated `installer.nsi`.
pub const MAIN_BINARY: &str = "tcursor-scaffold.exe";

/// Where NSIS puts the app. `src-tauri/tauri.conf.json` sets `bundle.windows.nsis.installMode`
/// to `currentUser`, so the generated script hands MultiUser.nsh `MULTIUSER_INSTALLMODE_INSTDIR
/// = <productName>` and `$INSTDIR` resolves to `%LOCALAPPDATA%\TCursor`.
pub fn destination() -> PathBuf {
    PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("TCursor")
}

/// Total size of every file under `root`, or 0 if it does not exist yet. Called once per poll,
/// against a folder holding a few dozen files.
pub fn dir_bytes(root: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            match entry.metadata() {
                Ok(m) if m.is_dir() => stack.push(entry.path()),
                Ok(m) => total += m.len(),
                Err(_) => {}
            }
        }
    }
    total
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// The child is up but nothing has landed in the destination yet.
    Preparing,
    /// Files are arriving, or - with no expected size - the timed fallback is pacing.
    Installing,
    /// Every expected byte is written and the child is still doing registry and shortcut work.
    Registering,
    Installed,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Step {
    pub percent: u32,
    pub phase: Phase,
    pub status: String,
}

const PACE_FROM: u32 = 3;
const PACE_PER_MS: u64 = 220;
const PACE_CAP: u32 = 92;
/// Files can finish long before the child does, so the bar waits at 97 for the exit code.
const RUNNING_CAP: u32 = 97;

/// What the window should show. `child_running == false` means the installer exited 0; a failure
/// never reaches this function, it goes straight to the error state.
///
/// `elapsed_ms` is used only when `expected` is 0, where there is nothing to measure and the old
/// timed pacing takes over.
pub fn step(bytes: u64, expected: u64, child_running: bool, elapsed_ms: u64) -> Step {
    if !child_running {
        return Step { percent: 100, phase: Phase::Installed, status: "Installed".into() };
    }
    if expected == 0 {
        let percent = (PACE_FROM + 2 * (elapsed_ms / PACE_PER_MS) as u32).min(PACE_CAP);
        return Step { percent, phase: Phase::Installing, status: "Installing files".into() };
    }
    if bytes == 0 {
        return Step { percent: 0, phase: Phase::Preparing, status: "Preparing".into() };
    }
    if bytes >= expected {
        return Step {
            percent: RUNNING_CAP,
            phase: Phase::Registering,
            status: "Registering shortcuts".into(),
        };
    }
    Step {
        percent: ((bytes * 100 / expected) as u32).min(RUNNING_CAP),
        phase: Phase::Installing,
        status: format!("Installing files, {} of {} MB", mb(bytes).max(1), mb(expected)),
    }
}

/// Megabytes as Windows counts them, rounded to nearest.
fn mb(bytes: u64) -> u64 {
    (bytes + 512 * 1024) / (1024 * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1024 * 1024;

    #[test]
    fn an_exited_child_is_always_finished() {
        for expected in [0, 38 * MB] {
            for bytes in [0, 12 * MB, 99 * MB] {
                let s = step(bytes, expected, false, 0);
                assert_eq!(s.percent, 100);
                assert_eq!(s.phase, Phase::Installed);
                assert_eq!(s.status, "Installed");
            }
        }
    }

    #[test]
    fn nothing_written_yet_reads_as_preparing() {
        let s = step(0, 38 * MB, true, 3_000);
        assert_eq!((s.percent, s.phase), (0, Phase::Preparing));
        assert_eq!(s.status, "Preparing");
    }

    #[test]
    fn bytes_map_to_a_percent_and_a_byte_count() {
        let s = step(12 * MB, 38 * MB, true, 0);
        assert_eq!((s.percent, s.phase), (31, Phase::Installing));
        assert_eq!(s.status, "Installing files, 12 of 38 MB");
    }

    #[test]
    fn the_first_bytes_still_read_as_one_megabyte() {
        let s = step(4_096, 100 * MB, true, 0);
        assert_eq!(s.percent, 0);
        assert_eq!(s.status, "Installing files, 1 of 100 MB");
    }

    #[test]
    fn megabytes_round_to_nearest() {
        assert_eq!(mb(0), 0);
        assert_eq!(mb(12 * MB + 600 * 1024), 13);
        assert_eq!(mb(12 * MB + 100 * 1024), 12);
        assert_eq!(step(12 * MB + 600 * 1024, 38 * MB, true, 0).status, "Installing files, 13 of 38 MB");
    }

    #[test]
    fn progress_is_clamped_until_the_child_exits() {
        assert_eq!(step(38 * MB - 1, 38 * MB, true, 0).percent, 97);
        for bytes in [38 * MB, 40 * MB] {
            let s = step(bytes, 38 * MB, true, 0);
            assert_eq!((s.percent, s.phase), (97, Phase::Registering));
            assert_eq!(s.status, "Registering shortcuts");
        }
    }

    #[test]
    fn elapsed_time_is_ignored_once_the_size_is_known() {
        for bytes in [0, 12 * MB, 38 * MB] {
            assert_eq!(step(bytes, 38 * MB, true, 0), step(bytes, 38 * MB, true, 600_000));
        }
    }

    #[test]
    fn an_unknown_size_falls_back_to_the_timed_pace() {
        for (elapsed, percent) in [(0, 3), (219, 3), (220, 5), (440, 7), (10_120, 92), (600_000, 92)] {
            let s = step(0, 0, true, elapsed);
            assert_eq!(s.percent, percent, "at {elapsed}ms");
            assert_eq!(s.phase, Phase::Installing);
            assert_eq!(s.status, "Installing files");
        }
    }

    #[test]
    fn the_timed_fallback_ignores_bytes_it_cannot_scale() {
        assert_eq!(step(0, 0, true, 440), step(99 * MB, 0, true, 440));
    }

    #[test]
    fn a_missing_destination_measures_zero() {
        assert_eq!(dir_bytes(Path::new(r"C:\this\path\does\not\exist")), 0);
    }

    #[test]
    fn dir_bytes_sums_nested_files() {
        let root = std::env::temp_dir().join(format!("tcursor_setup_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("resources")).unwrap();
        std::fs::write(root.join("app.exe"), vec![0u8; 1000]).unwrap();
        std::fs::write(root.join("resources/ffmpeg.exe"), vec![0u8; 2345]).unwrap();
        assert_eq!(dir_bytes(&root), 3345);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
