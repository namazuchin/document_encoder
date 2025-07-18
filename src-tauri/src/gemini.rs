use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::Emitter;
use tokio::time::{sleep, Duration};

use crate::types::{
    GeminiRequest, GeminiContent, GeminiPart, GeminiFileData, GeminiResponse,
    GeminiUploadResponse, GeminiGenerationConfig, ProgressUpdate, ImageEmbedFrequency
};

// Internal GeminiFileInfo for status polling (with optional fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiFileStatus {
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<String>,
    #[serde(default)]
    pub create_time: Option<String>,
    #[serde(default)]
    pub update_time: Option<String>,
    #[serde(default)]
    pub expiration_time: Option<String>,
    #[serde(default)]
    pub sha256_hash: Option<String>,
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

pub async fn upload_to_gemini_with_progress(
    file_path: &str,
    api_key: &str,
    app: &tauri::AppHandle,
    base_step: usize,
    total_steps: usize,
) -> Result<String> {
    let _emit_progress = |message: String| {
        let progress = ProgressUpdate {
            message: message.clone(),
            step: base_step,
            total_steps,
        };
        println!(
            "📡 [UPLOAD_EVENT] Emitting progress: step={}/{}, message={}",
            base_step, total_steps, message
        );
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

pub async fn upload_to_gemini_internal<F>(
    file_path: &str,
    api_key: &str,
    emit_progress: F,
) -> Result<String>
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

    println!(
        "📊 [UPLOAD] File info - Name: {}, Size: {} bytes, MIME: {}",
        file_name_for_display, file_size, mime_type
    );

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
        println!(
            "❌ [UPLOAD] Failed to start resumable upload: {}",
            error_text
        );
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
    println!(
        "📤 [UPLOAD] Step 2: Uploading file bytes ({} bytes)",
        file_size
    );
    emit_progress(format!(
        "ファイルをアップロード中... ({:.1} MB)",
        file_size as f64 / 1_000_000.0
    ));

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
    let upload_info: GeminiUploadResponse = upload_response.json().await
        .map_err(|e| anyhow::anyhow!("Failed to parse upload response: {}", e))?;
    let file_name_on_server = upload_info.file.name.clone();
    println!(
        "📋 [UPLOAD] File registered on server as: {}",
        file_name_on_server
    );

    // 3. Poll for file processing to complete.
    println!("⏳ [UPLOAD] Step 3: Waiting for file processing to complete...");
    emit_progress("ファイル処理の完了を待機中...".to_string());

    let mut retry_count = 0;
    let max_retries = 60; // 最大10分間待機

    loop {
        retry_count += 1;
        emit_progress(format!(
            "ファイル処理状況を確認中... ({}/{}回目)",
            retry_count, max_retries
        ));
        println!(
            "🔄 [UPLOAD] Checking file status (attempt {}/{})",
            retry_count, max_retries
        );

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
            return Err(anyhow::anyhow!("Failed to get file status: {}", error_text));
        }

        let file_info: GeminiFileStatus = get_response.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse file status response: {}", e))?;

        if let Some(state) = &file_info.state {
            println!("📊 [UPLOAD] File state: {}", state);
            match state.as_str() {
                "ACTIVE" => {
                    if let Some(uri) = file_info.uri {
                        emit_progress("ファイル処理完了！ドキュメント生成準備中...".to_string());
                        println!("🎉 [UPLOAD] File processing completed! URI: {}", uri);
                        return Ok(uri);
                    } else {
                        emit_progress(
                            "エラー: ファイルは処理されましたがURIが見つかりません".to_string(),
                        );
                        println!("❌ [UPLOAD] File is ACTIVE but URI is missing");
                        return Err(anyhow::anyhow!("File is ACTIVE but URI is missing."));
                    }
                }
                "PROCESSING" => {
                    if retry_count > max_retries {
                        emit_progress(
                            "タイムアウト: ファイル処理に時間がかかりすぎています".to_string(),
                        );
                        println!(
                            "⏰ [UPLOAD] File processing timeout after {} attempts",
                            max_retries
                        );
                        return Err(anyhow::anyhow!("File processing timeout."));
                    }
                    emit_progress(format!(
                        "ファイル処理中... 10秒後に再確認 ({}/{}回目)",
                        retry_count, max_retries
                    ));
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
                println!(
                    "⏰ [UPLOAD] File processing timeout (no state) after {} attempts",
                    max_retries
                );
                return Err(anyhow::anyhow!("File processing timeout (no state)."));
            }
            emit_progress(format!(
                "状態不明のためファイル処理中と仮定... ({}/{}回目)",
                retry_count, max_retries
            ));
            sleep(Duration::from_secs(5)).await;
        }
    }
}

pub async fn generate_with_gemini_with_progress(
    file_uris: &[String],
    language: &str,
    api_key: &str,
    temperature: f64,
    custom_prompt: Option<&str>,
    model: &str,
    embed_images: bool,
    image_embed_frequency: &ImageEmbedFrequency,
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

    generate_with_gemini_internal(file_uris, language, api_key, temperature, custom_prompt, model, embed_images, image_embed_frequency, emit_progress).await
}

pub async fn generate_with_gemini_internal<F>(
    file_uris: &[String],
    language: &str,
    api_key: &str,
    temperature: f64,
    custom_prompt: Option<&str>,
    model: &str,
    embed_images: bool,
    image_embed_frequency: &ImageEmbedFrequency,
    emit_progress: F,
) -> Result<String>
where
    F: Fn(String),
{
    println!("🤖 [GENERATE] Starting document generation with Gemini API");
    println!(
        "📋 [GENERATE] Language: {}, Files: {}",
        language,
        file_uris.len()
    );
    emit_progress("AIによるドキュメント生成を準備中...".to_string());
    let client = reqwest::Client::new();

    let prompt = if let Some(custom) = custom_prompt {
        let mut final_prompt = custom.to_string();
        if embed_images {
            let image_instruction = get_image_instruction(image_embed_frequency);
            final_prompt.push_str(&image_instruction);
        }
        final_prompt
    } else {
        let language_instruction = match language {
            "english" => "Please write the document in English",
            "japanese" | _ => "Please write the document in Japanese",
        };

        let mut base_prompt = format!("Please analyze the uploaded video(s) and create a comprehensive document based on the content. The document should include:
        
        1. Overview of the content
        2. Key points and important information
        3. Step-by-step instructions or procedures if applicable
        4. Technical details and specifications
        5. Any relevant notes or recommendations
        
        {} and format it in a clear, professional manner.", language_instruction);
        
        if embed_images {
            let image_instruction = get_image_instruction(image_embed_frequency);
            base_prompt.push_str(&image_instruction);
        }
        
        base_prompt
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
        generation_config: if temperature > 0.0 {
            Some(GeminiGenerationConfig {
                temperature: Some(temperature),
            })
        } else {
            None
        },
    };

    println!("🌐 [GENERATE] Sending request to Gemini API...");
    emit_progress("Gemini AIにドキュメント生成を依頼中...".to_string());
    let response = client
        .post(format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", model, api_key))
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
                    println!(
                        "📝 [GENERATE] Generated document length: {} characters",
                        text.len()
                    );
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

pub async fn integrate_documents(
    documents: &[String],
    language: &str,
    api_key: &str,
    temperature: f64,
    custom_prompt: Option<&str>,
    model: &str,
) -> Result<String> {
    let client = reqwest::Client::new();

    let integration_prompt = if let Some(custom) = custom_prompt {
        format!("{}\n\n=== Documents to integrate ===\n{}", 
            custom, 
            documents.iter()
                .enumerate()
                .map(|(i, doc)| format!("=== Document {} ===\n{}\n", i + 1, doc))
                .collect::<Vec<_>>()
                .join("\n")
        )
    } else {
        let language_instruction = match language {
            "english" => "Please write the integrated document in English",
            "japanese" | _ => "Please write the integrated document in Japanese",
        };

        format!(
            "Please integrate the following documents into one comprehensive, cohesive document. \
            Ensure proper flow, eliminate redundancy, organize the content logically, and maintain consistency throughout. {}:\n\n{}",
            language_instruction,
            documents.iter()
                .enumerate()
                .map(|(i, doc)| format!("=== Document {} ===\n{}\n", i + 1, doc))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let request = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart::Text {
                text: integration_prompt,
            }],
        }],
        generation_config: if temperature > 0.0 {
            Some(GeminiGenerationConfig {
                temperature: Some(temperature),
            })
        } else {
            None
        },
    };

    let response = client
        .post(format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", model, api_key))
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

pub fn get_mime_type(file_path: &str) -> String {
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

/// Generates image instruction based on embedding frequency
fn get_image_instruction(frequency: &ImageEmbedFrequency) -> String {
    match frequency {
        ImageEmbedFrequency::Minimal => {
            "\n\nIMPORTANT: When describing the most critical visual elements or key points in the document, please include screenshot references using this exact format: [Screenshot: XX:XXs] where XX:XX is the timestamp in MM:SS format (e.g., [Screenshot: 00:14s], [Screenshot: 01:23s]). Use these references sparingly, only for the most important moments that are essential for understanding.".to_string()
        },
        ImageEmbedFrequency::Moderate => {
            "\n\nIMPORTANT: When describing visual elements or important points in the document, please include screenshot references using this exact format: [Screenshot: XX:XXs] where XX:XX is the timestamp in MM:SS format (e.g., [Screenshot: 00:14s], [Screenshot: 01:23s]). Use these references to mark key moments that would benefit from visual representation.".to_string()
        },
        ImageEmbedFrequency::Detailed => {
            "\n\nIMPORTANT: When describing visual elements, UI components, or detailed explanations in the document, please include screenshot references using this exact format: [Screenshot: XX:XXs] where XX:XX is the timestamp in MM:SS format (e.g., [Screenshot: 00:14s], [Screenshot: 01:23s]). Use these references frequently to provide detailed visual context for readers.".to_string()
        }
    }
}

/// Parses timestamp string in various formats (MM:SS or SS.SS)
fn parse_timestamp(timestamp_str: &str) -> f64 {
    if timestamp_str.contains(':') {
        // Format: MM:SS or MM:SS.SS
        let parts: Vec<&str> = timestamp_str.split(':').collect();
        if parts.len() == 2 {
            let minutes = parts[0].parse::<f64>().unwrap_or(0.0);
            let seconds = parts[1].parse::<f64>().unwrap_or(0.0);
            return minutes * 60.0 + seconds;
        }
    } else {
        // Format: SS.SS
        return timestamp_str.parse::<f64>().unwrap_or(0.0);
    }
    0.0
}

/// Processes the generated document to extract screenshot placeholders and replace them with images
pub async fn process_document_with_images(
    document: &str,
    video_files: &[String],
    output_directory: &str,
    _image_embed_frequency: &ImageEmbedFrequency,
) -> Result<String> {
    // Create images directory
    let images_dir = Path::new(output_directory).join("images");
    if !images_dir.exists() {
        fs::create_dir_all(&images_dir)?;
    }

    // Extract screenshot placeholders using regex
    // Updated to handle formats like [Screenshot: 00:14s] and [Screenshot: 123.45s]
    let re = Regex::new(r"\[Screenshot:\s*(\d{1,2}:\d{2}(?:\.\d+)?|\d+(?:\.\d+)?)\s*s\]").unwrap();
    let mut processed_document = document.to_string();
    let mut image_counter = 1;

    // Collect all matches first to avoid borrowing issues
    let matches: Vec<(String, f64)> = re
        .captures_iter(document)
        .map(|caps| {
            let full_match = caps[0].to_string();
            let timestamp_str = &caps[1];
            let timestamp = parse_timestamp(timestamp_str);
            (full_match, timestamp)
        })
        .collect();
    
    println!("📊 [IMAGE] Found {} screenshot references to process", matches.len());

    // Get video durations to help determine which video contains the timestamp
    let mut video_durations = Vec::new();
    for video_path in video_files {
        match crate::video::get_video_duration(video_path).await {
            Ok(duration) => video_durations.push(duration),
            Err(e) => {
                println!("⚠️ Failed to get duration for {}: {}", video_path, e);
                video_durations.push(f64::INFINITY); // Assume infinite duration if we can't get it
            }
        }
    }

    for (placeholder, timestamp) in matches {
        let mut frame_extracted = false;
        
        // First, try to find the most appropriate video based on timestamp and duration
        let mut video_candidates: Vec<(usize, &String)> = video_files
            .iter()
            .enumerate()
            .filter(|(i, _)| timestamp <= video_durations[*i])
            .collect();
        
        // If no video can contain this timestamp, try all videos as fallback
        if video_candidates.is_empty() {
            video_candidates = video_files.iter().enumerate().collect();
        }
        
        // Try to extract frame from candidate videos
        for (video_index, video_path) in video_candidates {
            let video_no = video_index + 1; // 1-based indexing
            // Replace decimal point with underscore for filename compatibility
            let timestamp_str = timestamp.to_string().replace('.', "_");
            let image_filename = format!("image-{}-{}s.png", video_no, timestamp_str);
            let image_path = images_dir.join(&image_filename);
            
            // Extract frame from video
            match crate::video::extract_frame_from_video(
                video_path,
                timestamp,
                image_path.to_str().unwrap(),
            ).await {
                Ok(_) => {
                    let relative_image_path = format!("./images/{}", image_filename);
                    let markdown_image = format!("![Screenshot {}]({})", image_counter, relative_image_path);
                    processed_document = processed_document.replace(&placeholder, &markdown_image);
                    image_counter += 1;
                    frame_extracted = true;
                    println!("✅ Successfully extracted frame from video {} at {}s", video_no, timestamp);
                    break; // Stop trying other videos once successful
                }
                Err(e) => {
                    println!("⚠️ Failed to extract frame from video {} at {}s: {}", video_no, timestamp, e);
                    // Continue to try next video
                }
            }
        }
        
        // If no video could provide the frame, remove the placeholder
        if !frame_extracted {
            println!("❌ Failed to extract frame at {}s from any video", timestamp);
            processed_document = processed_document.replace(&placeholder, "");
        }
    }

    Ok(processed_document)
}