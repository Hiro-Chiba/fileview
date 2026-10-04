//! Video preview support using ffmpeg/ffprobe
//!
//! This module provides video thumbnail generation and metadata extraction
//! using external ffmpeg/ffprobe commands.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

/// Prefix a relative path starting with `-` with `./` so ffmpeg/ffprobe cannot
/// parse the filename as an option flag (argument injection).
fn safe_input_arg(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let starts_dash = path.to_str().map(|s| s.starts_with('-')).unwrap_or(false);
    if starts_dash {
        Path::new("./").join(path)
    } else {
        path.to_path_buf()
    }
}

/// Cached ffmpeg path detection
static FFMPEG_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Cached ffprobe path detection
static FFPROBE_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Find ffmpeg executable path (lazy detection with caching)
pub fn find_ffmpeg() -> Option<&'static PathBuf> {
    FFMPEG_PATH
        .get_or_init(|| crate::util::find_preview_tool("ffmpeg"))
        .as_ref()
}

/// Find ffprobe executable path (lazy detection with caching)
pub fn find_ffprobe() -> Option<&'static PathBuf> {
    FFPROBE_PATH
        .get_or_init(|| crate::util::find_preview_tool("ffprobe"))
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
        .arg(safe_input_arg(path))
        .output()?;

    if !output.status.success() {
        anyhow::bail!("ffprobe failed to analyze video");
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    parse_ffprobe_json(&json_str, file_size)
}

/// Parse ffprobe JSON output
fn parse_ffprobe_json(json_str: &str, file_size: u64) -> anyhow::Result<VideoMetadata> {
    let root: serde_json::Value = serde_json::from_str(json_str)?;
    let format = &root["format"];
    let streams = root["streams"].as_array();
    let video = streams.and_then(|streams| {
        streams
            .iter()
            .find(|stream| stream["codec_type"] == "video")
    });
    let audio = streams.and_then(|streams| {
        streams
            .iter()
            .find(|stream| stream["codec_type"] == "audio")
    });

    let duration = format["duration"]
        .as_str()
        .and_then(|value| value.parse::<f64>().ok())
        .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
        .unwrap_or_default();
    let bitrate = format["bit_rate"]
        .as_str()
        .and_then(|value| value.parse::<u64>().ok());
    let codec = video
        .and_then(|stream| stream["codec_name"].as_str())
        .filter(|name| !name.is_empty())
        .or_else(|| format["format_name"].as_str())
        .unwrap_or_default()
        .to_uppercase();
    let audio_codec = audio
        .and_then(|stream| stream["codec_name"].as_str())
        .map(str::to_uppercase);
    let dimension = |key: &str| {
        video
            .and_then(|stream| stream[key].as_u64())
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or_default()
    };
    let resolution = (dimension("width"), dimension("height"));
    let frame_rate = video.and_then(|stream| {
        stream["r_frame_rate"]
            .as_str()
            .and_then(parse_frame_rate)
            .or_else(|| stream["avg_frame_rate"].as_str().and_then(parse_frame_rate))
    });

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

/// Parse a positive, finite frame rate (e.g., "30/1" or "29.97").
fn parse_frame_rate(fps_str: &str) -> Option<f32> {
    let rate = if let Some((numerator, denominator)) = fps_str.split_once('/') {
        let num = numerator.parse::<f32>().ok()?;
        let den = denominator.parse::<f32>().ok()?;
        if !num.is_finite() || !den.is_finite() || num <= 0.0 || den <= 0.0 {
            return None;
        }
        num / den
    } else {
        fps_str.parse::<f32>().ok()?
    };
    (rate.is_finite() && rate > 0.0).then_some(rate)
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
        .arg(safe_input_arg(path))
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
            .arg(safe_input_arg(path))
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
    fn test_parse_ffprobe_pretty_json() {
        let json = r#"{
            "streams": [
                {
                    "codec_name": "aac", "codec_type": "audio",
                    "duration": "100", "bit_rate": "128000"
                },
                {
                    "codec_name": "h264", "codec_type": "video",
                    "width": 1920, "height": 1080,
                    "r_frame_rate": "30000/1001"
                }
            ],
            "format": {"duration": "120.5", "bit_rate": "5500000"}
        }"#;
        let meta = parse_ffprobe_json(json, 1234).unwrap();
        assert_eq!(meta.resolution, (1920, 1080));
        assert_eq!(meta.codec, "H264");
        assert_eq!(meta.audio_codec.as_deref(), Some("AAC"));
        assert_eq!(meta.duration, Duration::from_millis(120500));
        assert_eq!(meta.bitrate, Some(5500000));
        assert_eq!(meta.file_size, 1234);
        assert_eq!(meta.frame_rate, Some(29.97003));
    }

    #[test]
    fn test_parse_ffprobe_stream_boundaries() {
        let json = r#"{"streams":[{"codec_type":"video"},
            {"codec_type":"audio","codec_name":"aac","duration":"12","bit_rate":"128000"},
            {"codec_type":"video","codec_name":"vp9","width":640,"height":360}],
            "format":{"format_name":"matroska"}}"#;
        let meta = parse_ffprobe_json(json, 0).unwrap();
        assert_eq!(meta.codec, "MATROSKA");
        assert_eq!(meta.resolution, (0, 0));
        assert_eq!(meta.duration, Duration::ZERO);
        assert_eq!(meta.bitrate, None);
    }

    #[test]
    fn test_parse_ffprobe_invalid_duration() {
        for duration in ["-1", "NaN", "inf", "1e100", "N/A"] {
            let json = format!(r#"{{"format":{{"duration":"{duration}"}}}}"#);
            assert_eq!(
                parse_ffprobe_json(&json, 0).unwrap().duration,
                Duration::ZERO
            );
        }
    }

    #[test]
    fn test_parse_ffprobe_invalid_json() {
        assert!(parse_ffprobe_json("not json", 0).is_err());
    }

    #[test]
    fn test_parse_ffprobe_frame_rate_fallback() {
        let json = r#"{"streams":[{"codec_type":"video",
            "r_frame_rate":"0/0","avg_frame_rate":"24/1"}]}"#;
        assert_eq!(parse_ffprobe_json(json, 0).unwrap().frame_rate, Some(24.0));
    }

    #[test]
    fn test_parse_frame_rate_invalid_values() {
        for rate in [
            "0", "-30", "NaN", "inf", "1e100", "-30/1", "1/0", "0/1", "inf/1", "1/inf", "-1/-1",
        ] {
            assert_eq!(parse_frame_rate(rate), None, "rate: {rate}");
        }
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
