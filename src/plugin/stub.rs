//! No-op plugin manager used when the `lua` feature is disabled.
//!
//! `PluginManager::new()` always returns `Err(PluginError::Disabled)`, which
//! the event loop handles via `.ok()` so the optional manager stays `None`
//! for the entire session. The other methods exist so call sites that
//! happen to hold a `&mut PluginManager` (e.g. `plugin_test`) compile, but
//! they all degrade to "feature disabled" or no-op.

use std::path::PathBuf;

use super::api::{PluginAction, PluginEvent};

/// Stub error type. Mirrors the variants the real implementation surfaces
/// to callers, but always lands on `Disabled`.
#[derive(Debug)]
pub enum PluginError {
    /// The `lua` feature was not compiled in.
    Disabled,
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled => write!(
                f,
                "lua plugin support is not compiled in (rebuild with --features lua)"
            ),
        }
    }
}

impl std::error::Error for PluginError {}

/// Stub manager. Constructible only through `new()` to mirror the real API.
#[derive(Debug, Default)]
pub struct PluginManager;

impl PluginManager {
    /// Always returns `Err(Disabled)` so callers fall back to the no-plugin
    /// path. Mirrors the real `PluginManager::new` signature.
    pub fn new() -> Result<Self, PluginError> {
        Err(PluginError::Disabled)
    }

    pub fn load_plugins(&mut self) -> Result<(), PluginError> {
        Err(PluginError::Disabled)
    }

    pub fn update_context(
        &mut self,
        _focused: Option<PathBuf>,
        _root: PathBuf,
        _selected: Vec<PathBuf>,
    ) {
    }

    pub fn fire_event(
        &mut self,
        _event: PluginEvent,
        _arg: Option<&str>,
    ) -> Result<(), PluginError> {
        Ok(())
    }

    pub fn take_notifications(&mut self) -> Vec<String> {
        Vec::new()
    }

    pub fn take_actions(&mut self) -> Vec<PluginAction> {
        Vec::new()
    }

    pub fn exec(&mut self, _code: &str) -> Result<(), PluginError> {
        Err(PluginError::Disabled)
    }

    pub fn eval(&self, _code: &str) -> Result<String, PluginError> {
        Err(PluginError::Disabled)
    }

    pub fn has_command(&self, _name: &str) -> bool {
        false
    }

    pub fn list_commands(&self) -> Vec<String> {
        Vec::new()
    }

    pub fn invoke_command(&mut self, _name: &str) -> Result<(), PluginError> {
        Err(PluginError::Disabled)
    }

    pub fn has_previewer(&self, _pattern: &str) -> bool {
        false
    }

    pub fn list_previewers(&self) -> Vec<String> {
        Vec::new()
    }

    pub fn find_previewer(&self, _filename: &str) -> Option<String> {
        None
    }

    pub fn invoke_previewer(&mut self, _pattern: &str, _path: &str) -> Result<String, PluginError> {
        Err(PluginError::Disabled)
    }
}
