use super::{classify_end, decode_args};
use std::path::Path;

#[test]
fn a_clean_exit_is_end_of_stream() {
    assert_eq!(classify_end(true, "exit code: 0", 1200, "").unwrap(), false);
}

#[test]
fn a_failed_exit_errors_with_the_stderr_tail() {
    let e = classify_end(false, "exit code: 1", 0, "moov atom not found")
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("moov atom not found"),
        "stderr tail must survive into the message: {e}"
    );
    assert!(
        e.contains("0 frame(s)"),
        "frame count locates the failure: {e}"
    );
}

#[test]
fn a_failed_exit_without_stderr_is_still_an_error() {
    let e = classify_end(false, "exit code: 69", 42, "")
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("no stderr output") && e.contains("42 frame(s)"),
        "{e}"
    );
}

#[test]
fn positive_output_rate_emits_r_after_i() {
    let args = decode_args(
        Path::new("v.mp4"),
        30.0,
        false,
        None,
        None,
        None,
        None,
        "nv12",
    );
    let i = args.iter().position(|a| a == "-i").unwrap();
    let r = args.iter().position(|a| a == "-r").unwrap();
    assert!(r > i, "-r must come after -i for an output rate: {args:?}");
    assert_eq!(args[r + 1], "30.0000");
}

#[test]
fn positive_input_rate_emits_r_before_i() {
    let args = decode_args(
        Path::new("v.mp4"),
        60.0,
        true,
        None,
        None,
        None,
        None,
        "bgra",
    );
    let i = args.iter().position(|a| a == "-i").unwrap();
    let r = args.iter().position(|a| a == "-r").unwrap();
    assert!(r < i, "-r must come before -i for an input rate: {args:?}");
}

#[test]
fn non_positive_rate_omits_r_entirely() {
    let args = decode_args(
        Path::new("v.mp4"),
        0.0,
        false,
        None,
        None,
        None,
        None,
        "nv12",
    );
    assert!(!args.iter().any(|a| a == "-r"), "unexpected -r in {args:?}");
}

#[test]
fn cover_scale_emits_the_boxs_own_w_h() {
    let args = decode_args(
        Path::new("w.mp4"),
        60.0,
        false,
        None,
        None,
        Some((448, 252)),
        None,
        "bgra",
    );
    let vf = args.iter().position(|a| a == "-vf").expect("no -vf");
    assert_eq!(
        args[vf + 1],
        "scale=448:252:force_original_aspect_ratio=increase,crop=448:252"
    );
}

#[test]
fn cover_scale_of_equal_dims_is_the_old_square_filter() {
    let args = decode_args(
        Path::new("w.mp4"),
        60.0,
        false,
        None,
        None,
        Some((420, 420)),
        None,
        "bgra",
    );
    let vf = args.iter().position(|a| a == "-vf").expect("no -vf");
    assert_eq!(
        args[vf + 1],
        "scale=420:420:force_original_aspect_ratio=increase,crop=420:420"
    );
}

#[test]
fn crop_is_exact_and_comes_before_any_scale() {
    let args = decode_args(
        Path::new("v.mp4"),
        60.0,
        false,
        None,
        Some((1696, 954)),
        None,
        None,
        "nv12",
    );
    let vf = args.iter().position(|a| a == "-vf").expect("no -vf");
    assert_eq!(args[vf + 1], "crop=1696:954:0:0");
    let both = decode_args(
        Path::new("w.mp4"),
        60.0,
        false,
        None,
        Some((1696, 954)),
        None,
        Some((848, 477)),
        "nv12",
    );
    let vf = both.iter().position(|a| a == "-vf").unwrap();
    assert_eq!(
        both[vf + 1],
        "crop=1696:954:0:0,scale=848:477:flags=fast_bilinear"
    );
    let none = decode_args(
        Path::new("v.mp4"),
        60.0,
        false,
        None,
        None,
        None,
        None,
        "nv12",
    );
    assert!(
        !none.iter().any(|a| a == "-vf"),
        "no filter without crop or scale: {none:?}"
    );
}
