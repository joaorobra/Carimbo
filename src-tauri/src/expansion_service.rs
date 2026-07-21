//! Abbreviation-expansion service: productionizes the M0 keyboard-hook spike.
//!
//! Owns the low-level keyboard hook thread and a consumer that feeds keystrokes
//! into the OS-agnostic `Matcher`. On a trigger match it deletes the typed
//! trigger (backspaces) and injects the token-expanded replacement into the
//! focused app via the shared paste module.
//!
//! The service can be started and stopped at runtime (settings toggle / tray
//! pause), which is why the hook was designed to be re-installable.

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use serde::Serialize;
    use tauri::{AppHandle, Emitter, Manager, Runtime};

    use crate::core::db::Db;
    use crate::core::expansion::Matcher;
    use crate::core::repo::snippet_repo;
    use crate::core::tokens::{self, TokenContext};
    use crate::platform::os::clipboard;
    use crate::platform::os::focus;
    use crate::platform::os::keyboard_hook::KeyboardHook;
    use crate::platform::os::keymap;
    use crate::platform::os::paste::{insert_into_with_cursor, InjectMethod, InsertResult};
    use crate::state::{AppState, PendingRadial};

    /// Event emitted when an expansion is attempted into an elevated window we
    /// can't inject to — the UI shows a toast.
    pub const EXPANSION_BLOCKED: &str = "expansion:blocked-elevated";

    /// Event carrying the candidates for the disambiguation radial. The radial
    /// window listens for this to render the choices.
    pub const RADIAL_SHOW: &str = "radial:show";

    /// Max edit distance for a trigger to count as "confusably similar" to what
    /// the user typed (1–2 letters or case, per the product requirement).
    const RADIAL_MAX_DIST: usize = 2;

    /// One choice offered in the radial picker (serialized to the radial UI).
    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RadialCandidate {
        pub id: String,
        pub name: String,
        pub trigger: String,
        /// Short preview of the body (first line, trimmed) for the label.
        pub preview: String,
        /// Edit distance from what the user typed (0 = exact); drives ordering.
        pub distance: usize,
    }

    /// Handle to the running service. Dropping it uninstalls the hook and stops
    /// the consumer thread.
    pub struct ExpansionService {
        // Dropping the hook posts WM_QUIT to its thread and joins it.
        _hook: KeyboardHook,
        stop: Arc<AtomicBool>,
        paused: Arc<AtomicBool>,
        consumer: Option<std::thread::JoinHandle<()>>,
        // Shared matcher so triggers can be hot-reloaded without a restart.
        matcher: Arc<Mutex<Matcher>>,
    }

    impl ExpansionService {
        /// Install the hook and start processing. `inject_method` chooses paste
        /// vs type-out.
        pub fn start<R: Runtime>(
            app: AppHandle<R>,
            db: Arc<Db>,
            inject_method: InjectMethod,
        ) -> Result<Self, String> {
            let (hook, rx) = KeyboardHook::install()?;

            let matcher = Arc::new(Mutex::new(Matcher::new()));
            load_triggers(&db, &matcher);

            let stop = Arc::new(AtomicBool::new(false));
            let paused = Arc::new(AtomicBool::new(false));

            let matcher_c = matcher.clone();
            let stop_c = stop.clone();
            let paused_c = paused.clone();
            let db_c = db.clone();
            let app_c = app.clone();

            let consumer = std::thread::Builder::new()
                .name("carimbo-expansion".into())
                .spawn(move || {
                    for ev in rx.iter() {
                        if stop_c.load(Ordering::Relaxed) {
                            break;
                        }
                        if paused_c.load(Ordering::Relaxed) {
                            continue;
                        }
                        // Never process our own injected keystrokes.
                        if ev.injected {
                            continue;
                        }

                        // Update the buffer according to the key.
                        let expansion = {
                            let mut m = matcher_c.lock().unwrap();
                            if keymap::is_backspace(ev.vk) {
                                m.backspace();
                                None
                            } else if keymap::is_buffer_resetting_key(ev.vk) {
                                m.clear_buffer();
                                None
                            } else if let Some(c) = keymap::vk_to_char(ev.vk, ev.scan) {
                                m.push_char(c)
                            } else {
                                // Dead key / modifier: leave buffer as-is.
                                None
                            }
                        };

                        if let Some(exp) = expansion {
                            perform_expansion(&app_c, &db_c, &matcher_c, &exp, inject_method);
                        }
                    }
                })
                .map_err(|e| format!("failed to spawn expansion consumer: {e}"))?;

            Ok(ExpansionService {
                _hook: hook,
                stop,
                paused,
                consumer: Some(consumer),
                matcher,
            })
        }

        /// Reload the trigger set from the DB (call on `snippets:changed`).
        pub fn reload_triggers(&self, db: &Db) {
            load_triggers(db, &self.matcher);
        }

        pub fn set_paused(&self, paused: bool) {
            self.paused.store(paused, Ordering::Relaxed);
        }

        pub fn is_paused(&self) -> bool {
            self.paused.load(Ordering::Relaxed)
        }
    }

    impl Drop for ExpansionService {
        fn drop(&mut self) {
            // Signal the consumer to stop. The remaining fields drop in
            // declaration order, so `_hook` drops next: that posts WM_QUIT to the
            // hook thread and closes the keystroke channel, which ends the
            // consumer's `rx.iter()` and lets it exit. We don't join here — the
            // consumer is short-lived and exits as soon as the channel closes.
            self.stop.store(true, Ordering::Relaxed);
            // Detach the join handle; joining would block until the next
            // keystroke or channel close, with no benefit at shutdown.
            let _ = self.consumer.take();
        }
    }

    /// True if the current foreground app's executable is on the user's
    /// expansion-exclusion list (stored as a JSON array of exe names under
    /// `expansion.excludedApps`). Matching is case-insensitive on the base exe
    /// name (e.g. "KeePass.exe"). Best-effort: if we can't read the foreground
    /// process or the setting, we do NOT exclude (fail open — expansion still
    /// works rather than silently dying everywhere).
    fn foreground_is_excluded(db: &Db) -> bool {
        let Some(exe) = crate::platform::os::util::foreground_process_name() else {
            return false;
        };
        let excluded = {
            let conn = db.lock();
            crate::core::repo::settings_repo::excluded_apps(&conn)
        };
        let exe_lc = exe.to_lowercase();
        excluded.iter().any(|e| e.to_lowercase() == exe_lc)
    }

    fn load_triggers(db: &Db, matcher: &Arc<Mutex<Matcher>>) {
        let triggers = {
            let conn = db.lock();
            snippet_repo::all_triggers(&conn).unwrap_or_default()
        };
        if let Ok(mut m) = matcher.lock() {
            m.set_triggers(triggers);
        }
    }

    fn perform_expansion<R: Runtime>(
        app: &AppHandle<R>,
        db: &Db,
        matcher: &Arc<Mutex<Matcher>>,
        exp: &crate::core::expansion::Expansion,
        method: InjectMethod,
    ) {
        // Respect the per-app exclusion list: if the foreground app is one the
        // user chose never to expand in (password managers, terminals, games…),
        // silently skip. The typed trigger text is left as-is — nothing is lost.
        if foreground_is_excluded(db) {
            return;
        }

        // If the trigger the user typed is confusably similar to other triggers
        // (off by 1–2 letters or case), we don't guess — we show a small radial
        // picker and let them choose. Otherwise expand directly (the common case).
        let similar = {
            let m = matcher.lock().unwrap();
            m.similar_triggers(&exp.matched_trigger, RADIAL_MAX_DIST)
        };
        if similar.len() > 1 {
            if try_show_radial(app, db, exp, &similar) {
                return; // radial is up; completion happens on user pick
            }
            // If we couldn't build/show the radial, fall through to a direct
            // expansion so the user still gets *something*.
        }

        // If the fired snippet has `[[variables]]`, we can't fill them silently:
        // open the palette's fill-in form instead of expanding to blanks. Only
        // the unambiguous (non-radial) path reaches here.
        if !tokens::extract_variables(&exp.replacement).is_empty()
            && try_open_form(app, db, exp)
        {
            return; // form is up; completion happens on user submit
        }

        expand_now(app, db, &exp.replacement, None, exp.delete_chars, method);
    }

    /// Directly delete the trigger and inject the (token-expanded) replacement
    /// into the foreground app. Shared by the direct path and the radial pick.
    ///
    /// `snippet_id` identifies the snippet for the usage bump. The radial pick
    /// knows the exact id; the direct path passes `None` and we fall back to
    /// matching on the body (which is fine — the direct path only runs when the
    /// trigger was unambiguous).
    pub fn expand_now<R: Runtime>(
        app: &AppHandle<R>,
        db: &Db,
        replacement: &str,
        snippet_id: Option<&str>,
        delete_chars: usize,
        method: InjectMethod,
    ) {
        // Resolve the snippet's id (for the usage bump) and its optional rich
        // HTML body, up front and under one short lock. The radial path passes an
        // explicit id; the direct path finds the snippet by its body.
        let region = {
            let conn = db.lock();
            crate::core::region::Region::load(&conn)
        };
        let (resolved_id, body_html): (Option<String>, Option<String>) = {
            let conn = db.lock();
            match snippet_id {
                Some(id) => match snippet_repo::get(&conn, id) {
                    Ok(s) => (Some(s.id), s.body_html),
                    Err(_) => (Some(id.to_string()), None),
                },
                None => match snippet_repo::list(&conn) {
                    Ok(list) => list
                        .into_iter()
                        .find(|s| s.trigger.is_some() && s.body == replacement)
                        .map(|s| (Some(s.id), s.body_html))
                        .unwrap_or((None, None)),
                    Err(_) => (None, None),
                },
            }
        };

        // Resolve {clipboard} from the live clipboard only if needed (plain or
        // rich body references it). Form variables ([[key]]) can't be prompted
        // for on the typed-trigger path, so they expand to empty.
        let needs_clip = replacement.contains("{clipboard}")
            || body_html.as_deref().is_some_and(|h| h.contains("{clipboard}"));
        let ctx = TokenContext {
            clipboard: if needs_clip {
                clipboard::get_text()
            } else {
                None
            },
            ..Default::default()
        };
        let expanded = tokens::expand_full(replacement, &ctx, region);

        // The user is typing in the target already, so it IS the foreground
        // window. Capture it to satisfy insert_into's contract.
        let Some(target) = focus::capture_foreground() else {
            return;
        };

        let result = match body_html {
            // Rich snippet: paste HTML with the plain body as the fallback.
            // {cursor} isn't honoured in rich mode (caret math across markup
            // isn't reliable).
            Some(html) => {
                let html_expanded = tokens::expand(&html, &ctx, region);
                crate::platform::os::paste::insert_html_into(
                    target,
                    &html_expanded,
                    &expanded.text,
                    delete_chars,
                )
            }
            None => insert_into_with_cursor(
                target,
                &expanded.text,
                method,
                delete_chars,
                expanded.cursor_from_end,
            ),
        };
        match result {
            InsertResult::Elevated => {
                let _ = app.emit(EXPANSION_BLOCKED, ());
            }
            InsertResult::Ok => {
                if let Some(id) = resolved_id {
                    let conn = db.lock();
                    let _ = snippet_repo::bump_use(&conn, &id);
                }
            }
            _ => {}
        }
    }

    /// Build the candidate list and show the radial. Returns false if we can't
    /// (no candidates resolved, no foreground window, or no radial window), so
    /// the caller can fall back to a direct expansion.
    fn try_show_radial<R: Runtime>(
        app: &AppHandle<R>,
        db: &Db,
        exp: &crate::core::expansion::Expansion,
        similar: &[String],
    ) -> bool {
        // Resolve the similar triggers to full snippets, then rank by relevance:
        // exact-match-to-typed first, then closer edit distance, then most-used,
        // then alphabetical for stability.
        let snippets = {
            let conn = db.lock();
            match snippet_repo::by_triggers(&conn, similar) {
                Ok(s) => s,
                Err(_) => return false,
            }
        };
        if snippets.len() < 2 {
            return false;
        }

        let mut candidates: Vec<RadialCandidate> = snippets
            .into_iter()
            .map(|s| {
                let trig = s.trigger.clone().unwrap_or_default();
                let distance = crate::core::expansion::similar_distance(
                    &exp.matched_trigger,
                    &trig.to_lowercase(),
                    RADIAL_MAX_DIST,
                );
                RadialCandidate {
                    id: s.id,
                    name: s.name,
                    trigger: trig,
                    preview: preview_of(&s.body),
                    distance,
                }
            })
            .collect();
        candidates.sort_by(|a, b| {
            a.distance
                .cmp(&b.distance)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        // Record what we need to finish once the user chooses: the app to paste
        // back into and how much of the typed trigger to delete first.
        let Some(target) = focus::capture_foreground() else {
            return false;
        };
        if let Some(state) = app.try_state::<AppState>() {
            state.set_pending_radial(PendingRadial {
                target,
                delete_chars: exp.delete_chars,
            });
        } else {
            return false;
        }

        crate::commands::radial::show_radial(app, candidates)
    }

    /// Resolve the fired snippet by its matched trigger, capture the app being
    /// typed in, and open the palette straight into the fill-in form. Returns
    /// false if we can't (no snippet, no foreground window) so the caller falls
    /// back to a direct (blank-variable) expansion — better than nothing.
    fn try_open_form<R: Runtime>(
        app: &AppHandle<R>,
        db: &Db,
        exp: &crate::core::expansion::Expansion,
    ) -> bool {
        // `matched_trigger` is lowercased; `by_triggers` matches case-insensitively.
        let snippet = {
            let conn = db.lock();
            match snippet_repo::by_triggers(&conn, std::slice::from_ref(&exp.matched_trigger)) {
                Ok(mut s) => s.pop(),
                Err(_) => None,
            }
        };
        let Some(snippet) = snippet else {
            return false;
        };
        let Some(target) = focus::capture_foreground() else {
            return false;
        };

        crate::commands::palette::open_palette_form(
            app,
            snippet.id,
            target,
            exp.delete_chars,
        );
        true
    }

    /// First non-empty line of the body, trimmed and length-capped, for a label.
    fn preview_of(body: &str) -> String {
        let line = body.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        let line = line.trim();
        let max = 48;
        if line.chars().count() > max {
            let truncated: String = line.chars().take(max).collect();
            format!("{truncated}…")
        } else {
            line.to_string()
        }
    }
}

#[cfg(windows)]
pub use imp::*;
