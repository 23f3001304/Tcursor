use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub(crate) struct Msg<'a> {
    role: &'a str,
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<&'a [String]>,
}

#[derive(Serialize)]
pub(crate) struct ChatReq<'a> {
    model: &'a str,
    messages: Vec<Msg<'a>>,
    stream: bool,
    format: &'a str,
}

#[derive(Serialize)]
pub struct OllamaModel {
    pub name: String,
    pub vision: bool,
}

#[derive(Deserialize)]
struct RespMsg {
    content: String,
}

#[derive(Deserialize)]
struct ChatResp {
    message: RespMsg,
}

#[derive(Deserialize)]
struct TagsResp {
    models: Vec<TagEntry>,
}
#[derive(Deserialize)]
struct TagEntry {
    name: String,
}

pub fn list_models() -> Vec<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(2))
        .build();
    let resp = match agent.get("http://localhost:11434/api/tags").call() {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    resp.into_json::<TagsResp>()
        .map(|t| {
            t.models
                .into_iter()
                .map(|m| m.name)
                .filter(|n| !n.to_lowercase().contains("embed"))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn chat_body<'a>(
    model: &'a str,
    system: &'a str,
    user: &'a str,
    images: &'a [String],
) -> ChatReq<'a> {
    ChatReq {
        model,
        messages: vec![
            Msg {
                role: "system",
                content: system,
                images: None,
            },
            Msg {
                role: "user",
                content: user,
                images: (!images.is_empty()).then_some(images),
            },
        ],
        stream: false,
        format: "json",
    }
}

pub fn chat(model: &str, system: &str, user: &str) -> Result<String, String> {
    chat_with_images(model, system, user, &[])
}

pub fn chat_with_images(
    model: &str,
    system: &str,
    user: &str,
    images: &[String],
) -> Result<String, String> {
    let json_body = serde_json::to_value(chat_body(model, system, user, images))
        .map_err(|e| format!("serialize: {}", e))?;

    let read_secs = if images.is_empty() { 180 } else { 600 };
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(read_secs))
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

    let parsed: ChatResp = resp
        .into_json()
        .map_err(|e| format!("could not parse Ollama response: {}", e))?;

    Ok(parsed.message.content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_ride_on_the_user_message_not_on_the_request() {
        let imgs = vec!["QUJD".to_string()];
        let v = serde_json::to_value(chat_body("m", "sys", "transcript", &imgs)).unwrap();
        assert!(
            v.get("images").is_none(),
            "Ollama takes images per message, not per request"
        );
        assert_eq!(v["messages"][1]["images"][0], "QUJD");
        assert!(
            v["messages"][0].get("images").is_none(),
            "the system message carries no images"
        );
        assert_eq!(v["format"], "json");
        assert_eq!(v["stream"], false);
    }

    #[test]
    fn a_text_only_request_is_byte_identical_to_the_old_one() {
        let v = serde_json::to_value(chat_body("m", "sys", "transcript", &[])).unwrap();
        assert!(
            v["messages"][1].get("images").is_none(),
            "no empty images array: a text-only model must see exactly what it saw before"
        );
    }
}
