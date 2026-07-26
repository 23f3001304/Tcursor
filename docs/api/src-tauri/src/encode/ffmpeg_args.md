# src-tauri/src/encode/ffmpeg_args.rs

Pure ffmpeg CLI argument construction for one export encode, split out of `ffmpeg_encoder.rs` so the settings -> args mapping is unit-tested without spawning a real ffmpeg process (mirrors the `proc::choose_dir` / `ffio::crop_to_alpha` pattern elsewhere: pure logic kept separate from the I/O that uses it).

## export_args

```rust
pub fn export_args(format: Format, h264_encoder: &str, width: u32, height: u32, fps: f64, crf: u8, out_path: &str) -> Vec<String>
```

Builds the complete ffmpeg CLI arguments (everything after the program name) for one export encode: the rawvideo stdin input flags common to every format, then per-format video-encode flags, then the output path.

### Inputs

- `format: Format` (`export::settings::Format`) - which container/codec to build args for.
- `h264_encoder: &str` - the hardware-probed encoder name from `ffmpeg_encoder::h264_encoder()` (e.g. `"libx264"`, `"h264_nvenc"`). *Why a plain string, not resolved internally:* keeps this function pure (no `OnceLock`/process probe), so it is fully unit-testable with any encoder name. Ignored for non-`Mp4` formats.
- `width: u32`, `height: u32` - output frame dimensions, from the resolved `Layout`.
- `fps: f64` - output frame rate (`ExportSettings.fps` resolved via `Fps::resolve_hz`).
- `crf: u8` - the user's quality slider (18..28, `settings::DEFAULT_CRF` = 24). Drives quality for the two H.264 paths with a real quality knob today - software `libx264` (`-crf`) and `h264_nvenc` (`-cq`) - and the new `WebM` (VP9, constant-quality `-crf` + `-b:v 0`). `h264_qsv`/`h264_amf`/`h264_mf` keep today's fixed 12 Mbps bitrate UNCHANGED regardless of `crf` - they never had a quality knob before this feature, so the back-compat guarantee (default settings reproduce today's export byte-for-byte) holds for all five H.264 paths, not only the ones that gained a working slider. `Gif` ignores `crf` entirely: quality comes from the `palettegen`/`paletteuse` filter chain, not a bitrate/CRF knob.
- `out_path: &str` - destination file path, appended as the final argument.

### Returns

`Vec<String>` - the full argument list, ready to pass to `Command::args`. Always starts with `-y` (overwrite without prompting) and the rawvideo input flags (`-f rawvideo -pixel_format bgra -video_size WxH -framerate F -i pipe:0`); always ends with `out_path`.

### Implementation

- `Format::Mp4` - `-c:v {h264_encoder} -pix_fmt yuv420p`, then a `match` on `h264_encoder` for the quality/bitrate args: `libx264` -> `-preset veryfast -crf {crf}`; `h264_nvenc` -> `-preset p2 -rc vbr -cq {crf} -b:v 12M -bf 0`; `h264_qsv` -> `-preset veryfast -b:v 12M` (crf ignored); `h264_amf` -> `-quality speed -b:v 12M` (crf ignored); anything else (`h264_mf` included) -> `-b:v 12M` (crf ignored).
- `Format::WebM` - `-c:v libvpx-vp9 -pix_fmt yuv420p -crf {crf} -b:v 0 -deadline good -cpu-used 4` (constant-quality VP9; `-cpu-used 4` trades some compression efficiency for much faster encodes, since VP9 at its defaults is notoriously slow).
- `Format::Gif` - `-filter_complex "[0:v] split [a][b];[a] palettegen [p];[b][p] paletteuse" -loop 0`. No `-pix_fmt` (GIF's palette/`pal8` output is handled entirely by the filter chain + the `gif` muxer inferred from the `.gif` extension).

### Behaviors worth knowing

- `mp4_libx264_default_crf_matches_todays_hardcoded_args`, `mp4_nvenc_default_crf_matches_todays_hardcoded_args` - the back-compat guard: `crf: 24` (the default) reproduces the exact args `FfmpegFrameSink::spawn`'s old hardcoded "medium" branch built.
- `mp4_qsv_amf_mf_ignore_crf_and_keep_the_fixed_bitrate` - args are IDENTICAL across the full 18..28 crf range for these three encoder names (and any unrecognized name), always the fixed `-b:v 12M`.
- `webm_uses_constant_quality_vp9`, `gif_uses_palettegen_paletteuse_and_has_no_pix_fmt` - the new formats' argument shape.
- `every_format_ends_with_the_output_path_and_starts_with_rawvideo_input` - the shared prefix/suffix invariant across all three formats.

### Used by

- `src-tauri/src/encode/ffmpeg_encoder.rs` - `FfmpegFrameSink::new_medium` calls this to build its ffmpeg `Command`'s args.
