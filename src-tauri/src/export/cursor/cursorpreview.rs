// Editor-preview cursor: expose the export's selected cursor pack + the cursor-type track to
// the frontend so the canvas preview draws the SAME cursor the export renders (gated by style),
// instead of a generic arrow. Sprites are decoded/cropped/dark-inverted exactly like the export.
use std::path::PathBuf;
use crate::events::track::cursortype::{CursorTrack, CursorType};
use crate::export::cursor::cursordraw::decode_sprite;
use crate::export::cursor::pack;
use crate::export::preview::{base64_encode, png_encode, with_warm, PreviewSession};
use crate::session::paths::ProjectPaths;

/// One cursor sprite for the canvas preview: its type, a PNG data URL (cropped + dark-inverted
/// to match the export), the hotspot (0..1 of the cropped sprite), and the original canvas
/// height (so all shapes scale on the same basis the export uses).
#[derive(serde::Serialize)]
pub struct CursorSpriteDto { pub kind: CursorType, pub url: String, pub hot: [f32; 2], pub canvas_h: u32 }

/// The recording's selected cursor pack (built-in, or an imported pack falling back to the
/// built-in per-kind - see `export/cursor/pack.rs`) as PNG data URLs, decoded like the export
/// (crop to alpha, hotspot re-based, RGB-inverted for a dark theme). The preview draws these
/// when the recording's cursor style is Enhanced. Errors only if the Arrow fallback fails.
#[tauri::command]
pub fn cursor_sprites(folder: String) -> Result<Vec<CursorSpriteDto>, String> {
    let paths = ProjectPaths { folder: PathBuf::from(&folder) };
    let settings = crate::edit::seed::load_or_seed(&paths).settings;
    let dark = crate::win::theme::resolve_dark(settings.ui.theme);
    let mut out = Vec::new();
    for (kind, png, hot) in pack::sprite_sources(&settings.cursor.pack) {
        if let Some(mut spr) = decode_sprite(&png, hot) {
            if dark { invert_rgb(&mut spr.bgra); }
            if let Ok(bytes) = png_encode(&spr.bgra, spr.w, spr.h) {
                out.push(CursorSpriteDto {
                    kind,
                    url: format!("data:image/png;base64,{}", base64_encode(&bytes)),
                    hot: [spr.hot.0, spr.hot.1],
                    canvas_h: spr.canvas_h,
                });
            }
        }
    }
    if !out.iter().any(|d| d.kind == CursorType::Arrow) {
        return Err("cursor arrow sprite failed to decode".into());
    }
    Ok(out)
}

/// One cursor-shape change at output time `t` (ms).
#[derive(serde::Serialize)]
pub struct CursorKindSample { pub t: u32, pub kind: CursorType }

/// The cursor-type track in OUTPUT time, so the preview can pick the right sprite as the shape
/// changes. Maps each `cursor.json` sample (event time) to output ms the same way the click
/// track does (`et + events_ms - video_start`, dropping pre-start samples).
#[tauri::command]
pub fn cursor_kinds(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<Vec<CursorKindSample>, String> {
    with_warm(&session, &folder, |c, paths| {
        let off = c.renderer.events_ms() as i64 - c.meta.video_start as i64;
        Ok(CursorTrack::load(&paths.cursor()).samples.iter().filter_map(|&(et, kind)| {
            let t = et as i64 + off;
            (t >= 0).then_some(CursorKindSample { t: t as u32, kind })
        }).collect())
    })
}

/// Invert R/G/B in place for a dark-theme cursor; alpha untouched. Mirrors `cursorset`.
fn invert_rgb(bgra: &mut [u8]) {
    for px in bgra.chunks_exact_mut(4) {
        px[0] = 255 - px[0];
        px[1] = 255 - px[1];
        px[2] = 255 - px[2];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invert_flips_rgb_keeps_alpha() {
        let mut px = vec![10u8, 20, 30, 255];
        invert_rgb(&mut px);
        assert_eq!(px, vec![245, 235, 225, 255]);
    }
}
