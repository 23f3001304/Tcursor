use super::jpeg_encode;

fn ffmpeg_present() -> bool { crate::win::sys::proc::ffcmd("ffmpeg").arg("-version").output().is_ok() }

/// A 4x4 BGRA buffer comes back as a JPEG (SOI marker `FF D8`, EOI `FF D9`), and the encoder
/// never hands back PNG or raw bytes.
#[test]
fn a_bgra_buffer_encodes_to_a_jpeg() {
    if !ffmpeg_present() { eprintln!("ffmpeg missing; skipping"); return; }
    let mut bgra = vec![0u8; 4 * 4 * 4];
    for px in bgra.chunks_mut(4) { px.copy_from_slice(&[40, 90, 200, 255]); }
    let jpeg = jpeg_encode(&bgra, 4, 4).expect("encode");
    assert_eq!(&jpeg[..2], &[0xFF, 0xD8], "not a JPEG: {:?}", &jpeg[..4]);
    assert_eq!(&jpeg[jpeg.len() - 2..], &[0xFF, 0xD9]);
}
