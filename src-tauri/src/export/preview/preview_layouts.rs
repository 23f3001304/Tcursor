//! Tauri command returning ALL layout presets' panel rects (fractions of output) + alpha, so the
//! editor preview can cross-fade between layouts on its own (mirroring LayoutTrack::scene_at)
//! instead of only ever showing one static layout. Also returns each DOC SEGMENT's own resolved
//! rects (`segs`, T34) for a segment that carries an arrangement, so a posed segment previews its
//! real panels instead of its provenance preset's.
use crate::actions::model::LayoutId;
use crate::export::preview::{with_warm, PreviewSession};
use crate::export::scene::Panel;

/// `ring_px` is normalized like `radius` (fraction of output width); `ring_color` is only
/// meaningful when `ring_px > 0` (0/`[0,0,0]` on the screen panel, which never has a ring).
#[derive(serde::Serialize)]
pub struct PanelRectDto { pub rect: [f32; 4], pub radius: f32, pub alpha: f32, pub ring_px: f32, pub ring_color: [u8; 3] }

/// `arrangement` is the SAME preset expressed as poses (`scene::arrangement::arrangement_of_preset`)
/// - what the editor writes into a `SetArrangement` op to turn this preset into a directly
/// manipulable arrangement. It rides on this existing response rather than a second command: the
/// derivation is a pure function of the very `Scene` this call already resolves, so a dedicated
/// `arrangement_of` command would re-resolve the same thing behind another `with_warm` round trip.
#[derive(serde::Serialize)]
pub struct LayoutPresetDto { pub screen: PanelRectDto, pub cam: PanelRectDto,
    pub arrangement: crate::edit::model::Arrangement }

/// One `EditDoc.layout` segment's resolved panels, by id. `None` on a field means "this segment
/// doesn't override that panel - fall back to its `layout` preset", which is every field on a
/// segment with no `arrangement` at all; a segment WITH one always resolves both fields (`Some`),
/// since `resolve_arrangement` always returns a full scene (a hidden posed panel still gets a
/// real rect, just `alpha: 0`, per L1's design). Resolved via `FrameRenderer::resolve_seg`, the
/// exact function `LayoutTrack` uses per segment when the export runs - no pose math lives here.
#[derive(serde::Serialize)]
pub struct SegRectDto { pub id: String, pub screen: Option<PanelRectDto>, pub cam: Option<PanelRectDto> }

#[derive(serde::Serialize)]
pub struct LayoutPresets {
    pub screen: LayoutPresetDto, pub camera: LayoutPresetDto, pub presenter: LayoutPresetDto,
    pub screen_only: LayoutPresetDto, pub camera_only: LayoutPresetDto,
    pub segs: Vec<SegRectDto>,
}

fn panel_dto(p: &Panel, ow: f32, oh: f32) -> PanelRectDto {
    PanelRectDto { rect: [p.rect.x / ow, p.rect.y / oh, p.rect.w / ow, p.rect.h / oh],
        radius: p.radius / ow, alpha: p.alpha, ring_px: p.ring_px / ow, ring_color: p.ring_color }
}

/// One segment's `SegRectDto` - pure and independently testable (no warm renderer needed): a
/// segment with no `arrangement` never calls `scene` at all (`FnOnce`, invoked at most once, only
/// in the `Some` branch), so a doc full of plain-preset segments costs nothing extra per fetch.
fn seg_rect_dto(seg: &crate::edit::model::LayoutSeg, scene: impl FnOnce() -> crate::export::scene::Scene,
    ow: f32, oh: f32) -> SegRectDto {
    match &seg.arrangement {
        None => SegRectDto { id: seg.id.clone(), screen: None, cam: None },
        Some(_) => {
            let s = scene();
            SegRectDto { id: seg.id.clone(), screen: Some(panel_dto(&s.screen, ow, oh)), cam: Some(panel_dto(&s.camera, ow, oh)) }
        }
    }
}

/// All 5 layout presets' panel rects + alpha, plus each doc segment's own resolved rects when it
/// carries an arrangement, in one call - so the editor preview can resolve and cross-fade between
/// segments itself (matching `LayoutTrack::scene_at`, posed or not) instead of only ever showing
/// a segment's provenance preset. Reuses the warm renderer cache, so it is `async` +
/// `spawn_blocking` like every other `with_warm` command (see `preview_frame`): a cold cache runs
/// `FrameRenderer::new`, and this is one of six such calls the editor fires on the same mount
/// tick - as sync commands they queued on the main thread behind that build.
#[tauri::command]
pub async fn preview_layouts(folder: String, app: tauri::AppHandle) -> Result<LayoutPresets, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        with_warm(&app.state::<PreviewSession>(), &folder, |c, paths| {
            let (ow, oh) = (c.meta.out_w as f32, c.meta.out_h as f32);
            let one = |id: LayoutId| -> LayoutPresetDto {
                let s = c.renderer.resolve_layout(id);
                LayoutPresetDto { screen: panel_dto(&s.screen, ow, oh), cam: panel_dto(&s.camera, ow, oh),
                    arrangement: crate::export::scene::arrangement::arrangement_of_preset(&s, ow, oh) }
            };
            // `load_or_seed` is self-locking (see its own doc comment) - safe to call again here
            // even though `with_warm`'s own `build`/`reuse` already called it once this same
            // warm-up: once the doc is current (the common, warm-cache case) it is a pure read,
            // and nothing here needs a second lock of its own.
            let doc = crate::edit::seed::load_or_seed(paths);
            let segs = doc.layout.iter().map(|seg| seg_rect_dto(seg, || c.renderer.resolve_seg(seg), ow, oh)).collect();
            Ok(LayoutPresets {
                screen: one(LayoutId::Screen), camera: one(LayoutId::Camera), presenter: one(LayoutId::Presenter),
                screen_only: one(LayoutId::ScreenOnly), camera_only: one(LayoutId::CameraOnly),
                segs,
            })
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "preview_layouts_tests.rs"]
mod tests;
