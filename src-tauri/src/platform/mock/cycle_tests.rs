use super::*;

use std::path::PathBuf;

use crate::session::record::emit::TakeHooks;
use crate::session::record::recorder::{pause_take, resume_take, start_take, Recorder, TakeSpec};
use crate::session::record::recorder_stop::stop_take;

fn recents_path() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("TCursor")
        .join("recents.json")
}

#[test]
fn a_take_starts_pauses_resumes_and_stops_against_the_mock() {
    let (platform, calls) = platform();
    let recorder = Recorder::default();
    let base = std::env::temp_dir().join("tcursor-mock-cycle");
    // INVARIANT: the stop path touches the real recents list, so put it back byte for byte.
    let recents_before = std::fs::read(recents_path()).ok();

    let spec = TakeSpec {
        base: base.clone(),
        project_name: "take".into(),
        mic_id: None,
        target_id: None,
        system_audio: false,
        game_mode: false,
    };
    let folder = start_take(spec, TakeHooks::silent(), &platform, &recorder).expect("start");
    assert!(recorder.is_recording());
    assert_eq!(
        start_take(
            TakeSpec {
                base: base.clone(),
                project_name: "second".into(),
                mic_id: None,
                target_id: None,
                system_audio: false,
                game_mode: false,
            },
            TakeHooks::silent(),
            &platform,
            &recorder,
        ),
        Err("already recording".into()),
    );

    pause_take(&recorder).expect("pause");
    assert!(recorder.is_recording());
    resume_take(&recorder).expect("resume");

    let done = stop_take(&recorder, &|| {}).expect("stop");
    assert_eq!(done.frames, 2);
    assert_eq!(done.folder, folder);
    assert!(!recorder.is_busy());
    assert_eq!(pause_take(&recorder), Err("not recording".into()));

    let seen = calls.lock().unwrap_or_else(|e| e.into_inner()).clone();
    assert_eq!(
        seen,
        vec![
            "pointer",
            "hotkeys",
            "cursor_shapes",
            "start:primary",
            "stop"
        ],
    );

    let folder = PathBuf::from(&folder);
    for name in ["events.json", "sync.json", "project.tcursor"] {
        assert!(folder.join(name).is_file(), "{name} was not written");
    }
    let _ = std::fs::remove_dir_all(&base);
    match recents_before {
        Some(b) => std::fs::write(recents_path(), b).expect("restore recents"),
        None => {
            let _ = std::fs::remove_file(recents_path());
        }
    }
}
