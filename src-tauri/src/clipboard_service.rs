//! Wires the OS clipboard monitor to the clip repository: on each clipboard
//! change, read the content (honouring privacy-exclusion formats and skipping
//! our own paste writes), hash it, persist via `clip_repo`, and notify the UI.
//!
//! Also owns the periodic retention job.

use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Runtime};

use crate::core::classify::{classify_text, ContentType};
use crate::core::db::Db;
use crate::core::models::{ClipKind, NewClip};
use crate::core::repo::{clip_repo, settings_repo};
use crate::paths::AppPaths;

pub const CLIPBOARD_NEW: &str = "clipboard:new-entry";

const IMAGE_SUBDIR_MARKER: &str = "clips";
// Cap is on the RAW RGBA buffer, not the encoded PNG — a 4K full-screen
// PrintScreen is ~33 MB raw (3840×2160×4) but only a few MB as PNG, so the
// limit must comfortably clear common screenshot sizes.
const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;

/// Default retention: 30 days or 500 items, whichever is hit first.
const DEFAULT_RETENTION_DAYS: i64 = 30;
const DEFAULT_RETENTION_MAX: i64 = 500;

/// Start the clipboard monitor + retention loop. Returns a guard that stops
/// monitoring when dropped (kept alive in app state).
#[cfg(windows)]
pub fn start<R: Runtime>(
    app: AppHandle<R>,
    db: Arc<Db>,
    paths: AppPaths,
) -> Result<crate::platform::os::clipboard_monitor::ClipboardMonitor, String> {
    use crate::platform::os::clipboard::sequence_number;
    use crate::platform::os::clipboard_monitor::{
        should_skip_current_clipboard, ClipboardMonitor,
    };

    std::fs::create_dir_all(paths.clips_dir()).ok();

    let (monitor, rx) = ClipboardMonitor::start()?;

    // Consumer thread: process each clipboard change.
    let app_for_thread = app.clone();
    let db_for_thread = db.clone();
    let clips_dir = paths.clips_dir();
    std::thread::Builder::new()
        .name("carimbo-clip-consumer".into())
        .spawn(move || {
            for _update in rx.iter() {
                // Skip if a privacy-exclusion marker is present (password mgrs).
                if should_skip_current_clipboard() {
                    continue;
                }
                // Skip our own paste writes (self-ignore).
                let seq = sequence_number();
                if crate::platform::os::paste::was_recent_self_write(seq) {
                    continue;
                }

                match read_and_persist(&db_for_thread, &clips_dir) {
                    Ok(true) => {
                        let _ = app_for_thread.emit(CLIPBOARD_NEW, ());
                    }
                    Ok(false) => {} // duplicate or nothing to capture
                    Err(e) => tracing::debug!("clipboard capture skipped: {e}"),
                }
            }
        })
        .map_err(|e| format!("failed to spawn clip consumer: {e}"))?;

    // Retention loop: run once at startup then hourly.
    spawn_retention_loop(app, db, paths);

    Ok(monitor)
}

/// Read the current clipboard (text preferred, then image) and persist it.
/// Returns Ok(true) if a new entry was stored.
#[cfg(windows)]
fn read_and_persist(db: &Db, clips_dir: &std::path::Path) -> Result<bool, String> {
    use sha2::{Digest, Sha256};

    // Where the copy came from (best-effort; informational only).
    let source_app = crate::platform::os::util::foreground_process_name();

    // Files first (CF_HDROP): an Explorer file copy is more specific than the
    // text/DIB the same copy may also expose. Store the paths as text so they're
    // searchable and re-pastable, tagged `files` for a reveal action.
    if let Some(files) = crate::platform::os::clipboard::get_files() {
        let joined = files.join("\n");
        let hash = format!("{:x}", Sha256::digest(joined.as_bytes()));
        let preview = if files.len() == 1 {
            files[0].rsplit(['\\', '/']).next().unwrap_or(&files[0]).to_string()
        } else {
            format!("{} arquivos", files.len())
        };
        let clip = NewClip {
            kind: ClipKind::Text,
            content_type: ContentType::Files,
            content: Some(joined),
            image_path: None,
            preview,
            content_hash: hash,
            source_app,
        };
        let conn = db.lock();
        return clip_repo::insert_dedup(&conn, &clip).map_err(|e| e.to_string());
    }

    // arboard handles CF_UNICODETEXT and CF_DIB decoding robustly.
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;

    // Then text.
    if let Ok(text) = cb.get_text() {
        if !text.is_empty() {
            let hash = format!("{:x}", Sha256::digest(text.as_bytes()));
            let preview: String = text.chars().take(200).collect();
            let content_type = classify_text(&text);
            let clip = NewClip {
                kind: ClipKind::Text,
                content_type,
                content: Some(text),
                image_path: None,
                preview,
                content_hash: hash,
                source_app,
            };
            let conn = db.lock();
            return clip_repo::insert_dedup(&conn, &clip).map_err(|e| e.to_string());
        }
    }

    // Finally image.
    if let Ok(img) = cb.get_image() {
        let bytes_len = img.bytes.len();
        if bytes_len > MAX_IMAGE_BYTES {
            return Ok(false);
        }
        // Hash the raw RGBA for dedupe.
        let hash = format!("{:x}", Sha256::digest(&img.bytes));
        // Encode to PNG on disk.
        let width = img.width as u32;
        let height = img.height as u32;
        let buffer = image::RgbaImage::from_raw(width, height, img.bytes.into_owned())
            .ok_or_else(|| "invalid image buffer".to_string())?;
        let filename = format!("{}.png", &hash[..16.min(hash.len())]);
        let path = clips_dir.join(&filename);
        buffer.save(&path).map_err(|e| e.to_string())?;

        let clip = NewClip {
            kind: ClipKind::Image,
            content_type: ContentType::Image,
            content: None,
            image_path: Some(path.to_string_lossy().into_owned()),
            preview: format!("Imagem {width}×{height}"),
            content_hash: hash,
            source_app,
        };
        let conn = db.lock();
        return clip_repo::insert_dedup(&conn, &clip).map_err(|e| e.to_string());
    }

    Ok(false)
}

fn spawn_retention_loop<R: Runtime>(app: AppHandle<R>, db: Arc<Db>, _paths: AppPaths) {
    std::thread::Builder::new()
        .name("carimbo-clip-retention".into())
        .spawn(move || {
            // Small initial delay so startup isn't contended.
            std::thread::sleep(Duration::from_secs(5));
            loop {
                run_retention_once(&db);
                let _ = &app; // reserved for future "retention ran" events
                std::thread::sleep(Duration::from_secs(3600));
            }
        })
        .ok();
}

/// Apply the retention policy once, deleting pruned image files from disk.
pub fn run_retention_once(db: &Db) {
    let (days, max_items) = {
        let conn = db.lock();
        let days = settings_repo::get(&conn, "clipboard.retentionDays")
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str::<i64>(&v).ok())
            .unwrap_or(DEFAULT_RETENTION_DAYS);
        let max = settings_repo::get(&conn, "clipboard.retentionMax")
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str::<i64>(&v).ok())
            .unwrap_or(DEFAULT_RETENTION_MAX);
        (days, max)
    };
    let max_age_ms = days.saturating_mul(24 * 60 * 60 * 1000);

    let removed = {
        let conn = db.lock();
        clip_repo::prune(&conn, max_age_ms, max_items).unwrap_or_default()
    };
    for path in removed {
        let _ = std::fs::remove_file(&path);
    }
    // Keep the marker referenced so the constant isn't dead on some cfgs.
    let _ = IMAGE_SUBDIR_MARKER;
}
