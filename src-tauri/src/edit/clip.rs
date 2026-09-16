use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Clip {
    pub id: String,
    pub src_in_ms: u32,
    pub src_out_ms: u32,
    #[serde(default)]
    pub transition_in_ms: u32,
}

#[cfg(test)]
#[path = "clip_tests.rs"]
mod tests;
