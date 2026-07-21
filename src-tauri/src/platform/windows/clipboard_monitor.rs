//! Clipboard change monitor.
//!
//! Uses `AddClipboardFormatListener` on a hidden message-only window, so we get
//! a `WM_CLIPBOARDUPDATE` message on every clipboard change with zero polling
//! (no idle CPU). Runs on its own thread with a message pump.
//!
//! Privacy: before capturing, we honour the standard "don't record me" clipboard
//! formats that password managers set (KeePass, 1Password, etc.). We also skip
//! updates we caused ourselves (paste injection) via the clipboard sequence
//! number recorded at write time.

#[cfg(windows)]
mod imp {
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::sync::Mutex;

    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::DataExchange::{
        AddClipboardFormatListener, RegisterClipboardFormatW, RemoveClipboardFormatListener,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        PostThreadMessageW, RegisterClassW, TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE,
        WINDOW_STYLE, WM_CLIPBOARDUPDATE, WM_QUIT, WNDCLASSW,
    };

    /// A captured clipboard change. Content reading is done by the caller (via
    /// `arboard`/our clipboard module) so this stays a thin notifier.
    #[derive(Debug, Clone, Copy)]
    pub struct ClipboardUpdate;

    static SENDER: Mutex<Option<Sender<ClipboardUpdate>>> = Mutex::new(None);

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_CLIPBOARDUPDATE {
            if let Ok(guard) = SENDER.try_lock() {
                if let Some(tx) = guard.as_ref() {
                    let _ = tx.send(ClipboardUpdate);
                }
            }
            return LRESULT(0);
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    /// True if the clipboard currently carries a "do not record" marker set by
    /// password managers and similar privacy-sensitive apps.
    pub fn should_skip_current_clipboard() -> bool {
        use windows::Win32::System::DataExchange::IsClipboardFormatAvailable;
        // Presence alone means "do not record" for these two markers.
        // Registering returns the same atom the setter used.
        let markers: [PCWSTR; 2] = [
            w!("ExcludeClipboardContentFromMonitorProcessing"),
            w!("Clipboard Viewer Ignore"),
        ];
        unsafe {
            for name in markers {
                let fmt = RegisterClipboardFormatW(name);
                if fmt != 0 && IsClipboardFormatAvailable(fmt).is_ok() {
                    return true;
                }
            }
            // `CanIncludeInClipboardHistory` is a DWORD flag, not a presence
            // marker: 0 = exclude, non-zero = include. The Snipping Tool (and
            // PrintScreen, which routes through it) sets it to 1 on every
            // screenshot, so skipping on mere presence drops all screenshots.
            let fmt = RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory"));
            if fmt != 0 && IsClipboardFormatAvailable(fmt).is_ok() {
                // If the value can't be read (clipboard contended), err on the
                // privacy side and skip.
                return read_clipboard_dword(fmt).map_or(true, |v| v == 0);
            }
        }
        false
    }

    /// Read a DWORD-valued clipboard format (e.g. `CanIncludeInClipboardHistory`).
    /// Returns `None` if the clipboard can't be opened or the payload is missing
    /// or shorter than 4 bytes.
    fn read_clipboard_dword(fmt: u32) -> Option<u32> {
        use windows::Win32::Foundation::HGLOBAL;
        use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData};
        use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

        if !crate::platform::os::clipboard::open_with_retry() {
            return None;
        }
        let value = unsafe {
            match GetClipboardData(fmt) {
                Ok(h) if !h.is_invalid() => {
                    let hglobal = HGLOBAL(h.0);
                    if GlobalSize(hglobal) < 4 {
                        None
                    } else {
                        let ptr = GlobalLock(hglobal) as *const u32;
                        if ptr.is_null() {
                            None
                        } else {
                            let v = ptr.read_unaligned();
                            let _ = GlobalUnlock(hglobal);
                            Some(v)
                        }
                    }
                }
                _ => None,
            }
        };
        unsafe {
            let _ = CloseClipboard();
        }
        value
    }

    /// Owns the monitor thread + hidden window. Dropping it stops monitoring.
    pub struct ClipboardMonitor {
        thread: Option<std::thread::JoinHandle<()>>,
        thread_id: u32,
    }

    impl ClipboardMonitor {
        /// Start monitoring. Returns a receiver that yields one `ClipboardUpdate`
        /// per clipboard change. Only one monitor may run at a time.
        pub fn start() -> Result<(ClipboardMonitor, Receiver<ClipboardUpdate>), String> {
            let (tx, rx) = channel();
            {
                let mut guard = SENDER
                    .lock()
                    .map_err(|_| "clipboard monitor state poisoned".to_string())?;
                if guard.is_some() {
                    return Err("clipboard monitor already running".into());
                }
                *guard = Some(tx);
            }

            let (id_tx, id_rx) = channel::<u32>();
            let thread = std::thread::Builder::new()
                .name("carimbo-clip-monitor".into())
                .spawn(move || unsafe {
                    let hinstance = match GetModuleHandleW(None) {
                        Ok(h) => h,
                        Err(e) => {
                            tracing::error!("GetModuleHandleW failed: {e}");
                            let _ = id_tx.send(0);
                            return;
                        }
                    };
                    let hinst = windows::Win32::Foundation::HINSTANCE::from(hinstance);
                    let class_name = w!("CarimboClipboardMonitor");
                    let wc = WNDCLASSW {
                        lpfnWndProc: Some(wnd_proc),
                        hInstance: hinst,
                        lpszClassName: class_name,
                        ..Default::default()
                    };
                    // RegisterClassW returns 0 on failure; ignore "already
                    // registered" by proceeding to CreateWindow regardless.
                    RegisterClassW(&wc);

                    let hwnd = CreateWindowExW(
                        WINDOW_EX_STYLE(0),
                        class_name,
                        w!("Carimbo Clipboard Monitor"),
                        WINDOW_STYLE(0),
                        0,
                        0,
                        0,
                        0,
                        HWND_MESSAGE, // message-only window
                        windows::Win32::UI::WindowsAndMessaging::HMENU::default(),
                        hinst,
                        None,
                    );
                    let hwnd = match hwnd {
                        Ok(h) if !h.0.is_null() => h,
                        _ => {
                            tracing::error!("CreateWindowExW (message-only) failed");
                            let _ = id_tx.send(0);
                            return;
                        }
                    };

                    if AddClipboardFormatListener(hwnd).is_err() {
                        tracing::error!("AddClipboardFormatListener failed");
                        let _ = DestroyWindow(hwnd);
                        let _ = id_tx.send(0);
                        return;
                    }

                    let _ = id_tx.send(
                        windows::Win32::System::Threading::GetCurrentThreadId(),
                    );

                    // Pump messages until WM_QUIT. DispatchMessageW is what
                    // actually invokes wnd_proc for WM_CLIPBOARDUPDATE — without
                    // it the window procedure never runs and no change fires.
                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }

                    let _ = RemoveClipboardFormatListener(hwnd);
                    let _ = DestroyWindow(hwnd);
                })
                .map_err(|e| format!("failed to spawn clipboard thread: {e}"))?;

            let thread_id = id_rx.recv().unwrap_or(0);
            if thread_id == 0 {
                clear_sender();
                let _ = thread.join();
                return Err("clipboard monitor failed to initialize".into());
            }

            Ok((
                ClipboardMonitor {
                    thread: Some(thread),
                    thread_id,
                },
                rx,
            ))
        }
    }

    fn clear_sender() {
        if let Ok(mut g) = SENDER.lock() {
            *g = None;
        }
    }

    impl Drop for ClipboardMonitor {
        fn drop(&mut self) {
            if self.thread_id != 0 {
                unsafe {
                    let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
                }
            }
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
            clear_sender();
        }
    }
}

#[cfg(windows)]
pub use imp::*;
