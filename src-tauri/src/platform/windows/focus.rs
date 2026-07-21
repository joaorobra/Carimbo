//! Foreground-window capture and restore.
//!
//! The palette flow is: record the foreground window BEFORE showing the palette,
//! then after the user picks a snippet, restore that window to the foreground and
//! paste into it. Restoring foreground from a background process is deliberately
//! restricted by Windows; we use the documented escalation of workarounds and
//! always VERIFY the target is actually frontmost before pasting so we never
//! paste into the wrong application.

#[cfg(windows)]
mod imp {
    use std::thread::sleep;
    use std::time::Duration;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_MENU};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
    };

    use super::super::util::CARIMBO_INJECT_TAG;

    /// An opaque, validated handle to the window that was frontmost when we
    /// captured it. `HWND` is just a pointer; we re-check `IsWindow` before use.
    #[derive(Clone, Copy)]
    pub struct CapturedWindow(pub HWND);

    // HWND is not Send by default in the windows crate; the palette flow captures
    // and restores on the same (UI) thread, but we mark it so it can live in
    // shared state. Access is always re-validated with IsWindow before use.
    unsafe impl Send for CapturedWindow {}
    unsafe impl Sync for CapturedWindow {}

    /// Capture the current foreground window. Returns `None` if there is none
    /// (e.g. desktop focused).
    pub fn capture_foreground() -> Option<CapturedWindow> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            None
        } else {
            Some(CapturedWindow(hwnd))
        }
    }

    /// True if `w` still exists and is currently the foreground window.
    pub fn is_foreground(w: CapturedWindow) -> bool {
        unsafe { IsWindow(w.0).as_bool() && GetForegroundWindow() == w.0 }
    }

    /// Attempt to bring `w` back to the foreground. Returns true only if, after
    /// the attempt, `w` is verified frontmost. Uses escalating strategies:
    /// 1. Plain SetForegroundWindow (usually works right after we had focus).
    /// 2. AttachThreadInput bridge to bypass the foreground lock.
    /// 3. Synthetic ALT tap to satisfy the "user provided input" heuristic.
    pub fn restore_foreground(w: CapturedWindow) -> bool {
        unsafe {
            if !IsWindow(w.0).as_bool() {
                return false;
            }
        }

        // Strategy 1.
        unsafe {
            let _ = SetForegroundWindow(w.0);
        }
        if is_foreground(w) {
            return true;
        }

        // Strategy 2: attach our input queue to the target thread's so the
        // foreground-lock treats us as the same input context.
        unsafe {
            let our_tid = GetCurrentThreadId();
            let mut target_pid = 0u32;
            let target_tid = GetWindowThreadProcessId(w.0, Some(&mut target_pid));
            if target_tid != 0 && target_tid != our_tid {
                let attached = AttachThreadInput(our_tid, target_tid, true).as_bool();
                let _ = SetForegroundWindow(w.0);
                if attached {
                    let _ = AttachThreadInput(our_tid, target_tid, false);
                }
            }
        }
        if is_foreground(w) {
            return true;
        }

        // Strategy 3: a synthetic ALT press/release makes Windows consider this
        // an interactive foreground change. Tagged so our own hook ignores it.
        tap_alt();
        unsafe {
            let _ = SetForegroundWindow(w.0);
        }
        // Give the compositor a moment, then verify.
        sleep(Duration::from_millis(20));
        is_foreground(w)
    }

    fn tap_alt() {
        let down = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_MENU,
                    wScan: 0,
                    dwFlags: Default::default(),
                    time: 0,
                    dwExtraInfo: CARIMBO_INJECT_TAG,
                },
            },
        };
        let up = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_MENU,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: CARIMBO_INJECT_TAG,
                },
            },
        };
        let inputs = [down, up];
        unsafe {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
    }
}

#[cfg(windows)]
pub use imp::*;
