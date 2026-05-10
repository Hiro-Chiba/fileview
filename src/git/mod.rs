//! Git integration module

mod diff;
mod operations;
pub mod range;
mod status;

pub use diff::{get_diff, DiffLine, FileDiff};
pub use operations::{is_staged, stage, unstage};
pub use range::{compute as compute_diff_range, DiffRange};
pub use status::{FileStatus, GitStatus};
