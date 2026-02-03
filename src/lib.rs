//! FileView - A minimal file tree UI for terminal emulators
//!
//! This crate provides a VSCode-like file explorer TUI,
//! designed for use in modern terminal emulators like Ghostty.

pub mod action;
pub mod app;
pub mod core;
pub mod git;
pub mod handler;
pub mod integrate;
pub mod mcp;
pub mod plugin;
pub mod render;
pub mod tree;
pub mod watcher;
