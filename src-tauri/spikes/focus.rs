//! Spike A — foreground capture/restore + paste (M0 risk validation).
//!
//! Manual test procedure:
//!   1. Run `cargo run --bin spike_focus`.
//!   2. Within 4 seconds, click into a target app and place the caret where text
//!      should land (Notepad, VS Code editor, Chrome address bar, Word).
//!   3. The spike captures that window, waits, then restores it to the
//!      foreground and pastes a marker string via clipboard save/paste/restore.
//!   4. Verify: the marker appears in the target app AND your original clipboard
//!      contents are intact afterward.
//!
//! Run it several times against different apps to cover the M0 "≥3 real apps"
//! acceptance. This binary is excluded from release bundles.

#[cfg(windows)]
fn main() {
    use carimbo_lib::platform::os::{clipboard, focus, inject};
    use std::thread::sleep;
    use std::time::Duration;

    const MARKER: &str = "Carimbo spike A ✓ (acentuação: ção, ável)";

    println!("Spike A: focus + paste");
    println!("You have 4 seconds — click into a target app and place the caret.");
    for i in (1..=4).rev() {
        println!("  capturing in {i}...");
        sleep(Duration::from_secs(1));
    }

    let Some(target) = focus::capture_foreground() else {
        eprintln!("No foreground window captured (desktop focused?). Aborting.");
        return;
    };
    println!("Captured foreground window.");

    // Simulate the palette being shown/dismissed: bounce focus to this console,
    // then restore, exactly as the real flow will after the user picks a snippet.
    sleep(Duration::from_millis(400));

    if !focus::restore_foreground(target) {
        eprintln!("Could not restore target to foreground — NOT pasting (safe).");
        return;
    }
    println!("Restored target to foreground; verified frontmost.");

    // Save clipboard, place marker, paste, restore — the injection strategy.
    let snapshot = clipboard::TextSnapshot::capture();
    if !clipboard::set_text(MARKER) {
        eprintln!("Failed to set clipboard. Aborting.");
        return;
    }
    // Small settle so the target is ready to receive Ctrl+V.
    sleep(Duration::from_millis(60));
    inject::send_paste();
    // Give the target time to consume the paste before we restore the clipboard.
    sleep(Duration::from_millis(180));
    snapshot.restore();

    println!("Done. Check the target app for the marker, and your clipboard is restored.");
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Spike A is Windows-only.");
}
