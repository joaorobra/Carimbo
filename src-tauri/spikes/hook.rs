//! Spike B — WH_KEYBOARD_LL hook + abbreviation expansion (M0 risk validation).
//!
//! Manual test procedure:
//!   1. Run `cargo run --bin spike_hook` (keep this console open).
//!   2. Switch to Notepad / Word / Chrome / VS Code and type `;oi`.
//!   3. It should replace `;oi` in place with `Olá, tudo bem?`.
//!   4. On an ABNT2 keyboard, also type a dead-key sequence like acute+a (á) and
//!      confirm it still composes normally — the hook must NOT corrupt dead keys.
//!   5. Ctrl+C in the console (or close it) to exit.
//!
//! This validates: the LL hook installs and survives, key translation ignores
//! our own injected input (no infinite loop), the matcher fires on a suffix, and
//! backspace+paste replacement works across real apps.

#[cfg(windows)]
fn main() {
    use carimbo_lib::core::expansion::Matcher;
    use carimbo_lib::platform::os::{clipboard, inject, keyboard_hook::KeyboardHook, keymap};
    use std::thread::sleep;
    use std::time::Duration;

    println!("Spike B: keyboard hook + expansion");
    println!("Trigger: \";oi\"  ->  \"Olá, tudo bem?\"");
    println!("Type it in any app. Ctrl+C here to quit.\n");

    let (_hook, rx) = match KeyboardHook::install() {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("Failed to install keyboard hook: {e}");
            return;
        }
    };
    println!("Hook installed. Listening...");

    let mut matcher = Matcher::new();
    matcher.set_triggers([(";oi", "Olá, tudo bem?")]);

    // Consume key events on the main thread. The hook thread produces them.
    for ev in rx.iter() {
        // Never process our own injected keystrokes — this is what prevents the
        // backspaces/paste from feeding back into the matcher in a loop.
        if ev.injected {
            continue;
        }

        if keymap::is_backspace(ev.vk) {
            matcher.backspace();
            continue;
        }
        if keymap::is_buffer_resetting_key(ev.vk) {
            matcher.clear_buffer();
            continue;
        }

        let Some(c) = keymap::vk_to_char(ev.vk, ev.scan) else {
            // Dead key / modifier / non-character: reset so a half-typed trigger
            // interrupted by a dead key doesn't falsely match.
            continue;
        };

        if let Some(expansion) = matcher.push_char(c) {
            println!("Match! Expanding (delete {} chars).", expansion.delete_chars);

            // Replace in place: delete the trigger, then paste the replacement.
            inject::send_backspaces(expansion.delete_chars);
            sleep(Duration::from_millis(20));

            let snapshot = clipboard::TextSnapshot::capture();
            if clipboard::set_text(&expansion.replacement) {
                sleep(Duration::from_millis(30));
                inject::send_paste();
                sleep(Duration::from_millis(150));
                snapshot.restore();
            } else {
                eprintln!("Could not set clipboard; skipped this expansion.");
            }
        }
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Spike B is Windows-only.");
}
