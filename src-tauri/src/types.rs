use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VideoQuality {
    NoConversion,
    #[serde(rename = "1080p")]
    Quality1080p,
    #[serde(rename = "720p")]
    Quality720p,
    #[serde(rename = "480p")]
    Quality480p,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImageEmbedFrequency {
    #[serde(rename = "minimal")]
    Minimal, // 最小限（重要なポイントのみ）
    #[serde(rename = "moderate")]
    Moderate, // 適度（通常）
    #[serde(rename = "detailed")]
    Detailed, // 詳細（多め）
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FrameExtractionMethod {
    #[serde(rename = "standard")]
    Standard, // 標準の extract_frame_from_video
    #[serde(rename = "fast")]
    Fast, // 高速版 extract_frame_fast
    #[serde(rename = "multiple")]
    Multiple, // 複数同時 extract_multiple_frames_from_video
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeVideoInfo {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VideoSource {
    #[serde(rename = "local")]
    Local { files: Vec<VideoFile> },
    #[serde(rename = "youtube")]
    YouTube { video: YouTubeVideoInfo },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub gemini_api_key: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default)]
    pub custom_prompt: Option<String>,
    #[serde(default = "default_gemini_model")]
    pub gemini_model: String,
    #[serde(default)]
    pub embed_images: bool,
    #[serde(default = "default_image_embed_frequency")]
    pub image_embed_frequency: ImageEmbedFrequency,
    #[serde(default = "default_video_quality")]
    pub video_quality: VideoQuality,
    #[serde(default)]
    pub hardware_encoding: bool,
    // 実験用機能
    #[serde(default)]
    pub enable_experimental_features: bool,
    #[serde(default = "default_frame_extraction_method")]
    pub frame_extraction_method: FrameExtractionMethod,
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

pub fn default_temperature() -> f64 {
    0.0
}

pub fn default_gemini_model() -> String {
    "gemini-2.5-pro".to_string()
}

pub fn default_video_quality() -> VideoQuality {
    VideoQuality::NoConversion
}

pub fn default_image_embed_frequency() -> ImageEmbedFrequency {
    ImageEmbedFrequency::Moderate
}

pub fn default_frame_extraction_method() -> FrameExtractionMethod {
    FrameExtractionMethod::Standard
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
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
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
}
