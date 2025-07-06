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
