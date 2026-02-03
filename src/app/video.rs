//! Video preview support using ffmpeg/ffprobe
//!
//! This module provides video thumbnail generation and metadata extraction
//! using external ffmpeg/ffprobe commands.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

/// Cached ffmpeg path detection
static FFMPEG_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Cached ffprobe path detection
static FFPROBE_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Find ffmpeg executable path (lazy detection with caching)
pub fn find_ffmpeg() -> Option<&'static PathBuf> {
    FFMPEG_PATH
        .get_or_init(|| {
            let candidates = [
                "/usr/bin/ffmpeg",
                "/usr/local/bin/ffmpeg",
                "/opt/homebrew/bin/ffmpeg",
            ];
            for path in candidates {
                let p = PathBuf::from(path);
                if p.exists() {
                    return Some(p);
                }
            }
            // Fallback: which ffmpeg
            Command::new("which")
                .arg("ffmpeg")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| PathBuf::from(s.trim()))
                .filter(|p| p.exists())
        })
        .as_ref()
}

/// Find ffprobe executable path (lazy detection with caching)
pub fn find_ffprobe() -> Option<&'static PathBuf> {
    FFPROBE_PATH
        .get_or_init(|| {
            let candidates = [
                "/usr/bin/ffprobe",
                "/usr/local/bin/ffprobe",
                "/opt/homebrew/bin/ffprobe",
            ];
            for path in candidates {
                let p = PathBuf::from(path);
                if p.exists() {
                    return Some(p);
                }
            }
            // Fallback: which ffprobe
            Command::new("which")
                .arg("ffprobe")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| PathBuf::from(s.trim()))
                .filter(|p| p.exists())
        })
        .as_ref()
}

/// Check if a file is a video file
pub fn is_video_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(
        ext.as_deref(),
        Some(
            "mp4"
                | "mkv"
                | "webm"
                | "avi"
                | "mov"
                | "wmv"
                | "flv"
                | "m4v"
                | "mpg"
                | "mpeg"
                | "3gp"
                | "ogv"
        )
    )
}

/// Video metadata extracted from ffprobe
#[derive(Debug, Clone)]
pub struct VideoMetadata {
    /// Video duration
    pub duration: Duration,
    /// Video resolution (width, height)
    pub resolution: (u32, u32),
    /// Video codec name
    pub codec: String,
    /// Audio codec name (if present)
    pub audio_codec: Option<String>,
    /// File size in bytes
    pub file_size: u64,
    /// Frame rate (fps)
    pub frame_rate: Option<f32>,
    /// Bitrate in bits per second
    pub bitrate: Option<u64>,
}

impl VideoMetadata {
    /// Format duration as HH:MM:SS or MM:SS
    pub fn format_duration(&self) -> String {
        let total_secs = self.duration.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }

    /// Format resolution as WxH
    pub fn format_resolution(&self) -> String {
        format!("{}x{}", self.resolution.0, self.resolution.1)
    }

    /// Format bitrate as human-readable string
    pub fn format_bitrate(&self) -> Option<String> {
        self.bitrate.map(|b| {
            if b >= 1_000_000 {
                format!("{:.1} Mbps", b as f64 / 1_000_000.0)
            } else if b >= 1_000 {
                format!("{:.1} Kbps", b as f64 / 1_000.0)
            } else {
                format!("{} bps", b)
            }
        })
    }

    /// Format file size as human-readable string
    pub fn format_size(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.file_size >= GB {
            format!("{:.1} GB", self.file_size as f64 / GB as f64)
        } else if self.file_size >= MB {
            format!("{:.1} MB", self.file_size as f64 / MB as f64)
        } else if self.file_size >= KB {
            format!("{:.1} KB", self.file_size as f64 / KB as f64)
        } else {
            format!("{} B", self.file_size)
        }
    }
}

/// Extract video metadata using ffprobe
///
/// Uses ffprobe with JSON output for reliable parsing.
pub fn get_metadata(path: &Path) -> anyhow::Result<VideoMetadata> {
    let ffprobe = find_ffprobe().ok_or_else(|| anyhow::anyhow!("ffprobe not found"))?;

    // Get file size
    let file_size = std::fs::metadata(path)?.len();

    // Run ffprobe with JSON output
    // ffprobe -v quiet -print_format json -show_format -show_streams <input>
    let output = Command::new(ffprobe)
        .args(["-v", "quiet"])
        .args(["-print_format", "json"])
        .args(["-show_format", "-show_streams"])
        .arg(path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!("ffprobe failed to analyze video");
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    parse_ffprobe_json(&json_str, file_size)
}

/// Parse ffprobe JSON output
fn parse_ffprobe_json(json_str: &str, file_size: u64) -> anyhow::Result<VideoMetadata> {
    // Simple JSON parsing without external crate
    // We only need specific fields, so we parse manually

    let mut duration = Duration::ZERO;
    let mut resolution = (0u32, 0u32);
    let mut codec = String::new();
    let mut audio_codec = None;
    let mut frame_rate = None;
    let mut bitrate = None;

    // Parse duration from format section
    if let Some(dur_str) = extract_json_value(json_str, "duration") {
        if let Ok(dur_secs) = dur_str.parse::<f64>() {
            duration = Duration::from_secs_f64(dur_secs);
        }
    }

    // Parse bitrate from format section
    if let Some(br_str) = extract_json_value(json_str, "bit_rate") {
        if let Ok(br) = br_str.parse::<u64>() {
            bitrate = Some(br);
        }
    }

    // Find video stream section
    let streams_start = json_str.find("\"streams\"");
    if let Some(start) = streams_start {
        let streams_section = &json_str[start..];

        // Find video stream (codec_type: "video")
        if let Some(video_pos) = streams_section.find("\"codec_type\":\"video\"") {
            // Find the start of this stream object
            let stream_start = streams_section[..video_pos].rfind('{').unwrap_or(0);
            let stream_section = &streams_section[stream_start..];

            // Extract video codec
            if let Some(codec_name) = extract_json_value(stream_section, "codec_name") {
                codec = codec_name.to_uppercase();
            }

            // Extract resolution
            if let Some(width_str) = extract_json_value(stream_section, "width") {
                if let Ok(w) = width_str.parse::<u32>() {
                    resolution.0 = w;
                }
            }
            if let Some(height_str) = extract_json_value(stream_section, "height") {
                if let Ok(h) = height_str.parse::<u32>() {
                    resolution.1 = h;
                }
            }

            // Extract frame rate (e.g., "30/1" or "29.97")
            if let Some(fps_str) = extract_json_value(stream_section, "r_frame_rate") {
                frame_rate = parse_frame_rate(fps_str);
            } else if let Some(fps_str) = extract_json_value(stream_section, "avg_frame_rate") {
                frame_rate = parse_frame_rate(fps_str);
            }
        }

        // Find audio stream (codec_type: "audio")
        if let Some(audio_pos) = streams_section.find("\"codec_type\":\"audio\"") {
            let stream_start = streams_section[..audio_pos].rfind('{').unwrap_or(0);
            let stream_section = &streams_section[stream_start..];

            if let Some(codec_name) = extract_json_value(stream_section, "codec_name") {
                audio_codec = Some(codec_name.to_uppercase());
            }
        }
    }

    // Fallback: if codec is empty, try to get it from format
    if codec.is_empty() {
        if let Some(format_name) = extract_json_value(json_str, "format_name") {
            codec = format_name.to_uppercase();
        }
    }

    Ok(VideoMetadata {
        duration,
        resolution,
        codec,
        audio_codec,
        file_size,
        frame_rate,
        bitrate,
    })
}

/// Extract a string value from JSON (simple parser)
fn extract_json_value<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{}\":", key);
    let pos = json.find(&pattern)?;
    let rest = &json[pos + pattern.len()..];

    // Skip whitespace
    let rest = rest.trim_start();

    if rest.starts_with('"') {
        // String value
        let start = 1;
        let end = rest[start..].find('"')?;
        Some(&rest[start..start + end])
    } else {
        // Numeric or other value
        let end = rest.find([',', '}', '\n']).unwrap_or(rest.len());
        Some(rest[..end].trim())
    }
}

/// Parse frame rate string (e.g., "30/1" or "29.97")
fn parse_frame_rate(fps_str: &str) -> Option<f32> {
    if let Some(slash_pos) = fps_str.find('/') {
        let num = fps_str[..slash_pos].parse::<f32>().ok()?;
        let den = fps_str[slash_pos + 1..].parse::<f32>().ok()?;
        if den > 0.0 {
            return Some(num / den);
        }
    }
    fps_str.parse::<f32>().ok()
}

/// Extract a thumbnail frame from a video at 1 second
///
/// Uses ffmpeg to extract a single frame.
/// Returns the path to the generated thumbnail.
pub fn extract_thumbnail(path: &Path) -> anyhow::Result<PathBuf> {
    let ffmpeg = find_ffmpeg().ok_or_else(|| anyhow::anyhow!("ffmpeg not found"))?;

    // Create unique temp file for this video
    let temp_dir = std::env::temp_dir();
    let hash = simple_hash(path.to_string_lossy().as_bytes());
    let temp_path = temp_dir.join(format!("fv_thumb_{:x}.png", hash));

    // If thumbnail already exists and is recent, reuse it
    if temp_path.exists() {
        if let Ok(metadata) = temp_path.metadata() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    // Reuse if less than 1 hour old
                    if elapsed.as_secs() < 3600 {
                        return Ok(temp_path);
                    }
                }
            }
        }
    }

    // Extract thumbnail using ffmpeg
    // ffmpeg -y -i <input> -vframes 1 -ss 00:00:01 <output.png>
    // Use -ss before -i for faster seeking
    let status = Command::new(ffmpeg)
        .args(["-y", "-ss", "1"])
        .arg("-i")
        .arg(path)
        .args(["-vframes", "1"])
        .args([
            "-vf",
            "scale='min(800,iw)':'min(600,ih)':force_original_aspect_ratio=decrease",
        ])
        .arg(&temp_path)
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status()?;

    if !status.success() {
        // Try without -ss (some videos might be very short)
        let status = Command::new(ffmpeg)
            .args(["-y"])
            .arg("-i")
            .arg(path)
            .args(["-vframes", "1"])
            .args([
                "-vf",
                "scale='min(800,iw)':'min(600,ih)':force_original_aspect_ratio=decrease",
            ])
            .arg(&temp_path)
            .stderr(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .status()?;

        if !status.success() {
            anyhow::bail!("ffmpeg failed to extract thumbnail");
        }
    }

    if !temp_path.exists() {
        anyhow::bail!("ffmpeg did not create thumbnail");
    }

    Ok(temp_path)
}

/// Simple hash function for path-based caching
fn simple_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 5381;
    for byte in bytes {
        hash = hash.wrapping_mul(33).wrapping_add(*byte as u64);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_video_file() {
        assert!(is_video_file(Path::new("movie.mp4")));
        assert!(is_video_file(Path::new("movie.MP4")));
        assert!(is_video_file(Path::new("movie.mkv")));
        assert!(is_video_file(Path::new("movie.webm")));
        assert!(is_video_file(Path::new("movie.avi")));
        assert!(is_video_file(Path::new("movie.mov")));
        assert!(is_video_file(Path::new("movie.wmv")));
        assert!(is_video_file(Path::new("movie.flv")));
        assert!(is_video_file(Path::new("movie.m4v")));

        assert!(!is_video_file(Path::new("image.png")));
        assert!(!is_video_file(Path::new("audio.mp3")));
        assert!(!is_video_file(Path::new("document.pdf")));
        assert!(!is_video_file(Path::new("no_extension")));
    }

    #[test]
    fn test_video_metadata_format_duration() {
        let meta = VideoMetadata {
            duration: Duration::from_secs(3661), // 1:01:01
            resolution: (1920, 1080),
            codec: "H264".to_string(),
            audio_codec: Some("AAC".to_string()),
            file_size: 100_000_000,
            frame_rate: Some(30.0),
            bitrate: Some(5_000_000),
        };

        assert_eq!(meta.format_duration(), "01:01:01");

        let meta2 = VideoMetadata {
            duration: Duration::from_secs(125), // 2:05
            ..meta.clone()
        };
        assert_eq!(meta2.format_duration(), "02:05");
    }

    #[test]
    fn test_video_metadata_format_resolution() {
        let meta = VideoMetadata {
            duration: Duration::from_secs(60),
            resolution: (1920, 1080),
            codec: "H264".to_string(),
            audio_codec: None,
            file_size: 0,
            frame_rate: None,
            bitrate: None,
        };

        assert_eq!(meta.format_resolution(), "1920x1080");
    }

    #[test]
    fn test_video_metadata_format_bitrate() {
        let meta = VideoMetadata {
            duration: Duration::from_secs(60),
            resolution: (1920, 1080),
            codec: "H264".to_string(),
            audio_codec: None,
            file_size: 0,
            frame_rate: None,
            bitrate: Some(5_500_000),
        };

        assert_eq!(meta.format_bitrate(), Some("5.5 Mbps".to_string()));

        let meta2 = VideoMetadata {
            bitrate: Some(500_000),
            ..meta.clone()
        };
        assert_eq!(meta2.format_bitrate(), Some("500.0 Kbps".to_string()));

        let meta3 = VideoMetadata {
            bitrate: None,
            ..meta
        };
        assert_eq!(meta3.format_bitrate(), None);
    }

    #[test]
    fn test_parse_frame_rate() {
        assert_eq!(parse_frame_rate("30/1"), Some(30.0));
        assert_eq!(parse_frame_rate("60000/1001"), Some(59.94006)); // 59.94
        assert_eq!(parse_frame_rate("24/1"), Some(24.0));
        assert_eq!(parse_frame_rate("29.97"), Some(29.97));
        assert_eq!(parse_frame_rate("0/0"), None);
    }

    #[test]
    fn test_extract_json_value() {
        let json = r#"{"duration":"120.5","width":1920,"height":1080}"#;

        assert_eq!(extract_json_value(json, "duration"), Some("120.5"));
        assert_eq!(extract_json_value(json, "width"), Some("1920"));
        assert_eq!(extract_json_value(json, "height"), Some("1080"));
        assert_eq!(extract_json_value(json, "nonexistent"), None);
    }

    #[test]
    fn test_find_ffmpeg_consistent() {
        let result1 = find_ffmpeg();
        let result2 = find_ffmpeg();
        assert_eq!(result1.is_some(), result2.is_some());
    }

    #[test]
    fn test_find_ffprobe_consistent() {
        let result1 = find_ffprobe();
        let result2 = find_ffprobe();
        assert_eq!(result1.is_some(), result2.is_some());
    }
}
