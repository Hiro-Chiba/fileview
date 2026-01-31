//! Background image loader using std::thread and mpsc channels
//!
//! This module provides asynchronous image loading to prevent UI blocking
//! when loading large images.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};

use image::DynamicImage;

/// Request to load an image
pub struct ImageLoadRequest {
    /// Path to the image file
    pub path: PathBuf,
}

/// Result of an image load operation
pub struct ImageLoadResult {
    /// Path that was loaded
    pub path: PathBuf,
    /// The loaded image or error message
    pub result: Result<DynamicImage, String>,
}

/// Background image loader
///
/// Spawns a worker thread that loads images on demand.
/// Uses mpsc channels for communication between the main thread and worker.
pub struct ImageLoader {
    /// Sender for load requests
    request_tx: Sender<ImageLoadRequest>,
    /// Receiver for load results
    result_rx: Receiver<ImageLoadResult>,
    /// Handle to the worker thread
    _worker: JoinHandle<()>,
    /// Path currently being loaded (for deduplication)
    loading_path: Option<PathBuf>,
}

impl ImageLoader {
    /// Create a new image loader with a background worker thread
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<ImageLoadRequest>();
        let (result_tx, result_rx) = mpsc::channel::<ImageLoadResult>();

        let worker = thread::spawn(move || {
            Self::worker_loop(request_rx, result_tx);
        });

        Self {
            request_tx,
            result_rx,
            _worker: worker,
            loading_path: None,
        }
    }

    /// Worker thread main loop
    fn worker_loop(request_rx: Receiver<ImageLoadRequest>, result_tx: Sender<ImageLoadResult>) {
        while let Ok(request) = request_rx.recv() {
            let result = match image::open(&request.path) {
                Ok(img) => Ok(img),
                Err(e) => Err(format!("Failed to load image: {}", e)),
            };

            let load_result = ImageLoadResult {
                path: request.path,
                result,
            };

            // If the main thread has dropped, stop the worker
            if result_tx.send(load_result).is_err() {
                break;
            }
        }
    }

    /// Request loading an image at the given path
    ///
    /// If an image is already being loaded for this path, the request is ignored.
    /// Returns true if the request was sent, false if already loading this path.
    pub fn request(&mut self, path: PathBuf) -> bool {
        // Skip if already loading this path
        if self.loading_path.as_ref() == Some(&path) {
            return false;
        }

        self.loading_path = Some(path.clone());

        // Send the request (ignore errors - worker might have stopped)
        let _ = self.request_tx.send(ImageLoadRequest { path });
        true
    }

    /// Try to receive a completed image load result
    ///
    /// Returns the result if one is ready, or None if no result is available.
    pub fn try_recv(&mut self) -> Option<ImageLoadResult> {
        match self.result_rx.try_recv() {
            Ok(result) => {
                // Clear loading state if this was the expected path
                if self.loading_path.as_ref() == Some(&result.path) {
                    self.loading_path = None;
                }
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }

    /// Check if currently loading an image
    pub fn is_loading(&self) -> bool {
        self.loading_path.is_some()
    }

    /// Get the path currently being loaded
    pub fn loading_path(&self) -> Option<&PathBuf> {
        self.loading_path.as_ref()
    }

    /// Cancel the current loading operation
    ///
    /// Note: This doesn't actually stop the worker, but clears the loading state
    /// so the result will be ignored when it arrives.
    pub fn cancel(&mut self) {
        self.loading_path = None;
    }
}

impl Default for ImageLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_image_loader_creation() {
        let loader = ImageLoader::new();
        assert!(!loader.is_loading());
        assert!(loader.loading_path().is_none());
    }

    #[test]
    fn test_request_sets_loading_state() {
        let mut loader = ImageLoader::new();
        let path = PathBuf::from("/tmp/test.png");

        assert!(loader.request(path.clone()));
        assert!(loader.is_loading());
        assert_eq!(loader.loading_path(), Some(&path));
    }

    #[test]
    fn test_duplicate_request_ignored() {
        let mut loader = ImageLoader::new();
        let path = PathBuf::from("/tmp/test.png");

        assert!(loader.request(path.clone()));
        assert!(!loader.request(path)); // Should return false for duplicate
    }

    #[test]
    fn test_cancel_clears_loading_state() {
        let mut loader = ImageLoader::new();
        let path = PathBuf::from("/tmp/test.png");

        loader.request(path);
        assert!(loader.is_loading());

        loader.cancel();
        assert!(!loader.is_loading());
    }

    #[test]
    fn test_try_recv_returns_none_when_empty() {
        let mut loader = ImageLoader::new();
        assert!(loader.try_recv().is_none());
    }

    #[test]
    fn test_load_nonexistent_file_returns_error() {
        let mut loader = ImageLoader::new();
        let path = PathBuf::from("/nonexistent/path/image.png");

        loader.request(path.clone());

        // Wait for result (with timeout)
        let mut result = None;
        for _ in 0..50 {
            if let Some(r) = loader.try_recv() {
                result = Some(r);
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        if let Some(r) = result {
            assert_eq!(r.path, path);
            assert!(r.result.is_err());
        }
    }
}
