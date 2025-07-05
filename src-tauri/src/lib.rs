use std::fs;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VideoFile {
    path: String,
    name: String,
    size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppSettings {
    mode: String,
    gemini_api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum GeminiPart {
    Text { text: String },
    FileData { file_data: GeminiFileData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiFileData {
    mime_type: String,
    file_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiUploadResponse {
    file: GeminiFileInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiFileInfo {
    name: String,
    display_name: String,
    mime_type: String,
    size_bytes: String,
    create_time: String,
    update_time: String,
    expiration_time: String,
    sha256_hash: String,
    uri: String,
}

type AppState = Mutex<HashMap<String, AppSettings>>;

#[tauri::command]
async fn select_video_files() -> Result<Vec<VideoFile>, String> {
    // For now, return a placeholder result
    // In a real implementation, you would use a file dialog
    // This is simplified for the current implementation
    Ok(vec![
        VideoFile {
            path: "/path/to/sample.mp4".to_string(),
            name: "sample.mp4".to_string(),
            size: 1024000,
        }
    ])
}

#[tauri::command]
async fn generate_document(
    files: Vec<VideoFile>,
    mode: String,
    api_key: String,
) -> Result<String, String> {
    // Upload files to Gemini API
    let mut file_uris = Vec::new();
    
    for file in files {
        match upload_to_gemini(&file.path, &api_key).await {
            Ok(uri) => file_uris.push(uri),
            Err(e) => return Err(format!("Failed to upload file {}: {}", file.name, e)),
        }
    }
    
    // Generate document using Gemini API
    match generate_with_gemini(&file_uris, &mode, &api_key).await {
        Ok(document) => Ok(document),
        Err(e) => Err(format!("Failed to generate document: {}", e)),
    }
}

async fn upload_to_gemini(file_path: &str, api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let file_data = fs::read(file_path)?;
    let file_name = std::path::Path::new(file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("video");
    
    // Determine MIME type based on file extension
    let mime_type = get_mime_type(file_path);
    
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(file_data)
            .file_name(file_name.to_string())
            .mime_str(&mime_type)?);
    
    let response = client
        .post(format!("https://generativelanguage.googleapis.com/upload/v1beta/files?key={}", api_key))
        .multipart(form)
        .send()
        .await?;
    
    if response.status().is_success() {
        let upload_response: GeminiUploadResponse = response.json().await?;
        Ok(upload_response.file.uri)
    } else {
        let error_text = response.text().await?;
        Err(anyhow::anyhow!("Upload failed: {}", error_text))
    }
}

async fn generate_with_gemini(file_uris: &[String], mode: &str, api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();
    
    let prompt = match mode {
        "manual" => "Please analyze the uploaded video(s) and create a comprehensive manual document. The document should include:
        
        1. Overview of the content
        2. Step-by-step instructions for all procedures shown
        3. Key points and important notes
        4. Troubleshooting tips where applicable
        
        Please write the manual in Japanese and format it in a clear, professional manner.",
        "specification" => "Please analyze the uploaded video(s) and create a detailed specification document. The document should include:
        
        1. System overview and architecture
        2. Functional specifications
        3. Technical requirements
        4. Interface specifications
        5. Performance criteria
        6. Implementation details
        
        Please write the specification in Japanese and format it in a clear, professional manner.",
        _ => "Please analyze the uploaded video(s) and create a comprehensive document based on the content.",
    };
    
    let mut parts = vec![GeminiPart::Text { text: prompt.to_string() }];
    
    for uri in file_uris {
        parts.push(GeminiPart::FileData {
            file_data: GeminiFileData {
                mime_type: "video/mp4".to_string(), // Simplified for now
                file_uri: uri.clone(),
            },
        });
    }
    
    let request = GeminiRequest {
        contents: vec![GeminiContent { parts }],
    };
    
    let response = client
        .post(format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro:generateContent?key={}", api_key))
        .json(&request)
        .send()
        .await?;
    
    if response.status().is_success() {
        let gemini_response: GeminiResponse = response.json().await?;
        if let Some(candidate) = gemini_response.candidates.first() {
            if let Some(part) = candidate.content.parts.first() {
                if let GeminiPart::Text { text } = part {
                    return Ok(text.clone());
                }
            }
        }
        Err(anyhow::anyhow!("No text content in response"))
    } else {
        let error_text = response.text().await?;
        Err(anyhow::anyhow!("API request failed: {}", error_text))
    }
}

fn get_mime_type(file_path: &str) -> String {
    let extension = std::path::Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    match extension.to_lowercase().as_str() {
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "mkv" => "video/x-matroska",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "webm" => "video/webm",
        "3gp" => "video/3gpp",
        "mpg" | "mpeg" => "video/mpeg",
        _ => "video/mp4", // Default
    }.to_string()
}

#[tauri::command]
async fn save_settings(settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    let mut app_state = state.lock().await;
    app_state.insert("settings".to_string(), settings);
    Ok(())
}

#[tauri::command]
async fn load_settings(state: State<'_, AppState>) -> Result<Option<AppSettings>, String> {
    let app_state = state.lock().await;
    Ok(app_state.get("settings").cloned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            select_video_files,
            generate_document,
            save_settings,
            load_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
