//! Exercises the privacy-skip logic against the real OS clipboard, reproducing
//! the formats real apps set: the Snipping Tool / PrintScreen writes
//! `CanIncludeInClipboardHistory = 1` (an opt-IN), while password managers set
//! presence markers or the flag with value 0.
//!
//! These touch the machine's actual clipboard; the original text content is
//! saved and restored.
#![cfg(windows)]

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

use carimbo_lib::platform::os::clipboard::{get_text, set_text};
use carimbo_lib::platform::os::clipboard_monitor::should_skip_current_clipboard;

/// Empty the clipboard and set `flag_name` to a DWORD payload of `value`.
fn set_clipboard_with_flag(flag_name: PCWSTR, value: u32) {
    unsafe {
        OpenClipboard(HWND::default()).expect("open clipboard");
        EmptyClipboard().expect("empty clipboard");
        let fmt = RegisterClipboardFormatW(flag_name);
        assert_ne!(fmt, 0, "format registration failed");

        let hglobal = GlobalAlloc(GMEM_MOVEABLE, 4).expect("alloc");
        let ptr = GlobalLock(hglobal) as *mut u32;
        assert!(!ptr.is_null());
        ptr.write_unaligned(value);
        let _ = GlobalUnlock(hglobal);
        SetClipboardData(fmt, HANDLE(hglobal.0)).expect("set clipboard data");

        CloseClipboard().expect("close clipboard");
    }
}

/// One test fn (not several) so the global clipboard isn't hit concurrently.
#[test]
fn skip_logic_honours_flag_semantics() {
    let saved = get_text();

    // Snipping Tool / PrintScreen case: flag present with value 1 → capture.
    set_clipboard_with_flag(w!("CanIncludeInClipboardHistory"), 1);
    assert!(
        !should_skip_current_clipboard(),
        "CanIncludeInClipboardHistory=1 (screenshots) must NOT be skipped"
    );

    // Explicit opt-out: flag present with value 0 → skip.
    set_clipboard_with_flag(w!("CanIncludeInClipboardHistory"), 0);
    assert!(
        should_skip_current_clipboard(),
        "CanIncludeInClipboardHistory=0 must be skipped"
    );

    // Presence-only markers → skip regardless of payload.
    set_clipboard_with_flag(w!("Clipboard Viewer Ignore"), 1);
    assert!(
        should_skip_current_clipboard(),
        "Clipboard Viewer Ignore must be skipped"
    );
    set_clipboard_with_flag(w!("ExcludeClipboardContentFromMonitorProcessing"), 1);
    assert!(
        should_skip_current_clipboard(),
        "ExcludeClipboardContentFromMonitorProcessing must be skipped"
    );

    // Ordinary clipboard content with no markers → capture.
    assert!(set_text("carimbo-test"));
    assert!(!should_skip_current_clipboard());

    if let Some(t) = saved {
        set_text(&t);
    }
}
