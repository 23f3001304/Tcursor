use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Msg<'a> { role: &'a str, content: &'a str }

#[derive(Serialize)]
struct ChatReq<'a> {
    model: &'a str,
    messages: Vec<Msg<'a>>,
    stream: bool,
    format: &'a str,
}

#[derive(Deserialize)]
struct RespMsg { content: String }

#[derive(Deserialize)]
struct ChatResp { message: RespMsg }

#[derive(Deserialize)]
struct TagsResp { models: Vec<TagEntry> }
#[derive(Deserialize)]
struct TagEntry { name: String }

/// List locally-installed CHAT model names (`GET /api/tags`, embedding models filtered out), for
/// the AI panel's engine picker and as `ai_autoedit`'s default-model source. Returns an empty list
/// - never an error - when Ollama is unreachable or the response doesn't parse.
pub fn list_models() -> Vec<String> {
    let agent = ureq::AgentBuilder::new().timeout_connect(std::time::Duration::from_secs(2)).build();
    let resp = match agent.get("http://localhost:11434/api/tags").call() {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    // Drop embedding-only models (e.g. nomic-embed-text, *-embed-*): they don't support /api/chat,
    // so offering them in the Director picker or picking one as the default just 404s/errors.
    resp.into_json::<TagsResp>()
        .map(|t| t.models.into_iter().map(|m| m.name)
            .filter(|n| !n.to_lowercase().contains("embed"))
            .collect())
        .unwrap_or_default()
}

pub fn chat(model: &str, system: &str, user: &str) -> Result<String, String> {
    let body = ChatReq {
        model,
        messages: vec![
            Msg { role: "system", content: system },
            Msg { role: "user",   content: user },
        ],
        stream: false,
        format: "json",
    };

    let json_body = serde_json::to_value(&body)
        .map_err(|e| format!("serialize: {}", e))?;

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(180))
        .build();
    let resp = agent.post("http://localhost:11434/api/chat")
        .set("Content-Type", "application/json")
        .send_json(json_body)
        .map_err(|e| match e {
            ureq::Error::Transport(_) =>
                "Could not reach Ollama at localhost:11434. Is it running? Try: ollama serve".into(),
            ureq::Error::Status(404, _) =>
                format!("Model '{model}' isn't installed in Ollama. Pull it (ollama pull {model}) or pick an installed model."),
            ureq::Error::Status(code, _) =>
                format!("Ollama returned HTTP {}", code),
        })?;

    let parsed: ChatResp = resp.into_json()
        .map_err(|e| format!("could not parse Ollama response: {}", e))?;

    Ok(parsed.message.content)
}
