//! Virtual-key → character translation for the expansion matcher.
//!
//! Correctness note (this is the ABNT2 / dead-key trap the plan flagged):
//! `ToUnicodeEx` mutates the kernel keyboard state for the target layout when it
//! processes a dead key (e.g. the acute accent on Brazilian ABNT2). If we call
//! it against the LIVE keyboard state while the user is actually typing dead-key
//! sequences, we corrupt their input. We therefore translate against a COPY of
//! the keyboard state and, crucially, we do NOT try to compose dead keys inside
//! the matcher. A dead key returns "no character" here; the matcher just resets
//! its buffer on it. Triggers are plain ASCII-ish sequences (e.g. ";cpf"), so we
//! only need direct, non-composing translation.
//!
//! Thread-affinity note (the bug that made the M0 spike work but the service
//! not): the expansion consumer runs on a plain worker thread with no message
//! queue. `GetKeyboardState` only reflects input a thread pulls from ITS OWN
//! queue, so on the worker it stays all-zeros; and `GetKeyboardLayout(0)`
//! returns the WORKER thread's layout (typically US), not the foreground app's.
//! On an ABNT2 keyboard that combination makes `ToUnicodeEx` return the wrong
//! character (or none) for the very keys in ";cpf", so the trigger never forms
//! and nothing expands. We therefore (a) look up the FOREGROUND window's thread
//! layout rather than the calling thread's, and (b) build the modifier state
//! ourselves from `GetAsyncKeyState`, which is thread-independent — instead of
//! trusting the empty per-thread `GetKeyboardState`.

#[cfg(windows)]
mod imp {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyboardLayout, ToUnicodeEx, VIRTUAL_KEY, VK_CAPITAL, VK_CONTROL,
        VK_MENU, VK_SHIFT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    /// Translate a key event to a character, if it produces exactly one.
    /// Returns `None` for dead keys, modifiers, and non-character keys.
    ///
    /// `scan` is the hardware scan code from the hook struct. We build the
    /// keyboard state ourselves (so Shift/CapsLock are honoured) and pass it to
    /// ToUnicodeEx; the buffer is local, so we never mutate the real state.
    pub fn vk_to_char(vk: u32, scan: u32) -> Option<char> {
        unsafe {
            // Use the layout of the thread that owns the foreground window — the
            // app the user is actually typing into — NOT the calling (worker)
            // thread's layout, which would be wrong for non-US keyboards. Fall
            // back to the current thread's layout if there's no foreground window.
            let hkl = match foreground_thread_id() {
                Some(tid) => GetKeyboardLayout(tid),
                None => GetKeyboardLayout(0),
            };

            // GetKeyboardState is per-thread and is empty on our worker thread, so
            // reconstruct the bits ToUnicodeEx actually reads from the async key
            // state (thread-independent): Shift (down), and CapsLock (toggled).
            let mut state = [0u8; 256];
            if key_is_down(VK_SHIFT.0) {
                state[VK_SHIFT.0 as usize] = 0x80;
            }
            if key_is_down(VK_CONTROL.0) {
                state[VK_CONTROL.0 as usize] = 0x80;
            }
            if key_is_down(VK_MENU.0) {
                state[VK_MENU.0 as usize] = 0x80;
            }
            if key_is_toggled(VK_CAPITAL.0) {
                state[VK_CAPITAL.0 as usize] = 0x01;
            }

            let mut buf = [0u16; 8];
            // wFlags bit 2 (0x4) = "keyboard state is not changed" (Win10 1607+),
            // so a dead key won't alter kernel state. Combined with the local
            // buffer, dead keys stay inert here.
            let n = ToUnicodeEx(vk, scan, &state, &mut buf, 0x4, hkl);

            match n {
                // n == 1 → one real character produced.
                1 => char::from_u32(buf[0] as u32),
                // n < 0 → a dead key; n == 0 → no translation. Both: no char.
                _ => None,
            }
        }
    }

    /// The thread id owning the current foreground window, or `None` if there is
    /// no foreground window.
    unsafe fn foreground_thread_id() -> Option<u32> {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let tid = GetWindowThreadProcessId(hwnd, None);
        if tid == 0 {
            None
        } else {
            Some(tid)
        }
    }

    /// True if the given virtual key is currently physically down. `GetAsyncKeyState`
    /// is not tied to any thread's message queue, so it works from our worker.
    unsafe fn key_is_down(vk: u16) -> bool {
        // High-order bit of the SHORT return set => key is down.
        (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0
    }

    /// True if the given toggle key (e.g. CapsLock) is currently toggled on.
    unsafe fn key_is_toggled(vk: u16) -> bool {
        // Low-order bit set => key is toggled on.
        (GetAsyncKeyState(vk as i32) as u16 & 0x0001) != 0
    }

    /// Virtual-key codes that should reset the expansion buffer (they move the
    /// caret or commit, so any partial trigger before them is void).
    pub fn is_buffer_resetting_key(vk: u32) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LEFT, VK_NEXT, VK_PRIOR,
            VK_RETURN, VK_RIGHT, VK_TAB, VK_UP,
        };
        let v = VIRTUAL_KEY(vk as u16);
        matches!(
            v,
            VK_RETURN
                | VK_TAB
                | VK_ESCAPE
                | VK_LEFT
                | VK_RIGHT
                | VK_UP
                | VK_DOWN
                | VK_HOME
                | VK_END
                | VK_PRIOR
                | VK_NEXT
                | VK_DELETE
        )
    }

    /// True for the Backspace key (the matcher shortens its buffer by one).
    pub fn is_backspace(vk: u32) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::VK_BACK;
        VIRTUAL_KEY(vk as u16) == VK_BACK
    }
}

#[cfg(windows)]
pub use imp::*;
