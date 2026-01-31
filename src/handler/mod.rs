//! Handler module - Input event handling

pub mod action;
pub mod key;
pub mod keymap;
pub mod mouse;

pub use action::{
    get_filename_str, get_target_directory, handle_action, reload_tree, ActionContext,
    ActionResult, EntrySnapshot,
};
pub use key::{
    create_delete_targets, handle_key_event, handle_key_event_with_registry, update_input_buffer,
    KeyAction,
};
pub use keymap::{KeyBindingRegistry, KeymapFile};
pub use mouse::{handle_mouse_event, ClickDetector, MouseAction, PathBuffer};
