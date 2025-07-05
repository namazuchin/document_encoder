use std::fs;
use std::process::Command;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tauri::Manager;
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


#[tauri::command]
async fn select_video_files(app: tauri::AppHandle) -> Result<Vec<VideoFile>, String> {
    use tauri_plugin_dialog::DialogExt;
    use tokio::sync::oneshot;
    
    let (tx, rx) = oneshot::channel();
    
    app.dialog()
        .file()
        .add_filter("Video files", &["mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "3gp", "mpg", "mpeg"])
        .set_title("Select video files")
        .pick_files(move |files| {
            let _ = tx.send(files);
        });
    
    let files = rx.await.map_err(|e| format!("Failed to receive dialog result: {}", e))?;
    
    match files {
        Some(paths) => {
            let mut video_files = Vec::new();
            for file_path in paths {
                let path_str = file_path.to_string();
                let path_buf = std::path::PathBuf::from(&path_str);
                if let Ok(metadata) = fs::metadata(&path_buf) {
                    let file_name = path_buf.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    
                    video_files.push(VideoFile {
                        path: path_str,
                        name: file_name,
                        size: metadata.len(),
                    });
                }
            }
            Ok(video_files)
        }
        None => Ok(Vec::new()),
    }
}

#[tauri::command]
async fn generate_document(
    files: Vec<VideoFile>,
    mode: String,
    api_key: String,
) -> Result<String, String> {
    // Process files and split if necessary
    let mut processed_files = Vec::new();
    
    for file in files {
        match split_video_if_needed(&file.path).await {
            Ok(segments) => {
                if segments.len() > 1 {
                    // Video was split into multiple segments
                    for segment in segments {
                        processed_files.push(segment);
                    }
                } else {
                    // Video was not split (under 1 hour)
                    processed_files.push(file.path);
                }
            }
            Err(e) => return Err(format!("Failed to process file {}: {}", file.name, e)),
        }
    }
    
    // Upload files to Gemini API
    let mut file_uris = Vec::new();
    
    for file_path in processed_files {
        match upload_to_gemini(&file_path, &api_key).await {
            Ok(uri) => file_uris.push(uri),
            Err(e) => return Err(format!("Failed to upload file {}: {}", file_path, e)),
        }
    }
    
    // Generate documents for each file/segment
    let mut documents = Vec::new();
    
    for file_uri in file_uris {
        match generate_with_gemini(&[file_uri], &mode, &api_key).await {
            Ok(document) => documents.push(document),
            Err(e) => return Err(format!("Failed to generate document for file: {}", e)),
        }
    }
    
    // Integrate multiple documents if necessary
    let final_document = if documents.len() > 1 {
        match integrate_documents(&documents, &mode, &api_key).await {
            Ok(integrated) => integrated,
            Err(e) => return Err(format!("Failed to integrate documents: {}", e)),
        }
    } else {
        documents.into_iter().next().unwrap_or_default()
    };
    
    Ok(final_document)
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

async fn split_video_if_needed(video_path: &str) -> Result<Vec<String>> {
    // Get video duration using ffprobe
    let duration = get_video_duration(video_path).await?;
    
    // If duration is less than 1 hour (3600 seconds), no need to split
    if duration < 3600.0 {
        return Ok(vec![video_path.to_string()]);
    }
    
    // Calculate number of segments needed (each segment should be < 1 hour)
    let segment_duration = 3500.0; // 58 minutes and 20 seconds to be safe
    let num_segments = (duration / segment_duration).ceil() as i32;
    
    let mut segments = Vec::new();
    let temp_dir = tempfile::tempdir()?;
    let file_stem = Path::new(video_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let file_extension = Path::new(video_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mp4");
    
    for i in 0..num_segments {
        let start_time = i as f64 * segment_duration;
        let output_path = temp_dir.path().join(format!("{}_part_{:03}.{}", file_stem, i + 1, file_extension));
        
        // Use ffmpeg to split the video
        let output = Command::new("ffmpeg")
            .args(&[
                "-i", video_path,
                "-ss", &start_time.to_string(),
                "-t", &segment_duration.to_string(),
                "-c", "copy", // Copy without re-encoding for speed
                "-avoid_negative_ts", "make_zero",
                output_path.to_str().unwrap(),
                "-y" // Overwrite output files
            ])
            .output();
        
        match output {
            Ok(result) => {
                if result.status.success() {
                    segments.push(output_path.to_string_lossy().to_string());
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    return Err(anyhow::anyhow!("ffmpeg failed: {}", stderr));
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to execute ffmpeg: {}. Make sure ffmpeg is installed and in your PATH.", e));
            }
        }
    }
    
    Ok(segments)
}

async fn get_video_duration(video_path: &str) -> Result<f64> {
    let output = Command::new("ffprobe")
        .args(&[
            "-v", "quiet",
            "-show_entries", "format=duration",
            "-of", "csv=p=0",
            video_path
        ])
        .output();
    
    match output {
        Ok(result) => {
            if result.status.success() {
                let duration_str = String::from_utf8_lossy(&result.stdout);
                let duration: f64 = duration_str.trim().parse()
                    .map_err(|_| anyhow::anyhow!("Failed to parse duration"))?;
                Ok(duration)
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                Err(anyhow::anyhow!("ffprobe failed: {}", stderr))
            }
        }
        Err(e) => {
            Err(anyhow::anyhow!("Failed to execute ffprobe: {}. Make sure ffmpeg is installed and in your PATH.", e))
        }
    }
}

async fn integrate_documents(documents: &[String], mode: &str, api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();
    
    let integration_prompt = match mode {
        "manual" => {
            format!(
                "Please integrate the following manual documents into one comprehensive, cohesive manual. \
                Ensure proper flow, eliminate redundancy, and organize the content logically:\n\n{}",
                documents.iter()
                    .enumerate()
                    .map(|(i, doc)| format!("=== Document {} ===\n{}\n", i + 1, doc))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
        "specification" => {
            format!(
                "Please integrate the following specification documents into one comprehensive, cohesive specification. \
                Ensure technical consistency, proper organization, and eliminate redundancy:\n\n{}",
                documents.iter()
                    .enumerate()
                    .map(|(i, doc)| format!("=== Specification Part {} ===\n{}\n", i + 1, doc))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
        _ => {
            format!(
                "Please integrate the following documents into one comprehensive document:\n\n{}",
                documents.iter()
                    .enumerate()
                    .map(|(i, doc)| format!("=== Document {} ===\n{}\n", i + 1, doc))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    };
    
    let request = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart::Text { text: integration_prompt }],
        }],
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
        Err(anyhow::anyhow!("No text content in integration response"))
    } else {
        let error_text = response.text().await?;
        Err(anyhow::anyhow!("Document integration failed: {}", error_text))
    }
}

#[tauri::command]
async fn save_settings(settings: AppSettings, app: tauri::AppHandle) -> Result<(), String> {
    let config_path = get_config_file_path(&app)?;
    
    // Ensure the parent directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    
    // Encrypt sensitive data before saving
    let safe_settings = AppSettings {
        mode: settings.mode,
        gemini_api_key: encrypt_api_key(&settings.gemini_api_key),
    };
    
    let config_json = serde_json::to_string_pretty(&safe_settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    fs::write(&config_path, config_json)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;
    
    Ok(())
}

#[tauri::command]
async fn load_settings(app: tauri::AppHandle) -> Result<Option<AppSettings>, String> {
    let config_path = get_config_file_path(&app)?;
    
    if !config_path.exists() {
        return Ok(None);
    }
    
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    
    let mut settings: AppSettings = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to parse settings file: {}", e))?;
    
    // Decrypt sensitive data after loading
    settings.gemini_api_key = decrypt_api_key(&settings.gemini_api_key);
    
    Ok(Some(settings))
}

fn get_config_file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_dir = app.path().app_config_dir()
        .map_err(|e| format!("Failed to get app config directory: {}", e))?;
    
    Ok(app_dir.join("settings.json"))
}

fn encrypt_api_key(api_key: &str) -> String {
    // Simple XOR encryption with a fixed key for demonstration
    // In production, use proper encryption like AES
    let key = b"document_encoder_key_2024"; // 24-byte key
    let mut encrypted = Vec::new();
    
    for (i, byte) in api_key.bytes().enumerate() {
        encrypted.push(byte ^ key[i % key.len()]);
    }
    
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.encode(encrypted)
}

fn decrypt_api_key(encrypted_api_key: &str) -> String {
    // Decrypt using the same XOR method
    let key = b"document_encoder_key_2024";
    
    use base64::{Engine as _, engine::general_purpose};
    match general_purpose::STANDARD.decode(encrypted_api_key) {
        Ok(encrypted_bytes) => {
            let mut decrypted = Vec::new();
            
            for (i, byte) in encrypted_bytes.iter().enumerate() {
                decrypted.push(byte ^ key[i % key.len()]);
            }
            
            String::from_utf8(decrypted).unwrap_or_default()
        }
        Err(_) => {
            // If decryption fails, return the original string (for backwards compatibility)
            encrypted_api_key.to_string()
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            select_video_files,
            generate_document,
            save_settings,
            load_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
