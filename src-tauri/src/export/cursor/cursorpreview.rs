use crate::events::track::cursorlayer::CursorLayer;
use crate::events::track::cursortype::{CursorTrack, CursorType};
use crate::export::cursor::draw::cursordraw::decode_sprite;
use crate::export::cursor::pack;
use crate::export::preview::{base64_encode, png_encode, with_warm_app};
use crate::session::paths::ProjectPaths;
use std::path::PathBuf;

#[derive(serde::Serialize)]
pub struct CursorSpriteDto {
    pub kind: CursorType,
    pub url: String,
    pub hot: [f32; 2],
    pub canvas_h: u32,
}

#[derive(serde::Serialize)]
pub struct CursorPackDto {
    pub sprites: Vec<CursorSpriteDto>,
    pub busy_frames: Vec<CursorSpriteDto>,
    pub busy: Option<crate::export::cursor::draw::busy::BusySpec>,
    pub material: Option<String>,
}

#[tauri::command]
pub fn cursor_sprites(
    folder: String,
    platform: tauri::State<'_, std::sync::Arc<crate::platform::Platform>>,
) -> Result<CursorPackDto, String> {
    let paths = ProjectPaths {
        folder: PathBuf::from(&folder),
    };
    let settings = crate::edit::seed::load_or_seed(&paths).settings;
    let pack_id = &settings.cursor.pack;
    let dark =
        crate::settings::theme::resolve_dark(settings.ui.theme, platform.system.os_prefers_dark())
            && pack::theme_inverts(pack_id);
    let sprites: Vec<CursorSpriteDto> = pack::sprite_sources(pack_id)
        .into_iter()
        .filter_map(|(kind, png, hot)| sprite_dto(kind, &png, hot, dark))
        .collect();
    if !sprites.iter().any(|d| d.kind == CursorType::Arrow) {
        return Err("cursor arrow sprite failed to decode".into());
    }
    let busy_frames = pack::busy_frames(pack_id)
        .iter()
        .filter_map(|png| sprite_dto(CursorType::Busy, png, (0.5, 0.5), dark))
        .collect();
    Ok(CursorPackDto {
        sprites,
        busy_frames,
        busy: pack::busy_spec(pack_id),
        material: pack::material(pack_id),
    })
}

fn sprite_dto(
    kind: CursorType,
    png: &[u8],
    hot: (f32, f32),
    dark: bool,
) -> Option<CursorSpriteDto> {
    let mut spr = decode_sprite(png, hot)?;
    if dark {
        invert_rgb(&mut spr.bgra);
    }
    let bytes = png_encode(&spr.bgra, spr.w, spr.h).ok()?;
    Some(CursorSpriteDto {
        kind,
        url: format!("data:image/png;base64,{}", base64_encode(&bytes)),
        hot: [spr.hot.0, spr.hot.1],
        canvas_h: spr.canvas_h,
    })
}

#[derive(serde::Serialize)]
pub struct CursorKindSample {
    pub t: u32,
    pub kind: CursorType,
}

#[tauri::command]
pub async fn cursor_kinds(
    folder: String,
    app: tauri::AppHandle,
) -> Result<Vec<CursorKindSample>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        with_warm_app(&app, &folder, |c, paths| {
            let off = c.renderer.events_ms() as i64 - c.meta.video_start as i64;
            Ok(CursorTrack::load(&paths.cursor())
                .samples
                .iter()
                .filter_map(|&(et, kind)| {
                    let t = et as i64 + off;
                    (t >= 0).then_some(CursorKindSample { t: t as u32, kind })
                })
                .collect())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(serde::Serialize)]
pub struct CapturedCursorDto {
    pub id: u32,
    pub w: u32,
    pub h: u32,
    pub hx: u32,
    pub hy: u32,
    pub url: String,
}

#[derive(serde::Serialize)]
pub struct CursorLayerDto {
    pub cursors: Vec<CapturedCursorDto>,
    pub track: Vec<(u32, u32)>,
    pub src_w: u32,
    pub src_h: u32,
}

#[tauri::command]
pub fn cursor_layer(folder: String) -> Option<CursorLayerDto> {
    let paths = ProjectPaths {
        folder: PathBuf::from(&folder),
    };
    let layer = CursorLayer::load(&paths)?;
    let (src_w, src_h) =
        crate::export::pipeline::ffio::probe_dims(&paths.video()).unwrap_or((0, 0));
    let off = output_offset(&paths);
    let cursors = layer
        .cursors
        .iter()
        .filter_map(|e| {
            let bytes = std::fs::read(paths.cursor_dir().join(&e.file)).ok()?;
            Some(CapturedCursorDto {
                id: e.id,
                w: e.w,
                h: e.h,
                hx: e.hx,
                hy: e.hy,
                url: format!("data:image/png;base64,{}", base64_encode(&bytes)),
            })
        })
        .collect();
    let track = layer
        .track
        .iter()
        .filter_map(|&(et, id)| {
            let t = et as i64 + off;
            (t >= 0).then_some((t as u32, id))
        })
        .collect();
    Some(CursorLayerDto {
        cursors,
        track,
        src_w,
        src_h,
    })
}

fn output_offset(paths: &ProjectPaths) -> i64 {
    match crate::session::sync::SyncLog::load(&paths.sync()) {
        Ok(s) => match s.frames.first() {
            Some(&start) => s.events_ms as i64 - start as i64,
            None => 0,
        },
        Err(_) => 0,
    }
}

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
        let dir = std::env::temp_dir().join(format!("tcursor-curoff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let paths = ProjectPaths { folder: dir };
        SyncLog {
            frames: vec![1000, 1016],
            events_ms: 900,
            mic_ms: None,
            system_ms: None,
            ..Default::default()
        }
        .save(&paths.sync())
        .unwrap();
        assert_eq!(output_offset(&paths), -100);
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
