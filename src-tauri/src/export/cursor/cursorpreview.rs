// Editor-preview cursor: expose the export's selected cursor pack + the cursor-type track to
// the frontend so the canvas preview draws the SAME cursor the export renders (gated by style),
// instead of a generic arrow. Sprites are decoded/cropped/dark-inverted exactly like the export.
use std::path::PathBuf;
use crate::events::track::cursorlayer::CursorLayer;
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

/// The recording's selected pack, ready for the canvas preview: one sprite per kind, the pack's
/// explicit busy frames (empty unless it ships `busy_NN.png`), and its declared busy animation.
/// `busy` + `busy_frames` are what let the preview run the SAME `busy_pose` the export does.
#[derive(serde::Serialize)]
pub struct CursorPackDto {
    pub sprites: Vec<CursorSpriteDto>,
    pub busy_frames: Vec<CursorSpriteDto>,
    pub busy: Option<crate::export::cursor::busy::BusySpec>,
}

/// The recording's selected cursor pack (embedded, bundled, or imported - see
/// `export/cursor/pack.rs`) as PNG data URLs, decoded like the export (crop to alpha, hotspot
/// re-based, RGB-inverted for a dark theme). The preview draws these when the recording's cursor
/// style is Enhanced. Errors only if the Arrow fallback fails.
#[tauri::command]
pub fn cursor_sprites(folder: String) -> Result<CursorPackDto, String> {
    let paths = ProjectPaths { folder: PathBuf::from(&folder) };
    let settings = crate::edit::seed::load_or_seed(&paths).settings;
    let pack_id = &settings.cursor.pack;
    // Only the embedded default set inverts for a dark theme (see `pack::theme_inverts`).
    let dark = crate::win::theme::resolve_dark(settings.ui.theme) && pack::theme_inverts(pack_id);
    let sprites: Vec<CursorSpriteDto> = pack::sprite_sources(pack_id).into_iter()
        .filter_map(|(kind, png, hot)| sprite_dto(kind, &png, hot, dark)).collect();
    if !sprites.iter().any(|d| d.kind == CursorType::Arrow) {
        return Err("cursor arrow sprite failed to decode".into());
    }
    // Explicit frames get the busy sprite's own centered hotspot, matching `cursorset::prep`.
    let busy_frames = pack::busy_frames(pack_id).iter()
        .filter_map(|png| sprite_dto(CursorType::Busy, png, (0.5, 0.5), dark)).collect();
    Ok(CursorPackDto { sprites, busy_frames, busy: pack::busy_spec(pack_id) })
}

/// Decode one pack PNG the way the export does, then re-encode it as a data URL for the canvas.
fn sprite_dto(kind: CursorType, png: &[u8], hot: (f32, f32), dark: bool) -> Option<CursorSpriteDto> {
    let mut spr = decode_sprite(png, hot)?;
    if dark { invert_rgb(&mut spr.bgra); }
    let bytes = png_encode(&spr.bgra, spr.w, spr.h).ok()?;
    Some(CursorSpriteDto {
        kind,
        url: format!("data:image/png;base64,{}", base64_encode(&bytes)),
        hot: [spr.hot.0, spr.hot.1],
        canvas_h: spr.canvas_h,
    })
}

/// One cursor-shape change at output time `t` (ms).
#[derive(serde::Serialize)]
pub struct CursorKindSample { pub t: u32, pub kind: CursorType }

/// The cursor-type track in OUTPUT time, so the preview can pick the right sprite as the shape
/// changes. Maps each `cursor.json` sample (event time) to output ms the same way the click
/// track does (`et + events_ms - video_start`, dropping pre-start samples).
///
/// `async` + `spawn_blocking` like every other `with_warm` command (see `preview_frame`): the
/// body itself is a file read + map, but a cold cache runs `FrameRenderer::new` underneath it.
#[tauri::command]
pub async fn cursor_kinds(folder: String, app: tauri::AppHandle) -> Result<Vec<CursorKindSample>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        with_warm(&app.state::<PreviewSession>(), &folder, |c, paths| {
            let off = c.renderer.events_ms() as i64 - c.meta.video_start as i64;
            Ok(CursorTrack::load(&paths.cursor()).samples.iter().filter_map(|&(et, kind)| {
                let t = et as i64 + off;
                (t >= 0).then_some(CursorKindSample { t: t as u32, kind })
            }).collect())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// One captured OS-cursor bitmap for the canvas preview: its layer id, pixel size, hotspot in
/// pixels, and the recorded PNG as a data URL (untouched - it is the real cursor, not a sprite).
#[derive(serde::Serialize)]
pub struct CapturedCursorDto { pub id: u32, pub w: u32, pub h: u32, pub hx: u32, pub hy: u32, pub url: String }

/// The whole captured cursor layer, with `track` mapped to OUTPUT ms the same way `cursor_kinds`
/// maps the shape track, so the preview can look both up against the playhead. `src_w`/`src_h` are
/// the recorded video's own pixel size: the cursor bitmaps are in SOURCE pixels, so the preview
/// needs it to scale them relative to the screen content exactly as the export does.
#[derive(serde::Serialize)]
pub struct CursorLayerDto {
    pub cursors: Vec<CapturedCursorDto>,
    pub track: Vec<(u32, u32)>,
    pub src_w: u32,
    pub src_h: u32,
}

/// The recording's captured OS-cursor layer, or `None` for one made before the layer existed (and
/// for an unreadable one - a missing cursor is never an error). Sync like `cursor_sprites`, which
/// also shells out per call: a small JSON read, a handful of cursor-sized PNGs, and ONE `probe_dims`
/// - run only after the layer resolves, so a legacy project pays nothing for it.
#[tauri::command]
pub fn cursor_layer(folder: String) -> Option<CursorLayerDto> {
    let paths = ProjectPaths { folder: PathBuf::from(&folder) };
    let layer = CursorLayer::load(&paths)?;
    // The SAME probe `FrameRenderer::new` derives its `sw`/`sh` from, so preview and export scale
    // the cursor identically. `(0, 0)` if it fails - the preview then falls back to the panel-only
    // scale, and an export would not have built at all (its own `probe_dims` is a hard error).
    let (src_w, src_h) = crate::export::pipeline::ffio::probe_dims(&paths.video()).unwrap_or((0, 0));
    let off = output_offset(&paths);
    let cursors = layer.cursors.iter().filter_map(|e| {
        let bytes = std::fs::read(paths.cursor_dir().join(&e.file)).ok()?;
        Some(CapturedCursorDto { id: e.id, w: e.w, h: e.h, hx: e.hx, hy: e.hy,
            url: format!("data:image/png;base64,{}", base64_encode(&bytes)) })
    }).collect();
    let track = layer.track.iter().filter_map(|&(et, id)| {
        let t = et as i64 + off;
        (t >= 0).then_some((t as u32, id))
    }).collect();
    Some(CursorLayerDto { cursors, track, src_w, src_h })
}

/// Event time -> output time, in ms: `events_ms - video_start`, read straight from `sync.json`
/// (whose `frames[0]` IS the video start `build_timeline` hands the renderer). 0 for a recording
/// with no usable sync log, which is the same fallback the synthesized timeline uses.
fn output_offset(paths: &ProjectPaths) -> i64 {
    match crate::session::sync::SyncLog::load(&paths.sync()) {
        Ok(s) => match s.frames.first() {
            Some(&start) => s.events_ms as i64 - start as i64,
            None => 0,
        },
        Err(_) => 0,
    }
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
    use crate::session::sync::SyncLog;

    #[test]
    fn invert_flips_rgb_keeps_alpha() {
        let mut px = vec![10u8, 20, 30, 255];
        invert_rgb(&mut px);
        assert_eq!(px, vec![245, 235, 225, 255]);
    }

    #[test]
    fn the_layer_track_is_shifted_onto_the_output_clock() {
        // Mouse tracking started at 900ms and the first video frame landed at 1000ms, so an
        // event-time sample is 100ms EARLIER on the output clock - the same shift `cursor_kinds`
        // applies, so both tracks can be looked up against the one playhead.
        let dir = std::env::temp_dir().join(format!("tcursor-curoff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let paths = ProjectPaths { folder: dir };
        SyncLog { frames: vec![1000, 1016], events_ms: 900, mic_ms: None, system_ms: None }
            .save(&paths.sync()).unwrap();
        assert_eq!(output_offset(&paths), -100);
        // No sync.json at all (a synthesized timeline): no shift rather than a guess.
        let _ = std::fs::remove_file(paths.sync());
        assert_eq!(output_offset(&paths), 0);
        let _ = std::fs::remove_dir_all(&paths.folder);
    }

    #[test]
    fn a_recording_with_no_layer_has_no_captured_cursor() {
        let dir = std::env::temp_dir().join(format!("tcursor-curlayer-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(cursor_layer(dir.to_string_lossy().into_owned()).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
