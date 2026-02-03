//! Plugin system for FileView
//!
//! This module provides Lua scripting support for extending FileView.
//! Plugins can access file information, execute actions, and register
//! custom commands.
//!
//! # Plugin Location
//!
//! Plugins are loaded from `~/.config/fileview/plugins/init.lua`
//!
//! # Example Plugin
//!
//! ```lua
//! -- ~/.config/fileview/plugins/init.lua
//! fv.notify("FileView plugin loaded!")
//!
//! -- Access current file
//! local file = fv.current_file()
//! if file then
//!     print("Current file: " .. file)
//! end
//! ```

mod api;
mod lua;

pub use api::PluginContext;
pub use lua::{PluginError, PluginManager};
