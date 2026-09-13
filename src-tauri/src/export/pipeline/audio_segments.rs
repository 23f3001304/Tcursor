// The audio half of the time remap: the kept segments as an ffmpeg filter chain that trims each one
// out of the mixed track, re-times a speed span with `atempo` (pitch preserved), and concatenates
// them in order. Pure string building, so the exact command is pinned by tests; the identity (one
// segment at 1x) emits no chain at all, which keeps the no-cut mux command byte-identical.
use crate::export::remap::TimeMap;

/// One kept range in seconds of TRIMMED-video time (0 = the exported file's first frame), with the
/// factor it plays at.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioSeg { pub start_s: f64, pub end_s: f64, pub factor: f64 }

fn num(v: f64) -> String { format!("{v}") }

/// `atempo` only accepts 0.5..2 per instance (on every ffmpeg since 4.x; the bundled 8.1 takes more
/// but the chain is the portable form), so larger factors are chained: 4 is `atempo=2,atempo=2`,
/// 3 is `atempo=2,atempo=1.5`, 0.25 is `atempo=0.5,atempo=0.5`. Empty for 1.0.
pub fn atempo_chain(factor: f64) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut f = factor;
    while f > 2.0 + 1e-9 { parts.push("atempo=2".into()); f /= 2.0; }
    while f < 0.5 - 1e-9 { parts.push("atempo=0.5".into()); f *= 2.0; }
    if (f - 1.0).abs() > 1e-9 { parts.push(format!("atempo={}", num(f))); }
    parts.join(",")
}

fn one(input: &str, s: &AudioSeg, output: &str) -> String {
    let tempo = atempo_chain(s.factor);
    let tempo = if tempo.is_empty() { String::new() } else { format!(",{tempo}") };
    format!("{input}atrim=start={:.3}:end={:.3},asetpts=PTS-STARTPTS{tempo}{output}", s.start_s, s.end_s)
}

/// The chain from `input` (a labelled stream, e.g. `[x]`) to `output` (e.g. `[a]`): `asplit` into one
/// branch per segment, each trimmed, re-based and re-timed, then `concat`. `None` for the identity
/// (no segments, or exactly one at 1x), so the caller keeps its pre-remap command untouched.
pub fn segment_chain(input: &str, segs: &[AudioSeg], output: &str) -> Option<String> {
    let identity = segs.is_empty() || (segs.len() == 1 && (segs[0].factor - 1.0).abs() < 1e-9);
    if identity { return None; }
    if segs.len() == 1 { return Some(one(input, &segs[0], output)); }
    let n = segs.len();
    let split = format!("{input}asplit={n}{}", (0..n).map(|i| format!("[x{i}]")).collect::<String>());
    let branches: Vec<String> = segs.iter().enumerate().map(|(i, s)| one(&format!("[x{i}]"), s, &format!("[s{i}]"))).collect();
    let concat = format!("{}concat=n={n}:v=0:a=1{output}", (0..n).map(|i| format!("[s{i}]")).collect::<String>());
    Some(format!("{split};{};{concat}", branches.join(";")))
}

/// The map's segments in trimmed-video seconds: `trim_in_q_ms` is the frame-floored clip time of
/// output frame 0 (`plan[0] * 1000 / fps`), the same shift the mux applies to each input. Segments
/// with no frame at this rate contribute no audio either.
pub fn audio_segs(map: &TimeMap, out_fps: u64, trim_in_q_ms: u64) -> Vec<AudioSeg> {
    map.segments().iter().enumerate().filter(|(i, _)| map.frame_bounds(*i, out_fps).is_some()).map(|(_, s)| AudioSeg {
        start_s: (s.clip_start as f64 - trim_in_q_ms as f64) / 1000.0,
        end_s: (s.clip_end as f64 - trim_in_q_ms as f64) / 1000.0,
        factor: s.factor,
    }).collect()
}

#[cfg(test)]
#[path = "audio_segments_tests.rs"]
mod tests;
