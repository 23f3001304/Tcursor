use super::{sha256_file, sha256_hex, Sha256};
use std::io::Write;

#[test]
fn matches_the_standard_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn streaming_in_chunks_equals_hashing_in_one_go() {
    let data: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
    let mut h = Sha256::new();
    for chunk in data.chunks(97) {
        h.update(chunk);
    }
    assert_eq!(h.finish_hex(), sha256_hex(&data));
}

#[test]
fn a_message_that_straddles_the_length_padding_boundary_is_correct() {
    for n in [55usize, 56, 64] {
        let m = vec![b'a'; n];
        let mut h = Sha256::new();
        h.update(&m);
        assert_eq!(h.finish_hex(), sha256_hex(&m), "n = {n}");
    }
}

#[test]
fn hashing_a_file_agrees_with_hashing_its_bytes() {
    let data: Vec<u8> = (0..(3 << 20) as u32).map(|i| (i % 253) as u8).collect();
    let p = std::env::temp_dir().join(format!("tcursor-sha256-{}.bin", std::process::id()));
    std::fs::File::create(&p).unwrap().write_all(&data).unwrap();
    assert_eq!(sha256_file(&p).unwrap(), sha256_hex(&data));
    let _ = std::fs::remove_file(&p);
}
