use anyhow::Result;
use std::path::Path;
use std::process::Command;
use tempfile;

/// Splits a video file into segments if it's longer than 1 hour
/// Returns a vector of file paths for the segments (or the original file if no split needed)
pub async fn split_video_if_needed(video_path: &str) -> Result<Vec<String>> {
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

/// Gets the duration of a video file in seconds using ffprobe
pub async fn get_video_duration(video_path: &str) -> Result<f64> {
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

/// Checks if a video file is compatible with the system
/// This function can be extended to include more compatibility checks
pub async fn check_video_compatibility(video_path: &str) -> Result<bool> {
    // For now, just check if we can get the duration (indicates ffprobe can read the file)
    match get_video_duration(video_path).await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Extracts detailed video information using ffprobe
/// Returns a JSON string with video metadata
pub async fn extract_video_info_with_ffprobe(video_path: &str) -> Result<String> {
    let output = Command::new("ffprobe")
        .args(&[
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            video_path,
        ])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let info = String::from_utf8_lossy(&result.stdout);
                Ok(info.to_string())
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

/// Checks if a file is a video file based on its extension
pub fn is_video_file(file_path: &str) -> bool {
    let video_extensions = [
        "mp4", "avi", "mov", "wmv", "flv", "webm", "mkv", "m4v", "3gp", "ogv", "ts", "mts", "m2ts"
    ];
    
    if let Some(extension) = Path::new(file_path).extension() {
        if let Some(ext_str) = extension.to_str() {
            return video_extensions.contains(&ext_str.to_lowercase().as_str());
        }
    }
    false
}

/// Gets video metadata in a simplified format
pub async fn get_video_metadata(video_path: &str) -> Result<VideoMetadata> {
    let duration = get_video_duration(video_path).await?;
    let info_json = extract_video_info_with_ffprobe(video_path).await?;
    
    // Parse the JSON to extract additional metadata
    let parsed: serde_json::Value = serde_json::from_str(&info_json)?;
    
    let mut width = 0;
    let mut height = 0;
    let mut codec = "unknown".to_string();
    
    // Extract video stream information
    if let Some(streams) = parsed.get("streams").and_then(|s| s.as_array()) {
        for stream in streams {
            if let Some(codec_type) = stream.get("codec_type").and_then(|c| c.as_str()) {
                if codec_type == "video" {
                    width = stream.get("width").and_then(|w| w.as_u64()).unwrap_or(0) as u32;
                    height = stream.get("height").and_then(|h| h.as_u64()).unwrap_or(0) as u32;
                    codec = stream.get("codec_name").and_then(|c| c.as_str()).unwrap_or("unknown").to_string();
                    break;
                }
            }
        }
    }
    
    Ok(VideoMetadata {
        duration,
        width,
        height,
        codec,
        file_size: std::fs::metadata(video_path)?.len(),
    })
}

/// Represents video metadata
#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub file_size: u64,
}