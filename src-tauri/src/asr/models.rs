use crate::settings::captions::CaptionStyle;
use std::path::PathBuf;

#[derive(Debug)]
pub struct ModelSpec {
    pub id: &'static str,
    pub file: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
    pub multilingual: bool,
    pub label: &'static str,
}

pub const MODELS: [ModelSpec; 4] = [
    ModelSpec {
        id: "tiny.en",
        file: "ggml-tiny.en.bin",
        bytes: 77_704_715,
        multilingual: false,
        sha256: "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f",
        label: "Tiny (English)",
    },
    ModelSpec {
        id: "base.en",
        file: "ggml-base.en.bin",
        bytes: 147_964_211,
        multilingual: false,
        sha256: "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002",
        label: "Base (English)",
    },
    ModelSpec {
        id: "small.en",
        file: "ggml-small.en.bin",
        bytes: 487_614_201,
        multilingual: false,
        sha256: "c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d",
        label: "Small (English)",
    },
    ModelSpec {
        id: "base",
        file: "ggml-base.bin",
        bytes: 147_951_465,
        multilingual: true,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        label: "Base (multilingual)",
    },
];

pub const VAD_MODEL: ModelSpec = ModelSpec {
    id: "silero-vad",
    file: "ggml-silero-v5.1.2.bin",
    bytes: 885_098,
    multilingual: true,
    sha256: "29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf",
    label: "Voice activity (Silero)",
};

pub fn find(id: &str) -> Option<&'static ModelSpec> {
    MODELS.iter().find(|m| m.id == id)
}

pub fn vad_path() -> PathBuf {
    model_dir().join(VAD_MODEL.file)
}

pub fn model_dir() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("TCursor")
        .join("models")
        .join("whisper")
}

pub fn model_path(id: &str) -> PathBuf {
    model_dir().join(find(id).map(|m| m.file).unwrap_or("unknown.bin"))
}

pub fn is_installed(id: &str) -> bool {
    find(id).is_some() && model_path(id).exists()
}

pub fn url_for(m: &ModelSpec) -> String {
    let repo = if m.id == VAD_MODEL.id {
        "ggml-org/whisper-vad"
    } else {
        "ggerganov/whisper.cpp"
    };
    format!("https://huggingface.co/{repo}/resolve/main/{}", m.file)
}

pub fn resolve_model(style: &CaptionStyle) -> Result<&'static ModelSpec, String> {
    let m = find(&style.model).ok_or_else(|| {
        format!(
            "Unknown caption model {:?}. Pick one from the Captions panel.",
            style.model
        )
    })?;
    if style.language == "auto" && !m.multilingual {
        return Err(format!("{} only transcribes English. Switch the model to Base (multilingual) to detect the language automatically.", m.label));
    }
    Ok(m)
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod tests;
