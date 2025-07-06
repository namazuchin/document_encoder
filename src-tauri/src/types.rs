use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFile {
    pub path: String,
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub mode: String,
    pub gemini_api_key: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub custom_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub message: String,
    pub step: usize,
    pub total_steps: usize,
}

pub fn default_language() -> String {
    "japanese".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPreset {
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPresets {
    pub presets: Vec<PromptPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiContent {
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GeminiPart {
    Text { text: String },
    FileData { file_data: GeminiFileData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiFileData {
    pub mime_type: String,
    pub file_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCandidate {
    pub content: GeminiContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiUploadResponse {
    pub file: GeminiFileInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiFileInfo {
    pub name: String,
    pub uri: String,
    pub mime_type: String,
}