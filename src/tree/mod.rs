//! Tree module - File tree data structure and navigation

pub mod navigator;
pub mod node;

pub use navigator::{TreeNavigator, VisibleEntry};
pub use node::TreeEntry;
