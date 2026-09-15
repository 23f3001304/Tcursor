use super::*;

fn part(path: &str, first_ms: Option<u64>) -> Part {
    Part {
        path: PathBuf::from(path),
        first_ms,
    }
}

#[test]
fn part_paths_are_numbered_siblings_of_the_output() {
    let out = PathBuf::from("C:/rec/video.mp4");
    assert_eq!(part_path(&out, 1), PathBuf::from("C:/rec/video.part1.mp4"));
    assert_eq!(
        part_path(&out, 12),
        PathBuf::from("C:/rec/video.part12.mp4")
    );
}

#[test]
fn concat_list_uses_forward_slashes_and_escapes_quotes_by_reopening() {
    let parts = vec![
        part(r"C:\O'Brien\video.mp4", Some(0)),
        part(r"C:\O'Brien\video.part1.mp4", Some(0)),
    ];
    let list = concat_list(&parts);
    assert!(list.contains("file 'C:/O'\\''Brien/video.mp4'\n"), "{list}");
    assert!(
        list.contains("file 'C:/O'\\''Brien/video.part1.mp4'\n"),
        "{list}"
    );
    assert!(
        !list.contains("\\'B"),
        "backslash-escaped quote is not parsable by av_get_token: {list}"
    );
}

#[test]
fn concat_list_is_one_line_per_part_in_order() {
    let parts: Vec<Part> = (0..3)
        .map(|i| part(&format!("/r/v{i}.mp4"), None))
        .collect();
    assert_eq!(
        concat_list(&parts),
        "file '/r/v0.mp4'\nfile '/r/v1.mp4'\nfile '/r/v2.mp4'\n"
    );
}

#[test]
fn concat_list_carries_the_sync_clock_span_as_each_part_duration() {
    let parts = vec![
        part("/r/video.mp4", Some(25)),
        part("/r/video.part1.mp4", Some(1555)),
        part("/r/video.part2.mp4", Some(3555)),
    ];
    assert_eq!(
        concat_list(&parts),
        "file '/r/video.mp4'\nduration 1.530\n\
         file '/r/video.part1.mp4'\nduration 2.000\n\
         file '/r/video.part2.mp4'\n"
    );
}

#[test]
fn duration_directives_render_millisecond_precision() {
    let parts = vec![
        part("/r/a.mp4", Some(0)),
        part("/r/b.mp4", Some(7)),
        part("/r/c.mp4", Some(61_009)),
    ];
    let list = concat_list(&parts);
    assert!(list.contains("duration 0.007\n"), "{list}");
    assert!(list.contains("duration 61.002\n"), "{list}");
}

#[test]
fn a_part_without_frames_emits_no_duration_directive() {
    let parts = vec![
        part("/r/a.mp4", Some(100)),
        part("/r/b.mp4", None),
        part("/r/c.mp4", Some(900)),
    ];
    let list = concat_list(&parts);
    assert_eq!(list.matches("duration ").count(), 0, "{list}");
}
