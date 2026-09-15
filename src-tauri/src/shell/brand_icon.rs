use tauri::{AppHandle, Manager};

const ICON_NORMAL: &[u8] = include_bytes!("../../icons/icon.png");
const ICON_REC: &[u8] = include_bytes!("../../icons/icon-rec.png");

pub fn set_recording(app: &AppHandle, recording: bool) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    let Some(img) = decode_png(if recording { ICON_REC } else { ICON_NORMAL }) else {
        return;
    };
    let _ = win.set_icon(img);
}

pub fn set_export_progress(app: &AppHandle, pct: Option<u8>) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    let _ = win.set_progress_bar(progress_state(pct));
}

fn progress_state(pct: Option<u8>) -> tauri::window::ProgressBarState {
    use tauri::window::{ProgressBarState, ProgressBarStatus};
    match pct {
        Some(p) => ProgressBarState {
            status: Some(ProgressBarStatus::Normal),
            progress: Some(p.min(100) as u64),
        },
        None => ProgressBarState {
            status: Some(ProgressBarStatus::None),
            progress: None,
        },
    }
}

fn decode_png(bytes: &[u8]) -> Option<tauri::image::Image<'static>> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    Some(tauri::image::Image::new_owned(buf, info.width, info.height))
}

#[cfg(test)]
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
        let s = progress_state(Some(250));
        assert!(matches!(
            s.status,
            Some(tauri::window::ProgressBarStatus::Normal)
        ));
        assert_eq!(s.progress, Some(100));
    }

    #[test]
    fn progress_state_none_clears_the_bar() {
        let s = progress_state(None);
        assert!(matches!(
            s.status,
            Some(tauri::window::ProgressBarStatus::None)
        ));
        assert_eq!(s.progress, None);
    }
}
