use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

use anyhow::{anyhow, Result};
use log::debug;

use crate::types::VideoQuality;

#[derive(Debug, Clone)]
pub struct VideoResolution {
    pub width: u32,
    pub height: u32,
}
// Removed deprecated tauri::api::process::Command import

fn find_executable(name: &str) -> Result<PathBuf> {
    // First, check common paths for Homebrew and system installations
    let common_paths = [
        "/opt/homebrew/bin",      // Homebrew on Apple Silicon
        "/usr/local/bin",         // Homebrew on Intel Mac / general Unix
        "/usr/bin",               // System binaries
        "/bin",                   // Core system binaries
        "/opt/local/bin",         // MacPorts
        "/sw/bin",                // Fink
        "/usr/local/opt/ffmpeg/bin", // Homebrew ffmpeg formula specific
        "/opt/homebrew/opt/ffmpeg/bin", // Homebrew ffmpeg on Apple Silicon
    ];
    
    for path in common_paths.iter() {
        let executable_path = Path::new(path).join(name);
        if executable_path.is_file() {
            debug!("Found {} at: {:?}", name, executable_path);
            return Ok(executable_path);
        }
    }

    // If not found, use the `which` crate to search in PATH
    debug!("Searching for {} in PATH environment variable", name);
    which::which(name).map_err(|e| {
        // Log all the paths we searched
        debug!("Failed to find {} in common paths: {:?}", name, common_paths);
        debug!("PATH environment variable: {:?}", std::env::var("PATH"));
        
        anyhow!(
            "Failed to find '{}' executable: {}. Please ensure it is installed and in your PATH. Searched in: {:?}",
            name,
            e,
            common_paths
        )
    })
}

/// Gets the resolution of a video file using ffprobe
pub async fn get_video_resolution(video_path: &str) -> Result<VideoResolution> {
    debug!("Getting video resolution for: {}", video_path);
    let ffprobe_path = find_executable("ffprobe")?;

    let output = Command::new(&ffprobe_path)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=s=x:p=0",
            video_path,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("ffprobe failed: {}", stderr));
    }

    let resolution_str = String::from_utf8(output.stdout)?.trim().to_string();
    debug!("Got resolution: {}", resolution_str);

    let parts: Vec<&str> = resolution_str.split('x').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid resolution format: {}", resolution_str));
    }

    let width = parts[0].parse::<u32>().map_err(|e| {
        anyhow!("Failed to parse width '{}': {}", parts[0], e)
    })?;

    let height = parts[1].parse::<u32>().map_err(|e| {
        anyhow!("Failed to parse height '{}': {}", parts[1], e)
    })?;

    Ok(VideoResolution { width, height })
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

/// Extracts a frame from a video at the specified timestamp and saves it as an image
pub async fn extract_frame_from_video(
    video_path: &str,
    timestamp: f64,
    output_path: &str,
) -> Result<()> {
    debug!("Extracting frame from video: {} at timestamp: {}s", video_path, timestamp);
    
    let ffmpeg_path = find_executable("ffmpeg")?;
    
    let status = Command::new(&ffmpeg_path)
        .args([
            "-i",
            video_path,
            "-ss",
            &timestamp.to_string(),
            "-vframes",
            "1",
            "-q:v",
            "2",
            "-y",
            output_path,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()?;
    
    if !status.success() {
        return Err(anyhow!("Failed to extract frame from video at timestamp {}s", timestamp));
    }
    
    debug!("Successfully extracted frame to: {}", output_path);
    Ok(())
}

/// Encodes a video to the specified quality if conversion is needed
/// Returns the path to the encoded video (or original if no conversion needed)
pub async fn encode_video_if_needed<F>(
    video_path: &str,
    target_quality: &VideoQuality,
    output_dir: &Path,
    progress_callback: F,
    hardware_encoding: bool,
) -> Result<PathBuf>
where
    F: Fn(String),
{
    debug!("Checking if video encoding is needed for: {}", video_path);
    
    // If no conversion is requested, return original path
    if *target_quality == VideoQuality::NoConversion {
        return Ok(PathBuf::from(video_path));
    }
    
    // Get current resolution
    let current_resolution = get_video_resolution(video_path).await?;
    debug!("Current resolution: {}x{}", current_resolution.width, current_resolution.height);
    
    // Determine target resolution
    let (target_width, target_height) = match target_quality {
        VideoQuality::Quality1080p => (1920, 1080),
        VideoQuality::Quality720p => (1280, 720),
        VideoQuality::Quality480p => (854, 480),
        VideoQuality::NoConversion => unreachable!(),
    };
    
    // Check if encoding is needed
    let needs_encoding = current_resolution.height > target_height 
        || (current_resolution.height == target_height && current_resolution.width > target_width);
    
    if !needs_encoding {
        debug!("Video already at or below target quality, no encoding needed");
        return Ok(PathBuf::from(video_path));
    }
    
    progress_callback("動画のエンコードを開始しています...".to_string());
    
    let input_path = Path::new(video_path);
    let filename = input_path.file_stem()
        .ok_or_else(|| anyhow!("Invalid video file name"))?
        .to_str()
        .ok_or_else(|| anyhow!("Invalid video file name encoding"))?;
    
    let output_filename = format!("{}_{}.mp4", filename, target_quality_string(target_quality));
    let output_path = output_dir.join(output_filename);
    
    debug!("Encoding video to: {:?}", output_path);
    
    let ffmpeg_path = find_executable("ffmpeg")?;
    
    // Get video duration for progress calculation
    let duration = get_video_duration(video_path).await?;
    
    // Choose video encoder based on hardware encoding setting
    let video_encoder = if hardware_encoding {
        match get_best_hardware_encoder().await {
            Some(encoder) => {
                debug!("Using hardware encoder: {}", encoder);
                progress_callback(format!("ハードウェアエンコーダーを使用します: {}", encoder));
                
                // Test if hardware encoder is actually working
                if let Err(e) = test_hardware_encoder(&encoder).await {
                    debug!("Hardware encoder test failed: {}, falling back to software encoder", e);
                    progress_callback("ハードウェアエンコーダーのテストに失敗しました。ソフトウェアエンコーダーを使用します...".to_string());
                    "libx264".to_string()
                } else {
                    encoder
                }
            }
            None => {
                debug!("Hardware encoding requested but no hardware encoder available, falling back to software");
                progress_callback("ハードウェアエンコーダーが利用できません。ソフトウェアエンコーダーを使用します...".to_string());
                "libx264".to_string()
            }
        }
    } else {
        debug!("Using software encoder: libx264");
        "libx264".to_string()
    };
    
    // Build ffmpeg command arguments
    let scale_filter = format!("scale={}:{}", target_width, target_height);
    let mut args = vec![
        "-i", video_path,
        "-vf", &scale_filter,
        "-c:v", &video_encoder,
        "-c:a", "aac",
    ];
    
    // Add quality settings based on encoder type
    if video_encoder == "libx264" {
        // Software encoding quality settings
        args.extend_from_slice(&["-crf", "23"]);
    } else {
        // Hardware encoding quality settings
        args.extend_from_slice(&["-b:v", "5M"]); // 5 Mbps bitrate for hardware encoding
    }
    
    // Add progress and output settings
    args.extend_from_slice(&[
        "-progress", "pipe:1",
        "-y",
        output_path.to_str().unwrap(),
    ]);
    
    debug!("Executing ffmpeg command: {:?} {:?}", ffmpeg_path, args);
    let mut command = Command::new(&ffmpeg_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    // Monitor progress and capture stderr
    let mut stderr_output = String::new();
    
    // Read stderr in a separate thread to capture error messages
    let stderr_handle = if let Some(stderr) = command.stderr.take() {
        let stderr_reader = BufReader::new(stderr);
        Some(std::thread::spawn(move || {
            let mut errors = String::new();
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    errors.push_str(&line);
                    errors.push('\n');
                }
            }
            errors
        }))
    } else {
        None
    };
    
    // Monitor progress
    if let Some(stdout) = command.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if line.starts_with("out_time_ms=") {
                        if let Ok(time_ms) = line[12..].parse::<f64>() {
                            let current_time = time_ms / 1_000_000.0; // Convert microseconds to seconds
                            let progress_percent = ((current_time / duration) * 100.0).min(100.0);
                            progress_callback(format!("エンコード中... {:.1}%", progress_percent));
                        }
                    }
                }
                Err(_) => break,
            }
        }
    }
    
    let status = command.wait()?;
    
    // Get stderr output from the background thread
    if let Some(handle) = stderr_handle {
        if let Ok(errors) = handle.join() {
            stderr_output = errors;
        }
    }
    
    if !status.success() {
        debug!("ffmpeg stderr: {}", stderr_output);
        return Err(anyhow!("Video encoding failed: {}", stderr_output));
    }
    
    progress_callback("エンコードが完了しました".to_string());
    debug!("Video encoding completed: {:?}", output_path);
    
    Ok(output_path)
}

fn target_quality_string(quality: &VideoQuality) -> &str {
    match quality {
        VideoQuality::Quality1080p => "1080p",
        VideoQuality::Quality720p => "720p", 
        VideoQuality::Quality480p => "480p",
        VideoQuality::NoConversion => "original",
    }
}

/// Detects available hardware encoders on the system
pub async fn detect_hardware_encoders() -> Result<Vec<String>> {
    debug!("Detecting available hardware encoders");
    
    let ffmpeg_path = find_executable("ffmpeg")?;
    
    // Get list of available encoders
    let output = Command::new(&ffmpeg_path)
        .args(["-encoders"])
        .output()?;
    
    if !output.status.success() {
        return Err(anyhow!("Failed to get encoder list from ffmpeg"));
    }
    
    let encoder_list = String::from_utf8(output.stdout)?;
    let mut available_encoders = Vec::new();
    
    // Check for common hardware encoders
    let hardware_encoders = vec![
        ("h264_videotoolbox", "Apple VideoToolbox H.264"),
        ("h264_nvenc", "NVIDIA NVENC H.264"),
        ("h264_qsv", "Intel Quick Sync H.264"),
        ("h264_amf", "AMD AMF H.264"),
        ("h264_vaapi", "VAAPI H.264"),
        ("h264_v4l2m2m", "V4L2 Memory-to-Memory H.264"),
    ];
    
    for (encoder_name, display_name) in hardware_encoders {
        if encoder_list.contains(encoder_name) {
            debug!("Found hardware encoder: {}", display_name);
            available_encoders.push(display_name.to_string());
        }
    }
    
    debug!("Available hardware encoders: {:?}", available_encoders);
    Ok(available_encoders)
}

/// Checks if hardware encoding is available on the system
pub async fn is_hardware_encoding_available() -> bool {
    match detect_hardware_encoders().await {
        Ok(encoders) => !encoders.is_empty(),
        Err(e) => {
            debug!("Error detecting hardware encoders: {}", e);
            false
        }
    }
}

/// Tests if a hardware encoder is actually working
async fn test_hardware_encoder(encoder: &str) -> Result<()> {
    debug!("Testing hardware encoder: {}", encoder);
    
    let ffmpeg_path = find_executable("ffmpeg")?;
    
    // Create a simple test: generate a small test video and try to encode it
    let output = Command::new(&ffmpeg_path)
        .args([
            "-f", "lavfi",
            "-i", "testsrc=duration=1:size=320x240:rate=30",
            "-c:v", encoder,
            "-t", "1",
            "-f", "null",
            "-",
        ])
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!("Hardware encoder test failed: {}", stderr);
        return Err(anyhow!("Hardware encoder test failed: {}", stderr));
    }
    
    debug!("Hardware encoder test passed for: {}", encoder);
    Ok(())
}

/// Gets the best available hardware encoder for the current system
pub async fn get_best_hardware_encoder() -> Option<String> {
    let ffmpeg_path = match find_executable("ffmpeg") {
        Ok(path) => path,
        Err(_) => return None,
    };
    
    // Get list of available encoders
    let output = match Command::new(&ffmpeg_path)
        .args(["-encoders"])
        .output()
    {
        Ok(output) => output,
        Err(_) => return None,
    };
    
    if !output.status.success() {
        return None;
    }
    
    let encoder_list = String::from_utf8(output.stdout).ok()?;
    
    // Priority order of hardware encoders (best first)
    let encoder_priority = vec![
        "h264_videotoolbox", // Apple VideoToolbox (macOS)
        "h264_nvenc",        // NVIDIA NVENC
        "h264_qsv",          // Intel Quick Sync
        "h264_amf",          // AMD AMF
        "h264_vaapi",        // VAAPI
        "h264_v4l2m2m",      // V4L2 Memory-to-Memory
    ];
    
    for encoder in encoder_priority {
        if encoder_list.contains(encoder) {
            debug!("Selected hardware encoder: {}", encoder);
            return Some(encoder.to_string());
        }
    }
    
    None
}
