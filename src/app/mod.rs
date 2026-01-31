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

pub use config::Config;
pub use config_file::ConfigFile;
pub use event_loop::{run_app, AppResult};
pub use image_loader::ImageLoader;
pub use preview::PreviewState;
