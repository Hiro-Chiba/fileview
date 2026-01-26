//! Core module - Application state and view modes

pub mod mode;
pub mod state;

pub use mode::{InputPurpose, PendingAction, ViewMode};
pub use state::AppState;
