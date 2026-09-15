use crate::export::preview::with_warm_app;
use crate::export::render::{OUT_FPS, OUT_STEP_MS};
use crate::process::proc::ffcmd_bg;
use crate::session::paths::ProjectPaths;
use std::io::BufRead;
use std::path::PathBuf;

#[derive(serde::Serialize)]
pub struct CamSample {
    pub t: u32,
    pub scale: f32,
    pub cx: f32,
    pub cy: f32,
    pub curx: f32,
    pub cury: f32,
}

#[tauri::command]
pub async fn camera_track(folder: String, app: tauri::AppHandle) -> Result<Vec<CamSample>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        with_warm_app(&app, &folder, |c, _paths| {
            let vs = c.meta.video_start;
            c.renderer.reset_camera();
            let plan = c.renderer.time_map().frame_plan(OUT_FPS);
            let mut out = Vec::with_capacity(plan.len());
            c.renderer.walk_plan(
                vs,
                OUT_FPS,
                &plan,
                plan.len(),
                OUT_STEP_MS,
                |_, j, _, pose| {
                    let r = pose.scene.screen.rect;
                    let rel = |vx: f32, vy: f32| {
                        (
                            if r.w > 0.0 {
                                ((vx - r.x) / r.w).clamp(0.0, 1.0)
                            } else {
                                0.5
                            },
                            if r.h > 0.0 {
                                ((vy - r.y) / r.h).clamp(0.0, 1.0)
                            } else {
                                0.5
                            },
                        )
                    };
                    let (cx, cy) = rel(pose.cam.cx, pose.cam.cy);
                    let (curx, cury) = rel(pose.cur.x as f32, pose.cur.y as f32);
                    out.push(CamSample {
                        t: (j as u64 * 1000 / OUT_FPS) as u32,
                        scale: pose.cam.scale,
                        cx,
                        cy,
                        curx,
                        cury,
                    });
                    true
                },
            );
            Ok(out)
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(serde::Serialize)]
pub struct PreviewLayout {
    pub screen: [f32; 4],
    pub radius: f32,
    pub cam: Option<[f32; 9]>,
    pub canvas: [u32; 2],
}

#[tauri::command]
pub async fn preview_layout(
    folder: String,
    app: tauri::AppHandle,
) -> Result<PreviewLayout, String> {
    tauri::async_runtime::spawn_blocking(move || {
        with_warm_app(&app, &folder, |c, _paths| {
            c.renderer.reset_camera();
            let pose = c.renderer.step_camera(c.meta.video_start, 0, OUT_STEP_MS);
            let (ow, oh) = (c.meta.out_w as f32, c.meta.out_h as f32);
            let s = pose.scene.screen.rect;
            let cam = if pose.scene.camera.alpha > 0.5 {
                let cp = pose.scene.camera;
                let [rr, rg, rb] = cp.ring_color;
                Some([
                    cp.rect.x / ow,
                    cp.rect.y / oh,
                    cp.rect.w / ow,
                    cp.rect.h / oh,
                    cp.radius / ow,
                    cp.ring_px / ow,
                    rr as f32,
                    rg as f32,
                    rb as f32,
                ])
            } else {
                None
            };
            Ok(PreviewLayout {
                screen: [s.x / ow, s.y / oh, s.w / ow, s.h / oh],
                radius: pose.scene.screen.radius / ow,
                cam,
                canvas: [c.meta.out_w, c.meta.out_h],
            })
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(serde::Serialize)]
pub struct ClickSample {
    pub t: u32,
    pub x: f32,
    pub y: f32,
}

#[tauri::command]
pub async fn click_track(
    folder: String,
    app: tauri::AppHandle,
) -> Result<Vec<ClickSample>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        with_warm_app(&app, &folder, |c, _paths| {
            Ok(c.renderer
                .click_track(c.meta.video_start)
                .into_iter()
                .map(|(t, x, y)| ClickSample { t, x, y })
                .collect())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ensure_proxy(folder: String, height: u32) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || ensure_proxy_blocking(folder, height))
        .await
        .map_err(|e| e.to_string())?
}

pub(crate) fn ensure_proxy_blocking(folder: String, height: u32) -> Result<String, String> {
    ensure_proxy_with_progress(folder, height, &|_| {})
}

pub(crate) fn ensure_proxy_with_progress(
    folder: String,
    height: u32,
    on_progress: &dyn Fn(u32),
) -> Result<String, String> {
    let paths = ProjectPaths {
        folder: PathBuf::from(&folder),
    };
    let h = height.clamp(240, 2160) & !1;
    let proxy = paths.folder.join(format!("preview_{h}_rt.mp4"));
    crate::process::proc::generate_once(&proxy, || {
        let real = (crate::edit::seed::true_duration_ms(&paths) as f64 / 1000.0).max(0.05);
        let probed = crate::export::pipeline::ffio::probe_duration(&paths.video()).unwrap_or(0.0);
        let enc = if probed > 0.05 { probed } else { real };
        let k = real / enc;
        let vf = if (k - 1.0).abs() > 0.02 {
            format!("scale=-2:{h},setpts={k:.6}*PTS")
        } else {
            format!("scale=-2:{h}")
        };
        let tmp = crate::process::proc::tmp_sibling(&proxy);
        let mut child = ffcmd_bg("ffmpeg")
            .args(["-v", "error", "-nostats", "-progress", "pipe:1", "-y", "-i"])
            .arg(paths.video())
            .args([
                "-vf",
                &vf,
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-g",
                "60",
                "-crf",
                "27",
                "-pix_fmt",
                "yuv420p",
                "-movflags",
                "+faststart",
                "-an",
            ])
            .arg(&tmp)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        if let Some(out) = child.stdout.take() {
            for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
                if let Some(pct) = proxy_pct(&line, real) {
                    on_progress(pct);
                }
            }
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            let _ = std::fs::remove_file(&tmp);
            return Err("preview proxy transcode failed".into());
        }
        std::fs::rename(&tmp, &proxy).map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(proxy.to_string_lossy().to_string())
}

pub(crate) fn proxy_pct(line: &str, real_secs: f64) -> Option<u32> {
    let us: f64 = line.strip_prefix("out_time_us=")?.trim().parse().ok()?;
    Some(((us / 1e6 / real_secs.max(0.05)) * 100.0).clamp(0.0, 99.0) as u32)
}
