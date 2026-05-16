//! Tests for action handlers.
//!
//! Split into sibling modules by category for navigability:
//! - [`basic`]: smoke tests for individual actions
//! - [`state_transition`]: bug fixes captured as regression tests (Phase 13.2)
//! - [`sequence`]: multi-step user workflows (Phase 13.3)
//! - [`edge_cases`]: boundary conditions (Phase 13.4)
//! - [`focus`]: focus toggle and focus-aware behavior (Phase 14)
//! - [`scroll_bounds`]: preview scroll bounds (v1.9.2)

#![allow(clippy::module_inception, unused_imports)]

use std::path::Path;

pub(super) use crate::core::{AppState, FocusTarget, ViewMode};
pub(super) use crate::handler::key::KeyAction;
pub(super) use crate::integrate::exit_code;
pub(super) use crate::render::{
    ArchiveEntry, ArchivePreview, CustomPreview, DiffPreview, HexPreview, PdfPreview, Picker,
    TextPreview,
};
pub(super) use crate::tree::TreeNavigator;

pub(super) use super::{
    get_filename_str, get_target_directory, handle_action, ActionContext, ActionResult,
    EntrySnapshot,
};

/// Call `handle_action` with all required preview arguments stubbed out.
macro_rules! call_handle_action {
    ($action:expr, $state:expr, $navigator:expr, $path:expr, $entries:expr, $context:expr,
     $text_preview:expr, $hex_preview:expr, $archive_preview:expr) => {{
        let mut pdf_preview: Option<$crate::render::PdfPreview> = None;
        let mut diff_preview: Option<$crate::render::DiffPreview> = None;
        let mut custom_preview: Option<$crate::render::CustomPreview> = None;
        let mut image_picker: Option<$crate::render::Picker> = None;
        $crate::handler::action::handle_action(
            $action,
            $state,
            $navigator,
            $path,
            $entries,
            $context,
            $text_preview,
            $hex_preview,
            $archive_preview,
            &mut pdf_preview,
            &mut diff_preview,
            &mut custom_preview,
            &mut image_picker,
        )
    }};
}
pub(crate) use call_handle_action;

pub(super) fn create_test_state(root: &Path) -> AppState {
    AppState::new(root.to_path_buf())
}

pub(super) fn create_test_navigator(root: &Path) -> TreeNavigator {
    TreeNavigator::new(root, false).unwrap()
}

pub(super) fn create_test_entries(navigator: &mut TreeNavigator) -> Vec<EntrySnapshot> {
    navigator.ensure_cache();
    navigator
        .visible_entries()
        .iter()
        .map(|e| EntrySnapshot {
            path: e.path.clone(),
            name: e.name.clone(),
            is_dir: e.is_dir,
            depth: e.depth,
        })
        .collect()
}

mod basic;
mod edge_cases;
mod focus;
mod scroll_bounds;
mod sequence;
mod state_transition;
