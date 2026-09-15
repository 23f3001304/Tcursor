use crate::edit::ops::api::EditOp;

pub const NEW_ID: &str = "$new";

pub const PRE_ROLL_MS: u32 = 300;

pub const SNAP_MS: u32 = 1_000;

pub const WHY_MAX: usize = 120;
pub const MIN_SCALE: f32 = 1.2;
pub const MAX_SCALE: f32 = 3.0;

pub const RENDERABLE_KINDS: [ProposalKind; 6] = [
    ProposalKind::Zoom,
    ProposalKind::Layout,
    ProposalKind::Spotlight,
    ProposalKind::Trim,
    ProposalKind::Cut,
    ProposalKind::Speed,
];

#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProposalKind {
    Zoom,
    Layout,
    Spotlight,
    Trim,
    Cut,
    Speed,
}

impl ProposalKind {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_lowercase().as_str() {
            "zoom" => Self::Zoom,
            "layout" => Self::Layout,
            "spotlight" => Self::Spotlight,
            "trim" => Self::Trim,
            "cut" => Self::Cut,
            "speed" => Self::Speed,
            _ => return None,
        })
    }
}

#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct AiProposal {
    pub id: String,
    pub kind: ProposalKind,
    pub why: String,
    pub at_ms: u32,
    pub dur_ms: u32,
    pub rect: Option<[f32; 4]>,
    pub ops: Vec<EditOp>,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct AiRun {
    pub model: String,
    pub vision: bool,
    pub frames: usize,
    pub elapsed_ms: u64,
    pub proposals: Vec<AiProposal>,
}

#[derive(Clone, Copy, Debug)]
pub struct ClickAt {
    pub t_ms: u32,
    pub x: f32,
    pub y: f32,
}
