use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Emitter, Manager};

mod gemini;
mod types;
mod file;
mod video;

use crate::gemini::{
    upload_to_gemini_with_progress,
    generate_with_gemini_with_progress,
    integrate_documents,
};
use crate::types::{
    VideoFile, AppSettings, ProgressUpdate,
};
use crate::file::{
    select_video_files,
    select_save_directory,
    save_document_to_file,
};
use crate::video::{
    split_video_if_needed,
};



#[tauri::command]
async fn generate_document(
    files: Vec<VideoFile>,
    settings: AppSettings,
    app: tauri::AppHandle,
) -> Result<String, String> {
    println!(
        "🚀 [BACKEND] Starting generate_document with {} files",
        files.len()
    );
    println!(
        "📋 [BACKEND] Settings: mode={}, language={}",
        settings.mode, settings.language
    );

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
        println!(
            "📡 [EVENT] Emitting progress: step={}/{}, message={}",
            step, total, message
        );
        if let Err(e) = app_ref.emit("progress_update", &progress) {
            println!("❌ [EVENT] Failed to emit progress event: {}", e);
        } else {
            println!("✅ [EVENT] Successfully emitted progress event");
        }
    };

    emit_progress(
        &app,
        current_step,
        total_steps,
        "ドキュメント生成を開始しています...".to_string(),
    );

    // Process files and split if necessary
    let mut processed_files = Vec::new();

    for (index, file) in files.iter().enumerate() {
        current_step += 1;
        emit_progress(
            &app,
            current_step,
            total_steps,
            format!(
                "ファイル処理中 ({}/{}): {}",
                index + 1,
                files.len(),
                file.name
            ),
        );

        println!(
            "🎬 [BACKEND] Processing file {}/{}: {}",
            index + 1,
            files.len(),
            file.name
        );
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
    println!(
        "☁️ [BACKEND] Starting upload of {} processed files to Gemini API",
        processed_files.len()
    );

    for (index, file_path) in processed_files.iter().enumerate() {
        current_step += 1;
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("不明なファイル");
        emit_progress(
            &app,
            current_step,
            total_steps,
            format!(
                "ファイルアップロード中 ({}/{}): {}",
                index + 1,
                processed_files.len(),
                file_name
            ),
        );

        println!(
            "📤 [BACKEND] Uploading file {}/{}: {}",
            index + 1,
            processed_files.len(),
            file_path
        );
        match upload_to_gemini_with_progress(
            file_path,
            &settings.gemini_api_key,
            &app,
            current_step,
            total_steps,
        )
        .await
        {
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
    println!(
        "🤖 [BACKEND] Starting document generation for {} uploaded files",
        file_uris.len()
    );

    for (index, file_uri) in file_uris.iter().enumerate() {
        current_step += 1;
        emit_progress(
            &app,
            current_step,
            total_steps,
            format!("ドキュメント生成中 ({}/{})", index + 1, file_uris.len()),
        );

        println!(
            "📝 [BACKEND] Generating document {}/{} for URI: {}",
            index + 1,
            file_uris.len(),
            file_uri
        );
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
                println!(
                    "✅ [BACKEND] Successfully generated document {}/{} (length: {})",
                    index + 1,
                    file_uris.len(),
                    document.len()
                );
                documents.push(document);
            }
            Err(e) => {
                println!(
                    "❌ [BACKEND] Failed to generate document for file {}: {}",
                    file_uri, e
                );
                return Err(format!("Failed to generate document for file: {}", e));
            }
        }
    }

    // Integrate multiple documents if necessary
    let final_document = if documents.len() > 1 {
        current_step += 1;
        emit_progress(
            &app,
            current_step,
            total_steps,
            "複数のドキュメントを統合中...".to_string(),
        );

        println!(
            "🔗 [BACKEND] Integrating {} documents into final document",
            documents.len()
        );
        match integrate_documents(
            &documents,
            &settings.mode,
            &settings.language,
            &settings.gemini_api_key,
        )
        .await
        {
            Ok(integrated) => {
                println!(
                    "✅ [BACKEND] Successfully integrated documents (final length: {})",
                    integrated.len()
                );
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

    emit_progress(
        &app,
        total_steps,
        total_steps,
        "ドキュメント生成が完了しました！".to_string(),
    );
    println!(
        "🎉 [BACKEND] Document generation completed successfully (final length: {})",
        final_document.len()
    );
    Ok(final_document)
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
            load_settings,
            select_save_directory,
            save_document_to_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
