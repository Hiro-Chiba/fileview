//! Preview rendering (text, images, directory info, archives, PDFs, video, diff, custom)
//!
//! This module provides preview functionality for various file types:
//! - Text files with syntax highlighting
//! - Binary files with hex dump
//! - Images with terminal graphics protocols
//! - Archives (zip, tar.gz)
//! - PDFs (requires poppler-utils)
//! - Videos with thumbnail and metadata
//! - Git diffs
//! - Custom external command output
//! - Directory information

pub mod archive;
pub mod common;
pub mod custom;
pub mod diff;
pub mod directory;
pub mod hex;
pub mod image;
pub mod pdf;
pub mod text;
pub mod video;

// Re-export common utilities
pub use common::{format_size, get_border_style};

// Re-export archive types and functions
pub use archive::{
    is_archive_file, is_tar_gz_file, render_archive_preview, ArchiveEntry, ArchivePreview,
};

// Re-export custom preview
pub use custom::{render_custom_preview, CustomPreview};

// Re-export diff preview
pub use diff::{render_diff_preview, DiffPreview};

// Re-export directory info
pub use directory::{render_directory_info, DirectoryInfo};

// Re-export hex preview and binary detection
pub use hex::{is_binary_file, render_hex_preview, HexPreview};

// Re-export image preview
pub use image::{calculate_centered_image_area, is_image_file, render_image_preview, ImagePreview};

// Re-export PDF preview
pub use pdf::{find_pdftoppm, is_pdf_file, render_pdf_preview, PdfPreview};

// Re-export text preview and detection
pub use text::{is_text_file, render_text_preview, StyledLine, StyledSegment, TextPreview};

// Re-export video preview
pub use video::{render_video_preview, VideoPreview};
