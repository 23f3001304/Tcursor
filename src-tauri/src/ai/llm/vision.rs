use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const VISION_FAMILIES: [&str; 7] = [
    "clip",
    "mllama",
    "llava",
    "qwen2vl",
    "qwen2_5vl",
    "gemma3",
    "siglip",
];

fn cache() -> &'static Mutex<HashMap<String, bool>> {
    static VISION: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
    VISION.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn has_vision(model: &str) -> bool {
    if let Some(&hit) = cache()
        .lock()
        .ok()
        .and_then(|c| c.get(model).copied())
        .as_ref()
    {
        return hit;
    }
    let found = ask(model);
    if let Ok(mut c) = cache().lock() {
        c.insert(model.to_string(), found);
    }
    found
}

fn ask(model: &str) -> bool {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(2))
        .timeout_read(std::time::Duration::from_secs(10))
        .build();
    agent
        .post("http://localhost:11434/api/show")
        .send_json(serde_json::json!({ "model": model, "name": model }))
        .ok()
        .and_then(|r| r.into_string().ok())
        .is_some_and(|body| parse_show(&body))
}

pub(crate) fn parse_show(json: &str) -> bool {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let capable = v["capabilities"]
        .as_array()
        .is_some_and(|c| c.iter().any(|s| s.as_str() == Some("vision")));
    let family = v["details"]["families"].as_array().is_some_and(|f| {
        f.iter()
            .filter_map(|s| s.as_str())
            .any(|s| VISION_FAMILIES.contains(&s.to_lowercase().as_str()))
    });
    let projector = v["projector_info"]
        .as_object()
        .is_some_and(|o| !o.is_empty());
    capable || family || projector
}

#[cfg(test)]
mod tests {
    use super::parse_show;

    #[test]
    fn a_capabilities_array_naming_vision_is_the_primary_signal() {
        assert!(parse_show(r#"{"capabilities":["completion","vision"]}"#));
        assert!(!parse_show(r#"{"capabilities":["completion","tools"]}"#));
    }

    #[test]
    fn a_known_projector_family_still_counts_on_an_older_server() {
        assert!(parse_show(r#"{"details":{"families":["mllama","clip"]}}"#));
        assert!(parse_show(r#"{"details":{"families":["qwen2_5vl"]}}"#));
        assert!(!parse_show(r#"{"details":{"families":["llama"]}}"#));
    }

    #[test]
    fn a_non_empty_projector_block_counts_too() {
        assert!(parse_show(
            r#"{"projector_info":{"clip.has_vision_encoder":true}}"#
        ));
        assert!(!parse_show(r#"{"projector_info":{}}"#));
    }

    #[test]
    fn anything_unrecognised_is_text_only_and_never_an_error() {
        assert!(!parse_show("not json at all"));
        assert!(!parse_show("{}"));
        assert!(
            !parse_show(r#"{"capabilities":"vision"}"#),
            "a string where an array belongs is not a claim of vision"
        );
    }
}
