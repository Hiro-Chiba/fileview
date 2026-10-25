//! Preview rendering for each supported file type.

pub mod archive;
pub mod common;
pub mod custom;
pub mod diff;
pub mod directory;
pub mod hex;
pub mod image;
pub mod markdown;
pub mod pdf;
pub mod text;
pub mod video;
pub mod window;

pub use common::{format_size, get_border_style};

pub use archive::{
    is_archive_file, is_tar_gz_file, render_archive_preview, ArchiveEntry, ArchivePreview,
};

pub use custom::{render_custom_preview, CustomPreview};

pub use diff::{render_diff_preview, DiffPreview};

pub use directory::{render_directory_info, DirectoryInfo};

pub use hex::{is_binary_file, render_hex_preview, HexPreview};

pub use image::{
    calculate_centered_image_area, is_image_file, open_image, render_image_preview, ImagePreview,
};

pub use pdf::{find_pdftoppm, is_pdf_file, render_pdf_preview, PdfPreview};

pub use text::{
    is_text_file, looks_like_text, render_text_preview, warmup as warmup_syntax, StyledLine,
    StyledSegment, TextPreview,
};

pub use video::{render_video_preview, VideoPreview};
