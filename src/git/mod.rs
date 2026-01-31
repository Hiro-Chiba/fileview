//! Git integration module

mod diff;
mod operations;
mod status;

pub use diff::{get_diff, DiffLine, FileDiff};
pub use operations::{is_staged, stage, unstage};
pub use status::{FileStatus, GitStatus};
