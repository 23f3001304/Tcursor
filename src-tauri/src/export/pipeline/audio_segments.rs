use crate::export::remap::TimeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct AudioSeg {
    pub start_s: f64,
    pub end_s: f64,
    pub factor: f64,
}

pub const RAMP_S: f64 = 0.020;

fn num(v: f64) -> String {
    format!("{v}")
}

pub fn atempo_chain(factor: f64) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut f = factor;
    while f > 2.0 + 1e-9 {
        parts.push("atempo=2".into());
        f /= 2.0;
    }
    while f < 0.5 - 1e-9 {
        parts.push("atempo=0.5".into());
        f *= 2.0;
    }
    if (f - 1.0).abs() > 1e-9 {
        parts.push(format!("atempo={}", num(f)));
    }
    parts.join(",")
}

fn ramp(s: &AudioSeg) -> String {
    let out = ((s.end_s - s.start_s) / s.factor).max(0.0);
    let d = RAMP_S.min(out / 2.0);
    if d <= 0.0 {
        return String::new();
    }
    format!(
        ",afade=t=in:st=0:d={d:.3},afade=t=out:st={:.3}:d={d:.3}",
        out - d
    )
}

fn one(input: &str, s: &AudioSeg, output: &str, ramped: bool) -> String {
    let tempo = atempo_chain(s.factor);
    let tempo = if tempo.is_empty() {
        String::new()
    } else {
        format!(",{tempo}")
    };
    let fade = if ramped { ramp(s) } else { String::new() };
    format!(
        "{input}atrim=start={:.3}:end={:.3},asetpts=PTS-STARTPTS{tempo}{fade}{output}",
        s.start_s, s.end_s
    )
}

pub fn segment_chain(input: &str, segs: &[AudioSeg], output: &str) -> Option<String> {
    let identity = segs.is_empty() || (segs.len() == 1 && (segs[0].factor - 1.0).abs() < 1e-9);
    if identity {
        return None;
    }
    if segs.len() == 1 {
        return Some(one(input, &segs[0], output, false));
    }
    let n = segs.len();
    let split = format!(
        "{input}asplit={n}{}",
        (0..n).map(|i| format!("[x{i}]")).collect::<String>()
    );
    let branches: Vec<String> = segs
        .iter()
        .enumerate()
        .map(|(i, s)| one(&format!("[x{i}]"), s, &format!("[s{i}]"), true))
        .collect();
    let concat = format!(
        "{}concat=n={n}:v=0:a=1{output}",
        (0..n).map(|i| format!("[s{i}]")).collect::<String>()
    );
    Some(format!("{split};{};{concat}", branches.join(";")))
}

pub fn audio_segs(map: &TimeMap, out_fps: u64, trim_in_q_ms: u64) -> Vec<AudioSeg> {
    map.segments()
        .iter()
        .enumerate()
        .filter(|(i, _)| map.frame_bounds(*i, out_fps).is_some())
        .map(|(_, s)| AudioSeg {
            start_s: ((s.clip_start as f64 - trim_in_q_ms as f64) / 1000.0).max(0.0),
            end_s: (s.clip_end as f64 - trim_in_q_ms as f64) / 1000.0,
            factor: s.factor,
        })
        .collect()
}

pub fn audio_origin_q(plan: &[u64], out_fps: u64) -> u64 {
    plan.iter().copied().min().unwrap_or(0) * 1000 / out_fps.max(1)
}

#[cfg(test)]
#[path = "audio_segments_tests.rs"]
mod tests;
