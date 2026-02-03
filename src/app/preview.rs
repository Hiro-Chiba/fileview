//! Preview state management

use std::collections::HashMap;
use std::path::PathBuf;

use image::GenericImageView;

use crate::app::video::{extract_thumbnail, find_ffprobe, get_metadata, is_video_file};
use crate::app::ImageLoader;
use crate::core::AppState;
use crate::git::{self, FileStatus};
use crate::render::{
    find_pdftoppm, is_archive_file, is_binary_file, is_image_file, is_pdf_file, is_tar_gz_file,
    is_text_file, ArchivePreview, CustomPreview, DiffPreview, DirectoryInfo, HexPreview,
    ImagePreview, PdfPreview, Picker, TextPreview, VideoPreview,
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
    pub diff: Option<DiffPreview>,
    pub custom: Option<CustomPreview>,
    pub video: Option<VideoPreview>,
    pub last_path: Option<PathBuf>,
    /// Background image loader
    image_loader: ImageLoader,
    /// Path currently being loaded asynchronously
    pub loading_image_path: Option<PathBuf>,
    /// Video path currently loading thumbnail
    pub loading_video_thumbnail: Option<PathBuf>,
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
        self.diff = None;
        self.custom = None;
        self.video = None;
    }

    /// Update preview for the given path if it has changed
    pub fn update(
        &mut self,
        path: Option<&PathBuf>,
        image_picker: &mut Option<Picker>,
        state: &mut AppState,
    ) {
        self.update_with_custom(path, image_picker, state, &HashMap::new());
    }

    /// Update preview with custom preview support
    ///
    /// `custom_previews` maps file extensions to command templates.
    /// The command template can use `$f` as a placeholder for the file path.
    pub fn update_with_custom(
        &mut self,
        path: Option<&PathBuf>,
        image_picker: &mut Option<Picker>,
        state: &mut AppState,
        custom_previews: &HashMap<String, String>,
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

        // Check for custom preview first (if not a directory)
        if !path.is_dir() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if let Some(cmd) = custom_previews.get(ext) {
                    match CustomPreview::execute(cmd, path) {
                        Ok(preview) => {
                            self.custom = Some(preview);
                            self.text = None;
                            self.image = None;
                            self.dir_info = None;
                            self.hex = None;
                            self.archive = None;
                            self.pdf = None;
                            self.diff = None;
                            return;
                        }
                        Err(e) => {
                            state.set_message(format!("Custom preview failed: {}", e));
                            // Fall through to default preview
                        }
                    }
                }
            }
        }

        if path.is_dir() {
            // Load directory info
            if let Ok(info) = DirectoryInfo::from_path(path) {
                self.dir_info = Some(info);
                self.text = None;
                self.image = None;
                self.hex = None;
                self.archive = None;
                self.pdf = None;
                self.diff = None;
                self.custom = None;
            }
        } else if is_text_file(path) {
            // Check if file has git changes - if so, show diff instead
            let git_status = state
                .git_status
                .as_ref()
                .map(|g| g.get_status(path))
                .unwrap_or(FileStatus::Clean);

            let has_changes = matches!(
                git_status,
                FileStatus::Modified | FileStatus::Added | FileStatus::Deleted
            );

            if has_changes {
                // Try to get diff for changed files
                if let Some(ref git) = state.git_status {
                    let repo_root = git.repo_root();
                    // Try staged diff first, then unstaged
                    let diff = git::get_diff(repo_root, path, true)
                        .or_else(|| git::get_diff(repo_root, path, false));

                    if let Some(file_diff) = diff {
                        if !file_diff.is_empty() {
                            self.diff = Some(DiffPreview::new(file_diff));
                            self.text = None;
                            self.image = None;
                            self.dir_info = None;
                            self.hex = None;
                            self.archive = None;
                            self.pdf = None;
                            self.custom = None;
                            return;
                        }
                    }
                }
            }

            // Fall back to regular text preview
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    self.text = Some(TextPreview::with_highlighting(&content, path));
                    self.image = None;
                    self.dir_info = None;
                    self.hex = None;
                    self.archive = None;
                    self.pdf = None;
                    self.diff = None;
                    self.custom = None;
                }
                Err(e) => {
                    state.set_message(format!("Failed: preview - {}", e));
                    self.clear_all();
                }
            }
        } else if is_image_file(path) {
            // Start async image loading (non-blocking)
            if self.image_loader.request(path.to_path_buf()) {
                // Clear current preview while loading
                self.image = None;
                self.text = None;
                self.dir_info = None;
                self.hex = None;
                self.archive = None;
                self.pdf = None;
                self.diff = None;
                self.custom = None;
                self.video = None;
                self.loading_image_path = Some(path.to_path_buf());
            }
        } else if is_video_file(path) {
            // Video preview - requires ffprobe for metadata
            if find_ffprobe().is_some() {
                match get_metadata(path) {
                    Ok(metadata) => {
                        let mut video_preview = VideoPreview::new(path, metadata);

                        // Try to extract thumbnail
                        match extract_thumbnail(path) {
                            Ok(thumb_path) => {
                                // Request thumbnail loading
                                if self.image_loader.request(thumb_path.clone()) {
                                    self.loading_video_thumbnail = Some(path.to_path_buf());
                                }
                            }
                            Err(e) => {
                                video_preview.thumbnail_error =
                                    Some(format!("Failed to extract: {}", e));
                            }
                        }

                        self.video = Some(video_preview);
                        self.text = None;
                        self.image = None;
                        self.dir_info = None;
                        self.hex = None;
                        self.archive = None;
                        self.pdf = None;
                        self.diff = None;
                        self.custom = None;
                    }
                    Err(e) => {
                        state.set_message(format!("Failed: video preview - {}", e));
                        // Fall back to hex preview
                        self.load_hex_fallback(path, state);
                    }
                }
            } else {
                // ffprobe not installed - show message and fall back to hex preview
                state.set_message("Video preview requires ffprobe (ffmpeg)");
                self.load_hex_fallback(path, state);
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
                    self.diff = None;
                    self.custom = None;
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
                    self.diff = None;
                    self.custom = None;
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
                            self.diff = None;
                            self.custom = None;
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
                    self.diff = None;
                    self.custom = None;
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
                self.diff = None;
                self.archive = None;
                self.pdf = None;
                self.custom = None;
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
            || self.diff.is_some()
            || self.custom.is_some()
            || self.video.is_some()
    }

    /// Poll for completed image load results
    ///
    /// This should be called in the main event loop to receive
    /// asynchronously loaded images.
    ///
    /// Returns true if an image was successfully loaded.
    pub fn poll_image_result(
        &mut self,
        image_picker: &mut Option<Picker>,
        state: &mut AppState,
    ) -> bool {
        if let Some(result) = self.image_loader.try_recv() {
            // Check if this is for regular image preview
            if self.loading_image_path.as_ref() == Some(&result.path) {
                self.loading_image_path = None;

                match result.result {
                    Ok(dyn_img) => {
                        if let Some(ref mut picker) = image_picker {
                            let (width, height) = dyn_img.dimensions();
                            let protocol = picker.new_resize_protocol(dyn_img);
                            self.image = Some(ImagePreview {
                                width,
                                height,
                                protocol,
                            });
                            return true;
                        }
                    }
                    Err(e) => {
                        state.set_message(format!("Failed: preview - {}", e));
                    }
                }
            }
            // Check if this is for video thumbnail
            else if self.loading_video_thumbnail.is_some() {
                // The result path is the thumbnail path, not the video path
                // But we need to attach it to the current video preview
                if let Some(ref mut video) = self.video {
                    match result.result {
                        Ok(dyn_img) => {
                            if let Some(ref mut picker) = image_picker {
                                let (width, height) = dyn_img.dimensions();
                                let protocol = picker.new_resize_protocol(dyn_img);
                                video.thumbnail = Some(ImagePreview {
                                    width,
                                    height,
                                    protocol,
                                });
                                self.loading_video_thumbnail = None;
                                return true;
                            }
                        }
                        Err(e) => {
                            video.thumbnail_error = Some(format!("Load failed: {}", e));
                            self.loading_video_thumbnail = None;
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if an image is currently being loaded
    pub fn is_loading_image(&self) -> bool {
        self.loading_image_path.is_some() || self.loading_video_thumbnail.is_some()
    }
}
