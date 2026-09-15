use super::{pick_source, shift_words, to_output_ms, wav_for, AsrSource};
use crate::edit::captions::CaptionWord;

#[test]
fn an_audio_instant_lands_on_the_output_clock_by_adding_the_track_shift() {
    assert_eq!(to_output_ms(0, 500), Some(500));
    assert_eq!(to_output_ms(1200, 500), Some(1700));
    assert_eq!(to_output_ms(1000, -800), Some(200));
}

#[test]
fn a_word_spoken_before_the_video_started_is_dropped_and_a_straddling_one_is_clamped() {
    let words = vec![
        CaptionWord {
            start_ms: 0,
            end_ms: 300,
            text: "before".into(),
        },
        CaptionWord {
            start_ms: 600,
            end_ms: 1100,
            text: "straddles".into(),
        },
        CaptionWord {
            start_ms: 2000,
            end_ms: 2400,
            text: "after".into(),
        },
    ];
    let out = shift_words(words, -800);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].text, "straddles");
    assert_eq!(
        out[0].start_ms, 0,
        "a straddling word starts at output 0, it is not thrown away"
    );
    assert_eq!(out[0].end_ms, 300);
    assert_eq!(out[1].start_ms, 1200);
}

#[test]
fn to_output_ms_refuses_an_instant_that_is_still_negative() {
    assert_eq!(to_output_ms(100, -800), None);
}

#[test]
fn the_source_is_the_mic_when_there_is_one_and_the_system_track_otherwise() {
    let dir = std::env::temp_dir().join(format!("tcursor-asr-src-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let paths = crate::session::paths::ProjectPaths {
        folder: dir.clone(),
    };
    assert_eq!(
        pick_source(&paths),
        None,
        "a recording with no audio has nothing to transcribe"
    );
    std::fs::write(paths.system(), b"").unwrap();
    assert_eq!(pick_source(&paths), Some(AsrSource::System));
    std::fs::write(paths.mic(), b"").unwrap();
    assert_eq!(
        pick_source(&paths),
        Some(AsrSource::Mic),
        "the person talking wins over the desktop"
    );
    assert!(wav_for(&paths, AsrSource::Mic).ends_with("mic.wav"));
    assert!(wav_for(&paths, AsrSource::System).ends_with("system.wav"));
    let _ = std::fs::remove_dir_all(&dir);
}
