//! Preview state management

use std::collections::HashMap;
use std::path::PathBuf;

use image::GenericImageView;

use crate::app::preview_worker::{
    CachedPreview, PreviewCache, PreviewKind, PreviewPayload, PreviewWorker,
};
use crate::app::video::{extract_thumbnail, is_video_file};
use crate::app::ImageLoader;
use crate::core::AppState;
use crate::git::FileStatus;
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
    /// Background preview worker
    preview_worker: PreviewWorker,
    /// LRU preview cache
    preview_cache: PreviewCache,
    /// Whether a preview is currently being generated in the background
    pub is_loading: bool,
    /// Serial number of the latest preview request
    pending_serial: u64,
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

    /// Discard cached content after an explicit or watched filesystem refresh.
    pub fn invalidate(&mut self) {
        self.preview_cache.clear();
        self.image_loader.cancel();
        self.loading_image_path = None;
        self.loading_video_thumbnail = None;
        self.last_path = None;
        self.is_loading = false;
        self.clear_all();
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

        self.image_loader.cancel();
        self.loading_image_path = None;
        self.loading_video_thumbnail = None;
        self.last_path = path.cloned();
        self.is_loading = false;

        let Some(path) = path else {
            self.clear_all();
            return;
        };

        // Check for custom preview first (if configured for a non-directory)
        if let Some(cmd) = path
            .extension()
            .and_then(|extension| extension.to_str())
            .and_then(|extension| custom_previews.get(extension))
        {
            if !path.is_dir() {
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

        let metadata = path.metadata().ok();
        let is_dir = metadata.as_ref().is_some_and(|metadata| metadata.is_dir());
        let is_file = metadata.as_ref().is_some_and(|metadata| metadata.is_file());

        if is_dir {
            // Check cache first
            if let Some(CachedPreview::Directory(ref info)) = self.preview_cache.get(path) {
                self.dir_info = Some(info.clone());
                self.text = None;
                self.image = None;
                self.hex = None;
                self.archive = None;
                self.pdf = None;
                self.diff = None;
                self.custom = None;
                return;
            }
            // Dispatch to worker
            self.clear_all();
            self.is_loading = true;
            self.pending_serial =
                self.preview_worker
                    .request(path.to_path_buf(), PreviewKind::Directory, None, None);
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
                // Check cache for diff
                if let Some(CachedPreview::Diff(ref dp)) = self.preview_cache.get(path) {
                    self.diff = Some(dp.clone());
                    self.text = None;
                    self.image = None;
                    self.dir_info = None;
                    self.hex = None;
                    self.archive = None;
                    self.pdf = None;
                    self.custom = None;
                    return;
                }
                // Dispatch diff to worker
                if let Some(ref git) = state.git_status {
                    let repo_root = git.repo_root().to_path_buf();
                    self.clear_all();
                    self.is_loading = true;
                    self.pending_serial = self.preview_worker.request(
                        path.to_path_buf(),
                        PreviewKind::Diff,
                        Some(repo_root),
                        Some(git_status),
                    );
                    return;
                }
            }

            // Check cache for text
            if let Some(CachedPreview::Text(ref tp)) = self.preview_cache.get(path) {
                self.text = Some(tp.clone());
                self.image = None;
                self.dir_info = None;
                self.hex = None;
                self.archive = None;
                self.pdf = None;
                self.diff = None;
                self.custom = None;
                return;
            }
            // Dispatch text to worker
            self.clear_all();
            self.is_loading = true;
            self.pending_serial =
                self.preview_worker
                    .request(path.to_path_buf(), PreviewKind::Text, None, None);
        } else if is_image_file(path) {
            // Start async image loading (non-blocking) — existing ImageLoader
            if self.image_loader.request(path.to_path_buf()) {
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
            // Dispatch video metadata to worker
            self.clear_all();
            self.is_loading = true;
            self.pending_serial =
                self.preview_worker
                    .request(path.to_path_buf(), PreviewKind::VideoMeta, None, None);
        } else if is_tar_gz_file(path) || is_archive_file(path) {
            // Check cache for archive
            if let Some(CachedPreview::Archive(ref ap)) = self.preview_cache.get(path) {
                self.archive = Some(ap.clone());
                self.text = None;
                self.image = None;
                self.dir_info = None;
                self.hex = None;
                self.pdf = None;
                self.diff = None;
                self.custom = None;
                return;
            }
            // Dispatch archive to worker
            self.clear_all();
            self.is_loading = true;
            self.pending_serial =
                self.preview_worker
                    .request(path.to_path_buf(), PreviewKind::Archive, None, None);
        } else if is_pdf_file(path) {
            // PDF preview — stays synchronous (Picker is not Send)
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
                            self.load_hex_fallback(path, state);
                        }
                    }
                } else {
                    self.load_hex_fallback(path, state);
                }
            } else {
                state.set_message("PDF preview requires pdftoppm (poppler-utils)");
                self.load_hex_fallback(path, state);
            }
        } else if is_binary_file(path) || is_file {
            // Hex preview — stays synchronous (only 4KB, fast enough)
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

    /// Poll for completed preview results (both preview worker and image loader).
    ///
    /// This should be called every iteration of the main event loop.
    /// Returns true if any preview was updated (caller should set needs_redraw).
    pub fn poll_preview_result(
        &mut self,
        image_picker: &mut Option<Picker>,
        state: &mut AppState,
    ) -> bool {
        let mut changed = false;

        // --- Poll preview worker ---
        if let Some(response) = self.preview_worker.try_recv() {
            // Last-request-wins: ignore stale results
            if response.serial == self.pending_serial
                && self.last_path.as_ref() == Some(&response.path)
            {
                self.is_loading = false;
                match response.payload {
                    Ok(payload) => {
                        match payload {
                            PreviewPayload::Text(tp) => {
                                self.preview_cache
                                    .insert(response.path, CachedPreview::Text(tp.clone()));
                                self.text = Some(tp);
                            }
                            PreviewPayload::Diff(dp) => {
                                self.preview_cache
                                    .insert(response.path, CachedPreview::Diff(dp.clone()));
                                self.diff = Some(dp);
                            }
                            PreviewPayload::Directory(di) => {
                                self.preview_cache
                                    .insert(response.path, CachedPreview::Directory(di.clone()));
                                self.dir_info = Some(di);
                            }
                            PreviewPayload::Archive(ap) => {
                                self.preview_cache
                                    .insert(response.path, CachedPreview::Archive(ap.clone()));
                                self.archive = Some(ap);
                            }
                            PreviewPayload::VideoMeta(metadata) => {
                                let path = self.last_path.clone().unwrap();
                                let mut video_preview = VideoPreview::new(&path, metadata);

                                // Try to extract thumbnail and load via ImageLoader
                                match extract_thumbnail(&path) {
                                    Ok(thumb_path) => {
                                        if self.image_loader.request(thumb_path.clone()) {
                                            self.loading_video_thumbnail = Some(thumb_path);
                                        }
                                    }
                                    Err(e) => {
                                        video_preview.thumbnail_error =
                                            Some(format!("Failed to extract: {}", e));
                                    }
                                }

                                self.video = Some(video_preview);
                            }
                        }
                        changed = true;
                    }
                    Err(msg) => {
                        // Diff failure falls back to text
                        if msg == "No diff available" {
                            // Re-request as text preview
                            self.is_loading = true;
                            self.pending_serial = self.preview_worker.request(
                                self.last_path.as_ref().unwrap().clone(),
                                PreviewKind::Text,
                                None,
                                None,
                            );
                        } else if msg.starts_with("Video preview requires") {
                            state.set_message(msg);
                            if let Some(ref path) = self.last_path.clone() {
                                self.load_hex_fallback(path, state);
                            }
                            changed = true;
                        } else {
                            state.set_message(msg);
                            // For video meta errors, fall back to hex
                            if let Some(ref path) = self.last_path.clone() {
                                if is_video_file(path) {
                                    self.load_hex_fallback(path, state);
                                }
                            }
                            changed = true;
                        }
                    }
                }
            }
        }

        // --- Poll image loader (same as old poll_image_result) ---
        if let Some(result) = self.image_loader.try_recv() {
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
                            changed = true;
                        }
                    }
                    Err(e) => {
                        state.set_message(format!("Failed: preview - {}", e));
                    }
                }
            } else if self.loading_video_thumbnail.as_ref() == Some(&result.path) {
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
                                changed = true;
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

        changed
    }

    /// Check if an image is currently being loaded
    pub fn is_loading_image(&self) -> bool {
        self.loading_image_path.is_some() || self.loading_video_thumbnail.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn changing_path_cancels_pending_image_result() {
        let temp = tempdir().unwrap();
        let image = temp.path().join("image.png");
        let text = temp.path().join("file.txt");
        fs::write(&image, "not a real image").unwrap();
        fs::write(&text, "text").unwrap();

        let mut preview = PreviewState::new();
        let mut picker = None;
        let mut state = AppState::new(temp.path().to_path_buf());
        preview.update(Some(&image), &mut picker, &mut state);
        assert_eq!(preview.loading_image_path.as_ref(), Some(&image));

        preview.update(Some(&text), &mut picker, &mut state);
        assert!(preview.loading_image_path.is_none());
        assert_eq!(preview.last_path.as_ref(), Some(&text));
    }

    #[test]
    fn invalidate_discards_current_preview_identity() {
        let mut preview = PreviewState::new();
        preview.last_path = Some(PathBuf::from("file.txt"));
        preview.is_loading = true;

        preview.invalidate();

        assert!(preview.last_path.is_none());
        assert!(!preview.is_loading);
        assert!(!preview.has_content());
    }
}
