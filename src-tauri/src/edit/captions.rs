use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CaptionWord {
    pub start_ms: u32,
    pub end_ms: u32,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Caption {
    pub id: String,
    pub start_ms: u32,
    pub end_ms: u32,
    pub text: String,
    #[serde(default)]
    pub words: Vec<CaptionWord>,
}

#[cfg(test)]
#[path = "captions_tests.rs"]
mod tests;
