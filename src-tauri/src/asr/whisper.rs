use crate::asr::words::RawToken;
use std::ffi::{c_int, c_void};
use std::path::PathBuf;
use std::sync::Mutex;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperSysContext,
    WhisperSysState, WhisperVadParams,
};

pub struct AsrParams {
    pub model_path: PathBuf,
    pub language: String,
    pub threads: i32,
    pub vad_model: Option<PathBuf>,
}

pub fn default_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4) as i32
}

// INVARIANT: one whisper run per process at a time. ggml's Vulkan backend keeps global state, and
// two contexts initialised concurrently crash with STATUS_ACCESS_VIOLATION (seen in the tests).
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

pub fn transcribe(
    pcm: &[f32],
    p: &AsrParams,
    on_pct: &dyn Fn(u32),
    cancel: &dyn Fn() -> bool,
) -> Result<Vec<RawToken>, String> {
    let _serial = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = WhisperContext::new_with_params(&p.model_path, WhisperContextParameters::default())
        .map_err(|e| format!("could not load the caption model: {e}"))?;
    let mut state = ctx
        .create_state()
        .map_err(|e| format!("could not start the transcriber: {e}"))?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_n_threads(p.threads.max(1));
    params.set_language(if p.language == "auto" {
        None
    } else {
        Some(&p.language)
    });
    params.set_translate(false);
    params.set_token_timestamps(true);
    if let Some(vad) = p.vad_model.as_ref().and_then(|v| v.to_str()) {
        params.set_vad_model_path(Some(vad));
        params.set_vad_params(WhisperVadParams::default());
        params.enable_vad(true);
    }
    for quiet in [
        FullParams::set_print_progress,
        FullParams::set_print_special,
        FullParams::set_print_realtime,
        FullParams::set_print_timestamps,
    ] {
        quiet(&mut params, false);
    }
    // INVARIANT: `hooks` lives on this frame and outlives `state.full`, the only caller of the two
    // trampolines. whisper-rs 0.16's `set_abort_callback_safe` is not used on purpose: it stores a
    // boxed closure but instantiates its trampoline with the unboxed type, so the first abort poll
    // inside `whisper_full` jumps through a garbage pointer (STATUS_ACCESS_VIOLATION).
    let hooks = Hooks { on_pct, cancel };
    unsafe {
        params.set_progress_callback(Some(progress_trampoline));
        params.set_progress_callback_user_data(&hooks as *const Hooks as *mut c_void);
        params.set_abort_callback(Some(abort_trampoline));
        params.set_abort_callback_user_data(&hooks as *const Hooks as *mut c_void);
    }
    state
        .full(params, pcm)
        .map_err(|e| format!("transcription failed: {e}"))?;
    Ok(drop_silent(collect_tokens(&state), pcm))
}

pub const SILENCE_RMS: f32 = 0.001;

pub fn drop_silent(tokens: Vec<RawToken>, pcm: &[f32]) -> Vec<RawToken> {
    let rms = |a: usize, b: usize| {
        let s = &pcm[a.min(pcm.len())..b.min(pcm.len())];
        if s.is_empty() {
            return 0.0;
        }
        (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
    };
    tokens
        .into_iter()
        .filter(|t| {
            let (a, b) = (t.t0_cs.max(0) as usize * 160, t.t1_cs.max(0) as usize * 160);
            b <= a || rms(a, b) >= SILENCE_RMS
        })
        .collect()
}

struct Hooks<'a> {
    on_pct: &'a dyn Fn(u32),
    cancel: &'a dyn Fn() -> bool,
}

unsafe extern "C" fn progress_trampoline(
    _: *mut WhisperSysContext,
    _: *mut WhisperSysState,
    pct: c_int,
    user_data: *mut c_void,
) {
    let hooks = &*(user_data as *const Hooks);
    (hooks.on_pct)(pct.clamp(0, 100) as u32);
}

unsafe extern "C" fn abort_trampoline(user_data: *mut c_void) -> bool {
    let hooks = &*(user_data as *const Hooks);
    (hooks.cancel)()
}

fn collect_tokens(state: &whisper_rs::WhisperState) -> Vec<RawToken> {
    let mut out = Vec::new();
    for i in 0..state.full_n_segments() {
        let Some(seg) = state.get_segment(i) else {
            continue;
        };
        let (s0, span, n) = (
            seg.start_timestamp(),
            (seg.end_timestamp() - seg.start_timestamp()).max(0),
            seg.n_tokens().max(1) as i64,
        );
        for j in 0..seg.n_tokens() {
            let Some(tok) = seg.get_token(j) else {
                continue;
            };
            let Ok(text) = tok.to_str_lossy() else {
                continue;
            };
            let d = tok.token_data();
            let (t0_cs, t1_cs) = if d.t1 > d.t0 {
                (d.t0, d.t1)
            } else {
                (s0 + span * j as i64 / n, s0 + span * (j as i64 + 1) / n)
            };
            out.push(RawToken {
                text: text.into_owned(),
                t0_cs,
                t1_cs,
            });
        }
    }
    out
}

#[cfg(test)]
#[path = "whisper_tests.rs"]
mod tests;
