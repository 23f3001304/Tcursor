// Pure ffmpeg CLI argument construction for one export encode, split out of ffmpeg_encoder.rs so
// the settings -> args mapping is unit-testable without spawning a real ffmpeg process (mirrors
// the `proc::choose_dir` / `ffio::crop_to_alpha` pattern elsewhere: pure logic kept separate from
// the I/O that uses it).
use crate::export::settings::Format;

/// `&[&str]` -> `Vec<String>`, so literals and computed strings mix in one array below.
fn ss(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// Build the complete ffmpeg CLI arguments (everything after the program name) for one export
/// encode: the rawvideo stdin input flags common to every format, then per-format video-encode
/// flags, then the output path.
///
/// `h264_encoder` is the hardware-probed encoder name from `ffmpeg_encoder::h264_encoder()`
/// (ignored for non-`Mp4` formats). `crf` (18..28, `settings::DEFAULT_CRF` = 24) drives quality
/// for the two H.264 paths with a real quality knob today - software `libx264` (`-crf`) and
/// `h264_nvenc` (`-cq`) - and the new `WebM` (VP9, constant-quality `-crf` + `-b:v 0`).
/// `h264_qsv`/`h264_amf`/`h264_mf` keep today's fixed 12 Mbps bitrate UNCHANGED regardless of
/// `crf` - they never had a quality knob before this feature, so leaving them alone keeps the
/// back-compat guarantee (default settings reproduce today's export byte-for-byte) true for all
/// five H.264 paths, not only the ones that gained a working slider. `Gif` ignores `crf`
/// entirely: quality comes from the `palettegen`/`paletteuse` filter chain, not a bitrate/CRF knob.
pub fn export_args(format: Format, h264_encoder: &str, width: u32, height: u32, fps: f64, crf: u8, out_path: &str) -> Vec<String> {
    let size = format!("{width}x{height}");
    let fr = format!("{fps:.4}");
    let crf_s = crf.to_string();
    let mut a = ss(&["-y", "-f", "rawvideo", "-pixel_format", "bgra", "-video_size", size.as_str(),
        "-framerate", fr.as_str(), "-i", "pipe:0"]);
    match format {
        Format::Mp4 => {
            a.extend(ss(&["-c:v", h264_encoder, "-pix_fmt", "yuv420p"]));
            a.extend(match h264_encoder {
                "libx264" => ss(&["-preset", "veryfast", "-crf", crf_s.as_str()]),
                "h264_nvenc" => ss(&["-preset", "p2", "-rc", "vbr", "-cq", crf_s.as_str(), "-b:v", "12M", "-bf", "0"]),
                "h264_qsv" => ss(&["-preset", "veryfast", "-b:v", "12M"]),
                "h264_amf" => ss(&["-quality", "speed", "-b:v", "12M"]),
                _ => ss(&["-b:v", "12M"]), // h264_mf + any unrecognized fallback
            });
        }
        Format::WebM => {
            a.extend(ss(&["-c:v", "libvpx-vp9", "-pix_fmt", "yuv420p", "-crf", crf_s.as_str(),
                "-b:v", "0", "-deadline", "good", "-cpu-used", "4"]));
        }
        Format::Gif => {
            a.extend(ss(&["-filter_complex", "[0:v] split [a][b];[a] palettegen [p];[b][p] paletteuse", "-loop", "0"]));
        }
    }
    a.push(out_path.to_string());
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mp4_libx264_default_crf_matches_todays_hardcoded_args() {
        let a = export_args(Format::Mp4, "libx264", 1920, 1080, 60.0, 24, "out.mp4");
        assert!(a.windows(2).any(|w| w == ["-preset", "veryfast"]));
        assert!(a.windows(2).any(|w| w == ["-crf", "24"]));
        assert_eq!(a.last().unwrap(), "out.mp4");
    }

    #[test]
    fn mp4_nvenc_default_crf_matches_todays_hardcoded_args() {
        let a = export_args(Format::Mp4, "h264_nvenc", 1920, 1080, 60.0, 24, "out.mp4");
        assert!(a.windows(2).any(|w| w == ["-cq", "24"]));
        assert!(a.windows(2).any(|w| w == ["-b:v", "12M"]));
    }

    #[test]
    fn mp4_qsv_amf_mf_ignore_crf_and_keep_the_fixed_bitrate() {
        for enc in ["h264_qsv", "h264_amf", "h264_mf", "some_unknown_encoder"] {
            let lo = export_args(Format::Mp4, enc, 1920, 1080, 60.0, 18, "out.mp4");
            let hi = export_args(Format::Mp4, enc, 1920, 1080, 60.0, 28, "out.mp4");
            assert_eq!(lo, hi, "{enc} args must not depend on crf");
            assert!(lo.windows(2).any(|w| w == ["-b:v", "12M"]), "{enc}");
        }
    }

    #[test]
    fn webm_uses_constant_quality_vp9() {
        let a = export_args(Format::WebM, "", 1920, 1080, 30.0, 22, "out.webm");
        assert!(a.windows(2).any(|w| w == ["-c:v", "libvpx-vp9"]));
        assert!(a.windows(2).any(|w| w == ["-crf", "22"]));
        assert!(a.windows(2).any(|w| w == ["-b:v", "0"]));
    }

    #[test]
    fn gif_uses_palettegen_paletteuse_and_has_no_pix_fmt() {
        let a = export_args(Format::Gif, "", 320, 240, 15.0, 24, "out.gif");
        assert!(a.iter().any(|s| s.contains("palettegen") && s.contains("paletteuse")));
        assert!(a.windows(2).any(|w| w == ["-loop", "0"]));
        assert!(!a.iter().any(|s| s == "-pix_fmt"));
    }

    #[test]
    fn every_format_ends_with_the_output_path_and_starts_with_rawvideo_input() {
        for (fmt, enc) in [(Format::Mp4, "libx264"), (Format::WebM, ""), (Format::Gif, "")] {
            let a = export_args(fmt, enc, 100, 100, 30.0, 24, "OUT_MARKER");
            assert_eq!(a.last().unwrap(), "OUT_MARKER");
            assert_eq!(a[0], "-y");
            assert!(a.windows(2).any(|w| w == ["-video_size", "100x100"]));
        }
    }
}
