use crate::actions::model::{ActionEvent, ActionKind, ActionLog};
use crate::ai::plan::schema::{AiRun, ClickAt};
use crate::ai::plan::transcript::{self, sh};
use crate::ai::{frames, llm, plan};
use crate::events::model::{EventKind, EventLog};
use crate::events::track::cursortype::CursorTrack;
use crate::events::track::typing::TypingLog;
use crate::export::coordmap::to_frame;
use crate::session::paths::ProjectPaths;

pub fn propose(paths: &ProjectPaths, model: Option<String>) -> Result<AiRun, String> {
    let t0 = std::time::Instant::now();
    let doc = crate::edit::seed::load_or_seed(paths);
    let dur_ms = match crate::edit::seed::true_duration_ms(paths) {
        0 => doc.trim.out_ms,
        t => t,
    };
    let log = EventLog::load(&paths.events()).map_err(|e| e.to_string())?;
    let actions = ActionLog::load(&paths.actions())
        .map(|a| a.actions)
        .unwrap_or_default();
    let cursor = CursorTrack::load(&paths.cursor());
    let typing = TypingLog::load(&paths.typing()).ms;
    let shift = crate::edit::seed::output_shift(paths);
    let transcript = transcript::serialize(&log, &actions, &cursor, &typing, dur_ms, shift);

    let installed = llm::ollama::list_models();
    let model_name = crate::ai::commands::pick_model(model, &installed)?;
    let vision = llm::vision::has_vision(&model_name);

    let clicks = click_points(&log, shift);
    let layouts = layout_times(&actions, shift);
    let images: Vec<String> = if !vision {
        Vec::new()
    } else {
        let times = frames::sample::sample_times(
            &clicks.iter().map(|c| c.t_ms).collect::<Vec<_>>(),
            &layouts,
            dur_ms,
            frames::sample::MAX_FRAMES,
        );
        frames::extract::jpegs_at(
            &paths.folder.to_string_lossy(),
            &times,
            frames::extract::LONG_EDGE,
        )
        .iter()
        .map(|(_, b)| crate::export::preview::base64_encode(b))
        .collect()
    };

    let raw = llm::ollama::chat_with_images(
        &model_name,
        &llm::prompt::system_prompt(vision),
        &transcript,
        &images,
    )?;
    let mut proposals = plan::mapping::proposals_from_json(&raw, dur_ms, &clicks);
    for p in &mut proposals {
        if p.why.is_empty() {
            p.why = plan::narrate::why_for(p.kind, p.at_ms, &log, shift);
        }
    }
    Ok(AiRun {
        model: model_name,
        vision,
        frames: images.len(),
        elapsed_ms: t0.elapsed().as_millis() as u64,
        proposals,
    })
}

fn click_points(log: &EventLog, shift: i64) -> Vec<ClickAt> {
    let (w, h) = (log.screen.w.max(1) as f32, log.screen.h.max(1) as f32);
    log.events
        .iter()
        .filter(|e| e.kind == EventKind::Down)
        .map(|e| {
            let p = to_frame(&log.screen, e.x, e.y);
            ClickAt {
                t_ms: sh(e.t, shift),
                x: (p.x as f32 / w).clamp(0.0, 1.0),
                y: (p.y as f32 / h).clamp(0.0, 1.0),
            }
        })
        .collect()
}

fn layout_times(actions: &[ActionEvent], shift: i64) -> Vec<u32> {
    actions
        .iter()
        .filter(|a| matches!(a.kind, ActionKind::SetLayout(_)))
        .map(|a| sh(a.t, shift))
        .collect()
}
