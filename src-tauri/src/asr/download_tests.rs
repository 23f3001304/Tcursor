use super::{verify_or_remove, Resume};
use std::io::Write;

#[test]
fn resume_decides_from_the_part_length_alone() {
    assert_eq!(super::resume_from(0, 1000), Resume::Restart);
    assert_eq!(super::resume_from(400, 1000), Resume::Range(400));
    assert_eq!(super::resume_from(1000, 1000), Resume::Complete);
    assert_eq!(super::resume_from(1001, 1000), Resume::Restart);
}

#[test]
fn a_file_whose_digest_is_wrong_is_deleted_and_named_in_the_error() {
    let p = std::env::temp_dir().join(format!("tcursor-asr-bad-{}.bin", std::process::id()));
    std::fs::File::create(&p)
        .unwrap()
        .write_all(b"abc")
        .unwrap();
    let want = "0000000000000000000000000000000000000000000000000000000000000000";
    let err = verify_or_remove(&p, want).unwrap_err();
    assert!(
        err.contains("ba7816bf"),
        "the error must show what we actually got: {err}"
    );
    assert!(!p.exists(), "a corrupt download must not be left on disk");
}

#[test]
fn a_file_whose_digest_matches_is_kept() {
    let p = std::env::temp_dir().join(format!("tcursor-asr-good-{}.bin", std::process::id()));
    std::fs::File::create(&p)
        .unwrap()
        .write_all(b"abc")
        .unwrap();
    verify_or_remove(
        &p,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    )
    .unwrap();
    assert!(p.exists());
    let _ = std::fs::remove_file(&p);
}

#[test]
fn a_missing_file_is_an_error_that_names_it_rather_than_a_panic() {
    let p = std::env::temp_dir().join("tcursor-asr-not-here-at-all.bin");
    let _ = std::fs::remove_file(&p);
    let err = verify_or_remove(
        &p,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    )
    .unwrap_err();
    assert!(err.contains("tcursor-asr-not-here-at-all.bin"), "{err}");
}
