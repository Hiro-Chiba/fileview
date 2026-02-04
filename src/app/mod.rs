//! Application module
//!
//! This module contains the main application logic, configuration,
//! and event loop for FileView.

mod config;
mod config_file;
mod event_loop;
mod image_loader;
mod preview;
mod render;
mod video;

pub use config::{Config, InitAction, PluginAction, SessionAction};
pub use config_file::{CommandsConfig, ConfigFile, HooksConfig, PreviewConfig};
pub use event_loop::{run_app, AppResult};
pub use image_loader::ImageLoader;
pub use preview::PreviewState;
pub use video::{
    extract_thumbnail, find_ffmpeg, find_ffprobe, get_metadata, is_video_file, VideoMetadata,
};
