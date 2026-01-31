//! Preview state management

use std::path::PathBuf;

use crate::core::AppState;
use crate::render::{
    find_pdftoppm, is_archive_file, is_binary_file, is_image_file, is_pdf_file, is_tar_gz_file,
    is_text_file, ArchivePreview, DirectoryInfo, HexPreview, ImagePreview, PdfPreview, Picker,
    TextPreview,
};

/// Preview state container
#[derive(Default)]
pub struct PreviewState {
    pub text: Option<TextPreview>,
    pub image: Option<ImagePreview>,
    pub dir_info: Option<DirectoryInfo>,
    pub hex: Option<HexPreview>,
    pub archive: Option<ArchivePreview>,
    pub pdf: Option<PdfPreview>,
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
        self.archive = None;
        self.pdf = None;
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
                self.archive = None;
                self.pdf = None;
            }
        } else if is_text_file(path) {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    self.text = Some(TextPreview::new(&content));
                    self.image = None;
                    self.dir_info = None;
                    self.hex = None;
                    self.archive = None;
                    self.pdf = None;
                }
                Err(e) => {
                    state.set_message(format!("Failed: preview - {}", e));
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
                        self.archive = None;
                        self.pdf = None;
                    }
                    Err(e) => {
                        state.set_message(format!("Failed: preview - {}", e));
                        self.clear_all();
                    }
                }
            }
        } else if is_tar_gz_file(path) {
            // Handle tar.gz files separately (before is_archive_file check)
            match ArchivePreview::load_tar_gz(path) {
                Ok(archive) => {
                    self.archive = Some(archive);
                    self.text = None;
                    self.image = None;
                    self.dir_info = None;
                    self.hex = None;
                    self.pdf = None;
                }
                Err(e) => {
                    state.set_message(format!("Failed: preview - {}", e));
                    self.clear_all();
                }
            }
        } else if is_archive_file(path) {
            match ArchivePreview::load_zip(path) {
                Ok(archive) => {
                    self.archive = Some(archive);
                    self.text = None;
                    self.image = None;
                    self.dir_info = None;
                    self.hex = None;
                    self.pdf = None;
                }
                Err(e) => {
                    state.set_message(format!("Failed: preview - {}", e));
                    self.clear_all();
                }
            }
        } else if is_pdf_file(path) {
            // PDF preview - requires pdftoppm (poppler-utils)
            if find_pdftoppm().is_some() {
                if let Some(ref mut picker) = image_picker {
                    match PdfPreview::load(path, 1, picker) {
                        Ok(pdf) => {
                            self.pdf = Some(pdf);
                            self.text = None;
                            self.image = None;
                            self.dir_info = None;
                            self.hex = None;
                            self.archive = None;
                        }
                        Err(e) => {
                            state.set_message(format!("Failed: preview - {}", e));
                            // Fall back to hex preview
                            self.load_hex_fallback(path, state);
                        }
                    }
                } else {
                    // No image picker available - fall back to hex preview
                    self.load_hex_fallback(path, state);
                }
            } else {
                // pdftoppm not installed - show message and fall back to hex preview
                state.set_message("PDF preview requires pdftoppm (poppler-utils)");
                self.load_hex_fallback(path, state);
            }
        } else if is_binary_file(path) || path.is_file() {
            // Binary file or unknown type - show hex preview
            match HexPreview::load(path) {
                Ok(hex) => {
                    self.hex = Some(hex);
                    self.text = None;
                    self.image = None;
                    self.dir_info = None;
                    self.archive = None;
                    self.pdf = None;
                }
                Err(e) => {
                    state.set_message(format!("Failed: preview - {}", e));
                    self.clear_all();
                }
            }
        } else {
            self.clear_all();
        }
    }

    /// Load hex preview as fallback for PDF files
    fn load_hex_fallback(&mut self, path: &std::path::Path, state: &mut AppState) {
        match HexPreview::load(path) {
            Ok(hex) => {
                self.hex = Some(hex);
                self.text = None;
                self.image = None;
                self.dir_info = None;
                self.archive = None;
                self.pdf = None;
            }
            Err(e) => {
                state.set_message(format!("Failed: preview - {}", e));
                self.clear_all();
            }
        }
    }

    /// Check if any preview content is available
    pub fn has_content(&self) -> bool {
        self.text.is_some()
            || self.image.is_some()
            || self.dir_info.is_some()
            || self.hex.is_some()
            || self.archive.is_some()
            || self.pdf.is_some()
    }
}
