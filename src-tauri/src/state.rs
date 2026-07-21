//! Shared application state managed by Tauri and injected into commands.

use std::sync::{Arc, Mutex};

use crate::core::db::Db;

#[cfg(windows)]
use crate::platform::os::focus::CapturedWindow;

pub struct AppState {
    pub db: Arc<Db>,
    /// The palette hotkey accelerator string currently registered with the OS
    /// (e.g. "Control+Shift+Space"). Tracked so we can unregister it before
    /// registering a replacement when the user changes it in settings.
    pub current_hotkey: Mutex<Option<String>>,
    /// The *secondary* palette hotkey currently registered, if any. It opens the
    /// palette on the non-main tab (see `palette.mainTab`). `None` when the user
    /// hasn't set a second shortcut. Tracked, like `current_hotkey`, so it can be
    /// unregistered before a replacement is registered.
    pub current_hotkey2: Mutex<Option<String>>,
    /// The screen-color-picker hotkey currently registered, if any. Tracked so it
    /// can be unregistered before a replacement is registered when the user
    /// changes it in settings.
    pub current_color_hotkey: Mutex<Option<String>>,
    /// True while a color pick started by the color hotkey is in flight. When the
    /// pick completes, this tells the picker to open the manager on the Colors
    /// page with the picked color (vs. an in-view pick, which stays in place).
    pub color_pick_to_colors: std::sync::atomic::AtomicBool,
    /// The window that was frontmost when the palette hotkey fired. Set just
    /// before the palette is shown, consumed when a snippet is chosen so it can
    /// be pasted into the right app.
    #[cfg(windows)]
    pub palette_target: Mutex<Option<CapturedWindow>>,
    /// Keeps the clipboard monitor thread alive for the app's lifetime. Dropping
    /// it stops monitoring.
    #[cfg(windows)]
    pub clipboard_monitor:
        Mutex<Option<crate::platform::os::clipboard_monitor::ClipboardMonitor>>,
    /// The abbreviation-expansion service. `None` when disabled; installing it
    /// starts the keyboard hook, dropping it uninstalls the hook.
    #[cfg(windows)]
    pub expansion: Mutex<Option<crate::expansion_service::ExpansionService>>,
    /// Context for an in-flight disambiguation picker (radial). Set when the
    /// expansion engine detects several confusably-similar triggers and shows
    /// the radial; consumed when the user picks or dismisses it.
    #[cfg(windows)]
    pub pending_radial: Mutex<Option<PendingRadial>>,
    /// How many characters of a just-typed trigger to backspace before the next
    /// palette insert. Set only when a typed trigger opens the palette's fill-in
    /// form (the trigger text is still in the target app and must be removed
    /// before the expansion is pasted). None on the hotkey/tray path, where
    /// nothing was typed. Consumed by `palette_insert`.
    #[cfg(windows)]
    pub pending_trigger_delete: Mutex<Option<usize>>,
}

/// What the radial picker needs to complete an expansion once the user chooses:
/// the app to paste back into and how many characters of the typed trigger to
/// delete first.
#[cfg(windows)]
#[derive(Clone, Copy)]
pub struct PendingRadial {
    pub target: CapturedWindow,
    pub delete_chars: usize,
}

impl AppState {
    pub fn new(db: Arc<Db>) -> Self {
        AppState {
            db,
            current_hotkey: Mutex::new(None),
            current_hotkey2: Mutex::new(None),
            current_color_hotkey: Mutex::new(None),
            color_pick_to_colors: std::sync::atomic::AtomicBool::new(false),
            #[cfg(windows)]
            palette_target: Mutex::new(None),
            #[cfg(windows)]
            clipboard_monitor: Mutex::new(None),
            #[cfg(windows)]
            expansion: Mutex::new(None),
            #[cfg(windows)]
            pending_radial: Mutex::new(None),
            #[cfg(windows)]
            pending_trigger_delete: Mutex::new(None),
        }
    }

    #[cfg(windows)]
    pub fn set_pending_trigger_delete(&self, n: usize) {
        if let Ok(mut guard) = self.pending_trigger_delete.lock() {
            *guard = Some(n);
        }
    }

    #[cfg(windows)]
    pub fn take_pending_trigger_delete(&self) -> Option<usize> {
        self.pending_trigger_delete
            .lock()
            .ok()
            .and_then(|mut g| g.take())
    }

    #[cfg(windows)]
    pub fn set_pending_radial(&self, p: PendingRadial) {
        if let Ok(mut guard) = self.pending_radial.lock() {
            *guard = Some(p);
        }
    }

    #[cfg(windows)]
    pub fn take_pending_radial(&self) -> Option<PendingRadial> {
        self.pending_radial.lock().ok().and_then(|mut g| g.take())
    }

    #[cfg(windows)]
    pub fn set_clipboard_monitor(
        &self,
        m: crate::platform::os::clipboard_monitor::ClipboardMonitor,
    ) {
        if let Ok(mut guard) = self.clipboard_monitor.lock() {
            *guard = Some(m);
        }
    }

    #[cfg(windows)]
    pub fn set_palette_target(&self, w: Option<CapturedWindow>) {
        if let Ok(mut guard) = self.palette_target.lock() {
            *guard = w;
        }
    }

    #[cfg(windows)]
    pub fn take_palette_target(&self) -> Option<CapturedWindow> {
        self.palette_target.lock().ok().and_then(|mut g| g.take())
    }
}
