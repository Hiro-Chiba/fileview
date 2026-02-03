//! Core module - Application state and view modes

pub mod mode;
pub mod state;
pub mod tab;

pub use mode::{FocusTarget, InputPurpose, PendingAction, ViewMode};
pub use state::{AppState, PreviewDisplayMode, SortMode, UiDensity, BOOKMARK_SLOTS};
pub use tab::{Tab, TabManager};
