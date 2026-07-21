//! Low-level keyboard hook (WH_KEYBOARD_LL).
//!
//! Design constraints that shaped this module:
//! * The hook callback runs on a thread that MUST own a message pump, and must
//!   return quickly — Windows silently unhooks a callback that takes too long
//!   (LowLevelHooksTimeout, ~300 ms default). So the callback does the bare
//!   minimum: skip our own injected events, forward key info over a channel, and
//!   call CallNextHookEx.
//! * We never touch the DB, allocate on the heap, or take locks inside the
//!   callback. All matching/expansion happens on a separate consumer thread.
//! * A single global hook handle + channel sender are stored in statics because
//!   the C callback signature carries no user pointer.

#[cfg(windows)]
mod imp {
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::sync::Mutex;
    use windows::Win32::Foundation::{HMODULE, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
        KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
    };

    use super::super::util::CARIMBO_INJECT_TAG;

    /// A key-down event observed by the hook. `vk` is the virtual-key code,
    /// `scan` the hardware scan code, `injected` true if it carried our tag.
    #[derive(Clone, Copy, Debug)]
    pub struct KeyEvent {
        pub vk: u32,
        pub scan: u32,
        pub injected: bool,
    }

    // The callback has no user-data parameter, so the sender lives in a static.
    // A Mutex<Option<..>> (not OnceLock) so the hook can be uninstalled and
    // re-installed at runtime — the M4 "toggle expansion" setting depends on it.
    static SENDER: Mutex<Option<Sender<KeyEvent>>> = Mutex::new(None);

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        // code < 0 means we must pass through without processing (HC_ACTION is 0).
        if code >= 0 {
            let msg = wparam.0 as u32;
            if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                let injected = kb.dwExtraInfo == CARIMBO_INJECT_TAG;
                // try_lock keeps the callback non-blocking: the only contender is
                // install/drop, which hold the lock for a couple of instructions.
                // Missing an event during that window is harmless.
                if let Ok(guard) = SENDER.try_lock() {
                    if let Some(tx) = guard.as_ref() {
                        // Unbounded channel: send never blocks. If the receiver
                        // is gone we simply drop the event.
                        let _ = tx.send(KeyEvent {
                            vk: kb.vkCode,
                            scan: kb.scanCode,
                            injected,
                        });
                    }
                }
            }
        }
        // We are a passive listener: always let the keystroke continue to the
        // focused app. (The expansion engine deletes triggers by sending
        // backspaces AFTER the fact, it never swallows the user's keys here.)
        CallNextHookEx(None, code, wparam, lparam)
    }

    /// Owns the hook thread. Dropping it unhooks and stops the pump.
    pub struct KeyboardHook {
        thread: Option<std::thread::JoinHandle<()>>,
        thread_id: u32,
    }

    impl KeyboardHook {
        /// Install the hook on a dedicated thread and return a receiver of key
        /// events. Returns `Err` if a hook is already installed (only one
        /// process-global low-level hook is kept at a time).
        pub fn install() -> Result<(KeyboardHook, Receiver<KeyEvent>), String> {
            let (tx, rx) = channel::<KeyEvent>();
            {
                let mut guard = SENDER
                    .lock()
                    .map_err(|_| "keyboard hook state poisoned".to_string())?;
                if guard.is_some() {
                    return Err("keyboard hook already installed".into());
                }
                *guard = Some(tx);
            }

            // Channel to receive the hook thread's own thread-id so we can post
            // WM_QUIT to it on shutdown.
            let (id_tx, id_rx) = channel::<u32>();

            let thread = std::thread::Builder::new()
                .name("carimbo-kbd-hook".into())
                .spawn(move || {
                    // hMod may be null for a low-level hook whose proc lives in
                    // the current process; dwThreadId 0 makes it global.
                    let hook = unsafe {
                        SetWindowsHookExW(
                            WH_KEYBOARD_LL,
                            Some(hook_proc),
                            HMODULE::default(),
                            0,
                        )
                    };
                    let hook: HHOOK = match hook {
                        Ok(h) => h,
                        Err(e) => {
                            tracing::error!("SetWindowsHookExW failed: {e}");
                            let _ = id_tx.send(0);
                            return;
                        }
                    };
                    let _ = id_tx.send(unsafe {
                        windows::Win32::System::Threading::GetCurrentThreadId()
                    });

                    // Message pump — required to receive LL hook callbacks.
                    let mut msg = MSG::default();
                    unsafe {
                        // GetMessageW returns 0 on WM_QUIT, -1 on error.
                        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                            // No dispatch needed; the hook fires independently.
                        }
                        let _ = UnhookWindowsHookEx(hook);
                    }
                })
                .map_err(|e| format!("failed to spawn hook thread: {e}"))?;

            let thread_id = id_rx.recv().unwrap_or(0);
            if thread_id == 0 {
                // The hook thread failed to install the hook (or panicked before
                // reporting). Clear the sender we stored so a later install() can
                // succeed, and join the dead thread.
                clear_sender();
                let _ = thread.join();
                return Err("hook thread failed to install hook".into());
            }

            Ok((
                KeyboardHook {
                    thread: Some(thread),
                    thread_id,
                },
                rx,
            ))
        }
    }

    /// Clears the global sender so a subsequent `install()` can succeed. Safe to
    /// call even if already cleared.
    fn clear_sender() {
        if let Ok(mut guard) = SENDER.lock() {
            *guard = None;
        }
    }

    impl Drop for KeyboardHook {
        fn drop(&mut self) {
            // Post WM_QUIT to the hook thread's queue to break its message loop.
            if self.thread_id != 0 {
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                        self.thread_id,
                        windows::Win32::UI::WindowsAndMessaging::WM_QUIT,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
            }
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
            // Release the global sender so the hook can be re-installed later
            // (M4 runtime enable/disable toggle).
            clear_sender();
        }
    }
}

#[cfg(windows)]
pub use imp::*;
