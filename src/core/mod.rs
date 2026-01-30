//! Core module - Application state and view modes

pub mod mode;
pub mod state;

pub use mode::{FocusTarget, InputPurpose, PendingAction, ViewMode};
pub use state::{AppState, SortMode, BOOKMARK_SLOTS};
