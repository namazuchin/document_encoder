use std::process::Command;

// Simple test to verify FFmpeg check logic without Tauri
fn main() {
    println!("Testing FFmpeg availability check...");
    
    // Test find_executable logic for ffmpeg
    let result = find_executable("ffmpeg");
    match result {
        Ok(path) => {
            println!("✅ FFmpeg found at: {:?}", path);
            
            // Test find_executable logic for ffprobe
            let ffprobe_result = find_executable("ffprobe");
            match ffprobe_result {
                Ok(ffprobe_path) => {
                    println!("✅ FFprobe found at: {:?}", ffprobe_path);
                    println!("✅ Both FFmpeg and FFprobe are available");
                }
                Err(e) => {
                    println!("❌ FFprobe not found: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ FFmpeg not found: {}", e);
        }
    }
    
    // Test using system's which command as fallback
    println!("\n--- Testing with system 'which' command ---");
    test_with_which_command("ffmpeg");
    test_with_which_command("ffprobe");
}

fn find_executable(name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
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
        let executable_path = std::path::Path::new(path).join(name);
        if executable_path.is_file() {
            println!("Found {} at: {:?}", name, executable_path);
            return Ok(executable_path);
        }
    }

    // If not found in common paths, return error
    Err(format!(
        "Failed to find '{}' executable in common paths: {:?}. PATH environment variable: {:?}",
        name,
        common_paths,
        std::env::var("PATH").unwrap_or_else(|_| "Not available".to_string())
    ).into())
}

fn test_with_which_command(name: &str) {
    let output = Command::new("which")
        .arg(name)
        .output();
        
    match output {
        Ok(result) if result.status.success() => {
            let path_str = String::from_utf8_lossy(&result.stdout);
            let path = path_str.trim();
            println!("✅ {} found with 'which': {}", name, path);
        }
        Ok(_) => {
            println!("❌ {} not found with 'which'", name);
        }
        Err(e) => {
            println!("❌ Error running 'which' for {}: {}", name, e);
        }
    }
}