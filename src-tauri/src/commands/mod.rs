//! Tauri command layer. Commands are thin: validate/deserialize, call a repo or
//! core function, emit a change event. No business logic or SQL lives here.

pub mod backup;
pub mod clipboard;
pub mod color;
pub mod expansion;
pub mod folders;
pub mod hotkey;
pub mod palette;
pub mod radial;
pub mod settings;
pub mod snippets;
