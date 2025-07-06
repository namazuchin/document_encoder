use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, Result};
use log::debug;
// Removed deprecated tauri::api::process::Command import

fn find_executable(name: &str) -> Result<PathBuf> {
    // First, check common paths for Homebrew
    let common_paths = ["/opt/homebrew/bin", "/usr/local/bin"];
    for path in common_paths.iter() {
        let executable_path = Path::new(path).join(name);
        if executable_path.is_file() {
            return Ok(executable_path);
        }
    }

    // If not found, use the `which` crate to search in PATH
    which::which(name).map_err(|e| {
        anyhow!(
            "Failed to find '{}' executable: {}. Please ensure it is installed and in your PATH.",
            name,
            e
        )
    })
}

/// Gets the duration of a video file in seconds using ffprobe
pub async fn get_video_duration(video_path: &str) -> Result<f64> {
    debug!("Getting video duration for: {}", video_path);
    let ffprobe_path = find_executable("ffprobe")?;

    let output = Command::new(&ffprobe_path)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            video_path,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("ffprobe failed: {}", stderr));
    }

    let duration_str = String::from_utf8(output.stdout)?.trim().to_string();
    debug!("Got duration: {}", duration_str);

    duration_str.parse::<f64>().map_err(|e| {
        anyhow!(
            "Failed to parse ffprobe duration output '{}': {}",
            duration_str,
            e
        )
    })
}

/// Splits a video file into segments if it's longer than 1 hour
/// Returns a vector of file paths for the segments (or the original file if no split needed)
pub async fn split_video_if_needed(video_path: &Path) -> Result<Vec<PathBuf>> {
    let duration = get_video_duration(video_path.to_str().unwrap()).await?;
    debug!("Video duration: {} seconds", duration);

    if duration <= 3600.0 {
        return Ok(vec![video_path.to_path_buf()]);
    }

    debug!("Video is longer than 1 hour, splitting...");
    let ffmpeg_path = find_executable("ffmpeg")?;

    let mut segment_paths = Vec::new();
    let mut current_pos = 0.0;
    let mut segment_index = 0;

    while current_pos < duration {
        let segment_filename = format!(
            "{}_segment_{}.mp4",
            video_path.file_stem().unwrap().to_str().unwrap(),
            segment_index
        );
        let segment_path = video_path.parent().unwrap().join(&segment_filename);

        let status = Command::new(&ffmpeg_path)
            .args([
                "-i",
                video_path.to_str().unwrap(),
                "-ss",
                &current_pos.to_string(),
                "-t",
                "3600",
                "-c",
                "copy",
                segment_path.to_str().unwrap(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .status()?;

        if !status.success() {
            return Err(anyhow!("ffmpeg split failed for segment {}", segment_index));
        }

        segment_paths.push(segment_path);
        current_pos += 3600.0;
        segment_index += 1;
    }

    Ok(segment_paths)
}
