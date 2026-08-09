// Split from ffmpeg_encoder.rs per repo convention (#[path] sibling test module).
// Exercises `write_or_skip` directly against a `Vec<u8>` writer: `FfmpegFrameSink` itself
// always owns a real spawned `ffmpeg` child process, so its dimension guard is tested at
// the level it was extracted to instead (push() is a one-line pass-through onto this fn).
use super::*;
use crate::domain::time::Timestamp;

fn frame(width: u32, height: u32) -> Frame {
    Frame { width, height, bgra: vec![1, 2, 3, 4], ts: Timestamp(0) }
}

#[test]
fn writes_matching_frame_and_does_not_warn() {
    let mut buf: Vec<u8> = Vec::new();
    let mut warned = false;
    let written = write_or_skip(&mut buf, &frame(2, 2), (2, 2), &mut warned).unwrap();
    assert!(written);
    assert_eq!(buf, vec![1, 2, 3, 4]);
    assert!(!warned);
}

#[test]
fn skips_mismatched_frame_with_ok_false_and_no_write() {
    let mut buf: Vec<u8> = Vec::new();
    let mut warned = false;
    let result = write_or_skip(&mut buf, &frame(3, 3), (2, 2), &mut warned);
    assert_eq!(result.unwrap(), false);
    assert!(buf.is_empty());
    assert!(warned);
}

#[test]
fn warned_flag_latches_so_repeated_mismatches_only_flag_once() {
    let mut buf: Vec<u8> = Vec::new();
    let mut warned = false;
    assert!(!write_or_skip(&mut buf, &frame(3, 3), (2, 2), &mut warned).unwrap());
    assert!(warned);
    // Second mismatch: still Ok(false), still no write, and warned stays latched (the
    // eprintln! guard - `if !*warned` - never fires again after the first time).
    assert!(!write_or_skip(&mut buf, &frame(3, 3), (2, 2), &mut warned).unwrap());
    assert!(buf.is_empty());
    assert!(warned);
}

#[test]
fn recovers_and_writes_once_dimensions_match_again() {
    let mut buf: Vec<u8> = Vec::new();
    let mut warned = false;
    assert!(!write_or_skip(&mut buf, &frame(3, 3), (2, 2), &mut warned).unwrap()); // skipped
    assert!(write_or_skip(&mut buf, &frame(2, 2), (2, 2), &mut warned).unwrap()); // written
    assert_eq!(buf, vec![1, 2, 3, 4]);
}
