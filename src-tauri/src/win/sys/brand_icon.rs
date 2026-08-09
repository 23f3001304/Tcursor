// Dynamic app icon + Windows taskbar progress (Task 39 - "the wave lives"): swaps the main
// window's icon between the normal brand mark and a REC-lit variant while recording, and drives
// the taskbar progress bar during export. Both are pure brand flair - graceful no-ops on any
// failure or off-Windows (mirrors `capture_exclusion.rs`'s split), so a window-manager quirk or a
// headless test run can never turn a decorative touch into a crash or a stuck recording/export.
use tauri::{AppHandle, Manager};

const ICON_NORMAL: &[u8] = include_bytes!("../../../icons/icon.png");
const ICON_REC: &[u8] = include_bytes!("../../../icons/icon-rec.png");

/// Swap the main window's icon: the REC-lit variant while `recording`, else the normal mark.
/// Called from `start_recording`/`stop_recording` - paused stays REC-lit (still "recording" in
/// every sense that matters to this indicator; only Stop clears it).
pub fn set_recording(app: &AppHandle, recording: bool) {
    #[cfg(windows)]
    {
        let Some(win) = app.get_webview_window("main") else { return };
        let Some(img) = decode_png(if recording { ICON_REC } else { ICON_NORMAL }) else { return };
        let _ = win.set_icon(img);
    }
    #[cfg(not(windows))]
    let _ = (app, recording);
}

/// Drive the Windows taskbar progress bar from an export's percent-complete. `Some(pct)` shows a
/// normal-state bar at that progress; `None` clears it (called once export settles, success or
/// error, so a finished run doesn't leave a stale bar behind).
pub fn set_export_progress(app: &AppHandle, pct: Option<u8>) {
    #[cfg(windows)]
    {
        let Some(win) = app.get_webview_window("main") else { return };
        let _ = win.set_progress_bar(progress_state(pct));
    }
    #[cfg(not(windows))]
    let _ = (app, pct);
}

/// Pure: the `ProgressBarState` for a given percent - `None` clears the bar, `Some` shows it at
/// that value (clamped 0..100, though `on_progress`'s `u8` already can't exceed 255). Split out
/// from `set_export_progress` so the mapping itself is unit-testable without a real window.
#[cfg(windows)]
fn progress_state(pct: Option<u8>) -> tauri::window::ProgressBarState {
    use tauri::window::{ProgressBarState, ProgressBarStatus};
    match pct {
        Some(p) => ProgressBarState { status: Some(ProgressBarStatus::Normal), progress: Some(p.min(100) as u64) },
        None => ProgressBarState { status: Some(ProgressBarStatus::None), progress: None },
    }
}

/// Decode PNG `bytes` into a Tauri `Image`, entirely in-process via the `png` crate (already a
/// dependency for the preview PNG encoder) rather than enabling Tauri's own `image-png` feature
/// just for this one call site - the "no new deps" rule reads a feature flag that pulls in the
/// `image` crate's whole format-decoder stack as added weight, where three lines against a
/// dependency we already build gets the exact same result. `normalize_to_color8 | ALPHA` forces
/// RGBA8 output regardless of the source PNG's actual color type (both bundled icons are RGBA8
/// already, so this is a formality, not a real conversion, on the happy path).
#[cfg(windows)]
fn decode_png(bytes: &[u8]) -> Option<tauri::image::Image<'static>> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None; // shouldn't happen given the transform above; fail closed rather than misread bytes
    }
    buf.truncate(info.buffer_size());
    Some(tauri::image::Image::new_owned(buf, info.width, info.height))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn both_bundled_icons_decode_to_nonzero_rgba() {
        for bytes in [ICON_NORMAL, ICON_REC] {
            let img = decode_png(bytes).expect("bundled icon should decode");
            assert!(img.width() > 0 && img.height() > 0);
            assert_eq!(img.rgba().len(), (img.width() * img.height() * 4) as usize);
        }
    }

    #[test]
    fn progress_state_some_is_normal_status_clamped_to_100() {
        // ProgressBarStatus derives no PartialEq (tauri-runtime), so match it directly.
        let s = progress_state(Some(250)); // u8 can't actually exceed 255, but exercise the clamp anyway
        assert!(matches!(s.status, Some(tauri::window::ProgressBarStatus::Normal)));
        assert_eq!(s.progress, Some(100));
    }

    #[test]
    fn progress_state_none_clears_the_bar() {
        let s = progress_state(None);
        assert!(matches!(s.status, Some(tauri::window::ProgressBarStatus::None)));
        assert_eq!(s.progress, None);
    }
}
