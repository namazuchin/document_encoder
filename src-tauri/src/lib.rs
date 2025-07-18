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
    VideoFile, AppSettings, ProgressUpdate, PromptPreset,
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
        "📋 [BACKEND] Settings: language={}",
        settings.language
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
        match split_video_if_needed(&PathBuf::from(&file.path)).await {
            Ok(segments) => {
                if segments.len() > 1 {
                    println!("✂️ [BACKEND] Video split into {} segments", segments.len());
                    for segment in segments {
                        processed_files.push(segment);
                    }
                } else {
                    println!("✅ [BACKEND] Video is under 1 hour, no splitting needed");
                    processed_files.push(PathBuf::from(&file.path));
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
            file_path.display()
        );
        match upload_to_gemini_with_progress(
            &file_path.to_string_lossy(),
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
                println!("❌ [BACKEND] Failed to upload file {}: {}", file_path.display(), e);
                return Err(format!("Failed to upload file {}: {}", file_path.display(), e));
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
            &settings.language,
            &settings.gemini_api_key,
            settings.temperature,
            settings.custom_prompt.as_deref(),
            &settings.gemini_model,
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
            &settings.language,
            &settings.gemini_api_key,
            settings.temperature,
            settings.custom_prompt.as_deref(),
            &settings.gemini_model,
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
        gemini_api_key: encrypt_api_key(&settings.gemini_api_key),
        language: settings.language,
        temperature: settings.temperature,
        custom_prompt: settings.custom_prompt,
        gemini_model: settings.gemini_model,
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

fn get_prompt_presets_file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config directory: {}", e))?;

    Ok(app_dir.join("prompt_presets.xml"))
}

#[tauri::command]
async fn load_prompt_presets(app: tauri::AppHandle) -> Result<Vec<PromptPreset>, String> {
    let presets_path = get_prompt_presets_file_path(&app)?;
    
    if !presets_path.exists() {
        // Create default presets if file doesn't exist
        let default_presets = vec![
            PromptPreset {
                id: "default_manual".to_string(),
                name: "デフォルト（マニュアル）".to_string(),
                prompt: "この動画の内容を詳細に分析し、ユーザーマニュアルとして構成してください。操作手順、注意点、トラブルシューティングを含めて説明してください。".to_string(),
                is_default: true,
            },
            PromptPreset {
                id: "default_specification".to_string(),
                name: "デフォルト（仕様書）".to_string(),
                prompt: "この動画の内容を技術仕様書として構成してください。システムの概要、機能詳細、API仕様、データ構造を含めて説明してください。".to_string(),
                is_default: true,
            },
        ];
        
        save_prompt_presets_to_file(&default_presets, &presets_path)?;
        return Ok(default_presets);
    }

    let content = fs::read_to_string(&presets_path)
        .map_err(|e| format!("Failed to read presets file: {}", e))?;

    let mut loaded_presets = parse_prompt_presets_xml(&content)?;
    
    // Check if default presets exist and restore if missing
    let _has_default_manual = loaded_presets.iter().any(|p| p.id == "default_manual" && p.is_default);
    let _has_default_specification = loaded_presets.iter().any(|p| p.id == "default_specification" && p.is_default);
    
    let mut needs_save = false;
    
    // Fix existing default presets that might have is_default=false
    for preset in &mut loaded_presets {
        if (preset.id == "default_manual" || preset.id == "default_specification") && !preset.is_default {
            preset.is_default = true;
            needs_save = true;
            println!("🔧 [PRESETS] Fixed is_default flag for preset: {}", preset.id);
        }
    }
    
    // Re-check after fixing flags
    let has_default_manual_after_fix = loaded_presets.iter().any(|p| p.id == "default_manual" && p.is_default);
    let has_default_specification_after_fix = loaded_presets.iter().any(|p| p.id == "default_specification" && p.is_default);
    
    if !has_default_manual_after_fix {
        loaded_presets.push(PromptPreset {
            id: "default_manual".to_string(),
            name: "デフォルト（マニュアル）".to_string(),
            prompt: "この動画の内容を詳細に分析し、ユーザーマニュアルとして構成してください。操作手順、注意点、トラブルシューティングを含めて説明してください。".to_string(),
            is_default: true,
        });
        needs_save = true;
        println!("🔧 [PRESETS] Restored missing default manual preset");
    }
    
    if !has_default_specification_after_fix {
        loaded_presets.push(PromptPreset {
            id: "default_specification".to_string(),
            name: "デフォルト（仕様書）".to_string(),
            prompt: "この動画の内容を技術仕様書として構成してください。システムの概要、機能詳細、API仕様、データ構造を含めて説明してください。".to_string(),
            is_default: true,
        });
        needs_save = true;
        println!("🔧 [PRESETS] Restored missing default specification preset");
    }
    
    // Save the restored presets if any were missing
    if needs_save {
        if let Err(e) = save_prompt_presets_to_file(&loaded_presets, &presets_path) {
            println!("⚠️ [PRESETS] Failed to save restored default presets: {}", e);
        } else {
            println!("✅ [PRESETS] Successfully saved restored default presets");
        }
    }
    
    Ok(loaded_presets)
}

#[tauri::command]
async fn save_prompt_presets(presets: Vec<PromptPreset>, app: tauri::AppHandle) -> Result<(), String> {
    let presets_path = get_prompt_presets_file_path(&app)?;
    
    // Check if default presets are being preserved
    let has_default_manual = presets.iter().any(|p| p.id == "default_manual" && p.is_default);
    let has_default_specification = presets.iter().any(|p| p.id == "default_specification" && p.is_default);
    
    let mut final_presets = presets;
    
    // Ensure default presets are always present
    if !has_default_manual {
        final_presets.push(PromptPreset {
            id: "default_manual".to_string(),
            name: "デフォルト（マニュアル）".to_string(),
            prompt: "この動画の内容を詳細に分析し、ユーザーマニュアルとして構成してください。操作手順、注意点、トラブルシューティングを含めて説明してください。".to_string(),
            is_default: true,
        });
    }
    
    if !has_default_specification {
        final_presets.push(PromptPreset {
            id: "default_specification".to_string(),
            name: "デフォルト（仕様書）".to_string(),
            prompt: "この動画の内容を技術仕様書として構成してください。システムの概要、機能詳細、API仕様、データ構造を含めて説明してください。".to_string(),
            is_default: true,
        });
    }
    
    if let Some(parent) = presets_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    save_prompt_presets_to_file(&final_presets, &presets_path)
}

fn save_prompt_presets_to_file(presets: &[PromptPreset], path: &Path) -> Result<(), String> {
    // Ensure the parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    
    let mut xml_content = String::new();
    xml_content.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml_content.push_str("<prompt_presets>\n");
    
    for preset in presets {
        xml_content.push_str(&format!(
            "  <preset id=\"{}\" is_default=\"{}\">\n    <name>{}</name>\n    <prompt><![CDATA[{}]]></prompt>\n  </preset>\n",
            preset.id, preset.is_default, preset.name, preset.prompt
        ));
    }
    
    xml_content.push_str("</prompt_presets>\n");
    
    fs::write(path, xml_content)
        .map_err(|e| format!("Failed to write presets file: {}", e))?;
    
    Ok(())
}

fn parse_prompt_presets_xml(xml_content: &str) -> Result<Vec<PromptPreset>, String> {
    // Simple XML parsing for prompt presets
    let mut presets = Vec::new();
    
    // Find all preset blocks
    let mut current_pos = 0;
    while let Some(start) = xml_content[current_pos..].find("<preset id=\"") {
        let absolute_start = current_pos + start;
        let id_start = absolute_start + 12; // length of "<preset id=\""
        
        if let Some(id_end) = xml_content[id_start..].find("\"") {
            let id = xml_content[id_start..id_start + id_end].to_string();
            
            // Parse is_default attribute
            let mut is_default = false;
            if let Some(is_default_start) = xml_content[absolute_start..].find("is_default=\"") {
                let is_default_start = absolute_start + is_default_start + 12; // length of "is_default=\""
                if let Some(is_default_end) = xml_content[is_default_start..].find("\"") {
                    let is_default_str = &xml_content[is_default_start..is_default_start + is_default_end];
                    is_default = is_default_str == "true";
                }
            }
            
            // Find name
            if let Some(name_start) = xml_content[absolute_start..].find("<name>") {
                let name_start = absolute_start + name_start + 6; // length of "<name>"
                if let Some(name_end) = xml_content[name_start..].find("</name>") {
                    let name = xml_content[name_start..name_start + name_end].to_string();
                    
                    // Find prompt
                    if let Some(prompt_start) = xml_content[absolute_start..].find("<![CDATA[") {
                        let prompt_start = absolute_start + prompt_start + 9; // length of "<![CDATA["
                        if let Some(prompt_end) = xml_content[prompt_start..].find("]]>") {
                            let prompt = xml_content[prompt_start..prompt_start + prompt_end].to_string();
                            
                            presets.push(PromptPreset { id, name, prompt, is_default });
                        }
                    }
                }
            }
        }
        
        current_pos = absolute_start + 1;
    }
    
    Ok(presets)
}

#[tauri::command]
async fn import_prompt_presets_from_file(app: tauri::AppHandle) -> Result<Vec<PromptPreset>, String> {
    use tauri_plugin_dialog::DialogExt;
    
    let file_path = app
        .dialog()
        .file()
        .add_filter("XML files", &["xml"])
        .blocking_pick_file();
    
    match file_path {
        Some(path) => {
            let content = fs::read_to_string(path.as_path().unwrap())
                .map_err(|e| format!("Failed to read XML file: {}", e))?;
            
            let imported_presets = parse_prompt_presets_xml(&content)?;
            
            // Merge with existing presets and save
            let existing_presets = load_prompt_presets(app.clone()).await?;
            let mut all_presets = existing_presets;
            
            // Add imported presets with unique IDs
            for mut preset in imported_presets {
                // Generate unique ID if it already exists
                let mut new_id = preset.id.clone();
                let mut counter = 1;
                while all_presets.iter().any(|p| p.id == new_id) {
                    new_id = format!("{}_{}", preset.id, counter);
                    counter += 1;
                }
                preset.id = new_id;
                // Imported presets are not default
                preset.is_default = false;
                all_presets.push(preset);
            }
            
            save_prompt_presets(all_presets.clone(), app).await?;
            Ok(all_presets)
        }
        None => Err("No file selected".to_string()),
    }
}

#[tauri::command]
async fn export_prompt_presets_to_file(presets: Vec<PromptPreset>, app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_dialog::DialogExt;
    
    let file_path = app
        .dialog()
        .file()
        .add_filter("XML files", &["xml"])
        .set_file_name("prompt_presets.xml")
        .blocking_save_file();
    
    match file_path {
        Some(path) => {
            save_prompt_presets_to_file(&presets, path.as_path().unwrap())?;
            Ok(())
        }
        None => Err("No file selected".to_string()),
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
            save_document_to_file,
            load_prompt_presets,
            save_prompt_presets,
            import_prompt_presets_from_file,
            export_prompt_presets_to_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
