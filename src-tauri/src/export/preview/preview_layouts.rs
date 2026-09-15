use crate::actions::model::LayoutId;
use crate::export::preview::with_warm_app;
use crate::export::scene::Panel;

#[derive(serde::Serialize)]
pub struct PanelRectDto {
    pub rect: [f32; 4],
    pub radius: f32,
    pub alpha: f32,
    pub ring_px: f32,
    pub ring_color: [u8; 3],
}

#[derive(serde::Serialize)]
pub struct LayoutPresetDto {
    pub screen: PanelRectDto,
    pub cam: PanelRectDto,
    pub arrangement: crate::edit::model::Arrangement,
}

#[derive(serde::Serialize)]
pub struct SegRectDto {
    pub id: String,
    pub screen: Option<PanelRectDto>,
    pub cam: Option<PanelRectDto>,
}

#[derive(serde::Serialize)]
pub struct SourceSpanDto {
    pub start_ms: u32,
    pub src: [f32; 4],
    pub transition_ms: u32,
    pub fit: [f32; 2],
}

#[derive(serde::Serialize)]
pub struct LayoutPresets {
    pub screen: LayoutPresetDto,
    pub camera: LayoutPresetDto,
    pub presenter: LayoutPresetDto,
    pub screen_only: LayoutPresetDto,
    pub camera_only: LayoutPresetDto,
    pub segs: Vec<SegRectDto>,
    pub spans: Vec<SourceSpanDto>,
    pub inset_w: f32,
}

fn panel_dto(p: &Panel, ow: f32, oh: f32) -> PanelRectDto {
    PanelRectDto {
        rect: [p.rect.x / ow, p.rect.y / oh, p.rect.w / ow, p.rect.h / oh],
        radius: p.radius / ow,
        alpha: p.alpha,
        ring_px: p.ring_px / ow,
        ring_color: p.ring_color,
    }
}

fn seg_rect_dto(
    seg: &crate::edit::model::LayoutSeg,
    scene: impl FnOnce() -> crate::export::scene::Scene,
    ow: f32,
    oh: f32,
) -> SegRectDto {
    match &seg.arrangement {
        None => SegRectDto {
            id: seg.id.clone(),
            screen: None,
            cam: None,
        },
        Some(_) => {
            let s = scene();
            SegRectDto {
                id: seg.id.clone(),
                screen: Some(panel_dto(&s.screen, ow, oh)),
                cam: Some(panel_dto(&s.camera, ow, oh)),
            }
        }
    }
}

#[tauri::command]
pub async fn preview_layouts(
    folder: String,
    app: tauri::AppHandle,
) -> Result<LayoutPresets, String> {
    tauri::async_runtime::spawn_blocking(move || {
        with_warm_app(&app, &folder, |c, paths| {
            let (ow, oh) = (c.meta.out_w as f32, c.meta.out_h as f32);
            let one = |id: LayoutId| -> LayoutPresetDto {
                let s = c.renderer.resolve_layout(id);
                LayoutPresetDto {
                    screen: panel_dto(&s.screen, ow, oh),
                    cam: panel_dto(&s.camera, ow, oh),
                    arrangement: crate::export::scene::arrangement::arrangement_of_preset(
                        &s, ow, oh,
                    ),
                }
            };
            let doc = crate::edit::seed::load_or_seed(paths);
            let segs = doc
                .layout
                .iter()
                .map(|seg| seg_rect_dto(seg, || c.renderer.resolve_seg(seg), ow, oh))
                .collect();
            let (cw, ch) = (c.meta.sw.max(1) as f32, c.meta.sh.max(1) as f32);
            let spans = c
                .renderer
                .spans()
                .iter()
                .map(|s| {
                    let fit = c.renderer.span_fit(s.src);
                    SourceSpanDto {
                        start_ms: s.start_ms,
                        src: [s.src.x / cw, s.src.y / ch, s.src.w / cw, s.src.h / ch],
                        transition_ms: crate::export::render::spans::SWITCH_MS,
                        fit: [fit.0, fit.1],
                    }
                })
                .collect();
            Ok(LayoutPresets {
                screen: one(LayoutId::Screen),
                camera: one(LayoutId::Camera),
                presenter: one(LayoutId::Presenter),
                screen_only: one(LayoutId::ScreenOnly),
                camera_only: one(LayoutId::CameraOnly),
                segs,
                spans,
                inset_w: c.renderer.inset_w_frac(),
            })
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "preview_layouts_tests.rs"]
mod tests;
