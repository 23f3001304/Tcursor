//! Tauri command returning ALL layout presets' panel rects (fractions of output) + alpha, so the
//! editor preview can cross-fade between layouts on its own (mirroring LayoutTrack::scene_at)
//! instead of only ever showing one static layout.
use crate::actions::model::LayoutId;
use crate::export::preview::{with_warm, PreviewSession};
use crate::export::scene::Panel;

/// `ring_px` is normalized like `radius` (fraction of output width); `ring_color` is only
/// meaningful when `ring_px > 0` (0/`[0,0,0]` on the screen panel, which never has a ring).
#[derive(serde::Serialize)]
pub struct PanelRectDto { pub rect: [f32; 4], pub radius: f32, pub alpha: f32, pub ring_px: f32, pub ring_color: [u8; 3] }

#[derive(serde::Serialize)]
pub struct LayoutPresetDto { pub screen: PanelRectDto, pub cam: PanelRectDto }

#[derive(serde::Serialize)]
pub struct LayoutPresets {
    pub screen: LayoutPresetDto, pub camera: LayoutPresetDto, pub presenter: LayoutPresetDto,
    pub screen_only: LayoutPresetDto, pub camera_only: LayoutPresetDto,
}

fn panel_dto(p: &Panel, ow: f32, oh: f32) -> PanelRectDto {
    PanelRectDto { rect: [p.rect.x / ow, p.rect.y / oh, p.rect.w / ow, p.rect.h / oh],
        radius: p.radius / ow, alpha: p.alpha, ring_px: p.ring_px / ow, ring_color: p.ring_color }
}

/// All 5 layout presets' panel rects + alpha in one call, so the editor preview can resolve and
/// cross-fade between presets itself (matching `LayoutTrack::scene_at`) instead of only ever
/// showing the single static layout `preview_layout` returns. Reuses the warm renderer cache.
#[tauri::command]
pub fn preview_layouts(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<LayoutPresets, String> {
    with_warm(&session, &folder, |c, _paths| {
        let (ow, oh) = (c.meta.out_w as f32, c.meta.out_h as f32);
        let one = |id: LayoutId| -> LayoutPresetDto {
            let s = c.renderer.resolve_layout(id);
            LayoutPresetDto { screen: panel_dto(&s.screen, ow, oh), cam: panel_dto(&s.camera, ow, oh) }
        };
        Ok(LayoutPresets {
            screen: one(LayoutId::Screen), camera: one(LayoutId::Camera), presenter: one(LayoutId::Presenter),
            screen_only: one(LayoutId::ScreenOnly), camera_only: one(LayoutId::CameraOnly),
        })
    })
}
