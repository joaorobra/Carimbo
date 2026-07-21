//! Screen color sampling + click capture for the color picker.
//!
//! Two cooperating threads, mirroring `keyboard_hook.rs`'s constraints:
//! * A hook thread owns a WH_MOUSE_LL hook and a blocking `GetMessageW` pump.
//!   A blocked `GetMessageW` still services incoming hook calls immediately, so
//!   the callback adds no system-wide cursor latency. The callback only reads
//!   and writes atomics — no locks, no allocation — and swallows the confirming
//!   left click (and a cancelling right click) so it never reaches the app
//!   underneath the cursor.
//! * A worker thread polls the cursor ~30x/s, samples a small pixel grid around
//!   it via one BitBlt, and streams events over a channel. Esc is polled with
//!   `GetAsyncKeyState` (globally visible, no focus needed). Note Esc is NOT
//!   swallowed — we install no keyboard hook — so the focused app also sees it;
//!   right-click is the side-effect-free cancel.

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::time::{Duration, Instant};

    use windows::Win32::Foundation::{HMODULE, LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT,
        DIB_RGB_COLORS, ROP_CODE, SRCCOPY,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetCursorPos, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
        MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN,
        WM_RBUTTONUP,
    };

    /// A square of screen pixels centered on the cursor. `size` is odd; the
    /// center entry is the color under the cursor. `rgb` holds row-major
    /// R,G,B triples (`size * size * 3` bytes).
    pub struct GridSample {
        pub x: i32,
        pub y: i32,
        pub size: i32,
        pub rgb: Vec<u8>,
    }

    impl GridSample {
        pub fn center(&self) -> (u8, u8, u8) {
            let i = ((self.size * self.size) / 2 * 3) as usize;
            (self.rgb[i], self.rgb[i + 1], self.rgb[i + 2])
        }
    }

    pub enum PickerEvent {
        /// Cursor moved (or first tick): live preview payload.
        Move(GridSample),
        /// Left click: final sample, taken at the click coordinates.
        Picked(GridSample),
        /// Esc or right click.
        Cancelled,
    }

    // One picking session process-wide. The hook callback carries no user
    // pointer, so session flags are atomics the callback and worker share.
    static SESSION: AtomicBool = AtomicBool::new(false);
    /// Gates swallowing: the hook only eats clicks while a session is live.
    static HOOK_ACTIVE: AtomicBool = AtomicBool::new(false);
    static PICKED: AtomicBool = AtomicBool::new(false);
    static CANCELLED: AtomicBool = AtomicBool::new(false);
    static UP_SEEN: AtomicBool = AtomicBool::new(false);
    static CLICK_X: AtomicI32 = AtomicI32::new(0);
    static CLICK_Y: AtomicI32 = AtomicI32::new(0);

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 && HOOK_ACTIVE.load(Ordering::Relaxed) {
            let msg = wparam.0 as u32;
            match msg {
                WM_LBUTTONDOWN => {
                    let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
                    CLICK_X.store(ms.pt.x, Ordering::Relaxed);
                    CLICK_Y.store(ms.pt.y, Ordering::Relaxed);
                    PICKED.store(true, Ordering::Release);
                    return LRESULT(1); // swallow: this click picks, it must not land below
                }
                WM_LBUTTONUP if PICKED.load(Ordering::Relaxed) => {
                    UP_SEEN.store(true, Ordering::Release);
                    return LRESULT(1);
                }
                WM_RBUTTONDOWN => {
                    CANCELLED.store(true, Ordering::Release);
                    return LRESULT(1);
                }
                WM_RBUTTONUP if CANCELLED.load(Ordering::Relaxed) => {
                    return LRESULT(1);
                }
                _ => {}
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    /// Owns the mouse-hook thread. Dropping it unhooks and stops the pump.
    struct MouseHook {
        thread: Option<std::thread::JoinHandle<()>>,
        thread_id: u32,
    }

    impl MouseHook {
        fn install() -> Result<MouseHook, String> {
            let (id_tx, id_rx) = channel::<u32>();
            let thread = std::thread::Builder::new()
                .name("carimbo-mouse-hook".into())
                .spawn(move || {
                    let hook = unsafe {
                        SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), HMODULE::default(), 0)
                    };
                    let hook: HHOOK = match hook {
                        Ok(h) => h,
                        Err(e) => {
                            tracing::error!("SetWindowsHookExW(WH_MOUSE_LL) failed: {e}");
                            let _ = id_tx.send(0);
                            return;
                        }
                    };
                    let _ = id_tx.send(unsafe {
                        windows::Win32::System::Threading::GetCurrentThreadId()
                    });
                    let mut msg = MSG::default();
                    unsafe {
                        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {}
                        let _ = UnhookWindowsHookEx(hook);
                    }
                })
                .map_err(|e| format!("failed to spawn mouse hook thread: {e}"))?;

            let thread_id = id_rx.recv().unwrap_or(0);
            if thread_id == 0 {
                let _ = thread.join();
                return Err("mouse hook failed to install".into());
            }
            Ok(MouseHook {
                thread: Some(thread),
                thread_id,
            })
        }
    }

    impl Drop for MouseHook {
        fn drop(&mut self) {
            if self.thread_id != 0 {
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                        self.thread_id,
                        windows::Win32::UI::WindowsAndMessaging::WM_QUIT,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
            }
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
        }
    }

    /// Start a picking session. Streams [`PickerEvent`]s until a terminal
    /// `Picked`/`Cancelled` (always the last event before the channel closes).
    /// Errors if a session is already running or the hook can't be installed.
    pub fn start(grid_half: i32, poll_ms: u64) -> Result<Receiver<PickerEvent>, String> {
        if SESSION.swap(true, Ordering::SeqCst) {
            return Err("color picker already active".into());
        }
        PICKED.store(false, Ordering::SeqCst);
        CANCELLED.store(false, Ordering::SeqCst);
        UP_SEEN.store(false, Ordering::SeqCst);

        let hook = match MouseHook::install() {
            Ok(h) => h,
            Err(e) => {
                SESSION.store(false, Ordering::SeqCst);
                return Err(e);
            }
        };
        HOOK_ACTIVE.store(true, Ordering::SeqCst);

        let (tx, rx) = channel::<PickerEvent>();
        std::thread::Builder::new()
            .name("carimbo-color-pick".into())
            .spawn(move || {
                run_loop(&tx, grid_half, poll_ms);
                // Stop swallowing before the (blocking) unhook, then release the
                // session so a new pick can start.
                HOOK_ACTIVE.store(false, Ordering::SeqCst);
                drop(hook);
                SESSION.store(false, Ordering::SeqCst);
            })
            .map_err(|e| {
                HOOK_ACTIVE.store(false, Ordering::SeqCst);
                SESSION.store(false, Ordering::SeqCst);
                format!("failed to spawn color pick thread: {e}")
            })?;

        Ok(rx)
    }

    fn run_loop(tx: &Sender<PickerEvent>, grid_half: i32, poll_ms: u64) {
        loop {
            if CANCELLED.load(Ordering::Acquire) || esc_down() {
                let _ = tx.send(PickerEvent::Cancelled);
                return;
            }
            if PICKED.load(Ordering::Acquire) {
                // Give the matching button-up a beat to arrive so the hook
                // swallows it too; a lone orphaned up-event is harmless anyway.
                let deadline = Instant::now() + Duration::from_millis(300);
                while !UP_SEEN.load(Ordering::Acquire) && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(5));
                }
                let x = CLICK_X.load(Ordering::Relaxed);
                let y = CLICK_Y.load(Ordering::Relaxed);
                match sample_grid(x, y, grid_half) {
                    Some(s) => {
                        let _ = tx.send(PickerEvent::Picked(s));
                    }
                    None => {
                        let _ = tx.send(PickerEvent::Cancelled);
                    }
                }
                return;
            }

            let mut pt = POINT::default();
            if unsafe { GetCursorPos(&mut pt) }.is_ok() {
                if let Some(s) = sample_grid(pt.x, pt.y, grid_half) {
                    // Receiver gone (app shutting down) — stop quietly.
                    if tx.send(PickerEvent::Move(s)).is_err() {
                        return;
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(poll_ms));
        }
    }

    fn esc_down() -> bool {
        (unsafe { GetAsyncKeyState(VK_ESCAPE.0 as i32) } as u16 & 0x8000) != 0
    }

    /// Sample a `(2*half+1)`-sided square of screen pixels centered on virtual-
    /// screen coordinates `(cx, cy)` with a single BitBlt (GetPixel per cell
    /// would stall on GPU sync). CAPTUREBLT includes layered windows — the
    /// overlay avoids being captured by staying offset from the cursor. Pixels
    /// past the screen edge come back black, which the preview just shows.
    pub fn sample_grid(cx: i32, cy: i32, half: i32) -> Option<GridSample> {
        let size = half * 2 + 1;
        unsafe {
            let screen = GetDC(None);
            if screen.is_invalid() {
                return None;
            }
            let mem = CreateCompatibleDC(screen);
            let bmp = CreateCompatibleBitmap(screen, size, size);
            let old = SelectObject(mem, bmp);
            let blit = BitBlt(
                mem,
                0,
                0,
                size,
                size,
                screen,
                cx - half,
                cy - half,
                ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0),
            );
            // The bitmap must be deselected before GetDIBits may read it.
            SelectObject(mem, old);

            let mut rgb = None;
            if blit.is_ok() {
                let mut bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: size,
                        biHeight: -size, // top-down
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let mut bgra = vec![0u8; (size * size * 4) as usize];
                let lines = GetDIBits(
                    mem,
                    bmp,
                    0,
                    size as u32,
                    Some(bgra.as_mut_ptr() as *mut _),
                    &mut bmi,
                    DIB_RGB_COLORS,
                );
                if lines == size {
                    rgb = Some(
                        bgra.chunks_exact(4)
                            .flat_map(|px| [px[2], px[1], px[0]])
                            .collect::<Vec<u8>>(),
                    );
                }
            }

            let _ = DeleteObject(bmp);
            let _ = DeleteDC(mem);
            ReleaseDC(None, screen);

            rgb.map(|rgb| GridSample {
                x: cx,
                y: cy,
                size,
                rgb,
            })
        }
    }
}

#[cfg(windows)]
pub use imp::*;
