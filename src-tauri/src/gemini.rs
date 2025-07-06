use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::Emitter;
use tokio::time::{sleep, Duration};

use crate::types::{
    GeminiRequest, GeminiContent, GeminiPart, GeminiFileData, GeminiResponse,
    GeminiUploadResponse, ProgressUpdate
};

// Extended GeminiFileInfo for internal use (different from types.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiFileInfo {
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
    let upload_info: GeminiUploadResponse = upload_response.json().await?;
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

pub async fn generate_with_gemini_internal<F>(
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
    println!(
        "📋 [GENERATE] Mode: {}, Language: {}, Files: {}",
        mode,
        language,
        file_uris.len()
    );
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
        .post(format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro-preview-06-05:generateContent?key={}", api_key))
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