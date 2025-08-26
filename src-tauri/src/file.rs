use std::fs;
use crate::types::VideoFile;

#[tauri::command]
pub async fn select_video_files(app: tauri::AppHandle) -> Result<Vec<VideoFile>, String> {
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
                        duration: None,
                    });
                }
            }
            Ok(video_files)
        }
        None => Ok(Vec::new()),
    }
}

#[tauri::command]
pub async fn select_save_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    use tokio::sync::oneshot;

    let (tx, rx) = oneshot::channel();

    app.dialog()
        .file()
        .set_title("保存先ディレクトリを選択")
        .pick_folder(move |folder| {
            let _ = tx.send(folder);
        });

    let folder = rx
        .await
        .map_err(|e| format!("Failed to receive dialog result: {}", e))?;

    match folder {
        Some(path) => Ok(Some(path.to_string())),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn save_document_to_file(
    content: String,
    save_path: String,
    filename: String,
) -> Result<String, String> {
    use std::path::Path;

    let full_path = Path::new(&save_path).join(&filename);

    fs::write(&full_path, content).map_err(|e| format!("Failed to save document: {}", e))?;

    Ok(full_path.to_string_lossy().to_string())
}