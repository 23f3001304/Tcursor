use super::{default_threads, drop_silent, transcribe, AsrParams, SILENCE_RMS};
use crate::asr::models;
use crate::asr::words::RawToken;

#[test]
fn a_token_over_silence_is_dropped_and_one_over_sound_is_kept() {
    let mut pcm = vec![0.0f32; 16_000 * 2];
    for (i, x) in pcm.iter_mut().enumerate().skip(16_000) {
        *x = 0.2 * ((i as f32) * 0.05).sin();
    }
    let tok = |text: &str, t0_cs: i64, t1_cs: i64| RawToken {
        text: text.into(),
        t0_cs,
        t1_cs,
    };
    let kept = drop_silent(
        vec![
            tok(" you", 0, 99),
            tok(" hello", 100, 199),
            tok(" tail", 190, 190),
        ],
        &pcm,
    );
    let texts: Vec<&str> = kept.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(
        texts,
        [" hello", " tail"],
        "silence dropped, sound and a zero-width token kept"
    );
    assert!(SILENCE_RMS > 0.0 && SILENCE_RMS < 0.01);
}

fn model_or_skip(name: &str) -> Option<std::path::PathBuf> {
    let p = models::model_path("base.en");
    if p.exists() {
        return Some(p);
    }
    eprintln!(
        "SKIP {name}: ggml-base.en.bin not installed (run the Captions panel's download once)"
    );
    None
}

#[test]
fn one_second_of_silence_transcribes_to_no_usable_tokens() {
    let Some(model_path) = model_or_skip("one_second_of_silence_transcribes_to_no_usable_tokens")
    else {
        return;
    };
    let pcm = vec![0.0f32; 16_000];
    let params = AsrParams {
        model_path,
        language: "en".into(),
        threads: 2,
        vad_model: Some(models::vad_path()).filter(|p| p.exists()),
    };
    let toks = transcribe(&pcm, &params, &|_| {}, &|| false).expect("silence must not be an error");
    let words = crate::asr::words::words_from_tokens(&toks);
    assert!(
        words.is_empty()
            || words
                .iter()
                .all(|w| w.text.chars().all(|c| !c.is_alphanumeric())),
        "silence produced {words:?}"
    );
}

#[test]
fn a_cancel_returns_promptly_instead_of_running_to_completion() {
    let Some(model_path) =
        model_or_skip("a_cancel_returns_promptly_instead_of_running_to_completion")
    else {
        return;
    };
    let pcm = vec![0.0f32; 16_000 * 30];
    let params = AsrParams {
        model_path,
        language: "en".into(),
        threads: 2,
        vad_model: Some(models::vad_path()).filter(|p| p.exists()),
    };
    let t = std::time::Instant::now();
    let _ = transcribe(&pcm, &params, &|_| {}, &|| true);
    assert!(
        t.elapsed().as_secs() < 20,
        "cancel must abort the decode, took {:?}",
        t.elapsed()
    );
}

#[test]
fn the_thread_count_is_capped_so_a_transcription_never_takes_the_whole_machine() {
    let n = default_threads();
    assert!((1..=8).contains(&n), "threads out of range: {n}");
}
