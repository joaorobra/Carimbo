//! Text injection primitives: synthetic backspaces, Ctrl+V paste, and unicode
//! typing. Every event is tagged with `CARIMBO_INJECT_TAG` in `dwExtraInfo` so
//! our own low-level keyboard hook passes them through untouched.

#[cfg(windows)]
mod imp {
    use std::thread::sleep;
    use std::time::Duration;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_LEFT, VK_V,
    };

    use super::super::util::CARIMBO_INJECT_TAG;

    fn key_event(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: scan,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: CARIMBO_INJECT_TAG,
                },
            },
        }
    }

    fn send(inputs: &[INPUT]) {
        if inputs.is_empty() {
            return;
        }
        unsafe {
            SendInput(inputs, std::mem::size_of::<INPUT>() as i32);
        }
    }

    /// Send `count` Backspace key presses (down+up each).
    pub fn send_backspaces(count: usize) {
        if count == 0 {
            return;
        }
        let mut inputs = Vec::with_capacity(count * 2);
        for _ in 0..count {
            inputs.push(key_event(VK_BACK, 0, KEYBD_EVENT_FLAGS(0)));
            inputs.push(key_event(VK_BACK, 0, KEYEVENTF_KEYUP));
        }
        send(&inputs);
    }

    /// Send `count` Left-arrow key presses (down+up each). Used to move the caret
    /// back to a `{cursor}` marker after pasting an expanded snippet.
    pub fn send_left_arrows(count: usize) {
        if count == 0 {
            return;
        }
        let mut inputs = Vec::with_capacity(count * 2);
        for _ in 0..count {
            inputs.push(key_event(VK_LEFT, 0, KEYBD_EVENT_FLAGS(0)));
            inputs.push(key_event(VK_LEFT, 0, KEYEVENTF_KEYUP));
        }
        send(&inputs);
    }

    /// Simulate Ctrl+V.
    pub fn send_paste() {
        let inputs = [
            key_event(VK_CONTROL, 0, KEYBD_EVENT_FLAGS(0)),
            key_event(VK_V, 0, KEYBD_EVENT_FLAGS(0)),
            key_event(VK_V, 0, KEYEVENTF_KEYUP),
            key_event(VK_CONTROL, 0, KEYEVENTF_KEYUP),
        ];
        send(&inputs);
    }

    /// Type `text` as unicode key events. Slower than paste but never touches the
    /// clipboard — used as a fallback for paste-blocking apps/terminals.
    ///
    /// Each UTF-16 code unit is sent as a KEYEVENTF_UNICODE down+up pair, so
    /// characters outside the BMP (surrogate pairs) are handled correctly.
    pub fn type_unicode(text: &str) {
        let mut inputs = Vec::new();
        for unit in text.encode_utf16() {
            inputs.push(key_event(VIRTUAL_KEY(0), unit, KEYEVENTF_UNICODE));
            inputs.push(key_event(
                VIRTUAL_KEY(0),
                unit,
                KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
            ));
        }
        // Chunk large payloads so a single SendInput call isn't gigantic; also
        // gives slow apps a chance to keep up.
        const CHUNK: usize = 40; // 20 characters (down+up) per batch
        for batch in inputs.chunks(CHUNK) {
            send(batch);
            sleep(Duration::from_millis(1));
        }
    }
}

#[cfg(windows)]
pub use imp::*;
