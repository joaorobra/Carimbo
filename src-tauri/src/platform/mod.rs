//! Platform abstraction boundary.
//!
//! OS-specific integrations (clipboard monitoring, keyboard hooking, text
//! injection, foreground-window control) are implemented per-platform under the
//! submodules here. v1 ships the Windows implementation; Mac/Linux later add
//! sibling modules without touching `core/`.

#[cfg(windows)]
pub mod windows;

// Re-export the current platform under a stable name so call sites don't repeat
// cfg gates. As other OSes are added, mirror this pattern.
#[cfg(windows)]
pub use windows as os;
