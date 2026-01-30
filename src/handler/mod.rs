//! Handler module - Input event handling

pub mod action;
pub mod key;
pub mod mouse;

pub use action::{
    get_filename_str, get_target_directory, handle_action, reload_tree, ActionContext,
    ActionResult, EntrySnapshot,
};
pub use key::{create_delete_targets, handle_key_event, update_input_buffer, KeyAction};
pub use mouse::{handle_mouse_event, ClickDetector, MouseAction, PathBuffer};
