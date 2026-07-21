//! High-level paste orchestration used by the palette (M2) and expansion engine
//! (M4). Combines clipboard save/restore, focus restore, and injection into one
//! safe operation with a clear success/failure result.

#[cfg(windows)]
mod imp {
    use std::collections::HashSet;
    use std::sync::Mutex;
    use std::thread::sleep;
    use std::time::Duration;

    use super::super::clipboard::{self, sequence_number, TextSnapshot};
    use super::super::focus::{restore_foreground, CapturedWindow};
    use super::super::inject;
    use super::super::util::window_is_elevated_relative_to_us;

    // Clipboard sequence numbers produced by our own writes (setting the snippet
    // text and restoring the user's clipboard). The clipboard monitor consults
    // this set to ignore updates we caused, so paste injection never pollutes the
    // clipboard history.
    static SELF_WRITE_SEQS: Mutex<Option<HashSet<u32>>> = Mutex::new(None);

    fn record_self_write() {
        let seq = sequence_number();
        if let Ok(mut guard) = SELF_WRITE_SEQS.lock() {
            let set = guard.get_or_insert_with(HashSet::new);
            set.insert(seq);
            // Bound the set so it can't grow forever.
            if set.len() > 64 {
                set.clear();
                set.insert(seq);
            }
        }
    }

    /// True if `seq` matches a clipboard write Carimbo made recently. Consumes
    /// the entry so it only suppresses one notification.
    pub fn was_recent_self_write(seq: u32) -> bool {
        if let Ok(mut guard) = SELF_WRITE_SEQS.lock() {
            if let Some(set) = guard.as_mut() {
                return set.remove(&seq);
            }
        }
        false
    }

    /// Which strategy to use to place text into the target app.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum InjectMethod {
        /// Save clipboard, set text, Ctrl+V, restore clipboard. Fast, works
        /// almost everywhere, handles long/multiline text.
        Paste,
        /// Type the text as unicode key events. Slower but never touches the
        /// clipboard — for paste-blocking apps/terminals.
        Type,
    }

    /// Outcome of an insertion attempt, so callers can surface the right message.
    #[derive(Debug, PartialEq, Eq)]
    pub enum InsertResult {
        Ok,
        /// The target could not be brought back to the foreground; we did NOT
        /// paste (never paste into the wrong window).
        FocusLost,
        /// The target is an elevated window we (unelevated) cannot inject into.
        Elevated,
        /// Setting the clipboard failed (only for the Paste method).
        ClipboardFailed,
    }

    /// Restore `target` to the foreground and insert `text` into it.
    ///
    /// `delete_before` optionally sends that many backspaces first — used by the
    /// expansion engine to remove the typed trigger. The palette passes 0.
    pub fn insert_into(
        target: CapturedWindow,
        text: &str,
        method: InjectMethod,
        delete_before: usize,
    ) -> InsertResult {
        insert_into_with_cursor(target, text, method, delete_before, None)
    }

    /// Like [`insert_into`], but after inserting, move the caret back by
    /// `cursor_from_end` characters (Left-arrow presses) so it lands where a
    /// `{cursor}` token asked. `None` leaves the caret at the end (the default).
    pub fn insert_into_with_cursor(
        target: CapturedWindow,
        text: &str,
        method: InjectMethod,
        delete_before: usize,
        cursor_from_end: Option<usize>,
    ) -> InsertResult {
        // Refuse elevated targets up front — injection would silently no-op.
        if window_is_elevated_relative_to_us(target.0) {
            return InsertResult::Elevated;
        }

        if !restore_foreground(target) {
            return InsertResult::FocusLost;
        }

        // Small settle so the target's focus/caret is ready.
        sleep(Duration::from_millis(40));

        if delete_before > 0 {
            inject::send_backspaces(delete_before);
            sleep(Duration::from_millis(15));
        }

        let result = match method {
            InjectMethod::Type => {
                inject::type_unicode(text);
                InsertResult::Ok
            }
            InjectMethod::Paste => {
                let snapshot = TextSnapshot::capture();
                if !clipboard::set_text(text) {
                    return InsertResult::ClipboardFailed;
                }
                // Record so the clipboard monitor ignores this self-write.
                record_self_write();
                // Let the clipboard settle before Ctrl+V.
                sleep(Duration::from_millis(30));
                inject::send_paste();
                // Give the target time to consume the paste before we restore
                // the user's previous clipboard contents.
                sleep(Duration::from_millis(160));
                snapshot.restore();
                record_self_write();
                InsertResult::Ok
            }
        };

        // Move the caret to the {cursor} marker, if any. Only meaningful when the
        // insertion actually landed.
        if result == InsertResult::Ok {
            if let Some(back) = cursor_from_end {
                if back > 0 {
                    // The paste needs to be consumed before arrows count; the
                    // 160ms above already covers Paste, but Type is synchronous.
                    inject::send_left_arrows(back);
                }
            }
        }

        result
    }

    /// Restore `target` to the foreground and paste rich HTML into it, with
    /// `plain` as the text fallback for non-rich targets. `delete_before` sends
    /// that many backspaces first (trigger removal); the palette passes 0.
    /// Always uses the paste strategy — HTML can't be typed out.
    pub fn insert_html_into(
        target: CapturedWindow,
        html: &str,
        plain: &str,
        delete_before: usize,
    ) -> InsertResult {
        if window_is_elevated_relative_to_us(target.0) {
            return InsertResult::Elevated;
        }
        if !restore_foreground(target) {
            return InsertResult::FocusLost;
        }
        sleep(Duration::from_millis(40));

        if delete_before > 0 {
            inject::send_backspaces(delete_before);
            sleep(Duration::from_millis(15));
        }

        let snapshot = TextSnapshot::capture();
        if !clipboard::set_html(html, plain) {
            // Fall back to a plain-text paste if we couldn't build the HTML
            // clipboard payload — better to insert the words than nothing.
            if !clipboard::set_text(plain) {
                return InsertResult::ClipboardFailed;
            }
        }
        record_self_write();
        sleep(Duration::from_millis(30));
        inject::send_paste();
        sleep(Duration::from_millis(180));
        snapshot.restore();
        record_self_write();
        InsertResult::Ok
    }

    /// Restore `target` to the foreground and paste the image at `png_path` into
    /// it. Mirrors `insert_into`'s Paste branch but places a CF_DIB image on the
    /// clipboard instead of text.
    ///
    /// The previous clipboard *text* is snapshotted and restored afterward (the
    /// common case). We don't attempt to snapshot/restore an arbitrary prior
    /// image — that's a rare "paste an image while an image was already on the
    /// clipboard" edge, and the freshly-pasted image simply lingers on the
    /// clipboard, which is benign.
    pub fn insert_image_into(target: CapturedWindow, png_path: &str) -> InsertResult {
        if window_is_elevated_relative_to_us(target.0) {
            return InsertResult::Elevated;
        }
        if !restore_foreground(target) {
            return InsertResult::FocusLost;
        }
        // Small settle so the target's focus/caret is ready.
        sleep(Duration::from_millis(40));

        let snapshot = TextSnapshot::capture();
        if !clipboard::set_image_from_png(png_path) {
            return InsertResult::ClipboardFailed;
        }
        record_self_write();
        sleep(Duration::from_millis(30));
        inject::send_paste();
        // Images can take longer to consume than text; give the target room.
        sleep(Duration::from_millis(200));
        snapshot.restore();
        record_self_write();
        InsertResult::Ok
    }
}

#[cfg(windows)]
pub use imp::*;
