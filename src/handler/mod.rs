//! Handler module - Input event handling

pub mod key;
pub mod mouse;

pub use key::{create_delete_targets, handle_key_event, update_input_buffer, KeyAction};
pub use mouse::{
    handle_mouse_event, ClickDetector, DropDetector, MouseAction,
};
