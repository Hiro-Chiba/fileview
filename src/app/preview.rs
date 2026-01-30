//! Preview state management

use std::path::PathBuf;

use crate::core::AppState;
use crate::render::{
    is_binary_file, is_image_file, is_text_file, DirectoryInfo, HexPreview, ImagePreview, Picker,
    TextPreview,
};

/// Preview state container
#[derive(Default)]
pub struct PreviewState {
    pub text: Option<TextPreview>,
    pub image: Option<ImagePreview>,
    pub dir_info: Option<DirectoryInfo>,
    pub hex: Option<HexPreview>,
    pub last_path: Option<PathBuf>,
}

impl PreviewState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all preview data
    pub fn clear_all(&mut self) {
        self.text = None;
        self.image = None;
        self.dir_info = None;
        self.hex = None;
    }

    /// Update preview for the given path if it has changed
    pub fn update(
        &mut self,
        path: Option<&PathBuf>,
        image_picker: &mut Option<Picker>,
        state: &mut AppState,
    ) {
        // Only reload preview if the path changed
        if path == self.last_path.as_ref() {
            return;
        }

        self.last_path = path.cloned();

        let Some(path) = path else {
            self.clear_all();
            return;
        };

        if path.is_dir() {
            // Load directory info
            if let Ok(info) = DirectoryInfo::from_path(path) {
                self.dir_info = Some(info);
                self.text = None;
                self.image = None;
                self.hex = None;
            }
        } else if is_text_file(path) {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    self.text = Some(TextPreview::new(&content));
                    self.image = None;
                    self.dir_info = None;
                    self.hex = None;
                }
                Err(e) => {
                    state.set_message(format!("Cannot preview: {}", e));
                    self.clear_all();
                }
            }
        } else if is_image_file(path) {
            if let Some(ref mut picker) = image_picker {
                match ImagePreview::load(path, picker) {
                    Ok(img) => {
                        self.image = Some(img);
                        self.text = None;
                        self.dir_info = None;
                        self.hex = None;
                    }
                    Err(e) => {
                        state.set_message(format!("Cannot preview image: {}", e));
                        self.clear_all();
                    }
                }
            }
        } else if is_binary_file(path) || path.is_file() {
            // Binary file or unknown type - show hex preview
            match HexPreview::load(path) {
                Ok(hex) => {
                    self.hex = Some(hex);
                    self.text = None;
                    self.image = None;
                    self.dir_info = None;
                }
                Err(e) => {
                    state.set_message(format!("Cannot preview: {}", e));
                    self.clear_all();
                }
            }
        } else {
            self.clear_all();
        }
    }

    /// Check if any preview content is available
    pub fn has_content(&self) -> bool {
        self.text.is_some() || self.image.is_some() || self.dir_info.is_some() || self.hex.is_some()
    }
}
