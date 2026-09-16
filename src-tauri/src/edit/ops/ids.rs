use crate::edit::model::EditDoc;

pub(crate) fn next_zoom_id(doc: &EditDoc) -> String {
    let n = doc
        .zooms
        .iter()
        .filter_map(|z| z.id.strip_prefix('z').and_then(|s| s.parse::<u32>().ok()))
        .max()
        .map(|m| m + 1)
        .unwrap_or(doc.zooms.len() as u32);
    format!("z{}", n)
}

pub(crate) fn next_layout_id(doc: &EditDoc) -> String {
    let n = doc
        .layout
        .iter()
        .filter_map(|s| s.id.strip_prefix('l').and_then(|d| d.parse::<u32>().ok()))
        .max()
        .map(|m| m + 1)
        .unwrap_or(doc.layout.len() as u32);
    format!("l{}", n)
}

pub(crate) fn next_cam_id(doc: &EditDoc) -> String {
    let n = doc
        .camera_moves
        .iter()
        .filter_map(|m| m.id.strip_prefix('k').and_then(|d| d.parse::<u32>().ok()))
        .max()
        .map(|m| m + 1)
        .unwrap_or(doc.camera_moves.len() as u32);
    format!("k{}", n)
}

pub(crate) fn next_caption_id(doc: &EditDoc) -> String {
    let n = doc
        .captions
        .iter()
        .filter_map(|c| c.id.strip_prefix('c').and_then(|d| d.parse::<u32>().ok()))
        .max()
        .map(|m| m + 1)
        .unwrap_or(doc.captions.len() as u32);
    format!("c{}", n)
}

pub(crate) fn next_text_id(doc: &EditDoc) -> String {
    let n = doc
        .texts
        .iter()
        .filter_map(|t| t.id.strip_prefix('t').and_then(|d| d.parse::<u32>().ok()))
        .max()
        .map(|m| m + 1)
        .unwrap_or(doc.texts.len() as u32);
    format!("t{}", n)
}

pub(crate) fn next_clip_id(doc: &EditDoc) -> String {
    let n = doc
        .clips
        .iter()
        .filter_map(|c| c.id.strip_prefix("cl").and_then(|d| d.parse::<u32>().ok()))
        .max()
        .map(|m| m + 1)
        .unwrap_or(doc.clips.len() as u32);
    format!("cl{}", n)
}
