//! Thin wrappers over ShellExecuteW for the clipboard's type-specific actions:
//! open a URL in the default browser and reveal a file in Explorer. We use raw
//! Win32 (rather than a shell plugin) to stay consistent with the rest of the
//! platform layer and avoid extra capability plumbing.

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;

    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    fn to_wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// Open `target` with the default handler ("open" verb). For a URL this
    /// launches the default browser; for a file, its associated app. Returns
    /// false if the shell refused (the HINSTANCE result is <= 32 on failure).
    pub fn open(target: &str) -> bool {
        let verb = to_wide("open");
        let file = to_wide(target);
        let hinst = unsafe {
            ShellExecuteW(
                None,
                PCWSTR(verb.as_ptr()),
                PCWSTR(file.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        hinst.0 as isize > 32
    }

    /// Reveal `path` in Explorer, selecting the file. Uses `explorer /select,`.
    pub fn reveal(path: &str) -> bool {
        let verb = to_wide("open");
        let explorer = to_wide("explorer.exe");
        // The /select, argument must NOT be quoted around the whole thing; quote
        // only the path so paths with spaces work.
        let args = to_wide(&format!("/select,\"{path}\""));
        let hinst = unsafe {
            ShellExecuteW(
                None,
                PCWSTR(verb.as_ptr()),
                PCWSTR(explorer.as_ptr()),
                PCWSTR(args.as_ptr()),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        hinst.0 as isize > 32
    }
}

#[cfg(windows)]
pub use imp::*;
