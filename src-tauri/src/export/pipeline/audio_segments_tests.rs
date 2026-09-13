use super::*;

#[test]
fn atempo_chains_stay_inside_ffmpegs_classic_range() {
    assert_eq!(atempo_chain(1.0), "");
    assert_eq!(atempo_chain(2.0), "atempo=2");
    assert_eq!(atempo_chain(4.0), "atempo=2,atempo=2");
    assert_eq!(atempo_chain(3.0), "atempo=2,atempo=1.5");
    assert_eq!(atempo_chain(0.25), "atempo=0.5,atempo=0.5");
    assert_eq!(atempo_chain(8.0), "atempo=2,atempo=2,atempo=2");
}

#[test]
fn the_identity_emits_no_chain_so_the_mux_command_is_unchanged() {
    assert_eq!(segment_chain("[x]", &[AudioSeg { start_s: 0.0, end_s: 8.5, factor: 1.0 }], "[a]"), None);
    assert_eq!(segment_chain("[x]", &[], "[a]"), None);
}

#[test]
fn one_retimed_segment_needs_no_split_or_concat() {
    assert_eq!(segment_chain("[x]", &[AudioSeg { start_s: 0.0, end_s: 2.0, factor: 2.0 }], "[a]").unwrap(),
        "[x]atrim=start=0.000:end=2.000,asetpts=PTS-STARTPTS,atempo=2[a]");
}

#[test]
fn segments_split_trim_setpts_tempo_and_concat_in_order() {
    let segs = [AudioSeg { start_s: 0.0, end_s: 0.5, factor: 1.0 }, AudioSeg { start_s: 1.5, end_s: 2.5, factor: 2.0 }];
    assert_eq!(segment_chain("[x]", &segs, "[a]").unwrap(),
        "[x]asplit=2[x0][x1];[x0]atrim=start=0.000:end=0.500,asetpts=PTS-STARTPTS[s0];[x1]atrim=start=1.500:end=2.500,asetpts=PTS-STARTPTS,atempo=2[s1];[s0][s1]concat=n=2:v=0:a=1[a]");
}

#[test]
fn audio_segs_come_from_the_map_in_trimmed_video_seconds() {
    let m = crate::export::remap::tests::fixture();
    let segs = audio_segs(&m, 10, 500);
    assert_eq!(segs.len(), 7);
    assert_eq!((segs[0].start_s, segs[0].end_s, segs[0].factor), (0.0, 0.5, 1.0));
    assert_eq!((segs[2].start_s, segs[2].end_s, segs[2].factor), (2.0, 3.0, 2.0));
    assert_eq!((segs[6].start_s, segs[6].end_s, segs[6].factor), (7.5, 8.5, 1.0));
}
