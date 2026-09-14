use super::*;
use crate::audio::wav_writer::WavWriter;
use crate::export::pipeline::ffio::probe_duration;
use crate::session::sync::Segment;

fn strs(args: &[OsString]) -> Vec<String> { args.iter().map(|a| a.to_string_lossy().into_owned()).collect() }
fn part(path: &str, delay_ms: u64) -> MicPart { MicPart { path: PathBuf::from(path), delay_ms } }

fn filter_of(args: &[OsString]) -> String {
    let a = strs(args);
    let i = a.iter().position(|x| x == "-filter_complex").expect("a filter_complex arg");
    a[i + 1].clone()
}

/// The merge shells out for real; without an ffmpeg those assertions are skipped rather than
/// failed (the same rule `bg_thumbs` and `pipeline::ffio_tests` use).
fn ffmpeg_present() -> bool {
    crate::win::sys::proc::ffcmd("ffmpeg").arg("-version").output().is_ok()
}

fn temp_project(tag: &str) -> ProjectPaths {
    let folder = std::env::temp_dir().join(format!("tcursor_micmerge_{}_{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("temp project folder");
    ProjectPaths { folder }
}

/// `ms` of audible mono PCM at `rate` - a coarse sawtooth, so a merged file is obviously not the
/// silence a broken filter graph would produce.
fn write_wav(path: &Path, ms: u32, rate: u32) {
    let mut w = WavWriter::create(path.to_str().expect("utf8 temp path"), rate, 1).expect("create wav");
    let n = (rate as u64 * ms as u64 / 1000) as usize;
    w.write(&(0..n).map(|i| ((i % 96) as i16 - 48) * 300).collect::<Vec<i16>>());
    w.finalize().expect("finalize wav");
}

fn sync_with(mic_ms: Option<u64>, frames: Vec<u64>, segs: &[(&str, u64)]) -> SyncLog {
    let mic_segments = segs.iter().map(|(p, t)| Segment { path: (*p).into(), start_ms: *t }).collect();
    SyncLog { frames, mic_ms, mic_segments, ..Default::default() }
}

/// One input - the take started with the mic OFF and it was switched on later - needs no mix at
/// all, only the lead-in silence that puts it back where it was spoken.
#[test]
fn one_segment_is_delayed_into_place_without_a_mix() {
    let args = merge_args(&[part("mic_2.wav", 1500)], 48000, 1, Path::new("out.wav"));
    assert_eq!(strs(&args), ["-i", "mic_2.wav", "-filter_complex",
        "[0:a]aresample=48000,aformat=channel_layouts=mono,adelay=1500[a]",
        "-map", "[a]", "-c:a", "pcm_s16le", "out.wav"].map(String::from).to_vec());
}

/// Two segments with a 4.2 s gap between the first one's start and the second's: the first branch
/// carries no `adelay` (it IS the origin) and the mix is unnormalised so neither is halved.
#[test]
fn two_segments_mix_with_the_second_delayed_by_its_gap() {
    let args = merge_args(&[part("mic.wav", 0), part("mic_2.wav", 4200)], 48000, 1, Path::new("out.wav"));
    let a = strs(&args);
    assert_eq!(&a[..4], ["-i", "mic.wav", "-i", "mic_2.wav"].map(String::from));
    assert_eq!(filter_of(&args),
        "[0:a]aresample=48000,aformat=channel_layouts=mono[s0];\
         [1:a]aresample=48000,aformat=channel_layouts=mono,adelay=4200[s1];\
         [s0][s1]amix=inputs=2:normalize=0:dropout_transition=0[a]");
    assert_eq!(a.last().expect("an output path"), "out.wav");
}

/// Three segments, and the rate/layout mismatch a real switch produces: the second device ran at
/// 44100 stereo and the third at 16000 mono, but the FIRST segment's spec is the merged track's,
/// so every branch resamples to it and `adelay` carries one value per channel of THAT layout.
#[test]
fn three_segments_are_all_forced_to_the_first_ones_rate_and_layout() {
    let parts = [part("mic.wav", 0), part("mic_2.wav", 9000), part("mic_3.wav", 21500)];
    let args = merge_args(&parts, 44100, 2, Path::new("out.wav"));
    assert_eq!(filter_of(&args),
        "[0:a]aresample=44100,aformat=channel_layouts=stereo[s0];\
         [1:a]aresample=44100,aformat=channel_layouts=stereo,adelay=9000|9000[s1];\
         [2:a]aresample=44100,aformat=channel_layouts=stereo,adelay=21500|21500[s2];\
         [s0][s1][s2]amix=inputs=3:normalize=0:dropout_transition=0[a]");
    assert_eq!(strs(&args).iter().filter(|x| *x == "-i").count(), 3);
}

/// Delays are measured from `mic_ms`, the instant `sync.json` places `mic.wav` at - and a segment
/// whose file never existed (the mic was switched OFF at that boundary) is simply not an input.
#[test]
fn delays_are_measured_from_the_first_segments_own_start() {
    let paths = temp_project("origin");
    write_wav(&paths.mic(), 10, 48000);
    write_wav(&paths.folder.join("mic_3.wav"), 10, 48000);
    let sync = sync_with(Some(1000), vec![900], &[("mic_2.wav", 3000), ("mic_3.wav", 7000)]);
    assert_eq!(parts_of(&paths, &sync),
        vec![MicPart { path: paths.mic(), delay_ms: 0 },
             MicPart { path: paths.folder.join("mic_3.wav"), delay_ms: 6000 }]);
    let _ = std::fs::remove_dir_all(&paths.folder);
}

/// Mic off at the take's start: there is no `mic.wav` and no `mic_ms`, so the origin is the
/// video's first frame - exactly where `pipeline::audio_shift_ms` places a track with no start.
#[test]
fn a_take_that_started_muted_measures_from_the_videos_first_frame() {
    let paths = temp_project("muted");
    write_wav(&paths.folder.join("mic_2.wav"), 10, 48000);
    let sync = sync_with(None, vec![500, 533], &[("mic_2.wav", 4500)]);
    assert_eq!(parts_of(&paths, &sync),
        vec![MicPart { path: paths.folder.join("mic_2.wav"), delay_ms: 4000 }]);
    let _ = std::fs::remove_dir_all(&paths.folder);
}

/// The ordinary take: no switch, nothing listed, not one byte touched and no `segments/` folder.
#[test]
fn a_take_with_no_switch_is_left_exactly_as_it_was() {
    let paths = temp_project("none");
    write_wav(&paths.mic(), 20, 48000);
    let before = std::fs::read(paths.mic()).expect("read mic.wav");
    merge_mic_segments(&paths, &SyncLog::default()).expect("a no-op merge");
    assert_eq!(std::fs::read(paths.mic()).expect("read mic.wav"), before);
    assert!(!paths.folder.join("segments").exists());
    let _ = std::fs::remove_dir_all(&paths.folder);
}

/// End to end through a real ffmpeg: 100 ms, a 50 ms gap, 100 ms - one 250 ms `mic.wav`, with
/// both originals kept. Re-running finds the originals already moved and changes nothing.
#[test]
fn two_generated_wavs_merge_into_one_track_with_the_gap_between_them() {
    if !ffmpeg_present() { eprintln!("SKIPPED: no ffmpeg on PATH"); return; }
    let paths = temp_project("real");
    write_wav(&paths.mic(), 100, 48000);
    write_wav(&paths.folder.join("mic_2.wav"), 100, 48000);
    let sync = sync_with(Some(1000), vec![900], &[("mic_2.wav", 1150)]);
    merge_mic_segments(&paths, &sync).expect("merge");
    let dur = probe_duration(&paths.mic()).expect("probe the merged track");
    assert!((dur - 0.250).abs() < 0.005, "merged mic.wav is {dur}s, expected 0.250");
    assert!(paths.folder.join("segments").join("mic.wav").exists(), "the first segment is kept");
    assert!(paths.folder.join("segments").join("mic_2.wav").exists(), "the second segment is kept");
    merge_mic_segments(&paths, &sync).expect("a second pass is a no-op");
    assert!((probe_duration(&paths.mic()).expect("re-probe") - 0.250).abs() < 0.005);
    let _ = std::fs::remove_dir_all(&paths.folder);
}
