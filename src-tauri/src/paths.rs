//! Resolves where Carimbo stores its data (database, clipboard images, logs).
//!
//! Portable mode: if a file named `portable` (or `carimbo.portable`) sits next
//! to the executable, we keep all data in `<exe dir>\data\` so the app can run
//! from a USB stick or a locked-down machine without writing to %APPDATA%.
//! Otherwise we use the per-user app-data directory.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};

#[derive(Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
}

impl AppPaths {
    pub fn db_file(&self) -> PathBuf {
        self.data_dir.join("carimbo.db")
    }

    pub fn clips_dir(&self) -> PathBuf {
        self.data_dir.join("clips")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }
}

/// Determine the data directory and ensure it exists.
pub fn resolve<R: Runtime>(app: &AppHandle<R>) -> std::io::Result<AppPaths> {
    let data_dir = if let Some(portable) = portable_data_dir() {
        portable
    } else {
        // Per-user app data (e.g. %APPDATA%\br.com...carimbo on Windows).
        app.path()
            .app_data_dir()
            .unwrap_or_else(|_| std::env::temp_dir().join("carimbo"))
    };
    std::fs::create_dir_all(&data_dir)?;
    Ok(AppPaths { data_dir })
}

/// If running in portable mode, the `<exe dir>\data` path; else None.
fn portable_data_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let marker = dir.join("portable").exists() || dir.join("carimbo.portable").exists();
    if marker {
        Some(dir.join("data"))
    } else {
        None
    }
}
