//! Minimal clipboard text get/set with save-and-restore, used by the paste
//! injection strategy. The full clipboard-history monitor (image support,
//! exclusion formats) lands in M3; this covers the M0/M2 paste dance.
//!
//! Only CF_UNICODETEXT is preserved by save/restore. That is sufficient for the
//! paste strategy: we briefly place our snippet text, send Ctrl+V, then put the
//! user's text back. Non-text clipboard contents (images, files) are NOT
//! preserved by this simple path — M2 will upgrade save/restore to snapshot all
//! formats. Documented here so the limitation is not a silent surprise.

#[cfg(windows)]
mod imp {
    use std::thread::sleep;
    use std::time::Duration;
    use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::Ole::CF_UNICODETEXT;

    /// The clipboard's monotonic sequence number, which increments on every
    /// change. Used for self-ignore: record it right after we write the clipboard
    /// during paste injection, then skip the matching monitor notification.
    pub fn sequence_number() -> u32 {
        use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
        unsafe { GetClipboardSequenceNumber() }
    }

    /// Open the clipboard with a few retries — it is a contended global resource
    /// and another app may hold it open momentarily. Callers must pair a `true`
    /// return with `CloseClipboard`.
    pub fn open_with_retry() -> bool {
        for attempt in 0..6 {
            // A null owner window is fine for read/write from a background thread.
            let ok = unsafe { OpenClipboard(HWND::default()) }.is_ok();
            if ok {
                return true;
            }
            sleep(Duration::from_millis(10 * (attempt + 1)));
        }
        false
    }

    /// Read a file-drop (CF_HDROP) off the clipboard as newline-separated paths,
    /// if the current clipboard holds one. Returns `None` when there's no file
    /// list. Used so copying files in Explorer lands in history as re-pastable
    /// paths.
    pub fn get_files() -> Option<Vec<String>> {
        use windows::Win32::System::Ole::CF_HDROP;
        use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

        if !open_with_retry() {
            return None;
        }
        let result = unsafe {
            let handle = GetClipboardData(CF_HDROP.0 as u32);
            match handle {
                Ok(h) if !h.is_invalid() => {
                    let hdrop = HDROP(h.0);
                    // Passing u32::MAX as the index returns the file count.
                    let count = DragQueryFileW(hdrop, u32::MAX, None);
                    let mut files = Vec::with_capacity(count as usize);
                    for i in 0..count {
                        // First call with a null buffer returns the length (chars,
                        // excluding NUL). Then read into a sized buffer.
                        let len = DragQueryFileW(hdrop, i, None);
                        if len == 0 {
                            continue;
                        }
                        let mut buf = vec![0u16; len as usize + 1];
                        let written = DragQueryFileW(hdrop, i, Some(&mut buf));
                        if written > 0 {
                            files.push(String::from_utf16_lossy(&buf[..written as usize]));
                        }
                    }
                    if files.is_empty() {
                        None
                    } else {
                        Some(files)
                    }
                }
                _ => None,
            }
        };
        unsafe {
            let _ = CloseClipboard();
        }
        result
    }

    /// Place a PNG (loaded from disk) onto the clipboard as an image, using
    /// arboard to encode the CF_DIB/CF_DIBV5 formats Windows expects. Returns
    /// false on any decode/clipboard failure so the caller can report it.
    ///
    /// We reuse arboard here — the same crate used for reading history images —
    /// rather than hand-packing a BITMAPINFOHEADER, which is fiddly and easy to
    /// get subtly wrong (stride, top-down vs bottom-up, alpha premultiply).
    pub fn set_image_from_png(path: &str) -> bool {
        let img = match image::open(path) {
            Ok(i) => i.to_rgba8(),
            Err(_) => return false,
        };
        let (width, height) = (img.width() as usize, img.height() as usize);
        let data = arboard::ImageData {
            width,
            height,
            bytes: std::borrow::Cow::Owned(img.into_raw()),
        };
        match arboard::Clipboard::new() {
            Ok(mut cb) => cb.set_image(data).is_ok(),
            Err(_) => false,
        }
    }

    /// Place both a rich HTML fragment (CF_HTML) and a plain-text fallback
    /// (CF_UNICODETEXT) on the clipboard, so rich targets (Outlook, Word, web
    /// editors) paste formatting while plain targets get `plain`. Returns false
    /// on any failure so the caller can fall back to a plain-only paste.
    ///
    /// CF_HTML is a registered format whose payload is a specific ASCII header
    /// with byte offsets pointing at the fragment, followed by the HTML. We build
    /// that here per the documented "HTML Clipboard Format" spec.
    pub fn set_html(html: &str, plain: &str) -> bool {
        use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
        use windows::core::PCWSTR;

        // Register (or look up) the "HTML Format" clipboard format id.
        let fmt_name: Vec<u16> = "HTML Format\0".encode_utf16().collect();
        let cf_html = unsafe { RegisterClipboardFormatW(PCWSTR(fmt_name.as_ptr())) };
        if cf_html == 0 {
            return false;
        }

        let payload = build_cf_html(html);
        let payload_bytes = payload.as_bytes();

        if !open_with_retry() {
            return false;
        }
        let ok = unsafe {
            if EmptyClipboard().is_err() {
                let _ = CloseClipboard();
                return false;
            }
            // 1) CF_HTML (bytes, NUL-terminated — it's an ASCII/UTF-8 byte blob).
            let html_ok = match alloc_bytes_global(payload_bytes) {
                Some(hglobal) => {
                    let handle = HANDLE(hglobal.0);
                    if SetClipboardData(cf_html, handle).is_ok() {
                        true
                    } else {
                        let _ = GlobalFree(hglobal);
                        false
                    }
                }
                None => false,
            };
            // 2) CF_UNICODETEXT plain fallback, so non-rich targets still paste.
            if let Some(hglobal) = alloc_utf16_global(plain) {
                let handle = HANDLE(hglobal.0);
                if SetClipboardData(CF_UNICODETEXT.0 as u32, handle).is_err() {
                    let _ = GlobalFree(hglobal);
                }
            }
            html_ok
        };
        unsafe {
            let _ = CloseClipboard();
        }
        ok
    }

    /// Read the current clipboard text (CF_UNICODETEXT), if any.
    pub fn get_text() -> Option<String> {
        if !open_with_retry() {
            return None;
        }
        let result = unsafe {
            let handle = GetClipboardData(CF_UNICODETEXT.0 as u32);
            match handle {
                Ok(h) if !h.is_invalid() => read_utf16_global(h),
                _ => None,
            }
        };
        unsafe {
            let _ = CloseClipboard();
        }
        result
    }

    /// Replace the clipboard with `text` (CF_UNICODETEXT). Returns false on
    /// failure so callers can decide whether to proceed.
    pub fn set_text(text: &str) -> bool {
        if !open_with_retry() {
            return false;
        }
        let ok = unsafe {
            if EmptyClipboard().is_err() {
                let _ = CloseClipboard();
                return false;
            }
            match alloc_utf16_global(text) {
                Some(hglobal) => {
                    // Ownership of hglobal transfers to the system on success.
                    let handle = HANDLE(hglobal.0);
                    if SetClipboardData(CF_UNICODETEXT.0 as u32, handle).is_ok() {
                        true
                    } else {
                        // On failure we still own the memory; free it.
                        let _ = GlobalFree(hglobal);
                        false
                    }
                }
                None => false,
            }
        };
        unsafe {
            let _ = CloseClipboard();
        }
        ok
    }

    unsafe fn read_utf16_global(handle: HANDLE) -> Option<String> {
        let hglobal = HGLOBAL(handle.0);
        let ptr = GlobalLock(hglobal) as *const u16;
        if ptr.is_null() {
            return None;
        }
        // Find the NUL terminator.
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        let s = String::from_utf16_lossy(slice);
        let _ = GlobalUnlock(hglobal);
        Some(s)
    }

    /// Build a CF_HTML payload wrapping `fragment` (an HTML fragment). The format
    /// is a header of `Key:Value` lines with byte offsets, then the HTML with
    /// `<!--StartFragment-->`/`<!--EndFragment-->` markers. Offsets are byte
    /// counts from the start of the payload; we compute them in two passes since
    /// the header length depends on the (zero-padded, fixed-width) numbers.
    fn build_cf_html(fragment: &str) -> String {
        // Wrap the fragment in a minimal, valid document with the required
        // fragment markers.
        const PREFIX: &str = "<html><body><!--StartFragment-->";
        const SUFFIX: &str = "<!--EndFragment--></body></html>";

        // Header template with 10-digit, zero-padded offsets (fixed width so the
        // header's own byte length is constant regardless of the numbers).
        let header_tpl = |start_html: usize,
                          end_html: usize,
                          start_frag: usize,
                          end_frag: usize|
         -> String {
            format!(
                "Version:0.9\r\n\
                 StartHTML:{start_html:010}\r\n\
                 EndHTML:{end_html:010}\r\n\
                 StartFragment:{start_frag:010}\r\n\
                 EndFragment:{end_frag:010}\r\n"
            )
        };

        // First pass with placeholder zeros to measure the header length.
        let header_len = header_tpl(0, 0, 0, 0).len();
        let body = format!("{PREFIX}{fragment}{SUFFIX}");

        // Byte offsets (the whole payload is ASCII header + UTF-8 body; offsets
        // are byte counts). StartHTML points just past the header.
        let start_html = header_len;
        let start_fragment = start_html + PREFIX.len();
        let end_fragment = start_fragment + fragment.len();
        let end_html = start_html + body.len();

        let header = header_tpl(start_html, end_html, start_fragment, end_fragment);
        format!("{header}{body}")
    }

    /// Allocate a moveable global holding `bytes` plus a trailing NUL, for
    /// byte-oriented clipboard formats like CF_HTML.
    unsafe fn alloc_bytes_global(bytes: &[u8]) -> Option<HGLOBAL> {
        let total = bytes.len() + 1; // trailing NUL
        let hglobal = GlobalAlloc(GMEM_MOVEABLE, total).ok()?;
        let dst = GlobalLock(hglobal) as *mut u8;
        if dst.is_null() {
            let _ = GlobalFree(hglobal);
            return None;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        *dst.add(bytes.len()) = 0;
        let _ = GlobalUnlock(hglobal);
        Some(hglobal)
    }

    unsafe fn alloc_utf16_global(text: &str) -> Option<HGLOBAL> {
        let mut utf16: Vec<u16> = text.encode_utf16().collect();
        utf16.push(0); // NUL terminator
        let bytes = utf16.len() * std::mem::size_of::<u16>();
        let hglobal = GlobalAlloc(GMEM_MOVEABLE, bytes).ok()?;
        let dst = GlobalLock(hglobal) as *mut u16;
        if dst.is_null() {
            let _ = GlobalFree(hglobal);
            return None;
        }
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), dst, utf16.len());
        let _ = GlobalUnlock(hglobal);
        Some(hglobal)
    }

    #[cfg(test)]
    mod tests {
        use super::build_cf_html;

        #[test]
        fn cf_html_offsets_point_at_the_fragment() {
            let payload = build_cf_html("<b>Hi</b>");
            let bytes = payload.as_bytes();

            // Pull an integer value out of a "Key:0000000123\r\n" header line.
            let field = |key: &str| -> usize {
                let line = payload
                    .lines()
                    .find(|l| l.starts_with(key))
                    .unwrap_or_else(|| panic!("missing {key}"));
                line[key.len() + 1..].trim().parse().unwrap()
            };
            let start_frag = field("StartFragment");
            let end_frag = field("EndFragment");
            let start_html = field("StartHTML");
            let end_html = field("EndHTML");

            // The bytes between the fragment offsets are exactly our fragment.
            assert_eq!(&bytes[start_frag..end_frag], b"<b>Hi</b>");
            // StartHTML sits right after the header and EndHTML at the very end.
            assert!(payload[start_html..].starts_with("<html>"));
            assert_eq!(end_html, bytes.len());
        }
    }

    /// A saved snapshot of the clipboard text, restored on drop or explicitly.
    pub struct TextSnapshot {
        text: Option<String>,
    }

    impl TextSnapshot {
        /// Capture the current clipboard text.
        pub fn capture() -> Self {
            TextSnapshot { text: get_text() }
        }

        /// Put the captured text back on the clipboard. No-op if there was none.
        pub fn restore(&self) {
            if let Some(t) = &self.text {
                set_text(t);
            }
        }
    }
}

#[cfg(windows)]
pub use imp::*;
