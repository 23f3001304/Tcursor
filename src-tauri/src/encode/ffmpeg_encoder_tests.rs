use super::*;
use crate::domain::time::Timestamp;

fn frame(width: u32, height: u32) -> Frame {
    Frame {
        width,
        height,
        bgra: vec![1, 2, 3, 4],
        ts: Timestamp(0),
    }
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
    assert!(!write_or_skip(&mut buf, &frame(3, 3), (2, 2), &mut warned).unwrap());
    assert!(buf.is_empty());
    assert!(warned);
}

#[test]
fn recovers_and_writes_once_dimensions_match_again() {
    let mut buf: Vec<u8> = Vec::new();
    let mut warned = false;
    assert!(!write_or_skip(&mut buf, &frame(3, 3), (2, 2), &mut warned).unwrap());
    assert!(write_or_skip(&mut buf, &frame(2, 2), (2, 2), &mut warned).unwrap());
    assert_eq!(buf, vec![1, 2, 3, 4]);
}
