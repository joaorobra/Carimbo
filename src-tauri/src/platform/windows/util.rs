//! Small shared Win32 helpers used by the clipboard monitor, keyboard hook,
//! injector and focus tracker.

/// Tag placed in the `dwExtraInfo` field of every synthetic input event Carimbo
/// sends. The low-level keyboard hook checks this so we never re-process our own
/// injected keystrokes (backspaces, Ctrl+V, unicode typing).
///
/// Value is an arbitrary constant unlikely to collide with other software.
pub const CARIMBO_INJECT_TAG: usize = 0xCA_A1_B0_00;

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
    use windows::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    /// Returns the process id owning `hwnd`, or `None` if the handle is invalid.
    pub fn process_id_of_window(hwnd: HWND) -> Option<u32> {
        let mut pid: u32 = 0;
        // Returns the thread id; 0 means failure.
        let tid = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if tid == 0 || pid == 0 {
            None
        } else {
            Some(pid)
        }
    }

    /// Best-effort base name of the current foreground window's process
    /// (e.g. `chrome.exe`). Used to label where a clip was copied from. `None`
    /// when there's no foreground window or the process can't be queried
    /// (elevated apps deny us, which is fine — the label is purely informational).
    pub fn foreground_process_name() -> Option<String> {
        use windows::Win32::System::Threading::QueryFullProcessImageNameW;
        use windows::Win32::System::Threading::PROCESS_NAME_FORMAT;
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            return None;
        }
        let pid = process_id_of_window(hwnd)?;
        let handle =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
        if handle.is_invalid() {
            return None;
        }
        let mut buf = [0u16; 260];
        let mut len = buf.len() as u32;
        let ok = unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_FORMAT(0),
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
        }
        .is_ok();
        unsafe {
            let _ = CloseHandle(handle);
        }
        if !ok || len == 0 {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        // Keep just the executable base name, not the whole path.
        full.rsplit(['\\', '/']).next().map(|s| s.to_string())
    }

    /// Best-effort check of whether the process behind `hwnd` runs at a higher
    /// integrity level than us, i.e. we (unelevated) cannot inject input into it
    /// due to UIPI. Errs on the side of `false` (allow) only when we genuinely
    /// cannot tell — callers should still verify the paste landed.
    pub fn window_is_elevated_relative_to_us(hwnd: HWND) -> bool {
        let Some(pid) = process_id_of_window(hwnd) else {
            return false;
        };
        let our = process_integrity_level(unsafe { GetCurrentProcess() });
        let theirs = match unsafe {
            OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
        } {
            Ok(h) if !h.is_invalid() => {
                let lvl = process_integrity_level(h);
                unsafe {
                    let _ = CloseHandle(h);
                }
                lvl
            }
            // Access denied here is itself a strong signal the target is higher
            // integrity than us.
            _ => return true,
        };
        match (our, theirs) {
            (Some(a), Some(b)) => b > a,
            _ => false,
        }
    }

    /// Reads a process's integrity level RID (e.g. medium = 0x2000,
    /// high = 0x3000). `None` on failure.
    fn process_integrity_level(process: HANDLE) -> Option<u32> {
        use windows::Win32::Security::{
            GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
            SID_AND_ATTRIBUTES, TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
        };
        use windows::Win32::Foundation::HANDLE as WHANDLE;

        unsafe {
            let mut token = WHANDLE::default();
            if OpenProcessToken(process, TOKEN_QUERY, &mut token).is_err() {
                return None;
            }
            let mut needed: u32 = 0;
            // First call to size the buffer.
            let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut needed);
            if needed == 0 {
                let _ = CloseHandle(token);
                return None;
            }
            let mut buf = vec![0u8; needed as usize];
            let ok = GetTokenInformation(
                token,
                TokenIntegrityLevel,
                Some(buf.as_mut_ptr() as *mut _),
                needed,
                &mut needed,
            )
            .is_ok();
            let _ = CloseHandle(token);
            if !ok {
                return None;
            }
            let label = &*(buf.as_ptr() as *const TOKEN_MANDATORY_LABEL);
            let sid_attr: &SID_AND_ATTRIBUTES = &label.Label;
            let sid = sid_attr.Sid;
            if sid.0.is_null() {
                return None;
            }
            // The integrity RID is the last sub-authority of the label SID.
            let count_ptr = GetSidSubAuthorityCount(sid);
            if count_ptr.is_null() {
                return None;
            }
            let count = *count_ptr;
            if count == 0 {
                return None;
            }
            let rid_ptr = GetSidSubAuthority(sid, (count - 1) as u32);
            if rid_ptr.is_null() {
                return None;
            }
            Some(*rid_ptr)
        }
    }
}

#[cfg(windows)]
pub use imp::*;
