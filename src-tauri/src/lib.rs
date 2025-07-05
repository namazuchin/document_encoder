use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{Emitter, Manager};
use tokio::time::{sleep, Duration};

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
    #[serde(default = "default_language")]
    language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProgressUpdate {
    message: String,
    step: usize,
    total_steps: usize,
}

fn default_language() -> String {
    "japanese".to_string()
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
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    mime_type: Option<String>,
    #[serde(default)]
    size_bytes: Option<String>,
    #[serde(default)]
    create_time: Option<String>,
    #[serde(default)]
    update_time: Option<String>,
    #[serde(default)]
    expiration_time: Option<String>,
    #[serde(default)]
    sha256_hash: Option<String>,
    #[serde(default)]
    uri: Option<String>,
    #[serde(default)]
    state: Option<String>,
}

#[tauri::command]
async fn select_video_files(app: tauri::AppHandle) -> Result<Vec<VideoFile>, String> {
    use tauri_plugin_dialog::DialogExt;
    use tokio::sync::oneshot;

    let (tx, rx) = oneshot::channel();

    app.dialog()
        .file()
        .add_filter(
            "Video files",
            &[
                "mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "3gp", "mpg", "mpeg",
            ],
        )
        .set_title("Select video files")
        .pick_files(move |files| {
            let _ = tx.send(files);
        });

    let files = rx
        .await
        .map_err(|e| format!("Failed to receive dialog result: {}", e))?;

    match files {
        Some(paths) => {
            let mut video_files = Vec::new();
            for file_path in paths {
                let path_str = file_path.to_string();
                let path_buf = std::path::PathBuf::from(&path_str);
                if let Ok(metadata) = fs::metadata(&path_buf) {
                    let file_name = path_buf
                        .file_name()
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
async fn generate_document(files: Vec<VideoFile>, settings: AppSettings, app: tauri::AppHandle) -> Result<String, String> {
    println!("🚀 [BACKEND] Starting generate_document with {} files", files.len());
    println!("📋 [BACKEND] Settings: mode={}, language={}", settings.mode, settings.language);
    
    // Calculate total steps for progress tracking
    let total_steps = files.len() * 3 + if files.len() > 1 { 1 } else { 0 }; // Split, Upload, Generate per file + Integration
    let mut current_step = 0;
    
    // Helper function to emit progress
    let emit_progress = |app_ref: &tauri::AppHandle, step: usize, total: usize, message: String| {
        let progress = ProgressUpdate {
            message: message.clone(),
            step,
            total_steps: total,
        };
        println!("📡 [EVENT] Emitting progress: step={}/{}, message={}", step, total, message);
        if let Err(e) = app_ref.emit("progress_update", &progress) {
            println!("❌ [EVENT] Failed to emit progress event: {}", e);
        } else {
            println!("✅ [EVENT] Successfully emitted progress event");
        }
    };
    
    emit_progress(&app, current_step, total_steps, "ドキュメント生成を開始しています...".to_string());
    
    // Process files and split if necessary
    let mut processed_files = Vec::new();

    for (index, file) in files.iter().enumerate() {
        current_step += 1;
        emit_progress(&app, current_step, total_steps, format!("ファイル処理中 ({}/{}): {}", index + 1, files.len(), file.name));
        
        println!("🎬 [BACKEND] Processing file {}/{}: {}", index + 1, files.len(), file.name);
        match split_video_if_needed(&file.path).await {
            Ok(segments) => {
                if segments.len() > 1 {
                    println!("✂️ [BACKEND] Video split into {} segments", segments.len());
                    for segment in segments {
                        processed_files.push(segment);
                    }
                } else {
                    println!("✅ [BACKEND] Video is under 1 hour, no splitting needed");
                    processed_files.push(file.path.clone());
                }
            }
            Err(e) => {
                println!("❌ [BACKEND] Failed to process file {}: {}", file.name, e);
                return Err(format!("Failed to process file {}: {}", file.name, e));
            }
        }
    }

    // Upload files to Gemini API
    let mut file_uris = Vec::new();
    println!("☁️ [BACKEND] Starting upload of {} processed files to Gemini API", processed_files.len());

    for (index, file_path) in processed_files.iter().enumerate() {
        current_step += 1;
        let file_name = Path::new(file_path).file_name().and_then(|n| n.to_str()).unwrap_or("不明なファイル");
        emit_progress(&app, current_step, total_steps, format!("ファイルアップロード中 ({}/{}): {}", index + 1, processed_files.len(), file_name));
        
        println!("📤 [BACKEND] Uploading file {}/{}: {}", index + 1, processed_files.len(), file_path);
        match upload_to_gemini_with_progress(file_path, &settings.gemini_api_key, &app, current_step, total_steps).await {
            Ok(uri) => {
                println!("✅ [BACKEND] Successfully uploaded file, URI: {}", uri);
                file_uris.push(uri);
            }
            Err(e) => {
                println!("❌ [BACKEND] Failed to upload file {}: {}", file_path, e);
                return Err(format!("Failed to upload file {}: {}", file_path, e));
            }
        }
    }

    // Generate documents for each file/segment
    let mut documents = Vec::new();
    println!("🤖 [BACKEND] Starting document generation for {} uploaded files", file_uris.len());

    for (index, file_uri) in file_uris.iter().enumerate() {
        current_step += 1;
        emit_progress(&app, current_step, total_steps, format!("ドキュメント生成中 ({}/{})", index + 1, file_uris.len()));
        
        println!("📝 [BACKEND] Generating document {}/{} for URI: {}", index + 1, file_uris.len(), file_uri);
        match generate_with_gemini_with_progress(
            &[file_uri.clone()],
            &settings.mode,
            &settings.language,
            &settings.gemini_api_key,
            &app,
            current_step,
            total_steps,
        )
        .await
        {
            Ok(document) => {
                println!("✅ [BACKEND] Successfully generated document {}/{} (length: {})", index + 1, file_uris.len(), document.len());
                documents.push(document);
            }
            Err(e) => {
                println!("❌ [BACKEND] Failed to generate document for file {}: {}", file_uri, e);
                return Err(format!("Failed to generate document for file: {}", e));
            }
        }
    }

    // Integrate multiple documents if necessary
    let final_document = if documents.len() > 1 {
        current_step += 1;
        emit_progress(&app, current_step, total_steps, "複数のドキュメントを統合中...".to_string());
        
        println!("🔗 [BACKEND] Integrating {} documents into final document", documents.len());
        match integrate_documents(
            &documents,
            &settings.mode,
            &settings.language,
            &settings.gemini_api_key,
        )
        .await
        {
            Ok(integrated) => {
                println!("✅ [BACKEND] Successfully integrated documents (final length: {})", integrated.len());
                integrated
            }
            Err(e) => {
                println!("❌ [BACKEND] Failed to integrate documents: {}", e);
                return Err(format!("Failed to integrate documents: {}", e));
            }
        }
    } else {
        println!("📄 [BACKEND] Single document, no integration needed");
        documents.into_iter().next().unwrap_or_default()
    };

    emit_progress(&app, total_steps, total_steps, "ドキュメント生成が完了しました！".to_string());
    println!("🎉 [BACKEND] Document generation completed successfully (final length: {})", final_document.len());
    Ok(final_document)
}

async fn upload_to_gemini_with_progress(file_path: &str, api_key: &str, app: &tauri::AppHandle, base_step: usize, total_steps: usize) -> Result<String> {
    let emit_progress = |message: String| {
        let progress = ProgressUpdate {
            message: message.clone(),
            step: base_step,
            total_steps,
        };
        println!("📡 [UPLOAD_EVENT] Emitting progress: step={}/{}, message={}", base_step, total_steps, message);
        if let Err(e) = app.emit("progress_update", &progress) {
            println!("❌ [UPLOAD_EVENT] Failed to emit progress event: {}", e);
        } else {
            println!("✅ [UPLOAD_EVENT] Successfully emitted progress event");
        }
    };
    
    // Also create a detailed progress emitter that updates the main progress message
    let emit_detailed_progress = |detail_message: String| {
        let progress = ProgressUpdate {
            message: detail_message.clone(),
            step: base_step,
            total_steps,
        };
        if let Err(e) = app.emit("progress_update", &progress) {
            println!("❌ [UPLOAD_EVENT] Failed to emit detailed progress: {}", e);
        }
    };
    
    upload_to_gemini_internal(file_path, api_key, emit_detailed_progress).await
}

async fn upload_to_gemini_internal<F>(file_path: &str, api_key: &str, emit_progress: F) -> Result<String> 
where 
    F: Fn(String),
{
    println!("📂 [UPLOAD] Starting upload for file: {}", file_path);
    emit_progress("ファイルを読み込み中...".to_string());
    
    let client = reqwest::Client::new();
    let file_data = fs::read(file_path)?;
    let file_size = file_data.len();
    let file_name_for_display = Path::new(file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unnamed_video")
        .to_string();
    let mime_type = get_mime_type(file_path);
    
    println!("📊 [UPLOAD] File info - Name: {}, Size: {} bytes, MIME: {}", file_name_for_display, file_size, mime_type);

    // 1. Start resumable upload session
    println!("🌐 [UPLOAD] Step 1: Starting resumable upload session");
    emit_progress("アップロードセッションを開始中...".to_string());
    
    let start_request_body = serde_json::json!({
        "file": {
            "display_name": file_name_for_display
        }
    });

    let start_response = client
        .post(format!(
            "https://generativelanguage.googleapis.com/upload/v1beta/files?key={}",
            api_key
        ))
        .header("X-Goog-Upload-Protocol", "resumable")
        .header("X-Goog-Upload-Command", "start")
        .header("X-Goog-Upload-Header-Content-Length", file_size.to_string())
        .header("X-Goog-Upload-Header-Content-Type", &mime_type)
        .header("Content-Type", "application/json")
        .json(&start_request_body)
        .send()
        .await?;

    if !start_response.status().is_success() {
        let error_text = start_response.text().await?;
        println!("❌ [UPLOAD] Failed to start resumable upload: {}", error_text);
        return Err(anyhow::anyhow!(
            "Failed to start resumable upload: {}",
            error_text
        ));
    }

    let upload_url = match start_response.headers().get("X-Goog-Upload-URL") {
        Some(url) => {
            let url_str = url.to_str()?.to_string();
            println!("✅ [UPLOAD] Received upload URL: {}", url_str);
            url_str
        }
        None => {
            println!("❌ [UPLOAD] Did not receive upload URL in response headers");
            return Err(anyhow::anyhow!("Did not receive upload URL"));
        }
    };

    // 2. Upload the file bytes
    println!("📤 [UPLOAD] Step 2: Uploading file bytes ({} bytes)", file_size);
    emit_progress(format!("ファイルをアップロード中... ({:.1} MB)", file_size as f64 / 1_000_000.0));
    
    let upload_response = client
        .post(&upload_url)
        .header("Content-Length", file_size.to_string())
        .header("X-Goog-Upload-Offset", "0")
        .header("X-Goog-Upload-Command", "upload, finalize")
        .body(file_data)
        .send()
        .await?;

    if !upload_response.status().is_success() {
        let error_text = upload_response.text().await?;
        println!("❌ [UPLOAD] Failed to upload file content: {}", error_text);
        return Err(anyhow::anyhow!(
            "Failed to upload file content: {}",
            error_text
        ));
    }

    println!("✅ [UPLOAD] File upload completed successfully");
    let upload_info: GeminiUploadResponse = upload_response.json().await?;
    let file_name_on_server = upload_info.file.name.clone();
    println!("📋 [UPLOAD] File registered on server as: {}", file_name_on_server);

    // 3. Poll for file processing to complete.
    println!("⏳ [UPLOAD] Step 3: Waiting for file processing to complete...");
    emit_progress("ファイル処理の完了を待機中...".to_string());
    
    let mut retry_count = 0;
    let max_retries = 60; // 最大10分間待機

    loop {
        retry_count += 1;
        emit_progress(format!("ファイル処理状況を確認中... ({}/{}回目)", retry_count, max_retries));
        println!("🔄 [UPLOAD] Checking file status (attempt {}/{})", retry_count, max_retries);
        
        let get_response = client
            .get(format!(
                "https://generativelanguage.googleapis.com/v1beta/{}?key={}",
                file_name_on_server, api_key
            ))
            .send()
            .await?;

        if !get_response.status().is_success() {
            let error_text = get_response.text().await?;
            println!("❌ [UPLOAD] Failed to get file status: {}", error_text);
            return Err(anyhow::anyhow!(
                "Failed to get file status: {}",
                error_text
            ));
        }

        let file_info: GeminiFileInfo = get_response.json().await?;

        if let Some(state) = &file_info.state {
            println!("📊 [UPLOAD] File state: {}", state);
            match state.as_str() {
                "ACTIVE" => {
                    if let Some(uri) = file_info.uri {
                        emit_progress("ファイル処理完了！ドキュメント生成準備中...".to_string());
                        println!("🎉 [UPLOAD] File processing completed! URI: {}", uri);
                        return Ok(uri);
                    } else {
                        emit_progress("エラー: ファイルは処理されましたがURIが見つかりません".to_string());
                        println!("❌ [UPLOAD] File is ACTIVE but URI is missing");
                        return Err(anyhow::anyhow!("File is ACTIVE but URI is missing."));
                    }
                }
                "PROCESSING" => {
                    if retry_count > max_retries {
                        emit_progress("タイムアウト: ファイル処理に時間がかかりすぎています".to_string());
                        println!("⏰ [UPLOAD] File processing timeout after {} attempts", max_retries);
                        return Err(anyhow::anyhow!("File processing timeout."));
                    }
                    emit_progress(format!("ファイル処理中... 10秒後に再確認 ({}/{}回目)", retry_count, max_retries));
                    println!("⏳ [UPLOAD] File still processing, waiting 10 seconds...");
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
                "FAILED" => {
                    emit_progress("エラー: サーバーでファイル処理に失敗しました".to_string());
                    println!("❌ [UPLOAD] File processing failed on the server");
                    return Err(anyhow::anyhow!("File processing failed on the server."));
                }
                _ => {
                    emit_progress(format!("不明な状態: {}", state));
                    println!("❓ [UPLOAD] Unknown file state received: {}", state);
                    return Err(anyhow::anyhow!("Unknown file state received: {}", state));
                }
            }
        } else {
            println!("📊 [UPLOAD] No state field in response, assuming still processing");
            if retry_count > max_retries {
                emit_progress("タイムアウト: ファイル状態の確認に失敗しました".to_string());
                println!("⏰ [UPLOAD] File processing timeout (no state) after {} attempts", max_retries);
                return Err(anyhow::anyhow!("File processing timeout (no state)."));
            }
            emit_progress(format!("状態不明のためファイル処理中と仮定... ({}/{}回目)", retry_count, max_retries));
            sleep(Duration::from_secs(5)).await;
        }
    }
}

async fn generate_with_gemini_with_progress(
    file_uris: &[String],
    mode: &str,
    language: &str,
    api_key: &str,
    app: &tauri::AppHandle,
    base_step: usize,
    total_steps: usize,
) -> Result<String> {
    let emit_progress = |message: String| {
        let progress = ProgressUpdate {
            message: message.clone(),
            step: base_step,
            total_steps,
        };
        if let Err(e) = app.emit("progress_update", &progress) {
            println!("❌ [GENERATE_EVENT] Failed to emit progress: {}", e);
        }
    };
    
    generate_with_gemini_internal(file_uris, mode, language, api_key, emit_progress).await
}

async fn generate_with_gemini(
    file_uris: &[String],
    mode: &str,
    language: &str,
    api_key: &str,
) -> Result<String> {
    generate_with_gemini_internal(file_uris, mode, language, api_key, |_| {}).await
}

async fn generate_with_gemini_internal<F>(
    file_uris: &[String],
    mode: &str,
    language: &str,
    api_key: &str,
    emit_progress: F,
) -> Result<String> 
where
    F: Fn(String),
{
    println!("🤖 [GENERATE] Starting document generation with Gemini API");
    println!("📋 [GENERATE] Mode: {}, Language: {}, Files: {}", mode, language, file_uris.len());
    emit_progress("AIによるドキュメント生成を準備中...".to_string());
    let client = reqwest::Client::new();

    let language_instruction = match language {
        "english" => "Please write the document in English",
        "japanese" | _ => "Please write the document in Japanese",
    };

    let prompt = match mode {
        "manual" => format!("Please analyze the uploaded video(s) and create a comprehensive manual document. The document should include:
        
        1. Overview of the content
        2. Step-by-step instructions for all procedures shown
        3. Key points and important notes
        4. Troubleshooting tips where applicable
        
        {} and format it in a clear, professional manner.", language_instruction),
        "specification" => format!("Please analyze the uploaded video(s) and create a detailed specification document. The document should include:
        
        1. System overview and architecture
        2. Functional specifications
        3. Technical requirements
        4. Interface specifications
        5. Performance criteria
        6. Implementation details
        
        {} and format it in a clear, professional manner.", language_instruction),
        _ => format!("Please analyze the uploaded video(s) and create a comprehensive document based on the content. {}", language_instruction),
    };

    let mut parts = vec![GeminiPart::Text {
        text: prompt.to_string(),
    }];

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

    println!("🌐 [GENERATE] Sending request to Gemini API...");
    emit_progress("Gemini AIにドキュメント生成を依頼中...".to_string());
    let response = client
        .post(format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro-preview-06-05:generateContent?key={}", api_key))
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        println!("✅ [GENERATE] Received successful response from Gemini API");
        emit_progress("AIの応答を受信中...".to_string());
        let gemini_response: GeminiResponse = response.json().await?;
        if let Some(candidate) = gemini_response.candidates.first() {
            if let Some(part) = candidate.content.parts.first() {
                if let GeminiPart::Text { text } = part {
                    println!("📝 [GENERATE] Generated document length: {} characters", text.len());
                    emit_progress(format!("ドキュメント生成完了！ ({}文字)", text.len()));
                    return Ok(text.clone());
                }
            }
        }
        println!("❌ [GENERATE] No text content found in response");
        emit_progress("エラー: AIの応答にテキストが含まれていません".to_string());
        Err(anyhow::anyhow!("No text content in response"))
    } else {
        let error_text = response.text().await?;
        println!("❌ [GENERATE] API request failed: {}", error_text);
        emit_progress(format!("エラー: AI生成に失敗しました - {}", error_text));
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
    }
    .to_string()
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
        let output_path = temp_dir.path().join(format!(
            "{}_part_{:03}.{}",
            file_stem,
            i + 1,
            file_extension
        ));

        // Use ffmpeg to split the video
        let output = Command::new("ffmpeg")
            .args(&[
                "-i",
                video_path,
                "-ss",
                &start_time.to_string(),
                "-t",
                &segment_duration.to_string(),
                "-c",
                "copy", // Copy without re-encoding for speed
                "-avoid_negative_ts",
                "make_zero",
                output_path.to_str().unwrap(),
                "-y", // Overwrite output files
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
                return Err(anyhow::anyhow!(
                    "Failed to execute ffmpeg: {}. Make sure ffmpeg is installed and in your PATH.",
                    e
                ));
            }
        }
    }

    Ok(segments)
}

async fn get_video_duration(video_path: &str) -> Result<f64> {
    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "quiet",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            video_path,
        ])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let duration_str = String::from_utf8_lossy(&result.stdout);
                let duration: f64 = duration_str
                    .trim()
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Failed to parse duration"))?;
                Ok(duration)
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                Err(anyhow::anyhow!("ffprobe failed: {}", stderr))
            }
        }
        Err(e) => Err(anyhow::anyhow!(
            "Failed to execute ffprobe: {}. Make sure ffmpeg is installed and in your PATH.",
            e
        )),
    }
}

async fn integrate_documents(
    documents: &[String],
    mode: &str,
    language: &str,
    api_key: &str,
) -> Result<String> {
    let client = reqwest::Client::new();

    let language_instruction = match language {
        "english" => "Please write the integrated document in English",
        "japanese" | _ => "Please write the integrated document in Japanese",
    };

    let integration_prompt = match mode {
        "manual" => {
            format!(
                "Please integrate the following manual documents into one comprehensive, cohesive manual. \
                Ensure proper flow, eliminate redundancy, and organize the content logically. {}:\n\n{}",
                language_instruction,
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
                Ensure technical consistency, proper organization, and eliminate redundancy. {}:\n\n{}",
                language_instruction,
                documents.iter()
                    .enumerate()
                    .map(|(i, doc)| format!("=== Specification Part {} ===\n{}\n", i + 1, doc))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
        _ => {
            format!(
                "Please integrate the following documents into one comprehensive document. {}:\n\n{}",
                language_instruction,
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
            parts: vec![GeminiPart::Text {
                text: integration_prompt,
            }],
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
        Err(anyhow::anyhow!(
            "Document integration failed: {}",
            error_text
        ))
    }
}

#[tauri::command]
async fn save_settings(settings: AppSettings, app: tauri::AppHandle) -> Result<(), String> {
    // println!("save_settings called with: {:?}", settings);
    let config_path = get_config_file_path(&app)?;
    // println!("Config path: {:?}", config_path);

    // Ensure the parent directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Encrypt sensitive data before saving
    let safe_settings = AppSettings {
        mode: settings.mode,
        gemini_api_key: encrypt_api_key(&settings.gemini_api_key),
        language: settings.language,
    };

    let config_json = serde_json::to_string_pretty(&safe_settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(&config_path, config_json)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

    // println!("Settings saved successfully to: {:?}", config_path);
    Ok(())
}

#[tauri::command]
async fn load_settings(app: tauri::AppHandle) -> Result<Option<AppSettings>, String> {
    // println!("load_settings called");
    let config_path = get_config_file_path(&app)?;
    // println!("Config path: {:?}", config_path);

    if !config_path.exists() {
        // println!("Config file does not exist");
        return Ok(None);
    }

    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;

    let mut settings: AppSettings = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to parse settings file: {}", e))?;

    // Decrypt sensitive data after loading
    settings.gemini_api_key = decrypt_api_key(&settings.gemini_api_key);

    // println!("Loaded and decrypted settings: {:?}", settings);
    Ok(Some(settings))
}

fn get_config_file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_dir = app
        .path()
        .app_config_dir()
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

    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD.encode(encrypted)
}

fn decrypt_api_key(encrypted_api_key: &str) -> String {
    // Decrypt using the same XOR method
    let key = b"document_encoder_key_2024";

    use base64::{engine::general_purpose, Engine as _};
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
