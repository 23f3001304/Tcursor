// Split from vfr_segments.rs per repo convention (#[path] sibling test module) to stay under
// the 200-line file limit.
use super::*;

fn part(path: &str, first_ms: Option<u64>) -> Part {
    Part { path: PathBuf::from(path), first_ms }
}

#[test]
fn part_paths_are_numbered_siblings_of_the_output() {
    let out = PathBuf::from("C:/rec/video.mp4");
    assert_eq!(part_path(&out, 1), PathBuf::from("C:/rec/video.part1.mp4"));
    assert_eq!(part_path(&out, 12), PathBuf::from("C:/rec/video.part12.mp4"));
}

/// Inside a single-quoted concat token ffmpeg's `av_get_token` treats a backslash as an
/// ordinary character, so `\'` ends the token early: verified against the bundled ffmpeg, a
/// path under `O'Brien` resolved to `O\Brien` and the join failed outright. Only the
/// close-escape-reopen form parses.
#[test]
fn concat_list_uses_forward_slashes_and_escapes_quotes_by_reopening() {
    let parts = vec![part(r"C:\O'Brien\video.mp4", Some(0)), part(r"C:\O'Brien\video.part1.mp4", Some(0))];
    let list = concat_list(&parts);
    assert!(list.contains("file 'C:/O'\\''Brien/video.mp4'\n"), "{list}");
    assert!(list.contains("file 'C:/O'\\''Brien/video.part1.mp4'\n"), "{list}");
    assert!(!list.contains("\\'B"), "backslash-escaped quote is not parsable by av_get_token: {list}");
}

#[test]
fn concat_list_is_one_line_per_part_in_order() {
    let parts: Vec<Part> = (0..3).map(|i| part(&format!("/r/v{i}.mp4"), None)).collect();
    assert_eq!(concat_list(&parts), "file '/r/v0.mp4'\nfile '/r/v1.mp4'\nfile '/r/v2.mp4'\n");
}

/// The `duration` directive is what the demuxer offsets the FOLLOWING file by, so every part
/// but the last carries one, and it is the recording clock's own span between consecutive
/// parts' first frames - not the part's container duration, whose final-frame length is a VFR
/// guess (~50 ms with this encoder) that would drift every boundary and accumulate.
#[test]
fn concat_list_carries_the_sync_clock_span_as_each_part_duration() {
    let parts = vec![
        part("/r/video.mp4", Some(25)),        // span to the next part's first frame: 1530ms
        part("/r/video.part1.mp4", Some(1555)), // span: 2000ms
        part("/r/video.part2.mp4", Some(3555)), // last part: no directive
    ];
    assert_eq!(
        concat_list(&parts),
        "file '/r/video.mp4'\nduration 1.530\n\
         file '/r/video.part1.mp4'\nduration 2.000\n\
         file '/r/video.part2.mp4'\n"
    );
}

/// Sub-second and multi-second spans both render as a plain seconds-with-millis literal, which
/// is what the demuxer's duration parser takes.
#[test]
fn duration_directives_render_millisecond_precision() {
    let parts = vec![part("/r/a.mp4", Some(0)), part("/r/b.mp4", Some(7)), part("/r/c.mp4", Some(61_009))];
    let list = concat_list(&parts);
    assert!(list.contains("duration 0.007\n"), "{list}");
    assert!(list.contains("duration 61.002\n"), "{list}");
}

/// A part with no first frame (a span the user paused straight back out of) is normally
/// dropped before the list is built; if one ever reaches it, the boundary simply falls back to
/// the container duration rather than emitting a nonsense directive.
#[test]
fn a_part_without_frames_emits_no_duration_directive() {
    let parts = vec![part("/r/a.mp4", Some(100)), part("/r/b.mp4", None), part("/r/c.mp4", Some(900))];
    let list = concat_list(&parts);
    assert_eq!(list.matches("duration ").count(), 0, "{list}");
}
