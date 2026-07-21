//! Regression test for the expansion thread-affinity bug.
//!
//! The M0 spike consumed key events on the main (GUI) thread, so
//! `vk_to_char`'s `GetKeyboardState`/`GetKeyboardLayout(0)` were meaningful.
//! The production `ExpansionService` moved translation onto a plain worker
//! thread with no message queue, where those per-thread APIs return empty
//! state / the wrong layout — so on non-US keyboards the trigger never formed
//! and nothing expanded.
//!
//! `vk_to_char` must therefore translate correctly from a WORKER thread. We
//! translate a virtual key whose character is layout-stable for the letter row
//! (VK_A -> 'a' with no modifiers on US/ABNT2 alike) and assert it succeeds on
//! a spawned thread, exactly as the service does.

#![cfg(windows)]

use carimbo_lib::platform::os::keymap::vk_to_char;

// VK_A is 0x41; MapVirtualKey would give its scan code, but ToUnicodeEx accepts
// a 0 scan for a plain VK translation on these layouts. We pass the real scan to
// be faithful to the hook, mapping it via the OS.
fn scan_for(vk: u16) -> u32 {
    use windows::Win32::UI::Input::KeyboardAndMouse::{MapVirtualKeyW, MAPVK_VK_TO_VSC};
    unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) }
}

#[test]
fn vk_to_char_translates_from_worker_thread() {
    const VK_A: u16 = 0x41;
    let scan = scan_for(VK_A);

    // Translate on a spawned worker thread — the exact condition that broke the
    // service. Before the fix this returned None (empty state + wrong layout on
    // non-US keyboards); after it, the foreground layout + async modifier state
    // make it produce the letter.
    let handle = std::thread::spawn(move || vk_to_char(VK_A as u32, scan));
    let got = handle.join().expect("worker thread panicked");

    // On a US or ABNT2 layout, VK_A with no Shift/Caps is 'a'. We don't assert
    // the exact char across every conceivable exotic layout, only that the
    // worker-thread translation now yields *a* character instead of None.
    assert!(
        got.is_some(),
        "vk_to_char must translate on a worker thread (thread-affinity regression); got None"
    );
}
