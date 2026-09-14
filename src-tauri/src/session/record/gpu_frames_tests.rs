//! `gpu_frames.rs`'s tests, split out under `#[path]` so the callback file itself stays under the
//! 200-line cap now that it also carries the mid-take display switch's encoder handover.
use super::*;

fn times() -> FrameTimes { Arc::new(Mutex::new(Vec::new())) }

#[test]
fn an_encoded_frame_gets_its_timestamp() {
    let ts = times();
    assert!(record_if_encoded(&ts, 1100, Ok(())).is_ok());
    assert_eq!(*ts.lock().unwrap(), vec![1100]);
}

/// A frame the encoder rejected is not in `video.mp4`, so it must not be in `sync.json`
/// either - which since M1's salvage is written even when the take failed to finalize.
#[test]
fn a_failed_send_leaves_the_timestamps_untouched() {
    let ts = times();
    record_if_encoded(&ts, 100, Ok(())).unwrap();
    let err = record_if_encoded(&ts, 200, Err(anyhow::anyhow!("encoder gone")));
    assert!(err.is_err());
    assert_eq!(*ts.lock().unwrap(), vec![100], "an unencoded frame reached sync.json");
}

/// The send's error reaches the capture handler unchanged, so `CaptureControl::stop` still
/// propagates it and the stop path still reports the take as failed.
#[test]
fn the_encode_error_is_propagated_verbatim() {
    let err = record_if_encoded(&times(), 0, Err(anyhow::anyhow!("mf sample rejected")));
    assert_eq!(err.unwrap_err().to_string(), "mf sample rejected");
}
