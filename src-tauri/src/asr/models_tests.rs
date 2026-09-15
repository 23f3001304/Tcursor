use super::{find, is_installed, model_dir, model_path, resolve_model, url_for, MODELS};
use crate::settings::captions::CaptionStyle;

#[test]
fn every_entry_has_a_64_hex_digest_a_plausible_size_and_a_unique_id() {
    let mut ids: Vec<&str> = MODELS.iter().map(|m| m.id).collect();
    ids.sort_unstable();
    let n = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), n, "model ids must be unique");
    for m in MODELS.iter() {
        assert_eq!(m.sha256.len(), 64, "{}", m.id);
        assert!(
            m.sha256
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "{}",
            m.id
        );
        assert!(
            m.bytes > 50_000_000 && m.bytes < 1_000_000_000,
            "{} size {}",
            m.id,
            m.bytes
        );
        assert!(
            m.file.starts_with("ggml-") && m.file.ends_with(".bin"),
            "{}",
            m.id
        );
    }
}

#[test]
fn the_table_carries_the_four_models_the_spec_names() {
    assert_eq!(find("base.en").unwrap().bytes, 147_964_211);
    assert_eq!(find("small.en").unwrap().bytes, 487_614_201);
    assert_eq!(find("tiny.en").unwrap().bytes, 77_704_715);
    assert!(find("base").unwrap().multilingual);
    assert!(!find("base.en").unwrap().multilingual);
    assert!(find("nonsense").is_none());
}

#[test]
fn models_live_under_the_apps_own_data_dir() {
    let d = model_dir();
    assert!(
        d.ends_with("models/whisper") || d.ends_with("models\\whisper"),
        "{d:?}"
    );
    assert!(d.to_string_lossy().contains("TCursor"));
    assert!(model_path("base.en").ends_with("ggml-base.en.bin"));
}

#[test]
fn the_download_url_is_the_pinned_whisper_cpp_repo() {
    assert_eq!(
        url_for(find("base.en").unwrap()),
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin"
    );
}

#[test]
fn auto_language_needs_a_multilingual_model_and_says_so() {
    let en_only = CaptionStyle {
        language: "auto".into(),
        model: "base.en".into(),
        ..Default::default()
    };
    let err = resolve_model(&en_only).unwrap_err();
    let multilingual = find("base").unwrap().label;
    assert!(
        err.contains(multilingual),
        "the message must name the model to switch to: {err}"
    );
    let multi = CaptionStyle {
        language: "auto".into(),
        model: "base".into(),
        ..Default::default()
    };
    assert_eq!(resolve_model(&multi).unwrap().id, "base");
    let plain = CaptionStyle::default();
    assert_eq!(resolve_model(&plain).unwrap().id, "base.en");
}

#[test]
fn an_unknown_model_id_is_an_error_not_a_silent_default() {
    let s = CaptionStyle {
        model: "medium.en".into(),
        ..Default::default()
    };
    assert!(resolve_model(&s).is_err());
}

#[test]
fn installed_means_a_known_id_whose_file_is_really_on_disk() {
    assert!(
        !is_installed("nonsense"),
        "an unknown id can never be installed"
    );
    if !model_path("base.en").exists() {
        eprintln!("SKIP installed_means_a_known_id_whose_file_is_really_on_disk: ggml-base.en.bin not installed (run the Captions panel's download once)");
        return;
    }
    assert!(is_installed("base.en"));
}
