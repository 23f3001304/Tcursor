use crate::export::pipeline::ffio::RawDecoder;
use crate::export::remap::TimeMap;
use crate::export::render::{
    screen_mix, FramePose, FrameRenderer, RenderMeta, OUT_FPS, OUT_STEP_MS,
};
use crate::session::paths::ProjectPaths;
use anyhow::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewAt {
    Clip(u32),
    Out(u32),
}

pub fn at_instants(map: &TimeMap, at: PreviewAt) -> (u32, u32) {
    match at {
        PreviewAt::Clip(t) => (map.out_of(t), t),
        PreviewAt::Out(o) => (o, map.clip_of(o)),
    }
}

fn latched_ms(map: &TimeMap, prev_out_ms: u32, fps: u64) -> Option<u32> {
    let plan = map.frame_plan(fps);
    let last = (plan.len() as u64).checked_sub(1)?;
    let j = (prev_out_ms as u64 * fps / 1000).min(last) as usize;
    Some((plan[j] * 1000 / fps) as u32)
}

pub(crate) fn walk_to(r: &mut FrameRenderer, video_start: u64, out_ms: u32) -> FramePose {
    let map = r.time_map().clone();
    let plan = map.frame_plan(OUT_FPS);
    if plan.is_empty() {
        return r.step_camera(video_start, 0, OUT_STEP_MS);
    }
    let j_target = (out_ms as u64 * OUT_FPS / 1000).min(plan.len() as u64 - 1) as usize;
    r.walk_plan(
        video_start,
        OUT_FPS,
        &plan,
        j_target,
        OUT_STEP_MS,
        |_, _, _, _| true,
    )
    .expect("plan is non-empty")
}

fn decode_screen(
    paths: &ProjectPaths,
    meta: &RenderMeta,
    time_ms: u32,
    buf: &mut [u8],
) -> Result<()> {
    let mut dec = RawDecoder::spawn(
        &paths.video(),
        0.0,
        false,
        Some(time_ms as u64),
        meta.screen_crop,
        None,
        None,
        "nv12",
        meta.screen_bytes,
    )?;
    if !dec.read_frame(buf)? {
        anyhow::bail!("no screen frame at {time_ms}ms (past end of video)");
    }
    Ok(())
}

fn clip_dissolve(
    map: &TimeMap,
    meta: &RenderMeta,
    paths: &ProjectPaths,
    pose: &FramePose,
    cur: &[u8],
) -> Option<Vec<u8>> {
    let m = pose.clip_mix?;
    let prev_ms = latched_ms(map, m.prev_out_ms, OUT_FPS)?;
    let mut prev = vec![0u8; meta.screen_bytes];
    if let Err(e) = decode_screen(paths, meta, prev_ms, &mut prev) {
        eprintln!("[PREVIEW] outgoing clip frame at {prev_ms}ms: {e}");
        return None;
    }
    let mut mixed = Vec::new();
    screen_mix::blend_into(
        &mut mixed,
        cur,
        &prev,
        meta.sw,
        meta.sh,
        pose.scene.src,
        pose.scene.src,
        m.alpha,
    );
    Some(mixed)
}

pub(super) fn composite_frame(
    renderer: &mut FrameRenderer,
    meta: &RenderMeta,
    paths: &ProjectPaths,
    at: PreviewAt,
) -> Result<Vec<u8>> {
    renderer.reset_camera();
    let (out_ms, time_ms) = at_instants(renderer.time_map(), at);
    let pose = walk_to(renderer, meta.video_start, out_ms);

    let mut screen_buf = vec![0u8; meta.screen_bytes];
    decode_screen(paths, meta, time_ms, &mut screen_buf)?;
    if let Some(mixed) = clip_dissolve(renderer.time_map(), meta, paths, &pose, &screen_buf) {
        screen_buf = mixed;
    }

    let prev = pose.mix.and_then(|m| {
        let clip = renderer.time_map().clip_of(m.hold_ms);
        let mut b = vec![0u8; meta.screen_bytes];
        decode_screen(paths, meta, clip, &mut b).ok().map(|()| b)
    });

    let wc_dims = (meta.webcam_w, meta.webcam_h);
    let wc_bytes = (wc_dims.0 * wc_dims.1 * 4) as usize;
    let webcam: Option<(Vec<u8>, u32, u32)> = if paths.webcam().exists() {
        let mut buf = vec![0u8; wc_bytes];
        let mut wc_dec = RawDecoder::spawn(
            &paths.webcam(),
            OUT_FPS as f64,
            false,
            Some(meta.video_start + time_ms as u64),
            None,
            Some(wc_dims),
            None,
            "bgra",
            wc_bytes,
        )?;
        if let Err(e) = wc_dec.read_frame(&mut buf) {
            eprintln!("[PREVIEW] webcam frame at {time_ms}ms: {e}");
        }
        drop(wc_dec);
        Some((buf, wc_dims.0, wc_dims.1))
    } else {
        None
    };

    let wc_ref = webcam.as_ref().map(|(b, w, h)| (b.as_slice(), *w, *h));
    let mut bgra = Vec::new();
    renderer.composite_at(&pose, &screen_buf, prev.as_deref(), wc_ref, &mut bgra);
    Ok(bgra)
}

#[cfg(test)]
#[path = "compose_tests.rs"]
mod tests;
