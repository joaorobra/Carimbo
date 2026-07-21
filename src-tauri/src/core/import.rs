//! Import snippet libraries from *other* expanders, so switching to Carimbo
//! doesn't mean retyping everything. Distinct from [`crate::core::backup`], which
//! round-trips Carimbo's own versioned envelope (stable ids, use-stats, folders).
//! Here the input is foreign, so we only ever map to the fields a human authored:
//! a trigger, a body, and a display name. Everything else (id, timestamps,
//! frecency, folder) is assigned fresh by [`snippet_repo::create`], which also
//! gives us trigger normalization and the same "conflict drops the trigger, never
//! the snippet" guarantee the backup importer relies on — for free.
//!
//! Three shapes cover the tools people actually leave:
//!   - **espanso** — YAML with a top-level `matches:` list of `trigger`/`replace`.
//!   - **CSV** — `abbreviation,expansion[,label]` (TextExpander/aText/Beeftext
//!     exports, or a spreadsheet a user built by hand). Header row auto-detected.
//!   - **JSON** — a plain array of `{ trigger, body, name }` objects, the
//!     universal escape hatch for anything that can emit JSON.
//!
//! Parsing is pure (`&str -> Vec<NewSnippet>`) and dependency-free so each format
//! is trivially unit-testable; the DB write lives in one place at the bottom.

use rusqlite::Connection;

use crate::core::error::{CoreError, CoreResult};
use crate::core::models::NewSnippet;
use crate::core::repo::snippet_repo::{self, ImportOutcome};

/// The source format of an import file. Chosen by the caller (from the file
/// extension, with a content sniff as a fallback) so parsing never has to guess
/// mid-stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportFormat {
    /// espanso `match` YAML (`matches:` list of `trigger`/`replace`).
    Espanso,
    /// Comma-separated `abbreviation,expansion[,label]`.
    Csv,
    /// JSON array of `{ trigger, body/replace/expansion, name }`.
    Json,
}

impl ImportFormat {
    /// Best-guess format from a file name's extension. Returns `None` for
    /// extensions we don't recognize so the caller can sniff the content instead.
    pub fn from_extension(name: &str) -> Option<Self> {
        let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        match ext.as_str() {
            "yml" | "yaml" => Some(ImportFormat::Espanso),
            "csv" | "tsv" => Some(ImportFormat::Csv),
            "json" => Some(ImportFormat::Json),
            _ => None,
        }
    }

    /// Sniff the format from the content when the extension is unhelpful. A cheap
    /// heuristic — good enough to pick a parser; a genuinely malformed file still
    /// fails loudly in [`parse`].
    pub fn sniff(content: &str) -> Self {
        let trimmed = content.trim_start();
        if trimmed.starts_with('[') || trimmed.starts_with('{') {
            ImportFormat::Json
        } else if content.contains("matches:") || content.contains("- trigger:") {
            ImportFormat::Espanso
        } else {
            ImportFormat::Csv
        }
    }
}

/// What an import did, so the UI can report a clear result. Mirrors the backup
/// [`ImportReport`](crate::core::backup::ImportReport) shape the frontend already
/// renders, minus folders (a foreign import never carries them).
#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignImportReport {
    /// Snippets successfully added to the library.
    pub snippets_added: usize,
    /// Added, but their shortcut collided with an existing one and was dropped
    /// (the snippet itself is kept — nothing is ever lost to a trigger clash).
    pub triggers_dropped: usize,
    /// Rows in the file that carried no usable body and so were skipped.
    pub skipped: usize,
}

/// Parse `content` in the given `format` into ready-to-create snippets. Never
/// fails on individual bad rows — they're dropped (and surface as `skipped` once
/// created). Only a structurally unreadable file (e.g. `[` that never closes for
/// JSON) returns `Err`.
pub fn parse(content: &str, format: ImportFormat) -> CoreResult<Vec<NewSnippet>> {
    match format {
        ImportFormat::Espanso => Ok(parse_espanso(content)),
        ImportFormat::Csv => Ok(parse_csv(content)),
        ImportFormat::Json => parse_json(content),
    }
}

/// Build a snippet from raw trigger/body/name parts, mapping common foreign
/// placeholder syntax to Carimbo tokens. Returns `None` when there's no body to
/// insert (an empty match is meaningless). The name falls back to the trigger,
/// then to the first line of the body, so every snippet is findable in search.
fn make_snippet(
    trigger: Option<String>,
    body: String,
    name: Option<String>,
) -> Option<NewSnippet> {
    let body = normalize_body(&body);
    if body.trim().is_empty() {
        return None;
    }
    let trigger = trigger
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    let name = name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .or_else(|| trigger.clone())
        .unwrap_or_else(|| first_line(&body));
    Some(NewSnippet {
        name,
        trigger,
        body,
        body_html: None,
        folder_id: None,
        is_favorite: false,
    })
}

/// Map foreign dynamic placeholders to Carimbo's `{token}` vocabulary where the
/// meaning is unambiguous. Unknown placeholders are left verbatim so nothing is
/// silently corrupted — the user sees them and can fix them in the editor.
fn normalize_body(body: &str) -> String {
    // espanso uses `$|$` to mark the caret position -> Carimbo `{cursor}`.
    // Its clipboard extension surfaces as `{{clipboard}}` -> `{clipboard}`.
    body.replace("$|$", "{cursor}")
        .replace("{{clipboard}}", "{clipboard}")
}

/// First non-empty line of a body, trimmed and length-capped, for use as a
/// display name when the source gave us none.
fn first_line(body: &str) -> String {
    let line = body
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("Imported snippet");
    let capped: String = line.chars().take(60).collect();
    if capped.is_empty() {
        "Imported snippet".to_string()
    } else {
        capped
    }
}

// --- espanso ----------------------------------------------------------------

/// Parse an espanso `match` file. We deliberately hand-parse the tiny subset we
/// need — a `matches:` list where each item has `trigger`/`triggers` and
/// `replace` — rather than pull in a full YAML dependency. Multi-line `replace`
/// blocks (`replace: |` / `>`) are supported; anything more exotic is skipped
/// (it drops to a `skipped` count, never crashes the import).
fn parse_espanso(content: &str) -> Vec<NewSnippet> {
    let mut out = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    // Pending fields for the match currently being assembled.
    let mut trigger: Option<String> = None;
    let mut label: Option<String> = None;
    let mut replace: Option<String> = None;

    let flush = |trigger: &mut Option<String>,
                 label: &mut Option<String>,
                 replace: &mut Option<String>,
                 out: &mut Vec<NewSnippet>| {
        if let Some(body) = replace.take() {
            if let Some(s) = make_snippet(trigger.take(), body, label.take()) {
                out.push(s);
            }
        }
        *trigger = None;
        *label = None;
    };

    while i < lines.len() {
        let raw = lines[i];
        let trimmed = raw.trim_start();

        // A new list item (`- trigger:` / `- triggers:`) starts a fresh match.
        let is_item_start = trimmed.starts_with("- ");
        if is_item_start {
            flush(&mut trigger, &mut label, &mut replace, &mut out);
        }

        // Strip a leading list dash so `- trigger: x` parses like `trigger: x`.
        let field = trimmed.strip_prefix("- ").unwrap_or(trimmed);

        if let Some(v) = field.strip_prefix("trigger:") {
            trigger = Some(unquote(v.trim()));
        } else if let Some(v) = field.strip_prefix("triggers:") {
            // Inline list `[";a", ";b"]` -> take the first; block list handled
            // by the `- ` items that follow (rare in practice, first wins).
            let first = v
                .trim()
                .trim_start_matches('[')
                .split(',')
                .next()
                .map(|s| unquote(s.trim().trim_end_matches(']')))
                .filter(|s| !s.is_empty());
            if first.is_some() {
                trigger = first;
            }
        } else if let Some(v) = field.strip_prefix("label:") {
            label = Some(unquote(v.trim()));
        } else if let Some(v) = field.strip_prefix("replace:") {
            let v = v.trim();
            if v == "|" || v == ">" || v == "|-" || v == ">-" || v.is_empty() {
                // Block scalar: gather the following more-indented lines.
                let (block, consumed) = gather_block(&lines, i + 1, raw);
                replace = Some(block);
                i += consumed;
            } else {
                replace = Some(unquote(v));
            }
        }
        i += 1;
    }
    // Flush the final match.
    flush(&mut trigger, &mut label, &mut replace, &mut out);
    out
}

/// Collect a YAML block scalar: consecutive lines indented deeper than the
/// `replace:` key's own indentation. Returns the de-indented text and how many
/// lines were consumed.
fn gather_block(lines: &[&str], start: usize, key_line: &str) -> (String, usize) {
    let base_indent = indent_of(key_line);
    let mut collected: Vec<&str> = Vec::new();
    let mut j = start;
    while j < lines.len() {
        let l = lines[j];
        if l.trim().is_empty() {
            collected.push("");
            j += 1;
            continue;
        }
        if indent_of(l) <= base_indent {
            break;
        }
        collected.push(l);
        j += 1;
    }
    // De-indent by the least indentation among non-empty collected lines.
    let min_indent = collected
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| indent_of(l))
        .min()
        .unwrap_or(0);
    let text = collected
        .iter()
        .map(|l| {
            if l.len() >= min_indent {
                &l[min_indent..]
            } else {
                l.trim_start()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    // Trim only the trailing blank lines the block collector may have added.
    (text.trim_end_matches('\n').to_string(), j - start)
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Strip matching surrounding single or double quotes and unescape the handful
/// of sequences that matter for a one-line YAML scalar.
fn unquote(s: &str) -> String {
    let s = s.trim();
    let inner = if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        &s[1..s.len() - 1]
    } else {
        s
    };
    inner
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
}

// --- CSV --------------------------------------------------------------------

/// Parse a comma- (or tab-) separated file: `abbreviation,expansion[,label]`.
/// A header row is detected and skipped when the first column looks like a
/// column name (`trigger`/`abbreviation`/`shortcut`/`keyword`). Quoted fields
/// with embedded commas, quotes (`""`), and newlines are handled.
fn parse_csv(content: &str) -> Vec<NewSnippet> {
    let delim = if content.contains('\t') && !content.contains(',') {
        '\t'
    } else {
        ','
    };
    let rows = parse_csv_rows(content, delim);
    let mut out = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        if row.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        // Skip a header row if the first cell names a column.
        if idx == 0 {
            let head = row[0].trim().to_ascii_lowercase();
            if matches!(
                head.as_str(),
                "trigger" | "abbreviation" | "shortcut" | "keyword" | "abbr"
            ) {
                continue;
            }
        }
        let trigger = row.first().cloned();
        let body = row.get(1).cloned().unwrap_or_default();
        let label = row.get(2).cloned();
        if let Some(s) = make_snippet(trigger, body, label) {
            out.push(s);
        }
    }
    out
}

/// Minimal RFC-4180-ish CSV reader: quote-aware, handles `""` escapes and
/// newlines inside quoted fields. Sufficient for the flat two/three-column
/// exports expanders produce.
fn parse_csv_rows(content: &str, delim: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
        } else if c == '"' {
            in_quotes = true;
        } else if c == delim {
            row.push(std::mem::take(&mut field));
        } else if c == '\n' {
            row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut row));
        } else if c == '\r' {
            // swallow; the following \n (if any) closes the row
        } else {
            field.push(c);
        }
    }
    // Final field/row if the file didn't end with a newline.
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

// --- JSON -------------------------------------------------------------------

/// Parse a JSON array of snippet-ish objects. Each object may name its body via
/// `body`, `replace`, `expansion`, or `text`, and its trigger via `trigger`,
/// `abbreviation`, or `shortcut` — covering the common export vocabularies. A
/// top-level object with a `matches` array (espanso-as-JSON) is unwrapped.
fn parse_json(content: &str) -> CoreResult<Vec<NewSnippet>> {
    let value: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| CoreError::Invalid(format!("not valid JSON: {e}")))?;

    let items = match value {
        serde_json::Value::Array(a) => a,
        serde_json::Value::Object(ref o) => o
            .get("matches")
            .or_else(|| o.get("snippets"))
            .and_then(|v| v.as_array())
            .cloned()
            .ok_or_else(|| {
                CoreError::Invalid(
                    "JSON must be an array of snippets, or an object with a \
                     \"matches\"/\"snippets\" array"
                        .into(),
                )
            })?,
        _ => {
            return Err(CoreError::Invalid(
                "JSON must be an array of snippets".into(),
            ))
        }
    };

    let pick = |o: &serde_json::Map<String, serde_json::Value>, keys: &[&str]| {
        keys.iter()
            .find_map(|k| o.get(*k).and_then(|v| v.as_str()))
            .map(|s| s.to_string())
    };

    let mut out = Vec::new();
    for item in items {
        let Some(o) = item.as_object() else { continue };
        let body = pick(o, &["body", "replace", "expansion", "text", "content"]);
        let Some(body) = body else { continue };
        let trigger = pick(o, &["trigger", "abbreviation", "shortcut", "keyword", "abbr"]);
        let name = pick(o, &["name", "label", "title"]);
        if let Some(s) = make_snippet(trigger, body, name) {
            out.push(s);
        }
    }
    Ok(out)
}

// --- persistence ------------------------------------------------------------

/// Parse `content` and create every resulting snippet, reusing
/// [`snippet_repo::create`] so trigger normalization and conflict handling match
/// the rest of the app exactly. A trigger that clashes with an existing one is
/// retried without the trigger (the snippet still imports) — counted separately
/// so the UI can tell the user which shortcuts didn't survive.
pub fn import(
    conn: &Connection,
    content: &str,
    format: ImportFormat,
) -> CoreResult<ForeignImportReport> {
    let snippets = parse(content, format)?;
    let mut report = ForeignImportReport::default();
    for mut s in snippets {
        match create_absorbing_conflict(conn, &mut s)? {
            ImportOutcome::Inserted => report.snippets_added += 1,
            ImportOutcome::InsertedTriggerDropped => {
                report.snippets_added += 1;
                report.triggers_dropped += 1;
            }
            ImportOutcome::Skipped => report.skipped += 1,
        }
    }
    Ok(report)
}

/// Create one snippet; on a duplicate-trigger error, retry without the trigger so
/// the content is never lost. Mirrors the backup importer's guarantee for a
/// fresh (new-id) snippet.
fn create_absorbing_conflict(
    conn: &Connection,
    s: &mut NewSnippet,
) -> CoreResult<ImportOutcome> {
    match snippet_repo::create(conn, s.clone()) {
        Ok(_) => Ok(ImportOutcome::Inserted),
        Err(CoreError::DuplicateTrigger(_)) if s.trigger.is_some() => {
            let mut without = s.clone();
            without.trigger = None;
            snippet_repo::create(conn, without)?;
            Ok(ImportOutcome::InsertedTriggerDropped)
        }
        Err(CoreError::Invalid(_)) => Ok(ImportOutcome::Skipped),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::Db;
    use crate::core::repo::snippet_repo;

    #[test]
    fn espanso_simple_matches() {
        let yaml = "matches:\n  - trigger: \";br\"\n    replace: \"Best regards,\"\n  - trigger: \";cpf\"\n    replace: \"123.456.789-00\"\n";
        let s = parse_espanso(yaml);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].trigger.as_deref(), Some(";br"));
        assert_eq!(s[0].body, "Best regards,");
        assert_eq!(s[1].trigger.as_deref(), Some(";cpf"));
    }

    #[test]
    fn espanso_multiline_block_and_cursor() {
        let yaml = "matches:\n  - trigger: \":sig\"\n    replace: |\n      Line one\n      Line two $|$\n";
        let s = parse_espanso(yaml);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].body, "Line one\nLine two {cursor}");
    }

    #[test]
    fn espanso_label_becomes_name() {
        let yaml =
            "matches:\n  - trigger: \";addr\"\n    label: Home address\n    replace: 123 Main St\n";
        let s = parse_espanso(yaml);
        assert_eq!(s[0].name, "Home address");
    }

    #[test]
    fn csv_with_header_and_quotes() {
        let csv = "abbreviation,expansion,label\n\";hi\",\"Hello, world\",Greeting\n\";x\",bye,\n";
        let s = parse_csv(csv);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].trigger.as_deref(), Some(";hi"));
        assert_eq!(s[0].body, "Hello, world"); // embedded comma survived
        assert_eq!(s[0].name, "Greeting");
        assert_eq!(s[1].body, "bye");
    }

    #[test]
    fn csv_no_header_two_columns() {
        let csv = ";ty,Thank you!\n";
        let s = parse_csv(csv);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].trigger.as_deref(), Some(";ty"));
        assert_eq!(s[0].name, ";ty"); // falls back to trigger
    }

    #[test]
    fn json_array_various_keys() {
        let json = r#"[
            {"trigger": ";a", "body": "Alpha"},
            {"abbreviation": ";b", "expansion": "Bravo", "name": "B"},
            {"replace": "no trigger ok"}
        ]"#;
        let s = parse_json(json).unwrap();
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].body, "Alpha");
        assert_eq!(s[1].trigger.as_deref(), Some(";b"));
        assert_eq!(s[1].name, "B");
        assert_eq!(s[2].trigger, None);
    }

    #[test]
    fn json_object_with_matches_wrapper() {
        let json = r#"{"matches": [{"trigger": ";z", "replace": "Zed"}]}"#;
        let s = parse_json(json).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].body, "Zed");
    }

    #[test]
    fn empty_body_rows_are_dropped() {
        let csv = ";empty,\n;good,has body\n";
        let s = parse_csv(csv);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].trigger.as_deref(), Some(";good"));
    }

    #[test]
    fn format_detection() {
        assert_eq!(ImportFormat::from_extension("a.yml"), Some(ImportFormat::Espanso));
        assert_eq!(ImportFormat::from_extension("A.CSV"), Some(ImportFormat::Csv));
        assert_eq!(ImportFormat::from_extension("x.json"), Some(ImportFormat::Json));
        assert_eq!(ImportFormat::from_extension("x.txt"), None);
        assert_eq!(ImportFormat::sniff("[{\"a\":1}]"), ImportFormat::Json);
        assert_eq!(ImportFormat::sniff("matches:\n  - trigger: x"), ImportFormat::Espanso);
        assert_eq!(ImportFormat::sniff(";a,b\n"), ImportFormat::Csv);
    }

    #[test]
    fn import_into_db_reports_counts_and_conflicts() {
        let db = Db::open_in_memory().unwrap();
        {
            let conn = db.lock();
            // Pre-existing snippet owns ";dup".
            snippet_repo::create(
                &conn,
                NewSnippet {
                    name: "Existing".into(),
                    trigger: Some(";dup".into()),
                    body: "already here".into(),
                    body_html: None,
                    folder_id: None,
                    is_favorite: false,
                },
            )
            .unwrap();
        }
        let csv = ";new,Fresh one\n;dup,Wants dup trigger\n";
        let conn = db.lock();
        let report = import(&conn, csv, ImportFormat::Csv).unwrap();
        assert_eq!(report.snippets_added, 2);
        assert_eq!(report.triggers_dropped, 1); // ";dup" collided, kept snippet
        // Three snippets total now; only one carries ";dup".
        let all = snippet_repo::list(&conn).unwrap();
        assert_eq!(all.len(), 3);
        let dup = all.iter().filter(|x| x.trigger.as_deref() == Some(";dup")).count();
        assert_eq!(dup, 1);
    }
}
