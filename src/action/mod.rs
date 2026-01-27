//! Action module - File operations and clipboard

pub mod clipboard;
pub mod file;

pub use clipboard::{Clipboard, ClipboardContent};
pub use file::{copy_to, create_dir, create_file, delete, rename};
