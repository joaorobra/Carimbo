//! Windows-specific platform implementations. Everything OS-specific lives here;
//! `core/` and the rest of the app depend only on the abstractions in
//! `platform/mod.rs`.

pub mod clipboard;
pub mod clipboard_monitor;
pub mod color_picker;
pub mod focus;
pub mod inject;
pub mod keyboard_hook;
pub mod keymap;
pub mod paste;
pub mod shell;
pub mod util;
